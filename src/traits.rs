use anyhow::Result;
use std::path::{Path, PathBuf};

/// Trait for Git operations to enable mocking in tests
pub trait GitOperations {
    /// Opens a git repository at the specified path
    ///
    /// # Errors
    /// Returns an error if the path is not a valid git repository
    fn open(path: &Path) -> Result<Box<dyn GitOperations>>
    where
        Self: Sized;
    fn get_repo_path(&self) -> PathBuf;
    /// Checks if a branch exists in the repository
    ///
    /// # Errors
    /// Returns an error if git operations fail
    fn branch_exists(&self, branch_name: &str) -> Result<bool>;
    /// Creates a new worktree for the specified branch
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to create the worktree
    /// - Branch doesn't exist and create_branch is false
    /// - Git operations fail
    fn create_worktree(
        &self,
        branch_name: &str,
        worktree_path: &Path,
        create_branch: bool,
    ) -> Result<()>;
    /// Removes a worktree from the repository
    ///
    /// # Errors
    /// Returns an error if git operations fail
    fn remove_worktree(&self, worktree_name: &str) -> Result<()>;
    /// Lists all worktrees in the repository
    ///
    /// # Errors
    /// Returns an error if git operations fail
    fn list_worktrees(&self) -> Result<Vec<String>>;
    /// Deletes a branch from the repository
    ///
    /// # Errors
    /// Returns an error if:
    /// - Branch doesn't exist
    /// - Git operations fail
    fn delete_branch(&self, branch_name: &str) -> Result<()>;

    /// Enables worktree-specific configuration and copies parent repo's effective config
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to enable worktree configuration
    /// - Failed to read parent repository configuration
    /// - Failed to set worktree-specific configuration
    fn inherit_config(&self, worktree_path: &Path) -> Result<()>;
}
