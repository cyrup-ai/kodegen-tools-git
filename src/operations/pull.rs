//! Git pull operations (fetch + merge)

use crate::{GitResult, RepoHandle, FetchOpts, MergeOpts, MergeOutcome};

/// Options for pull operation
#[derive(Debug, Clone)]
pub struct PullOpts {
    /// Remote name (defaults to "origin")
    pub remote: String,
    /// Branch name to pull (e.g., "main")
    pub branch: String,
    /// Allow fast-forward merges
    pub fast_forward: bool,
    /// Automatically create merge commit
    pub auto_commit: bool,
}

/// Result of pull operation
#[derive(Debug, Clone)]
pub struct PullResult {
    /// Merge outcome after fetch
    pub merge_outcome: MergeOutcome,
}

/// Pull from remote (fetch + merge)
///
/// Note: The branch parameter should be the local branch name, not the remote tracking branch.
/// This function will construct the remote tracking branch name (e.g., "origin/main").
pub async fn pull(repo: RepoHandle, opts: PullOpts) -> GitResult<PullResult> {
    // Step 1: Fetch from remote
    let fetch_opts = FetchOpts {
        remote: opts.remote.clone(),
        refspecs: vec![],
        prune: false,
    };
    crate::fetch(repo.clone(), fetch_opts).await?;

    // Step 2: Construct the remote tracking branch name
    // If branch is "main" and remote is "origin", we merge "origin/main"
    let remote_branch = format!("{}/{}", opts.remote, opts.branch);

    // Step 3: Merge with fetched changes
    let merge_opts = MergeOpts::new(&remote_branch)
        .no_ff(!opts.fast_forward)
        .commit(opts.auto_commit);

    let merge_outcome = crate::merge(repo, merge_opts).await?;

    Ok(PullResult { merge_outcome })
}
