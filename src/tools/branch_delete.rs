//! Git branch deletion tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitBranchDeleteArgs, GitBranchDeleteOutput, BranchDeletePrompts};
use std::path::Path;

/// Tool for deleting Git branches
#[derive(Clone)]
pub struct GitBranchDeleteTool;

impl Tool for GitBranchDeleteTool {
    type Args = GitBranchDeleteArgs;
    type Prompts = BranchDeletePrompts;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
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

        // Terminal summary with colored output and icons
        let force_str = if args.force { "yes" } else { "no" };
        let summary = format!(
            "\x1b[31mBranch Deleted: {}\x1b[0m\n\
             Force: {}",
            args.branch,
            force_str
        );

        Ok(ToolResponse::new(summary, GitBranchDeleteOutput {
            success: true,
            branch: args.branch.clone(),
            message: format!("Deleted branch '{}'", args.branch),
        }))
    }
}
