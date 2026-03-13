## ADDED Requirements

### Requirement: on_create config section defines post-create commands
The `.worktree-config.toml` file SHALL support an `[on-create]` section with a `commands` list of shell command strings. These commands SHALL be executed sequentially in the worktree directory after all files have been copied and symlinked.

#### Scenario: Commands run in worktree directory after create
- **WHEN** `.worktree-config.toml` includes `[on-create] commands = ["mise trust", "mise install"]` and a worktree is created
- **THEN** `mise trust` is run first, then `mise install`, both with the worktree directory as the working directory

#### Scenario: No on_create section results in no commands run
- **WHEN** `.worktree-config.toml` has no `[on-create]` section
- **THEN** no post-create commands are run and worktree creation completes normally

### Requirement: First failing command halts remaining commands with a warning
If a post-create command exits with a non-zero status, the remaining commands SHALL be skipped. A warning SHALL be printed identifying which command failed and its exit code. The worktree itself SHALL remain created.

#### Scenario: Failing command stops subsequent commands
- **WHEN** the second of three `on-create` commands exits with a non-zero status
- **THEN** the third command is not run, a warning is printed, and the worktree directory exists

#### Scenario: Worktree is usable after hook failure
- **WHEN** an `on-create` command fails
- **THEN** the worktree directory, branch checkout, copied files, and symlinks are all intact

### Requirement: Command output is streamed to the user
Post-create commands SHALL have their stdout and stderr streamed to the terminal so the user can observe progress.

#### Scenario: Command output visible during execution
- **WHEN** an `on-create` command produces output (e.g., `pnpm install` progress)
- **THEN** that output is visible to the user in real time during worktree creation
