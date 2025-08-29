use clap::{Parser, Subcommand};
use worktree::Result;
use worktree::commands::{create, list, remove, status, sync_config};

#[derive(Parser)]
#[command(name = "worktree")]
#[command(about = "A CLI tool for managing git worktrees with enhanced features")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new worktree
    Create {
        /// Branch name for the worktree
        branch: String,
        /// Custom path for the worktree (optional)
        #[arg(short, long)]
        path: Option<String>,
        /// Create a new branch if it doesn't exist
        #[arg(short = 'b', long)]
        create_branch: bool,
    },
    /// List all worktrees
    List {
        /// Show worktrees for current repo only
        #[arg(long)]
        current: bool,
    },
    /// Remove a worktree
    Remove {
        /// Branch name or path to remove
        target: String,
        /// Also delete the associated branch
        #[arg(short, long)]
        delete_branch: bool,
    },
    /// Show worktree status
    Status,
    /// Sync config files between worktrees
    SyncConfig {
        /// Source branch or path
        from: String,
        /// Target branch or path
        to: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create {
            branch,
            path,
            create_branch,
        } => {
            create::create_worktree(&branch, path.as_deref(), create_branch)?;
        }
        Commands::List { current } => {
            list::list_worktrees(current)?;
        }
        Commands::Remove {
            target,
            delete_branch,
        } => {
            remove::remove_worktree(&target, delete_branch)?;
        }
        Commands::Status => {
            status::show_status()?;
        }
        Commands::SyncConfig { from, to } => {
            sync_config::sync_config(&from, &to)?;
        }
    }

    Ok(())
}
