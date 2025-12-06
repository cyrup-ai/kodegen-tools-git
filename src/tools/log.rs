//! Git log tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitLogArgs, GitLogOutput, GitCommitInfo, GitAuthorInfo, LogPrompts};
use std::path::Path;
use tokio_stream::StreamExt;

/// Tool for listing Git commit history
#[derive(Clone)]
pub struct GitLogTool;

impl Tool for GitLogTool {
    type Args = GitLogArgs;
    type Prompts = LogPrompts;

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

    async fn execute(&self, args: Self::Args, ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
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
        let mut stream = crate::log(repo, opts, ctx.pwd());

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

                    commits.push(GitCommitInfo {
                        id: commit_info.id.to_string(),
                        author: GitAuthorInfo {
                            name: commit_info.author.name.clone(),
                            email: commit_info.author.email.clone(),
                            time: commit_info.author.time.to_rfc3339(),
                        },
                        summary: commit_info.summary.clone(),
                        time: commit_info.time.to_rfc3339(),
                    });
                }
                Err(e) => {
                    return Err(McpError::Other(anyhow::anyhow!("{e}")));
                }
            }
        }

        // Build summary
        let summary = if commits.is_empty() {
            "\x1b[36m󰄶 Commit History\x1b[0m\n 󰗚 Commits: 0 · No commits found".to_string()
        } else {
            let latest_message = commits
                .first()
                .map(|c| c.summary.as_str())
                .unwrap_or("");

            format!(
                "\x1b[36m󰄶 Commit History\x1b[0m\n 󰗚 Commits: {} · Latest: {}",
                commits.len(),
                latest_message
            )
        };

        let count = commits.len();

        Ok(ToolResponse::new(summary, GitLogOutput {
            success: true,
            commits,
            count,
        }))
    }
}
