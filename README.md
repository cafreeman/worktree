# Worktree CLI

A powerful CLI tool that transforms git worktree management from painful to effortless. Stop juggling multiple local repos, losing config files, or manually organizing worktree directories.

**The Problem:** Git worktrees are incredibly useful for parallel development, but the native git commands are cumbersome. You end up with worktrees scattered across your filesystem, config files that don't transfer, and no easy way to navigate between them.

**The Solution:** Worktree CLI provides a centralized, intelligent system that handles all the complexity for you. Create, manage, and navigate worktrees with simple commands while automatically maintaining your development environment.

## Why Use Worktree CLI?

- ⚡ **Zero Setup Friction** - Create worktrees anywhere in your project with one command
- 🏠 **Centralized & Organized** - All worktrees live in `~/.worktrees/<repo>/<branch>/` - no more scattered directories
- 🔄 **Config Sync Magic** - Your `.env`, `.vscode/`, and local config files automatically follow you to new worktrees
- 🧭 **Effortless Navigation** - Jump between worktrees instantly with smart completions and interactive selection
- 🔙 **Quick Return** - Navigate back to original repo from any worktree with `worktree back`
- 🧹 **Self-Cleaning** - Automatically cleans up orphaned branches and references to prevent git clutter
- 🔧 **Developer-Friendly** - Shell integration with intelligent tab completions and directory changing
- 🛡️ **Handles Edge Cases** - Safe branch name sanitization, git config inheritance, and sync state management

## Installation

> **⚠️ Important:** This tool requires shell integration to function properly. The `worktree jump` and `worktree back` commands won't work without it, and you'll miss out on intelligent tab completions. Make sure to complete both installation steps below.

### 1. Install from crates.io

```bash
# Install the latest version from crates.io
cargo install worktree
```

This will install the `worktree-bin` binary to your cargo bin directory (typically `~/.cargo/bin/`). Make sure this directory is in your PATH.

### 2. Set Up Shell Integration with Completions

**Important:** The `worktree` command is a shell function that wraps `worktree-bin` to enable directory changing and provides enhanced tab completions automatically. Without this integration, `worktree jump` and `worktree back` won't be able to change your current directory.

Add the following to your shell configuration:

#### Bash

Add to your `~/.bashrc` or `~/.bash_profile`:

```bash
# Generate and source worktree shell integration with completions
eval "$(worktree-bin init bash)"
```

#### Zsh

Add to your `~/.zshrc`:

```bash
# Generate and source worktree shell integration with completions
eval "$(worktree-bin init zsh)"
```

#### Fish

Add to your Fish config (`~/.config/fish/config.fish`):

```fish
# Generate and source worktree shell integration with completions
worktree-bin init fish | source
```

**Note:** The shell integration provides sophisticated tab completions that enhance your workflow:

**Command & Flag Completion:**

- All subcommands (`create`, `list`, `jump`, etc.) with intelligent suggestions
- All flags and options with descriptions (powered by clap)
- Context-aware completion based on your current command

**Dynamic Worktree Completion:**

- Live completion of worktree names for `worktree jump`
- Fuzzy matching - type partial names and get suggestions
- Pressing TAB on empty `worktree jump` triggers interactive selection
- Completion respects `--current` flag to show only current repository worktrees

**Smart Navigation:**

- Shell integration enables `worktree jump` and `worktree back` to actually change directories
- All other commands are delegated to the binary while maintaining completion support

### 3. Reload Your Shell

```bash
# Reload your shell configuration
source ~/.bashrc   # for bash
source ~/.zshrc    # for zsh
# or restart your terminal
```

### Verify Installation

```bash
# Test that worktree command is available
worktree --help

# Test shell integration works
worktree status
```

## Quick Start

```bash
# Check what's currently set up
worktree status

# Create a worktree for an existing branch
worktree create feature/auth

# Create a worktree with a brand new branch
worktree create --new-branch feature/payments

# Jump between worktrees instantly
worktree jump feature/auth

# Use interactive selection when you can't remember the name
worktree jump --interactive

# Navigate back to the original repo from any worktree
worktree back

# See all your worktrees
worktree list

# Clean up when branches get out of sync
worktree cleanup

# Remove a worktree when you're done (deletes branch by default)
worktree remove feature/auth

# Keep the branch but remove the worktree
worktree remove feature/payments --keep-branch
```

## Commands

| Command                   | Description                               | Key Options                         |
| ------------------------- | ----------------------------------------- | ----------------------------------- |
| `create <branch>`         | Create a new worktree                     | `--new-branch`, `--existing-branch` |
| `list`                    | List all worktrees                        | `--current`                         |
| `remove <target>`         | Remove a worktree                         | `--keep-branch`                     |
| `status`                  | Show worktree status                      | -                                   |
| `sync-config <from> <to>` | Sync config files between worktrees       | -                                   |
| `jump [target]`           | Navigate to a worktree directory          | `--interactive`, `--current`        |
| `back`                    | Navigate back to original repository      | -                                   |
| `cleanup`                 | Clean up orphaned branches and references | -                                   |
| `completions <shell>`     | Generate shell completions                | -                                   |
| `init <shell>`            | Generate shell integration                | -                                   |

### `create` - Create a new worktree

```bash
worktree create <branch> [OPTIONS]
```

**Options:**

- `--new-branch` - Force creation of a new branch (fail if it already exists)
- `--existing-branch` - Only use an existing branch (fail if it doesn't exist)

**Examples:**

```bash
# Create worktree for existing branch
worktree create feature/login

# Create worktree with new branch
worktree create --new-branch feature/new-thing
```

### `list` - List all worktrees

```bash
worktree list [OPTIONS]
```

**Options:**

- `--current` - Show worktrees for current repo only

**Examples:**

```bash
# List all managed worktrees
worktree list

# List worktrees for current repository
worktree list --current
```

### `remove` - Remove a worktree

```bash
worktree remove <target> [OPTIONS]
```

**Options:**

- `--keep-branch` - Keep the branch (only remove the worktree, branch deleted by default)

**Examples:**

```bash
# Remove worktree by branch name
worktree remove feature/auth

# Remove worktree (deletes branch by default)
worktree remove feature/auth

# Remove worktree but keep the branch
worktree remove feature/auth --keep-branch
```

### `status` - Show worktree status

```bash
worktree status
```

Displays comprehensive information about:

- Git worktrees vs managed worktrees
- Directory existence status
- Synchronization state
- Repository information

### `sync-config` - Sync config files between worktrees

```bash
worktree sync-config <from> <to>
```

**Examples:**

```bash
# Sync config from main to feature branch
worktree sync-config main feature/auth

# Sync using paths
worktree sync-config ~/.worktrees/project/main ~/.worktrees/project/feature
```

### `jump` - Navigate to a worktree directory

```bash
worktree jump [target] [OPTIONS]
```

**Options:**

- `--interactive` - Launch interactive selection mode
- `--current` - Show worktrees for current repo only

**Examples:**

```bash
# Jump to a specific worktree
worktree jump feature/auth

# Interactive selection (also triggered by pressing TAB on empty jump)
worktree jump --interactive

# Jump with tab completion - type partial name and press TAB
worktree jump feat<TAB>  # completes to available worktrees

# Current repo worktrees only
worktree jump --current
```

### `cleanup` - Clean up orphaned branches and worktree references

```bash
worktree cleanup
```

Automatically cleans up your workspace by:

- Removing git branches that have no corresponding worktree directory
- Cleaning up branch mappings for non-existent worktrees
- Removing git worktree references that point to non-existent directories

This command is useful when worktrees get out of sync due to manual deletion or filesystem issues.

**Examples:**

```bash
# Clean up orphaned branches and references
worktree cleanup
```

### `back` - Navigate back to the original repository

```bash
worktree back
```

Navigates back to the original repository directory that the current worktree was created from. This command only works when executed from within a worktree directory that was created using `worktree create`.

**Examples:**

```bash
# From within a worktree, return to the original repo
worktree back
```

### `completions` - Generate shell completions

```bash
worktree completions <SHELL>
```

Generates native shell completions for the specified shell. This is separate from the integrated completions provided by `worktree init`.

**Options:**

- `<SHELL>` - Shell to generate completions for (bash, zsh, fish)

**Examples:**

```bash
# Generate completions for bash
worktree completions bash > /usr/local/etc/bash_completion.d/worktree

# Generate completions for zsh
worktree completions zsh > ~/.local/share/zsh/site-functions/_worktree
```

## Configuration

### `.worktree-config.toml`

Create a `.worktree-config.toml` file in your repository root to customize which files are copied to new worktrees:

```toml
[copy-patterns]
include = [
    ".env*",
    ".vscode/",
    "*.local.json",
    "config/local/*",
    ".idea/",
    "docker-compose.override.yml"
]
exclude = [
    "node_modules/",
    "target/",
    ".git/",
    "*.log",
    "*.tmp",
    "dist/",
    "build/"
]
```

**Default patterns (if no config file exists):**

Include:

- `.env*` - Environment files
- `.vscode/` - VS Code settings
- `*.local.json` - Local configuration files
- `config/local/*` - Local config directories

Exclude:

- `node_modules/`, `target/` - Build artifacts
- `.git/` - Git directory
- `*.log`, `*.tmp` - Temporary files

## Storage Organization

Worktrees are organized in a clean, predictable structure:

```
~/.worktrees/
├── my-project/
│   ├── main/
│   ├── feature-auth/          # branch: feature/auth
│   ├── bugfix-login/          # branch: bugfix/login
│   └── develop/
├── another-repo/
│   ├── main/
│   └── feature-xyz/           # branch: feature/xyz
└── third-project/
    └── experimental/
```

**Branch Name Sanitization:**
Branch names containing slashes and special characters are automatically sanitized for safe filesystem storage:

- `feature/auth` → `feature-auth/`
- `bugfix/critical-issue` → `bugfix-critical-issue/`
- `release/v1.0` → `release-v1.0/`

The original branch names are preserved and displayed in all commands.

## Use Cases

### 1. Feature Development Workflow

```bash
# Start working on a new feature (from main repo)
worktree create --new-branch feature/payments

# Jump to the new worktree instantly
worktree jump feature/payments

# Your .env, .vscode/, and config files are already there!
# Work on your feature...

# Jump back to main when needed
worktree back

# When done, remove it (deletes branch by default)
worktree remove feature/payments
```

### 2. Parallel Development

```bash
# Work on multiple features simultaneously
worktree create --new-branch feature/auth
worktree create --new-branch feature/dashboard
worktree create bugfix/critical-issue

# Jump between them effortlessly
worktree jump auth           # Tab completion works!
worktree jump dashboard
worktree jump critical

# Or use interactive selection
worktree jump --interactive
```

### 3. Code Review & Testing

```bash
# Create temporary worktree for PR review
worktree create pr-123

# Jump to review (config already synced)
worktree jump pr-123

# Test the changes, then return to your work
worktree back

# Clean up when done
worktree remove pr-123
```

### 4. Maintenance & Cleanup

```bash
# Regular maintenance - clean up orphaned branches
worktree cleanup

# Check what's currently active
worktree status

# List all your worktrees across projects
worktree list
```

## Troubleshooting

### Worktree Commands Don't Change Directory

**Problem:** `worktree jump` or `worktree back` doesn't change your current directory.

**Solution:** You need to set up shell integration. The binary alone cannot change the shell's directory.

```bash
# Add to your shell profile (.bashrc, .zshrc, etc.)
eval "$(worktree-bin init bash)"  # or zsh/fish
source ~/.bashrc  # reload your shell
```

### Sync Issues Between Git and Filesystem

**Problem:** `worktree status` shows inconsistent state between git worktrees and directories.

**Solution:** Use the cleanup command to automatically fix sync issues.

```bash
worktree cleanup  # Removes orphaned branches and references
worktree status   # Verify everything is clean
```

### Tab Completion Not Working

**Problem:** Tab completion for `worktree jump` doesn't show worktree names.

**Solutions:**

1. Ensure shell integration is set up (see above)
2. Check that `worktree-bin` is in your PATH
3. Restart your shell after setup

### Config Files Not Copying

**Problem:** Your `.env` or config files aren't appearing in new worktrees.

**Solutions:**

1. Check `.worktree-config.toml` syntax in your repo root
2. Verify file patterns match your files (use `*` for wildcards)
3. Ensure files aren't excluded by exclude patterns
4. Check that source files exist and aren't gitignored

```bash
# Debug: see what patterns are being used
worktree create --new-branch test-config
# Check if files copied to ~/.worktrees/repo/test-config/
```

### Permission Issues

**Problem:** Cannot create or access worktree directories.

**Solutions:**

```bash
# Ensure worktree directory is writable
chmod -R u+w ~/.worktrees/

# Check disk space
df -h ~/.worktrees/

# Verify directory ownership
ls -la ~/.worktrees/
```

### Back Navigation Not Working

**Problem:** `worktree back` says no origin information available.

**Solution:** This affects worktrees created before the back feature was added.

```bash
# Recreate the worktree to enable back navigation
worktree remove old-worktree --keep-branch
worktree create old-worktree
```

## Advanced Usage

### Git Configuration Inheritance

When creating a new worktree, the tool automatically inherits git configuration from the parent repository. This includes:

- User name and email settings
- Custom git aliases and configurations
- Repository-specific settings
- Credential helpers and authentication settings

This ensures that your git workflow remains consistent across all worktrees without manual configuration.

### Origin Tracking and Back Navigation

Each worktree stores metadata about its origin repository, enabling seamless navigation:

- **Automatic origin tracking**: When creating a worktree, the tool stores the path to the original repository
- **Smart back navigation**: Use `worktree back` from any worktree to return to the original repo
- **Cross-platform paths**: Handles path canonicalization and symlinks correctly (e.g., `/var` → `/private/var` on macOS)

This feature is particularly useful when working with multiple projects or when you need to quickly return to the main repository.

### Integration with IDEs

The consistent storage structure makes it easy to:

- Configure IDE project templates
- Set up automated workflows
- Create shell aliases for common operations

### Shell Aliases

Add these to your shell profile for convenience:

```bash
alias wt='worktree'
alias wtc='worktree create'
alias wtl='worktree list --current'
alias wts='worktree status'
```

## Contributing

1. Fork the repository
2. Create a feature branch: `worktree create --new-branch feature-name`
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## License

MIT License - see LICENSE file for details.
