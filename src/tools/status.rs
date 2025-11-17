//! Git status tool

use gix::bstr::ByteSlice;
use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitStatusArgs, GitStatusPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for checking repository status
#[derive(Clone)]
pub struct GitStatusTool;

impl Tool for GitStatusTool {
    type Args = GitStatusArgs;
    type PromptArgs = GitStatusPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_STATUS
    }

    fn description() -> &'static str {
        "Show repository status including current branch, upstream tracking, \
         and working directory state."
    }

    fn read_only() -> bool {
        true // Only reads repository state
    }

    fn destructive() -> bool {
        false // No modifications
    }

    fn idempotent() -> bool {
        true // Safe to call multiple times
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Get is_clean status - inline to avoid Send issues
        let repo_for_clean = repo.clone();
        let is_clean = tokio::task::spawn_blocking(move || {
            let inner = repo_for_clean.clone_inner();
            inner.is_dirty().map(|dirty| !dirty)
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to check clean status: {e}")))?;

        // Get branch information - fully inlined
        let repo_for_branch = repo.clone();
        let (branch_name, commit_hash, upstream, is_detached) = tokio::task::spawn_blocking(move || {
            let inner = repo_for_branch.clone_inner();

            let mut head = inner.head().map_err(|e| anyhow::anyhow!("Failed to get HEAD: {e}"))?;

            let is_detached_head = head.referent_name().is_none();

            let branch_name = head
                .referent_name()
                .and_then(|name| {
                    name.shorten()
                        .to_str()
                        .ok()
                        .map(std::string::ToString::to_string)
                })
                .unwrap_or_else(|| "detached HEAD".to_string());

            let commit = head
                .peel_to_commit()
                .map_err(|e| anyhow::anyhow!("Failed to get commit: {e}"))?;

            let commit_hash = commit.id().to_string();

            // Get upstream information
            let config = inner.config_snapshot();
            let upstream = if let Some(branch_ref) = head.referent_name() {
                let branch_short = branch_ref.shorten();
                let branch_section = format!("branch.{branch_short}");

                let remote_name = config
                    .string(format!("{branch_section}.remote"))
                    .map(|s| s.to_string());

                let merge_ref = config
                    .string(format!("{branch_section}.merge"))
                    .map(|s| s.to_string());

                if let (Some(remote), Some(merge)) = (remote_name, merge_ref) {
                    Some(format!(
                        "{}/{}",
                        remote,
                        merge.trim_start_matches("refs/heads/")
                    ))
                } else {
                    None
                }
            } else {
                None
            };

            Ok::<_, anyhow::Error>((branch_name, commit_hash, upstream, is_detached_head))
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(McpError::Other)?;

        // Calculate ahead/behind counts if upstream exists
        let (ahead_count, behind_count) = if let Some(ref upstream_ref) = upstream {
            let repo_for_counts = repo.clone();
            let upstream_clone = upstream_ref.clone();
            let commit_hash_clone = commit_hash.clone();

            tokio::task::spawn_blocking(move || {
                let inner = repo_for_counts.clone_inner();

                // Parse local commit ID using rev_parse
                let local_commit_id = match inner.rev_parse_single(commit_hash_clone.as_bytes()) {
                    Ok(obj) => obj.detach(),
                    Err(_) => return (None, None),
                };

                // Convert upstream ref string to full reference path
                let upstream_ref_path = if upstream_clone.starts_with("refs/") {
                    upstream_clone.clone()
                } else {
                    format!("refs/remotes/{}", upstream_clone)
                };

                // Try to find the upstream reference
                let upstream_commit_id = match inner.try_find_reference(upstream_ref_path.as_bytes().as_bstr()) {
                    Ok(Some(mut r)) => match r.peel_to_id() {
                        Ok(id) => id.detach(),
                        Err(_) => return (None, None),
                    },
                    _ => return (None, None),
                };

                // If both commits are the same, return (0, 0)
                if local_commit_id == upstream_commit_id {
                    return (Some(0), Some(0));
                }

                // For simplicity, we'll skip the ahead/behind calculation
                // as it requires merge-base computation which is complex
                (None, None)
            })
            .await
            .unwrap_or((None, None))
        } else {
            (None, None)
        };

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let mut summary = String::from("\x1b[36m 󰊢 Repository Status\x1b[0m\n");

        summary.push_str(&format!(
            "  Branch: {}\n\
             Commit: {}\n",
            branch_name,
            &commit_hash[..7.min(commit_hash.len())]
        ));

        // Upstream tracking if configured
        if let Some(ref upstream_str) = upstream {
            let tracking = if let (Some(ahead), Some(behind)) = (ahead_count, behind_count) {
                format!("  Tracking: {} [↑{} ↓{}]\n", upstream_str, ahead, behind)
            } else {
                format!("  Tracking: {}\n", upstream_str)
            };
            summary.push_str(&tracking);
        }

        // Working directory state
        let state_indicator = if is_clean {
            "\x1b[32m✓ Clean\x1b[0m"
        } else {
            "\x1b[33m⚠ Dirty\x1b[0m"
        };
        summary.push_str(&format!("  State: {}", state_indicator));

        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "branch": branch_name,
            "commit": commit_hash,
            "upstream": upstream,
            "ahead": ahead_count,
            "behind": behind_count,
            "is_clean": is_clean,
            "is_detached": is_detached
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to serialize JSON: {e}")))?;
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
