//! Git add (staging) tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitAddArgs, GitAddPromptArgs, GitAddOutput};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
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

        Ok(ToolResponse::new(summary, GitAddOutput {
            success: true,
            all: args.all,
            paths: if args.all { vec![".".to_string()] } else { paths_to_stage },
            count: if args.all { 1 } else { count },
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use git_add to stage files for committing?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_add tool stages file changes to the Git index, which is the intermediate \
                     step between making changes in your working directory and creating a commit. \
                     The index holds all staged changes that will be included in the next commit.\n\n\
                     Basic usage examples:\n\n\
                     1. Stage specific files:\n   \
                     git_add({\"path\": \"/repo\", \"paths\": [\"src/main.rs\", \"Cargo.toml\"]})\n\n\
                     2. Stage all modified files:\n   \
                     git_add({\"path\": \"/repo\", \"all\": true})\n\n\
                     3. Stage a single file:\n   \
                     git_add({\"path\": \"/repo\", \"paths\": [\"README.md\"]})\n\n\
                     4. Force-add files that match .gitignore patterns:\n   \
                     git_add({\"path\": \"/repo\", \"paths\": [\"secret.key\"], \"force\": true})\n\n\
                     The tool automatically:\n\
                     - Validates that the repository path exists and is a valid Git repository\n\
                     - Opens the repository and accesses the Git index\n\
                     - Stages the specified files or patterns to the index\n\
                     - Leaves unstaged changes in your working directory untouched\n\
                     - Returns success status with a count of staged file patterns\n\n\
                     Safety notes and best practices:\n\
                     - This tool ONLY modifies the Git index (staging area), never your working directory files\n\
                     - It never creates commits, pushes to remotes, or deletes any files\n\
                     - Safe to call multiple times on the same files (idempotent operation)\n\
                     - Files can still be modified after staging - you can re-stage them if needed\n\
                     - Use git_status before staging to review what changes exist\n\
                     - Use git_status after staging to verify what will be committed\n\n\
                     Git workflow context:\n\
                     The typical Git workflow is: git_status (review changes) → git_add (stage changes) → \
                     git_commit (create commit) → git_push (share changes). This tool handles the staging \
                     step. Use selective staging (specific paths) when you want to commit only certain \
                     changes, or use all=true to stage everything at once.",
                ),
            },
        ])
    }
}
