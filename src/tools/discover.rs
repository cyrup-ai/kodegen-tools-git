//! Git repository discovery tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitDiscoverArgs, GitDiscoverPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for discovering Git repositories by searching upward
#[derive(Clone)]
pub struct GitDiscoverTool;

impl Tool for GitDiscoverTool {
    type Args = GitDiscoverArgs;
    type PromptArgs = GitDiscoverPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_DISCOVER
    }

    fn description() -> &'static str {
        "Discover a Git repository by searching upward from the given path. \
         This will traverse parent directories until it finds a .git directory \
         or reaches the filesystem root."
    }

    fn read_only() -> bool {
        true // Only searches, doesn't modify
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true // Safe to call repeatedly
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        let repo = crate::discover_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Extract the working directory path from the discovered repository
        let repo_root = repo.raw()
            .workdir()
            .ok_or_else(|| McpError::Other(anyhow::anyhow!("Repository has no working directory")))?
            .display()
            .to_string();

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "\x1b[36m Discover Repository: {}\x1b[0m\n\
              Started from: {} · Found: {}",
            repo_root, args.path, repo_root
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "searched_from": args.path,
            "message": format!("Discovered Git repository from path {}", args.path)
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
