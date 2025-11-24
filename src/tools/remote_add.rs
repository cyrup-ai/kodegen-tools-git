//! Git remote add tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitRemoteAddArgs, GitRemoteAddPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content, PromptMessageRole, PromptMessageContent};
use serde_json::json;
use std::path::Path;

/// Tool for adding remote repositories
#[derive(Clone)]
pub struct GitRemoteAddTool;

impl Tool for GitRemoteAddTool {
    type Args = GitRemoteAddArgs;
    type PromptArgs = GitRemoteAddPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_REMOTE_ADD
    }

    fn description() -> &'static str {
        "Add a new remote repository. \
         Configures a named remote with fetch/push URLs for collaboration."
    }

    fn read_only() -> bool {
        false // Modifies repository configuration
    }

    fn destructive() -> bool {
        false // Non-destructive operation
    }

    fn idempotent() -> bool {
        true // Safe to add same remote multiple times
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build add options
        let opts = crate::RemoteAddOpts {
            name: args.name.clone(),
            url: args.url.clone(),
            force: args.force,
        };

        // Execute add
        crate::add_remote(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let summary = format!(
            "\x1b[32m Add Remote\x1b[0m\n  ✓ {} ➜ {}",
            args.name, args.url
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "remote_name": args.name,
            "remote_url": args.url
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "workflow_type".to_string(),
                title: None,
                description: Some(
                    "Customize examples to a specific workflow: 'simple' (basic origin setup), \
                     'multi_remote' (origin + upstream pattern), or 'advanced' (multiple remote URLs, SSH keys)".to_string()
                ),
                required: Some(false),
            },
            PromptArgument {
                name: "url_format".to_string(),
                title: None,
                description: Some(
                    "Focus examples on specific URL format: 'https' (HTTPS URLs), \
                     'ssh' (SSH URLs), 'git' (git:// protocol), or 'all' (mixed examples)".to_string()
                ),
                required: Some(false),
            },
        ]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            // ========================================
            // QUESTION 1: Basic Understanding
            // ========================================
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use the git_remote_add tool to configure remote repositories?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_remote_add tool configures named remote repositories in Git. \
                     Remote repositories enable collaboration and synchronization across different locations.\n\n\
                     BASIC USAGE:\n\
                     git_remote_add({\n  \
                       \"path\": \"/path/to/repo\",\n  \
                       \"name\": \"origin\",\n  \
                       \"url\": \"https://github.com/user/repo.git\"\n\
                     })\n\n\
                     KEY CONCEPTS:\n\
                     • Remote Name: Identifier for the repository (\"origin\" is convention for primary remote)\n\
                     • Remote URL: Address where repository code is stored (HTTPS, SSH, git protocol, or file://)\n\
                     • Idempotent: Safe to call multiple times with same arguments\n\
                     • Non-Destructive: Adding remotes never deletes code",
                ),
            },

            // ========================================
            // QUESTION 2: URL Formats
            // ========================================
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What URL formats does git_remote_add support, and when should I use each?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The tool supports four URL formats:\n\n\
                     1. HTTPS (Recommended for Public Repositories):\n  \
                     git_remote_add({\"path\": \".\", \"name\": \"origin\", \
                     \"url\": \"https://github.com/user/repo.git\"})\n  \
                     • Advantages: Works through firewalls, no SSH key setup needed\n  \
                     • Disadvantages: Requires password/token authentication per push\n  \
                     • Use Case: CI/CD, shared environments, teams without SSH infrastructure\n\n\
                     2. SSH (Recommended for Private Repositories + Developers):\n  \
                     git_remote_add({\"path\": \".\", \"name\": \"origin\", \
                     \"url\": \"git@github.com:user/repo.git\"})\n  \
                     • Advantages: Automated authentication via SSH keys, no token management\n  \
                     • Disadvantages: Requires SSH key setup, may not work through restrictive firewalls\n  \
                     • Use Case: Developer machines, private repositories, automated deploys with key-based auth\n\n\
                     3. Git Protocol (Legacy):\n  \
                     git_remote_add({\"path\": \".\", \"name\": \"origin\", \
                     \"url\": \"git://github.com/user/repo.git\"})\n  \
                     • Advantages: Fast, read-only access\n  \
                     • Disadvantages: Deprecated by GitHub and most providers, no authentication\n  \
                     • Use Case: Read-only mirrors, legacy systems only\n\n\
                     4. File Protocol (Local Testing):\n  \
                     git_remote_add({\"path\": \".\", \"name\": \"local_backup\", \
                     \"url\": \"file:///mnt/backup/repo.git\"})\n  \
                     • Advantages: Fast local access, useful for backups and testing\n  \
                     • Disadvantages: Only works on same machine/mounted storage\n  \
                     • Use Case: Local backups, testing multi-repo workflows",
                ),
            },

            // ========================================
            // QUESTION 3: Remote Naming Conventions
            // ========================================
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What are the best practices for naming remotes? Are there standard naming conventions?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Yes! Standard Git remote naming conventions exist:\n\n\
                     STANDARD REMOTES:\n\
                     • origin: Your primary repository (where you push by default)\n  \
                       git_remote_add({\"path\": \".\", \"name\": \"origin\", \
                       \"url\": \"https://github.com/user/repo.git\"})\n\
                     • upstream: The authoritative upstream repository (typically in open-source)\n  \
                       git_remote_add({\"path\": \".\", \"name\": \"upstream\", \
                       \"url\": \"https://github.com/upstream-owner/repo.git\"})\n\n\
                     COMMON ADDITIONAL REMOTES:\n\
                     • backup: Off-site or local backup repository\n  \
                       git_remote_add({\"path\": \".\", \"name\": \"backup\", \
                       \"url\": \"file:///mnt/backup/repo.git\"})\n\
                     • staging: Deployment staging environment\n  \
                       git_remote_add({\"path\": \".\", \"name\": \"staging\", \
                       \"url\": \"git@staging.example.com:repo.git\"})\n\
                     • mirror: Read-only mirror for distribution\n  \
                       git_remote_add({\"path\": \".\", \"name\": \"mirror\", \
                       \"url\": \"https://mirror.example.com/repo.git\"})\n\n\
                     NAMING RULES:\n\
                     ✓ Use lowercase, hyphens for multi-word names (e.g., \"github-mirror\")\n\
                     ✓ Keep names short but descriptive\n\
                     ✓ Avoid spaces and special characters\n\
                     ✓ Use consistent names across team repositories",
                ),
            },

            // ========================================
            // QUESTION 4: Multi-Remote Workflows
            // ========================================
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I set up the origin + upstream pattern for contributing to open-source projects?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The origin + upstream pattern is essential for open-source contributions:\n\n\
                     WORKFLOW:\n\n\
                     1. Fork the upstream repository on GitHub (creates user/repo.git)\n\n\
                     2. Clone your fork locally:\n  \
                     git clone https://github.com/YOUR-USERNAME/repo.git\n\n\
                     3. Add upstream remote:\n  \
                     git_remote_add({\n  \
                       \"path\": \".\",\n  \
                       \"name\": \"upstream\",\n  \
                       \"url\": \"https://github.com/upstream-owner/repo.git\"\n\
                     })\n\n\
                     4. Verify both remotes exist:\n  \
                     origin  ➜ https://github.com/YOUR-USERNAME/repo.git (your fork)\n  \
                     upstream ➜ https://github.com/upstream-owner/repo.git (official)\n\n\
                     RATIONALE:\n\
                     • origin: Your fork where you push your changes\n\
                     • upstream: Official repository where you pull latest changes from\n\n\
                     TYPICAL WORKFLOW:\n\
                     git fetch upstream        # Get latest changes from official repo\n\
                     git rebase upstream/main  # Rebase your work on latest upstream\n\
                     git push origin main      # Push rebased work to your fork\n\
                     # Create pull request on GitHub from your fork to upstream",
                ),
            },

            // ========================================
            // QUESTION 5: The Force Flag and Gotchas
            // ========================================
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "When should I use the force flag? What happens if I add a remote with an existing name?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The force flag controls behavior when a remote with the same name already exists:\n\n\
                     SCENARIO 1: Adding remote that doesn't exist (force = false/omitted):\n  \
                     git_remote_add({\"path\": \".\", \"name\": \"origin\", \
                     \"url\": \"https://github.com/user/repo.git\"})\n  \
                     ✓ SUCCESS: Remote created\n\n\
                     SCENARIO 2: Remote exists, force = false (Default):\n  \
                     git_remote_add({\"path\": \".\", \"name\": \"origin\", \
                     \"url\": \"https://github.com/different-url/repo.git\"})\n  \
                     ✗ ERROR: Remote 'origin' already exists\n\n\
                     SCENARIO 3: Need to change existing remote URL:\n  \
                     git_remote_add({\n  \
                       \"path\": \".\",\n  \
                       \"name\": \"origin\",\n  \
                       \"url\": \"https://github.com/new-url/repo.git\",\n  \
                       \"force\": true\n  \
                     })\n  \
                     ✓ SUCCESS: Remote URL replaced\n\n\
                     IMPORTANT GOTCHAS:\n\
                     • Changing origin URL after cloning:\n  \
                       Idempotent means calling with same args is safe,\n  \
                       but changing URLs requires force=true\n\n\
                     • force=true overwrites silently:\n  \
                       Previous URL is lost without warning\n  \
                       Document remote changes in commit messages or team communication\n\n\
                     • Verify URL after update:\n  \
                       Always verify the new URL points to correct repository\n  \
                       Use 'git remote -v' to display all remotes and verify",
                ),
            },

            // ========================================
            // QUESTION 6: Error Cases
            // ========================================
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What errors can occur when using git_remote_add, and how do I handle them?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Common error scenarios and solutions:\n\n\
                     ERROR 1: Invalid repository path\n  \
                     Cause: Path doesn't point to valid Git repository\n  \
                     Solution: Verify path exists and contains .git directory\n\n\
                     ERROR 2: Remote with name already exists\n  \
                     Cause: Attempted to add remote with existing name, force=false\n  \
                     Example: git_remote_add({\"path\": \".\", \"name\": \"origin\", \
                     \"url\": \"...\"}) when origin already exists\n  \
                     Solution: Use force=true to overwrite OR use different name\n\n\
                     ERROR 3: Invalid URL format\n  \
                     Cause: URL syntax is malformed\n  \
                     Example: \"htp://github.com/user/repo.git\" (typo in http)\n  \
                     Solution: Verify URL uses correct protocol and format\n\n\
                     ERROR 4: Network unreachable (URL validation)\n  \
                     Cause: URL points to inaccessible server (May be deferred to git pull/fetch)\n  \
                     Solution: Verify network connection, URL correctness\n\n\
                     BEST PRACTICE ERROR HANDLING:\n  \
                     • Always verify tool output confirms remote was added\n  \
                     • Use git_remote_list to verify after adding\n  \
                     • Test fetch/push to ensure connectivity\n  \
                     • Document remote purpose in repository documentation",
                ),
            },
        ])
    }
}
