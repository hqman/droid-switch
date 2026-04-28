use anyhow::Result;
use serde_json::json;

use super::fmt_identity;
use crate::cli::StatusArgs;
use crate::factory::identity_from_dir;
use crate::paths::Paths;
use crate::state::State;

pub fn run(paths: &Paths, args: StatusArgs) -> Result<()> {
    let state = State::load(&paths.state_file())?;
    let id = identity_from_dir(&paths.factory);

    if args.json {
        let v = json!({
            "active": state.active,
            "email": id.email,
            "subject": id.subject,
            "expires_at": id.expires_at.map(|e| e.to_rfc3339()),
            "factory_dir": paths.factory.display().to_string(),
            "dsw_home": paths.home.display().to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&v)?);
        return Ok(());
    }

    println!("profile : {}", state.active.as_deref().unwrap_or("(none)"));
    println!("identity: {}", fmt_identity(&id));
    println!("factory : {}", paths.factory.display());
    println!("home    : {}", paths.home.display());
    Ok(())
}
