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

    #[must_use]
    pub fn sanitize_branch_name(branch_name: &str) -> String {
        branch_name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-")
    }

    #[must_use]
    pub fn get_worktree_path(&self, repo_name: &str, branch_name: &str) -> PathBuf {
        let safe_branch_name = Self::sanitize_branch_name(branch_name);
        self.root_dir.join(repo_name).join(safe_branch_name)
    }

    /// Returns the path to the managed-branch flag file for a given branch
    fn get_managed_branch_flag_path(&self, repo_name: &str, branch_name: &str) -> PathBuf {
        let safe_branch_name = Self::sanitize_branch_name(branch_name);
        self.root_dir
            .join(repo_name)
            .join(".managed-branches")
            .join(safe_branch_name)
    }

    /// Retrieves the original branch name from a sanitized name
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to read the mapping file
    /// - Failed to parse the mapping data
    pub fn get_original_branch_name(
        &self,
        repo_name: &str,
        sanitized_name: &str,
    ) -> Result<Option<String>> {
        // We need a way to map back from sanitized names to original branch names
        // For now, we'll store a mapping file in each repo directory
        let mapping_file = self.root_dir.join(repo_name).join(".branch-mapping");

        if !mapping_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&mapping_file)?;
        for line in content.lines() {
            if let Some((sanitized, original)) = line.split_once(" -> ") {
                if sanitized == sanitized_name {
                    return Ok(Some(original.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Returns all (sanitized, original) pairs from the branch mapping file
    ///
    /// # Errors
    /// Returns an error if reading the mapping file fails
    fn read_all_branch_mappings(
        &self,
        repo_name: &str,
    ) -> Result<Vec<(String, String)>> {
        let mapping_file = self.root_dir.join(repo_name).join(".branch-mapping");

        if !mapping_file.exists() {
            return Ok(vec![]);
        }

        let content = std::fs::read_to_string(&mapping_file)?;
        let mut pairs = Vec::new();
        for line in content.lines() {
            if let Some((sanitized, original)) = line.split_once(" -> ") {
                pairs.push((sanitized.to_string(), original.to_string()));
            }
        }
        Ok(pairs)
    }

    /// Stores a mapping between original and sanitized branch names
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to create the mapping directory
    /// - Failed to write the mapping file
    /// - Failed to serialize the mapping data
    /// - A different branch already maps to the same sanitized form (collision)
    pub fn store_branch_mapping(
        &self,
        repo_name: &str,
        original_branch: &str,
        sanitized_branch: &str,
    ) -> Result<()> {
        let repo_dir = self.root_dir.join(repo_name);
        std::fs::create_dir_all(&repo_dir)?;

        // Check for collision: another original branch already maps to the same sanitized name
        let existing_mappings = self.read_all_branch_mappings(repo_name)?;
        for (existing_sanitized, existing_original) in &existing_mappings {
            if existing_sanitized == sanitized_branch && existing_original != original_branch {
                anyhow::bail!(
                    "Branch name '{}' conflicts with the sanitized form of existing branch '{}'",
                    sanitized_branch,
                    existing_original
                );
            }
        }

        let mapping_file = repo_dir.join(".branch-mapping");
        let mapping_entry = format!("{} -> {}\n", sanitized_branch, original_branch);

        // Read existing mappings
        let mut existing_content = if mapping_file.exists() {
            std::fs::read_to_string(&mapping_file)?
        } else {
            String::new()
        };

        // Check if mapping already exists (exact line match)
        let search_line = format!("{} -> {}", sanitized_branch, original_branch);
        if !existing_content.lines().any(|line| line == search_line) {
            existing_content.push_str(&mapping_entry);
            // Write atomically: write to temp then rename
            let tmp_path = mapping_file.with_extension("tmp");
            std::fs::write(&tmp_path, &existing_content)?;
            std::fs::rename(&tmp_path, &mapping_file)?;
        }

        Ok(())
    }

    /// Removes a mapping entry for the given original branch name
    ///
    /// # Errors
    /// Returns an error if reading or writing the mapping file fails
    pub fn remove_branch_mapping(&self, repo_name: &str, original_branch: &str) -> Result<()> {
        let mapping_file = self.root_dir.join(repo_name).join(".branch-mapping");

        if !mapping_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&mapping_file)?;

        // Keep lines that do not map to this original branch
        let new_content: String = content
            .lines()
            .filter(|line| {
                if let Some((_sanitized, original)) = line.split_once(" -> ") {
                    original != original_branch
                } else {
                    true
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let final_content = if new_content.is_empty() {
            String::new()
        } else {
            format!("{}\n", new_content)
        };

        // Write atomically: write to temp then rename
        let tmp_path = mapping_file.with_extension("tmp");
        std::fs::write(&tmp_path, &final_content)?;
        std::fs::rename(&tmp_path, &mapping_file)?;

        Ok(())
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
                    // Skip the .managed-branches directory as it's not a worktree
                    if name != ".managed-branches" {
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

    /// Marks a branch as managed by this CLI (created via worktree create)
    ///
    /// # Errors
    /// Returns an error if the marker file cannot be created
    pub fn mark_branch_managed(&self, repo_name: &str, branch_name: &str) -> Result<()> {
        let repo_dir = self.root_dir.join(repo_name).join(".managed-branches");
        std::fs::create_dir_all(&repo_dir)?;

        let flag_path = self.get_managed_branch_flag_path(repo_name, branch_name);

        // Write atomically: write to temp then rename
        let tmp_path = flag_path.with_extension("tmp");
        std::fs::write(&tmp_path, b"1")?;
        std::fs::rename(&tmp_path, &flag_path)?;

        Ok(())
    }

    /// Checks if a branch is managed by this CLI
    #[must_use]
    pub fn is_branch_managed(&self, repo_name: &str, branch_name: &str) -> bool {
        let flag_path = self.get_managed_branch_flag_path(repo_name, branch_name);
        flag_path.exists()
    }

    /// Unmarks a branch as managed by this CLI
    pub fn unmark_branch_managed(&self, repo_name: &str, branch_name: &str) {
        let flag_path = self.get_managed_branch_flag_path(repo_name, branch_name);
        if flag_path.exists() {
            // Ignore error if already removed by concurrent cleanup
            let _ = std::fs::remove_file(&flag_path);
        }
    }

    /// Stores origin information for a worktree
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to create the storage directory
    /// - Failed to write the origin mapping file
    pub fn store_worktree_origin(
        &self,
        repo_name: &str,
        branch_name: &str,
        origin_path: &str,
    ) -> Result<()> {
        let repo_dir = self.root_dir.join(repo_name);
        std::fs::create_dir_all(&repo_dir)?;

        let origin_mapping_file = repo_dir.join(".worktree-origins");
        let sanitized_branch = Self::sanitize_branch_name(branch_name);
        let mapping_entry = format!("{} -> {}\n", sanitized_branch, origin_path);

        // Read existing mappings
        let mut existing_content = if origin_mapping_file.exists() {
            std::fs::read_to_string(&origin_mapping_file)?
        } else {
            String::new()
        };

        // Check if mapping already exists (exact line match)
        let search_line = format!("{} -> {}", sanitized_branch, origin_path);
        if !existing_content.lines().any(|line| line == search_line) {
            existing_content.push_str(&mapping_entry);
            // Write atomically: write to temp then rename
            let tmp_path = origin_mapping_file.with_extension("tmp");
            std::fs::write(&tmp_path, &existing_content)?;
            std::fs::rename(&tmp_path, &origin_mapping_file)?;
        }

        Ok(())
    }

    /// Retrieves origin information for a worktree
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to read the origin mapping file
    pub fn get_worktree_origin(
        &self,
        repo_name: &str,
        branch_name: &str,
    ) -> Result<Option<String>> {
        let origin_mapping_file = self.root_dir.join(repo_name).join(".worktree-origins");

        if !origin_mapping_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&origin_mapping_file)?;
        let sanitized_branch = Self::sanitize_branch_name(branch_name);

        for line in content.lines() {
            if let Some((branch, origin)) = line.split_once(" -> ") {
                if branch == sanitized_branch {
                    return Ok(Some(origin.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Removes origin information for a worktree
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to read or write the origin mapping file
    pub fn remove_worktree_origin(&self, repo_name: &str, branch_name: &str) -> Result<()> {
        let origin_mapping_file = self.root_dir.join(repo_name).join(".worktree-origins");

        if !origin_mapping_file.exists() {
            return Ok(()); // Nothing to remove
        }

        let content = std::fs::read_to_string(&origin_mapping_file)?;
        let sanitized_branch = Self::sanitize_branch_name(branch_name);

        // Filter out the line for this branch
        let new_content: String = content
            .lines()
            .filter(|line| {
                if let Some((branch, _)) = line.split_once(" -> ") {
                    branch != sanitized_branch
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_storage(tmp: &TempDir) -> WorktreeStorage {
        let root = tmp.path().join("worktrees");
        std::fs::create_dir_all(&root).unwrap();
        WorktreeStorage { root_dir: root }
    }

    // ── sanitize_branch_name ─────────────────────────────────────────────────

    #[test]
    fn test_sanitize_slashes() {
        assert_eq!(
            WorktreeStorage::sanitize_branch_name("feature/auth"),
            "feature-auth"
        );
    }

    #[test]
    fn test_sanitize_backslash() {
        assert_eq!(
            WorktreeStorage::sanitize_branch_name("feature\\auth"),
            "feature-auth"
        );
    }

    #[test]
    fn test_sanitize_multiple_special_chars() {
        assert_eq!(
            WorktreeStorage::sanitize_branch_name("a/b\\c:d*e?f\"g<h>i|j"),
            "a-b-c-d-e-f-g-h-i-j"
        );
    }

    #[test]
    fn test_sanitize_already_sanitized() {
        assert_eq!(
            WorktreeStorage::sanitize_branch_name("feature-auth"),
            "feature-auth"
        );
    }

    #[test]
    fn test_sanitize_no_special_chars() {
        assert_eq!(
            WorktreeStorage::sanitize_branch_name("main"),
            "main"
        );
    }

    // ── store_branch_mapping / get_original_branch_name ─────────────────────

    #[test]
    fn test_branch_mapping_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let storage = make_storage(&tmp);

        storage
            .store_branch_mapping("myrepo", "feature/auth", "feature-auth")
            .unwrap();

        let result = storage
            .get_original_branch_name("myrepo", "feature-auth")
            .unwrap();
        assert_eq!(result, Some("feature/auth".to_string()));
    }

    #[test]
    fn test_branch_mapping_missing_returns_none() {
        let tmp = TempDir::new().unwrap();
        let storage = make_storage(&tmp);

        let result = storage
            .get_original_branch_name("myrepo", "no-such-branch")
            .unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_branch_mapping_no_duplicate_entries() {
        let tmp = TempDir::new().unwrap();
        let storage = make_storage(&tmp);

        // Store the same mapping twice — should not duplicate
        storage
            .store_branch_mapping("myrepo", "feature/auth", "feature-auth")
            .unwrap();
        storage
            .store_branch_mapping("myrepo", "feature/auth", "feature-auth")
            .unwrap();

        let mapping_file = storage.root_dir.join("myrepo").join(".branch-mapping");
        let content = std::fs::read_to_string(&mapping_file).unwrap();
        let count = content
            .lines()
            .filter(|l| *l == "feature-auth -> feature/auth")
            .count();
        assert_eq!(count, 1);
    }

    // ── remove_branch_mapping ────────────────────────────────────────────────

    #[test]
    fn test_remove_branch_mapping_removes_correct_entry() {
        let tmp = TempDir::new().unwrap();
        let storage = make_storage(&tmp);

        storage
            .store_branch_mapping("myrepo", "feature/auth", "feature-auth")
            .unwrap();
        storage
            .store_branch_mapping("myrepo", "bugfix/login", "bugfix-login")
            .unwrap();

        storage
            .remove_branch_mapping("myrepo", "feature/auth")
            .unwrap();

        // Removed entry should be gone
        let result = storage
            .get_original_branch_name("myrepo", "feature-auth")
            .unwrap();
        assert_eq!(result, None);

        // Other entry should remain
        let result = storage
            .get_original_branch_name("myrepo", "bugfix-login")
            .unwrap();
        assert_eq!(result, Some("bugfix/login".to_string()));
    }

    #[test]
    fn test_remove_branch_mapping_no_file_is_ok() {
        let tmp = TempDir::new().unwrap();
        let storage = make_storage(&tmp);

        // Should not error when there is no mapping file
        storage
            .remove_branch_mapping("myrepo", "feature/auth")
            .unwrap();
    }

    // ── store_worktree_origin duplicate detection (exact line) ───────────────

    #[test]
    fn test_store_worktree_origin_no_duplicate() {
        let tmp = TempDir::new().unwrap();
        let storage = make_storage(&tmp);

        storage
            .store_worktree_origin("myrepo", "feature/auth", "/home/user/repo")
            .unwrap();
        storage
            .store_worktree_origin("myrepo", "feature/auth", "/home/user/repo")
            .unwrap();

        let origin_file = storage.root_dir.join("myrepo").join(".worktree-origins");
        let content = std::fs::read_to_string(&origin_file).unwrap();
        let count = content
            .lines()
            .filter(|l| *l == "feature-auth -> /home/user/repo")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_store_worktree_origin_substring_not_false_positive() {
        // "feature-a" is a substring of "x-feature-a -> x-feature/a"
        // The second store should NOT be skipped.
        let tmp = TempDir::new().unwrap();
        let storage = make_storage(&tmp);

        // Store entry whose text contains "feature-a" as a substring
        storage
            .store_worktree_origin("myrepo", "x-feature/a", "/repo")
            .unwrap();

        // Now store an entry for "feature-a" (different branch)
        storage
            .store_worktree_origin("myrepo", "feature-a", "/repo2")
            .unwrap();

        let result = storage
            .get_worktree_origin("myrepo", "feature-a")
            .unwrap();
        assert_eq!(result, Some("/repo2".to_string()));
    }

    // ── collision detection (Issue 1) ────────────────────────────────────────

    #[test]
    fn test_branch_name_collision_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let storage = make_storage(&tmp);

        // Store feature/auth first
        storage
            .store_branch_mapping("myrepo", "feature/auth", "feature-auth")
            .unwrap();

        // Attempting to store feature-auth (same sanitized form, different original) must fail
        let result = storage.store_branch_mapping("myrepo", "feature-auth", "feature-auth");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("conflicts") || msg.contains("feature/auth"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn test_same_branch_no_collision() {
        let tmp = TempDir::new().unwrap();
        let storage = make_storage(&tmp);

        // Re-storing the exact same mapping should be fine (idempotent)
        storage
            .store_branch_mapping("myrepo", "feature/auth", "feature-auth")
            .unwrap();
        storage
            .store_branch_mapping("myrepo", "feature/auth", "feature-auth")
            .unwrap();
    }
}
