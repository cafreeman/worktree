#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

//! Configuration system tests
//!
//! These tests validate configuration file parsing, merging behavior,
//! error handling, and integration with worktree creation and sync commands.

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

use test_support::CliTestEnvironment;
use worktree::config::WorktreeConfig;

// ==================== CONFIGURATION LOADING TESTS ====================

#[test]
fn test_no_config_file_uses_defaults() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Load config from repo with no config file
    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    // Should have default include patterns
    let includes = config.copy_patterns.include.as_ref().unwrap();
    assert!(includes.contains(&".env*".to_string()));
    assert!(includes.contains(&".vscode/".to_string()));
    assert!(includes.contains(&"*.local.json".to_string()));
    assert!(includes.contains(&"config/local/*".to_string()));

    // Should have default exclude patterns
    let excludes = config.copy_patterns.exclude.as_ref().unwrap();
    assert!(excludes.contains(&"node_modules/".to_string()));
    assert!(excludes.contains(&"target/".to_string()));
    assert!(excludes.contains(&".git/".to_string()));
    assert!(excludes.contains(&"*.log".to_string()));
    assert!(excludes.contains(&"*.tmp".to_string()));

    Ok(())
}

#[test]
fn test_complete_config_works_legacy() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create traditional config with both include and exclude
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[copy-patterns]
include = ["custom.conf", "*.env"]
exclude = ["*.secret", "temp/"]
"#,
    )?;

    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    // Should have user patterns + defaults (precedence-based merging)
    let includes = config.copy_patterns.include.as_ref().unwrap();
    // Should have user includes + defaults
    assert!(includes.contains(&"custom.conf".to_string()));
    assert!(includes.contains(&"*.env".to_string()));
    // Plus default includes
    assert!(includes.contains(&".env*".to_string()));
    assert!(includes.contains(&".vscode/".to_string()));
    assert!(includes.contains(&"*.local.json".to_string()));
    assert!(includes.contains(&"config/local/*".to_string()));

    let excludes = config.copy_patterns.exclude.as_ref().unwrap();
    // Should have user excludes + defaults
    assert!(excludes.contains(&"*.secret".to_string()));
    assert!(excludes.contains(&"temp/".to_string()));
    // Plus default excludes
    assert!(excludes.contains(&"node_modules/".to_string()));
    assert!(excludes.contains(&"target/".to_string()));

    Ok(())
}

// ==================== ENHANCED CONFIGURATION BEHAVIOR TESTS ====================

#[test]
fn test_blank_config_file_uses_defaults() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create completely empty config file
    env.repo_dir.child(".worktree-config.toml").write_str("")?;

    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    // Should use default patterns when file is empty
    let includes = config.copy_patterns.include.as_ref().unwrap();
    assert!(includes.contains(&".env*".to_string()));
    assert!(includes.contains(&".vscode/".to_string()));
    assert!(includes.contains(&"*.local.json".to_string()));
    assert!(includes.contains(&"config/local/*".to_string()));

    let excludes = config.copy_patterns.exclude.as_ref().unwrap();
    assert!(excludes.contains(&"node_modules/".to_string()));
    assert!(excludes.contains(&"target/".to_string()));
    assert!(excludes.contains(&".git/".to_string()));
    assert!(excludes.contains(&"*.log".to_string()));
    assert!(excludes.contains(&"*.tmp".to_string()));

    Ok(())
}

#[test]
fn test_config_with_only_include_merges_defaults() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Config with only include field should merge with defaults
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[copy-patterns]
include = ["mise.toml", "docker-compose.yml"]
"#,
    )?;

    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    // Should merge: user includes + default includes (additive merging)
    let includes = config.copy_patterns.include.as_ref().unwrap();
    assert!(includes.contains(&"mise.toml".to_string()));
    assert!(includes.contains(&"docker-compose.yml".to_string()));
    // Should also have defaults
    assert!(includes.contains(&".env*".to_string()));
    assert!(includes.contains(&".vscode/".to_string()));
    assert!(includes.contains(&"*.local.json".to_string()));
    assert!(includes.contains(&"config/local/*".to_string()));

    let excludes = config.copy_patterns.exclude.as_ref().unwrap();
    // Should have all default excludes
    assert!(excludes.contains(&"node_modules/".to_string()));
    assert!(excludes.contains(&"target/".to_string()));
    assert!(excludes.contains(&".git/".to_string()));
    assert!(excludes.contains(&"*.log".to_string()));
    assert!(excludes.contains(&"*.tmp".to_string()));

    Ok(())
}

#[test]
fn test_config_with_only_exclude_merges_defaults() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Config with only exclude field should merge with defaults
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[copy-patterns]
exclude = ["*.secret", "private/"]
"#,
    )?;

    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    // Should merge: default includes + user excludes + default excludes
    let includes = config.copy_patterns.include.as_ref().unwrap();
    // Should have all default includes
    assert!(includes.contains(&".env*".to_string()));
    assert!(includes.contains(&".vscode/".to_string()));
    assert!(includes.contains(&"*.local.json".to_string()));
    assert!(includes.contains(&"config/local/*".to_string()));

    let excludes = config.copy_patterns.exclude.as_ref().unwrap();
    // Should have user excludes
    assert!(excludes.contains(&"*.secret".to_string()));
    assert!(excludes.contains(&"private/".to_string()));
    // Plus default excludes
    assert!(excludes.contains(&"node_modules/".to_string()));
    assert!(excludes.contains(&"target/".to_string()));
    assert!(excludes.contains(&".git/".to_string()));
    assert!(excludes.contains(&"*.log".to_string()));
    assert!(excludes.contains(&"*.tmp".to_string()));

    Ok(())
}

#[test]
fn test_user_exclude_overrides_default_include() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Config that excludes something normally included by default
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[copy-patterns]
exclude = [".vscode/"]
"#,
    )?;

    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    let includes = config.copy_patterns.include.as_ref().unwrap();
    // Should have all default includes
    assert!(includes.contains(&".env*".to_string()));
    assert!(includes.contains(&".vscode/".to_string())); // Still in includes
    assert!(includes.contains(&"*.local.json".to_string()));
    assert!(includes.contains(&"config/local/*".to_string()));

    let excludes = config.copy_patterns.exclude.as_ref().unwrap();
    // Should have user exclude + default excludes
    assert!(excludes.contains(&".vscode/".to_string())); // User exclude wins
    assert!(excludes.contains(&"node_modules/".to_string()));
    assert!(excludes.contains(&"target/".to_string()));

    Ok(())
}

#[test]
fn test_unknown_keys_ignored() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Config with unknown sections/keys should be ignored
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[copy-patterns]
include = ["mise.toml"]

[unknown-section]
some_key = "value"

[copy-patterns.unknown-key]
value = 123
"#,
    )?;

    // Should parse successfully despite unknown keys
    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    let includes = config.copy_patterns.include.as_ref().unwrap();
    assert!(includes.contains(&"mise.toml".to_string()));
    // Should merge with defaults
    assert!(includes.contains(&".env*".to_string()));
    assert!(includes.contains(&".vscode/".to_string()));

    Ok(())
}

#[test]
fn test_invalid_toml_fallback_to_defaults() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Invalid TOML should fallback to defaults gracefully
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[copy-patterns
include = ["missing bracket"
invalid syntax here
"#,
    )?;

    // Should not fail, but use defaults with warning
    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    // Should fall back to defaults
    let includes = config.copy_patterns.include.as_ref().unwrap();
    assert!(includes.contains(&".env*".to_string()));
    assert!(includes.contains(&".vscode/".to_string()));
    assert!(includes.contains(&"*.local.json".to_string()));
    assert!(includes.contains(&"config/local/*".to_string()));

    let excludes = config.copy_patterns.exclude.as_ref().unwrap();
    assert!(excludes.contains(&"node_modules/".to_string()));
    assert!(excludes.contains(&"target/".to_string()));

    Ok(())
}

#[test]
fn test_partial_toml_missing_copy_patterns_section() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // TOML without [copy-patterns] section should use defaults
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[other-section]
key = "value"
"#,
    )?;

    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    // Should use defaults when section is missing
    let includes = config.copy_patterns.include.as_ref().unwrap();
    assert!(includes.contains(&".env*".to_string()));
    assert!(includes.contains(&".vscode/".to_string()));
    assert!(includes.contains(&"*.local.json".to_string()));
    assert!(includes.contains(&"config/local/*".to_string()));

    let excludes = config.copy_patterns.exclude.as_ref().unwrap();
    assert!(excludes.contains(&"node_modules/".to_string()));
    assert!(excludes.contains(&"target/".to_string()));

    Ok(())
}

// ==================== CONFIGURATION INTEGRATION TESTS ====================

#[test]
fn test_create_worktree_with_partial_config_integration() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create config with only include (merging test)
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[copy-patterns]
include = ["mise.toml"]
"#,
    )?;

    // Create files to copy
    env.repo_dir.child("mise.toml").write_str("tool = 'mise'")?;
    env.repo_dir.child(".env").write_str("DEFAULT=value")?;
    env.repo_dir.child("node_modules").create_dir_all()?;
    env.repo_dir
        .child("node_modules")
        .child("package")
        .write_str("should be excluded")?;

    // Create worktree
    env.run_command(&["create", "test-branch"])?
        .assert()
        .success();

    let worktree_path = env.worktree_path("test-branch");

    // Should copy both user patterns and defaults
    worktree_path
        .child("mise.toml")
        .assert(predicate::path::exists()); // User include
    worktree_path
        .child(".env")
        .assert(predicate::path::exists()); // Default include

    // Should exclude default excludes
    worktree_path
        .child("node_modules")
        .assert(predicate::path::missing());

    Ok(())
}

#[test]
fn test_sync_config_with_precedence_integration() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Config that excludes .vscode/ (precedence test)
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[copy-patterns]
include = ["mise.toml"]
exclude = [".vscode/"]
"#,
    )?;

    // Create worktrees
    env.run_command(&["create", "source"])?.assert().success();
    env.run_command(&["create", "target"])?.assert().success();

    let source_path = env.worktree_path("source");
    let target_path = env.worktree_path("target");

    // Create files in source
    source_path.child("mise.toml").write_str("config")?;
    source_path.child(".env").write_str("env")?;
    source_path.child(".vscode").create_dir_all()?;
    source_path
        .child(".vscode")
        .child("settings.json")
        .write_str("{}")?;

    // Sync config
    env.run_command(&["sync-config", "source", "target"])?
        .assert()
        .success();

    // Should copy includes (user + defaults)
    target_path
        .child("mise.toml")
        .assert(predicate::path::exists());
    target_path.child(".env").assert(predicate::path::exists());

    // Should NOT copy excluded patterns (user exclude wins)
    target_path
        .child(".vscode")
        .assert(predicate::path::missing());

    Ok(())
}

// ==================== EDGE CASE TESTS ====================

#[test]
fn test_empty_include_exclude_arrays() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Config with empty arrays should still get defaults
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[copy-patterns]
include = []
exclude = []
"#,
    )?;

    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    // Should have defaults (precedence-based merging always adds defaults)
    let includes = config.copy_patterns.include.as_ref().unwrap();
    assert!(includes.contains(&".env*".to_string()));
    assert!(includes.contains(&".vscode/".to_string()));

    let excludes = config.copy_patterns.exclude.as_ref().unwrap();
    assert!(excludes.contains(&"node_modules/".to_string()));
    assert!(excludes.contains(&"target/".to_string()));

    Ok(())
}

#[test]
fn test_duplicate_patterns_in_user_config() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Config with duplicate patterns should be deduplicated
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[copy-patterns]
include = ["mise.toml", "mise.toml", "docker-compose.yml"]
"#,
    )?;

    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    let includes = config.copy_patterns.include.as_ref().unwrap();
    assert!(includes.contains(&"mise.toml".to_string()));
    assert!(includes.contains(&"docker-compose.yml".to_string()));

    // Should only have one instance of mise.toml
    let mise_count = includes.iter().filter(|&p| p == "mise.toml").count();
    assert_eq!(mise_count, 1);

    Ok(())
}

#[test]
fn test_user_include_overrides_default_exclude() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Config that includes something normally excluded by default
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[copy-patterns]
include = ["node_modules/.cache"]
"#,
    )?;

    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    let includes = config.copy_patterns.include.as_ref().unwrap();
    // Should have user include + defaults
    assert!(includes.contains(&"node_modules/.cache".to_string()));
    assert!(includes.contains(&".env*".to_string()));
    assert!(includes.contains(&".vscode/".to_string()));

    let excludes = config.copy_patterns.exclude.as_ref().unwrap();
    // Should have default excludes (including node_modules/)
    assert!(excludes.contains(&"node_modules/".to_string()));
    assert!(excludes.contains(&"target/".to_string()));

    Ok(())
}

#[test]
fn test_precedence_based_merging() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Test precedence-based merging with both include and exclude
    env.repo_dir.child(".worktree-config.toml").write_str(
        r#"
[copy-patterns]
include = ["custom.conf"]
exclude = ["*.secret"]
"#,
    )?;

    let config = WorktreeConfig::load_from_repo(&env.repo_dir.to_path_buf())?;

    // Should have user patterns + defaults (precedence-based merging)
    let includes = config.copy_patterns.include.as_ref().unwrap();
    assert!(includes.contains(&"custom.conf".to_string()));
    // Should also have default includes merged in
    assert!(includes.contains(&".env*".to_string()));
    assert!(includes.contains(&".vscode/".to_string()));

    let excludes = config.copy_patterns.exclude.as_ref().unwrap();
    assert!(excludes.contains(&"*.secret".to_string()));
    // Should also have default excludes merged in
    assert!(excludes.contains(&"node_modules/".to_string()));
    assert!(excludes.contains(&"target/".to_string()));

    Ok(())
}
