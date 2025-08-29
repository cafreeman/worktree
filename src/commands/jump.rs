use anyhow::Result;
use inquire::Select;
use std::path::PathBuf;

use crate::git::GitRepo;
use crate::storage::WorktreeStorage;

/// Jump to a worktree directory
///
/// # Errors
/// Returns an error if:
/// - Failed to access the storage system
/// - Failed to determine current repository
/// - Git operations fail
/// - Interactive selection fails
pub fn jump_worktree(
    target: Option<String>,
    interactive: bool,
    list_completions: bool,
    current_repo_only: bool,
) -> Result<()> {
    let storage = WorktreeStorage::new()?;

    if list_completions {
        list_worktree_completions(&storage, current_repo_only)?;
        return Ok(());
    }

    let target_path = if interactive || target.is_none() {
        select_worktree_interactive(&storage, current_repo_only)?
    } else {
        find_worktree_by_name(&storage, &target.unwrap(), current_repo_only)?
    };

    // Output just the path (shell function will handle cd)
    println!("{}", target_path.display());
    Ok(())
}

fn list_worktree_completions(storage: &WorktreeStorage, current_repo_only: bool) -> Result<()> {
    let worktrees = get_available_worktrees(storage, current_repo_only)?;

    for (_, branch, _) in worktrees {
        // For completions, we want the original branch name
        println!("{}", branch);
    }

    Ok(())
}

fn select_worktree_interactive(
    storage: &WorktreeStorage,
    current_repo_only: bool,
) -> Result<PathBuf> {
    let worktrees = get_available_worktrees(storage, current_repo_only)?;

    if worktrees.is_empty() {
        anyhow::bail!("No worktrees found");
    }

    // Format for display: "repo/branch (path)"
    let options: Vec<String> = worktrees
        .iter()
        .map(|(repo, branch, path)| format!("{}/{} ({})", repo, branch, path.display()))
        .collect();

    let selection = Select::new("Jump to worktree:", options)
        .with_page_size(10)
        .with_vim_mode(true)
        .prompt()?;

    // Extract path from selection
    if let Some(path_start) = selection.rfind(" (") {
        let path_str = &selection[path_start + 2..selection.len() - 1];
        Ok(PathBuf::from(path_str))
    } else {
        anyhow::bail!("Invalid selection format")
    }
}

fn find_worktree_by_name(
    storage: &WorktreeStorage,
    target: &str,
    current_repo_only: bool,
) -> Result<PathBuf> {
    let worktrees = get_available_worktrees(storage, current_repo_only)?;

    // Try exact match first (with original branch names)
    for (_repo, branch, path) in &worktrees {
        if branch == target {
            return Ok(path.clone());
        }
    }

    // Try partial match
    let matches: Vec<_> = worktrees
        .iter()
        .filter(|(_, branch, _)| branch.contains(target))
        .collect();

    match matches.len() {
        0 => anyhow::bail!("No worktree found matching '{}'", target),
        1 => Ok(matches[0].2.clone()),
        _ => {
            // Multiple matches - show them and ask user to be more specific
            eprintln!(
                "Multiple worktrees match '{}'. Please be more specific:",
                target
            );
            for (repo, branch, _) in matches {
                eprintln!("  {}/{}", repo, branch);
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
