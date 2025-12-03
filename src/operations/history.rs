//! Git file history for code investigation.
//!
//! Provides actual diff content, not just metadata.
//! Optimized for AI agents scanning and searching history.

use std::path::PathBuf;

use regex::Regex;

use crate::{GitError, GitResult, RepoHandle};

/// Options for history operation
#[derive(Debug, Clone)]
pub struct HistoryOpts {
    pub file: PathBuf,
    pub search: Option<Regex>,
    pub limit: usize,
    pub since: Option<String>,
    pub until: Option<String>,
}

impl HistoryOpts {
    pub fn new<P: Into<PathBuf>>(file: P) -> Self {
        Self {
            file: file.into(),
            search: None,
            limit: 20,
            since: None,
            until: None,
        }
    }

    pub fn search(mut self, pattern: &str) -> GitResult<Self> {
        self.search = Some(Regex::new(pattern).map_err(|e| {
            GitError::InvalidInput(format!("Invalid search regex: {e}"))
        })?);
        Ok(self)
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = n;
        self
    }

    pub fn since(mut self, rev: impl Into<String>) -> Self {
        self.since = Some(rev.into());
        self
    }

    pub fn until(mut self, rev: impl Into<String>) -> Self {
        self.until = Some(rev.into());
        self
    }
}

/// A commit with its diff
#[derive(Debug, Clone)]
pub struct HistoryCommit {
    pub id: String,        // Short hash (7 chars)
    pub summary: String,
    pub time: chrono::DateTime<chrono::Utc>,
    pub additions: u32,
    pub deletions: u32,
    pub diff: String,      // Unified diff with context
}

/// History result (two modes)
#[derive(Debug)]
pub enum HistoryResult {
    /// Per-commit diffs
    Commits {
        file: String,
        total_examined: usize,
        commits: Vec<HistoryCommit>,
    },
    /// Cumulative diff between two revisions
    Range {
        file: String,
        since: String,
        until: String,
        additions: u32,
        deletions: u32,
        diff: String,
    },
}

/// Execute history query
pub async fn history(repo: RepoHandle, opts: HistoryOpts) -> GitResult<HistoryResult> {
    let repo_inner = repo.clone_inner();

    tokio::task::spawn_blocking(move || history_sync(&repo_inner, opts))
        .await
        .map_err(|e| GitError::InvalidInput(format!("Task join error: {e}")))?
}

fn history_sync(repo: &gix::Repository, opts: HistoryOpts) -> GitResult<HistoryResult> {
    use gix::bstr::ByteSlice;

    // Normalize file path
    let workdir = repo.workdir().ok_or_else(|| {
        GitError::InvalidInput("Cannot query history in bare repository".to_string())
    })?;

    let file_path = if opts.file.is_absolute() {
        opts.file
            .strip_prefix(workdir)
            .map_err(|_| {
                GitError::InvalidInput(format!(
                    "Path {} is not within repository",
                    opts.file.display()
                ))
            })?
            .to_path_buf()
    } else {
        opts.file.clone()
    };

    // Resolve start revision
    let since_id = if let Some(ref rev) = opts.since {
        repo.rev_parse_single(rev.as_str())
            .map_err(|e| GitError::Gix(Box::new(e)))?
            .detach()
    } else {
        repo.head_id()
            .map_err(|e| GitError::Gix(Box::new(e)))?
            .detach()
    };

    // RANGE MODE: cumulative diff between two revisions
    if let Some(ref until_rev) = opts.until {
        let until_id = repo
            .rev_parse_single(until_rev.as_str())
            .map_err(|e| GitError::Gix(Box::new(e)))?
            .detach();

        let (additions, deletions, diff) = compute_file_diff(repo, since_id, until_id, &file_path)?;

        return Ok(HistoryResult::Range {
            file: file_path.to_string_lossy().to_string(),
            since: opts.since.unwrap_or_else(|| "HEAD".to_string()),
            until: until_rev.clone(),
            additions,
            deletions,
            diff,
        });
    }

    // COMMITS MODE: per-commit diffs
    let mut commits = Vec::new();
    let mut total_examined = 0;

    let rev_walk = repo
        .rev_walk([since_id])
        .all()
        .map_err(|e| GitError::Gix(e.into()))?;

    for commit_result in rev_walk {
        if commits.len() >= opts.limit {
            break;
        }

        let info = commit_result.map_err(|e| GitError::Gix(e.into()))?;
        let commit = repo
            .find_object(info.id)
            .map_err(|e| GitError::Gix(e.into()))?
            .into_commit();

        total_examined += 1;

        // REUSE commit_touches_path from log.rs
        if !crate::operations::log::commit_touches_path(repo, &commit, &file_path)? {
            continue;
        }

        // Compute diff against parent
        let parent_id = commit.parent_ids().next().map(|p| p.detach());

        let (additions, deletions, diff) = if let Some(pid) = parent_id {
            compute_file_diff(repo, pid, info.id, &file_path)?
        } else {
            compute_file_diff_from_empty(repo, info.id, &file_path)?
        };

        // Skip if diff is empty
        if diff.is_empty() {
            continue;
        }

        // Apply search filter
        if let Some(ref re) = opts.search
            && !re.is_match(&diff)
        {
            continue;
        }

        // Get commit metadata
        let time = commit.time().map_err(|e| GitError::Gix(Box::new(e)))?;
        let commit_time = {
            use chrono::TimeZone;
            chrono::Utc
                .timestamp_opt(time.seconds, 0)
                .single()
                .ok_or_else(|| {
                    GitError::InvalidInput(format!("Invalid timestamp {}", time.seconds))
                })?
        };

        commits.push(HistoryCommit {
            id: info.id.to_string()[..7].to_string(), // Short hash
            summary: commit
                .message()
                .map(|msg| msg.summary().as_bstr().to_string())
                .unwrap_or_default(),
            time: commit_time,
            additions,
            deletions,
            diff,
        });
    }

    Ok(HistoryResult::Commits {
        file: file_path.to_string_lossy().to_string(),
        total_examined,
        commits,
    })
}

/// Compute unified diff for a file between two commits
fn compute_file_diff(
    repo: &gix::Repository,
    from_id: gix::ObjectId,
    to_id: gix::ObjectId,
    file_path: &std::path::Path,
) -> GitResult<(u32, u32, String)> {
    let from_content = get_file_at_commit(repo, from_id, file_path)?;
    let to_content = get_file_at_commit(repo, to_id, file_path)?;

    compute_diff(&from_content, &to_content)
}

/// Compute diff for file added in first commit
fn compute_file_diff_from_empty(
    repo: &gix::Repository,
    commit_id: gix::ObjectId,
    file_path: &std::path::Path,
) -> GitResult<(u32, u32, String)> {
    let content = get_file_at_commit(repo, commit_id, file_path)?;
    compute_diff("", &content)
}

/// Get file content at a specific commit
fn get_file_at_commit(
    repo: &gix::Repository,
    commit_id: gix::ObjectId,
    file_path: &std::path::Path,
) -> GitResult<String> {
    let commit = repo
        .find_object(commit_id)
        .map_err(|e| GitError::Gix(e.into()))?
        .try_into_commit()
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    let tree = commit.tree().map_err(|e| GitError::Gix(Box::new(e)))?;

    match tree
        .lookup_entry_by_path(file_path)
        .map_err(|e| GitError::Gix(Box::new(e)))?
    {
        Some(entry) => {
            let blob = repo
                .find_object(entry.oid())
                .map_err(|e| GitError::Gix(e.into()))?
                .try_into_blob()
                .map_err(|e| GitError::Gix(Box::new(e)))?;
            Ok(String::from_utf8_lossy(blob.data.as_slice()).to_string())
        }
        None => Ok(String::new()),
    }
}

/// Compute unified diff between two strings using similar crate
fn compute_diff(old: &str, new: &str) -> GitResult<(u32, u32, String)> {
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old, new);

    let mut additions = 0u32;
    let mut deletions = 0u32;
    let mut lines = Vec::new();

    // Generate unified diff with 3 lines of context
    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        for change in hunk.iter_changes() {
            let prefix = match change.tag() {
                ChangeTag::Insert => {
                    additions += 1;
                    "+"
                }
                ChangeTag::Delete => {
                    deletions += 1;
                    "-"
                }
                ChangeTag::Equal => " ",
            };
            lines.push(format!("{}{}", prefix, change.value().trim_end()));
        }
    }

    Ok((additions, deletions, lines.join("\n")))
}
