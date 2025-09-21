#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

//! Modern integration tests for the create command
//!
//! These tests use assert_fs for declarative filesystem setup and validation,
//! testing the create command with real CLI execution and comprehensive
//! error handling scenarios.

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::{
    CliTestEnvironment, assert_config_files_copied, create_sample_config_files,
    create_worktree_config,
};

/// Test create command with config file copying using declarative filesystem assertions
#[test]
fn test_create_worktree_with_config_files() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Declarative filesystem setup
    create_worktree_config(
        &env.repo_dir,
        &[".env*", ".vscode/", "*.local.json"],
        &["node_modules/"],
    )?;
    create_sample_config_files(&env.repo_dir)?;

    // Execute command with assert_cmd
    env.run_command(&["create", "feature/config-test"])?
        .assert()
        .success();

    // Declarative assertions using predicates
    let worktree_path = env.worktree_path("feature/config-test");
    worktree_path.assert(predicate::path::is_dir());

    // Verify config files were copied
    assert_config_files_copied(&worktree_path)?;

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
    // Create a branch but don't check it out
    std::process::Command::new("git")
        .args(["branch", "feature/existing-test"])
        .current_dir(env.repo_dir.path())
        .output()?;

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

    // Set some custom git config in the test repo
    std::process::Command::new("git")
        .args(["config", "core.editor", "nano"])
        .current_dir(env.repo_dir.path())
        .output()?;

    std::process::Command::new("git")
        .args(["config", "user.signingkey", "test-key-123"])
        .current_dir(env.repo_dir.path())
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
        .current_dir(worktree_path.path())
        .output()?;

    if output.status.success() {
        let user_name = String::from_utf8(output.stdout)?;
        assert_eq!(user_name.trim(), "Test User");
    }

    // Check that extensions.worktreeConfig is enabled
    let output = std::process::Command::new("git")
        .args(["config", "extensions.worktreeConfig"])
        .current_dir(env.repo_dir.path())
        .output()?;

    let config_value = String::from_utf8(output.stdout)?;
    assert_eq!(config_value.trim(), "true");

    Ok(())
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
        mapping_file.assert(predicate::str::contains(format!(
            "{} -> {}",
            expected_dir, original_branch
        )));
    }

    Ok(())
}

/// Test creating worktree with --from flag from different reference types
#[test]
fn test_create_worktree_with_from_flag() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a test branch and tag for testing --from functionality
    std::process::Command::new("git")
        .args(["branch", "test-source-branch"])
        .current_dir(env.repo_dir.path())
        .output()?;

    std::process::Command::new("git")
        .args(["tag", "test-tag-v1.0"])
        .current_dir(env.repo_dir.path())
        .output()?;

    // Test 1: Create worktree from specific branch
    env.run_command(&[
        "create",
        "feature/from-branch",
        "--from",
        "test-source-branch",
    ])?
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "Creating new branch: feature/from-branch",
    ))
    .stdout(predicate::str::contains("✓ Worktree created successfully!"));

    env.worktree_path("feature/from-branch")
        .assert(predicate::path::is_dir());

    // Test 2: Create worktree from tag
    env.run_command(&["create", "feature/from-tag", "--from", "test-tag-v1.0"])?
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Creating new branch: feature/from-tag",
        ));

    env.worktree_path("feature/from-tag")
        .assert(predicate::path::is_dir());

    // Test 3: Create worktree from main branch (existing)
    env.run_command(&["create", "feature/from-main", "--from", "main"])?
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Creating new branch: feature/from-main",
        ));

    env.worktree_path("feature/from-main")
        .assert(predicate::path::is_dir());

    Ok(())
}

/// Test --from flag with existing branch mode (should ignore --from)
#[test]
fn test_create_worktree_from_with_existing_branch_mode() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a source branch
    std::process::Command::new("git")
        .args(["branch", "source-branch"])
        .current_dir(env.repo_dir.path())
        .output()?;

    // Create target branch to use in existing mode
    std::process::Command::new("git")
        .args(["branch", "target-branch"])
        .current_dir(env.repo_dir.path())
        .output()?;

    // Test: --from should be ignored when using existing branch
    env.run_command(&[
        "create",
        "--existing-branch",
        "target-branch",
        "--from",
        "source-branch",
    ])?
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "Using existing branch: target-branch",
    ));

    env.worktree_path("target-branch")
        .assert(predicate::path::is_dir());

    Ok(())
}

/// Test --from flag with --new-branch mode
#[test]
fn test_create_worktree_from_with_new_branch_mode() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a source branch
    std::process::Command::new("git")
        .args(["branch", "source-for-new"])
        .current_dir(env.repo_dir.path())
        .output()?;

    // Test: --from should work with --new-branch mode
    env.run_command(&[
        "create",
        "--new-branch",
        "new-from-source",
        "--from",
        "source-for-new",
    ])?
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "Creating new branch: new-from-source",
    ));

    env.worktree_path("new-from-source")
        .assert(predicate::path::is_dir());

    Ok(())
}

/// Test error handling for invalid --from references
#[test]
fn test_create_worktree_from_invalid_reference() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Test 1: Invalid branch name
    env.run_command(&["create", "feature/invalid", "--from", "non-existent-branch"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Failed to resolve reference 'non-existent-branch'",
        ));

    // Test 2: Invalid commit hash
    env.run_command(&["create", "feature/invalid2", "--from", "abcdef123456"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Failed to resolve reference 'abcdef123456'",
        ));

    // Test 3: Invalid tag
    env.run_command(&["create", "feature/invalid3", "--from", "non-existent-tag"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Failed to resolve reference 'non-existent-tag'",
        ));

    // Verify no worktrees were created
    assert!(!env.worktree_path("feature/invalid").path().exists());
    assert!(!env.worktree_path("feature/invalid2").path().exists());
    assert!(!env.worktree_path("feature/invalid3").path().exists());

    Ok(())
}

/// Test --list-from-completions flag
#[test]
fn test_list_from_completions() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create some test branches and tags
    std::process::Command::new("git")
        .args(["branch", "completion-test-branch"])
        .current_dir(env.repo_dir.path())
        .output()?;

    std::process::Command::new("git")
        .args(["tag", "completion-test-tag"])
        .current_dir(env.repo_dir.path())
        .output()?;

    // Test completion output
    let output = env
        .run_command(&["create", "dummy", "--list-from-completions"])?
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout_str = String::from_utf8(output)?;

    // Should contain local branches
    assert!(stdout_str.contains("main"));
    assert!(stdout_str.contains("completion-test-branch"));

    // Should contain tags
    assert!(stdout_str.contains("completion-test-tag"));

    // Each reference should be on its own line
    let lines: Vec<&str> = stdout_str.trim().split('\n').collect();
    assert!(lines.len() >= 3); // At least main, test-branch, and test-tag

    // Verify format (one reference per line, no extra characters)
    for line in lines {
        assert!(!line.trim().is_empty());
        assert!(!line.contains(' ')); // No spaces in reference names
    }

    Ok(())
}

/// Test --from with commit hash
#[test]
fn test_create_worktree_from_commit_hash() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Get the current commit hash
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env.repo_dir.path())
        .output()?;

    let commit_hash = String::from_utf8(output.stdout)?.trim().to_string();

    // Test creating worktree from full commit hash
    env.run_command(&["create", "feature/from-commit", "--from", &commit_hash])?
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Creating new branch: feature/from-commit",
        ));

    env.worktree_path("feature/from-commit")
        .assert(predicate::path::is_dir());

    // Test creating worktree from short commit hash (first 7 characters)
    let short_hash = &commit_hash[..7];
    env.run_command(&["create", "feature/from-short-commit", "--from", short_hash])?
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Creating new branch: feature/from-short-commit",
        ));

    env.worktree_path("feature/from-short-commit")
        .assert(predicate::path::is_dir());

    Ok(())
}

/// Test interactive --from selection functionality with various git references
#[test]
fn test_create_worktree_interactive_from_selection() -> Result<()> {
    // Skip this test in CI environments where TTY is not available
    if CliTestEnvironment::is_ci() {
        eprintln!("Skipping interactive test in CI environment");
        return Ok(());
    }

    let env = CliTestEnvironment::new()?;

    // Create test references: local branch, remote branch, and tag
    std::process::Command::new("git")
        .args(["branch", "test-interactive-branch"])
        .current_dir(env.repo_dir.path())
        .output()?;

    std::process::Command::new("git")
        .args(["tag", "test-interactive-tag"])
        .current_dir(env.repo_dir.path())
        .output()?;

    // Create a remote and remote branch
    std::process::Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/test/repo.git",
        ])
        .current_dir(env.repo_dir.path())
        .output()?;

    std::process::Command::new("git")
        .args(["branch", "origin/test-remote-branch"])
        .current_dir(env.repo_dir.path())
        .output()?;

    // Test that the interactive mode flag is accepted and launches the interactive UI
    // The command will fail because there's no TTY for interactive input, but it should
    // show that the interactive selection was attempted
    env.run_command(&["create", "test-branch", "--interactive-from"])?
        .assert()
        .failure() // Should fail because we don't have interactive input (no TTY)
        .stderr(predicate::str::contains("Choose a category:"));
    // This confirms the interactive selection UI was launched

    Ok(())
}

/// Test interactive selection with no git references available
#[test]
fn test_interactive_from_selection_no_references() -> Result<()> {
    // Create a minimal test environment with no branches or tags
    let temp_dir = assert_fs::TempDir::new()?;
    let repo_dir = temp_dir.child("repo");
    let storage_dir = temp_dir.child("storage");

    // Create actual directories
    std::fs::create_dir_all(&repo_dir)?;
    std::fs::create_dir_all(&storage_dir)?;

    // Initialize empty git repo (no commits, no branches beyond initial)
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .output()?;

    // First build the binary
    let project_root = std::env::current_dir()?; // Get the project root
    std::process::Command::new("cargo")
        .args(["build", "--bin", "worktree-bin"])
        .current_dir(&project_root)
        .output()?;

    // Test the interactive mode with no references
    let binary_path = project_root.join("target/debug/worktree-bin");
    let cmd_result = std::process::Command::new(&binary_path)
        .args(["create", "test", "--interactive-from"])
        .current_dir(&repo_dir) // Run from the git repo directory
        .env("HOME", storage_dir.path())
        .env("WORKTREE_STORAGE_ROOT", storage_dir.path()) // Set storage root
        .output()?;

    // Should fail with "No git references found" error
    assert!(!cmd_result.status.success());
    let stderr = String::from_utf8(cmd_result.stderr)?;
    assert!(stderr.contains("No git references found"));

    Ok(())
}

/// Test that create command with no arguments triggers interactive workflow
#[test]
fn test_create_interactive_workflow_trigger() -> Result<()> {
    // Skip this test in CI environments where TTY is not available
    if CliTestEnvironment::is_ci() {
        eprintln!("Skipping interactive test in CI environment");
        return Ok(());
    }

    let env = CliTestEnvironment::new()?;

    // Test that the interactive workflow is launched when no branch name is provided
    // The command will fail because there's no TTY for interactive input, but it should
    // show the interactive branch name prompt
    env.run_command(&["create"])?
        .assert()
        .failure() // Should fail because we don't have interactive input (no TTY)
        .stderr(predicate::str::contains(
            "Enter the branch name for the new worktree",
        ));
    // This confirms the interactive workflow was launched

    Ok(())
}

/// Test that list-from-completions works correctly for shell completion
#[test]
fn test_list_from_completions_integration() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create some test references
    std::process::Command::new("git")
        .args(["branch", "feature/test-branch"])
        .current_dir(env.repo_dir.path())
        .output()?;

    std::process::Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(env.repo_dir.path())
        .output()?;

    // Test the list-from-completions output
    let mut cmd = env.run_command(&["create", "dummy", "--list-from-completions"])?;
    cmd.assert().success();
    let output = cmd.output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let lines: Vec<&str> = stdout.lines().collect();

    // Should contain our test references
    assert!(lines.contains(&"main"));
    assert!(lines.contains(&"feature/test-branch"));
    assert!(lines.contains(&"v1.0.0"));

    Ok(())
}

/// Test that the create command help shows expected flags
#[test]
fn test_create_command_help() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Test that help output shows the expected user-facing flags
    env.run_command(&["create", "--help"])?
        .assert()
        .success()
        .stdout(predicate::str::contains("--from"))
        .stdout(predicate::str::contains("--new-branch"))
        .stdout(predicate::str::contains("--existing-branch"));

    Ok(())
}

/// Test branch name validation function
#[test]
fn test_branch_name_validation() {
    use inquire::validator::Validation;
    use worktree::commands::create::validate_branch_name_internal;

    // Valid names - should return Validation::Valid
    assert!(matches!(
        validate_branch_name_internal("feature/auth"),
        Validation::Valid
    ));
    assert!(matches!(
        validate_branch_name_internal("bugfix-123"),
        Validation::Valid
    ));
    assert!(matches!(
        validate_branch_name_internal("main"),
        Validation::Valid
    ));
    assert!(matches!(
        validate_branch_name_internal("develop"),
        Validation::Valid
    ));
    assert!(matches!(
        validate_branch_name_internal("feature_branch"),
        Validation::Valid
    ));

    // Invalid names - should return Validation::Invalid(...)
    assert!(matches!(
        validate_branch_name_internal(""),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_branch_name_internal("   "),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_branch_name_internal("feature..auth"),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_branch_name_internal("/feature"),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_branch_name_internal("feature/"),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_branch_name_internal("feature auth"),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_branch_name_internal("feature~"),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_branch_name_internal("feature^"),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_branch_name_internal("feature:"),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_branch_name_internal("feature?"),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_branch_name_internal("feature*"),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_branch_name_internal("feature["),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_branch_name_internal("feature\\"),
        Validation::Invalid(_)
    ));
}

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
        create_worktree_config(
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
        create_sample_config_files(&env.repo_dir)?;

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
