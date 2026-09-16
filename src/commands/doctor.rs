//! Storage and metadata diagnostics.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::commands::shared::paths_match;
use crate::commands::status::detect_config_drift;
use crate::config::WorktreeConfig;
use crate::git::GitRepo;
use crate::storage::{WorktreeOriginEntry, WorktreeStorage};

struct Finding {
    severity: &'static str,
    feature: Option<String>,
    message: String,
}

/// Runs storage diagnostics and optionally repairs origin metadata.
///
/// # Errors
/// Returns an error if storage, repository discovery, or metadata operations
/// fail.
pub fn doctor_worktrees(fix: bool, current_repo_only: bool) -> Result<()> {
    let storage = WorktreeStorage::new()?;
    let selected_repo = if current_repo_only {
        let current_dir = std::env::current_dir()?;
        let repo = GitRepo::open(&current_dir)?;
        let repo_path = repo.get_repo_path()?;
        Some(WorktreeStorage::get_repo_name(repo_path)?)
    } else {
        None
    };

    let mut repo_names = if let Some(repo_name) = selected_repo.clone() {
        vec![repo_name]
    } else {
        storage
            .list_all_worktrees()?
            .into_iter()
            .map(|(repo_name, _)| repo_name)
            .collect()
    };

    if let Some(repo_name) = selected_repo {
        if !repo_names.contains(&repo_name) {
            repo_names.push(repo_name);
        }
    }

    println!("Worktree Doctor");
    println!("{}", "=".repeat(40));
    println!("Storage root: {}", storage.get_root_dir().display());
    println!();

    let mut checked = 0usize;
    let mut findings = Vec::new();
    let mut fixable_count = 0usize;

    for repo_name in repo_names {
        let (repo_findings, repo_checked, repo_fixable) = inspect_repo(&storage, &repo_name, fix)?;
        if repo_checked > 0 || !repo_findings.is_empty() {
            println!("Repo: {repo_name}");
            for finding in &repo_findings {
                let feature = finding
                    .feature
                    .as_deref()
                    .map(|name| format!("{name}: "))
                    .unwrap_or_default();
                let marker = match finding.severity {
                    "error" => "✗",
                    "warning" => "⚠",
                    _ => "ℹ",
                };
                println!("  {marker} {feature}{}", finding.message);
            }
            if repo_findings.is_empty() {
                println!("  ✓ healthy");
            }
            println!();
        }
        checked += repo_checked;
        fixable_count += repo_fixable;
        findings.extend(repo_findings);
    }

    let errors = findings
        .iter()
        .filter(|finding| finding.severity == "error")
        .count();
    let warnings = findings
        .iter()
        .filter(|finding| finding.severity == "warning")
        .count();
    println!(
        "Summary: {checked} checked, {} findings ({errors} errors, {warnings} warnings)",
        findings.len()
    );
    if fixable_count > 0 {
        println!("Fixable with --fix: {fixable_count} (origin metadata only)");
    }
    if findings
        .iter()
        .any(|finding| finding.message.contains("drift"))
    {
        println!("Run `worktree sync` to fix drifted worktrees.");
    }

    Ok(())
}

fn inspect_repo(
    storage: &WorktreeStorage,
    repo_name: &str,
    fix: bool,
) -> Result<(Vec<Finding>, usize, usize)> {
    let mut findings = Vec::new();
    let mut fixable_count = 0usize;
    let worktrees = storage.list_repo_worktrees(repo_name)?;
    let origins = storage.list_worktree_origins(repo_name)?;
    let worktree_set: HashSet<String> = worktrees.iter().cloned().collect();

    let mut seen_features = HashSet::new();
    for entry in &origins {
        match (&entry.feature_name, &entry.origin_path) {
            (Some(feature), Some(_)) if !seen_features.insert(feature.clone()) => {
                findings.push(Finding {
                    severity: "warning",
                    feature: Some(feature.clone()),
                    message: "duplicate origin entry".to_string(),
                });
                fixable_count += 1;
            }
            (None, None) => {
                findings.push(Finding {
                    severity: "warning",
                    feature: None,
                    message: format!("malformed origin entry: {}", entry.raw_line),
                });
                fixable_count += 1;
            }
            _ => {}
        }
    }

    let live_origins: HashSet<PathBuf> = origins
        .iter()
        .filter_map(|entry| entry.origin_path.as_deref())
        .map(Path::new)
        .filter(|path| path.exists())
        .map(|path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
        .collect();
    if live_origins.len() > 1 {
        findings.push(Finding {
            severity: "warning",
            feature: None,
            message: format!(
                "repository-name collision: {} distinct origin repositories share this storage key",
                live_origins.len()
            ),
        });
    }

    let mut git_by_origin: HashMap<PathBuf, Vec<(String, PathBuf, bool)>> = HashMap::new();
    for origin in &live_origins {
        if let Ok(repo) = GitRepo::open(origin) {
            if let Ok(worktrees_for_origin) = repo.list_worktrees_with_paths() {
                git_by_origin.insert(origin.clone(), worktrees_for_origin);
            }
        }
    }

    for feature in &worktrees {
        checked_feature(
            storage,
            repo_name,
            feature,
            &origins,
            &git_by_origin,
            &mut findings,
            &mut fixable_count,
        )?;
    }

    for entry in &origins {
        let Some(feature) = entry.feature_name.as_deref() else {
            continue;
        };
        if !worktree_set.contains(feature) {
            findings.push(Finding {
                severity: "warning",
                feature: Some(feature.to_string()),
                message: "origin entry exists but worktree directory is missing".to_string(),
            });
            fixable_count += 1;
        }
    }

    for worktrees_for_origin in git_by_origin.values() {
        for (name, path, is_prunable) in worktrees_for_origin {
            if *is_prunable {
                findings.push(Finding {
                    severity: "info",
                    feature: Some(name.clone()),
                    message: "git worktree is prunable; run `worktree cleanup`".to_string(),
                });
            }
            let managed_path = worktrees
                .iter()
                .any(|feature| paths_match(path, &storage.get_worktree_path(repo_name, feature)));
            if !managed_path {
                findings.push(Finding {
                    severity: "warning",
                    feature: Some(name.clone()),
                    message: format!("git worktree is unmanaged at {}", path.display()),
                });
            }
        }
    }

    if fix {
        repair_origins(storage, repo_name, &worktrees, &origins)?;
    }

    Ok((findings, worktrees.len(), fixable_count))
}

fn checked_feature(
    storage: &WorktreeStorage,
    repo_name: &str,
    feature: &str,
    origins: &[WorktreeOriginEntry],
    git_by_origin: &HashMap<PathBuf, Vec<(String, PathBuf, bool)>>,
    findings: &mut Vec<Finding>,
    fixable_count: &mut usize,
) -> Result<()> {
    let path = storage.get_worktree_path(repo_name, feature);
    let matching: Vec<&WorktreeOriginEntry> = origins
        .iter()
        .filter(|entry| entry.feature_name.as_deref() == Some(feature))
        .collect();

    let Some(origin_entry) = matching.first() else {
        findings.push(Finding {
            severity: "error",
            feature: Some(feature.to_string()),
            message: "no origin entry (back/sync may fail)".to_string(),
        });
        *fixable_count += 1;
        if GitRepo::open(&path).is_ok() {
            findings.push(Finding {
                severity: "warning",
                feature: Some(feature.to_string()),
                message: "storage directory is not registered with the tool".to_string(),
            });
        }
        return Ok(());
    };

    let Some(origin) = origin_entry.origin_path.as_deref() else {
        return Ok(());
    };
    let origin_path = Path::new(origin);
    if !origin_path.exists() {
        findings.push(Finding {
            severity: "error",
            feature: Some(feature.to_string()),
            message: format!("origin does not exist: {origin}"),
        });
        *fixable_count += 1;
        return Ok(());
    }

    let canonical_origin = origin_path
        .canonicalize()
        .unwrap_or_else(|_| origin_path.to_path_buf());
    let registered = git_by_origin
        .get(&canonical_origin)
        .is_some_and(|worktrees| {
            worktrees
                .iter()
                .any(|(_, git_path, _)| paths_match(git_path, &path))
        });
    if !registered {
        findings.push(Finding {
            severity: "warning",
            feature: Some(feature.to_string()),
            message: "storage directory is not registered with git".to_string(),
        });
    }

    if path.exists() {
        let config = WorktreeConfig::load_from_repo(origin_path)?;
        let drift = detect_config_drift(origin_path, &path, &config)?;
        if !drift.is_empty() {
            findings.push(Finding {
                severity: "info",
                feature: Some(feature.to_string()),
                message: format!(
                    "config drift detected ({} path(s)); run `worktree sync`",
                    drift.len()
                ),
            });
        }
    }

    Ok(())
}

fn repair_origins(
    storage: &WorktreeStorage,
    repo_name: &str,
    worktrees: &[String],
    origins: &[WorktreeOriginEntry],
) -> Result<()> {
    let mut repaired = Vec::new();

    for feature in worktrees {
        let path = storage.get_worktree_path(repo_name, feature);
        let existing = origins
            .iter()
            .find(|entry| entry.feature_name.as_deref() == Some(feature));
        let origin = existing
            .and_then(|entry| entry.origin_path.as_deref())
            .and_then(|origin| {
                let path = Path::new(origin);
                path.exists().then(|| origin.to_string())
            })
            .or_else(|| derive_origin(&path));

        if let Some(origin) = origin {
            repaired.push((feature.clone(), origin));
        }
    }

    // Only well-formed entries for live storage directories survive an
    // explicit repair. Collisions and filesystem contents are untouched.
    storage.rewrite_worktree_origins(repo_name, &repaired)
}

fn derive_origin(worktree_path: &Path) -> Option<String> {
    let repo = GitRepo::open(worktree_path).ok()?;
    let origin = repo.get_repo_path().ok()?.canonicalize().ok()?;
    Some(origin.to_string_lossy().to_string())
}
