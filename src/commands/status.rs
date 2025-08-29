use anyhow::Result;

use crate::git::GitRepo;
use crate::storage::WorktreeStorage;

pub fn show_status() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let git_repo = GitRepo::open(&current_dir)?;
    let repo_path = git_repo.get_repo_path();

    let storage = WorktreeStorage::new()?;
    let repo_name = storage.get_repo_name(repo_path)?;

    println!("Git Worktree Status");
    println!("{}", "=".repeat(40));
    println!("Repository: {}", repo_name);
    println!("Repository path: {}", repo_path.display());
    println!();

    let git_worktrees = git_repo.list_worktrees()?;
    let managed_worktrees = storage.list_repo_worktrees(&repo_name)?;

    println!("Git worktrees ({}):", git_worktrees.len());
    for worktree in &git_worktrees {
        let worktree_path = storage.get_worktree_path(&repo_name, worktree);
        let managed = if managed_worktrees.contains(worktree) {
            "📁"
        } else {
            "⚠"
        };
        let exists = if worktree_path.exists() { "✓" } else { "✗" };

        println!(
            "  {} {} {} ({})",
            managed,
            exists,
            worktree,
            worktree_path.display()
        );
    }

    println!();
    println!("Managed worktrees ({}):", managed_worktrees.len());
    for worktree in &managed_worktrees {
        let worktree_path = storage.get_worktree_path(&repo_name, worktree);
        let in_git = if git_worktrees.contains(worktree) {
            "🔗"
        } else {
            "⚠"
        };
        let exists = if worktree_path.exists() { "✓" } else { "✗" };

        println!(
            "  {} {} {} ({})",
            in_git,
            exists,
            worktree,
            worktree_path.display()
        );
    }

    println!();
    println!("Legend:");
    println!("  📁 = Managed by this tool");
    println!("  🔗 = Registered in git");
    println!("  ✓ = Directory exists");
    println!("  ✗ = Directory missing");
    println!("  ⚠ = Inconsistent state");

    Ok(())
}
