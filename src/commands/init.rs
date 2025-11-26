use clap::{Command, ValueEnum};
use clap_complete::{Shell as CompleteShell, generate};
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
        jump|switch)
            # Handle jump/switch specially - call rust binary and cd to result
            local cmd="$1"
            shift
            local result
            if [ $# -eq 0 ]; then
                # Interactive mode
                result=$(worktree-bin "$cmd" --interactive)
            else
                # Direct mode
                result=$(worktree-bin "$cmd" "$@")
            fi

            if [ -n "$result" ]; then
                cd "$result" || return 1
            fi
            ;;
        back)
            # Handle back specially - call rust binary and cd to result
            local result
            result=$(worktree-bin back)
            if [ -n "$result" ]; then
                cd "$result" || return 1
            fi
            ;;
        create)
            # Handle create specially - support interactive workflow
            if [ $# -eq 1 ]; then
                # No arguments provided - launch interactive workflow
                worktree-bin create
            else
                # Arguments provided - pass through normally
                worktree-bin "$@"
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

    # If we're completing the subcommand (COMP_CWORD == 1), delegate to clap
    if [ "$COMP_CWORD" -eq 1 ]; then
        # Delegate to clap for subcommand completion
        local saved_comp_words=("${{COMP_WORDS[@]}}")
        COMP_WORDS[0]="worktree-bin"
        _worktree_clap "worktree-bin" "$cur" "$prev"
        COMP_WORDS=("${{saved_comp_words[@]}}")
        return 0
    fi

    # Handle jump/switch subcommand specially
    if [ "${{COMP_WORDS[1]}}" = "jump" ] || [ "${{COMP_WORDS[1]}}" = "switch" ]; then
        # Trigger interactive mode on empty tab
        if [ "${{#COMP_WORDS[@]}}" -eq 3 ] && [ -z "$cur" ]; then
            worktree "${{COMP_WORDS[1]}}"
            return 0
        fi

        # Complete jump/switch command
        if [[ "$cur" == -* ]]; then
            # Complete flags for jump/switch
            COMPREPLY=($(compgen -W "--interactive --current --help" -- "$cur"))
        else
            # Complete worktree names
            local worktrees=$(worktree-bin "${{COMP_WORDS[1]}}" --list-completions 2>/dev/null)
            COMPREPLY=($(compgen -W "$worktrees" -- "$cur"))
        fi
    elif [ "${{COMP_WORDS[1]}}" = "remove" ]; then
        # Trigger interactive mode on empty tab
        if [ "${{#COMP_WORDS[@]}}" -eq 3 ] && [ -z "$cur" ]; then
            worktree remove --interactive
            return 0
        fi

        # Complete remove command
        if [[ "$cur" == -* ]]; then
            # Complete flags for remove
            COMPREPLY=($(compgen -W "--interactive --current --keep-branch --help" -- "$cur"))
        else
            # Complete worktree names
            local worktrees=$(worktree-bin remove --list-completions 2>/dev/null)
            COMPREPLY=($(compgen -W "$worktrees" -- "$cur"))
        fi
    elif [ "${{COMP_WORDS[1]}}" = "create" ]; then
        # Handle create command specially
        if [ "$prev" = "--from" ]; then
            # Get git references for --from flag completion
            local git_refs=$(worktree-bin create --list-from-completions 2>/dev/null)

            # Check if we got any references
            if [[ -z "$git_refs" ]]; then
                COMPREPLY=()
                return
            fi

            # Enable programmable completion with fuzzy matching
            # This allows partial matches anywhere in the string
            local IFS=$'\n'
            local filtered_refs=()

            if [[ -n "$cur" ]]; then
                # Filter refs that contain the current input (case-insensitive)
                while IFS= read -r ref; do
                    if [[ -n "$ref" && "${{ref,,}}" == *"${{cur,,}}"* ]]; then
                        filtered_refs+=("$ref")
                    fi
                done <<< "$git_refs"

                if [[ ${{#filtered_refs[@]}} -gt 0 ]]; then
                    COMPREPLY=($(printf '%s\n' "${{filtered_refs[@]}}" | head -20))
                else
                    COMPREPLY=($(printf '%s\n' $git_refs | head -20))
                fi
            else
                COMPREPLY=($(printf '%s\n' $git_refs | head -20))
            fi
        elif [[ "$cur" == -* ]]; then
            # Complete flags for create command
            COMPREPLY=($(compgen -W "--from --new-branch --existing-branch --interactive-from --help" -- "$cur"))
        else
            # Complete branch name argument (the first positional argument)
            # Check if we're completing the branch name (no branch argument provided yet)
            local has_branch_arg=false
            for ((i=2; i<${{#COMP_WORDS[@]}}-1; i++)); do
                if [[ "${{COMP_WORDS[i]}}" != -* ]] && [[ "${{COMP_WORDS[i-1]}}" != "--from" ]]; then
                    has_branch_arg=true
                    break
                fi
            done

            if [ "$has_branch_arg" = false ]; then
                # Complete branch names from git references
                local git_refs=$(worktree-bin create --list-from-completions 2>/dev/null)
                if [[ -n "$git_refs" ]]; then
                    local IFS=$'\n'
                    local filtered_refs=()

                    if [[ -n "$cur" ]]; then
                        while IFS= read -r ref; do
                            if [[ -n "$ref" && "${{ref,,}}" == *"${{cur,,}}"* ]]; then
                                filtered_refs+=("$ref")
                            fi
                        done <<< "$git_refs"

                        if [[ ${{#filtered_refs[@]}} -gt 0 ]]; then
                            COMPREPLY=($(printf '%s\n' "${{filtered_refs[@]}}" | head -20))
                        else
                            COMPREPLY=($(printf '%s\n' $git_refs | head -20))
                        fi
                    else
                        COMPREPLY=($(printf '%s\n' $git_refs | head -20))
                    fi
                fi
            fi
        fi
    else
        # For all other commands, delegate to clap completion
        local saved_comp_words=("${{COMP_WORDS[@]}}")
        COMP_WORDS[0]="worktree-bin"
        _worktree_clap "worktree-bin" "$cur" "$prev"
        COMP_WORDS=("${{saved_comp_words[@]}}")
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
        jump|switch)
            # Handle jump/switch specially - call rust binary and cd to result
            local cmd="$1"
            shift
            local result
            if [ $# -eq 0 ]; then
                # Interactive mode
                result=$(worktree-bin "$cmd" --interactive)
            else
                # Direct mode
                result=$(worktree-bin "$cmd" "$@")
            fi

            if [ -n "$result" ]; then
                cd "$result" || return 1
            fi
            ;;
        back)
            # Handle back specially - call rust binary and cd to result
            local result
            result=$(worktree-bin back)
            if [ -n "$result" ]; then
                cd "$result" || return 1
            fi
            ;;
        create)
            # Handle create specially - support interactive workflow
            if [ $# -eq 1 ]; then
                # No arguments provided - launch interactive workflow
                worktree-bin create
            else
                # Arguments provided - pass through normally
                worktree-bin "$@"
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

# Helper function for git reference completion
_worktree_git_refs() {{
    local -a all_refs local_branches remote_branches tags
    all_refs=($(worktree-bin create --list-from-completions 2>/dev/null))

    if [[ ${{#all_refs[@]}} -gt 0 ]]; then
        # Separate references by type
        for ref in "${{all_refs[@]}}"; do
            case "$ref" in
                origin/*)
                    remote_branches+=("$ref")
                    ;;
                v[0-9]*|*.[0-9]*|*-[0-9]*)
                    tags+=("$ref")
                    ;;
                *)
                    local_branches+=("$ref")
                    ;;
            esac
        done

        # Configure single-column display and fuzzy matching
        zstyle ':completion:*:git-references:*' list-grouped true
        zstyle ':completion:*:git-references:*' format '%B%F{{cyan}}%d%f%b'
        zstyle ':completion:*:git-references:*' matcher-list 'r:|[._-]=* r:|=*' 'l:|=* r:|=*'
        zstyle ':completion:*:git-references:*' list-packed false
        zstyle ':completion:*:git-references:*' list-columns 1

        # Present grouped completions
        if [[ ${{#local_branches[@]}} -gt 0 ]]; then
            _describe -t local-branches 'Local Branches' local_branches
        fi
        if [[ ${{#remote_branches[@]}} -gt 0 ]]; then
            _describe -t remote-branches 'Remote Branches' remote_branches
        fi
        if [[ ${{#tags[@]}} -gt 0 ]]; then
            _describe -t tags 'Tags' tags
        fi
    else
        _message 'no git references available'
    fi
}}

# Fallback function for when user types partial reference name
_worktree_git_refs_fallback() {{
    local -a all_refs
    all_refs=($(worktree-bin create --list-from-completions 2>/dev/null))

    if [[ ${{#all_refs[@]}} -gt 0 ]]; then
        _describe 'git references' all_refs
    else
        _message 'no git references available'
    fi
}}

# Create our custom _worktree function for the shell wrapper
_worktree() {{
    local line state context curcontext="$curcontext"
    typeset -A opt_args

    # If we're completing the subcommand (only 'worktree' has been typed), delegate to clap
    if [[ ${{#words[@]}} -eq 2 || -z "${{words[2]}}" ]]; then
        # Delegate to clap for subcommand completion
        local original_words=("${{words[@]}}")
        words[1]="worktree-bin"
        _worktree_clap "$@"
        local result=$?
        words=("${{original_words[@]}}")
        return $result
    fi

    case "${{words[2]}}" in
        jump|switch)
            # Handle jump/switch subcommand specially
            if [[ ${{#words[@]}} -le 3 && "${{words[CURRENT]}}" != -* ]]; then
                # Complete worktree names for jump/switch command
                local -a worktrees
                worktrees=($(worktree-bin "${{words[2]}}" --list-completions 2>/dev/null))
                if [[ ${{#worktrees[@]}} -gt 0 ]]; then
                    _describe 'worktrees' worktrees
                else
                    _message 'no worktrees available'
                fi
                return 0
            elif [[ "${{words[CURRENT]}}" == -* ]]; then
                # Complete flags for jump/switch command
                _arguments -s : \
                    '--interactive[Launch interactive selection mode]' \
                    '--current[Current repo only]' \
                    '--help[Print help]' \
                    '-h[Print help]'
                return 0
            fi
            ;;
        remove)
            # Handle remove subcommand specially
            if [[ ${{#words[@]}} -le 3 && "${{words[CURRENT]}}" != -* ]]; then
                # Complete worktree names for remove command
                local -a worktrees
                worktrees=($(worktree-bin remove --list-completions 2>/dev/null))
                if [[ ${{#worktrees[@]}} -gt 0 ]]; then
                    _describe 'worktrees' worktrees
                else
                    _message 'no worktrees available'
                fi
                return 0
            elif [[ "${{words[CURRENT]}}" == -* ]]; then
                # Complete flags for remove command
                _arguments -s : \
                    '--interactive[Launch interactive selection mode]' \
                    '--current[Current repo only]' \
                    '--keep-branch[Keep the branch (only remove the worktree)]' \
                    '--help[Print help]' \
                    '-h[Print help]'
                return 0
            fi
            ;;
        create)
            # Handle create subcommand with standard argument completion
            _arguments -s : \
                '--from=[Starting point for new branch]:FROM:_worktree_git_refs_fallback' \
                '--new-branch[Force creation of a new branch]' \
                '--existing-branch[Only use an existing branch]' \
                '--interactive-from[Launch interactive selection for --from reference]' \
                '--help[Print help]' \
                '-h[Print help]' \
                ':branch -- Branch name for the worktree:_worktree_git_refs_fallback'
            return 0
            ;;
        *)
            # For all other commands, delegate to clap completions
            local original_words=("${{words[@]}}")
            words[1]="worktree-bin"
            _worktree_clap "$@"
            local result=$?
            words=("${{original_words[@]}}")
            return $result
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
        case jump switch
            # Handle jump/switch specially - call rust binary and cd to result
            set cmd $argv[1]
            set -e argv[1]
            set result
            if test (count $argv) -eq 0
                # Interactive mode
                set result (worktree-bin $cmd --interactive)
            else
                # Direct mode
                set result (worktree-bin $cmd $argv)
            end

            if test -n "$result"
                cd "$result"
            end
        case back
            # Handle back specially - call rust binary and cd to result
            set result (worktree-bin back)
            if test -n "$result"
                cd "$result"
            end
        case create
            # Handle create specially - support interactive workflow
            if test (count $argv) -eq 1
                # No arguments provided - launch interactive workflow
                worktree-bin create
            else
                # Arguments provided - pass through normally
                worktree-bin $argv
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

# Override the jump, switch, and remove argument completions to add custom worktree names
complete -c worktree -n '__fish_seen_subcommand_from jump' -a '(worktree-bin jump --list-completions 2>/dev/null)' -d 'Available worktrees'
complete -c worktree -n '__fish_seen_subcommand_from switch' -a '(worktree-bin switch --list-completions 2>/dev/null)' -d 'Available worktrees'
complete -c worktree -n '__fish_seen_subcommand_from remove' -a '(worktree-bin remove --list-completions 2>/dev/null)' -d 'Available worktrees'

# Override the --from flag completion for create command
complete -c worktree -n '__fish_seen_subcommand_from create' -l from -a '(worktree-bin create --list-from-completions 2>/dev/null)' -d 'Git references'

# Add branch name completion for create command (positional argument)
# This completes the branch name when user types: worktree create <TAB>
complete -c worktree -n '__fish_seen_subcommand_from create; and not __fish_seen_subcommand_from (worktree-bin create --list-from-completions 2>/dev/null)' -a '(worktree-bin create --list-from-completions 2>/dev/null)' -d 'Branch name'

# The clap-generated completions handle all other subcommands and flags"#
    );
}
