use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::git::GitRepo;
use crate::selection::{RealSelectionProvider, SelectionProvider};
use crate::storage::{WorktreeStorage, read_worktree_head_branch};

/// Removes a worktree, preserving branches by default
///
/// # Errors
/// Returns an error if the target worktree doesn't exist, storage access fails,
/// git operations fail, or the worktree directory cannot be removed.
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
/// Returns an error if the target worktree doesn't exist, storage access fails,
/// git operations fail, or the worktree directory cannot be removed.
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

    let (worktree_path, feature_name) = if interactive || target.is_none() {
        select_worktree_for_removal(&storage, current_repo_only, provider)?
    } else if let Some(target_str) = target {
        resolve_target(target_str, &storage, &repo_name)?
    } else {
        anyhow::bail!("No target specified for worktree removal");
    };

    if !worktree_path.exists() {
        anyhow::bail!("Worktree path does not exist: {}", worktree_path.display());
    }

    println!(
        "Removing worktree '{}': {}",
        feature_name,
        worktree_path.display()
    );

    // Read current branch from worktree HEAD before removing it
    let current_branch = read_worktree_head_branch(&worktree_path);

    // Use the feature name (directory name) as the worktree name for git
    let worktree_name = worktree_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&feature_name);

    // Remove the filesystem directory first
    if worktree_path.exists() {
        fs::remove_dir_all(&worktree_path).context("Failed to remove worktree directory")?;
    }

    git_repo
        .remove_worktree(worktree_name)
        .context("Failed to remove worktree from git")?;

    // Clean up origin information
    if let Err(e) = storage.remove_worktree_origin(&repo_name, &feature_name) {
        println!("⚠ Warning: Failed to clean up origin information: {}", e);
    }

    // Delete branch only when explicitly requested via --delete-branch
    if delete_branch {
        if let Some(branch) = &current_branch {
            println!("Deleting branch: {}", branch);
            match git_repo.delete_branch(branch) {
                Ok(_) => println!("✓ Branch deleted successfully"),
                Err(e) => println!("⚠ Warning: Failed to delete branch: {}", e),
            }
        } else {
            println!("⚠ Warning: Could not determine branch to delete (detached HEAD or error)");
        }
    } else if let Some(branch) = &current_branch {
        println!(
            "Branch '{}' preserved (use --delete-branch to remove it)",
            branch
        );
    }

    println!("✓ Worktree removed successfully!");

    Ok(())
}

fn resolve_target(
    target: &str,
    storage: &WorktreeStorage,
    repo_name: &str,
) -> Result<(PathBuf, String)> {
    // Match by feature name (directory name) directly
    let worktree_path = storage.get_worktree_path(repo_name, target);
    if worktree_path.exists() {
        return Ok((worktree_path, target.to_string()));
    }

    // Try partial match against known worktrees
    let known = storage.list_repo_worktrees(repo_name)?;
    let matches: Vec<&String> = known.iter().filter(|name| name.contains(target)).collect();

    match matches.len() {
        0 => anyhow::bail!("No worktree found matching '{}'", target),
        1 => {
            let feature_name = matches[0].clone();
            let path = storage.get_worktree_path(repo_name, &feature_name);
            Ok((path, feature_name))
        }
        _ => {
            eprintln!(
                "Multiple worktrees match '{}'. Please be more specific:",
                target
            );
            for name in &matches {
                eprintln!("  {}", name);
            }
            anyhow::bail!("Ambiguous worktree name '{}'", target);
        }
    }
}

fn list_worktree_completions(storage: &WorktreeStorage, current_repo_only: bool) -> Result<()> {
    let worktrees = get_available_worktrees(storage, current_repo_only)?;

    for (_, feature_name, _) in worktrees {
        println!("{}", feature_name);
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

    // Format: "feature-name (current-branch)  /path"
    let options: Vec<String> = worktrees
        .iter()
        .map(|(repo, feature_name, path)| {
            let branch_info = read_worktree_head_branch(path)
                .map(|b| format!(" ({})", b))
                .unwrap_or_default();
            format!(
                "{}/{}{} ({})",
                repo,
                feature_name,
                branch_info,
                path.display()
            )
        })
        .collect();

    let selection = provider.select("Select worktree to remove:", options.clone())?;

    let index = options
        .iter()
        .position(|o| o == &selection)
        .ok_or_else(|| anyhow::anyhow!("Selected option not found in list"))?;

    let (_, feature_name, path) = &worktrees[index];
    Ok((path.clone(), feature_name.clone()))
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
            for feature_name in repo_worktrees {
                let worktree_path = storage.get_worktree_path(&repo_name, &feature_name);
                if worktree_path.exists() {
                    worktrees.push((repo_name.clone(), feature_name, worktree_path));
                }
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
