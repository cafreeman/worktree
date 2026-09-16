use anyhow::Result;
use inquire::{Select, Text, validator::Validation};
use std::error::Error;
use std::fmt;

use crate::git::GitRepo;

/// Type alias for validation functions
pub type ValidatorFn = fn(&str) -> Result<Validation, Box<dyn Error + Send + Sync>>;

/// Represents a git reference option with visual grouping
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitRefOption {
    /// A selectable git reference
    Reference { name: String, display: String },
    /// A visual separator/header (non-selectable)
    Separator(String),
}

impl fmt::Display for GitRefOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitRefOption::Reference { display, .. } => write!(f, "{}", display),
            GitRefOption::Separator(label) => {
                if label.is_empty() {
                    write!(f, "") // Empty separator for spacing
                } else {
                    write!(f, "─── {} ───", label)
                }
            }
        }
    }
}

/// Trait for providing interactive selection functionality
/// This allows us to abstract away the interactive prompts for testing
pub trait SelectionProvider {
    /// Present a selection menu and return the user's choice
    ///
    /// # Errors
    /// Returns an error if the selection process fails or user cancels
    fn select(&self, prompt: &str, options: Vec<String>) -> Result<String>;

    /// Present a grouped selection menu with visual separators
    ///
    /// # Errors
    /// Returns an error if the selection process fails or user cancels
    fn select_grouped(&self, prompt: &str, options: Vec<GitRefOption>) -> Result<String>;

    /// Get text input from the user with validation
    ///
    /// # Errors
    /// Returns an error if the input process fails or user cancels
    fn get_text_input(&self, prompt: &str, validator: Option<ValidatorFn>) -> Result<String>;
}

/// Real implementation using inquire::Select for production use
pub struct RealSelectionProvider;

impl SelectionProvider for RealSelectionProvider {
    fn select(&self, prompt: &str, options: Vec<String>) -> Result<String> {
        let selection = Select::new(prompt, options)
            .with_page_size(10)
            .with_vim_mode(true)
            .prompt()?;
        Ok(selection)
    }

    fn select_grouped(&self, prompt: &str, options: Vec<GitRefOption>) -> Result<String> {
        // Parse options into groups
        let mut groups: Vec<(String, Vec<String>)> = Vec::new();
        let mut current_group_name = String::new();
        let mut current_group_refs = Vec::new();

        for option in options {
            match option {
                GitRefOption::Separator(label) => {
                    // Save previous group if it exists
                    if !current_group_refs.is_empty() {
                        groups.push((current_group_name.clone(), current_group_refs.clone()));
                        current_group_refs.clear();
                    }
                    // Start new group (skip empty separators)
                    if !label.is_empty() {
                        current_group_name = label;
                    }
                }
                GitRefOption::Reference { name, .. } => {
                    current_group_refs.push(name);
                }
            }
        }

        // Add the last group
        if !current_group_refs.is_empty() {
            groups.push((current_group_name, current_group_refs));
        }

        // If we only have one group, select directly from it
        if groups.len() == 1 {
            let (_, refs) = &groups[0];
            return self.select(prompt, refs.clone());
        }

        // Multiple groups: let user choose group first, then reference
        let group_names: Vec<String> = groups
            .iter()
            .map(|(name, refs)| format!("{} ({} items)", name, refs.len()))
            .collect();

        let selected_group = self.select("Choose a category:", group_names)?;

        // Find the selected group and its references
        for (group_name, refs) in groups.iter() {
            let group_display = format!("{} ({} items)", group_name, refs.len());
            if group_display == selected_group {
                return self.select(&format!("Choose from {}:", group_name), refs.clone());
            }
        }

        anyhow::bail!("Selected group not found")
    }

    fn get_text_input(&self, prompt: &str, validator: Option<ValidatorFn>) -> Result<String> {
        let mut text_prompt = Text::new(prompt);

        if let Some(validation_fn) = validator {
            text_prompt = text_prompt.with_validator(validation_fn);
        }

        let result = text_prompt.prompt()?;
        Ok(result)
    }
}

/// Mock implementation for testing that returns a predetermined value
pub struct MockSelectionProvider {
    pub response: String,
}

impl MockSelectionProvider {
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }
}

impl SelectionProvider for MockSelectionProvider {
    fn select(&self, _prompt: &str, options: Vec<String>) -> Result<String> {
        // Validate that the response is actually in the options
        if options.contains(&self.response) {
            Ok(self.response.clone())
        } else {
            anyhow::bail!("Mock response '{}' not found in options", self.response)
        }
    }

    fn select_grouped(&self, _prompt: &str, options: Vec<GitRefOption>) -> Result<String> {
        // Extract only the selectable reference names from the grouped options
        let selectable_values: Vec<String> = options
            .into_iter()
            .filter_map(|opt| match opt {
                GitRefOption::Reference { name, .. } => Some(name),
                GitRefOption::Separator(_) => None,
            })
            .collect();

        // Validate that the response is actually in the selectable options
        if selectable_values.contains(&self.response) {
            Ok(self.response.clone())
        } else {
            anyhow::bail!(
                "Mock response '{}' not found in grouped options",
                self.response
            )
        }
    }

    fn get_text_input(&self, _prompt: &str, _validator: Option<ValidatorFn>) -> Result<String> {
        // For testing, return a predetermined response
        Ok(self.response.clone())
    }
}

/// Select a git reference interactively using visual separators
///
/// # Errors
/// Returns an error if:
/// - Git operations fail
/// - Interactive selection fails or is cancelled
/// - No git references available
pub fn select_git_reference_interactive(
    git_repo: &GitRepo,
    provider: &dyn SelectionProvider,
) -> Result<String> {
    // Get all references
    let local_branches = git_repo.list_local_branches()?;
    let remote_branches = git_repo.list_remote_branches()?;
    let tags = git_repo.list_tags()?;

    if local_branches.is_empty() && remote_branches.is_empty() && tags.is_empty() {
        anyhow::bail!("No git references found");
    }

    // Create grouped options with visual separators
    let mut options = Vec::new();

    // Local branches first (most commonly used)
    if !local_branches.is_empty() {
        options.push(GitRefOption::Separator("Local Branches".to_string()));
        for branch in &local_branches {
            options.push(GitRefOption::Reference {
                name: branch.clone(),
                display: format!("  {}", branch), // Indent for visual grouping
            });
        }
    }

    // Remote branches second
    if !remote_branches.is_empty() {
        if !options.is_empty() {
            // Add spacing if there are previous sections
            options.push(GitRefOption::Separator(String::new())); // Empty separator for spacing
        }
        options.push(GitRefOption::Separator("Remote Branches".to_string()));
        for branch in &remote_branches {
            options.push(GitRefOption::Reference {
                name: branch.clone(),
                display: format!("  {}", branch), // Indent for visual grouping
            });
        }
    }

    // Tags last
    if !tags.is_empty() {
        if !options.is_empty() {
            // Add spacing if there are previous sections
            options.push(GitRefOption::Separator(String::new())); // Empty separator for spacing
        }
        options.push(GitRefOption::Separator("Tags".to_string()));
        for tag in &tags {
            options.push(GitRefOption::Reference {
                name: tag.clone(),
                display: format!("  {}", tag), // Indent for visual grouping
            });
        }
    }

    if options.is_empty() {
        anyhow::bail!("No git references found");
    }

    provider.select_grouped("Select git reference to create worktree from:", options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_selection_provider_valid_response() {
        let options = vec!["option1".to_string(), "option2".to_string()];
        let provider = MockSelectionProvider::new("option1");

        let result = provider.select("Test prompt", options);
        assert!(matches!(result, Ok(ref s) if s == "option1"));
    }

    #[test]
    fn test_mock_selection_provider_invalid_response() {
        let options = vec!["option1".to_string(), "option2".to_string()];
        let provider = MockSelectionProvider::new("invalid");

        let result = provider.select("Test prompt", options);
        assert!(result.is_err());
    }

    #[test]
    fn test_git_ref_option_formatting() {
        // Test GitRefOption display formatting
        let local_ref = GitRefOption::Reference {
            name: "main".to_string(),
            display: "  main".to_string(),
        };
        let separator = GitRefOption::Separator("Local Branches".to_string());
        let empty_separator = GitRefOption::Separator(String::new());

        // Test display formatting
        assert_eq!(format!("{}", local_ref), "  main");
        assert_eq!(format!("{}", separator), "─── Local Branches ───");
        assert_eq!(format!("{}", empty_separator), "");

        // Test that reference names are preserved correctly
        if let GitRefOption::Reference { name, .. } = local_ref {
            assert_eq!(name, "main");
        } else {
            unreachable!("Expected Reference variant");
        }
    }

    #[test]
    fn test_select_grouped_functionality() {
        // Create mock provider that will return "main" for any selection
        let provider = MockSelectionProvider::new("main");

        // Create a grouped list with separators
        let options = vec![
            GitRefOption::Separator("Local Branches".to_string()),
            GitRefOption::Reference {
                name: "main".to_string(),
                display: "  main".to_string(),
            },
            GitRefOption::Reference {
                name: "feature".to_string(),
                display: "  feature".to_string(),
            },
            GitRefOption::Separator(String::new()), // Empty separator for spacing
            GitRefOption::Separator("Remote Branches".to_string()),
            GitRefOption::Reference {
                name: "origin/develop".to_string(),
                display: "  origin/develop".to_string(),
            },
        ];

        let result = provider.select_grouped("Choose base for new branch:", options);

        if let Ok(selected) = result {
            assert_eq!(selected, "main");
        } else {
            unreachable!("Selection should succeed in test: {:?}", result);
        }
    }

    #[test]
    fn test_git_ref_option_extraction() {
        // Test that we can correctly extract names from GitRefOption variants
        let ref_option = GitRefOption::Reference {
            name: "origin/feature".to_string(),
            display: "  origin/feature".to_string(),
        };

        match ref_option {
            GitRefOption::Reference { name, .. } => {
                assert_eq!(name, "origin/feature");
            }
            GitRefOption::Separator(_) => {
                unreachable!("Expected Reference, got Separator");
            }
        }

        let separator = GitRefOption::Separator("Remote Branches".to_string());
        match separator {
            GitRefOption::Separator(label) => {
                assert_eq!(label, "Remote Branches");
            }
            GitRefOption::Reference { .. } => {
                unreachable!("Expected Separator, got Reference");
            }
        }
    }
}
