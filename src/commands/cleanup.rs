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

    // Check each branch to see if it has a corresponding worktree directory
    for branch in &branches {
        if main_branches.contains(&branch.as_str()) {
            continue;
        }

        let worktree_path = storage.get_worktree_path(&repo_name, branch);

        if !worktree_path.exists() {
            println!("🗑️  Found orphaned branch: {}", branch);

            // Try to delete the branch
            match git_repo.delete_branch(branch) {
                Ok(_) => {
                    println!("   ✓ Deleted branch: {}", branch);
                    cleaned_branches.push(branch.clone());
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
    // This is handled by checking git worktree list and removing orphaned entries
    match std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(&current_dir)
        .output()
    {
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut current_worktree_path = None;
            let mut orphaned_worktrees = Vec::new();

            for line in output_str.lines() {
                if let Some(path) = line.strip_prefix("worktree ") {
                    current_worktree_path = Some(path.to_string());
                } else if line.starts_with("branch ") {
                    if let Some(path) = current_worktree_path.take() {
                        if !std::path::Path::new(&path).exists()
                            && !path.ends_with(&current_dir.to_string_lossy().to_string())
                        {
                            orphaned_worktrees.push(path);
                        }
                    }
                }
            }

            for orphaned_path in orphaned_worktrees {
                println!(
                    "🗑️  Found orphaned git worktree reference: {}",
                    orphaned_path
                );
                if let Some(worktree_name) = std::path::Path::new(&orphaned_path).file_name() {
                    if let Some(name_str) = worktree_name.to_str() {
                        match git_repo.remove_worktree(name_str) {
                            Ok(_) => println!("   ✓ Removed git worktree reference: {}", name_str),
                            Err(e) => println!(
                                "   ⚠ Warning: Could not remove git worktree reference {}: {}",
                                name_str, e
                            ),
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("   ⚠ Warning: Could not check git worktree list: {}", e);
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
