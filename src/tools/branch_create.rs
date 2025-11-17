//! Git branch creation tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitBranchCreateArgs, GitBranchCreatePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for creating Git branches
#[derive(Clone)]
pub struct GitBranchCreateTool;

impl Tool for GitBranchCreateTool {
    type Args = GitBranchCreateArgs;
    type PromptArgs = GitBranchCreatePromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_BRANCH_CREATE
    }

    fn description() -> &'static str {
        "Create a new branch in a Git repository. \
         Optionally specify a starting point and checkout the branch after creation."
    }

    fn read_only() -> bool {
        false // Creates branches
    }

    fn destructive() -> bool {
        false // Only creates, doesn't delete
    }

    fn idempotent() -> bool {
        false // Will fail if branch exists without force
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build branch options
        let opts = crate::BranchOpts {
            name: args.branch.clone(),
            start_point: args.from_branch.clone(),
            force: args.force,
            checkout: args.checkout,
            track: false,
        };

        // Create branch
        crate::branch(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "󰊢 \x1b[32mBranch Created: {}\x1b[0m\n\
              From: {} · Checkout: {}",
            args.branch,
            args.from_branch.as_deref().unwrap_or("HEAD"),
            if args.checkout { "yes" } else { "no" }
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "branch": args.branch,
            "message": format!("Created branch '{}'", args.branch)
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
