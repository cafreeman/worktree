#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

//! Repository resolution tests: commands invoked from inside a linked
//! worktree must resolve to the MAIN repository, not the worktree's own
//! directory. Storage is keyed by the main repo's name, so getting this
//! wrong historically placed nested worktrees under the wrong repo
//! (`~/.worktrees/<feature-of-containing-worktree>/...`) and broke
//! `list --current`, `back`, and `sync` broadcast.

use anyhow::Result;
use assert_cmd::assert::Assert;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

use test_support::CliTestEnvironment;

/// Runs the CLI from a specific working directory (not the origin repo).
fn run_in(env: &CliTestEnvironment, cwd: &Path, args: &[&str]) -> Assert {
    let mut cmd = env.run_command(args).expect("command should construct");
    cmd.current_dir(cwd);
    cmd.assert()
}

#[test]
fn test_create_from_inside_worktree_uses_main_repo() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "alpha", "feature/alpha"])?
        .assert()
        .success();

    // Create beta from INSIDE alpha's worktree
    run_in(
        &env,
        env.worktree_path("alpha").path(),
        &["create", "beta", "feature/beta"],
    )
    .success();

    // beta must live under the main repo's storage dir, not under "alpha"
    env.storage_dir
        .child("test_repo")
        .child("beta")
        .assert(predicate::path::is_dir());
    assert!(
        !env.storage_dir.child("alpha").child("beta").exists(),
        "beta must not be nested under the containing worktree's feature name"
    );
    Ok(())
}

#[test]
fn test_list_current_from_inside_worktree_names_main_repo() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "alpha", "feature/alpha"])?
        .assert()
        .success();
    env.run_command(&["create", "beta", "feature/beta"])?
        .assert()
        .success();

    let output = run_in(
        &env,
        env.worktree_path("alpha").path(),
        &["list", "--current"],
    )
    .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("test_repo"),
        "list --current must name the main repo, got: {stdout}"
    );
    assert!(stdout.contains("beta"), "must list sibling worktrees");
    Ok(())
}

#[test]
fn test_back_from_worktree_created_inside_another_worktree() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "alpha", "feature/alpha"])?
        .assert()
        .success();

    // beta is created from inside alpha; its origin must still be the main repo
    run_in(
        &env,
        env.worktree_path("alpha").path(),
        &["create", "beta", "feature/beta"],
    )
    .success();

    let output = run_in(
        &env,
        env.storage_dir.child("test_repo").child("beta").path(),
        &["back"],
    )
    .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let printed = PathBuf::from(stdout.trim());
    assert_eq!(
        printed.canonicalize().unwrap(),
        env.repo_dir.path().canonicalize().unwrap(),
        "back must return the main repo, got {printed:?}"
    );
    Ok(())
}
