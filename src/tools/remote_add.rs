//! Git remote add tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitRemoteAddArgs, GitRemoteAddPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for adding remote repositories
#[derive(Clone)]
pub struct GitRemoteAddTool;

impl Tool for GitRemoteAddTool {
    type Args = GitRemoteAddArgs;
    type PromptArgs = GitRemoteAddPromptArgs;

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

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
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

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "\x1b[32m Add Remote\x1b[0m\n  ✓ {} ➜ {}",
            args.name, args.url
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "remote_name": args.name,
            "remote_url": args.url
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
