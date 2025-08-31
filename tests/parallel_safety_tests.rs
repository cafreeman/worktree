//! Parallel test safety validation
//!
//! These tests validate that our test infrastructure properly isolates tests
//! when run in parallel, ensuring no interference between concurrent test
//! executions through proper temporary directory and storage management.

use anyhow::{Context, Result};
use assert_fs::prelude::*;
use std::sync::{Arc, Barrier};
use std::thread;

mod cli_test_helpers;
use cli_test_helpers::{CliTestEnvironment, patterns};

/// Helper function to get stdout from command execution
fn get_stdout(env: &CliTestEnvironment, args: &[&str]) -> Result<String> {
    let assert_output = env.run_command(args)?.assert().success();
    let output = assert_output.get_output();
    Ok(String::from_utf8(output.stdout.clone())?)
}

/// Test that multiple test environments can be created concurrently
#[test]
fn test_concurrent_environment_creation() -> Result<()> {
    let num_threads = 4;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = Vec::new();

    // Spawn multiple threads that create test environments simultaneously
    for i in 0..num_threads {
        let barrier_clone = Arc::clone(&barrier);

        let handle = thread::spawn(move || -> Result<()> {
            // Wait for all threads to reach this point
            barrier_clone.wait();

            // Create test environment
            let env = CliTestEnvironment::new()?;

            // Perform basic operations to ensure environment is functional
            let branch_name = format!("feature/parallel-test-{}", i);
            env.run_command(&["create", &branch_name])?
                .assert()
                .success();

            // Verify the worktree was created properly
            let worktree_path = env.worktree_path(&branch_name);
            worktree_path.assert(predicates::path::exists());

            // Test basic operations
            env.run_command(&["list"])?.assert().success();
            env.run_command(&["status"])?.assert().success();

            Ok(())
        });

        handles.push(handle);
    }

    // Wait for all threads to complete and check results
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.join() {
            Ok(result) => result.context(format!("Thread {} failed", i))?,
            Err(_) => anyhow::bail!("Thread {} panicked", i),
        }
    }

    Ok(())
}

/// Test that storage directories are properly isolated between tests
#[test]
fn test_storage_isolation() -> Result<()> {
    let num_envs = 3;
    let mut environments = Vec::new();
    let mut storage_paths = Vec::new();

    // Create multiple test environments
    for i in 0..num_envs {
        let env = CliTestEnvironment::new()?;
        let branch_name = format!("feature/isolation-{}", i);

        // Create a worktree in each environment
        env.run_command(&["create", &branch_name])?
            .assert()
            .success();

        // Store the storage path for verification
        storage_paths.push(env.storage_dir.path().to_path_buf());
        environments.push(env);
    }

    // Verify all storage directories are different
    for i in 0..num_envs {
        for j in i + 1..num_envs {
            assert_ne!(
                storage_paths[i], storage_paths[j],
                "Storage directories should be unique between test environments"
            );
        }
    }

    // Verify each environment only sees its own worktrees
    for (i, env) in environments.iter().enumerate() {
        let output_str = get_stdout(env, &["list"])?;

        // Should contain own worktree
        let own_branch = format!("feature/isolation-{}", i);
        assert!(
            output_str.contains(&own_branch),
            "Environment {} should see its own worktree {}",
            i,
            own_branch
        );

        // Should not contain other worktrees
        for j in 0..num_envs {
            if i != j {
                let other_branch = format!("feature/isolation-{}", j);
                assert!(
                    !output_str.contains(&other_branch),
                    "Environment {} should not see worktree {} from environment {}",
                    i,
                    other_branch,
                    j
                );
            }
        }
    }

    Ok(())
}

/// Test that repository directories are properly isolated
#[test]
fn test_repository_isolation() -> Result<()> {
    let env1 = CliTestEnvironment::new()?;
    let env2 = CliTestEnvironment::new()?;

    // Verify different repo directories
    assert_ne!(
        env1.repo_dir.path(),
        env2.repo_dir.path(),
        "Repository directories should be unique"
    );

    // Create different files in each repo
    env1.repo_dir.child("file1.txt").write_str("content1")?;
    env2.repo_dir.child("file2.txt").write_str("content2")?;

    // Verify files don't appear in the other environment
    env1.repo_dir
        .child("file1.txt")
        .assert(predicates::path::exists());
    env1.repo_dir
        .child("file2.txt")
        .assert(predicates::path::missing());

    env2.repo_dir
        .child("file2.txt")
        .assert(predicates::path::exists());
    env2.repo_dir
        .child("file1.txt")
        .assert(predicates::path::missing());

    // Create worktrees with the same name in both environments
    env1.run_command(&["create", "feature/same-name"])?
        .assert()
        .success();
    env2.run_command(&["create", "feature/same-name"])?
        .assert()
        .success();

    // Verify they're in different locations
    let worktree1 = env1.worktree_path("feature/same-name");
    let worktree2 = env2.worktree_path("feature/same-name");

    assert_ne!(
        worktree1.path(),
        worktree2.path(),
        "Worktrees with same name should be in different locations"
    );

    worktree1.assert(predicates::path::exists());
    worktree2.assert(predicates::path::exists());

    Ok(())
}

/// Test that config operations are isolated between environments
#[test]
fn test_config_isolation() -> Result<()> {
    let env1 = CliTestEnvironment::new()?;
    let env2 = CliTestEnvironment::new()?;

    // Create different config setups in each environment
    patterns::create_worktree_config(&env1.repo_dir, &[".env*", "*.config"], &["*.log"])?;

    patterns::create_worktree_config(&env2.repo_dir, &[".vscode/", "*.json"], &["node_modules/"])?;

    // Create worktrees in each environment
    env1.run_command(&["create", "feature/config1"])?
        .assert()
        .success();
    env2.run_command(&["create", "feature/config2"])?
        .assert()
        .success();

    // Create config files specific to each environment
    env1.repo_dir.child(".env").write_str("ENV1=true")?;
    env1.repo_dir.child("app.config").write_str("config1")?;

    env2.repo_dir.child(".vscode").create_dir_all()?;
    env2.repo_dir
        .child(".vscode")
        .child("settings.json")
        .write_str(r#"{"env2": true}"#)?;

    // Test sync within each environment (should work with their own configs)
    let source1 = env1.worktree_path("feature/config1");
    source1.child(".env").write_str("UPDATED_ENV1=true")?;

    let source2 = env2.worktree_path("feature/config2");
    source2.child(".vscode").create_dir_all()?;
    source2
        .child(".vscode")
        .child("settings.json")
        .write_str(r#"{"updated": true}"#)?;

    // Each environment should handle its own config patterns correctly
    // (We can't easily test sync between worktrees in the same environment here,
    // but we can verify the environments don't interfere with each other)

    // Verify environment 1 has its config files
    env1.repo_dir
        .child(".env")
        .assert(predicates::str::contains("ENV1"));
    env1.repo_dir
        .child("app.config")
        .assert(predicates::str::contains("config1"));

    // Verify environment 2 has its config files
    env2.repo_dir
        .child(".vscode")
        .child("settings.json")
        .assert(predicates::str::contains("env2"));

    // Verify environments don't see each other's config
    env1.repo_dir
        .child(".vscode")
        .assert(predicates::path::missing());
    env2.repo_dir
        .child(".env")
        .assert(predicates::path::missing());
    env2.repo_dir
        .child("app.config")
        .assert(predicates::path::missing());

    Ok(())
}

/// Test concurrent read operations (safer than concurrent writes)
#[test]
fn test_concurrent_worktree_operations() -> Result<()> {
    let env = Arc::new(CliTestEnvironment::new()?);

    // Pre-create some worktrees for concurrent reading
    for i in 0..5 {
        let branch = format!("feature/pre-created-{}", i);
        env.run_command(&["create", &branch])?.assert().success();
    }

    let num_threads = 3;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = Vec::new();

    // Test concurrent read operations
    for i in 0..num_threads {
        let env_clone = Arc::clone(&env);
        let barrier_clone = Arc::clone(&barrier);

        let handle = thread::spawn(move || -> Result<()> {
            barrier_clone.wait();

            // Each thread performs different read operations
            match i {
                0 => {
                    // Thread 0: List worktrees multiple times
                    for _j in 0..3 {
                        env_clone.run_command(&["list"])?.assert().success();
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
                1 => {
                    // Thread 1: Check status multiple times
                    for _j in 0..3 {
                        env_clone.run_command(&["status"])?.assert().success();
                        thread::sleep(std::time::Duration::from_millis(15));
                    }
                }
                2 => {
                    // Thread 2: Create one new worktree with unique name
                    let unique_branch = format!("concurrent/thread-{}-unique", std::process::id());
                    env_clone
                        .run_command(&["create", &unique_branch])?
                        .assert()
                        .success();

                    // Then do read operations
                    env_clone.run_command(&["list"])?.assert().success();
                }
                _ => unreachable!(),
            }

            Ok(())
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.join() {
            Ok(result) => result.context(format!("Concurrent operation thread {} failed", i))?,
            Err(_) => anyhow::bail!("Concurrent operation thread {} panicked", i),
        }
    }

    // Verify all pre-created worktrees are still there
    let output_str = get_stdout(&env, &["list"])?;

    for i in 0..5 {
        let branch = format!("feature/pre-created-{}", i);
        assert!(
            output_str.contains(&branch),
            "Should contain worktree {}",
            branch
        );
    }

    // Verify the unique concurrent worktree was created
    let unique_branch = format!("concurrent/thread-{}-unique", std::process::id());
    assert!(
        output_str.contains(&unique_branch),
        "Should contain concurrent worktree"
    );

    Ok(())
}

/// Test that temporary directory cleanup doesn't interfere between tests
#[test]
fn test_cleanup_isolation() -> Result<()> {
    let env1 = CliTestEnvironment::new()?;
    let temp_path1 = env1.repo_dir.path().to_path_buf();

    // Create a worktree
    env1.run_command(&["create", "feature/cleanup-test"])?
        .assert()
        .success();
    let worktree_path1 = env1.worktree_path("feature/cleanup-test");
    worktree_path1.assert(predicates::path::exists());

    // Drop env1 (simulating test cleanup)
    drop(env1);

    // Create a new environment
    let env2 = CliTestEnvironment::new()?;
    let temp_path2 = env2.repo_dir.path().to_path_buf();

    // Verify paths are different
    assert_ne!(
        temp_path1, temp_path2,
        "Temporary paths should be different"
    );

    // Verify new environment works independently
    env2.run_command(&["create", "feature/new-test"])?
        .assert()
        .success();
    let worktree_path2 = env2.worktree_path("feature/new-test");
    worktree_path2.assert(predicates::path::exists());

    // Verify we can't see the old environment's worktrees
    let output_str = get_stdout(&env2, &["list"])?;

    assert!(
        output_str.contains("feature/new-test"),
        "Should see new worktree"
    );
    assert!(
        !output_str.contains("feature/cleanup-test"),
        "Should not see old worktree"
    );

    Ok(())
}

/// Test environment variables isolation
#[test]
fn test_environment_variable_isolation() -> Result<()> {
    let env1 = CliTestEnvironment::new()?;
    let env2 = CliTestEnvironment::new()?;

    // Get the storage paths that should be different
    let storage1 = env1.storage_dir.path();
    let storage2 = env2.storage_dir.path();

    assert_ne!(storage1, storage2, "Storage paths should be different");

    // Each environment should use its own storage path
    // This is ensured by our CliTestEnvironment setting WORKTREE_STORAGE_ROOT

    // Create worktrees in each environment
    env1.run_command(&["create", "feature/env1"])?
        .assert()
        .success();
    env2.run_command(&["create", "feature/env2"])?
        .assert()
        .success();

    // Verify each environment only sees its own worktrees
    let output1 = get_stdout(&env1, &["list"])?;
    let output2 = get_stdout(&env2, &["list"])?;

    assert!(
        output1.contains("feature/env1"),
        "Env1 should see its worktree"
    );
    assert!(
        !output1.contains("feature/env2"),
        "Env1 should not see env2's worktree"
    );

    assert!(
        output2.contains("feature/env2"),
        "Env2 should see its worktree"
    );
    assert!(
        !output2.contains("feature/env1"),
        "Env2 should not see env1's worktree"
    );

    Ok(())
}

/// Test that git operations are properly isolated
#[test]
fn test_git_isolation() -> Result<()> {
    let env1 = CliTestEnvironment::new()?;
    let env2 = CliTestEnvironment::new()?;

    // Create different commits in each repository
    env1.repo_dir
        .child("file1.txt")
        .write_str("Environment 1 content")?;
    env2.repo_dir
        .child("file2.txt")
        .write_str("Environment 2 content")?;

    // Create worktrees that might have the same branch names
    env1.run_command(&["create", "feature/git-test"])?
        .assert()
        .success();
    env2.run_command(&["create", "feature/git-test"])?
        .assert()
        .success();

    // Verify each worktree is in its own git repository context
    let worktree1 = env1.worktree_path("feature/git-test");
    let worktree2 = env2.worktree_path("feature/git-test");

    // Both should exist but in different locations
    worktree1.assert(predicates::path::exists());
    worktree2.assert(predicates::path::exists());
    assert_ne!(worktree1.path(), worktree2.path());

    // The worktrees should be isolated from each other's repo files
    // Note: Files created in repo root don't automatically appear in worktrees
    // This test verifies the worktrees are in different repository contexts
    // We can verify this by checking the worktree paths are different
    assert_ne!(
        worktree1.path().parent(),
        worktree2.path().parent(),
        "Worktrees should be in different storage hierarchies"
    );

    Ok(())
}

/// Test that test helpers maintain isolation under stress
#[test]
fn test_stress_isolation() -> Result<()> {
    let num_iterations = 10;
    let mut environments = Vec::new();

    // Create many environments rapidly
    for i in 0..num_iterations {
        let env = CliTestEnvironment::new()?;

        // Perform operations in each
        let branch = format!("stress/test-{}", i);
        env.run_command(&["create", &branch])?.assert().success();

        // Store for later verification
        environments.push(env);
    }

    // Verify all environments remain isolated
    for (i, env) in environments.iter().enumerate() {
        let output_str = get_stdout(env, &["list"])?;

        // Should see own worktree
        let own_branch = format!("stress/test-{}", i);
        assert!(
            output_str.contains(&own_branch),
            "Env {} should see {}",
            i,
            own_branch
        );

        // Should not see others (spot check a few)
        if i > 0 {
            let other_branch = format!("stress/test-{}", i - 1);
            assert!(
                !output_str.contains(&other_branch),
                "Env {} should not see {}",
                i,
                other_branch
            );
        }
        if i < num_iterations - 1 {
            let other_branch = format!("stress/test-{}", i + 1);
            assert!(
                !output_str.contains(&other_branch),
                "Env {} should not see {}",
                i,
                other_branch
            );
        }
    }

    Ok(())
}
