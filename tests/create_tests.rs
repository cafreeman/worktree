use anyhow::Result;
use worktree::commands::create;

mod test_helpers;
use test_helpers::TestEnvironment;

#[test]
fn test_create_worktree_simple() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create a worktree
        create::create_worktree("feature/test", None, false)?;

        // Verify real files were created
        let worktree_path = env.storage_root.join("test_repo").join("feature-test");
        assert!(worktree_path.exists());

        // Verify branch mapping file
        let mapping_file = env.storage_root.join("test_repo").join(".branch-mapping");
        assert!(mapping_file.exists());

        let mapping_content = std::fs::read_to_string(mapping_file)?;
        assert!(mapping_content.contains("feature-test -> feature/test"));

        Ok(())
    })
}

#[test]
fn test_create_worktree_custom_path() -> Result<()> {
    let env = TestEnvironment::new()?;
    let custom_path = env.temp_dir.path().join("custom_location");

    env.run_test(|| {
        create::create_worktree("feature/test", Some(custom_path.to_str().unwrap()), false)?;

        // Custom path should exist
        assert!(custom_path.exists());

        // No branch mapping for custom paths
        let mapping_file = env.storage_root.join("test_repo").join(".branch-mapping");
        assert!(!mapping_file.exists());

        Ok(())
    })
}

#[test]
fn test_create_worktree_path_exists() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Pre-create the worktree directory
        let worktree_path = env.storage_root.join("test_repo").join("feature-test");
        std::fs::create_dir_all(&worktree_path)?;

        let result = create::create_worktree("feature/test", None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));

        Ok(())
    })
}
