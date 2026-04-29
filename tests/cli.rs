//! End-to-end CLI tests. We drive the compiled `dsw` binary against a
//! synthetic `~/.factory` directory containing valid AES-GCM-encrypted
//! credentials, so every command runs through the real code path except the
//! one thing that needs a real browser: the OAuth login itself.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use dsw::factory::testing::{write_legacy_bundle, write_synthetic_bundle};
use predicates::prelude::*;
use tempfile::TempDir;

struct Sandbox {
    _td: TempDir,
    home: std::path::PathBuf,
    factory: std::path::PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let td = TempDir::new().unwrap();
        let home = td.path().join("dsw");
        let factory = td.path().join("factory");
        fs::create_dir_all(&factory).unwrap();
        Self {
            _td: td,
            home,
            factory,
        }
    }

    fn cmd(&self) -> Command {
        let mut c = Command::cargo_bin("dsw").unwrap();
        c.env("DSW_HOME", &self.home)
            .env("FACTORY_DIR", &self.factory)
            // Make sure we don't accidentally pick up the user's PATH droid:
            .env_remove("HOME");
        c
    }

    fn write_live(&self, email: &str) {
        // Wipe and rewrite.
        for f in ["auth.v2.file", "auth.v2.key", "auth.encrypted"] {
            let _ = fs::remove_file(self.factory.join(f));
        }
        write_synthetic_bundle(&self.factory, email, 1_900_000_000);
    }
}

fn read_live(factory: &Path) -> Vec<u8> {
    fs::read(factory.join("auth.v2.file")).unwrap()
}

fn read_profile(home: &Path, name: &str) -> Vec<u8> {
    fs::read(home.join("profiles").join(name).join("auth.v2.file")).unwrap()
}

#[test]
fn import_then_status_then_list() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");

    sb.cmd()
        .args(["import", "main"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice@example.com"));

    sb.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice@example.com"))
        .stdout(predicate::str::contains("main"));

    sb.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main"))
        .stdout(predicate::str::contains("alice@example.com"));
}

#[test]
fn no_args_suggests_import_when_live_login_exists() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");

    sb.cmd()
        .assert()
        .success()
        .stdout(predicate::str::contains("dsw import main"));
}

#[test]
fn no_args_suggests_login_when_no_live_login_exists() {
    let sb = Sandbox::new();

    sb.cmd()
        .assert()
        .success()
        .stdout(predicate::str::contains("No Droid login was found"))
        .stdout(predicate::str::contains("droid"))
        .stdout(predicate::str::contains("dsw add main"));
}

#[test]
fn empty_list_suggests_import_when_live_login_exists() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");

    sb.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No profiles yet"))
        .stdout(predicate::str::contains("dsw import main"));
}

#[test]
fn empty_list_suggests_login_when_no_live_login_exists() {
    let sb = Sandbox::new();

    sb.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No Droid login was found"))
        .stdout(predicate::str::contains("dsw add main"));
}

#[test]
fn switch_between_two_profiles_round_trips() {
    let sb = Sandbox::new();

    // 1. Save 'main' from alice.
    sb.write_live("alice@example.com");
    sb.cmd().args(["import", "main"]).assert().success();
    let alice_bytes = read_live(&sb.factory);

    // 2. Switch live to bob, save 'work'.
    sb.write_live("bob@example.com");
    sb.cmd().args(["import", "work"]).assert().success();
    let bob_bytes = read_live(&sb.factory);
    assert_ne!(alice_bytes, bob_bytes);

    // 3. Switch back to main; live bytes should match alice's snapshot.
    sb.cmd()
        .args(["use", "main"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice@example.com"));
    assert_eq!(read_live(&sb.factory), alice_bytes);

    // 4. Switch to work; bytes match bob's snapshot.
    sb.cmd()
        .args(["use", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bob@example.com"));
    assert_eq!(read_live(&sb.factory), bob_bytes);
}

#[test]
fn use_unknown_profile_errors() {
    let sb = Sandbox::new();
    sb.cmd()
        .args(["use", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn import_force_overwrites() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");
    sb.cmd().args(["import", "main"]).assert().success();
    sb.write_live("bob@example.com");

    sb.cmd()
        .args(["import", "main"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    sb.cmd()
        .args(["import", "main", "--force"])
        .assert()
        .success();

    sb.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bob@example.com"));
}

#[test]
fn rename_updates_active_marker() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");
    sb.cmd().args(["import", "main"]).assert().success();
    sb.cmd()
        .args(["rename", "main", "personal"])
        .assert()
        .success();
    sb.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("personal"));
}

#[test]
fn remove_yes_clears_active() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");
    sb.cmd().args(["import", "main"]).assert().success();
    sb.cmd().args(["remove", "main", "-y"]).assert().success();
    sb.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No profiles yet"));
}

#[test]
fn legacy_only_bundle_still_identified() {
    let sb = Sandbox::new();
    write_legacy_bundle(&sb.factory, "legacy@example.com", 1_900_000_000);
    sb.cmd().args(["import", "old"]).assert().success();
    sb.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("legacy@example.com"));
}

#[test]
fn switch_creates_backup() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");
    sb.cmd().args(["import", "main"]).assert().success();
    sb.write_live("bob@example.com");
    sb.cmd().args(["import", "work"]).assert().success();

    // Switching from work -> main should create a backup of work's live state.
    sb.cmd().args(["use", "main"]).assert().success();

    sb.cmd()
        .args(["backup", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bob@example.com"));
}

#[test]
fn list_json_is_parseable() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");
    sb.cmd().args(["import", "main"]).assert().success();

    let out = sb.cmd().args(["list", "--json"]).assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(v.is_array());
    assert_eq!(v[0]["name"], "main");
    assert_eq!(v[0]["email"], "alice@example.com");
    assert_eq!(v[0]["active"], true);
}

#[test]
fn invalid_profile_name_rejected() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");
    sb.cmd()
        .args(["import", "Bad Name"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("profile name"));
}

#[test]
fn use_same_profile_is_idempotent() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");
    sb.cmd().args(["import", "main"]).assert().success();
    sb.cmd()
        .args(["use", "main"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already on 'main'"));
}

#[test]
fn sync_updates_active_profile_from_live_auth() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");
    sb.cmd().args(["import", "main"]).assert().success();

    sb.write_live("alice.fresh@example.com");
    sb.cmd()
        .args(["sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice.fresh@example.com"));

    sb.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice.fresh@example.com"));
}

#[test]
fn sync_requires_active_profile() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");

    sb.cmd()
        .args(["sync"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no active profile"));
}

#[test]
fn sync_requires_live_auth_without_deleting_profile() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");
    sb.cmd().args(["import", "main"]).assert().success();
    for f in ["auth.v2.file", "auth.v2.key", "auth.encrypted"] {
        let _ = fs::remove_file(sb.factory.join(f));
    }

    sb.cmd()
        .args(["sync"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no auth files found"));

    sb.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice@example.com"));
}

#[test]
fn sync_all_updates_matching_profiles_only() {
    let sb = Sandbox::new();

    sb.write_live("alice@example.com");
    sb.cmd().args(["import", "main"]).assert().success();
    let old_main = read_profile(&sb.home, "main");

    sb.write_live("bob@example.com");
    sb.cmd().args(["import", "work"]).assert().success();
    let old_work = read_profile(&sb.home, "work");

    sb.write_live("alice@example.com");
    let fresh_alice = read_live(&sb.factory);
    assert_ne!(fresh_alice, old_main);

    sb.cmd()
        .args(["sync", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 matching profile(s): main"));

    assert_eq!(read_profile(&sb.home, "main"), fresh_alice);
    assert_eq!(read_profile(&sb.home, "work"), old_work);
}

#[test]
fn sync_all_does_not_require_active_profile() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");

    sb.cmd()
        .args(["sync", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("no matching profiles"));
}

#[test]
fn add_no_login_just_snapshots() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");
    sb.cmd()
        .args(["add", "main", "--no-login"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice@example.com"));
}

/// End-to-end exercise of the OAuth watcher: a fake `droid` (`sleep 60`) is
/// spawned, then a background thread writes a brand new bundle for a
/// different email into `~/.factory`. The watcher should detect the change,
/// SIGTERM the fake droid, and snapshot as the named profile - without the
/// user typing anything.
#[test]
fn add_watcher_detects_login_and_snapshots() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");

    // Background simulator: waits for the watcher to be ready, then writes a
    // bundle for a different email (simulates a successful OAuth login).
    let factory = sb.factory.clone();
    let ready_file = sb._td.path().join(".watcher-ready");
    let ready_clone = ready_file.clone();
    let sim = std::thread::spawn(move || {
        // Poll for up to 5s for the watcher to come up.
        for _ in 0..50 {
            if ready_clone.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        // Small extra delay so the watcher has at least one poll iteration
        // with the original `before_mtime`.
        std::thread::sleep(std::time::Duration::from_millis(50));
        for f in ["auth.v2.file", "auth.v2.key", "auth.encrypted"] {
            let _ = fs::remove_file(factory.join(f));
        }
        write_synthetic_bundle(&factory, "bob@example.com", 1_900_000_000);
    });

    sb.cmd()
        .env("DSW_DROID_BIN", "sleep")
        .env("DSW_DROID_ARGS", "10")
        .env("DSW_READY_FILE", &ready_file)
        .args(["add", "bob"])
        .timeout(std::time::Duration::from_secs(8))
        .assert()
        .success()
        .stdout(predicate::str::contains("bob@example.com"));

    sim.join().unwrap();

    // Verify the snapshot is on disk and active.
    sb.cmd()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bob@example.com"));
}

#[test]
fn add_watcher_fails_when_droid_exits_without_login() {
    let sb = Sandbox::new();
    sb.write_live("alice@example.com");

    // Fake droid exits immediately (`true`); no new bundle is ever written.
    sb.cmd()
        .env("DSW_DROID_BIN", "true")
        .args(["add", "newone"])
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .failure()
        .stderr(predicate::str::contains("no new login detected"));

    // Live bundle wasn't touched.
    sb.cmd()
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alice@example.com"));
}
