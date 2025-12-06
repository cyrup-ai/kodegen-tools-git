//! Git remote remove tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitRemoteRemoveArgs, GitRemoteRemoveOutput, RemoteRemovePrompts};
use std::path::Path;

/// Tool for removing remote repositories
#[derive(Clone)]
pub struct GitRemoteRemoveTool;

impl Tool for GitRemoteRemoveTool {
    type Args = GitRemoteRemoveArgs;
    type Prompts = RemoteRemovePrompts;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
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

        // Terminal summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "\x1b[32m Remote Removed\x1b[0m\n\
             {} deleted from configuration",
            args.name
        );

        Ok(ToolResponse::new(summary, GitRemoteRemoveOutput {
            success: true,
            name: args.name.clone(),
            message: format!("Remote '{}' removed", args.name),
        }))
    }
}
