//! TOML-local API-key reader (T1914).
//!
//! Design § Q3 = C resolution: keys live in `config/agent.toml.local`
//! (git-ignored), overlaid on the committed `config/agent.toml`. The
//! committed file carries the provider section shape (`base_url`,
//! …) but never `api_key = "..."`; the operator drops in
//! `api_key = "sk-..."` lines under each `[llm.providers.<name>]`
//! section in the `.local` overlay.
//!
//! Public surface (T1914 acceptance):
//!
//! - [`load_keys`] — reads the `.local` overlay and returns a [`KeyMap`].
//! - [`KeyMap`] — a `HashMap<String /* provider */, String /* api_key */>`
//!   with a `Drop` impl that zeroes the buffers (best-effort: a process
//!   forensic dump can still recover keys from heap snapshots taken
//!   before drop, but the on-disk forensic surface is reduced).
//!
//! Error conditions surface as [`LlmError::Auth`] with operator-actionable
//! messages naming the file path and the missing key.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config::LlmConfig;
use crate::error::LlmError;
use crate::redact::redact;

/// In-memory provider → api_key map. Zeroes buffers on drop.
#[derive(Default)]
pub struct KeyMap {
    inner: HashMap<String, String>,
}

impl KeyMap {
    /// Look up the API key for `provider_name` (`"anthropic"`,
    /// `"openai"`, …). Returns `None` if the key wasn't loaded.
    #[must_use]
    pub fn get(&self, provider_name: &str) -> Option<&str> {
        self.inner.get(provider_name).map(String::as_str)
    }

    /// Insert a key. Used by `load_keys` during parse.
    pub fn insert(&mut self, provider_name: impl Into<String>, key: impl Into<String>) {
        self.inner.insert(provider_name.into(), key.into());
    }

    /// Iterate provider names present in the map (for forensic logs).
    pub fn provider_names(&self) -> impl Iterator<Item = &str> {
        self.inner.keys().map(String::as_str)
    }

    /// Return a redacted view of the loaded keys for debug logs.
    #[must_use]
    pub fn debug_view(&self) -> Vec<(String, String)> {
        self.inner
            .iter()
            .map(|(k, v)| (k.clone(), redact(v)))
            .collect()
    }
}

impl Drop for KeyMap {
    fn drop(&mut self) {
        // Best-effort: overwrite each key with zeros before drop. The
        // String's heap allocation is still freed by the standard
        // allocator (which may or may not zero on free); this clears
        // any active references before that point.
        for (_, key) in self.inner.iter_mut() {
            // Mutate in place: replace every byte with 0.
            // SAFETY: we own this String; rewriting bytes is sound.
            // SAFETY-note: keys are ASCII; in-place byte write
            // preserves UTF-8 validity.
            unsafe {
                let bytes = key.as_bytes_mut();
                for b in bytes {
                    *b = 0;
                }
            }
        }
        self.inner.clear();
    }
}

impl std::fmt::Debug for KeyMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyMap")
            .field("providers", &self.inner.keys().collect::<Vec<_>>())
            .field("keys", &"<redacted>")
            .finish()
    }
}

/// Layered TOML shape for `agent.toml.local`.
#[derive(Debug, Deserialize)]
struct LocalRoot {
    #[serde(default)]
    llm: Option<LocalLlmSection>,
}

#[derive(Debug, Deserialize)]
struct LocalLlmSection {
    #[serde(default)]
    providers: HashMap<String, LocalProviderEntry>,
}

#[derive(Debug, Deserialize)]
struct LocalProviderEntry {
    #[serde(default)]
    api_key: Option<String>,
}

/// Load API keys from `config/agent.toml.local`, looked up alongside
/// the committed `config/agent.toml`. Returns [`LlmError::Auth`] when:
///
/// - the `.local` file is missing AND `cfg.default_provider` is not
///   `"ollama"` (Ollama needs no auth);
/// - the file exists but the `default_provider`'s `api_key` is missing.
///
/// **Path discovery.** Uses the parent dir of `config_path` (a typical
/// invocation passes `Path::new("config/agent.toml")`). For tests, a
/// custom path can be supplied via [`load_keys_from_path`].
///
/// # Errors
///
/// Returns [`LlmError::Auth`] per the cases above.
pub fn load_keys(cfg: &LlmConfig) -> Result<KeyMap, LlmError> {
    load_keys_from_path(cfg, Path::new("config/agent.toml"))
}

/// Test-friendly variant: load keys relative to `agent_toml_path`'s
/// parent. The `.local` file is `agent_toml_path` + `.local` suffix
/// (matches the convention: `agent.toml` → `agent.toml.local`).
///
/// # Errors
///
/// See [`load_keys`].
pub fn load_keys_from_path(cfg: &LlmConfig, agent_toml_path: &Path) -> Result<KeyMap, LlmError> {
    let local_path = local_overlay_path(agent_toml_path);

    if !local_path.exists() {
        // Ollama needs no auth — let the factory proceed.
        if cfg.default_provider == "ollama" {
            return Ok(KeyMap::default());
        }
        return Err(LlmError::Auth(format!(
            "{} not found; copy {}.example and edit in real keys",
            local_path.display(),
            local_path.display()
        )));
    }

    let toml_str = std::fs::read_to_string(&local_path)
        .map_err(|e| LlmError::Auth(format!("failed to read {}: {}", local_path.display(), e)))?;
    let parsed: LocalRoot = toml::from_str(&toml_str)
        .map_err(|e| LlmError::Auth(format!("failed to parse {}: {}", local_path.display(), e)))?;

    let mut keys = KeyMap::default();
    if let Some(section) = parsed.llm {
        for (name, entry) in section.providers {
            if let Some(api_key) = entry.api_key {
                keys.insert(name, api_key);
            }
        }
    }

    // Final check: the configured default provider must have a key
    // (unless it's Ollama).
    if cfg.default_provider != "ollama" && keys.get(&cfg.default_provider).is_none() {
        return Err(LlmError::Auth(format!(
            "{}.api_key not set in {}",
            cfg.default_provider,
            local_path.display()
        )));
    }

    Ok(keys)
}

fn local_overlay_path(agent_toml_path: &Path) -> PathBuf {
    let mut p = agent_toml_path.as_os_str().to_owned();
    p.push(".local");
    PathBuf::from(p)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    /// Write a temporary `agent.toml.local` and return the path to the
    /// faux `agent.toml` we'd pass into `load_keys_from_path`.
    fn write_overlay(dir: &Path, content: &str) -> PathBuf {
        let agent_toml = dir.join("agent.toml");
        // We don't need the committed agent.toml to exist — only the
        // overlay matters. But create an empty stub so path-resolution
        // tests in other contexts behave the same.
        fs::write(&agent_toml, "").unwrap();
        let overlay = dir.join("agent.toml.local");
        let mut f = fs::File::create(&overlay).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        agent_toml
    }

    fn tempdir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    /// T1914 (a): missing `.local` under Anthropic provider →
    /// `LlmError::Auth` whose message names the path.
    #[test]
    fn t1914_a_missing_local_under_anthropic_errors() {
        let td = tempdir();
        let agent_toml = td.path().join("agent.toml");
        fs::write(&agent_toml, "").unwrap();
        // NOTE: no `.local` written.

        let cfg = LlmConfig::default(); // default_provider = "anthropic"
        let err = load_keys_from_path(&cfg, &agent_toml)
            .expect_err("missing .local should error under anthropic");
        match err {
            LlmError::Auth(msg) => {
                assert!(
                    msg.contains("agent.toml.local"),
                    "error must name the local path: {msg}"
                );
            }
            other => panic!("expected Auth, got {other:?}"),
        }
    }

    /// Missing `.local` under Ollama provider → `Ok(empty KeyMap)`
    /// (Ollama needs no auth).
    #[test]
    fn t1914_missing_local_under_ollama_returns_ok() {
        let td = tempdir();
        let agent_toml = td.path().join("agent.toml");
        fs::write(&agent_toml, "").unwrap();

        let cfg = LlmConfig {
            default_provider: "ollama".to_string(),
            ..Default::default()
        };
        let keys = load_keys_from_path(&cfg, &agent_toml).expect("ollama works without overlay");
        assert!(keys.inner.is_empty());
    }

    /// T1914 (b): `.local` present but anthropic key missing →
    /// `LlmError::Auth` whose message names the key.
    #[test]
    fn t1914_b_local_present_but_anthropic_key_missing() {
        let td = tempdir();
        let agent_toml = write_overlay(
            td.path(),
            r#"
[llm.providers.openai]
api_key = "sk-openai-test"
"#,
        );

        let cfg = LlmConfig::default(); // default_provider = "anthropic"
        let err = load_keys_from_path(&cfg, &agent_toml)
            .expect_err("missing anthropic key under anthropic provider should error");
        match err {
            LlmError::Auth(msg) => {
                assert!(msg.contains("anthropic"), "must name provider: {msg}");
                assert!(
                    msg.contains("api_key"),
                    "must mention the missing field: {msg}"
                );
            }
            other => panic!("expected Auth, got {other:?}"),
        }
    }

    /// T1914 (c): `.local` with placeholder `sk-ant-test-stub-...`
    /// parses ok (no key-strength validation).
    #[test]
    fn t1914_c_placeholder_key_parses_ok() {
        let td = tempdir();
        let agent_toml = write_overlay(
            td.path(),
            r#"
[llm.providers.anthropic]
api_key = "sk-ant-test-stub-12345"
"#,
        );

        let cfg = LlmConfig::default();
        let keys = load_keys_from_path(&cfg, &agent_toml).expect("placeholder key parses");
        assert_eq!(keys.get("anthropic"), Some("sk-ant-test-stub-12345"));
    }

    /// Debug-view of a loaded key map redacts the secret.
    #[test]
    fn t1914_debug_view_redacts_keys() {
        let mut keys = KeyMap::default();
        keys.insert("anthropic", "sk-ant-secret-deadbeef");
        let view = keys.debug_view();
        assert_eq!(view.len(), 1);
        assert_eq!(view[0].0, "anthropic");
        assert!(!view[0].1.contains("secret-deadbeef"));
        assert!(view[0].1.starts_with("sk-ant-"));
    }
}
