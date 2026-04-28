//! Read identifying info (email, token expiry) from a Factory droid auth bundle.
//!
//! `~/.factory/auth.v2.file` is `AES-256-GCM(key=auth.v2.key)` of a JSON
//! payload `{ "access_token": "<JWT>", "refresh_token": "..." }`. The on-disk
//! format is three colon-separated base64 segments: `iv:tag:ciphertext`,
//! where `iv` is 16 bytes and `tag` is 16 bytes.
//!
//! `auth.encrypted` is a legacy plaintext JSON variant we use as a fallback.

use std::fs;
use std::path::Path;

use aes_gcm::aead::generic_array::typenum::{U12, U16};
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::Aead;
use aes_gcm::aes::Aes256;
use aes_gcm::{AesGcm, KeyInit};

// Factory's TypeScript code (node:crypto) uses a 16-byte IV. The de-facto
// standard for AES-GCM is 12, and our test helpers also produce 12-byte IVs,
// so we accept both.
type Aes256Gcm12 = AesGcm<Aes256, U12>;
type Aes256Gcm16 = AesGcm<Aes256, U16>;
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64URL;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Identity decoded from a Factory auth bundle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub email: Option<String>,
    pub subject: Option<String>,
    /// Token `exp` claim as UTC datetime, if present.
    pub expires_at: Option<DateTime<Utc>>,
}

impl Identity {
    pub fn unknown() -> Self {
        Self {
            email: None,
            subject: None,
            expires_at: None,
        }
    }

    pub fn display_email(&self) -> String {
        self.email
            .clone()
            .or_else(|| self.subject.clone())
            .unwrap_or_else(|| "-".to_string())
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        matches!(self.expires_at, Some(exp) if exp <= now)
    }
}

#[derive(Deserialize)]
struct AuthPayload {
    access_token: String,
    #[allow(dead_code)]
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct JwtClaims {
    email: Option<String>,
    sub: Option<String>,
    exp: Option<i64>,
}

/// Decrypt `auth.v2.file` using `auth.v2.key` and return the JSON payload.
pub fn decrypt_v2(auth_v2_file: &Path, auth_v2_key: &Path) -> Result<String> {
    let key_b64 = fs::read_to_string(auth_v2_key)
        .with_context(|| format!("read key file {}", auth_v2_key.display()))?;
    let key = B64
        .decode(key_b64.trim())
        .context("decode auth.v2.key (base64)")?;
    if key.len() != 32 {
        return Err(anyhow!(
            "auth.v2.key must decode to 32 bytes (got {})",
            key.len()
        ));
    }

    let raw = fs::read_to_string(auth_v2_file)
        .with_context(|| format!("read {}", auth_v2_file.display()))?;
    let parts: Vec<&str> = raw.trim().split(':').collect();
    if parts.len() != 3 {
        return Err(anyhow!(
            "auth.v2.file must have 3 colon-separated segments, got {}",
            parts.len()
        ));
    }
    let iv = B64.decode(parts[0]).context("decode iv")?;
    let tag = B64.decode(parts[1]).context("decode tag")?;
    let ct = B64.decode(parts[2]).context("decode ciphertext")?;

    // aes-gcm appends the tag to the ciphertext.
    let mut combined = ct;
    combined.extend_from_slice(&tag);

    let pt = match iv.len() {
        12 => {
            let cipher =
                Aes256Gcm12::new_from_slice(&key).map_err(|e| anyhow!("invalid key: {e}"))?;
            let nonce = GenericArray::<u8, U12>::from_slice(&iv);
            cipher.decrypt(nonce, combined.as_ref())
        }
        16 => {
            let cipher =
                Aes256Gcm16::new_from_slice(&key).map_err(|e| anyhow!("invalid key: {e}"))?;
            let nonce = GenericArray::<u8, U16>::from_slice(&iv);
            cipher.decrypt(nonce, combined.as_ref())
        }
        n => return Err(anyhow!("unsupported IV length {n} (expected 12 or 16)")),
    }
    .map_err(|_| anyhow!("AES-GCM decryption failed (wrong key or corrupt file)"))?;

    String::from_utf8(pt).context("decrypted payload is not UTF-8")
}

/// Decode email + expiry from a JWT access token. Does NOT verify the
/// signature - we only use it to identify the account.
pub fn identity_from_jwt(jwt: &str) -> Result<Identity> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return Err(anyhow!("not a JWT (expected 3 segments)"));
    }
    let payload = B64URL.decode(parts[1]).context("decode JWT payload")?;
    let claims: JwtClaims = serde_json::from_slice(&payload).context("parse JWT claims")?;
    Ok(Identity {
        email: claims.email,
        subject: claims.sub,
        expires_at: claims.exp.and_then(|s| DateTime::from_timestamp(s, 0)),
    })
}

/// Best-effort identity for a directory containing Factory auth files.
/// Tries `auth.v2.file` + `auth.v2.key` first, falls back to legacy
/// `auth.encrypted` (which is plaintext JSON despite the name).
/// Never panics; returns `Identity::unknown()` if everything fails.
pub fn identity_from_dir(dir: &Path) -> Identity {
    if let Ok(id) = try_v2(dir) {
        return id;
    }
    if let Ok(id) = try_legacy(dir) {
        return id;
    }
    Identity::unknown()
}

fn try_v2(dir: &Path) -> Result<Identity> {
    let f = dir.join("auth.v2.file");
    let k = dir.join("auth.v2.key");
    if !f.is_file() || !k.is_file() {
        return Err(anyhow!("v2 files missing"));
    }
    let payload_json = decrypt_v2(&f, &k)?;
    let payload: AuthPayload = serde_json::from_str(&payload_json).context("parse auth payload")?;
    identity_from_jwt(&payload.access_token)
}

fn try_legacy(dir: &Path) -> Result<Identity> {
    let p = dir.join("auth.encrypted");
    if !p.is_file() {
        return Err(anyhow!("legacy file missing"));
    }
    let raw = fs::read_to_string(&p)?;
    let payload: AuthPayload = serde_json::from_str(&raw)?;
    identity_from_jwt(&payload.access_token)
}

/// Test helpers for synthesizing valid Factory auth bundles.
///
/// Public so integration tests can use it. Not part of the stable API.
#[doc(hidden)]
pub mod testing {
    use super::*;
    use aes_gcm::aead::rand_core::RngCore;
    use aes_gcm::aead::OsRng;
    use aes_gcm::AeadCore;
    use aes_gcm::Aes256Gcm; // 12-byte nonce default; matches our typical case

    /// Write a synthetic v2 auth bundle to `dir` whose embedded JWT carries
    /// the given email and `exp` (unix seconds).
    pub fn write_synthetic_bundle(dir: &Path, email: &str, exp: i64) {
        let header = B64URL.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let claims = serde_json::json!({
            "email": email,
            "sub": "user_test",
            "exp": exp,
        });
        let payload = B64URL.encode(serde_json::to_vec(&claims).unwrap());
        let sig = B64URL.encode(b"sig");
        let jwt = format!("{header}.{payload}.{sig}");

        let auth_json = serde_json::json!({
            "access_token": jwt,
            "refresh_token": "rt_test",
        })
        .to_string();

        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).unwrap();
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ct_with_tag = cipher.encrypt(&nonce, auth_json.as_bytes()).unwrap();
        let (ct, tag) = ct_with_tag.split_at(ct_with_tag.len() - 16);

        let file_contents = format!(
            "{}:{}:{}",
            B64.encode(nonce),
            B64.encode(tag),
            B64.encode(ct),
        );
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("auth.v2.file"), file_contents).unwrap();
        fs::write(dir.join("auth.v2.key"), B64.encode(key_bytes)).unwrap();
    }

    /// Like `write_synthetic_bundle` but uses a 16-byte IV (matches Factory's
    /// node:crypto-produced format).
    pub fn write_synthetic_bundle_16iv(dir: &Path, email: &str, exp: i64) {
        let header = B64URL.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let claims = serde_json::json!({"email": email, "sub": "u", "exp": exp});
        let payload = B64URL.encode(serde_json::to_vec(&claims).unwrap());
        let jwt = format!("{header}.{payload}.{}", B64URL.encode(b"sig"));
        let auth_json = serde_json::json!({"access_token": jwt, "refresh_token": "r"}).to_string();

        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);
        let mut iv = [0u8; 16];
        OsRng.fill_bytes(&mut iv);

        let cipher = Aes256Gcm16::new_from_slice(&key_bytes).unwrap();
        let nonce = GenericArray::<u8, U16>::from_slice(&iv);
        let ct_with_tag = cipher.encrypt(nonce, auth_json.as_bytes()).unwrap();
        let (ct, tag) = ct_with_tag.split_at(ct_with_tag.len() - 16);

        let file_contents = format!("{}:{}:{}", B64.encode(iv), B64.encode(tag), B64.encode(ct),);
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("auth.v2.file"), file_contents).unwrap();
        fs::write(dir.join("auth.v2.key"), B64.encode(key_bytes)).unwrap();
    }

    /// Write a legacy plaintext `auth.encrypted` file (no v2 bundle).
    pub fn write_legacy_bundle(dir: &Path, email: &str, exp: i64) {
        let header = B64URL.encode(br#"{"alg":"none"}"#);
        let claims = serde_json::json!({"email": email, "sub": "u1", "exp": exp}).to_string();
        let payload = B64URL.encode(claims.as_bytes());
        let sig = B64URL.encode(b"x");
        let jwt = format!("{header}.{payload}.{sig}");
        let body = serde_json::json!({"access_token": jwt, "refresh_token": "r"}).to_string();
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("auth.encrypted"), body).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn roundtrip_v2_bundle() {
        let td = TempDir::new().unwrap();
        write_synthetic_bundle(td.path(), "alice@example.com", 1_900_000_000);
        let id = identity_from_dir(td.path());
        assert_eq!(id.email.as_deref(), Some("alice@example.com"));
        assert!(id.expires_at.is_some());
    }

    #[test]
    fn legacy_fallback() {
        let td = TempDir::new().unwrap();
        write_legacy_bundle(td.path(), "bob@example.com", 1_900_000_000);
        let id = identity_from_dir(td.path());
        assert_eq!(id.email.as_deref(), Some("bob@example.com"));
    }

    #[test]
    fn unknown_when_empty() {
        let td = TempDir::new().unwrap();
        let id = identity_from_dir(td.path());
        assert_eq!(id, Identity::unknown());
    }

    #[test]
    fn roundtrip_v2_bundle_with_16_byte_iv() {
        let td = TempDir::new().unwrap();
        write_synthetic_bundle_16iv(td.path(), "carol@example.com", 1_900_000_000);
        let id = identity_from_dir(td.path());
        assert_eq!(id.email.as_deref(), Some("carol@example.com"));
    }
}
