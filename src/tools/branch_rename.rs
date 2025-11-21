//! Git branch renaming tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitBranchRenameArgs, GitBranchRenamePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for renaming Git branches
#[derive(Clone)]
pub struct GitBranchRenameTool;

impl Tool for GitBranchRenameTool {
    type Args = GitBranchRenameArgs;
    type PromptArgs = GitBranchRenamePromptArgs;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
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

        let mut contents = Vec::new();

        // Terminal summary
        let force_text = if args.force { "yes" } else { "no" };
        let summary = format!(
            "\x1b[33m Branch Renamed: {} → {}\x1b[0m\n\
              Force: {}",
            args.old_name, args.new_name, force_text
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "old_name": args.old_name,
            "new_name": args.new_name,
            "message": format!("Renamed branch '{}' to '{}'", args.old_name, args.new_name)
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
