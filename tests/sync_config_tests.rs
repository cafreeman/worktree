use anyhow::Result;
use std::fs;
use worktree::commands::{
    create::{self, CreateMode},
    sync_config,
};

mod test_helpers;
use test_helpers::TestEnvironment;

#[test]
fn test_sync_config_between_worktrees() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create source and target worktrees
        create::create_worktree("feature/source", None, CreateMode::Smart)?;
        create::create_worktree("feature/target", None, CreateMode::Smart)?;

        let source_path = env.storage_root.join("test_repo").join("feature-source");
        let target_path = env.storage_root.join("test_repo").join("feature-target");
        assert!(source_path.exists());
        assert!(target_path.exists());

        // Create some config files in the source worktree
        let vscode_dir = source_path.join(".vscode");
        fs::create_dir_all(&vscode_dir)?;
        fs::write(vscode_dir.join("settings.json"), r#"{"editor.tabSize": 4}"#)?;

        // Test sync_config
        sync_config::sync_config("feature/source", "feature/target")?;

        // Verify config files were copied
        let target_vscode = target_path.join(".vscode").join("settings.json");
        assert!(target_vscode.exists());
        let content = fs::read_to_string(target_vscode)?;
        assert!(content.contains("tabSize"));

        Ok(())
    })
}

#[test]
fn test_sync_config_with_sanitized_names() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create worktrees with special characters
        create::create_worktree("feature/sync-source", None, CreateMode::Smart)?;
        create::create_worktree("feature/sync-target", None, CreateMode::Smart)?;

        let source_path = env
            .storage_root
            .join("test_repo")
            .join("feature-sync-source");
        let target_path = env
            .storage_root
            .join("test_repo")
            .join("feature-sync-target");
        assert!(source_path.exists());
        assert!(target_path.exists());

        // Create config in source (using a pattern that matches default config)
        let vscode_dir = source_path.join(".vscode");
        fs::create_dir_all(&vscode_dir)?;
        fs::write(vscode_dir.join("settings.json"), r#"{"test": "value"}"#)?;

        // Test sync using sanitized names
        sync_config::sync_config("feature-sync-source", "feature-sync-target")?;

        // Verify sync worked
        let target_config = target_path.join(".vscode").join("settings.json");
        assert!(target_config.exists());

        Ok(())
    })
}

#[test]
fn test_sync_config_with_absolute_paths() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create worktrees
        create::create_worktree("feature/abs-source", None, CreateMode::Smart)?;
        create::create_worktree("feature/abs-target", None, CreateMode::Smart)?;

        let source_path = env
            .storage_root
            .join("test_repo")
            .join("feature-abs-source");
        let target_path = env
            .storage_root
            .join("test_repo")
            .join("feature-abs-target");
        assert!(source_path.exists());
        assert!(target_path.exists());

        // Create config files (using a pattern that matches default config)
        fs::write(source_path.join(".env.local"), "TEST_VAR=hello")?;

        // Test sync using absolute paths
        sync_config::sync_config(source_path.to_str().unwrap(), target_path.to_str().unwrap())?;

        // Verify sync worked
        let target_env = target_path.join(".env.local");
        assert!(target_env.exists());

        Ok(())
    })
}

#[test]
fn test_sync_config_nonexistent_source() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create only target worktree
        create::create_worktree("feature/target-only", None, CreateMode::Smart)?;

        // Try to sync from nonexistent source
        let result = sync_config::sync_config("nonexistent", "feature/target-only");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));

        Ok(())
    })
}

#[test]
fn test_sync_config_nonexistent_target() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create only source worktree
        create::create_worktree("feature/source-only", None, CreateMode::Smart)?;

        // Try to sync to nonexistent target
        let result = sync_config::sync_config("feature/source-only", "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));

        Ok(())
    })
}

#[test]
fn test_sync_config_mixed_worktree_types() -> Result<()> {
    let env = TestEnvironment::new()?;
    let custom_source = env.temp_dir.path().join("custom_source");
    let custom_target = env.temp_dir.path().join("custom_target");

    env.run_test(|| {
        // Create custom path worktrees
        create::create_worktree(
            "feature/custom-source",
            Some(custom_source.to_str().unwrap()),
            CreateMode::Smart,
        )?;
        create::create_worktree(
            "feature/custom-target",
            Some(custom_target.to_str().unwrap()),
            CreateMode::Smart,
        )?;

        assert!(custom_source.exists());
        assert!(custom_target.exists());

        // Create config in custom source
        let env_file = custom_source.join(".env");
        fs::write(&env_file, "NODE_ENV=development")?;

        // Sync from custom source to custom target
        sync_config::sync_config(
            custom_source.to_str().unwrap(),
            custom_target.to_str().unwrap(),
        )?;

        // Verify sync worked
        let target_env = custom_target.join(".env");
        assert!(target_env.exists());
        let content = fs::read_to_string(target_env)?;
        assert!(content.contains("NODE_ENV"));

        Ok(())
    })
}
