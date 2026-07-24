## ADDED Requirements

### Requirement: status reports config drift by default
`worktree status` SHALL, by default (no flag required), check every managed worktree of the current repo against `.worktree-config.toml`'s `[symlink-patterns]` and `[copy-patterns]` and report any drift found.

#### Scenario: Drift check runs without any extra flag
- **WHEN** `worktree status` is run in a repo with managed worktrees
- **THEN** config drift is checked and reported for each managed worktree without requiring any additional command-line flag

### Requirement: status flags missing or non-symlinked symlink-pattern matches
For each `[symlink-patterns]` match in the origin repo, `status` SHALL report, per managed worktree, whether the corresponding path is missing, present but not a symlink, or correctly symlinked.

#### Scenario: Missing symlink is flagged
- **WHEN** a `[symlink-patterns]` match exists in the origin repo but no corresponding path exists in a managed worktree
- **THEN** `status` reports that path as missing for that worktree

#### Scenario: Plain copy where a symlink is expected is flagged
- **WHEN** a `[symlink-patterns]` match exists in the origin repo, and the corresponding path in a managed worktree exists but is not a symlink
- **THEN** `status` reports that path as present-as-copy for that worktree

#### Scenario: Correctly symlinked path is not flagged as drift
- **WHEN** a `[symlink-patterns]` match is correctly symlinked from a managed worktree to the origin repo
- **THEN** `status` does not report that path as drifted

### Requirement: status flags missing copy-pattern matches only
For each `[copy-patterns]` match in the origin repo, `status` SHALL report, per managed worktree, whether the corresponding path is missing. `status` SHALL NOT compare file contents between origin and worktree copies.

#### Scenario: Missing copy is flagged
- **WHEN** a `[copy-patterns]` match exists in the origin repo but no corresponding path exists in a managed worktree
- **THEN** `status` reports that path as missing for that worktree

#### Scenario: Differing content in an existing copy is not flagged
- **WHEN** a `[copy-patterns]` match exists in both the origin repo and a managed worktree, with different file contents
- **THEN** `status` does not report that path as drifted

### Requirement: drifted status output hints at the fix
When `status` reports any drifted config for a worktree, it SHALL include a hint to run `worktree sync` to resolve it.

#### Scenario: Hint shown when drift is present
- **WHEN** `status` reports at least one drifted path for any managed worktree
- **THEN** the output includes a suggestion to run `worktree sync`

#### Scenario: No hint shown when nothing is drifted
- **WHEN** `status` finds no drifted config for any managed worktree
- **THEN** no `sync` hint is printed
