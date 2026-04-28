use std::path::Path;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

use anyhow::{anyhow, Result};

use super::{ensure_home, fmt_identity};
use crate::cli::AddArgs;
use crate::factory::identity_from_dir;
use crate::paths::{Paths, AUTH_FILES};
use crate::profile;
use crate::state::State;

/// Poll interval for watching `auth.v2.file`.
const POLL_MS: u64 = 200;
/// Once we detect a write, give droid this long to finish flushing both
/// `auth.v2.file` and `auth.v2.key` before we tear it down.
const SETTLE_MS: u64 = 1500;
/// After SIGTERM, how long to wait before SIGKILL.
#[cfg(unix)]
const KILL_GRACE_MS: u64 = 3000;
const CHILD_POLL_MS: u64 = 100;

pub fn run(paths: &Paths, args: AddArgs) -> Result<()> {
    ensure_home(paths)?;

    if paths.profile_dir(&args.name).exists() {
        anyhow::bail!(
            "profile {:?} already exists - use `dsw remove {0}` first or pick a new name",
            args.name
        );
    }

    if !args.no_login {
        // Snapshot the current login first so we can both:
        // 1. compare emails to detect a real account change after login
        // 2. roll back if the user aborts midway
        let before = identity_from_dir(&paths.factory);
        let before_mtime = mtime(&paths.factory.join("auth.v2.file"));

        if before.email.is_some() {
            // Save current as a hidden 'pre-add' rescue snapshot.
            let _ = profile::remove(paths, "_pre_add");
            profile::snapshot_live(paths, "_pre_add").ok();
        }

        println!(
            "launching `droid`. log in to the account you want to call '{}' in the browser.",
            args.name
        );
        println!("  (we'll detect the new login automatically and exit droid for you)");

        let bin = std::env::var("DSW_DROID_BIN").unwrap_or_else(|_| "droid".to_string());
        let mut cmd = Command::new(&bin);
        // Allow tests to inject extra args via DSW_DROID_ARGS.
        if let Ok(extra) = std::env::var("DSW_DROID_ARGS") {
            for a in extra.split('\u{1f}').filter(|s| !s.is_empty()) {
                cmd.arg(a);
            }
        }
        let mut child = cmd.spawn().map_err(|e| {
            anyhow!("failed to launch `{bin}` ({e}). install Factory droid or pass --no-login.")
        })?;
        let detected = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let watcher_factory = paths.factory.clone();
        let watcher_before_email = before.email.clone();
        let watcher_before_mtime = before_mtime;
        let detected_thread = detected.clone();
        let stop_thread = stop.clone();

        let watcher = thread::spawn(move || {
            login_watcher(
                &watcher_factory,
                watcher_before_email.as_deref(),
                watcher_before_mtime,
                detected_thread,
                stop_thread,
            );
        });

        // Poll the child so the watcher can ask us to terminate it using
        // platform-native process APIs.
        loop {
            if detected.load(Ordering::SeqCst) {
                terminate_child(&mut child);
                break;
            }
            if child.try_wait()?.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(CHILD_POLL_MS));
        }
        stop.store(true, Ordering::SeqCst);
        let _ = watcher.join();

        if !detected.load(Ordering::SeqCst) {
            // User exited without logging in (or logged in to the SAME account).
            anyhow::bail!(
                "no new login detected. nothing was saved. (if you logged in to the same email \
                 you already had, droid kept the same account - try `droid /logout` first.)"
            );
        }
    }

    profile::snapshot_live(paths, &args.name)?;

    let mut state = State::load(&paths.state_file())?;
    state.active = Some(args.name.clone());
    state.save(&paths.state_file())?;

    let id = identity_from_dir(&paths.profile_dir(&args.name));
    println!("saved as profile '{}'  ({})", args.name, fmt_identity(&id));
    Ok(())
}

/// File-system watcher. Polls `auth.v2.file` for change of mtime AND a
/// successful decode whose email differs from `before_email`. When detected,
/// waits for the file to settle, then tells the main thread to stop `droid`.
fn login_watcher(
    factory: &Path,
    before_email: Option<&str>,
    before_mtime: Option<SystemTime>,
    detected: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) {
    let debug = std::env::var("DSW_DEBUG").is_ok();
    let auth_v2 = factory.join("auth.v2.file");
    if debug {
        eprintln!(
            "[watcher] watching {} (before_email={:?}, before_mtime={:?})",
            auth_v2.display(),
            before_email,
            before_mtime,
        );
    }
    // Test-only sync: drop a marker so the test simulator can wait until the
    // watcher has captured `before_email` before writing the new bundle.
    if let Ok(p) = std::env::var("DSW_READY_FILE") {
        let _ = std::fs::write(&p, "ready");
    }

    loop {
        if stop.load(Ordering::SeqCst) {
            if debug {
                eprintln!("[watcher] stopping");
            }
            return;
        }

        let now_mtime = mtime(&auth_v2);
        if now_mtime != before_mtime && now_mtime.is_some() {
            let id = identity_from_dir(factory);
            if debug {
                eprintln!(
                    "[watcher] auth.v2.file changed (mtime={:?}, email={:?})",
                    now_mtime, id.email
                );
            }
            if let Some(email) = id.email.as_deref() {
                if Some(email) != before_email {
                    if debug {
                        eprintln!("[watcher] new login detected: {email}");
                    }
                    thread::sleep(Duration::from_millis(SETTLE_MS));
                    if all_auth_files_exist(factory) {
                        if debug {
                            eprintln!("[watcher] auth bundle settled");
                        }
                        detected.store(true, Ordering::SeqCst);
                    }
                    return;
                }
            }
        }

        thread::sleep(Duration::from_millis(POLL_MS));
    }
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
}

fn all_auth_files_exist(factory: &Path) -> bool {
    AUTH_FILES.iter().any(|f| factory.join(f).is_file())
}

#[cfg(unix)]
fn terminate_child(child: &mut Child) {
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }

    let attempts = KILL_GRACE_MS / CHILD_POLL_MS;
    for _ in 0..attempts {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        thread::sleep(Duration::from_millis(CHILD_POLL_MS));
    }

    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(not(unix))]
fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
