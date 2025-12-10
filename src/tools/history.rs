//! Git history tool - investigate how code evolved

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{
    GitHistoryArgs, GitHistoryCommit, GitHistoryOutput, GIT_HISTORY, HistoryPrompts,
};
use std::path::Path;

/// Tool for investigating file history with actual diffs
#[derive(Clone)]
pub struct GitHistoryTool;

impl Tool for GitHistoryTool {
    type Args = GitHistoryArgs;
    type Prompts = HistoryPrompts;

    fn name() -> &'static str {
        GIT_HISTORY
    }

    fn description() -> &'static str {
        "Investigate how a file changed over time with actual diffs. \
         Search for when specific code was added, removed, or modified. \
         Compare versions to see cumulative changes."
    }

    fn read_only() -> bool {
        true
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true
    }

    async fn execute(
        &self,
        args: Self::Args,
        _ctx: ToolExecutionContext,
    ) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let repo = crate::open_repo(Path::new(&args.path))
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build options
        let mut opts = crate::HistoryOpts::new(&args.file).limit(args.limit);

        if let Some(ref search) = args.search {
            opts = opts
                .search(search)
                .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;
        }
        if let Some(since) = args.since {
            opts = opts.since(since);
        }
        if let Some(until) = args.until {
            opts = opts.until(until);
        }

        let result = crate::history(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Format output based on mode
        match result {
            crate::HistoryResult::Commits {
                file,
                total_examined,
                commits,
            } => {
                let search_note = if args.search.is_some() {
                    format!(" matching \"{}\"", args.search.as_ref().map_or("", |s| s.as_str()))
                } else {
                    String::new()
                };

                let mut summary = format!(
                    "\x1b[36mFile History: {}\x1b[0m\n Found: {} commits{}\n Examined: {} total commits\n\n",
                    file, commits.len(), search_note, total_examined
                );

                for c in &commits {
                    summary.push_str(&format!(
                        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                         \x1b[33m{}\x1b[0m · {} · \x1b[32m+{}\x1b[0m \x1b[31m-{}\x1b[0m\n\
                         {}\n\n\
                         {}\n\n",
                        c.id,
                        c.time.format("%Y-%m-%d %H:%M"),
                        c.additions,
                        c.deletions,
                        c.summary,
                        c.diff
                    ));
                }

                let output = GitHistoryOutput {
                    success: true,
                    file,
                    mode: "commits".to_string(),
                    total_examined: Some(total_examined),
                    commits: Some(
                        commits
                            .into_iter()
                            .map(|c| GitHistoryCommit {
                                id: c.id,
                                summary: c.summary,
                                time: c.time.to_rfc3339(),
                                additions: c.additions,
                                deletions: c.deletions,
                                diff: c.diff,
                            })
                            .collect(),
                    ),
                    since: None,
                    until: None,
                    additions: None,
                    deletions: None,
                    diff: None,
                };

                Ok(ToolResponse::new(summary, output))
            }

            crate::HistoryResult::Range {
                file,
                since,
                until,
                additions,
                deletions,
                diff,
            } => {
                let summary = format!(
                    "\x1b[36mFile History: {} ({} → {})\x1b[0m\n Changes: \x1b[32m+{}\x1b[0m \x1b[31m-{}\x1b[0m lines\n\n\
                     ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                     {}\n",
                    file, since, until, additions, deletions, diff
                );

                let output = GitHistoryOutput {
                    success: true,
                    file,
                    mode: "range".to_string(),
                    total_examined: None,
                    commits: None,
                    since: Some(since),
                    until: Some(until),
                    additions: Some(additions),
                    deletions: Some(deletions),
                    diff: Some(diff),
                };

                Ok(ToolResponse::new(summary, output))
            }
        }
    }
}
