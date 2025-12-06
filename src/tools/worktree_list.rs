//! Git worktree list tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitWorktreeListArgs, GitWorktreeListOutput, GitWorktreeInfo, WorktreeListPrompts};
use std::path::Path;

/// Tool for listing worktrees
#[derive(Clone)]
pub struct GitWorktreeListTool;

impl Tool for GitWorktreeListTool {
    type Args = GitWorktreeListArgs;
    type Prompts = WorktreeListPrompts;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
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

        let worktrees_output: Vec<GitWorktreeInfo> = worktrees
            .iter()
            .map(|wt| GitWorktreeInfo {
                path: wt.path.display().to_string(),
                git_dir: wt.git_dir.display().to_string(),
                is_main: wt.is_main,
                is_bare: wt.is_bare,
                head_commit: wt.head_commit.map(|id| id.to_string()),
                head_branch: wt.head_branch.clone(),
                is_locked: wt.is_locked,
                lock_reason: wt.lock_reason.clone(),
                is_detached: wt.is_detached,
            })
            .collect();

        // Terminal summary with ANSI color codes and Nerd Font icons
        let count = worktrees.len();
        let main_path = worktrees.iter()
            .find(|wt| wt.is_main)
            .map(|wt| wt.path.display().to_string())
            .unwrap_or_else(|| "none".to_string());

        let summary = format!(
            "\x1b[36m Worktrees\x1b[0m\n\
              Total: {} · Main: {}",
            count, main_path
        );

        Ok(ToolResponse::new(summary, GitWorktreeListOutput {
            success: true,
            worktrees: worktrees_output,
            count,
        }))
    }
}
