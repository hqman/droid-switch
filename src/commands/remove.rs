use std::io::{self, Write};

use anyhow::Result;

use crate::cli::RemoveArgs;
use crate::paths::Paths;
use crate::profile;
use crate::state::State;

pub fn run(paths: &Paths, args: RemoveArgs) -> Result<()> {
    if !args.yes {
        print!("delete profile {:?}? [y/N] ", args.name);
        io::stdout().flush().ok();
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        let ans = buf.trim().to_ascii_lowercase();
        if ans != "y" && ans != "yes" {
            println!("aborted");
            return Ok(());
        }
    }
    profile::remove(paths, &args.name)?;
    let mut state = State::load(&paths.state_file())?;
    if state.active.as_deref() == Some(args.name.as_str()) {
        state.active = None;
        state.save(&paths.state_file())?;
    }
    println!("removed profile '{}'", args.name);
    Ok(())
}
