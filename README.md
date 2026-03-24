# Worktree

A powerful CLI tool for managing git worktrees with enhanced features that simplify multitasking across multiple branches.

## What is `worktree`?

`worktree` solves the common problem of needing to work on multiple git branches simultaneously. Instead of constantly switching branches and losing your work context, `worktree` creates separate working directories for each feature while sharing the same git history.

Each worktree is identified by a **feature name** — a short, memorable label you choose (e.g. `auth`, `payment-system`). The feature name becomes the directory name and is independent of the branch name, so you can rename branches or reuse worktrees without losing your layout.

**Key Benefits:**

- **Organized Storage** - Keeps all worktrees in `~/.worktrees/<repo-name>/<feature-name>/`
- **Smart Config Management** - Automatically copies or symlinks important config files (`.env`, `.vscode`, etc.) to new worktrees
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

### 3. Install the Agent Skill (Optional)

If you use an AI coding agent (e.g. Claude Code), install the companion skill so your agent knows how to use `worktree` correctly:

```bash
worktree skill install
```

This installs a skill file into `~/.agents/skills/worktree-manager/` and creates a symlink at `~/.claude/skills/worktree-manager` for Claude Code to pick up automatically. The skill teaches your agent the correct commands, flags, and shell-wrapper behavior.

```bash
worktree skill status   # Check if installed and up to date
worktree skill update   # Update after upgrading the binary
worktree skill uninstall  # Remove if no longer needed
```

## Commands

| Command                        | Description                                                    |
| ------------------------------ | -------------------------------------------------------------- |
| `create <feature-name> [branch]` | Create a new worktree with the given feature name            |
| `list`                         | List all worktrees across all repositories                     |
| `jump [feature-name]`          | Switch to a worktree (interactive if no name specified)        |
| `switch [feature-name]`        | Alias for `jump`                                               |
| `remove [feature-name]`        | Remove a worktree (interactive if no name specified)           |
| `status`                       | Show detailed status of current worktree and branches          |
| `sync-config <from> <to>`      | Copy config files between worktrees                            |
| `back`                         | Return to the original repository                              |
| `cleanup`                      | Clean up orphaned worktree references                          |
| `skill <install\|uninstall\|update\|status>` | Manage the companion agent skill             |

## Interactive Features

### Tab Completion & Interactive Selection

- **`jump`/`switch`** without arguments opens an interactive worktree selector
- **`remove`** without arguments opens an interactive removal menu
- **Tab completion** shows available worktrees for `jump`, `switch`, and `remove` commands
- **Empty tab** in bash/zsh automatically launches interactive mode

### Autocomplete

The shell integration provides intelligent autocomplete:

- Command and flag completion for all subcommands
- Feature name completion for `jump`, `switch`, and `remove`
- Git reference completion for the `--from` flag on `create`
- Context-aware suggestions based on current repository

## Typical Workflow

`worktree` is designed for developers who need to multitask across different features, especially when working with LLM coding assistants:

### 1. Create Worktrees for Different Tasks

```bash
# Create a worktree named "auth" on branch feature/user-auth
worktree create auth feature/user-auth

# Create a worktree named "payments" on branch feature/payment-system
worktree create payments feature/payment-system

# Create a worktree named "security" branching from a specific point
worktree create security bugfix/security-patch --from main
```

If the branch already exists it will be reused; if it doesn't exist it will be created.

### 2. Jump Between Contexts

```bash
# Switch to auth feature (using jump or switch alias)
worktree jump auth
# or
worktree switch auth

# Work with your LLM assistant on authentication...

# Quickly switch to payment feature
worktree switch payments

# Work on payment system while auth context is preserved...
```

### 3. Manage and Clean Up

```bash
# See what you're working on
worktree list
worktree status

# Sync config changes from one worktree to another
worktree sync-config auth payments

# Remove a worktree — branch is preserved by default
worktree remove security

# Remove and also delete the branch
worktree remove auth --delete-branch

# Return to main repo
worktree back
```

### 4. Multitasking Benefits

- **Context Preservation** - Each worktree maintains its own files, git state, and development environment
- **LLM Agent Friendly** - Switch between features without losing conversation context or file states
- **Config Synchronization** - Important files (`.env`, `.vscode`, IDE settings) are automatically copied
- **Centralized Organization** - All worktrees live in `~/.worktrees/` for easy management

## Storage Organization

`worktree` organizes all worktrees in a centralized location keyed by feature name:

```
~/.worktrees/
├── my-project/
│   ├── auth/
│   ├── payments/
│   └── security/
└── another-project/
    ├── main/
    └── api-v2/
```

The directory name is always the feature name you provided — independent of the branch name checked out inside.

## Configuration

Create a `.worktree-config.toml` in your repository root to customize which files are copied or symlinked to new worktrees.

### Copy Patterns

Files matching these patterns are copied into each new worktree:

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
    "*.tmp"
]
```

### Symlink Patterns

Files matching these patterns are symlinked into each new worktree instead of copied. Edits in any worktree immediately affect the origin file — useful for secrets or shared tooling you never want to duplicate:

```toml
[symlink-patterns]
include = [
    ".env",
    "scripts/"
]
```

### Post-Create Hooks

Shell commands to run after a worktree is created. Commands run in the new worktree directory via `sh -c`. A failing command prints a warning and skips remaining hooks, but the worktree is still created:

```toml
[on-create]
commands = [
    "npm install",
    "cp .env.example .env.local"
]
```

### Flexible Configuration Options

You can specify only the patterns you want to customize. Your configuration merges with defaults using precedence rules:

```toml
# Add custom includes (merges with defaults)
[copy-patterns]
include = ["mise.toml", "docker-compose.yml"]
```

```toml
# Exclude something normally included by default
[copy-patterns]
exclude = [".vscode/"]
```

Your choices always override the defaults when there's a conflict.

### Default Patterns

If no config file exists, these patterns are used:

- **Included**: `.env*`, `.vscode/`, `*.local.json`, `config/local/*`
- **Excluded**: `node_modules/`, `target/`, `.git/`, `*.log`, `*.tmp`

## Advanced Features

### Config File Synchronization

Sync configuration changes between worktrees without manual copying:

```bash
# Copy config files from the auth worktree to the payments worktree
worktree sync-config auth payments

# Also accepts absolute paths
worktree sync-config ~/.worktrees/my-project/auth ~/.worktrees/my-project/payments
```

### Cleanup Operations

Remove orphaned git worktree references:

```bash
worktree cleanup
```

### Custom Storage Location

Override the default storage location with an environment variable:

```bash
export WORKTREE_STORAGE_ROOT=/path/to/custom/location
```
