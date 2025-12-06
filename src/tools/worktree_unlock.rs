//! Git worktree unlock tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitWorktreeUnlockArgs, GitWorktreeUnlockOutput, WorktreeUnlockPrompts};
use std::path::{Path, PathBuf};

/// Tool for unlocking worktrees
#[derive(Clone)]
pub struct GitWorktreeUnlockTool;

impl Tool for GitWorktreeUnlockTool {
    type Args = GitWorktreeUnlockArgs;
    type Prompts = WorktreeUnlockPrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_WORKTREE_UNLOCK
    }

    fn description() -> &'static str {
        "Unlock a locked worktree. \
         Removes the lock that prevents worktree deletion."
    }

    fn read_only() -> bool {
        false // Removes lock file
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        false // Fails if not locked
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Execute worktree unlock
        crate::worktree_unlock(repo, PathBuf::from(&args.worktree_path))
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Terminal summary
        let summary = format!(
            "\x1b[32m Worktree Unlocked: {}\x1b[0m\n\
              Status: unlocked",
            args.worktree_path
        );

        Ok(ToolResponse::new(summary, GitWorktreeUnlockOutput {
            success: true,
            worktree_path: args.worktree_path.clone(),
            message: "Worktree unlocked".to_string(),
        }))
    }
}
