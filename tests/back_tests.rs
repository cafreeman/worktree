use std::env;
use std::fs;
use tempfile::TempDir;
use temp_env::with_var;

use worktree::commands::back;
use worktree::commands::create;
use worktree::storage::WorktreeStorage;

mod test_helpers;
use test_helpers::TestEnvironment;

#[test]
fn test_back_from_worktree_success() {
    let env = TestEnvironment::new().unwrap();
    
    env.run_test(|| {
        // Create a worktree
        create::create_worktree("feature-branch", create::CreateMode::Smart)?;

        // Change to the worktree directory
        let storage = WorktreeStorage::new()?;
        let worktree_path = storage.get_worktree_path("test_repo", "feature-branch");
        
        // Simulate being in the worktree directory
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&worktree_path)?;

        // Test back command
        let result = back::back_to_origin();
        
        // Restore original directory
        std::env::set_current_dir(original_dir)?;
        assert!(result.is_ok());
        Ok(())
    }).unwrap();
}

#[test]
fn test_back_from_non_worktree_directory() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_string_lossy().to_string();

    with_var("WORKTREE_STORAGE_ROOT", Some(&storage_path), || {
        // Try to run back from a non-worktree directory
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();

        let result = back::back_to_origin();
        
        env::set_current_dir(original_dir).unwrap();

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Not currently in a worktree directory"));
    });
}

#[test]
fn test_back_with_missing_origin_info() {
    let env = TestEnvironment::new().unwrap();
    
    env.run_test(|| {
        // Create a worktree
        create::create_worktree("feature-branch", create::CreateMode::Smart)?;

        // Manually remove the origin information to simulate old worktree
        let storage = WorktreeStorage::new()?;
        let origin_file = storage.get_repo_storage_dir("test_repo").join(".worktree-origins");
        if origin_file.exists() {
            fs::remove_file(&origin_file)?;
        }

        let worktree_path = storage.get_worktree_path("test_repo", "feature-branch");
        
        // Simulate being in the worktree directory
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&worktree_path)?;

        // Test back command
        let result = back::back_to_origin();
        
        // Restore original directory
        std::env::set_current_dir(original_dir)?;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("No origin information available"));
        Ok(())
    }).unwrap();
}

#[test]
fn test_back_with_sanitized_branch_names() {
    let env = TestEnvironment::new().unwrap();
    
    env.run_test(|| {
        // Create a worktree with a branch name that needs sanitization
        let branch_name = "feature/auth-system";
        create::create_worktree(branch_name, create::CreateMode::Smart)?;

        let storage = WorktreeStorage::new()?;
        let worktree_path = storage.get_worktree_path("test_repo", branch_name);
        
        // Simulate being in the worktree directory
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&worktree_path)?;

        // Test back command
        let result = back::back_to_origin();
        
        // Restore original directory
        std::env::set_current_dir(original_dir)?;

        assert!(result.is_ok());
        Ok(())
    }).unwrap();
}

#[test]
fn test_back_from_subdirectory_in_worktree() {
    let env = TestEnvironment::new().unwrap();
    
    env.run_test(|| {
        // Create a worktree
        create::create_worktree("feature-branch", create::CreateMode::Smart)?;

        let storage = WorktreeStorage::new()?;
        let worktree_path = storage.get_worktree_path("test_repo", "feature-branch");
        
        // Create a subdirectory in the worktree
        let subdir = worktree_path.join("src");
        fs::create_dir_all(&subdir)?;
        
        // Simulate being in the subdirectory
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&subdir)?;

        // Test back command from subdirectory
        let result = back::back_to_origin();
        
        // Restore original directory
        std::env::set_current_dir(original_dir)?;

        assert!(result.is_ok());
        Ok(())
    }).unwrap();
}