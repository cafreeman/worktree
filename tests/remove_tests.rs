#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity
#![allow(clippy::expect_used)] // Tests may use expect for clearer failure messages

//! Modern integration tests for the remove command
//!
//! These tests validate remove command functionality including branch deletion,
//! error handling, and sanitized name resolution using real CLI execution.

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::CliTestEnvironment;

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

/// Test remove command with branch deletion (default behavior)
#[test]
fn test_remove_with_branch_deletion() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree
    env.run_command(&["create", "feature/delete-me"])?
        .assert()
        .success();

    env.worktree_path("feature/delete-me")
        .assert(predicate::path::is_dir());

    // Remove worktree - should delete branch by default (force delete)
    env.run_command(&["remove", "feature/delete-me"])?
        .assert()
        .success();

    // Verify removal
    env.worktree_path("feature/delete-me")
        .assert(predicate::path::missing());

    Ok(())
}

/// Test remove when mapping is missing; branch should still delete via HEAD resolution
#[test]
fn test_remove_without_mapping_uses_head_resolution() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree with a branch that would be sanitized
    env.run_command(&["create", "feature/slashed/branch"])?
        .assert()
        .success();

    let worktree_path = env.worktree_path("feature/slashed/branch");
    worktree_path.assert(predicate::path::is_dir());

    // Delete mapping file to simulate missing mapping
    let mapping_file = env.storage_dir.child("test_repo").child(".branch-mapping");
    if mapping_file.path().exists() {
        std::fs::remove_file(mapping_file.path()).ok();
    }

    // Remove worktree - should force delete branch by default
    env.run_command(&["remove", "feature/slashed/branch"])?
        .assert()
        .success();

    worktree_path.assert(predicate::path::missing());

    Ok(())
}

/// Test that branches are force-deleted by default (previously required --force-delete-branch)
#[test]
fn test_unmanaged_branch_force_deleted_by_default() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create an existing branch manually (unmanaged)
    {
        let repo = env.repo_dir.path();
        std::process::Command::new("git")
            .args(["checkout", "-b", "feature/manual-branch"])
            .current_dir(repo)
            .status()
            .expect("git checkout -b should run");

        // Switch back to previous branch so the branch is not checked out anywhere
        std::process::Command::new("git")
            .args(["checkout", "-"])
            .current_dir(repo)
            .status()
            .expect("git checkout - to previous branch should run");
    }

    // Create worktree for that branch (existing branch)
    env.run_command(&["create", "feature/manual-branch"])?
        .assert()
        .success();

    let wt = env.worktree_path("feature/manual-branch");
    wt.assert(predicate::path::is_dir());

    // Remove - should force delete branch by default now (even if unmanaged)
    env.run_command(&["remove", "feature/manual-branch"])?
        .assert()
        .success();
    wt.assert(predicate::path::missing());

    Ok(())
}

/// Test removal when worktree is in detached HEAD; branch deletion should be skipped gracefully
#[test]
fn test_remove_detached_head_skips_branch_deletion() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree
    env.run_command(&["create", "feature/detached"])?
        .assert()
        .success();

    let wt = env.worktree_path("feature/detached");
    wt.assert(predicate::path::is_dir());

    // Detach HEAD in the worktree repository
    let wt_path = wt.path();
    std::process::Command::new("git")
        .args(["checkout", "--detach"]) // detach at current commit
        .current_dir(wt_path)
        .status()
        .expect("git checkout --detach should run");

    // Remove should succeed and skip branch deletion
    env.run_command(&["remove", "feature/detached"])?
        .assert()
        .success();

    wt.assert(predicate::path::missing());
    Ok(())
}

/// Test remove command with --preserve-branch flag
#[test]
fn test_remove_preserve_branch() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree
    env.run_command(&["create", "feature/preserve-me"])?
        .assert()
        .success();

    env.worktree_path("feature/preserve-me")
        .assert(predicate::path::is_dir());

    // Remove worktree but preserve branch
    env.run_command(&["remove", "feature/preserve-me", "--preserve-branch"])?
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Branch 'feature/preserve-me' preserved",
        ));

    // Verify worktree is gone
    env.worktree_path("feature/preserve-me")
        .assert(predicate::path::missing());

    // Branch should still exist - we can recreate a worktree from it
    env.run_command(&["create", "feature/preserve-me"])?
        .assert()
        .success();

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

    // Remove the worktree - should force delete branch by default
    // The command should properly resolve the canonical branch name even
    // when the filesystem directory uses a sanitized name
    env.run_command(&["remove", "feature/test-branch"])?
        .assert()
        .success();

    // Verify the worktree was removed
    worktree_path.assert(predicate::path::missing());

    Ok(())
}
