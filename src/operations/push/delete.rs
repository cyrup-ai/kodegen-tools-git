//! Remote deletion operations for tags and branches

use crate::operations::auth::{self, GitCommandOpts};
use crate::{GitError, GitResult, RepoHandle};

/// Delete a tag from remote repository
///
/// Requires proper authentication configuration - see [module-level docs](index.html).
/// Sets `GIT_TERMINAL_PROMPT=0` to prevent hanging on authentication prompts.
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote` - Remote name
/// * `tag_name` - Name of the tag to delete
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, delete_remote_tag};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// delete_remote_tag(&repo, "origin", "v1.0.0").await?;
/// # Ok(())
/// # }
/// ```
pub async fn delete_remote_tag(repo: &RepoHandle, remote: &str, tag_name: &str) -> GitResult<()> {
    let work_dir = repo
        .raw()
        .workdir()
        .ok_or_else(|| GitError::InvalidInput("Repository has no working directory".to_string()))?
        .to_path_buf();

    let tag_name = tag_name.strip_prefix("refs/tags/").unwrap_or(tag_name);
    validate_ref_name(tag_name, "tag")?;

    let refspec = format!("refs/tags/{tag_name}");

    let output = auth::run_git_command(
        &["push", remote, "--delete", &refspec],
        GitCommandOpts::new(work_dir).with_timeout(300),
    )
    .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidInput(format!(
            "Failed to delete remote tag '{tag_name}': {stderr}"
        )));
    }

    Ok(())
}

/// Delete a branch from remote repository
///
/// Requires proper authentication configuration - see [module-level docs](index.html).
/// Sets `GIT_TERMINAL_PROMPT=0` to prevent hanging on authentication prompts.
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote` - Remote name
/// * `branch_name` - Name of the branch to delete
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, delete_remote_branch};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// delete_remote_branch(&repo, "origin", "feature-branch").await?;
/// # Ok(())
/// # }
/// ```
pub async fn delete_remote_branch(
    repo: &RepoHandle,
    remote: &str,
    branch_name: &str,
) -> GitResult<()> {
    let work_dir = repo
        .raw()
        .workdir()
        .ok_or_else(|| GitError::InvalidInput("Repository has no working directory".to_string()))?
        .to_path_buf();

    let branch_name = branch_name
        .strip_prefix("refs/heads/")
        .unwrap_or(branch_name);
    validate_ref_name(branch_name, "branch")?;

    let refspec = format!("refs/heads/{branch_name}");

    let output = auth::run_git_command(
        &["push", remote, "--delete", &refspec],
        GitCommandOpts::new(work_dir).with_timeout(300),
    )
    .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidInput(format!(
            "Failed to delete remote branch '{branch_name}': {stderr}"
        )));
    }

    Ok(())
}

fn validate_ref_name(name: &str, ref_type: &str) -> GitResult<()> {
    if name.is_empty() {
        return Err(GitError::InvalidInput(format!(
            "{ref_type} name cannot be empty"
        )));
    }
    if name.contains("..") || name.starts_with('/') {
        return Err(GitError::InvalidInput(format!(
            "Invalid {ref_type} name: {name}"
        )));
    }
    Ok(())
}
