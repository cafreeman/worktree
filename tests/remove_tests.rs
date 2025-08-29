use anyhow::Result;
use worktree::commands::{create, remove};

mod test_helpers;
use test_helpers::TestEnvironment;

#[test]
fn test_remove_worktree_success() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create a worktree using real git
        create::create_worktree("feature/test", None, false)?;

        let worktree_path = env.storage_root.join("test_repo").join("feature-test");
        assert!(worktree_path.exists());

        // Remove the worktree
        remove::remove_worktree("feature/test", false)?;

        // Verify directory is removed
        assert!(!worktree_path.exists());

        Ok(())
    })
}

#[test]
fn test_remove_worktree_with_branch_deletion() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create a worktree
        create::create_worktree("feature/delete-me", None, false)?;

        let worktree_path = env.storage_root.join("test_repo").join("feature-delete-me");
        assert!(worktree_path.exists());

        // Remove the worktree and delete branch
        remove::remove_worktree("feature/delete-me", true)?;

        // Verify directory is removed
        assert!(!worktree_path.exists());

        Ok(())
    })
}

#[test]
fn test_remove_worktree_by_sanitized_name() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create a worktree with special characters
        create::create_worktree("feature/test-branch", None, false)?;

        let worktree_path = env
            .storage_root
            .join("test_repo")
            .join("feature-test-branch");
        assert!(worktree_path.exists());

        // Remove using the sanitized name
        remove::remove_worktree("feature-test-branch", false)?;

        // Verify directory is removed
        assert!(!worktree_path.exists());

        Ok(())
    })
}

#[test]
fn test_remove_worktree_by_absolute_path() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create a worktree
        create::create_worktree("feature/abs-path", None, false)?;

        let worktree_path = env.storage_root.join("test_repo").join("feature-abs-path");
        assert!(worktree_path.exists());

        // Remove using absolute path
        remove::remove_worktree(worktree_path.to_str().unwrap(), false)?;

        // Verify directory is removed
        assert!(!worktree_path.exists());

        Ok(())
    })
}

#[test]
fn test_remove_worktree_nonexistent() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Try to remove a worktree that doesn't exist
        let result = remove::remove_worktree("nonexistent", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));

        Ok(())
    })
}

#[test]
fn test_remove_custom_path_worktree() -> Result<()> {
    let env = TestEnvironment::new()?;
    let custom_path = env.temp_dir.path().join("custom_remove_test");

    env.run_test(|| {
        // Create a worktree with custom path
        create::create_worktree(
            "feature/custom-remove",
            Some(custom_path.to_str().unwrap()),
            false,
        )?;
        assert!(custom_path.exists());

        // Remove using the custom path
        remove::remove_worktree(custom_path.to_str().unwrap(), false)?;

        // Verify custom path directory is removed
        assert!(!custom_path.exists());

        Ok(())
    })
}
