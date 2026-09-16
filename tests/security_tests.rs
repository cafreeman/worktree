#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

//! Security regression tests: user-supplied names must never resolve outside
//! the managed storage directory.
//!
//! `remove`/`sync` build filesystem paths from positional arguments. Before
//! validation was added, targets like `../../victim` (or `.`, `..`, or an
//! absolute path) escaped the storage root and could delete or overwrite
//! arbitrary directories.

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::CliTestEnvironment;

/// Test layout: `<temp>/test_repo` (origin), `<temp>/worktrees` (storage root),
/// `<temp>/victim` (directory outside the storage root that must survive).
/// `<storage>/test_repo/../../victim` resolves to `<temp>/victim`.
fn setup_with_victim() -> Result<(CliTestEnvironment, std::path::PathBuf)> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "auth", "feature/auth"])?
        .assert()
        .success();

    let temp_root = env.storage_dir.path().parent().unwrap().to_path_buf();
    let victim = temp_root.join("victim");
    std::fs::create_dir_all(&victim)?;
    std::fs::write(victim.join("precious.txt"), "important data")?;
    Ok((env, victim))
}

#[test]
fn test_remove_rejects_parent_traversal() -> Result<()> {
    let (env, victim) = setup_with_victim()?;

    env.run_command(&["remove", "../../victim"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));

    assert!(
        victim.join("precious.txt").exists(),
        "victim directory must survive a traversal attempt"
    );
    Ok(())
}

#[test]
fn test_remove_rejects_dot_and_dotdot() -> Result<()> {
    let (env, _victim) = setup_with_victim()?;

    // `.` resolves to the repo storage dir; `..` resolves to the storage root.
    // Both must be refused.
    env.run_command(&["remove", "."])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
    env.run_command(&["remove", ".."])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));

    // The worktree created in setup must still exist
    env.worktree_path("auth").assert(predicate::path::is_dir());
    Ok(())
}

#[test]
fn test_remove_rejects_absolute_path_target() -> Result<()> {
    let (env, victim) = setup_with_victim()?;

    env.run_command(&["remove", victim.to_str().unwrap()])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));

    assert!(
        victim.join("precious.txt").exists(),
        "victim directory must survive an absolute-path removal attempt"
    );
    Ok(())
}

#[test]
fn test_remove_rejects_hidden_name() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["remove", ".env"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
    Ok(())
}

#[test]
fn test_sync_rejects_traversal_in_either_position() -> Result<()> {
    let (env, victim) = setup_with_victim()?;

    env.run_command(&["sync", "../../victim", "auth"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));

    env.run_command(&["sync", "auth", "../../victim"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));

    assert!(
        victim.join("precious.txt").exists(),
        "victim directory must survive a sync traversal attempt"
    );
    Ok(())
}

#[test]
fn test_create_rejects_traversal_feature_name() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "..", "feature/escape"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));

    env.run_command(&["create", ".", "feature/escape"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));

    // No successful create happened, so the storage root must be empty —
    // in particular nothing may have escaped into or above it.
    let entries: Vec<_> = std::fs::read_dir(env.storage_dir.path())?
        .filter_map(std::result::Result::ok)
        .collect();
    assert!(
        entries.is_empty(),
        "storage root should contain no directories after rejected creates"
    );
    Ok(())
}
