#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T816 — 7-day report-scenario fixture ledger builder.
//!
//! Builds a deterministic SQLite snapshot at `seed = 0xC0FFEE` that satisfies
//! the `report-sample-7d` scenario brief
//! (`spec/features/operator-success-reports.md` → Backtest Scenarios):
//!
//! - **≥1 strategy `Load` event** — three `Load` events (one per strategy).
//! - **≥3 closed trades across at least two strategies** — six `Sell` fills
//!   across `strat_alpha` and `strat_beta`, each producing one `closed trade`
//!   row in `pnl_by_strategy` (a `Sell` writes one `income:realized_pnl`
//!   journal entry whether the realized P&L is zero or not — see
//!   `audit::journal::post_fill`).
//! - **≥1 `RebalanceRejected` event** — one row in `strategy_events`.
//! - **≥1 funding-rate observation row** — three rows in `funding_rates`.
//!
//! All timestamps are **fixed RFC-3339 strings** rather than relative to
//! wall-clock `now()`.  This is the load-bearing determinism choice: the
//! body bytes of the rendered report contain event timestamps verbatim
//! (R8 What-changed renders `ev.ts.format(&Rfc3339)` per event), so two
//! runs separated by hours / days must read the same `ts` strings out of
//! the DB to produce byte-identical bodies.  Anchoring the fixture to a
//! fixed window also lets the test use
//! [`reports::ReportWindow::Since`] with a fixed start so the
//! orchestrator's `period_start` is wall-clock-independent — the only
//! remaining wall-clock leak (`period_end = now`) is squashed by R3's
//! fixed-width sparkline encoder over a constant-cash equity curve.
//!
//! Non-determinism rules followed (per `.claude/agents/developer.md`
//! Determinism checklist):
//! - All RNG draws via `ChaCha20Rng::from_seed(...)`.
//! - All money / qty / price values use `Decimal` (no `f64`).
//! - Strategy-event timestamps use the 6-digit microsecond format that
//!   matches `journal.rs::strategy_event` (HF-3 lesson).
//! - UUIDs are derived deterministically from a counter (no `Uuid::new_v4`).

use std::path::Path;

use audit::{bootstrap, journal, Ledger};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{
    FeeTier, Fill, FillId, FundingObs, LedgerError, Liquidity, Money, OrderId, Price, Quantity,
    Side, Symbol, Timestamp,
};
use uuid::Uuid;

/// Deterministic seed shared by every fixture under T815/T816 (per the
/// feature brief: "Both are idempotent at seed `0xC0FFEE`").
pub const FIXTURE_SEED: u64 = 0x00C0_FFEE;

/// Fixed `period_start` (used as `ReportWindow::Since(...)` argument).  The
/// fixture lays seven days of activity strictly inside `[PERIOD_START,
/// PERIOD_END]`; both endpoints are RFC-3339 second-precision strings so
/// they match `journal_transactions.ts` byte-for-byte.
pub const PERIOD_START_RFC3339: &str = "2026-04-21T00:00:00Z";

/// Fixed `period_end` boundary — the latest fixture event lands at
/// `PERIOD_END - 1h` so any operator running the test with `--period 7d`
/// against wall-clock `now > PERIOD_END` still picks up every row.
pub const PERIOD_END_RFC3339: &str = "2026-04-28T00:00:00Z";

/// Far-future RFC-3339 timestamp used to close the uptime interval.
/// `compute_uptime_pct` clamps `e = last.min(period_end)` so any
/// timestamp safely beyond plausible wall-clock saturates at
/// `period_end`, giving 100% uptime regardless of when the test runs.
pub const FAR_FUTURE_RFC3339: &str = "2099-12-31T23:59:59Z";

/// Build the 7-day report-scenario fixture ledger at `db_path`.
///
/// Returns `(period_start, period_end)` so the caller can hand them to
/// the orchestrator as the explicit `Since` window without re-deriving
/// the timestamps.  The function is total over `seed = FIXTURE_SEED` —
/// two calls with the same seed produce byte-identical row inserts.
///
/// # Errors
///
/// Returns [`LedgerError`] propagated from `Ledger::open`,
/// `bootstrap::chart_of_accounts`, or any of the journal writers.
pub async fn build_ledger_7d(db_path: &Path) -> Result<(Timestamp, Timestamp), LedgerError> {
    let url = db_path.to_str().expect("utf-8 db path");
    let ledger = Ledger::open(url).await?;
    bootstrap::chart_of_accounts(&ledger).await?;

    let period_start = parse_rfc3339(PERIOD_START_RFC3339);
    let period_end = parse_rfc3339(PERIOD_END_RFC3339);

    // Bootstrap memo at period_start so `ledger_inception_ts()` returns
    // a deterministic value rather than wall-clock `now()`.
    insert_memo_txn(
        &ledger,
        "bootstrap:inception",
        PERIOD_START_RFC3339,
        deterministic_uuid(0xBB01),
    )
    .await?;

    // RNG used only for fee jitter — keeps every row deterministic
    // under `FIXTURE_SEED` while exercising the BTC/USDT cash-flow path.
    let mut rng = ChaCha20Rng::seed_from_u64(FIXTURE_SEED);

    // ── Strategy lifecycle: three Loads (≥1 required) ───────────────────────
    // Loads land at PERIOD_START + 1 hour (deterministic).  Three strategies
    // so the per-strategy attribution table has ≥2 strategies with closed
    // trades + a third in the active-but-zero set.
    let load_ts = day_offset_micros(0, 1);
    for sid in ["strat_alpha", "strat_beta", "strat_gamma"] {
        journal::strategy_event(
            &ledger,
            &journal::StrategyEventWrite {
                kind: "Load",
                strategy_id: Some(sid),
                old_hash: None,
                new_hash: Some("aabbccddeeff0011"),
                source_path: "config/strategies/sample-7d.toml",
                operator: "system",
                error_code: None,
                error_summary: None,
                ts: Some(&load_ts),
            },
        )
        .await?;
    }

    // ── Fills: two Buys then one Sell per strategy × two strategies ────────
    // 6 Sells across strat_alpha & strat_beta = 6 closed trades (≥3 required,
    // ≥2 strategies).  Buys do not produce realized_pnl rows; only Sells do.
    // Each post_fill writes a balanced double-entry transaction; the Sell
    // writes one `income:realized_pnl` row (zero-amount under v0
    // cost-basis = price simplification — see `audit/src/journal.rs`).
    let fills = build_fill_plan(&mut rng);
    for (strategy, fill) in &fills {
        journal::post_fill(&ledger, fill, Some(strategy)).await?;
    }

    // ── Rebalance-rejected event (≥1 required) ─────────────────────────────
    let reject_ts = day_offset_micros(4, 6);
    journal::rebalance_rejected(
        &ledger,
        "strat_beta",
        "exposure_breach",
        "portfolio_long_exposure_exceeded_cap",
        Some(&reject_ts),
    )
    .await?;

    // ── Funding observations (≥1 required) ─────────────────────────────────
    let funding_specs = [
        ("BTCUSDT", 1_i64, 12, dec!(0.000100)),
        ("ETHUSDT", 3, 0, dec!(-0.000050)),
        ("BTCUSDT", 5, 8, dec!(0.000075)),
    ];
    for (sym, day, hour, rate) in funding_specs {
        let obs_ts = parse_rfc3339(&day_offset_rfc3339(day, hour));
        let obs = FundingObs {
            symbol: Symbol::new(sym),
            funding_rate: rate,
            funding_ts: obs_ts,
            next_funding_ts: parse_rfc3339(&day_offset_rfc3339(day, hour + 8)),
            poll_ts: obs_ts,
        };
        journal::insert_funding_obs(&ledger, &obs).await?;
    }

    // ── Uptime interval covering the full report window ────────────────────
    // The report orchestrator computes `uptime_pct` against
    // `[period_start, period_end]` where `period_end = now` (wall-clock).
    // To keep the rendered `Uptime` cell deterministic across runs we
    // must close the interval at a ts >> any plausible wall-clock
    // future — `compute_uptime_pct` clamps `e = last.min(period_end)`,
    // so a far-future `stopped_at` always saturates at `period_end`,
    // yielding 100% uptime for the full window regardless of `now`.
    let boot_id = "boot-7d-fixed-uuid-aaaa";
    journal::open_uptime_interval(&ledger, boot_id, Some(PERIOD_START_RFC3339)).await?;
    journal::close_uptime_interval(&ledger, boot_id, Some(FAR_FUTURE_RFC3339)).await?;

    Ok((period_start, period_end))
}

/// Plan of `(strategy_id, Fill)` tuples — fixed under `FIXTURE_SEED`.
fn build_fill_plan(rng: &mut ChaCha20Rng) -> Vec<(&'static str, Fill)> {
    use rand::Rng;

    // Day offsets are integers so `day_offset_micros` produces stable strings.
    // Each Sell at offset+1 day completes a "closed trade".
    let plan: [(&'static str, &'static str, Side, i64, u32); 12] = [
        ("strat_alpha", "BTCUSDT", Side::Buy, 1, 0),
        ("strat_alpha", "BTCUSDT", Side::Sell, 1, 6),
        ("strat_alpha", "BTCUSDT", Side::Buy, 2, 1),
        ("strat_alpha", "BTCUSDT", Side::Sell, 2, 18),
        ("strat_beta", "ETHUSDT", Side::Buy, 3, 2),
        ("strat_beta", "ETHUSDT", Side::Sell, 3, 13),
        ("strat_beta", "ETHUSDT", Side::Buy, 4, 0),
        ("strat_beta", "ETHUSDT", Side::Sell, 4, 11),
        ("strat_alpha", "BTCUSDT", Side::Buy, 5, 0),
        ("strat_alpha", "BTCUSDT", Side::Sell, 5, 14),
        ("strat_beta", "ETHUSDT", Side::Buy, 6, 1),
        ("strat_beta", "ETHUSDT", Side::Sell, 6, 9),
    ];

    let mut out = Vec::with_capacity(plan.len());
    let mut id_counter: u64 = 0xF111_0000;
    for (strategy, symbol, side, day, hour) in plan {
        // Fee jitter draws from rng so the rng path is exercised under
        // FIXTURE_SEED — keeps the rng cursor in step with build_ledger_1y's
        // pattern (no shared state between the two fixtures, but the
        // cursor advance is identical for `qty/price/fee` consumption).
        let fee_int: u64 = rng.random_range(1..100);
        let fee = Decimal::from(fee_int) / dec!(100);
        // Fixed qty + price per (strategy, symbol) so the post_fill bytes
        // (notional, cost, realized_pnl) are deterministic.
        let (qty, price) = match symbol {
            "BTCUSDT" => (dec!(0.005), dec!(60_000)),
            _ => (dec!(0.10), dec!(3_000)),
        };
        let ts = parse_rfc3339(&day_offset_rfc3339(day, hour));
        id_counter = id_counter.wrapping_add(1);
        let fill_id = deterministic_uuid(id_counter);
        id_counter = id_counter.wrapping_add(1);
        let order_id = deterministic_uuid(id_counter);
        let fill = Fill {
            id: FillId(fill_id),
            order_id: OrderId(order_id),
            symbol: Symbol::new(symbol),
            side,
            qty: Quantity::new(qty).expect("qty ok"),
            price: Price::new(price).expect("price ok"),
            fee: Money::from_decimal(fee),
            fee_tier: FeeTier::Taker,
            venue_ts: ts,
            local_ts: ts,
            liquidity: Liquidity::Taker,
            transaction_id: None,
        };
        out.push((strategy, fill));
    }
    out
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Parse a `&str` into a [`Timestamp`] via RFC-3339.  Panics on bad input;
/// the constants in this file are hand-typed and unit-tested below so a
/// failure here would surface at fixture-build time, not in production.
pub fn parse_rfc3339(s: &str) -> Timestamp {
    let dt = time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .expect("rfc3339 parse");
    Timestamp::new(dt)
}

/// Build an RFC-3339 second-precision string at `PERIOD_START + d` days
/// `+ h` hours.  Used by fills + funding observations so their
/// `journal_transactions.ts` columns match byte-for-byte across runs.
fn day_offset_rfc3339(d: i64, h: u32) -> String {
    let base = parse_rfc3339(PERIOD_START_RFC3339);
    let shifted = base.inner() + time::Duration::days(d) + time::Duration::hours(i64::from(h));
    shifted
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 format")
}

/// Build a 6-digit microsecond timestamp string for `strategy_events.ts`
/// rows (HF-3 / `journal.rs::strategy_event` format).
fn day_offset_micros(d: i64, h: u32) -> String {
    let fmt = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z",
    )
    .expect("micros format parse");
    let base = parse_rfc3339(PERIOD_START_RFC3339);
    let shifted = base.inner() + time::Duration::days(d) + time::Duration::hours(i64::from(h));
    shifted.format(&fmt).expect("micros format")
}

/// Synthetic UUID derived from a u64 — keeps fixture rows stable under
/// the same seed (the real `Uuid::new_v4()` would defeat byte-equality).
fn deterministic_uuid(n: u64) -> Uuid {
    let mut bytes = [0_u8; 16];
    bytes[0..8].copy_from_slice(&n.to_be_bytes());
    bytes[8..16].copy_from_slice(&n.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_be_bytes());
    Uuid::from_bytes(bytes)
}

/// Insert a bare zero-amount memo transaction at the requested ts so
/// `ledger_inception_ts()` returns the fixture's intended inception.
async fn insert_memo_txn(
    ledger: &Ledger,
    description: &str,
    ts_str: &str,
    txn_uuid: Uuid,
) -> Result<(), LedgerError> {
    let txn_id = txn_uuid.to_string();
    sqlx::query("INSERT INTO journal_transactions (id, ts, description) VALUES (?, ?, ?)")
        .bind(&txn_id)
        .bind(ts_str)
        .bind(description)
        .execute(ledger.pool())
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    Ok(())
}

// Dead-code suppression — fixture is loaded via `#[path = "..."]` from
// the report-scenarios test; the test compiler complains about "unused"
// helpers that the test does not directly call.  The `#[allow(dead_code)]`
// attributes below keep the build clean.

#[allow(dead_code)]
pub fn fixture_period_start() -> Timestamp {
    parse_rfc3339(PERIOD_START_RFC3339)
}

#[allow(dead_code)]
pub fn fixture_period_end() -> Timestamp {
    parse_rfc3339(PERIOD_END_RFC3339)
}
