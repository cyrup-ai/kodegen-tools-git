//! Git pull tool

use gix::bstr::ByteSlice;
use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitPullArgs, GitPullPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for pulling from remote repositories
#[derive(Clone)]
pub struct GitPullTool;

impl Tool for GitPullTool {
    type Args = GitPullArgs;
    type PromptArgs = GitPullPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_PULL
    }

    fn description() -> &'static str {
        "Pull changes from a remote repository. \
         Fetches and merges remote changes into the current branch. \
         Equivalent to running 'git fetch' followed by 'git merge'."
    }

    fn read_only() -> bool {
        false // Modifies HEAD and working tree
    }

    fn destructive() -> bool {
        false // Non-destructive merge operation
    }

    fn idempotent() -> bool {
        false // Can create new merge commits
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Get current branch name without holding a reference across await
        // We clone the inner repository to avoid Send issues
        let repo_for_current = repo.clone();
        let branch_name = {
            let inner = repo_for_current.clone_inner();
            tokio::task::spawn_blocking(move || {
                let head = inner.head().ok()?;
                head.referent_name()
                    .and_then(|name| {
                        name.shorten()
                            .to_str()
                            .ok()
                            .map(std::string::ToString::to_string)
                    })
            })
            .await
            .ok()
            .and_then(|x| x)
            .unwrap_or_else(|| "HEAD".to_string())
        };

        // Build pull options
        let opts = crate::PullOpts {
            remote: args.remote.clone(),
            branch: branch_name,
            fast_forward: args.fast_forward,
            auto_commit: args.auto_commit,
        };

        // Execute pull
        let result = crate::pull(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let mut contents = Vec::new();

        // Determine merge outcome string
        let merge_outcome_str = match &result.merge_outcome {
            crate::MergeOutcome::FastForward(_) => "fast_forward",
            crate::MergeOutcome::MergeCommit(_) => "merge_commit",
            crate::MergeOutcome::AlreadyUpToDate => "already_up_to_date",
        };

        // Terminal summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "\x1b[36m ⬇ Pull from {}\x1b[0m\n  ℹ Merge: {}",
            args.remote, merge_outcome_str
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "remote": args.remote,
            "merge_outcome": merge_outcome_str
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
