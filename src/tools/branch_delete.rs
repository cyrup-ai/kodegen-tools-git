//! Git branch deletion tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitBranchDeleteArgs, GitBranchDeletePromptArgs, GitBranchDeleteOutput};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
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
            "\x1b[31m󰆴 Branch Deleted: {}\x1b[0m\n\
             󰋽 Force: {}",
            args.branch,
            force_str
        );

        Ok(ToolResponse::new(summary, GitBranchDeleteOutput {
            success: true,
            branch: args.branch.clone(),
            message: format!("Deleted branch '{}'", args.branch),
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I delete a git branch safely?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_branch_delete tool removes branches from a git repository:\n\n\
                     Usage: git_branch_delete({\"path\": \"/path/to/repo\", \"branch\": \"feature-name\"})\n\n\
                     Basic examples:\n\
                     1. Delete a merged feature branch:\n\
                        git_branch_delete({\"path\": \".\", \"branch\": \"feature/login\"})\n\n\
                     2. Delete local branch after force-pushing:\n\
                        git_branch_delete({\"path\": \".\", \"branch\": \"experimental\", \"force\": true})\n\n\
                     3. Clean up old feature branches:\n\
                        git_branch_delete({\"path\": \"/projects/myapp\", \"branch\": \"old-feature\"})\n\n\
                     Key parameters:\n\
                     - path: Repository root directory (required)\n\
                     - branch: Name of branch to delete (required)\n\
                     - force: Force deletion even if not fully merged (optional, default: false)\n\n\
                     Safety features:\n\
                     - Cannot delete the currently checked-out branch (returns error)\n\
                     - Without force=true, only allows deletion of fully merged branches\n\
                     - With force=true, deletes branch regardless of merge status (use with caution)\n\n\
                     When to use:\n\
                     - After merging feature branches to main/develop\n\
                     - Cleaning up stale branches from local repository\n\
                     - Removing branches that have been force-pushed to remote\n\n\
                     When NOT to use:\n\
                     - Do not delete the currently checked-out branch (branch_create or branch_rename instead)\n\
                     - Do not use force=true lightly - verify the branch is truly no longer needed\n\
                     - For remote branches, use git push with --delete flag (via terminal tool)\n\n\
                     IMPORTANT: This operation is permanent! Once deleted, the branch ref is removed locally.\n\
                     If commits exist on the remote, the branch can still be recreated, but the local history is gone.",
                ),
            },
        ])
    }
}
