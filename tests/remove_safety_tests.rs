#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

//! Safety tests for the remove command's uncommitted-work check.
//!
//! `remove` must refuse dirty worktrees unless `--force` is passed, while
//! tool-copied config files, symlink patterns, and build artifacts never
//! block removal (worktrees are "born dirty" by design).

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::{CliTestEnvironment, create_worktree_config};

#[test]
fn test_remove_clean_worktree_succeeds() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "clean", "feature/clean"])?
        .assert()
        .success();

    env.run_command(&["remove", "clean"])?.assert().success();
    env.worktree_path("clean")
        .assert(predicate::path::missing());
    Ok(())
}

#[test]
fn test_remove_refuses_modified_tracked_file() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "dirty", "feature/dirty"])?
        .assert()
        .success();

    let wt = env.worktree_path("dirty");
    std::fs::write(wt.path().join("README.md"), "modified content")?;

    // Refused without --force; directory survives
    env.run_command(&["remove", "dirty"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("uncommitted"))
        .stderr(predicate::str::contains("--force"));
    wt.assert(predicate::path::exists());

    // --force removes it anyway
    env.run_command(&["remove", "dirty", "--force"])?
        .assert()
        .success();
    wt.assert(predicate::path::missing());
    Ok(())
}

#[test]
fn test_remove_refuses_user_untracked_file() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "scratch", "feature/scratch"])?
        .assert()
        .success();

    let wt = env.worktree_path("scratch");
    std::fs::write(wt.path().join("notes.txt"), "my unpublished notes")?;

    env.run_command(&["remove", "scratch"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("uncommitted"));

    // The untracked file must survive a refused removal
    assert!(wt.path().join("notes.txt").exists());
    Ok(())
}

#[test]
fn test_remove_refuses_staged_file() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "staged", "feature/staged"])?
        .assert()
        .success();

    let wt = env.worktree_path("staged");
    std::fs::write(wt.path().join("new.rs"), "fn main() {}")?;
    let status = std::process::Command::new("git")
        .args(["add", "new.rs"])
        .current_dir(wt.path())
        .status()
        .expect("git add should run");
    assert!(status.success(), "git add should succeed");

    env.run_command(&["remove", "staged"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("uncommitted"));
    Ok(())
}

#[test]
fn test_remove_allows_tool_copied_config_files() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    // Origin .env + copy pattern: the worktree is "born dirty" with an
    // untracked (but tool-managed) .env — removal must not require --force.
    env.repo_dir.child(".env").write_str("TEST=1")?;
    create_worktree_config(&env.repo_dir, &[".env*"], &[])?;

    env.run_command(&["create", "copied", "feature/copied"])?
        .assert()
        .success();
    env.worktree_path("copied")
        .child(".env")
        .assert(predicate::path::exists());

    env.run_command(&["remove", "copied"])?.assert().success();
    env.worktree_path("copied")
        .assert(predicate::path::missing());
    Ok(())
}

#[test]
fn test_remove_allows_build_artifacts() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "built", "feature/built"])?
        .assert()
        .success();

    let wt = env.worktree_path("built");
    let pkg = wt.path().join("node_modules").join("pkg");
    std::fs::create_dir_all(&pkg)?;
    std::fs::write(pkg.join("index.js"), "x")?;

    env.run_command(&["remove", "built"])?.assert().success();
    wt.assert(predicate::path::missing());
    Ok(())
}
