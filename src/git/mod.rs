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

    /// Returns the working directory of the main repository.
    ///
    /// When discovered from inside a linked worktree, this is the original
    /// repository's directory, not the worktree's own directory. Storage
    /// naming, origin tracking, and config loading all key off the main
    /// repository, so they stay consistent no matter where a command runs.
    ///
    /// # Errors
    /// Returns an error for bare repositories (they have no working directory).
    pub fn get_repo_path(&self) -> Result<&Path> {
        if self.repo.is_bare() {
            anyhow::bail!("Bare repositories are not supported (no working directory)");
        }

        // commondir() points at the main repo's .git directory both from the
        // main repo itself and from any linked worktree; its parent is the
        // main working directory.
        let common_git_dir = self.repo.commondir();

        // A linked worktree of a bare main repo has no main working directory
        // either; commondir's parent would be the directory CONTAINING the
        // bare repo, which is not a worktree of anything.
        if let Ok(main_repo) = Repository::open(common_git_dir) {
            if main_repo.is_bare() {
                anyhow::bail!("Bare repositories are not supported (no working directory)");
            }
        }

        common_git_dir
            .parent()
            .context("Failed to determine main repository working directory")
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
        self.create_worktree_from(branch_name, worktree_path, create_branch, None)
    }

    /// Creates a new worktree for the specified branch from a specific starting point
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to create the worktree
    /// - Branch doesn't exist and create_branch is false
    /// - Failed to resolve the starting reference
    /// - Git operations fail
    pub fn create_worktree_from(
        &self,
        branch_name: &str,
        worktree_path: &Path,
        create_branch: bool,
        from_ref: Option<&str>,
    ) -> Result<()> {
        // Create branch if needed
        if create_branch {
            let target_commit = if let Some(from_ref) = from_ref {
                self.resolve_reference(from_ref)?
            } else {
                let head = self.repo.head()?;
                head.peel_to_commit()?
            };
            self.repo.branch(branch_name, &target_commit, false)?;
        }

        // Get the branch reference to use for the worktree
        let branch = self
            .repo
            .find_branch(branch_name, BranchType::Local)
            .with_context(|| format!("Failed to find branch '{}'", branch_name))?;

        // Use the directory name as the worktree name to avoid filesystem conflicts
        let worktree_name = worktree_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(branch_name);

        // Configure options to use the specified branch
        let mut opts = git2::WorktreeAddOptions::new();
        opts.reference(Some(branch.get()));

        self.repo
            .worktree(worktree_name, worktree_path, Some(&opts))?;

        Ok(())
    }

    /// Resolves a git reference (branch, tag, commit) to a commit object
    ///
    /// # Errors
    /// Returns an error if:
    /// - The reference cannot be found
    /// - The reference cannot be resolved to a commit
    /// - Git operations fail
    pub fn resolve_reference(&self, reference: &str) -> Result<git2::Commit<'_>> {
        let obj = self
            .repo
            .revparse_single(reference)
            .with_context(|| format!("Failed to resolve reference '{}'", reference))?;
        obj.peel_to_commit()
            .with_context(|| format!("Reference '{}' does not point to a commit", reference))
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
            .flatten()
            .map(std::string::ToString::to_string)
            .collect())
    }

    /// Lists all worktrees with their paths and prunable status
    ///
    /// Returns a vector of `(name, path, is_prunable)` tuples.
    /// Worktrees whose path no longer exists on disk are prunable.
    ///
    /// # Errors
    /// Returns an error if git operations fail
    pub fn list_worktrees_with_paths(&self) -> Result<Vec<(String, std::path::PathBuf, bool)>> {
        let worktree_names = self.repo.worktrees()?;
        let mut result = Vec::new();

        for name in worktree_names.iter().flatten().flatten() {
            if let Ok(wt) = self.repo.find_worktree(name) {
                let path = wt.path().to_path_buf();
                let is_prunable = wt.is_prunable(None).unwrap_or(false);
                result.push((name.to_string(), path, is_prunable));
            }
        }

        Ok(result)
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

    /// Lists all local branches in the repository
    ///
    /// # Errors
    /// Returns an error if git operations fail
    pub fn list_local_branches(&self) -> Result<Vec<String>> {
        let branches = self.repo.branches(Some(BranchType::Local))?;
        let mut branch_names = Vec::new();

        for branch_result in branches {
            let (branch, _) = branch_result?;
            if let Some(name) = branch.name()? {
                branch_names.push(name.to_string());
            }
        }

        Ok(branch_names)
    }

    /// Lists all remote branches in the repository
    ///
    /// # Errors
    /// Returns an error if git operations fail
    pub fn list_remote_branches(&self) -> Result<Vec<String>> {
        let branches = self.repo.branches(Some(BranchType::Remote))?;
        let mut branch_names = Vec::new();

        for branch_result in branches {
            let (branch, _) = branch_result?;
            if let Some(name) = branch.name()? {
                branch_names.push(name.to_string());
            }
        }

        Ok(branch_names)
    }

    /// Lists all tags in the repository
    ///
    /// # Errors
    /// Returns an error if git operations fail
    pub fn list_tags(&self) -> Result<Vec<String>> {
        let tags = self.repo.tag_names(None)?;
        let mut tag_names = Vec::new();

        for tag in tags.iter().flatten().flatten() {
            tag_names.push(tag.to_string());
        }

        Ok(tag_names)
    }
}

impl GitOperations for GitRepo {
    fn get_repo_path(&self) -> Result<PathBuf> {
        Ok(self.get_repo_path()?.to_path_buf())
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

    fn create_worktree_from(
        &self,
        branch_name: &str,
        worktree_path: &Path,
        create_branch: bool,
        from_ref: Option<&str>,
    ) -> Result<()> {
        self.create_worktree_from(branch_name, worktree_path, create_branch, from_ref)
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

    fn list_local_branches(&self) -> Result<Vec<String>> {
        self.list_local_branches()
    }

    fn list_remote_branches(&self) -> Result<Vec<String>> {
        self.list_remote_branches()
    }

    fn list_tags(&self) -> Result<Vec<String>> {
        self.list_tags()
    }
}
