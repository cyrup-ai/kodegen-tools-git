//! Git fetch tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitFetchArgs, GitFetchPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for fetching from remote repositories
#[derive(Clone)]
pub struct GitFetchTool;

impl Tool for GitFetchTool {
    type Args = GitFetchArgs;
    type PromptArgs = GitFetchPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_FETCH
    }

    fn description() -> &'static str {
        "Fetch updates from a remote repository. \
         Downloads objects and refs from another repository."
    }

    fn read_only() -> bool {
        false // Fetches refs
    }

    fn destructive() -> bool {
        false // Only adds, doesn't delete except with prune
    }

    fn idempotent() -> bool {
        true // Safe to fetch repeatedly
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build fetch options
        let mut opts = crate::FetchOpts::from_remote(&args.remote);
        for refspec in &args.refspecs {
            opts = opts.add_refspec(refspec);
        }
        opts = opts.prune(args.prune);

        // Execute fetch
        crate::fetch(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let mut contents = Vec::new();

        // Terminal summary (2 lines with ANSI formatting)
        let prune_status = if args.prune { "yes" } else { "no" };

        let summary = format!(
            "\x1b[36m󰇚 Fetch: {}\x1b[0m\n 󰗚 Refs updated: synced · Prune: {}",
            args.remote, prune_status
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "remote": args.remote,
            "pruned": args.prune
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
