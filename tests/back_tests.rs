//! Modern integration tests for the back command
//!
//! These tests validate the back command CLI behavior, focusing on help and error conditions
//! since the back command requires running from within a worktree directory.

use anyhow::Result;

use test_support::CliTestEnvironment;

/// Helper function to get stdout from command execution
fn get_stdout(env: &CliTestEnvironment, args: &[&str]) -> Result<String> {
    let assert_output = env.run_command(args)?.assert().success();
    let output = assert_output.get_output();
    Ok(String::from_utf8(output.stdout.clone())?)
}

/// Test back command help
#[test]
fn test_back_command_help() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Test help flag
    let help_str = get_stdout(&env, &["back", "--help"])?;
    assert!(
        help_str.contains("back"),
        "Help output should mention the back command"
    );

    Ok(())
}

/// Test back command error when not in worktree directory
#[test]
fn test_back_command_error_handling() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Back command should fail when not in a worktree directory
    let mut cmd = env.run_command(&["back"])?;
    cmd.assert().failure(); // Should exit with failure code

    Ok(())
}

/// Test back command requires worktree context
#[test]
fn test_back_requires_worktree_context() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree for completeness but test error from repo root
    env.run_command(&["create", "feature/test-context"])?
        .assert()
        .success();

    // Test that back command shows appropriate error from repo root
    let mut cmd = env.run_command(&["back"])?;
    let assert_result = cmd.assert().failure();
    let output = assert_result.get_output();
    let stderr = String::from_utf8(output.stderr.clone())?;

    assert!(
        stderr.contains("worktree directory"),
        "Error message should mention worktree directory requirement"
    );

    Ok(())
}
