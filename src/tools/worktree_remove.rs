//! Git worktree remove tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreeRemoveArgs, GitWorktreeRemovePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
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

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
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

        let mut contents = Vec::new();

        // Terminal summary
        let force_display = if args.force { "yes" } else { "no" };
        let summary = format!(
            "\x1b[31m Worktree Removed: {}\x1b[0m\n\
              Force: {}",
            args.worktree_path,
            force_display
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "worktree_path": args.worktree_path,
            "message": "Worktree removed"
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
