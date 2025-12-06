//! Git stash tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitStashArgs, GitStashOutput, StashPrompts};
use std::path::Path;

/// Tool for stashing changes
#[derive(Clone)]
pub struct GitStashTool;

impl Tool for GitStashTool {
    type Args = GitStashArgs;
    type Prompts = StashPrompts;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {}", e)))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{}", e)))?;

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

            Ok(ToolResponse::new(summary, GitStashOutput {
                success: true,
                operation: "save".to_string(),
                name: Some(stash_info.name),
                message: Some(stash_info.message),
                commit_hash: Some(stash_info.commit_hash),
            }))
        } else if args.operation.as_str() == "pop" {
            // Pop stash
            crate::stash_pop(repo, None)
                .await
                .map_err(|e| McpError::Other(anyhow::anyhow!("{}", e)))?;

            // Terminal summary
            let summary = "\x1b[32m ✓ Stash Popped\x1b[0m\n\
                 Changes restored to working directory".to_string();

            Ok(ToolResponse::new(summary, GitStashOutput {
                success: true,
                operation: "pop".to_string(),
                name: None,
                message: None,
                commit_hash: None,
            }))
        } else {
            Err(McpError::Other(anyhow::anyhow!(
                "Invalid stash operation: {}. Use 'save' or 'pop'",
                args.operation
            )))
        }
    }
}
