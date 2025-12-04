// Category HTTP Server: Git Tools
//
// This binary serves git-related tools over HTTP/HTTPS transport.
// Managed by kodegend daemon, typically running on port 30450.

use anyhow::Result;
use kodegen_server_http::{run_http_server, Managers, RouterSet, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};

#[tokio::main]
async fn main() -> Result<()> {
    run_http_server("git", |_config, _tracker| {
        Box::pin(async move {
        let mut tool_router = ToolRouter::new();
        let mut prompt_router = PromptRouter::new();
        let managers = Managers::new();

        // Register all git tools (zero-state structs, no constructors)
        use kodegen_tools_git::*;

        // Repository initialization (4 tools)
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitInitTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitOpenTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitCloneTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitDiscoverTool);

        // Branch operations (4 tools)
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitBranchCreateTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitBranchDeleteTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitBranchListTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitBranchRenameTool);

        // Core git operations (9 tools)
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitCommitTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitLogTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitDiffTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitHistoryTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitAddTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitCheckoutTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitResetTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitStatusTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitStashTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitTagTool);

        // Remote operations (7 tools)
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitFetchTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitMergeTool);

        // Worktree operations (6 tools)
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeAddTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeRemoveTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeListTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeLockTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeUnlockTool);
        (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreePruneTool);

        Ok(RouterSet::new(tool_router, prompt_router, managers))
        })
    })
    .await
}
