#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

//! Modern integration tests for the sync command
//!
//! These tests validate configuration file synchronization between worktrees,
//! including custom patterns, error handling, and content preservation.

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::{
    CliTestEnvironment, assert_config_files_copied, create_sample_config_files,
    create_worktree_config, create_worktree_config_with_symlinks,
};

/// Test basic configuration file synchronization between worktrees
#[test]
fn test_sync_between_worktrees() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create source and target worktrees using feature-name + branch pairs
    env.run_command(&["create", "source", "feature/source"])?
        .assert()
        .success();

    env.run_command(&["create", "target", "feature/target"])?
        .assert()
        .success();

    // Verify worktrees exist
    let source_path = env.worktree_path("source");
    let target_path = env.worktree_path("target");
    source_path.assert(predicate::path::is_dir());
    target_path.assert(predicate::path::is_dir());

    // Create config files in source using our helper
    create_sample_config_files(&source_path)?;

    // Test sync command
    env.run_command(&["sync", "source", "target"])?
        .assert()
        .success();

    // Verify config files were copied to target
    target_path
        .child(".env")
        .assert(predicate::str::contains("TEST_VAR"));
    target_path
        .child(".vscode")
        .child("settings.json")
        .assert(predicate::path::exists());
    target_path
        .child("config.local.json")
        .assert(predicate::str::contains("debug"));

    Ok(())
}

/// Test sync command with custom configuration patterns
#[test]
fn test_sync_with_custom_patterns() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create custom worktree config
    create_worktree_config(
        &env.repo_dir,
        &[".env*", ".vscode/", "*.local.*", "custom-config.yml"],
        &["node_modules/", "target/"],
    )?;

    // Create worktrees
    env.run_command(&["create", "custom-source", "feature/custom-source"])?
        .assert()
        .success();

    env.run_command(&["create", "custom-target", "feature/custom-target"])?
        .assert()
        .success();

    let source_path = env.worktree_path("custom-source");
    let target_path = env.worktree_path("custom-target");

    // Create config files matching our custom patterns
    source_path
        .child(".env.development")
        .write_str("API_URL=dev.example.com")?;
    source_path
        .child("database.local.conf")
        .write_str("host=localhost")?;
    source_path
        .child("custom-config.yml")
        .write_str("version: 1.0")?;

    // Create .vscode directory and file
    let vscode_dir = source_path.child(".vscode");
    vscode_dir.create_dir_all()?;
    vscode_dir
        .child("launch.json")
        .write_str(r#"{"type": "node"}"#)?;

    // Test sync
    env.run_command(&["sync", "custom-source", "custom-target"])?
        .assert()
        .success();

    // Verify all custom patterns were copied
    target_path
        .child(".env.development")
        .assert(predicate::str::contains("API_URL"));
    target_path
        .child("database.local.conf")
        .assert(predicate::str::contains("localhost"));
    target_path
        .child("custom-config.yml")
        .assert(predicate::str::contains("version"));
    target_path
        .child(".vscode")
        .child("launch.json")
        .assert(predicate::str::contains("node"));

    Ok(())
}

/// Test sync command using absolute filesystem paths
#[test]
fn test_sync_with_absolute_paths() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees
    env.run_command(&["create", "abs-source", "feature/abs-source"])?
        .assert()
        .success();

    env.run_command(&["create", "abs-target", "feature/abs-target"])?
        .assert()
        .success();

    let source_path = env.worktree_path("abs-source");
    let target_path = env.worktree_path("abs-target");

    // Create config files that match default patterns
    source_path
        .child(".env.local")
        .write_str("TEST_VAR=hello")?;
    source_path
        .child("config.local.json")
        .write_str(r#"{"test": "value"}"#)?;

    // Test sync using absolute paths
    env.run_command(&[
        "sync",
        &source_path.to_string_lossy(),
        &target_path.to_string_lossy(),
    ])?
    .assert()
    .success();

    // Verify sync worked
    target_path
        .child(".env.local")
        .assert(predicate::str::contains("TEST_VAR"));
    target_path
        .child("config.local.json")
        .assert(predicate::str::contains("test"));

    Ok(())
}

/// Test sync with feature names (no slash required)
#[test]
fn test_sync_with_feature_names() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees using feature names
    env.run_command(&["create", "sync-source", "feature/sync-source"])?
        .assert()
        .success();

    env.run_command(&["create", "sync-target", "feature/sync-target"])?
        .assert()
        .success();

    let source_path = env.worktree_path("sync-source");
    let target_path = env.worktree_path("sync-target");

    // Create config files
    create_sample_config_files(&source_path)?;

    // Test sync using feature names directly
    env.run_command(&["sync", "sync-source", "sync-target"])?
        .assert()
        .success();

    // Verify sync worked
    assert_config_files_copied(&target_path)?;

    Ok(())
}

/// Test error handling when source worktree doesn't exist
#[test]
fn test_sync_nonexistent_source() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create only target worktree
    env.run_command(&["create", "target-only", "feature/target-only"])?
        .assert()
        .success();

    // Try to sync from nonexistent source
    env.run_command(&["sync", "nonexistent", "target-only"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));

    Ok(())
}

/// Test error handling when target worktree doesn't exist
#[test]
fn test_sync_nonexistent_target() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create only source worktree
    env.run_command(&["create", "source-only", "feature/source-only"])?
        .assert()
        .success();

    // Try to sync to nonexistent target
    env.run_command(&["sync", "source-only", "nonexistent"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));

    Ok(())
}

/// Test selective file copying with include/exclude patterns
#[test]
fn test_sync_exclude_patterns() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create config with specific exclude patterns
    create_worktree_config(
        &env.repo_dir,
        &[".env*", ".vscode/", "*.local.*"],
        &["*.log", "*.tmp", "node_modules/", "secret.*"],
    )?;

    // Create worktrees
    env.run_command(&["create", "exclude-source", "feature/exclude-source"])?
        .assert()
        .success();

    env.run_command(&["create", "exclude-target", "feature/exclude-target"])?
        .assert()
        .success();

    let source_path = env.worktree_path("exclude-source");
    let target_path = env.worktree_path("exclude-target");

    // Create files that should be copied
    source_path.child(".env").write_str("SHOULD_COPY=yes")?;
    source_path
        .child("config.local.json")
        .write_str(r#"{"copy": true}"#)?;

    // Create files that should be excluded
    source_path
        .child("debug.log")
        .write_str("should not copy")?;
    source_path.child("temp.tmp").write_str("should not copy")?;
    source_path
        .child("secret.key")
        .write_str("should not copy")?;

    // Test sync
    env.run_command(&["sync", "exclude-source", "exclude-target"])?
        .assert()
        .success();

    // Verify included files were copied
    target_path
        .child(".env")
        .assert(predicate::str::contains("SHOULD_COPY"));
    target_path
        .child("config.local.json")
        .assert(predicate::str::contains("copy"));

    // Verify excluded files were NOT copied
    target_path
        .child("debug.log")
        .assert(predicate::path::missing());
    target_path
        .child("temp.tmp")
        .assert(predicate::path::missing());
    target_path
        .child("secret.key")
        .assert(predicate::path::missing());

    Ok(())
}

/// Test sync command preserves file content and structure
#[test]
fn test_sync_preserves_content() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees
    env.run_command(&["create", "preserve-source", "feature/preserve-source"])?
        .assert()
        .success();

    env.run_command(&["create", "preserve-target", "feature/preserve-target"])?
        .assert()
        .success();

    let source_path = env.worktree_path("preserve-source");
    let target_path = env.worktree_path("preserve-target");

    // Create a complex config file with specific content
    let complex_config = r#"{
    "editor.fontSize": 14,
    "editor.tabSize": 2,
    "files.autoSave": "onWindowChange",
    "terminal.integrated.fontSize": 12,
    "workbench.colorTheme": "Dark+ (default dark)"
}"#;

    let vscode_dir = source_path.child(".vscode");
    vscode_dir.create_dir_all()?;
    vscode_dir
        .child("settings.json")
        .write_str(complex_config)?;

    // Test sync
    env.run_command(&["sync", "preserve-source", "preserve-target"])?
        .assert()
        .success();

    // Verify exact content preservation
    target_path
        .child(".vscode")
        .child("settings.json")
        .assert(predicate::str::contains("editor.fontSize"))
        .assert(predicate::str::contains("workbench.colorTheme"))
        .assert(predicate::str::contains("Dark+ (default dark)"));

    Ok(())
}

/// Test sync command behavior when source has no config files
#[test]
fn test_sync_empty_source() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees
    env.run_command(&["create", "empty-source", "feature/empty-source"])?
        .assert()
        .success();

    env.run_command(&["create", "empty-target", "feature/empty-target"])?
        .assert()
        .success();

    // Don't create any config files in source

    // Test sync - should succeed but copy nothing
    env.run_command(&["sync", "empty-source", "empty-target"])?
        .assert()
        .success();

    // Verify target remains clean (just git files)
    let target_path = env.worktree_path("empty-target");
    target_path.child(".git").assert(predicate::path::exists());

    // But no config files should exist
    target_path.child(".env").assert(predicate::path::missing());
    target_path
        .child(".vscode")
        .assert(predicate::path::missing());

    Ok(())
}

/// Test that `sync` with exactly one of from/to is a usage error
#[test]
fn test_sync_one_argument_is_usage_error() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "only-one", "feature/only-one"])?
        .assert()
        .success();

    env.run_command(&["sync", "only-one"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("Both"));

    Ok(())
}

/// Test that `sync` is symlink-aware: a pattern added after worktree creation
/// gets pushed out as a symlink by a pairwise sync.
#[test]
fn test_sync_pushes_out_new_symlink_pattern() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "sym-source", "feature/sym-source"])?
        .assert()
        .success();
    env.run_command(&["create", "sym-target", "feature/sym-target"])?
        .assert()
        .success();

    let source_path = env.worktree_path("sym-source");
    let target_path = env.worktree_path("sym-target");

    // Add a symlink pattern to the repo config *after* worktrees already exist,
    // and create the matching file only now (mirrors the openspec-init scenario).
    create_worktree_config_with_symlinks(&env.repo_dir, &[], &["shared.md"])?;
    source_path.child("shared.md").write_str("shared content")?;

    env.run_command(&["sync", "sym-source", "sym-target"])?
        .assert()
        .success();

    let link = target_path.child("shared.md");
    link.assert(predicate::path::exists());
    let metadata = std::fs::symlink_metadata(link.path())?;
    assert!(
        metadata.file_type().is_symlink(),
        "sync should create a symlink for a newly-added symlink pattern"
    );

    Ok(())
}

/// Test that `sync` force-relinks a path that already exists as a plain copy
/// once it starts matching a symlink pattern.
#[test]
fn test_sync_relinks_existing_plain_copy() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Start with a copy-pattern only, so the file gets copied (not symlinked) on create.
    create_worktree_config(&env.repo_dir, &["shared.md"], &[])?;

    env.run_command(&["create", "relink-source", "feature/relink-source"])?
        .assert()
        .success();

    let source_path = env.worktree_path("relink-source");
    source_path.child("shared.md").write_str("origin content")?;

    env.run_command(&["create", "relink-target", "feature/relink-target"])?
        .assert()
        .success();
    let target_path = env.worktree_path("relink-target");

    // First sync: plain copy (matches copy-pattern only).
    env.run_command(&["sync", "relink-source", "relink-target"])?
        .assert()
        .success();

    let link = target_path.child("shared.md");
    let metadata_before = std::fs::symlink_metadata(link.path())?;
    assert!(
        !metadata_before.file_type().is_symlink(),
        "shared.md should start as a plain copy"
    );

    // Now the same path also becomes a symlink-pattern match.
    create_worktree_config_with_symlinks(&env.repo_dir, &["shared.md"], &["shared.md"])?;

    env.run_command(&["sync", "relink-source", "relink-target"])?
        .assert()
        .success();

    let metadata_after = std::fs::symlink_metadata(link.path())?;
    assert!(
        metadata_after.file_type().is_symlink(),
        "sync should replace the existing plain copy with a symlink"
    );

    Ok(())
}

/// Test that broadcast mode (`sync` with no args) reaches every worktree of the current repo.
#[test]
fn test_sync_broadcast_reaches_all_worktrees_of_repo() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "broadcast-a", "feature/broadcast-a"])?
        .assert()
        .success();
    env.run_command(&["create", "broadcast-b", "feature/broadcast-b"])?
        .assert()
        .success();

    // Add config *after* both worktrees exist.
    create_sample_config_files(&env.repo_dir)?;

    // Run broadcast sync from inside the origin repo.
    env.run_command(&["sync"])?.assert().success();

    assert_config_files_copied(&env.worktree_path("broadcast-a"))?;
    assert_config_files_copied(&env.worktree_path("broadcast-b"))?;

    Ok(())
}

/// Test that broadcast mode run from inside a worktree still sources from that
/// worktree's origin repo, not from the worktree itself.
#[test]
fn test_sync_broadcast_from_inside_worktree_uses_origin() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "inner-a", "feature/inner-a"])?
        .assert()
        .success();
    env.run_command(&["create", "inner-b", "feature/inner-b"])?
        .assert()
        .success();

    create_sample_config_files(&env.repo_dir)?;

    let inner_a_path = env.worktree_path("inner-a");

    // Run `sync` with cwd set to worktree inner-a instead of the origin repo.
    let mut cmd = assert_cmd::Command::cargo_bin("worktree-bin")?;
    cmd.current_dir(inner_a_path.path())
        .env("WORKTREE_STORAGE_ROOT", env.storage_dir.path())
        .arg("sync");
    cmd.assert().success();

    assert_config_files_copied(&env.worktree_path("inner-a"))?;
    assert_config_files_copied(&env.worktree_path("inner-b"))?;

    Ok(())
}
