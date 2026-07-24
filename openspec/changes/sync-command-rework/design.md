## Context

`sync-config` (`src/commands/sync_config.rs`) currently only calls `create::copy_config_files`, never `create::create_symlinks`, so it cannot push out newly-added `[symlink-patterns]` entries. It requires explicit `from`/`to` arguments — there's no way to fan a change out to every worktree of a repo in one call. Separately, `worktree status` (`src/commands/status.rs`) only reports git/storage worktree registration consistency; it has no visibility into config or symlink drift.

The building blocks for a repo-scoped broadcast already exist and are unused for this purpose: `WorktreeStorage::list_repo_worktrees(repo_name)` (repo-scoped) and `WorktreeStorage::get_worktree_origin(repo_name, feature_name)` (per-worktree origin metadata, already written at `create` time). `list_all_worktrees()` also exists but is out of scope — it spans every repo the tool manages and must not be touched by this change.

## Goals / Non-Goals

**Goals:**
- Replace `sync-config` with `sync`, keeping a pairwise escape hatch.
- Make `sync` symlink-aware, reusing `create_symlinks`/`copy_config_files` rather than duplicating logic.
- Default `sync` (no args) to a repo-scoped broadcast: origin → every worktree of *that repo only*.
- Force-relink paths that are drifted in the specific sense of "now matches `[symlink-patterns]` but exists as a plain file/dir" — this is the direct target case (e.g. `.cursor/skills/openspec-*` copied before the pattern existed).
- Add config drift detection to `status`, on by default, covering both `[symlink-patterns]` (missing or not-a-symlink) and `[copy-patterns]` (missing only).

**Non-Goals:**
- Detecting or reconciling content differences in copied files (copies are expected to diverge; that's their purpose).
- Touching symlinks that exist but point somewhere unexpected (not created by this tool, or pointing at a different origin) — that's a conflict to surface, not silently overwrite.
- Any cross-repo behavior — `sync` never operates over `list_all_worktrees()`.
- A confirmation/dry-run prompt before broadcast (decided against; `status` already told you what's drifted, so `sync` acting on that is not a surprise).

## Decisions

**Rename `sync-config` → `sync`, with `from`/`to` becoming optional.**
Clap subcommand `SyncConfig { from: String, to: String }` becomes `Sync { from: Option<String>, to: Option<String> }`. Both present → pairwise mode (today's behavior, now symlink-aware). Both absent → repo-scoped broadcast mode. One-present-one-absent is a usage error. Alternative considered: keep `sync-config` and add a new `sync` broadcast-only command — rejected because it leaves two overlapping commands with the "sync-config isn't symlink-aware" bug still needing an independent fix, and the user explicitly wants one `sync` command.

**Broadcast mode resolves origin from the current working directory, not from stored metadata of a target.**
Repo-scoped broadcast run from anywhere inside repo A (origin or one of its own worktrees, via `GitRepo::open` + repo path resolution already used elsewhere in the codebase) determines `repo_name`, then calls `storage.list_repo_worktrees(&repo_name)` and syncs origin → each entry. Running it from inside a worktree still broadcasts using that worktree's origin repo as source (via `get_worktree_origin`), not the worktree itself as source — origin is always the source of truth for broadcast mode.

**Force-relink is scoped narrowly: only paths matching `[symlink-patterns]` that exist as a non-symlink.**
`create_symlinks`'s existing "skip if exists" guard (`create.rs:157`) must gain a mode where, if the existing target is a plain file/dir (not a symlink), it's removed and replaced with the symlink; if it's already a symlink (to anywhere), it's left untouched — a symlink pointing elsewhere is a signal something else is going on and should be surfaced (via `status`) rather than silently overwritten. This mode is used only by `sync`; `create_symlinks` as called from `create_worktree_internal` keeps today's non-destructive skip behavior, since a fresh worktree shouldn't have anything at those paths yet anyway.

**`status`'s drift-checking reuses `create.rs`'s pattern-matching helpers rather than reimplementing them.**
`find_matching_files`, `should_exclude_file`, and `is_covered_by_symlink_pattern` are currently private to `create.rs`. They become `pub(crate)` (or move to a shared `config`-adjacent module) so `status.rs` can call them read-only, without duplicating glob logic.

**Drift output is per-worktree, appended to the existing "Managed worktrees" section in `status`, not a separate table.**
Keeps the existing report structure (`status.rs:50-67`) as the anchor point; each worktree's drift lines are indented under its existing summary line. Avoids introducing a second worktree enumeration/report format alongside the current one.

## Risks / Trade-offs

- **[Broadcast overwrites files across multiple worktrees in one command]** → Mitigated by narrowing force-relink to only the unambiguous case (matches a symlink pattern, exists as non-symlink); anything ambiguous (wrong-target symlink) is left alone and surfaced via `status` instead.
- **[Renaming the CLI subcommand is a breaking change for anyone scripting `sync-config`]** → Acceptable pre-1.0 (v0.5.1); call it out explicitly as **BREAKING** in the proposal and changelog.
- **[Exposing `create.rs` helpers as `pub(crate)` widens internal coupling between `create` and `status`]** → Contained to the `commands` module; no public API surface change.
- **[Broadcast mode determines "origin" differently depending on where it's invoked from (origin repo vs. one of its worktrees)]** → Both cases resolve to the same origin path via existing `get_worktree_origin`/`get_repo_path` mechanisms already used by `back` and `create`; no new resolution logic needed, just reuse.

## Open Questions

None outstanding. The one open question from initial design — whether `sync`'s pairwise escape hatch (`sync <from> <to>`) should also force-relink — was resolved during implementation: pairwise mode shares the same `sync_one` logic as broadcast mode (`src/commands/sync.rs`), so both force-relink identically rather than pairwise keeping the original non-destructive skip-if-exists behavior.
