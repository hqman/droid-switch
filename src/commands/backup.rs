use std::io::{self, Write};

use anyhow::Result;
use comfy_table::{presets::ASCII_FULL, Table};

use crate::backup;
use crate::cli::{BackupArgs, BackupCommand};
use crate::factory::identity_from_dir;
use crate::paths::Paths;

pub fn run(paths: &Paths, args: BackupArgs) -> Result<()> {
    match args.command {
        BackupCommand::List => list(paths),
        BackupCommand::Restore { id, yes } => restore(paths, &id, yes),
        BackupCommand::Prune { keep } => prune(paths, keep),
    }
}

fn list(paths: &Paths) -> Result<()> {
    let entries = backup::list(paths)?;
    if entries.is_empty() {
        println!("no backups");
        return Ok(());
    }
    let mut table = Table::new();
    table.load_preset(ASCII_FULL);
    table.set_header(vec!["id", "previous profile", "email"]);
    for e in &entries {
        let prev = e
            .meta
            .as_ref()
            .and_then(|m| m.previous_profile.clone())
            .unwrap_or_else(|| "-".into());
        let email = e
            .meta
            .as_ref()
            .and_then(|m| m.email.clone())
            .unwrap_or_else(|| "-".into());
        table.add_row(vec![e.id.clone(), prev, email]);
    }
    println!("{table}");
    Ok(())
}

fn restore(paths: &Paths, id: &str, yes: bool) -> Result<()> {
    if !yes {
        let dir = paths.backups_dir().join(id);
        let id_str = identity_from_dir(&dir);
        print!(
            "restore backup {id} ({})? this will overwrite the live droid login. [y/N] ",
            id_str.display_email()
        );
        io::stdout().flush().ok();
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        let ans = buf.trim().to_ascii_lowercase();
        if ans != "y" && ans != "yes" {
            println!("aborted");
            return Ok(());
        }
    }
    backup::restore(paths, id)?;
    println!("restored backup {id}");
    Ok(())
}

fn prune(paths: &Paths, keep: usize) -> Result<()> {
    let removed = backup::prune(paths, keep)?;
    println!("pruned {removed} backup(s); kept {keep} most recent");
    Ok(())
}
