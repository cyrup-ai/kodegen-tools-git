//! Git stash operations

use crate::{GitError, GitResult, RepoHandle};
use gix::bstr::ByteSlice;

/// Options for stash save
#[derive(Debug, Clone)]
pub struct StashOpts {
    /// Optional stash message/description
    pub message: Option<String>,
    /// Include untracked files in stash
    pub include_untracked: bool,
}

/// Information about a stash entry
#[derive(Debug, Clone)]
pub struct StashInfo {
    /// Stash name (e.g., "stash@{0}")
    pub name: String,
    /// Stash message/description
    pub message: String,
    /// Commit hash of stashed state
    pub commit_hash: String,
}

/// Save working directory changes to stash
pub async fn stash_save(repo: RepoHandle, opts: StashOpts) -> GitResult<StashInfo> {
    let repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        // Check if there are changes to stash
        let is_dirty = repo_clone
            .is_dirty()
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        if !is_dirty {
            return Err(GitError::InvalidInput(
                "No changes to stash (working directory is clean)".to_string(),
            ));
        }

        // Get current branch for context
        let head = repo_clone.head().map_err(|e| GitError::Gix(Box::new(e)))?;
        let branch_name = head
            .referent_name()
            .and_then(|name| {
                name.shorten()
                    .to_str()
                    .ok()
                    .map(std::string::ToString::to_string)
            })
            .unwrap_or_else(|| "HEAD".to_string());

        // Build stash message
        let message = if let Some(msg) = opts.message {
            format!("WIP on {}: {}", branch_name, msg)
        } else {
            format!("WIP on {}", branch_name)
        };

        // Get working directory
        let work_dir = repo_clone
            .workdir()
            .ok_or_else(|| GitError::InvalidInput("Repository has no working directory".to_string()))?;

        // Create stash via git stash command
        // (Using command-line as gix doesn't have direct stash API)
        let mut cmd = std::process::Command::new("git");
        cmd.arg("stash")
            .arg("push")
            .arg("-m")
            .arg(&message)
            .current_dir(work_dir);

        if opts.include_untracked {
            cmd.arg("-u");
        }

        let output = cmd
            .output()
            .map_err(|e| GitError::InvalidInput(format!("Failed to run git stash: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::InvalidInput(format!(
                "Stash failed: {}",
                stderr
            )));
        }

        // Get the stash commit hash
        let list_output = std::process::Command::new("git")
            .arg("stash")
            .arg("list")
            .arg("-1")
            .arg("--format=%H")
            .current_dir(work_dir)
            .output()
            .map_err(|e| GitError::InvalidInput(format!("Failed to get stash info: {}", e)))?;

        let list_str = String::from_utf8_lossy(&list_output.stdout);
        let commit_hash = list_str.trim().to_string();

        if commit_hash.is_empty() {
            return Err(GitError::InvalidInput(
                "Failed to retrieve stash commit hash".to_string(),
            ));
        }

        Ok(StashInfo {
            name: "stash@{0}".to_string(),
            message,
            commit_hash,
        })
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}

/// Apply and remove stash entry
pub async fn stash_pop(repo: RepoHandle, stash_name: Option<&str>) -> GitResult<()> {
    let repo_clone = repo.clone_inner();
    let stash_name = stash_name.unwrap_or("stash@{0}").to_string();

    tokio::task::spawn_blocking(move || {
        // Get working directory
        let work_dir = repo_clone
            .workdir()
            .ok_or_else(|| GitError::InvalidInput("Repository has no working directory".to_string()))?;

        let output = std::process::Command::new("git")
            .arg("stash")
            .arg("pop")
            .arg(&stash_name)
            .current_dir(work_dir)
            .output()
            .map_err(|e| GitError::InvalidInput(format!("Failed to pop stash: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::InvalidInput(format!(
                "Failed to pop stash: {}",
                stderr
            )));
        }

        Ok(())
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))?
}
