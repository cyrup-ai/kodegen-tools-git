//! Git push tool

use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse, McpError};
use kodegen_mcp_schema::git::{GitPushArgs, GitPushOutput, PushPrompts};
use std::path::Path;

/// Tool for pushing commits and tags to remote repositories
#[derive(Clone)]
pub struct GitPushTool;

impl Tool for GitPushTool {
    type Args = GitPushArgs;
    type Prompts = PushPrompts;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_PUSH
    }

    fn description() -> &'static str {
        "Push commits and/or tags to a remote repository. \
         Supports force push, selective refspecs, and all tags. \
         Requires proper authentication setup (SSH keys or credential helpers)."
    }

    fn read_only() -> bool {
        false // Modifies remote repository
    }

    fn destructive() -> bool {
        false // Only adds refs, not deletes (unless force pushing)
    }

    fn idempotent() -> bool {
        true // Safe to push same refs multiple times (no-op if already pushed)
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_schema::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository and execute push in a spawn_blocking context
        // to avoid Send issues with RepoHandle
        let remote = args.remote.clone();
        let refspecs = args.refspecs.clone();
        let force = args.force;
        let tags = args.tags;
        let timeout_secs = args.timeout_secs;
        let path_buf = path.to_path_buf();

        let result = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                // Open repository
                let repo = crate::open_repo(&path_buf)
                    .await
                    .map_err(|e| anyhow::anyhow!("Task execution failed: {e}"))?
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

                // Build push options
                let opts = crate::PushOpts {
                    remote,
                    refspecs,
                    force,
                    tags,
                    timeout_secs,
                };

                // Execute push
                crate::push(&repo, opts)
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))
            })
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(|e: anyhow::Error| McpError::Other(e))?;

        // Terminal summary
        let mut details = vec![
            format!("Remote: {}", args.remote),
            format!("Refs pushed: {}", result.commits_pushed),
        ];

        if result.tags_pushed > 0 {
            details.push(format!("Tags pushed: {}", result.tags_pushed));
        }

        if args.force {
            details.push("Force push: Yes".to_string());
        }

        if !result.warnings.is_empty() {
            details.push(format!("Warnings: {}", result.warnings.join("; ")));
        }

        let summary = format!(
            "✓ Push completed\n\n{}",
            details.join("\n")
        );

        Ok(ToolResponse::new(summary, GitPushOutput {
            success: true,
            remote: args.remote.clone(),
            refs_pushed: result.commits_pushed as u32,
            tags_pushed: result.tags_pushed as u32,
            force: args.force,
            warnings: result.warnings,
        }))
    }
}
