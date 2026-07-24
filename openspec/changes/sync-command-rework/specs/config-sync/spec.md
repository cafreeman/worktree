## ADDED Requirements

### Requirement: `sync` command replaces `sync-config`
The CLI SHALL expose a `sync` subcommand that replaces the removed `sync-config` subcommand. `sync` accepts two optional positional arguments, `from` and `to`.

#### Scenario: sync-config no longer exists
- **WHEN** the CLI is invoked with `sync-config`
- **THEN** it is rejected as an unrecognized subcommand

#### Scenario: sync with both arguments runs pairwise mode
- **WHEN** `worktree sync <from> <to>` is invoked with both arguments present
- **THEN** config is synced from the resolved `from` worktree/path to the resolved `to` worktree/path, as `sync-config` did previously

#### Scenario: sync with no arguments runs broadcast mode
- **WHEN** `worktree sync` is invoked with neither argument present
- **THEN** broadcast mode runs (see broadcast requirement below)

#### Scenario: sync with exactly one argument is a usage error
- **WHEN** `worktree sync <from>` is invoked with only one of `from`/`to` present
- **THEN** the command exits with an error explaining that both or neither must be given

### Requirement: Broadcast mode syncs origin to all worktrees of the current repo only
Running `sync` with no arguments SHALL resolve the current repo (from the working directory, whether that's the origin repo or one of its worktrees), determine that repo's origin path, and sync config from the origin to every worktree returned by the repo-scoped worktree listing. It SHALL NOT operate across other repos managed by the tool.

#### Scenario: Broadcast reaches every worktree of the repo
- **WHEN** repo A has worktrees B and C, and `worktree sync` is run from inside A
- **THEN** config is synced from A to both B and C

#### Scenario: Broadcast run from inside a worktree still uses repo origin as source
- **WHEN** `worktree sync` is run from inside worktree B (whose origin is repo A, which also has worktree C)
- **THEN** config is synced from A (not from B) to both B and C

#### Scenario: Broadcast does not touch other repos
- **WHEN** the tool manages worktrees for both repo A and unrelated repo X
- **THEN** running `worktree sync` from inside repo A does not read or write any files under repo X's worktrees

### Requirement: sync is symlink-aware
`sync`, in both pairwise and broadcast modes, SHALL create symlinks for `[symlink-patterns]` matches in addition to copying `[copy-patterns]` matches, mirroring the behavior `create` already applies when a worktree is first created.

#### Scenario: New symlink pattern is pushed to an existing worktree
- **WHEN** `.worktree-config.toml` is updated to add a new `[symlink-patterns]` entry after worktree B already exists, and `worktree sync` is run
- **THEN** the matching path is created in worktree B as a symlink pointing at the corresponding path in the origin repo

### Requirement: sync force-relinks paths that newly match a symlink pattern
If a path matches `[symlink-patterns]` and already exists in the target worktree as a plain file or directory (not a symlink), `sync` SHALL remove it and replace it with a symlink to the origin path. If the existing path is already a symlink (regardless of its target), `sync` SHALL leave it untouched.

#### Scenario: Existing plain copy is replaced with a symlink
- **WHEN** `.cursor/skills/openspec-init.md` already exists as a plain file in worktree B (from an earlier copy) and is added to `[symlink-patterns]`, then `worktree sync` is run
- **THEN** `.cursor/skills/openspec-init.md` in worktree B becomes a symlink to the origin repo's copy

#### Scenario: Existing symlink to an unexpected target is left alone
- **WHEN** a path matching `[symlink-patterns]` already exists in the target worktree as a symlink pointing somewhere other than the expected origin path
- **THEN** `sync` does not modify or replace it
