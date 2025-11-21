//! Git remote list tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitRemoteListArgs, GitRemoteListPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for listing remote repositories
#[derive(Clone)]
pub struct GitRemoteListTool;

impl Tool for GitRemoteListTool {
    type Args = GitRemoteListArgs;
    type PromptArgs = GitRemoteListPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_REMOTE_LIST
    }

    fn description() -> &'static str {
        "List all configured remote repositories. \
         Shows remote names and their fetch/push URLs."
    }

    fn read_only() -> bool {
        true // Only reads configuration
    }

    fn destructive() -> bool {
        false // No modifications
    }

    fn idempotent() -> bool {
        true // Safe to call multiple times
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository and list remotes in a spawn_blocking context
        // to avoid Send issues with RepoHandle
        let path_buf = path.to_path_buf();

        let remotes = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                // Open repository
                let repo = crate::open_repo(&path_buf)
                    .await
                    .map_err(|e| anyhow::anyhow!("Task execution failed: {e}"))?
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

                // List remotes
                crate::list_remotes(&repo)
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))
            })
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(|e: anyhow::Error| McpError::Other(e))?;

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let mut summary = format!(
            "\x1b[34m🔗 Remotes ({})\x1b[0m",
            remotes.len()
        );

        if remotes.is_empty() {
            summary.push_str("\n  No remotes configured");
        } else {
            for remote in &remotes {
                let urls = if remote.fetch_url == remote.push_url {
                    remote.fetch_url.clone()
                } else {
                    format!("fetch: {} | push: {}", remote.fetch_url, remote.push_url)
                };
                summary.push_str(&format!("\n  {} ➜ {}", remote.name, urls));
            }
        }

        contents.push(Content::text(summary));

        // JSON metadata
        let remote_list: Vec<serde_json::Value> = remotes
            .iter()
            .map(|r| {
                json!({
                    "name": r.name,
                    "fetch_url": r.fetch_url,
                    "push_url": r.push_url
                })
            })
            .collect();

        let metadata = json!({
            "success": true,
            "count": remotes.len(),
            "remotes": remote_list
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
