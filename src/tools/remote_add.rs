//! Git remote add tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitRemoteAddArgs, GitRemoteAddOutput, RemoteAddPrompts};
use std::path::Path;

/// Tool for adding remote repositories
#[derive(Clone)]
pub struct GitRemoteAddTool;

impl Tool for GitRemoteAddTool {
    type Args = GitRemoteAddArgs;
    type Prompts = RemoteAddPrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_REMOTE_ADD
    }

    fn description() -> &'static str {
        "Add a new remote repository. \
         Configures a named remote with fetch/push URLs for collaboration."
    }

    fn read_only() -> bool {
        false // Modifies repository configuration
    }

    fn destructive() -> bool {
        false // Non-destructive operation
    }

    fn idempotent() -> bool {
        true // Safe to add same remote multiple times
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build add options
        let opts = crate::RemoteAddOpts {
            name: args.name.clone(),
            url: args.url.clone(),
            force: args.force,
        };

        // Execute add
        crate::add_remote(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Terminal summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "\x1b[32m Add Remote\x1b[0m\n  {} -> {}",
            args.name, args.url
        );

        Ok(ToolResponse::new(summary, GitRemoteAddOutput {
            success: true,
            name: args.name.clone(),
            url: args.url.clone(),
            message: format!("Added remote '{}' with URL '{}'", args.name, args.url),
        }))
    }
}
