# Release Process

This document describes the maintainer-run release process for the `worktree`
CLI. Releases use `cargo-release` for versioning, changelog replacement, the
release commit, tagging, and pushing. Publishing to crates.io is a separate,
explicit step.

## Current Release Policy

The behavior in `release.toml` is authoritative:

- `publish = false`: `cargo release` never publishes to crates.io.
- `push = true`: an executed release pushes the release commit and tag.
- `tag = true`: an executed release creates a version tag.
- No pre-release hooks are configured. Quality checks must be run manually.
- Running `cargo release <level>` without `--execute` is a dry run.

Only the maintainer should execute or publish a release.

## Prerequisites

1. Install `cargo-release`:

   ```bash
   cargo install cargo-release
   ```

2. Confirm the current behavior before relying on this document:

   ```bash
   cargo release --version
   cargo release config
   ```

3. Start from the `main` branch with a clean working tree:

   ```bash
   git switch main
   git pull --ff-only origin main
   git status --short --branch
   ```

4. Ensure `[Unreleased]` in `CHANGELOG.md` describes every user-visible,
   breaking, fixed, removed, and security-relevant change.

## Required Quality Checks

Run these manually before preparing the release:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
cargo publish --dry-run
```

Also run the real-repository smoke/configuration matrix when behavior affecting
worktree creation, synchronization, removal, shell integration, or agent state
has changed.

## Release Steps

### 1. Choose the version

Follow Semantic Versioning:

- `patch`: compatible bug fixes
- `minor`: backward-compatible features or meaningful pre-1.0 changes
- `major`: incompatible API or behavior changes after 1.0

Call out breaking pre-1.0 behavior explicitly in the changelog.

### 2. Preview the release

`cargo-release` defaults to dry-run mode:

```bash
cargo release minor
```

Review the proposed version, changelog replacement, package set, commit, and
tag. Use `patch`, `minor`, `major`, or an explicit version as appropriate.

### 3. Execute the release commit and tag

This is an outward-facing action. It updates files, creates the release commit
and tag, and pushes them because `push = true`:

```bash
cargo release minor --execute
```

It does **not** publish to crates.io because `publish = false`.

### 4. Verify Git state

```bash
git status --short --branch
git log -1 --oneline
git tag --points-at HEAD
git ls-remote --tags origin
```

Confirm that:

- `Cargo.toml` and `Cargo.lock` contain the intended version.
- `CHANGELOG.md` has a new dated version section.
- `[Unreleased]` is empty and its comparison link starts at the new tag.
- The release commit and tag are present on the remote.

### 5. Publish manually

Publishing is intentionally separate and irreversible:

```bash
cargo publish --dry-run
cargo publish
```

Verify the published version on crates.io after the command succeeds.

## Changelog Automation

`release.toml` performs two replacements:

1. It inserts `## [<version>] - <date>` below `## [Unreleased]`.
2. It updates the comparison links at the bottom of `CHANGELOG.md`.

Before execution, the current link must follow this form:

```markdown
[Unreleased]: https://github.com/cafreeman/worktree/compare/v0.6.0...HEAD
```

Do not manually create the dated release heading before running
`cargo release`; the configured replacement creates it.

## Failure and Recovery

### Dry run fails

No release state was changed. Fix the reported issue and rerun the dry run.

### Execution fails before push

Inspect `git status`, the latest commit, and local tags before changing
anything. Preserve unrelated work. Revert only the state created by the failed
release attempt.

### Execution pushed but publishing has not happened

Do not rewrite or force-push shared history automatically. Coordinate the
correction, then use a normal follow-up commit or a new version. Delete a
published remote tag only when the maintainer explicitly decides that is the
correct recovery.

### Publishing fails

The Git release may already exist. Fix authentication, network, package, or
dependency issues, rerun `cargo publish --dry-run`, then retry `cargo publish`
for the same version if crates.io has not accepted it.

### A bad version was published

crates.io releases cannot be overwritten. Yank the affected version and
publish a corrected version:

```bash
cargo yank --version <version>
```

## Release Checklist

- [ ] `main` is current and the working tree is clean.
- [ ] `[Unreleased]` fully describes the release.
- [ ] Formatting, Clippy, tests, release build, and publish dry-run pass.
- [ ] Real-repository behavior has been smoke-tested where relevant.
- [ ] `cargo release <level>` dry-run output is reviewed.
- [ ] The maintainer approves the version.
- [ ] `cargo release <level> --execute` creates and pushes the commit/tag.
- [ ] Version files, changelog heading, links, commit, and tag are verified.
- [ ] `cargo publish --dry-run` passes for the release commit.
- [ ] `cargo publish` is run explicitly by the maintainer.
- [ ] The crates.io release is verified.
