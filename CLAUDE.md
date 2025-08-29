# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust CLI application called "worktree" that manages git worktrees with enhanced features including centralized storage, automatic config file synchronization, and intelligent branch management. The binary is named `worktree-bin` and includes shell integration for directory navigation.

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
- **commands/**: Individual command implementations (create, list, remove, status, sync_config, init, jump)
- **storage/**: Manages worktree storage in `~/.worktrees/<repo>/<branch>/` with branch name sanitization and mapping
- **config/**: Handles `.worktree-config.toml` files for customizing file copy patterns
- **git/**: Git operations wrapper using git2 crate, implements GitOperations trait
- **traits.rs**: Defines GitOperations trait for testability and abstraction

### Key Design Patterns
- **Trait-based abstraction**: GitOperations trait enables mocking for tests
- **Centralized storage**: All worktrees stored under `~/.worktrees/` with predictable structure
- **Branch name sanitization**: Converts `feature/auth` to `feature-auth` for filesystem compatibility, maintains mapping
- **Configuration-driven file copying**: Uses glob patterns from `.worktree-config.toml` or sensible defaults
- **Shell integration**: Generates shell functions for directory navigation and completions

### Core Components
- **WorktreeStorage**: Manages the `~/.worktrees/` directory structure and branch name mapping
- **WorktreeConfig**: Loads and manages copy patterns from `.worktree-config.toml`
- **GitRepo**: Wraps git2 operations for worktree management
- **Shell Integration**: Generates bash/zsh/fish functions for `worktree` command wrapper

### Testing
- Unit tests in `tests/` directory with comprehensive coverage
- Uses `tempfile` and `temp-env` for isolated test environments
- Test helpers in `test_helpers.rs` for common setup

### Clippy Configuration
Strict linting is enforced with:
- `unwrap_used = "deny"` and `panic = "deny"` to prevent runtime panics
- Performance lints for efficient code
- Documentation requirements for error handling
- MSRV set to 1.70.0

The codebase prioritizes safety, error handling, and maintainability with comprehensive error documentation and trait-based design for testability.
- Clean up any stray files, directories, or git branches that you create while testing