use anyhow::Result;
use comfy_table::{presets::ASCII_FULL, Table};
use serde::Serialize;

use super::fmt_expiry;
use crate::cli::ListArgs;
use crate::factory::{identity_from_dir, Identity};
use crate::paths::Paths;
use crate::profile;
use crate::state::State;

#[derive(Serialize)]
struct Row {
    name: String,
    active: bool,
    email: Option<String>,
    expires_at: Option<String>,
}

pub fn run(paths: &Paths, args: ListArgs) -> Result<()> {
    let names = profile::list(paths)?;
    let active = State::load(&paths.state_file())?.active;

    let rows: Vec<Row> = names
        .iter()
        .map(|name| {
            let id: Identity = identity_from_dir(&paths.profile_dir(name));
            Row {
                name: name.clone(),
                active: active.as_deref() == Some(name.as_str()),
                email: id.email,
                expires_at: id.expires_at.map(|e| e.to_rfc3339()),
            }
        })
        .collect();

    if args.json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if rows.is_empty() {
        println!("no profiles yet - try `dsw import <name>` or `dsw add <name>`");
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(ASCII_FULL);
    table.set_header(vec!["", "name", "email", "token"]);
    for r in &rows {
        let active_mark = if r.active { "*" } else { "" };
        let email = r.email.clone().unwrap_or_else(|| "-".into());
        let exp = r
            .expires_at
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| fmt_expiry(d.with_timezone(&chrono::Utc)))
            .unwrap_or_else(|| "-".into());
        table.add_row(vec![active_mark.to_string(), r.name.clone(), email, exp]);
    }
    println!("{table}");
    Ok(())
}
