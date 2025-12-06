//! Git merge tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitMergeArgs, GitMergeOutput, MergePrompts};
use std::path::Path;

/// Tool for merging branches
#[derive(Clone)]
pub struct GitMergeTool;

impl Tool for GitMergeTool {
    type Args = GitMergeArgs;
    type Prompts = MergePrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_MERGE
    }

    fn description() -> &'static str {
        "Merge a branch or commit into the current branch. \
         Joins two or more development histories together."
    }

    fn read_only() -> bool {
        false // Modifies HEAD and working tree
    }

    fn destructive() -> bool {
        true // Can overwrite local changes
    }

    fn idempotent() -> bool {
        false // Creates new commits
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build merge options (note inverse logic for no_ff)
        let mut opts = crate::MergeOpts::new(&args.branch);
        opts = opts.no_ff(!args.fast_forward); // Inverse logic
        opts = opts.commit(args.auto_commit);

        // Execute merge
        let outcome = crate::merge(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let (merge_type, commit_id) = match outcome {
            crate::MergeOutcome::FastForward(id) => {
                ("fast_forward", Some(id.to_string()))
            },
            crate::MergeOutcome::MergeCommit(id) => {
                ("merge_commit", Some(id.to_string()))
            },
            crate::MergeOutcome::AlreadyUpToDate => {
                // Terminal summary
                let summary = format!("✓ Already up to date\n\nBranch: {}", args.branch);

                return Ok(ToolResponse::new(summary, GitMergeOutput {
                    success: true,
                    merge_type: "already_up_to_date".to_string(),
                    commit_id: None,
                    message: "Already up to date".to_string(),
                }));
            }
        };

        // Terminal summary with ANSI yellow color and Nerd Font icons
        // Successful merges have no conflicts (conflicts cause errors in the merge operation)
        let summary = format!(
            "\x1b[33m\u{e727} Merge: {}\x1b[0m\n\
             \u{2139} Strategy: {} · Conflicts: 0",
            args.branch, merge_type
        );

        Ok(ToolResponse::new(summary, GitMergeOutput {
            success: true,
            merge_type: merge_type.to_string(),
            commit_id,
            message: format!("Merged '{}' ({})", args.branch, merge_type),
        }))
    }
}
