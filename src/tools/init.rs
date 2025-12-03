//! Git repository initialization tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitInitArgs, GitInitPromptArgs, GitInitOutput};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use std::path::Path;

/// Tool for initializing Git repositories
#[derive(Clone)]
pub struct GitInitTool;

impl Tool for GitInitTool {
    type Args = GitInitArgs;
    type PromptArgs = GitInitPromptArgs;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
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

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "repo_type".to_string(),
                title: None,
                description: Some(
                    "Repository type to focus on (e.g., 'normal', 'bare', or 'both')".to_string()
                ),
                required: Some(false),
            }
        ]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What does git_init do and when should I use it?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_init tool initializes a new Git repository at a specified path. \
                     It creates the version control infrastructure needed to start tracking changes. \
                     There are two types of repositories you can create:\n\n\
                     NORMAL REPOSITORIES (for local development):\n\
                     git_init({\"path\": \"/home/user/my-project\"})\n\n\
                     Creates a standard Git repository with:\n\
                     - A working directory where you edit files\n\
                     - A .git/ subdirectory containing version control metadata\n\
                     - Ready for local commits and branch operations\n\
                     - Typical workflow: commit locally, then push to remote\n\n\
                     BARE REPOSITORIES (for server/centralized setups):\n\
                     git_init({\"path\": \"/var/git/my-repo.git\", \"bare\": true})\n\n\
                     Creates a repository without a working directory:\n\
                     - No .git subdirectory (the repository itself IS the storage)\n\
                     - Cannot directly commit changes in this repository\n\
                     - Designed to be a centralized repository that others push to\n\
                     - Common pattern: use with git_remote_add on client repositories\n\
                     - Server-friendly: smaller footprint, used by hosting platforms\n\n\
                     KEY PARAMETERS:\n\
                     - path (required): Where to initialize the repository. Must not already exist.\n\
                     - bare (optional, default false): Set to true for server-style repositories\n\
                     - initial_branch (optional): Branch name for the repository (informational)\n\n\
                     IMPORTANT CONSTRAINTS:\n\
                     - The target path must NOT exist before calling git_init\n\
                     - After init, the repository is empty (no commits yet)\n\
                     - Normal repos need files to be added before first commit\n\n\
                     COMMON WORKFLOWS:\n\
                     1. Local project setup:\n\
                        a) git_init({\"path\": \"./myapp\"})\n\
                        b) Create files in ./myapp\n\
                        c) git_add to stage changes\n\
                        d) git_commit to create first commit\n\n\
                     2. Setting up a shared server repository:\n\
                        a) git_init({\"path\": \"/var/git/shared.git\", \"bare\": true})\n\
                        b) On client machines: git_clone from that path\n\
                        c) Team members push/pull from the bare repository\n\n\
                     3. Monorepo initialization:\n\
                        git_init({\"path\": \".\"}) in existing directory (if empty)\n\
                        then organize with git_worktree_add for multiple working trees\n\n\
                     GOTCHAS & BEST PRACTICES:\n\
                     - Error if path exists: must use empty directories only\n\
                     - After init: repository has no commits, so checkout operations need at least one commit\n\
                     - Bare repositories: typically use .git extension by convention\n\
                     - When to clone vs init: use git_clone for existing remote repos, git_init for new local projects\n\
                     - Integration: after init, use git_add and git_commit to create your first snapshot\n\n\
                     ALTERNATIVE APPROACHES:\n\
                     - If cloning from existing repo: use git_clone instead\n\
                     - If working with remote: use git_remote_add after init to connect to upstream\n\
                     - For temporary branches: consider git_worktree_add after repository initialization",
                ),
            },
        ])
    }
}
