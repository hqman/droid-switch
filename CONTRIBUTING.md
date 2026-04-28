# Contributing

Thanks for taking the time to improve Droid Switch.

## Development Setup

Install Rust 1.80 or newer, then clone the repository and run:

```sh
cargo test --locked
```

## Checks

Before opening a pull request, run:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

## Pull Requests

- Keep changes focused.
- Add or update tests for behavior changes.
- Update `README.md` when user-facing commands or workflows change.
- Do not commit local auth files, build artifacts, or secrets.

## Reporting Issues

Please include:

- Operating system and shell.
- Droid Switch version.
- The command you ran.
- The full error output.
- Whether `dsw doctor` reports any failed checks.
