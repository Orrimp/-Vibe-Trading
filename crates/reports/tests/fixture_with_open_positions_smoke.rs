#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1004 — smoke tests for the new
//! `build_ledger_with_open_positions_7d` fixture (test-only,
//! non-anchored; per architect Q5).
//!
//! These tests guard the fixture's row counts, ledger integrity, and
//! the open-position projection produced by `audit::query::open_positions_at`
//! (T1002, also landed parallel-Wave-1).  The full V1 / V4 / V7 assertions
//! land in T1005 (`crates/audit/tests/open_positions_at.rs`); this file is
//! the T1004 acceptance probe — it exists so the fixture has at least one
//! consumer that exercises the builder end-to-end and can fail loudly if
//! the row plan drifts.

use audit::{Ledger, query};
use tempfile::tempdir;

#[path = "fixtures/build_ledger_with_open_positions_7d.rs"]
mod build_ledger_with_open_positions_7d;

use build_ledger_with_open_positions_7d::build_ledger_with_open_positions_7d;

/// T1004 acceptance smoke — opens the fixture, asks `recent_fills` for
/// every row, asserts 14 = 12 closed + 2 open. Independent of T1002 (the
/// open-position reader) so it runs green today.
#[tokio::test]
async fn t1004_fixture_emits_two_open_positions() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("with-open-positions-7d.sqlite");
    let (_period_start, _period_end) = build_ledger_with_open_positions_7d(&db_path)
        .await
        .expect("build fixture");

    let url = db_path.to_str().expect("utf-8 db path");
    let ledger = Ledger::open(url).await.expect("re-open ledger");

    // 14 fills total: 12 closed (Buy, Sell) pairs + 2 dangling Buys.
    let fills = query::recent_fills(&ledger, usize::MAX)
        .await
        .expect("recent_fills");
    assert_eq!(
        fills.len(),
        14,
        "fixture should emit 14 fills (12 closed + 2 open); got {}",
        fills.len()
    );

    // Per-symbol fill counts: BTCUSDT has 3 closed (Buy, Sell) pairs (6
    // fills) + 1 dangling Buy = 7; ETHUSDT has 3 closed pairs (6 fills)
    // + 1 dangling Buy = 7. Both symbols carry exactly one open position
    // at period_end (the dangling Buy).
    let btc_fills = fills
        .iter()
        .filter(|f| f.symbol == trading_core::Symbol::new("BTCUSDT"))
        .count();
    let eth_fills = fills
        .iter()
        .filter(|f| f.symbol == trading_core::Symbol::new("ETHUSDT"))
        .count();
    assert_eq!(btc_fills, 7, "BTCUSDT fill count (6 closed + 1 dangling)");
    assert_eq!(eth_fills, 7, "ETHUSDT fill count (6 closed + 1 dangling)");
}

/// T1004 deterministic-rebuild probe — the fixture must be byte-identical
/// across two calls with the same seed.  Builds two separate SQLite files
/// and asserts their `recent_fills` projections match (length + by-row
/// `(symbol, side, qty, price)` tuples).
#[tokio::test]
async fn t1004_fixture_two_builds_byte_identical_fills() {
    let dir = tempdir().expect("tempdir");

    let db_a = dir.path().join("a.sqlite");
    let _ = build_ledger_with_open_positions_7d(&db_a)
        .await
        .expect("build a");
    let ledger_a = Ledger::open(db_a.to_str().expect("utf-8"))
        .await
        .expect("open a");
    let fills_a = query::recent_fills(&ledger_a, usize::MAX)
        .await
        .expect("fills a");

    let db_b = dir.path().join("b.sqlite");
    let _ = build_ledger_with_open_positions_7d(&db_b)
        .await
        .expect("build b");
    let ledger_b = Ledger::open(db_b.to_str().expect("utf-8"))
        .await
        .expect("open b");
    let fills_b = query::recent_fills(&ledger_b, usize::MAX)
        .await
        .expect("fills b");

    assert_eq!(fills_a.len(), fills_b.len(), "fill counts diverged");
    // `recent_fills` returns rows in deterministic order (DB ORDER BY
    // ts DESC + id), so equal seed → equal projection at every index.
    for (i, (fa, fb)) in fills_a.iter().zip(fills_b.iter()).enumerate() {
        assert_eq!(
            fa.symbol, fb.symbol,
            "row {i}: symbol diverged ({} vs {})",
            fa.symbol, fb.symbol
        );
        assert_eq!(fa.side, fb.side, "row {i}: side diverged");
        assert_eq!(fa.qty, fb.qty, "row {i}: qty diverged");
        assert_eq!(fa.price, fb.price, "row {i}: price diverged");
        assert_eq!(fa.venue_ts, fb.venue_ts, "row {i}: venue_ts diverged");
    }
}

/// T1004 smoke against the T1002 reader — calls
/// `audit::query::open_positions_at(&ledger, period_end)` and asserts
/// exactly 2 `OpenPosition` rows in deterministic alphabetical order
/// (BTCUSDT before ETHUSDT, R6).  This is a thin smoke; the full
/// byte-for-byte V1 assertions (qty, avg_cost_basis, opened_at,
/// strategy_id) live in T1005 at `crates/audit/tests/open_positions_at.rs`.
#[tokio::test]
async fn t1004_fixture_has_expected_open_positions_at_period_end() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("with-open-positions-7d-smoke.sqlite");
    let (_period_start, period_end) = build_ledger_with_open_positions_7d(&db_path)
        .await
        .expect("build fixture");

    let url = db_path.to_str().expect("utf-8 db path");
    let ledger = Ledger::open(url).await.expect("re-open ledger");

    let positions = query::open_positions_at(&ledger, period_end)
        .await
        .expect("open_positions_at");

    assert_eq!(
        positions.len(),
        2,
        "fixture should emit 2 open positions at period_end (one BTCUSDT + \
         one ETHUSDT dangling Buy); got {}",
        positions.len()
    );
    // R6: alphabetical sort — BTCUSDT before ETHUSDT.
    assert_eq!(
        positions[0].symbol,
        trading_core::Symbol::new("BTCUSDT"),
        "first position should be BTCUSDT (alphabetical)"
    );
    assert_eq!(
        positions[1].symbol,
        trading_core::Symbol::new("ETHUSDT"),
        "second position should be ETHUSDT (alphabetical)"
    );
}
