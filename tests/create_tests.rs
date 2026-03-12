#![allow(clippy::unwrap_used)]

//! Integration tests for the create command (feature-named worktrees)

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::{
    CliTestEnvironment, assert_config_files_copied, create_sample_config_files,
    create_worktree_config,
};

/// Test create command with config file copying
#[test]
fn test_create_worktree_with_config_files() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    create_worktree_config(
        &env.repo_dir,
        &[".env*", ".vscode/", "*.local.json"],
        &["node_modules/"],
    )?;
    create_sample_config_files(&env.repo_dir)?;

    // Create with feature name "config-test" on branch "feature/config-test"
    env.run_command(&["create", "config-test", "feature/config-test"])?
        .assert()
        .success();

    let worktree_path = env.worktree_path("config-test");
    worktree_path.assert(predicate::path::is_dir());

    assert_config_files_copied(&worktree_path)?;

    worktree_path
        .child(".git")
        .assert(predicate::path::exists());

    // No .branch-mapping file should be created
    let mapping_file = env.storage_dir.child("test_repo").child(".branch-mapping");
    assert!(!mapping_file.path().exists(), ".branch-mapping should not be created");

    Ok(())
}

/// Test error handling when target directory already exists
#[test]
fn test_create_worktree_directory_already_exists() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    let worktree_path = env.worktree_path("existing");
    worktree_path.create_dir_all()?;

    env.run_command(&["create", "existing", "feature/existing"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    Ok(())
}

/// Test feature name validation - slash should be rejected
#[test]
fn test_create_feature_name_with_slash_rejected() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "feature/invalid"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid character").or(predicate::str::contains("/")));

    Ok(())
}

/// Test smart mode - creates new branch automatically
#[test]
fn test_create_worktree_smart_mode_new_branch() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "smart", "feature/smart-mode"])?
        .assert()
        .success();

    env.worktree_path("smart")
        .assert(predicate::path::is_dir());

    Ok(())
}

/// Test smart mode - uses existing branch
#[test]
fn test_create_worktree_smart_mode_existing_branch() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a branch first
    std::process::Command::new("git")
        .args(["branch", "feature/existing-test"])
        .current_dir(env.repo_dir.path())
        .output()?;

    env.run_command(&["create", "existing-test", "feature/existing-test"])?
        .assert()
        .success()
        .stdout(predicate::str::contains("Using existing branch"));

    env.worktree_path("existing-test")
        .assert(predicate::path::is_dir());

    Ok(())
}

/// Test git configuration inheritance in created worktrees
#[test]
fn test_git_config_inheritance() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    std::process::Command::new("git")
        .args(["config", "core.editor", "nano"])
        .current_dir(env.repo_dir.path())
        .output()?;

    env.run_command(&["create", "config-inherit", "feature/config-inherit"])?
        .assert()
        .success();

    let worktree_path = env.worktree_path("config-inherit");
    worktree_path.assert(predicate::path::exists());

    let output = std::process::Command::new("git")
        .args(["config", "extensions.worktreeConfig"])
        .current_dir(env.repo_dir.path())
        .output()?;

    let config_value = String::from_utf8(output.stdout)?;
    assert_eq!(config_value.trim(), "true");

    Ok(())
}

/// Test creating worktree with --from flag
#[test]
fn test_create_worktree_with_from_flag() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    std::process::Command::new("git")
        .args(["branch", "test-source-branch"])
        .current_dir(env.repo_dir.path())
        .output()?;

    std::process::Command::new("git")
        .args(["tag", "test-tag-v1.0"])
        .current_dir(env.repo_dir.path())
        .output()?;

    // From specific branch
    env.run_command(&[
        "create",
        "from-branch",
        "feature/from-branch",
        "--from",
        "test-source-branch",
    ])?
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "Creating new branch: feature/from-branch",
    ));

    env.worktree_path("from-branch")
        .assert(predicate::path::is_dir());

    // From tag
    env.run_command(&["create", "from-tag", "feature/from-tag", "--from", "test-tag-v1.0"])?
        .assert()
        .success();

    env.worktree_path("from-tag")
        .assert(predicate::path::is_dir());

    Ok(())
}

/// Test error handling for invalid --from references
#[test]
fn test_create_worktree_from_invalid_reference() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "invalid", "feature/invalid", "--from", "non-existent-branch"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Failed to resolve reference 'non-existent-branch'",
        ));

    assert!(!env.worktree_path("invalid").path().exists());

    Ok(())
}

/// Test --list-from-completions flag
#[test]
fn test_list_from_completions() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    std::process::Command::new("git")
        .args(["branch", "completion-test-branch"])
        .current_dir(env.repo_dir.path())
        .output()?;

    std::process::Command::new("git")
        .args(["tag", "completion-test-tag"])
        .current_dir(env.repo_dir.path())
        .output()?;

    let output = env
        .run_command(&["create", "dummy", "dummy-branch", "--list-from-completions"])?
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout_str = String::from_utf8(output)?;
    assert!(stdout_str.contains("main"));
    assert!(stdout_str.contains("completion-test-branch"));
    assert!(stdout_str.contains("completion-test-tag"));

    Ok(())
}

/// Test --from with commit hash
#[test]
fn test_create_worktree_from_commit_hash() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env.repo_dir.path())
        .output()?;

    let commit_hash = String::from_utf8(output.stdout)?.trim().to_string();

    env.run_command(&["create", "from-commit", "feature/from-commit", "--from", &commit_hash])?
        .assert()
        .success();

    env.worktree_path("from-commit")
        .assert(predicate::path::is_dir());

    Ok(())
}

/// Test that create command with no arguments triggers interactive workflow
#[test]
fn test_create_interactive_workflow_trigger() -> Result<()> {
    if CliTestEnvironment::is_ci() {
        eprintln!("Skipping interactive test in CI environment");
        return Ok(());
    }

    let env = CliTestEnvironment::new()?;

    env.run_command(&["create"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("Feature name"));

    Ok(())
}

/// Test that the create command help shows expected flags
#[test]
fn test_create_command_help() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "--help"])?
        .assert()
        .success()
        .stdout(predicate::str::contains("--from"))
        .stdout(predicate::str::contains("FEATURE_NAME"))
        .stdout(predicate::str::contains("branch"));

    // Should NOT contain old --new-branch / --existing-branch flags
    let output = env.run_command(&["create", "--help"])?.output()?;
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        !stdout.contains("--new-branch"),
        "--new-branch should be removed"
    );
    assert!(
        !stdout.contains("--existing-branch"),
        "--existing-branch should be removed"
    );

    Ok(())
}

/// Test feature name validation function
#[test]
fn test_feature_name_validation() {
    use inquire::validator::Validation;
    use worktree::commands::create::validate_feature_name_internal;

    // Valid names
    assert!(matches!(
        validate_feature_name_internal("auth"),
        Validation::Valid
    ));
    assert!(matches!(
        validate_feature_name_internal("my-feature"),
        Validation::Valid
    ));
    assert!(matches!(
        validate_feature_name_internal("feature_123"),
        Validation::Valid
    ));

    // Invalid names
    assert!(matches!(
        validate_feature_name_internal(""),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_feature_name_internal("feature/auth"),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_feature_name_internal("feat\\ure"),
        Validation::Invalid(_)
    ));
    assert!(matches!(
        validate_feature_name_internal("feat:ure"),
        Validation::Invalid(_)
    ));
}

/// Test that creating a worktree with an existing feature name fails
#[test]
fn test_feature_name_already_exists() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    env.run_command(&["create", "auth", "feature/auth"])?
        .assert()
        .success();

    env.run_command(&["create", "auth", "feature/auth-v2"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_cli_test_environment_setup() -> Result<()> {
        let env = CliTestEnvironment::new()?;

        env.repo_dir.assert(predicate::path::is_dir());
        env.repo_dir.child(".git").assert(predicate::path::exists());
        env.repo_dir
            .child("README.md")
            .assert(predicate::str::contains("# Test Repo"));

        env.storage_dir.assert(predicate::path::is_dir());

        let cmd_result = env.run_command(&["--help"]);
        assert!(cmd_result.is_ok(), "Should be able to create command");

        Ok(())
    }

    #[test]
    fn test_pattern_helpers() -> Result<()> {
        let env = CliTestEnvironment::new()?;

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

        create_sample_config_files(&env.repo_dir)?;

        env.repo_dir
            .child(".env")
            .assert(predicate::str::contains("TEST_VAR"));
        env.repo_dir
            .child(".vscode")
            .child("settings.json")
            .assert(predicate::path::exists());

        Ok(())
    }
}
