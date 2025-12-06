//! Git fetch tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitFetchArgs, GitFetchOutput, FetchPrompts};
use std::path::Path;

/// Tool for fetching from remote repositories
#[derive(Clone)]
pub struct GitFetchTool;

impl Tool for GitFetchTool {
    type Args = GitFetchArgs;
    type Prompts = FetchPrompts;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
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

        // Terminal summary (2 lines with ANSI formatting)
        let prune_status = if args.prune { "yes" } else { "no" };

        let summary = format!(
            "\x1b[36m󰇚 Fetch: {}\x1b[0m\n 󰗚 Refs updated: synced · Prune: {}",
            args.remote, prune_status
        );

        Ok(ToolResponse::new(summary, GitFetchOutput {
            success: true,
            remote: args.remote.clone(),
            pruned: args.prune,
        }))
    }
}
