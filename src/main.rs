use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use worktree::Result;
use worktree::commands::init::Shell;
use worktree::commands::{back, cleanup, create, init, jump, list, remove, status, sync_config};

#[derive(Parser)]
#[command(name = "worktree")]
#[command(about = "A CLI tool for managing git worktrees with enhanced features")]
#[command(version)]
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
        /// Force creation of a new branch (fail if it already exists)
        #[arg(long, conflicts_with = "existing_branch")]
        new_branch: bool,
        /// Only use an existing branch (fail if it doesn't exist)
        #[arg(long, conflicts_with = "new_branch")]
        existing_branch: bool,
    },
    /// List all worktrees
    List {
        /// Show worktrees for current repo only
        #[arg(long)]
        current: bool,
    },
    /// Remove a worktree
    Remove {
        /// Branch name or path to remove. If not provided, opens interactive selection
        #[arg(value_hint = ValueHint::Other)]
        target: Option<String>,
        /// Keep the branch (only remove the worktree)
        #[arg(long)]
        keep_branch: bool,
        /// Launch interactive selection mode
        #[arg(long)]
        interactive: bool,
        /// List available worktrees for completion (internal use)
        #[arg(long, hide = true)]
        list_completions: bool,
        /// Show worktrees for current repo only
        #[arg(long)]
        current: bool,
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
    /// Clean up orphaned branches and worktree references
    Cleanup,
    /// Navigate back to the original repository
    Back,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create {
            branch,
            new_branch,
            existing_branch,
        } => {
            let mode = if new_branch {
                create::CreateMode::NewBranch
            } else if existing_branch {
                create::CreateMode::ExistingBranch
            } else {
                create::CreateMode::Smart
            };
            create::create_worktree(&branch, mode)?;
        }
        Commands::List { current } => {
            list::list_worktrees(current)?;
        }
        Commands::Remove {
            target,
            keep_branch,
            interactive,
            list_completions,
            current,
        } => {
            remove::remove_worktree(
                target.as_deref(),
                !keep_branch,
                interactive,
                list_completions,
                current,
            )?;
        }
        Commands::Status => {
            status::show_status()?;
        }
        Commands::SyncConfig { from, to } => {
            sync_config::sync_config(&from, &to)?;
        }
        Commands::Init { shell } => {
            init::generate_shell_integration(shell);
        }
        Commands::Jump {
            target,
            interactive,
            list_completions,
            current,
        } => {
            jump::jump_worktree(target.as_deref(), interactive, list_completions, current)?;
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            init::generate_completions(shell, &mut cmd);
        }
        Commands::Cleanup => {
            cleanup::cleanup_worktrees()?;
        }
        Commands::Back => {
            back::back_to_origin()?;
        }
    }

    Ok(())
}
