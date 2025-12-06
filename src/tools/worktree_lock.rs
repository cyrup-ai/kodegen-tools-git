//! Git worktree lock tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitWorktreeLockArgs, GitWorktreeLockOutput, WorktreeLockPrompts};
use std::path::Path;

/// Tool for locking worktrees
#[derive(Clone)]
pub struct GitWorktreeLockTool;

impl Tool for GitWorktreeLockTool {
    type Args = GitWorktreeLockArgs;
    type Prompts = WorktreeLockPrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_WORKTREE_LOCK
    }

    fn description() -> &'static str {
        "Lock a worktree to prevent deletion. \
         Useful for worktrees on removable media or network drives."
    }

    fn read_only() -> bool {
        false // Writes lock file
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        false // Fails if already locked
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build worktree lock options
        let mut opts = crate::WorktreeLockOpts::new(&args.worktree_path);
        if let Some(ref reason) = args.reason {
            opts = opts.reason(reason);
        }

        // Execute worktree lock
        crate::worktree_lock(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Terminal summary
        let summary = format!(
            "\x1b[33m Worktree Locked: {}\x1b[0m\n\
              Reason: {}",
            args.worktree_path,
            args.reason.as_deref().unwrap_or("none")
        );

        Ok(ToolResponse::new(summary, GitWorktreeLockOutput {
            success: true,
            worktree_path: args.worktree_path.clone(),
            reason: args.reason.clone(),
            message: "Worktree locked".to_string(),
        }))
    }
}
