//! Canonicalisation helpers for the replay-cache key.
//!
//! `request_hash` is the SHA-256 over a deterministic `CanonicalContext` JSON
//! serialisation (see `types.rs :: ForecastContext::request_hash()`). This
//! module exposes helper functions consumed by `ForecastContext` and by the
//! Wave D re-record binary.
//!
//! ## Why a separate module?
//!
//! The hash algorithm is security-adjacent (cache-break = full replay re-record;
//! non-trivial operational cost per K5). Isolating it makes future audits,
//! version-bump reviews, and schema-change diffs easier.
//!
//! ## Cross-references
//!
//! - `spec/v1/v3-llm-forecaster/decomp.md § T-AR-2` — architect-locked hash algorithm.
//! - `spec/v1/v3-llm-forecaster/decomp.md § T-AR-5` — determinism contract.
//! - `crates/strategy/src/llm_forecaster/types.rs` — `ForecastContext::request_hash()`.

use sha2::{Digest, Sha256};

use super::types::{CACHE_SCHEMA_VERSION, PROMPT_TEMPLATE_VERSION};

/// Hex-encode a 32-byte SHA-256 digest as lowercase 64-character string.
///
/// Used in error messages and in the backtest report reasoning-trace SHA
/// histogram (ADR-0039 § T-AR-6 body shape: `format!("{:x}", sha)`).
#[must_use]
pub fn hex_encode(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 a byte slice and return the 32-byte digest.
#[must_use]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Verify that `CACHE_SCHEMA_VERSION` and `PROMPT_TEMPLATE_VERSION` are
/// coherent (both non-zero, schema >= 1). Called as a debug_assert in tests.
#[must_use]
pub fn versions_coherent() -> bool {
    CACHE_SCHEMA_VERSION >= 1 && PROMPT_TEMPLATE_VERSION >= 1
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// hex_encode produces lowercase 64-char strings.
    #[test]
    fn hex_encode_produces_64_char_lowercase_string() {
        let bytes = [0xABu8; 32];
        let hex = hex_encode(&bytes);
        assert_eq!(hex.len(), 64, "hex must be 64 chars");
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()), "must be hex");
        // AB → "ab" lowercase
        assert!(
            hex.chars().all(|c| !c.is_ascii_uppercase()),
            "must be lowercase"
        );
        assert_eq!(&hex[0..2], "ab");
    }

    /// sha256 is deterministic.
    #[test]
    fn sha256_is_deterministic() {
        let data = b"test input";
        let h1 = sha256(data);
        let h2 = sha256(data);
        assert_eq!(h1, h2);
    }

    /// sha256 of empty input is the well-known SHA-256 of "".
    #[test]
    fn sha256_of_empty_is_well_known() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let h = sha256(b"");
        let hex = hex_encode(&h);
        assert_eq!(
            hex,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// Version constants are coherent.
    #[test]
    fn version_constants_coherent() {
        assert!(
            versions_coherent(),
            "CACHE_SCHEMA_VERSION and PROMPT_TEMPLATE_VERSION must be >= 1"
        );
    }
}
