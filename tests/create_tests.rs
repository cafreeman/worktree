use anyhow::Result;
use std::path::{Path, PathBuf};
use worktree::commands::create::{self, CreateMode};

mod test_helpers;
use test_helpers::TestEnvironment;

/// Mock GitOperations for testing config inheritance
#[derive(Clone)]
struct MockGitRepo {
    repo_path: PathBuf,
    inherit_config_called: std::sync::Arc<std::sync::Mutex<bool>>,
}

impl MockGitRepo {
    fn new(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            inherit_config_called: std::sync::Arc::new(std::sync::Mutex::new(false)),
        }
    }

    fn was_inherit_config_called(&self) -> bool {
        *self.inherit_config_called.lock().unwrap()
    }
}

impl worktree::traits::GitOperations for MockGitRepo {
    fn open(_path: &Path) -> Result<Box<dyn worktree::traits::GitOperations>>
    where
        Self: Sized,
    {
        unreachable!("Use new() for testing")
    }

    fn get_repo_path(&self) -> PathBuf {
        self.repo_path.clone()
    }

    fn branch_exists(&self, _branch_name: &str) -> Result<bool> {
        // Return false so that smart mode will create the branch
        Ok(false)
    }

    fn create_worktree(
        &self,
        _branch_name: &str,
        worktree_path: &Path,
        _create_branch: bool,
    ) -> Result<()> {
        std::fs::create_dir_all(worktree_path)?;

        // Create a minimal git worktree structure
        let git_dir = worktree_path.join(".git");
        std::fs::write(
            &git_dir,
            format!("gitdir: {}", self.repo_path.join(".git").display()),
        )?;

        Ok(())
    }

    fn remove_worktree(&self, _worktree_name: &str) -> Result<()> {
        Ok(())
    }

    fn list_worktrees(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }

    fn delete_branch(&self, _branch_name: &str) -> Result<()> {
        Ok(())
    }

    fn inherit_config(&self, _worktree_path: &Path) -> Result<()> {
        *self.inherit_config_called.lock().unwrap() = true;
        Ok(())
    }
}

#[test]
fn test_create_worktree_simple() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Create a worktree (smart mode will create the branch since it doesn't exist)
        create::create_worktree("feature/test", CreateMode::Smart)?;

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
fn test_create_worktree_path_exists() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Pre-create the worktree directory
        let worktree_path = env.storage_root.join("test_repo").join("feature-test");
        std::fs::create_dir_all(&worktree_path)?;

        let result = create::create_worktree("feature/test", CreateMode::Smart);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));

        Ok(())
    })
}

#[test]
fn test_config_inheritance_is_called() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        let mock_git = MockGitRepo::new(env.repo_path.clone());

        // Test that config inheritance is called during worktree creation
        create::create_worktree_with_git(&mock_git, "feature/test", CreateMode::Smart)?;

        // Verify that inherit_config was called
        assert!(
            mock_git.was_inherit_config_called(),
            "inherit_config should have been called"
        );

        Ok(())
    })
}

#[test]
fn test_config_inheritance_with_real_git() -> Result<()> {
    let env = TestEnvironment::new()?;

    env.run_test(|| {
        // Set some additional git config in the test repo
        std::process::Command::new("git")
            .args(["config", "core.editor", "nano"])
            .current_dir(&env.repo_path)
            .output()?;

        std::process::Command::new("git")
            .args(["config", "user.signingkey", "test-key"])
            .current_dir(&env.repo_path)
            .output()?;

        // Create a worktree (this will use real GitRepo and config inheritance)
        create::create_worktree("feature/config-test", CreateMode::Smart)?;

        let worktree_path = env
            .storage_root
            .join("test_repo")
            .join("feature-config-test");
        assert!(worktree_path.exists(), "Worktree should be created");

        // Check that extensions.worktreeConfig is enabled in the main repo
        let output = std::process::Command::new("git")
            .args(["config", "extensions.worktreeConfig"])
            .current_dir(&env.repo_path)
            .output()?;

        let config_value = String::from_utf8(output.stdout)?;
        assert_eq!(
            config_value.trim(),
            "true",
            "worktreeConfig extension should be enabled"
        );

        // Verify worktree has inherited config by checking the worktree-specific config
        let output = std::process::Command::new("git")
            .args(["config", "--worktree", "--get", "user.name"])
            .current_dir(&worktree_path)
            .output()?;

        if output.status.success() {
            let user_name = String::from_utf8(output.stdout)?;
            assert_eq!(
                user_name.trim(),
                "Test User",
                "user.name should be inherited"
            );
        }

        Ok(())
    })
}
