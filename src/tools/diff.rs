//! Git diff tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitDiffArgs, GitDiffOutput, GitDiffFile, DiffPrompts};
use std::path::Path;

/// Tool for displaying Git diffs
#[derive(Clone)]
pub struct GitDiffTool;

impl Tool for GitDiffTool {
    type Args = GitDiffArgs;
    type Prompts = DiffPrompts;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
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

        // Terminal summary
        let summary = format_diff_output(&stats, &args.from, &args.to);

        // Build output files
        let files: Vec<GitDiffFile> = stats.files.iter().map(|f| GitDiffFile {
            path: f.path.clone(),
            change_type: format!("{:?}", f.change_type),
            additions: f.additions as u32,
            deletions: f.deletions as u32,
        }).collect();

        Ok(ToolResponse::new(summary, GitDiffOutput {
            success: true,
            from: args.from.clone(),
            to: args.to.clone().unwrap_or_else(|| "working directory".to_string()),
            files_changed: stats.total_files_changed as u32,
            insertions: stats.total_additions as u32,
            deletions: stats.total_deletions as u32,
            files,
        }))
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
