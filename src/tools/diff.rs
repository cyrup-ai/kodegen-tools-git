//! Git diff tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitDiffArgs, GitDiffPromptArgs, GitDiffOutput, GitDiffFile};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
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

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "use_case".to_string(),
            title: None,
            description: Some(
                "Type of diff example to focus on (e.g., 'commit_to_commit', 'branch_comparison', 'staged_changes', 'working_directory')".to_string()
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "When and why should I use git_diff? What are the main ways to compare code?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_diff tool is essential for code review, understanding history, and auditing changes. \
                     It supports three main comparison patterns:\n\n\
                     1. COMMIT-TO-COMMIT: Compare specific commits using their hashes\n\
                        git_diff({\"path\": \".\", \"from\": \"abc1234\", \"to\": \"def5678\"})\n\
                        Use this when reviewing exactly what changed between two points in history.\n\n\
                     2. BRANCH COMPARISON: Compare branches, ideal for previewing pull requests\n\
                        git_diff({\"path\": \".\", \"from\": \"main\", \"to\": \"feature-branch\"})\n\
                        Shows all changes that would be merged into the target branch.\n\n\
                     3. WORKING DIRECTORY: Omit the 'to' parameter to see uncommitted changes\n\
                        git_diff({\"path\": \".\", \"from\": \"HEAD\"})\n\
                        Displays file modifications not yet staged or committed.\n\n\
                     Output Format:\n\
                     - Icons show change type: (added), (deleted), (modified), (renamed)\n\
                     - Each file shows: +additions, -deletions\n\
                     - Summary line shows total files changed and statistics\n\
                     - JSON metadata contains structured data for programmatic access\n\n\
                     Common reasons to use git_diff:\n\
                     - Code review before merging\n\
                     - Understanding what changed in a commit\n\
                     - Auditing changes before deployment\n\
                     - Tracking accidental modifications to files",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What are some practical workflows where git_diff is helpful?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Here are five practical workflows using git_diff:\n\n\
                     WORKFLOW 1: Code Review Before Merging\n\
                     git_diff({\"path\": \".\", \"from\": \"main\", \"to\": \"feature-branch\"})\n\
                     Review all changes a feature branch introduces before merging. This prevents \
                     unintended side effects and ensures code quality standards.\n\n\
                     WORKFLOW 2: Check Uncommitted Changes\n\
                     git_diff({\"path\": \".\", \"from\": \"HEAD\"})\n\
                     See exactly what you've modified in your working directory before staging or \
                     committing. Essential for keeping changes focused and organized.\n\n\
                     WORKFLOW 3: Bisect Helper (Review Individual Commits)\n\
                     git_diff({\"path\": \".\", \"from\": \"abc1234~1\", \"to\": \"abc1234\"})\n\
                     Inspect the changes introduced by a specific commit using parent comparison. \
                     Useful for understanding bug fixes or refactoring changes.\n\n\
                     WORKFLOW 4: Release Notes Generation\n\
                     git_diff({\"path\": \".\", \"from\": \"v1.0.0\", \"to\": \"v1.1.0\"})\n\
                     Compare tagged versions to automatically identify what features and fixes were \
                     included in the new release.\n\n\
                     WORKFLOW 5: Merge Conflict Understanding\n\
                     git_diff({\"path\": \".\", \"from\": \"main\", \"to\": \"conflicting-branch\"})\n\
                     Understand the full scope of diverging changes before manually resolving conflicts. \
                     Gives context for merge conflict resolution decisions.",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What are the best practices and common gotchas I should watch out for?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Best Practices:\n\
                     - Use absolute paths or '.' for the repository root\n\
                     - Use 'HEAD' to reference the current commit consistently\n\
                     - Handle merge commits carefully; understand they have multiple parents\n\
                     - Prefer branch names over short hashes for clarity and stability\n\
                     - Always check the file count first; don't assume small diffs\n\n\
                     GOTCHA 1: Forgetting to omit 'to' for working directory\n\
                     WRONG: git_diff({\"path\": \".\", \"from\": \"HEAD\", \"to\": \"working\"})\n\
                     RIGHT: git_diff({\"path\": \".\", \"from\": \"HEAD\"})\n\
                     When comparing against uncommitted changes, never include a 'to' parameter.\n\n\
                     GOTCHA 2: Direction matters - from/to order affects output\n\
                     git_diff({\"path\": \".\", \"from\": \"main\", \"to\": \"feature\"}) shows feature changes\n\
                     git_diff({\"path\": \".\", \"from\": \"feature\", \"to\": \"main\"}) shows main changes\n\
                     The order determines perspective; be intentional about which branch you're comparing from.\n\n\
                     GOTCHA 3: Short hash ambiguity in large repositories\n\
                     Use at least 7-8 character hashes or full hashes to avoid ambiguity errors.\n\
                     Branch names are preferred as they're unambiguous and self-documenting.\n\n\
                     GOTCHA 4: Rename detection shows as single entry\n\
                     Renamed files appear as one change with the 'renamed' label, not as separate \
                     added/deleted entries. This makes the file count smaller than expected.\n\n\
                     GOTCHA 5: JSON metadata contains structured data for programmatic access\n\
                     The JSON output includes precise counts and file-by-file statistics. Use this \
                     for automation rather than parsing terminal output, which may have formatting changes.",
                ),
            },
        ])
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
