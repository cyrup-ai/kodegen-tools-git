//! Git reset tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitResetArgs, GitResetPromptArgs, ResetMode};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for resetting repository to a specific commit
#[derive(Clone)]
pub struct GitResetTool;

impl Tool for GitResetTool {
    type Args = GitResetArgs;
    type PromptArgs = GitResetPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_RESET
    }

    fn description() -> &'static str {
        "Reset repository to a specific commit. \
         Soft: move HEAD only. Mixed: move HEAD and reset index. \
         Hard: move HEAD, reset index, and working directory."
    }

    fn read_only() -> bool {
        false // Modifies repository state
    }

    fn destructive() -> bool {
        true // Can discard local changes (especially hard mode)
    }

    fn idempotent() -> bool {
        true // Safe to reset to same target multiple times
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository and execute reset in a spawn_blocking context
        // to avoid Send issues with RepoHandle
        let mode = args.mode;
        let target_for_output = args.target.clone();
        let target = args.target;
        let path_buf = path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                // Open repository
                let repo = crate::open_repo(&path_buf)
                    .await
                    .map_err(|e| anyhow::anyhow!("Task execution failed: {e}"))?
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

                // Map schema ResetMode to operation ResetMode
                let op_mode = match mode {
                    ResetMode::Soft => crate::ResetMode::Soft,
                    ResetMode::Mixed => crate::ResetMode::Mixed,
                    ResetMode::Hard => crate::ResetMode::Hard,
                };

                // Build reset options
                let opts = crate::ResetOpts {
                    target,
                    mode: op_mode,
                    cancel_token: None,
                };

                // Execute reset
                crate::reset(&repo, opts)
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))
            })
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(|e: anyhow::Error| McpError::Other(e))?;

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let mode_str = match mode {
            ResetMode::Soft => "soft",
            ResetMode::Mixed => "mixed",
            ResetMode::Hard => "hard",
        };

        let summary = format!(
            "\x1b[33m ⟲ Reset Complete\x1b[0m\n\
             ℹ Mode: {} · Target: {}",
            mode_str, target_for_output
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "mode": mode_str,
            "target": target_for_output
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
