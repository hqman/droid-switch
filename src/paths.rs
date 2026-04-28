//! Filesystem layout for dsw and the Factory `droid` CLI.
//!
//! All paths are resolved through `Paths` so tests can redirect them to
//! temporary directories via the `DSW_HOME` and `FACTORY_DIR` env vars.

use std::env;
use std::path::{Path, PathBuf};

/// The three credential files dsw treats as the "auth bundle". Anything
/// outside this list (sessions, history, settings) is shared across profiles
/// and never touched.
pub const AUTH_FILES: &[&str] = &["auth.v2.file", "auth.v2.key", "auth.encrypted"];

#[derive(Debug, Clone)]
pub struct Paths {
    /// `~/.dsw` (or `$DSW_HOME`)
    pub home: PathBuf,
    /// `~/.factory`  (or `$FACTORY_DIR`) - the live Factory droid config dir
    pub factory: PathBuf,
}

impl Paths {
    pub fn from_env() -> Self {
        let home = env::var_os("DSW_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .expect("could not determine home directory")
                    .join(".dsw")
            });
        let factory = env::var_os("FACTORY_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .expect("could not determine home directory")
                    .join(".factory")
            });
        Self { home, factory }
    }

    /// Construct paths explicitly (used by tests).
    pub fn new(home: impl Into<PathBuf>, factory: impl Into<PathBuf>) -> Self {
        Self {
            home: home.into(),
            factory: factory.into(),
        }
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.home.join("profiles")
    }

    pub fn profile_dir(&self, name: &str) -> PathBuf {
        self.profiles_dir().join(name)
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.home.join("backups")
    }

    pub fn state_file(&self) -> PathBuf {
        self.home.join("state.json")
    }

    /// Live Factory auth file paths inside `~/.factory/`.
    pub fn live_auth_files(&self) -> Vec<PathBuf> {
        AUTH_FILES.iter().map(|f| self.factory.join(f)).collect()
    }

    /// Saved auth file paths inside a profile directory.
    pub fn profile_auth_files(&self, name: &str) -> Vec<PathBuf> {
        let dir = self.profile_dir(name);
        AUTH_FILES.iter().map(|f| dir.join(f)).collect()
    }
}

/// Validate a profile name. Lowercase ascii letters/digits + `-` `_`. Length
/// 1..=64. No leading/trailing dashes. This keeps filenames portable and
/// disallows shell-meaningful characters.
pub fn validate_profile_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() || name.len() > 64 {
        anyhow::bail!("profile name must be 1..=64 chars: {name:?}");
    }
    if name.starts_with('-') || name.ends_with('-') {
        anyhow::bail!("profile name cannot start or end with '-': {name:?}");
    }
    for ch in name.chars() {
        let ok = ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_';
        if !ok {
            anyhow::bail!("profile name may only contain a-z 0-9 - _ (got {ch:?} in {name:?})");
        }
    }
    Ok(())
}

/// Convenience: check whether a path looks like a populated profile dir.
pub fn looks_like_profile(dir: &Path) -> bool {
    dir.is_dir() && dir.join("auth.v2.file").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_names() {
        for ok in ["main", "work", "client-1", "a_b", "x"] {
            validate_profile_name(ok).unwrap();
        }
        for bad in ["", "-bad", "bad-", "Bad", "a b", "a/b", "a.b", "a!"] {
            assert!(
                validate_profile_name(bad).is_err(),
                "expected {bad:?} to fail"
            );
        }
    }

    #[test]
    fn paths_use_env() {
        std::env::set_var("DSW_HOME", "/tmp/dsw-test");
        std::env::set_var("FACTORY_DIR", "/tmp/factory-test");
        let p = Paths::from_env();
        assert_eq!(p.home, PathBuf::from("/tmp/dsw-test"));
        assert_eq!(p.factory, PathBuf::from("/tmp/factory-test"));
        assert_eq!(
            p.profile_dir("foo"),
            PathBuf::from("/tmp/dsw-test/profiles/foo")
        );
        std::env::remove_var("DSW_HOME");
        std::env::remove_var("FACTORY_DIR");
    }
}
