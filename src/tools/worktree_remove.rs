//! Git worktree remove tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreeRemoveArgs, GitWorktreeRemovePromptArgs, GitWorktreeRemoveOutput};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use std::path::Path;

/// Tool for removing worktrees
#[derive(Clone)]
pub struct GitWorktreeRemoveTool;

impl Tool for GitWorktreeRemoveTool {
    type Args = GitWorktreeRemoveArgs;
    type PromptArgs = GitWorktreeRemovePromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_WORKTREE_REMOVE
    }

    fn description() -> &'static str {
        "Remove a worktree and its associated administrative files. \
         Cannot remove locked worktrees without force flag."
    }

    fn read_only() -> bool {
        false // Deletes files
    }

    fn destructive() -> bool {
        true // Removes worktree and files
    }

    fn idempotent() -> bool {
        false // Fails if worktree doesn't exist
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build worktree remove options
        let opts = crate::WorktreeRemoveOpts::new(&args.worktree_path).force(args.force);

        // Execute worktree remove
        crate::worktree_remove(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Terminal summary
        let force_display = if args.force { "yes" } else { "no" };
        let summary = format!(
            "\x1b[31m Worktree Removed: {}\x1b[0m\n\
              Force: {}",
            args.worktree_path,
            force_display
        );

        Ok(ToolResponse::new(summary, GitWorktreeRemoveOutput {
            success: true,
            worktree_path: args.worktree_path.clone(),
            message: "Worktree removed".to_string(),
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I remove a Git worktree and clean up its files?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_worktree_remove tool safely removes a worktree and its associated \
                     administrative files from a Git repository. Here's how to use it:\n\n\
                     Basic usage:\n\
                     git_worktree_remove({\"path\": \"/path/to/repo\", \"worktree_path\": \"../my-feature-worktree\"})\n\n\
                     This returns:\n\
                     1. A human-readable summary showing the removed worktree path and force flag status\n\
                     2. A JSON response with:\n\
                        - success: boolean indicating operation success\n\
                        - worktree_path: the path of the removed worktree\n\
                        - message: confirmation message\n\n\
                     Key parameters:\n\
                     - path: Path to the Git repository (any subdirectory works)\n\
                     - worktree_path: Path to the worktree to remove (can be relative or absolute)\n\
                     - force: Set to true to remove locked worktrees (default: false)\n\n\
                     Important behaviors:\n\
                     - By default, cannot remove locked worktrees (must use force flag)\n\
                     - Locked worktrees are protected to prevent accidental deletion during active use\n\
                     - Force removal bypasses locking restrictions but should be used carefully\n\
                     - Removes the worktree directory AND git administrative files\n\
                     - Fails if worktree path does not exist\n\n\
                     Common use cases:\n\
                     - Cleanup: Remove completed feature worktrees after merging to main\n\
                     - Maintenance: Clean up stale worktrees from failed operations\n\
                     - Recovery: Use force flag to remove worktrees when lock files are corrupted\n\
                     - CI/CD: Automate worktree cleanup in build pipelines\n\n\
                     Examples:\n\
                     1. Remove a completed feature worktree:\n\
                        git_worktree_remove({\"path\": \".\", \"worktree_path\": \"../my-feature\", \"force\": false})\n\n\
                     2. Force remove a stuck/locked worktree:\n\
                        git_worktree_remove({\"path\": \".\", \"worktree_path\": \"../buggy-worktree\", \"force\": true})\n\n\
                     Best practices:\n\
                     - Always verify the worktree is no longer needed before removal\n\
                     - Remove worktrees in the reverse order of creation for cleaner history\n\
                     - Use force flag only when necessary (locked worktrees indicate active use)\n\
                     - Consider checking worktree status before removal with git_worktree_list",
                ),
            },
        ])
    }
}
