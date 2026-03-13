## 1. Storage Layer Cleanup

- [x] 1.1 Remove `store_branch_mapping` and `remove_branch_mapping` methods from `WorktreeStorage`
- [x] 1.2 Remove `get_original_branch_name` method from `WorktreeStorage`
- [x] 1.3 Remove `mark_branch_managed`, `unmark_branch_managed`, and `is_branch_managed` methods from `WorktreeStorage`
- [x] 1.4 Remove `get_managed_branch_flag_path` helper from `WorktreeStorage`
- [x] 1.5 Remove `.branch-mapping` and `.managed-branches/` file I/O from storage module
- [x] 1.6 Update `get_worktree_path` to accept a feature name directly (no sanitization needed — validate at input time instead)
- [x] 1.7 Add `validate_feature_name` function to storage (rejects `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`)

## 2. Config: symlink-patterns and on-create

- [x] 2.1 Add `SymlinkPatterns` struct to `src/config/mod.rs` with `include: Option<Vec<String>>`
- [x] 2.2 Add `OnCreate` struct to `src/config/mod.rs` with `commands: Option<Vec<String>>`
- [x] 2.3 Add `symlink_patterns: SymlinkPatterns` and `on_create: OnCreate` fields to `WorktreeConfig`
- [x] 2.4 Update `WorktreeConfig::default()` to include empty defaults for new sections
- [x] 2.5 Ensure TOML deserialization handles missing `[symlink-patterns]` and `[on-create]` sections gracefully

## 3. Create Command: feature name + symlinks + hooks

- [x] 3.1 Update `main.rs` `Create` subcommand: rename `branch` arg to `feature_name`, add `branch` as second optional positional arg
- [x] 3.2 Remove `--new-branch` and `--existing-branch` flags from `Create` subcommand (smart mode only)
- [x] 3.3 Update `create_worktree_internal` signature to accept `feature_name: &str` and `branch: Option<&str>` separately
- [x] 3.4 Update worktree path derivation to use `feature_name` instead of `branch` name
- [x] 3.5 Update interactive create workflow: prompt for feature name first, then branch, then base ref (only if branch is new)
- [x] 3.6 Add feature name validation in both interactive and non-interactive paths
- [x] 3.7 Remove `store_branch_mapping` and `mark_branch_managed` calls from `create_worktree_internal`
- [x] 3.8 Implement `create_symlinks` function: iterate `symlink_patterns.include`, resolve origin path, call `std::os::unix::fs::symlink`
- [x] 3.9 Add symlink precedence logic: skip `copy_config_files` for paths that match a symlink pattern
- [x] 3.10 Implement `run_on_create_hooks` function: iterate `on_create.commands`, spawn each via `std::process::Command` in worktree dir, stream output, stop on non-zero exit with warning
- [x] 3.11 Call `create_symlinks` and `run_on_create_hooks` in `create_worktree_internal` after config file copying

## 4. Remove Command: flip branch deletion default

- [x] 4.1 Rename `--preserve-branch` flag to `--delete-branch` in `main.rs` `Remove` subcommand (semantics inverted)
- [x] 4.2 Update `remove_worktree` and `remove_worktree_with_provider` signatures: replace `preserve_branch: bool` with `delete_branch: bool`
- [x] 4.3 Invert the branch deletion condition in `remove_worktree_with_provider` (delete only when `delete_branch` is true)
- [x] 4.4 Remove `unmark_branch_managed` and `remove_branch_mapping` calls from remove logic
- [x] 4.5 Simplify `resolve_target` in `remove.rs`: match by feature name (directory name) directly, no branch mapping lookup

## 5. List and Jump: dynamic HEAD reading

- [x] 5.1 Add `read_worktree_head_branch(path: &Path) -> Option<String>` helper (using `git2::Repository::open(path)?.head()?.shorthand()`)
- [x] 5.2 Update `list_current_repo_worktrees` to call `read_worktree_head_branch` instead of `get_original_branch_name`
- [x] 5.3 Update `list_all_worktrees` similarly
- [x] 5.4 Update display format to `<feature-name>  (<current-branch>)  <path>`
- [x] 5.5 Update `get_available_worktrees` in `jump.rs` to use `read_worktree_head_branch` for display label
- [x] 5.6 Update `find_worktree_by_name` in `jump.rs` to match against feature name (directory name), not branch name
- [x] 5.7 Update completions output in `jump.rs` to emit feature names instead of branch names

## 6. Remove Completion in jump.rs and remove.rs

- [x] 6.1 Update `list_worktree_completions` in `remove.rs` to emit feature names
- [x] 6.2 Update interactive selection display in `select_worktree_for_removal` to show `<feature-name> (<current-branch>)`

## 7. Tests

- [x] 7.1 Update existing `create` tests: replace branch-name args with feature-name + branch pairs
- [x] 7.2 Update existing `remove` tests: replace `preserve_branch: bool` with `delete_branch: bool` and invert test assertions
- [x] 7.3 Update existing `list`/`jump` tests: mock `read_worktree_head_branch` or use real git repos in temp dirs
- [x] 7.4 Add tests for `validate_feature_name` (valid names, names with slashes, names with special chars)
- [x] 7.5 Add tests for `create_symlinks` (symlink created, missing origin path skipped, symlink precedence over copy)
- [x] 7.6 Add tests for `run_on_create_hooks` (commands run in order, stops on failure, worktree intact after failure)
- [x] 7.7 Remove tests that exercise branch mapping or managed-branch tracking

## 8. Documentation and Changelog

- [x] 8.1 Update `--help` text for `create` subcommand to reflect new argument shape
- [x] 8.2 Update `--help` text for `remove` subcommand to reflect `--delete-branch` and new default
- [x] 8.3 Add `CHANGELOG.md` entry documenting both breaking changes and new config sections
- [x] 8.4 Update `CLAUDE.md` or README if it describes the create/remove UX
