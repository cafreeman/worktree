use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorktreeConfig {
    #[serde(rename = "copy-patterns")]
    pub copy_patterns: CopyPatterns,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CopyPatterns {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            copy_patterns: CopyPatterns {
                include: vec![
                    ".env*".to_string(),
                    ".vscode/".to_string(),
                    "*.local.json".to_string(),
                    "config/local/*".to_string(),
                ],
                exclude: vec![
                    "node_modules/".to_string(),
                    "target/".to_string(),
                    ".git/".to_string(),
                    "*.log".to_string(),
                    "*.tmp".to_string(),
                ],
            },
        }
    }
}

impl WorktreeConfig {
    pub fn load_from_repo(repo_path: &Path) -> Result<Self> {
        let config_path = repo_path.join(".worktree-config.toml");

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: WorktreeConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }
}
