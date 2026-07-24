## Why

`sync-config` never learned to symlink: it only copies files (`sync_config.rs` calls `copy_config_files` but never `create_symlinks`), so adding a new `[symlink-patterns]` entry to `.worktree-config.toml` after worktrees already exist has no way to reach those worktrees. Worse, it's pairwise and manual (`sync-config <from> <to>`), so fanning a change out to every worktree of a repo means invoking it once per worktree by hand. There's also no way to see which worktrees are missing config/symlinks in the first place — you only find out by noticing a file isn't there.

## What Changes

- **BREAKING**: Rename `sync-config` to `sync`. The old two-argument form becomes an explicit escape hatch (`worktree sync <from> <to>`) rather than the primary interface.
- `sync` run with no arguments from inside a repo (origin or a worktree) resolves that repo's origin and broadcasts config to **every worktree of that repo** (via `WorktreeStorage::list_repo_worktrees`, scoped by `repo_name` — never the global `list_all_worktrees`).
- `sync` becomes symlink-aware: it now calls `create_symlinks` in addition to `copy_config_files` for each target worktree.
- `sync` force-relinks: if a path newly matches `[symlink-patterns]` but already exists in a worktree as a plain file/dir (e.g. from an earlier copy), `sync` replaces it with a symlink instead of silently skipping it. This only applies to paths `status` would already flag as drifted (see below) — not to arbitrary existing symlinks pointing somewhere unexpected, which still need investigation, not silent overwrite.
- `worktree status` gains config drift detection, on by default, checked against every managed worktree of the current repo:
  - `[symlink-patterns]`: flags a match that's missing, or present but not currently a symlink to the origin path.
  - `[copy-patterns]`: flags a match that's missing entirely (does not flag content differences — copies are expected to diverge).
  - Drifted entries print a hint to run `worktree sync`.

## Capabilities

### New Capabilities
- `config-sync`: The `sync` command — repo-scoped broadcast of copy- and symlink-patterns from a repo's origin to all of its worktrees, symlink-aware and force-relinking on detected drift, plus the pairwise `sync <from> <to>` escape hatch. Replaces the `sync-config` command entirely.
- `worktree-status`: Config drift detection in `worktree status` — reports, per managed worktree, which symlink-patterns and copy-patterns are missing or out of sync with origin, with a hint to run `sync`.

### Modified Capabilities
(none — `config-symlinks` covers creation-time symlink behavior only and is unaffected; `sync-config` was never captured as a spec)

## Impact

- `src/commands/sync_config.rs` → replaced by `src/commands/sync.rs` (or renamed in place); `main.rs` CLI subcommand renamed from `SyncConfig` to `Sync`, with `from`/`to` becoming optional positional args instead of required.
- `src/commands/create.rs`: `create_symlinks` and `copy_config_files` are reused as-is by the new `sync` command; `create_symlinks`'s "skip if exists" behavior needs a variant/parameter for force-relink mode so worktree creation itself is unaffected.
- `src/commands/status.rs`: gains config drift checks, reusing pattern-matching helpers currently private to `create.rs` (may need to be exposed via `pub(crate)` or moved to a shared location).
- `src/storage/mod.rs`: no changes needed — `list_repo_worktrees` and `get_worktree_origin` already provide what's needed.
- Docs referencing `sync-config`: `README.md`, `CLAUDE.md`, `TESTING.md`, `assets/skill/SKILL.md`, `src/commands/init.rs` (shell completions/integration text).
- Tests: `tests/sync_config_tests.rs` needs rework/rename; `tests/workflow_tests.rs` and `tests/config_tests.rs` reference the old command name.
