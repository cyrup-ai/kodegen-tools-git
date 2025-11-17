//! `kodegen_git` - A Git facade over the gix (Gitoxide) library
//!
//! This library provides an async-first Git service layer with comprehensive
//! operation support using the modern gix crate. Each Git operation is
//! implemented in its own module with builder patterns for ergonomic usage.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use gix::hash::ObjectId;
use thiserror::Error;

// Module declarations
pub mod operations;
pub mod runtime;
pub mod tools;

// Re-export runtime types
pub use runtime::{AsyncStream, AsyncTask, EmitterBuilder};

// Re-export Git operations
pub use operations::{
    AddOpts, BranchInfo, BranchOpts, ChangeType, CheckoutOpts, CloneOpts, CommitOpts, CommitResult,
    DetailedCommitInfo, DiffOpts, DiffStats, FetchOpts, FileDiffStats, GitUrl, LogOpts, MergeOpts,
    MergeOutcome, PullOpts, PullResult, PushOpts, PushResult, RemoteAddOpts, RemoteInfo, RepoPaths,
    RepositoryInfo, ResetMode, ResetOpts, Signature, TagInfo, TagOpts, WorktreeAddOpts, WorktreeInfo,
    WorktreeLockOpts, WorktreeRemoveOpts, add, add_remote, branch, check_remote_branch_exists,
    check_remote_tag_exists, checkout, clone_repo, commit, create_tag, current_branch, delete_branch,
    delete_remote_branch, delete_remote_tag, delete_tag, diff, discover_repo, fetch,
    get_commit_details, get_repo_paths, head_commit, init_bare_repo, init_repo, is_clean,
    is_detached, is_repository, list_branches, list_remotes, list_tags, list_worktrees, log, merge,
    open_repo, parse_git_url, probe_repository, pull, push, push_current_branch, push_tags,
    remote_exists, remove_remote, rename_branch, reset, reset_hard, reset_mixed, reset_soft,
    stash_pop, stash_save, StashInfo, StashOpts, tag_exists, worktree_add, worktree_lock,
    worktree_prune, worktree_remove, worktree_unlock,
};

// Re-export MCP tools
pub use tools::{
    GitAddTool, GitBranchCreateTool, GitBranchDeleteTool, GitBranchListTool, GitBranchRenameTool,
    GitCheckoutTool, GitCloneTool, GitCommitTool, GitDiffTool, GitDiscoverTool, GitFetchTool,
    GitInitTool, GitLogTool, GitMergeTool, GitOpenTool, GitPullTool, GitPushTool, GitRemoteAddTool,
    GitRemoteListTool, GitRemoteRemoveTool, GitResetTool, GitStashTool, GitStatusTool, GitTagTool,
    GitWorktreeAddTool, GitWorktreeListTool, GitWorktreeLockTool, GitWorktreePruneTool,
    GitWorktreeRemoveTool, GitWorktreeUnlockTool,
};

/// Error types for `GitGix` operations
#[derive(Debug, Error)]
pub enum GitError {
    #[error("Gix error: {0}")]
    Gix(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Repository not found at path: {0}")]
    RepoNotFound(PathBuf),

    #[error("Remote `{0}` not found")]
    RemoteNotFound(String),

    #[error("Branch `{0}` not found")]
    BranchNotFound(String),

    #[error("Reference `{0}` not found")]
    ReferenceNotFound(String),

    #[error("Merge conflict: {0}")]
    MergeConflict(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(&'static str),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Channel closed prematurely")]
    ChannelClosed,

    #[error("Operation aborted by user")]
    Aborted,

    #[error("Worktree already exists at path: {0}")]
    WorktreeAlreadyExists(PathBuf),

    #[error("Worktree not found: {0}")]
    WorktreeNotFound(String),

    #[error("Worktree is locked: {0}")]
    WorktreeLocked(String),

    #[error("Branch '{0}' is already checked out in another worktree")]
    BranchInUse(String),

    #[error("Cannot modify main worktree")]
    CannotModifyMainWorktree,

    #[error("Invalid worktree name: {0}")]
    InvalidWorktreeName(String),
}

impl From<gix::open::Error> for GitError {
    fn from(e: gix::open::Error) -> Self {
        GitError::Gix(Box::new(e))
    }
}

impl From<gix::discover::Error> for GitError {
    fn from(e: gix::discover::Error) -> Self {
        GitError::Gix(Box::new(e))
    }
}

impl From<gix::init::Error> for GitError {
    fn from(e: gix::init::Error) -> Self {
        GitError::Gix(Box::new(e))
    }
}

impl From<gix::clone::Error> for GitError {
    fn from(e: gix::clone::Error) -> Self {
        GitError::Gix(Box::new(e))
    }
}

/// Convenience result alias.
pub type GitResult<T> = Result<T, GitError>;

/// Strong-typed repository wrapper with cheap cloning.
///
/// Wraps a single `gix::Repository` instance. Cloning this handle creates
/// a new repository instance that shares underlying data structures (refs, objects)
/// but has independent thread-local buffers, making it Send-safe.
///
/// # Performance
///
/// Cloning this handle is cheap - it shares the underlying ODB and refs database
/// but clears thread-local buffers. This is the design of `gix::Repository` itself.
///
/// # Thread Safety
///
/// The wrapped `gix::Repository` is `Send` but not `Sync`.
/// Each clone can be safely moved to a different thread.
#[derive(Debug, Clone)]
pub struct RepoHandle {
    inner: gix::Repository,
}

impl RepoHandle {
    /// Create from an existing `gix::Repository`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let repo = gix::open("/path/to/repo")?;
    /// let handle = RepoHandle::new(repo);
    /// let handle2 = handle.clone(); // Cheap clone with shared data!
    /// ```
    #[inline]
    pub fn new(inner: gix::Repository) -> Self {
        Self { inner }
    }

    /// Access the underlying `gix::Repository` with zero cost.
    ///
    /// Returns a reference to the repository. No Result needed
    /// because the repository is already open and validated.
    ///
    /// # Example
    ///
    /// ```rust
    /// let head = handle.raw().head()?;
    /// let config = handle.raw().config_snapshot();
    /// ```
    #[inline]
    pub fn raw(&self) -> &gix::Repository {
        &self.inner
    }

    /// Clone the underlying repository for use in async tasks.
    ///
    /// Creates a cheap clone of the repository with shared refs and objects
    /// but with thread-local buffers cleared. This makes the cloned repository
    /// Send-safe for use in `spawn_blocking`.
    ///
    /// This is equivalent to cloning the `RepoHandle` itself, provided for
    /// API compatibility.
    ///
    /// # Example
    ///
    /// ```rust
    /// let repo_clone = handle.clone_inner();
    /// tokio::task::spawn_blocking(move || {
    ///     // Use repo_clone safely in blocking task
    /// });
    /// ```
    #[inline]
    pub fn clone_inner(&self) -> gix::Repository {
        self.inner.clone()
    }
}

/// A unique commit identifier.
pub type CommitId = ObjectId;

/// Lightweight commit metadata for streaming logs.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: CommitId,
    pub author: Signature,
    pub summary: String,
    pub time: DateTime<Utc>,
}

/// Backward compatibility module providing nested namespace for git operations.
///
/// This module re-exports all operations in their original nested structure
/// for compatibility with existing code that expects `kodegen_git::git::*` paths.
pub mod git {
    /// Add operation re-exports
    pub mod add {
        pub use crate::operations::add::*;
    }

    /// Branch operation re-exports
    pub mod branch {
        pub use crate::operations::branch::*;
    }

    /// Checkout operation re-exports
    pub mod checkout {
        pub use crate::operations::checkout::*;
    }

    /// Clone operation re-exports
    pub mod clone {
        pub use crate::operations::clone::*;
    }

    /// Commit operation re-exports
    pub mod commit {
        pub use crate::operations::commit::*;
    }

    /// Fetch operation re-exports
    pub mod fetch {
        pub use crate::operations::fetch::*;
    }

    /// Log operation re-exports
    pub mod log {
        pub use crate::operations::log::*;
    }

    /// Merge operation re-exports
    pub mod merge {
        pub use crate::operations::merge::*;
    }

    /// Repository open/init operation re-exports
    pub mod open {
        pub use crate::operations::open::*;
    }
}

/// Start the HTTP server programmatically for embedded mode
///
/// This is called by kodegend instead of spawning an external process.
/// Blocks until the server shuts down.
///
/// # Arguments
/// * `addr` - Socket address to bind to (e.g., "127.0.0.1:30450")
/// * `tls_cert` - Optional path to TLS certificate file
/// * `tls_key` - Optional path to TLS private key file
///
/// # Returns
/// ServerHandle for graceful shutdown, or error if startup fails
pub async fn start_server(
    addr: std::net::SocketAddr,
    tls_cert: Option<std::path::PathBuf>,
    tls_key: Option<std::path::PathBuf>,
) -> anyhow::Result<kodegen_server_http::ServerHandle> {
    use kodegen_server_http::{create_http_server, Managers, RouterSet, register_tool};
    use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
    use std::time::Duration;

    let tls_config = match (tls_cert, tls_key) {
        (Some(cert), Some(key)) => Some((cert, key)),
        _ => None,
    };

    let shutdown_timeout = Duration::from_secs(30);

    create_http_server("git", addr, tls_config, shutdown_timeout, Duration::ZERO, |_config, _tracker| {
        Box::pin(async move {
            let mut tool_router = ToolRouter::new();
            let mut prompt_router = PromptRouter::new();
            let managers = Managers::new();

            // Register all 27 git tools (zero-state structs, no constructors)

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

            // Core git operations (5 tools)
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitCommitTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitLogTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitDiffTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitAddTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitCheckoutTool);

            // Remote operations (7 tools)
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitFetchTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitMergeTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitPullTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitPushTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitRemoteAddTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitRemoteListTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitRemoteRemoveTool);

            // Worktree operations (6 tools)
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeAddTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeRemoveTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeListTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeLockTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreeUnlockTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitWorktreePruneTool);

            // Other operations (4 tools)
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitResetTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitStashTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitStatusTool);
            (tool_router, prompt_router) = register_tool(tool_router, prompt_router, GitTagTool);

            Ok(RouterSet::new(tool_router, prompt_router, managers))
        })
    }).await
}
