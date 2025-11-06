//! Git push operations
//!
//! Provides functionality for pushing commits and tags to remote repositories.
//! Uses native git CLI since gix doesn't yet support push operations.
//!
//! **Dependency**: Requires git to be installed and available in PATH.
//!
//! # Authentication
//!
//! This module relies on git's configured authentication methods. Authentication
//! configuration is critical for production use to avoid hangs, timeouts, and failures.
//!
//! ## SSH (Recommended for automation)
//!
//! SSH authentication requires keys to be properly configured and loaded:
//! - SSH keys must be loaded in ssh-agent
//! - Or SSH key must not have a passphrase
//! - Respects user's `~/.ssh/config` settings
//! - Environment variable: `SSH_AUTH_SOCK` (set by ssh-agent)
//!
//! **Setup for CI/CD:**
//! ```bash
//! # Start SSH agent and add key
//! eval "$(ssh-agent -s)"
//! ssh-add ~/.ssh/id_rsa
//!
//! # Or disable strict host checking for CI
//! export GIT_SSH_COMMAND="ssh -o StrictHostKeyChecking=no"
//! ```
//!
//! ## HTTPS
//!
//! HTTPS authentication requires credential configuration:
//! - Credential helper: `git config --global credential.helper store`
//! - Or environment variables: `GIT_ASKPASS`, `GIT_USERNAME`, `GIT_PASSWORD`
//! - Or credentials stored in git config (not recommended for security)
//!
//! **WARNING**: HTTPS push will fail with an error if credentials are needed
//! because `GIT_TERMINAL_PROMPT=0` is set by this implementation to prevent
//! hanging on password prompts in automation scenarios.
//!
//! ## Preventing Hangs in CI/CD
//!
//! This implementation sets `GIT_TERMINAL_PROMPT=0` to prevent git from prompting
//! for credentials, which would cause indefinite hangs in automated environments.
//! If authentication is not properly configured, the push will fail immediately
//! rather than hang.
//!
//! **Recommended practices:**
//! ```rust
//! use std::env;
//!
//! // For SSH in CI/CD environments
//! env::set_var("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=no");
//!
//! // For HTTPS with credential helper
//! // Run: git config --global credential.helper store
//! // Or set GIT_ASKPASS to a script that provides credentials
//! ```
//!
//! # Examples
//!
//! ## CI/CD Setup (GitHub Actions)
//!
//! ```yaml
//! # Using SSH with GitHub Actions
//! - name: Setup SSH
//!   uses: webfactory/ssh-agent@v0.5.4
//!   with:
//!     ssh-private-key: ${{ secrets.SSH_PRIVATE_KEY }}
//!
//! # Using HTTPS with personal access token
//! - name: Configure Git Credentials
//!   run: |
//!     git config --global credential.helper store
//!     echo "https://${{ secrets.GITHUB_TOKEN }}@github.com" > ~/.git-credentials
//! ```
//!
//! ## Local Development
//!
//! ```bash
//! # SSH authentication (recommended)
//! eval "$(ssh-agent -s)"
//! ssh-add ~/.ssh/id_rsa
//!
//! # HTTPS with credential helper
//! git config --global credential.helper store
//! # First push will prompt for credentials, then store them
//!
//! # HTTPS with personal access token
//! git config --global credential.helper 'store --file ~/.git-credentials'
//! echo "https://username:token@github.com" > ~/.git-credentials
//! chmod 600 ~/.git-credentials
//! ```
//!
//! # Troubleshooting
//!
//! ## "Failed to authenticate" or "Permission denied"
//! - Verify SSH key is loaded: `ssh-add -l`
//! - Test SSH connection: `ssh -T git@github.com`
//! - Check credential helper: `git config --get credential.helper`
//!
//! ## "Operation timed out"
//! - Check network connectivity to remote
//! - Verify remote URL is correct: `git remote -v`
//! - Increase timeout via `PushOpts.timeout_secs`
//!
//! ## "Authentication required" in CI/CD
//! - Ensure SSH key is added to ssh-agent in CI workflow
//! - Or configure HTTPS credential helper before push
//! - Verify `GIT_SSH_COMMAND` or `GIT_ASKPASS` environment variables
//!
//! For more details, see:
//! - [Git Credential Storage](https://git-scm.com/docs/gitcredentials)
//! - [Git SSH Configuration](https://git-scm.com/docs/git-config#Documentation/git-config.txt-coresshCommand)

mod core;
mod delete;
mod check;

pub use core::{push, push_current_branch, push_tags};
pub use delete::{delete_remote_tag, delete_remote_branch};
pub use check::{check_remote_branch_exists, check_remote_tag_exists};

/// Options for push operation
#[derive(Debug, Clone)]
pub struct PushOpts {
    /// Remote name (defaults to "origin")
    pub remote: String,
    /// Refspecs to push (empty means current branch)
    pub refspecs: Vec<String>,
    /// Force push
    pub force: bool,
    /// Push all tags
    pub tags: bool,
    /// Timeout in seconds (default: 300)
    pub timeout_secs: Option<u64>,
}

impl Default for PushOpts {
    fn default() -> Self {
        Self {
            remote: "origin".to_string(),
            refspecs: Vec::new(),
            force: false,
            tags: false,
            timeout_secs: None,
        }
    }
}

/// Result of push operation
#[derive(Debug, Clone)]
pub struct PushResult {
    /// Number of refs (branches/tags) successfully pushed
    ///
    /// Note: This counts the number of ref updates, not individual commits.
    /// For example, pushing a branch with 5 commits counts as 1 ref update.
    pub commits_pushed: usize,

    /// Number of tags pushed (conservative estimate)
    ///
    /// **Note:** Returns 1 when `--tags` is used and push succeeds, or counts
    /// the number of `refs/tags/*` refspecs provided. Does not parse git output
    /// for exact count due to fragility. Sufficient for most telemetry use cases.
    pub tags_pushed: usize,

    /// Any warnings or messages
    pub warnings: Vec<String>,
}
