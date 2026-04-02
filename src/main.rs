use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use worktree::Result;
use worktree::commands::init::Shell;
use worktree::commands::skill::SkillAction;
use worktree::commands::{
    back, cleanup, create, init, jump, list, remove, skill, status, sync_config,
};

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
        /// Feature name for the worktree (used as directory name). If not provided, will prompt interactively.
        #[arg(value_hint = ValueHint::Other)]
        feature_name: Option<String>,
        /// Starting branch for the worktree (create new or use existing). If not provided, will prompt.
        #[arg(value_hint = ValueHint::Other)]
        branch: Option<String>,
        /// Starting point for new branch (branch, commit, tag)
        #[arg(long)]
        from: Option<String>,
        /// Launch interactive selection for --from reference
        #[arg(long)]
        interactive_from: bool,
        /// List available git references for completion (internal use)
        #[arg(long, hide = true)]
        list_from_completions: bool,
    },
    /// List all worktrees
    #[command(visible_alias = "ls")]
    List {
        /// Show worktrees for current repo only
        #[arg(long)]
        current: bool,
    },
    /// Remove a worktree
    Remove {
        /// Feature name or path to remove. If not provided, opens interactive selection.
        #[arg(value_hint = ValueHint::Other)]
        target: Option<String>,
        /// Also delete the branch checked out in this worktree
        #[arg(long)]
        delete_branch: bool,
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
        /// Target worktree (feature name). If not provided, opens interactive selection.
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
    /// Manage the worktree-manager agent skill
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create {
            feature_name,
            branch,
            from,
            interactive_from,
            list_from_completions,
        } => {
            if list_from_completions {
                create::list_git_ref_completions()?;
                return Ok(());
            }

            match (feature_name, branch, from, interactive_from) {
                // No args — full interactive workflow
                (None, None, None, false) => {
                    create::interactive_create_workflow()?;
                }
                // Feature name provided, wants interactive --from selection
                (Some(feat), branch_arg, None, true) => {
                    create::interactive_from_selection(&feat, branch_arg.as_deref())?;
                }
                // Feature name provided, no branch — prompt for branch interactively
                (Some(feat), None, _from_ref, false) => {
                    create::interactive_create_with_feature(&feat)?;
                }
                // Both feature name and branch provided
                (Some(feat), Some(branch_arg), from_ref, false) => {
                    create::create_worktree(&feat, Some(&branch_arg), from_ref.as_deref())?;
                }
                // Invalid: --from without feature name
                (None, _, Some(_), _) => {
                    anyhow::bail!(
                        "Cannot specify --from without a feature name. Use interactive mode instead."
                    );
                }
                // Invalid: --interactive-from without feature name
                (None, _, _, true) => {
                    anyhow::bail!(
                        "--interactive-from requires a feature name. Use interactive mode instead."
                    );
                }
                // Feature + branch + from + interactive_from: use from ref
                (Some(feat), Some(branch_arg), Some(from_ref), true) => {
                    create::create_worktree(&feat, Some(&branch_arg), Some(&from_ref))?;
                }
                // Catch-all: invalid combinations
                _ => {
                    anyhow::bail!(
                        "Invalid argument combination. Run 'worktree create --help' for usage."
                    );
                }
            }
        }
        Commands::List { current } => {
            list::list_worktrees(current)?;
        }
        Commands::Remove {
            target,
            delete_branch,
            interactive,
            list_completions,
            current,
        } => {
            remove::remove_worktree(
                target.as_deref(),
                delete_branch,
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
        Commands::Skill { action } => {
            skill::run_skill_command(&action)?;
        }
    }

    Ok(())
}
