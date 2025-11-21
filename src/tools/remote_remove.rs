//! Git remote remove tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitRemoteRemoveArgs, GitRemoteRemovePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for removing remote repositories
#[derive(Clone)]
pub struct GitRemoteRemoveTool;

impl Tool for GitRemoteRemoveTool {
    type Args = GitRemoteRemoveArgs;
    type PromptArgs = GitRemoteRemovePromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_REMOTE_REMOVE
    }

    fn description() -> &'static str {
        "Remove a configured remote repository. \
         Deletes the remote from repository configuration."
    }

    fn read_only() -> bool {
        false // Modifies repository configuration
    }

    fn destructive() -> bool {
        true // Removes configuration entries
    }

    fn idempotent() -> bool {
        false // Cannot remove non-existent remote
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Execute remove
        crate::remove_remote(repo, &args.name)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "\x1b[32m ✓ Remote Removed\x1b[0m\n\
             {} deleted from configuration",
            args.name
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "remote_name": args.name
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
