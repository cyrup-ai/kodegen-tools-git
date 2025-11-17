//! Git repository opening tool

use gix::bstr::ByteSlice;
use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitOpenArgs, GitOpenPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for opening existing Git repositories
#[derive(Clone)]
pub struct GitOpenTool;

impl Tool for GitOpenTool {
    type Args = GitOpenArgs;
    type PromptArgs = GitOpenPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_OPEN
    }

    fn description() -> &'static str {
        "Open an existing Git repository at the specified path. \
         The repository must already exist at the given location."
    }

    fn read_only() -> bool {
        true // Only reads, doesn't modify
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true // Opening same repo multiple times is safe
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Get current branch information - fully inlined to avoid Send issues
        let repo_for_branch = repo.clone();
        let branch_name = tokio::task::spawn_blocking(move || {
            let inner = repo_for_branch.clone_inner();

            let head = inner.head().map_err(|e| anyhow::anyhow!("Failed to get HEAD: {e}"))?;

            let branch = head
                .referent_name()
                .and_then(|name| {
                    name.shorten()
                        .to_str()
                        .ok()
                        .map(std::string::ToString::to_string)
                })
                .unwrap_or_else(|| "detached HEAD".to_string());

            Ok::<_, anyhow::Error>(branch)
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(McpError::Other)?;

        // Get is_clean status - inline to avoid Send issues
        let repo_for_clean = repo.clone();
        let is_clean = tokio::task::spawn_blocking(move || {
            let inner = repo_for_clean.clone_inner();
            inner.is_dirty().map(|dirty| !dirty)
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to check clean status: {e}")))?;

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let status = if is_clean { "clean" } else { "dirty" };
        let summary = format!(
            "\x1b[36m Open Repository: {}\x1b[0m\n\
              Branch: {} · Status: {}",
            args.path,
            branch_name,
            status
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "path": args.path,
            "branch": branch_name,
            "is_clean": is_clean,
            "message": format!("Opened Git repository at {}", args.path)
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to serialize metadata: {e}")))?;
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
