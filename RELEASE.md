# Release Process

This document outlines the process for releasing new versions of the worktree CLI.

## Overview

The project uses `cargo-release` for automated version management, git operations, and publishing. Once initiated by the maintainer, the entire release process is automated.

## Prerequisites

1. Ensure you have `cargo-release` installed:

   ```bash
   cargo install cargo-release
   ```

2. Verify you're on the main branch and have a clean working directory:

   ```bash
   git checkout main
   git pull origin main
   git status  # Should show clean working tree
   ```

3. Run all quality checks:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test
   cargo build --release
   ```

## Release Steps

### 1. Update CHANGELOG.md

Before releasing, add your changes to the `[Unreleased]` section in `CHANGELOG.md`:

```markdown
## [Unreleased]

### Added
- New feature description

### Fixed  
- Bug fix description
```

**Note:** cargo-release will automatically:
- Create a new version section with the release date
- Update the version links at the bottom
- Leave a clean `[Unreleased]` section for future changes

### 2. Choose Version Bump

Follow [Semantic Versioning](https://semver.org/):

- **patch** (0.1.0 → 0.1.1): Bug fixes, small improvements
- **minor** (0.1.0 → 0.2.0): New features, backwards compatible
- **major** (0.1.0 → 1.0.0): Breaking changes

### 3. Run cargo-release

Execute the release command:

```bash
# For patch releases (most common)
cargo release patch

# For minor releases
cargo release minor

# For major releases
cargo release major
```

This will automatically:

- Update version in `Cargo.toml`
- **Update CHANGELOG.md** (create new version section, update links)
- Run pre-release hooks (fmt, clippy, test, build)
- Create a release commit
- Create and push a git tag
- **Publish to crates.io**

**Important:** Only run this command when you're ready to publish, as it will immediately push to git and publish to crates.io. Ensure:

- All tests pass
- Documentation is up to date
- You've tested the functionality
- The release has been reviewed

## Safety Features

The release configuration includes several safety measures:

- **Quality Checks**: Automated fmt, clippy, test, and build checks
- **Manual Initiation**: Requires explicit `cargo release` command from maintainer
- **Automatic Changelog**: Updates CHANGELOG.md with new version sections and links
- **Git Integration**: Automatic tagging and pushing
- **Automated Publishing**: Publishes to crates.io as part of release process
- **Verification**: Package contents are verified before operations

## Troubleshooting

### Release Command Fails

If `cargo release` fails:

1. **Uncommitted changes**: Commit or stash your changes
2. **Pre-release hook failures**: Fix the failing checks (fmt, clippy, tests)
3. **Network issues**: Ensure you can push to the remote repository

### Version Already Exists

If the version already exists:

1. Check if you've already run the release command
2. Look at git tags: `git tag -l`
3. If needed, delete the tag: `git tag -d v0.1.1` (then `git push origin --delete v0.1.1`)

### Publishing Issues

If `cargo publish` fails:

1. **Authentication**: Run `cargo login` with your crates.io token
2. **Network**: Check your internet connection
3. **Package size**: Ensure package isn't too large
4. **Dependencies**: Verify all dependencies are available on crates.io

## Rollback Procedure

If you need to rollback a release:

### Before Publishing to crates.io

1. Reset to previous commit:

   ```bash
   git reset --hard HEAD~1
   ```

2. Delete the tag:
   ```bash
   git tag -d v0.1.1
   git push origin --delete v0.1.1
   ```

### After Publishing to crates.io

You cannot unpublish from crates.io, but you can yank a version:

```bash
cargo yank --version 0.1.1
```

Then immediately release a fixed version.

## Automation Notes

The current setup includes automated changelog generation. Future enhancements could include:

- GitHub Actions for automated testing
- Release drafts for review
- Integration with conventional commits
- Enhanced changelog templates

## Checklist

Use this checklist for each release:

- [ ] Add changes to `[Unreleased]` section in CHANGELOG.md
- [ ] Commit changelog changes
- [ ] Run `cargo release <level>` (this will automatically update changelog and publish)
- [ ] Verify git tag was created: `git tag -l`
- [ ] Check the release commit looks correct
- [ ] Verify CHANGELOG.md was updated with new version section
- [ ] Verify on crates.io that new version is published
- [ ] Update any external documentation
