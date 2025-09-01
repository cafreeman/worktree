# Testing Guide

This document covers testing practices, helper utilities, and development workflows for the worktree project.

## Test Helpers

The project uses a dedicated `test-support` crate for integration test utilities:

```rust
use test_support::{CliTestEnvironment, create_worktree_config, assert_config_files_copied};

#[test]
fn test_example() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    // ... test implementation
    Ok(())
}
```

### Available Helpers

- `CliTestEnvironment`: Test environment with git repo and storage setup
- `create_worktree_config()`: Create test configuration files
- `create_sample_config_files()`: Create sample config files for testing
- `assert_config_files_copied()`: Assert config files were copied correctly

### Adding New Helpers

1. Add functions to `tests/test-support/src/` modules
2. Re-export in `tests/test-support/src/lib.rs`
3. Import in test files as needed

## Running Tests

### Standard Test Run

```bash
cargo test --all-features
```

## Test Architecture

### Integration Tests

Integration tests are organized by command functionality:

- `tests/create_tests.rs` - Worktree creation and configuration
- `tests/jump_tests.rs` - Navigation and interactive selection
- `tests/remove_tests.rs` - Worktree removal and cleanup
- `tests/list_tests.rs` - Listing and status commands
- `tests/status_tests.rs` - Status reporting
- `tests/sync_config_tests.rs` - Configuration synchronization
- `tests/workflow_tests.rs` - End-to-end user workflows
- `tests/parallel_safety_tests.rs` - Concurrent test execution safety
- `tests/completion_tests.rs` - Shell completion functionality
- `tests/back_tests.rs` - Back navigation functionality

### Test Support Crate

The `tests/test-support/` crate provides shared utilities:

```
tests/test-support/
├── Cargo.toml          # Dev-only dependencies
├── src/
│   ├── lib.rs          # Re-exports and public API
│   ├── test_env.rs     # CliTestEnvironment and git setup
│   └── patterns.rs     # Config file helpers and assertions
```

### Test Environment

Each test gets a clean environment with:

- Temporary git repository with initial commit
- Isolated storage directory (`~/.worktrees/`)
- Proper git configuration (user.name, user.email)
- Automatic cleanup via `TempDir`

## Writing Tests

### Basic Test Structure

```rust
use test_support::CliTestEnvironment;

#[test]
fn test_feature() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Setup
    env.run_command(&["create", "feature/test"])?
        .assert()
        .success();

    // Assertions
    env.worktree_path("feature/test")
        .assert(predicate::path::is_dir());

    Ok(())
}
```

### Testing with Configuration

```rust
use test_support::{
    CliTestEnvironment,
    create_worktree_config,
    create_sample_config_files,
    assert_config_files_copied
};

#[test]
fn test_with_config() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Create test configuration
    create_worktree_config(
        &env.repo_dir,
        &[".env*", ".vscode/"],
        &["node_modules/", "target/"],
    )?;

    // Create sample files
    create_sample_config_files(&env.repo_dir)?;

    // Create worktree
    env.run_command(&["create", "feature/config-test"])?
        .assert()
        .success();

    // Verify config files were copied
    let worktree_path = env.worktree_path("feature/config-test");
    assert_config_files_copied(&worktree_path)?;

    Ok(())
}
```

### Testing Error Conditions

```rust
#[test]
fn test_error_handling() -> Result<()> {
    let env = CliTestEnvironment::new()?;

    // Pre-create directory to trigger error
    let worktree_path = env.worktree_path("feature/existing");
    worktree_path.create_dir_all()?;

    // Attempt to create worktree - should fail
    env.run_command(&["create", "feature/existing"])?
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    Ok(())
}
```

## Best Practices

### Test Isolation

- Each test gets a fresh `CliTestEnvironment`
- Tests run in parallel safely
- No shared state between tests
- Automatic cleanup via `TempDir`

### Assertions

Use `assert_fs` and `predicates` for declarative assertions:

```rust
// Good: Declarative assertions
worktree_path.assert(predicate::path::is_dir());
config_file.assert(predicate::str::contains("expected content"));

// Avoid: Manual assertions
assert!(worktree_path.path().is_dir());
assert!(config_content.contains("expected content"));
```

### Error Handling

- Use `Result<()>` return type for tests
- Let `?` operator handle errors naturally
- Test both success and failure paths

### Performance

- Tests should complete quickly (< 1 second each)
- Use `CliTestEnvironment` for setup, not manual git commands
- Avoid unnecessary file I/O in test loops

## Debugging Tests

### Verbose Output

```bash
cargo test -- --nocapture
```

### Single Test

```bash
cargo test test_specific_function_name
```

### Test with Debug Info

```bash
RUST_LOG=debug cargo test test_name
```

## Contributing

When adding new tests:

1. Follow existing patterns and naming conventions
2. Use appropriate test helpers from `test-support`
3. Ensure tests are isolated and can run in parallel
4. Add tests for both success and error cases

For more information, see the main [README.md](README.md).
