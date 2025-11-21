//! Git worktree unlock tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreeUnlockArgs, GitWorktreeUnlockPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
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
        Ok(vec![])
    }
}
