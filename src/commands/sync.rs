use anyhow::Result;

use super::{ensure_home, fmt_identity};
use crate::cli::SyncArgs;
use crate::factory::{identity_from_dir, Identity};
use crate::paths::{Paths, AUTH_FILES};
use crate::profile;
use crate::state::State;

pub fn run(paths: &Paths, args: SyncArgs) -> Result<()> {
    ensure_home(paths)?;

    if !has_live_auth(paths) {
        anyhow::bail!(
            "no auth files found in {} - start `droid` and log in first",
            paths.factory.display()
        );
    }

    if args.all {
        return sync_all(paths);
    }

    sync_active(paths)
}

fn sync_active(paths: &Paths) -> Result<()> {
    let state = State::load(&paths.state_file())?;
    let Some(active) = state.active else {
        anyhow::bail!("no active profile - run `dsw use <name>` or `dsw import <name>` first");
    };

    if !paths.profile_dir(&active).is_dir() {
        anyhow::bail!(
            "active profile {:?} does not exist - run `dsw list` to see available profiles",
            active
        );
    }

    profile::snapshot_live(paths, &active)?;

    let id = identity_from_dir(&paths.profile_dir(&active));
    println!(
        "synced current login to profile '{}'  ({})",
        active,
        fmt_identity(&id)
    );
    Ok(())
}

fn sync_all(paths: &Paths) -> Result<()> {
    let live = identity_from_dir(&paths.factory);
    if live.email.is_none() && live.subject.is_none() {
        anyhow::bail!("live identity has no email or subject - cannot match profiles safely");
    }

    let mut synced = Vec::new();
    for name in profile::list(paths)? {
        let saved = identity_from_dir(&paths.profile_dir(&name));
        if identities_match(&live, &saved) {
            profile::snapshot_live(paths, &name)?;
            synced.push(name);
        }
    }

    if synced.is_empty() {
        println!(
            "no matching profiles for current login  ({})",
            fmt_identity(&live)
        );
    } else {
        println!(
            "synced current login to {} matching profile(s): {}  ({})",
            synced.len(),
            synced.join(", "),
            fmt_identity(&live)
        );
    }
    Ok(())
}

fn identities_match(a: &Identity, b: &Identity) -> bool {
    if let (Some(a_email), Some(b_email)) = (&a.email, &b.email) {
        return a_email == b_email;
    }
    a.subject.is_some() && a.subject == b.subject
}

fn has_live_auth(paths: &Paths) -> bool {
    AUTH_FILES.iter().any(|f| paths.factory.join(f).is_file())
}
