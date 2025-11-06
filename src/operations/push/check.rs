//! Remote existence check operations for tags and branches

use crate::{GitError, GitResult, RepoHandle};
use tokio::process::Command;
use tokio::time::Duration;

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
/// if check_remote_branch_exists(&repo, "origin", "v1.2.3").await? {
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

    let remote = remote.to_string();
    let branch_name_owned = branch_name.to_string();
    let refspec = format!("refs/heads/{branch_name_owned}");

    // Default 30 second timeout for ls-remote (should be quick)
    let timeout_duration = Duration::from_secs(30);

    let mut cmd = Command::new("git");
    cmd.current_dir(&work_dir);
    cmd.env("GIT_TERMINAL_PROMPT", "0"); // Prevent credential prompts
    cmd.arg("ls-remote");
    cmd.arg("--heads"); // Only list branches
    cmd.arg(&remote);
    cmd.arg(&refspec);

    // Capture stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Spawn child process
    let mut child = cmd.spawn().map_err(GitError::Io)?;

    // Wait with timeout
    let status = tokio::select! {
        result = child.wait() => {
            result.map_err(GitError::Io)?
        }
        () = tokio::time::sleep(timeout_duration) => {
            // Timeout - kill the child process
            let _ = child.kill().await;
            return Err(GitError::InvalidInput("ls-remote operation timed out after 30 seconds".to_string()));
        }
    };

    // Read stdout and stderr
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
            "ls-remote failed: {stderr}"
        )));
    }

    // If output is non-empty, the branch exists
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.trim().is_empty())
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

    // Normalize tag name: strip "refs/tags/" prefix if present
    let tag_name = tag_name.strip_prefix("refs/tags/").unwrap_or(tag_name);

    // Validate tag name format
    if tag_name.is_empty() {
        return Err(GitError::InvalidInput(
            "Tag name cannot be empty".to_string(),
        ));
    }

    let remote = remote.to_string();
    let tag_name_owned = tag_name.to_string();
    let refspec = format!("refs/tags/{tag_name_owned}");

    // Default 30 second timeout for ls-remote (should be quick)
    let timeout_duration = Duration::from_secs(30);

    let mut cmd = Command::new("git");
    cmd.current_dir(&work_dir);
    cmd.env("GIT_TERMINAL_PROMPT", "0"); // Prevent credential prompts
    cmd.arg("ls-remote");
    cmd.arg("--tags"); // Only list tags
    cmd.arg(&remote);
    cmd.arg(&refspec);

    // Capture stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Spawn child process
    let mut child = cmd.spawn().map_err(GitError::Io)?;

    // Wait with timeout
    let status = tokio::select! {
        result = child.wait() => {
            result.map_err(GitError::Io)?
        }
        () = tokio::time::sleep(timeout_duration) => {
            // Timeout - kill the child process
            let _ = child.kill().await;
            return Err(GitError::InvalidInput("ls-remote operation timed out after 30 seconds".to_string()));
        }
    };

    // Read stdout and stderr
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
            "ls-remote failed: {stderr}"
        )));
    }

    // If output is non-empty, the tag exists
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.trim().is_empty())
}
