//! T1914 acceptance — TOML-local key reader.
//!
//! Three acceptance criteria from `spec/v1/v2-llm-strategy/tasks.md`:
//!
//! - (a) missing `.local` → `LlmError::Auth` whose message names the
//!   config path.
//! - (b) `.local` present but anthropic key missing under
//!   `default_provider = "anthropic"` → `LlmError::Auth` whose
//!   message names the key.
//! - (c) `.local` present with placeholder `sk-ant-test-stub-...`
//!   parses ok (no key-strength validation).

use std::fs;

use llm::auth::load_keys_from_path;
use llm::{LlmConfig, LlmError};

fn make_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[test]
fn t1914_a_missing_local_names_the_path() {
    let td = make_dir();
    let agent_toml = td.path().join("agent.toml");
    fs::write(&agent_toml, "").unwrap();

    let cfg = LlmConfig::default(); // default_provider = "anthropic"
    let err = load_keys_from_path(&cfg, &agent_toml).expect_err("must error");
    let LlmError::Auth(msg) = err else {
        panic!("expected Auth");
    };
    assert!(
        msg.contains("agent.toml.local"),
        "error must mention the config path: {msg}"
    );
}

#[test]
fn t1914_b_missing_anthropic_key_names_the_provider() {
    let td = make_dir();
    let agent_toml = td.path().join("agent.toml");
    fs::write(&agent_toml, "").unwrap();
    let overlay = td.path().join("agent.toml.local");
    fs::write(
        &overlay,
        r#"
[llm.providers.openai]
api_key = "sk-openai-test"
"#,
    )
    .unwrap();

    let cfg = LlmConfig::default();
    let err = load_keys_from_path(&cfg, &agent_toml).expect_err("anthropic key missing");
    let LlmError::Auth(msg) = err else {
        panic!("expected Auth");
    };
    assert!(msg.contains("anthropic"), "must name provider: {msg}");
    assert!(msg.contains("api_key"), "must name field: {msg}");
}

#[test]
fn t1914_c_placeholder_key_parses_ok() {
    let td = make_dir();
    let agent_toml = td.path().join("agent.toml");
    fs::write(&agent_toml, "").unwrap();
    let overlay = td.path().join("agent.toml.local");
    fs::write(
        &overlay,
        r#"
[llm.providers.anthropic]
api_key = "sk-ant-test-stub-deadbeef"
"#,
    )
    .unwrap();

    let cfg = LlmConfig::default();
    let keys = load_keys_from_path(&cfg, &agent_toml).expect("placeholder parses");
    assert_eq!(keys.get("anthropic"), Some("sk-ant-test-stub-deadbeef"));
}
