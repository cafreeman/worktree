use anyhow::Result;
use std::path::PathBuf;
use temp_env::with_var;
use tempfile::TempDir;

/// A test environment that sets up real git repositories and ensures cleanup
pub struct TestEnvironment {
    #[allow(dead_code)] // Needed to keep temp directory alive during tests
    pub temp_dir: TempDir,
    pub repo_path: PathBuf,
    pub storage_root: PathBuf,
}

impl TestEnvironment {
    /// Creates a new test environment with a temporary git repository
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to create temporary directory
    /// - Failed to initialize git repository
    /// - Failed to configure git user settings
    /// - Failed to create initial commit
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let repo_path = temp_dir.path().join("test_repo");
        let storage_root = temp_dir.path().join("worktrees");

        // Create directories
        std::fs::create_dir_all(&repo_path)?;
        std::fs::create_dir_all(&storage_root)?;

        // Initialize real git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()?;

        // Configure git user
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()?;
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()?;

        // Create initial commit
        std::fs::write(repo_path.join("README.md"), "# Test Repo")?;
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .output()?;
        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()?;

        Ok(Self {
            temp_dir,
            repo_path,
            storage_root,
        })
    }

    /// Run a test function with proper environment setup and cleanup
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to get current directory
    /// - Failed to change to test directory
    /// - Test function itself returns an error
    pub fn run_test<F>(&self, test_fn: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        // Save original directory
        let original_dir = std::env::current_dir()?;

        // Set up environment and run test
        let result = with_var(
            "WORKTREE_STORAGE_ROOT",
            Some(self.storage_root.to_string_lossy().to_string()),
            || {
                // Change to repo directory
                // This is test code, so unwrap is acceptable
                #[allow(clippy::expect_used)]
                {
                    std::env::set_current_dir(&self.repo_path)
                        .expect("Failed to change to test repo directory");
                }

                // Run the test
                test_fn()
            },
        );

        // Always restore directory, even if test fails
        let _ = std::env::set_current_dir(&original_dir);

        result
    }
}
