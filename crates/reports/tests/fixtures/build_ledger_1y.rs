#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T815 — 1-year-history fixture ledger builder for the perf smoke test.
//!
//! Composes a deterministic SQLite snapshot at `seed = 0xC0FFEE` covering
//! roughly one year of activity:
//!
//! - **Inception**: 365 days before `period_end`.
//! - **Fills**: ~10 fills/day across 4 strategies × 365 days = ~3,650 fills.
//! - **Strategy events**: ~3 events/week (Load/Swap/Unload) × 52 weeks ≈ 156.
//! - **Funding observations**: 200 hourly snapshots over the year.
//! - **Kill-switch trips**: 8 deterministic events spread over the year.
//!
//! All timestamps are computed deterministically from a fixed `period_end`
//! (the caller passes it in) and a deterministic stride, so two calls with
//! the same `seed` produce byte-identical SQL inserts.  Randomness is
//! limited to `ChaCha20Rng::from_seed(...)` to draw fill quantities and
//! prices within bounded ranges — this matches the determinism rules
//! enumerated in `.claude/agents/developer.md` ("Determinism checklist").
//!
//! The fixture is sized for 90d activity (~900 fills + ~37 events fall in
//! the `--period 90d` window) but the ledger spans the full year so the
//! orchestrator's full-table reads (`recent_fills(usize::MAX)`,
//! `recent_journal(usize::MAX)`) exercise realistic depth.

use std::path::Path;

use audit::{bootstrap, journal, Ledger};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    FeeTier, Fill, FillId, FundingObs, LedgerError, Liquidity, Money, OrderId, Price, Quantity,
    Side, Symbol, Timestamp,
};
use uuid::Uuid;

/// Deterministic seed shared by every fixture under T815/T816 (per the
/// feature brief: "Both are idempotent at seed `0xC0FFEE`").
pub const FIXTURE_SEED: u64 = 0x00C0_FFEE;

/// Build a 1-year-history fixture ledger at `db_path`.
///
/// Returns `(inception, period_end)` so the caller can pass them to the
/// orchestrator without re-deriving wall-clock-relative timestamps.  The
/// `period_end` value lands at the latest fixture timestamp, so a
/// `--period 90d` window resolves cleanly inside the fixture's coverage.
///
/// # Errors
///
/// Returns [`LedgerError`] propagated from `Ledger::open`,
/// `bootstrap::chart_of_accounts`, or any of the journal writers.
pub async fn build_ledger_1y(
    db_path: &Path,
    period_end: Timestamp,
) -> Result<(Timestamp, Timestamp), LedgerError> {
    let url = db_path.to_str().expect("utf-8 db path");
    let ledger = Ledger::open(url).await?;
    bootstrap::chart_of_accounts(&ledger).await?;

    // Inception ts — 365 days before `period_end`.  We tag the ledger
    // with an opening-balance memo at this ts so `ledger_inception_ts()`
    // returns it when the orchestrator queries the SQLite MIN(ts).
    let inception = period_end.inner().saturating_sub(time::Duration::days(365));
    let inception_ts = Timestamp::new(inception);

    // Bootstrap memo at inception so realized_pnl_since works.
    insert_memo_txn(
        &ledger,
        "bootstrap:inception",
        rfc3339(inception_ts).as_str(),
    )
    .await?;
    // Opening-balance memo (preserves balance invariant; matches the v0
    // bootstrap convention used elsewhere in the workspace).
    journal::registry_event(&ledger, "Bootstrap", "initial seed", "{}").await?;

    let mut rng = ChaCha20Rng::seed_from_u64(FIXTURE_SEED);

    // ── Fills: 10/day × 365 days = 3,650 fills across 4 strategies ──
    let strategies = ["strat_a", "strat_b", "strat_c", "strat_d"];
    let symbols = ["BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT"];
    let mut day_cursor = inception_ts;
    let stride = time::Duration::minutes(144); // 1440 / 10 = 144 m between fills
    let mut fills_written = 0_usize;
    // Per-`(symbol, strategy)` group state: the last Buy qty + lot
    // index counter.  We must keep every group long-only at all times
    // (Q8 / `audit::query::open_positions_at` raises on net-negative
    // qty), so each Sell matches the qty of the immediately-preceding
    // Buy in the same group.  The original side-alternation scheme
    // (side = `fills_written.is_multiple_of(2)`) interacted with the
    // 4-cycle group index to yield Sell-only groups for two of the
    // four `(symbol, strategy)` pairs — that violates the long-only
    // invariant the new T1003 unrealized-P&L reader requires.
    use std::collections::BTreeMap;
    let mut group_state: BTreeMap<(&'static str, &'static str), (Decimal, usize)> = BTreeMap::new();
    while day_cursor <= period_end && fills_written < 3650 {
        for n in 0_i32..10 {
            let ts = Timestamp::new(day_cursor.inner() + stride * n);
            if ts > period_end {
                break;
            }
            let strategy = strategies[fills_written % strategies.len()];
            let symbol = symbols[fills_written % symbols.len()];
            // Random qty in [0.001, 0.05] BTC-scale; price in
            // [10_000, 90_000] USDT-scale.  Decimal-only — no f64.
            let qty_micro: u64 = rng.random_range(1_000..50_000);
            let qty_drawn = Decimal::from(qty_micro) / dec!(1_000_000);
            let price_int: u64 = rng.random_range(10_000..90_000);
            let price = Decimal::from(price_int);
            let fee_int: u64 = rng.random_range(1..100);
            let fee = Decimal::from(fee_int) / dec!(100); // [0.01, 1.00]
                                                          // Side alternates Buy/Sell **within the same
                                                          // (symbol, strategy) group** so half the fills are Sells
                                                          // (feeds `pnl_by_strategy`'s realized-P&L join) AND every
                                                          // Sell zeroes the running qty rather than driving it
                                                          // negative.
            let entry = group_state
                .entry((symbol, strategy))
                .or_insert((qty_drawn, 0));
            let (qty, side) = if entry.1.is_multiple_of(2) {
                // Even lot index → open the lot with a Buy.  Cache the
                // drawn qty so the next Sell in this group can mirror it.
                entry.0 = qty_drawn;
                (qty_drawn, Side::Buy)
            } else {
                // Odd lot index → close the lot with a Sell of EXACTLY
                // the prior Buy's qty.  This keeps the group's running
                // qty walking 0 → +qty → 0 (long-only at all times).
                (entry.0, Side::Sell)
            };
            entry.1 += 1;
            let fill = Fill {
                id: FillId(deterministic_uuid(rng.random::<u64>())),
                order_id: OrderId(deterministic_uuid(rng.random::<u64>())),
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
            journal::post_fill(&ledger, &fill, Some(strategy)).await?;
            fills_written += 1;
        }
        day_cursor = Timestamp::new(day_cursor.inner() + time::Duration::days(1));
    }

    // ── Strategy events: ~3/week (Load/Swap/Unload), 52 weeks → ~156 ──
    let mut week_cursor = inception_ts;
    let week_step = time::Duration::days(7);
    let event_kinds = ["Load", "Swap", "Unload"];
    let mut events_written = 0_usize;
    while week_cursor <= period_end && events_written < 156 {
        for k in 0..3 {
            let ts = Timestamp::new(week_cursor.inner() + time::Duration::hours(k * 8));
            if ts > period_end {
                break;
            }
            let kind = event_kinds[events_written % event_kinds.len()];
            let strategy = strategies[events_written % strategies.len()];
            let ts_str = rfc3339_micros(ts);
            journal::strategy_event(
                &ledger,
                &journal::StrategyEventWrite {
                    kind,
                    strategy_id: Some(strategy),
                    old_hash: Some("aabbccdd"),
                    new_hash: Some("eeff0011"),
                    source_path: "config/strategies/fixture.toml",
                    operator: "system",
                    error_code: None,
                    error_summary: None,
                    ts: Some(&ts_str),
                },
            )
            .await?;
            events_written += 1;
        }
        week_cursor = Timestamp::new(week_cursor.inner() + week_step);
    }

    // ── Funding observations: 200 evenly-spaced over the year ──
    // 365 days / 200 ≈ 1.825 days per observation.
    let funding_step = time::Duration::hours(43); // ~1.79 days
    let mut funding_cursor = inception_ts;
    let mut funding_written = 0_usize;
    while funding_cursor <= period_end && funding_written < 200 {
        let symbol = symbols[funding_written % symbols.len()];
        let rate_int: i64 = rng.random_range(-200..200);
        let rate = Decimal::from(rate_int) / dec!(1_000_000); // ±0.0002
        let obs = FundingObs {
            symbol: Symbol::new(symbol),
            funding_rate: rate,
            funding_ts: funding_cursor,
            next_funding_ts: Timestamp::new(funding_cursor.inner() + time::Duration::hours(8)),
            poll_ts: Timestamp::new(funding_cursor.inner() + time::Duration::seconds(1)),
        };
        journal::insert_funding_obs(&ledger, &obs).await?;
        funding_cursor = Timestamp::new(funding_cursor.inner() + funding_step);
        funding_written += 1;
    }

    // ── Kill-switch trips: 8 events at deterministic offsets over the year ──
    // Use the lower-level `strategy_event` writer with a fixed ts so the
    // fixture stays deterministic (the public `kill_switch_tripped` API
    // hard-codes `now_utc()` which would defeat seeded reproducibility).
    let trip_offsets_days = [30_i64, 75, 120, 165, 210, 255, 300, 345];
    for (idx, off) in trip_offsets_days.iter().enumerate() {
        let ts = Timestamp::new(inception_ts.inner() + time::Duration::days(*off));
        if ts > period_end {
            break;
        }
        let ts_str = rfc3339_micros(ts);
        let reason = if idx % 2 == 0 {
            "clock_skew"
        } else {
            "data_feed_stall"
        };
        journal::strategy_event(
            &ledger,
            &journal::StrategyEventWrite {
                kind: "KillSwitchTripped",
                strategy_id: None,
                old_hash: None,
                new_hash: None,
                source_path: "",
                operator: "kill_switch",
                error_code: Some("kill_switch_tripped"),
                error_summary: Some(reason),
                ts: Some(&ts_str),
            },
        )
        .await?;
    }

    // ── Open + close one uptime interval at the start of the year ──
    // `system_health::compute_uptime_pct` reads these to compute
    // R7's uptime cell; one interval covering the whole year keeps the
    // body's R7 numerator non-zero without inflating the row count.
    let boot_id = Uuid::new_v4().to_string();
    let boot_ts = rfc3339_micros(inception_ts);
    let stop_ts = rfc3339_micros(period_end);
    journal::open_uptime_interval(&ledger, &boot_id, Some(&boot_ts)).await?;
    journal::close_uptime_interval(&ledger, &boot_id, Some(&stop_ts)).await?;

    Ok((inception_ts, period_end))
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// RFC-3339 second precision (matches `journal_transactions.ts` rows
/// produced by `post_fill` / `registry_event`).
fn rfc3339(ts: Timestamp) -> String {
    ts.inner()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 format")
}

/// RFC-3339 6-digit microsecond precision (matches `strategy_events.ts`
/// rows — see `journal::strategy_event`).
fn rfc3339_micros(ts: Timestamp) -> String {
    let fmt = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z",
    )
    .expect("rfc3339 micros format parse");
    ts.inner().format(&fmt).expect("rfc3339 micros format")
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
/// `ledger_inception_ts()` returns the fixture's intended inception
/// (rather than the wall-clock ts the `bootstrap` memo would emit).
async fn insert_memo_txn(
    ledger: &Ledger,
    description: &str,
    ts_str: &str,
) -> Result<(), LedgerError> {
    let txn_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO journal_transactions (id, ts, description) VALUES (?, ?, ?)")
        .bind(&txn_id)
        .bind(ts_str)
        .bind(description)
        .execute(ledger.pool())
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    Ok(())
}

// Dead-code suppression — fixture is used only by the perf smoke test;
// the test compiler complains about "unused" symbols when it picks the
// file up via `#[path]`.  The `#[allow(dead_code)]` attributes below
// keep the build clean.

#[allow(dead_code)]
pub fn epoch_anchor() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::days(20_000))
}
