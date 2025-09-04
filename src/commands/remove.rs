use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::git::GitRepo;
use crate::selection::{
    RealSelectionProvider, SelectionProvider, extract_branch_from_selection,
    extract_path_from_selection,
};
use crate::storage::WorktreeStorage;

/// Removes a worktree and optionally deletes the associated branch
///
/// # Errors
/// Returns an error if:
/// - The target worktree doesn't exist
/// - Failed to access storage system
/// - Git operations fail
/// - Failed to remove worktree directory
/// - Interactive selection fails
pub fn remove_worktree(
    target: Option<&str>,
    delete_branch: bool,
    interactive: bool,
    list_completions: bool,
    current_repo_only: bool,
) -> Result<()> {
    remove_worktree_with_provider(
        target,
        delete_branch,
        interactive,
        list_completions,
        current_repo_only,
        &RealSelectionProvider,
    )
}

/// Removes a worktree with a custom selection provider (for testing)
///
/// # Errors
/// Returns an error if:
/// - The target worktree doesn't exist
/// - Failed to access storage system
/// - Git operations fail
/// - Failed to remove worktree directory
/// - Interactive selection fails
pub fn remove_worktree_with_provider(
    target: Option<&str>,
    delete_branch: bool,
    interactive: bool,
    list_completions: bool,
    current_repo_only: bool,
    provider: &dyn SelectionProvider,
) -> Result<()> {
    let storage = WorktreeStorage::new()?;

    if list_completions {
        list_worktree_completions(&storage, current_repo_only)?;
        return Ok(());
    }

    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let repo_path = git_repo.get_repo_path();
    let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

    let (worktree_path, branch_name) = if interactive || target.is_none() {
        select_worktree_for_removal(&storage, current_repo_only, provider)?
    } else if let Some(target_str) = target {
        resolve_target(target_str, &storage, &repo_name)?
    } else {
        anyhow::bail!("No target specified for worktree removal");
    };

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
            Ok(_) => {
                println!("✓ Branch deleted successfully");
                // Unmark managed status
                storage.unmark_branch_managed(&repo_name, &branch_name);
                // Optionally remove mapping for this branch if desired; keep for cleanup symmetry
            }
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
                    if let Some(original_branch) =
                        storage.get_original_branch_name(repo_name, sanitized_name)?
                    {
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

    // Helper function to check if target contains characters that would be sanitized
    let contains_special_chars = |s: &str| {
        s.chars()
            .any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
    };

    // If target contains special characters, it's likely a canonical branch name
    if contains_special_chars(target) {
        let worktree_path = storage.get_worktree_path(repo_name, target);
        if worktree_path.exists() {
            return Ok((worktree_path, target.to_string()));
        }
        anyhow::bail!("No worktree found for branch '{}'", target);
    }

    // Target doesn't contain special chars - it could be either canonical or sanitized
    // Try as canonical first
    let worktree_path = storage.get_worktree_path(repo_name, target);
    if worktree_path.exists() {
        // Check if there's a mapping that shows this is actually a sanitized name
        if let Some(original_branch) = storage.get_original_branch_name(repo_name, target)? {
            // Target is sanitized, return the original branch name
            return Ok((worktree_path, original_branch));
        }

        // No mapping found - check if the branch actually exists in git
        // If git has a branch with this exact name, then target is canonical
        let current_dir = std::env::current_dir()?;
        if let Ok(git_repo) = crate::git::GitRepo::open(&current_dir) {
            if let Ok(branches) = git_repo.list_local_branches() {
                if branches.contains(&target.to_string()) {
                    // Git has a branch with this exact name, so target is canonical
                    return Ok((worktree_path, target.to_string()));
                }
            }
        }

        // Git doesn't have a branch with this name, so target is likely sanitized
        // but we can't resolve it without the mapping - error out for safety
        anyhow::bail!(
            "Cannot determine canonical branch name for '{}'. The branch mapping file may be missing or corrupted. \
             Please specify the full branch name (e.g., 'feature/branch-name' instead of 'feature-branch-name')",
            target
        );
    }

    // Target doesn't exist as canonical, try as sanitized with mapping lookup
    if let Some(original_branch) = storage.get_original_branch_name(repo_name, target)? {
        let path = storage.get_worktree_path(repo_name, &original_branch);
        if path.exists() {
            return Ok((path, original_branch));
        }
    }

    anyhow::bail!("No worktree found matching '{}'", target);
}

fn list_worktree_completions(storage: &WorktreeStorage, current_repo_only: bool) -> Result<()> {
    let worktrees = get_available_worktrees(storage, current_repo_only)?;

    for (_, branch, _) in worktrees {
        // For completions, we want the original branch name
        println!("{}", branch);
    }

    Ok(())
}

fn select_worktree_for_removal(
    storage: &WorktreeStorage,
    current_repo_only: bool,
    provider: &dyn SelectionProvider,
) -> Result<(PathBuf, String)> {
    let worktrees = get_available_worktrees(storage, current_repo_only)?;

    if worktrees.is_empty() {
        anyhow::bail!("No worktrees found");
    }

    // Format for display: "repo/branch (path)"
    let options: Vec<String> = worktrees
        .iter()
        .map(|(repo, branch, path)| format!("{}/{} ({})", repo, branch, path.display()))
        .collect();

    let selection = provider.select("Select worktree to remove:", options)?;

    // Extract path and branch from selection using helper functions
    let path = extract_path_from_selection(&selection)?;
    let branch = extract_branch_from_selection(&selection)?;

    Ok((path, branch))
}

fn get_available_worktrees(
    storage: &WorktreeStorage,
    current_repo_only: bool,
) -> Result<Vec<(String, String, PathBuf)>> {
    let mut worktrees = Vec::new();

    if current_repo_only {
        let current_dir = std::env::current_dir()?;
        if let Ok(git_repo) = GitRepo::open(&current_dir) {
            let repo_path = git_repo.get_repo_path();
            let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

            let repo_worktrees = storage.list_repo_worktrees(&repo_name)?;
            for worktree in repo_worktrees {
                let worktree_path = storage.get_worktree_path(&repo_name, &worktree);
                if worktree_path.exists() {
                    // Get original branch name or fall back to sanitized
                    let display_name = storage
                        .get_original_branch_name(&repo_name, &worktree)?
                        .unwrap_or_else(|| worktree.clone());

                    worktrees.push((repo_name.clone(), display_name, worktree_path));
                }
            }
        }
    } else {
        let all_worktrees = storage.list_all_worktrees()?;
        for (repo_name, repo_worktrees) in all_worktrees {
            for worktree in repo_worktrees {
                let worktree_path = storage.get_worktree_path(&repo_name, &worktree);
                if worktree_path.exists() {
                    // Get original branch name or fall back to sanitized
                    let display_name = storage
                        .get_original_branch_name(&repo_name, &worktree)?
                        .unwrap_or_else(|| worktree.clone());

                    worktrees.push((repo_name.clone(), display_name, worktree_path));
                }
            }
        }
    }

    Ok(worktrees)
}
