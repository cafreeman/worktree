## Context

The `worktree` CLI currently has no mechanism to distribute its companion agent skill. Users must manually copy or install the skill separately. The skill file needs to ship alongside the binary and be installable with a single command. The codebase follows a consistent pattern: one module per command in `src/commands/`, registered in `main.rs` via a `Commands` enum variant.

## Goals / Non-Goals

**Goals:**
- Add a `worktree skill <action>` subcommand with `install`, `uninstall`, and `update` actions
- Embed the skill file in the binary at compile time so it ships with every release
- Install to the standard agent skill locations (`~/.agents/skills/worktree-manager/` + `~/.claude/skills/worktree-manager` symlink)
- Detect and report the current installation status (not installed / installed / update available)

**Non-Goals:**
- Fetching the skill from the network — the embedded version is the source of truth
- Supporting agent platforms other than Claude Code (`~/.claude/`) in this iteration
- Versioning or upgrading across binary releases (update = overwrite with embedded version)

## Decisions

### 1. Embed with `include_str!` at compile time

The skill file lives at `assets/skill/SKILL.md` in the repo and is embedded via `include_str!()` in `src/commands/skill.rs`. This means:
- No runtime file lookups or network requests
- The installed version always matches the binary version
- `update` simply re-writes the file from the embedded bytes

**Alternative considered:** Ship the skill as a separate file alongside the binary (e.g., in a `share/` directory). Rejected because it complicates installation (users would need to install a directory, not just a binary) and breaks when the binary is copied alone.

### 2. `worktree skill` as a subcommand with nested actions

```
worktree skill install
worktree skill uninstall
worktree skill update
worktree skill status   (shows whether installed, and if update is available)
```

This mirrors the pattern of tools like `rustup component` and `cargo install`. The nested action approach is cleaner than flat commands (`worktree install-skill`) and is more extensible.

**Alternative considered:** Flat flags on a single `worktree skill` command (e.g., `--install`, `--uninstall`). Rejected — harder to add future actions and less idiomatic.

### 3. Installation target: `~/.agents/skills/` + `~/.claude/skills/` symlink

Two writes on install:
1. Write `SKILL.md` to `~/.agents/skills/worktree-manager/SKILL.md` (the canonical location)
2. Create a symlink `~/.claude/skills/worktree-manager → ../../.agents/skills/worktree-manager`

This matches the layout already established by the user's existing skills. The `~/.claude/` symlink is what Claude Code reads. On uninstall, both are removed.

**Alternative considered:** Write directly to `~/.claude/skills/`. Rejected because `~/.agents/skills/` is the user-owned canonical store; `~/.claude/` is the agent-facing view.

### 4. "Update available" detection via content hash

On `worktree skill status`, compare a SHA-256 of the installed `SKILL.md` against the embedded content's hash. If they differ, report "update available". This is cheap (one file read) and doesn't require versioning metadata.

### 5. No shell wrapper changes needed

`worktree skill install` is a regular command — it doesn't need to change the shell's working directory. The shell wrapper intercepts only `jump`, `switch`, `back`, and `create`. No changes to `init.rs` or generated shell functions.

## Risks / Trade-offs

- **Symlink to `~/.claude/` may not exist on all setups** → Mitigation: if `~/.claude/` doesn't exist, skip symlink creation and print a note. The skill still installs to `~/.agents/skills/` and is usable if the user manually links it later.
- **Embedded skill becomes stale between releases** → Accepted trade-off. Users run `worktree skill update` after upgrading the binary.
- **`~/.agents/skills/` is not a universal standard** → The skill creator convention we've established. Document clearly what is installed and where so users can adapt if needed.

## Migration Plan

No migration required — this is purely additive. Existing installs are unaffected.
