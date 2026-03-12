#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

use anyhow::{Context, Result};
use assert_fs::prelude::*;
use assert_fs::TempDir;

use std::process::Command;

/// Modern test environment using assert_fs for simplified setup and cleanup
pub struct CliTestEnvironment {
    pub repo_dir: assert_fs::fixture::ChildPath,
    pub storage_dir: assert_fs::fixture::ChildPath,
    _temp_dir: TempDir,
}

impl CliTestEnvironment {
    /// Creates a new test environment with a real git repository and storage directory
    ///
    /// # Errors
    /// Returns an error if the temporary directory or git repository cannot be created.
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;
        let repo_dir = temp_dir.child("test_repo");
        let storage_dir = temp_dir.child("worktrees");

        repo_dir.create_dir_all()?;
        storage_dir.create_dir_all()?;

        Self::run_git_command(&repo_dir, &["init"])?;
        Self::run_git_command(&repo_dir, &["config", "user.name", "Test User"])?;
        Self::run_git_command(&repo_dir, &["config", "user.email", "test@example.com"])?;

        repo_dir.child("README.md").write_str("# Test Repo")?;
        Self::run_git_command(&repo_dir, &["add", "."])?;
        Self::run_git_command(&repo_dir, &["commit", "-m", "Initial commit"])?;
        Self::run_git_command(&repo_dir, &["branch", "-M", "main"])?;

        Ok(Self {
            repo_dir,
            storage_dir,
            _temp_dir: temp_dir,
        })
    }

    fn run_git_command(repo_path: &assert_fs::fixture::ChildPath, args: &[&str]) -> Result<()> {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo_path.path())
            .output()
            .context("Failed to execute git command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git command failed: {}", stderr);
        }

        Ok(())
    }

    /// Execute a non-interactive CLI command with proper environment setup
    ///
    /// # Errors
    /// Returns an error if the command setup fails.
    pub fn run_command(&self, args: &[&str]) -> Result<assert_cmd::Command> {
        let mut cmd = assert_cmd::Command::cargo_bin("worktree-bin")
            .context("Failed to find worktree-bin binary")?;

        cmd.current_dir(self.repo_dir.path())
            .env("WORKTREE_STORAGE_ROOT", self.storage_dir.path());

        cmd.args(args);
        Ok(cmd)
    }

    /// Get the path to a worktree within the storage directory.
    /// Under the feature-named model, the path is the feature name directly (no sanitization).
    pub fn worktree_path(&self, feature_name: &str) -> assert_fs::fixture::ChildPath {
        self.storage_dir.child("test_repo").child(feature_name)
    }

    /// Check if we're running in a CI environment or without a TTY
    pub fn is_ci() -> bool {
        use std::io::IsTerminal;
        std::env::var("CI").is_ok()
            || std::env::var("GITHUB_ACTIONS").is_ok()
            || std::env::var("GITLAB_CI").is_ok()
            || std::env::var("TRAVIS").is_ok()
            || std::env::var("CIRCLECI").is_ok()
            || !std::io::stdin().is_terminal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use predicates::prelude::*;

    #[test]
    fn test_cli_test_environment_creation() -> Result<()> {
        let env = CliTestEnvironment::new()?;

        env.repo_dir.assert(predicate::path::is_dir());
        env.repo_dir.child(".git").assert(predicate::path::exists());
        env.repo_dir
            .child("README.md")
            .assert(predicate::str::contains("# Test Repo"));

        env.storage_dir.assert(predicate::path::is_dir());

        Ok(())
    }

    #[test]
    fn test_worktree_path_uses_feature_name_directly() -> Result<()> {
        let env = CliTestEnvironment::new()?;

        let path = env.worktree_path("auth");
        assert!(path.path().to_string_lossy().ends_with("test_repo/auth"));

        Ok(())
    }
}
