use anyhow::Result;
use inquire::Select;
use std::path::PathBuf;

/// Trait for providing interactive selection functionality
/// This allows us to abstract away the interactive prompts for testing
pub trait SelectionProvider {
    /// Present a selection menu and return the user's choice
    ///
    /// # Errors
    /// Returns an error if the selection process fails or user cancels
    fn select(&self, prompt: &str, options: Vec<String>) -> Result<String>;
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
}

/// Helper function to parse path from selection string formatted as "repo/branch (path)"
///
/// # Errors
/// Returns an error if the selection string is not in the expected format
pub fn extract_path_from_selection(selection: &str) -> Result<PathBuf> {
    if let Some(path_start) = selection.rfind(" (") {
        let path_str = &selection[path_start + 2..selection.len() - 1];
        Ok(PathBuf::from(path_str))
    } else {
        anyhow::bail!("Invalid selection format: {}", selection)
    }
}

/// Helper function to extract branch name from selection string formatted as "repo/branch (path)"
///
/// # Errors
/// Returns an error if the selection string is not in the expected format
pub fn extract_branch_from_selection(selection: &str) -> Result<String> {
    if let Some(path_start) = selection.rfind(" (") {
        let branch_part = &selection[..path_start];
        if let Some(slash_pos) = branch_part.rfind('/') {
            Ok(branch_part[slash_pos + 1..].to_string())
        } else {
            anyhow::bail!("Invalid selection format: {}", selection)
        }
    } else {
        anyhow::bail!("Invalid selection format: {}", selection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_selection_provider_valid_response() {
        let options = vec!["option1".to_string(), "option2".to_string()];
        let provider = MockSelectionProvider::new("option1");

        let result = provider.select("Test prompt", options);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "option1");
    }

    #[test]
    fn test_mock_selection_provider_invalid_response() {
        let options = vec!["option1".to_string(), "option2".to_string()];
        let provider = MockSelectionProvider::new("invalid");

        let result = provider.select("Test prompt", options);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_path_from_selection() {
        let selection = "repo/branch (/some/path)";
        let result = extract_path_from_selection(selection);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/some/path"));
    }

    #[test]
    fn test_extract_branch_from_selection() {
        let selection = "repo/feature-branch (/some/path)";
        let result = extract_branch_from_selection(selection);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "feature-branch");
    }

    #[test]
    fn test_extract_from_invalid_selection() {
        let invalid_selection = "invalid format";
        assert!(extract_path_from_selection(invalid_selection).is_err());
        assert!(extract_branch_from_selection(invalid_selection).is_err());
    }
}
