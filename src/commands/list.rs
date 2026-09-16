use anyhow::Result;
use serde::Serialize;

use crate::git::GitRepo;
use crate::storage::{WorktreeStorage, read_worktree_head_branch};

#[derive(Debug, Serialize)]
struct ListOutput {
    current_repo_only: bool,
    repository: Option<String>,
    worktrees: Vec<ListWorktreeOutput>,
}

#[derive(Debug, Serialize)]
struct ListWorktreeOutput {
    repository: String,
    feature: String,
    path: String,
    exists: bool,
    branch: Option<String>,
}

/// Lists all worktrees, optionally filtered to current repository only
///
/// # Errors
/// Returns an error if storage access or git operations fail.
pub fn list_worktrees(current_repo_only: bool, json: bool) -> Result<()> {
    let storage = WorktreeStorage::new()?;

    if json {
        return list_worktrees_json(&storage, current_repo_only);
    }

    if current_repo_only {
        list_current_repo_worktrees(&storage)?;
    } else {
        list_all_worktrees(&storage)?;
    }

    Ok(())
}

fn list_worktrees_json(storage: &WorktreeStorage, current_repo_only: bool) -> Result<()> {
    let (repository, worktrees) = if current_repo_only {
        let current_dir = std::env::current_dir()?;
        let git_repo = GitRepo::open(&current_dir)?;
        let repo_path = git_repo.get_repo_path()?;
        let repo_name = WorktreeStorage::get_repo_name(repo_path)?;
        let features = storage.list_repo_worktrees(&repo_name)?;
        (
            Some(repo_name.clone()),
            features
                .into_iter()
                .map(|feature| list_entry(storage, &repo_name, feature))
                .collect(),
        )
    } else {
        let mut entries = Vec::new();
        for (repo_name, features) in storage.list_all_worktrees()? {
            entries.extend(
                features
                    .into_iter()
                    .map(|feature| list_entry(storage, &repo_name, feature)),
            );
        }
        (None, entries)
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&ListOutput {
            current_repo_only,
            repository,
            worktrees,
        })?
    );
    Ok(())
}

fn list_entry(storage: &WorktreeStorage, repo_name: &str, feature: String) -> ListWorktreeOutput {
    let path = storage.get_worktree_path(repo_name, &feature);
    let exists = path.exists();
    ListWorktreeOutput {
        repository: repo_name.to_string(),
        feature,
        path: path.display().to_string(),
        exists,
        branch: exists.then(|| read_worktree_head_branch(&path)).flatten(),
    }
}

fn list_current_repo_worktrees(storage: &WorktreeStorage) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let repo_path = git_repo.get_repo_path()?;
    let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

    println!("Worktrees for repository: {}", repo_name);
    println!("{}", "=".repeat(40));

    let worktrees = storage.list_repo_worktrees(&repo_name)?;

    if worktrees.is_empty() {
        println!("No worktrees found for this repository.");
        return Ok(());
    }

    for feature_name in worktrees {
        let worktree_path = storage.get_worktree_path(&repo_name, &feature_name);
        let status = if worktree_path.exists() {
            "✓ Active"
        } else {
            "✗ Missing"
        };

        let branch_info = if worktree_path.exists() {
            read_worktree_head_branch(&worktree_path)
                .map(|b| format!(" ({})", b))
                .unwrap_or_else(|| " (detached)".to_string())
        } else {
            String::new()
        };

        println!(
            "  {} {}{}  {}",
            status,
            feature_name,
            branch_info,
            worktree_path.display()
        );
    }

    Ok(())
}

fn list_all_worktrees(storage: &WorktreeStorage) -> Result<()> {
    println!("All managed worktrees:");
    println!("{}", "=".repeat(40));

    let all_worktrees = storage.list_all_worktrees()?;

    if all_worktrees.is_empty() {
        println!("No worktrees found.");
        return Ok(());
    }

    for (repo_name, worktrees) in all_worktrees {
        if worktrees.is_empty() {
            continue;
        }

        println!("\n📁 {}", repo_name);
        for feature_name in worktrees {
            let worktree_path = storage.get_worktree_path(&repo_name, &feature_name);
            let status = if worktree_path.exists() { "✓" } else { "✗" };

            let branch_info = if worktree_path.exists() {
                read_worktree_head_branch(&worktree_path)
                    .map(|b| format!(" ({})", b))
                    .unwrap_or_else(|| " (detached)".to_string())
            } else {
                String::new()
            };

            println!(
                "  {} {}{}  {}",
                status,
                feature_name,
                branch_info,
                worktree_path.display()
            );
        }
    }

    Ok(())
}
