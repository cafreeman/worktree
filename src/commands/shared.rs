//! Helpers shared by multiple command implementations.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::git::GitRepo;
use crate::storage::WorktreeStorage;

/// Describes an available worktree: `(repo name, feature name, path)`.
pub type AvailableWorktree = (String, String, PathBuf);

/// Canonicalizes a path for comparisons, falling back to the original path
/// when it does not exist yet.
#[must_use]
pub fn canonical_or_raw(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

/// Compares paths while tolerating platform-specific symlink canonicalization.
#[must_use]
pub fn paths_match(left: &Path, right: &Path) -> bool {
    canonical_or_raw(left.to_path_buf()) == canonical_or_raw(right.to_path_buf())
}

/// Lists available (on-disk) worktrees: for the current repository only, or
/// across all repositories in storage.
///
/// # Errors
/// Returns an error if storage access fails, or — with `current_repo_only` —
/// if the current directory is not inside a git repository.
pub fn get_available_worktrees(
    storage: &WorktreeStorage,
    current_repo_only: bool,
) -> Result<Vec<AvailableWorktree>> {
    let mut worktrees = Vec::new();

    if current_repo_only {
        let current_dir = std::env::current_dir()?;
        let git_repo = GitRepo::open(&current_dir)?;
        let repo_path = git_repo.get_repo_path()?;
        let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

        let repo_worktrees = storage.list_repo_worktrees(&repo_name)?;
        for feature_name in repo_worktrees {
            let worktree_path = storage.get_worktree_path(&repo_name, &feature_name);
            if worktree_path.exists() {
                worktrees.push((repo_name.clone(), feature_name, worktree_path));
            }
        }
    } else {
        let all_worktrees = storage.list_all_worktrees()?;
        for (repo_name, repo_worktrees) in all_worktrees {
            for feature_name in repo_worktrees {
                let worktree_path = storage.get_worktree_path(&repo_name, &feature_name);
                if worktree_path.exists() {
                    worktrees.push((repo_name.clone(), feature_name, worktree_path));
                }
            }
        }
    }

    Ok(worktrees)
}
