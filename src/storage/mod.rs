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

    fn sanitize_branch_name(branch_name: &str) -> String {
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

    /// Stores a mapping between original and sanitized branch names
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to create the mapping directory
    /// - Failed to write the mapping file
    /// - Failed to serialize the mapping data
    pub fn store_branch_mapping(
        &self,
        repo_name: &str,
        original_branch: &str,
        sanitized_branch: &str,
    ) -> Result<()> {
        let repo_dir = self.root_dir.join(repo_name);
        std::fs::create_dir_all(&repo_dir)?;

        let mapping_file = repo_dir.join(".branch-mapping");
        let mapping_entry = format!("{} -> {}\n", sanitized_branch, original_branch);

        // Read existing mappings
        let mut existing_content = if mapping_file.exists() {
            std::fs::read_to_string(&mapping_file)?
        } else {
            String::new()
        };

        // Check if mapping already exists
        let search_line = format!("{} -> {}", sanitized_branch, original_branch);
        if !existing_content.contains(&search_line) {
            existing_content.push_str(&mapping_entry);
            std::fs::write(&mapping_file, existing_content)?;
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

        std::fs::write(&mapping_file, final_content)?;
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

        // Check if mapping already exists
        let search_line = format!("{} -> {}", sanitized_branch, origin_path);
        if !existing_content.contains(&search_line) {
            existing_content.push_str(&mapping_entry);
            std::fs::write(&origin_mapping_file, existing_content)?;
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

        std::fs::write(&origin_mapping_file, final_content)?;

        Ok(())
    }
}
