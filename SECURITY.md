# Security Policy

## Reporting a Vulnerability

Please do not open a public issue for vulnerabilities involving credential
handling, auth file exposure, or unsafe filesystem behavior.

Report security issues privately to the project maintainer. Include:

- A clear description of the issue.
- Steps to reproduce it.
- The affected operating system.
- Any relevant command output with secrets removed.

## Credential Handling

Droid Switch copies local Droid authentication files between profile
directories. Treat `~/.factory` and `~/.dsw` as sensitive data.

Never share these files in issues, logs, screenshots, or pull requests:

- `auth.v2.file`
- `auth.v2.key`
- `auth.encrypted`
