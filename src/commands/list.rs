use anyhow::Result;
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

    print_rows(&rows);
    Ok(())
}

fn print_rows(rows: &[Row]) {
    let rendered: Vec<(&str, &str, String, String)> = rows
        .iter()
        .map(|r| {
            let active = if r.active { "*" } else { " " };
            let email = r.email.clone().unwrap_or_else(|| "-".into());
            let token = r
                .expires_at
                .as_ref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|d| fmt_expiry(d.with_timezone(&chrono::Utc)))
                .unwrap_or_else(|| "-".into());
            (active, r.name.as_str(), email, token)
        })
        .collect();

    let name_w = rendered
        .iter()
        .map(|(_, name, _, _)| name.len())
        .max()
        .unwrap_or(0)
        .max("name".len());
    let email_w = rendered
        .iter()
        .map(|(_, _, email, _)| email.len())
        .max()
        .unwrap_or(0)
        .max("email".len());
    let token_w = rendered
        .iter()
        .map(|(_, _, _, token)| token.len())
        .max()
        .unwrap_or(0)
        .max("token".len());

    println!(
        "  {:name_w$}  {:email_w$}  {:token_w$}",
        "name", "email", "token"
    );
    println!("  {:-<name_w$}  {:-<email_w$}  {:-<token_w$}", "", "", "");
    for (active, name, email, token) in rendered {
        println!("{active} {name:name_w$}  {email:email_w$}  {token:token_w$}");
    }
}
