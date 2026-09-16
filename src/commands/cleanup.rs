use crate::commands::shared::canonical_or_raw;
use crate::git::GitRepo;
use crate::storage::WorktreeStorage;
use anyhow::Result;

/// Cleans up orphaned worktree references and directories
///
/// # Errors
/// Returns an error if git or storage access fails.
pub fn cleanup_worktrees() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let repo_path = git_repo.get_repo_path()?;

    let storage = WorktreeStorage::new()?;
    let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

    println!("🔍 Analyzing worktree state...");

    let mut cleaned = Vec::new();
    let canonical_current = canonical_or_raw(current_dir);

    // Clean up any git worktree references that point to non-existent directories
    match git_repo.list_worktrees_with_paths() {
        Ok(worktrees) => {
            for (name, path, is_prunable) in worktrees {
                let canonical_path = canonical_or_raw(path);

                // Never prune the worktree the caller is standing in
                if canonical_current.starts_with(&canonical_path) {
                    continue;
                }

                if is_prunable || !canonical_path.exists() {
                    println!(
                        "🗑️  Found orphaned git worktree reference: {}",
                        canonical_path.display()
                    );
                    match git_repo.remove_worktree(&name) {
                        Ok(_) => {
                            println!("   ✓ Removed git worktree reference: {}", name);
                            cleaned.push(name);
                        }
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

    // Report worktree directories for feature worktrees whose git reference no longer exists
    // (i.e., the dir exists but git doesn't know about that worktree anymore)
    if let Ok(repo_worktrees) = storage.list_repo_worktrees(&repo_name) {
        let git_worktree_paths: Vec<_> = git_repo
            .list_worktrees_with_paths()
            .unwrap_or_default()
            .into_iter()
            .map(|(_, path, _)| canonical_or_raw(path))
            .collect();

        for feature_name in repo_worktrees {
            let path = storage.get_worktree_path(&repo_name, &feature_name);
            if path.exists() && !git_worktree_paths.contains(&canonical_or_raw(path.clone())) {
                // Check if git even knows about this path
                // It may be a newly created worktree or an orphan
                // Only remove if we're confident it's orphaned (git prune would handle it)
                // For safety, just report it
                println!(
                    "ℹ️  Worktree directory exists but may not be registered with git: {} ({})",
                    feature_name,
                    path.display()
                );
            }
        }
    }

    if cleaned.is_empty() {
        println!("✨ Everything looks clean! No orphaned worktree references found.");
    } else {
        println!("\n✅ Cleanup complete!");
        println!(
            "   Removed {} orphaned git worktree reference(s)",
            cleaned.len()
        );
    }

    Ok(())
}
