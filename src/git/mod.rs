use anyhow::{Context, Result};
use git2::{BranchType, Repository};
use std::collections::HashMap;
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

        for tag in tags.iter().flatten() {
            tag_names.push(tag.to_string());
        }

        Ok(tag_names)
    }

    /// Enables worktree-specific configuration and copies parent repo's effective config
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to enable worktree configuration
    /// - Failed to read parent repository configuration
    /// - Failed to set worktree-specific configuration
    pub fn inherit_config(&self, worktree_path: &Path) -> Result<()> {
        // First, enable worktree-specific configuration for the main repository
        let mut main_config = self
            .repo
            .config()
            .context("Failed to get repository config")?;
        main_config
            .set_bool("extensions.worktreeConfig", true)
            .context("Failed to enable worktree config extension")?;

        // Open the worktree repository to set its config
        let worktree_repo =
            Repository::open(worktree_path).context("Failed to open worktree repository")?;

        // Get the effective config from the parent repository (includes conditional includes)
        let parent_config = self
            .get_effective_config()
            .context("Failed to read parent repository config")?;

        // Set worktree-specific configuration
        let mut worktree_config = worktree_repo
            .config()
            .context("Failed to get worktree config")?;

        // Copy relevant configuration keys to the worktree
        for (key, config_value) in parent_config {
            if should_inherit_config_key(&key) {
                match config_value {
                    ConfigValue::String(s) => {
                        if let Err(e) = worktree_config.set_str(&key, &s) {
                            eprintln!("Warning: Failed to set config {}: {}", key, e);
                        }
                    }
                    ConfigValue::Bool(b) => {
                        if let Err(e) = worktree_config.set_bool(&key, b) {
                            eprintln!("Warning: Failed to set config {}: {}", key, e);
                        }
                    }
                    ConfigValue::Int(i) => {
                        if let Err(e) = worktree_config.set_i64(&key, i) {
                            eprintln!("Warning: Failed to set config {}: {}", key, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Reads the effective configuration from the parent repository
    fn get_effective_config(&self) -> Result<HashMap<String, ConfigValue>> {
        let mut config = self
            .repo
            .config()
            .context("Failed to get repository config")?;

        let mut config_map = HashMap::new();

        // Get a snapshot of the current config which includes all effective values
        let snapshot = config
            .snapshot()
            .context("Failed to create config snapshot")?;

        let mut entries = snapshot
            .entries(None)
            .context("Failed to get config entries")?;

        while let Some(entry_result) = entries.next() {
            if let Ok(entry) = entry_result {
                if let Some(name) = entry.name() {
                    let key = name.to_string();

                    if let Some(value_str) = entry.value() {
                        // Try to determine the type and parse accordingly
                        let config_value = if let Ok(bool_val) = config.get_bool(&key) {
                            ConfigValue::Bool(bool_val)
                        } else if let Ok(int_val) = config.get_i64(&key) {
                            ConfigValue::Int(int_val)
                        } else {
                            ConfigValue::String(value_str.to_string())
                        };

                        config_map.insert(key, config_value);
                    }
                }
            }
        }

        Ok(config_map)
    }
}

#[derive(Debug, Clone)]
enum ConfigValue {
    String(String),
    Bool(bool),
    Int(i64),
}

/// Determines which configuration keys should be inherited by worktrees
fn should_inherit_config_key(key: &str) -> bool {
    // Don't inherit keys that are specific to the main repository
    const EXCLUDED_KEYS: &[&str] = &[
        "core.bare",
        "core.worktree",
        "core.repositoryformatversion",
        "extensions.worktreeconfig",
    ];

    // Don't inherit keys that start with excluded prefixes
    const EXCLUDED_PREFIXES: &[&str] = &["branch.", "remote.", "submodule."];

    // Include keys that are typically user-specific and should be inherited
    const INCLUDED_PREFIXES: &[&str] = &[
        "user.",
        "commit.",
        "gpg.",
        "credential.",
        "push.",
        "pull.",
        "merge.",
        "diff.",
        "log.",
        "color.",
        "core.editor",
        "core.pager",
        "core.autocrlf",
        "core.filemode",
        "init.defaultbranch",
    ];

    // Check if key should be excluded
    if EXCLUDED_KEYS.contains(&key) {
        return false;
    }

    if EXCLUDED_PREFIXES
        .iter()
        .any(|prefix| key.starts_with(prefix))
    {
        return false;
    }

    // Include if it matches an included prefix
    if INCLUDED_PREFIXES
        .iter()
        .any(|prefix| key.starts_with(prefix))
    {
        return true;
    }

    // For core.* keys, only include specific ones
    if key.starts_with("core.") {
        return INCLUDED_PREFIXES
            .iter()
            .any(|prefix| key == prefix.trim_end_matches('.'));
    }

    // Default to not inheriting unknown keys
    false
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

    fn inherit_config(&self, worktree_path: &Path) -> Result<()> {
        self.inherit_config(worktree_path)
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
