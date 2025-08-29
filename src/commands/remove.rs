use anyhow::{Context, Result};
use std::fs;

use crate::git::GitRepo;
use crate::storage::WorktreeStorage;

/// Removes a worktree and optionally deletes the associated branch
///
/// # Errors
/// Returns an error if:
/// - The target worktree doesn't exist
/// - Failed to access storage system
/// - Git operations fail
/// - Failed to remove worktree directory
pub fn remove_worktree(target: &str, delete_branch: bool) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let repo_path = git_repo.get_repo_path();

    let storage = WorktreeStorage::new()?;
    let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

    let (worktree_path, branch_name) = resolve_target(target, &storage, &repo_name)?;

    if !worktree_path.exists() {
        anyhow::bail!("Worktree path does not exist: {}", worktree_path.display());
    }

    println!("Removing worktree: {}", worktree_path.display());
    println!("Branch: {}", branch_name);

    // Use the directory name (sanitized) as the worktree name for git
    let worktree_name = worktree_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&branch_name);

    git_repo
        .remove_worktree(worktree_name)
        .context("Failed to remove worktree from git")?;

    if worktree_path.exists() {
        fs::remove_dir_all(&worktree_path).context("Failed to remove worktree directory")?;
    }

    // Clean up origin information
    if let Err(e) = storage.remove_worktree_origin(&repo_name, &branch_name) {
        println!("⚠ Warning: Failed to clean up origin information: {}", e);
    }

    if delete_branch {
        println!("Deleting branch: {}", branch_name);
        match git_repo.delete_branch(&branch_name) {
            Ok(_) => println!("✓ Branch deleted successfully"),
            Err(e) => println!("⚠ Warning: Failed to delete branch: {}", e),
        }
    }

    println!("✓ Worktree removed successfully!");

    Ok(())
}

fn resolve_target(
    target: &str,
    storage: &WorktreeStorage,
    repo_name: &str,
) -> Result<(std::path::PathBuf, String)> {
    use std::path::Path;
    
    // Check if target is an absolute path
    let target_path = Path::new(target);
    if target_path.is_absolute() {
        // Verify this is a valid worktree path within our storage structure
        let storage_root = storage.get_repo_storage_dir(repo_name);
        if let Ok(relative_path) = target_path.strip_prefix(&storage_root) {
            if let Some(branch_dir) = relative_path.file_name() {
                if let Some(sanitized_name) = branch_dir.to_str() {
                    // Try to get the original branch name from the sanitized name
                    if let Some(original_branch) = storage.get_original_branch_name(repo_name, sanitized_name)? {
                        return Ok((target_path.to_path_buf(), original_branch));
                    } else {
                        // If no mapping exists, the sanitized name might be the original
                        return Ok((target_path.to_path_buf(), sanitized_name.to_string()));
                    }
                }
            }
        }
        anyhow::bail!("Invalid worktree path: {}", target);
    }

    // Try target as original branch name first
    let worktree_path = storage.get_worktree_path(repo_name, target);
    if worktree_path.exists() {
        return Ok((worktree_path, target.to_string()));
    }

    // If that doesn't exist, maybe target is already a sanitized name
    // Check if there's a mapping from this sanitized name to an original
    if let Some(original_branch) = storage.get_original_branch_name(repo_name, target)? {
        let path = storage.get_worktree_path(repo_name, &original_branch);
        if path.exists() {
            return Ok((path, original_branch));
        }
    }

    // Fallback: use target as-is (might be a sanitized name without mapping)
    Ok((worktree_path, target.to_string()))
}
