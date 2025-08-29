use clap::{Command, ValueEnum};
use clap_complete::{generate, Shell as CompleteShell};
use std::io;

#[derive(ValueEnum, Clone, Copy)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

/// Generate shell integration for the specified shell
pub fn generate_shell_integration(shell: Shell) {
    match shell {
        Shell::Bash => print_bash_integration(),
        Shell::Zsh => print_zsh_integration(),
        Shell::Fish => print_fish_integration(),
    }
}

/// Generate native shell completions using clap
pub fn generate_completions(shell: Shell, cmd: &mut Command) {
    let clap_shell = match shell {
        Shell::Bash => CompleteShell::Bash,
        Shell::Zsh => CompleteShell::Zsh,
        Shell::Fish => CompleteShell::Fish,
    };
    
    generate(clap_shell, cmd, cmd.get_name().to_string(), &mut io::stdout());
}


fn print_bash_integration() {
    println!(
        r#"# Worktree shell integration for Bash
# This replaces the worktree command with a shell function that can change directories

worktree() {{
    case "$1" in
        jump)
            # Handle jump specially - call rust binary and cd to result
            shift
            local result
            if [ $# -eq 0 ]; then
                # Interactive mode
                result=$(worktree-bin jump --interactive 2>/dev/null)
            else
                # Direct mode
                result=$(worktree-bin jump "$@" 2>/dev/null)
            fi

            if [ -n "$result" ]; then
                cd "$result" || return 1
            fi
            ;;
        *)
            # Delegate everything else to the rust binary
            worktree-bin "$@"
            ;;
    esac
}}

# Load clap-generated completions for worktree-bin
_worktree_bin_clap_complete() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    
    # Use clap-generated completions by calling worktree-bin with completion
    local completions
    completions=$(worktree-bin completions bash 2>/dev/null | bash -c 'source /dev/stdin && complete -p worktree-bin' 2>/dev/null)
    
    # Extract and execute the completion function
    if [[ -n "$completions" ]]; then
        eval "$completions"
        # Call the clap-generated completion function
        _worktree-bin
        return 0
    fi
    
    # Fallback to basic completion
    COMPREPLY=($(compgen -W "create list remove status sync-config jump init completions --help --version" -- "$cur"))
}}

# Enhanced completion for the worktree shell function
_worktree_complete() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    local prev="${{COMP_WORDS[COMP_CWORD-1]}}"

    if [ "${{COMP_WORDS[1]}}" = "jump" ]; then
        # Special handling for jump subcommand
        if [ "${{#COMP_WORDS[@]}}" -eq 3 ] && [ -z "$cur" ]; then
            # Launch interactive mode on TAB for empty jump
            worktree jump
            return 0
        fi
        
        # Combine clap completions with custom jump completion
        if [[ "$cur" == -* ]]; then
            # Complete flags using clap
            _worktree_bin_clap_complete
        else
            # Complete worktree names using custom logic
            local worktrees=$(worktree-bin jump --list-completions 2>/dev/null)
            COMPREPLY=($(compgen -W "$worktrees" -- "$cur"))
        fi
    else
        # Use clap-generated completions for all other commands
        _worktree_bin_clap_complete
    fi
}}

complete -F _worktree_complete worktree"#
    );
}

fn print_zsh_integration() {
    println!(
        r#"# Worktree shell integration for Zsh
# This replaces the worktree command with a shell function that can change directories

worktree() {{
    case "$1" in
        jump)
            # Handle jump specially - call rust binary and cd to result
            shift
            local result
            if [ $# -eq 0 ]; then
                # Interactive mode
                result=$(worktree-bin jump --interactive 2>/dev/null)
            else
                # Direct mode
                result=$(worktree-bin jump "$@" 2>/dev/null)
            fi

            if [ -n "$result" ]; then
                cd "$result" || return 1
            fi
            ;;
        *)
            # Delegate everything else to the rust binary
            worktree-bin "$@"
            ;;
    esac
}}

# Load clap-generated completion for worktree-bin
_worktree_bin_clap_complete() {{
    # Generate and source clap completions
    local completion_script
    completion_script=$(worktree-bin completions zsh 2>/dev/null)
    
    if [[ -n "$completion_script" ]]; then
        # Temporarily define the clap completion
        eval "$completion_script"
        # Call the generated completion function
        _worktree-bin
    else
        # Fallback completion
        local -a subcommands
        subcommands=(
            'create:Create a new worktree'
            'list:List all worktrees'
            'remove:Remove a worktree'
            'status:Show worktree status'
            'sync-config:Sync config files between worktrees'
            'jump:Jump to a worktree directory'
            'init:Generate shell integration'
            'completions:Generate shell completions'
        )
        _describe 'commands' subcommands
    fi
}}

# Enhanced Zsh completion that triggers interactive mode on TAB
_worktree() {{
    local state line
    
    if [ ${{#words[@]}} -eq 2 ]; then
        # Use clap-generated completions for subcommands
        _worktree_bin_clap_complete
    elif [ "${{words[2]}}" = "jump" ]; then
        # Special handling for jump arguments
        if [ ${{#words[@]}} -eq 3 ] && [ -z "${{words[3]}}" ]; then
            # Launch interactive mode for empty jump completion
            worktree jump
            return 0
        fi
        
        # Check if current word is a flag
        if [[ "${{words[CURRENT]}}" == -* ]]; then
            # Use clap completions for flags
            _worktree_bin_clap_complete
        else
            # Use custom completion for worktree names
            local -a worktrees
            worktrees=($(worktree-bin jump --list-completions 2>/dev/null))
            _describe 'worktrees' worktrees
        fi
    else
        # Use clap-generated completions for other subcommands
        _worktree_bin_clap_complete
    fi
}}

compdef _worktree worktree"#
    );
}

fn print_fish_integration() {
    println!(
        r#"# Worktree shell integration for Fish
# This replaces the worktree command with a shell function that can change directories

function worktree
    switch $argv[1]
        case jump
            # Handle jump specially - call rust binary and cd to result
            set -e argv[1]
            set result
            if test (count $argv) -eq 0
                # Interactive mode
                set result (worktree-bin jump --interactive 2>/dev/null)
            else
                # Direct mode
                set result (worktree-bin jump $argv 2>/dev/null)
            end

            if test -n "$result"
                cd "$result"
            end
        case '*'
            # Delegate everything else to the rust binary
            worktree-bin $argv
    end
end

# Load clap-generated Fish completions
function _worktree_load_clap_completions
    # Generate clap completions and load them
    set -l completion_script (worktree-bin completions fish 2>/dev/null)
    if test -n "$completion_script"
        # Source the clap-generated completions
        echo "$completion_script" | source
    end
end

# Load the clap completions when this script is sourced
_worktree_load_clap_completions

# Enhanced Fish completion for worktree with custom jump logic
# Override the jump completion to add custom worktree name completion
complete -c worktree -n '__fish_seen_subcommand_from jump' -a '(worktree-bin jump --list-completions 2>/dev/null)' -d 'Available worktrees'

# Note: The clap-generated completions will handle all other subcommands and flags"#
    );
}
