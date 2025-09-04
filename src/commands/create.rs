use anyhow::{Context, Result};
use std::path::Path;

use crate::config::WorktreeConfig;
use crate::git::GitRepo;
use crate::storage::WorktreeStorage;

/// Mode for creating worktrees
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CreateMode {
    /// Smart mode: create branch if needed, use existing if present
    Smart,
    /// Force new branch creation (fail if exists)
    NewBranch,
    /// Only use existing branch (fail if doesn't exist)
    ExistingBranch,
}

/// Creates a new worktree for the specified branch
///
/// # Errors
/// Returns an error if:
/// - The current directory is not a git repository
/// - The branch doesn't exist and mode is ExistingBranch
/// - The branch exists and mode is NewBranch
/// - Failed to create the worktree directory
/// - Git operations fail
pub fn create_worktree(branch: &str, mode: CreateMode) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    create_worktree_internal(&git_repo, branch, mode)
}

/// Test version that accepts a mock git repository
///
/// # Errors
/// Returns an error if:
/// - The branch doesn't exist and mode is ExistingBranch
/// - The branch exists and mode is NewBranch
/// - Failed to create the worktree directory
/// - Git operations fail
pub fn create_worktree_with_git(
    git_repo: &dyn crate::traits::GitOperations,
    branch: &str,
    mode: CreateMode,
) -> Result<()> {
    create_worktree_internal(git_repo, branch, mode)
}

fn create_worktree_internal(
    git_repo: &dyn crate::traits::GitOperations,
    branch: &str,
    mode: CreateMode,
) -> Result<()> {
    let repo_path = git_repo.get_repo_path();
    let storage = WorktreeStorage::new()?;
    let repo_name = WorktreeStorage::get_repo_name(&repo_path)?;
    let worktree_path = storage.get_worktree_path(&repo_name, branch);

    // Pre-flight checks
    if worktree_path.exists() {
        anyhow::bail!("Worktree path already exists: {}", worktree_path.display());
    }

    let branch_exists = git_repo.branch_exists(branch)?;

    // Validate based on mode
    match mode {
        CreateMode::NewBranch => {
            if branch_exists {
                anyhow::bail!(
                    "Branch '{}' already exists. Use 'worktree create {}' (without --new-branch) to create a worktree for it",
                    branch,
                    branch
                );
            }
        }
        CreateMode::ExistingBranch => {
            if !branch_exists {
                anyhow::bail!(
                    "Branch '{}' doesn't exist. Use 'worktree create {}' (without --existing-branch) to create it",
                    branch,
                    branch
                );
            }
        }
        CreateMode::Smart => {
            // No validation needed - we'll handle both cases
        }
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

    // Determine if we need to create the branch
    let create_branch = !branch_exists;

    if create_branch {
        println!("Creating new branch: {}", branch);
    } else {
        println!("Using existing branch: {}", branch);
    }

    git_repo.create_worktree(branch, &worktree_path, create_branch)?;

    // Inherit git configuration from parent repository
    println!("Inheriting git configuration from parent repository...");
    if let Err(e) = git_repo.inherit_config(&worktree_path) {
        eprintln!("Warning: Failed to inherit git config: {}", e);
        eprintln!("Worktree will use default git configuration.");
    } else {
        println!("✓ Git configuration inherited successfully");
    }

    // Store branch mapping
    let sanitized_name = worktree_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(branch);
    storage.store_branch_mapping(&repo_name, branch, sanitized_name)?;

    let config = WorktreeConfig::load_from_repo(&repo_path)?;
    copy_config_files(&repo_path, &worktree_path, &config)?;

    // Store origin information for back navigation
    store_origin_info(&storage, &repo_name, branch, &repo_path)?;

    // Mark branch as managed only if we created it in this operation
    if create_branch {
        if let Err(e) = storage.mark_branch_managed(&repo_name, branch) {
            eprintln!("Warning: Failed to mark branch as managed: {}", e);
        }
    }

    println!("✓ Worktree created successfully!");
    println!("  Branch: {}", branch);
    println!("  Path: {}", worktree_path.display());

    Ok(())
}

/// Copies configuration files from source to target based on config patterns
///
/// # Errors
/// Returns an error if:
/// - Failed to read files from source directory
/// - Failed to create target directory
/// - Failed to copy files
/// - Pattern matching fails
pub fn copy_config_files(
    source_path: &Path,
    target_path: &Path,
    config: &WorktreeConfig,
) -> Result<()> {
    println!("Copying configuration files...");

    for pattern in config.copy_patterns.include.as_ref().unwrap_or(&vec![]) {
        if let Some(matches) = find_matching_files(source_path, pattern)? {
            for source_file in matches {
                if should_exclude_file(
                    &source_file,
                    config.copy_patterns.exclude.as_ref().unwrap_or(&vec![]),
                )? {
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

/// Stores the origin repository path in storage metadata for back navigation
///
/// # Errors
/// Returns an error if:
/// - Failed to store the origin mapping
/// - Failed to canonicalize the repository path
fn store_origin_info(
    storage: &WorktreeStorage,
    repo_name: &str,
    branch_name: &str,
    repo_path: &Path,
) -> Result<()> {
    // Store the canonical path to the repository
    let canonical_repo_path = repo_path.canonicalize().with_context(|| {
        format!(
            "Failed to canonicalize repository path: {}",
            repo_path.display()
        )
    })?;

    storage
        .store_worktree_origin(
            repo_name,
            branch_name,
            &canonical_repo_path.to_string_lossy(),
        )
        .context("Failed to store worktree origin information")?;

    Ok(())
}
