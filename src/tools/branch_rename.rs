//! Git branch renaming tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitBranchRenameArgs, GitBranchRenameOutput, BranchRenamePrompts};
use std::path::Path;

/// Tool for renaming Git branches
#[derive(Clone)]
pub struct GitBranchRenameTool;

impl Tool for GitBranchRenameTool {
    type Args = GitBranchRenameArgs;
    type Prompts = BranchRenamePrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_BRANCH_RENAME
    }

    fn description() -> &'static str {
        "Rename a branch in a Git repository. \
         Automatically updates HEAD if renaming the current branch."
    }

    fn read_only() -> bool {
        false // Modifies repository
    }

    fn destructive() -> bool {
        false // Renames, doesn't delete
    }

    fn idempotent() -> bool {
        false // Will fail if already renamed
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Rename branch
        crate::rename_branch(
            repo,
            args.old_name.clone(),
            args.new_name.clone(),
            args.force,
        )
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Terminal summary
        let force_text = if args.force { "yes" } else { "no" };
        let summary = format!(
            "\x1b[33m Branch Renamed: {} → {}\x1b[0m\n\
              Force: {}",
            args.old_name, args.new_name, force_text
        );

        Ok(ToolResponse::new(summary, GitBranchRenameOutput {
            success: true,
            old_name: args.old_name.clone(),
            new_name: args.new_name.clone(),
            message: format!("Renamed branch '{}' to '{}'", args.old_name, args.new_name),
        }))
    }
}
