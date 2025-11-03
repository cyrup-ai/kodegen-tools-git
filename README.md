# kodegen_tools_git

[![License: Apache 2.0 OR MIT](https://img.shields.io/badge/license-Apache%202.0%20OR%20MIT-blue.svg)](LICENSE.md)
[![Crates.io](https://img.shields.io/crates/v/kodegen_tools_git)](https://crates.io/crates/kodegen_tools_git)

Memory-efficient, blazing-fast Git operations for AI code generation agents via Model Context Protocol (MCP).

Part of the [KODEGEN.ai](https://kodegen.ai) ecosystem.

## Features

- **🚀 High Performance**: Built on [gix](https://github.com/Byron/gitoxide) (Gitoxide), a fast Rust Git implementation
- **⚡ Async-First**: Fully asynchronous API with tokio integration
- **🔧 MCP Native**: 20+ tools implementing the Model Context Protocol for AI agents
- **💡 Ergonomic API**: Builder patterns and strongly-typed interfaces
- **🔒 Type Safe**: Comprehensive error handling with domain-specific error types
- **📦 Zero-Config**: Stateless tools with minimal setup required

## Available Git Tools

### Repository Operations
- `git_init` - Initialize new repositories
- `git_open` - Open existing repositories
- `git_clone` - Clone remote repositories
- `git_discover` - Discover repository from any path

### Branch Management
- `git_branch_create` - Create new branches
- `git_branch_delete` - Delete branches
- `git_branch_list` - List all branches
- `git_branch_rename` - Rename branches

### Core Operations
- `git_add` - Stage files for commit
- `git_commit` - Create commits with full metadata
- `git_checkout` - Switch branches or restore files
- `git_log` - View commit history with streaming support

### Remote Operations
- `git_fetch` - Fetch from remotes
- `git_merge` - Merge branches

### Worktree Management
- `git_worktree_add` - Create linked worktrees
- `git_worktree_remove` - Remove worktrees
- `git_worktree_list` - List all worktrees
- `git_worktree_lock` - Lock worktrees
- `git_worktree_unlock` - Unlock worktrees
- `git_worktree_prune` - Prune stale worktrees

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
kodegen_tools_git = "0.1"
```

## Usage

### As a Library

```rust
use kodegen_tools_git::{open_repo, CommitOpts, Signature};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Open a repository
    let repo = open_repo("/path/to/repo").await??;

    // Create a commit with builder pattern
    let commit_id = kodegen_tools_git::commit(
        repo,
        CommitOpts::message("feat: add new feature")
            .all(true)
            .author(Signature::new("Your Name", "you@example.com"))
    ).await?;

    println!("Created commit: {}", commit_id);
    Ok(())
}
```

### As an MCP Server

The binary runs an HTTP server exposing all Git tools via MCP:

```bash
cargo run --bin kodegen-git
```

The server typically runs on port 30450 and is managed by the `kodegend` daemon.

### Using MCP Tools

Connect via MCP client:

```rust
use kodegen_mcp_client::tools;
use serde_json::json;

// Call git_commit tool
let result = client.call_tool(
    tools::GIT_COMMIT,
    json!({
        "path": "/path/to/repo",
        "message": "feat: add feature",
        "all": true
    })
).await?;
```

## Examples

See the `examples/` directory for comprehensive demonstrations:

```bash
# Run the full Git demo (20+ operations)
cargo run --example git_demo

# Run direct API usage example
cargo run --example direct_comprehensive
```

## Architecture

### Three-Layer Design

1. **Operations Layer** (`src/operations/`)
   - Pure Git logic using `gix`
   - Builder patterns for ergonomic configuration
   - Async functions wrapping blocking operations

2. **Tools Layer** (`src/tools/`)
   - MCP tool wrappers
   - JSON schema validation
   - Protocol bridging

3. **Runtime Layer** (`src/runtime/`)
   - Async task execution
   - Streaming results
   - Progress reporting

### RepoHandle

The `RepoHandle` type provides cheap cloning for thread-safe repository access:

```rust
let repo = open_repo("/path/to/repo").await??;
let repo_clone = repo.clone(); // Cheap! Shares underlying data
```

This enables safe concurrent operations while maintaining the performance benefits of `gix`.

## Development

### Build

```bash
cargo build
```

### Test

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Lint

```bash
cargo clippy --no-deps
```

## Performance

Built on `gix` (Gitoxide), this library provides:

- **Fast object access**: Efficient ODB operations
- **Minimal allocations**: Zero-copy where possible
- **Concurrent operations**: Cheap repository cloning enables parallelism
- **Streaming logs**: Memory-efficient commit history traversal

## Contributing

Contributions are welcome! Please ensure:

- Tests pass: `cargo test`
- Code is formatted: `cargo fmt`
- Clippy is happy: `cargo clippy`

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE.md) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE.md) or http://opensource.org/licenses/MIT)

at your option.

## Links

- [KODEGEN.ai](https://kodegen.ai) - Homepage
- [GitHub Repository](https://github.com/cyrup-ai/kodegen-tools-git)
- [Gitoxide](https://github.com/Byron/gitoxide) - The underlying Git implementation
- [MCP Specification](https://modelcontextprotocol.io) - Model Context Protocol

---

Built with ❤️ by [KODEGEN.ai](https://kodegen.ai)
