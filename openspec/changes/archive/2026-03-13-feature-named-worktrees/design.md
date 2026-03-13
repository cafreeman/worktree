## Context

Worktrees are currently stored at `~/.worktrees/<repo>/<sanitized-branch-name>/` and all metadata (branch mappings, managed-branch flags, origin tracking) is keyed by branch name. This couples worktree identity and lifecycle to a single branch, which breaks down for stacked PR workflows where a single feature spans multiple branches.

The storage layer (`WorktreeStorage`) carries several mechanisms that exist solely to maintain the branch-name-as-identity invariant: `.branch-mapping` files that reverse-map sanitized names back to originals, and `.managed-branches/` flag files that track which branches were created by the tool (to know whether to delete them on remove). Both can be removed entirely under the new model.

## Goals / Non-Goals

**Goals:**
- Worktrees are identified by a user-supplied feature name, not a branch name
- `create <feature-name>` provisions the worktree directory; starting branch is a separate concern
- `remove` preserves branches by default
- `list` and `jump` display/match by feature name, with current HEAD branch shown alongside
- Symlinks to origin repo supported in `.worktree-config.toml`
- Post-create shell commands supported in `.worktree-config.toml`
- Branch mapping and managed-branch tracking machinery removed

**Non-Goals:**
- Tooling to manage branch switching inside a worktree (that's `git`)
- Tracking which branches have been used in a worktree over time
- Migrating existing branch-named worktrees automatically

## Decisions

### Storage path uses feature name directly

`~/.worktrees/<repo>/<feature-name>/` — the directory name is now the feature name. No sanitization mapping needed because feature names are user-supplied and can be validated at input time (no slashes, no special characters). This eliminates `.branch-mapping` entirely.

*Alternative considered*: Keep branch-named dirs, add a metadata file mapping feature name → dir. Rejected — adds indirection for no benefit; the path itself should be the identity.

### Branch mapping and managed-branch tracking removed

These exist only to support the old model. With feature names as identity and `remove` defaulting to preserve-branch, there is nothing to map back and no tool-created branches to track.

*Risk*: Existing worktrees created under the old model will become unmanaged (their dirs exist but no metadata). Mitigation: document this as a breaking change; users should remove old worktrees before upgrading or manage them manually.

### `create` interactive flow: feature name → branch name → base ref

Three-step interactive flow:
1. "Feature name:" — text input, validated (no `/`, no special chars)
2. "Starting branch:" — text input
3. "Base branch:" — git ref selector (shown only if starting branch doesn't exist yet)

Non-interactive: `worktree create <feature-name> <branch> [--from <ref>]`

Smart mode is preserved: if the branch already exists, use it; if not, create it from `--from` (or HEAD if omitted).

### `remove` default flips to preserve-branch

The `--preserve-branch` flag exists today but is non-default. Under the new model, branch deletion becomes opt-in via `--delete-branch`. The feature lifetime outlasts any individual branch; deleting branches on worktree removal would be destructive by default.

### `list` and `jump` read HEAD branch dynamically

Instead of looking up stored branch metadata, read the HEAD of each worktree directory using `git2::Repository::open(path)?.head()?.shorthand()`. This always reflects the actual current state, even after manual `git checkout` inside the worktree.

Display format: `<feature-name>  (<current-branch>)  <path>`

`jump --target` matches against feature name (exact, then partial). The current branch is not a match target for non-interactive jump — users identify worktrees by feature name.

### Config: symlink_patterns section

New section in `.worktree-config.toml`:

```toml
[symlink-patterns]
include = ["openspec/", ".mise.toml"]
```

At create time, matching paths are symlinked to the origin repo path (already tracked in storage for `back` navigation) rather than copied. Symlink creation uses `std::os::unix::fs::symlink`. No Windows support planned.

If a path matches both `copy-patterns` and `symlink-patterns`, symlink takes precedence.

### Config: on_create section

New section in `.worktree-config.toml`:

```toml
[on-create]
commands = [
  "mise trust",
  "mise install",
  "pnpm install",
]
```

Commands are run sequentially in the worktree directory after all files are copied/symlinked. On first failure, remaining commands are skipped and a warning is printed. The worktree is still created (failure is non-fatal to the create operation itself, but the user is warned clearly).

*Alternative considered*: Abort worktree creation on hook failure. Rejected — a failed `mise install` due to network issues shouldn't leave you with a half-created worktree to clean up.

## Risks / Trade-offs

**Breaking change to `create` CLI** → Existing scripts or muscle memory using `worktree create <branch>` will break. Mitigation: clear changelog entry, consider a deprecation warning if the arg looks like a branch name (contains `/`).

**Branch deletion default flip** → Users accustomed to `remove` cleaning up branches will now need `--delete-branch`. Mitigation: clear changelog, prominent note in `--help`.

**Symlinks break if origin repo moves** → All symlinks point to the origin path stored at create time. If the main repo is moved, symlinks go stale. Mitigation: document this; `sync-config` command could be extended to re-create symlinks from current origin.

**on_create commands are arbitrary shell** → No sandboxing. Only run commands from `.worktree-config.toml` in the repo being worked on — same trust level as running `make` or `npm install`. Acceptable for this tool's use case.

**Dynamic HEAD read on list/jump** → Slightly more I/O than reading stored metadata. In practice, worktree counts are small (2-10) and `git2::Repository::open` is fast. Not a concern.

## Open Questions

- Should `worktree create` support `--no-branch` (detached HEAD start)? Not needed for the stated use case but potentially useful.
- Should `sync-config` be extended to re-run symlink creation (useful if symlink targets change)?
