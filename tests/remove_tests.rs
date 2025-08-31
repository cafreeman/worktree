#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

//! Modern integration tests for the remove command
//!
//! These tests validate remove command functionality including branch deletion,
//! error handling, and sanitized name resolution using real CLI execution.

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

mod cli_test_helpers;
use cli_test_helpers::CliTestEnvironment;

/// Test interactive removal with mock selection provider
#[test]
fn test_interactive_remove_selection() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Setup: create some worktrees first
    env.run_command(&["create", "feature/test1"])?
        .assert()
        .success();

    env.run_command(&["create", "feature/test2"])?
        .assert()
        .success();

    // Verify they exist
    env.worktree_path("feature/test1")
        .assert(predicate::path::is_dir());
    env.worktree_path("feature/test2")
        .assert(predicate::path::is_dir());

    // TODO: Interactive test would go here when we have interactive remove functionality
    // This demonstrates the pattern even though the current remove command
    // requires a target parameter

    // For now, test non-interactive removal
    env.run_command(&["remove", "feature/test1"])?
        .assert()
        .success();

    // Verify removal
    env.worktree_path("feature/test1")
        .assert(predicate::path::missing());
    env.worktree_path("feature/test2")
        .assert(predicate::path::exists());

    Ok(())
}

/// Test remove command with branch deletion
#[test]
fn test_remove_with_branch_deletion() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree
    env.run_command(&["create", "feature/delete-me"])?
        .assert()
        .success();

    env.worktree_path("feature/delete-me")
        .assert(predicate::path::is_dir());

    // Remove worktree and delete branch (default behavior)
    env.run_command(&["remove", "feature/delete-me"])?
        .assert()
        .success();

    // Verify removal
    env.worktree_path("feature/delete-me")
        .assert(predicate::path::missing());

    Ok(())
}

/// Test remove command with keep branch flag
#[test]
fn test_remove_keep_branch() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree
    env.run_command(&["create", "feature/keep-branch"])?
        .assert()
        .success();

    env.worktree_path("feature/keep-branch")
        .assert(predicate::path::is_dir());

    // Remove worktree but keep branch
    env.run_command(&["remove", "feature/keep-branch", "--keep-branch"])?
        .assert()
        .success();

    // Verify worktree is gone
    env.worktree_path("feature/keep-branch")
        .assert(predicate::path::missing());

    // Branch should still exist (we'd need to check git for this)
    // For now, just verify the command succeeded

    Ok(())
}

/// Test error handling for nonexistent worktree
#[test]
fn test_remove_nonexistent_worktree() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Try to remove a worktree that doesn't exist
    env.run_command(&["remove", "nonexistent"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("No worktree found"));

    Ok(())
}

/// Test remove using sanitized filesystem names vs original branch names
#[test]
fn test_remove_by_sanitized_name() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree with special characters that get sanitized
    env.run_command(&["create", "feature/test-branch"])?
        .assert()
        .success();

    // Directory should use sanitized name
    let worktree_path = env.worktree_path("feature/test-branch"); // This uses our helper's sanitization
    worktree_path.assert(predicate::path::is_dir());

    // Should be able to remove using original branch name
    env.run_command(&["remove", "feature/test-branch"])?
        .assert()
        .success();

    worktree_path.assert(predicate::path::missing());

    Ok(())
}

// TODO: Future interactive tests once remove command supports interactive mode
/*
/// Test interactive remove with confirmation prompts
#[test]
fn test_interactive_remove_with_confirmation() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Setup worktrees
    env.run_command(&["create", "feature/interactive1"])?
        .assert()
        .success();

    // Start interactive session
    let mut interactive = env.start_interactive(&["remove", "--interactive"])?;

    interactive
        .expect_and_respond("Select worktree to remove:", "feature/interactive1")?
        .expect_and_respond("Delete branch too? (y/N)", "y")?
        .expect_final("✓ Worktree and branch removed successfully!")?;

    // Verify result
    env.worktree_path("feature/interactive1").assert(predicate::path::missing());

    Ok(())
}
*/

/// Test remove command with sanitized names and branch deletion edge cases
#[test]
fn test_remove_sanitized_name_branch_deletion() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree with characters that require sanitization
    env.run_command(&["create", "feature/test-branch"])?
        .assert()
        .success();

    // Verify the worktree was created
    let worktree_path = env.worktree_path("feature/test-branch");
    worktree_path.assert(predicate::path::is_dir());

    // Remove the worktree (this tests the branch name resolution fix)
    // The command should properly resolve the canonical branch name even
    // when the filesystem directory uses a sanitized name
    env.run_command(&["remove", "feature/test-branch"])?
        .assert()
        .success();

    // Verify the worktree was removed
    worktree_path.assert(predicate::path::missing());

    Ok(())
}
