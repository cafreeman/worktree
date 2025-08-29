use anyhow::Result;
use worktree::commands::{create, status};

mod test_helpers;
use test_helpers::TestEnvironment;

#[test]
fn test_show_status_empty() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Test status when no worktrees exist - should succeed
        status::show_status()?;

        Ok(())
    })
}

#[test]
fn test_show_status_with_worktrees() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create some worktrees
        create::create_worktree("feature/status1", None, false)?;
        create::create_worktree("feature/status2", None, false)?;

        // Verify worktrees were created
        let worktree_path1 = env.storage_root.join("test_repo").join("feature-status1");
        let worktree_path2 = env.storage_root.join("test_repo").join("feature-status2");
        assert!(worktree_path1.exists());
        assert!(worktree_path2.exists());

        // Test status - should show the worktrees
        status::show_status()?;

        Ok(())
    })
}

#[test]
fn test_show_status_missing_directories() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create a worktree
        create::create_worktree("feature/missing", None, false)?;

        let worktree_path = env.storage_root.join("test_repo").join("feature-missing");
        assert!(worktree_path.exists());

        // Delete the directory to simulate missing worktree
        std::fs::remove_dir_all(&worktree_path)?;
        assert!(!worktree_path.exists());

        // Status should still work and show inconsistent state
        status::show_status()?;

        Ok(())
    })
}

#[test]
fn test_show_status_custom_path_worktree() -> Result<()> {
    let env = TestEnvironment::new()?;
    let custom_path = env.temp_dir.path().join("custom_status_test");

    env.run_test(|| {
        // Create a managed worktree
        create::create_worktree("feature/managed-status", None, false)?;

        // Create a custom path worktree (not managed)
        create::create_worktree(
            "feature/custom-status",
            Some(custom_path.to_str().unwrap()),
            false,
        )?;
        assert!(custom_path.exists());

        // Status should show both worktrees and their different states
        status::show_status()?;

        Ok(())
    })
}

#[test]
fn test_show_status_mixed_scenarios() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create multiple worktrees in different states

        // 1. Normal active worktree
        create::create_worktree("feature/active", None, false)?;

        // 2. Worktree that will be missing
        create::create_worktree("feature/will-be-missing", None, false)?;

        let missing_path = env
            .storage_root
            .join("test_repo")
            .join("feature-will-be-missing");
        assert!(missing_path.exists());
        std::fs::remove_dir_all(&missing_path)?;
        assert!(!missing_path.exists());

        // 3. Worktree with special characters
        create::create_worktree("feature/test-special", None, false)?;

        // Status should handle all these scenarios
        status::show_status()?;

        Ok(())
    })
}
