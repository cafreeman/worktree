# Worktree CLI

A powerful CLI tool for managing git worktrees with enhanced features including centralized storage, automatic config file synchronization, and intelligent branch management.

## Features

- 🗂️ **Centralized Storage** - Organizes worktrees in `~/.worktrees/<repo-name>/<branch-name>/`
- ⚙️ **Smart Config Management** - Automatically copies gitignored config files to new worktrees
- 🔄 **Branch Synchronization** - Keeps worktrees and git branches in sync
- 📋 **Comprehensive Status** - Shows detailed worktree and branch status
- 🎯 **Configurable Patterns** - Customize which files to copy via `.worktree-config.toml`
- 🛡️ **Safe Branch Names** - Automatically sanitizes branch names with slashes and special characters

## Installation

```bash
cargo build --release
cp target/release/worktree /usr/local/bin/  # or add to PATH
```

## Quick Start

```bash
# Check current status
worktree status

# Create a new worktree for feature development
worktree create feature/auth

# Create a worktree with a new branch
worktree create -b new-feature

# List all worktrees
worktree list

# List worktrees for current repo only
worktree list --current

# Remove a worktree
worktree remove feature/auth

# Remove worktree and delete the branch
worktree remove feature/auth --delete-branch
```

## Commands

### `create` - Create a new worktree

```bash
worktree create <branch> [OPTIONS]
```

**Options:**
- `-p, --path <PATH>` - Custom path for the worktree (optional)
- `-b, --create-branch` - Create a new branch if it doesn't exist

**Examples:**
```bash
# Create worktree for existing branch
worktree create feature/login

# Create worktree with new branch
worktree create -b feature/new-thing

# Create worktree at custom location
worktree create feature/auth --path ~/custom/path
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
- `-d, --delete-branch` - Also delete the associated branch

**Examples:**
```bash
# Remove worktree by branch name
worktree remove feature/auth

# Remove worktree and delete branch
worktree remove feature/auth --delete-branch

# Remove worktree by path
worktree remove ~/.worktrees/my-project/feature-auth
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

### 1. Feature Development
```bash
# Start working on a new feature
worktree create -b feature/payments

# Work in the new worktree
cd ~/.worktrees/my-project/feature-payments

# When done, remove it
worktree remove feature/payments --delete-branch
```

### 2. Bug Fixes on Multiple Branches
```bash
# Create worktrees for different versions
worktree create release/v1.0
worktree create release/v2.0

# Apply fixes to both
# Config files are automatically synced
```

### 3. Code Review
```bash
# Create temporary worktree for PR review
worktree create pr-123

# Review code without affecting main workspace
cd ~/.worktrees/my-project/pr-123

# Clean up when done
worktree remove pr-123
```

### 4. Development Environment Management
```bash
# Sync updated config to all worktrees
worktree sync-config main feature/auth
worktree sync-config main bugfix/critical

# Check which worktrees need attention
worktree status
```

## Troubleshooting

### Worktree exists but not in git
```bash
worktree status  # Shows inconsistent state
# Remove the directory manually and recreate
```

### Config files not copying
1. Check `.worktree-config.toml` syntax
2. Verify file patterns match your files
3. Ensure files aren't excluded by exclude patterns

### Permission issues
```bash
# Ensure worktree directory is writable
chmod -R u+w ~/.worktrees/
```

## Advanced Usage

### Custom Storage Location

Set a custom worktree storage location by modifying the storage module or using environment variables (future enhancement).

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
2. Create a feature branch: `worktree create -b feature-name`
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## License

MIT License - see LICENSE file for details.