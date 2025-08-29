use anyhow::{Context, Result};
use std::path::Path;

use crate::config::WorktreeConfig;
use crate::git::GitRepo;
use crate::storage::WorktreeStorage;

pub fn create_worktree(branch: &str, custom_path: Option<&str>, create_branch: bool) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    create_worktree_internal(&git_repo, branch, custom_path, create_branch)
}

/// Test version that accepts a mock git repository
pub fn create_worktree_with_git(
    git_repo: &dyn crate::traits::GitOperations,
    branch: &str,
    custom_path: Option<&str>,
    create_branch: bool,
) -> Result<()> {
    create_worktree_internal(git_repo, branch, custom_path, create_branch)
}

fn create_worktree_internal(
    git_repo: &dyn crate::traits::GitOperations,
    branch: &str,
    custom_path: Option<&str>,
    create_branch: bool,
) -> Result<()> {
    let repo_path = git_repo.get_repo_path();
    let storage = WorktreeStorage::new()?;
    let repo_name = storage.get_repo_name(&repo_path)?;
    let worktree_path = if let Some(path) = custom_path {
        Path::new(path).to_path_buf()
    } else {
        storage.get_worktree_path(&repo_name, branch)
    };

    if worktree_path.exists() {
        anyhow::bail!("Worktree path already exists: {}", worktree_path.display());
    }

    // Ensure parent directory exists
    if let Some(parent) = worktree_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;
    }

    println!(
        "Creating worktree for branch '{}' at: {}",
        branch,
        worktree_path.display()
    );

    if create_branch || !git_repo.branch_exists(branch)? {
        println!("Creating new branch: {}", branch);
    }

    git_repo.create_worktree(branch, &worktree_path, create_branch)?;

    // Store branch mapping if using default storage location
    if custom_path.is_none() {
        let sanitized_name = worktree_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(branch);
        storage.store_branch_mapping(&repo_name, branch, sanitized_name)?;
    }

    let config = WorktreeConfig::load_from_repo(&repo_path)?;
    copy_config_files(&repo_path, &worktree_path, &config)?;

    println!("✓ Worktree created successfully!");
    println!("  Branch: {}", branch);
    println!("  Path: {}", worktree_path.display());

    Ok(())
}

pub fn copy_config_files(
    source_path: &Path,
    target_path: &Path,
    config: &WorktreeConfig,
) -> Result<()> {
    println!("Copying configuration files...");

    for pattern in &config.copy_patterns.include {
        if let Some(matches) = find_matching_files(source_path, pattern)? {
            for source_file in matches {
                if should_exclude_file(&source_file, &config.copy_patterns.exclude)? {
                    continue;
                }

                let relative_path = source_file.strip_prefix(source_path)?;
                let target_file = target_path.join(relative_path);

                if let Some(parent) = target_file.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                if source_file.is_file() {
                    std::fs::copy(&source_file, &target_file)
                        .with_context(|| format!("Failed to copy {}", relative_path.display()))?;
                    println!("  Copied: {}", relative_path.display());
                } else if source_file.is_dir() {
                    copy_dir_recursive(&source_file, &target_file)?;
                    println!("  Copied directory: {}", relative_path.display());
                }
            }
        }
    }

    Ok(())
}

fn find_matching_files(base_path: &Path, pattern: &str) -> Result<Option<Vec<std::path::PathBuf>>> {
    let mut matches = Vec::new();

    if pattern.contains('*') {
        for entry in glob::glob(&base_path.join(pattern).to_string_lossy())? {
            matches.push(entry?);
        }
    } else {
        let path = base_path.join(pattern);
        if path.exists() {
            matches.push(path);
        }
    }

    if matches.is_empty() {
        Ok(None)
    } else {
        Ok(Some(matches))
    }
}

fn should_exclude_file(file_path: &Path, exclude_patterns: &[String]) -> Result<bool> {
    let file_str = file_path.to_string_lossy();

    for pattern in exclude_patterns {
        if pattern.contains('*') {
            if glob::Pattern::new(pattern)?.matches(&file_str) {
                return Ok(true);
            }
        } else if file_str.contains(pattern) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target)?;

    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            std::fs::copy(&source_path, &target_path)?;
        }
    }

    Ok(())
}
