#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::CliTestEnvironment;

/// When a managed branch's worktree dir is removed manually, cleanup deletes only that managed branch
#[test]
fn test_cleanup_deletes_only_managed_orphan_branches() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a managed branch via CLI
    env.run_command(&["create", "feature/managed-a"])?.assert().success();

    // Create an independent branch NOT via CLI (unmanaged)
    // Create branch and do not create a worktree
    {
        let output = std::process::Command::new("git")
            .args(["checkout", "-b", "feature/unmanaged-b"]) // create branch
            .current_dir(env.repo_dir.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        // switch back to default branch to avoid detachment
        let _ = std::process::Command::new("git")
            .args(["checkout", "master"]) // repo init likely creates master
            .current_dir(env.repo_dir.path())
            .output()
            .unwrap();
    }

    // Simulate orphaning: remove the managed worktree directory
    let managed_path = env.worktree_path("feature/managed-a");
    managed_path.assert(predicate::path::is_dir());
    managed_path.remove_dir_all()?;

    // Run cleanup
    env.run_command(&["cleanup"])?.assert().success();

    // Managed branch should be deleted
    // Verify by trying to checkout it fails
    let checkout = std::process::Command::new("git")
        .args(["checkout", "feature/managed-a"]) 
        .current_dir(env.repo_dir.path())
        .output()
        .unwrap();
    assert!(!checkout.status.success());

    // Unmanaged branch should still exist
    let checkout_unmanaged = std::process::Command::new("git")
        .args(["checkout", "feature/unmanaged-b"]) 
        .current_dir(env.repo_dir.path())
        .output()
        .unwrap();
    assert!(checkout_unmanaged.status.success());

    Ok(())
}

/// When a branch is deleted outside the CLI, cleanup prunes the orphaned worktree directory and metadata
#[test]
fn test_cleanup_prunes_orphaned_directories_for_deleted_branches() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a managed branch via CLI
    env.run_command(&["create", "feature/to-be-deleted"])?.assert().success();
    let wt_path = env.worktree_path("feature/to-be-deleted");
    wt_path.assert(predicate::path::is_dir());

    // Delete the branch outside the CLI
    let output = std::process::Command::new("git")
        .args(["branch", "-D", "feature/to-be-deleted"]) 
        .current_dir(env.repo_dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Run cleanup
    env.run_command(&["cleanup"])?.assert().success();

    // Worktree directory should be pruned
    wt_path.assert(predicate::path::missing());

    Ok(())
}

