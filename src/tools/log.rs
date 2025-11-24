//! Git log tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitLogArgs, GitLogPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content, PromptMessageContent, PromptMessageRole};
use serde_json::json;
use std::path::Path;
use tokio_stream::StreamExt;

/// Tool for listing Git commit history
#[derive(Clone)]
pub struct GitLogTool;

impl Tool for GitLogTool {
    type Args = GitLogArgs;
    type PromptArgs = GitLogPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_LOG
    }

    fn description() -> &'static str {
        "List commit history from a Git repository. \
         Optionally filter by file path and limit the number of results."
    }

    fn read_only() -> bool {
        true // Only reads, doesn't modify
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

        // Build log options
        let mut opts = crate::LogOpts::new();

        if let Some(max_count) = args.max_count {
            opts = opts.max_count(max_count + args.skip);
        }

        if let Some(path_filter) = args.path_filter {
            opts = opts.path(path_filter);
        }

        // Get log stream
        let mut stream = crate::log(repo, opts);

        // Collect commits
        let mut commits = Vec::new();
        let mut skipped = 0;

        while let Some(result) = stream.next().await {
            match result {
                Ok(commit_info) => {
                    // Skip first N commits if requested
                    if skipped < args.skip {
                        skipped += 1;
                        continue;
                    }

                    commits.push(json!({
                        "id": commit_info.id.to_string(),
                        "author": {
                            "name": commit_info.author.name,
                            "email": commit_info.author.email,
                            "time": commit_info.author.time.to_rfc3339()
                        },
                        "summary": commit_info.summary,
                        "time": commit_info.time.to_rfc3339()
                    }));
                }
                Err(e) => {
                    return Err(McpError::Other(anyhow::anyhow!("{e}")));
                }
            }
        }

        let mut contents = Vec::new();

        // ========================================
        // Content[0]: Human-Readable Summary
        // ========================================
        let summary = if commits.is_empty() {
            "\x1b[36m󰄶 Commit History\x1b[0m\n 󰗚 Commits: 0 · No commits found".to_string()
        } else {
            let latest_message = commits
                .first()
                .and_then(|c| c.get("summary"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            format!(
                "\x1b[36m󰄶 Commit History\x1b[0m\n 󰗚 Commits: {} · Latest: {}",
                commits.len(),
                latest_message
            )
        };
        contents.push(Content::text(summary));

        // ========================================
        // Content[1]: Machine-Parseable JSON
        // ========================================
        let metadata = json!({
            "success": true,
            "commits": commits,
            "count": commits.len()
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
                content: PromptMessageContent::text(
                    "How do I use the git_log tool to view commit history and search for specific commits?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_log tool displays commit history from a Git repository. Here's how to use it:\n\n\
                     ## Basic Usage\n\
                     View recent commits: git_log({\"path\": \".\"})\n\n\
                     ## Limiting Results\n\
                     View last 10 commits: git_log({\"path\": \".\", \"max_count\": 10})\n\
                     Skip first 20 commits: git_log({\"path\": \".\", \"skip\": 20})\n\
                     Skip 20, show next 10: git_log({\"path\": \".\", \"skip\": 20, \"max_count\": 10})\n\n\
                     ## Filtering by File\n\
                     Find commits affecting a file: git_log({\"path\": \".\", \"path_filter\": \"src/main.rs\"})\n\
                     Commits for specific path (last 5): git_log({\"path\": \".\", \"path_filter\": \"docs/\", \"max_count\": 5})\n\n\
                     ## Output Format\n\
                     Each commit includes:\n\
                     - id: Full commit SHA\n\
                     - author.name: Committer name\n\
                     - author.email: Committer email\n\
                     - author.time: Commit timestamp (RFC 3339 format)\n\
                     - summary: Commit message subject line\n\
                     - time: Commit creation timestamp\n\n\
                     ## Common Patterns\n\
                     Find a change in specific file: Use path_filter to narrow search, then inspect output\n\
                     Efficient history browsing: Use pagination (max_count + skip) to avoid loading entire history\n\
                     Debugging merges: View all commits affecting a directory/file to trace changes\n\n\
                     ## Best Practices\n\
                     - Use max_count to limit results and improve performance on large repositories\n\
                     - Combine path_filter with max_count when searching within a specific file or directory\n\
                     - The skip parameter is useful for pagination: request first batch, then next batch, etc.\n\
                     - Commits are ordered from newest to oldest (chronological reverse order)\n\
                     - Empty results mean no commits match the filter criteria\n\n\
                     ## Important Notes\n\
                     - This tool is read-only and safe to call multiple times\n\
                     - Path must be a valid repository directory\n\
                     - path_filter uses Git's pathspec syntax (glob patterns work)\n\
                     - Large skip values may be slow on large repositories",
                ),
            },
        ])
    }
}
