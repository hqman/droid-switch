use anyhow::Result;

use super::{ensure_home, fmt_identity};
use crate::cli::ImportArgs;
use crate::factory::identity_from_dir;
use crate::paths::Paths;
use crate::profile;
use crate::state::State;

pub fn run(paths: &Paths, args: ImportArgs) -> Result<()> {
    ensure_home(paths)?;

    if paths.profile_dir(&args.name).exists() && !args.force {
        anyhow::bail!(
            "profile {:?} already exists - pass --force to overwrite",
            args.name
        );
    }
    profile::snapshot_live(paths, &args.name)?;

    let mut state = State::load(&paths.state_file())?;
    state.active = Some(args.name.clone());
    state.save(&paths.state_file())?;

    let id = identity_from_dir(&paths.profile_dir(&args.name));
    println!(
        "saved current login as profile '{}'  ({})",
        args.name,
        fmt_identity(&id)
    );
    Ok(())
}
