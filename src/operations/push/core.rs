//! Core push operations

use crate::{GitError, GitResult, RepoHandle};
use super::{PushOpts, PushResult};
use tokio::process::Command;
use tokio::time::Duration;

/// Push to remote repository
///
/// Pushes commits and/or tags to the specified remote using native git CLI.
///
/// Note: This uses the git command-line tool rather than gix library calls
/// because gix does not yet support push operations. Requires git to be
/// installed and available in PATH.
///
/// # Authentication
///
/// **IMPORTANT**: This function requires proper git authentication configuration.
/// See the [module-level documentation](index.html) for detailed authentication setup.
///
/// This implementation sets `GIT_TERMINAL_PROMPT=0` to prevent hanging on credential
/// prompts in automated environments. If authentication is not configured, the push
/// will fail immediately with an error rather than hang waiting for user input.
///
/// **Quick setup:**
/// - **SSH**: Ensure keys are loaded in ssh-agent: `ssh-add ~/.ssh/id_rsa`
/// - **HTTPS**: Configure credential helper: `git config --global credential.helper store`
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `opts` - Push options
///
/// # Returns
///
/// Returns `PushResult` containing the number of refs successfully pushed
/// (not individual commits). If 3 branches are pushed, `commits_pushed` will be 3
/// regardless of how many commits each branch contains.
///
/// # Errors
///
/// Returns `GitError::InvalidInput` if:
/// - Push fails due to authentication issues
/// - git command is not found in PATH
/// - Network connectivity issues
/// - Remote repository rejects the push
/// - Operation times out (default: 300 seconds, configurable via `opts.timeout_secs`)
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, push, PushOpts};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// let result = push(&repo, PushOpts {
///     remote: "origin".to_string(),
///     refspecs: vec![],
///     force: false,
///     tags: false,
///     timeout_secs: None,
/// }).await?;
/// println!("Pushed {} commits", result.commits_pushed);
/// # Ok(())
/// # }
/// ```
pub async fn push(repo: &RepoHandle, opts: PushOpts) -> GitResult<PushResult> {
    let work_dir = repo
        .raw()
        .workdir()
        .ok_or_else(|| GitError::InvalidInput("Repository has no working directory".to_string()))?
        .to_path_buf();

    let PushOpts {
        remote,
        refspecs,
        force,
        tags,
        timeout_secs,
    } = opts;

    // Default 5 minute timeout (configurable via opts)
    let timeout_duration = Duration::from_secs(timeout_secs.unwrap_or(300));

    let mut cmd = Command::new("git");
    cmd.current_dir(&work_dir);
    cmd.arg("push");

    // Prevent credential prompts from hanging
    cmd.env("GIT_TERMINAL_PROMPT", "0");

    // Force English output for consistent parsing (locale-independent)
    cmd.env("LC_ALL", "C");
    cmd.env("LANG", "C");

    // Capture stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    if force {
        cmd.arg("--force");
    }

    if tags {
        cmd.arg("--tags");
    }

    cmd.arg(&remote);

    for refspec in &refspecs {
        cmd.arg(refspec);
    }

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
            return Err(GitError::InvalidInput(format!("Push operation timed out after {} seconds", timeout_secs.unwrap_or(300))));
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
        return Err(GitError::InvalidInput(format!("Push failed: {stderr}")));
    }

    // Parse output to estimate what was pushed
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    // Count successful ref updates (branches/tags pushed)
    // Note: This counts refs, not individual commits, as accurate commit
    // counting would require additional git commands (git rev-list)
    let commits_pushed = combined
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();

            // Must contain the push arrow
            if !trimmed.contains(" -> ") {
                return false;
            }

            // Exclude errors and rejections
            if trimmed.starts_with('!')
                || trimmed.starts_with("error:")
                || trimmed.contains("[rejected]")
            {
                return false;
            }

            // Match successful update patterns:
            // "   abc123..def456  ref -> ref" (commit range)
            // " * [new branch]    ref -> ref" (new branch)
            // " + abc123...def456 ref -> ref" (forced update)
            trimmed.starts_with(|c: char| c.is_ascii_hexdigit())
                || trimmed.starts_with("* [new")
                || trimmed.starts_with('+')
        })
        .count();
    // Conservative tag counting: indicate whether tags were pushed
    // without attempting fragile output parsing
    let tags_pushed = if tags && output.status.success() {
        // --tags flag used and push succeeded
        1 // At least some tags were pushed (conservative estimate)
    } else if output.status.success() && refspecs.iter().any(|r| r.contains("refs/tags/")) {
        // Specific tag refspecs provided and push succeeded
        refspecs.iter().filter(|r| r.contains("refs/tags/")).count()
    } else {
        0
    };

    let mut warnings = Vec::new();
    // Check the force flag directly instead of parsing output (locale-independent)
    if force {
        warnings.push("Force push executed".to_string());
    }

    Ok(PushResult {
        commits_pushed,
        tags_pushed,
        warnings,
    })
}

/// Push current branch to remote
///
/// Convenience function that pushes the current branch to the specified remote.
/// Requires proper authentication configuration - see [module-level docs](index.html).
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote` - Remote name (defaults to "origin")
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, push_current_branch};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// push_current_branch(&repo, "origin").await?;
/// # Ok(())
/// # }
/// ```
pub async fn push_current_branch(repo: &RepoHandle, remote: &str) -> GitResult<PushResult> {
    push(
        repo,
        PushOpts {
            remote: remote.to_string(),
            refspecs: Vec::new(),
            force: false,
            tags: false,
            timeout_secs: None,
        },
    )
    .await
}

/// Push all tags to remote
///
/// Convenience function that pushes all tags to the specified remote.
/// Requires proper authentication configuration - see [module-level docs](index.html).
///
/// # Arguments
///
/// * `repo` - Repository handle
/// * `remote` - Remote name (defaults to "origin")
///
/// # Example
///
/// ```rust,no_run
/// use kodegen_git::{open_repo, push_tags};
///
/// # async fn example() -> kodegen_git::GitResult<()> {
/// let repo = open_repo("/path/to/repo")?;
/// push_tags(&repo, "origin").await?;
/// # Ok(())
/// # }
/// ```
pub async fn push_tags(repo: &RepoHandle, remote: &str) -> GitResult<PushResult> {
    push(
        repo,
        PushOpts {
            remote: remote.to_string(),
            refspecs: Vec::new(),
            force: false,
            tags: true,
            timeout_secs: None,
        },
    )
    .await
}
