# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Remove: `--force-delete-branch` flag to allow deleting unmanaged branches

### Fixed

- Remove: Resolve canonical branch from worktree HEAD before pruning to avoid
  intermittent branch NotFound errors when mapping is missing or sanitized names
  differ from canonical names
- Remove: Fallback to storage mapping when HEAD resolution fails; otherwise
  skip deletion with a clear warning
- Remove: Clean up branch mapping and origin metadata after successful deletion

### Changed

- Remove: By default, skip deleting branches not managed by this CLI; require
  `--force-delete-branch` to delete unmanaged branches

## [0.3.1] - 2025-09-04

### Added

- Managed branch tracking to safely identify branches created via this CLI:
  - New per-branch markers stored under `~/.worktrees/<repo>/.managed-branches/`
  - Tracking is resilient and written only after successful creation

### Changed

- Cleanup behavior is now safe and precise:
  - Only deletes orphan branches that were created by this CLI (managed)
  - Preserves independent branches that were not created via `worktree`
  - Prunes orphaned worktree directories and associated metadata if the git branch was deleted externally
  - Continues to remove orphaned git worktree references

### Tests

- Added integration tests covering selective branch deletion and orphan directory pruning

## [0.3.0] - 2025-09-02

### Added

- **Enhanced Completion System:**

  - Added intelligent tab completion for `remove` command with worktree name suggestions
  - Implemented `--list-completions` flag for `remove` command to support shell completion
  - Enhanced bash, zsh, and fish shell integration with smart worktree name completion
  - Added interactive mode trigger on empty tab completion for `remove` command

- **Selection Provider Architecture:**

  - Created `SelectionProvider` trait for abstracting interactive prompts
  - Implemented `RealSelectionProvider` for production use with `inquire::Select`
  - Added `MockSelectionProvider` for comprehensive testing of interactive functionality
  - Enhanced testability of interactive CLI features

- **Enhanced Configuration System:**
  - Improved configuration structures with optional fields and better error handling
  - Added comprehensive configuration loading and merging behavior validation
  - Enhanced documentation for flexible configuration system with precedence rules

### Changed

- **Interactive Command Improvements:**

  - Refactored `jump` and `remove` commands to support provider-based selection
  - Enhanced `remove` command with interactive selection when no target specified
  - Improved completion handling with current repository filtering options
  - Added support for `--current` flag in remove completions

- **Testing Infrastructure Modernization:**

  - Migrated all tests to modern patterns using `assert_fs`, `assert_cmd`, and `rexpect`
  - Replaced manual temporary directory management with declarative filesystem testing
  - Implemented comprehensive interactive CLI testing with `rexpect` for user prompts
  - Added 89 modern tests across 10 test modules with full command coverage
  - Removed legacy testing infrastructure and eliminated test duplication
  - Established modern testing patterns for all future CLI development
  - Split test helpers into focused modules with dedicated test-support crate
  - Eliminated all dead code warnings across test suite

- **Rust 2024 Edition Migration:**
  - Updated project to use Rust 2024 edition for latest language features and improvements

### Fixed

- **Critical Bug Fix in Remove Command:**
  - Fixed dangerous fallback behavior in `remove` command that could incorrectly use sanitized branch names instead of canonical branch names
  - Resolved issue where missing or corrupted branch mapping files caused git operations to fail silently
  - Enhanced branch name resolution logic to safely distinguish canonical vs sanitized names
  - Added git repository verification before assuming branch names are canonical
  - Improved error handling to provide clear guidance when branch mappings are corrupted

## [0.2.0] - 2025-08-29

### Added

- **New Commands:**
  - `worktree back` - Navigate back to original repository from worktrees
  - `worktree cleanup` - Clean up orphaned branches and git references
- **Enhanced Shell Integration:**
  - Better completion support for all commands
  - Native bash/zsh/fish shell functions with improved directory changing
- **Origin Tracking & Cleanup:**
  - Automatic storage of origin repository paths in `.worktree-origins` metadata
  - Proper cleanup of origin information when worktrees are removed
  - Back navigation support that works reliably for all worktrees
- **Cargo Release Infrastructure:**
  - Initial cargo-release configuration for automated publishing
  - Streamlined release workflow with version management

### Changed

- **Smart Branch/Worktree Synchronization (BREAKING CHANGES):**
  - `create` command now automatically creates branches when needed (removed `-b/--create-branch` flag)
  - Added `--new-branch` flag to force new branch creation (fails if branch exists)
  - Added `--existing-branch` flag to require existing branch (fails if branch doesn't exist)
  - `remove` command now deletes branches by default (removed `-d/--delete-branch` flag)
  - Added `--keep-branch` flag to preserve branches when removing worktrees
- **Simplified Architecture:**
  - All worktrees now use centralized storage exclusively in `~/.worktrees/<repo>/<branch>/`
  - Improved mental model: one worktree = one branch
  - Atomic operations with comprehensive pre-flight validation

### Removed

- **Custom Path Support:**
  - Removed `-p/--path` CLI option from create command
  - Removed custom path handling from remove command logic
  - Simplified codebase with consistent worktree management

### Fixed

- Fixed "reference already exists" error when creating worktrees
- Resolved branch/worktree synchronization issues where they could get out of sync
- Enhanced error messages that guide users to correct solutions
- Proper cleanup of git references and filesystem state

## [0.1.0] - 2024-08-29

### Added

- Initial release of worktree CLI
- **Core Commands:**

  - `create` - Create new worktrees with intelligent branch handling
  - `list` - List all managed worktrees
  - `remove` - Remove worktrees with optional branch deletion
  - `status` - Show comprehensive worktree and git status
  - `jump` - Navigate to worktree directories with interactive selection
  - `back` - Navigate back to original repository from worktrees
  - `cleanup` - Clean up orphaned branches and git references
  - `sync-config` - Sync configuration files between worktrees

- **Key Features:**

  - 🏠 **Centralized Storage** - All worktrees organized in `~/.worktrees/<repo>/<branch>/`
  - 🔄 **Config Sync** - Automatic copying of `.env`, `.vscode/`, and custom config files
  - 🛡️ **Safe Branch Names** - Automatic sanitization of branch names with slashes/special chars
  - 🧭 **Smart Navigation** - Shell integration with tab completions and directory changing
  - 🔙 **Origin Tracking** - Back navigation support with automatic origin path storage
  - 🧹 **Self-Cleaning** - Automatic cleanup of orphaned branches and references

- **Shell Integration:**

  - Native bash/zsh/fish shell functions for directory changing
  - Intelligent tab completions for all commands and worktree names
  - Interactive selection modes for easy worktree discovery

- **Configuration System:**

  - `.worktree-config.toml` support for customizing file copy patterns
  - Git config inheritance from parent repositories
  - Sensible defaults for common development files

- **Safety & Reliability:**
  - Comprehensive error handling and validation
  - Dry-run capabilities for testing operations
  - Proper cleanup of git references and filesystem state
  - Cross-platform path handling and symlink resolution

[Unreleased]: https://github.com/cafreeman/worktree/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/cafreeman/worktree/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/cafreeman/worktree/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/cafreeman/worktree/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/cafreeman/worktree/releases/tag/v0.1.0
