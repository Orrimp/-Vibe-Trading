#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1005 — V1 + V4 + V7 verification gates for the v1+ real
//! mark-to-market unrealized-P&L feature
//! (`spec/features/real-mtm-unrealized-pnl.md`).
//!
//! Per the architect's task spec
//! (`spec/tasks/real-mtm-unrealized-pnl.md` → T1005):
//!
//! - **V1 — reader correctness.** Open the
//!   `build_ledger_with_open_positions_7d` fixture (T1004); call
//!   `audit::query::open_positions_at(&ledger, period_end)`; assert
//!   the returned `Vec<OpenPosition>` is byte-identical (`assert_eq!`)
//!   to the architect's hand-computed expected vec of two rows
//!   (BTCUSDT before ETHUSDT, alphabetical sort, R6).
//! - **V4 — reconciliation invariant.** For every `transaction_id` in
//!   `journal_transactions`, call `audit::journal::verify_balance(...)`
//!   and assert `Ok` — the new reader must not introduce any
//!   debit/credit imbalance (it is read-only over the journal).
//! - **V7 — determinism.** Two consecutive
//!   `open_positions_at(&ledger, period_end)` calls on the same opened
//!   ledger return `Vec<OpenPosition>` slices that compare equal
//!   byte-for-byte.
//! - **ADR-0068 D7 — signed reader.** A tiny in-tempfile fixture with one
//!   `Sell` against zero `Buy`s (a sell-to-open short) now materializes
//!   as a signed `OpenPosition` with `qty < 0` — NOT a `LedgerError::Database`.
//!   The old Q8 long-only invariant is superseded by ADR-0068 D7 (reader-only
//!   relaxation; writer + reconciler unchanged).
//!
//! The fixture file
//! (`crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs`)
//! is mounted via `#[path]` so the tests below exercise the same
//! deterministic ledger that the orchestrator (T1003) and V2 / V6
//! (T1006) consume.  The fixture's fee-jitter RNG is seeded by
//! `ChaCha20Rng::seed_from_u64(FIXTURE_SEED)`; `rand` and `rand_chacha`
//! were added to `crates/audit/Cargo.toml`'s `[dev-dependencies]`
//! solely so the same fixture compiles when re-mounted from the audit
//! tests directory.
//!
//! T1002's `crates/audit/tests/open_positions.rs` already covers the
//! algorithmic surface of the reader (8 unit-style integration tests
//! across empty / single / closed / weighted-avg / partial-close /
//! multi-symbol-sort / strategy_id branches).  This file is strictly
//! additive — V1 / V4 / V7 against the shared T1004 fixture, plus
//! the ADR-0068 D7 signed-position tests.

use audit::{Ledger, journal, query};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tempfile::tempdir;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, StrategyId, Symbol,
    Timestamp, Venue,
};

#[path = "../../reports/tests/fixtures/build_ledger_with_open_positions_7d.rs"]
mod fixture;

use fixture::{
    BTC_MARK_AT_PERIOD_END, ETH_MARK_AT_PERIOD_END, build_ledger_with_open_positions_7d,
    fixture_period_end, parse_rfc3339,
};

// `BTC_MARK_AT_PERIOD_END` and `ETH_MARK_AT_PERIOD_END` are pulled in only
// to surface a compile-time dependency on the fixture's mark constants
// (so a future fixture refactor that drops them flags this test file).
// They are not consumed by the V1 / V4 / V7 assertions below.
const _: rust_decimal::Decimal = BTC_MARK_AT_PERIOD_END;
const _: rust_decimal::Decimal = ETH_MARK_AT_PERIOD_END;

/// V1 — reader correctness.
///
/// Open the T1004 fixture; call `open_positions_at(&ledger, period_end)`;
/// assert the returned `Vec<OpenPosition>` matches the architect's
/// hand-computed expected vec byte-for-byte (BTCUSDT before ETHUSDT,
/// alphabetical R6 sort).  The two rows correspond to the fixture's two
/// dangling Buys at day 6 hour 20 — `(strat_alpha, BTCUSDT, qty=0.01,
/// price=60_000)` and `(strat_beta, ETHUSDT, qty=0.20, price=3_000)`.
///
/// The first dangling Buy lands in the `(strat_alpha, BTCUSDT)` group
/// that already saw 3 closed (Buy, Sell) pairs; weighted-average cost
/// basis collapses to the single open Buy's price (60_000) because each
/// preceding Sell zeroed both `running_qty` and `running_notional` per
/// architect Design § Q7.
#[tokio::test]
async fn t1005_v1_reader_emits_two_open_positions() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("t1005-v1.sqlite");
    let (_period_start, period_end) = build_ledger_with_open_positions_7d(&db_path)
        .await
        .expect("build T1004 fixture");

    let url = db_path.to_str().expect("utf-8 db path");
    let ledger = Ledger::open(url).await.expect("re-open ledger");

    let positions = query::open_positions_at(&ledger, period_end)
        .await
        .expect("open_positions_at");

    // Hand-computed expected vec (architect Design § Q5):
    //   row 0: BTCUSDT  qty 0.01  cost 60_000  opened 2026-04-27T20:00:00Z  strat_alpha
    //   row 1: ETHUSDT  qty 0.20  cost  3_000  opened 2026-04-27T20:00:00Z  strat_beta
    let day6_hour20: Timestamp = parse_rfc3339("2026-04-27T20:00:00Z");

    assert_eq!(
        positions.len(),
        2,
        "fixture should emit 2 open positions at period_end (one BTCUSDT + \
         one ETHUSDT dangling Buy); got {}",
        positions.len()
    );

    // Row 0 — BTCUSDT (alphabetical first per R6).
    let btc = &positions[0];
    assert_eq!(btc.symbol, Symbol::new("BTCUSDT"), "row 0 symbol");
    assert_eq!(btc.qty, dec!(0.01), "row 0 qty");
    assert_eq!(
        btc.avg_cost_basis,
        Money::from_decimal(dec!(60_000)),
        "row 0 avg_cost_basis (per-unit, USDT per BTC)"
    );
    assert_eq!(
        btc.opened_at, day6_hour20,
        "row 0 opened_at — first un-closed Buy at fixture day-6 hour-20"
    );
    assert_eq!(
        btc.strategy_id,
        Some(StrategyId::new("strat_alpha")),
        "row 0 strategy_id"
    );

    // Row 1 — ETHUSDT (alphabetical second).
    let eth = &positions[1];
    assert_eq!(eth.symbol, Symbol::new("ETHUSDT"), "row 1 symbol");
    assert_eq!(eth.qty, dec!(0.20), "row 1 qty");
    assert_eq!(
        eth.avg_cost_basis,
        Money::from_decimal(dec!(3_000)),
        "row 1 avg_cost_basis (per-unit, USDT per ETH)"
    );
    assert_eq!(
        eth.opened_at, day6_hour20,
        "row 1 opened_at — first un-closed Buy at fixture day-6 hour-20"
    );
    assert_eq!(
        eth.strategy_id,
        Some(StrategyId::new("strat_beta")),
        "row 1 strategy_id"
    );

    // Belt-and-braces: the architect's spec says "byte-identical via
    // `assert_eq!`" against a hand-computed expected vec, so build that
    // vec and compare in one shot — this catches any future field
    // additions that the per-field asserts above might miss.
    let expected = vec![
        trading_core::OpenPosition {
            symbol: Symbol::new("BTCUSDT"),
            qty: dec!(0.01),
            avg_cost_basis: Money::from_decimal(dec!(60_000)),
            opened_at: day6_hour20,
            strategy_id: Some(StrategyId::new("strat_alpha")),
        },
        trading_core::OpenPosition {
            symbol: Symbol::new("ETHUSDT"),
            qty: dec!(0.20),
            avg_cost_basis: Money::from_decimal(dec!(3_000)),
            opened_at: day6_hour20,
            strategy_id: Some(StrategyId::new("strat_beta")),
        },
    ];
    assert_eq!(
        positions, expected,
        "byte-identical match against hand-computed expected vec (R6 sort)"
    );
}

/// V4 — reconciliation invariant: `Σ debits == Σ credits` per transaction.
///
/// The new reader (T1002) is read-only over `journal_transactions` /
/// `journal_entries`; no new accounts, no schema change. Therefore every
/// transaction in the fixture's journal must still satisfy
/// `audit::journal::verify_balance(&ledger, txn_id) == Ok(())`.
///
/// The fixture writes:
///   - 1 bootstrap memo (`bootstrap:inception` — zero-amount, no entries),
///   - 14 `post_fill` transactions (each writes 4 balanced entries: Dr
///     position + Cr cash + Dr fee + Cr cash, see `journal::post_fill`),
///   - 0 strategy-event journal_entries (those go to `strategy_events`),
///   - 0 uptime-interval journal_entries (those go to `uptime_intervals`).
///
/// `verify_balance` iterates `journal_entries` for the given txn_id; for
/// the 14 post_fill rows the sums must exactly cancel. For the bootstrap
/// memo no rows are returned, so the function early-returns Ok (the
/// `Σ debits − Σ credits == 0` tolerance trivially holds).
#[tokio::test]
async fn t1005_v4_balance_invariant_per_txn() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("t1005-v4.sqlite");
    let _ = build_ledger_with_open_positions_7d(&db_path)
        .await
        .expect("build T1004 fixture");

    let url = db_path.to_str().expect("utf-8 db path");
    let ledger = Ledger::open(url).await.expect("re-open ledger");

    // Pull every transaction id directly from the journal_transactions
    // header table.  Using sqlx::query_as keeps the test independent of
    // any future read-helper additions and matches the pattern in
    // `crates/audit/tests/ledger_integration.rs`.
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM journal_transactions ORDER BY ts ASC, id ASC")
            .fetch_all(ledger.pool())
            .await
            .expect("select txn ids");

    assert!(
        rows.len() >= 14,
        "fixture should include at least 14 post_fill transactions \
         (12 closed + 2 open); got {} txn rows",
        rows.len()
    );

    for (txn_id,) in &rows {
        journal::verify_balance(&ledger, txn_id)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "verify_balance failed for txn {txn_id}: {e:?} — \
                 the new reader must not introduce any debit/credit imbalance"
                )
            });
    }
}

/// V7 — determinism: two reads return byte-identical Vecs.
///
/// `open_positions_at(&ledger, period_end)` is a pure SQL fold over
/// `journal_transactions` — no `SystemTime::now()`, no `f64`, no
/// `HashMap` (architect Design § Determinism guardrails). Two calls
/// against the same opened ledger must therefore yield `Vec<OpenPosition>`
/// slices that `assert_eq!` byte-for-byte.
#[tokio::test]
async fn t1005_v7_two_reads_byte_identical() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("t1005-v7.sqlite");
    let (_period_start, period_end) = build_ledger_with_open_positions_7d(&db_path)
        .await
        .expect("build T1004 fixture");

    let url = db_path.to_str().expect("utf-8 db path");
    let ledger = Ledger::open(url).await.expect("re-open ledger");

    let first = query::open_positions_at(&ledger, period_end)
        .await
        .expect("open_positions_at #1");
    let second = query::open_positions_at(&ledger, period_end)
        .await
        .expect("open_positions_at #2");

    assert_eq!(
        first, second,
        "two consecutive reads against the same ledger must be byte-identical (R6)"
    );
    // Belt-and-braces: also asserts the `fixture_period_end()` helper
    // matches the period_end the builder returned, so a future fixture
    // refactor that drifts the constants gets caught here.
    assert_eq!(
        period_end,
        fixture_period_end(),
        "fixture's period_end constant must match the builder's return value"
    );
}

/// ADR-0068 D7 — a journaled sell-to-open materializes as a signed `OpenPosition`
/// (`qty < 0`), NOT a `LedgerError::Database` error.
///
/// This test is the acceptance criterion for the reader-only relaxation
/// from ADR-0068 D7. The writer (`post_fill_with_signal`) is unchanged —
/// only the reader's materializer changes.
///
/// Previously the Q8 test expected a `LedgerError::Database` for
/// net-negative qty. After ADR-0068 D7, the reader emits a signed row.
#[tokio::test]
async fn t1005_d7_sell_to_open_materializes_as_signed_position() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("t1005-d7-signed.sqlite");
    let url = db_path.to_str().expect("utf-8 db path");

    // Open + bootstrap a clean ledger.
    let ledger = Ledger::open(url).await.expect("open ledger");
    audit::bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");

    // Sell-to-open (short): one Sell of qty=0.5 at price=70_000 against zero Buys.
    // After D7 the reader must emit qty=-0.5, avg_cost_basis=70_000.
    let venue_ts = parse_rfc3339("2026-04-27T20:00:00Z");
    let sell = Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: Side::Sell,
        qty: Quantity::new(dec!(0.5)).expect("qty"),
        price: Price::new(dec!(70_000)).expect("price"),
        fee: Money::from_decimal(dec!(0)),
        fee_tier: FeeTier::Taker,
        venue_ts,
        local_ts: venue_ts,
        liquidity: Liquidity::Taker,
        transaction_id: None,
    };
    journal::post_fill(&ledger, &sell, Venue::Binance, Some("strat_alpha"))
        .await
        .expect("post_fill sell-to-open");

    let positions = query::open_positions_at(&ledger, fixture_period_end())
        .await
        .expect("open_positions_at must not raise on a short position (ADR-0068 D7)");

    assert_eq!(
        positions.len(),
        1,
        "a single sell-to-open should materialize as 1 open position (short); got: {positions:?}"
    );
    let pos = &positions[0];
    assert_eq!(pos.symbol, Symbol::new("BTCUSDT"), "symbol");
    assert!(
        pos.qty < Decimal::ZERO,
        "short position qty must be negative (ADR-0068 D7); got qty={}",
        pos.qty
    );
    assert_eq!(
        pos.qty,
        dec!(-0.5),
        "qty must equal -(fill qty); got: {}",
        pos.qty
    );
    assert_eq!(
        pos.avg_cost_basis,
        Money::from_decimal(dec!(70_000)),
        "avg_cost_basis must be the open (sell) price for a short lot; got: {:?}",
        pos.avg_cost_basis
    );
    assert_eq!(
        pos.strategy_id,
        Some(StrategyId::new("strat_alpha")),
        "strategy_id must be preserved"
    );
}

/// ADR-0068 D7 regression — a long ledger reads back byte-identical after the
/// reader relaxation. The existing V1/V4/V7 tests cover the full fixture; this
/// targeted test proves an existing long position is unaffected.
#[tokio::test]
async fn t1005_d7_long_position_byte_identical_after_relaxation() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("t1005-d7-long-regression.sqlite");
    let url = db_path.to_str().expect("utf-8 db path");

    let ledger = Ledger::open(url).await.expect("open ledger");
    audit::bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");

    // Buy-to-open a long position.
    let venue_ts = parse_rfc3339("2026-04-27T20:00:00Z");
    let buy = Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: Side::Buy,
        qty: Quantity::new(dec!(0.01)).expect("qty"),
        price: Price::new(dec!(60_000)).expect("price"),
        fee: Money::from_decimal(dec!(0)),
        fee_tier: FeeTier::Taker,
        venue_ts,
        local_ts: venue_ts,
        liquidity: Liquidity::Taker,
        transaction_id: None,
    };
    journal::post_fill(&ledger, &buy, Venue::Binance, Some("strat_alpha"))
        .await
        .expect("post_fill buy");

    let positions = query::open_positions_at(&ledger, fixture_period_end())
        .await
        .expect("long position must materialize");

    assert_eq!(positions.len(), 1, "one open position");
    let pos = &positions[0];
    assert!(
        pos.qty > Decimal::ZERO,
        "long position qty must be positive; got: {}",
        pos.qty
    );
    assert_eq!(pos.qty, dec!(0.01), "long qty");
    assert_eq!(
        pos.avg_cost_basis,
        Money::from_decimal(dec!(60_000)),
        "long avg_cost_basis"
    );
}
