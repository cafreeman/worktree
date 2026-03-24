## Why

Users who install the `worktree` CLI have no way to also install the companion agent skill that teaches their coding agent how to use it correctly. Shipping skill management as a first-class CLI command lets the tool distribute and maintain its own agent integration.

## What Changes

- New `worktree skill` subcommand with three actions:
  - `worktree skill install` — installs the worktree-manager skill into the user's coding agent skill directory
  - `worktree skill uninstall` — removes the installed skill
  - `worktree skill update` — updates the skill to the version bundled with the current binary
- The skill file (`SKILL.md`) is embedded in the binary at compile time using `include_str!`
- Installation writes the skill to `~/.agents/skills/worktree-manager/SKILL.md` and creates a symlink at `~/.claude/skills/worktree-manager` (the Claude Code skill directory)
- The command detects whether the skill is already installed and reports its version/status

## Capabilities

### New Capabilities

- `skill-management`: CLI subcommand for installing, uninstalling, and updating the bundled worktree-manager agent skill into the user's coding agent environment

### Modified Capabilities

(none)

## Impact

- **`src/main.rs`**: New `skill` subcommand added to the clap CLI
- **`src/commands/`**: New `skill.rs` command module with install/uninstall/update actions
- **`src/lib.rs`**: Expose new skill command module
- **Embedded asset**: `assets/skill/SKILL.md` added to the repo and embedded via `include_str!` at compile time
- **Shell integration**: No changes — `worktree skill` is a regular command, not a navigation command, so no shell wrapper changes needed
- **No new dependencies**: Uses only `std::fs` for file operations and existing `dirs`-equivalent path resolution
