use anyhow::{Context, Result};
use std::path::Path;

use crate::commands::create;
use crate::config::WorktreeConfig;
use crate::git::GitRepo;
use crate::storage::WorktreeStorage;

/// Synchronizes configuration files between two worktrees
///
/// # Errors
/// Returns an error if:
/// - Source or target worktree doesn't exist
/// - Failed to access storage system
/// - Failed to copy configuration files
/// - Permission issues with file operations
pub fn sync_config(from: &str, to: &str) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let repo_path = git_repo.get_repo_path();

    let storage = WorktreeStorage::new()?;
    let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

    let (from_path, _) = resolve_worktree_path(from, &storage, &repo_name)?;
    let (to_path, _) = resolve_worktree_path(to, &storage, &repo_name)?;

    if !from_path.exists() {
        anyhow::bail!("Source worktree does not exist: {}", from_path.display());
    }

    if !to_path.exists() {
        anyhow::bail!("Target worktree does not exist: {}", to_path.display());
    }

    println!("Syncing config files:");
    println!("  From: {}", from_path.display());
    println!("  To: {}", to_path.display());
    println!();

    let config = WorktreeConfig::load_from_repo(repo_path)?;
    create::copy_config_files(&from_path, &to_path, &config)?;

    println!("✓ Config files synced successfully!");

    Ok(())
}

fn resolve_worktree_path(
    target: &str,
    storage: &WorktreeStorage,
    repo_name: &str,
) -> Result<(std::path::PathBuf, String)> {
    let target_path = Path::new(target);

    if target_path.is_absolute() && target_path.exists() {
        let branch_name = target_path
            .file_name()
            .and_then(|name| name.to_str())
            .context("Could not determine branch name from path")?
            .to_string();
        return Ok((target_path.to_path_buf(), branch_name));
    }

    let worktree_path = storage.get_worktree_path(repo_name, target);
    Ok((worktree_path, target.to_string()))
}
