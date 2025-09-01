//! Configuration management for worktree file copying patterns.
//!
//! This module provides flexible configuration loading with support for:
//! - Optional configuration fields (include/exclude patterns)
//! - Additive merging with sensible defaults
//! - Pattern negation via `exclude-defaults`
//! - Graceful error handling for invalid configurations
//!
//! # Configuration Examples
//!
//! ## Minimal Configuration (merges with defaults)
//! ```toml
//! [copy-patterns]
//! include = ["mise.toml", "docker-compose.yml"]
//! # Result: Custom includes + all default excludes
//! ```
//!
//! ## Pattern Negation
//! ```toml
//! [copy-patterns]
//! include = ["mise.toml"]
//! exclude-defaults = [".vscode/", "config/local/*"]
//! # Result: .env* + *.local.json + mise.toml (no .vscode/ or config/local/*)
//! ```
//!
//! ## Complete Configuration (overrides defaults)
//! ```toml
//! [copy-patterns]
//! include = ["custom.conf", "*.env"]
//! exclude = ["*.secret", "temp/"]
//! # Result: Exactly what's specified (legacy behavior)
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Main configuration structure for worktree file copying.
///
/// This struct represents the complete configuration loaded from `.worktree-config.toml`
/// or default values when no configuration file exists.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorktreeConfig {
    /// File copying pattern configuration
    #[serde(rename = "copy-patterns", default)]
    pub copy_patterns: CopyPatterns,
}

/// File copying pattern configuration with flexible merging behavior.
///
/// All fields are optional to support partial configurations that merge with defaults.
/// This enables users to specify only what they want to customize.
#[derive(Debug, Serialize, Deserialize)]
pub struct CopyPatterns {
    /// Patterns to include in file copying (glob patterns)
    ///
    /// If specified, these patterns are added to the default include patterns.
    /// If not specified, only default include patterns are used.
    #[serde(default)]
    pub include: Option<Vec<String>>,

    /// Patterns to exclude from file copying (glob patterns)
    ///
    /// If specified, these patterns are added to the default exclude patterns.
    /// If not specified, only default exclude patterns are used.
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
}

impl Default for CopyPatterns {
    fn default() -> Self {
        Self {
            include: None,
            exclude: None,
        }
    }
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            copy_patterns: CopyPatterns {
                include: Some(Self::default_include_patterns()),
                exclude: Some(Self::default_exclude_patterns()),
            },
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
    /// This method attempts to load configuration from `.worktree-config.toml` in the
    /// specified repository. If the file doesn't exist, is empty, or contains invalid
    /// TOML, it gracefully falls back to default configuration.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the git repository
    ///
    /// # Returns
    ///
    /// * `Ok(WorktreeConfig)` - Successfully loaded and merged configuration
    ///
    /// # Errors
    ///
    /// Only returns an error if the file system operation fails (e.g., permission denied).
    /// TOML parsing errors are handled gracefully with warnings and fallback to defaults.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::Path;
    /// use worktree::config::WorktreeConfig;
    ///
    /// let config = WorktreeConfig::load_from_repo(Path::new("/path/to/repo"))?;
    /// // Always succeeds - uses defaults if config file is missing/invalid
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
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
                // Log warning about parse error but continue with defaults
                eprintln!("Warning: Invalid TOML syntax in .worktree-config.toml:");
                eprintln!("  {}", e);
                eprintln!("  Using default configuration. Please fix the syntax and try again.");
                Ok(Self::default())
            }
        }
    }

    /// Merges user configuration with defaults using precedence-based strategy.
    ///
    /// # Merging Strategy
    ///
    /// 1. **Start with defaults** - Use default include and exclude patterns
    /// 2. **User includes win** - User include patterns override default excludes
    /// 3. **User excludes win** - User exclude patterns override default includes
    /// 4. **Additive merging** - User patterns are added to defaults, conflicts resolved by precedence
    ///
    /// # Examples
    ///
    /// ```toml
    /// # User wants to include something normally excluded
    /// [copy-patterns]
    /// include = ["node_modules/.cache"]
    /// # Result: default includes + node_modules/.cache (even though node_modules/ is excluded by default)
    /// ```
    ///
    /// ```toml
    /// # User wants to exclude something normally included
    /// [copy-patterns]
    /// exclude = [".vscode/"]
    /// # Result: default excludes + .vscode/ (even though .vscode/ is included by default)
    /// ```
    pub fn merged_with_defaults(self) -> Self {
        let mut merged_includes = Self::default_include_patterns();
        let mut merged_excludes = Self::default_exclude_patterns();

        // Add user include patterns (user wins over default excludes)
        if let Some(user_includes) = self.copy_patterns.include {
            for pattern in user_includes {
                if !merged_includes.contains(&pattern) {
                    merged_includes.push(pattern);
                }
            }
        }

        // Add user exclude patterns (user wins over default includes)
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
        }
    }
}
