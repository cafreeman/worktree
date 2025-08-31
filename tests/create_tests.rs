#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

//! Modern integration tests for the create command
//!
//! These tests use assert_fs for declarative filesystem setup and validation,
//! testing the create command with real CLI execution and comprehensive
//! error handling scenarios.

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

mod cli_test_helpers;
use cli_test_helpers::{CliTestEnvironment, patterns};

/// Test create command with config file copying using declarative filesystem assertions
#[test]
fn test_create_worktree_with_config_files() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Declarative filesystem setup
    patterns::create_worktree_config(
        &env.repo_dir,
        &[".env*", ".vscode/", "*.local.json"],
        &["node_modules/"],
    )?;
    patterns::create_sample_config_files(&env.repo_dir)?;

    // Execute command with assert_cmd
    env.run_command(&["create", "feature/config-test"])?
        .assert()
        .success();

    // Declarative assertions using predicates
    let worktree_path = env.worktree_path("feature/config-test");
    worktree_path.assert(predicate::path::is_dir());

    // Verify config files were copied
    patterns::assert_config_files_copied(&worktree_path)?;

    // Verify git worktree structure
    worktree_path
        .child(".git")
        .assert(predicate::path::exists());

    // Check branch mapping file
    let mapping_file = env.storage_dir.child("test_repo").child(".branch-mapping");
    mapping_file.assert(predicate::str::contains(
        "feature-config-test -> feature/config-test",
    ));

    Ok(())
}

/// Test error handling when target directory already exists
#[test]
fn test_create_worktree_directory_already_exists() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Pre-create the target directory
    let worktree_path = env.worktree_path("feature/existing");
    worktree_path.create_dir_all()?;

    // Attempt to create worktree - should fail
    env.run_command(&["create", "feature/existing"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    Ok(())
}

/// Test different branch creation modes (smart, new-branch, existing-branch)
#[test]
fn test_create_worktree_modes() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Test smart mode (default) - should create new branch automatically
    env.run_command(&["create", "feature/smart-mode"])?
        .assert()
        .success();

    env.worktree_path("feature/smart-mode")
        .assert(predicate::path::is_dir());

    // Test explicit new branch mode
    env.run_command(&["create", "--new-branch", "feature/explicit-new"])?
        .assert()
        .success();

    env.worktree_path("feature/explicit-new")
        .assert(predicate::path::is_dir());

    // Test existing branch mode - create a branch first, then use it
    env.run_test(|| {
        // Create a branch but don't check it out
        std::process::Command::new("git")
            .args(["branch", "feature/existing-test"])
            .current_dir(&env.repo_dir)
            .output()?;
        Ok(())
    })?;

    env.run_command(&["create", "--existing-branch", "feature/existing-test"])?
        .assert()
        .success();

    env.worktree_path("feature/existing-test")
        .assert(predicate::path::is_dir());

    Ok(())
}

/// Test git configuration inheritance in created worktrees
#[test]
fn test_git_config_inheritance() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_test(|| {
        // Set some custom git config in the test repo
        std::process::Command::new("git")
            .args(["config", "core.editor", "nano"])
            .current_dir(&env.repo_dir)
            .output()?;

        std::process::Command::new("git")
            .args(["config", "user.signingkey", "test-key-123"])
            .current_dir(&env.repo_dir)
            .output()?;

        // Create worktree
        env.run_command(&["create", "feature/config-inherit"])?
            .assert()
            .success();

        let worktree_path = env.worktree_path("feature/config-inherit");
        worktree_path.assert(predicate::path::exists());

        // Verify config inheritance by checking worktree-specific config
        let output = std::process::Command::new("git")
            .args(["config", "--worktree", "--get", "user.name"])
            .current_dir(&worktree_path)
            .output()?;

        if output.status.success() {
            let user_name = String::from_utf8(output.stdout)?;
            assert_eq!(user_name.trim(), "Test User");
        }

        // Check that extensions.worktreeConfig is enabled
        let output = std::process::Command::new("git")
            .args(["config", "extensions.worktreeConfig"])
            .current_dir(&env.repo_dir)
            .output()?;

        let config_value = String::from_utf8(output.stdout)?;
        assert_eq!(config_value.trim(), "true");

        Ok(())
    })
}

/// Test branch name sanitization for filesystem storage
#[test]
fn test_branch_name_sanitization() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Test various branch names that need sanitization
    let test_cases = vec![
        ("feature/user-auth", "feature-user-auth"),
        ("bugfix/critical-issue", "bugfix-critical-issue"),
        ("release/v1.0.0", "release-v1.0.0"),
    ];

    for (original_branch, expected_dir) in test_cases {
        env.run_command(&["create", "--new-branch", original_branch])?
            .assert()
            .success();

        // Check that directory uses sanitized name
        let worktree_path = env.storage_dir.child("test_repo").child(expected_dir);
        worktree_path.assert(predicate::path::is_dir());

        // Check that mapping preserves original name
        let mapping_file = env.storage_dir.child("test_repo").child(".branch-mapping");
        mapping_file.assert(predicate::str::contains(&format!(
            "{} -> {}",
            expected_dir, original_branch
        )));
    }

    Ok(())
}

// TODO: Add interactive tests once we implement interactive features in create command
// These would test scenarios like:
// - Interactive branch name input
// - Confirmation prompts for branch creation
// - Interactive selection of base branch

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test that the CLI helper framework itself works correctly
    #[test]
    fn test_cli_test_environment_setup() -> Result<()> {
        let env = CliTestEnvironment::new()?;

        // Verify git repository setup
        env.repo_dir.assert(predicate::path::is_dir());
        env.repo_dir.child(".git").assert(predicate::path::exists());
        env.repo_dir
            .child("README.md")
            .assert(predicate::str::contains("# Test Repo"));

        // Verify storage directory
        env.storage_dir.assert(predicate::path::is_dir());

        // Test command execution setup
        let cmd_result = env.run_command(&["--help"]);
        assert!(cmd_result.is_ok(), "Should be able to create command");

        Ok(())
    }

    /// Test pattern helper functions
    #[test]
    fn test_pattern_helpers() -> Result<()> {
        let env = CliTestEnvironment::new()?;

        // Test config creation helper
        patterns::create_worktree_config(
            &env.repo_dir,
            &[".env*", ".vscode/"],
            &["node_modules/", "target/"],
        )?;

        env.repo_dir
            .child(".worktree-config.toml")
            .assert(predicate::str::contains("copy-patterns"))
            .assert(predicate::str::contains(".env*"))
            .assert(predicate::str::contains("node_modules/"));

        // Test sample file creation
        patterns::create_sample_config_files(&env.repo_dir)?;

        env.repo_dir
            .child(".env")
            .assert(predicate::str::contains("TEST_VAR"));
        env.repo_dir
            .child(".vscode")
            .child("settings.json")
            .assert(predicate::path::exists());
        env.repo_dir
            .child("config.local.json")
            .assert(predicate::str::contains("debug"));

        Ok(())
    }
}
