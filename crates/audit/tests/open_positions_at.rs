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
//! - **Q8 short-position branch.** A tiny in-tempfile fixture with one
//!   `Sell` against zero `Buy`s raises
//!   `LedgerError::Database("open_positions_at: net-negative qty …")`
//!   (architect Design § Q8 — long-only at v1+).
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
//! multi-symbol-sort / strategy_id / net-negative branches).  This
//! file is strictly additive — V1 / V4 / V7 against the shared T1004
//! fixture, plus the `t1005_q8_short_position_raises` named branch
//! the architect's acceptance criteria pin.

use audit::{journal, query, Ledger};
use rust_decimal_macros::dec;
use tempfile::tempdir;
use trading_core::{
    FeeTier, Fill, FillId, LedgerError, Liquidity, Money, OrderId, Price, Quantity, Side,
    StrategyId, Symbol, Timestamp, Venue,
};

#[path = "../../reports/tests/fixtures/build_ledger_with_open_positions_7d.rs"]
mod fixture;

use fixture::{
    build_ledger_with_open_positions_7d, fixture_period_end, parse_rfc3339, BTC_MARK_AT_PERIOD_END,
    ETH_MARK_AT_PERIOD_END,
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

/// Q8 — net-negative qty raises `LedgerError::Database`.
///
/// Architect Design § Q8: v1+ is long-only; one Sell against zero Buys is
/// a malformed long-only ledger and the reader must surface it loudly.
/// Builds a tiny in-tempfile fixture (independent of the T1004
/// `build_ledger_with_open_positions_7d` activity plan) with exactly one
/// Sell of `qty=1` against zero Buys, then asserts the returned error.
#[tokio::test]
async fn t1005_q8_short_position_raises() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("t1005-q8.sqlite");
    let url = db_path.to_str().expect("utf-8 db path");

    // Open + bootstrap a clean ledger; no fixture-builder needed.
    let ledger = Ledger::open(url).await.expect("open ledger");
    audit::bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");

    // Single Sell, no prior Buy — `running_qty` walks `0 → -1` for the
    // (BTCUSDT, strat_alpha) group, which the reader rejects.
    let venue_ts = parse_rfc3339("2026-04-27T20:00:00Z");
    let sell = Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: Side::Sell,
        qty: Quantity::new(dec!(1)).expect("qty"),
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
        .expect("post_fill");

    let result = query::open_positions_at(&ledger, fixture_period_end()).await;

    match result {
        Err(LedgerError::Database(msg)) => {
            assert!(
                msg.contains("net-negative qty"),
                "error message should mention 'net-negative qty' (architect Q8); got: {msg}"
            );
            assert!(
                msg.contains("open_positions_at"),
                "error message should be tagged with the function name; got: {msg}"
            );
        }
        Err(other) => panic!("expected LedgerError::Database, got {other:?}"),
        Ok(positions) => {
            panic!("expected Err for net-negative qty (Q8 long-only), got Ok({positions:?})")
        }
    }
}
