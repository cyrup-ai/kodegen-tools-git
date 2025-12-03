//! MCP Tools for Git operations
//!
//! This module provides Model Context Protocol (MCP) tool wrappers around
//! the core Git operations for use in AI agent systems.

// Repository Operations
pub mod clone;
pub mod discover;
pub mod init;
pub mod open;

// Branch Operations
pub mod branch_create;
pub mod branch_delete;
pub mod branch_list;
pub mod branch_rename;

// Commit & Staging Operations
pub mod add;
pub mod checkout;
pub mod commit;
pub mod diff;
pub mod history;
pub mod log;

// Remote Operations
pub mod fetch;
pub mod merge;
pub mod pull;
pub mod push;
pub mod remote_add;
pub mod remote_list;
pub mod remote_remove;
pub mod reset;

// Worktree Operations
pub mod worktree_add;
pub mod worktree_list;
pub mod worktree_lock;
pub mod worktree_prune;
pub mod worktree_remove;
pub mod worktree_unlock;

// Status Operations
pub mod status;

// Stash Operations
pub mod stash;

// Tag Operations
pub mod tag;

// Re-export tools
pub use clone::GitCloneTool;
pub use discover::GitDiscoverTool;
pub use init::GitInitTool;
pub use open::GitOpenTool;

pub use branch_create::GitBranchCreateTool;
pub use branch_delete::GitBranchDeleteTool;
pub use branch_list::GitBranchListTool;
pub use branch_rename::GitBranchRenameTool;

pub use add::GitAddTool;
pub use checkout::GitCheckoutTool;
pub use commit::GitCommitTool;
pub use diff::GitDiffTool;
pub use history::GitHistoryTool;
pub use log::GitLogTool;

pub use fetch::GitFetchTool;
pub use merge::GitMergeTool;
pub use pull::GitPullTool;
pub use push::GitPushTool;
pub use remote_add::GitRemoteAddTool;
pub use remote_list::GitRemoteListTool;
pub use remote_remove::GitRemoteRemoveTool;
pub use reset::GitResetTool;

pub use worktree_add::GitWorktreeAddTool;
pub use worktree_list::GitWorktreeListTool;
pub use worktree_lock::GitWorktreeLockTool;
pub use worktree_prune::GitWorktreePruneTool;
pub use worktree_remove::GitWorktreeRemoveTool;
pub use worktree_unlock::GitWorktreeUnlockTool;

pub use status::GitStatusTool;
pub use stash::GitStashTool;
pub use tag::GitTagTool;
