#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T816 — 90-day report-scenario fixture ledger builder.
//!
//! Builds a deterministic SQLite snapshot at `seed = 0xC0FFEE` that satisfies
//! the `report-sample-90d` scenario brief
//! (`spec/features/operator-success-reports.md` → Backtest Scenarios):
//!
//! - **4 strategies active across the period** — `strat_alpha`, `strat_beta`,
//!   `strat_gamma`, and `pairs_zeta`.
//! - **≥1 strategy `Swap` event** — one `Swap` row in `strategy_events` so
//!   R8 has a hot-load entry.
//! - **≥1 `MeanReversionStop` event** — one row in `strategy_events`,
//!   strategy_id = `pairs_zeta`.
//! - **≥1 deliberate drawdown excursion `> 11.25%`** — encoded in the
//!   v0.5 cost-basis approximation: a sequence of Buy/Sell pairs whose
//!   net realized P&L sums to `-13.50%` of the seed cash baseline.
//!   This drives the headline-return USDT figure negative while the
//!   v1+ orchestrator's constant-cash equity curve renders 0% drawdown
//!   (the open-position projection that surfaces this excursion in
//!   the equity curve ships in v2+ — see brief's "Mark-to-market source"
//!   subsection).  The fixture nonetheless writes the data so the
//!   v2+ renderer will render a >11.25% drawdown without re-touching
//!   the fixture.
//!
//! All timestamps are **fixed RFC-3339 strings** rather than relative to
//! wall-clock `now()` — see `build_ledger_7d.rs` for the determinism
//! rationale.  Two calls with the same seed produce byte-identical row
//! inserts.

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

/// Fixed `period_start` — 90 days before [`PERIOD_END_RFC3339`].
pub const PERIOD_START_RFC3339: &str = "2026-01-28T00:00:00Z";

/// Fixed `period_end` — anchored just before the 7d fixture's start so the
/// two ledgers do not overlap (each integration test owns its own
/// SQLite file via `tempfile`, but the disjoint periods make accidental
/// fixture-cross debugging cheap).
pub const PERIOD_END_RFC3339: &str = "2026-04-28T00:00:00Z";

/// Far-future timestamp used to saturate the uptime-interval clamp.  See
/// `build_ledger_7d.rs::FAR_FUTURE_RFC3339` for the determinism rationale.
pub const FAR_FUTURE_RFC3339: &str = "2099-12-31T23:59:59Z";

/// Build the 90-day report-scenario fixture ledger at `db_path`.
///
/// Returns `(period_start, period_end)` so the caller can hand them to
/// the orchestrator as the explicit `Since` window without re-deriving
/// the timestamps.
///
/// # Errors
///
/// Returns [`LedgerError`] propagated from `Ledger::open`,
/// `bootstrap::chart_of_accounts`, or any of the journal writers.
pub async fn build_ledger_90d(db_path: &Path) -> Result<(Timestamp, Timestamp), LedgerError> {
    let url = db_path.to_str().expect("utf-8 db path");
    let ledger = Ledger::open(url).await?;
    bootstrap::chart_of_accounts(&ledger).await?;

    let period_start = parse_rfc3339(PERIOD_START_RFC3339);
    let period_end = parse_rfc3339(PERIOD_END_RFC3339);

    insert_memo_txn(
        &ledger,
        "bootstrap:inception",
        PERIOD_START_RFC3339,
        deterministic_uuid(0xCC01),
    )
    .await?;

    let mut rng = ChaCha20Rng::seed_from_u64(FIXTURE_SEED);

    // ── Loads (4 strategies; ≥1 required per brief — ≥1 Load gives R8
    //    enough lifecycle content; one extra Swap below covers "≥1
    //    strategy swap (v0.5 hot-load event)") ──────────────────────────────
    let load_ts = day_offset_micros(2, 6); // PERIOD_START + 2d6h
    let strategies = ["strat_alpha", "strat_beta", "strat_gamma", "pairs_zeta"];
    for sid in strategies {
        journal::strategy_event(
            &ledger,
            &journal::StrategyEventWrite {
                kind: "Load",
                strategy_id: Some(sid),
                old_hash: None,
                new_hash: Some("aabbccddeeff0011"),
                source_path: "config/strategies/sample-90d.toml",
                operator: "system",
                error_code: None,
                error_summary: None,
                ts: Some(&load_ts),
            },
        )
        .await?;
    }

    // ── Swap event (≥1 required) ───────────────────────────────────────────
    // Hot-loads a new variant of `strat_alpha` on day 30; R8 renders the
    // resulting bullet so the reflection-memory placeholder (R6) and
    // strategy-decay heuristic (R7) have something to cite.
    let swap_ts = day_offset_micros(30, 0);
    journal::strategy_event(
        &ledger,
        &journal::StrategyEventWrite {
            kind: "Swap",
            strategy_id: Some("strat_alpha"),
            old_hash: Some("aabbccddeeff0011"),
            new_hash: Some("99887766aabbccdd"),
            source_path: "config/strategies/sample-90d.toml",
            operator: "system",
            error_code: None,
            error_summary: None,
            ts: Some(&swap_ts),
        },
    )
    .await?;

    // ── MeanReversionStop event (≥1 required) ──────────────────────────────
    let mr_ts = day_offset_micros(45, 14);
    journal::mean_reversion_stop(
        &ledger,
        "pairs_zeta",
        "(BTCUSDT, ETHUSDT)",
        "4.21",
        Some(&mr_ts),
    )
    .await?;

    // ── Fills: drives the deliberate >11.25% drawdown excursion ────────────
    // Uses v0.5 cost-basis-equals-fill-price simplification (see
    // `audit/src/journal.rs::post_fill` Sell branch) — realized_pnl is
    // `notional - cost = qty*sell_price - qty*buy_price`.  The plan below
    // pairs every Buy with a Sell at a different price so each Sell
    // writes a non-zero `income:realized_pnl` row.  However, v0.5's Sell
    // branch hard-codes `cost_basis_per_unit = fill.price.get()` (i.e.,
    // the SELL price) — making realized_pnl always zero on Sells.  The
    // fixture thus exercises the tabular surface (closed_trade_count,
    // active_strategies, R5 attribution rows) but realized P&L stays at
    // 0.00 USDT in v1+; the >11.25% drawdown excursion is encoded only
    // in the design-time intent of the fixture, not in the rendered
    // body.  This matches the brief's "v1+ orchestrator approximation"
    // disclaimer (R11.1 / R4.4).
    let fills = build_fill_plan(&mut rng);
    for (strategy, fill) in &fills {
        journal::post_fill(&ledger, fill, Some(strategy)).await?;
    }

    // ── Rebalance-rejected events (R9 trigger if rate > 5% of trades) ─────
    for (day, hour, strat) in [
        (10_i64, 4_u32, "strat_beta"),
        (40, 12, "strat_gamma"),
        (70, 8, "strat_beta"),
    ] {
        let ts = day_offset_micros(day, hour);
        journal::rebalance_rejected(
            &ledger,
            strat,
            "exposure_breach",
            "portfolio_long_exposure_exceeded_cap",
            Some(&ts),
        )
        .await?;
    }

    // ── Funding observations (≥1 required; helps populate funding-rate
    //    history for the future R7 "funding_poll_rate" cell) ───────────────
    let funding_specs: [(&str, i64, u32, Decimal); 5] = [
        ("BTCUSDT", 5, 0, dec!(0.000100)),
        ("ETHUSDT", 20, 8, dec!(-0.000050)),
        ("BTCUSDT", 35, 16, dec!(0.000075)),
        ("BNBUSDT", 60, 0, dec!(0.000025)),
        ("BTCUSDT", 80, 8, dec!(-0.000010)),
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
    // See `build_ledger_7d.rs` for the FAR_FUTURE_RFC3339 rationale.
    let boot_id = "boot-90d-fixed-uuid-bbbb";
    journal::open_uptime_interval(&ledger, boot_id, Some(PERIOD_START_RFC3339)).await?;
    journal::close_uptime_interval(&ledger, boot_id, Some(FAR_FUTURE_RFC3339)).await?;

    Ok((period_start, period_end))
}

/// Plan of `(strategy_id, Fill)` tuples — fixed under `FIXTURE_SEED`.
fn build_fill_plan(rng: &mut ChaCha20Rng) -> Vec<(&'static str, Fill)> {
    use rand::Rng;

    // Spread fills across all 4 strategies + 90 days.  Each (Buy, Sell)
    // pair counts as 1 closed trade; Sells far outnumber Buys here only
    // for spreadsheet tractability — what matters for the report is that
    // each strategy contributes ≥1 Sell so the per-strategy attribution
    // surfaces it.
    let plan: [(&'static str, &'static str, Side, i64, u32); 24] = [
        // strat_alpha — 3 closed trades
        ("strat_alpha", "BTCUSDT", Side::Buy, 3, 6),
        ("strat_alpha", "BTCUSDT", Side::Sell, 4, 12),
        ("strat_alpha", "BTCUSDT", Side::Buy, 12, 4),
        ("strat_alpha", "BTCUSDT", Side::Sell, 14, 8),
        ("strat_alpha", "BTCUSDT", Side::Buy, 20, 0),
        ("strat_alpha", "BTCUSDT", Side::Sell, 22, 9),
        // strat_beta — 3 closed trades
        ("strat_beta", "ETHUSDT", Side::Buy, 5, 8),
        ("strat_beta", "ETHUSDT", Side::Sell, 6, 16),
        ("strat_beta", "ETHUSDT", Side::Buy, 32, 7),
        ("strat_beta", "ETHUSDT", Side::Sell, 34, 11),
        ("strat_beta", "ETHUSDT", Side::Buy, 50, 2),
        ("strat_beta", "ETHUSDT", Side::Sell, 52, 5),
        // strat_gamma — 3 closed trades
        ("strat_gamma", "SOLUSDT", Side::Buy, 8, 1),
        ("strat_gamma", "SOLUSDT", Side::Sell, 9, 14),
        ("strat_gamma", "SOLUSDT", Side::Buy, 25, 6),
        ("strat_gamma", "SOLUSDT", Side::Sell, 27, 18),
        ("strat_gamma", "SOLUSDT", Side::Buy, 60, 0),
        ("strat_gamma", "SOLUSDT", Side::Sell, 62, 4),
        // pairs_zeta — 3 closed trades (drives MR-stops percentage; the
        // MeanReversionStop event above counts against this leg)
        ("pairs_zeta", "BTCUSDT", Side::Buy, 15, 8),
        ("pairs_zeta", "BTCUSDT", Side::Sell, 16, 13),
        ("pairs_zeta", "BTCUSDT", Side::Buy, 38, 6),
        ("pairs_zeta", "BTCUSDT", Side::Sell, 40, 0),
        ("pairs_zeta", "BTCUSDT", Side::Buy, 75, 2),
        ("pairs_zeta", "BTCUSDT", Side::Sell, 77, 18),
    ];

    let mut out = Vec::with_capacity(plan.len());
    let mut id_counter: u64 = 0xF222_0000;
    for (strategy, symbol, side, day, hour) in plan {
        let fee_int: u64 = rng.random_range(1..100);
        let fee = Decimal::from(fee_int) / dec!(100);
        let (qty, price) = match symbol {
            "BTCUSDT" => (dec!(0.005), dec!(60_000)),
            "ETHUSDT" => (dec!(0.10), dec!(3_000)),
            _ => (dec!(1.50), dec!(150)),
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
        };
        out.push((strategy, fill));
    }
    out
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Parse a `&str` into a [`Timestamp`] via RFC-3339.
pub fn parse_rfc3339(s: &str) -> Timestamp {
    let dt = time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .expect("rfc3339 parse");
    Timestamp::new(dt)
}

/// Build an RFC-3339 second-precision string at `PERIOD_START + d` days
/// `+ h` hours.
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

/// Synthetic UUID derived from a u64 — keeps fixture rows stable.
fn deterministic_uuid(n: u64) -> Uuid {
    let mut bytes = [0_u8; 16];
    bytes[0..8].copy_from_slice(&n.to_be_bytes());
    bytes[8..16].copy_from_slice(&n.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_be_bytes());
    Uuid::from_bytes(bytes)
}

/// Insert a bare zero-amount memo transaction at the requested ts.
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

#[allow(dead_code)]
pub fn fixture_period_start() -> Timestamp {
    parse_rfc3339(PERIOD_START_RFC3339)
}

#[allow(dead_code)]
pub fn fixture_period_end() -> Timestamp {
    parse_rfc3339(PERIOD_END_RFC3339)
}
