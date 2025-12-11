//! Remote existence check operations for tags and branches

use crate::operations::auth::{self, GitCommandOpts};
use crate::{GitError, GitResult, RepoHandle};

/// Check if a branch exists on remote repository
///
/// Uses `git ls-remote` to check if a branch exists on the remote without
/// fetching all refs. This is faster and lighter than a full fetch.
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote` - Remote name
/// * `branch_name` - Name of the branch to check
///
/// # Returns
///
/// * `Ok(true)` - Branch exists on remote
/// * `Ok(false)` - Branch does not exist on remote
/// * `Err(_)` - Network or authentication error
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, check_remote_branch_exists};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// if check_remote_branch_exists(&repo, "origin", "main").await? {
///     println!("Branch exists on remote");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn check_remote_branch_exists(
    repo: &RepoHandle,
    remote: &str,
    branch_name: &str,
) -> GitResult<bool> {
    let work_dir = repo
        .raw()
        .workdir()
        .ok_or_else(|| GitError::InvalidInput("Repository has no working directory".to_string()))?
        .to_path_buf();

    let branch_name = branch_name
        .strip_prefix("refs/heads/")
        .unwrap_or(branch_name);

    if branch_name.is_empty() {
        return Err(GitError::InvalidInput(
            "Branch name cannot be empty".to_string(),
        ));
    }

    let refspec = format!("refs/heads/{branch_name}");

    let output = auth::run_git_command(
        &["ls-remote", "--heads", remote, &refspec],
        GitCommandOpts::new(work_dir).with_timeout(30),
    )
    .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidInput(format!("ls-remote failed: {stderr}")));
    }

    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

/// Check if a tag exists on remote repository
///
/// Uses `git ls-remote` to check if a tag exists on the remote without
/// fetching all refs. This is faster and lighter than a full fetch.
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote` - Remote name
/// * `tag_name` - Name of the tag to check
///
/// # Returns
///
/// * `Ok(true)` - Tag exists on remote
/// * `Ok(false)` - Tag does not exist on remote
/// * `Err(_)` - Network or authentication error
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, check_remote_tag_exists};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// if check_remote_tag_exists(&repo, "origin", "v1.2.3").await? {
///     println!("Tag exists on remote");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn check_remote_tag_exists(
    repo: &RepoHandle,
    remote: &str,
    tag_name: &str,
) -> GitResult<bool> {
    let work_dir = repo
        .raw()
        .workdir()
        .ok_or_else(|| GitError::InvalidInput("Repository has no working directory".to_string()))?
        .to_path_buf();

    let tag_name = tag_name.strip_prefix("refs/tags/").unwrap_or(tag_name);

    if tag_name.is_empty() {
        return Err(GitError::InvalidInput(
            "Tag name cannot be empty".to_string(),
        ));
    }

    let refspec = format!("refs/tags/{tag_name}");

    let output = auth::run_git_command(
        &["ls-remote", "--tags", remote, &refspec],
        GitCommandOpts::new(work_dir).with_timeout(30),
    )
    .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidInput(format!("ls-remote failed: {stderr}")));
    }

    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}
