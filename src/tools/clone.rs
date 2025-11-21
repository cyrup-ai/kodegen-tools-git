//! Git repository cloning tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitCloneArgs, GitClonePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;

/// Tool for cloning remote Git repositories
#[derive(Clone)]
pub struct GitCloneTool;

impl Tool for GitCloneTool {
    type Args = GitCloneArgs;
    type PromptArgs = GitClonePromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_CLONE
    }

    fn description() -> &'static str {
        "Clone a remote Git repository to a local path. \
         Supports shallow cloning (limited history) and branch-specific cloning. \
         The destination path must not already exist."
    }

    fn read_only() -> bool {
        false // Creates files/directories
    }

    fn destructive() -> bool {
        false // Only creates, doesn't delete
    }

    fn idempotent() -> bool {
        false // Will fail if destination exists
    }

    fn open_world() -> bool {
        true // Makes network requests
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let mut opts = crate::CloneOpts::new(&args.url, &args.path);

        if let Some(depth) = args.depth {
            opts = opts.shallow(depth);
        }

        if let Some(ref branch) = args.branch {
            opts = opts.branch(branch);
        }

        let repo = crate::clone_repo(opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Get the actual branch name from HEAD
        let branch_name = if let Some(ref b) = args.branch {
            b.clone()
        } else {
            // Query the repository HEAD to get the default branch
            repo.raw()
                .head_name()
                .ok()
                .flatten()
                .map(|name| name.shorten().to_string())
                .unwrap_or_else(|| "HEAD".to_string())
        };

        let mut contents = Vec::new();

        // Build optional metadata
        let mut metadata_parts = vec![
            format!("Destination: {}", args.path),
            format!("Branch: {}", branch_name),
        ];

        if let Some(depth) = args.depth {
            metadata_parts.push(format!("Depth: {}", depth));
        }

        // Line 1: Green colored clone action with URL
        // Line 2: White metadata line
        let summary = format!(
            "\x1b[32m󰇚 Clone: {}\x1b[0m\n\
             󰉋 {}",
            args.url,
            metadata_parts.join(" · ")
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "url": args.url,
            "path": args.path,
            "branch": branch_name,
            "shallow": args.depth.is_some(),
            "depth": args.depth,
            "message": format!("Cloned {} to {}", args.url, args.path)
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
