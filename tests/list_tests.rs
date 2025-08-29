#![allow(clippy::unwrap_used)] // Tests use unwrap for simplicity

use anyhow::Result;
use worktree::commands::{
    create::{self, CreateMode},
    list,
};

mod test_helpers;
use test_helpers::TestEnvironment;

#[test]
fn test_list_worktrees_empty() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Test listing when no worktrees exist - should succeed
        list::list_worktrees(false)?;
        list::list_worktrees(true)?;

        Ok(())
    })
}

#[test]
fn test_list_worktrees_with_content() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create some worktrees first
        create::create_worktree("feature/test1", CreateMode::Smart)?;
        create::create_worktree("feature/test2", CreateMode::Smart)?;

        // Test listing all worktrees
        list::list_worktrees(false)?;

        // Test current repo only
        list::list_worktrees(true)?;

        // Verify the worktrees were actually created
        let worktree_path1 = env.storage_root.join("test_repo").join("feature-test1");
        let worktree_path2 = env.storage_root.join("test_repo").join("feature-test2");
        assert!(worktree_path1.exists());
        assert!(worktree_path2.exists());

        Ok(())
    })
}

#[test]
fn test_list_worktrees_mixed_states() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create a worktree
        create::create_worktree("feature/active", CreateMode::Smart)?;

        let worktree_path = env.storage_root.join("test_repo").join("feature-active");
        assert!(worktree_path.exists());

        // Delete the directory to simulate a missing worktree
        std::fs::remove_dir_all(&worktree_path)?;
        assert!(!worktree_path.exists());

        // List should still work and show the missing worktree
        list::list_worktrees(true)?;

        Ok(())
    })
}
