//! # Worktree CLI
//!
//! A powerful CLI tool for managing git worktrees with enhanced features including centralized storage,
//! automatic config file synchronization, and intelligent branch management.
//!
//! ## Features
//!
//! - **Centralized Storage** - Organizes worktrees in `~/.worktrees/<repo-name>/<branch-name>/`
//! - **Smart Config Management** - Automatically copies gitignored config files to new worktrees
//! - **Branch Synchronization** - Keeps worktrees and git branches in sync
//! - **Comprehensive Status** - Shows detailed worktree and branch status
//! - **Configurable Patterns** - Customize which files to copy via `.worktree-config.toml`
//! - **Safe Branch Names** - Automatically sanitizes branch names with slashes and special characters
//!
//! ## Quick Start
//!
//! ```bash
//! # Create a new worktree for feature development
//! worktree create feature/auth
//!
//! # Create a worktree with a new branch
//! worktree create -b new-feature
//!
//! # List all worktrees
//! worktree list
//!
//! # Remove a worktree
//! worktree remove feature/auth
//! ```
//!
//! ## Module Structure
//!
//! - [`commands`] - Individual command implementations (create, list, remove, status, etc.)
//! - [`storage`] - Manages worktree storage in `~/.worktrees/` with branch name sanitization
//! - [`config`] - Handles `.worktree-config.toml` files for customizing file copy patterns
//! - [`git`] - Git operations wrapper using git2 crate
//! - [`selection`] - Abstracts interactive selection prompts for testability
//! - [`traits`] - Defines GitOperations trait for testability and abstraction

pub mod commands;
pub mod config;
pub mod git;
pub mod selection;
pub mod storage;
pub mod traits;

pub use anyhow::Result;
