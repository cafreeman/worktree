# Feature: Feature-Named Worktrees

## Purpose

Worktrees are identified by a user-supplied feature name rather than by branch name. This decouples worktree identity from the branch checked out inside it, allowing branches to be switched freely without affecting how the worktree is referenced.

## Requirements

### Requirement: Worktree is identified by feature name
A worktree SHALL be created with a user-supplied feature name as its identity. The feature name SHALL be used as the directory name under `~/.worktrees/<repo>/`. The feature name SHALL NOT contain `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, or `|` characters.

#### Scenario: Create with valid feature name
- **WHEN** user runs `worktree create auth`
- **THEN** a worktree directory is created at `~/.worktrees/<repo>/auth/`

#### Scenario: Create with invalid feature name containing slash
- **WHEN** user runs `worktree create feature/auth`
- **THEN** the command fails with an error explaining that feature names cannot contain `/`

#### Scenario: Create with already-existing feature name
- **WHEN** user runs `worktree create auth` and `~/.worktrees/<repo>/auth/` already exists
- **THEN** the command fails with an error indicating the worktree already exists

### Requirement: Create command accepts feature name and starting branch separately
The `worktree create` command SHALL accept a feature name as its first positional argument. The starting branch SHALL be supplied as an optional second positional argument or via interactive prompt. An optional `--from <ref>` flag SHALL specify the base ref when creating a new branch.

#### Scenario: Non-interactive create with feature name and new branch
- **WHEN** user runs `worktree create auth feature/auth-base`
- **THEN** branch `feature/auth-base` is created and the worktree starts on it

#### Scenario: Non-interactive create with feature name and existing branch
- **WHEN** user runs `worktree create auth feature/auth-base` and branch `feature/auth-base` already exists
- **THEN** the worktree is created checked out to the existing branch (smart mode)

#### Scenario: Non-interactive create with explicit base ref
- **WHEN** user runs `worktree create auth feature/auth-base --from main`
- **THEN** branch `feature/auth-base` is created from `main` and the worktree starts on it

#### Scenario: Interactive create prompts for feature name when omitted
- **WHEN** user runs `worktree create` with no arguments
- **THEN** the tool prompts for feature name, then starting branch, then base ref (if branch is new)

#### Scenario: Interactive create skips base ref prompt for existing branch
- **WHEN** user runs interactive create and enters a branch name that already exists
- **THEN** the base ref prompt is skipped and the worktree is created on the existing branch

### Requirement: Remove command preserves branches by default
The `worktree remove` command SHALL preserve all branches by default when removing a worktree. An explicit `--delete-branch` flag SHALL be required to delete the associated branch.

#### Scenario: Remove without flags preserves branch
- **WHEN** user runs `worktree remove auth`
- **THEN** the worktree directory is removed and the branch is left intact

#### Scenario: Remove with --delete-branch deletes branch
- **WHEN** user runs `worktree remove auth --delete-branch`
- **THEN** the worktree directory is removed and the current HEAD branch of that worktree is deleted

### Requirement: List and jump display current HEAD branch dynamically
The `worktree list` and `worktree jump` commands SHALL display the current checked-out branch for each worktree by reading the HEAD of the worktree directory at runtime. Stored branch metadata SHALL NOT be used for display purposes.

#### Scenario: List shows feature name and current branch
- **WHEN** user runs `worktree list`
- **THEN** each worktree is displayed with its feature name and its current HEAD branch

#### Scenario: Jump interactive picker shows current branch
- **WHEN** user runs `worktree jump` interactively
- **THEN** each option shows the feature name and the current HEAD branch of that worktree

#### Scenario: Jump by feature name resolves correctly after branch switch
- **WHEN** user manually runs `git checkout feature/auth-ui` inside a worktree named `auth`, then runs `worktree jump auth` from another context
- **THEN** the tool navigates to the `auth` worktree regardless of which branch it is currently on

### Requirement: Branch mapping and managed-branch tracking are removed
The tool SHALL NOT maintain `.branch-mapping` or `.managed-branches/` metadata files. These concepts do not apply when worktree identity is decoupled from branch name.

#### Scenario: No branch mapping file created on worktree create
- **WHEN** user creates a new worktree
- **THEN** no `.branch-mapping` file is written to the repo storage directory

#### Scenario: No managed-branch flag created on worktree create
- **WHEN** user creates a new worktree with a new branch
- **THEN** no `.managed-branches/` flag file is written
