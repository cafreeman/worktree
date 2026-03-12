#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::CliTestEnvironment;

/// When a managed worktree dir is removed manually, cleanup removes orphaned git references
#[test]
fn test_cleanup_removes_orphaned_git_references() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a managed worktree via CLI
    env.run_command(&["create", "managed-a", "feature/managed-a"])?
        .assert()
        .success();

    // Create an independent branch NOT via CLI (unmanaged)
    {
        let output = std::process::Command::new("git")
            .args(["checkout", "-b", "feature/unmanaged-b"])
            .current_dir(env.repo_dir.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        // Switch back to default branch to avoid detachment
        let _ = std::process::Command::new("git")
            .args(["checkout", "main"])
            .current_dir(env.repo_dir.path())
            .output()
            .unwrap();
    }

    // Simulate orphaning: remove the managed worktree directory
    let managed_path = env.worktree_path("managed-a");
    managed_path.assert(predicate::path::is_dir());
    std::fs::remove_dir_all(managed_path.path())?;

    // Run cleanup — should succeed
    env.run_command(&["cleanup"])?.assert().success();

    Ok(())
}

/// When a branch is deleted outside the CLI, cleanup prunes orphaned references
#[test]
fn test_cleanup_prunes_orphaned_directories_for_deleted_branches() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a managed worktree via CLI
    env.run_command(&["create", "to-be-deleted", "feature/to-be-deleted"])?
        .assert()
        .success();
    let wt_path = env.worktree_path("to-be-deleted");
    wt_path.assert(predicate::path::is_dir());

    // Remove the worktree using git's own command (using the directory name)
    let remove_worktree_output = std::process::Command::new("git")
        .args(["worktree", "remove", "to-be-deleted"])
        .current_dir(env.repo_dir.path())
        .output()
        .unwrap();
    assert!(
        remove_worktree_output.status.success(),
        "Failed to remove worktree: {}",
        String::from_utf8_lossy(&remove_worktree_output.stderr)
    );

    let output = std::process::Command::new("git")
        .args(["branch", "-D", "feature/to-be-deleted"])
        .current_dir(env.repo_dir.path())
        .output()
        .unwrap();

    if !output.status.success() {
        eprintln!("Git command failed:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        eprintln!("status: {:?}", output.status);
    }
    assert!(output.status.success());

    // Run cleanup
    env.run_command(&["cleanup"])?.assert().success();

    // Worktree directory should be pruned (already removed by git worktree remove above)
    wt_path.assert(predicate::path::missing());

    Ok(())
}
