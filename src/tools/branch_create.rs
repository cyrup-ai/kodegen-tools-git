//! Git branch creation tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitBranchCreateArgs, GitBranchCreatePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole, Content};
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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
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
        vec![PromptArgument {
            name: "focus_area".to_string(),
            title: None,
            description: Some(
                "Optional focus area for examples: 'basic' (simple workflows), 'advanced' (complex scenarios), \
                 or 'best-practices' (naming conventions and Git workflow patterns)".to_string(),
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I create a new branch and switch to it?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "To create a new branch and immediately switch to it, use the `checkout` parameter. \
                    This combines branch creation and checkout into a single operation:\n\n\
                    Example:\n\
                    {\n  \
                      \"path\": \"/path/to/repo\",\n  \
                      \"branch\": \"feature-new-login\",\n  \
                      \"checkout\": true\n\
                    }\n\n\
                    This creates 'feature-new-login' from your current HEAD and switches to it. \
                    If you only want to create the branch without switching, set `checkout` to false."
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I create a branch from a specific commit or another branch?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the `from_branch` parameter to specify a starting point. This can be a branch name, tag, or commit hash:\n\n\
                    Creating from another branch:\n\
                    {\n  \
                      \"path\": \"/path/to/repo\",\n  \
                      \"branch\": \"hotfix-critical-bug\",\n  \
                      \"from_branch\": \"main\",\n  \
                      \"checkout\": true\n\
                    }\n\n\
                    Creating from a specific commit:\n\
                    {\n  \
                      \"path\": \"/path/to/repo\",\n  \
                      \"branch\": \"recovery-branch\",\n  \
                      \"from_branch\": \"a1b2c3d4\",\n  \
                      \"checkout\": false\n\
                    }\n\n\
                    If you omit `from_branch`, the branch is created from HEAD (your current position)."
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("What if the branch name already exists?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "By default, attempting to create an existing branch will fail. Use the `force` parameter to overwrite:\n\n\
                    {\n  \
                      \"path\": \"/path/to/repo\",\n  \
                      \"branch\": \"feature-rework\",\n  \
                      \"force\": true,\n  \
                      \"checkout\": true\n\
                    }\n\n\
                    Warning: Using `force: true` will reset the branch to point to the new starting point, \
                    potentially losing commits if the branch previously existed. Use with caution. \
                    This is useful when you want to recreate a branch from scratch or reset it to match another branch."
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("What are Git branch naming conventions I should follow?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Follow these best practices for branch naming:\n\n\
                    1. Use lowercase with hyphens (not spaces or underscores):\n   \
                    Good: 'feature-user-auth', 'bugfix-login-error'\n   \
                    Bad: 'Feature_User_Auth', 'bugfix login error'\n\n\
                    2. Use prefixes to indicate branch type:\n   \
                    - 'feature/' for new features: 'feature/payment-integration'\n   \
                    - 'bugfix/' or 'fix/' for bug fixes: 'bugfix/null-pointer-error'\n   \
                    - 'hotfix/' for urgent production fixes: 'hotfix/security-patch'\n   \
                    - 'release/' for release branches: 'release/v2.0.0'\n   \
                    - 'experimental/' for experiments: 'experimental/new-architecture'\n\n\
                    3. Be descriptive but concise:\n   \
                    Good: 'feature/oauth-google-login'\n   \
                    Bad: 'feature/add-authentication-using-oauth-for-google'\n\n\
                    4. Include ticket/issue numbers when applicable:\n   \
                    'feature/jira-123-user-dashboard'\n   \
                    'bugfix/gh-456-memory-leak'\n\n\
                    Example with good naming:\n\
                    {\n  \
                      \"path\": \"/path/to/repo\",\n  \
                      \"branch\": \"feature/oauth-integration\",\n  \
                      \"from_branch\": \"develop\",\n  \
                      \"checkout\": true\n\
                    }"
                ),
            },
        ])
    }
}
