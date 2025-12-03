//! Git remote list tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitRemoteListArgs, GitRemoteListPromptArgs, GitRemoteListOutput, GitRemoteInfo};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use std::path::Path;

/// Tool for listing remote repositories
#[derive(Clone)]
pub struct GitRemoteListTool;

impl Tool for GitRemoteListTool {
    type Args = GitRemoteListArgs;
    type PromptArgs = GitRemoteListPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_REMOTE_LIST
    }

    fn description() -> &'static str {
        "List all configured remote repositories. \
         Shows remote names and their fetch/push URLs."
    }

    fn read_only() -> bool {
        true // Only reads configuration
    }

    fn destructive() -> bool {
        false // No modifications
    }

    fn idempotent() -> bool {
        true // Safe to call multiple times
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository and list remotes in a spawn_blocking context
        // to avoid Send issues with RepoHandle
        let path_buf = path.to_path_buf();

        let remotes = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                // Open repository
                let repo = crate::open_repo(&path_buf)
                    .await
                    .map_err(|e| anyhow::anyhow!("Task execution failed: {e}"))?
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

                // List remotes
                crate::list_remotes(&repo)
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))
            })
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(|e: anyhow::Error| McpError::Other(e))?;

        // Terminal summary with ANSI colors and Nerd Font icons
        let mut summary = format!(
            "\x1b[34m Remotes ({})\x1b[0m",
            remotes.len()
        );

        if remotes.is_empty() {
            summary.push_str("\n  No remotes configured");
        } else {
            for remote in &remotes {
                let urls = if remote.fetch_url == remote.push_url {
                    remote.fetch_url.clone()
                } else {
                    format!("fetch: {} | push: {}", remote.fetch_url, remote.push_url)
                };
                summary.push_str(&format!("\n  {} -> {}", remote.name, urls));
            }
        }

        let remotes_output: Vec<GitRemoteInfo> = remotes
            .iter()
            .map(|r| GitRemoteInfo {
                name: r.name.clone(),
                fetch_url: r.fetch_url.clone(),
                push_url: r.push_url.clone(),
            })
            .collect();

        let count = remotes_output.len();

        Ok(ToolResponse::new(summary, GitRemoteListOutput {
            success: true,
            count,
            remotes: remotes_output,
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "detail_level".to_string(),
            title: None,
            description: Some(
                "Detail level for examples (e.g., 'basic', 'advanced')".to_string()
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use git_remote_list to see all remotes in a repository?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_remote_list tool enumerates all configured remote repositories in a Git repository. \
                     Here's how to use it:\n\n\
                     Basic usage:\n\
                     git_remote_list({\"path\": \"/path/to/repo\"})\n\n\
                     This returns:\n\
                     1. A human-readable summary showing all remotes with their URLs\n\
                     2. A JSON response with:\n\
                        - success: boolean indicating operation success\n\
                        - count: total number of configured remotes\n\
                        - remotes: array of objects containing name, fetch_url, and push_url\n\n\
                     Key behaviors:\n\
                     - Lists all configured remotes (origin, upstream, etc.)\n\
                     - Shows both fetch and push URLs (may differ for some workflows)\n\
                     - Consolidates display if fetch and push URLs are identical\n\
                     - Returns empty list if no remotes are configured\n\
                     - Works with any repository format (bare or working tree)\n\n\
                     Common use cases:\n\
                     - Discovering available remotes before push/pull operations\n\
                     - Validating remote configuration in CI/CD workflows\n\
                     - Monitoring multi-remote setups (e.g., origin, upstream, fork)\n\
                     - Automating workflows that depend on remote enumeration\n\
                     - Verifying correct fetch/push URL pairing for collaboration",
                ),
            },
        ])
    }
}
