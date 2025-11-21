//! Git stash tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitStashArgs, GitStashPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for stashing changes
#[derive(Clone)]
pub struct GitStashTool;

impl Tool for GitStashTool {
    type Args = GitStashArgs;
    type PromptArgs = GitStashPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_STASH
    }

    fn description() -> &'static str {
        "Save uncommitted changes without committing. \
         Operations: 'save' to stash changes, 'pop' to apply and remove stash."
    }

    fn read_only() -> bool {
        false // Modifies working directory and creates refs
    }

    fn destructive() -> bool {
        false // Non-destructive (stash preserves changes)
    }

    fn idempotent() -> bool {
        false // Pop is not idempotent (consumes stash)
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {}", e)))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{}", e)))?;

        let mut contents = Vec::new();

        if args.operation.as_str() == "save" {
            // Save stash
            let opts = crate::StashOpts {
                message: args.message.clone(),
                include_untracked: args.include_untracked,
            };

            let stash_info = crate::stash_save(repo.clone(), opts)
                .await
                .map_err(|e| McpError::Other(anyhow::anyhow!("{}", e)))?;

            // Terminal summary
            let commit_short = &stash_info.commit_hash[..7.min(stash_info.commit_hash.len())];
            let summary = format!(
                "\x1b[36m 💾 Stash Saved\x1b[0m\n\
                 ℹ {}\n\
                 Commit: {}",
                stash_info.message,
                commit_short
            );
            contents.push(Content::text(summary));

            // JSON metadata
            let metadata = json!({
                "success": true,
                "operation": "save",
                "name": stash_info.name,
                "message": stash_info.message,
                "commit_hash": stash_info.commit_hash
            });
            let json_str = serde_json::to_string_pretty(&metadata)
                .unwrap_or_else(|_| "{}".to_string());
            contents.push(Content::text(json_str));
        } else if args.operation.as_str() == "pop" {
            // Pop stash
            crate::stash_pop(repo, None)
                .await
                .map_err(|e| McpError::Other(anyhow::anyhow!("{}", e)))?;

            // Terminal summary
            let summary = "\x1b[32m ✓ Stash Popped\x1b[0m\n\
                 Changes restored to working directory".to_string();
            contents.push(Content::text(summary));

            // JSON metadata
            let metadata = json!({
                "success": true,
                "operation": "pop"
            });
            let json_str = serde_json::to_string_pretty(&metadata)
                .unwrap_or_else(|_| "{}".to_string());
            contents.push(Content::text(json_str));
        } else {
            return Err(McpError::Other(anyhow::anyhow!(
                "Invalid stash operation: {}. Use 'save' or 'pop'",
                args.operation
            )));
        }

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
