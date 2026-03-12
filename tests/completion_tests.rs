//! Comprehensive completion validation tests
//!
//! These tests validate completion output format, edge cases, and error handling
//! for the jump and remove commands' --list-completions functionality.

use anyhow::Result;
use assert_fs::prelude::*;

use test_support::{CliTestEnvironment, create_worktree_config};

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

    // Create multiple worktrees with feature names and different branch names
    let worktrees = [
        ("user-auth", "feature/user-auth"),
        ("login-fix", "bugfix/login-issue"),
        ("release-v1-2", "release/v1.2.0"),
        ("critical-bug", "hotfix/critical-bug"),
        ("api-v2", "feature/api-v2"),
    ];

    for (feature, branch) in &worktrees {
        env.run_command(&["create", feature, branch])?.assert().success();
    }

    // Test completion output
    let output = get_stdout(&env, &["jump", "--list-completions"])?;

    // Verify format: one feature name per line, no extra formatting
    let lines: Vec<&str> = output.trim().split('\n').collect();
    assert_eq!(
        lines.len(),
        worktrees.len(),
        "Should output one line per worktree"
    );

    // Verify each feature name appears exactly once
    for (feature, _) in &worktrees {
        assert!(
            lines.contains(feature),
            "Completion should include feature: {}",
            feature
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
    }

    Ok(())
}

/// Test remove command completion output format and content
#[test]
fn test_remove_completion_output_format() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktrees with feature names
    let worktrees = [
        ("main-backup", "main-backup"),
        ("user-profile", "feature/user-profile"),
        ("memory-leak", "bugfix/memory_leak"),
        ("update-deps", "chore/update-deps"),
    ];

    for (feature, branch) in &worktrees {
        env.run_command(&["create", feature, branch])?.assert().success();
    }

    // Test completion output
    let output = get_stdout(&env, &["remove", "--list-completions"])?;

    // Verify format consistency with jump
    let lines: Vec<&str> = output.trim().split('\n').collect();
    assert_eq!(
        lines.len(),
        worktrees.len(),
        "Should output one line per worktree"
    );

    // Verify exact feature names in completion output
    for (feature, _) in &worktrees {
        assert!(
            lines.contains(feature),
            "Remove completion should include: {}",
            feature
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
    let current_worktrees = [
        ("current-1", "feature/current-1"),
        ("current-2", "feature/current-2"),
    ];
    for (feature, branch) in &current_worktrees {
        env.run_command(&["create", feature, branch])?.assert().success();
    }

    // Test current repo only filtering for jump
    let jump_output = get_stdout(&env, &["jump", "--list-completions", "--current"])?;
    let jump_lines: Vec<&str> = jump_output
        .trim()
        .split('\n')
        .filter(|l| !l.is_empty())
        .collect();

    for (feature, _) in &current_worktrees {
        assert!(
            jump_lines.contains(feature),
            "Current repo filter should include: {}",
            feature
        );
    }

    // Test current repo only filtering for remove
    let remove_output = get_stdout(&env, &["remove", "--list-completions", "--current"])?;
    let remove_lines: Vec<&str> = remove_output
        .trim()
        .split('\n')
        .filter(|l| !l.is_empty())
        .collect();

    for (feature, _) in &current_worktrees {
        assert!(
            remove_lines.contains(feature),
            "Current repo filter should include: {}",
            feature
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
    let worktrees = [
        ("z-last-feature", "z-last-feature"),
        ("a-first-feature", "a-first-feature"),
        ("m-middle-feature", "m-middle-feature"),
        ("zebra", "feature/zebra"),
        ("alpha", "feature/alpha"),
    ];

    for (feature, branch) in &worktrees {
        env.run_command(&["create", feature, branch])?.assert().success();
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

    // Verify all feature names are present
    let lines: Vec<&str> = output1.trim().split('\n').collect();
    assert_eq!(
        lines.len(),
        worktrees.len(),
        "Should include all created worktrees"
    );

    for (feature, _) in &worktrees {
        assert!(lines.contains(feature), "Should include feature: {}", feature);
    }

    Ok(())
}

/// Test completion output with feature names containing special characters
#[test]
fn test_completion_feature_names() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Feature names can contain hyphens, underscores, dots — but not slashes
    let worktrees = [
        ("user-profile", "feature/user_profile"),
        ("issue-123", "bugfix/issue-123"),
        ("fix-critical", "hotfix/fix-critical"),
        ("api-v2-1", "feature/api-v2.1"),
    ];

    for (feature, branch) in &worktrees {
        env.run_command(&["create", feature, branch])?.assert().success();
    }

    // Get completion output
    let output = get_stdout(&env, &["jump", "--list-completions"])?;
    let lines: Vec<&str> = output.trim().split('\n').collect();

    // Verify feature names appear in completions
    for (feature, _) in &worktrees {
        assert!(
            lines.contains(feature),
            "Should complete feature name: {}",
            feature
        );
    }

    // Verify completions show feature names (not branch names)
    assert_eq!(
        lines.len(),
        worktrees.len(),
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
    create_worktree_config(
        &env.repo_dir,
        &[".env*", ".vscode/", "*.local.json"],
        &["node_modules/", "*.log"],
    )?;

    // Create worktrees
    let worktrees = [
        ("with-config", "feature/with-config"),
        ("also-config", "bugfix/also-config"),
    ];
    for (feature, branch) in &worktrees {
        env.run_command(&["create", feature, branch])?.assert().success();
    }

    // Verify completions work normally regardless of config setup
    let output = get_stdout(&env, &["jump", "--list-completions"])?;
    let lines: Vec<&str> = output.trim().split('\n').collect();

    for (feature, _) in &worktrees {
        assert!(
            lines.contains(feature),
            "Config setup should not affect completions: {}",
            feature
        );
    }

    Ok(())
}

/// Test completion output encoding and newlines
#[test]
fn test_completion_output_encoding() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a single worktree
    env.run_command(&["create", "test-encoding", "feature/encoding"])?
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
        "test-encoding",
        "Should output exact feature name"
    );

    // Verify no carriage returns or other special characters
    assert!(
        !output.contains('\r'),
        "Should not contain carriage returns"
    );
    assert!(!output.contains('\t'), "Should not contain tabs");

    Ok(())
}

/// Test completion with very long feature names
#[test]
fn test_completion_long_feature_names() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create worktree with very long feature name
    let long_feature = "this-is-a-very-long-feature-name-that-tests-how-completions-handle-lengthy-identifiers-with-multiple-segments";
    let long_branch = "feature/very-long-branch-name-for-testing";
    env.run_command(&["create", long_feature, long_branch])?
        .assert()
        .success();

    let output = get_stdout(&env, &["jump", "--list-completions"])?;

    // Verify long feature name is included complete and untruncated
    assert!(
        output.contains(long_feature),
        "Should include full long feature name"
    );
    assert_eq!(
        output.trim(),
        long_feature,
        "Should output complete feature name without truncation"
    );

    Ok(())
}

/// Test completion error handling with corrupted storage
#[test]
fn test_completion_error_handling() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a worktree then manually corrupt its storage
    env.run_command(&["create", "test-corrupt", "feature/corrupt"])?
        .assert()
        .success();

    // Remove the actual worktree directory while keeping storage entry
    let worktree_path = env.worktree_path("test-corrupt");
    let _ = std::fs::remove_dir_all(worktree_path.child(".git").path()); // This might fail, that's ok

    // Completions should still work (might return the feature or skip it gracefully)
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
    let mut features = Vec::new();
    for i in 0..50 {
        let feature = format!("test-{:03}", i);
        let branch = format!("feature/test-{:03}", i);
        features.push(feature.clone());
        env.run_command(&["create", &feature, &branch])?.assert().success();
    }

    // Test completion still works efficiently
    let output = get_stdout(&env, &["jump", "--list-completions"])?;
    let lines: Vec<&str> = output.trim().split('\n').collect();

    assert_eq!(lines.len(), features.len(), "Should list all worktrees");

    // Verify random sampling of feature names
    assert!(
        lines.contains(&"test-000"),
        "Should include first feature"
    );
    assert!(
        lines.contains(&"test-025"),
        "Should include middle feature"
    );
    assert!(
        lines.contains(&"test-049"),
        "Should include last feature"
    );

    Ok(())
}
