#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

//! Modern integration tests for the sync-config command
//!
//! These tests validate configuration file synchronization between worktrees,
//! including custom patterns, error handling, and content preservation.

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::{
    CliTestEnvironment, assert_config_files_copied, create_sample_config_files,
    create_worktree_config,
};

/// Test basic configuration file synchronization between worktrees
#[test]
fn test_sync_config_between_worktrees() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create source and target worktrees
    env.run_command(&["create", "feature/source"])?
        .assert()
        .success();

    env.run_command(&["create", "feature/target"])?
        .assert()
        .success();

    // Verify worktrees exist
    let source_path = env.worktree_path("feature/source");
    let target_path = env.worktree_path("feature/target");
    source_path.assert(predicate::path::is_dir());
    target_path.assert(predicate::path::is_dir());

    // Create config files in source using our helper
    create_sample_config_files(&source_path)?;

    // Test sync-config command
    env.run_command(&["sync-config", "feature/source", "feature/target"])?
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
fn test_sync_config_with_custom_patterns() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create custom worktree config
    create_worktree_config(
        &env.repo_dir,
        &[".env*", ".vscode/", "*.local.*", "custom-config.yml"],
        &["node_modules/", "target/"],
    )?;

    // Create worktrees
    env.run_command(&["create", "feature/custom-source"])?
        .assert()
        .success();

    env.run_command(&["create", "feature/custom-target"])?
        .assert()
        .success();

    let source_path = env.worktree_path("feature/custom-source");
    let target_path = env.worktree_path("feature/custom-target");

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
    env.run_command(&[
        "sync-config",
        "feature/custom-source",
        "feature/custom-target",
    ])?
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
fn test_sync_config_with_absolute_paths() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees
    env.run_command(&["create", "feature/abs-source"])?
        .assert()
        .success();

    env.run_command(&["create", "feature/abs-target"])?
        .assert()
        .success();

    let source_path = env.worktree_path("feature/abs-source");
    let target_path = env.worktree_path("feature/abs-target");

    // Create config files that match default patterns
    source_path
        .child(".env.local")
        .write_str("TEST_VAR=hello")?;
    source_path
        .child("config.local.json")
        .write_str(r#"{"test": "value"}"#)?;

    // Test sync using absolute paths
    env.run_command(&[
        "sync-config",
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

/// Test sync command with sanitized branch names
#[test]
fn test_sync_config_with_sanitized_names() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees with special characters
    env.run_command(&["create", "feature/sync-source"])?
        .assert()
        .success();

    env.run_command(&["create", "feature/sync-target"])?
        .assert()
        .success();

    let source_path = env.worktree_path("feature/sync-source");
    let target_path = env.worktree_path("feature/sync-target");

    // Create config files
    create_sample_config_files(&source_path)?;

    // Test sync using original branch names (with special chars)
    env.run_command(&["sync-config", "feature/sync-source", "feature/sync-target"])?
        .assert()
        .success();

    // Verify sync worked
    assert_config_files_copied(&target_path)?;

    Ok(())
}

/// Test error handling when source worktree doesn't exist
#[test]
fn test_sync_config_nonexistent_source() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create only target worktree
    env.run_command(&["create", "feature/target-only"])?
        .assert()
        .success();

    // Try to sync from nonexistent source
    env.run_command(&["sync-config", "nonexistent", "feature/target-only"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));

    Ok(())
}

/// Test error handling when target worktree doesn't exist
#[test]
fn test_sync_config_nonexistent_target() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create only source worktree
    env.run_command(&["create", "feature/source-only"])?
        .assert()
        .success();

    // Try to sync to nonexistent target
    env.run_command(&["sync-config", "feature/source-only", "nonexistent"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));

    Ok(())
}

/// Test selective file copying with include/exclude patterns
#[test]
fn test_sync_config_exclude_patterns() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create config with specific exclude patterns
    create_worktree_config(
        &env.repo_dir,
        &[".env*", ".vscode/", "*.local.*"],
        &["*.log", "*.tmp", "node_modules/", "secret.*"],
    )?;

    // Create worktrees
    env.run_command(&["create", "feature/exclude-source"])?
        .assert()
        .success();

    env.run_command(&["create", "feature/exclude-target"])?
        .assert()
        .success();

    let source_path = env.worktree_path("feature/exclude-source");
    let target_path = env.worktree_path("feature/exclude-target");

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
    env.run_command(&[
        "sync-config",
        "feature/exclude-source",
        "feature/exclude-target",
    ])?
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
fn test_sync_config_preserves_content() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees
    env.run_command(&["create", "feature/preserve-source"])?
        .assert()
        .success();

    env.run_command(&["create", "feature/preserve-target"])?
        .assert()
        .success();

    let source_path = env.worktree_path("feature/preserve-source");
    let target_path = env.worktree_path("feature/preserve-target");

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
    env.run_command(&[
        "sync-config",
        "feature/preserve-source",
        "feature/preserve-target",
    ])?
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
fn test_sync_config_empty_source() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees
    env.run_command(&["create", "feature/empty-source"])?
        .assert()
        .success();

    env.run_command(&["create", "feature/empty-target"])?
        .assert()
        .success();

    // Don't create any config files in source

    // Test sync - should succeed but copy nothing
    env.run_command(&[
        "sync-config",
        "feature/empty-source",
        "feature/empty-target",
    ])?
    .assert()
    .success();

    // Verify target remains clean (just git files)
    let target_path = env.worktree_path("feature/empty-target");
    target_path.child(".git").assert(predicate::path::exists());

    // But no config files should exist
    target_path.child(".env").assert(predicate::path::missing());
    target_path
        .child(".vscode")
        .assert(predicate::path::missing());

    Ok(())
}
