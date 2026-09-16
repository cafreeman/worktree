use anyhow::{Context, Result};
use inquire::validator::Validation;
use std::error::Error;
use std::path::Path;

use crate::config::WorktreeConfig;
use crate::git::GitRepo;
use crate::selection::{
    RealSelectionProvider, SelectionProvider, select_git_reference_interactive,
};
use crate::storage::WorktreeStorage;

/// Creates a new worktree for the specified feature
///
/// # Errors
/// Returns an error if:
/// - The current directory is not a git repository
/// - The feature name is invalid
/// - The worktree path already exists
/// - Git operations fail
pub fn create_worktree(feature_name: &str, branch: Option<&str>, from: Option<&str>) -> Result<()> {
    create_worktree_with_links(feature_name, branch, from, &[])
}

/// Creates a worktree with one-off symlink patterns in addition to repository
/// configuration.
///
/// # Errors
/// Returns an error if worktree creation or link setup fails.
pub fn create_worktree_with_links(
    feature_name: &str,
    branch: Option<&str>,
    from: Option<&str>,
    link_patterns: &[String],
) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    create_worktree_internal(&git_repo, feature_name, branch, from, link_patterns)
}

fn create_worktree_internal(
    git_repo: &dyn crate::traits::GitOperations,
    feature_name: &str,
    branch: Option<&str>,
    from: Option<&str>,
    link_patterns: &[String],
) -> Result<()> {
    // Validate feature name
    WorktreeStorage::validate_feature_name(feature_name)?;

    let branch_name = branch.unwrap_or(feature_name);

    let repo_path = git_repo.get_repo_path()?;
    let storage = WorktreeStorage::new()?;
    let repo_name = WorktreeStorage::get_repo_name(&repo_path)?;
    let worktree_path = storage.get_worktree_path(&repo_name, feature_name);

    // Pre-flight check
    if worktree_path.exists() {
        anyhow::bail!(
            "Worktree '{}' already exists at: {}",
            feature_name,
            worktree_path.display()
        );
    }

    let branch_exists = git_repo.branch_exists(branch_name)?;

    // Ensure parent directory exists
    if let Some(parent) = worktree_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;
    }

    println!(
        "Creating worktree '{}' at: {}",
        feature_name,
        worktree_path.display()
    );

    let create_branch = !branch_exists;

    if create_branch {
        println!("Creating new branch: {}", branch_name);
    } else {
        println!("Using existing branch: {}", branch_name);
    }

    git_repo.create_worktree_from(branch_name, &worktree_path, create_branch, from)?;

    let config = WorktreeConfig::load_from_repo(&repo_path)?;

    // Create symlinks first (takes precedence over copy)
    create_symlinks_with_patterns(&repo_path, &worktree_path, &config, false, link_patterns)?;

    // Copy config files, skipping any that are covered by symlinks
    copy_config_files_with_symlinks(&repo_path, &worktree_path, &config, link_patterns)?;

    // Store origin information for back navigation
    store_origin_info(&storage, &repo_name, feature_name, &repo_path)?;

    // Run post-create hooks
    run_on_create_hooks(&worktree_path, &config)?;

    println!("✓ Worktree created successfully!");
    println!("  Feature: {}", feature_name);
    println!("  Branch: {}", branch_name);
    println!("  Path: {}", worktree_path.display());

    Ok(())
}

/// Creates symlinks in the worktree for patterns listed in `[symlink-patterns]`.
/// Symlinks point to the absolute path in the origin repo.
///
/// When `force` is `false` (worktree creation), any existing path at the target
/// location is left untouched. When `force` is `true` (used by `sync`), a path
/// that already exists as a plain file/dir (not a symlink) is replaced with the
/// symlink; a path that is already a symlink is left untouched regardless of its
/// target.
///
/// # Errors
/// Returns an error if symlink creation fails for reasons other than missing origin path.
pub fn create_symlinks(
    source_path: &Path,
    target_path: &Path,
    config: &WorktreeConfig,
    force: bool,
) -> Result<()> {
    create_symlinks_with_patterns(source_path, target_path, config, force, &[])
}

/// Creates configured and one-off symlinks in a worktree.
///
/// # Errors
/// Returns an error if symlink creation fails for reasons other than missing origin path.
pub fn create_symlinks_with_patterns(
    source_path: &Path,
    target_path: &Path,
    config: &WorktreeConfig,
    force: bool,
    extra_patterns: &[String],
) -> Result<()> {
    let configured_patterns = config.symlink_patterns.include.as_deref().unwrap_or(&[]);
    if configured_patterns.is_empty() && extra_patterns.is_empty() {
        return Ok(());
    }
    let patterns: Vec<&str> = configured_patterns
        .iter()
        .map(String::as_str)
        .chain(extra_patterns.iter().map(String::as_str))
        .collect();

    println!("Creating symlinks...");

    for pattern in patterns {
        if let Some(matches) = find_matching_files(source_path, pattern)? {
            for source_file in matches {
                let relative_path = source_file.strip_prefix(source_path)?;
                let target_link = target_path.join(relative_path);

                // Canonicalize the origin path for the symlink target
                let canonical_source = source_file.canonicalize().with_context(|| {
                    format!(
                        "Failed to canonicalize symlink source: {}",
                        source_file.display()
                    )
                })?;

                // Create parent dir if needed
                if let Some(parent) = target_link.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                if let Ok(existing_meta) = target_link.symlink_metadata() {
                    // Already a symlink: leave it alone regardless of where it points.
                    if existing_meta.file_type().is_symlink() {
                        continue;
                    }

                    // A plain file/dir already exists. Without force, skip (e.g. already copied).
                    if !force {
                        continue;
                    }

                    // With force, replace the existing plain file/dir with the symlink.
                    if existing_meta.is_dir() {
                        std::fs::remove_dir_all(&target_link).with_context(|| {
                            format!(
                                "Failed to remove existing directory before relinking: {}",
                                target_link.display()
                            )
                        })?;
                    } else {
                        std::fs::remove_file(&target_link).with_context(|| {
                            format!(
                                "Failed to remove existing file before relinking: {}",
                                target_link.display()
                            )
                        })?;
                    }
                }

                std::os::unix::fs::symlink(&canonical_source, &target_link).with_context(|| {
                    format!(
                        "Failed to create symlink {} -> {}",
                        target_link.display(),
                        canonical_source.display()
                    )
                })?;

                println!(
                    "  Symlinked: {} -> {}",
                    relative_path.display(),
                    canonical_source.display()
                );
            }
        } else {
            eprintln!(
                "Warning: Symlink pattern '{}' did not match any files in origin repo — skipping",
                pattern
            );
        }
    }

    Ok(())
}

/// Copies configuration files from source to target based on config patterns,
/// skipping any paths that are covered by symlink patterns.
///
/// # Errors
/// Returns an error if file operations fail.
pub fn copy_config_files(
    source_path: &Path,
    target_path: &Path,
    config: &WorktreeConfig,
) -> Result<()> {
    copy_config_files_with_symlinks(source_path, target_path, config, &[])
}

/// Copies configuration files while treating one-off symlink patterns as
/// higher precedence than copy patterns.
///
/// # Errors
/// Returns an error if file operations fail.
pub fn copy_config_files_with_symlinks(
    source_path: &Path,
    target_path: &Path,
    config: &WorktreeConfig,
    extra_symlink_patterns: &[String],
) -> Result<()> {
    println!("Copying configuration files...");

    let mut symlink_patterns = config.symlink_patterns.include.clone().unwrap_or_default();
    symlink_patterns.extend(extra_symlink_patterns.iter().cloned());

    for pattern in config.copy_patterns.include.as_deref().unwrap_or_default() {
        if let Some(matches) = find_matching_files(source_path, pattern)? {
            for source_file in matches {
                if should_exclude_file(
                    &source_file,
                    source_path,
                    config.copy_patterns.exclude.as_deref().unwrap_or_default(),
                )? {
                    continue;
                }

                // Skip if already covered by a symlink pattern
                if is_covered_by_symlink_pattern(&source_file, source_path, &symlink_patterns) {
                    continue;
                }

                let relative_path = source_file.strip_prefix(source_path)?;
                let target_file = target_path.join(relative_path);

                // Skip if a symlink already exists at the target (defer to create_symlinks)
                if target_file
                    .symlink_metadata()
                    .is_ok_and(|m| m.file_type().is_symlink())
                {
                    continue;
                }

                if let Some(parent) = target_file.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                if source_file.is_file() {
                    std::fs::copy(&source_file, &target_file)
                        .with_context(|| format!("Failed to copy {}", relative_path.display()))?;
                    println!("  Copied: {}", relative_path.display());
                } else if source_file.is_dir() {
                    copy_dir_recursive(
                        &source_file,
                        &target_file,
                        source_path,
                        config.copy_patterns.exclude.as_deref().unwrap_or_default(),
                        &symlink_patterns,
                    )?;
                    println!("  Copied directory: {}", relative_path.display());
                }
            }
        }
    }

    Ok(())
}

/// Checks if a file path is covered by any symlink pattern
pub(crate) fn is_covered_by_symlink_pattern(
    file_path: &Path,
    base_path: &Path,
    symlink_patterns: &[String],
) -> bool {
    if symlink_patterns.is_empty() {
        return false;
    }

    let Ok(relative) = file_path.strip_prefix(base_path) else {
        return false;
    };

    let rel_str = relative.to_string_lossy();

    for pattern in symlink_patterns {
        // Check if the relative path starts with the pattern (prefix match for dirs)
        let normalized_pattern = pattern.trim_end_matches('/');
        if rel_str == *pattern
            || rel_str == normalized_pattern
            || rel_str.starts_with(&format!("{}/", normalized_pattern))
        {
            return true;
        }

        // Glob match
        if pattern.contains('*') {
            if let Ok(p) = glob::Pattern::new(pattern) {
                if p.matches(&rel_str) {
                    return true;
                }
            }
        }
    }

    false
}

/// Runs post-create hooks defined in `[on-create] commands`.
/// On first failure, remaining commands are skipped and a warning is printed.
/// The worktree remains created regardless.
///
/// # Errors
/// Never returns Err — hook failures are warnings, not errors.
#[allow(clippy::unnecessary_wraps)]
pub fn run_on_create_hooks(worktree_path: &Path, config: &WorktreeConfig) -> Result<()> {
    let commands = match config.on_create.commands.as_deref() {
        Some(c) if !c.is_empty() => c,
        _ => return Ok(()),
    };

    println!("Running post-create hooks...");

    for cmd_str in commands {
        println!("  Running: {}", cmd_str);

        let status = std::process::Command::new("sh")
            .args(["-c", cmd_str.as_str()])
            .current_dir(worktree_path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("  ✓ Done: {}", cmd_str);
            }
            Ok(s) => {
                eprintln!(
                    "⚠ Warning: Hook command failed with exit code {}: {}",
                    s.code().unwrap_or(-1),
                    cmd_str
                );
                eprintln!("  Remaining post-create commands skipped.");
                break;
            }
            Err(e) => {
                eprintln!("⚠ Warning: Failed to run hook command '{}': {}", cmd_str, e);
                eprintln!("  Remaining post-create commands skipped.");
                break;
            }
        }
    }

    Ok(())
}

pub(crate) fn find_matching_files(
    base_path: &Path,
    pattern: &str,
) -> Result<Option<Vec<std::path::PathBuf>>> {
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

/// Checks whether a file path is excluded by any exclude pattern.
///
/// Matching is done against the path RELATIVE to the source root; the old
/// substring match against the absolute path made a repo checked out under
/// e.g. `~/dev/target/myrepo` exclude everything. Glob patterns match the
/// relative path; plain patterns match the exact relative path, a
/// directory prefix, or any single path segment.
pub(crate) fn should_exclude_file(
    file_path: &Path,
    source_root: &Path,
    exclude_patterns: &[String],
) -> Result<bool> {
    let Ok(relative) = file_path.strip_prefix(source_root) else {
        return Ok(false);
    };
    let rel_str = relative.to_string_lossy();

    for pattern in exclude_patterns {
        if pattern.contains('*') {
            if glob::Pattern::new(pattern)?.matches(&rel_str) {
                return Ok(true);
            }
        } else if plain_pattern_matches(&rel_str, pattern) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Matches a relative path against a plain (non-glob) pattern: exact path,
/// directory prefix (`target` covers `target/anything`), or any single
/// path segment equal to the pattern.
fn plain_pattern_matches(rel: &str, pattern: &str) -> bool {
    let normalized = pattern.trim_end_matches('/');

    if rel == normalized {
        return true;
    }
    if rel.starts_with(&format!("{normalized}/")) {
        return true;
    }
    rel.split('/').any(|segment| segment == normalized)
}

fn copy_dir_recursive(
    source: &Path,
    target: &Path,
    source_root: &Path,
    exclude_patterns: &[String],
    symlink_patterns: &[String],
) -> Result<()> {
    std::fs::create_dir_all(target)?;

    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if should_exclude_file(&source_path, source_root, exclude_patterns)?
            || is_covered_by_symlink_pattern(&source_path, source_root, symlink_patterns)
            || target_path
                .symlink_metadata()
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            continue;
        }

        if source_path.is_dir() {
            copy_dir_recursive(
                &source_path,
                &target_path,
                source_root,
                exclude_patterns,
                symlink_patterns,
            )?;
        } else {
            std::fs::copy(&source_path, &target_path)?;
        }
    }

    Ok(())
}

/// Stores the origin repository path in storage metadata for back navigation
///
/// # Errors
/// Returns an error if storing origin information fails.
fn store_origin_info(
    storage: &WorktreeStorage,
    repo_name: &str,
    feature_name: &str,
    repo_path: &Path,
) -> Result<()> {
    let canonical_repo_path = repo_path.canonicalize().with_context(|| {
        format!(
            "Failed to canonicalize repository path: {}",
            repo_path.display()
        )
    })?;

    storage
        .store_worktree_origin(
            repo_name,
            feature_name,
            &canonical_repo_path.to_string_lossy(),
        )
        .context("Failed to store worktree origin information")?;

    Ok(())
}

/// Lists all git references (branches and tags) for shell completion
///
/// # Errors
/// Returns an error if git operations fail.
pub fn list_git_ref_completions() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;

    let local_branches = git_repo
        .list_local_branches()
        .context("Failed to list local branches")?;
    let remote_branches = git_repo
        .list_remote_branches()
        .context("Failed to list remote branches")?;
    let tags = git_repo.list_tags().context("Failed to list tags")?;

    for branch in local_branches {
        println!("{}", branch);
    }
    for branch in remote_branches {
        println!("{}", branch);
    }
    for tag in tags {
        println!("{}", tag);
    }

    Ok(())
}

/// Handle interactive selection for --from flag
///
/// # Errors
/// Returns an error if interactive selection fails.
pub fn interactive_from_selection(feature_name: &str, branch: Option<&str>) -> Result<()> {
    interactive_from_selection_with_links(feature_name, branch, &[])
}

/// Selects a starting reference and creates a worktree with extra symlink patterns.
///
/// # Errors
/// Returns an error if interactive selection or worktree creation fails.
pub fn interactive_from_selection_with_links(
    feature_name: &str,
    branch: Option<&str>,
    link_patterns: &[String],
) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;

    let provider = RealSelectionProvider;
    let selected_ref = select_git_reference_interactive(&git_repo, &provider)?;

    create_worktree_with_links(feature_name, branch, Some(&selected_ref), link_patterns)?;

    Ok(())
}

/// Feature name validator for interactive input
#[must_use]
pub fn validate_feature_name_internal(input: &str) -> Validation {
    match WorktreeStorage::validate_feature_name(input) {
        Ok(()) => Validation::Valid,
        Err(e) => Validation::Invalid(e.to_string().into()),
    }
}

/// Wrapper for inquire validator
///
/// # Errors
/// Returns an error if input is too long for system validation.
pub fn validate_feature_name(input: &str) -> Result<Validation, Box<dyn Error + Send + Sync>> {
    if input.len() > 1000 {
        return Err("Feature name is too long for system validation".into());
    }
    Ok(validate_feature_name_internal(input))
}

/// Branch name validator - checks for valid branch name format
#[must_use]
pub fn validate_branch_name_internal(input: &str) -> Validation {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Validation::Invalid("Branch name cannot be empty".into());
    }

    if trimmed == "@" || trimmed.starts_with('-') {
        return Validation::Invalid(
            "Branch name is reserved or would be parsed as an option".into(),
        );
    }

    if input.contains("..")
        || input.contains("//")
        || input.contains("@{")
        || input.starts_with('/')
        || input.ends_with('/')
        || input.contains(' ')
        || input.contains('~')
        || input.contains('^')
        || input.contains(':')
        || input.contains('?')
        || input.contains('*')
        || input.contains('[')
        || input.contains('\\')
    {
        return Validation::Invalid("Branch name contains invalid characters".into());
    }

    if input.ends_with(".lock") {
        return Validation::Invalid("Branch name cannot end with '.lock'".into());
    }

    if input.ends_with('.') {
        return Validation::Invalid("Branch name cannot end with '.'".into());
    }

    Validation::Valid
}

/// Wrapper for inquire validator
///
/// # Errors
/// Returns an error if the input is somehow malformed.
pub fn validate_branch_name(input: &str) -> Result<Validation, Box<dyn Error + Send + Sync>> {
    if input.len() > 1000 {
        return Err("Branch name is too long for system validation".into());
    }
    Ok(validate_branch_name_internal(input))
}

/// Handle the full interactive create workflow (no args provided)
///
/// # Errors
/// Returns an error if interactive prompts fail or worktree creation fails.
pub fn interactive_create_workflow() -> Result<()> {
    let provider = RealSelectionProvider;

    // Step 1: Get feature name
    let feature_name = provider.get_text_input(
        "Feature name (used as the worktree directory name):",
        Some(validate_feature_name),
    )?;

    // Step 2: Get starting branch name
    let branch_name =
        provider.get_text_input("Starting branch name:", Some(validate_branch_name))?;

    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let branch_exists = git_repo.branch_exists(&branch_name)?;

    // Step 3: If branch is new, optionally get a base ref
    let from_ref = if !branch_exists {
        let selected_ref = select_git_reference_interactive(&git_repo, &provider)?;
        Some(selected_ref)
    } else {
        None
    };

    create_worktree(&feature_name, Some(&branch_name), from_ref.as_deref())?;

    Ok(())
}

/// Interactive workflow when feature name is known but branch is not provided
///
/// # Errors
/// Returns an error if interactive prompts fail or worktree creation fails.
pub fn interactive_create_with_feature(feature_name: &str) -> Result<()> {
    interactive_create_with_feature_and_links(feature_name, &[])
}

/// Runs the feature-known interactive workflow with one-off symlink patterns.
///
/// # Errors
/// Returns an error if interactive prompts or worktree creation fail.
pub fn interactive_create_with_feature_and_links(
    feature_name: &str,
    link_patterns: &[String],
) -> Result<()> {
    let provider = RealSelectionProvider;

    // Validate feature name first
    WorktreeStorage::validate_feature_name(feature_name)?;

    // Step 1: Get starting branch name
    let branch_name =
        provider.get_text_input("Starting branch name:", Some(validate_branch_name))?;

    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let branch_exists = git_repo.branch_exists(&branch_name)?;

    // Step 2: If branch is new, get a base ref
    let from_ref = if !branch_exists {
        let selected_ref = select_git_reference_interactive(&git_repo, &provider)?;
        Some(selected_ref)
    } else {
        None
    };

    create_worktree_with_links(
        feature_name,
        Some(&branch_name),
        from_ref.as_deref(),
        link_patterns,
    )?;

    Ok(())
}

/// Interactive workflow when the feature name and `--from` reference are known
/// but the branch is not: prompt for the branch and create it from the given
/// starting point (no reference picker needed). The `--from` reference only
/// applies when the branch is new; an existing branch is reused as-is.
///
/// # Errors
/// Returns an error if interactive prompts fail or worktree creation fails.
pub fn interactive_create_with_feature_and_from(feature_name: &str, from_ref: &str) -> Result<()> {
    interactive_create_with_feature_and_from_and_links(feature_name, from_ref, &[])
}

/// Prompts for a branch and creates it from a reference with extra symlinks.
///
/// # Errors
/// Returns an error if the prompt or worktree creation fails.
pub fn interactive_create_with_feature_and_from_and_links(
    feature_name: &str,
    from_ref: &str,
    link_patterns: &[String],
) -> Result<()> {
    let provider = RealSelectionProvider;

    WorktreeStorage::validate_feature_name(feature_name)?;

    let prompt = format!(
        "Starting branch name (new branches start from '{}'):",
        from_ref
    );
    let branch_name = provider.get_text_input(&prompt, Some(validate_branch_name))?;

    create_worktree_with_links(
        feature_name,
        Some(&branch_name),
        Some(from_ref),
        link_patterns,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::config::{OnCreate, SymlinkPatterns, WorktreeConfig};
    use std::fs;
    use tempfile::TempDir;

    fn make_config_with_symlinks(patterns: Vec<String>) -> WorktreeConfig {
        WorktreeConfig {
            copy_patterns: crate::config::CopyPatterns {
                include: Some(vec![]),
                exclude: Some(vec![]),
            },
            symlink_patterns: SymlinkPatterns {
                include: Some(patterns),
            },
            on_create: OnCreate { commands: None },
        }
    }

    fn make_config_with_hooks(commands: Vec<String>) -> WorktreeConfig {
        WorktreeConfig {
            copy_patterns: crate::config::CopyPatterns {
                include: Some(vec![]),
                exclude: Some(vec![]),
            },
            symlink_patterns: SymlinkPatterns { include: None },
            on_create: OnCreate {
                commands: Some(commands),
            },
        }
    }

    // ── create_symlinks ──────────────────────────────────────────────────────

    #[test]
    fn test_create_symlinks_creates_symlink_for_matching_path() {
        let tmp = TempDir::new().unwrap();
        let origin = tmp.path().join("origin");
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&origin).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        // Create a file in the origin that will be symlinked
        fs::write(origin.join("shared-data.txt"), "content").unwrap();

        let config = make_config_with_symlinks(vec!["shared-data.txt".to_string()]);
        create_symlinks(&origin, &worktree, &config, false).unwrap();

        let link = worktree.join("shared-data.txt");
        assert!(link.symlink_metadata().is_ok(), "symlink should exist");
        assert!(
            link.symlink_metadata().unwrap().file_type().is_symlink(),
            "should be a symlink not a copy"
        );
        assert_eq!(fs::read_to_string(&link).unwrap(), "content");
    }

    #[test]
    fn test_create_symlinks_skips_missing_origin_path_with_no_error() {
        let tmp = TempDir::new().unwrap();
        let origin = tmp.path().join("origin");
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&origin).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        // Pattern matches nothing in origin — should not error, should not create anything
        let config = make_config_with_symlinks(vec!["does-not-exist.txt".to_string()]);
        let result = create_symlinks(&origin, &worktree, &config, false);

        assert!(
            result.is_ok(),
            "missing origin path should not cause an error"
        );
        assert!(
            !worktree.join("does-not-exist.txt").exists(),
            "no symlink should be created for missing path"
        );
    }

    #[test]
    fn test_create_symlinks_takes_precedence_over_copy() {
        let tmp = TempDir::new().unwrap();
        let origin = tmp.path().join("origin");
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&origin).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        fs::write(origin.join(".env"), "ORIGIN=1").unwrap();

        // Config: symlink .env AND copy .env* — symlink should take precedence
        let config = WorktreeConfig {
            copy_patterns: crate::config::CopyPatterns {
                include: Some(vec![".env*".to_string()]),
                exclude: Some(vec![]),
            },
            symlink_patterns: SymlinkPatterns {
                include: Some(vec![".env".to_string()]),
            },
            on_create: OnCreate { commands: None },
        };

        // First create symlinks (as in create_worktree_internal)
        create_symlinks(&origin, &worktree, &config, false).unwrap();
        // Then copy (should skip .env because it's already symlinked)
        copy_config_files(&origin, &worktree, &config).unwrap();

        let target = worktree.join(".env");
        assert!(target.symlink_metadata().is_ok(), ".env should exist");
        assert!(
            target.symlink_metadata().unwrap().file_type().is_symlink(),
            ".env should be a symlink, not a copy"
        );
    }

    #[test]
    fn test_create_symlinks_force_replaces_existing_plain_file() {
        let tmp = TempDir::new().unwrap();
        let origin = tmp.path().join("origin");
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&origin).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        fs::write(origin.join("shared.md"), "origin content").unwrap();
        // Worktree already has a plain copy at this path (e.g. from an earlier copy-pattern sync)
        fs::write(worktree.join("shared.md"), "stale local copy").unwrap();

        let config = make_config_with_symlinks(vec!["shared.md".to_string()]);
        create_symlinks(&origin, &worktree, &config, true).unwrap();

        let link = worktree.join("shared.md");
        assert!(
            link.symlink_metadata().unwrap().file_type().is_symlink(),
            "existing plain file should be replaced with a symlink when force=true"
        );
        assert_eq!(fs::read_to_string(&link).unwrap(), "origin content");
    }

    #[test]
    fn test_create_symlinks_force_leaves_existing_symlink_untouched() {
        let tmp = TempDir::new().unwrap();
        let origin = tmp.path().join("origin");
        let worktree = tmp.path().join("worktree");
        let elsewhere = tmp.path().join("elsewhere.md");
        fs::create_dir_all(&origin).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        fs::write(origin.join("shared.md"), "origin content").unwrap();
        fs::write(&elsewhere, "unrelated content").unwrap();

        // Worktree already has a symlink pointing somewhere unexpected
        std::os::unix::fs::symlink(&elsewhere, worktree.join("shared.md")).unwrap();

        let config = make_config_with_symlinks(vec!["shared.md".to_string()]);
        create_symlinks(&origin, &worktree, &config, true).unwrap();

        let link = worktree.join("shared.md");
        let target = fs::read_link(&link).unwrap();
        assert_eq!(
            target, elsewhere,
            "existing symlink should be left untouched even with force=true, regardless of target"
        );
    }

    // ── run_on_create_hooks ──────────────────────────────────────────────────

    #[test]
    fn test_run_on_create_hooks_runs_commands_in_order() {
        let tmp = TempDir::new().unwrap();
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&worktree).unwrap();

        // Commands write to a file in sequence to verify ordering
        let marker = worktree.join("order.txt");
        let config = make_config_with_hooks(vec![
            format!("sh -c 'echo first >> {}'", marker.display()),
            format!("sh -c 'echo second >> {}'", marker.display()),
            format!("sh -c 'echo third >> {}'", marker.display()),
        ]);

        run_on_create_hooks(&worktree, &config).unwrap();

        let content = fs::read_to_string(&marker).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_run_on_create_hooks_stops_on_first_failure() {
        let tmp = TempDir::new().unwrap();
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&worktree).unwrap();

        let marker = worktree.join("ran.txt");
        let config = make_config_with_hooks(vec![
            format!("sh -c 'echo before-fail >> {}'", marker.display()),
            "sh -c 'exit 1'".to_string(), // fails here
            format!("sh -c 'echo after-fail >> {}'", marker.display()),
        ]);

        // Should succeed (hook failure is non-fatal to the create operation)
        let result = run_on_create_hooks(&worktree, &config);
        assert!(result.is_ok(), "hook failure should not propagate as Err");

        let content = fs::read_to_string(&marker).unwrap_or_default();
        let lines: Vec<&str> = content.lines().collect();
        // First command ran, third command did not
        assert!(
            lines.contains(&"before-fail"),
            "command before failure should have run"
        );
        assert!(
            !lines.contains(&"after-fail"),
            "command after failure should NOT have run"
        );
    }

    #[test]
    fn test_run_on_create_hooks_worktree_intact_after_failure() {
        let tmp = TempDir::new().unwrap();
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&worktree).unwrap();

        // Put a file in the worktree before hooks run
        fs::write(worktree.join("important.txt"), "do not delete").unwrap();

        let config = make_config_with_hooks(vec!["sh -c 'exit 42'".to_string()]);

        run_on_create_hooks(&worktree, &config).unwrap();

        // Worktree directory and its contents must still exist
        assert!(
            worktree.exists(),
            "worktree directory should still exist after hook failure"
        );
        assert!(
            worktree.join("important.txt").exists(),
            "worktree contents should be intact after hook failure"
        );
    }

    #[test]
    fn test_run_on_create_hooks_no_commands_is_noop() {
        let tmp = TempDir::new().unwrap();
        let worktree = tmp.path().join("worktree");
        fs::create_dir_all(&worktree).unwrap();

        let config = WorktreeConfig::default();
        let result = run_on_create_hooks(&worktree, &config);
        assert!(result.is_ok());
    }

    // ── should_exclude_file ──────────────────────────────────────────────────

    #[test]
    fn test_should_exclude_file_matches_relative_path_only() {
        let base = Path::new("/home/user/dev/target/myrepo");

        // Regression: the old substring match against the ABSOLUTE path
        // excluded everything in repos under a directory like "target".
        assert!(!should_exclude_file(&base.join(".env"), base, &["target/".to_string()]).unwrap());

        // Directory prefix patterns exclude contents
        assert!(
            should_exclude_file(
                &base.join("target/debug/app"),
                base,
                &["target/".to_string()]
            )
            .unwrap()
        );
        assert!(
            !should_exclude_file(&base.join("src/main.rs"), base, &["target/".to_string()])
                .unwrap()
        );

        // Glob patterns match the relative path
        assert!(
            should_exclude_file(&base.join("logs/debug.log"), base, &["*.log".to_string()])
                .unwrap()
        );

        // Plain patterns match a segment anywhere
        assert!(
            should_exclude_file(
                &base.join("src/build/output.js"),
                base,
                &["build".to_string()]
            )
            .unwrap()
        );

        // Paths outside the source root are never excluded
        assert!(
            !should_exclude_file(Path::new("/elsewhere/x.log"), base, &["*.log".to_string()])
                .unwrap()
        );
    }

    // ── validate_branch_name_internal ────────────────────────────────────────

    #[test]
    fn test_validate_branch_name_rejects_git_reserved_forms() {
        assert!(matches!(
            validate_branch_name_internal("main"),
            Validation::Valid
        ));
        assert!(matches!(
            validate_branch_name_internal("feature/auth"),
            Validation::Valid
        ));

        for invalid in ["a//b", "a@{b", "@", "-opt", "a..b", "ends/"] {
            assert!(
                matches!(
                    validate_branch_name_internal(invalid),
                    Validation::Invalid(_)
                ),
                "branch name '{invalid}' should be rejected"
            );
        }
    }
}
