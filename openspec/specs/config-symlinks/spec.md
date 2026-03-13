# Feature: Config Symlinks

## Purpose

The `.worktree-config.toml` configuration file supports a `[symlink-patterns]` section that causes matching paths to be symlinked into the worktree rather than copied. This allows shared directories (such as specs or assets) to remain live references to the origin repo rather than static snapshots.

## Requirements

### Requirement: symlink-patterns config section creates symlinks instead of copies
The `.worktree-config.toml` file SHALL support a `[symlink-patterns]` section with an `include` list of glob patterns. Matching paths SHALL be symlinked into the worktree pointing at the corresponding path in the origin repo, rather than copied.

#### Scenario: Symlink created for matching path
- **WHEN** `.worktree-config.toml` includes `[symlink-patterns] include = ["openspec/"]` and a worktree is created
- **THEN** `<worktree>/openspec` is a symlink pointing to `<origin-repo>/openspec/`

#### Scenario: Symlink reflects live changes to origin
- **WHEN** a file inside `<origin-repo>/openspec/` is modified after the worktree is created
- **THEN** the change is immediately visible at `<worktree>/openspec/` via the symlink

#### Scenario: Missing origin path is skipped with warning
- **WHEN** a symlink pattern matches a path that does not exist in the origin repo
- **THEN** the symlink is not created and a warning is printed, but worktree creation continues

### Requirement: symlink-patterns takes precedence over copy-patterns for overlapping paths
If a path matches both `[copy-patterns]` and `[symlink-patterns]`, the path SHALL be symlinked and not copied.

#### Scenario: Overlapping pattern resolved in favour of symlink
- **WHEN** a path matches both `copy-patterns.include` and `symlink-patterns.include`
- **THEN** the path is symlinked to the origin repo and no copy is made

### Requirement: Symlinks point to origin repo path
Symlink targets SHALL be absolute paths derived from the origin repo path stored at worktree creation time (the same path used by `worktree back`).

#### Scenario: Symlink target is absolute path in origin repo
- **WHEN** a worktree is created from origin repo at `/Users/cfreeman/projects/myrepo`
- **THEN** symlinks target paths under `/Users/cfreeman/projects/myrepo/`
