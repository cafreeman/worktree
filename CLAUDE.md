# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust CLI application called "worktree" that manages git worktrees with enhanced features including centralized storage in `~/.worktrees/`, automatic config file synchronization, intelligent branch management, and back navigation. The binary is named `worktree-bin` and includes shell integration for directory navigation.

## Development Commands

### Build and Test
```bash
# Build the project
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Run specific test module
cargo test test_module_name

# Check code without building
cargo check
```

### Code Quality
```bash
# Run Clippy linter (configured with strict rules in Cargo.toml and clippy.toml)
cargo clippy

# Run formatter
cargo fmt

# Run all lints and checks
cargo clippy && cargo fmt --check
```

### Binary Usage
```bash
# Run the binary directly
cargo run -- <command>

# Or after building
./target/release/worktree-bin <command>
```

## Architecture

### Module Structure
- **main.rs**: CLI entry point using clap for argument parsing, dispatches to command modules
- **lib.rs**: Library crate root, exposes all modules and the main `Result` type from anyhow
- **commands/**: Individual command implementations (create, list, remove, status, sync_config, init, jump, back, cleanup)
- **storage/**: Manages worktree storage in `~/.worktrees/<repo>/<feature-name>/` with feature name validation and origin tracking
- **config/**: Handles `.worktree-config.toml` files for customizing copy patterns, symlink patterns, and on-create hooks
- **git/**: Git operations wrapper using git2 crate, implements GitOperations trait
- **traits.rs**: Defines GitOperations trait for testability and abstraction

### Key Design Patterns
- **Trait-based abstraction**: GitOperations trait enables mocking for tests
- **Centralized storage**: All worktrees stored under `~/.worktrees/` with predictable structure (no custom paths)
- **Feature-name-as-identity**: Worktrees are identified by a user-supplied feature name (the directory name), decoupled from the branch name. No branch name sanitization or mapping is performed.
- **Configuration-driven file management**: Uses glob patterns from `.worktree-config.toml` for copying, symlinking, and post-create hooks; falls back to sensible defaults
- **Origin tracking**: Stores origin repository paths for back navigation in `.worktree-origins` metadata files
- **Shell integration**: Generates shell functions for directory navigation and completions

### Core Components
- **WorktreeStorage**: Manages the `~/.worktrees/` directory structure, feature name validation, and origin tracking
- **WorktreeConfig**: Loads and manages copy patterns, symlink patterns, and on-create hooks from `.worktree-config.toml`
- **GitRepo**: Wraps git2 operations for worktree management
- **Shell Integration**: Generates bash/zsh/fish functions for `worktree` command wrapper with `jump` and `back` navigation

### Testing
- Unit tests in `tests/` directory with comprehensive coverage
- Uses `assert_fs`, `assert_cmd`, and `temp-env` for isolated test environments
- Test helpers in `tests/test-support/` crate for common setup

### Clippy Configuration
Strict linting is enforced with:
- `unwrap_used = "deny"` and `panic = "deny"` to prevent runtime panics
- Performance lints for efficient code
- Documentation requirements for error handling
- MSRV set to 1.70.0

The codebase prioritizes safety, error handling, and maintainability with comprehensive error documentation and trait-based design for testability.

## Release Management

### cargo-release Setup
The project uses `cargo-release` for automated version management and release preparation. Configuration is in `release.toml`.

**IMPORTANT:** The release configuration is set to NEVER auto-publish. All releases must be done manually by the maintainer.

### Release Workflow
1. **Version Bumping**: Use `cargo release <level>` where level is `patch`, `minor`, or `major`
2. **Manual Publishing**: After version bump, manually run `cargo publish` if desired
3. **Changelog**: Update `CHANGELOG.md` following Keep a Changelog format

### Safety Features
- Manual initiation only - releases must be explicitly triggered by maintainer
- Pre-release hooks run quality checks (fmt, clippy, test, build)
- Automatic git tagging and pushing
- Automatic publishing to crates.io once initiated

### Release Process
When ready to publish a new version:
```bash
# Update CHANGELOG.md first, then run:
cargo release patch  # or minor/major

# This will automatically:
# 1. Update version in Cargo.toml
# 2. Run quality checks (fmt, clippy, test, build)  
# 3. Create release commit and git tag
# 4. Push to git remote
# 5. Publish to crates.io
```
- Clean up any stray files, directories, or git branches that you create while testing
- Never release this package on your own