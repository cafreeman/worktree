use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use crate::commands::create;
use crate::commands::shared::paths_match;
use crate::config::WorktreeConfig;
use crate::git::GitRepo;
use crate::storage::WorktreeStorage;

/// A single piece of config drift detected for a managed worktree.
pub(crate) struct DriftEntry {
    pub(crate) relative_path: String,
    pub(crate) kind: DriftKind,
}

pub(crate) enum DriftKind {
    MissingSymlink,
    CopyInsteadOfSymlink,
    MissingCopy,
}

impl DriftKind {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            DriftKind::MissingSymlink => "not linked (expected symlink)",
            DriftKind::CopyInsteadOfSymlink => "copy exists, should be symlink",
            DriftKind::MissingCopy => "missing (copy-pattern)",
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct StatusOutput {
    pub(crate) repository: String,
    pub(crate) repository_path: String,
    pub(crate) git_worktrees: Vec<GitWorktreeOutput>,
    pub(crate) managed_worktrees: Vec<ManagedWorktreeOutput>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GitWorktreeOutput {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) exists: bool,
    pub(crate) managed: bool,
    pub(crate) prunable: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ManagedWorktreeOutput {
    pub(crate) feature: String,
    pub(crate) path: String,
    pub(crate) exists: bool,
    pub(crate) registered: bool,
    pub(crate) drift: Vec<String>,
}

/// Shows the status of all worktrees in the current repository
///
/// # Errors
/// Returns an error if:
/// - Not in a git repository
/// - Failed to access storage system
/// - Git operations fail
pub fn show_status(json: bool) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let repo_path = git_repo.get_repo_path()?;

    let storage = WorktreeStorage::new()?;
    let repo_name = WorktreeStorage::get_repo_name(repo_path)?;

    let git_worktrees = git_repo.list_worktrees_with_paths()?;
    let managed_worktrees = storage.list_repo_worktrees(&repo_name)?;

    let config = WorktreeConfig::load_from_repo(repo_path)?;

    if json {
        let output = build_status_output(
            &repo_name,
            repo_path,
            &git_worktrees,
            &managed_worktrees,
            &config,
            &storage,
        )?;
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("Git Worktree Status");
    println!("{}", "=".repeat(40));
    println!("Repository: {}", repo_name);
    println!("Repository path: {}", repo_path.display());
    println!();

    println!("Git worktrees ({}):", git_worktrees.len());
    for (worktree, worktree_path, _) in &git_worktrees {
        let expected_path = storage.get_worktree_path(&repo_name, worktree);
        let managed =
            if managed_worktrees.contains(worktree) && paths_match(worktree_path, &expected_path) {
                "📁"
            } else {
                "⚠"
            };
        let exists = if worktree_path.exists() { "✓" } else { "✗" };
        let name = if managed == "📁" {
            worktree.clone()
        } else {
            format!("{worktree} (unmanaged)")
        };

        println!(
            "  {} {} {} ({})",
            managed,
            exists,
            name,
            worktree_path.display()
        );
    }

    println!();
    println!("Managed worktrees ({}):", managed_worktrees.len());
    let mut any_drift = false;
    for worktree in &managed_worktrees {
        let worktree_path = storage.get_worktree_path(&repo_name, worktree);
        let in_git = if git_worktrees
            .iter()
            .any(|(name, path, _)| name == worktree && paths_match(path, &worktree_path))
        {
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
pub(crate) fn detect_config_drift(
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
            if create::should_exclude_file(&source_file, repo_path, exclude_patterns)? {
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

fn build_status_output(
    repo_name: &str,
    repo_path: &Path,
    git_worktrees: &[(String, std::path::PathBuf, bool)],
    managed_worktrees: &[String],
    config: &WorktreeConfig,
    storage: &WorktreeStorage,
) -> Result<StatusOutput> {
    let git_output = git_worktrees
        .iter()
        .map(|(name, path, prunable)| GitWorktreeOutput {
            name: name.clone(),
            path: path.display().to_string(),
            exists: path.exists(),
            managed: managed_worktrees.contains(name)
                && paths_match(path, &storage.get_worktree_path(repo_name, name)),
            prunable: *prunable,
        })
        .collect();

    let mut managed_output = Vec::new();
    for feature in managed_worktrees {
        let path = storage.get_worktree_path(repo_name, feature);
        let drift = if path.exists() {
            detect_config_drift(repo_path, &path, config)?
                .into_iter()
                .map(|entry| format!("{}: {}", entry.relative_path, entry.kind.label()))
                .collect()
        } else {
            Vec::new()
        };
        managed_output.push(ManagedWorktreeOutput {
            feature: feature.clone(),
            path: path.display().to_string(),
            exists: path.exists(),
            registered: git_worktrees
                .iter()
                .any(|(name, git_path, _)| name == feature && paths_match(git_path, &path)),
            drift,
        });
    }

    Ok(StatusOutput {
        repository: repo_name.to_string(),
        repository_path: repo_path.display().to_string(),
        git_worktrees: git_output,
        managed_worktrees: managed_output,
    })
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
