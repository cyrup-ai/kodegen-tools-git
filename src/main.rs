// Category HTTP Server: Git Tools
//
// This binary serves git-related tools over HTTP/HTTPS transport.
// Managed by kodegend daemon, typically running on port kodegen_config::PORT_GIT (30444).

use anyhow::Result;
use kodegen_config::CATEGORY_GIT;
use kodegen_server_http::{ServerBuilder, Managers, RouterSet, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};

#[tokio::main]
async fn main() -> Result<()> {
    ServerBuilder::new()
        .category(CATEGORY_GIT)
        .register_tools(|| async {
            let tool_router = ToolRouter::new();
            let prompt_router = PromptRouter::new();
            let managers = Managers::new();

            // Register all git tools (zero-state structs, no constructors)
            use kodegen_tools_git::*;

            // Repository initialization (4 tools)
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitInitTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitOpenTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitCloneTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitDiscoverTool);

            // Branch operations (4 tools)
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitBranchCreateTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitBranchDeleteTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitBranchListTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitBranchRenameTool);

            // Core git operations (9 tools)
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitCommitTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitLogTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitDiffTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitHistoryTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitAddTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitCheckoutTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitResetTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitStatusTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitStashTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitTagTool);

            // Remote operations (7 tools)
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitFetchTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitMergeTool);

            // Worktree operations (6 tools)
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeAddTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeRemoveTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeListTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeLockTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeUnlockTool);
            let (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreePruneTool);

            Ok(RouterSet::new(tool_router, prompt_router, managers))
        })
        .run()
        .await
}
