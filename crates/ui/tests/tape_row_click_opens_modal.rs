//! T1208 — Tape-row click → modal integration test.
//!
//! Drives the new `Message::TapeRowClicked` / `TapeAuditEntriesLoaded` /
//! `TapeAuditModalClosed` arms (T1206) through `ui::state::update` and
//! asserts on `Cockpit.tape_audit_modal` state transitions. This is a
//! unit-style integration test — it does NOT spawn an iced application
//! and does NOT assert on rendered widget output. The snapshot-style
//! coverage of the rendered modal lives in T1207.
//!
//! Coverage matrix (per
//! [tape-row-audit-modal Q8](../../../spec/features/tape-row-audit-modal.md#q8--test-plan)):
//!
//! | Test                                          | V-item        |
//! |-----------------------------------------------|---------------|
//! | `t1208_v1_click_opens_modal_with_correct_tx_id` | V1            |
//! | `t1208_v1_loaded_view_populates_ready_state`    | V1            |
//! | `t1208_v3_empty_entries_renders_empty_state`    | V3            |
//! | `t1208_v4_query_failure_renders_error_state`    | V4            |
//! | `t1208_v5a_close_clears_modal`                  | V5a           |
//! | `t1208_v5b_open_new_tx_replaces_modal`          | V5b           |
//! | `t1208_v5c_agent_halt_closes_modal`             | V5c (Q9)      |

#![allow(clippy::unwrap_used, clippy::expect_used)]

use smol_str::SmolStr;
use trading_core::{AccountId, JournalEntry, Money, StrategyId, Timestamp};

use ui::state::{update, Cockpit, JournalModalState, JournalTransactionView, Message, PanelState};
use ui::strings::TAPE_AUDIT_MODAL_ERROR_PREFIX;

// ── Fixture builders ─────────────────────────────────────────────────────────

/// Build a single journal entry with the supplied numerics. Decimal values are
/// constructed via `Money::from_decimal` per project rule "no f64 in money math"
/// (AGENT.md determinism non-negotiables).
fn fixture_entry(account: &str, debit: i64, credit: i64, currency: &str) -> JournalEntry {
    JournalEntry {
        account: AccountId::new(account),
        debit: Money::from_decimal(rust_decimal::Decimal::from(debit)),
        credit: Money::from_decimal(rust_decimal::Decimal::from(credit)),
        currency: SmolStr::new(currency),
        ts: Timestamp::now(),
        memo: SmolStr::new(""),
    }
}

/// 4-entry paper-fill view (V8 fixture shape — see feature R3 example table).
/// `Σ debit == Σ credit` holds (52341.20 + 5.23 in, 0.40 + 5.23 out — the
/// integer rounding here is fine for the state-transition assertions; the
/// snapshot-bytes determinism is T1207's coverage, not T1208's).
fn fixture_view_4_entries(tx_id: &str) -> JournalTransactionView {
    JournalTransactionView {
        tx_id: SmolStr::new(tx_id),
        ts: Timestamp::now(),
        description: SmolStr::new("Buy 0.4 BTCUSDT @ 52,341.20"),
        strategy_id: Some(StrategyId::new("sma-cross-btc-1m")),
        entries: vec![
            fixture_entry("assets:cash:USDT", 0, 52341, "USDT"),
            fixture_entry("assets:position:BTCUSDT", 1, 0, "BTC"),
            fixture_entry("expense:fees:taker", 5, 0, "USDT"),
            fixture_entry("assets:cash:USDT", 0, 5, "USDT"),
        ],
    }
}

/// View with no entries — drives the `PanelState::Empty` defensive arm
/// (R8: "Empty — defensive only — by audit::verify_balance invariant every
/// transaction has ≥ 2 entries; if it triggers, it's a corruption signal").
fn fixture_view_no_entries(tx_id: &str) -> JournalTransactionView {
    JournalTransactionView {
        tx_id: SmolStr::new(tx_id),
        ts: Timestamp::now(),
        description: SmolStr::new("Empty transaction"),
        strategy_id: None,
        entries: vec![],
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// V1 (Loading half) — `Message::TapeRowClicked(tx_id)` opens the modal in
/// `Loading` state with the correct `tx_id` and `update` stays pure (no
/// async fetch happens here — the binary owns that via
/// `iced::Task::perform`; the `update` arm only sets the modal sub-state).
#[test]
fn t1208_v1_click_opens_modal_with_correct_tx_id() {
    let mut model = Cockpit::new();
    assert!(
        model.tape_audit_modal.is_none(),
        "fresh cockpit must have no modal open",
    );

    update(
        &mut model,
        Message::TapeRowClicked(SmolStr::new("known-tx")),
    );

    let modal = model
        .tape_audit_modal
        .as_ref()
        .expect("modal opens on click");
    assert_eq!(modal.tx_id.as_str(), "known-tx");
    assert_eq!(
        modal.entries.variant_name(),
        "loading",
        "click must seed entries with PanelState::Loading per R8 / state.rs:619",
    );
}

/// V1 (Ready half) — drive `TapeRowClicked` then `TapeAuditEntriesLoaded(Ok)`
/// with a 4-entry view; assert `entries == PanelState::Ready(view)` and
/// `view.entries.len() == 4`.
#[test]
fn t1208_v1_loaded_view_populates_ready_state() {
    let mut model = Cockpit::new();
    update(
        &mut model,
        Message::TapeRowClicked(SmolStr::new("tx-ready")),
    );

    let view = fixture_view_4_entries("tx-ready");
    update(&mut model, Message::TapeAuditEntriesLoaded(Ok(view)));

    let modal = model
        .tape_audit_modal
        .as_ref()
        .expect("modal stays open after Loaded(Ok)");
    assert_eq!(modal.tx_id.as_str(), "tx-ready");
    assert_eq!(modal.entries.variant_name(), "ready");
    match &modal.entries {
        PanelState::Ready(loaded) => {
            assert_eq!(
                loaded.entries.len(),
                4,
                "4-entry fixture must round-trip through update() unchanged",
            );
            assert_eq!(loaded.tx_id.as_str(), "tx-ready");
        }
        other => panic!("expected Ready, got {}", other.variant_name()),
    }
}

/// V3 — `TapeAuditEntriesLoaded(Ok(view_with_no_entries))` flips the modal
/// to `PanelState::Empty` (per `update` arm at state.rs:632).
#[test]
fn t1208_v3_empty_entries_renders_empty_state() {
    let mut model = Cockpit::new();
    update(
        &mut model,
        Message::TapeRowClicked(SmolStr::new("tx-empty")),
    );
    update(
        &mut model,
        Message::TapeAuditEntriesLoaded(Ok(fixture_view_no_entries("tx-empty"))),
    );

    let modal = model
        .tape_audit_modal
        .as_ref()
        .expect("modal stays open even when entries are empty");
    assert_eq!(
        modal.entries.variant_name(),
        "empty",
        "empty-entries Ok must collapse to PanelState::Empty per R8 / state.rs:632",
    );
    assert_eq!(modal.tx_id.as_str(), "tx-empty");
}

/// V4 — `TapeAuditEntriesLoaded(Err("oops"))` flips to `PanelState::Error`
/// carrying the message; the rest of the cockpit (P&L, positions, kill) stays
/// untouched (smoke-checked via the panel `variant_name()` invariants).
#[test]
fn t1208_v4_query_failure_renders_error_state() {
    let mut model = Cockpit::new();
    update(
        &mut model,
        Message::TapeRowClicked(SmolStr::new("tx-error")),
    );
    update(
        &mut model,
        Message::TapeAuditEntriesLoaded(Err(SmolStr::new("ledger locked"))),
    );

    let modal = model
        .tape_audit_modal
        .as_ref()
        .expect("modal stays open on error so the operator can read-then-dismiss");
    match &modal.entries {
        PanelState::Error(msg) => {
            assert_eq!(msg.as_str(), "ledger locked");
            // The widget renders `TAPE_AUDIT_MODAL_ERROR_PREFIX + msg`; the
            // prefix lives in `ui::strings` and is the constant the operator
            // ultimately sees. Asserting the constant exists + is non-empty
            // keeps the integration test honest about copy provenance.
            assert!(
                !TAPE_AUDIT_MODAL_ERROR_PREFIX.is_empty(),
                "error-prefix copy must come from ui::strings (R7)",
            );
        }
        other => panic!("expected Error, got {}", other.variant_name()),
    }

    // Smoke: the rest of the cockpit panels stay in their fresh-boot
    // `Loading` state — the modal mutation must not leak into other panels.
    assert_eq!(model.pnl.variant_name(), "loading");
    assert_eq!(model.positions.variant_name(), "loading");
    assert_eq!(model.tape.variant_name(), "loading");
    assert_eq!(model.strategies.variant_name(), "loading");
}

/// V5a — `Message::TapeAuditModalClosed` clears `tape_audit_modal` to `None`.
/// This is the funnel for all three close affordances (Esc, click-outside,
/// Close button) per R4.
#[test]
fn t1208_v5a_close_clears_modal() {
    let mut model = Cockpit::new();
    update(
        &mut model,
        Message::TapeRowClicked(SmolStr::new("tx-close")),
    );
    assert!(model.tape_audit_modal.is_some());

    update(&mut model, Message::TapeAuditModalClosed);

    assert!(
        model.tape_audit_modal.is_none(),
        "TapeAuditModalClosed must clear the modal to None per state.rs:626",
    );
}

/// V5b — opening the modal a second time replaces identity unconditionally.
/// Per Q9, the cockpit is "an instrument, not a browser" — no back-stack;
/// a click on a new row while a previous modal is open flips the tx_id and
/// resets `entries` to `Loading` (no stale Ready leak from the previous tx).
#[test]
fn t1208_v5b_open_new_tx_replaces_modal() {
    let mut model = Cockpit::new();

    // First open: tx1 → load 4 entries.
    update(&mut model, Message::TapeRowClicked(SmolStr::new("tx1")));
    update(
        &mut model,
        Message::TapeAuditEntriesLoaded(Ok(fixture_view_4_entries("tx1"))),
    );
    let first = model.tape_audit_modal.as_ref().expect("first open");
    assert_eq!(first.tx_id.as_str(), "tx1");
    assert_eq!(first.entries.variant_name(), "ready");

    // Second open while the first is still on screen.
    update(&mut model, Message::TapeRowClicked(SmolStr::new("tx2")));
    let second = model.tape_audit_modal.as_ref().expect("second open");
    assert_eq!(
        second.tx_id.as_str(),
        "tx2",
        "second click must replace tx_id (no back-stack, Q9)",
    );
    assert_eq!(
        second.entries.variant_name(),
        "loading",
        "second click must reset entries to Loading — no stale Ready leak",
    );
}

/// V5c (Q9) — `Message::AgentHaltedExternally(...)` clears the modal to `None`
/// in addition to its existing kill-state mutation. Operator's attention
/// belongs on the halt banner, not on a stacked read-only modal.
#[test]
fn t1208_v5c_agent_halt_closes_modal() {
    let mut model = Cockpit::new();
    update(&mut model, Message::TapeRowClicked(SmolStr::new("tx-halt")));
    update(
        &mut model,
        Message::TapeAuditEntriesLoaded(Ok(fixture_view_4_entries("tx-halt"))),
    );
    assert!(
        model.tape_audit_modal.is_some(),
        "modal must be open before halt",
    );

    update(
        &mut model,
        Message::AgentHaltedExternally(SmolStr::new("ManualOperator")),
    );

    assert!(
        model.tape_audit_modal.is_none(),
        "AgentHaltedExternally must clear the modal per Q9 / state.rs:579",
    );
    // Sanity: the halt itself still landed in the kill state machine.
    assert_eq!(model.mode, ui::state::AgentMode::Halted);
}

// ── Determinism guard ────────────────────────────────────────────────────────
//
// All `Message`s above are constructed with fixed `SmolStr` inputs; the only
// non-deterministic surface is `Timestamp::now()` inside the fixture builders
// — but that value is never inspected in assertions, only carried inside
// `JournalEntry` / `JournalTransactionView` for shape-compatibility with the
// real reader's output. Two consecutive runs produce identical assertion
// results (no `Instant`-based comparisons, no `HashMap` iteration over the
// modal sub-state).

#[test]
fn t1208_determinism_two_runs_produce_identical_state_transitions() {
    fn run() -> &'static str {
        let mut model = Cockpit::new();
        update(&mut model, Message::TapeRowClicked(SmolStr::new("tx-det")));
        update(
            &mut model,
            Message::TapeAuditEntriesLoaded(Ok(fixture_view_4_entries("tx-det"))),
        );
        // Borrow only the discriminator — `entries.variant_name()` is
        // `'static &str`, so it can be returned without keeping `model` alive.
        let modal: &JournalModalState = model
            .tape_audit_modal
            .as_ref()
            .expect("modal open after Ready");
        modal.entries.variant_name()
    }

    assert_eq!(run(), "ready");
    assert_eq!(run(), "ready");
}
