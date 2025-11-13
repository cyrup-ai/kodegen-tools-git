//! Git worktree prune tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreePruneArgs, GitWorktreePrunePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for pruning stale worktrees
#[derive(Clone)]
pub struct GitWorktreePruneTool;

impl Tool for GitWorktreePruneTool {
    type Args = GitWorktreePruneArgs;
    type PromptArgs = GitWorktreePrunePromptArgs;

    fn name() -> &'static str {
        "git_worktree_prune"
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

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
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

        let mut contents = Vec::new();

        // Terminal summary
        let summary = if pruned.is_empty() {
            "✓ No stale worktrees to prune".to_string()
        } else {
            let pruned_list = pruned.iter()
                .map(|name| format!("  • {}", name))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "✓ Pruned {} stale worktree(s)\n\n{}",
                pruned.len(),
                pruned_list
            )
        };
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "pruned": pruned,
            "count": pruned.len(),
            "message": format!("Pruned {} stale worktree(s)", pruned.len())
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
