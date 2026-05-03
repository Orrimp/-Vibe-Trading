#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1007 — V8 perf smoke for `audit::query::open_positions_at`.
//!
//! Walks the V8 acceptance criterion from
//! `spec/features/real-mtm-unrealized-pnl.md`:
//!
//! > **V8 — Perf smoke.** A fixture with 100 fills (50 buy/sell
//! > pairs) + 5 open positions runs `open_positions_at(...)` in
//! > < 100ms wall-clock on the developer's box, asserted in
//! > `tests/perf_smoke.rs` (matches the v1+ R13 precedent).
//!
//! Per `spec/tasks/real-mtm-unrealized-pnl.md` T1007:
//!
//! - **100 fills** = 50 (Buy, Sell) pairs (fully closed groups —
//!   exercise the "skip net-zero groups" path), **plus 5 unmatched
//!   Buys** across 5 distinct (symbol, strategy_id) pairs, for a
//!   total of 105 inserted fills and 5 expected `OpenPosition` rows.
//! - 3 warmup iterations + 1 measured iteration to amortize SQLite
//!   page-cache cold-start.
//! - Measured iteration must complete in `< 100ms` wall-clock.
//!
//! The test follows the established `crates/reports/tests/perf_smoke.rs`
//! (T815) pattern — `Instant::now()` + `assert!(elapsed < BUDGET, ...)`
//! with a verbose failure message.  Release mode is the spec intent
//! (cargo test -p reports --test perf_smoke_open_positions --release).
//!
//! If V8 fails on the developer's box, route `HANDOFF → architect`
//! to escalate the conditional follow-up migration
//! `006_open_positions_index.sql` (Q3 escape hatch).

use std::time::{Duration, Instant};

use audit::{bootstrap, journal, query, Ledger};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    FeeTier, Fill, FillId, LedgerError, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol,
    Timestamp,
};

/// V8 wall-clock budget — assert `< 100ms`.
const PERF_BUDGET: Duration = Duration::from_millis(100);

/// Number of fully-closed (Buy, Sell) pairs in the fixture.  Each pair
/// is two fills, so 50 pairs = 100 fills.
const CLOSED_PAIRS: usize = 50;

/// Number of unmatched Buys (dangling) across distinct
/// (symbol, strategy_id) groups — these become `OpenPosition` rows.
const OPEN_POSITIONS: usize = 5;

/// Distinct symbols rotated through the closed-pair plan AND used as
/// the 5 unique symbols for the dangling Buys.  Five entries → 5
/// distinct (symbol, strategy_id) groups for the open positions
/// (one dangling Buy per symbol).
const SYMBOLS: [&str; 5] = ["BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT", "XRPUSDT"];

/// Distinct strategy_ids rotated through the closed-pair plan AND
/// pinned per-symbol for the 5 dangling Buys.
const STRATEGIES: [&str; 5] = [
    "strat_alpha",
    "strat_beta",
    "strat_gamma",
    "strat_delta",
    "strat_epsilon",
];

/// Per-symbol entry price for the dangling Buys (used for the
/// closed-pair plan as well so the cost-basis arithmetic in
/// `open_positions_at` exercises the weighted-average / proportional-
/// release path on every group).
fn entry_price_for(symbol: &str) -> Decimal {
    match symbol {
        "BTCUSDT" => dec!(60_000),
        "ETHUSDT" => dec!(3_000),
        "SOLUSDT" => dec!(150),
        "BNBUSDT" => dec!(500),
        "XRPUSDT" => dec!(0.6),
        _ => dec!(1),
    }
}

/// Per-symbol qty for the dangling Buys + closed-pair plan.
fn qty_for(symbol: &str) -> Decimal {
    match symbol {
        "BTCUSDT" => dec!(0.005),
        "ETHUSDT" => dec!(0.10),
        "SOLUSDT" => dec!(2),
        "BNBUSDT" => dec!(0.50),
        "XRPUSDT" => dec!(100),
        _ => dec!(1),
    }
}

/// Build a `Fill` with deterministic seconds-from-epoch venue_ts.
fn make_fill(symbol: &str, side: Side, qty: Decimal, price: Decimal, venue_ts_secs: i64) -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new(symbol),
        side,
        qty: Quantity::new(qty).expect("qty ok"),
        price: Price::new(price).expect("price ok"),
        fee: Money::from_decimal(dec!(0)),
        fee_tier: FeeTier::Taker,
        venue_ts: ts_offset_secs(venue_ts_secs),
        local_ts: ts_offset_secs(venue_ts_secs),
        liquidity: Liquidity::Taker,
    }
}

fn ts_offset_secs(secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs))
}

fn ts_far_future() -> Timestamp {
    // 100 years past epoch — well past every fixture timestamp below.
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::days(36500))
}

/// Build the perf fixture: 50 closed (Buy, Sell) pairs (= 100 fills)
/// plus 5 unmatched Buys across 5 distinct (symbol, strategy_id)
/// groups, for 105 total fills and 5 expected `OpenPosition` rows.
///
/// Each closed pair lives in its own per-pair group keyed by a unique
/// strategy_id (`pair_strat_<i>`) so the Buy and Sell net to exactly
/// zero within that group — the `open_positions_at` reader skips them
/// per Q7 / Q8 semantics, leaving only the 5 dangling Buys in the
/// final `Vec<OpenPosition>`.  Net-negative qty is impossible by
/// construction (no group ever sees Sell > Buy).
async fn build_perf_fixture() -> Result<Ledger, LedgerError> {
    let ledger = Ledger::in_memory().await?;
    bootstrap::chart_of_accounts(&ledger).await?;

    // ── 50 fully-closed (Buy, Sell) pairs = 100 fills ─────────────────────────
    //
    // Each pair gets its own (symbol, strategy_id) group via a unique
    // pair-scoped strategy_id; this keeps each group's running_qty
    // walking 0 → +qty → 0 (long-only at all times) so Q8 never fires.
    // Buys land at venue_ts = pair_idx * 2; Sells at venue_ts = pair_idx * 2 + 1.
    let mut venue_ts_secs: i64 = 0;
    for pair_idx in 0..CLOSED_PAIRS {
        let symbol = SYMBOLS[pair_idx % SYMBOLS.len()];
        // Per-pair strategy_id — `pair_strat_<i>` for every closed pair —
        // ensures distinct groups so Buy/Sell net-to-zero per group.
        let strategy_id = format!("pair_strat_{pair_idx:03}");
        let qty = qty_for(symbol);
        let entry_price = entry_price_for(symbol);
        let exit_price = entry_price * dec!(1.05); // +5% exit; arithmetic doesn't matter

        let buy = make_fill(symbol, Side::Buy, qty, entry_price, venue_ts_secs);
        venue_ts_secs += 1;
        journal::post_fill(&ledger, &buy, Some(&strategy_id)).await?;

        let sell = make_fill(symbol, Side::Sell, qty, exit_price, venue_ts_secs);
        venue_ts_secs += 1;
        journal::post_fill(&ledger, &sell, Some(&strategy_id)).await?;
    }

    // ── 5 unmatched Buys = 5 expected open positions ──────────────────────────
    //
    // One dangling Buy per (symbol, strategy_id) — strategies pinned
    // per-symbol so every dangling Buy lives in its own distinct group.
    // These groups carry net qty > 0 at end-of-scan and surface as
    // OpenPosition rows.
    for (i, symbol) in SYMBOLS.iter().enumerate().take(OPEN_POSITIONS) {
        let strategy_id = STRATEGIES[i % STRATEGIES.len()];
        let qty = qty_for(symbol);
        let price = entry_price_for(symbol);
        let buy = make_fill(symbol, Side::Buy, qty, price, venue_ts_secs);
        venue_ts_secs += 1;
        journal::post_fill(&ledger, &buy, Some(strategy_id)).await?;
    }

    Ok(ledger)
}

/// T1007 — V8 perf smoke. `open_positions_at` on a fixture with 100
/// closed-pair fills + 5 unmatched Buys (= 5 open positions) must
/// finish in `< 100ms` wall-clock.
///
/// Uses `Ledger::in_memory()` so SQLite I/O is RAM-bound and the
/// budget reflects the audit-query path itself, not disk latency.
/// Three warmup iterations amortize the page-cache cold-start; the
/// 4th iteration is the measured one.  Matches the existing T815
/// pattern at `crates/reports/tests/perf_smoke.rs`.
#[tokio::test(flavor = "multi_thread")]
async fn t1007_perf_smoke_open_positions_under_100ms() {
    let ledger = build_perf_fixture()
        .await
        .expect("build perf fixture (50 closed pairs + 5 dangling Buys)");

    let period_end = ts_far_future();

    // ── Sanity check: the fixture really emits 5 OpenPosition rows ────────────
    //
    // Run once before the timed loop so a fixture-shape regression
    // surfaces as a clean assertion failure, not a silently-wrong
    // perf number.  Also acts as the first warmup pass.
    let positions = query::open_positions_at(&ledger, period_end)
        .await
        .expect("open_positions_at sanity probe");
    assert_eq!(
        positions.len(),
        OPEN_POSITIONS,
        "fixture should emit exactly {OPEN_POSITIONS} open positions \
         (one dangling Buy per symbol); got {}",
        positions.len()
    );

    // ── Two more warmups (3 total including the sanity probe) ─────────────────
    for _ in 0..2 {
        let _ = query::open_positions_at(&ledger, period_end)
            .await
            .expect("open_positions_at warmup");
    }

    // ── Measured iteration ────────────────────────────────────────────────────
    let started = Instant::now();
    let result = query::open_positions_at(&ledger, period_end).await;
    let elapsed = started.elapsed();

    let positions = result.expect("open_positions_at measured iteration");
    assert_eq!(
        positions.len(),
        OPEN_POSITIONS,
        "measured iteration should emit {OPEN_POSITIONS} open positions; got {}",
        positions.len()
    );

    assert!(
        elapsed < PERF_BUDGET,
        "V8 perf budget blown: open_positions_at returned in {:?} (budget < {:?}). \
         Route HANDOFF → architect to escalate the conditional follow-up migration \
         `006_open_positions_index.sql` (spec/features/real-mtm-unrealized-pnl.md Q3 \
         escape hatch).",
        elapsed,
        PERF_BUDGET,
    );
    eprintln!(
        "T1007 V8 wall-clock: {:.3}ms (budget < {}ms) — PASS",
        elapsed.as_secs_f64() * 1000.0,
        PERF_BUDGET.as_millis(),
    );
}
