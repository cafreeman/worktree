use anyhow::Result;
use std::path::{Path, PathBuf};

/// Trait for Git operations to enable mocking in tests
pub trait GitOperations {
    /// Returns the working directory of the main repository.
    ///
    /// When discovered from inside a linked worktree, this is the original
    /// repository's directory, not the worktree's own directory.
    ///
    /// # Errors
    /// Returns an error for bare repositories (they have no working directory).
    fn get_repo_path(&self) -> Result<PathBuf>;
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
    /// Creates a new worktree for the specified branch from a specific starting point
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to create the worktree
    /// - Branch doesn't exist and create_branch is false
    /// - Failed to resolve the starting reference
    /// - Git operations fail
    fn create_worktree_from(
        &self,
        branch_name: &str,
        worktree_path: &Path,
        create_branch: bool,
        from_ref: Option<&str>,
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

    /// Lists all local branches in the repository
    ///
    /// # Errors
    /// Returns an error if git operations fail
    fn list_local_branches(&self) -> Result<Vec<String>>;
    /// Lists all remote branches in the repository
    ///
    /// # Errors
    /// Returns an error if git operations fail
    fn list_remote_branches(&self) -> Result<Vec<String>>;
    /// Lists all tags in the repository
    ///
    /// # Errors
    /// Returns an error if git operations fail
    fn list_tags(&self) -> Result<Vec<String>>;
}
