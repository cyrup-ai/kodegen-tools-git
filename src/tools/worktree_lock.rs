//! Git worktree lock tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreeLockArgs, GitWorktreeLockPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content, PromptMessageRole, PromptMessageContent};
use serde_json::json;
use std::path::Path;

/// Tool for locking worktrees
#[derive(Clone)]
pub struct GitWorktreeLockTool;

impl Tool for GitWorktreeLockTool {
    type Args = GitWorktreeLockArgs;
    type PromptArgs = GitWorktreeLockPromptArgs;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
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

        let mut contents = Vec::new();

        // Terminal summary
        let summary = format!(
            "\x1b[33m Worktree Locked: {}\x1b[0m\n\
              Reason: {}",
            args.worktree_path,
            args.reason.as_deref().unwrap_or("none")
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "worktree_path": args.worktree_path,
            "reason": args.reason,
            "message": "Worktree locked"
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "focus".to_string(),
            title: None,
            description: Some(
                "Optional focus area: 'basic' for simple locking, 'advanced' for edge cases and removable media"
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
                    "What is git worktree locking and when should I use it?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Git worktree locking prevents a worktree from being deleted, which is useful in several scenarios:\n\n\
                     COMMON USE CASES:\n\
                     1. Removable media: Lock worktrees on USB drives or external disks to prevent accidental deletion\n\
                     2. Network drives: Lock worktrees on network shares that might be unmounted unexpectedly\n\
                     3. Temporary work: Lock a worktree while you're actively using it to prevent cleanup scripts from removing it\n\
                     4. Shared repositories: Lock worktrees in shared repos where cleanup might happen automatically\n\n\
                     BASIC USAGE:\n\
                     git_worktree_lock({\n\
                       \"path\": \"/path/to/repo\",\n\
                       \"worktree_path\": \"/path/to/worktree\"\n\
                     })\n\n\
                     WITH DOCUMENTATION:\n\
                     git_worktree_lock({\n\
                       \"path\": \"/path/to/repo\",\n\
                       \"worktree_path\": \"/mnt/usb/my-feature-branch\",\n\
                       \"reason\": \"On removable USB drive\"\n\
                     })\n\n\
                     KEY POINTS:\n\
                     - Lock reason is optional but recommended for clarity in shared environments\n\
                     - Locked worktrees persist across git commands until explicitly unlocked\n\
                     - Locks prevent 'git worktree prune' and 'git worktree remove' from deleting the directory\n\
                     - The tool fails (not idempotent) if the worktree is already locked\n\
                     - Lock information is stored in .git/worktrees/<name>/locked file\n\n\
                     COMMON GOTCHAS:\n\
                     - Cannot re-lock an already-locked worktree (must unlock first)\n\
                     - Reason string is just documentation - it doesn't enforce behavior\n\
                     - Locking only affects worktree deletion; the worktree remains fully functional\n\
                     - Must use git_worktree_unlock to remove the lock before deletion",
                ),
            },
        ])
    }
}
