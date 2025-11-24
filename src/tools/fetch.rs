//! Git fetch tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitFetchArgs, GitFetchPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole, Content};
use serde_json::json;
use std::path::Path;

/// Tool for fetching from remote repositories
#[derive(Clone)]
pub struct GitFetchTool;

impl Tool for GitFetchTool {
    type Args = GitFetchArgs;
    type PromptArgs = GitFetchPromptArgs;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
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

        let mut contents = Vec::new();

        // Terminal summary (2 lines with ANSI formatting)
        let prune_status = if args.prune { "yes" } else { "no" };

        let summary = format!(
            "\x1b[36m󰇚 Fetch: {}\x1b[0m\n 󰗚 Refs updated: synced · Prune: {}",
            args.remote, prune_status
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "remote": args.remote,
            "pruned": args.prune
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "fetch_scenario".to_string(),
            title: None,
            description: Some(
                "Fetch use case to focus on (e.g., 'basic_fetch', 'prune_deleted', 'multiple_remotes', 'tracking_branches')".to_string()
            ),
            required: Some(false),
        }]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use git_fetch? What does it do and when should I use it?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_fetch tool downloads objects and refs from a remote repository without modifying your \
                     working directory or current branch. It updates your local tracking branches (e.g., origin/main) \
                     to reflect the remote repository's state.\n\n\
                     What fetch does:\n\
                     - Downloads commits, objects, and references from a remote\n\
                     - Updates remote-tracking branches (origin/*, upstream/*)\n\
                     - Does NOT modify your working directory or local branches\n\
                     - Safe to run multiple times (idempotent)\n\n\
                     When to use git_fetch:\n\
                     1. Before pulling to see if there are new changes: git_fetch({\"path\": \".\", \"remote\": \"origin\"})\n\
                     2. To update all remote-tracking branches: git_fetch({\"path\": \".\", \"remote\": \"origin\"})\n\
                     3. To clean up deleted remote branches: git_fetch({\"path\": \".\", \"remote\": \"origin\", \"prune\": true})\n\
                     4. To fetch from a specific remote: git_fetch({\"path\": \".\", \"remote\": \"upstream\"})\n\n\
                     Key workflow:\n\
                     - Fetch first (download latest) → then pull/merge (integrate into your branch)\n\
                     - Fetch + Prune → clean up stale remote-tracking branches\n\n\
                     Parameters explained:\n\
                     - path: Repository directory (required)\n\
                     - remote: Remote name to fetch from, default \"origin\" (required)\n\
                     - refspecs: Custom ref specifications to fetch (optional, uses configured defaults if empty)\n\
                     - prune: Delete remote-tracking branches that don't exist on remote (optional, default false)\n\n\
                     Common use cases:\n\
                     1. Standard fetch: git_fetch({\"path\": \".\", \"remote\": \"origin\"})\n\
                        Updates all tracking branches to match remote state\n\n\
                     2. Fetch and prune: git_fetch({\"path\": \".\", \"remote\": \"origin\", \"prune\": true})\n\
                        Fetches and removes local origin/* branches for deleted remote branches\n\n\
                     3. Multiple remotes: First fetch(origin), then fetch(upstream) for syncing fork\n\
                        git_fetch({\"path\": \".\", \"remote\": \"origin\"})\n\
                        git_fetch({\"path\": \".\", \"remote\": \"upstream\"})\n\n\
                     4. Custom refspecs: git_fetch({\"path\": \".\", \"remote\": \"origin\", \
                        \"refspecs\": [\"refs/heads/main:refs/remotes/origin/main\"]})\n\
                        Fetch specific branches explicitly\n\n\
                     Important notes:\n\
                     - Fetch is NON-DESTRUCTIVE: it only adds data, never deletes local work\n\
                     - Safe to run: Multiple fetches don't cause conflicts\n\
                     - Doesn't auto-merge: Use git_pull after fetch to integrate changes\n\
                     - Prune is optional: Only use when you want to clean up deleted remote branches\n\
                     - Works offline for local queries but requires network for remote communication",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How is fetch different from pull? What's the advantage of fetch?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Fetch vs Pull - Key Differences:\n\n\
                     FETCH (Download only):\n\
                     - Only downloads objects and updates remote-tracking branches\n\
                     - Does not merge or modify your current branch\n\
                     - Safe to do anytime without affecting your work\n\
                     - Example: git_fetch({\"path\": \".\", \"remote\": \"origin\"})\n\n\
                     PULL (Download + Merge):\n\
                     - Combines fetch + merge in one operation\n\
                     - Modifies your current branch (integrates changes)\n\
                     - Can cause conflicts that need resolution\n\
                     - More direct but less control\n\n\
                     Advantages of explicit FETCH:\n\
                     1. Inspect changes BEFORE merging: Review origin/main before merging to your main\n\
                     2. Control over merge strategy: Fetch first, then choose merge vs rebase\n\
                     3. Avoid conflicts: Check remote changes before deciding how to integrate\n\
                     4. Non-destructive: Never affects your local branches, safe to use anytime\n\
                     5. Better for teams: Synchronized fetches before coordinated pulls\n\
                     6. Multiple remotes: Fetch from upstream/origin, then decide which to merge\n\n\
                     Best Practice Workflow:\n\
                     1. git_fetch({\"path\": \".\", \"remote\": \"origin\"}) - Download latest\n\
                     2. Review origin/main vs your main locally\n\
                     3. Decide: merge, rebase, or cherry-pick specific commits\n\
                     4. Then integrate using git_merge or git_pull as appropriate",
                ),
            },
        ])
    }
}
