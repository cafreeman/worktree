use anyhow::{Result, bail};
use clap::Subcommand;
use std::fs;
use std::path::PathBuf;

const EMBEDDED_SKILL: &str = include_str!("../../assets/skill/SKILL.md");

#[derive(Subcommand, Clone)]
pub enum SkillAction {
    /// Install the worktree-manager agent skill into your coding agent
    Install,
    /// Remove the installed worktree-manager agent skill
    Uninstall,
    /// Update the installed skill to match the version bundled with this binary
    Update,
    /// Show whether the skill is installed and if an update is available
    Status,
}

/// Dispatches the `worktree skill` subcommand.
///
/// # Errors
/// Returns an error if file system operations fail or a precondition is not met.
pub fn run_skill_command(action: &SkillAction) -> Result<()> {
    match action {
        SkillAction::Install => install_skill(),
        SkillAction::Uninstall => uninstall_skill(),
        SkillAction::Update => update_skill(),
        SkillAction::Status => skill_status(),
    }
}

fn skill_dir() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    Ok(home.join(".agents").join("skills").join("worktree-manager"))
}

fn skill_file() -> Result<PathBuf> {
    Ok(skill_dir()?.join("SKILL.md"))
}

fn claude_skills_link() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    Ok(home.join(".claude").join("skills").join("worktree-manager"))
}

fn is_installed() -> Result<bool> {
    Ok(skill_file()?.exists())
}

fn is_current() -> Result<bool> {
    let path = skill_file()?;
    if !path.exists() {
        return Ok(false);
    }
    let installed = fs::read_to_string(&path)?;
    Ok(installed == EMBEDDED_SKILL)
}

/// Installs the embedded skill into `~/.agents/skills/worktree-manager/` and
/// creates a symlink at `~/.claude/skills/worktree-manager`.
///
/// # Errors
/// Returns an error if directory creation or file writing fails.
pub fn install_skill() -> Result<()> {
    let dir = skill_dir()?;
    let file = skill_file()?;

    if file.exists() {
        let installed = fs::read_to_string(&file)?;
        if installed == EMBEDDED_SKILL {
            println!("✅ Skill is already installed and up to date.");
            return Ok(());
        }
        // File exists but is outdated — still install (overwrite)
    }

    fs::create_dir_all(&dir)?;
    fs::write(&file, EMBEDDED_SKILL)?;
    println!("✅ Skill installed to {}", file.display());

    // Create symlink in ~/.claude/skills/
    let link = claude_skills_link()?;
    if let Some(parent) = link.parent() {
        if parent.exists() {
            if link.exists() || link.is_symlink() {
                // Remove stale or existing symlink before recreating
                #[cfg(unix)]
                fs::remove_file(&link)?;
                #[cfg(not(unix))]
                fs::remove_dir(&link).or_else(|_| fs::remove_file(&link))?;
            }
            #[cfg(unix)]
            std::os::unix::fs::symlink(&dir, &link)?;
            #[cfg(not(unix))]
            {
                // Windows: create a directory junction or copy
                bail!(
                    "Symlink creation is not supported on this platform. The skill is installed at {} but you will need to manually link it to your agent's skill directory.",
                    dir.display()
                );
            }
            println!("🔗 Symlink created at {}", link.display());
        } else {
            println!(
                "ℹ️  Note: {} does not exist — symlink skipped. \
                 The skill is installed at {} and will work once your agent reads from that path.",
                parent.display(),
                dir.display()
            );
        }
    }

    println!("\nTo use the skill, your coding agent should pick it up automatically.");
    println!("Run `worktree skill status` to verify.");
    Ok(())
}

/// Removes the installed skill and its symlink.
///
/// # Errors
/// Returns an error if file system removal fails.
pub fn uninstall_skill() -> Result<()> {
    let dir = skill_dir()?;
    let link = claude_skills_link()?;
    let mut removed_any = false;

    if dir.exists() {
        fs::remove_dir_all(&dir)?;
        println!("🗑️  Removed {}", dir.display());
        removed_any = true;
    }

    if link.exists() || link.is_symlink() {
        #[cfg(unix)]
        fs::remove_file(&link)?;
        #[cfg(not(unix))]
        fs::remove_dir(&link).or_else(|_| fs::remove_file(&link))?;
        println!("🗑️  Removed symlink at {}", link.display());
        removed_any = true;
    }

    if removed_any {
        println!("✅ Skill uninstalled.");
    } else {
        println!("ℹ️  Skill is not installed — nothing to remove.");
    }

    Ok(())
}

/// Overwrites the installed skill with the version embedded in this binary.
///
/// # Errors
/// Returns an error if the skill is not installed or the file cannot be written.
pub fn update_skill() -> Result<()> {
    if !is_installed()? {
        bail!("Skill is not installed. Run `worktree skill install` first.");
    }

    if is_current()? {
        println!("✅ Skill is already up to date.");
        return Ok(());
    }

    let file = skill_file()?;
    fs::write(&file, EMBEDDED_SKILL)?;
    println!("✅ Skill updated at {}", file.display());
    Ok(())
}

/// Prints the current installation status of the skill.
///
/// # Errors
/// Returns an error if the home directory cannot be determined or files cannot be read.
pub fn skill_status() -> Result<()> {
    if !is_installed()? {
        println!("❌ Skill is not installed.");
        println!("   Run `worktree skill install` to install it.");
        return Ok(());
    }

    if is_current()? {
        println!("✅ Skill is installed and up to date.");
        println!("   Location: {}", skill_file()?.display());
    } else {
        println!("⚠️  Skill is installed but an update is available.");
        println!("   Location: {}", skill_file()?.display());
        println!("   Run `worktree skill update` to apply the update.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn with_temp_home<F: FnOnce() -> Result<()>>(f: F) -> Result<()> {
        let tmp = TempDir::new()?;
        // Override HOME so dirs::home_dir() returns tmp path
        temp_env::with_var(
            "HOME",
            Some(tmp.path().to_str().expect("valid path")),
            || f(),
        )
    }

    #[test]
    fn test_install_creates_files() -> Result<()> {
        with_temp_home(|| {
            let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home"))?;
            // Create ~/.claude/skills/ so symlink can be attempted
            fs::create_dir_all(home.join(".claude").join("skills"))?;

            install_skill()?;

            let file = skill_file()?;
            assert!(file.exists(), "SKILL.md should exist after install");
            assert_eq!(fs::read_to_string(&file)?, EMBEDDED_SKILL);

            let link = claude_skills_link()?;
            assert!(link.exists() || link.is_symlink(), "symlink should exist");

            Ok(())
        })
    }

    #[test]
    fn test_install_idempotent() -> Result<()> {
        with_temp_home(|| {
            let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home"))?;
            fs::create_dir_all(home.join(".claude").join("skills"))?;

            install_skill()?;
            // Second install should succeed without error
            install_skill()?;

            assert!(skill_file()?.exists());
            Ok(())
        })
    }

    #[test]
    fn test_install_without_claude_dir() -> Result<()> {
        with_temp_home(|| {
            // Do NOT create ~/.claude/skills/ — symlink should be skipped gracefully
            install_skill()?;
            assert!(skill_file()?.exists(), "SKILL.md should still be installed");
            Ok(())
        })
    }

    #[test]
    fn test_uninstall_removes_files() -> Result<()> {
        with_temp_home(|| {
            let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home"))?;
            fs::create_dir_all(home.join(".claude").join("skills"))?;
            install_skill()?;
            assert!(skill_file()?.exists());

            uninstall_skill()?;
            assert!(!skill_dir()?.exists(), "skill dir should be removed");
            assert!(
                !claude_skills_link()?.is_symlink(),
                "symlink should be removed"
            );
            Ok(())
        })
    }

    #[test]
    fn test_uninstall_when_not_installed() -> Result<()> {
        with_temp_home(|| {
            // Should succeed even if nothing is installed
            uninstall_skill()?;
            Ok(())
        })
    }

    #[test]
    fn test_update_when_outdated() -> Result<()> {
        with_temp_home(|| {
            // Install stale content manually
            let dir = skill_dir()?;
            fs::create_dir_all(&dir)?;
            fs::write(dir.join("SKILL.md"), "old content")?;

            update_skill()?;
            assert_eq!(fs::read_to_string(skill_file()?)?, EMBEDDED_SKILL);
            Ok(())
        })
    }

    #[test]
    fn test_update_when_current() -> Result<()> {
        with_temp_home(|| {
            let dir = skill_dir()?;
            fs::create_dir_all(&dir)?;
            fs::write(dir.join("SKILL.md"), EMBEDDED_SKILL)?;

            // Should succeed and report up to date
            update_skill()?;
            Ok(())
        })
    }

    #[test]
    fn test_update_when_not_installed() -> Result<()> {
        with_temp_home(|| {
            let result = update_skill();
            assert!(result.is_err(), "update should fail when not installed");
            Ok(())
        })
    }

    #[test]
    fn test_status_not_installed() -> Result<()> {
        with_temp_home(|| {
            // Should succeed and print not-installed message
            skill_status()?;
            Ok(())
        })
    }

    #[test]
    fn test_status_installed_current() -> Result<()> {
        with_temp_home(|| {
            let dir = skill_dir()?;
            fs::create_dir_all(&dir)?;
            fs::write(dir.join("SKILL.md"), EMBEDDED_SKILL)?;

            skill_status()?;
            Ok(())
        })
    }

    #[test]
    fn test_status_installed_outdated() -> Result<()> {
        with_temp_home(|| {
            let dir = skill_dir()?;
            fs::create_dir_all(&dir)?;
            fs::write(dir.join("SKILL.md"), "stale content")?;

            skill_status()?;
            Ok(())
        })
    }
}
