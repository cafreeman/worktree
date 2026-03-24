## ADDED Requirements

### Requirement: Skill install
The CLI SHALL provide a `worktree skill install` command that writes the bundled `SKILL.md` to `~/.agents/skills/worktree-manager/SKILL.md` and creates a symlink at `~/.claude/skills/worktree-manager` pointing to the installed directory. The skill file SHALL be embedded in the binary at compile time and SHALL NOT be fetched from the network.

#### Scenario: Fresh install succeeds
- **WHEN** the user runs `worktree skill install` and no skill is currently installed
- **THEN** the directory `~/.agents/skills/worktree-manager/` is created, `SKILL.md` is written to it, a symlink is created at `~/.claude/skills/worktree-manager`, and a success message is printed

#### Scenario: Install when already installed
- **WHEN** the user runs `worktree skill install` and the skill is already installed with identical content
- **THEN** the command prints a message indicating the skill is already up to date and exits successfully without modifying any files

#### Scenario: Claude skills directory does not exist
- **WHEN** the user runs `worktree skill install` and `~/.claude/skills/` does not exist
- **THEN** the skill is still written to `~/.agents/skills/worktree-manager/SKILL.md`, a note is printed that the `~/.claude/skills/` symlink was skipped, and the command exits successfully

### Requirement: Skill uninstall
The CLI SHALL provide a `worktree skill uninstall` command that removes `~/.agents/skills/worktree-manager/` and the symlink at `~/.claude/skills/worktree-manager`.

#### Scenario: Uninstall when installed
- **WHEN** the user runs `worktree skill uninstall` and the skill is currently installed
- **THEN** `~/.agents/skills/worktree-manager/` is deleted, the `~/.claude/skills/worktree-manager` symlink is removed, and a success message is printed

#### Scenario: Uninstall when not installed
- **WHEN** the user runs `worktree skill uninstall` and no skill is installed
- **THEN** the command prints a message indicating nothing is installed and exits successfully without error

### Requirement: Skill update
The CLI SHALL provide a `worktree skill update` command that overwrites the installed `SKILL.md` with the version embedded in the current binary.

#### Scenario: Update when skill is outdated
- **WHEN** the user runs `worktree skill update` and the installed skill content differs from the embedded version
- **THEN** `~/.agents/skills/worktree-manager/SKILL.md` is overwritten with the embedded content and a success message is printed

#### Scenario: Update when skill is already current
- **WHEN** the user runs `worktree skill update` and the installed skill content matches the embedded version
- **THEN** the command prints a message indicating the skill is already up to date and exits successfully without modifying any files

#### Scenario: Update when not installed
- **WHEN** the user runs `worktree skill update` and no skill is installed
- **THEN** the command exits with an error message indicating the skill is not installed and instructs the user to run `worktree skill install`

### Requirement: Skill status
The CLI SHALL provide a `worktree skill status` command that reports the current installation state.

#### Scenario: Not installed
- **WHEN** the user runs `worktree skill status` and no skill is installed
- **THEN** the output indicates the skill is not installed and suggests running `worktree skill install`

#### Scenario: Installed and current
- **WHEN** the user runs `worktree skill status` and the installed skill matches the embedded version
- **THEN** the output indicates the skill is installed and up to date

#### Scenario: Installed but outdated
- **WHEN** the user runs `worktree skill status` and the installed skill content differs from the embedded version
- **THEN** the output indicates the skill is installed but an update is available and suggests running `worktree skill update`
