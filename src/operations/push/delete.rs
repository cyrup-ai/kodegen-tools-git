//! Remote deletion operations for tags and branches

use crate::{GitError, GitResult, RepoHandle};
use tokio::process::Command;
use tokio::time::Duration;

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

    // Normalize tag name: strip "refs/tags/" prefix if present
    let tag_name = tag_name.strip_prefix("refs/tags/").unwrap_or(tag_name);

    // Validate tag name format
    if tag_name.is_empty() {
        return Err(GitError::InvalidInput(
            "Tag name cannot be empty".to_string(),
        ));
    }
    if tag_name.contains("..") {
        return Err(GitError::InvalidInput(format!(
            "Invalid tag name: {tag_name}"
        )));
    }
    if tag_name.starts_with('/') {
        return Err(GitError::InvalidInput(format!(
            "Invalid tag name: {tag_name}"
        )));
    }

    let remote = remote.to_string();
    let tag_name_owned = tag_name.to_string();

    // Default 5 minute timeout
    let timeout_duration = Duration::from_secs(300);

    let mut cmd = Command::new("git");
    cmd.current_dir(&work_dir);
    cmd.env("GIT_TERMINAL_PROMPT", "0"); // Prevent credential prompts from hanging
    cmd.arg("push");
    cmd.arg(&remote);
    cmd.arg("--delete");
    cmd.arg(format!("refs/tags/{tag_name_owned}"));

    // Capture stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Spawn child process with handle for proper cancellation
    let mut child = cmd.spawn().map_err(GitError::Io)?;

    // Wait with timeout and cancellation support using select!
    let status = tokio::select! {
        result = child.wait() => {
            result.map_err(GitError::Io)?
        }
        () = tokio::time::sleep(timeout_duration) => {
            // Timeout - kill the child process
            let _ = child.kill().await;
            return Err(GitError::InvalidInput("Delete remote tag operation timed out after 300 seconds".to_string()));
        }
    };

    // Read stdout and stderr after process completes
    use tokio::io::AsyncReadExt;
    let mut stdout_data = Vec::new();
    let mut stderr_data = Vec::new();

    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_end(&mut stdout_data).await;
    }
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_end(&mut stderr_data).await;
    }

    let output = std::process::Output {
        status,
        stdout: stdout_data,
        stderr: stderr_data,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidInput(format!(
            "Failed to delete remote tag '{tag_name_owned}': {stderr}"
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
/// delete_remote_branch(&repo, "origin", "v1.2.3").await?;
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

    // Normalize branch name: strip "refs/heads/" prefix if present
    let branch_name = branch_name
        .strip_prefix("refs/heads/")
        .unwrap_or(branch_name);

    // Validate branch name format
    if branch_name.is_empty() {
        return Err(GitError::InvalidInput(
            "Branch name cannot be empty".to_string(),
        ));
    }
    if branch_name.contains("..") {
        return Err(GitError::InvalidInput(format!(
            "Invalid branch name: {branch_name}"
        )));
    }
    if branch_name.starts_with('/') {
        return Err(GitError::InvalidInput(format!(
            "Invalid branch name: {branch_name}"
        )));
    }

    let remote = remote.to_string();
    let branch_name_owned = branch_name.to_string();

    // Default 5 minute timeout
    let timeout_duration = Duration::from_secs(300);

    let mut cmd = Command::new("git");
    cmd.current_dir(&work_dir);
    cmd.env("GIT_TERMINAL_PROMPT", "0"); // Prevent credential prompts from hanging
    cmd.arg("push");
    cmd.arg(&remote);
    cmd.arg("--delete");
    cmd.arg(format!("refs/heads/{branch_name_owned}")); // Use full ref to avoid ambiguity with tags

    // Capture stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Spawn child process with handle for proper cancellation
    let mut child = cmd.spawn().map_err(GitError::Io)?;

    // Wait with timeout and cancellation support using select!
    let status = tokio::select! {
        result = child.wait() => {
            result.map_err(GitError::Io)?
        }
        () = tokio::time::sleep(timeout_duration) => {
            // Timeout - kill the child process
            let _ = child.kill().await;
            return Err(GitError::InvalidInput("Delete remote branch operation timed out after 300 seconds".to_string()));
        }
    };

    // Read stdout and stderr after process completes
    use tokio::io::AsyncReadExt;
    let mut stdout_data = Vec::new();
    let mut stderr_data = Vec::new();

    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_end(&mut stdout_data).await;
    }
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_end(&mut stderr_data).await;
    }

    let output = std::process::Output {
        status,
        stdout: stdout_data,
        stderr: stderr_data,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::InvalidInput(format!(
            "Failed to delete remote branch '{branch_name_owned}': {stderr}"
        )));
    }

    Ok(())
}
