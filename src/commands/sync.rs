use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::commands::create;
use crate::config::WorktreeConfig;
use crate::git::GitRepo;
use crate::storage::WorktreeStorage;

/// Syncs configuration files (and symlinks) between worktrees.
///
/// With both `from` and `to` given, syncs that specific pair (pairwise mode).
/// With neither given, resolves the current repo's origin and broadcasts config
/// to every worktree of that repo (broadcast mode). Exactly one of `from`/`to`
/// given is a usage error.
///
/// # Errors
/// Returns an error if:
/// - Exactly one of `from`/`to` is given
/// - Source or target worktree doesn't exist
/// - Failed to access storage system
/// - Failed to copy or symlink configuration files
pub fn sync(from: Option<&str>, to: Option<&str>) -> Result<()> {
    match (from, to) {
        (Some(from), Some(to)) => sync_pair_by_name(from, to),
        (None, None) => sync_broadcast(),
        _ => anyhow::bail!(
            "Both 'from' and 'to' must be given for a specific sync, or neither for broadcast mode (sync all worktrees of the current repo)."
        ),
    }
}

/// Pairwise mode: resolve `from`/`to` as worktree names or paths and sync between them.
fn sync_pair_by_name(from: &str, to: &str) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let repo_path = git_repo.get_repo_path();

    let storage = WorktreeStorage::new()?;
    let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

    let (from_path, _) = resolve_worktree_path(from, &storage, &repo_name)?;
    let (to_path, _) = resolve_worktree_path(to, &storage, &repo_name)?;

    let config = WorktreeConfig::load_from_repo(repo_path)?;
    sync_one(&from_path, &to_path, &config)
}

/// Broadcast mode: sync from the current repo's origin to every worktree of that repo.
fn sync_broadcast() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let storage = WorktreeStorage::new()?;

    let (repo_name, origin_path) = resolve_repo_origin(&current_dir, &storage)?;

    let worktree_names = storage.list_repo_worktrees(&repo_name)?;
    if worktree_names.is_empty() {
        println!("No worktrees found for repo '{}'.", repo_name);
        return Ok(());
    }

    let config = WorktreeConfig::load_from_repo(&origin_path)?;

    println!("Syncing config from origin: {}", origin_path.display());
    println!();

    for worktree_name in worktree_names {
        let target_path = storage.get_worktree_path(&repo_name, &worktree_name);

        if target_path == origin_path {
            continue;
        }

        println!("→ {}", worktree_name);
        sync_one(&origin_path, &target_path, &config)?;
        println!();
    }

    println!("✓ Broadcast sync complete!");

    Ok(())
}

/// Determines the origin repo path and repo name for broadcast mode, whether invoked
/// from inside the origin repo itself or from inside one of its managed worktrees.
fn resolve_repo_origin(current_dir: &Path, storage: &WorktreeStorage) -> Result<(String, PathBuf)> {
    let storage_root = storage
        .get_root_dir()
        .canonicalize()
        .unwrap_or_else(|_| storage.get_root_dir().clone());
    let canonical_current = current_dir
        .canonicalize()
        .unwrap_or_else(|_| current_dir.to_path_buf());

    if let Ok(relative_path) = canonical_current.strip_prefix(&storage_root) {
        let components: Vec<_> = relative_path.components().collect();
        if components.len() >= 2 {
            let repo_name = components[0].as_os_str().to_string_lossy().to_string();
            let feature_name = components[1].as_os_str().to_string_lossy().to_string();

            let origin = storage
                .get_worktree_origin(&repo_name, &feature_name)?
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "No origin information available for this worktree.\n\
                        This worktree may have been created before the back feature was added."
                    )
                })?;

            return Ok((repo_name, PathBuf::from(origin)));
        }
    }

    // Not inside a managed worktree — treat the current directory as the origin repo itself.
    let git_repo = GitRepo::open(current_dir)?;
    let repo_path = git_repo.get_repo_path();
    let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

    Ok((repo_name, repo_path.to_path_buf()))
}

/// Syncs symlinks (force-relinking drifted paths) then copies config files
/// from `source_path` to `target_path`.
fn sync_one(source_path: &Path, target_path: &Path, config: &WorktreeConfig) -> Result<()> {
    if !source_path.exists() {
        anyhow::bail!("Source worktree does not exist: {}", source_path.display());
    }

    if !target_path.exists() {
        anyhow::bail!("Target worktree does not exist: {}", target_path.display());
    }

    println!("Syncing config files:");
    println!("  From: {}", source_path.display());
    println!("  To: {}", target_path.display());
    println!();

    create::create_symlinks(source_path, target_path, config, true)?;
    create::copy_config_files(source_path, target_path, config)?;

    println!("✓ Config files synced successfully!");

    Ok(())
}

fn resolve_worktree_path(
    target: &str,
    storage: &WorktreeStorage,
    repo_name: &str,
) -> Result<(PathBuf, String)> {
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
