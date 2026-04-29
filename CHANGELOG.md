# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses semantic versioning.

## [0.1.3] - 2026-04-29

### Added

- Added `dsw sync` to save refreshed live Droid auth back to the active profile.
- Added `dsw sync --all` to save the current live auth to matching saved profiles.

## [0.1.2] - 2026-04-28

### Added

- Added first-run guidance when running `dsw` with no command.
- Improved empty profile list guidance based on whether a Droid login exists.

## [0.1.1] - 2026-04-28

### Changed

- Simplified `dsw list` output.
- Hid internal rescue profiles from user-facing profile lists.

## [0.1.0] - 2026-04-28

### Added

- Initial open-source release.
- Profile import, add, use, list, status, remove, and rename commands.
- Automatic backups before account switches.
- Backup list, restore, and prune commands.
- Doctor command for installation and token checks.
- GitHub release workflow with macOS, Linux, and Windows binaries.
