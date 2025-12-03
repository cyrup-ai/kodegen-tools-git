//! Git reset tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, ToolResponse, error::McpError};
use kodegen_mcp_schema::git::{GitResetArgs, GitResetPromptArgs, GitResetOutput, ResetMode};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use std::path::Path;

/// Tool for resetting repository to a specific commit
#[derive(Clone)]
pub struct GitResetTool;

impl Tool for GitResetTool {
    type Args = GitResetArgs;
    type PromptArgs = GitResetPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_RESET
    }

    fn description() -> &'static str {
        "Reset repository to a specific commit. \
         Soft: move HEAD only. Mixed: move HEAD and reset index. \
         Hard: move HEAD, reset index, and working directory."
    }

    fn read_only() -> bool {
        false // Modifies repository state
    }

    fn destructive() -> bool {
        true // Can discard local changes (especially hard mode)
    }

    fn idempotent() -> bool {
        true // Safe to reset to same target multiple times
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<<Self::Args as kodegen_mcp_tool::ToolArgs>::Output>, McpError> {
        let path = Path::new(&args.path);

        // Open repository and execute reset in a spawn_blocking context
        // to avoid Send issues with RepoHandle
        let mode = args.mode;
        let target_for_output = args.target.clone();
        let target = args.target;
        let path_buf = path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                // Open repository
                let repo = crate::open_repo(&path_buf)
                    .await
                    .map_err(|e| anyhow::anyhow!("Task execution failed: {e}"))?
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

                // Map schema ResetMode to operation ResetMode
                let op_mode = match mode {
                    ResetMode::Soft => crate::ResetMode::Soft,
                    ResetMode::Mixed => crate::ResetMode::Mixed,
                    ResetMode::Hard => crate::ResetMode::Hard,
                };

                // Build reset options
                let opts = crate::ResetOpts {
                    target,
                    mode: op_mode,
                    cancel_token: None,
                };

                // Execute reset
                crate::reset(&repo, opts)
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))
            })
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(|e: anyhow::Error| McpError::Other(e))?;

        // Terminal summary with ANSI colors and Nerd Font icons
        let mode_str = match mode {
            ResetMode::Soft => "soft",
            ResetMode::Mixed => "mixed",
            ResetMode::Hard => "hard",
        };

        let summary = format!(
            "\x1b[33m ⟲ Reset Complete\x1b[0m\n\
             ℹ Mode: {} · Target: {}",
            mode_str, target_for_output
        );

        Ok(ToolResponse::new(summary, GitResetOutput {
            success: true,
            mode: mode_str.to_string(),
            target: target_for_output,
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
                    "What are the differences between soft, mixed, and hard git reset, and when should I use each?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Git reset has three modes that affect different parts of your repository state:\n\n\
                     SOFT RESET (--soft)\n\
                     - Moves HEAD to target commit\n\
                     - Leaves staging area (index) unchanged\n\
                     - Leaves working directory unchanged\n\
                     - Use case: Undo commits but keep changes staged for re-committing\n\
                     - Example: git_reset({\"path\": \".\", \"target\": \"HEAD~1\", \"mode\": \"soft\"})\n\n\
                     MIXED RESET (--mixed, default)\n\
                     - Moves HEAD to target commit\n\
                     - Resets staging area to match target\n\
                     - Leaves working directory unchanged\n\
                     - Use case: Unstage changes or undo last commit while keeping modifications\n\
                     - Example: git_reset({\"path\": \".\", \"target\": \"HEAD~2\", \"mode\": \"mixed\"})\n\n\
                     HARD RESET (--hard)\n\
                     - Moves HEAD to target commit\n\
                     - Resets staging area to match target\n\
                     - Resets working directory to match target (DESTRUCTIVE)\n\
                     - Use case: Complete checkout to specific state, discarding all changes\n\
                     - Example: git_reset({\"path\": \".\", \"target\": \"origin/main\", \"mode\": \"hard\"})\n\n\
                     IMPORTANT SAFETY CONSIDERATIONS:\n\
                     - Hard reset discards uncommitted work permanently\n\
                     - Use reflog (git reflog) to recover from hard resets\n\
                     - Soft/mixed reset are safer for local-only branches\n\
                     - Never hard reset branches shared with others\n\
                     - Consider backing up uncommitted changes before hard reset\n\n\
                     TARGET SPECIFICATION:\n\
                     - Commit hash: \"abc1234\" or full \"abc1234567890...\"\n\
                     - Relative reference: \"HEAD~1\", \"HEAD~5\", \"HEAD^2\"\n\
                     - Branch name: \"main\", \"develop\", \"origin/feature\"\n\
                     - Tag name: \"v1.0.0\"\n\n\
                     COMMON WORKFLOWS:\n\
                     1. Fix last commit (soft): soft reset HEAD~1, then recommit\n\
                     2. Unstage accidental files (mixed): mixed reset to current HEAD\n\
                     3. Discard all changes (hard): hard reset to origin/main\n\
                     4. Undo local commits (soft): soft reset to upstream branch",
                ),
            },
        ])
    }
}
