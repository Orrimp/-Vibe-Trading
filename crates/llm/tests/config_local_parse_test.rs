//! T1930 — `config/agent.toml.local.example` template parse test.
//!
//! Verifies that the committed example overlay file parses as a valid
//! `LocalOverrideConfig` shape (the minimal local-shape used by the
//! `auth::load_keys` reader) and that the four provider keys are
//! placeholder strings (not real-API prefixes).
//!
//! The test reads the file relative to the workspace root via
//! `CARGO_MANIFEST_DIR` walking up two levels (`crates/llm` →
//! `crates/` → workspace root).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

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

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map_or_else(|| crate_dir.clone(), Path::to_path_buf)
}

/// T1930 (a) — `config/agent.toml.local.example` parses as a valid
/// local-overlay shape.
#[test]
fn t1930_a_example_template_parses() {
    let path = workspace_root()
        .join("config")
        .join("agent.toml.local.example");
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let parsed: LocalRoot =
        toml::from_str(&content).expect("agent.toml.local.example parses as LocalRoot");
    let providers = parsed.llm.expect("[llm] section present").providers;

    // Anthropic + OpenAI + OpenRouter + DeepSeek = 4 keys.
    assert!(
        providers.contains_key("anthropic"),
        "must include anthropic provider"
    );
    assert!(
        providers.contains_key("openai"),
        "must include openai provider"
    );
    assert!(
        providers.contains_key("openrouter"),
        "must include openrouter provider"
    );
    assert!(
        providers.contains_key("deepseek"),
        "must include deepseek provider"
    );

    // Every key must be a placeholder (contains "stub" or all-zero
    // sequence). This guards against an operator accidentally
    // committing a real key in the example file — a real key would
    // not contain the literal "stub".
    for (name, entry) in &providers {
        let key = entry
            .api_key
            .as_deref()
            .unwrap_or_else(|| panic!("{name}.api_key must be present in the template"));
        assert!(
            key.contains("stub") || key.contains("0000000000"),
            "{name}.api_key must look like a placeholder (contains 'stub' or zero-run): {key}"
        );
    }
}

/// T1930 (b) — the same example file is consumed by the agent's
/// `Config::load` overlay path: simulate by feeding the example
/// directly into the auth-crate's local-overlay parser shape and
/// asserting it yields four keys.
#[test]
fn t1930_b_example_template_yields_four_keys() {
    let path = workspace_root()
        .join("config")
        .join("agent.toml.local.example");
    let content = std::fs::read_to_string(&path).expect("read example");
    let parsed: LocalRoot = toml::from_str(&content).expect("parse");
    let providers = parsed.llm.expect("[llm] section").providers;
    let key_count = providers.values().filter(|e| e.api_key.is_some()).count();
    assert_eq!(
        key_count, 4,
        "example template ships placeholders for exactly four providers"
    );
}
