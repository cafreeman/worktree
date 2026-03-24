#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

//! Modern integration tests for the jump command
//!
//! These tests validate jump command path output, completion functionality,
//! and error handling using real CLI execution and filesystem assertions.

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::CliTestEnvironment;

/// Helper function to get stdout from command execution
fn get_stdout(env: &CliTestEnvironment, args: &[&str]) -> Result<String> {
    let assert_output = env.run_command(args)?.assert().success();
    let output = assert_output.get_output();
    Ok(String::from_utf8(output.stdout.clone())?)
}

/// Test jump command outputs correct worktree path for shell integration
#[test]
fn test_jump_path_output() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree with feature name "path-test" on branch "feature/path-test"
    env.run_command(&["create", "path-test", "feature/path-test"])?
        .assert()
        .success();

    // Verify worktree exists at feature name path
    let worktree_path = env.worktree_path("path-test");
    worktree_path.assert(predicate::path::is_dir());

    // Test jump command outputs the correct path using feature name
    let output_path = get_stdout(&env, &["jump", "path-test"])?;
    assert_eq!(output_path.trim(), worktree_path.to_string_lossy());

    Ok(())
}

/// Test jump command with partial feature name matching
#[test]
fn test_jump_partial_matching() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree with unique feature name
    env.run_command(&["create", "unique-identifier", "feature/unique-identifier"])?
        .assert()
        .success();

    // Jump with partial match should work
    let output_path = get_stdout(&env, &["jump", "unique"])?;
    let expected_path = env.worktree_path("unique-identifier");
    assert_eq!(output_path.trim(), expected_path.to_string_lossy());

    Ok(())
}

/// Test error handling for ambiguous partial matches
#[test]
fn test_jump_ambiguous_match() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create multiple worktrees with similar feature names
    env.run_command(&["create", "test-alpha", "feature/test-alpha"])?
        .assert()
        .success();

    env.run_command(&["create", "test-beta", "feature/test-beta"])?
        .assert()
        .success();

    // Jump with ambiguous partial match should fail with helpful error
    env.run_command(&["jump", "test"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("Ambiguous worktree name"))
        .stderr(predicate::str::contains("test-alpha"))
        .stderr(predicate::str::contains("test-beta"));

    Ok(())
}

/// Test error handling when jumping to nonexistent worktree
#[test]
fn test_jump_nonexistent_worktree() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Try to jump to nonexistent worktree
    env.run_command(&["jump", "nonexistent"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("No worktree found matching"));

    Ok(())
}

/// Test completion mode lists available worktrees by feature name
#[test]
fn test_jump_list_completions() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create some worktrees
    env.run_command(&["create", "completion1", "feature/completion1"])?
        .assert()
        .success();

    env.run_command(&["create", "completion2", "feature/completion2"])?
        .assert()
        .success();

    // Test completion listing — should show feature names
    let stdout = get_stdout(&env, &["jump", "--list-completions"])?;

    assert!(stdout.contains("completion1"));
    assert!(stdout.contains("completion2"));

    Ok(())
}

/// Test completion with current repo filtering
#[test]
fn test_jump_completions_current_repo_only() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees
    env.run_command(&["create", "current-test", "feature/current-test"])?
        .assert()
        .success();

    // Test completion with current repo filter
    let stdout = get_stdout(&env, &["jump", "--list-completions", "--current"])?;
    assert!(stdout.contains("current-test"));

    Ok(())
}

/// Test completion when no worktrees exist
#[test]
fn test_jump_completions_empty() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Test completion with no worktrees
    let stdout = get_stdout(&env, &["jump", "--list-completions"])?;
    // Should succeed but output nothing
    assert!(stdout.trim().is_empty());

    Ok(())
}

/// Test jump with feature names (no slash in feature name)
#[test]
fn test_jump_with_feature_names() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktree with feature name (branch can have slash, feature name cannot)
    env.run_command(&["create", "test-branch", "feature/test-branch"])?
        .assert()
        .success();

    // Should be able to jump using feature name
    let output_path = get_stdout(&env, &["jump", "test-branch"])?;
    let expected_path = env.worktree_path("test-branch");
    assert_eq!(output_path.trim(), expected_path.to_string_lossy());

    Ok(())
}

/// Test error when no worktrees exist for interactive mode
#[test]
fn test_jump_interactive_no_worktrees() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Try interactive mode with no worktrees
    env.run_command(&["jump", "--interactive"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("No worktrees found"));

    Ok(())
}

/// Test current repository filtering functionality
#[test]
fn test_jump_current_repo_filtering() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree in current repo
    env.run_command(&["create", "current-filter", "feature/current-filter"])?
        .assert()
        .success();

    // Jump with current repo only should work
    let output_path = get_stdout(&env, &["jump", "current-filter", "--current"])?;
    let expected_path = env.worktree_path("current-filter");
    assert_eq!(output_path.trim(), expected_path.to_string_lossy());

    Ok(())
}

/// Test jumping between multiple worktrees by feature name
#[test]
fn test_jump_multiple_worktrees() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create multiple worktrees using feature-name + branch pairs
    let worktrees = [
        ("auth", "feature/auth"),
        ("payments", "feature/payments"),
        ("critical", "bugfix/critical"),
    ];

    for (feature, branch) in &worktrees {
        env.run_command(&["create", feature, branch])?
            .assert()
            .success();
    }

    // Test jumping to each one by feature name
    for (feature, _) in &worktrees {
        let output_path = get_stdout(&env, &["jump", feature])?;
        let expected_path = env.worktree_path(feature);
        assert_eq!(output_path.trim(), expected_path.to_string_lossy());
    }

    Ok(())
}
