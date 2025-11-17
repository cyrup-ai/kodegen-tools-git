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
        kodegen_mcp_schema::git::GIT_ADD
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

        // Build pattern string for display
        let pattern = if args.all {
            "all".to_string()
        } else {
            // Show first 3 paths, then "+N more" if exceeds
            let shown = paths_to_stage
                .iter()
                .take(3)
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            if paths_to_stage.len() > 3 {
                format!("{} +{} more", shown, paths_to_stage.len() - 3)
            } else {
                shown
            }
        };

        // Terminal summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "\x1b[32m✚ Staged Changes\x1b[0m\n  📄 Files: {} · Pattern: {}",
            count, pattern
        );
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
