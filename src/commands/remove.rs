use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::WorktreeConfig;
use crate::git::GitRepo;
use crate::selection::{RealSelectionProvider, SelectionProvider};
use crate::storage::{WorktreeStorage, read_worktree_head_branch};

/// Top-level directories whose untracked contents are treated as regenerable
/// build/dependency artifacts and never block worktree removal.
const IGNORED_UNTRACKED_DIRS: &[&str] = &[
    ".venv",
    "venv",
    "node_modules",
    "target",
    "dist",
    "build",
    "__pycache__",
    ".next",
    ".tox",
    ".mypy_cache",
    ".ruff_cache",
    ".pre-commit-cache",
];

/// Untracked file names that never block worktree removal (OS/editor noise).
const IGNORED_UNTRACKED_FILES: &[&str] = &[".DS_Store"];

/// Summary of uncommitted work found in a worktree before removal.
struct DirtyReport {
    tracked_changes: Vec<String>,
    untracked_files: Vec<String>,
}

/// Removes a worktree, preserving branches by default
///
/// # Errors
/// Returns an error if the target worktree doesn't exist, storage access fails,
/// git operations fail, the worktree has uncommitted work (use `--force`),
/// or the worktree directory cannot be removed.
pub fn remove_worktree(
    target: Option<&str>,
    delete_branch: bool,
    force: bool,
    interactive: bool,
    list_completions: bool,
    current_repo_only: bool,
) -> Result<()> {
    remove_worktree_with_provider(
        target,
        delete_branch,
        force,
        interactive,
        list_completions,
        current_repo_only,
        &RealSelectionProvider,
    )
}

/// Removes a worktree with a custom selection provider (for testing)
///
/// # Errors
/// Returns an error if the target worktree doesn't exist, storage access fails,
/// git operations fail, the worktree has uncommitted work (use `--force`),
/// or the worktree directory cannot be removed.
pub fn remove_worktree_with_provider(
    target: Option<&str>,
    delete_branch: bool,
    force: bool,
    interactive: bool,
    list_completions: bool,
    current_repo_only: bool,
    provider: &dyn SelectionProvider,
) -> Result<()> {
    let storage = WorktreeStorage::new()?;

    if list_completions {
        list_worktree_completions(&storage, current_repo_only)?;
        return Ok(());
    }

    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let repo_path = git_repo.get_repo_path()?;
    let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

    let (worktree_path, feature_name) = if interactive || target.is_none() {
        select_worktree_for_removal(&storage, current_repo_only, provider)?
    } else if let Some(target_str) = target {
        resolve_target(target_str, &storage, &repo_name)?
    } else {
        anyhow::bail!("No target specified for worktree removal");
    };

    if !worktree_path.exists() {
        anyhow::bail!("Worktree path does not exist: {}", worktree_path.display());
    }

    // Safety: refuse to delete worktrees that may hold uncommitted work.
    if !force {
        let config = WorktreeConfig::load_from_repo(repo_path)?;
        if let Some(report) = check_worktree_dirty(&worktree_path, repo_path, &config)? {
            anyhow::bail!(
                "Refusing to remove worktree '{}': it may contain uncommitted work.\n{}Use --force to delete it anyway.",
                feature_name,
                format_dirty_report(&report)
            );
        }
    }

    // Warn if the caller's shell is inside the worktree being removed
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|dir| dir.canonicalize().ok());
    let worktree_canonical = worktree_path.canonicalize().ok();
    if let (Some(current), Some(worktree)) = (cwd, worktree_canonical) {
        if current.starts_with(&worktree) {
            eprintln!(
                "⚠ Warning: you are inside the worktree being removed; cd out of it first or your shell will be left in a deleted directory."
            );
        }
    }

    println!(
        "Removing worktree '{}': {}",
        feature_name,
        worktree_path.display()
    );

    // Read current branch from worktree HEAD before removing it
    let current_branch = read_worktree_head_branch(&worktree_path);

    // Use the feature name (directory name) as the worktree name for git
    let worktree_name = worktree_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&feature_name);

    // Remove the filesystem directory first
    if worktree_path.exists() {
        fs::remove_dir_all(&worktree_path).context("Failed to remove worktree directory")?;
    }

    git_repo
        .remove_worktree(worktree_name)
        .context("Failed to remove worktree from git")?;

    // Clean up origin information
    if let Err(e) = storage.remove_worktree_origin(&repo_name, &feature_name) {
        println!("⚠ Warning: Failed to clean up origin information: {}", e);
    }

    // Delete branch only when explicitly requested via --delete-branch
    if delete_branch {
        if let Some(branch) = &current_branch {
            println!("Deleting branch: {}", branch);
            match git_repo.delete_branch(branch) {
                Ok(_) => println!("✓ Branch deleted successfully"),
                Err(e) => println!("⚠ Warning: Failed to delete branch: {}", e),
            }
        } else {
            println!("⚠ Warning: Could not determine branch to delete (detached HEAD or error)");
        }
    } else if let Some(branch) = &current_branch {
        println!(
            "Branch '{}' preserved (use --delete-branch to remove it)",
            branch
        );
    }

    println!("✓ Worktree removed successfully!");

    Ok(())
}

/// Checks a worktree for uncommitted work before removal.
///
/// Tracked changes (staged, modified, deleted, renamed) always count as dirty.
/// Untracked files only count when they are NOT explainable by the tool
/// itself: files copied or symlinked per copy/symlink patterns, files that
/// exist in the origin repo at the same relative path, excluded patterns,
/// well-known regenerable build directories, and OS/editor noise files are
/// all safe to delete without a `--force`.
fn check_worktree_dirty(
    worktree_path: &Path,
    origin_repo_path: &Path,
    config: &WorktreeConfig,
) -> Result<Option<DirtyReport>> {
    let repo = git2::Repository::open(worktree_path)?;

    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;

    let mut tracked_changes = Vec::new();
    let mut untracked_files = Vec::new();

    for entry in statuses.iter() {
        let Ok(path) = entry.path() else {
            continue;
        };
        let path = path.to_string();

        if entry.status().is_wt_new() {
            if !is_untracked_noise(&path, origin_repo_path, config) {
                untracked_files.push(path);
            }
        } else {
            // Staged, modified, deleted, renamed, typechanged, or conflicted
            tracked_changes.push(path);
        }
    }

    if tracked_changes.is_empty() && untracked_files.is_empty() {
        Ok(None)
    } else {
        Ok(Some(DirtyReport {
            tracked_changes,
            untracked_files,
        }))
    }
}

/// Returns true when an untracked path is explainable by the tool (or the OS)
/// and should not block removal.
fn is_untracked_noise(rel_path: &str, origin_repo_path: &Path, config: &WorktreeConfig) -> bool {
    // Present in the origin at the same relative path: the tool copied or
    // symlinked it, or it's shared state mirrored from the origin.
    if origin_repo_path.join(rel_path).symlink_metadata().is_ok() {
        return true;
    }

    let basename = rel_path.rsplit('/').next().unwrap_or_default();
    if IGNORED_UNTRACKED_FILES.contains(&basename) {
        return true;
    }

    let first_segment = rel_path.split('/').next().unwrap_or_default();
    if IGNORED_UNTRACKED_DIRS.contains(&first_segment) {
        return true;
    }

    // Managed by the tool's own patterns (copy, symlink, or exclude).
    let copy_includes = config.copy_patterns.include.as_deref().unwrap_or_default();
    let symlink_includes = config
        .symlink_patterns
        .include
        .as_deref()
        .unwrap_or_default();
    let excludes = config.copy_patterns.exclude.as_deref().unwrap_or_default();

    for pattern in copy_includes
        .iter()
        .chain(symlink_includes.iter())
        .chain(excludes.iter())
    {
        if matches_pattern(rel_path, pattern) {
            return true;
        }
    }

    false
}

/// Matches a `/`-separated relative path against a config glob or path
/// pattern. Directory patterns (trailing `/`) match their contents.
fn matches_pattern(rel_path: &str, pattern: &str) -> bool {
    let normalized = pattern.trim_end_matches('/');

    if rel_path == pattern || rel_path == normalized {
        return true;
    }
    if rel_path.starts_with(&format!("{normalized}/")) {
        return true;
    }
    if pattern.contains('*') && glob::Pattern::new(pattern).is_ok_and(|p| p.matches(rel_path)) {
        return true;
    }

    false
}

/// Formats the dirty report for the refusal error message.
fn format_dirty_report(report: &DirtyReport) -> String {
    let mut msg = String::new();

    if !report.tracked_changes.is_empty() {
        msg.push_str(&format!(
            "  - {} modified/staged tracked file(s), e.g.:\n",
            report.tracked_changes.len()
        ));
        for path in report.tracked_changes.iter().take(5) {
            msg.push_str(&format!("      {path}\n"));
        }
    }

    if !report.untracked_files.is_empty() {
        msg.push_str(&format!(
            "  - {} untracked file(s) not managed by this tool, e.g.:\n",
            report.untracked_files.len()
        ));
        for path in report.untracked_files.iter().take(5) {
            msg.push_str(&format!("      {path}\n"));
        }
    }

    msg
}

fn resolve_target(
    target: &str,
    storage: &WorktreeStorage,
    repo_name: &str,
) -> Result<(PathBuf, String)> {
    // Match by feature name (directory name) directly.
    // resolve_managed_path validates the name (rejecting traversal like `..`
    // or absolute paths) and asserts the path stays inside the storage dir.
    let worktree_path = storage.resolve_managed_path(repo_name, target)?;
    if worktree_path.exists() {
        return Ok((worktree_path, target.to_string()));
    }

    // Try partial match against known worktrees
    let known = storage.list_repo_worktrees(repo_name)?;
    let matches: Vec<&String> = known.iter().filter(|name| name.contains(target)).collect();

    match matches.len() {
        0 => anyhow::bail!("No worktree found matching '{}'", target),
        1 => {
            let feature_name = matches[0].clone();
            let path = storage.resolve_managed_path(repo_name, &feature_name)?;
            Ok((path, feature_name))
        }
        _ => {
            eprintln!(
                "Multiple worktrees match '{}'. Please be more specific:",
                target
            );
            for name in &matches {
                eprintln!("  {}", name);
            }
            anyhow::bail!("Ambiguous worktree name '{}'", target);
        }
    }
}

fn list_worktree_completions(storage: &WorktreeStorage, current_repo_only: bool) -> Result<()> {
    let worktrees = get_available_worktrees(storage, current_repo_only)?;

    for (_, feature_name, _) in worktrees {
        println!("{}", feature_name);
    }

    Ok(())
}

fn select_worktree_for_removal(
    storage: &WorktreeStorage,
    current_repo_only: bool,
    provider: &dyn SelectionProvider,
) -> Result<(PathBuf, String)> {
    let worktrees = get_available_worktrees(storage, current_repo_only)?;

    if worktrees.is_empty() {
        anyhow::bail!("No worktrees found");
    }

    // Format: "feature-name (current-branch)  /path"
    let options: Vec<String> = worktrees
        .iter()
        .map(|(repo, feature_name, path)| {
            let branch_info = read_worktree_head_branch(path)
                .map(|b| format!(" ({})", b))
                .unwrap_or_default();
            format!(
                "{}/{}{} ({})",
                repo,
                feature_name,
                branch_info,
                path.display()
            )
        })
        .collect();

    let selection = provider.select("Select worktree to remove:", options.clone())?;

    let index = options
        .iter()
        .position(|o| o == &selection)
        .ok_or_else(|| anyhow::anyhow!("Selected option not found in list"))?;

    let (_, feature_name, path) = &worktrees[index];
    Ok((path.clone(), feature_name.clone()))
}

fn get_available_worktrees(
    storage: &WorktreeStorage,
    current_repo_only: bool,
) -> Result<Vec<(String, String, PathBuf)>> {
    let mut worktrees = Vec::new();

    if current_repo_only {
        let current_dir = std::env::current_dir()?;
        if let Ok(git_repo) = GitRepo::open(&current_dir) {
            let repo_path = git_repo.get_repo_path()?;
            let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

            let repo_worktrees = storage.list_repo_worktrees(&repo_name)?;
            for feature_name in repo_worktrees {
                let worktree_path = storage.get_worktree_path(&repo_name, &feature_name);
                if worktree_path.exists() {
                    worktrees.push((repo_name.clone(), feature_name, worktree_path));
                }
            }
        }
    } else {
        let all_worktrees = storage.list_all_worktrees()?;
        for (repo_name, repo_worktrees) in all_worktrees {
            for feature_name in repo_worktrees {
                let worktree_path = storage.get_worktree_path(&repo_name, &feature_name);
                if worktree_path.exists() {
                    worktrees.push((repo_name.clone(), feature_name, worktree_path));
                }
            }
        }
    }

    Ok(worktrees)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::config::{CopyPatterns, OnCreate, SymlinkPatterns};
    use tempfile::TempDir;

    fn test_config() -> WorktreeConfig {
        WorktreeConfig {
            copy_patterns: CopyPatterns {
                include: Some(vec![".env*".to_string(), ".vscode/".to_string()]),
                exclude: Some(vec!["node_modules/".to_string(), "*.log".to_string()]),
            },
            symlink_patterns: SymlinkPatterns {
                include: Some(vec!["scripts/".to_string()]),
            },
            on_create: OnCreate { commands: None },
        }
    }

    #[test]
    fn test_matches_pattern() {
        assert!(matches_pattern(".env", ".env*"));
        assert!(matches_pattern(".env.local", ".env*"));
        assert!(matches_pattern(".vscode", ".vscode/"));
        assert!(matches_pattern(".vscode/settings.json", ".vscode/"));
        assert!(matches_pattern("node_modules/x/y.js", "node_modules/"));
        assert!(!matches_pattern("src/main.rs", ".env*"));
        assert!(!matches_pattern("env", ".env*"));
    }

    #[test]
    fn test_is_untracked_noise() {
        let tmp = TempDir::new().unwrap();
        let origin = tmp.path().join("origin");
        std::fs::create_dir_all(&origin).unwrap();
        std::fs::write(origin.join(".env"), "x").unwrap();
        std::fs::create_dir_all(origin.join("scripts")).unwrap();

        let config = test_config();

        // Copied from origin (same relative path)
        assert!(is_untracked_noise(".env", &origin, &config));
        // Covered by copy/symlink patterns even though not present in origin
        assert!(is_untracked_noise(".env.extra", &origin, &config));
        assert!(is_untracked_noise("scripts/run.sh", &origin, &config));
        // Covered by exclude patterns
        assert!(is_untracked_noise(
            "node_modules/pkg/index.js",
            &origin,
            &config
        ));
        assert!(is_untracked_noise("debug.log", &origin, &config));
        // Build artifacts by directory name
        assert!(is_untracked_noise("dist/bundle.js", &origin, &config));
        assert!(is_untracked_noise("target/debug/x", &origin, &config));
        // OS noise
        assert!(is_untracked_noise(".DS_Store", &origin, &config));

        // User work: not noise
        assert!(!is_untracked_noise("notes.txt", &origin, &config));
        assert!(!is_untracked_noise("src/new_idea.rs", &origin, &config));
    }
}
