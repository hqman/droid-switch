pub mod add;
pub mod backup;
pub mod doctor;
pub mod import;
pub mod init;
pub mod list;
pub mod remove;
pub mod rename;
pub mod status;
pub mod sync;
pub mod use_;

use std::fs;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::factory::Identity;
use crate::paths::Paths;

/// Pretty-print an identity for terminal output.
pub fn fmt_identity(id: &Identity) -> String {
    let email = id.display_email();
    match id.expires_at {
        Some(exp) => format!("{email}  ({})", fmt_expiry(exp)),
        None => email,
    }
}

pub fn fmt_expiry(exp: DateTime<Utc>) -> String {
    let now = Utc::now();
    if exp <= now {
        "expired".to_string()
    } else {
        let dur = exp - now;
        let days = dur.num_days();
        if days >= 1 {
            format!("expires in {}d", days)
        } else {
            format!("expires in {}h", dur.num_hours().max(1))
        }
    }
}

/// Ensure `~/.dsw/{profiles,backups}` exist with `0700` perms.
pub fn ensure_home(paths: &Paths) -> Result<()> {
    for dir in [&paths.home, &paths.profiles_dir(), &paths.backups_dir()] {
        fs::create_dir_all(dir)?;
        set_dir_private(dir)?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_dir_private(dir: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(dir)?.permissions();
    perms.set_mode(0o700);
    let _ = fs::set_permissions(dir, perms);
    Ok(())
}

#[cfg(not(unix))]
fn set_dir_private(_dir: &std::path::Path) -> Result<()> {
    Ok(())
}
