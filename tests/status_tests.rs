//! Modern integration tests for the status command
//!
//! These tests validate the status command CLI behavior using real command execution.

use anyhow::Result;

mod cli_test_helpers;
use cli_test_helpers::CliTestEnvironment;

/// Helper function to get stdout from command execution
fn get_stdout(env: &CliTestEnvironment, args: &[&str]) -> Result<String> {
    let assert_output = env.run_command(args)?.assert().success();
    let output = assert_output.get_output();
    Ok(String::from_utf8(output.stdout.clone())?)
}

/// Test status command with no worktrees
#[test]
fn test_status_empty() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Status should succeed even with no worktrees
    env.run_command(&["status"])?.assert().success();

    Ok(())
}

/// Test status command with worktrees
#[test]
fn test_status_with_worktrees() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create several worktrees
    let branches = ["feature/status-test", "bugfix/status"];
    for branch in &branches {
        env.run_command(&["create", branch])?.assert().success();
    }

    // Status should succeed with worktrees
    let output_str = get_stdout(&env, &["status"])?;

    // Should contain some information about the worktrees
    assert!(
        !output_str.trim().is_empty(),
        "Status command should produce some output with worktrees"
    );

    Ok(())
}

/// Test status command help
#[test]
fn test_status_help() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Test help flag
    let help_str = get_stdout(&env, &["status", "--help"])?;
    assert!(
        help_str.contains("status"),
        "Help output should mention the status command"
    );

    Ok(())
}

/// Test status command basic functionality
#[test]
fn test_status_basic() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree
    env.run_command(&["create", "feature/basic-status"])?
        .assert()
        .success();

    // Status command should work
    env.run_command(&["status"])?.assert().success();

    Ok(())
}
