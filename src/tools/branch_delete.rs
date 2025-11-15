//! Git branch deletion tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitBranchDeleteArgs, GitBranchDeletePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for deleting Git branches
#[derive(Clone)]
pub struct GitBranchDeleteTool;

impl Tool for GitBranchDeleteTool {
    type Args = GitBranchDeleteArgs;
    type PromptArgs = GitBranchDeletePromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_BRANCH_DELETE
    }

    fn description() -> &'static str {
        "Delete a branch from a Git repository. \
         Cannot delete the currently checked-out branch."
    }

    fn read_only() -> bool {
        false // Modifies repository
    }

    fn destructive() -> bool {
        true // Deletes branches
    }

    fn idempotent() -> bool {
        false // Will fail if branch doesn't exist
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Delete branch
        crate::delete_branch(repo, args.branch.clone(), args.force)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let mut contents = Vec::new();

        // Terminal summary
        let summary = format!(
            "✓ Branch deleted\n\n\
             Branch: {}{}",
            args.branch,
            if args.force { " (forced)" } else { "" }
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "branch": args.branch,
            "message": format!("Deleted branch '{}'", args.branch)
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
