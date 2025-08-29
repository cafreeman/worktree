use anyhow::Result;

use crate::git::GitRepo;
use crate::storage::WorktreeStorage;

pub fn list_worktrees(current_repo_only: bool) -> Result<()> {
    let storage = WorktreeStorage::new()?;

    if current_repo_only {
        list_current_repo_worktrees(&storage)?;
    } else {
        list_all_worktrees(&storage)?;
    }

    Ok(())
}

fn list_current_repo_worktrees(storage: &WorktreeStorage) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let repo_path = git_repo.get_repo_path();
    let repo_name = storage.get_repo_name(repo_path)?;

    println!("Worktrees for repository: {}", repo_name);
    println!("{}", "=".repeat(40));

    let worktrees = storage.list_repo_worktrees(&repo_name)?;

    if worktrees.is_empty() {
        println!("No worktrees found for this repository.");
        return Ok(());
    }

    for worktree in worktrees {
        let worktree_path = storage.get_worktree_path(&repo_name, &worktree);
        let status = if worktree_path.exists() {
            "✓ Active"
        } else {
            "✗ Missing"
        };

        // Try to get original branch name, fallback to sanitized name
        let display_name = storage
            .get_original_branch_name(&repo_name, &worktree)?
            .unwrap_or_else(|| worktree.clone());

        println!(
            "  {} {} ({})",
            status,
            display_name,
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
        for worktree in worktrees {
            let worktree_path = storage.get_worktree_path(&repo_name, &worktree);
            let status = if worktree_path.exists() { "✓" } else { "✗" };

            // Try to get original branch name, fallback to sanitized name
            let display_name = storage
                .get_original_branch_name(&repo_name, &worktree)?
                .unwrap_or_else(|| worktree.clone());

            println!(
                "  {} {} ({})",
                status,
                display_name,
                worktree_path.display()
            );
        }
    }

    Ok(())
}
