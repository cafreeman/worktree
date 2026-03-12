use anyhow::{Context, Result};
use std::fs;

use crate::git::GitRepo;
use crate::storage::WorktreeStorage;

/// Cleans up orphaned worktrees and branches to fix sync issues
///
/// This command will:
/// - Remove git branches that have no corresponding worktree directory
/// - Clean up branch mappings for non-existent worktrees
/// - Remove any git worktree references that point to non-existent directories
///
/// # Errors
/// Returns an error if:
/// - Failed to access git repository
/// - Failed to access storage system
/// - Git operations fail
pub fn cleanup_worktrees() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let repo_path = git_repo.get_repo_path();

    let storage = WorktreeStorage::new()?;
    let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

    println!("🔍 Analyzing worktree state...");

    // Get all local branches (excluding main/master)
    let branches = git_repo.list_local_branches()?;
    let main_branches = ["main", "master"];

    let mut cleaned_branches = Vec::new();
    let mut cleaned_mappings = Vec::new();

    // Check each branch to see if it is managed and has a missing worktree directory
    for branch in &branches {
        if main_branches.contains(&branch.as_str()) {
            continue;
        }

        let worktree_path = storage.get_worktree_path(&repo_name, branch);

        if !worktree_path.exists() && storage.is_branch_managed(&repo_name, branch) {
            println!("🗑️  Found orphaned managed branch: {}", branch);

            // Try to delete the branch
            match git_repo.delete_branch(branch) {
                Ok(_) => {
                    println!("   ✓ Deleted branch: {}", branch);
                    cleaned_branches.push(branch.clone());
                    // Unmark as managed and remove mapping
                    storage.unmark_branch_managed(&repo_name, branch);
                    if let Err(e) = storage.remove_branch_mapping(&repo_name, branch) {
                        println!(
                            "   ⚠ Warning: Failed to remove branch mapping for {}: {}",
                            branch, e
                        );
                    }
                }
                Err(e) => {
                    println!("   ⚠ Warning: Could not delete branch {}: {}", branch, e);
                }
            }
        }
    }

    // Clean up branch mappings for branches that no longer exist
    let repo_storage_dir = storage.get_repo_storage_dir(&repo_name);
    if repo_storage_dir.exists() {
        // Read the branch mapping file
        let mapping_file = repo_storage_dir.join(".branch-mapping");
        if mapping_file.exists() {
            match fs::read_to_string(&mapping_file) {
                Ok(content) => {
                    let mut new_lines = Vec::new();
                    let mut removed_mappings = Vec::new();

                    for line in content.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }

                        if let Some((_sanitized, original)) = line.split_once(" -> ") {
                            let worktree_path = storage.get_worktree_path(&repo_name, original);
                            if worktree_path.exists() {
                                new_lines.push(line.to_string());
                            } else {
                                removed_mappings.push(original.to_string());
                            }
                        }
                    }

                    if !removed_mappings.is_empty() {
                        // Write back the cleaned mapping file
                        let new_content = new_lines.join("\n") + "\n";
                        fs::write(&mapping_file, new_content)
                            .context("Failed to update branch mapping file")?;

                        for mapping in &removed_mappings {
                            println!("   ✓ Cleaned mapping for: {}", mapping);
                            cleaned_mappings.push(mapping.clone());
                        }
                    }
                }
                Err(e) => {
                    println!("   ⚠ Warning: Could not read branch mapping file: {}", e);
                }
            }
        }
    }

    // Clean up any git worktree references that point to non-existent directories
    // Use git2 API to list worktrees and find orphaned references
    match git_repo.list_worktrees_with_paths() {
        Ok(worktrees) => {
            for (name, path, is_prunable) in worktrees {
                // Skip the current working directory (main worktree check)
                if path == current_dir {
                    continue;
                }

                if is_prunable || !path.exists() {
                    println!("🗑️  Found orphaned git worktree reference: {}", path.display());
                    match git_repo.remove_worktree(&name) {
                        Ok(_) => println!("   ✓ Removed git worktree reference: {}", name),
                        Err(e) => println!(
                            "   ⚠ Warning: Could not remove git worktree reference {}: {}",
                            name, e
                        ),
                    }
                }
            }
        }
        Err(e) => {
            println!("   ⚠ Warning: Could not check git worktree list: {}", e);
        }
    }

    // Prune orphaned worktree directories for branches that no longer exist in git
    // Iterate managed worktrees stored under repo storage and check for branch existence
    if let Ok(repo_worktrees) = storage.list_repo_worktrees(&repo_name) {
        for sanitized in repo_worktrees {
            // Get the original branch name for accurate git checks.
            // If the mapping is missing or corrupted, skip this entry rather than
            // using the sanitized name (which would never match git branch names
            // like "feature/auth" stored as "feature-auth") and cause false orphan detection.
            let original_branch = match storage.get_original_branch_name(&repo_name, &sanitized) {
                Ok(Some(name)) => name,
                Ok(None) => {
                    eprintln!(
                        "Warning: Could not determine original branch name for worktree directory '{}', skipping",
                        sanitized
                    );
                    continue;
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Could not determine original branch name for worktree directory '{}', skipping: {}",
                        sanitized, e
                    );
                    continue;
                }
            };

            if !branches.contains(&original_branch) {
                // Branch no longer exists in git; remove directory and clean metadata
                let path = storage.get_worktree_path(&repo_name, &sanitized);
                if path.exists() {
                    println!(
                        "🗑️  Found orphaned worktree directory for deleted branch: {} ({})",
                        original_branch,
                        path.display()
                    );
                    if let Err(e) = fs::remove_dir_all(&path) {
                        println!("   ⚠ Warning: Failed to remove worktree directory: {}", e);
                    }
                }
                if let Err(e) = storage.remove_branch_mapping(&repo_name, &original_branch) {
                    println!(
                        "   ⚠ Warning: Failed to remove branch mapping for {}: {}",
                        original_branch, e
                    );
                } else {
                    cleaned_mappings.push(original_branch.clone());
                }
                if let Err(e) = storage.remove_worktree_origin(&repo_name, &original_branch) {
                    println!(
                        "   ⚠ Warning: Failed to remove origin info for {}: {}",
                        original_branch, e
                    );
                }
            }
        }
    }

    // Summary
    if cleaned_branches.is_empty() && cleaned_mappings.is_empty() {
        println!("✨ Everything looks clean! No orphaned branches or mappings found.");
    } else {
        println!("\n✅ Cleanup complete!");
        if !cleaned_branches.is_empty() {
            println!(
                "   Removed {} orphaned branch(es): {}",
                cleaned_branches.len(),
                cleaned_branches.join(", ")
            );
        }
        if !cleaned_mappings.is_empty() {
            println!("   Cleaned {} mapping(s)", cleaned_mappings.len());
        }
    }

    Ok(())
}
