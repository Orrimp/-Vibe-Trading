//! T2010 — Chart marker click → journal-transaction modal integration
//! test (chart-buy-sell-emphasis v1.9).
//!
//! Asserts that the existing `Message::TapeRowClicked(transaction_id)`
//! arm — which the chart canvas's `ChartProgram::update` impl dispatches
//! on `mouse::Event::ButtonPressed(Left)` for fill markers (R4.5) —
//! drives the cockpit's `tape_audit_modal` sub-state through
//! `Loading`/`Ready` exactly as a tape-row click does. No new modal
//! widget; the chart click reuses the shipped tape-row-audit-modal
//! machinery (V4).
//!
//! Pure unit-style integration test — no iced runtime spawned.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{
    AccountId, FeeTier, FillView, JournalEntry, Money, Price, Quantity, Side, StrategyId, Symbol,
    Timestamp,
};

use ui::state::{update, Cockpit, JournalTransactionView, Message, PanelState};

fn fixed_ts() -> Timestamp {
    let dt = time::OffsetDateTime::from_unix_timestamp(1_705_320_000)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    Timestamp::new(dt)
}

fn make_fill(tx_id: &str) -> FillView {
    FillView {
        symbol: Symbol::new("BTCUSDT"),
        side: Side::Buy,
        price: Price::new(dec!(40_010)).unwrap(),
        qty: Quantity::new(dec!(0.1)).unwrap(),
        fee: Money::from_decimal(dec!(0.5)),
        fee_tier: FeeTier::Taker,
        venue_ts: fixed_ts(),
        transaction_id: SmolStr::new(tx_id),
    }
}

fn fixture_view(tx_id: &str) -> JournalTransactionView {
    JournalTransactionView {
        tx_id: SmolStr::new(tx_id),
        ts: fixed_ts(),
        description: SmolStr::new("Buy 0.1 BTCUSDT @ 40,010"),
        strategy_id: Some(StrategyId::new("sma_crossover")),
        entries: vec![JournalEntry {
            account: AccountId::new("assets:cash:USDT"),
            debit: Money::from_decimal(dec!(0)),
            credit: Money::from_decimal(dec!(4001)),
            currency: SmolStr::new("USDT"),
            ts: fixed_ts(),
            memo: SmolStr::new(""),
        }],
    }
}

/// T2010 V4 — clicking a chart fill marker dispatches
/// `Message::TapeRowClicked(transaction_id)`, which flips
/// `cockpit.tape_audit_modal` from `None` to
/// `Some(JournalModalState { tx_id, entries: Loading })`.
///
/// Then loading the entries flips `entries` to `Ready(view)` with the
/// view's `tx_id` matching the clicked marker's transaction id (R4.5).
#[test]
fn chart_marker_click_opens_modal_with_clicked_tx_id() {
    let mut cockpit = Cockpit::default();
    let fill = make_fill("tx-abc-001");
    cockpit.chart_markers = PanelState::Ready(vec![fill.clone()]);
    assert!(cockpit.tape_audit_modal.is_none(), "cold-start: no modal");

    // Simulate the chart canvas's click-dispatch — it sends the same
    // `TapeRowClicked` arm the tape-row click sends. This is the load-
    // bearing R11.3 invariant: no new modal widget, no new message arm.
    update(
        &mut cockpit,
        Message::TapeRowClicked(fill.transaction_id.clone()),
    );

    let modal = cockpit
        .tape_audit_modal
        .as_ref()
        .expect("click should open modal");
    assert_eq!(
        modal.tx_id.as_str(),
        "tx-abc-001",
        "modal carries clicked marker's transaction id"
    );
    assert!(
        matches!(modal.entries, PanelState::Loading),
        "entries start in Loading until the async fetch returns"
    );

    // Simulate the async fetch returning Ok(view).
    let view = fixture_view("tx-abc-001");
    update(
        &mut cockpit,
        Message::TapeAuditEntriesLoaded(Ok(view.clone())),
    );

    let modal = cockpit.tape_audit_modal.as_ref().expect("modal still open");
    match &modal.entries {
        PanelState::Ready(v) => assert_eq!(v.tx_id.as_str(), "tx-abc-001"),
        other => panic!("expected Ready entries, got {other:?}"),
    }
}

/// T2010 — Clicking a chart marker for a different transaction replaces
/// the modal state with the new tx-id (Q9 — only one modal at a time).
#[test]
fn chart_marker_click_replaces_modal_on_second_click() {
    let mut cockpit = Cockpit::default();
    cockpit.chart_markers = PanelState::Ready(vec![make_fill("tx-first"), make_fill("tx-second")]);

    update(
        &mut cockpit,
        Message::TapeRowClicked(SmolStr::new("tx-first")),
    );
    update(
        &mut cockpit,
        Message::TapeRowClicked(SmolStr::new("tx-second")),
    );

    let modal = cockpit.tape_audit_modal.as_ref().expect("modal still open");
    assert_eq!(
        modal.tx_id.as_str(),
        "tx-second",
        "second click replaces the modal id"
    );
}
