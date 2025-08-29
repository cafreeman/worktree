use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use worktree::Result;
use worktree::commands::init::Shell;
use worktree::commands::{create, init, jump, list, remove, status, sync_config};

#[derive(Parser)]
#[command(name = "worktree")]
#[command(about = "A CLI tool for managing git worktrees with enhanced features")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new worktree
    Create {
        /// Branch name for the worktree
        #[arg(value_hint = ValueHint::Other)]
        branch: String,
        /// Custom path for the worktree (optional)
        #[arg(short, long, value_hint = ValueHint::DirPath)]
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
        #[arg(value_hint = ValueHint::Other)]
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
        #[arg(value_hint = ValueHint::Other)]
        from: String,
        /// Target branch or path
        #[arg(value_hint = ValueHint::Other)]
        to: String,
    },
    /// Generate shell integration for directory navigation
    Init {
        /// Shell to generate integration for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Jump to a worktree directory
    Jump {
        /// Target worktree (branch name). If not provided, opens interactive selection
        #[arg(value_hint = ValueHint::Other)]
        target: Option<String>,
        /// Launch interactive selection mode
        #[arg(long)]
        interactive: bool,
        /// List available worktrees for completion (internal use)
        #[arg(long, hide = true)]
        list_completions: bool,
        /// Current repo only
        #[arg(long)]
        current: bool,
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
        Commands::Init { shell } => {
            init::generate_shell_integration(shell)?;
        }
        Commands::Jump {
            target,
            interactive,
            list_completions,
            current,
        } => {
            jump::jump_worktree(target, interactive, list_completions, current)?;
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            init::generate_completions(shell, &mut cmd)?;
        }
    }

    Ok(())
}
