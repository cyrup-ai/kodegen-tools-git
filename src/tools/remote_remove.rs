//! Git remote remove tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitRemoteRemoveArgs, GitRemoteRemovePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole, Content};
use serde_json::json;
use std::path::Path;

/// Tool for removing remote repositories
#[derive(Clone)]
pub struct GitRemoteRemoveTool;

impl Tool for GitRemoteRemoveTool {
    type Args = GitRemoteRemoveArgs;
    type PromptArgs = GitRemoteRemovePromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_REMOTE_REMOVE
    }

    fn description() -> &'static str {
        "Remove a configured remote repository. \
         Deletes the remote from repository configuration."
    }

    fn read_only() -> bool {
        false // Modifies repository configuration
    }

    fn destructive() -> bool {
        true // Removes configuration entries
    }

    fn idempotent() -> bool {
        false // Cannot remove non-existent remote
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Execute remove
        crate::remove_remote(repo, &args.name)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "\x1b[32m ✓ Remote Removed\x1b[0m\n\
             {} deleted from configuration",
            args.name
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "remote_name": args.name
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
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I remove a Git remote?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_remote_remove tool deletes a configured remote from your Git \
                     repository. This is useful for:\n\n\
                     1. **Fork workflows**: After merging PR branches and finishing contribution, \
                     remove the upstream or fork remote\n\n\
                     2. **Cleanup**: Remove stale, incorrect, or abandoned remote references\n\n\
                     3. **Migration**: Reorganizing remotes when moving repositories or changing \
                     Git hosting\n\n\
                     4. **Multiple remotes**: Cleaning up when you have too many remotes configured\n\n\
                     USAGE: git_remote_remove({\"path\": \"/path/to/repo\", \"name\": \"origin\"})\n\n\
                     PARAMETERS:\n\
                     - path: Path to the Git repository (required)\n\
                     - name: Name of the remote to remove, e.g., \"origin\", \"upstream\", \
                     \"fork\" (required)\n\n\
                     COMMON SCENARIOS:\n\n\
                     **Scenario 1: Remove upstream after fork contribution**\n\
                     Workflow: Clone fork -> add upstream -> fetch -> merge -> remove upstream\n\
                     git_remote_remove({\"path\": \".\", \"name\": \"upstream\"})\n\n\
                     **Scenario 2: Remove fork origin and switch to main repo**\n\
                     After contributing to a project, use the main repository:\n\
                     git_remote_remove({\"path\": \".\", \"name\": \"origin\"})\n\
                     Then add the main repo as origin\n\n\
                     **Scenario 3: Cleanup multiple test remotes**\n\
                     First, list remotes: git_remote_list({\"path\": \".\"})\n\
                     Then remove unwanted ones:\n\
                     git_remote_remove({\"path\": \".\", \"name\": \"test-remote\"})\n\n\
                     IMPORTANT WARNINGS:\n\n\
                     ⚠️ DESTRUCTIVE: This operation deletes the remote configuration entry. \
                     You cannot undo this without manually re-adding the remote.\n\n\
                     ⚠️ NON-IDEMPOTENT: Attempting to remove a remote that doesn't exist will \
                     fail. Always verify the remote exists before removing.\n\n\
                     ⚠️ CONNECTIVITY: Removing a remote doesn't affect branches that were created \
                     from that remote. Those branches remain in your repository.\n\n\
                     BEST PRACTICES:\n\n\
                     1. **Always verify first**: Use git_remote_list to see all configured \
                     remotes before removing\n\
                     2. **Understand the context**: Know why the remote was added and what it's \
                     used for\n\
                     3. **Document locally**: If removing a shared remote, ensure team knows about \
                     the change\n\
                     4. **Consider force sync**: After removing, use git_fetch to update your \
                     repository state\n\n\
                     ERROR CASES:\n\
                     - Remote not found: \"error: Could not remove config section 'remote.xxx'\" \
                     → Check spelling with git_remote_list\n\
                     - Permission denied: Check directory permissions on .git/config\n\
                     - Not in repo: Run from repository root or provide correct path\n\n\
                     RECOVERY:\n\
                     If you accidentally remove a remote, re-add it with the original URL \
                     using git_remote_add.",
                ),
            },
        ])
    }
}
