//! Git worktree list tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreeListArgs, GitWorktreeListPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content, PromptMessageRole, PromptMessageContent};
use serde_json::json;
use std::path::Path;

/// Tool for listing worktrees
#[derive(Clone)]
pub struct GitWorktreeListTool;

impl Tool for GitWorktreeListTool {
    type Args = GitWorktreeListArgs;
    type PromptArgs = GitWorktreeListPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_WORKTREE_LIST
    }

    fn description() -> &'static str {
        "List all worktrees in the repository with detailed status. \
         Returns main worktree and all linked worktrees with their paths, branches, \
         lock status, and HEAD information."
    }

    fn read_only() -> bool {
        true // Only lists
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true // Safe to call repeatedly
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Execute worktree list
        let worktrees = crate::list_worktrees(repo)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let worktrees_json: Vec<_> = worktrees
            .iter()
            .map(|wt| {
                json!({
                    "path": wt.path.display().to_string(),
                    "git_dir": wt.git_dir.display().to_string(),
                    "is_main": wt.is_main,
                    "is_bare": wt.is_bare,
                    "head_commit": wt.head_commit.map(|id| id.to_string()),
                    "head_branch": wt.head_branch.clone(),
                    "is_locked": wt.is_locked,
                    "lock_reason": wt.lock_reason.clone(),
                    "is_detached": wt.is_detached
                })
            })
            .collect();

        let mut contents = Vec::new();

        // Terminal summary with ANSI color codes and Nerd Font icons
        let count = worktrees.len();
        let main_path = worktrees.iter()
            .find(|wt| wt.is_main)
            .map(|wt| wt.path.display().to_string())
            .unwrap_or_else(|| "none".to_string());

        let summary = format!(
            "\x1b[36m Worktrees\x1b[0m\n\
              Total: {} · Main: {}",
            count, main_path
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "worktrees": worktrees_json,
            "count": worktrees.len()
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "focus_area".to_string(),
            title: None,
            description: Some(
                "Optional focus area for teaching: 'basic_usage', 'output_fields', 'use_cases', or 'all'"
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
                    "How do I list Git worktrees and understand the output?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_worktree_list tool shows all worktrees in a repository:\n\n\
                     Basic usage:\n\
                     git_worktree_list({\"path\": \".\"}) or git_worktree_list({\"path\": \"/path/to/repo\"})\n\n\
                     Output includes these fields for each worktree:\n\
                     - path: Filesystem path to the worktree\n\
                     - git_dir: Path to .git directory for this worktree\n\
                     - is_main: true if this is the main repository (not a linked worktree)\n\
                     - is_bare: true if this is a bare repository\n\
                     - head_branch: Name of the checked-out branch (e.g., \"main\", \"feature-x\")\n\
                     - head_commit: SHA of the current commit\n\
                     - is_locked: true if worktree is locked (prevents removal)\n\
                     - lock_reason: Optional reason why worktree is locked\n\
                     - is_detached: true if HEAD is detached (not on a branch)\n\n\
                     Common use cases:\n\
                     1. See all active work branches: Check all worktrees and their branches\n\
                     2. Find worktree locations: Get filesystem paths for navigation\n\
                     3. Identify locked worktrees: See which worktrees are protected from removal\n\
                     4. Detect detached HEAD states: Find worktrees not on a branch\n\n\
                     Integration with other tools:\n\
                     - Use before git_worktree_remove to see what can be removed\n\
                     - Use with git_worktree_lock/unlock to manage worktree protection\n\
                     - Combine with git_worktree_prune to clean up deleted worktrees\n\n\
                     Best practices:\n\
                     - Run from main repository or any worktree (all show same results)\n\
                     - Check is_locked before attempting removal\n\
                     - Monitor detached worktrees (may indicate forgotten checkouts)",
                ),
            },
        ])
    }
}
