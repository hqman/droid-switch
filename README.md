# Droid Switch

![Droid Switch header](assets/social/droid-switch-header.png)

[简体中文](README_CN.md)

### A tiny account switcher for Factory Droid

[![Version](https://img.shields.io/github/v/release/hqman/droid-switch?label=version)](https://github.com/hqman/droid-switch/releases)
[![Platform](https://img.shields.io/badge/platform-macOS%20Apple%20Silicon%20%7C%20Linux%20%7C%20Windows-lightgrey)](https://github.com/hqman/droid-switch/releases)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![CI](https://github.com/hqman/droid-switch/actions/workflows/ci.yml/badge.svg)](https://github.com/hqman/droid-switch/actions/workflows/ci.yml)

Droid Switch is a small command-line tool for switching between multiple
Factory Droid accounts.

It only swaps Droid authentication files. Your sessions, history, settings, and
other `~/.factory` data stay shared.

## Why Droid Switch?

Factory Droid stores login state locally. If you use more than one account, you
usually need to log out, log in again, or manually move auth files around.

Droid Switch turns each login into a named profile, then copies only the auth
bundle when you switch accounts.

- No manual file copying.
- No separate `~/.factory` directory per account.
- No changes to sessions, history, or settings.
- Automatic backup before each switch.

## Features

- Save the current Droid login as a named profile.
- Launch Droid and capture a new login automatically.
- Switch between saved profiles without re-authenticating each time.
- List profiles with decoded account email and token expiry when available.
- Restore automatic backups if a switch was wrong.
- Run on macOS Apple Silicon, Linux, and Windows.

## Install

macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/hqman/droid-switch/main/install.sh | sh
```

Windows PowerShell:

```powershell
iwr https://raw.githubusercontent.com/hqman/droid-switch/main/install.ps1 -UseB | iex
```

The installer downloads the latest GitHub release into a user-local binary
directory. Set `DSW_VERSION`, `DSW_REPO`, or `DSW_INSTALL_DIR` to
override the default release, repository, or install location.

You can also download binaries directly from the
[Releases](https://github.com/hqman/droid-switch/releases) page.

Rust users can install from crates.io:

```sh
cargo install droid-switch
```

## Quick Start

If you are already logged in to Droid, save the current account first:

```sh
dsw import main
```

Add another account:

```sh
dsw add work
```

This opens Droid. Sign in with the other account in your browser, and Droid
Switch will save it automatically.

Switch accounts anytime:

```sh
dsw use main
dsw use work
```

Check your profiles:

```sh
dsw list
dsw status
dsw sync
dsw sync --all
```

## Commands

```text
dsw init [--import-as <name>]   Create storage and optionally import the current login
dsw import <name> [--force]     Save the current live login as a profile
dsw add <name> [--no-login]     Launch Droid, wait for login, then save it
dsw use <name>                  Activate a saved profile
dsw list [--json]               List saved profiles
dsw status [--json]             Show the active profile and live identity
dsw sync [--all]                Save live auth back to saved profile(s)
dsw remove <name> [-y]          Delete a profile
dsw rename <old> <new>          Rename a profile
dsw doctor [--json]             Check paths, permissions, Droid, and tokens
dsw backup list                 List automatic backups
dsw backup restore <id> [-y]    Restore a backup
dsw backup prune [--keep <n>]   Keep only the newest backups
```

Profile names must be 1 to 64 characters and may only contain lowercase ASCII
letters, digits, `-`, and `_`.

## Data Storage

Droid Switch uses these paths by default:

```text
~/.factory/                     Live Factory Droid config directory
~/.dsw/profiles/<name>/         Saved profile auth files
~/.dsw/backups/<id>/            Automatic switch backups
~/.dsw/state.json               Active profile state
```

It copies only these files:

- `auth.v2.file`
- `auth.v2.key`
- `auth.encrypted`

Everything else in `~/.factory` is left untouched.

## Safety

- Saved profiles and backups contain local auth material.
- Do not share `~/.factory` or `~/.dsw` files in issues or logs.
- On Unix-like systems, Droid Switch sets private permissions for its storage
  directories and copied auth files.
- Run `dsw doctor` if switching behaves unexpectedly.

## FAQ

### Does it change my Droid sessions or settings?

No. It only copies the auth files listed above.

### Can I switch back to the account I am using now?

Yes. First save it with `dsw import <name>`, then you can switch back to it
with `dsw use <name>`.

### What happens before a switch?

Droid Switch creates a backup of the current live auth files under
`~/.dsw/backups`.

### How do I save refreshed Droid tokens?

Droid can refresh the current live login in `~/.factory`. Run `dsw sync` after
using Droid to save that refreshed auth bundle back to the active profile.

Use `dsw sync --all` to save the current live auth bundle to every saved
profile with the same decoded email or subject. This is intentionally safe: it
does not switch accounts, start Droid, or refresh other accounts.

### Can I use a custom config directory?

Yes. Set `DSW_HOME` for Droid Switch storage or `FACTORY_DIR` for the live
Droid config directory.

## Build From Source

Install Rust 1.80 or newer, then run:

```sh
cargo build --release
```

Run checks:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

## Release

Create and push a version tag:

```sh
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions will publish prebuilt binaries for macOS, Linux, and Windows.

## Contributing

Issues and pull requests are welcome. Please run the checks above before
opening a pull request.

## License

MIT. See [LICENSE](LICENSE).
