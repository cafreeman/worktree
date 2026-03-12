use anyhow::Result;
use std::path::PathBuf;

use crate::storage::WorktreeStorage;

/// Navigate back to the original repository that this worktree was created from
///
/// # Errors
/// Returns an error if not in a managed worktree directory, origin info is missing,
/// or the origin path no longer exists.
pub fn back_to_origin() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let storage = WorktreeStorage::new()?;

    let (repo_name, feature_name) = determine_current_worktree(&current_dir, &storage)?;

    let origin_path = storage
        .get_worktree_origin(&repo_name, &feature_name)?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No origin information available for this worktree.\n\
            This worktree may have been created before the back feature was added."
            )
        })?;

    let origin_pathbuf = PathBuf::from(&origin_path);

    if !origin_pathbuf.exists() {
        anyhow::bail!(
            "Origin repository no longer exists at: {}\n\
            The original repository may have been moved or deleted.",
            origin_path
        );
    }

    if !origin_pathbuf.is_dir() {
        anyhow::bail!("Origin path is not a directory: {}", origin_path);
    }

    println!("{}", origin_path);
    Ok(())
}

/// Determines the current worktree from the current directory path.
/// Returns (repo_name, feature_name) where feature_name is the directory name.
///
/// # Errors
/// Returns an error if not in a worktree directory managed by this tool.
fn determine_current_worktree(
    current_dir: &std::path::Path,
    storage: &WorktreeStorage,
) -> Result<(String, String)> {
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
            return Ok((repo_name, feature_name));
        }
    }

    anyhow::bail!(
        "Not currently in a worktree directory managed by this tool.\n\
        The back command only works from within worktree directories created by 'worktree create'."
    )
}
