## 1. Add Skill Asset

- [x] 1.1 Create `assets/skill/` directory and copy `~/.agents/skills/worktree-manager/SKILL.md` into it as the canonical embedded asset
- [x] 1.2 Verify `assets/skill/SKILL.md` contains the current skill content and is tracked by git

## 2. Implement `src/commands/skill.rs`

- [x] 2.1 Create `src/commands/skill.rs` with `include_str!("../../assets/skill/SKILL.md")` to embed the skill at compile time
- [x] 2.2 Implement `install_skill()` — creates `~/.agents/skills/worktree-manager/`, writes `SKILL.md`, creates symlink at `~/.claude/skills/worktree-manager`; handles already-installed (idempotent) and missing `~/.claude/skills/` cases
- [x] 2.3 Implement `uninstall_skill()` — removes `~/.agents/skills/worktree-manager/` directory and `~/.claude/skills/worktree-manager` symlink; handles not-installed case gracefully
- [x] 2.4 Implement `update_skill()` — compares installed content against embedded via SHA-256 hash; overwrites if different, reports up-to-date if same, errors if not installed
- [x] 2.5 Implement `skill_status()` — reports one of: not installed, installed+current, installed+outdated (with update suggestion)
- [x] 2.6 Add a `SkillAction` enum (`Install`, `Uninstall`, `Update`, `Status`) and a `run_skill_command(action: SkillAction)` dispatch function

## 3. Wire Up CLI

- [x] 3.1 Add `skill` to `src/commands/mod.rs` exports
- [x] 3.2 Add `Skill` variant to the `Commands` enum in `main.rs` with a nested `#[command(subcommand)] action: SkillAction` argument
- [x] 3.3 Add `SkillAction` clap subcommand enum in `main.rs` (or re-export from `skill.rs`) with `Install`, `Uninstall`, `Update`, `Status` variants and help text
- [x] 3.4 Add match arm for `Commands::Skill { action }` in `main()` dispatching to `skill::run_skill_command(action)`

## 4. Tests

- [x] 4.1 Write unit test for `install_skill()` in a temp directory — verify `SKILL.md` written and symlink created
- [x] 4.2 Write unit test for idempotent install — running install twice should not fail
- [x] 4.3 Write unit test for `uninstall_skill()` — verify files removed; verify no-op when not installed
- [x] 4.4 Write unit test for `update_skill()` — verify overwrites when content differs, no-op when same, errors when not installed
- [x] 4.5 Write unit test for `skill_status()` — verify correct output for each of the three states

## 5. Docs & Release Prep

- [x] 5.1 Update `README.md` to document `worktree skill install` with a one-liner install instruction for users
- [x] 5.2 Add changelog entry under `[Unreleased]` for the new `skill` subcommand
- [x] 5.3 Run `cargo clippy && cargo fmt --check && cargo test` and fix any issues
