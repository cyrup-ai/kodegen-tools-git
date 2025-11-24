//! Git repository discovery tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitDiscoverArgs, GitDiscoverPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content, PromptMessageRole, PromptMessageContent};
use serde_json::json;
use std::path::Path;

/// Tool for discovering Git repositories by searching upward
#[derive(Clone)]
pub struct GitDiscoverTool;

impl Tool for GitDiscoverTool {
    type Args = GitDiscoverArgs;
    type PromptArgs = GitDiscoverPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_DISCOVER
    }

    fn description() -> &'static str {
        "Discover a Git repository by searching upward from the given path. \
         This will traverse parent directories until it finds a .git directory \
         or reaches the filesystem root."
    }

    fn read_only() -> bool {
        true // Only searches, doesn't modify
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true // Safe to call repeatedly
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        let repo = crate::discover_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Extract the working directory path from the discovered repository
        let repo_root = repo.raw()
            .workdir()
            .ok_or_else(|| McpError::Other(anyhow::anyhow!("Repository has no working directory")))?
            .display()
            .to_string();

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "\x1b[36m Discover Repository: {}\x1b[0m\n\
              Started from: {} · Found: {}",
            repo_root, args.path, repo_root
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "searched_from": args.path,
            "message": format!("Discovered Git repository from path {}", args.path)
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "focus_area".to_string(),
            title: Some("Focus Area".to_string()),
            description: Some(
                "What aspect to focus on: 'basic-usage' (when and how to use), \
                 'search-strategy' (how upward search works), or 'edge-cases' \
                 (bare repos, filesystem boundaries)".to_string()
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use git_discover to find a Git repository?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_discover tool locates a Git repository by searching upward through \
                     the filesystem hierarchy from a given starting path.\n\n\
                     \
                     BASIC USAGE:\n\
                     git_discover({\"path\": \"/some/deep/nested/dir\"})\n\
                     Returns: The root directory of the Git repository (where .git/ exists)\n\n\
                     \
                     COMMON USE CASES:\n\
                     1. Determine repository context: When given an arbitrary file path, \
                     find its containing repository\n\
                     2. Locate .git directory: Get the exact path to the repository root \
                     (useful for git operations)\n\
                     3. Validate git presence: Verify a path is within a Git repository\n\n\
                     \
                     SEARCH BEHAVIOR:\n\
                     - Starts at the provided path\n\
                     - Walks upward through parent directories\n\
                     - Stops at the first .git/ directory found\n\
                     - Errors if no repository is found before filesystem root\n\
                     - Works from any subdirectory (doesn't require starting from repo root)\n\n\
                     \
                     PARAMETERS:\n\
                     - path (required): String path to search from. Can be:\n\
                       - Absolute path: \"/home/user/project/src/module\"\n\
                       - Relative path: \"./src/components\"\n\
                       - Nested deeply: search is efficient and always finds repo\n\n\
                     \
                     OUTPUT:\n\
                     Returns JSON metadata with:\n\
                     - success: Boolean indicating if repo was found\n\
                     - searched_from: Original path provided\n\
                     - message: Descriptive message about the discovery\n\n\
                     \
                     BEST PRACTICES:\n\
                     1. Use before running other git operations to ensure you're in a repo\n\
                     2. Cache the result if calling multiple tools (repo root doesn't change)\n\
                     3. Handle the error case when no repository exists\n\n\
                     \
                     IMPORTANT NOTES:\n\
                     - Searches stop at filesystem root; bare repositories with .git as file \
                     are not supported by this tool\n\
                     - The search is idempotent: calling multiple times returns the same result\n\
                     - Permissions: requires read access to directory hierarchy\n\
                     - Safe: tool is read-only and never modifies filesystem",
                ),
            },
        ])
    }
}
