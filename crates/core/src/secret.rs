//! `SecretSource` trait + `SecretString` newtype — the shared secret vocabulary.
//!
//! **Placement rationale (AQ-5, feature.md § A1):** this module lives in
//! `crates/core` (zero deps, already the shared vocabulary crate for
//! `Money`/`Order`) so both `crates/exec` and `crates/agent` can consume
//! `&dyn SecretSource` without a dependency cycle.  The *impls*
//! (`EnvSecretSource`, `LocalFileSecretSource`) live in `crates/agent::secret`.
//!
//! **Binding law 1 (ADR-0054 § D3, verbatim):** No secrets in git, ever.
//! The safe path is the only path — keys are read from env or a git-ignored
//! local file and are NEVER logged, NEVER written to the audit ledger,
//! NEVER serialized, NEVER committed.

use serde::{Serialize, Serializer};
use thiserror::Error;

// ── SecretError ───────────────────────────────────────────────────────────────

/// Error returned by [`SecretSource::get`].
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SecretError {
    /// The requested key was not found in this source.
    #[error("secret not found: {0}")]
    Missing(String),
    /// An I/O or parse error while reading the source.
    #[error("secret source I/O error: {0}")]
    Io(String),
}

// ── SecretString ──────────────────────────────────────────────────────────────

/// A secret value whose plaintext is NEVER logged, NEVER serialized,
/// and NEVER copied into any struct's `Debug`.
///
/// The plaintext is reachable ONLY via [`SecretString::expose_secret`], which
/// is consumed exclusively by `sign.rs` (borrowed, never stored).
///
/// `Debug` and `Display` both emit `"<redacted>"`.
/// `Serialize` always fails — there is no code path that serializes the value.
pub struct SecretString(String);

impl SecretString {
    /// Wrap a secret value.  Only `EnvSecretSource` / `LocalFileSecretSource`
    /// (and the test fake `FakeSecretSource`) call this.
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Expose the raw bytes for HMAC signing **only**.
    ///
    /// # Caller contract
    /// The caller MUST:
    /// 1. Use the bytes only for HMAC input (pass to `sign::sign`).
    /// 2. Never copy the bytes into a struct, log them, or return them.
    /// 3. Drop the reference before any await point.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Expose the raw string for API-key header construction **only**.
    ///
    /// Same caller contract as [`expose_secret`]: use once, never store,
    /// never log.
    #[must_use]
    pub fn expose_str(&self) -> &str {
        &self.0
    }
}

/// Always emits `"<redacted>"` — the value is never in any log or error chain.
impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<redacted>")
    }
}

/// Always emits `"<redacted>"`.
impl std::fmt::Display for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<redacted>")
    }
}

/// Serialization is REFUSED — there is no code path that serializes a secret.
///
/// If any serde-based path ever reaches this, it gets an explicit error rather
/// than silently emitting plaintext.
impl Serialize for SecretString {
    fn serialize<S: Serializer>(&self, _s: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(
            "SecretString must never be serialized (ADR-0054 invariant i)",
        ))
    }
}

// ── SecretSource trait ────────────────────────────────────────────────────────

/// Source of secret values (API keys, etc.).
///
/// The safe path is the **only** path: every consumer takes `&dyn SecretSource`
/// — there is no API to pass a literal key in code or committed config.
///
/// **Returns `Err(SecretError::Missing)` when absent — NEVER a default/empty
/// key, NEVER a silent unauthenticated request.**  This is AC-3.
pub trait SecretSource: Send + Sync {
    /// Get a secret by name.  Returns [`SecretError::Missing`] when absent.
    ///
    /// # Errors
    /// - [`SecretError::Missing`] when the key is not present in this source.
    /// - [`SecretError::Io`] on an I/O or parse failure.
    fn get(&self, key: &str) -> Result<SecretString, SecretError>;

    /// Presence-only probe for the F2 arming guard condition (4).
    ///
    /// Never reads or exposes the value — returns `true` iff [`get`] would
    /// return `Ok`.
    fn has(&self, key: &str) -> bool {
        self.get(key).is_ok()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// AC-2 (adversarial): `Debug` and `Display` emit `"<redacted>"`;
    /// `serde` serialization is refused.  Fixture uses an obviously-fake
    /// placeholder — no real key material here.
    #[test]
    fn secret_never_logged_or_serialized() {
        let s = SecretString::new("FAKE_TESTNET_KEY_DO_NOT_USE".to_string());

        // Debug must not reveal the value
        let dbg = format!("{s:?}");
        assert_eq!(dbg, "<redacted>", "Debug leaked plaintext: {dbg}");
        assert!(!dbg.contains("FAKE_TESTNET_KEY_DO_NOT_USE"));

        // Display must not reveal the value
        let disp = format!("{s}");
        assert_eq!(disp, "<redacted>", "Display leaked plaintext: {disp}");
        assert!(!disp.contains("FAKE_TESTNET_KEY_DO_NOT_USE"));

        // Serialize must fail
        let result = serde_json::to_string(&s);
        assert!(result.is_err(), "Serialize should have been refused");
        if let Ok(json) = result {
            assert!(
                !json.contains("FAKE_TESTNET_KEY_DO_NOT_USE"),
                "serde leaked secret: {json}"
            );
        }

        // expose_secret gives back the bytes
        assert_eq!(s.expose_secret(), b"FAKE_TESTNET_KEY_DO_NOT_USE");
        assert_eq!(s.expose_str(), "FAKE_TESTNET_KEY_DO_NOT_USE");
    }

    /// `has` is true when `get` returns `Ok`.
    #[test]
    fn has_proxies_get() {
        struct AlwaysPresent;
        impl SecretSource for AlwaysPresent {
            fn get(&self, _key: &str) -> Result<SecretString, SecretError> {
                Ok(SecretString::new("VALUE".to_string()))
            }
        }
        struct AlwaysMissing;
        impl SecretSource for AlwaysMissing {
            fn get(&self, key: &str) -> Result<SecretString, SecretError> {
                Err(SecretError::Missing(key.to_string()))
            }
        }
        assert!(AlwaysPresent.has("anything"));
        assert!(!AlwaysMissing.has("anything"));
    }
}
