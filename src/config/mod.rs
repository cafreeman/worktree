//! Configuration management for worktree file copying patterns.
//!
//! This module provides flexible configuration loading with support for:
//! - Optional configuration fields (include/exclude patterns)
//! - Additive merging with sensible defaults
//! - Symlink patterns for long-lived shared files
//! - Post-create hooks for setup automation

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Main configuration structure for worktree file copying.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorktreeConfig {
    /// File copying pattern configuration
    #[serde(rename = "copy-patterns", default)]
    pub copy_patterns: CopyPatterns,
    /// Symlink pattern configuration (symlinks instead of copies)
    #[serde(rename = "symlink-patterns", default)]
    pub symlink_patterns: SymlinkPatterns,
    /// Post-create hook configuration
    #[serde(rename = "on-create", default)]
    pub on_create: OnCreate,
}

/// File copying pattern configuration with flexible merging behavior.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CopyPatterns {
    /// Patterns to include in file copying (glob patterns)
    #[serde(default)]
    pub include: Option<Vec<String>>,
    /// Patterns to exclude from file copying (glob patterns)
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
}

/// Symlink pattern configuration. Matching paths are symlinked to the origin repo
/// instead of copied.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SymlinkPatterns {
    /// Patterns to symlink (glob patterns or exact paths)
    #[serde(default)]
    pub include: Option<Vec<String>>,
}

/// Post-create hook configuration. Commands run sequentially in the worktree directory
/// after all files are copied and symlinked.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct OnCreate {
    /// Shell command strings to execute after worktree creation
    #[serde(default)]
    pub commands: Option<Vec<String>>,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            copy_patterns: CopyPatterns {
                include: Some(Self::default_include_patterns()),
                exclude: Some(Self::default_exclude_patterns()),
            },
            symlink_patterns: SymlinkPatterns { include: None },
            on_create: OnCreate { commands: None },
        }
    }
}

impl WorktreeConfig {
    /// Default include patterns for file copying
    fn default_include_patterns() -> Vec<String> {
        vec![
            ".env*".to_string(),
            ".vscode/".to_string(),
            "*.local.json".to_string(),
            "config/local/*".to_string(),
        ]
    }

    /// Default exclude patterns for file copying
    fn default_exclude_patterns() -> Vec<String> {
        vec![
            "node_modules/".to_string(),
            "target/".to_string(),
            ".git/".to_string(),
            "*.log".to_string(),
            "*.tmp".to_string(),
        ]
    }

    /// Loads worktree configuration from a repository with robust error handling.
    ///
    /// # Errors
    /// Only returns an error if the file system operation fails (e.g., permission denied).
    /// TOML parsing errors are handled gracefully with warnings and fallback to defaults.
    pub fn load_from_repo(repo_path: &Path) -> Result<Self> {
        let config_path = repo_path.join(".worktree-config.toml");

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        // Handle empty/blank files
        if content.trim().is_empty() {
            return Ok(Self::default());
        }

        // Try to parse the TOML, fall back to defaults on error
        match toml::from_str::<WorktreeConfig>(&content) {
            Ok(config) => Ok(config.merged_with_defaults()),
            Err(e) => {
                eprintln!("Warning: Invalid TOML syntax in .worktree-config.toml:");
                eprintln!("  {}", e);
                eprintln!("  Using default configuration. Please fix the syntax and try again.");
                Ok(Self::default())
            }
        }
    }

    /// Merges user configuration with defaults.
    #[must_use]
    pub fn merged_with_defaults(self) -> Self {
        let mut merged_includes = Self::default_include_patterns();
        let mut merged_excludes = Self::default_exclude_patterns();

        if let Some(user_includes) = self.copy_patterns.include {
            for pattern in user_includes {
                if !merged_includes.contains(&pattern) {
                    merged_includes.push(pattern);
                }
            }
        }

        if let Some(user_excludes) = self.copy_patterns.exclude {
            for pattern in user_excludes {
                if !merged_excludes.contains(&pattern) {
                    merged_excludes.push(pattern);
                }
            }
        }

        Self {
            copy_patterns: CopyPatterns {
                include: Some(merged_includes),
                exclude: Some(merged_excludes),
            },
            symlink_patterns: self.symlink_patterns,
            on_create: self.on_create,
        }
    }
}
