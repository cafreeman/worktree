---
name: worktree-manager
description: >
  Use this skill for any task involving the `worktree` CLI tool — a shell wrapper for
  managing git worktrees with centralized storage under ~/.worktrees/. Invoke it when the
  user wants to: create a new worktree (from a branch, tag, or commit), navigate between
  worktrees (jump/switch/back), list or remove worktrees, clean up orphaned refs, sync
  config files between worktrees, or configure .worktree-config.toml (copy-patterns,
  symlink-patterns, on-create hooks). Also invoke when the user is confused about why
  `worktree-bin jump` doesn't change their directory (the shell wrapper is required), or
  when they ask how the tool works in general. Prefer this skill over general git advice
  whenever the user mentions "worktree", working in ~/.worktrees/, or managing parallel
  feature branches with isolation.
---

# Worktree Manager Skill

This skill guides you in using the `worktree` shell CLI to manage git worktrees with
centralized storage, config syncing, and back navigation.

## Critical: Use the Shell Wrapper, Not the Binary

**Always use `worktree` (the shell function), never `worktree-bin` (the Rust binary directly).**

The shell wrapper is required because:
- `worktree jump`, `worktree switch`, and `worktree back` need to change the shell's
  working directory — the wrapper captures the output path and runs `cd` for you
- Calling `worktree-bin jump` directly will print the path but NOT change your directory

The shell wrapper is activated by adding this to your shell config:
- Bash/Zsh: `eval "$(worktree-bin init bash)"` or `eval "$(worktree-bin init zsh)"`
- Fish: `worktree-bin init fish | source`

## Key Concepts

**Feature name**: The identity of a worktree — a user-supplied name that becomes the
directory name in `~/.worktrees/<repo>/<feature-name>/`. Independent of the branch name.
You can have multiple worktrees pointing to different branches, all named by their purpose.

**Branch name**: The git branch in the worktree. Defaults to the feature name if not specified.

**Storage**: All worktrees live under `~/.worktrees/<repo-name>/`. Override with
`$WORKTREE_STORAGE_ROOT`.

## Commands Reference

### `worktree create [feature-name] [branch]`

Create a new worktree. The feature name is your identifier; the branch is optional.

```bash
# Full explicit
worktree create auth-redesign feature/auth-v2

# Feature name only (branch defaults to feature name)
worktree create auth-redesign

# From a specific base ref (branch, tag, or commit)
worktree create hotfix bugfix/critical --from v1.2.3

# Fully interactive (prompts for everything)
worktree create
```

Flags:
- `--from <ref>` — Base ref (branch, tag, commit) to create branch from
- `--interactive-from` — Pick base ref interactively

After creation, files matching `.worktree-config.toml` patterns are copied/symlinked and
`on-create` hooks are run automatically.

### `worktree list [--current]`

List all worktrees. Shows feature name, branch, and status.

```bash
worktree list                  # All worktrees across all repos
worktree list --current        # Only worktrees for the current repo
```

### `worktree jump [feature-name]` / `worktree switch [feature-name]`

Navigate to a worktree (changes your working directory). `switch` is an alias for `jump`.

```bash
worktree jump auth-redesign    # Jump directly
worktree jump                  # Interactive picker
worktree switch payments       # Same as jump
```

Flags:
- `--interactive` — Force interactive selection
- `--current` — Only show worktrees for the current repo

### `worktree back`

Return to the original repository from a worktree. No arguments needed.

```bash
worktree back
```

### `worktree remove [feature-name]`

Remove a worktree. By default, the branch is preserved.

```bash
worktree remove auth-redesign              # Remove, keep branch
worktree remove auth-redesign --delete-branch  # Remove and delete branch
worktree remove                            # Interactive picker
```

Flags:
- `--delete-branch` — Also delete the git branch
- `--interactive` — Force interactive selection
- `--current` — Only show worktrees for the current repo

### `worktree status`

Show detailed status of the current worktree including git alignment info.

```bash
worktree status
```

### `worktree sync-config <from> <to>`

Copy config files (as defined in `.worktree-config.toml`) from one worktree to another.
Accepts feature names or absolute paths.

```bash
worktree sync-config auth-redesign payments
```

### `worktree cleanup`

Remove orphaned git worktree references (worktrees that were deleted without proper cleanup).

```bash
worktree cleanup
```

## Configuration: `.worktree-config.toml`

Place this file in the repository root to control what gets copied/symlinked when creating
worktrees, and what commands run post-creation.

```toml
[copy-patterns]
include = [
    ".env*",
    ".vscode/",
    "*.local.json",
    "config/local/*",
]
exclude = [
    "node_modules/",
    "target/",
    "*.log",
]

[symlink-patterns]
include = [
    ".env",          # Symlinks stay in sync across all worktrees
    "scripts/",
]

[on-create]
commands = [
    "npm install",
    "cp .env.example .env.local",
]
```

**Rules:**
- `copy-patterns`: Files are physically copied into new worktrees. Patterns merge with defaults.
- `symlink-patterns`: Files are symlinked to the origin — edits anywhere affect all worktrees.
  Symlink patterns take precedence over copy patterns.
- `on-create`: Shell commands run in the new worktree directory after creation.
  Commands run via `sh -c`; a failing command warns but doesn't abort.

## Common Workflows

### Starting a new feature

```bash
cd /path/to/my-repo
worktree create my-feature feature/cool-thing --from main
worktree jump my-feature
# ... work ...
worktree back
```

### Switching between active features

```bash
worktree list --current           # See what's available
worktree jump                     # Interactive picker
```

### Cleanup after merging

```bash
worktree remove merged-feature --delete-branch
worktree cleanup                  # Clean up any orphaned refs
```

### Keep secrets in sync across worktrees

Add to `.worktree-config.toml`:
```toml
[symlink-patterns]
include = [".env"]
```
Then new worktrees get a symlink to the origin's `.env`, so changes propagate everywhere.

## Storage Layout

```
~/.worktrees/
└── my-repo/
    ├── .worktree-origins          # Metadata for `back` navigation
    ├── my-feature/                # Worktree directory
    │   ├── .git                   # Linked git dir
    │   └── <project files>
    └── other-feature/
```

## What Agents Should NOT Do

- Do not call `worktree-bin` directly for navigation commands
- Do not manually create directories under `~/.worktrees/` — let the CLI manage storage
- Do not try to infer the worktree path manually; use `worktree jump` to navigate
- Feature names must not contain: `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`
