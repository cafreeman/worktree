# Worktree

A powerful CLI tool for managing git worktrees with enhanced features that simplify multitasking across multiple branches.

## What is `worktree`?

`worktree` solves the common problem of needing to work on multiple git branches simultaneously. Instead of constantly switching branches and losing your work context, `worktree` creates separate working directories for each branch while sharing the same git history.

**Key Benefits:**

- **Organized Storage** - Keeps all worktrees in `~/.worktrees/<repo-name>/<branch-name>/`
- **Smart Config Management** - Automatically copies important config files (`.env`, `.vscode`, etc.) to new worktrees
- **Seamless Navigation** - Jump between worktrees instantly with interactive selection
- **Perfect for LLM Workflows** - Work on multiple features simultaneously without losing context

## Installation

### 1. Install the Binary

```bash
cargo install worktree
```

### 2. Set Up Shell Integration

**Important:** The `worktree` command is a shell function that wraps `worktree-bin` to enable directory changing and provides enhanced tab completions automatically. Without this integration, `worktree jump`/`worktree switch` and `worktree back` won't be able to change your current directory.

Add the following to your shell configuration:

#### Bash

```bash
# Add to ~/.bashrc
eval "$(worktree-bin init bash)"
```

#### Zsh

```bash
# Add to ~/.zshrc
eval "$(worktree-bin init zsh)"
```

#### Fish

```bash
# Add to ~/.config/fish/config.fish
worktree-bin init fish | source
```

## Commands

| Command                   | Description                                               |
| ------------------------- | --------------------------------------------------------- |
| `create <branch>`         | Create a new worktree for the specified branch            |
| `list`                    | List all worktrees across all repositories                |
| `jump [branch]`           | Switch to a worktree (interactive if no branch specified) |
| `switch [branch]`         | Alias for `jump` - switch to a worktree                   |
| `remove [branch]`         | Remove a worktree (interactive if no branch specified)    |
| `status`                  | Show detailed status of current worktree and branches     |
| `sync-config <from> <to>` | Copy config files between worktrees                       |
| `back`                    | Return to the original repository                         |
| `cleanup`                 | Clean up orphaned worktree references                     |

## Interactive Features

### Tab Completion & Interactive Selection

- **`jump`/`switch`** without arguments opens an interactive worktree selector
- **`remove`** without arguments opens an interactive removal menu
- **Tab completion** shows available worktrees for `jump`, `switch`, and `remove` commands
- **Empty tab** in bash/zsh automatically launches interactive mode

### Autocomplete

The shell integration provides intelligent autocomplete:

- Command and flag completion for all subcommands
- Worktree name completion for `jump` and `remove`
- Context-aware suggestions based on current repository

## Typical Workflow

`worktree` is designed for developers who need to multitask across different features, especially when working with LLM coding assistants:

### 1. Create Worktrees for Different Tasks

```bash
# Create worktrees for different features
worktree create feature/user-auth
worktree create feature/payment-system
worktree create bugfix/security-patch
```

### 2. Jump Between Contexts

```bash
# Switch to auth feature (using jump or switch alias)
worktree jump feature/user-auth
# or
worktree switch feature/user-auth

# Work with your LLM assistant on authentication...

# Quickly switch to payment feature
worktree switch feature/payment-system

# Work on payment system while auth context is preserved...
```

### 3. Manage and Clean Up

```bash
# See what you're working on
worktree list
worktree status

# Sync config changes between worktrees
worktree sync-config feature/user-auth feature/payment-system

# Clean up completed features
# By default, branch will be deleted only if it was created by this CLI
worktree remove feature/user-auth

# To keep the branch
worktree remove feature/user-auth --keep-branch

# To force-delete an unmanaged branch (e.g., created outside this CLI)
worktree remove feature/user-auth --force-delete-branch

# Return to main repo
worktree back
```

### 4. Multitasking Benefits

- **Context Preservation** - Each worktree maintains its own files, git state, and development environment
- **LLM Agent Friendly** - Switch between features without losing conversation context or file states
- **Config Synchronization** - Important files (`.env`, `.vscode`, IDE settings) are automatically copied
- **Centralized Organization** - All worktrees live in `~/.worktrees/` for easy management

## Storage Organization

`worktree` organizes all worktrees in a centralized location:

```
~/.worktrees/
├── my-project/
│   ├── main/
│   ├── feature-auth/
│   └── bugfix-security-patch/
└── another-project/
    ├── main/
    └── feature-api/
```

### Branch Name Sanitization

Branch names with special characters are automatically sanitized for filesystem safety:

- `feature/user-auth` → `feature-user-auth`
- `hotfix/security:patch` → `hotfix-security-patch`
- Original branch names are preserved and mapped back correctly

## Configuration

Create a `.worktree-config.toml` in your repository root to customize which files are copied to new worktrees. The configuration system is flexible and supports partial configurations that merge with sensible defaults.

### Basic Configuration

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

### Flexible Configuration Options

You can specify only the patterns you want to customize. Your configuration merges with defaults using precedence rules:

```toml
# Add custom includes (merges with defaults)
[copy-patterns]
include = ["mise.toml", "docker-compose.yml"]
# Result: Default includes + custom includes + default excludes
```

```toml
# Add custom excludes (merges with defaults)
[copy-patterns]
exclude = ["*.secret", "private/"]
# Result: Default includes + default excludes + custom excludes
```

### Precedence Rules

Your configuration wins when there are conflicts:

```toml
# Include something normally excluded by default
[copy-patterns]
include = ["node_modules/.cache"]
# Result: Default includes + node_modules/.cache + default excludes
# (node_modules/.cache gets included despite node_modules/ being excluded)
```

```toml
# Exclude something normally included by default
[copy-patterns]
exclude = [".vscode/"]
# Result: Default includes + default excludes + .vscode/
# (.vscode/ gets excluded despite being included by default)
```

This approach is simple and intuitive - your choices always override the defaults when there's a conflict.

### Default Patterns

If no config file exists, these patterns are used:

- **Included**: `.env*`, `.vscode/`, `*.local.json`, `config/local/*`
- **Excluded**: `node_modules/`, `target/`, `.git/`, `*.log`, `*.tmp`

## Advanced Features

### Config File Synchronization

Sync configuration changes between worktrees without manual copying:

```bash
# Copy config files from one worktree to another
worktree sync-config feature/auth feature/payment
```

### Cleanup Operations

Remove orphaned references and clean up unused worktrees:

```bash
# Clean up all orphaned worktree references
worktree cleanup
```

### Custom Storage Location

Override the default storage location with an environment variable:

```bash
export WORKTREE_STORAGE_ROOT=/path/to/custom/location
```
