#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1304 / V3 — chained metadata→entries fetch produces a complete
//! `JournalTransactionView` with populated `description` and
//! `strategy_id`. Pins the live-mode chain end-to-end without coupling
//! to the modal's rendered shape (Q5 wiring smoke test, NOT a new
//! snapshot).
//!
//! ## What this test asserts
//!
//! The architect's
//! [Design § Q4](../../../spec/features/journal-transactions-metadata.md#q4--sequential-await-not-tokiojoin)
//! pins the cockpit_live `Task::perform` chain as: metadata first
//! (with a `None` short-circuit), then entries. T1303 wires that chain
//! at `crates/ui/src/bin/cockpit_live.rs:496-552`. This test drives
//! the same two-reader sequence the closure invokes (the iced
//! `Task::perform` runtime is heavyweight in tests; a structurally
//! equivalent direct invocation is the V3 coverage agreed by the
//! architect).
//!
//! Two `#[tokio::test]` cases:
//!
//! 1. `t1304_v3_chained_fetch_populates_view_header` — happy path.
//!    Boot in-memory ledger, post one paper Buy with a strategy_id,
//!    drive the chain, assert the resulting `JournalTransactionView`
//!    carries non-empty `description` and `Some(strategy_id)` (no
//!    longer the T1206 partial defaults).
//! 2. `t1304_v3b_unknown_tx_short_circuits_to_error` —
//!    [Design § Q6](../../../spec/features/journal-transactions-metadata.md#q6--partial-failure-semantics-any-err--error-state)
//!    `None`-arm. A bogus UUID against an empty ledger returns
//!    `Err("{TAPE_AUDIT_MODAL_ERROR_PREFIX}unknown transaction")` —
//!    the entries query is skipped (Q4 short-circuit benefit) and the
//!    modal renders its existing error state.

use audit::query::{journal_entries_for_transaction, journal_transaction_metadata};
use audit::{bootstrap, journal, Ledger};
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use time::OffsetDateTime;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, StrategyId, Symbol,
    Timestamp, Venue,
};
use ui::state::JournalTransactionView;
use ui::strings::TAPE_AUDIT_MODAL_ERROR_PREFIX;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Open an in-memory ledger with the chart-of-accounts bootstrapped —
/// same fixture as the T1302 boundary tests so we exercise the chain
/// against the real `journal_transactions` + `journal_entries` schema.
async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

fn ts_offset_secs(secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs))
}

/// Mirror of the T1302 paper-fill fixture. The `journal::post_fill`
/// description site renders this as `"buy 0.4 BTCUSDT @ 52341.20"`
/// (lowercase `Side`, no thousands separator).
fn make_paper_buy_fill() -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: Side::Buy,
        qty: Quantity::new(dec!(0.4)).expect("qty ok"),
        price: Price::new(dec!(52341.20)).expect("price ok"),
        fee: Money::from_decimal(dec!(5.23)),
        fee_tier: FeeTier::Taker,
        venue_ts: ts_offset_secs(1_000),
        local_ts: ts_offset_secs(1_000),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

/// Drive the same two-reader sequence the cockpit_live `Task::perform`
/// closure invokes at `crates/ui/src/bin/cockpit_live.rs:496-552`.
/// Returns `Result<JournalTransactionView, SmolStr>` — byte-identical
/// to the closure's return type (T1206's
/// `Message::TapeAuditEntriesLoaded` payload shape).
async fn drive_chain(ledger: &Ledger, tx_id_str: &str) -> Result<JournalTransactionView, SmolStr> {
    let meta = match journal_transaction_metadata(ledger, tx_id_str).await {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Err(SmolStr::new(format!(
                "{TAPE_AUDIT_MODAL_ERROR_PREFIX}unknown transaction"
            )));
        }
        Err(e) => {
            return Err(SmolStr::new(format!("{TAPE_AUDIT_MODAL_ERROR_PREFIX}{e}")));
        }
    };
    match journal_entries_for_transaction(ledger, tx_id_str).await {
        Ok(entries) => Ok(JournalTransactionView {
            tx_id: meta.transaction_id,
            ts: meta.ts,
            description: meta.description,
            strategy_id: meta.strategy_id,
            entries,
        }),
        Err(e) => Err(SmolStr::new(format!("{TAPE_AUDIT_MODAL_ERROR_PREFIX}{e}"))),
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// V3 happy path — the chained fetch populates `description` AND
/// `strategy_id` on the resulting view, replacing the T1206 defaults
/// (`description: SmolStr::default()`, `strategy_id: None`).
#[tokio::test]
async fn t1304_v3_chained_fetch_populates_view_header() {
    let ledger = open_ledger().await;

    let fill = make_paper_buy_fill();
    let expected_ts = fill.venue_ts;
    let txn_id = journal::post_fill(&ledger, &fill, Venue::Binance, Some("sma-cross-btc-1m"))
        .await
        .expect("post Buy fill");

    let view = drive_chain(&ledger, &txn_id)
        .await
        .expect("chained fetch must succeed for a freshly-posted fill");

    assert_eq!(
        view.tx_id.as_str(),
        txn_id.as_str(),
        "view.tx_id must round-trip the post_fill return value"
    );
    assert!(
        !view.description.as_str().is_empty(),
        "view.description must be non-empty after the chained fetch \
         (was empty SmolStr::default() in the T1206 partial-view path)"
    );
    assert_eq!(
        view.description.as_str(),
        "buy 0.4 BTCUSDT @ 52341.20",
        "view.description must match the journal::post_fill format!() shape"
    );
    assert_eq!(
        view.strategy_id,
        Some(StrategyId::new("sma-cross-btc-1m")),
        "view.strategy_id must round-trip the post_fill argument \
         (was None in the T1206 partial-view path)"
    );
    assert_eq!(
        view.ts, expected_ts,
        "view.ts must round-trip the fill's venue_ts (no longer Timestamp::now() proxy)"
    );
    assert!(
        !view.entries.is_empty(),
        "paper Buy writes >= 2 journal entries (chart-of-accounts double-entry)"
    );
}

/// V3b defensive — Q6 `None`-arm. A non-existent tx_id short-circuits
/// the chain (entries query NEVER fires) and produces the architect's
/// "unknown transaction" error. Guards against a regression that
/// swallows the metadata `None` arm into a partial render.
#[tokio::test]
async fn t1304_v3b_unknown_tx_short_circuits_to_error() {
    let ledger = open_ledger().await;

    // Fresh ledger -> any tx_id is unknown.
    let bogus_tx_id = "00000000-0000-0000-0000-deadbeefcafe";
    let result = drive_chain(&ledger, bogus_tx_id).await;

    let err = result.expect_err("bogus tx_id must short-circuit to Err per Q6");
    let expected = format!("{TAPE_AUDIT_MODAL_ERROR_PREFIX}unknown transaction");
    assert_eq!(
        err.as_str(),
        expected,
        "the metadata-None arm must produce the exact \
         `{{TAPE_AUDIT_MODAL_ERROR_PREFIX}}unknown transaction` string \
         the modal renders as PanelState::Error"
    );
}
