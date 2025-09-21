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
        /// Branch name for the worktree (if not provided, will prompt interactively)
        #[arg(value_hint = ValueHint::Other)]
        branch: Option<String>,
        /// Starting point for new branch (branch, commit, tag)
        #[arg(long)]
        from: Option<String>,
        /// Force creation of a new branch (fail if it already exists)
        #[arg(long, conflicts_with = "existing_branch")]
        new_branch: bool,
        /// Only use an existing branch (fail if it doesn't exist)
        #[arg(long, conflicts_with = "new_branch")]
        existing_branch: bool,
        /// Launch interactive selection for --from reference
        #[arg(long)]
        interactive_from: bool,
        /// List available git references for completion (internal use)
        #[arg(long, hide = true)]
        list_from_completions: bool,
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
        /// Force deletion of branch even if unmanaged
        #[arg(long)]
        force_delete_branch: bool,
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
    #[command(visible_alias = "switch")]
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
            from,
            new_branch,
            existing_branch,
            interactive_from,
            list_from_completions,
        } => {
            if list_from_completions {
                create::list_git_ref_completions()?;
                return Ok(());
            }

            // Handle different execution modes
            match (branch, from, interactive_from) {
                // No branch provided - launch full interactive workflow
                (None, None, false) => {
                    create::interactive_create_workflow()?;
                }
                // Branch provided but wants interactive --from selection
                (Some(branch_name), None, true) => {
                    create::interactive_from_selection(&branch_name)?;
                }
                // Traditional command-line usage
                (Some(branch_name), from_ref, false) => {
                    let mode = if new_branch {
                        create::CreateMode::NewBranch
                    } else if existing_branch {
                        create::CreateMode::ExistingBranch
                    } else {
                        create::CreateMode::Smart
                    };
                    create::create_worktree(&branch_name, from_ref.as_deref(), mode)?;
                }
                // Invalid combinations
                (None, Some(_), _) => {
                    anyhow::bail!(
                        "Cannot specify --from without a branch name. Use interactive mode instead."
                    );
                }
                (None, None, true) => {
                    anyhow::bail!(
                        "--interactive-from requires a branch name. Use interactive mode instead."
                    );
                }
                // Branch provided with from_ref AND interactive_from - use the from_ref
                (Some(branch_name), Some(from_ref), true) => {
                    let mode = if new_branch {
                        create::CreateMode::NewBranch
                    } else if existing_branch {
                        create::CreateMode::ExistingBranch
                    } else {
                        create::CreateMode::Smart
                    };
                    create::create_worktree(&branch_name, Some(&from_ref), mode)?;
                }
            }
        }
        Commands::List { current } => {
            list::list_worktrees(current)?;
        }
        Commands::Remove {
            target,
            keep_branch,
            force_delete_branch,
            interactive,
            list_completions,
            current,
        } => {
            remove::remove_worktree(
                target.as_deref(),
                !keep_branch,
                force_delete_branch,
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
