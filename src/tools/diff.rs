//! Git diff tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitDiffArgs, GitDiffPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for displaying Git diffs
#[derive(Clone)]
pub struct GitDiffTool;

impl Tool for GitDiffTool {
    type Args = GitDiffArgs;
    type PromptArgs = GitDiffPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_DIFF
    }

    fn description() -> &'static str {
        "Show differences between Git revisions. \
         Compare two commits, branches, or working directory against HEAD. \
         Displays file changes with statistics."
    }

    fn read_only() -> bool {
        true // Only reads
    }

    fn destructive() -> bool {
        false // No side effects
    }

    fn idempotent() -> bool {
        true // Same output for same inputs
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build diff options
        let mut opts = crate::DiffOpts::new(&args.from);
        if let Some(to) = args.to.clone() {
            opts = opts.to(to);
        }

        // Execute diff
        let stats = crate::diff(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let mut contents = Vec::new();

        // Terminal summary
        contents.push(Content::text(format_diff_output(&stats, &args.from, &args.to)));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "from": args.from,
            "to": args.to.unwrap_or_else(|| "working directory".to_string()),
            "files_changed": stats.total_files_changed,
            "insertions": stats.total_additions,
            "deletions": stats.total_deletions,
            "files": stats.files.iter().map(|f| json!({
                "path": f.path,
                "change_type": format!("{:?}", f.change_type),
                "additions": f.additions,
                "deletions": f.deletions,
            })).collect::<Vec<_>>()
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

/// Format diff statistics for terminal output with colors and icons
fn format_diff_output(stats: &crate::DiffStats, from: &str, to: &Option<String>) -> String {
    let to_str = to.as_deref().unwrap_or("working directory");

    let mut output = String::new();

    // Header line with diff icon ( from git icon)
    output.push_str(&format!(
        "\x1b[36m  Diff: {} → {}\x1b[0m\n",
        from, to_str
    ));

    // File listing
    for file in &stats.files {
        let change_icon = match file.change_type {
            crate::ChangeType::Added => "\x1b[32m\x1b[0m",     // Green  (added)
            crate::ChangeType::Deleted => "\x1b[31m\x1b[0m",   // Red  (deleted)
            crate::ChangeType::Modified => "\x1b[33m\x1b[0m",  // Yellow  (modified)
            crate::ChangeType::Renamed => "\x1b[35m\x1b[0m",  // Magenta  (renamed)
        };

        let change_label = match file.change_type {
            crate::ChangeType::Added => "added",
            crate::ChangeType::Deleted => "deleted",
            crate::ChangeType::Modified => "modified",
            crate::ChangeType::Renamed => "renamed",
        };

        output.push_str(&format!(
            "  {} {} \x1b[90m({}: +{}, -{}\x1b[0m\n",
            change_icon, file.path, change_label, file.additions, file.deletions
        ));
    }

    // Summary line
    output.push_str(&format!(
        "\n  \x1b[1m{} files changed, {} insertions(+), {} deletions(-)\x1b[0m",
        stats.total_files_changed, stats.total_additions, stats.total_deletions
    ));

    output
}
