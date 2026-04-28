use std::path::PathBuf;

use anyhow::Result;
use chrono::Utc;
use comfy_table::{presets::ASCII_FULL, Table};
use serde::Serialize;

use crate::cli::DoctorArgs;
use crate::factory::identity_from_dir;
use crate::paths::Paths;
use crate::profile;

#[derive(Serialize)]
struct Check {
    name: String,
    ok: bool,
    detail: String,
}

pub fn run(paths: &Paths, args: DoctorArgs) -> Result<()> {
    let mut checks: Vec<Check> = Vec::new();

    // 1. droid binary on PATH
    let droid = which_binary("droid");
    checks.push(Check {
        name: "droid binary".into(),
        ok: droid.is_some(),
        detail: droid
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "not found on PATH (install Factory droid)".into()),
    });

    // 2. ~/.factory exists
    checks.push(Check {
        name: "factory dir".into(),
        ok: paths.factory.is_dir(),
        detail: paths.factory.display().to_string(),
    });

    // 3. dsw home
    checks.push(Check {
        name: "dsw home".into(),
        ok: paths.home.is_dir(),
        detail: paths.home.display().to_string(),
    });

    check_home_permissions(paths, &mut checks);

    // 5. live identity
    let live = identity_from_dir(&paths.factory);
    let now = Utc::now();
    checks.push(Check {
        name: "live identity".into(),
        ok: live.email.is_some() && !live.is_expired(now),
        detail: format!(
            "{} {}",
            live.display_email(),
            live.expires_at
                .map(|e| if e <= now {
                    "EXPIRED".to_string()
                } else {
                    format!("exp {}", e.format("%Y-%m-%d"))
                })
                .unwrap_or_default()
        ),
    });

    // 6. each profile decodes & isn't expired
    for name in profile::list(paths)? {
        let id = identity_from_dir(&paths.profile_dir(&name));
        let expired = id.is_expired(now);
        let ok = id.email.is_some() && !expired;
        let detail = format!(
            "{}{}",
            id.display_email(),
            if expired { " (EXPIRED)" } else { "" }
        );
        checks.push(Check {
            name: format!("profile {name}"),
            ok,
            detail,
        });
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&checks)?);
    } else {
        let mut table = Table::new();
        table.load_preset(ASCII_FULL);
        table.set_header(vec!["", "check", "detail"]);
        for c in &checks {
            table.add_row(vec![
                if c.ok { "ok" } else { "fail" }.to_string(),
                c.name.clone(),
                c.detail.clone(),
            ]);
        }
        println!("{table}");
    }

    let any_fail = checks.iter().any(|c| !c.ok);
    if any_fail {
        anyhow::bail!("some checks failed");
    }
    Ok(())
}

fn which_binary(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for candidate in binary_candidates(&dir, name) {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(unix)]
fn check_home_permissions(paths: &Paths, checks: &mut Vec<Check>) {
    use std::os::unix::fs::PermissionsExt;
    if paths.home.is_dir() {
        let mode = std::fs::metadata(&paths.home)
            .map(|m| m.permissions().mode() & 0o777)
            .unwrap_or(0);
        checks.push(Check {
            name: "home perms".into(),
            ok: mode == 0o700,
            detail: format!("0o{:o}", mode),
        });
    }
}

#[cfg(not(unix))]
fn check_home_permissions(_paths: &Paths, _checks: &mut Vec<Check>) {}

fn binary_candidates(dir: &std::path::Path, name: &str) -> Vec<PathBuf> {
    let direct = dir.join(name);
    #[cfg(windows)]
    {
        let mut candidates = vec![direct];
        if std::path::Path::new(name).extension().is_none() {
            let pathext = std::env::var_os("PATHEXT")
                .map(|v| v.to_string_lossy().into_owned())
                .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());
            candidates.extend(
                pathext
                    .split(';')
                    .filter(|ext| !ext.is_empty())
                    .map(|ext| dir.join(format!("{name}{ext}"))),
            );
        }
        candidates
    }
    #[cfg(not(windows))]
    {
        vec![direct]
    }
}
