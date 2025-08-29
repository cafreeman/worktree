#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

use anyhow::Result;
use worktree::commands::{
    create::{self, CreateMode},
    remove,
};
use worktree::storage::WorktreeStorage;

mod test_helpers;
use test_helpers::TestEnvironment;

#[test]
fn test_remove_worktree_success() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create a worktree using real git
        create::create_worktree("feature/test", CreateMode::Smart)?;

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
        create::create_worktree("feature/delete-me", CreateMode::Smart)?;

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
        create::create_worktree("feature/test-branch", CreateMode::Smart)?;

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
        create::create_worktree("feature/abs-path", CreateMode::Smart)?;

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
fn test_remove_worktree_cleans_up_origin_info() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create a worktree
        create::create_worktree("feature/cleanup-test", CreateMode::Smart)?;

        let storage = WorktreeStorage::new()?;
        let worktree_path = storage.get_worktree_path("test_repo", "feature/cleanup-test");
        assert!(worktree_path.exists());

        // Verify origin info exists
        let origin_file = storage
            .get_repo_storage_dir("test_repo")
            .join(".worktree-origins");
        assert!(origin_file.exists());
        let content = std::fs::read_to_string(&origin_file)?;
        assert!(content.contains("feature-cleanup-test"));

        // Remove the worktree
        remove::remove_worktree("feature/cleanup-test", false)?;

        // Verify worktree is gone
        assert!(!worktree_path.exists());

        // Verify origin info is cleaned up
        if origin_file.exists() {
            let content = std::fs::read_to_string(&origin_file)?;
            assert!(!content.contains("feature-cleanup-test"));
        }

        Ok(())
    })
}

#[test]
fn test_remove_worktree_with_branch_deletion_cleans_up_origin() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create a worktree
        create::create_worktree("feature/delete-test", CreateMode::Smart)?;

        let storage = WorktreeStorage::new()?;
        let worktree_path = storage.get_worktree_path("test_repo", "feature/delete-test");
        assert!(worktree_path.exists());

        // Verify origin info exists
        let origin_file = storage
            .get_repo_storage_dir("test_repo")
            .join(".worktree-origins");
        assert!(origin_file.exists());
        let content = std::fs::read_to_string(&origin_file)?;
        assert!(content.contains("feature-delete-test"));

        // Remove the worktree and delete the branch
        remove::remove_worktree("feature/delete-test", true)?;

        // Verify worktree is gone
        assert!(!worktree_path.exists());

        // Verify origin info is cleaned up
        if origin_file.exists() {
            let content = std::fs::read_to_string(&origin_file)?;
            assert!(!content.contains("feature-delete-test"));
        }

        Ok(())
    })
}
