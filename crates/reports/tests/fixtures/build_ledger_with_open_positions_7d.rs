#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1004 — 7-day fixture WITH open positions at `period_end`.
//!
//! **Test-only, NOT anchored.** This fixture exists to exercise the v1+
//! real mark-to-market unrealized-P&L code path (feature
//! `spec/features/real-mtm-unrealized-pnl.md`) — V1, V2, V7, V8. Anchored
//! fixtures (`build_ledger_7d.rs`, `build_ledger_90d.rs`) MUST stay
//! byte-identical (architect Q4 → 11/11 PASS) and are not modified by
//! this feature; this file is the third, non-anchored fixture per
//! architect Q5.
//!
//! Activity plan (architect-binding per `spec/tasks/real-mtm-unrealized-pnl.md`
//! → T1004):
//!
//! - Copies the 7d 12-fill plan: 6 perfectly-symmetric (Buy, Sell) pairs
//!   across `strat_alpha` / BTCUSDT + `strat_beta` / ETHUSDT (full closes
//!   BEFORE `period_end` — exercises the "skip net-zero groups" path).
//! - **Plus 2 dangling Buys at day 6 hour 20 with NO matching Sell:**
//!   `(strat_alpha, BTCUSDT, Side::Buy, qty=0.01, price=60_000)` and
//!   `(strat_beta, ETHUSDT, Side::Buy, qty=0.20, price=3_000)`. Both
//!   surface as `OpenPosition` rows at `period_end` — exercises the
//!   multi-symbol sort (BTCUSDT before ETHUSDT alphabetical, R6). The
//!   first dangling Buy on BTCUSDT lands in the same
//!   `(strat_alpha, BTCUSDT)` group that already saw 3 closed
//!   (Buy, Sell) pairs in the symmetric plan above — this exercises the
//!   Q7 weighted-average / proportional-release accounting:
//!   `running_qty` walks `0 → +qty → 0 → +qty → 0 → +qty → 0 → +0.01`,
//!   and `running_notional` arrives at exactly `0.01 * 60_000 = 600`
//!   after every Sell zeroed both accumulators.
//! - 14 fills total = 12 closed + 2 open.
//!
//! V2 hand-computed expected (per architect Design § Q5):
//!
//! - BTC: `0.01 * (70_000 − 60_000) = +100.00 USDT`.
//! - ETH: `0.20 * (3_500  −  3_000) = +100.00 USDT`.
//! - `Σ unrealized = +200.00 USDT`.
//!
//! Determinism: seed `0xC0FFEE` (consistent with sibling fixtures);
//! all timestamps fixed RFC-3339; `Decimal` only; deterministic UUIDs;
//! no `Uuid::new_v4`, no `thread_rng`. Two `build_ledger_with_open_positions_7d`
//! calls with the same seed produce byte-identical row inserts.

use std::path::Path;

use audit::{bootstrap, journal, query, Ledger};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{
    FeeTier, Fill, FillId, JournalEntryView, LedgerError, Liquidity, Money, OrderId, Price,
    Quantity, Side, Symbol, Timestamp,
};
use uuid::Uuid;

/// Deterministic seed shared by every fixture under T815/T816/T1004.
pub const FIXTURE_SEED: u64 = 0x00C0_FFEE;

/// Fixed `period_start` — same window as `build_ledger_7d` for parity.
pub const PERIOD_START_RFC3339: &str = "2026-04-21T00:00:00Z";

/// Fixed `period_end` — same window as `build_ledger_7d` for parity.
pub const PERIOD_END_RFC3339: &str = "2026-04-28T00:00:00Z";

/// Far-future RFC-3339 timestamp used to close the uptime interval.
pub const FAR_FUTURE_RFC3339: &str = "2099-12-31T23:59:59Z";

/// Mark price for BTCUSDT at `period_end` referenced by V2 (the
/// `FrozenMarkSource` ships from `frozen_marks_csv()` below).
#[allow(dead_code)]
pub const BTC_MARK_AT_PERIOD_END: Decimal = dec!(70_000);

/// Mark price for ETHUSDT at `period_end` referenced by V2.
#[allow(dead_code)]
pub const ETH_MARK_AT_PERIOD_END: Decimal = dec!(3_500);

/// Build the 7-day fixture WITH open positions at `db_path`.
///
/// Returns `(period_start, period_end)` so the caller can hand them to
/// the orchestrator as the explicit `Since` window without re-deriving
/// the timestamps. Total under `seed = FIXTURE_SEED` — two calls produce
/// byte-identical row inserts.
///
/// # Errors
///
/// Returns [`LedgerError`] propagated from `Ledger::open`,
/// `bootstrap::chart_of_accounts`, or any of the journal writers.
pub async fn build_ledger_with_open_positions_7d(
    db_path: &Path,
) -> Result<(Timestamp, Timestamp), LedgerError> {
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
        deterministic_uuid(0xDD01),
    )
    .await?;

    // RNG used only for fee jitter; keeps every row deterministic under
    // FIXTURE_SEED while exercising the BTC/USDT cash-flow path.
    let mut rng = ChaCha20Rng::seed_from_u64(FIXTURE_SEED);

    // ── Strategy lifecycle: two Loads (the open-position fixture is
    //    intentionally narrower than build_ledger_7d's three-strategy plan
    //    because R8 / what-changed coverage is owned by the anchored
    //    fixtures; this fixture's job is open-position projection alone). ─
    let load_ts = day_offset_micros(0, 1);
    for sid in ["strat_alpha", "strat_beta"] {
        journal::strategy_event(
            &ledger,
            &journal::StrategyEventWrite {
                kind: "Load",
                strategy_id: Some(sid),
                old_hash: None,
                new_hash: Some("aabbccddeeff0011"),
                source_path: "config/strategies/sample-with-open-7d.toml",
                operator: "system",
                error_code: None,
                error_summary: None,
                ts: Some(&load_ts),
                venue: None,
            },
        )
        .await?;
    }

    // ── Fills: 12 closed + 2 open = 14 total ───────────────────────────────
    let fills = build_fill_plan(&mut rng);
    for (strategy, fill) in &fills {
        journal::post_fill(&ledger, fill, Some(strategy)).await?;
    }

    // ── Uptime interval covering the full report window ────────────────────
    let boot_id = "boot-with-open-7d-fixed-uuid-cccc";
    journal::open_uptime_interval(&ledger, boot_id, Some(PERIOD_START_RFC3339)).await?;
    journal::close_uptime_interval(&ledger, boot_id, Some(FAR_FUTURE_RFC3339)).await?;

    Ok((period_start, period_end))
}

/// Plan of `(strategy_id, Fill)` tuples — fixed under `FIXTURE_SEED`.
///
/// First 12 rows mirror `build_ledger_7d`'s symmetric plan exactly (so
/// every (Buy, Sell) pair zeroes its `(symbol, strategy_id)` group at
/// `period_end`).  Last 2 rows are dangling Buys at day 6 hour 20 — they
/// remain open at `period_end` and are the rows that V1/V2 assert against.
fn build_fill_plan(rng: &mut ChaCha20Rng) -> Vec<(&'static str, Fill)> {
    use rand::Rng;

    // 12 closed pairs (identical to build_ledger_7d so the exact
    // symmetric path is exercised) + 2 dangling Buys at day 6 hour 20.
    let plan: [(&'static str, &'static str, Side, i64, u32); 14] = [
        // ── 12 closed (Buy, Sell) pairs — identical to build_ledger_7d ──
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
        // ── 2 dangling Buys at day 6 hour 20 (NO matching Sell) ─────────
        ("strat_alpha", "BTCUSDT", Side::Buy, 6, 20),
        ("strat_beta", "ETHUSDT", Side::Buy, 6, 20),
    ];

    let mut out = Vec::with_capacity(plan.len());
    let mut id_counter: u64 = 0xF333_0000;
    for (strategy, symbol, side, day, hour) in plan {
        // Fee jitter draws from rng — keeps the rng cursor stepping in
        // step with the sibling fixtures' fee-jitter pattern (same path).
        let fee_int: u64 = rng.random_range(1..100);
        let fee = Decimal::from(fee_int) / dec!(100);
        // (qty, price) selection rule:
        //   - Closed-pair rows use the same (qty, price) as build_ledger_7d
        //     so the symmetric Buy/Sell pairs net to zero per group.
        //   - Dangling Buy rows use the architect-mandated (qty, price)
        //     from spec/features/real-mtm-unrealized-pnl.md § Q5:
        //       BTCUSDT: qty=0.01, price=60_000.
        //       ETHUSDT: qty=0.20, price=3_000.
        //   The dangling rows are uniquely identified by (day=6, hour=20)
        //   — the symmetric plan never lands a fill at that offset.
        let is_dangling = day == 6 && hour == 20;
        let (qty, price) = if is_dangling {
            match symbol {
                "BTCUSDT" => (dec!(0.01), dec!(60_000)),
                _ => (dec!(0.20), dec!(3_000)),
            }
        } else {
            match symbol {
                "BTCUSDT" => (dec!(0.005), dec!(60_000)),
                _ => (dec!(0.10), dec!(3_000)),
            }
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

/// CSV body covering BTCUSDT + ETHUSDT marks at both `period_start` and
/// `period_end` — fed to a `FrozenMarkSource::from_csv_str(...)` by V2 so
/// `MarkSource::close_at(symbol, period_end)` resolves cleanly.
///
/// V2 hand-computed expected:
///   - BTC: `0.01 * (70_000 − 60_000) = +100.00 USDT`.
///   - ETH: `0.20 * (3_500  −  3_000) = +100.00 USDT`.
///   - `Σ unrealized = +200.00 USDT`.
#[allow(dead_code)]
pub fn frozen_marks_csv() -> &'static str {
    "symbol,ts,close\n\
     BTCUSDT,2026-04-21T00:00:00Z,60000\n\
     BTCUSDT,2026-04-28T00:00:00Z,70000\n\
     ETHUSDT,2026-04-21T00:00:00Z,3000\n\
     ETHUSDT,2026-04-28T00:00:00Z,3500\n"
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
// downstream test files (T1005, T1006, T1007). Keeps the build clean.

#[allow(dead_code)]
pub fn fixture_period_start() -> Timestamp {
    parse_rfc3339(PERIOD_START_RFC3339)
}

#[allow(dead_code)]
pub fn fixture_period_end() -> Timestamp {
    parse_rfc3339(PERIOD_END_RFC3339)
}

// ── T1104 — Mixed legacy/new ledger fixture ────────────────────────────────────
//
// Adds a small, deterministic ledger that mixes pre-006 ("legacy") rows with
// post-006 ("per-pair") rows. Used by T1106 V3/V7 to exercise
// `audit::query::open_positions_at` across the migration boundary
// (per-symbol-position-accounts § Q6).

/// Day-6 hour-19 RFC-3339 timestamp for the first legacy fill (BTCUSDT).
const LEGACY_BTC_FILL_RFC3339: &str = "2026-04-27T19:00:00Z";
/// Day-6 hour-19 + 1 hour RFC-3339 timestamp for the second legacy fill (ETHUSDT).
const LEGACY_ETH_FILL_RFC3339: &str = "2026-04-27T19:00:01Z";
/// Day-6 hour-19 + 2 hours RFC-3339 timestamp for the post-006 fill (SOLUSDT).
const POST_SOL_FILL_RFC3339: &str = "2026-04-27T19:00:02Z";

/// Deterministic UUID seeds for the mixed fixture rows. Distinct from the
/// `0xF333_xxxx` family used by the closed/dangling plan above so the two
/// builders never collide on a primary key.
const MIXED_BTC_TXN_SEED: u64 = 0xC114_0101;
const MIXED_BTC_E1_SEED: u64 = 0xC114_0102;
const MIXED_BTC_E2_SEED: u64 = 0xC114_0103;
const MIXED_ETH_TXN_SEED: u64 = 0xC114_0201;
const MIXED_ETH_E1_SEED: u64 = 0xC114_0202;
const MIXED_ETH_E2_SEED: u64 = 0xC114_0203;
const MIXED_SOL_FILL_SEED: u64 = 0xC114_0301;
const MIXED_SOL_ORDER_SEED: u64 = 0xC114_0302;

/// Build a mixed legacy/new ledger that exercises V3 + V7 across the
/// migration boundary.
///
/// Pre-006 rows write the literal `assets:position:BTC` account regardless
/// of the underlying symbol (deliberate raw-SQL bypass of `post_fill` so the
/// fixture mimics a pre-T1102 ledger faithfully); post-006 rows write the
/// per-pair account-id via the updated `audit::journal::post_fill`.
///
/// Layout (in chronological order):
/// - **Legacy** BTCUSDT Buy `qty = 1.0 @ price = 60_000`, `assets:position:BTC`
///   account, no `strategy_id`.
/// - **Legacy** ETHUSDT Buy `qty = 5.0 @ price = 2_500`, `assets:position:BTC`
///   account, no `strategy_id`.
/// - **Post-006** SOLUSDT Buy `qty = 10.0 @ price = 100`, written via
///   `post_fill` with `strategy_id = Some("test_strategy")`. Lands on
///   `assets:position:SOLUSDT`.
///
/// Net effect at the period_end timestamp: three open positions
/// (BTCUSDT, ETHUSDT, SOLUSDT) — one per symbol.
///
/// **Determinism.** All UUIDs are seeded from compile-time constants
/// (no `Uuid::new_v4`). All timestamps are fixed RFC-3339 second-precision
/// strings (the same format `post_fill` writes). Two builds against
/// independent fresh ledgers produce byte-identical
/// `Vec<JournalEntryView>` projections (R10 / V7).
///
/// **Caller contract.** The caller MUST hand in a freshly-opened `Ledger`
/// for which `bootstrap::chart_of_accounts` has run (so the legacy
/// `assets:position:BTC` account exists for the raw-SQL inserts) AND
/// migration `006` has been applied (so the per-pair `assets:position:SOLUSDT`
/// row exists for the `post_fill` write). `Ledger::open` runs every
/// migration on open, so the standard test pattern `Ledger::open(...)` +
/// `chart_of_accounts(...)` satisfies both.
///
/// # Errors
///
/// Returns [`LedgerError`] propagated from raw `sqlx::query` execution
/// or from `audit::journal::post_fill` / `audit::query::recent_journal`.
#[allow(dead_code)]
pub async fn build_ledger_mixed_legacy_and_per_symbol_7d(
    ledger: &Ledger,
) -> Result<Vec<JournalEntryView>, LedgerError> {
    // 1) Pre-006 BTCUSDT legacy Buy — direct SQL, account_id = assets:position:BTC.
    insert_legacy_buy(
        ledger,
        deterministic_uuid(MIXED_BTC_TXN_SEED),
        deterministic_uuid(MIXED_BTC_E1_SEED),
        deterministic_uuid(MIXED_BTC_E2_SEED),
        LEGACY_BTC_FILL_RFC3339,
        "buy 1.0 BTCUSDT @ 60000",
        dec!(60_000),
    )
    .await?;

    // 2) Pre-006 ETHUSDT legacy Buy — direct SQL, account_id = assets:position:BTC.
    insert_legacy_buy(
        ledger,
        deterministic_uuid(MIXED_ETH_TXN_SEED),
        deterministic_uuid(MIXED_ETH_E1_SEED),
        deterministic_uuid(MIXED_ETH_E2_SEED),
        LEGACY_ETH_FILL_RFC3339,
        "buy 5.0 ETHUSDT @ 2500",
        dec!(12_500),
    )
    .await?;

    // 3) Post-006 SOLUSDT Buy via the updated `post_fill` — writes to the
    //    per-pair `assets:position:SOLUSDT` account (T1102 writer path).
    let sol_ts = parse_rfc3339(POST_SOL_FILL_RFC3339);
    let sol_fill = Fill {
        id: FillId(deterministic_uuid(MIXED_SOL_FILL_SEED)),
        order_id: OrderId(deterministic_uuid(MIXED_SOL_ORDER_SEED)),
        symbol: Symbol::new("SOLUSDT"),
        side: Side::Buy,
        qty: Quantity::new(dec!(10.0)).expect("qty ok"),
        price: Price::new(dec!(100)).expect("price ok"),
        fee: Money::from_decimal(dec!(0)),
        fee_tier: FeeTier::Taker,
        venue_ts: sol_ts,
        local_ts: sol_ts,
        liquidity: Liquidity::Taker,
        transaction_id: None,
    };
    journal::post_fill(ledger, &sol_fill, Some("test_strategy")).await?;

    // Project the resulting ledger as a `Vec<JournalEntryView>` for the
    // caller to assert against (T1106). `recent_journal` orders by
    // `ts DESC, rowid DESC` — deterministic across rebuilds.
    query::recent_journal(ledger, usize::MAX).await
}

/// Insert a legacy-shape pre-006 Buy as a balanced double-entry transaction:
/// `Dr assets:position:BTC notional` + `Cr assets:cash:USDT notional`.
///
/// **Why raw SQL?** Post-T1102 `post_fill` formats the account-id from
/// `fill.symbol`, so it can never reproduce the legacy "everything goes
/// to `assets:position:BTC`" shape. T1104 V3 needs that legacy shape to
/// exercise the reader's mixed-ledger path; raw SQL is the deliberate
/// bypass.
///
/// `description` matches the writer's `format!("{} {} {} @ {}", side, qty,
/// symbol, price)` shape so `open_positions_at`'s description-parse and
/// `pnl_by_symbol` continue to attribute the row to the correct symbol.
async fn insert_legacy_buy(
    ledger: &Ledger,
    txn_uuid: Uuid,
    debit_entry_uuid: Uuid,
    credit_entry_uuid: Uuid,
    ts_str: &str,
    description: &str,
    notional: Decimal,
) -> Result<(), LedgerError> {
    let txn_id = txn_uuid.to_string();

    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, strategy_id) \
         VALUES (?, ?, ?, NULL)",
    )
    .bind(&txn_id)
    .bind(ts_str)
    .bind(description)
    .execute(ledger.pool())
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    // Dr assets:position:BTC  notional  (legacy single-bucket account).
    sqlx::query(
        "INSERT INTO journal_entries \
         (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(debit_entry_uuid.to_string())
    .bind(&txn_id)
    .bind("assets:position:BTC")
    .bind(notional.to_string())
    .bind("0")
    .bind(ts_str)
    .execute(ledger.pool())
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    // Cr assets:cash:USDT     notional.
    sqlx::query(
        "INSERT INTO journal_entries \
         (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(credit_entry_uuid.to_string())
    .bind(&txn_id)
    .bind("assets:cash:USDT")
    .bind("0")
    .bind(notional.to_string())
    .bind(ts_str)
    .execute(ledger.pool())
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    Ok(())
}

// Smoke tests for this fixture live in a sibling integration test target —
// `crates/reports/tests/fixture_with_open_positions_smoke.rs` — which loads
// this file via `#[path = "..."]` and exercises the fixture builder.
// Inline `#[cfg(test)] mod tests` would not be picked up because cargo only
// auto-generates test binaries from top-level files in `tests/`, not from
// files re-mounted via `#[path]`.
