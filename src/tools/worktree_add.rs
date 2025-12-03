//! Git worktree add tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreeAddArgs, GitWorktreeAddPromptArgs, GitWorktreeAddOutput};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use std::path::Path;

/// Tool for adding worktrees
#[derive(Clone)]
pub struct GitWorktreeAddTool;

impl Tool for GitWorktreeAddTool {
    type Args = GitWorktreeAddArgs;
    type PromptArgs = GitWorktreeAddPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_WORKTREE_ADD
    }

    fn description() -> &'static str {
        "Create a new worktree linked to the repository. \
         Allows working on multiple branches simultaneously."
    }

    fn read_only() -> bool {
        false // Creates worktree
    }

    fn destructive() -> bool {
        false // Creates new files
    }

    fn idempotent() -> bool {
        false // Fails if worktree exists
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build worktree add options
        let mut opts = crate::WorktreeAddOpts::new(&args.worktree_path);
        if let Some(ref branch) = args.branch {
            opts = opts.committish(branch);
        }
        opts = opts.force(args.force);

        // Execute worktree add
        let created_path = crate::worktree_add(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Terminal summary with ANSI colors and icons
        let branch_display = args.branch.as_deref().unwrap_or("(detached)");
        let checkout_ref = args.branch.as_deref().unwrap_or("HEAD");

        let summary = format!(
            "\x1b[32m Worktree Added: {}\x1b[0m\n\
              Branch: {} · Checkout: {}",
            created_path.display(),
            branch_display,
            checkout_ref
        );

        Ok(ToolResponse::new(summary, GitWorktreeAddOutput {
            success: true,
            worktree_path: created_path.display().to_string(),
            branch: args.branch.clone(),
            message: format!("Worktree created at {}", created_path.display()),
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "use_case".to_string(),
            title: None,
            description: Some(
                "Type of scenario to focus on: 'basic' (simple add), 'branch_checkout' (adding with branch), \
                 or 'concurrent' (parallel development workflow)".to_string(),
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use git_worktree_add to work on multiple branches simultaneously?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Git worktrees allow you to work on multiple branches in parallel without switching \
                    your main working directory. Each worktree is an independent working directory linked to \
                    the same repository.\n\n\
                    Basic usage patterns:\n\n\
                    1. Create a simple worktree for a new feature:\n\
                       git_worktree_add({\"path\": \".\", \"worktree_path\": \"../feature-xyz\"})\n\n\
                    2. Create a worktree and immediately checkout a specific branch:\n\
                       git_worktree_add({\"path\": \".\", \"worktree_path\": \"../bugfix-123\", \"branch\": \"bugfix/issue-123\"})\n\n\
                    3. Create a worktree tracking a remote branch:\n\
                       git_worktree_add({\"path\": \".\", \"worktree_path\": \"../upstream-sync\", \"branch\": \"origin/main\"})\n\n\
                    Key parameters explained:\n\
                    - path: Root directory of your git repository (usually \".\" for current repo)\n\
                    - worktree_path: Location where the new worktree will be created (directory must not exist)\n\
                    - branch: Optional branch/tag/commit to checkout (defaults to HEAD if omitted)\n\
                    - force: Should be false (default) - only use force=true to recover from corrupted worktrees\n\n\
                    Typical workflow for parallel development:\n\
                    1. Create main worktree for ongoing work: git_worktree_add({\"path\": \".\", \"worktree_path\": \"../main-work\"})\n\
                    2. Create feature worktree: git_worktree_add({\"path\": \".\", \"worktree_path\": \"../feature-work\", \"branch\": \"feature/new-feature\"})\n\
                    3. Work independently in each directory\n\
                    4. Use git_worktree_list to see all worktrees\n\
                    5. Use git_worktree_remove when finished with a worktree\n\n\
                    Important considerations:\n\
                    - The worktree_path directory must not already exist (fails safely with error)\n\
                    - If a path exists and you need to override, set force=true (handles cleanup)\n\
                    - Each worktree has its own working directory but shares the .git database\n\
                    - Consider using git_worktree_lock to prevent accidental deletion\n\
                    - Worktrees improve productivity for projects with long-lived feature branches\n\
                    - Related tools: git_worktree_list, git_worktree_remove, git_worktree_lock, git_worktree_unlock, git_worktree_prune",
                ),
            },
        ])
    }
}
