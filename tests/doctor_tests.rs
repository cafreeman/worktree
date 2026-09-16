//! Integration tests for storage diagnostics.

use anyhow::Result;
use assert_cmd::Command as AssertCommand;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;

use test_support::CliTestEnvironment;

fn get_stdout(env: &CliTestEnvironment, args: &[&str]) -> Result<String> {
    let assertion = env.run_command(args)?.assert().success();
    let output = assertion.get_output();
    Ok(String::from_utf8(output.stdout.clone())?)
}

#[test]
fn test_doctor_healthy_repo_reports_no_findings() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "doctor-healthy", "feature/doctor-healthy"])?
        .assert()
        .success();

    let output = get_stdout(&env, &["doctor"])?;
    assert!(output.contains("0 findings"));
    assert!(output.contains("healthy"));

    Ok(())
}

#[test]
fn test_doctor_reports_and_repairs_missing_origin() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "doctor-missing", "feature/doctor-missing"])?
        .assert()
        .success();

    let origins = env
        .storage_dir
        .child("test_repo")
        .child(".worktree-origins");
    origins.write_str("")?;

    env.run_command(&["doctor"])?
        .assert()
        .success()
        .stdout(predicate::str::contains("no origin entry"));

    env.run_command(&["doctor", "--fix"])?.assert().success();
    assert!(std::fs::read_to_string(origins.path())?.contains("doctor-missing -> "));

    Ok(())
}

#[test]
fn test_doctor_removes_malformed_and_orphan_metadata() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "doctor-valid", "feature/doctor-valid"])?
        .assert()
        .success();

    let origins = env
        .storage_dir
        .child("test_repo")
        .child(".worktree-origins");
    origins.write_str("malformed\norphan -> /missing/origin\ndoctor-valid -> /wrong\n")?;

    env.run_command(&["doctor", "--fix"])?.assert().success();

    let content = std::fs::read_to_string(origins.path())?;
    assert!(!content.contains("malformed"));
    assert!(!content.contains("orphan"));
    assert!(content.contains("doctor-valid -> "));

    Ok(())
}

#[test]
fn test_doctor_reports_unregistered_storage_directory() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.storage_dir
        .child("test_repo")
        .child("stray")
        .create_dir_all()?;
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            "--detach",
            env.storage_dir
                .child("test_repo")
                .child("stray")
                .path()
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("stray path is not UTF-8"))?,
            "HEAD",
        ])
        .current_dir(env.repo_dir.path())
        .output()?;
    assert!(output.status.success());

    let output = get_stdout(&env, &["doctor"])?;
    assert!(output.contains("no origin entry"));
    assert!(output.contains("not registered with the tool"));

    Ok(())
}

#[test]
fn test_doctor_warns_about_repo_name_collision() -> Result<()> {
    let env = CliTestEnvironment::new()?;
    env.run_command(&["create", "first", "feature/first"])?
        .assert()
        .success();

    let second_root = TempDir::new()?;
    let second_repo = second_root.path().join("test_repo");
    std::fs::create_dir_all(&second_repo)?;
    for args in [
        &["init"][..],
        &["config", "user.name", "Test User"][..],
        &["config", "user.email", "test@example.com"][..],
    ] {
        let output = Command::new("git")
            .args(args)
            .current_dir(&second_repo)
            .output()?;
        assert!(output.status.success());
    }
    std::fs::write(second_repo.join("README.md"), "# second")?;
    for args in [&["add", "."][..], &["commit", "-m", "initial"][..]] {
        let output = Command::new("git")
            .args(args)
            .current_dir(&second_repo)
            .output()?;
        assert!(output.status.success());
    }

    AssertCommand::cargo_bin("worktree-bin")?
        .current_dir(&second_repo)
        .env("WORKTREE_STORAGE_ROOT", env.storage_dir.path())
        .args(["create", "second", "feature/second"])
        .assert()
        .success();

    let output = get_stdout(&env, &["doctor"])?;
    assert!(output.contains("repository-name collision"));

    Ok(())
}
