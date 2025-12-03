//! Git operations module
//!
//! Provides local Git repository operations using the gix (Gitoxide) library.

pub mod add;
pub mod branch;
pub mod checkout;
pub mod clone;
pub mod commit;
pub mod diff;
pub mod fetch;
pub mod history;
pub mod introspection;
pub mod log;
pub mod merge;
pub mod open;
pub mod pull;
pub mod push;
pub mod remote;
pub mod reset;
pub mod stash;
pub mod status;
pub mod tag;
pub mod worktree;

// Re-export operation functions
pub use add::{AddOpts, add};
pub use branch::{BranchOpts, branch, delete_branch, list_branches, rename_branch};
pub use checkout::{CheckoutOpts, checkout};
pub use clone::{CloneOpts, clone_repo};
pub use commit::{CommitOpts, CommitResult, Signature, commit};
pub use diff::{ChangeType, DiffOpts, DiffStats, FileDiffStats, diff};
pub use fetch::{FetchOpts, fetch};
pub use history::{HistoryCommit, HistoryOpts, HistoryResult, history};
pub use introspection::{DetailedCommitInfo, GitUrl, RepoPaths, get_commit_details, get_repo_paths, parse_git_url};
pub use log::{LogOpts, log};
pub use merge::{MergeOpts, MergeOutcome, merge};
pub use open::{
    RepositoryInfo, discover_repo, init_bare_repo, init_repo, is_repository, open_repo,
    probe_repository,
};
pub use pull::{PullOpts, PullResult, pull};
pub use push::{
    PushOpts, PushResult, check_remote_branch_exists, check_remote_tag_exists,
    delete_remote_branch, delete_remote_tag, push, push_current_branch, push_tags,
};
pub use remote::{RemoteAddOpts, add_remote, remove_remote};
pub use reset::{ResetMode, ResetOpts, reset, reset_hard, reset_mixed, reset_soft};
pub use stash::{StashInfo, StashOpts, stash_pop, stash_save};
pub use status::{
    BranchInfo, RemoteInfo, current_branch, head_commit, is_clean, is_detached, list_remotes,
    remote_exists,
};
pub use tag::{TagInfo, TagOpts, create_tag, delete_tag, list_tags, tag_exists};
pub use worktree::{
    WorktreeAddOpts, WorktreeInfo, WorktreeLockOpts, WorktreeRemoveOpts, list_worktrees,
    worktree_add, worktree_lock, worktree_prune, worktree_remove, worktree_unlock,
};
