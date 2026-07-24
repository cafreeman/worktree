use anyhow::Result;
use std::path::Path;

use crate::commands::create;
use crate::config::WorktreeConfig;
use crate::git::GitRepo;
use crate::storage::WorktreeStorage;

/// A single piece of config drift detected for a managed worktree.
struct DriftEntry {
    relative_path: String,
    kind: DriftKind,
}

enum DriftKind {
    MissingSymlink,
    CopyInsteadOfSymlink,
    MissingCopy,
}

impl DriftKind {
    fn label(&self) -> &'static str {
        match self {
            DriftKind::MissingSymlink => "not linked (expected symlink)",
            DriftKind::CopyInsteadOfSymlink => "copy exists, should be symlink",
            DriftKind::MissingCopy => "missing (copy-pattern)",
        }
    }
}

/// Shows the status of all worktrees in the current repository
///
/// # Errors
/// Returns an error if:
/// - Not in a git repository
/// - Failed to access storage system
/// - Git operations fail
pub fn show_status() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let repo_path = git_repo.get_repo_path();

    let storage = WorktreeStorage::new()?;
    let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

    println!("Git Worktree Status");
    println!("{}", "=".repeat(40));
    println!("Repository: {}", repo_name);
    println!("Repository path: {}", repo_path.display());
    println!();

    let git_worktrees = git_repo.list_worktrees()?;
    let managed_worktrees = storage.list_repo_worktrees(&repo_name)?;

    let config = WorktreeConfig::load_from_repo(repo_path)?;

    println!("Git worktrees ({}):", git_worktrees.len());
    for worktree in &git_worktrees {
        let worktree_path = storage.get_worktree_path(&repo_name, worktree);
        let managed = if managed_worktrees.contains(worktree) {
            "📁"
        } else {
            "⚠"
        };
        let exists = if worktree_path.exists() { "✓" } else { "✗" };

        println!(
            "  {} {} {} ({})",
            managed,
            exists,
            worktree,
            worktree_path.display()
        );
    }

    println!();
    println!("Managed worktrees ({}):", managed_worktrees.len());
    let mut any_drift = false;
    for worktree in &managed_worktrees {
        let worktree_path = storage.get_worktree_path(&repo_name, worktree);
        let in_git = if git_worktrees.contains(worktree) {
            "🔗"
        } else {
            "⚠"
        };
        let exists = if worktree_path.exists() { "✓" } else { "✗" };

        println!(
            "  {} {} {} ({})",
            in_git,
            exists,
            worktree,
            worktree_path.display()
        );

        if worktree_path.exists() {
            let drift = detect_config_drift(repo_path, &worktree_path, &config)?;
            for entry in &drift {
                println!("      ⚠ {}   {}", entry.relative_path, entry.kind.label());
            }
            if !drift.is_empty() {
                any_drift = true;
            }
        }
    }

    println!();
    println!("Legend:");
    println!("  📁 = Managed by this tool");
    println!("  🔗 = Registered in git");
    println!("  ✓ = Directory exists");
    println!("  ✗ = Directory missing");
    println!("  ⚠ = Inconsistent state");

    if any_drift {
        println!();
        println!("Run `worktree sync` to fix drifted worktrees.");
    }

    Ok(())
}

/// Detects config drift for a single worktree against the origin repo's config patterns.
///
/// # Errors
/// Returns an error if pattern matching against the filesystem fails.
fn detect_config_drift(
    repo_path: &Path,
    worktree_path: &Path,
    config: &WorktreeConfig,
) -> Result<Vec<DriftEntry>> {
    let mut drift = Vec::new();

    let symlink_patterns = config.symlink_patterns.include.as_deref().unwrap_or(&[]);
    for pattern in symlink_patterns {
        let Some(matches) = create::find_matching_files(repo_path, pattern)? else {
            continue;
        };

        for source_file in matches {
            let Ok(relative_path) = source_file.strip_prefix(repo_path) else {
                continue;
            };
            let target = worktree_path.join(relative_path);

            match target.symlink_metadata() {
                Ok(meta) if meta.file_type().is_symlink() => {
                    // Correctly symlinked — not drift.
                }
                Ok(_) => {
                    drift.push(DriftEntry {
                        relative_path: relative_path.display().to_string(),
                        kind: DriftKind::CopyInsteadOfSymlink,
                    });
                }
                Err(_) => {
                    drift.push(DriftEntry {
                        relative_path: relative_path.display().to_string(),
                        kind: DriftKind::MissingSymlink,
                    });
                }
            }
        }
    }

    let copy_patterns = config.copy_patterns.include.as_deref().unwrap_or_default();
    let exclude_patterns = config.copy_patterns.exclude.as_deref().unwrap_or_default();
    for pattern in copy_patterns {
        let Some(matches) = create::find_matching_files(repo_path, pattern)? else {
            continue;
        };

        for source_file in matches {
            if create::should_exclude_file(&source_file, exclude_patterns)? {
                continue;
            }
            if create::is_covered_by_symlink_pattern(&source_file, repo_path, symlink_patterns) {
                continue;
            }

            let Ok(relative_path) = source_file.strip_prefix(repo_path) else {
                continue;
            };
            let target = worktree_path.join(relative_path);

            if target.symlink_metadata().is_err() {
                drift.push(DriftEntry {
                    relative_path: relative_path.display().to_string(),
                    kind: DriftKind::MissingCopy,
                });
            }
        }
    }

    Ok(drift)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::config::{CopyPatterns, OnCreate, SymlinkPatterns};
    use std::fs;
    use tempfile::TempDir;

    fn config_with(symlink_include: Vec<String>, copy_include: Vec<String>) -> WorktreeConfig {
        WorktreeConfig {
            copy_patterns: CopyPatterns {
                include: Some(copy_include),
                exclude: Some(vec![]),
            },
            symlink_patterns: SymlinkPatterns {
                include: Some(symlink_include),
            },
            on_create: OnCreate { commands: None },
        }
    }

    #[test]
    fn test_missing_symlink_is_flagged() {
        let tmp = TempDir::new().unwrap();
        let origin = tmp.path().join("origin");
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&origin).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        fs::write(origin.join("shared.md"), "content").unwrap();

        let config = config_with(vec!["shared.md".to_string()], vec![]);
        let drift = detect_config_drift(&origin, &worktree, &config).unwrap();

        assert_eq!(drift.len(), 1);
        assert!(matches!(drift[0].kind, DriftKind::MissingSymlink));
    }

    #[test]
    fn test_plain_copy_where_symlink_expected_is_flagged() {
        let tmp = TempDir::new().unwrap();
        let origin = tmp.path().join("origin");
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&origin).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        fs::write(origin.join("shared.md"), "origin content").unwrap();
        fs::write(worktree.join("shared.md"), "stale copy").unwrap();

        let config = config_with(vec!["shared.md".to_string()], vec![]);
        let drift = detect_config_drift(&origin, &worktree, &config).unwrap();

        assert_eq!(drift.len(), 1);
        assert!(matches!(drift[0].kind, DriftKind::CopyInsteadOfSymlink));
    }

    #[test]
    fn test_correctly_symlinked_path_is_not_flagged() {
        let tmp = TempDir::new().unwrap();
        let origin = tmp.path().join("origin");
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&origin).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        fs::write(origin.join("shared.md"), "content").unwrap();
        std::os::unix::fs::symlink(origin.join("shared.md"), worktree.join("shared.md")).unwrap();

        let config = config_with(vec!["shared.md".to_string()], vec![]);
        let drift = detect_config_drift(&origin, &worktree, &config).unwrap();

        assert!(drift.is_empty());
    }

    #[test]
    fn test_missing_copy_is_flagged() {
        let tmp = TempDir::new().unwrap();
        let origin = tmp.path().join("origin");
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&origin).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        fs::write(origin.join(".env"), "ORIGIN=1").unwrap();

        let config = config_with(vec![], vec![".env".to_string()]);
        let drift = detect_config_drift(&origin, &worktree, &config).unwrap();

        assert_eq!(drift.len(), 1);
        assert!(matches!(drift[0].kind, DriftKind::MissingCopy));
    }

    #[test]
    fn test_differing_content_copy_is_not_flagged() {
        let tmp = TempDir::new().unwrap();
        let origin = tmp.path().join("origin");
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&origin).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        fs::write(origin.join(".env"), "ORIGIN=1").unwrap();
        fs::write(worktree.join(".env"), "LOCAL=2").unwrap();

        let config = config_with(vec![], vec![".env".to_string()]);
        let drift = detect_config_drift(&origin, &worktree, &config).unwrap();

        assert!(drift.is_empty());
    }
}
