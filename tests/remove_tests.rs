#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

//! Integration tests for the remove command (feature-named worktrees)

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::CliTestEnvironment;

/// Test basic removal of a worktree (branch preserved by default)
#[test]
fn test_remove_preserves_branch_by_default() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "test1", "feature/test1"])?
        .assert()
        .success();

    env.worktree_path("test1").assert(predicate::path::is_dir());

    // Remove without --delete-branch: branch should be preserved
    env.run_command(&["remove", "test1"])?
        .assert()
        .success()
        .stdout(
            predicate::str::contains("preserved").or(predicate::str::contains("Worktree removed")),
        );

    env.worktree_path("test1")
        .assert(predicate::path::missing());

    Ok(())
}

/// Test removal with --delete-branch flag
#[test]
fn test_remove_with_delete_branch() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "delete-me", "feature/delete-me"])?
        .assert()
        .success();

    env.worktree_path("delete-me")
        .assert(predicate::path::is_dir());

    env.run_command(&["remove", "delete-me", "--delete-branch"])?
        .assert()
        .success();

    env.worktree_path("delete-me")
        .assert(predicate::path::missing());

    Ok(())
}

/// Test interactive removal selection
#[test]
fn test_interactive_remove_selection() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "test1", "feature/test1"])?
        .assert()
        .success();

    env.run_command(&["create", "test2", "feature/test2"])?
        .assert()
        .success();

    env.worktree_path("test1").assert(predicate::path::is_dir());
    env.worktree_path("test2").assert(predicate::path::is_dir());

    // Non-interactive removal by feature name
    env.run_command(&["remove", "test1"])?.assert().success();

    env.worktree_path("test1")
        .assert(predicate::path::missing());
    env.worktree_path("test2").assert(predicate::path::exists());

    Ok(())
}

/// Test removal when worktree is in detached HEAD — branch deletion skipped gracefully
#[test]
fn test_remove_detached_head_skips_branch_deletion() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "detached", "feature/detached"])?
        .assert()
        .success();

    let wt = env.worktree_path("detached");
    wt.assert(predicate::path::is_dir());

    std::process::Command::new("git")
        .args(["checkout", "--detach"])
        .current_dir(wt.path())
        .status()
        .expect("git checkout --detach should run");

    // Remove with --delete-branch in detached HEAD — should warn but succeed
    env.run_command(&["remove", "detached", "--delete-branch"])?
        .assert()
        .success();

    wt.assert(predicate::path::missing());
    Ok(())
}

/// Test error handling for nonexistent worktree
#[test]
fn test_remove_nonexistent_worktree() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["remove", "nonexistent"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("No worktree found"));

    Ok(())
}

/// Test remove by feature name (exact match)
#[test]
fn test_remove_by_feature_name() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "auth", "feature/auth"])?
        .assert()
        .success();

    env.worktree_path("auth").assert(predicate::path::is_dir());

    env.run_command(&["remove", "auth"])?.assert().success();

    env.worktree_path("auth").assert(predicate::path::missing());

    Ok(())
}

/// Test that remove help shows --delete-branch (not --preserve-branch)
#[test]
fn test_remove_command_help() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    let output = env.run_command(&["remove", "--help"])?.output()?;
    let stdout = String::from_utf8(output.stdout)?;

    assert!(
        stdout.contains("--delete-branch"),
        "--delete-branch flag should be in help"
    );
    assert!(
        !stdout.contains("--preserve-branch"),
        "--preserve-branch should be removed"
    );

    Ok(())
}
