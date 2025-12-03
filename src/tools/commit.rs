//! Git commit tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitCommitArgs, GitCommitPromptArgs, GitCommitOutput};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
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

        // Terminal summary (2 lines with ANSI formatting)
        let commit_short = &commit_id.to_string()[..7.min(commit_id.to_string().len())];
        let first_line = args.message.lines().next().unwrap_or("").to_string();

        let summary = format!(
            "\x1b[32m\u{e725}  Commit: {}\x1b[0m\n\u{f292}  SHA: {} · Files: {}",
            first_line, commit_short, file_count
        );

        Ok(ToolResponse::new(summary, GitCommitOutput {
            success: true,
            commit_id: commit_id.to_string(),
            message: args.message.clone(),
            file_count,
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "detail_level".to_string(),
            title: None,
            description: Some(
                "Level of detail in examples: 'basic' for simple commits, 'advanced' for multi-author and staging patterns"
                    .to_string(),
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use git_commit to create commits with different strategies and author information?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_commit tool creates commits with full control over staging and authorship:\n\n\
                     1. Simple commit (uses git config author):\n\
                        git_commit({\"path\": \".\", \"message\": \"Fix: update dependencies\"})\n\n\
                     2. Commit all modified files (auto-stage):\n\
                        git_commit({\"path\": \".\", \"message\": \"Refactor: simplify error handling\", \"all\": true})\n\n\
                     3. Commit with custom author:\n\
                        git_commit({\"path\": \".\", \"message\": \"Docs: update README\", \"author_name\": \"Alice Dev\", \"author_email\": \"alice@example.com\"})\n\n\
                     4. Commit all with custom author:\n\
                        git_commit({\"path\": \".\", \"message\": \"Release: v1.0.0\", \"all\": true, \"author_name\": \"Release Bot\", \"author_email\": \"release@example.com\"})\n\n\
                     Key parameters:\n\
                     - path: Repository root directory (required)\n\
                     - message: Commit message (required) - use conventional commit format (Fix:, Feat:, Docs:, etc.)\n\
                     - all: When true, automatically stages all modified tracked files before creating commit (default: false)\n\
                     - author_name & author_email: Override git config for this specific commit (both required together, optional)\n\n\
                     Best practices:\n\
                     - Use conventional commit prefixes (Fix:, Feat:, Refactor:, Docs:, Test:, Chore:) for clarity\n\
                     - Set 'all: true' only when committing all modified files; otherwise stage specific files with git_add first\n\
                     - Use custom author only for special commits (bots, imported history); normally uses git config\n\
                     - Commit message appears in tool output with first line extracted as title\n\
                     - Returns commit SHA (7-char short form), file count, and commit metadata as JSON",
                ),
            },
        ])
    }
}
