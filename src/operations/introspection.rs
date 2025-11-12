//! Git repository introspection operations.
//!
//! This module provides functions for inspecting Git repository internals
//! such as commit metadata, repository paths, and URL parsing.

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::{CommitId, GitError, GitResult, RepoHandle, Signature};

/// Detailed commit information including parents and short hash.
#[derive(Debug, Clone)]
pub struct DetailedCommitInfo {
    pub id: CommitId,
    pub short_id: String,
    pub message: String,
    pub author: Signature,
    pub committer: Signature,
    pub timestamp: DateTime<Utc>,
    pub parent_ids: Vec<CommitId>,
}

/// Repository paths.
#[derive(Debug, Clone)]
pub struct RepoPaths {
    pub git_dir: PathBuf,
    pub work_dir: Option<PathBuf>,
}

/// Parsed Git URL information.
#[derive(Debug, Clone)]
pub struct GitUrl {
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub owner: Option<String>,
    pub repo: Option<String>,
}

/// Get detailed information about a commit by ID.
///
/// Returns comprehensive commit metadata including author, committer, timestamp,
/// message, and parent commit IDs.
///
/// # Example
///
/// ```rust
/// use kodegen_tools_git::{open_repo, get_commit_details};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let repo = open_repo("/path/to/repo").await?;
/// let head_id = repo.raw().head_id().ok().expect("No HEAD");
/// let info = get_commit_details(&repo, &head_id.to_string()).await?;
/// println!("Author: {} <{}>", info.author.name, info.author.email);
/// # Ok(())
/// # }
/// ```
pub async fn get_commit_details(repo: &RepoHandle, commit_id: &str) -> GitResult<DetailedCommitInfo> {
    let repo_clone = repo.clone_inner();
    let commit_id_str = commit_id.to_string();

    tokio::task::spawn_blocking(move || {
        use gix::bstr::ByteSlice;

        // Parse commit ID
        let oid = repo_clone
            .rev_parse_single(commit_id_str.as_bytes().as_bstr())
            .map_err(|e| GitError::Gix(Box::new(e)))?
            .object()
            .map_err(|e| GitError::Gix(Box::new(e)))?
            .id;

        // Find commit object
        let commit = repo_clone
            .find_commit(oid)
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        // Extract metadata
        let id = commit.id().detach();
        let short_id = commit
            .id()
            .shorten()
            .map(|prefix| prefix.to_string())
            .unwrap_or_else(|_| id.to_string());

        let message = commit
            .message()
            .map(|m| m.title.to_string())
            .unwrap_or_else(|_| "No commit message".to_string());

        // Extract author
        let author_ref = commit.author().map_err(|e| GitError::Gix(Box::new(e)))?;
        let author_time = parse_git_time(author_ref.time)?;
        let author = Signature {
            name: author_ref.name.to_string(),
            email: author_ref.email.to_string(),
            time: author_time,
        };

        // Extract committer
        let committer_ref = commit.committer().map_err(|e| GitError::Gix(Box::new(e)))?;
        let committer_time = parse_git_time(committer_ref.time)?;
        let committer = Signature {
            name: committer_ref.name.to_string(),
            email: committer_ref.email.to_string(),
            time: committer_time,
        };

        let timestamp = author_time;
        let parent_ids = commit.parent_ids().map(|id| id.detach()).collect();

        Ok(DetailedCommitInfo {
            id,
            short_id,
            message,
            author,
            committer,
            timestamp,
            parent_ids,
        })
    })
    .await
    .map_err(|e| GitError::InvalidInput(format!("Task join error: {e}")))?
}

/// Parse Git time format from string representation.
///
/// Returns an error instead of silently falling back to current time.
fn parse_git_time(time_str: &str) -> GitResult<DateTime<Utc>> {
    // Git time format: "<seconds> <timezone>"
    let seconds = time_str
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| GitError::InvalidInput(
            format!("Failed to parse Git timestamp: {}", time_str)
        ))?;

    DateTime::from_timestamp(seconds, 0)
        .ok_or_else(|| GitError::InvalidInput(
            format!("Invalid timestamp value: {}", seconds)
        ))
}

/// Get repository directory paths.
///
/// Returns the Git directory (.git) and optional working directory.
///
/// # Example
///
/// ```rust
/// use kodegen_tools_git::{open_repo, get_repo_paths};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let repo = open_repo("/path/to/repo").await?;
/// let paths = get_repo_paths(&repo).await?;
/// println!("Git dir: {:?}", paths.git_dir);
/// println!("Work dir: {:?}", paths.work_dir);
/// # Ok(())
/// # }
/// ```
pub async fn get_repo_paths(repo: &RepoHandle) -> GitResult<RepoPaths> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        let git_dir = repo_clone.path().to_path_buf();
        let work_dir = repo_clone.workdir().map(PathBuf::from);

        Ok(RepoPaths { git_dir, work_dir })
    })
    .await
    .map_err(|e| GitError::InvalidInput(format!("Task join error: {e}")))?
}

/// Parse a Git URL into its components.
///
/// Supports multiple URL formats:
/// - SSH SCP-like: `git@github.com:owner/repo.git`
/// - SSH protocol: `ssh://git@github.com/owner/repo.git`
/// - HTTPS: `https://github.com/owner/repo.git`
/// - HTTP: `http://github.com/owner/repo.git`
///
/// For GitHub URLs, extracts owner and repo names.
///
/// # Example
///
/// ```rust
/// use kodegen_tools_git::parse_git_url;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let url = parse_git_url("git@github.com:owner/repo.git").await?;
/// assert_eq!(url.owner, Some("owner".to_string()));
/// assert_eq!(url.repo, Some("repo".to_string()));
/// # Ok(())
/// # }
/// ```
pub async fn parse_git_url(url: &str) -> GitResult<GitUrl> {
    let url = url.to_string();

    tokio::task::spawn_blocking(move || {
        use gix::bstr::{BStr, ByteSlice};

        // Parse URL using gix_url
        let parsed = gix_url::parse(BStr::new(url.as_bytes()))
            .map_err(|e| GitError::Parse(format!("Failed to parse Git URL: {e}")))?;

        let scheme = parsed.scheme.as_str().to_string();
        let host = parsed
            .host()
            .map(|h| h.to_string())
            .unwrap_or_default();
        let path = parsed.path.to_str_lossy().to_string();

        // Try to extract owner/repo from path
        let (owner, repo) = extract_owner_repo(&path);

        Ok(GitUrl {
            scheme,
            host,
            path,
            owner,
            repo,
        })
    })
    .await
    .map_err(|e| GitError::InvalidInput(format!("Task join error: {e}")))?
}

/// Extract owner and repo from a Git path.
fn extract_owner_repo(path: &str) -> (Option<String>, Option<String>) {
    // Remove leading slash
    let path = path.trim_start_matches('/');

    // Remove .git suffix
    let path = path.trim_end_matches(".git");

    // Split by '/'
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() >= 2 {
        let owner = parts[0].to_string();
        let repo = parts[1].to_string();
        (Some(owner), Some(repo))
    } else {
        (None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_owner_repo() {
        let (owner, repo) = extract_owner_repo("/owner/repo.git");
        assert_eq!(owner, Some("owner".to_string()));
        assert_eq!(repo, Some("repo".to_string()));

        let (owner, repo) = extract_owner_repo("owner/repo");
        assert_eq!(owner, Some("owner".to_string()));
        assert_eq!(repo, Some("repo".to_string()));

        let (owner, repo) = extract_owner_repo("invalid");
        assert_eq!(owner, None);
        assert_eq!(repo, None);
    }
}
