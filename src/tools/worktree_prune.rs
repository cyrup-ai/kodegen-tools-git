//! Git worktree prune tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreePruneArgs, GitWorktreePrunePromptArgs, GitWorktreePruneOutput};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use std::path::Path;

/// Tool for pruning stale worktrees
#[derive(Clone)]
pub struct GitWorktreePruneTool;

impl Tool for GitWorktreePruneTool {
    type Args = GitWorktreePruneArgs;
    type PromptArgs = GitWorktreePrunePromptArgs;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
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
            "\x1b[31m󰍳 Worktrees Pruned\x1b[0m\n\
             󰋽 Removed: {} stale worktrees",
            pruned.len()
        );

        Ok(ToolResponse::new(summary, GitWorktreePruneOutput {
            success: true,
            pruned_count: pruned.len(),
            message: format!("Pruned {} stale worktree(s)", pruned.len()),
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "scenario_type".to_string(),
            title: None,
            description: Some(
                "Type of scenario to focus examples on (e.g., 'manual_deletion', 'cleanup', 'maintenance')"
                    .to_string(),
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use git_worktree_prune to clean up stale worktree administrative files?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_worktree_prune tool removes stale worktree administrative files from .git/worktrees/.\n\n\
                     **What it does:**\n\
                     - Scans the repository for linked worktrees\n\
                     - Identifies worktrees whose directories no longer exist\n\
                     - Removes their administrative entries from .git/worktrees/\n\
                     - Returns list of pruned worktree names\n\n\
                     **When to use it:**\n\
                     1. After manually deleting a worktree's directory (instead of using git worktree remove)\n\
                     2. When a worktree's directory is inaccessible or corrupted\n\
                     3. During repository maintenance to clean stale metadata\n\
                     4. When git worktree list shows entries that no longer exist\n\n\
                     **Basic usage:**\n\
                     git_worktree_prune({\"path\": \".\"})\n\
                     git_worktree_prune({\"path\": \"/path/to/repo\"})\n\n\
                     **Common workflow:**\n\
                     1. List current worktrees: git_worktree_list({\"path\": \".\"})\n\
                     2. Identify stale entries (missing directories)\n\
                     3. Run prune: git_worktree_prune({\"path\": \".\"})\n\
                     4. Verify cleanup: git_worktree_list({\"path\": \".\"})\n\n\
                     **Safety guarantees:**\n\
                     - Idempotent: Safe to call multiple times without side effects\n\
                     - Only removes admin files, not working directories\n\
                     - Only prunes entries where working directory doesn't exist\n\
                     - Best-effort: Continues if individual prune attempts fail\n\n\
                     **Key characteristics:**\n\
                     - Non-read-only: Modifies .git/worktrees/ directory\n\
                     - Destructive: Removes admin files (but non-critical cleanup)\n\
                     - Idempotent: Safe to call repeatedly\n\
                     - Repository path is the only required parameter\n\n\
                     **Related tools:**\n\
                     - git_worktree_add: Create new worktrees\n\
                     - git_worktree_remove: Properly remove worktrees (preferred method)\n\
                     - git_worktree_list: List all worktrees\n\
                     - git_worktree_lock: Prevent accidental removal",
                ),
            },
        ])
    }
}
