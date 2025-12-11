//! Centralized authentication and git configuration for network operations
//!
//! Single source of truth for:
//! - Reading user's git configuration (SSH, credentials) via git binary
//! - Configuring gix clone operations with proper auth
//! - Running authenticated git CLI commands (push, ls-remote, delete)
//! - Generating helpful error messages for auth failures

use std::path::PathBuf;
use std::process::{Output, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;

use crate::{GitError, GitResult};

// ============================================================================
// Git Config Reader (Cached)
// ============================================================================

/// Cached git configuration for auth-related settings
static GIT_CONFIG: OnceLock<GitConfig> = OnceLock::new();

/// Git configuration relevant for authentication
#[derive(Debug, Clone, Default)]
pub struct GitConfig {
    /// Custom SSH command (core.sshCommand)
    pub ssh_command: Option<String>,
    /// SSH program variant (ssh.variant)
    pub ssh_variant: Option<String>,
    /// Credential helper (credential.helper)
    pub credential_helper: Option<String>,
}

impl GitConfig {
    /// Read git configuration from the system using git binary
    fn read() -> Self {
        let mut config = GitConfig::default();

        // Only read config if git is available
        if !git_available() {
            return config;
        }

        config.ssh_command = git_config_get("core.sshCommand");
        config.ssh_variant = git_config_get("ssh.variant");
        config.credential_helper = git_config_get("credential.helper");

        config
    }

    /// Convert to gix config override format: ["key=value", ...]
    ///
    /// These overrides are passed to `PrepareFetch::with_in_memory_config_overrides()`
    pub fn to_gix_overrides(&self) -> Vec<String> {
        let mut overrides = Vec::new();

        if let Some(ref v) = self.ssh_command {
            overrides.push(format!("core.sshCommand={v}"));
        }
        if let Some(ref v) = self.ssh_variant {
            overrides.push(format!("ssh.variant={v}"));
        }
        // Note: credential.helper is for HTTPS, gix handles this via askpass
        if let Some(ref v) = self.credential_helper {
            overrides.push(format!("credential.helper={v}"));
        }

        overrides
    }
}

/// Get cached git configuration (reads once, caches forever)
pub fn get_config() -> &'static GitConfig {
    GIT_CONFIG.get_or_init(GitConfig::read)
}

/// Check if git binary is available
pub fn git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Read a single git config value using git binary
fn git_config_get(key: &str) -> Option<String> {
    std::process::Command::new("git")
        .args(["config", "--get", key])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// ============================================================================
// gix Configuration (clone)
// ============================================================================

/// Configure a gix clone operation with auth settings
///
/// Call this after `gix::prepare_clone()` to inject user's git config.
///
/// Why this is needed: `gix::prepare_clone()` creates a fresh repo that has
/// NO access to the user's global git config. This function reads the user's
/// config via the git binary and injects it as in-memory overrides.
pub fn configure_clone(prepare: gix::clone::PrepareFetch) -> gix::clone::PrepareFetch {
    let overrides = get_config().to_gix_overrides();
    if overrides.is_empty() {
        prepare
    } else {
        prepare.with_in_memory_config_overrides(overrides)
    }
}

// ============================================================================
// Git CLI Wrapper (push, ls-remote, delete)
// ============================================================================

/// Options for running an authenticated git command
#[derive(Debug, Clone)]
pub struct GitCommandOpts {
    /// Working directory for the command
    pub work_dir: PathBuf,
    /// Timeout in seconds (default: 300)
    pub timeout_secs: u64,
}

impl GitCommandOpts {
    /// Create options with work_dir and default timeout
    pub fn new(work_dir: PathBuf) -> Self {
        Self {
            work_dir,
            timeout_secs: 300,
        }
    }

    /// Set timeout in seconds
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// Run an authenticated git command with proper environment setup
///
/// This is the SINGLE PLACE where git CLI commands are executed.
/// Consolidates duplicated code from push/core.rs, push/check.rs, push/delete.rs
///
/// Handles:
/// - Setting GIT_TERMINAL_PROMPT=0 to prevent hanging on credential prompts
/// - Setting LC_ALL=C for consistent output parsing
/// - Timeout handling with proper child process cleanup
/// - Auth error detection and helpful messaging
pub async fn run_git_command(args: &[&str], opts: GitCommandOpts) -> GitResult<Output> {
    let timeout_duration = Duration::from_secs(opts.timeout_secs);

    let mut cmd = TokioCommand::new("git");
    cmd.current_dir(&opts.work_dir);
    cmd.args(args);

    // Prevent credential prompts from hanging in automation
    cmd.env("GIT_TERMINAL_PROMPT", "0");

    // Force English output for consistent parsing
    cmd.env("LC_ALL", "C");
    cmd.env("LANG", "C");

    // Capture output
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Spawn child process
    let mut child = cmd.spawn().map_err(GitError::Io)?;

    // Wait with timeout
    let status = tokio::select! {
        result = child.wait() => result.map_err(GitError::Io)?,
        () = tokio::time::sleep(timeout_duration) => {
            let _ = child.kill().await;
            return Err(GitError::InvalidInput(format!(
                "Git operation timed out after {} seconds", opts.timeout_secs
            )));
        }
    };

    // Read output
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_end(&mut stdout).await;
    }
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_end(&mut stderr).await;
    }

    let output = Output { status, stdout, stderr };

    // Check for auth errors and provide helpful message
    if !output.status.success() {
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        if is_auth_error(&stderr_str) {
            let url = args
                .iter()
                .find(|a| a.contains('@') || a.starts_with("http"))
                .map(|s| s.to_string())
                .unwrap_or_else(|| "remote".to_string());
            return Err(GitError::InvalidInput(auth_error_message(&url)));
        }
    }

    Ok(output)
}

/// Check if an error message indicates an authentication failure
fn is_auth_error(stderr: &str) -> bool {
    let s = stderr.to_lowercase();
    s.contains("authentication")
        || s.contains("permission denied")
        || s.contains("could not read username")
        || s.contains("could not read password")
        || s.contains("host key verification failed")
        || s.contains("repository not found") // Often means no access
}

// ============================================================================
// Error Messages
// ============================================================================

/// Generate helpful error message for authentication failures
pub fn auth_error_message(url: &str) -> String {
    let is_ssh = url.contains("git@") || url.starts_with("ssh://");

    if is_ssh {
        format!(
            r#"SSH authentication failed for '{url}'.

Setup SSH authentication:

1. Ensure SSH key exists:
   ls ~/.ssh/id_ed25519 || ssh-keygen -t ed25519

2. Start ssh-agent and add key:
   eval "$(ssh-agent -s)"
   ssh-add ~/.ssh/id_ed25519

3. Add public key to Git host (GitHub/GitLab/Bitbucket)

4. Test connection:
   ssh -T git@github.com

CI/CD: export GIT_SSH_COMMAND="ssh -o StrictHostKeyChecking=no"
"#
        )
    } else {
        format!(
            r#"HTTPS authentication failed for '{url}'.

Setup credential helper:

macOS:   git config --global credential.helper osxkeychain
Windows: git config --global credential.helper manager
Linux:   git config --global credential.helper store

CI/CD with token:
  echo "https://TOKEN@github.com" > ~/.git-credentials
  git config --global credential.helper store
"#
        )
    }
}
