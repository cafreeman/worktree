#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

use anyhow::{Context, Result};
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;
use rexpect::session::{PtySession, spawn_command};

use std::process::Command;

/// Modern test environment using assert_fs for simplified setup and cleanup
pub struct CliTestEnvironment {
    pub temp_dir: TempDir,
    pub repo_dir: assert_fs::fixture::ChildPath,
    pub storage_dir: assert_fs::fixture::ChildPath,
}

impl CliTestEnvironment {
    /// Creates a new test environment with a real git repository and storage directory
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to create temporary directory
    /// - Failed to initialize git repository
    /// - Failed to configure git settings
    /// - Failed to create initial commit
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;
        let repo_dir = temp_dir.child("test_repo");
        let storage_dir = temp_dir.child("worktrees");

        // Create directories
        repo_dir.create_dir_all()?;
        storage_dir.create_dir_all()?;

        // Initialize real git repo
        Self::run_git_command(&repo_dir, &["init"])?;
        Self::run_git_command(&repo_dir, &["config", "user.name", "Test User"])?;
        Self::run_git_command(&repo_dir, &["config", "user.email", "test@example.com"])?;

        // Create initial commit
        repo_dir.child("README.md").write_str("# Test Repo")?;
        Self::run_git_command(&repo_dir, &["add", "."])?;
        Self::run_git_command(&repo_dir, &["commit", "-m", "Initial commit"])?;

        Ok(Self {
            temp_dir,
            repo_dir,
            storage_dir,
        })
    }

    /// Run a git command in the repository directory
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
    /// Returns an error if the command setup fails
    pub fn run_command(&self, args: &[&str]) -> Result<assert_cmd::Command> {
        let mut cmd = assert_cmd::Command::cargo_bin("worktree-bin")
            .context("Failed to find worktree-bin binary")?;

        cmd.current_dir(self.repo_dir.path())
            .env("WORKTREE_STORAGE_ROOT", self.storage_dir.path());

        cmd.args(args);
        Ok(cmd)
    }

    /// Start an interactive CLI session for testing with rexpect
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to find the binary
    /// - Failed to spawn the process
    /// - Session setup fails
    pub fn start_interactive(&self, args: &[&str]) -> Result<InteractiveTest> {
        let mut cmd = Command::new("cargo");
        cmd.args(&["run", "--bin", "worktree-bin", "--"]);
        cmd.args(args);
        cmd.current_dir(self.repo_dir.path());
        cmd.env("WORKTREE_STORAGE_ROOT", self.storage_dir.path());

        let session =
            spawn_command(cmd, Some(5000)).context("Failed to spawn interactive command")?;

        Ok(InteractiveTest::new(session))
    }

    /// Get the path to a worktree within the storage directory
    pub fn worktree_path(&self, branch_name: &str) -> assert_fs::fixture::ChildPath {
        // Use the same sanitization logic as the main application
        let sanitized = branch_name.replace('/', "-");
        self.storage_dir.child("test_repo").child(&sanitized)
    }

    /// Run a test function with proper environment setup and cleanup
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to get current directory
    /// - Failed to change to test directory
    /// - Test function returns an error
    pub fn run_test<F>(&self, test_fn: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        // Save original directory
        let original_dir = std::env::current_dir().context("Failed to get current directory")?;

        // Set up environment and run test
        std::env::set_current_dir(self.repo_dir.path())
            .context("Failed to change to test repo directory")?;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            temp_env::with_var(
                "WORKTREE_STORAGE_ROOT",
                Some(self.storage_dir.path().to_string_lossy().to_string()),
                test_fn,
            )
        }));

        // Always restore directory, even if test panics
        let _ = std::env::set_current_dir(&original_dir);

        match result {
            Ok(test_result) => test_result,
            Err(_) => anyhow::bail!("Test panicked"),
        }
    }
}

/// Wrapper for interactive CLI testing using rexpect
pub struct InteractiveTest {
    session: PtySession,
}

impl InteractiveTest {
    /// Create a new interactive test session
    pub fn new(session: PtySession) -> Self {
        Self { session }
    }

    /// Expect a specific prompt and send a response
    ///
    /// # Errors
    /// Returns an error if:
    /// - The expected prompt is not found within timeout
    /// - Failed to send the response
    pub fn expect_and_respond(&mut self, prompt: &str, response: &str) -> Result<&mut Self> {
        self.session
            .exp_string(prompt)
            .with_context(|| format!("Expected prompt '{}' not found", prompt))?;

        self.session
            .send_line(response)
            .with_context(|| format!("Failed to send response '{}'", response))?;

        Ok(self)
    }

    /// Expect final output without sending a response
    ///
    /// # Errors
    /// Returns an error if the expected output is not found within timeout
    pub fn expect_final(&mut self, output: &str) -> Result<()> {
        self.session
            .exp_string(output)
            .with_context(|| format!("Expected final output '{}' not found", output))?;

        Ok(())
    }
}

/// Helper functions for common test patterns
pub mod patterns {
    use super::*;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_test_environment_creation() -> Result<()> {
        let env = CliTestEnvironment::new()?;

        // Test that git repo was created properly
        env.repo_dir.assert(predicate::path::is_dir());
        env.repo_dir.child(".git").assert(predicate::path::exists());
        env.repo_dir
            .child("README.md")
            .assert(predicate::str::contains("# Test Repo"));

        // Test that storage directory exists
        env.storage_dir.assert(predicate::path::is_dir());

        Ok(())
    }

    #[test]
    fn test_worktree_path_sanitization() -> Result<()> {
        let env = CliTestEnvironment::new()?;

        // Test branch name sanitization
        let path = env.worktree_path("feature/test-branch");
        assert!(
            path.path()
                .to_string_lossy()
                .contains("feature-test-branch")
        );

        Ok(())
    }

    #[test]
    fn test_config_file_helpers() -> Result<()> {
        let env = CliTestEnvironment::new()?;

        // Test config creation
        patterns::create_worktree_config(
            &env.repo_dir,
            &[".env*", ".vscode/"],
            &["node_modules/", "target/"],
        )?;

        env.repo_dir
            .child(".worktree-config.toml")
            .assert(predicate::str::contains("copy-patterns"));

        // Test sample file creation
        patterns::create_sample_config_files(&env.repo_dir)?;

        env.repo_dir
            .child(".env")
            .assert(predicate::str::contains("TEST_VAR"));

        Ok(())
    }
}
