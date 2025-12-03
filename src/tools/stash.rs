//! Git stash tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitStashArgs, GitStashPromptArgs, GitStashOutput};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use std::path::Path;

/// Tool for stashing changes
#[derive(Clone)]
pub struct GitStashTool;

impl Tool for GitStashTool {
    type Args = GitStashArgs;
    type PromptArgs = GitStashPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_STASH
    }

    fn description() -> &'static str {
        "Save uncommitted changes without committing. \
         Operations: 'save' to stash changes, 'pop' to apply and remove stash."
    }

    fn read_only() -> bool {
        false // Modifies working directory and creates refs
    }

    fn destructive() -> bool {
        false // Non-destructive (stash preserves changes)
    }

    fn idempotent() -> bool {
        false // Pop is not idempotent (consumes stash)
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {}", e)))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{}", e)))?;

        if args.operation.as_str() == "save" {
            // Save stash
            let opts = crate::StashOpts {
                message: args.message.clone(),
                include_untracked: args.include_untracked,
            };

            let stash_info = crate::stash_save(repo.clone(), opts)
                .await
                .map_err(|e| McpError::Other(anyhow::anyhow!("{}", e)))?;

            // Terminal summary
            let commit_short = &stash_info.commit_hash[..7.min(stash_info.commit_hash.len())];
            let summary = format!(
                "\x1b[36m 💾 Stash Saved\x1b[0m\n\
                 ℹ {}\n\
                 Commit: {}",
                stash_info.message,
                commit_short
            );

            Ok(ToolResponse::new(summary, GitStashOutput {
                success: true,
                operation: "save".to_string(),
                name: Some(stash_info.name),
                message: Some(stash_info.message),
                commit_hash: Some(stash_info.commit_hash),
            }))
        } else if args.operation.as_str() == "pop" {
            // Pop stash
            crate::stash_pop(repo, None)
                .await
                .map_err(|e| McpError::Other(anyhow::anyhow!("{}", e)))?;

            // Terminal summary
            let summary = "\x1b[32m ✓ Stash Popped\x1b[0m\n\
                 Changes restored to working directory".to_string();

            Ok(ToolResponse::new(summary, GitStashOutput {
                success: true,
                operation: "pop".to_string(),
                name: None,
                message: None,
                commit_hash: None,
            }))
        } else {
            Err(McpError::Other(anyhow::anyhow!(
                "Invalid stash operation: {}. Use 'save' or 'pop'",
                args.operation
            )))
        }
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "scenario".to_string(),
            title: None,
            description: Some(
                "Specific scenario to focus on (e.g., 'workflow', 'recovery', 'cleanup')".to_string()
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use git_stash to temporarily save and apply changes?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_stash tool temporarily saves uncommitted changes without committing them, \
                     allowing you to switch branches or clean your working directory. Here's how to use it:\n\n\
                     \
                     OPERATIONS\n\
                     \n\
                     1. Save changes to stash (default operation):\n\
                        git_stash({\"path\": \"/path/to/repo\", \"operation\": \"save\", \n\
                        \"message\": \"WIP: feature implementation\", \"include_untracked\": true})\n\n\
                     2. Apply stash and remove it from the stack:\n\
                        git_stash({\"path\": \"/path/to/repo\", \"operation\": \"pop\"})\n\n\
                     \
                     PARAMETERS\n\
                     \n\
                     - path: Required. Absolute path to the Git repository\n\
                     - operation: Either \"save\" (default) or \"pop\"\n\
                     - message: Optional. Descriptive text for save operations to identify the stash later\n\
                     - include_untracked: Boolean (default: true). When true, includes new untracked files \
                     in the stash\n\n\
                     \
                     KEY BEHAVIORS\n\
                     \n\
                     - Stash is a LIFO (Last-In-First-Out) stack: pop always retrieves the most recent stash\n\
                     - Pop is destructive on the stash but non-destructive on working directory: \
                     it removes the stash entry after applying\n\
                     - Untracked files can be selectively included/excluded via include_untracked parameter\n\
                     - Stashes are local to the repository and not pushed to remotes\n\
                     - Save returns commit hash and stash name for reference\n\
                     - Pop returns success status\n\n\
                     \
                     COMMON WORKFLOWS\n\
                     \n\
                     1. Switching branches without committing:\n\
                        - Save current changes: git_stash save operation\n\
                        - Switch branch: git_checkout with new branch\n\
                        - Return to original branch and restore: git_stash pop operation\n\n\
                     2. Cleaning working directory temporarily:\n\
                        - Save all changes: git_stash save with include_untracked: true\n\
                        - Run clean-directory operations\n\
                        - Restore when done: git_stash pop\n\n\
                     3. Separating concerns into different stashes:\n\
                        - Make changes for feature A\n\
                        - Save with message: git_stash save with message: \"Feature A work\"\n\
                        - Make different changes for feature B\n\
                        - Save with message: git_stash save with message: \"Feature B work\"\n\
                        - Pop to apply most recent, or cherry-pick specific stashes\n\n\
                     \
                     RESPONSE FORMAT\n\
                     \n\
                     Both operations return:\n\
                     - Human-readable summary with emoji indicators and status\n\
                     - JSON metadata containing success flag and operation-specific details:\n\
                       - Save: commit hash, stash name, message\n\
                       - Pop: success flag\n\n\
                     \
                     IMPORTANT GOTCHAS\n\
                     \n\
                     - Pop fails gracefully if stash is empty\n\
                     - Include_untracked should be true to capture new files (not just modifications)\n\
                     - Stashes don't prevent branch switching; use for temporary work\n\
                     - Message parameter is ignored in pop operations\n\
                     - Path must be a valid Git repository or operation fails",
                ),
            },
        ])
    }
}
