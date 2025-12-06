//! Git worktree prune tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitWorktreePruneArgs, GitWorktreePruneOutput, WorktreePrunePrompts};
use std::path::Path;

/// Tool for pruning stale worktrees
#[derive(Clone)]
pub struct GitWorktreePruneTool;

impl Tool for GitWorktreePruneTool {
    type Args = GitWorktreePruneArgs;
    type Prompts = WorktreePrunePrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_WORKTREE_PRUNE
    }

    fn description() -> &'static str {
        "Remove stale worktree administrative files. \
         Cleans up .git/worktrees/ entries for worktrees whose directories have been manually deleted. \
         Returns list of pruned worktree names."
    }

    fn read_only() -> bool {
        false // Removes stale admin files
    }

    fn destructive() -> bool {
        true // Deletes worktree admin
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

        // Execute worktree prune
        let pruned = crate::worktree_prune(repo)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Terminal summary
        let summary = format!(
            "\x1b[31m Worktrees Pruned\x1b[0m\n\
              Removed: {} stale worktrees",
            pruned.len()
        );

        Ok(ToolResponse::new(summary, GitWorktreePruneOutput {
            success: true,
            pruned_count: pruned.len(),
            message: format!("Pruned {} stale worktree(s)", pruned.len()),
        }))
    }
}
