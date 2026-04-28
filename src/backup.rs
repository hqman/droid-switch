//! Auto-snapshot the live auth files into `~/.dsw/backups/<timestamp>/`
//! before each `use`. Lets us roll back if a switch turns out to be wrong.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::paths::{Paths, AUTH_FILES};
use crate::profile::copy_secure;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMeta {
    /// Profile name that was active when this backup was taken (if known).
    pub previous_profile: Option<String>,
    /// Email of the account that was live (best-effort).
    pub email: Option<String>,
    /// ISO-8601 UTC timestamp.
    pub timestamp: String,
}

#[derive(Debug, Clone)]
pub struct BackupEntry {
    pub id: String,
    pub path: PathBuf,
    pub meta: Option<BackupMeta>,
}

/// Snapshot the live auth files into a new timestamped directory. No-op (and
/// returns Ok(None)) if there are no live auth files at all.
pub fn create(paths: &Paths, previous_profile: Option<&str>) -> Result<Option<PathBuf>> {
    let any_live = AUTH_FILES.iter().any(|f| paths.factory.join(f).is_file());
    if !any_live {
        return Ok(None);
    }
    let id = Utc::now().format("%Y%m%dT%H%M%S%3fZ").to_string();
    let dir = paths.backups_dir().join(&id);
    fs::create_dir_all(&dir)?;

    for f in AUTH_FILES {
        let s = paths.factory.join(f);
        if s.is_file() {
            copy_secure(&s, &dir.join(f))?;
        }
    }

    let email = crate::factory::identity_from_dir(&paths.factory).email;
    let meta = BackupMeta {
        previous_profile: previous_profile.map(str::to_string),
        email,
        timestamp: id.clone(),
    };
    fs::write(dir.join("meta.json"), serde_json::to_vec_pretty(&meta)?)?;

    Ok(Some(dir))
}

/// List backups newest first.
pub fn list(paths: &Paths) -> Result<Vec<BackupEntry>> {
    let dir = paths.backups_dir();
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let meta = read_meta(&path).ok();
        out.push(BackupEntry { id, path, meta });
    }
    out.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(out)
}

fn read_meta(dir: &Path) -> Result<BackupMeta> {
    let s = fs::read_to_string(dir.join("meta.json"))?;
    Ok(serde_json::from_str(&s)?)
}

/// Restore a backup by id, replacing live auth files. Pre-flights existence.
pub fn restore(paths: &Paths, id: &str) -> Result<()> {
    let dir = paths.backups_dir().join(id);
    if !dir.is_dir() {
        return Err(anyhow!("backup {id:?} not found"));
    }
    fs::create_dir_all(&paths.factory)?;
    for f in AUTH_FILES {
        let s = dir.join(f);
        let d = paths.factory.join(f);
        if s.is_file() {
            copy_secure(&s, &d)?;
        } else {
            let _ = fs::remove_file(&d);
        }
    }
    Ok(())
}

/// Keep only the `keep` most-recent backups. Returns how many were removed.
pub fn prune(paths: &Paths, keep: usize) -> Result<usize> {
    let entries = list(paths)?;
    let mut removed = 0;
    for stale in entries.into_iter().skip(keep) {
        fs::remove_dir_all(&stale.path)
            .with_context(|| format!("remove {}", stale.path.display()))?;
        removed += 1;
    }
    Ok(removed)
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
    fn create_and_list_and_restore() {
        let (_td, paths) = setup();
        write_synthetic_bundle(&paths.factory, "a@x.com", 1_900_000_000);
        let b = create(&paths, Some("main")).unwrap().unwrap();
        assert!(b.join("auth.v2.file").is_file());
        assert!(b.join("meta.json").is_file());

        // Mutate live, then restore.
        fs::write(paths.factory.join("auth.v2.file"), "broken").unwrap();
        let id = b.file_name().unwrap().to_string_lossy().into_owned();
        restore(&paths, &id).unwrap();
        let restored = fs::read_to_string(paths.factory.join("auth.v2.file")).unwrap();
        assert_ne!(restored, "broken");

        let entries = list(&paths).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, id);
        assert_eq!(
            entries[0]
                .meta
                .as_ref()
                .unwrap()
                .previous_profile
                .as_deref(),
            Some("main")
        );
    }

    #[test]
    fn create_noop_without_live_files() {
        let (_td, paths) = setup();
        assert!(create(&paths, None).unwrap().is_none());
    }

    #[test]
    fn prune_keeps_n_newest() {
        let (_td, paths) = setup();
        write_synthetic_bundle(&paths.factory, "a@x.com", 1_900_000_000);
        for _ in 0..5 {
            create(&paths, None).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(list(&paths).unwrap().len(), 5);
        let removed = prune(&paths, 2).unwrap();
        assert_eq!(removed, 3);
        assert_eq!(list(&paths).unwrap().len(), 2);
    }
}
