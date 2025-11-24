//! Git repository opening tool

use gix::bstr::ByteSlice;
use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitOpenArgs, GitOpenPromptArgs};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;
use std::path::Path;

/// Tool for opening existing Git repositories
#[derive(Clone)]
pub struct GitOpenTool;

impl Tool for GitOpenTool {
    type Args = GitOpenArgs;
    type PromptArgs = GitOpenPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_OPEN
    }

    fn description() -> &'static str {
        "Open an existing Git repository at the specified path. \
         The repository must already exist at the given location."
    }

    fn read_only() -> bool {
        true // Only reads, doesn't modify
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true // Opening same repo multiple times is safe
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Get current branch information - fully inlined to avoid Send issues
        let repo_for_branch = repo.clone();
        let branch_name = tokio::task::spawn_blocking(move || {
            let inner = repo_for_branch.clone_inner();

            let head = inner.head().map_err(|e| anyhow::anyhow!("Failed to get HEAD: {e}"))?;

            let branch = head
                .referent_name()
                .and_then(|name| {
                    name.shorten()
                        .to_str()
                        .ok()
                        .map(std::string::ToString::to_string)
                })
                .unwrap_or_else(|| "detached HEAD".to_string());

            Ok::<_, anyhow::Error>(branch)
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(McpError::Other)?;

        // Get is_clean status - inline to avoid Send issues
        let repo_for_clean = repo.clone();
        let is_clean = tokio::task::spawn_blocking(move || {
            let inner = repo_for_clean.clone_inner();
            inner.is_dirty().map(|dirty| !dirty)
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to check clean status: {e}")))?;

        let mut contents = Vec::new();

        // Terminal summary with ANSI colors and Nerd Font icons
        let status = if is_clean { "clean" } else { "dirty" };
        let summary = format!(
            "\x1b[36m Open Repository: {}\x1b[0m\n\
              Branch: {} · Status: {}",
            args.path,
            branch_name,
            status
        );
        contents.push(Content::text(summary));

        // JSON metadata
        let metadata = json!({
            "success": true,
            "path": args.path,
            "branch": branch_name,
            "is_clean": is_clean,
            "message": format!("Opened Git repository at {}", args.path)
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to serialize metadata: {e}")))?;
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![PromptArgument {
            name: "use_case".to_string(),
            title: None,
            description: Some(
                "Optional focus area: 'basic' for fundamental usage, 'workflows' for \
                 common development patterns, 'troubleshooting' for diagnosing repo issues, \
                 'ci_integration' for continuous integration contexts (default: 'basic')"
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
                    "What does the git_open tool do and when should I use it?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_open tool inspects and opens an existing Git repository, \
                     returning its current state. Use it to:\n\n\
                     1. Verify you're working with the correct repository\n\
                     2. Check which branch is currently checked out\n\
                     3. Determine if the working directory is clean (no uncommitted changes)\n\
                     4. Get repository metadata at the start of a workflow\n\n\
                     Example usage: git_open({\"path\": \"/path/to/repo\"})\n\n\
                     This returns:\n\
                     - Current branch name (or \"detached HEAD\" if in detached state)\n\
                     - Working directory status (clean or dirty)\n\
                     - Terminal summary with ANSI colors\n\
                     - JSON metadata for programmatic access"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I interpret the output? What does 'clean' vs 'dirty' mean?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The git_open tool returns two key pieces of information:\n\n\
                     1. BRANCH NAME:\n\
                        - Normal output: \"main\", \"feature/new-feature\", etc.\n\
                        - Detached HEAD: Shows \"detached HEAD\" when HEAD doesn't point to a branch\n\n\
                     2. WORKING DIRECTORY STATUS:\n\
                        - \"clean\": All changes are committed, working directory matches HEAD\n\
                        - \"dirty\": There are uncommitted changes (modified files, untracked files, etc.)\n\n\
                     Example output:\n\
                     Terminal: \"Open Repository: /path/to/repo\\nBranch: main · Status: clean\"\n\n\
                     JSON metadata:\n\
                     {\n\
                       \"success\": true,\n\
                       \"path\": \"/path/to/repo\",\n\
                       \"branch\": \"main\",\n\
                       \"is_clean\": true,\n\
                       \"message\": \"Opened Git repository at /path/to/repo\"\n\
                     }\n\n\
                     Use is_clean to conditionally proceed with operations that require a clean state."
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What are common patterns for using git_open in a workflow?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Common workflow patterns:\n\n\
                     1. REPOSITORY VERIFICATION (START OF TASK):\n\
                        - Call git_open first to confirm working with the correct repository\n\
                        - Check if you're on the right branch before proceeding\n\n\
                     2. PRE-COMMIT VALIDATION:\n\
                        - Use git_open to check is_clean before pulling or merging\n\
                        - Abort operations if working directory is dirty\n\n\
                     3. BRANCH DETECTION:\n\
                        - Use the branch field to conditionally execute branch-specific operations\n\
                        - Example: Only pull if on main branch\n\n\
                     4. CI/CD INTEGRATION:\n\
                        - Call git_open in CI pipelines to verify repository state\n\
                        - Log branch and status for audit trails\n\n\
                     5. MULTI-REPO OPERATIONS:\n\
                        - When working with multiple repositories, use git_open on each\n\
                        - to ensure correct context before making changes"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "What happens if the path doesn't exist or isn't a Git repository?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Error handling:\n\n\
                     1. NON-EXISTENT PATH:\n\
                        - Returns McpError with message about directory not existing\n\
                        - The operation fails immediately\n\n\
                     2. NON-GIT DIRECTORY:\n\
                        - Returns McpError if the path exists but isn't a Git repository\n\
                        - Error message indicates the path is not a valid git repository\n\n\
                     3. PERMISSION DENIED:\n\
                        - Returns McpError if you lack permissions to read the directory\n\
                        - Check file permissions and re-try with appropriate access\n\n\
                     BEST PRACTICE: Always handle errors from git_open gracefully:\n\
                     - Check if the path exists first (using filesystem tools if needed)\n\
                     - Provide clear error messages to the user\n\
                     - Consider suggesting the correct repository path"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How does git_open integrate with other Git operations?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Integration patterns with other git tools:\n\n\
                     1. WITH GIT_CHECKOUT:\n\
                        - Use git_open to verify initial branch state\n\
                        - After git_checkout, use git_open again to confirm the switch\n\n\
                     2. WITH GIT_COMMIT:\n\
                        - Call git_open first to ensure you're on the right branch\n\
                        - After committing, git_open shows is_clean=true if no staged changes remain\n\n\
                     3. WITH GIT_PULL/GIT_PUSH:\n\
                        - Use git_open to check is_clean before pulling (avoids conflicts)\n\
                        - Use git_open after pushing to verify repository state\n\n\
                     4. WITH GIT_STATUS:\n\
                        - git_open gives a quick summary (branch + clean status)\n\
                        - git_status provides detailed change information\n\
                        - Use git_open for quick checks, git_status for detailed analysis\n\n\
                     5. WITH GIT_DISCOVER:\n\
                        - Use git_discover to find the repository root\n\
                        - Then use git_open on that root path\n\n\
                     WORKFLOW TIP: Use git_open as a checkpoint between major operations \
                     to ensure repository state is as expected."
                ),
            },
        ])
    }
}
