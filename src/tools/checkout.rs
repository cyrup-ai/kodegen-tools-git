//! Git checkout tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitCheckoutArgs, GitCheckoutOutput, GitCheckoutPrompts};
use std::path::Path;

/// Detect reference type from target string
///
/// Uses heuristics to determine if the target is a commit, tag, or branch:
/// - Commit: 40 hex characters (full SHA) or 7-39 hex characters (short SHA)
/// - Tag: Starts with 'v' followed by digits (version pattern like v1.0.0)
/// - Branch: Everything else (default)
fn detect_ref_type(target: &str) -> &'static str {
    // Check if it's a commit hash (7-40 hex characters)
    if target.len() >= 7 && target.len() <= 40
        && target.chars().all(|c| c.is_ascii_hexdigit())
    {
        return "commit";
    }

    // Check if it looks like a version tag (starts with 'v' followed by digit)
    if target.starts_with('v') && target.len() > 1
        && let Some(c) = target.chars().nth(1)
        && c.is_ascii_digit()
    {
        return "tag";
    }

    // Default to branch
    "branch"
}

/// Tool for checking out Git references
#[derive(Clone)]
pub struct GitCheckoutTool;

impl Tool for GitCheckoutTool {
    type Args = GitCheckoutArgs;
    type Prompts = GitCheckoutPrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_CHECKOUT
    }

    fn description() -> &'static str {
        "Checkout a Git reference (branch, tag, or commit) or restore specific files. \
         Without paths: switches branches/commits. With paths: restores files from the reference."
    }

    fn read_only() -> bool {
        false // Modifies working directory
    }

    fn destructive() -> bool {
        true // Can discard local changes with force
    }

    fn idempotent() -> bool {
        true // Checking out same ref multiple times is safe
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // If create flag is set, create the branch first
        if args.create {
            let branch_opts = crate::BranchOpts {
                name: args.target.clone(),
                start_point: None, // Use HEAD
                force: false,
                checkout: false, // We'll checkout separately
                track: false,
            };

            crate::branch(repo.clone(), branch_opts)
                .await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
                .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;
        }

        // Build checkout options
        let mut opts = crate::CheckoutOpts::new(&args.target);
        opts = opts.force(args.force);

        // Add file paths if specified
        if let Some(ref file_paths) = args.paths {
            opts = opts.paths(file_paths.iter().map(std::string::String::as_str));
        }

        // Execute checkout
        crate::checkout(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let message = if args.create {
            format!("Created and checked out branch '{}'", args.target)
        } else if let Some(ref paths) = args.paths {
            format!("Restored {} file(s) from '{}'", paths.len(), args.target)
        } else {
            format!("Checked out '{}'", args.target)
        };

        // Detect reference type
        let ref_type = if args.create {
            "branch"
        } else {
            detect_ref_type(&args.target)
        };

        let create_str = if args.create { "yes" } else { "no" };

        // Terminal summary with ANSI blue color and Nerd Font icons
        let summary = format!(
            "\x1b[34m\u{E725} Checkout: {}\x1b[0m\n\
             \u{E948} Type: {} · Create: {}",
            args.target, ref_type, create_str
        );

        Ok(ToolResponse::new(summary, GitCheckoutOutput {
            success: true,
            target: args.target.clone(),
            created: args.create,
            paths: args.paths.clone(),
            message,
        }))
    }
}
