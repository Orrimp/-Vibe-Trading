#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]
//! T1106 — V3 + V7 verification gates for the per-symbol-position-accounts
//! feature (`spec/features/per-symbol-position-accounts.md`).
//!
//! Per the architect's task spec (`spec/tasks/per-symbol-position-accounts.md`
//! → T1106) and Design § "Test strategy (per V-item)":
//!
//! - **V3 — `open_positions_at` correct on mixed pre/post ledgers.**
//!   Build the T1104 mixed fixture (`build_ledger_mixed_legacy_and_per_symbol_7d`)
//!   which contains 2 pre-006 "legacy" rows (BTCUSDT + ETHUSDT both writing
//!   to the literal `assets:position:BTC` account via raw SQL) AND 1 post-006
//!   row (SOLUSDT written via `audit::journal::post_fill` which targets the
//!   per-pair `assets:position:SOLUSDT` account). Call
//!   `audit::query::open_positions_at(&ledger, period_end)` and assert the
//!   returned `Vec<OpenPosition>` contains exactly 3 rows
//!   (BTCUSDT, ETHUSDT, SOLUSDT) with correct
//!   `(qty, avg_cost_basis, opened_at, strategy_id)` tuples. Sort order
//!   follows the T1002 `(symbol ASC, strategy_id ASC, None last)` invariant
//!   (alphabetical by symbol — BTCUSDT before ETHUSDT before SOLUSDT).
//! - **V7 — Determinism.** Two consecutive `open_positions_at(...)` calls
//!   on the same opened ledger return `Vec<OpenPosition>` slices that
//!   compare equal byte-for-byte via `assert_eq!` (R10 inherited from
//!   real-mtm, widened to a mixed-shape fixture).
//!
//! The Q4 cross-check warn-emit is NOT explicitly asserted here — it is an
//! observation-only path (`tracing::warn!` rather than a hard error), and
//! the task spec line 486 says: "for V3 + V7 the assertions on
//! `Vec<OpenPosition>` shape are sufficient".
//!
//! Fixture imports follow the same `#[path = "fixtures/..."]` pattern T1004
//! established (so the audit-side T1005 and the reports-side T1106 share a
//! single source of fixture truth).

use audit::{Ledger, bootstrap, query};
use rust_decimal_macros::dec;
use tempfile::tempdir;
use trading_core::{Money, OpenPosition, StrategyId, Symbol};

#[path = "fixtures/build_ledger_with_open_positions_7d.rs"]
mod fixture;

use fixture::{build_ledger_mixed_legacy_and_per_symbol_7d, parse_rfc3339};

/// V3 — `open_positions_at` correct on a mixed legacy/post-migration ledger.
///
/// Mixed fixture row plan (T1104, see
/// `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs`):
///
/// | Row              | Side | Symbol  | Qty  | Price  | account_id              | strategy_id        |
/// |------------------|------|---------|------|--------|-------------------------|--------------------|
/// | Legacy raw-SQL   | Buy  | BTCUSDT | 1.0  | 60_000 | `assets:position:BTC`   | `NULL`             |
/// | Legacy raw-SQL   | Buy  | ETHUSDT | 5.0  |  2_500 | `assets:position:BTC`   | `NULL`             |
/// | Post-006 `post_fill` | Buy | SOLUSDT | 10.0 |    100 | `assets:position:SOLUSDT` | `Some("test_strategy")` |
///
/// Expected `Vec<OpenPosition>` after sort (R6 — `(symbol ASC,
/// strategy_id ASC, None last)`):
///   - row 0 — BTCUSDT, qty 1.0,  avg_cost_basis 60_000, opened 2026-04-27T19:00:00Z, strategy_id None
///   - row 1 — ETHUSDT, qty 5.0,  avg_cost_basis  2_500, opened 2026-04-27T19:00:01Z, strategy_id None
///   - row 2 — SOLUSDT, qty 10.0, avg_cost_basis    100, opened 2026-04-27T19:00:02Z, strategy_id Some("test_strategy")
#[tokio::test]
async fn t1106_v3_mixed_ledger_correct_open_positions() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("t1106-v3.sqlite");
    let url = db_path.to_str().expect("utf-8 db path");

    // `Ledger::open` runs every migration on open, including 006 (which
    // seeds `assets:position:SOLUSDT`); `chart_of_accounts` ensures the
    // legacy `assets:position:BTC` row exists for the raw-SQL inserts
    // (per the fixture's caller contract).
    let ledger = Ledger::open(url).await.expect("open ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");

    let _entries = build_ledger_mixed_legacy_and_per_symbol_7d(&ledger)
        .await
        .expect("build T1104 mixed fixture");

    // Read at a period_end strictly after the latest mixed-fixture row
    // (`POST_SOL_FILL_RFC3339 = 2026-04-27T19:00:02Z`). Use the same
    // `PERIOD_END_RFC3339 = 2026-04-28T00:00:00Z` constant the existing
    // T1004 fixture uses for parity.
    let period_end = parse_rfc3339("2026-04-28T00:00:00Z");

    let positions = query::open_positions_at(&ledger, period_end)
        .await
        .expect("open_positions_at");

    assert_eq!(
        positions.len(),
        3,
        "mixed fixture should emit 3 open positions (BTCUSDT, ETHUSDT, \
         SOLUSDT) at period_end; got {}",
        positions.len()
    );

    // Hand-computed expected vec — built byte-identical-style so a future
    // field addition on `OpenPosition` flags this test by failing equality.
    let expected = vec![
        // Row 0 — BTCUSDT (alphabetical first, R6).
        OpenPosition {
            symbol: Symbol::new("BTCUSDT"),
            qty: dec!(1.0),
            avg_cost_basis: Money::from_decimal(dec!(60_000)),
            opened_at: parse_rfc3339("2026-04-27T19:00:00Z"),
            strategy_id: None,
        },
        // Row 1 — ETHUSDT (alphabetical second).
        OpenPosition {
            symbol: Symbol::new("ETHUSDT"),
            qty: dec!(5.0),
            avg_cost_basis: Money::from_decimal(dec!(2_500)),
            opened_at: parse_rfc3339("2026-04-27T19:00:01Z"),
            strategy_id: None,
        },
        // Row 2 — SOLUSDT (alphabetical third).
        OpenPosition {
            symbol: Symbol::new("SOLUSDT"),
            qty: dec!(10.0),
            avg_cost_basis: Money::from_decimal(dec!(100)),
            opened_at: parse_rfc3339("2026-04-27T19:00:02Z"),
            strategy_id: Some(StrategyId::new("test_strategy")),
        },
    ];

    assert_eq!(
        positions, expected,
        "byte-identical match against hand-computed mixed-ledger expected \
         vec (V3 — pre-006 legacy `assets:position:BTC` rows + post-006 \
         per-pair `assets:position:SOLUSDT` row)"
    );
}

/// V7 — Determinism: two consecutive reads return byte-identical `Vec`s.
///
/// `open_positions_at(&ledger, period_end)` is a pure SQL fold over
/// `journal_transactions` LEFT JOIN `journal_entries` — `BTreeMap`-keyed
/// accumulator, deterministic sort, `Decimal` arithmetic only (no `f64`),
/// no `SystemTime::now()` (architect Design § Determinism guardrails). Two
/// calls against the same opened mixed-ledger fixture must therefore yield
/// `Vec<OpenPosition>` slices that `assert_eq!` byte-for-byte.
#[tokio::test]
async fn t1106_v7_two_reads_byte_identical() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("t1106-v7.sqlite");
    let url = db_path.to_str().expect("utf-8 db path");

    let ledger = Ledger::open(url).await.expect("open ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");

    let _entries = build_ledger_mixed_legacy_and_per_symbol_7d(&ledger)
        .await
        .expect("build T1104 mixed fixture");

    let period_end = parse_rfc3339("2026-04-28T00:00:00Z");

    let first = query::open_positions_at(&ledger, period_end)
        .await
        .expect("open_positions_at #1");
    let second = query::open_positions_at(&ledger, period_end)
        .await
        .expect("open_positions_at #2");

    assert_eq!(
        first, second,
        "two consecutive reads of the mixed-ledger fixture must be \
         byte-identical (R10 — same content, same order, byte-for-byte)"
    );
    // Belt-and-braces: confirm the read returned the expected row count
    // (3 rows — BTCUSDT, ETHUSDT, SOLUSDT). Without this, a future
    // regression that returned an empty `Vec` from both reads would still
    // satisfy `assert_eq!(first, second)`.
    assert_eq!(
        first.len(),
        3,
        "determinism test must read 3 positions (a regression returning \
         empty `Vec` would otherwise pass byte-equality vacuously)"
    );
}
