use anyhow::Result;
use std::path::PathBuf;

use crate::storage::WorktreeStorage;

/// Navigate back to the original repository that this worktree was created from
///
/// # Errors
/// Returns an error if:
/// - Not currently in a worktree directory
/// - No origin information is stored for this worktree
/// - Failed to read the origin information
/// - The origin path no longer exists
pub fn back_to_origin() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let storage = WorktreeStorage::new()?;

    // Try to determine which worktree we're in by checking the path structure
    let (repo_name, branch_name) = determine_current_worktree(&current_dir, &storage)?;

    // Get the origin information from storage
    let origin_path = storage
        .get_worktree_origin(&repo_name, &branch_name)?
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

    // Output the path (shell function will handle cd)
    println!("{}", origin_path);
    Ok(())
}

/// Determines the current worktree from the current directory path
///
/// # Errors
/// Returns an error if:
/// - Not in a worktree directory managed by this tool
/// - Failed to parse the directory structure
fn determine_current_worktree(
    current_dir: &std::path::Path,
    storage: &WorktreeStorage,
) -> Result<(String, String)> {
    // Check if we're in a worktree directory under the storage root
    // Use canonical paths to handle symlinks correctly (e.g., /var -> /private/var on macOS)
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
            let sanitized_branch = components[1].as_os_str().to_string_lossy().to_string();

            // Get the original branch name from the mapping
            let original_branch = storage
                .get_original_branch_name(&repo_name, &sanitized_branch)?
                .unwrap_or(sanitized_branch);

            return Ok((repo_name, original_branch));
        }
    }

    anyhow::bail!(
        "Not currently in a worktree directory managed by this tool.\n\
        The back command only works from within worktree directories created by 'worktree create'."
    )
}
