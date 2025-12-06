//! Git worktree add tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitWorktreeAddArgs, GitWorktreeAddOutput, WorktreeAddPrompts};
use std::path::Path;

/// Tool for adding worktrees
#[derive(Clone)]
pub struct GitWorktreeAddTool;

impl Tool for GitWorktreeAddTool {
    type Args = GitWorktreeAddArgs;
    type Prompts = WorktreeAddPrompts;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
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
}
