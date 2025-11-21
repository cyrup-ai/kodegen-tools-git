//! Git branch listing tool

use gix::bstr::ByteSlice;
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitBranchListArgs, GitBranchListPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for listing Git branches
#[derive(Clone)]
pub struct GitBranchListTool;

impl Tool for GitBranchListTool {
    type Args = GitBranchListArgs;
    type PromptArgs = GitBranchListPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_BRANCH_LIST
    }

    fn description() -> &'static str {
        "List all local branches in a Git repository."
    }

    fn read_only() -> bool {
        true // Only reads, doesn't modify
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true // Safe to call repeatedly
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Get current branch name
        // We clone the inner repository to avoid holding a reference across await points
        let repo_for_current = repo.clone();
        let current_branch_name = {
            let inner = repo_for_current.clone_inner();
            tokio::task::spawn_blocking(move || {
                let head = inner.head().ok()?;
                head.referent_name()
                    .and_then(|name| {
                        name.shorten()
                            .to_str()
                            .ok()
                            .map(std::string::ToString::to_string)
                    })
            })
            .await
            .ok()
            .and_then(|x| x)
            .unwrap_or_else(|| "unknown".to_string())
        };

        // List branches
        let branches = crate::list_branches(repo)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "\x1b[36m\u{EDA6} Branches\x1b[0m\n\
             \u{E725} Total: {} · Current: {}",
            branches.len(),
            current_branch_name
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "branches": branches,
            "count": branches.len()
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
