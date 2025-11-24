//! Git worktree unlock tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreeUnlockArgs, GitWorktreeUnlockPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole, Content};
use serde_json::json;
use std::path::{Path, PathBuf};

/// Tool for unlocking worktrees
#[derive(Clone)]
pub struct GitWorktreeUnlockTool;

impl Tool for GitWorktreeUnlockTool {
    type Args = GitWorktreeUnlockArgs;
    type PromptArgs = GitWorktreeUnlockPromptArgs;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
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

        let mut contents = Vec::new();

        // Terminal summary
        let summary = format!(
            "\x1b[32m Worktree Unlocked: {}\x1b[0m\n\
              Status: unlocked",
            args.worktree_path
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "worktree_path": args.worktree_path,
            "message": "Worktree unlocked"
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
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What is a locked worktree and when do I need to unlock one?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Git worktrees are independent working directories linked to the same repository. \
                     A worktree can be locked to prevent accidental deletion, useful for worktrees on:\n\n\
                     • Removable media (USB drives, external hard drives)\n\
                     • Network drives (NFS, SMB shares)\n\
                     • Temporary or experimental setups\n\n\
                     When a worktree is locked, `git worktree remove` will fail unless you unlock it first. \
                     Use this tool to remove the lock.",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I unlock a worktree?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use git_worktree_unlock with the repository path and worktree path:\n\n\
                     {\"path\": \"/home/user/my-repo\", \"worktree_path\": \"/home/user/my-repo/wt-feature\"}\n\n\
                     This removes the administrative lock file, allowing the worktree to be deleted with `git worktree remove`. \
                     The unlock operation fails if the worktree is not currently locked.",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What's the relationship between worktree_lock and worktree_unlock?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "They are complementary operations:\n\n\
                     • git_worktree_lock: Creates a lock file to prevent deletion. \
                     Optionally accepts a reason (e.g., \"On portable storage\").\n\n\
                     • git_worktree_unlock: Removes the lock file, allowing deletion again.\n\n\
                     Neither operation is idempotent - lock fails if already locked, unlock fails if not locked. \
                     Use git_worktree_list to check current worktree status including lock information.",
                ),
            },
        ])
    }
}
