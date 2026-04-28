use anyhow::Result;

use crate::cli::RenameArgs;
use crate::paths::Paths;
use crate::profile;
use crate::state::State;

pub fn run(paths: &Paths, args: RenameArgs) -> Result<()> {
    profile::rename(paths, &args.old, &args.new)?;
    let mut state = State::load(&paths.state_file())?;
    if state.active.as_deref() == Some(args.old.as_str()) {
        state.active = Some(args.new.clone());
        state.save(&paths.state_file())?;
    }
    println!("renamed '{}' -> '{}'", args.old, args.new);
    Ok(())
}
