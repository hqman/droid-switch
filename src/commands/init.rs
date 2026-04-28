use anyhow::Result;

use super::{ensure_home, fmt_identity};
use crate::cli::InitArgs;
use crate::factory::identity_from_dir;
use crate::paths::{Paths, AUTH_FILES};
use crate::profile;
use crate::state::State;

pub fn run(paths: &Paths, args: InitArgs) -> Result<()> {
    ensure_home(paths)?;
    println!("created {}", paths.home.display());

    // If user passed --import-as, snapshot the live login.
    if let Some(name) = args.import_as.as_deref() {
        let any_live = AUTH_FILES.iter().any(|f| paths.factory.join(f).is_file());
        if !any_live {
            println!(
                "no live login found in {} - skipping import",
                paths.factory.display()
            );
        } else {
            profile::snapshot_live(paths, name)?;
            let mut state = State::load(&paths.state_file())?;
            state.active = Some(name.to_string());
            state.save(&paths.state_file())?;
            let id = identity_from_dir(&paths.profile_dir(name));
            println!(
                "imported current login as profile '{name}'  ({})",
                fmt_identity(&id)
            );
        }
    } else {
        println!("tip: run `dsw import <name>` to save the current `droid` login as a profile");
    }
    Ok(())
}
