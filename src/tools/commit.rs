//! Git commit tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitCommitArgs, GitCommitPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for creating Git commits
#[derive(Clone)]
pub struct GitCommitTool;

impl Tool for GitCommitTool {
    type Args = GitCommitArgs;
    type PromptArgs = GitCommitPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_COMMIT
    }

    fn description() -> &'static str {
        "Create a new commit in a Git repository. \
         Optionally specify author information and stage all modified files."
    }

    fn read_only() -> bool {
        false // Creates commits
    }

    fn destructive() -> bool {
        false // Only creates, doesn't delete
    }

    fn idempotent() -> bool {
        false // Creates new commits each time
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build commit options
        let mut opts = crate::CommitOpts::message(&args.message);
        opts = opts.all(args.all);

        // Set author if provided
        if let (Some(name), Some(email)) = (args.author_name.clone(), args.author_email.clone()) {
            let author = crate::Signature::new(name, email);
            opts = opts.author(author);
        }

        // Create commit
        let commit_result = crate::commit(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let commit_id = commit_result.id;
        let file_count = commit_result.file_count;
        let mut contents = Vec::new();

        // Terminal summary (2 lines with ANSI formatting)
        let commit_short = &commit_id.to_string()[..7.min(commit_id.to_string().len())];
        let first_line = args.message.lines().next().unwrap_or("").to_string();

        let summary = format!(
            "\x1b[32m\u{e725}  Commit: {}\x1b[0m\n\u{f292}  SHA: {} · Files: {}",
            first_line, commit_short, file_count
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "commit_id": commit_id.to_string(),
            "message": args.message,
            "file_count": file_count
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
