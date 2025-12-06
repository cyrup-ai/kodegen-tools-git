//! Git repository initialization tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitInitArgs, GitInitOutput, InitPrompts};
use std::path::Path;

/// Tool for initializing Git repositories
#[derive(Clone)]
pub struct GitInitTool;

impl Tool for GitInitTool {
    type Args = GitInitArgs;
    type Prompts = InitPrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_INIT
    }

    fn description() -> &'static str {
        "Initialize a new Git repository at the specified path. \
         Supports both normal repositories (with working directory) and \
         bare repositories (without working directory, typically for servers)."
    }

    fn read_only() -> bool {
        false // Creates files/directories
    }

    fn destructive() -> bool {
        false // Only creates, doesn't delete
    }

    fn idempotent() -> bool {
        false // Will fail if repo already exists
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Call appropriate function based on bare flag
        let task = if args.bare {
            crate::init_bare_repo(path)
        } else {
            crate::init_repo(path)
        };

        // Await AsyncTask, handle both layers of Result
        let _repo = task
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Terminal summary
        let repo_type = if args.bare { "bare" } else { "normal" };

        // Line 1: Green colored init action with path
        // Line 2: White metadata line with type and path
        let summary = format!(
            "\x1b[32m Init Repository: {}\x1b[0m\n\
              Type: {} · Path: {}",
            args.path,
            repo_type,
            args.path
        );

        Ok(ToolResponse::new(summary, GitInitOutput {
            success: true,
            path: args.path.clone(),
            bare: args.bare,
            message: format!("Initialized {} Git repository at {}", repo_type, args.path),
        }))
    }
}
