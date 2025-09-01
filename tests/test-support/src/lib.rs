//! Test support utilities for worktree integration tests
//!
//! This crate provides shared test helpers and utilities for integration tests.
//! It's designed to be used only during development and testing, not published.

pub mod patterns;
pub mod test_env;

// Re-export commonly used items for convenience
pub use patterns::{
    assert_config_files_copied, create_sample_config_files, create_worktree_config,
};
pub use test_env::CliTestEnvironment;
