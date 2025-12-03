//! Git branch renaming tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitBranchRenameArgs, GitBranchRenamePromptArgs, GitBranchRenameOutput};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
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

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "scenario_type".to_string(),
            title: None,
            description: Some(
                "Type of scenario to focus on (e.g., 'feature-naming', 'cleanup', 'hotfix')"
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
                    "How do I use git_branch_rename to safely rename branches in a repository?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_branch_rename tool renames branches in a Git repository with automatic HEAD updates:\n\n\
                     1. Basic rename: git_branch_rename({\"path\": \"/repo\", \"old_name\": \"feature/old\", \"new_name\": \"feature/new\"})\n\
                     2. Force rename (overwrite): git_branch_rename({\"path\": \"/repo\", \"old_name\": \"feature/old\", \"new_name\": \"feature/new\", \"force\": true})\n\
                     3. Rename current branch: git_branch_rename({\"path\": \"/repo\", \"old_name\": \"current\", \"new_name\": \"updated\"})\n\n\
                     What this tool does:\n\
                     - Renames an existing Git branch to a new name\n\
                     - Automatically updates HEAD if you're renaming the currently checked-out branch\n\
                     - Returns JSON metadata with success status and operation details\n\n\
                     Common use cases:\n\
                     - Fix branch naming mistakes (typos in branch names)\n\
                     - Standardize naming conventions (feature/X → features/X)\n\
                     - Clean up temporary branches before deletion\n\
                     - Prepare branches for team collaboration with consistent naming\n\n\
                     Important parameters:\n\
                     - path: Must be the root directory of a valid Git repository\n\
                     - old_name: Must match an existing branch name exactly\n\
                     - new_name: Will fail if this branch already exists (unless force=true)\n\
                     - force: Set to true to overwrite if new_name already exists\n\n\
                     Tool behavior notes:\n\
                     - NOT idempotent: Renaming the same branch twice will fail\n\
                     - NOT destructive: No data is deleted, only renamed\n\
                     - MODIFIES state: Changes the repository's branch structure\n\
                     - AUTO-HEAD update: If renaming the current branch, HEAD is automatically updated\n\n\
                     Best practices:\n\
                     - Coordinate with teammates before renaming shared branches\n\
                     - Use consistent naming patterns (e.g., feature/, bugfix/, release/)\n\
                     - Verify branch exists with git_branch_list before renaming\n\
                     - Check if branch is published before renaming (inform team of changes)",
                ),
            },
        ])
    }
}
