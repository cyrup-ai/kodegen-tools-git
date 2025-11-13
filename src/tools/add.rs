//! Git add (staging) tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitAddArgs, GitAddPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for staging files in Git
#[derive(Clone)]
pub struct GitAddTool;

impl Tool for GitAddTool {
    type Args = GitAddArgs;
    type PromptArgs = GitAddPromptArgs;

    fn name() -> &'static str {
        "git_add"
    }

    fn description() -> &'static str {
        "Stage file changes for commit in a Git repository. \
         Specify paths to stage specific files."
    }

    fn read_only() -> bool {
        false // Modifies index
    }

    fn destructive() -> bool {
        false // Only stages, doesn't delete
    }

    fn idempotent() -> bool {
        true // Staging same files multiple times is safe
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Determine which paths to stage
        let paths_to_stage = if args.all {
            // Use "." to stage all files (AddOpts supports glob patterns)
            vec![".".to_string()]
        } else if args.paths.is_empty() {
            return Err(McpError::InvalidArguments(
                "No paths specified to stage. Provide paths or use all=true.".to_string(),
            ));
        } else {
            args.paths.clone()
        };

        // Build add options
        let mut opts = crate::AddOpts::new(paths_to_stage.clone());
        opts = opts.force(args.force);

        // Execute add
        crate::add(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let count = paths_to_stage.len();

        let mut contents = Vec::new();

        // Terminal summary
        let summary = if args.all {
            "✓ Files staged\n\nAll modified files staged".to_string()
        } else {
            format!(
                "✓ Files staged ({})\n\n{}",
                count,
                paths_to_stage.iter()
                    .map(|p| format!("  • {}", p))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "all": args.all,
            "paths": if args.all { vec![".".to_string()] } else { paths_to_stage },
            "count": if args.all { 1 } else { count }
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
