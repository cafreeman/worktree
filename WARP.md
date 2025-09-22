# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Quick Commands

### Build
- `cargo build` - Standard debug build
- `cargo build --release` - Optimized release build

### Test  
- `cargo test` - Run all tests
- `cargo test --test create_tests` - Run specific integration test file
- `cargo test commands::create` - Run specific module tests
- `cargo test -- --ignored` - Run ignored tests

### Code Quality
- `cargo fmt --all` - Format code
- `cargo fmt --all --check` - Check formatting
- `cargo clippy --all-targets --all-features -- -D warnings` - Run linter
- `cargo check --all-targets --all-features` - Quick syntax check

### Run Binary
- `cargo run -- list` - Run via cargo (note the `--`)
- `cargo run -- create feature/my-branch` - Create worktree example
- `./target/release/worktree-bin list` - Run release binary directly

## Architecture

### Module Structure
- `src/main.rs` - CLI entry point using clap, dispatches to command modules
- `src/lib.rs` - Library crate root, exposes modules and `Result` type
- `src/traits.rs` - `GitOperations` trait for testability and abstraction
- `src/commands/` - Individual command implementations (create, list, remove, etc.)
- `src/storage/` - Manages `~/.worktrees/` directory with branch name sanitization
- `src/config/` - `WorktreeConfig` for `.worktree-config.toml` file copy patterns  
- `src/git/` - `GitRepo` wrapper around git2 crate, implements `GitOperations`
- `src/selection.rs` - Interactive selection prompts abstracted for testing

### Key Design Patterns
- **Trait-based git operations**: `GitOperations` trait enables mocking for tests
- **Centralized storage**: All worktrees stored under `~/.worktrees/<repo>/<branch>/`
- **Branch name sanitization**: Converts `feature/auth` to `feature-auth` for filesystem safety
- **Configuration-driven file copying**: Uses glob patterns from `.worktree-config.toml`
- **Origin tracking**: Stores origin repository paths for back navigation

### Data Flow
CLI parsing → command dispatch → commands orchestrate `GitRepo` and `WorktreeStorage` using `WorktreeConfig` → shell integration outputs navigation commands.

### Testing Approach
- Integration tests in `tests/` using `tempfile` and `temp-env` for isolation
- Test helpers in `tests/helpers/` for common setup patterns
- All git operations use temporary repositories, never user's real repos

## Configuration

### Clippy Lints
- `unwrap_used = "deny"` and `panic = "deny"` enforced in `Cargo.toml`
- All errors must be handled gracefully with `anyhow::Result`

### MSRV
- Minimum Supported Rust Version: 1.70.0 (configured in `clippy.toml`)

### Binary Names
- Binary name: `worktree-bin` (shell wrapper exists for directory navigation)
- Shell integration generates wrapper functions for bash/zsh/fish

## Release Management

### Tooling
- Uses `cargo-release` with configuration in `release.toml`
- Publishing is manual only - never auto-publishes

### Workflow
1. **Pre-release checks**: 
   - `cargo fmt --all --check`
   - `cargo clippy --all-targets --all-features -- -D warnings` 
   - `cargo test --all-features`
   - `cargo build --release`
2. **Version and tag**: `cargo release patch|minor|major`
3. **Manual publish**: `cargo publish` (after version bump completes)

## Safety and Development Practices

### Error Handling
- No `unwrap()` or `panic!()` in production code paths (enforced by Clippy)
- All errors surfaced with actionable messages via `anyhow`

### Path Safety
- Branch name sanitization for filesystem compatibility
- All worktree operations scoped to `~/.worktrees/` directory
- Origin tracking prevents orphaned worktree references

### Shell Integration  
- Binary outputs shell commands for evaluation, never mutates shell environment directly
- Directory navigation requires shell wrapper for `jump`/`back` commands

### Testing Isolation
- Tests use isolated temporary directories and git repositories
- Never operate against user's actual repositories during testing
- Comprehensive coverage using `tempfile` and `temp-env` crates