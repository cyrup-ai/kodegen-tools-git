//! Git merge tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitMergeArgs, GitMergePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for merging branches
#[derive(Clone)]
pub struct GitMergeTool;

impl Tool for GitMergeTool {
    type Args = GitMergeArgs;
    type PromptArgs = GitMergePromptArgs;

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

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
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

        let mut contents = Vec::new();

        let (merge_type, commit_id, summary_msg) = match outcome {
            crate::MergeOutcome::FastForward(id) => {
                ("fast_forward", id.to_string(), "Fast-forward merge")
            },
            crate::MergeOutcome::MergeCommit(id) => {
                ("merge_commit", id.to_string(), "Merge commit created")
            },
            crate::MergeOutcome::AlreadyUpToDate => {
                // Terminal summary
                let summary = format!("✓ Already up to date\n\nBranch: {}", args.branch);
                contents.push(Content::text(summary));

                // JSON metadata
                let metadata = json!({
                    "success": true,
                    "merge_type": "already_up_to_date",
                    "message": "Already up to date"
                });
                let json_str = serde_json::to_string_pretty(&metadata)
                    .unwrap_or_else(|_| "{}".to_string());
                contents.push(Content::text(json_str));

                return Ok(contents);
            }
        };

        // Terminal summary for successful merges
        let commit_short = &commit_id[..7.min(commit_id.len())];
        let summary = format!(
            "✓ {}\n\n\
             Branch: {}\n\
             Type: {}\n\
             Commit: {}",
            summary_msg, args.branch, merge_type, commit_short
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "merge_type": merge_type,
            "commit_id": commit_id,
            "message": format!("Merged '{}' ({})", args.branch, merge_type)
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
