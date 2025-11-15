//! Git worktree list tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreeListArgs, GitWorktreeListPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for listing worktrees
#[derive(Clone)]
pub struct GitWorktreeListTool;

impl Tool for GitWorktreeListTool {
    type Args = GitWorktreeListArgs;
    type PromptArgs = GitWorktreeListPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_WORKTREE_LIST
    }

    fn description() -> &'static str {
        "List all worktrees in the repository with detailed status. \
         Returns main worktree and all linked worktrees with their paths, branches, \
         lock status, and HEAD information."
    }

    fn read_only() -> bool {
        true // Only lists
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true // Safe to call repeatedly
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Execute worktree list
        let worktrees = crate::list_worktrees(repo)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let worktrees_json: Vec<_> = worktrees
            .iter()
            .map(|wt| {
                json!({
                    "path": wt.path.display().to_string(),
                    "git_dir": wt.git_dir.display().to_string(),
                    "is_main": wt.is_main,
                    "is_bare": wt.is_bare,
                    "head_commit": wt.head_commit.map(|id| id.to_string()),
                    "head_branch": wt.head_branch.clone(),
                    "is_locked": wt.is_locked,
                    "lock_reason": wt.lock_reason.clone(),
                    "is_detached": wt.is_detached
                })
            })
            .collect();

        let mut contents = Vec::new();

        // Terminal summary
        let worktree_list = if worktrees.is_empty() {
            "No worktrees found".to_string()
        } else {
            worktrees.iter()
                .map(|wt| {
                    let main_indicator = if wt.is_main { " (main)" } else { "" };
                    let locked_indicator = if wt.is_locked { " [locked]" } else { "" };
                    let branch_info = wt.head_branch.as_ref()
                        .map(|b| format!(" - {}", b))
                        .unwrap_or_else(|| " - detached".to_string());
                    format!("  • {}{}{}{}", wt.path.display(), main_indicator, branch_info, locked_indicator)
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let summary = format!(
            "✓ Worktrees listed ({})\n\n{}",
            worktrees.len(),
            worktree_list
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "worktrees": worktrees_json,
            "count": worktrees.len()
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
