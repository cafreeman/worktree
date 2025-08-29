use anyhow::{Context, Result};
use git2::{BranchType, Repository};
use std::path::{Path, PathBuf};

use crate::traits::GitOperations;

pub struct GitRepo {
    repo: Repository,
}

impl GitRepo {
    /// Opens a git repository at the specified path
    ///
    /// # Errors
    /// Returns an error if:
    /// - The path is not a valid git repository
    /// - Failed to access the repository
    pub fn open(path: &Path) -> Result<Self> {
        let repo = Repository::discover(path).context("Failed to find git repository")?;
        Ok(Self { repo })
    }

    #[must_use]
    pub fn get_repo_path(&self) -> &Path {
        self.repo.workdir().unwrap_or_else(|| self.repo.path())
    }

    /// Checks if a branch exists in the repository
    ///
    /// # Errors
    /// Returns an error if git operations fail
    pub fn branch_exists(&self, branch_name: &str) -> Result<bool> {
        match self.repo.find_branch(branch_name, BranchType::Local) {
            Ok(_) => Ok(true),
            Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// Creates a new worktree for the specified branch
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to create the worktree
    /// - Branch doesn't exist and create_branch is false
    /// - Git operations fail
    pub fn create_worktree(
        &self,
        branch_name: &str,
        worktree_path: &Path,
        create_branch: bool,
    ) -> Result<()> {
        if create_branch && !self.branch_exists(branch_name)? {
            let head = self.repo.head()?;
            let target_commit = head.peel_to_commit()?;
            self.repo.branch(branch_name, &target_commit, false)?;
        }

        // Use the directory name as the worktree name to avoid filesystem conflicts
        let worktree_name = worktree_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(branch_name);

        self.repo.worktree(
            worktree_name,
            worktree_path,
            Some(&git2::WorktreeAddOptions::new()),
        )?;

        Ok(())
    }

    /// Removes a worktree from the repository
    ///
    /// # Errors
    /// Returns an error if git operations fail
    pub fn remove_worktree(&self, worktree_name: &str) -> Result<()> {
        let worktree = self.repo.find_worktree(worktree_name)?;
        worktree.prune(Some(git2::WorktreePruneOptions::new().valid(true)))?;
        Ok(())
    }

    /// Lists all worktrees in the repository
    ///
    /// # Errors
    /// Returns an error if git operations fail
    pub fn list_worktrees(&self) -> Result<Vec<String>> {
        let worktree_names = self.repo.worktrees()?;
        Ok(worktree_names
            .into_iter()
            .flatten()
            .map(std::string::ToString::to_string)
            .collect())
    }

    /// Deletes a branch from the repository
    ///
    /// # Errors
    /// Returns an error if:
    /// - Branch doesn't exist
    /// - Git operations fail
    pub fn delete_branch(&self, branch_name: &str) -> Result<()> {
        let mut branch = self.repo.find_branch(branch_name, BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }
}

impl GitOperations for GitRepo {
    fn open(path: &Path) -> Result<Box<dyn GitOperations>> {
        let git_repo = GitRepo::open(path)?;
        Ok(Box::new(git_repo))
    }

    fn get_repo_path(&self) -> PathBuf {
        self.get_repo_path().to_path_buf()
    }

    fn branch_exists(&self, branch_name: &str) -> Result<bool> {
        self.branch_exists(branch_name)
    }

    fn create_worktree(
        &self,
        branch_name: &str,
        worktree_path: &Path,
        create_branch: bool,
    ) -> Result<()> {
        self.create_worktree(branch_name, worktree_path, create_branch)
    }

    fn remove_worktree(&self, worktree_name: &str) -> Result<()> {
        self.remove_worktree(worktree_name)
    }

    fn list_worktrees(&self) -> Result<Vec<String>> {
        self.list_worktrees()
    }

    fn delete_branch(&self, branch_name: &str) -> Result<()> {
        self.delete_branch(branch_name)
    }
}
