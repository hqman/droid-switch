use anyhow::Result;

use super::{ensure_home, fmt_identity};
use crate::backup;
use crate::cli::UseArgs;
use crate::factory::identity_from_dir;
use crate::paths::{Paths, AUTH_FILES};
use crate::profile;
use crate::state::State;

pub fn run(paths: &Paths, args: UseArgs) -> Result<()> {
    ensure_home(paths)?;

    let target = &args.name;
    if !paths.profile_dir(target).is_dir() {
        anyhow::bail!(
            "profile {:?} does not exist - run `dsw list` to see available profiles",
            target
        );
    }

    let mut state = State::load(&paths.state_file())?;
    let prev = state.active.clone();

    // Already active?
    if prev.as_deref() == Some(target.as_str()) {
        let id = identity_from_dir(&paths.factory);
        println!("already on '{target}'  ({})", fmt_identity(&id));
        return Ok(());
    }

    // 1. Auto-backup of current live state.
    let backup_path = backup::create(paths, prev.as_deref())?;

    // 2. If the previous active profile is one we know, refresh its snapshot
    //    from the live files so any token refresh that happened in-session is
    //    persisted before we overwrite.
    if let Some(name) = prev.as_deref() {
        if paths.profile_dir(name).is_dir() && any_live(paths) {
            // Best-effort; don't fail the switch if this fails.
            let _ = profile::snapshot_live(paths, name);
        }
    }

    // 3. Atomic-ish swap: write to live atomically per-file via copy_secure.
    if let Err(e) = profile::restore_to_live(paths, target) {
        // Roll back to the backup we just took so we never leave the user
        // half-switched.
        if let Some(b) = &backup_path {
            let id = b.file_name().unwrap().to_string_lossy();
            let _ = backup::restore(paths, &id);
        }
        return Err(e.context("switch failed; rolled back to backup"));
    }

    state.active = Some(target.clone());
    state.save(&paths.state_file())?;

    let id = identity_from_dir(&paths.factory);
    println!("-> {}  ({})", target, fmt_identity(&id));
    if let Some(p) = backup_path {
        let id = p.file_name().unwrap().to_string_lossy();
        println!("  backup: {}", id);
    }
    Ok(())
}

fn any_live(paths: &Paths) -> bool {
    AUTH_FILES.iter().any(|f| paths.factory.join(f).is_file())
}
