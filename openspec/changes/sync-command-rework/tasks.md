## 1. create.rs: force-relink support

- [x] 1.1 Add a force-relink mode/parameter to `create_symlinks` (or a new variant) that, when a matched path already exists as a plain file/dir (not a symlink), removes it and creates the symlink; leaves existing symlinks untouched regardless of target
- [x] 1.2 Keep `create_worktree_internal`'s call to `create_symlinks` on the existing non-destructive skip-if-exists behavior (force-relink is opt-in, used only by `sync`)
- [x] 1.3 Expose `find_matching_files`, `should_exclude_file`, and `is_covered_by_symlink_pattern` as `pub(crate)` so `status.rs` can reuse them read-only
- [x] 1.4 Unit tests: force-relink replaces a plain file with a symlink; force-relink leaves an existing symlink (even to an unexpected target) untouched

## 2. sync command (replaces sync-config)

- [x] 2.1 Rename `src/commands/sync_config.rs` to `src/commands/sync.rs` (update `mod.rs` export)
- [x] 2.2 Change `main.rs`'s `Commands::SyncConfig { from, to }` to `Commands::Sync { from: Option<String>, to: Option<String> }`; reject the case where exactly one of `from`/`to` is given with a clear usage error
- [x] 2.3 Implement pairwise mode: same resolution as today's `sync_config`, but now also calling `create_symlinks` in force-relink mode (in addition to `copy_config_files`)
- [x] 2.4 Implement broadcast mode: resolve current repo's origin (reuse existing origin-resolution used by `back`/`create`), call `storage.list_repo_worktrees(&repo_name)`, and run the pairwise sync logic from origin to each returned worktree
- [x] 2.5 Verify broadcast mode never calls `list_all_worktrees()` and only touches worktrees under the resolved `repo_name`
- [x] 2.6 Update CLI help text / shell completions in `init.rs` referencing `sync-config`
- [x] 2.7 Integration tests: broadcast from origin reaches all of that repo's worktrees and no others; broadcast run from inside a worktree still sources from that worktree's origin; symlink pattern added after worktree creation is pushed out by `sync`; existing plain copy is relinked by `sync`

## 3. status: config drift detection

- [x] 3.1 In `status.rs`, load `.worktree-config.toml` via `WorktreeConfig::load_from_repo` for the current repo
- [x] 3.2 For each managed worktree, evaluate `[symlink-patterns]` matches: missing / present-but-not-symlink / correctly-symlinked
- [x] 3.3 For each managed worktree, evaluate `[copy-patterns]` matches: missing only (no content comparison)
- [x] 3.4 Render drift lines under each worktree's existing summary line in the "Managed worktrees" section
- [x] 3.5 Print a `Run \`worktree sync\`` hint when any drift is found; omit it when nothing is drifted
- [x] 3.6 Unit/integration tests covering: missing symlink flagged, plain-copy-where-symlink-expected flagged, correctly-symlinked path not flagged, missing copy flagged, differing-content copy not flagged, hint shown/omitted appropriately (unit tests in status.rs; integration coverage added in section 4)

## 4. Cleanup and docs

- [x] 4.1 Rename/rework `tests/sync_config_tests.rs` to cover the new `sync` command (pairwise + broadcast + symlink-aware + force-relink)
- [x] 4.2 Update references to `sync-config` in `tests/workflow_tests.rs` and `tests/config_tests.rs`
- [x] 4.3 Update `README.md`, `CLAUDE.md`, `TESTING.md`, and `assets/skill/SKILL.md` to describe `sync` instead of `sync-config`
- [x] 4.4 Add a `CHANGELOG.md` entry marking the `sync-config` → `sync` rename as **BREAKING**
- [x] 4.5 Run `cargo clippy && cargo fmt --check && cargo test` and fix any fallout
