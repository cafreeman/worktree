#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

//! Modern workflow integration tests
//!
//! These tests validate complete user workflows combining multiple commands,
//! including create → configure → jump → sync → remove cycles. They test
//! real user scenarios and edge cases with comprehensive error handling.

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::{
    CliTestEnvironment, assert_config_files_copied, create_sample_config_files,
    create_worktree_config,
};

/// Helper function to get stdout from command execution
fn get_stdout(env: &CliTestEnvironment, args: &[&str]) -> Result<String> {
    let assert_output = env.run_command(args)?.assert().success();
    let output = assert_output.get_output();
    Ok(String::from_utf8(output.stdout.clone())?)
}

/// Test complete development workflow: create → configure → jump → sync → remove
#[test]
fn test_complete_development_workflow() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Setup: Create project configuration
    create_worktree_config(
        &env.repo_dir,
        &[".env*", ".vscode/", "*.local.json"],
        &["node_modules/", "target/"],
    )?;
    create_sample_config_files(&env.repo_dir)?;

    // Step 1: Create main feature worktree
    env.run_command(&["create", "feature/payment-system"])?
        .assert()
        .success();

    let main_worktree = env.worktree_path("feature/payment-system");
    main_worktree.assert(predicate::path::is_dir());

    // Verify config files were copied during creation
    assert_config_files_copied(&main_worktree)?;

    // Step 2: Jump to the worktree (simulate developer workflow)
    let jump_output = get_stdout(&env, &["jump", "feature/payment-system"])?;
    assert_eq!(jump_output.trim(), main_worktree.to_string_lossy());

    // Step 3: Create additional worktree for sub-feature
    env.run_command(&["create", "feature/payment-integration"])?
        .assert()
        .success();

    let sub_worktree = env.worktree_path("feature/payment-integration");
    sub_worktree.assert(predicate::path::is_dir());

    // Step 4: Modify config in main worktree and sync to sub-feature
    main_worktree
        .child(".env")
        .write_str("API_URL=payment.example.com\nDEBUG=true")?;
    main_worktree
        .child("payment.local.json")
        .write_str(r#"{"stripe_key": "test_key"}"#)?;

    env.run_command(&[
        "sync-config",
        "feature/payment-system",
        "feature/payment-integration",
    ])?
    .assert()
    .success();

    // Verify sync worked
    sub_worktree
        .child(".env")
        .assert(predicate::str::contains("payment.example.com"));
    sub_worktree
        .child("payment.local.json")
        .assert(predicate::str::contains("stripe_key"));

    // Step 5: Test jumping between worktrees
    let jump_to_sub = get_stdout(&env, &["jump", "payment-integration"])?;
    assert_eq!(jump_to_sub.trim(), sub_worktree.to_string_lossy());

    let jump_back = get_stdout(&env, &["jump", "payment-system"])?;
    assert_eq!(jump_back.trim(), main_worktree.to_string_lossy());

    // Step 6: Clean up completed feature
    env.run_command(&["remove", "feature/payment-integration"])?
        .assert()
        .success();

    sub_worktree.assert(predicate::path::missing());
    main_worktree.assert(predicate::path::exists()); // Main feature still exists

    // Step 7: Final cleanup
    env.run_command(&["remove", "feature/payment-system"])?
        .assert()
        .success();

    main_worktree.assert(predicate::path::missing());

    Ok(())
}

/// Test multi-worktree development cycle with completions and navigation
#[test]
fn test_multi_worktree_development_cycle() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Setup multiple worktrees for different features
    let features = [
        "feature/auth",
        "feature/dashboard",
        "bugfix/login-issue",
        "feature/api-v2",
    ];

    // Step 1: Create all worktrees
    for feature in &features {
        env.run_command(&["create", feature])?.assert().success();
    }

    // Verify all were created
    for feature in &features {
        let worktree_path = env.worktree_path(feature);
        worktree_path.assert(predicate::path::is_dir());
    }

    // Step 2: Test completion listing includes all worktrees
    let completions = get_stdout(&env, &["jump", "--list-completions"])?;
    for feature in &features {
        assert!(
            completions.contains(feature),
            "Completions should include {}",
            feature
        );
    }

    // Step 3: Test jumping to each worktree
    for feature in &features {
        let jump_output = get_stdout(&env, &["jump", feature])?;
        let expected_path = env.worktree_path(feature);
        assert_eq!(jump_output.trim(), expected_path.to_string_lossy());
    }

    // Step 4: Test partial matching works correctly
    let auth_output = get_stdout(&env, &["jump", "auth"])?;
    let auth_path = env.worktree_path("feature/auth");
    assert_eq!(auth_output.trim(), auth_path.to_string_lossy());

    // Step 5: Cleanup completed features (keeping others)
    env.run_command(&["remove", "bugfix/login-issue"])?
        .assert()
        .success();

    env.run_command(&["remove", "feature/auth"])?
        .assert()
        .success();

    // Verify selective removal
    env.worktree_path("bugfix/login-issue")
        .assert(predicate::path::missing());
    env.worktree_path("feature/auth")
        .assert(predicate::path::missing());
    env.worktree_path("feature/dashboard")
        .assert(predicate::path::exists());
    env.worktree_path("feature/api-v2")
        .assert(predicate::path::exists());

    // Step 6: Verify updated completions
    let updated_completions = get_stdout(&env, &["jump", "--list-completions"])?;
    assert!(!updated_completions.contains("feature/auth"));
    assert!(!updated_completions.contains("bugfix/login-issue"));
    assert!(updated_completions.contains("feature/dashboard"));
    assert!(updated_completions.contains("feature/api-v2"));

    Ok(())
}

/// Test configuration inheritance and synchronization workflows
#[test]
fn test_config_inheritance_workflow() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Step 1: Setup base configuration
    create_worktree_config(
        &env.repo_dir,
        &[".env*", ".vscode/", "*.local.json", "docker-*"],
        &["*.log", "node_modules/"],
    )?;

    // Create comprehensive config setup
    create_sample_config_files(&env.repo_dir)?;
    env.repo_dir
        .child("docker-compose.dev.yml")
        .write_str("version: '3'")?;

    // Step 2: Create base worktree and verify config inheritance
    env.run_command(&["create", "feature/base-config"])?
        .assert()
        .success();

    let base_path = env.worktree_path("feature/base-config");
    assert_config_files_copied(&base_path)?;
    base_path
        .child("docker-compose.dev.yml")
        .assert(predicate::str::contains("version"));

    // Step 3: Modify config in base worktree
    base_path
        .child(".env")
        .write_str("NODE_ENV=development\nAPI_URL=localhost:3000")?;
    base_path
        .child("app.local.json")
        .write_str(r#"{"feature_flags": {"new_ui": true}}"#)?;

    // Step 4: Create derivative worktree
    env.run_command(&["create", "feature/derived-config"])?
        .assert()
        .success();

    let derived_path = env.worktree_path("feature/derived-config");

    // Step 5: Sync enhanced config to derivative
    env.run_command(&[
        "sync-config",
        "feature/base-config",
        "feature/derived-config",
    ])?
    .assert()
    .success();

    // Verify sync of modified config
    derived_path
        .child(".env")
        .assert(predicate::str::contains("NODE_ENV=development"));
    derived_path
        .child("app.local.json")
        .assert(predicate::str::contains("feature_flags"));

    // Step 6: Test workflow with both worktrees
    let base_jump = get_stdout(&env, &["jump", "base-config"])?;
    assert_eq!(base_jump.trim(), base_path.to_string_lossy());

    let derived_jump = get_stdout(&env, &["jump", "derived-config"])?;
    assert_eq!(derived_jump.trim(), derived_path.to_string_lossy());

    Ok(())
}

/// Test error handling and system stability during failure scenarios
#[test]
fn test_error_recovery_workflow() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Step 1: Create successful worktree
    env.run_command(&["create", "feature/success"])?
        .assert()
        .success();

    let success_path = env.worktree_path("feature/success");
    success_path.assert(predicate::path::is_dir());

    // Step 2: Attempt to create duplicate (should fail)
    env.run_command(&["create", "feature/success"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    // Verify original is still intact
    success_path.assert(predicate::path::is_dir());

    // Step 3: Try invalid operations
    env.run_command(&["jump", "nonexistent"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("No worktree found"));

    env.run_command(&["remove", "nonexistent"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("No worktree found"));

    env.run_command(&["sync-config", "nonexistent", "feature/success"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));

    // Step 4: Verify system remains stable after errors
    let jump_output = get_stdout(&env, &["jump", "feature/success"])?;
    assert_eq!(jump_output.trim(), success_path.to_string_lossy());

    let completions = get_stdout(&env, &["jump", "--list-completions"])?;
    assert!(completions.contains("feature/success"));

    // Step 5: Clean recovery
    env.run_command(&["remove", "feature/success"])?
        .assert()
        .success();

    success_path.assert(predicate::path::missing());

    Ok(())
}

/// Test branch name edge cases and sanitization in multi-command workflows
#[test]
fn test_branch_name_edge_cases_workflow() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Test various challenging branch names
    let challenging_names = [
        "feature/user-auth",
        "feature/payment_gateway",
        "bugfix/critical-issue",
        "feature/api-v2.1",
        "release/v1.0.0",
    ];

    // Step 1: Create worktrees with challenging names
    for name in &challenging_names {
        env.run_command(&["create", name])?.assert().success();
    }

    // Step 2: Verify all can be jumped to using original names
    for name in &challenging_names {
        let jump_output = get_stdout(&env, &["jump", name])?;
        let expected_path = env.worktree_path(name);
        assert_eq!(jump_output.trim(), expected_path.to_string_lossy());
    }

    // Step 3: Test completion includes all names
    let completions = get_stdout(&env, &["jump", "--list-completions"])?;
    for name in &challenging_names {
        assert!(completions.contains(name));
    }

    // Step 4: Test sync between worktrees with special characters
    let source_path = env.worktree_path("feature/user-auth");
    source_path
        .child(".env")
        .write_str("AUTH_SERVICE=enabled")?;

    env.run_command(&["sync-config", "feature/user-auth", "feature/api-v2.1"])?
        .assert()
        .success();

    let target_path = env.worktree_path("feature/api-v2.1");
    target_path
        .child(".env")
        .assert(predicate::str::contains("AUTH_SERVICE"));

    // Step 5: Cleanup all
    for name in &challenging_names {
        env.run_command(&["remove", name])?.assert().success();

        let worktree_path = env.worktree_path(name);
        worktree_path.assert(predicate::path::missing());
    }

    Ok(())
}

/// Test performance and scale with multiple worktrees and bulk operations
#[test]
fn test_scale_workflow() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create a moderate number of worktrees to test scale
    let count = 10;
    let mut worktree_names = Vec::new();

    // Step 1: Bulk creation
    for i in 0..count {
        let name = format!("feature/bulk-{:02}", i);
        worktree_names.push(name.clone());

        env.run_command(&["create", &name])?.assert().success();
    }

    // Step 2: Verify all exist
    for name in &worktree_names {
        let path = env.worktree_path(name);
        path.assert(predicate::path::is_dir());
    }

    // Step 3: Test completion performance (should list all)
    let completions = get_stdout(&env, &["jump", "--list-completions"])?;
    for name in &worktree_names {
        assert!(completions.contains(name));
    }

    // Step 4: Test jumping to random selections
    let test_indices = [0, 3, 7, 9];
    for &idx in &test_indices {
        let name = &worktree_names[idx];
        let jump_output = get_stdout(&env, &["jump", name])?;
        let expected_path = env.worktree_path(name);
        assert_eq!(jump_output.trim(), expected_path.to_string_lossy());
    }

    // Step 5: Bulk cleanup
    for name in &worktree_names {
        env.run_command(&["remove", name])?.assert().success();
    }

    // Step 6: Verify all cleaned up
    for name in &worktree_names {
        let path = env.worktree_path(name);
        path.assert(predicate::path::missing());
    }

    // Final verification - completions should be empty
    let final_completions = get_stdout(&env, &["jump", "--list-completions"])?;
    assert!(final_completions.trim().is_empty());

    Ok(())
}
