//! Integration tests for `crates/llm/src/redact_layer.rs`.
//!
//! Tests the `RedactLayer` + thread-local pipeline. The `RedactLayer` stores
//! redacted field overrides in `REDACTED_FIELDS` (thread-local) for the
//! downstream `RedactingFormatFields` formatter, and records meta-events in
//! `META_EVENTS` (thread-local) for integration test assertions.
//!
//! In production, meta-events are ALSO written to stderr via `eprintln!`
//! (the only side-channel that works inside tracing's `on_event` — `tracing::warn!`
//! is dropped by tracing-core's reentrancy guard when called from `on_event`).
//!
//! # Test cases (T-RED-D14..D16)
//!
//! - WARN-mode meta-event recorded (R4.2)
//! - gate-mode: no meta-event (R4.3)
//! - gate + verbose: meta-event recorded
//! - non-secret field: no meta-event, REDACTED_FIELDS empty
//! - marker-field bypass with reason (D-RED-4)
//! - marker-field missing reason (D-RED-4)
//! - password field name rule
//! - thread-local peek/take
//! - only-secret-fields in REDACTED_FIELDS
//! - P-RED-1 falsification probe (#[ignore])
//! - P-RED-2 falsification probe (#[ignore])

use tracing::Subscriber;
use tracing_subscriber::layer::SubscriberExt;

// ── Helper: build a subscriber for each test ──────────────────────────────────
//
// Each test uses `tracing::subscriber::with_default` to scope the subscriber.
// This avoids global init collisions between parallel tests.

fn build_warn_subscriber() -> impl Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>
{
    tracing_subscriber::registry().with(llm::RedactLayer::warn_mode())
}

fn build_gate_subscriber() -> impl Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>
{
    tracing_subscriber::registry().with(llm::RedactLayer::gate_mode())
}

fn build_gate_verbose_subscriber()
-> impl Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a> {
    tracing_subscriber::registry().with(llm::RedactLayer::gate_verbose_mode())
}

/// Drain meta-events and redacted-fields accumulated since last call.
fn drain_state() -> (
    Vec<(String, String)>,
    std::collections::HashMap<String, String>,
) {
    let meta = llm::take_meta_events();
    let fields = llm::take_redacted_fields();
    (meta, fields)
}

// ── T-RED-D14: WARN-mode meta-event (R4.2) ───────────────────────────────────

#[test]
fn warn_mode_records_meta_event_for_secret_field() {
    // Clear any prior state from other tests on this thread.
    drain_state();

    let subscriber = build_warn_subscriber();
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            api_key = "sk-ant-api03-ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abc",
            "test event"
        );
    });

    let (meta, fields) = drain_state();

    // WARN mode: meta-event recorded in META_EVENTS thread-local.
    assert!(
        !meta.is_empty(),
        "WARN-mode: expected meta-event in META_EVENTS; got: {meta:?}"
    );

    // REDACTED_FIELDS should have "api_key" with a redacted value.
    assert!(
        fields.contains_key("api_key"),
        "REDACTED_FIELDS should contain api_key; got: {fields:?}"
    );
    let redacted_val = &fields["api_key"];
    assert!(
        redacted_val.contains("***"),
        "redacted value should contain ***: {redacted_val}"
    );
    assert!(
        !redacted_val.contains("ABCDEFGHIJKLMNOP"),
        "redacted value should not contain original secret: {redacted_val}"
    );
}

// ── T-RED-D14: gate-mode no meta-event (R4.3) ────────────────────────────────

#[test]
fn gate_mode_no_meta_event() {
    drain_state();

    let subscriber = build_gate_subscriber();
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            api_key = "sk-ant-api03-ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abc",
            "gate test"
        );
    });

    let (meta, fields) = drain_state();

    // Gate mode: NO meta-event recorded.
    assert!(
        meta.is_empty(),
        "gate-mode: expected NO meta-events; got: {meta:?}"
    );

    // But the field IS still redacted (REDACTED_FIELDS populated).
    assert!(
        fields.contains_key("api_key"),
        "gate-mode: REDACTED_FIELDS should still be populated; got: {fields:?}"
    );
}

// ── T-RED-D14: gate + verbose emits meta-event ───────────────────────────────

#[test]
fn gate_verbose_records_meta_event() {
    drain_state();

    let subscriber = build_gate_verbose_subscriber();
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            api_key = "sk-ant-api03-ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abc",
            "gate verbose"
        );
    });

    let (meta, _fields) = drain_state();
    assert!(
        !meta.is_empty(),
        "gate+verbose: expected meta-event; got: {meta:?}"
    );
}

// ── T-RED-D14: non-secret field: no meta-event, REDACTED_FIELDS empty ─────────

#[test]
fn non_secret_field_produces_no_state() {
    drain_state();

    let subscriber = build_warn_subscriber();
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(some_field = "hello world", "non-secret event");
    });

    let (meta, fields) = drain_state();
    assert!(
        meta.is_empty(),
        "non-secret field triggered meta-event: {meta:?}"
    );
    assert!(
        fields.is_empty(),
        "non-secret field populated REDACTED_FIELDS: {fields:?}"
    );
}

// ── T-RED-D14: password field name rule ──────────────────────────────────────

#[test]
fn password_field_name_triggers_warn_meta_event() {
    drain_state();

    let subscriber = build_warn_subscriber();
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(password = "hunter2", "password event");
    });

    let (meta, fields) = drain_state();
    assert!(
        !meta.is_empty(),
        "password field should trigger meta-event in WARN mode: {meta:?}"
    );
    assert!(
        fields.contains_key("password"),
        "password field should be in REDACTED_FIELDS: {fields:?}"
    );
    let rv = &fields["password"];
    assert!(
        !rv.contains("hunter2"),
        "redacted password should not contain original: {rv}"
    );
}

// ── T-RED-D14: marker-field bypass with reason (D-RED-4) ─────────────────────

#[test]
fn marker_field_bypass_with_reason_no_redaction() {
    drain_state();

    let subscriber = build_warn_subscriber();
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            api_key_doc = "sk-ant-api03-DOCUMENTATIONEXAMPLENOTREAL",
            __redact_skip = "api_key_doc",
            __redact_reason = "documentation example; not a real key",
            "bypass test"
        );
    });

    let (meta, fields) = drain_state();

    // The field was skipped → no redaction → no meta-event.
    assert!(
        meta.is_empty(),
        "bypassed field should not produce a meta-event: {meta:?}"
    );
    // REDACTED_FIELDS should NOT contain api_key_doc (it was skipped).
    assert!(
        !fields.contains_key("api_key_doc"),
        "bypassed field should not be in REDACTED_FIELDS: {fields:?}"
    );
}

// ── T-RED-D14: marker-field missing reason → fail-safe-closed ────────────────

#[test]
fn marker_field_missing_reason_still_redacts() {
    drain_state();

    let subscriber = build_warn_subscriber();
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            password = "hunter2secret",
            __redact_skip = "password",
            // __redact_reason absent → fail-safe-closed: skip NOT applied
            "missing reason test"
        );
    });

    let (meta, fields) = drain_state();

    // missing-reason meta-event should fire.
    assert!(
        !meta.is_empty(),
        "missing reason should produce a meta-event: {meta:?}"
    );
    // password MUST be redacted (skip was rejected).
    assert!(
        fields.contains_key("password"),
        "password should still be in REDACTED_FIELDS (skip rejected): {fields:?}"
    );
    let rv = &fields["password"];
    assert!(!rv.contains("hunter2"), "password should be redacted: {rv}");
}

// ── T-RED-D14: thread-local peek/take ────────────────────────────────────────

#[test]
fn thread_local_peek_and_take_work() {
    drain_state();

    let subscriber = build_warn_subscriber();
    let peeked = tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            api_key = "sk-ant-api03-ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abc",
            "peek test"
        );
        llm::peek_redacted_fields()
    });

    assert!(
        peeked.contains_key("api_key"),
        "api_key should be in REDACTED_FIELDS after on_event: {peeked:?}"
    );
    let rv = peeked.get("api_key").expect("api_key");
    assert!(
        rv.contains("***"),
        "redacted value should contain ***: {rv}"
    );
    assert!(
        !rv.contains("ABCDEFGHIJKLMNOP"),
        "redacted value should not contain secret: {rv}"
    );
}

// ── T-RED-D14: only secret fields in REDACTED_FIELDS ─────────────────────────

#[test]
fn only_secret_fields_are_in_redacted_fields_map() {
    drain_state();

    let subscriber = build_warn_subscriber();
    let (found_safe, found_secret) = tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            safe_field = "normal value",
            api_key = "sk-ant-api03-ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abc",
            "multi-field event"
        );
        let m = llm::peek_redacted_fields();
        (m.contains_key("safe_field"), m.contains_key("api_key"))
    });

    assert!(!found_safe, "safe_field should NOT be in REDACTED_FIELDS");
    assert!(found_secret, "api_key SHOULD be in REDACTED_FIELDS");
}

// ── P-RED-1 falsification probe (D-RED-9): #[ignore] ─────────────────────────
//
// Run via: `cargo test -p llm --test redact_layer -- --ignored p_red_1`
//
// To execute the probe:
// 1. In `redact_layer.rs` `on_event`: comment out `REDACTED_FIELDS.with(...)` assignment
//    AND `emit_meta_to_stderr(...)` calls, so no state is stored.
// 2. Run with `--ignored`.
// 3. Observe: `meta` is empty AND `fields` is empty → assertions FAIL,
//    confirming on_event processing is load-bearing.
// 4. Revert.
#[ignore]
#[test]
fn p_red_1_layer_load_bearing() {
    drain_state();

    let subscriber = build_warn_subscriber();
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            password = "hunter2",
            "P-RED-1 probe: layer load-bearing test"
        );
    });

    let (meta, fields) = drain_state();

    assert!(
        !meta.is_empty(),
        "PROBE P-RED-1: on_event IS load-bearing — meta-event present confirms processing. \
         Comment out REDACTED_FIELDS assignment + emit_meta_to_stderr calls and re-run: \
         this assertion will fail (empty meta), confirming the layer is required."
    );
    assert!(
        fields.contains_key("password"),
        "PROBE P-RED-1: REDACTED_FIELDS IS populated — confirms on_event stores overrides. \
         Comment out the REDACTED_FIELDS.with() assignment and re-run: this will fail."
    );
}

// ── P-RED-2 falsification probe (D-RED-9): #[ignore] ─────────────────────────
//
// Run via: `cargo test -p llm --test redact_layer -- --ignored p_red_2`
//
// This probe verifies that the `RedactingFormatFields` reads REDACTED_FIELDS.
// In the correct setup, REDACTED_FIELDS is populated before fmt::Layer renders.
// If RedactLayer were registered AFTER fmt::Layer, REDACTED_FIELDS would be
// empty when fmt::Layer.on_event fires (since RedactLayer.on_event runs after).
//
// This probe documents the ordering dependency — the actual behavior requires
// a running binary with captured stderr, not an in-process test.
#[ignore]
#[test]
fn p_red_2_layer_ordering_documented() {
    // Layer ordering in tracing-subscriber 0.3.x:
    // `.with(A).with(B)` → A.on_event fires FIRST, then B.on_event.
    //
    // Correct install_global order: registry().with(RedactLayer).with(fmt_layer)
    // → RedactLayer.on_event fires FIRST (populates REDACTED_FIELDS)
    // → fmt_layer.on_event fires SECOND (reads REDACTED_FIELDS via RedactingFormatFields)
    //
    // Wrong order: registry().with(fmt_layer).with(RedactLayer)
    // → fmt_layer.on_event fires FIRST (reads empty REDACTED_FIELDS → no substitution)
    // → RedactLayer.on_event fires SECOND (populates REDACTED_FIELDS — too late)
    //
    // This test documents the dependency. Falsification requires running a binary
    // with wrong order and observing raw values in stderr output.
    eprintln!("P-RED-2: see tracing_init.rs for the ordering contract documentation.");
    // P-RED-2 is a documentation probe — it passes by not panicking.
}
