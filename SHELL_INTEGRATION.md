# Shell Integration for Worktree Navigation

This document explains how to set up and use the shell integration for seamless worktree navigation.

## Quick Setup

1. **Install the binary** (if not already done):

   ```bash
   cargo install --path .
   ```

2. **Generate and install shell completions** (optional but recommended):

   ```bash
   # For Bash
   worktree-bin completions bash > ~/.local/share/bash-completion/completions/worktree-bin

   # For Zsh
   worktree-bin completions zsh > ~/.zfunc/_worktree-bin

   # For Fish
   worktree-bin completions fish > ~/.config/fish/completions/worktree-bin.fish
   ```

3. **Add shell integration** to your shell configuration:

   **For Bash** (`~/.bashrc`):

   ```bash
   eval "$(worktree-bin init bash)"
   ```

   **For Zsh** (`~/.zshrc`):

   ```bash
   eval "$(worktree-bin init zsh)"
   ```

   **For Fish** (`~/.config/fish/config.fish`):

   ```bash
   worktree-bin init fish | source
   ```

4. **Reload your shell**:
   ```bash
   source ~/.bashrc    # or ~/.zshrc
   ```

## Usage

Once set up, you can use the familiar `worktree` command with enhanced navigation:

### Interactive Jump

```bash
worktree jump
# Opens interactive selection with arrow keys
# ? Jump to worktree:
# > myrepo/feature-branch (/path/to/worktree)
#   myrepo/main          (/path/to/main)
#   otherrepo/dev        (/path/to/dev)
```

### Direct Jump with Autocomplete

```bash
worktree jump feat<TAB>    # Autocompletes available worktrees
worktree jump feature-branch    # Jumps directly
```

### Interactive on Empty Tab

```bash
worktree jump<TAB>    # Automatically opens interactive selection!
```

### All Other Commands Work Normally

```bash
worktree create new-feature
worktree list
worktree remove old-feature
worktree status
worktree sync-config source target
worktree completions bash  # Generate completions
```

### Shell Completions

```bash
# Generate completions for your shell
worktree-bin completions bash    # For bash
worktree-bin completions zsh     # For zsh
worktree-bin completions fish    # For fish

# Use with shell integration for enhanced experience
worktree completions bash        # Also works through shell function
```

## How It Works

The shell integration creates a `worktree()` function that:

- **Intercepts `jump` commands** and handles directory changing
- **Delegates all other commands** to the underlying `worktree-bin` binary
- **Provides smart autocompletion** for worktree names
- **Triggers interactive mode** on empty tab completion

## Features

- ✅ **Natural interface**: Use `worktree jump` as expected
- ✅ **Interactive selection**: Built-in fuzzy selection with arrow keys
- ✅ **Smart autocompletion**: Tab completion for worktree names
- ✅ **Multiple shells**: Bash, Zsh, and Fish support
- ✅ **Backwards compatible**: All existing commands work unchanged
- ✅ **No conflicts**: No new command names to remember
- ✅ **Native completions**: Full shell completion support for all commands
- ✅ **Dual access**: Use either `worktree` or `worktree-bin` directly

## Troubleshooting

### Binary not found

If you get "worktree-bin: command not found", ensure the binary is in your PATH:

```bash
which worktree-bin
```

### Shell integration not working

1. Check that you've reloaded your shell after adding the eval line
2. Verify the integration was added by running: `type worktree`
3. It should show "worktree is a function" instead of a binary path

### Interactive mode not working

Make sure the `inquire` dependency is available by rebuilding:

```bash
cargo build --release
```

## Advanced Usage

### Current repo only

```bash
worktree jump --current feature-branch
```

### Force interactive mode

```bash
worktree jump --interactive
```

### List available completions (for debugging)

```bash
worktree-bin jump --list-completions
```

### Generate completions for installation

```bash
# Install system-wide completions
sudo worktree-bin completions bash > /etc/bash_completion.d/worktree-bin
sudo worktree-bin completions zsh > /usr/share/zsh/site-functions/_worktree-bin

# Or user-specific
worktree-bin completions bash > ~/.local/share/bash-completion/completions/worktree-bin
```
