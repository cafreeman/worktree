#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

use anyhow::Result;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Create a basic worktree configuration file for testing
pub fn create_worktree_config(
    repo_dir: &assert_fs::fixture::ChildPath,
    include_patterns: &[&str],
    exclude_patterns: &[&str],
) -> Result<()> {
    let config_content = format!(
        r#"[copy-patterns]
include = {:?}
exclude = {:?}
"#,
        include_patterns, exclude_patterns
    );

    repo_dir
        .child(".worktree-config.toml")
        .write_str(&config_content)?;

    Ok(())
}

/// Create sample files that match typical config patterns
pub fn create_sample_config_files(repo_dir: &assert_fs::fixture::ChildPath) -> Result<()> {
    // Create .env file
    repo_dir.child(".env").write_str("TEST_VAR=test_value")?;

    // Create .vscode directory with settings
    let vscode_dir = repo_dir.child(".vscode");
    vscode_dir.create_dir_all()?;
    vscode_dir
        .child("settings.json")
        .write_str(r#"{"editor.fontSize": 14}"#)?;

    // Create local config file
    repo_dir
        .child("config.local.json")
        .write_str(r#"{"debug": true}"#)?;

    Ok(())
}

/// Assert that config files were copied to a worktree
pub fn assert_config_files_copied(worktree_path: &assert_fs::fixture::ChildPath) -> Result<()> {
    // Check .env file
    worktree_path
        .child(".env")
        .assert(predicate::path::exists())
        .assert(predicate::str::contains("TEST_VAR=test_value"));

    // Check .vscode settings
    worktree_path
        .child(".vscode")
        .child("settings.json")
        .assert(predicate::path::exists())
        .assert(predicate::str::contains("editor.fontSize"));

    // Check local config
    worktree_path
        .child("config.local.json")
        .assert(predicate::path::exists())
        .assert(predicate::str::contains("debug"));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_file_helpers() -> Result<()> {
        let temp_dir = assert_fs::TempDir::new()?;
        let repo_dir = temp_dir.child("test_repo");
        repo_dir.create_dir_all()?;

        // Test config creation
        create_worktree_config(
            &repo_dir,
            &[".env*", ".vscode/"],
            &["node_modules/", "target/"],
        )?;

        repo_dir
            .child(".worktree-config.toml")
            .assert(predicate::str::contains("copy-patterns"));

        // Test sample file creation
        create_sample_config_files(&repo_dir)?;

        repo_dir
            .child(".env")
            .assert(predicate::str::contains("TEST_VAR"));

        Ok(())
    }
}
