use anyhow::Result;
use std::path::{Path, PathBuf};

/// Trait for Git operations to enable mocking in tests
pub trait GitOperations {
    fn open(path: &Path) -> Result<Box<dyn GitOperations>>
    where
        Self: Sized;
    fn get_repo_path(&self) -> PathBuf;
    fn branch_exists(&self, branch_name: &str) -> Result<bool>;
    fn create_worktree(
        &self,
        branch_name: &str,
        worktree_path: &Path,
        create_branch: bool,
    ) -> Result<()>;
    fn remove_worktree(&self, worktree_name: &str) -> Result<()>;
    fn list_worktrees(&self) -> Result<Vec<String>>;
    fn delete_branch(&self, branch_name: &str) -> Result<()>;
}
