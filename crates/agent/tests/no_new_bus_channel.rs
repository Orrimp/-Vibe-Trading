//! T1808 / R8.3 — bus invariant test.
//!
//! Asserts the `agent::bus::EventBus` field set is unchanged from the
//! v1+ snapshot. Mirrors the body-no-volatile-metadata pattern from
//! `crates/reports/tests/body_no_volatile_metadata.rs`. Fails CI if a
//! future PR adds a new bus channel for cards (the reflection writer
//! mpsc must stay private to `ReflectionWriter`).
//!
//! **Updated 2026-05-26 (cockpit-activity-status-bar v0.1.0 Wave A)**:
//! Added `activity_tx` (capacity 256) per architect-approved D1 spec.
//! This is the only permitted addition at v0.1.0 — any further new
//! field still requires architect escalation before this snapshot is
//! updated.

use std::fs;
use std::path::PathBuf;

/// Pinned set of channel field names declared on `EventBus`.  This is
/// the **shape contract** of the bus; reflection-memory adds zero
/// channels (Q8 — internal mpsc on `ReflectionWriter`, not on the
/// bus).
///
/// `activity_tx` added at cockpit-activity-status-bar v0.1.0 per D1.
const EXPECTED_FIELDS: &[&str] = &[
    "activity_tx",
    "fills_tx",
    "positions_tx",
    "bars_tx",
    "ticks_tx",
    "pnl_tx",
    "mode_tx",
    "strategy_loaded_tx",
    "strategy_swapped_tx",
    "strategy_error_tx",
    "funding_obs_tx",
    "market_health_tx",
    "risk_telemetry_tx",
];

#[test]
fn t1808_event_bus_field_set_unchanged_v1_plus_snapshot() {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "src", "bus.rs"]
        .iter()
        .collect();
    let src = fs::read_to_string(&path).expect("read bus.rs");

    // Find every `pub struct EventBus { ... }` body and extract the
    // identifier on the LHS of `_tx: broadcast::Sender<...>` lines.
    let start = src
        .find("pub struct EventBus")
        .expect("EventBus struct not found");
    let body_start = src[start..]
        .find('{')
        .expect("EventBus struct missing opening brace")
        + start;
    let body_end = src[body_start..]
        .find('}')
        .expect("EventBus struct missing closing brace")
        + body_start;
    let body = &src[body_start..body_end];

    let mut found: Vec<String> = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        // Match `name_tx: broadcast::Sender<...>`.
        let Some(idx) = line.find("_tx") else {
            continue;
        };
        // Pull out the identifier preceding `_tx`.
        let prefix = &line[..idx];
        let ident_start = prefix
            .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .map_or(0, |p| p + 1);
        let ident = format!("{}_tx", &prefix[ident_start..]);
        // Sanity: must be on a `_tx: broadcast::Sender<...>` line.
        if !line.contains("broadcast::Sender") {
            continue;
        }
        found.push(ident);
    }

    found.sort();
    let mut expected: Vec<String> = EXPECTED_FIELDS.iter().map(|s| (*s).into()).collect();
    expected.sort();

    assert_eq!(
        found, expected,
        "EventBus channel set drifted from v1+ snapshot.\n\
         Found: {found:?}\n\
         Expected: {expected:?}\n\
         If a new channel was added intentionally, escalate to architect — \
         reflection-memory's writer mpsc is internal (R8.3), not a bus channel."
    );
}
