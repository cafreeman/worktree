//! Comprehensive completion validation tests
//!
//! These tests validate completion output format, edge cases, and error handling
//! for the jump and remove commands' --list-completions functionality.

use anyhow::Result;
use assert_fs::prelude::*;

mod cli_test_helpers;
use cli_test_helpers::{CliTestEnvironment, patterns};

/// Helper function to get stdout from command execution
fn get_stdout(env: &CliTestEnvironment, args: &[&str]) -> Result<String> {
    let assert_output = env.run_command(args)?.assert().success();
    let output = assert_output.get_output();
    Ok(String::from_utf8(output.stdout.clone())?)
}

/// Test jump command completion output format and content
#[test]
fn test_jump_completion_output_format() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create multiple worktrees with different naming patterns
    let branches = [
        "feature/user-auth",
        "bugfix/login-issue",
        "release/v1.2.0",
        "hotfix/critical-bug",
        "feature/api-v2",
    ];

    for branch in &branches {
        env.run_command(&["create", branch])?.assert().success();
    }

    // Test completion output
    let output = get_stdout(&env, &["jump", "--list-completions"])?;

    // Verify format: one branch per line, no extra formatting
    let lines: Vec<&str> = output.trim().split('\n').collect();
    assert_eq!(
        lines.len(),
        branches.len(),
        "Should output one line per worktree"
    );

    // Verify each branch appears exactly once
    for branch in &branches {
        assert!(
            lines.contains(branch),
            "Completion should include branch: {}",
            branch
        );
    }

    // Verify no extra formatting (no paths, no descriptions)
    for line in &lines {
        assert!(
            !line.contains('('),
            "Completion lines should not contain paths: {}",
            line
        );
        assert!(
            !line.contains(" ("),
            "Completion lines should not contain path descriptions: {}",
            line
        );
        assert!(
            line.chars()
                .all(|c| c.is_alphanumeric() || "-/_.".contains(c)),
            "Completion line has unexpected characters: {}",
            line
        );
    }

    Ok(())
}

/// Test remove command completion output format and content
#[test]
fn test_remove_completion_output_format() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees with edge case names
    let branches = [
        "main-backup",
        "feature/user-profile",
        "bugfix/memory_leak",
        "chore/update-deps",
    ];

    for branch in &branches {
        env.run_command(&["create", branch])?.assert().success();
    }

    // Test completion output
    let output = get_stdout(&env, &["remove", "--list-completions"])?;

    // Verify format consistency with jump
    let lines: Vec<&str> = output.trim().split('\n').collect();
    assert_eq!(
        lines.len(),
        branches.len(),
        "Should output one line per worktree"
    );

    // Verify exact branch names (no sanitization in completion output)
    for branch in &branches {
        assert!(
            lines.contains(branch),
            "Remove completion should include: {}",
            branch
        );
    }

    Ok(())
}

/// Test completion output when no worktrees exist
#[test]
fn test_completion_output_empty() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Test jump completions with empty storage
    let jump_output = get_stdout(&env, &["jump", "--list-completions"])?;
    assert!(
        jump_output.trim().is_empty(),
        "Jump completions should be empty when no worktrees exist"
    );

    // Test remove completions with empty storage
    let remove_output = get_stdout(&env, &["remove", "--list-completions"])?;
    assert!(
        remove_output.trim().is_empty(),
        "Remove completions should be empty when no worktrees exist"
    );

    Ok(())
}

/// Test completion output with current repo filtering
#[test]
fn test_completion_current_repo_filtering() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees in current repo
    let current_branches = ["feature/current-1", "feature/current-2"];
    for branch in &current_branches {
        env.run_command(&["create", branch])?.assert().success();
    }

    // Test current repo only filtering for jump
    let jump_output = get_stdout(&env, &["jump", "--list-completions", "--current"])?;
    let jump_lines: Vec<&str> = jump_output
        .trim()
        .split('\n')
        .filter(|l| !l.is_empty())
        .collect();

    for branch in &current_branches {
        assert!(
            jump_lines.contains(branch),
            "Current repo filter should include: {}",
            branch
        );
    }

    // Test current repo only filtering for remove
    let remove_output = get_stdout(&env, &["remove", "--list-completions", "--current"])?;
    let remove_lines: Vec<&str> = remove_output
        .trim()
        .split('\n')
        .filter(|l| !l.is_empty())
        .collect();

    for branch in &current_branches {
        assert!(
            remove_lines.contains(branch),
            "Current repo filter should include: {}",
            branch
        );
    }

    // Verify both commands return the same completions with --current
    assert_eq!(
        jump_lines, remove_lines,
        "Jump and remove should return same completions with --current"
    );

    Ok(())
}

/// Test completion output stability and ordering
#[test]
fn test_completion_output_stability() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees in non-alphabetical order
    let branches = [
        "z-last-feature",
        "a-first-feature",
        "m-middle-feature",
        "feature/zebra",
        "feature/alpha",
    ];

    for branch in &branches {
        env.run_command(&["create", branch])?.assert().success();
    }

    // Get completion output multiple times
    let output1 = get_stdout(&env, &["jump", "--list-completions"])?;
    let output2 = get_stdout(&env, &["jump", "--list-completions"])?;
    let output3 = get_stdout(&env, &["remove", "--list-completions"])?;

    // Verify output is stable (same order each time)
    assert_eq!(output1, output2, "Jump completion output should be stable");
    assert_eq!(
        output1, output3,
        "Jump and remove completions should be identical"
    );

    // Verify all branches are present
    let lines: Vec<&str> = output1.trim().split('\n').collect();
    assert_eq!(
        lines.len(),
        branches.len(),
        "Should include all created worktrees"
    );

    for branch in &branches {
        assert!(lines.contains(branch), "Should include branch: {}", branch);
    }

    Ok(())
}

/// Test completion output with special characters in branch names
#[test]
fn test_completion_special_characters() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create branches with special characters that get sanitized for filesystem
    let branches = [
        "feature/user_profile",
        "bugfix/issue-123",
        "hotfix/fix-critical",
        "feature/api-v2.1",
    ];

    for branch in &branches {
        env.run_command(&["create", branch])?.assert().success();
    }

    // Get completion output
    let output = get_stdout(&env, &["jump", "--list-completions"])?;
    let lines: Vec<&str> = output.trim().split('\n').collect();

    // Verify original branch names appear in completions (not sanitized versions)
    for branch in &branches {
        assert!(
            lines.contains(branch),
            "Should complete original branch name: {}",
            branch
        );
    }

    // Verify completions show the original branch names (this test focuses on format consistency)
    assert_eq!(
        lines.len(),
        branches.len(),
        "Should have correct number of completions"
    );

    // Verify output format is consistent
    for line in &lines {
        assert!(
            !line.contains('('),
            "Completion should not contain path info"
        );
        assert!(!line.is_empty(), "Completion lines should not be empty");
    }

    Ok(())
}

/// Test completion behavior with config file patterns
#[test]
fn test_completion_with_config_setup() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Setup config patterns
    patterns::create_worktree_config(
        &env.repo_dir,
        &[".env*", ".vscode/", "*.local.json"],
        &["node_modules/", "*.log"],
    )?;

    // Create worktrees
    let branches = ["feature/with-config", "bugfix/also-config"];
    for branch in &branches {
        env.run_command(&["create", branch])?.assert().success();
    }

    // Verify completions work normally regardless of config setup
    let output = get_stdout(&env, &["jump", "--list-completions"])?;
    let lines: Vec<&str> = output.trim().split('\n').collect();

    for branch in &branches {
        assert!(
            lines.contains(branch),
            "Config setup should not affect completions: {}",
            branch
        );
    }

    Ok(())
}

/// Test completion output encoding and newlines
#[test]
fn test_completion_output_encoding() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a single worktree to test output format
    env.run_command(&["create", "test/encoding"])?
        .assert()
        .success();

    let output = get_stdout(&env, &["jump", "--list-completions"])?;

    // Verify output ends with newline but doesn't have extra newlines
    assert!(
        output.ends_with('\n'),
        "Completion output should end with newline"
    );
    assert_eq!(
        output.trim().lines().count(),
        1,
        "Should have exactly one line of content"
    );
    assert_eq!(
        output.trim(),
        "test/encoding",
        "Should output exact branch name"
    );

    // Verify no carriage returns or other special characters
    assert!(
        !output.contains('\r'),
        "Should not contain carriage returns"
    );
    assert!(!output.contains('\t'), "Should not contain tabs");

    Ok(())
}

/// Test completion with very long branch names
#[test]
fn test_completion_long_branch_names() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktree with very long branch name
    let long_branch = "feature/this-is-a-very-long-branch-name-that-tests-how-completions-handle-lengthy-identifiers-with-multiple-segments";
    env.run_command(&["create", long_branch])?
        .assert()
        .success();

    let output = get_stdout(&env, &["jump", "--list-completions"])?;

    // Verify long branch name is included complete and untruncated
    assert!(
        output.contains(long_branch),
        "Should include full long branch name"
    );
    assert_eq!(
        output.trim(),
        long_branch,
        "Should output complete branch name without truncation"
    );

    Ok(())
}

/// Test completion error handling with corrupted storage
#[test]
fn test_completion_error_handling() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree then manually corrupt its storage
    env.run_command(&["create", "test/corrupt"])?
        .assert()
        .success();

    // Remove the actual worktree directory while keeping storage entry
    let worktree_path = env.worktree_path("test/corrupt");
    let _ = std::fs::remove_dir_all(worktree_path.child(".git").path()); // This might fail, that's ok

    // Completions should still work (might return the branch or skip it gracefully)
    let result = env.run_command(&["jump", "--list-completions"]);

    // Command should not panic/crash, regardless of whether it includes the corrupted entry
    match result {
        Ok(mut cmd) => {
            cmd.assert().success();
            // If successful, output should be valid UTF-8 at minimum
        }
        Err(_) => {
            // If it fails, it should fail gracefully, not panic
            // This is acceptable behavior for corrupted storage
        }
    }

    Ok(())
}

/// Test completion performance with many worktrees
#[test]
fn test_completion_performance_many_worktrees() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create many worktrees to test performance
    let mut branches = Vec::new();
    for i in 0..50 {
        let branch = format!("feature/test-{:03}", i);
        branches.push(branch.clone());
        env.run_command(&["create", &branch])?.assert().success();
    }

    // Test completion still works efficiently
    let output = get_stdout(&env, &["jump", "--list-completions"])?;
    let lines: Vec<&str> = output.trim().split('\n').collect();

    assert_eq!(lines.len(), branches.len(), "Should list all worktrees");

    // Verify random sampling of branches
    assert!(
        lines.contains(&"feature/test-000"),
        "Should include first branch"
    );
    assert!(
        lines.contains(&"feature/test-025"),
        "Should include middle branch"
    );
    assert!(
        lines.contains(&"feature/test-049"),
        "Should include last branch"
    );

    Ok(())
}
