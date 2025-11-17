//! Git remote operations

use crate::{GitError, GitResult, RepoHandle};
use gix::bstr::ByteSlice;
use gix::config::parse::section::ValueName;
use std::borrow::Cow;

/// Options for adding a remote
#[derive(Debug, Clone)]
pub struct RemoteAddOpts {
    /// Remote name (e.g., "origin", "upstream")
    pub name: String,
    /// Remote URL (https, git, ssh, or file URL)
    pub url: String,
    /// Force add (overwrite if exists)
    pub force: bool,
}

/// Add a new remote to repository configuration
pub async fn add_remote(repo: RepoHandle, opts: RemoteAddOpts) -> GitResult<()> {
    let mut repo_clone = repo.clone_inner();

    tokio::task::spawn_blocking(move || {
        // Validate URL format
        if !is_valid_git_url(&opts.url) {
            return Err(GitError::InvalidInput(format!(
                "Invalid Git URL format: {}",
                opts.url
            )));
        }

        // Check if remote exists
        if !opts.force
            && repo_clone
                .find_remote(opts.name.as_bytes().as_bstr())
                .is_ok()
        {
            return Err(GitError::InvalidInput(format!(
                "Remote '{}' already exists",
                opts.name
            )));
        }

        // Add remote via config
        let mut config = repo_clone.config_snapshot_mut();

        // Create remote section with remote name as subsection
        let mut section = config
            .new_section("remote", Some(Cow::Owned(opts.name.clone().into())))
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        // Set remote.<name>.url = <url>
        let url_key = ValueName::try_from("url").map_err(|e| GitError::Gix(Box::new(e)))?;
        section.push(url_key, Some(opts.url.as_bytes().as_bstr()));

        // Set remote.<name>.fetch = +refs/heads/*:refs/remotes/<name>/*
        let fetch_key = ValueName::try_from("fetch").map_err(|e| GitError::Gix(Box::new(e)))?;
        let refspec = format!("+refs/heads/*:refs/remotes/{}/*", opts.name);
        section.push(fetch_key, Some(refspec.as_bytes().as_bstr()));

        // Commit the config changes
        drop(section);
        config
            .commit()
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        Ok(())
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))??;

    Ok(())
}

/// Remove a remote from repository configuration
pub async fn remove_remote(repo: RepoHandle, name: &str) -> GitResult<()> {
    let mut repo_clone = repo.clone_inner();
    let name = name.to_string();

    tokio::task::spawn_blocking(move || {
        // Check if remote exists
        if repo_clone
            .find_remote(name.as_bytes().as_bstr())
            .is_err()
        {
            return Err(GitError::InvalidInput(format!(
                "Remote '{}' does not exist",
                name
            )));
        }

        // Remove remote via config
        let mut config = repo_clone.config_snapshot_mut();

        let section_name = format!("remote.{}", name);

        // Remove all keys under the remote section
        if config.remove_section(&section_name, None).is_none() {
            return Err(GitError::InvalidInput(format!(
                "Remote '{}' not found in configuration",
                name
            )));
        }

        // Commit the config changes
        config
            .commit()
            .map_err(|e| GitError::Gix(Box::new(e)))?;

        Ok(())
    })
    .await
    .map_err(|e| GitError::Gix(Box::new(e)))??;

    Ok(())
}

/// Validate Git URL format
fn is_valid_git_url(url: &str) -> bool {
    url.starts_with("https://")
        || url.starts_with("http://")
        || url.starts_with("git://")
        || url.starts_with("ssh://")
        || url.starts_with("file://")
        || (url.contains('@') && url.contains(':')) // SSH format like git@github.com:user/repo.git
}
