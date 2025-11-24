//! Git checkout tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitCheckoutArgs, GitCheckoutPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content, PromptMessageRole, PromptMessageContent};
use serde_json::json;
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
    type PromptArgs = GitCheckoutPromptArgs;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
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

        let mut contents = Vec::new();

        // Terminal summary with ANSI blue color and Nerd Font icons
        let summary = format!(
            "\x1b[34m\u{E725} Checkout: {}\x1b[0m\n\
             \u{E948} Type: {} · Create: {}",
            args.target, ref_type, create_str
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "target": args.target,
            "created": args.create,
            "paths": args.paths,
            "message": message
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "scenario".to_string(),
            title: None,
            description: Some(
                "Type of scenario to focus on (e.g., 'branch_switch', 'file_restore', 'commit_checkout', 'create_and_checkout')"
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
                    "How do I use git_checkout to switch branches, restore files, and create branches?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_checkout tool can switch to branches/tags/commits and restore files from references.\n\n\
                     1. Basic branch switch:\n\
                        git_checkout({\"path\": \".\", \"target\": \"main\"})\n\n\
                     2. Create and checkout branch in one operation:\n\
                        git_checkout({\"path\": \".\", \"target\": \"feature-x\", \"create\": true})\n\n\
                     3. Checkout specific commit by SHA (full or short):\n\
                        git_checkout({\"path\": \".\", \"target\": \"a1b2c3d\"})\n\n\
                     4. Checkout tag:\n\
                        git_checkout({\"path\": \".\", \"target\": \"v1.0.0\"})\n\n\
                     5. Restore specific files from another branch:\n\
                        git_checkout({\"path\": \".\", \"target\": \"main\", \"paths\": [\"config.json\", \"src/app.rs\"]})\n\n\
                     6. Force checkout (discard local changes):\n\
                        git_checkout({\"path\": \".\", \"target\": \"develop\", \"force\": true})\n\n\
                     Key parameters:\n\
                     - path: Repository directory (required)\n\
                     - target: Branch name, tag (v-prefixed), or commit hash (required)\n\
                     - paths: Optional file paths to restore (without paths, switches branches)\n\
                     - create: Automatically create the branch if it doesn't exist\n\
                     - force: Discard local changes and untracked files\n\n\
                     Reference detection (automatic):\n\
                     - Commits: 7-40 hex characters (e.g., 'a1b2c3d' or full SHA)\n\
                     - Tags: Start with 'v' followed by digit (e.g., 'v1.0.0', 'v2.1.3')\n\
                     - Branches: Everything else (e.g., 'main', 'feature/new-ui')\n\n\
                     File restoration workflow:\n\
                     - Without paths: Switches entire branch/commit\n\
                     - With paths: Restores only specified files from target reference\n\
                     - Useful for cherry-picking files from other branches without full checkout\n\n\
                     Important safety notes:\n\
                     - Checkout fails if local changes would be overwritten (unless force=true)\n\
                     - force flag discards ALL uncommitted changes - use with caution\n\
                     - create flag only works with branch names, not with commits or tags\n\
                     - File restoration with paths does not change current branch\n\
                     - Short commit SHAs must be 7+ characters to avoid ambiguity",
                ),
            },
        ])
    }
}
