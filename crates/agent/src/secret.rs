//! `SecretSource` implementations for the agent.
//!
//! **Trait declaration** lives in `crates/core::secret` (shared vocabulary;
//! no dependency cycle).  **Impls** live here (agent owns the secret boundary —
//! same crate as the LLM-key overlay `merge_llm_local_overlay`,
//! `config.rs:612-651`, and F2's future arming guard).
//!
//! Two impls:
//! - [`EnvSecretSource`] — reads from process environment (the default, CI-safe).
//! - [`LocalFileSecretSource`] — reads from the git-ignored
//!   `config/agent.toml.local` (the proven LLM-key precedent); never touches
//!   the committed config.
//!
//! **Binding law 1 (ADR-0054 § D3):** No secrets in git, ever.  The safe path
//! is the only path: both impls return `Err(SecretError::Missing)` when absent —
//! **never a default/empty key, never a silent unauthenticated request** (AC-3).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use trading_core::secret::{SecretError, SecretSource, SecretString};

// ── EnvSecretSource ───────────────────────────────────────────────────────────

/// Reads secrets from environment variables.
///
/// Looks up `key` in the process environment with `std::env::var`.
/// Returns [`SecretError::Missing`] when absent — **never** a default or
/// empty key.
///
/// This is the default source for CI and production (operator sets
/// `BINANCE_API_KEY` / `BINANCE_API_SECRET` in their shell or process
/// environment; the repo disk is never touched).
pub struct EnvSecretSource;

impl SecretSource for EnvSecretSource {
    fn get(&self, key: &str) -> Result<SecretString, SecretError> {
        match std::env::var(key) {
            Ok(val) if !val.is_empty() => Ok(SecretString::new(val)),
            Ok(_) => Err(SecretError::Missing(format!(
                "env var `{key}` is set but empty"
            ))),
            Err(_) => Err(SecretError::Missing(format!("env var `{key}` not set"))),
        }
    }
}

// ── LocalFileSecretSource ─────────────────────────────────────────────────────

/// Reads secrets from a git-ignored TOML file (the proven LLM-key precedent,
/// `config.rs:612-651`).
///
/// The file must be a TOML table of `key = "value"` pairs.
/// The committed config carries only placeholders; the `.local` file is in
/// `.gitignore` and never committed.
///
/// Returns [`SecretError::Missing`] when the key is absent in the file.
/// Returns [`SecretError::Io`] when the file cannot be read or parsed.
pub struct LocalFileSecretSource {
    path: PathBuf,
}

impl LocalFileSecretSource {
    /// Construct pointing at `path` (typically `config/agent.toml.local`).
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Default path used by the agent: `config/agent.toml.local`
    /// (relative to the process working directory, same as the config loader).
    pub fn default_path() -> PathBuf {
        PathBuf::from("config/agent.toml.local")
    }

    fn load(&self) -> Result<HashMap<String, String>, SecretError> {
        let contents = std::fs::read_to_string(&self.path)
            .map_err(|e| SecretError::Io(format!("cannot read {:?}: {}", self.path, e)))?;
        let table: toml::Value = contents
            .parse()
            .map_err(|e| SecretError::Io(format!("cannot parse {:?}: {}", self.path, e)))?;
        let mut map = HashMap::new();
        if let Some(t) = table.as_table() {
            for (k, v) in t {
                if let Some(s) = v.as_str() {
                    map.insert(k.clone(), s.to_string());
                }
            }
        }
        Ok(map)
    }
}

impl SecretSource for LocalFileSecretSource {
    fn get(&self, key: &str) -> Result<SecretString, SecretError> {
        if !Path::new(&self.path).exists() {
            return Err(SecretError::Missing(format!(
                "local secret file {:?} does not exist",
                self.path
            )));
        }
        let map = self.load()?;
        match map.get(key) {
            Some(v) if !v.is_empty() => Ok(SecretString::new(v.clone())),
            Some(_) => Err(SecretError::Missing(format!(
                "key `{key}` is present but empty in {:?}",
                self.path
            ))),
            None => Err(SecretError::Missing(format!(
                "key `{key}` not found in {:?}",
                self.path
            ))),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// AC-3 (adversarial): absent env var → `Err(SecretError::Missing)`.
    /// Never a default/empty key, never silent.
    #[test]
    fn missing_secret_fails_closed_env() {
        // Use a key that is definitively not in the test environment.
        let src = EnvSecretSource;
        let result = src.get("BINANCE_F1_TEST_KEY_DEFINITELY_ABSENT_AC3");
        match result {
            Err(SecretError::Missing(_)) => {} // expected
            Ok(_) => panic!("expected Missing, got Ok — key should not be in env"),
            Err(SecretError::Io(e)) => panic!("expected Missing, got Io: {e}"),
        }
        // has() mirrors get()
        assert!(!src.has("BINANCE_F1_TEST_KEY_DEFINITELY_ABSENT_AC3"));
    }

    /// AC-3: empty env var → `Missing` (not a silent empty key).
    #[test]
    fn empty_env_var_is_missing() {
        // We temporarily set the var; use a unique key to avoid test pollution.
        // SAFETY: test-only mutation of a unique key; single-threaded test context.
        unsafe {
            std::env::set_var("BINANCE_F1_TEST_EMPTY_KEY", "");
        }
        let src = EnvSecretSource;
        let result = src.get("BINANCE_F1_TEST_EMPTY_KEY");
        // SAFETY: test cleanup.
        unsafe {
            std::env::remove_var("BINANCE_F1_TEST_EMPTY_KEY");
        }
        assert!(
            matches!(result, Err(SecretError::Missing(_))),
            "empty env var should be Missing"
        );
    }

    /// AC-3: `LocalFileSecretSource` returns `Missing` when the file doesn't exist.
    #[test]
    fn missing_secret_fails_closed_local_file() {
        let src = LocalFileSecretSource::new("/tmp/definitely_nonexistent_f1_test_file.toml");
        let result = src.get("BINANCE_API_KEY");
        assert!(
            matches!(result, Err(SecretError::Missing(_))),
            "nonexistent file should be Missing"
        );
    }

    /// `LocalFileSecretSource` reads values from a real TOML file.
    #[test]
    fn local_file_reads_value() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test_secrets.toml");
        std::fs::write(
            &path,
            b"BINANCE_API_KEY = \"FAKE_TESTNET_KEY_DO_NOT_USE\"\n\
              BINANCE_API_SECRET = \"FAKE_TESTNET_SECRET_DO_NOT_USE\"\n",
        )
        .expect("write test file");
        let src = LocalFileSecretSource::new(&path);
        let key = src.get("BINANCE_API_KEY").expect("should be present");
        assert_eq!(key.expose_str(), "FAKE_TESTNET_KEY_DO_NOT_USE");
        let secret = src.get("BINANCE_API_SECRET").expect("should be present");
        assert_eq!(secret.expose_str(), "FAKE_TESTNET_SECRET_DO_NOT_USE");
        // Missing key
        let missing = src.get("NONEXISTENT_KEY");
        assert!(matches!(missing, Err(SecretError::Missing(_))));
    }
}
