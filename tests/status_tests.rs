//! Modern integration tests for the status command
//!
//! These tests validate the status command CLI behavior using real command execution.

use anyhow::Result;
use std::process::Command;

use test_support::CliTestEnvironment;

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

    // Create several worktrees using feature-name + branch pairs
    let worktrees = [
        ("status-test", "feature/status-test"),
        ("status-fix", "bugfix/status"),
    ];
    for (feature, branch) in &worktrees {
        env.run_command(&["create", feature, branch])?
            .assert()
            .success();
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

    // Create a worktree using feature-name + branch
    env.run_command(&["create", "basic-status", "feature/basic-status"])?
        .assert()
        .success();

    // Status command should work
    env.run_command(&["status"])?.assert().success();

    Ok(())
}

#[test]
fn test_status_reports_foreign_worktree_as_unmanaged() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    let foreign_path = env
        .repo_dir
        .path()
        .parent()
        .ok_or_else(|| anyhow::anyhow!("test repository has no parent"))?
        .join("foreign-worktree");

    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            "--detach",
            foreign_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("foreign path is not UTF-8"))?,
            "HEAD",
        ])
        .current_dir(env.repo_dir.path())
        .output()?;
    assert!(
        output.status.success(),
        "failed to create foreign worktree: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let status = get_stdout(&env, &["status"])?;
    assert!(status.contains("unmanaged"));
    assert!(
        status.contains(
            foreign_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("foreign path is not UTF-8"))?
        )
    );
    assert!(!status.contains("✗ foreign-worktree"));

    Ok(())
}

#[test]
fn test_status_json_contains_registration_and_drift_fields() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "json-status", "feature/json-status"])?
        .assert()
        .success();

    let output = get_stdout(&env, &["status", "--json"])?;
    let json: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(json["repository"], "test_repo");
    assert!(json["git_worktrees"].is_array());
    assert_eq!(json["managed_worktrees"][0]["feature"], "json-status");
    assert_eq!(json["managed_worktrees"][0]["registered"], true);
    assert!(json["managed_worktrees"][0]["drift"].is_array());

    Ok(())
}
