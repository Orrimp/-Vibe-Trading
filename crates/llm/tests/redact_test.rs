//! T1915 — `redact()` helper acceptance.
//!
//! Acceptance criteria a + b from `spec/v2-llm-strategy/tasks.md`. Test
//! (c) — tracing-subscriber field rewrite — is deferred to pass 4
//! follow-up (see `crates/llm/src/redact.rs` module docs).

use llm::redact::redact;

#[test]
fn t1915_a_anthropic_secret_not_present_in_output() {
    let red = redact("sk-ant-secret-12345");
    assert!(
        !red.contains("secret-12345"),
        "redacted output must not contain full secret substring: {red}"
    );
}

#[test]
fn t1915_b_short_key_redacted() {
    let red = redact("sk-shortie");
    assert!(
        !red.contains("shortie"),
        "redacted output leaks short-key content: {red}"
    );
}
