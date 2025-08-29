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

    generate(
        clap_shell,
        cmd,
        cmd.get_name().to_string(),
        &mut io::stdout(),
    );
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
                result=$(worktree-bin jump --interactive)
            else
                # Direct mode
                result=$(worktree-bin jump "$@")
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

# Load clap-generated completions
_worktree_clap_available=false
if command -v worktree-bin >/dev/null 2>&1; then
    # Load clap completions and rename function to avoid conflicts
    eval "$(worktree-bin completions bash 2>/dev/null)"
    if declare -F _worktree >/dev/null 2>&1; then
        eval "$(declare -f _worktree | sed 's/_worktree/_worktree_clap/')"
        unset -f _worktree
        _worktree_clap_available=true
    fi
fi

# Enhanced completion for the worktree shell function  
_worktree_complete() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    local prev="${{COMP_WORDS[COMP_CWORD-1]}}"

    # Handle jump subcommand specially
    if [ "${{COMP_WORDS[1]}}" = "jump" ]; then
        # Trigger interactive mode on empty tab
        if [ "${{#COMP_WORDS[@]}}" -eq 3 ] && [ -z "$cur" ]; then
            worktree jump
            return 0
        fi
        
        # Complete jump command
        if [[ "$cur" == -* ]]; then
            # Complete flags for jump
            COMPREPLY=($(compgen -W "--interactive --current --help" -- "$cur"))
        else
            # Complete worktree names
            local worktrees=$(worktree-bin jump --list-completions 2>/dev/null)
            COMPREPLY=($(compgen -W "$worktrees" -- "$cur"))
        fi
    else
        # For all other commands, delegate to clap completion if available
        if [ "$_worktree_clap_available" = "true" ] && declare -F _worktree_clap >/dev/null 2>&1; then
            # Temporarily modify COMP_WORDS to make it look like worktree-bin
            local saved_comp_words=("${{COMP_WORDS[@]}}")
            COMP_WORDS[0]="worktree-bin"
            _worktree_clap
            COMP_WORDS=("${{saved_comp_words[@]}}")
        else
            # Fallback to basic completion
            COMPREPLY=($(compgen -W "create list remove status sync-config jump init completions cleanup --help --version" -- "$cur"))
        fi
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
                result=$(worktree-bin jump --interactive)
            else
                # Direct mode
                result=$(worktree-bin jump "$@")
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

# Load clap-generated completions
_worktree_clap_available=false
if command -v worktree-bin >/dev/null 2>&1; then
    # Load clap completions but strip the problematic conditional registration at the end
    # Using a function to avoid 'local' at the top level which prints during sourcing
    __worktree_load_completions() {{
        local clap_completion
        clap_completion="$(worktree-bin completions zsh 2>/dev/null | sed '/^if \[ "$funcstack\[1\]" = "_worktree" \]; then/,/^fi$/d')"
        if [[ -n "$clap_completion" ]]; then
            eval "$clap_completion"
            if (( $+functions[_worktree] )); then
                functions[_worktree_clap]=${{functions[_worktree]}}
                unfunction _worktree
                _worktree_clap_available=true
            fi
        fi
    }}
    __worktree_load_completions
    unfunction __worktree_load_completions
fi

# Create our custom _worktree function for the shell wrapper
_worktree() {{
    local line state context curcontext="$curcontext"
    typeset -A opt_args
    
    case "${{words[2]}}" in
        jump)
            # Handle jump subcommand specially
            if [[ ${{#words[@]}} -le 3 && "${{words[CURRENT]}}" != -* ]]; then
                # Complete worktree names for jump command
                local -a worktrees
                worktrees=($(worktree-bin jump --list-completions 2>/dev/null))
                if [[ ${{#worktrees[@]}} -gt 0 ]]; then
                    _describe 'worktrees' worktrees
                else
                    _message 'no worktrees available'
                fi
                return 0
            elif [[ "${{words[CURRENT]}}" == -* ]]; then
                # Complete flags for jump command
                _arguments -s : \
                    '--interactive[Launch interactive selection mode]' \
                    '--current[Current repo only]' \
                    '--help[Print help]' \
                    '-h[Print help]'
                return 0
            fi
            ;;
        *)
            # For all other commands, delegate to clap completions if available
            if [[ "$_worktree_clap_available" = "true" ]]; then
                # Modify the first word to be worktree-bin for delegation
                local original_words=("${{words[@]}}")
                words[1]="worktree-bin"
                _worktree_clap "$@"
                local result=$?
                words=("${{original_words[@]}}")
                return $result
            else
                # Fallback: basic subcommand completion
                if [[ ${{#words[@]}} -eq 2 ]]; then
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
                        'cleanup:Clean up orphaned branches and worktree references'
                    )
                    _describe 'worktree commands' subcommands
                    return 0
                fi
            fi
            ;;
    esac
}}

# Register the completion (only if compinit has been called)
if (( $+functions[compdef] )); then
    compdef _worktree worktree
fi"#
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
                set result (worktree-bin jump --interactive)
            else
                # Direct mode
                set result (worktree-bin jump $argv)
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
if command -q worktree-bin
    eval (worktree-bin completions fish 2>/dev/null)
end

# Override only the jump argument completion to add custom worktree names
complete -c worktree -n '__fish_seen_subcommand_from jump' -a '(worktree-bin jump --list-completions 2>/dev/null)' -d 'Available worktrees'

# The clap-generated completions handle all other subcommands and flags"#
    );
}
