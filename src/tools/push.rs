//! Git push tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitPushArgs, GitPushPromptArgs, GitPushOutput};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use std::path::Path;

/// Tool for pushing commits and tags to remote repositories
#[derive(Clone)]
pub struct GitPushTool;

impl Tool for GitPushTool {
    type Args = GitPushArgs;
    type PromptArgs = GitPushPromptArgs;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
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

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "scenario".to_string(),
                title: None,
                description: Some(
                    "Type of push scenario to focus on: 'basic', 'selective', 'force', or 'tags' \
                     (default shows all scenarios)"
                        .to_string(),
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use the git_push tool to push commits to a remote repository?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_push tool pushes commits and tags from your local repository to a remote. It's essential for sharing your work and collaborating with teams.\n\n\
                     CORE PARAMETERS (6 total):\n\
                     - path: Local repository path (must be a valid Git repo)\n\
                     - remote: Remote name (typically 'origin')\n\
                     - refspecs: Which refs to push (e.g., ['main', 'develop']). If empty, pushes current branch\n\
                     - force: Force overwrite remote refs (dangerous - see safety warnings below)\n\
                     - tags: Also push annotated/lightweight tags\n\
                     - timeout_secs: Network operation timeout in seconds\n\n\
                     BASIC USAGE EXAMPLES (5 scenarios):\n\n\
                     1. Push current branch to origin:\n\
                        git_push({\"path\": \"/repo\", \"remote\": \"origin\", \"refspecs\": [], \"force\": false, \"tags\": false, \"timeout_secs\": 30})\n\n\
                     2. Push specific branches:\n\
                        git_push({\"path\": \"/repo\", \"remote\": \"origin\", \"refspecs\": [\"main\", \"develop\"], \"force\": false, \"tags\": false, \"timeout_secs\": 30})\n\n\
                     3. Push with tags:\n\
                        git_push({\"path\": \"/repo\", \"remote\": \"origin\", \"refspecs\": [\"main\"], \"force\": false, \"tags\": true, \"timeout_secs\": 30})\n\n\
                     4. Push all branches:\n\
                        git_push({\"path\": \"/repo\", \"remote\": \"origin\", \"refspecs\": [\"main\", \"develop\", \"staging\"], \"force\": false, \"tags\": false, \"timeout_secs\": 30})\n\n\
                     5. Force push (dangerous - only for fixing mistakes):\n\
                        git_push({\"path\": \"/repo\", \"remote\": \"origin\", \"refspecs\": [\"main\"], \"force\": true, \"tags\": false, \"timeout_secs\": 30})\n\n\
                     AUTHENTICATION REQUIREMENTS:\n\
                     - SSH Keys: ~/.ssh/id_rsa with proper permissions (600)\n\
                     - Credential Helper: git config credential.helper store\n\
                     - Personal Access Token (GitHub/GitLab): Stored in credential helper or SSH\n\
                     - HTTPS URLs: Use credential helper or personal access tokens\n\
                     - SSH URLs: Use SSH keys or SSH agent\n\
                     - Test with: git ls-remote origin (checks authentication)\n\n\
                     CRITICAL SAFETY WARNINGS:\n\
                     WARNING 1: Force push (+) overwrites remote history - use only on personal branches!\n\
                     WARNING 2: Never force push to protected branches (main, production, release)\n\
                     WARNING 3: Force push on shared branches will corrupt teammates' work\n\
                     WARNING 4: Always communicate with team before forcing any public push\n\
                     WARNING 5: Use branch protection rules to prevent accidental force pushes",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What should I know about refspecs and selective pushing?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Refspecs are the Git references (branches/tags) you want to push. They control exactly what gets sent to the remote.\n\n\
                     REFSPEC EXAMPLES (5+ scenarios):\n\n\
                     1. Empty refspecs - push current branch:\n\
                        refspecs: []\n\
                        Result: Pushes whatever branch you're currently on\n\
                        Use case: Simple local development, default behavior\n\n\
                     2. Single branch:\n\
                        refspecs: [\"main\"]\n\
                        Result: Pushes only main branch, doesn't affect others\n\
                        Use case: Careful selective pushes, CI/CD workflows\n\n\
                     3. Multiple branches:\n\
                        refspecs: [\"main\", \"develop\", \"staging\"]\n\
                        Result: Pushes all three branches in single operation\n\
                        Use case: Coordinated multi-branch releases\n\n\
                     4. Tag references:\n\
                        refspecs: [\"refs/tags/v1.0.0\"]\n\
                        Combined with tags: true parameter\n\
                        Result: Pushes specific tag versions\n\
                        Use case: Release management, semantic versioning\n\n\
                     5. Feature branch development:\n\
                        refspecs: [\"feature/new-auth\", \"feature/api-v2\", \"main\"]\n\
                        Result: Push multiple features alongside main\n\
                        Use case: Feature branch workflows, parallel development\n\n\
                     Best practice: Always specify explicit refspecs in scripts/CI to avoid accidental pushes",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What happens if the push fails? How do I handle rejected refs?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Push failures are common when working with teams. Here's how to diagnose and recover.\n\n\
                     ERROR SCENARIO 1 - Non-Fast-Forward (most common):\n\
                     Error: \"failed to push some refs to 'origin'\"\n\
                     Cause: Remote has commits you don't have locally. Push would lose history.\n\
                     Fix: Pull first, then push\n\
                        1. git_pull(...) // Merge remote changes into your branch\n\
                        2. Resolve any merge conflicts\n\
                        3. git_push(...) // Now push succeeds\n\n\
                     ERROR SCENARIO 2 - Authentication Failure:\n\
                     Error: \"fatal: Authentication failed\", \"permission denied (publickey)\"\n\
                     Cause: No SSH keys, bad credentials, or missing credential helper\n\
                     Fix: Setup authentication\n\
                        1. SSH: ssh-keygen -t ed25519 (generate key)\n\
                        2. Add public key to GitHub/GitLab settings\n\
                        3. Test: ssh -T git@github.com\n\
                        4. For HTTPS: git config credential.helper store\n\n\
                     ERROR SCENARIO 3 - Protected Branch (push denied):\n\
                     Error: \"You do not have permission to update the protected ref\"\n\
                     Cause: Branch is protected and requires pull requests\n\
                     Fix: Use pull request instead\n\
                        1. Push to feature branch (not protected)\n\
                        2. Create pull request on remote service\n\
                        3. Get approval and merge through PR\n\n\
                     ERROR SCENARIO 4 - Connection Timeout:\n\
                     Error: \"Operation timed out\", \"unable to access repository\"\n\
                     Cause: Network issue, remote server slow, firewall blocking\n\
                     Fix: Retry with longer timeout\n\
                        1. Increase timeout_secs parameter: timeout_secs: 60 (was 30)\n\
                        2. Check network connectivity: ping github.com\n\
                        3. Retry push with new timeout\n\n\
                     GENERAL RECOVERY STRATEGY:\n\
                     1. Diagnose: Read error message carefully\n\
                     2. Fetch: git fetch to see remote state\n\
                     3. Compare: git log to see what's different\n\
                     4. Fix: Follow appropriate scenario above\n\
                     5. Retry: Attempt git_push(...) again",
                ),
            },
        ])
    }
}
