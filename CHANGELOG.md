# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/cafreeman/worktree/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/cafreeman/worktree/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/cafreeman/worktree/releases/tag/v0.1.0
