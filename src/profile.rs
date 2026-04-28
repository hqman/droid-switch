//! Profile = a saved snapshot of the Factory droid auth bundle.

use std::fs;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use anyhow::{anyhow, Context, Result};

use crate::paths::{validate_profile_name, Paths, AUTH_FILES};

/// Snapshot the live `~/.factory/` auth files into a named profile dir.
/// At least one auth file must exist on the live side or this errors.
pub fn snapshot_live(paths: &Paths, name: &str) -> Result<usize> {
    validate_profile_name(name)?;
    let dst = paths.profile_dir(name);
    fs::create_dir_all(&dst).with_context(|| format!("create {}", dst.display()))?;
    set_dir_perms_700(&dst)?;

    let mut copied = 0usize;
    for f in AUTH_FILES {
        let src = paths.factory.join(f);
        let to = dst.join(f);
        if src.is_file() {
            copy_secure(&src, &to)?;
            copied += 1;
        } else {
            // Profile shouldn't carry stale files from a previous account.
            let _ = fs::remove_file(&to);
        }
    }
    if copied == 0 {
        // Roll back the empty dir to keep `list` honest.
        let _ = fs::remove_dir_all(&dst);
        return Err(anyhow!(
            "no auth files found in {} - log in with `droid` first",
            paths.factory.display()
        ));
    }
    Ok(copied)
}

/// Activate a profile by copying its files back over the live auth files.
/// Files missing from the profile dir are removed from the live dir to
/// avoid mixing two accounts' credentials.
pub fn restore_to_live(paths: &Paths, name: &str) -> Result<()> {
    validate_profile_name(name)?;
    let src = paths.profile_dir(name);
    if !src.is_dir() {
        return Err(anyhow!("profile {name:?} does not exist"));
    }
    fs::create_dir_all(&paths.factory)?;

    for f in AUTH_FILES {
        let s = src.join(f);
        let d = paths.factory.join(f);
        if s.is_file() {
            copy_secure(&s, &d)?;
        } else {
            let _ = fs::remove_file(&d);
        }
    }
    Ok(())
}

/// Delete a profile directory.
pub fn remove(paths: &Paths, name: &str) -> Result<()> {
    validate_profile_name(name)?;
    let p = paths.profile_dir(name);
    if !p.is_dir() {
        return Err(anyhow!("profile {name:?} does not exist"));
    }
    fs::remove_dir_all(&p).with_context(|| format!("remove {}", p.display()))?;
    Ok(())
}

/// Rename a profile directory.
pub fn rename(paths: &Paths, old: &str, new: &str) -> Result<()> {
    validate_profile_name(old)?;
    validate_profile_name(new)?;
    let from = paths.profile_dir(old);
    let to = paths.profile_dir(new);
    if !from.is_dir() {
        return Err(anyhow!("profile {old:?} does not exist"));
    }
    if to.exists() {
        return Err(anyhow!("profile {new:?} already exists"));
    }
    fs::rename(&from, &to)
        .with_context(|| format!("rename {} -> {}", from.display(), to.display()))?;
    Ok(())
}

/// All profile names sorted alphabetically.
pub fn list(paths: &Paths) -> Result<Vec<String>> {
    let dir = paths.profiles_dir();
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(n) = entry.file_name().to_str() {
                if validate_profile_name(n).is_ok() {
                    names.push(n.to_string());
                }
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Copy a file with `0600` perms on the destination (best-effort on unix).
pub fn copy_secure(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(from, to).with_context(|| format!("copy {} -> {}", from.display(), to.display()))?;
    set_file_perms_600(to)?;
    Ok(())
}

#[cfg(unix)]
fn set_file_perms_600(path: &Path) -> Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_file_perms_600(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_dir_perms_700(dir: &Path) -> Result<()> {
    let mut perms = fs::metadata(dir)?.permissions();
    perms.set_mode(0o700);
    fs::set_permissions(dir, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_dir_perms_700(_dir: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::testing::write_synthetic_bundle;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Paths) {
        let td = TempDir::new().unwrap();
        let paths = Paths::new(td.path().join("dsw"), td.path().join("factory"));
        (td, paths)
    }

    #[test]
    fn snapshot_then_restore() {
        let (_td, paths) = setup();
        write_synthetic_bundle(&paths.factory, "a@x.com", 1_900_000_000);

        let copied = snapshot_live(&paths, "main").unwrap();
        assert_eq!(copied, 2); // auth.v2.file + auth.v2.key

        // Wipe live files.
        for p in paths.live_auth_files() {
            let _ = fs::remove_file(&p);
        }
        // Restore.
        restore_to_live(&paths, "main").unwrap();
        assert!(paths.factory.join("auth.v2.file").is_file());
        assert!(paths.factory.join("auth.v2.key").is_file());
    }

    #[test]
    fn snapshot_errors_when_no_live_auth() {
        let (_td, paths) = setup();
        fs::create_dir_all(&paths.factory).unwrap();
        let err = snapshot_live(&paths, "main").unwrap_err();
        assert!(err.to_string().contains("no auth files"));
    }

    #[test]
    fn restore_errors_for_missing_profile() {
        let (_td, paths) = setup();
        let err = restore_to_live(&paths, "nope").unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }

    #[test]
    fn list_sorted() {
        let (_td, paths) = setup();
        for name in ["work", "main", "client-1"] {
            write_synthetic_bundle(&paths.factory, &format!("{name}@x.com"), 1_900_000_000);
            snapshot_live(&paths, name).unwrap();
        }
        assert_eq!(list(&paths).unwrap(), vec!["client-1", "main", "work"]);
    }

    #[test]
    fn rename_works() {
        let (_td, paths) = setup();
        write_synthetic_bundle(&paths.factory, "a@x.com", 1_900_000_000);
        snapshot_live(&paths, "old").unwrap();
        rename(&paths, "old", "new").unwrap();
        assert_eq!(list(&paths).unwrap(), vec!["new"]);
    }

    #[test]
    fn rename_collision() {
        let (_td, paths) = setup();
        write_synthetic_bundle(&paths.factory, "a@x.com", 1_900_000_000);
        snapshot_live(&paths, "a").unwrap();
        snapshot_live(&paths, "b").unwrap();
        let err = rename(&paths, "a", "b").unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn restore_removes_stale_files() {
        let (_td, paths) = setup();
        // Save a profile with only the v2 bundle (no auth.encrypted).
        write_synthetic_bundle(&paths.factory, "a@x.com", 1_900_000_000);
        snapshot_live(&paths, "p1").unwrap();
        // Now drop a stray legacy file in live.
        fs::write(paths.factory.join("auth.encrypted"), "stale").unwrap();
        restore_to_live(&paths, "p1").unwrap();
        assert!(!paths.factory.join("auth.encrypted").exists());
    }
}
