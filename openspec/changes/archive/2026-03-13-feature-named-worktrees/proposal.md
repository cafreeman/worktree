## Why

Worktrees are currently identified by branch name, coupling their lifecycle to a single branch. In practice, feature work spans multiple branches (stacked PRs), so a worktree needs to outlive any individual branch — the current design can't accommodate this without delete-and-recreate churn.

## What Changes

- **BREAKING**: `worktree create` takes a feature name as identity instead of a branch name; starting branch is a separate prompt/flag
- **BREAKING**: `worktree remove` preserves branches by default (no longer deletes them)
- New `[symlink_patterns]` section in `.worktree-config.toml` — creates symlinks to origin repo path instead of copying files
- New `[on_create]` section in `.worktree-config.toml` — shell commands run after worktree creation; bails on first failure with warning
- Branch mapping and managed-branch tracking removed from storage layer (no longer needed)
- `worktree list` and `worktree jump` display current HEAD branch dynamically rather than stored branch metadata

## Capabilities

### New Capabilities
- `feature-named-worktrees`: Worktree identity based on feature name, decoupled from branch; create/remove/jump/list behavior updated accordingly
- `config-symlinks`: Symlink support in `.worktree-config.toml` for long-lived shared files that must stay in sync across worktrees
- `post-create-hooks`: Configurable shell commands that run automatically after a worktree is created

### Modified Capabilities

*(none — no existing specs)*

## Impact

- `src/commands/create.rs` — new argument shape, interactive flow changes
- `src/commands/remove.rs` — default branch deletion behavior inverted
- `src/commands/list.rs`, `src/commands/jump.rs` — read HEAD dynamically
- `src/storage/mod.rs` — branch mapping and managed-branch tracking removed
- `src/config/` — new `symlink_patterns` and `on_create` config sections
- `.worktree-config.toml` format extended (backwards-compatible additions, except `copy_patterns` behavior unchanged)
- Shell completions may need updating for new `create` argument shape
