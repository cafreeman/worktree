use anyhow::Result;
use std::path::PathBuf;

use crate::git::GitRepo;
use crate::selection::{RealSelectionProvider, SelectionProvider};
use crate::storage::{WorktreeStorage, read_worktree_head_branch};

/// Jump to a worktree directory
///
/// # Errors
/// Returns an error if storage access fails, the target is not found, or interactive
/// selection fails.
pub fn jump_worktree(
    target: Option<&str>,
    interactive: bool,
    list_completions: bool,
    current_repo_only: bool,
) -> Result<()> {
    jump_worktree_with_provider(
        target,
        interactive,
        list_completions,
        current_repo_only,
        &RealSelectionProvider,
    )
}

/// Jump to a worktree directory with a custom selection provider (for testing)
///
/// # Errors
/// Returns an error if storage access fails, the target is not found, or interactive
/// selection fails.
pub fn jump_worktree_with_provider(
    target: Option<&str>,
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

    let target_path = if interactive || target.is_none() {
        select_worktree_interactive(&storage, current_repo_only, provider)?
    } else if let Some(target_name) = target {
        find_worktree_by_name(&storage, target_name, current_repo_only)?
    } else {
        anyhow::bail!("No target specified for worktree jump");
    };

    // Output just the path (shell function will handle cd)
    println!("{}", target_path.display());
    Ok(())
}

fn list_worktree_completions(storage: &WorktreeStorage, current_repo_only: bool) -> Result<()> {
    let worktrees = get_available_worktrees(storage, current_repo_only)?;

    for (_, feature_name, _) in worktrees {
        // Emit feature names for completions
        println!("{}", feature_name);
    }

    Ok(())
}

fn select_worktree_interactive(
    storage: &WorktreeStorage,
    current_repo_only: bool,
    provider: &dyn SelectionProvider,
) -> Result<PathBuf> {
    let worktrees = get_available_worktrees(storage, current_repo_only)?;

    if worktrees.is_empty() {
        anyhow::bail!("No worktrees found");
    }

    // Format: "repo/feature-name (current-branch)  /path"
    let options: Vec<String> = worktrees
        .iter()
        .map(|(repo, feature_name, path)| {
            let branch_info = read_worktree_head_branch(path)
                .map(|b| format!(" ({})", b))
                .unwrap_or_default();
            format!("{}/{}{} ({})", repo, feature_name, branch_info, path.display())
        })
        .collect();

    let selection = provider.select("Jump to worktree:", options.clone())?;

    let index = options
        .iter()
        .position(|o| o == &selection)
        .ok_or_else(|| anyhow::anyhow!("Selected option not found in list"))?;

    Ok(worktrees[index].2.clone())
}

fn find_worktree_by_name(
    storage: &WorktreeStorage,
    target: &str,
    current_repo_only: bool,
) -> Result<PathBuf> {
    let worktrees = get_available_worktrees(storage, current_repo_only)?;

    // Try exact match against feature name (directory name)
    for (_repo, feature_name, path) in &worktrees {
        if feature_name == target {
            return Ok(path.clone());
        }
    }

    // Try partial match against feature name
    let matches: Vec<_> = worktrees
        .iter()
        .filter(|(_, feature_name, _)| feature_name.contains(target))
        .collect();

    match matches.len() {
        0 => anyhow::bail!("No worktree found matching '{}'", target),
        1 => Ok(matches[0].2.clone()),
        _ => {
            eprintln!(
                "Multiple worktrees match '{}'. Please be more specific:",
                target
            );
            for (repo, feature_name, _) in matches {
                eprintln!("  {}/{}", repo, feature_name);
            }
            anyhow::bail!("Ambiguous worktree name");
        }
    }
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
