use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct WorktreeStorage {
    root_dir: PathBuf,
}

impl WorktreeStorage {
    pub fn new() -> Result<Self> {
        let root_dir = if let Ok(custom_root) = std::env::var("WORKTREE_STORAGE_ROOT") {
            PathBuf::from(custom_root)
        } else {
            dirs::data_dir()
                .context("Failed to get user data directory")?
                .join("worktrees")
        };

        std::fs::create_dir_all(&root_dir).context("Failed to create worktrees directory")?;

        Ok(Self { root_dir })
    }

    pub fn get_repo_name(&self, repo_path: &Path) -> Result<String> {
        if let Some(name) = repo_path.file_name() {
            Ok(name.to_string_lossy().to_string())
        } else {
            anyhow::bail!("Could not determine repository name from path")
        }
    }

    fn sanitize_branch_name(&self, branch_name: &str) -> String {
        branch_name
            .replace('/', "-")
            .replace('\\', "-")
            .replace(':', "-")
            .replace('*', "-")
            .replace('?', "-")
            .replace('"', "-")
            .replace('<', "-")
            .replace('>', "-")
            .replace('|', "-")
    }

    pub fn get_worktree_path(&self, repo_name: &str, branch_name: &str) -> PathBuf {
        let safe_branch_name = self.sanitize_branch_name(branch_name);
        self.root_dir.join(repo_name).join(safe_branch_name)
    }

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
                    worktrees.push(name.to_string());
                }
            }
        }

        Ok(worktrees)
    }

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
}
