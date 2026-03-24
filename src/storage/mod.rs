use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct WorktreeStorage {
    root_dir: PathBuf,
}

impl WorktreeStorage {
    /// Creates a new WorktreeStorage instance
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to determine home directory
    /// - Failed to create storage directory
    pub fn new() -> Result<Self> {
        let root_dir = if let Ok(custom_root) = std::env::var("WORKTREE_STORAGE_ROOT") {
            PathBuf::from(custom_root)
        } else {
            dirs::home_dir()
                .context("Failed to get user home directory")?
                .join(".worktrees")
        };

        std::fs::create_dir_all(&root_dir).context("Failed to create worktrees directory")?;

        Ok(Self { root_dir })
    }

    /// Extracts repository name from a path
    ///
    /// # Errors
    /// Returns an error if the path doesn't have a valid file name
    pub fn get_repo_name(repo_path: &Path) -> Result<String> {
        if let Some(name) = repo_path.file_name() {
            Ok(name.to_string_lossy().to_string())
        } else {
            anyhow::bail!("Could not determine repository name from path")
        }
    }

    /// Validates a feature name, rejecting characters that are invalid for directory names.
    ///
    /// Feature names must not contain: `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`
    ///
    /// # Errors
    /// Returns an error if the name contains invalid characters or is empty.
    pub fn validate_feature_name(name: &str) -> Result<()> {
        if name.trim().is_empty() {
            anyhow::bail!("Feature name cannot be empty");
        }

        let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
        for ch in invalid_chars {
            if name.contains(ch) {
                anyhow::bail!(
                    "Feature name '{}' contains invalid character '{}'. \
                     Feature names cannot contain: / \\ : * ? \" < > |",
                    name,
                    ch
                );
            }
        }

        Ok(())
    }

    /// Returns the worktree path for the given feature name (no sanitization)
    #[must_use]
    pub fn get_worktree_path(&self, repo_name: &str, feature_name: &str) -> PathBuf {
        self.root_dir.join(repo_name).join(feature_name)
    }

    /// Lists all worktrees for a specific repository
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to read the repository directory
    /// - Directory access issues
    pub fn list_repo_worktrees(&self, repo_name: &str) -> Result<Vec<String>> {
        let repo_dir = self.root_dir.join(repo_name);

        if !repo_dir.exists() {
            return Ok(vec![]);
        }

        let mut worktrees = Vec::new();
        for entry in std::fs::read_dir(&repo_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    // Skip hidden directories (e.g. .git metadata)
                    if !name.starts_with('.') {
                        worktrees.push(name.to_string());
                    }
                }
            }
        }

        Ok(worktrees)
    }

    /// Lists all worktrees across all repositories
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to read the storage directory
    /// - Directory access issues
    pub fn list_all_worktrees(&self) -> Result<Vec<(String, Vec<String>)>> {
        let mut all_worktrees = Vec::new();

        if !self.root_dir.exists() {
            return Ok(all_worktrees);
        }

        for entry in std::fs::read_dir(&self.root_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(repo_name) = entry.file_name().to_str() {
                    let worktrees = self.list_repo_worktrees(repo_name)?;
                    all_worktrees.push((repo_name.to_string(), worktrees));
                }
            }
        }

        Ok(all_worktrees)
    }

    /// Gets the storage directory for a specific repository
    #[must_use]
    pub fn get_repo_storage_dir(&self, repo_name: &str) -> PathBuf {
        self.root_dir.join(repo_name)
    }

    /// Gets the root storage directory
    #[must_use]
    pub fn get_root_dir(&self) -> &PathBuf {
        &self.root_dir
    }

    /// Stores origin information for a worktree (keyed by feature name)
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to create the storage directory
    /// - Failed to write the origin mapping file
    pub fn store_worktree_origin(
        &self,
        repo_name: &str,
        feature_name: &str,
        origin_path: &str,
    ) -> Result<()> {
        let repo_dir = self.root_dir.join(repo_name);
        std::fs::create_dir_all(&repo_dir)?;

        let origin_mapping_file = repo_dir.join(".worktree-origins");
        let mapping_entry = format!("{} -> {}\n", feature_name, origin_path);

        // Read existing mappings
        let mut existing_content = if origin_mapping_file.exists() {
            std::fs::read_to_string(&origin_mapping_file)?
        } else {
            String::new()
        };

        // Check if mapping already exists (exact line match)
        let search_line = format!("{} -> {}", feature_name, origin_path);
        if !existing_content.lines().any(|line| line == search_line) {
            existing_content.push_str(&mapping_entry);
            // Write atomically: write to temp then rename
            let tmp_path = origin_mapping_file.with_extension("tmp");
            std::fs::write(&tmp_path, &existing_content)?;
            std::fs::rename(&tmp_path, &origin_mapping_file)?;
        }

        Ok(())
    }

    /// Retrieves origin information for a worktree (keyed by feature name)
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to read the origin mapping file
    pub fn get_worktree_origin(
        &self,
        repo_name: &str,
        feature_name: &str,
    ) -> Result<Option<String>> {
        let origin_mapping_file = self.root_dir.join(repo_name).join(".worktree-origins");

        if !origin_mapping_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&origin_mapping_file)?;

        for line in content.lines() {
            if let Some((key, origin)) = line.split_once(" -> ") {
                if key == feature_name {
                    return Ok(Some(origin.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Removes origin information for a worktree (keyed by feature name)
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to read or write the origin mapping file
    pub fn remove_worktree_origin(&self, repo_name: &str, feature_name: &str) -> Result<()> {
        let origin_mapping_file = self.root_dir.join(repo_name).join(".worktree-origins");

        if !origin_mapping_file.exists() {
            return Ok(()); // Nothing to remove
        }

        let content = std::fs::read_to_string(&origin_mapping_file)?;

        // Filter out the line for this feature name
        let new_content: String = content
            .lines()
            .filter(|line| {
                if let Some((key, _)) = line.split_once(" -> ") {
                    key != feature_name
                } else {
                    true // Keep malformed lines
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Add trailing newline if there's content
        let final_content = if new_content.is_empty() {
            String::new()
        } else {
            format!("{}\n", new_content)
        };

        // Write atomically: write to temp then rename
        let tmp_path = origin_mapping_file.with_extension("tmp");
        std::fs::write(&tmp_path, &final_content)?;
        std::fs::rename(&tmp_path, &origin_mapping_file)?;

        Ok(())
    }
}

/// Reads the current HEAD branch name of a worktree directory.
/// Returns None if the worktree is in detached HEAD state or cannot be opened.
#[must_use]
pub fn read_worktree_head_branch(path: &Path) -> Option<String> {
    let repo = git2::Repository::open(path).ok()?;
    let head = repo.head().ok()?;
    if head.is_branch() {
        head.shorthand().map(ToString::to_string)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_storage(tmp: &TempDir) -> Result<WorktreeStorage> {
        let root = tmp.path().join("worktrees");
        std::fs::create_dir_all(&root)?;
        Ok(WorktreeStorage { root_dir: root })
    }

    // ── validate_feature_name ────────────────────────────────────────────────

    #[test]
    fn test_validate_feature_name_valid() {
        assert!(WorktreeStorage::validate_feature_name("auth").is_ok());
        assert!(WorktreeStorage::validate_feature_name("my-feature").is_ok());
        assert!(WorktreeStorage::validate_feature_name("feature_123").is_ok());
        assert!(WorktreeStorage::validate_feature_name("auth-v2.0").is_ok());
    }

    #[test]
    fn test_validate_feature_name_slash_rejected() {
        let result = WorktreeStorage::validate_feature_name("feature/auth");
        assert!(result.is_err());
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(msg.contains('/') || msg.contains("invalid character"));
        }
    }

    #[test]
    fn test_validate_feature_name_special_chars_rejected() {
        for ch in &['\\', ':', '*', '?', '"', '<', '>', '|'] {
            let name = format!("feature{}auth", ch);
            assert!(
                WorktreeStorage::validate_feature_name(&name).is_err(),
                "Should reject name containing '{}'",
                ch
            );
        }
    }

    #[test]
    fn test_validate_feature_name_empty_rejected() {
        assert!(WorktreeStorage::validate_feature_name("").is_err());
        assert!(WorktreeStorage::validate_feature_name("   ").is_err());
    }

    // ── get_worktree_path ────────────────────────────────────────────────────

    #[test]
    fn test_get_worktree_path_uses_feature_name_directly() -> Result<()> {
        let tmp = TempDir::new()?;
        let storage = make_storage(&tmp)?;
        let path = storage.get_worktree_path("myrepo", "auth");
        assert!(path.to_string_lossy().ends_with("myrepo/auth"));
        Ok(())
    }

    // ── store_worktree_origin / get_worktree_origin ──────────────────────────

    #[test]
    fn test_store_worktree_origin_no_duplicate() -> Result<()> {
        let tmp = TempDir::new()?;
        let storage = make_storage(&tmp)?;

        storage.store_worktree_origin("myrepo", "auth", "/home/user/repo")?;
        storage.store_worktree_origin("myrepo", "auth", "/home/user/repo")?;

        let origin_file = storage.root_dir.join("myrepo").join(".worktree-origins");
        let content = std::fs::read_to_string(&origin_file)?;
        let count = content
            .lines()
            .filter(|l| *l == "auth -> /home/user/repo")
            .count();
        assert_eq!(count, 1);
        Ok(())
    }

    #[test]
    fn test_store_worktree_origin_roundtrip() -> Result<()> {
        let tmp = TempDir::new()?;
        let storage = make_storage(&tmp)?;

        storage.store_worktree_origin("myrepo", "auth", "/home/user/repo")?;

        let result = storage.get_worktree_origin("myrepo", "auth")?;
        assert_eq!(result, Some("/home/user/repo".to_string()));
        Ok(())
    }

    #[test]
    fn test_store_worktree_origin_different_features_independent() -> Result<()> {
        let tmp = TempDir::new()?;
        let storage = make_storage(&tmp)?;

        storage.store_worktree_origin("myrepo", "auth", "/repo1")?;
        storage.store_worktree_origin("myrepo", "payments", "/repo2")?;

        assert_eq!(
            storage.get_worktree_origin("myrepo", "auth")?,
            Some("/repo1".to_string())
        );
        assert_eq!(
            storage.get_worktree_origin("myrepo", "payments")?,
            Some("/repo2".to_string())
        );
        Ok(())
    }

    // ── list_repo_worktrees ──────────────────────────────────────────────────

    #[test]
    fn test_list_repo_worktrees_skips_hidden_dirs() -> Result<()> {
        let tmp = TempDir::new()?;
        let storage = make_storage(&tmp)?;
        let repo_dir = storage.root_dir.join("myrepo");
        std::fs::create_dir_all(repo_dir.join("auth"))?;
        std::fs::create_dir_all(repo_dir.join("payments"))?;
        // Hidden dir should be skipped
        std::fs::create_dir_all(repo_dir.join(".hidden"))?;
        // File should not appear (not a dir)
        std::fs::write(repo_dir.join(".worktree-origins"), "")?;

        let worktrees = storage.list_repo_worktrees("myrepo")?;
        assert!(worktrees.contains(&"auth".to_string()));
        assert!(worktrees.contains(&"payments".to_string()));
        assert!(!worktrees.contains(&".hidden".to_string()));
        assert_eq!(worktrees.len(), 2);
        Ok(())
    }
}
