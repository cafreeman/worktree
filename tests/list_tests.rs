//! Modern integration tests for the list command
//!
//! These tests validate the list command CLI behavior using real command execution.

use anyhow::Result;

mod cli_test_helpers;
use cli_test_helpers::CliTestEnvironment;

/// Helper function to get stdout from command execution
fn get_stdout(env: &CliTestEnvironment, args: &[&str]) -> Result<String> {
    let assert_output = env.run_command(args)?.assert().success();
    let output = assert_output.get_output();
    Ok(String::from_utf8(output.stdout.clone())?)
}

/// Test list command with no worktrees
#[test]
fn test_list_empty() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // List should succeed even with no worktrees
    let output = get_stdout(&env, &["list"])?;

    // Output should be empty or contain header only
    assert!(
        output.trim().is_empty() || output.contains("No worktrees"),
        "List command should handle empty case gracefully"
    );

    Ok(())
}

/// Test list command with multiple worktrees
#[test]
fn test_list_multiple_worktrees() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create several worktrees
    let branches = ["feature/list-test", "bugfix/minor", "release/v1.0"];
    for branch in &branches {
        env.run_command(&["create", branch])?.assert().success();
    }

    // Test list command
    let output = get_stdout(&env, &["list"])?;

    // All branches should appear in the output
    for branch in &branches {
        assert!(
            output.contains(branch),
            "List output should contain branch: {}",
            branch
        );
    }

    Ok(())
}

/// Test list command with current repo flag
#[test]
fn test_list_current_repo() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree
    env.run_command(&["create", "feature/current-test"])?
        .assert()
        .success();

    // Test list with current repo flag
    let output = get_stdout(&env, &["list", "--current"])?;

    assert!(
        output.contains("feature/current-test"),
        "List --current should show current repo worktrees"
    );

    Ok(())
}
