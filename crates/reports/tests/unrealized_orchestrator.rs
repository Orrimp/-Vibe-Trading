#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1006 — V2 (positive path): orchestrator integration produces the
//! architect-mandated `+200.00 USDT` unrealized P&L when both open
//! positions resolve through the [`FrozenMarkSource`].
//!
//! Per `spec/tasks/real-mtm-unrealized-pnl.md` → T1006 acceptance and
//! `spec/features/real-mtm-unrealized-pnl.md` → V2:
//!
//! Open `build_ledger_with_open_positions_7d` (T1004) — 12 closed
//! (Buy, Sell) pairs + 2 dangling Buys at day 6 hour 20:
//!
//!   - `(strat_alpha, BTCUSDT, qty=0.01, price=60_000)`
//!   - `(strat_beta,  ETHUSDT, qty=0.20, price=3_000)`
//!
//! Frozen marks at `period_end`: `BTCUSDT @ 70_000` + `ETHUSDT @ 3_500`.
//!
//! Hand-computed (architect Design § Q5):
//!
//!   - BTC: `0.01 * (70_000 − 60_000) = +100.00 USDT`
//!   - ETH: `0.20 * ( 3_500 −  3_000) = +100.00 USDT`
//!   - `Σ unrealized = +200.00 USDT`
//!
//! `generate(...)` is run end-to-end; the body's R11 reconciliation
//! appendix is parsed for the headline-row Ledger-side cell and asserted
//! against `+200`.  The mark-unavailable footnote is asserted absent
//! (every position resolves).
//!
//! V2 sibling (mark miss / V6) lives at
//! `crates/reports/tests/mark_unavailable_warns.rs`; both files share
//! the T1004 fixture via `#[path = "fixtures/..."]`.

use reports::{FrozenMarkSource, ReportWindow, render::reconciliation::MARK_UNAVAILABLE_FOOTNOTE};
use tempfile::TempDir;

#[path = "fixtures/build_ledger_with_open_positions_7d.rs"]
mod build_ledger_with_open_positions_7d;

use crate::build_ledger_with_open_positions_7d::{
    FIXTURE_SEED, build_ledger_with_open_positions_7d, fixture_period_end, fixture_period_start,
};

/// Slice off the front-matter and return the body bytes — same
/// convention as `scripts/hash_report.py` and the anchor regression
/// gate.  Identical to the helper in `t1003_orchestrator_smoke.rs`;
/// duplicated here to keep T1006 standalone (different test target).
fn body_after_fence(full: &str) -> &str {
    let Some(rest) = full.strip_prefix("---\n") else {
        return full;
    };
    let close_marker = "\n---\n";
    rest.find(close_marker)
        .map_or("", |pos| &rest[pos + close_marker.len()..])
}

/// Synthesize a `FrozenMarkSource` CSV in the shape its loader
/// expects (header `symbol,close_time,close`, `close_time` in unix
/// millis).  The fixture's `frozen_marks_csv()` helper uses an older
/// RFC-3339 schema that does not match the loader's contract; we
/// inline the millis here so V2 is self-contained.
fn marks_csv(start_ms: i64, end_ms: i64) -> String {
    format!(
        "symbol,close_time,close\n\
         BTCUSDT,{start_ms},60000\n\
         BTCUSDT,{end_ms},70000\n\
         ETHUSDT,{start_ms},3000\n\
         ETHUSDT,{end_ms},3500\n",
    )
}

/// Parse the headline reconciliation row (R11 identity #1) out of the
/// rendered body and return the Ledger-side cell as a `Decimal`.  Row
/// format is `| identity | report | ledger | Δ | Pass? |`.
fn parse_headline_ledger_value(body: &str) -> rust_decimal::Decimal {
    let row = body
        .lines()
        .find(|l| l.contains("headline_return = realized + unrealized"))
        .expect("headline reconciliation row present in R11 appendix");
    let cells: Vec<&str> = row.split('|').map(str::trim).collect();
    // Layout after split: [empty, identity, report, ledger, delta, pass, empty]
    assert!(
        cells.len() >= 6,
        "headline reconciliation row must have ≥5 inner cells; got {cells:?}",
    );
    cells[3]
        .parse()
        .expect("ledger cell parses as rust_decimal::Decimal")
}

// ── V2 — positive: open positions + resolving marks → unrealized = +200 ──

#[tokio::test]
async fn t1006_v2_unrealized_equals_200_usdt() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit-v2-unrealized.db");
    let _ = build_ledger_with_open_positions_7d(&db_path)
        .await
        .expect("build T1004 fixture");

    let start_ms = fixture_period_start().unix_millis();
    let end_ms = fixture_period_end().unix_millis();
    let csv_body = marks_csv(start_ms, end_ms);
    let frozen = FrozenMarkSource::from_csv_str(&csv_body).expect("frozen marks parse");

    let out_path = dir.path().join("report.md");
    reports::generate(
        // Use `Since(period_start)` so the fixture's fixed RFC-3339 window
        // matches the resolved `[period_start, period_end]` range.  Same
        // pattern as the T1003 orchestrator smoke and the `report_scenarios`
        // anchor harness.
        ReportWindow::Since(fixture_period_start()),
        &db_path,
        &frozen,
        &out_path,
        Some(FIXTURE_SEED),
    )
    .await
    .expect("generate should succeed on a balanced fixture with all marks resolved");

    let full = std::fs::read_to_string(&out_path).expect("read rendered report");
    let body = body_after_fence(&full);

    // Body must carry the R11 reconciliation appendix.
    assert!(
        body.contains("## Reconciliation"),
        "body should carry the R11 reconciliation appendix",
    );

    // V2 acceptance: the headline reconciliation row's Ledger-side cell
    // (= `realized + unrealized`) MUST equal `+200`.  Every closed
    // (Buy, Sell) pair in the fixture nets to ZERO realized P&L (Buy and
    // Sell at the same price), so `realized = 0` and the row reduces to
    // `0 + unrealized = unrealized`.  Per the architect-mandated marks
    // (BTC@70_000, ETH@3_500) and the dangling Buys (BTC@60_000 qty=0.01,
    // ETH@3_000 qty=0.20), the hand-computed `Σ unrealized = +200.00 USDT`.
    let ledger_val = parse_headline_ledger_value(body);
    assert_eq!(
        ledger_val,
        rust_decimal_macros::dec!(200),
        "T1006 V2 expected `+200.00 USDT` (BTC +100 + ETH +100); got {ledger_val}",
    );

    // Mark-unavailable footnote MUST NOT appear when every open-position
    // mark resolved cleanly — the footnote is gated on `mark_misses > 0`,
    // and Q6 contract requires it absent on the all-resolved branch.
    assert!(
        !body.contains(MARK_UNAVAILABLE_FOOTNOTE),
        "T1006 V2 expected no mark-unavailable footnote when every \
         open-position mark resolves; got body containing the footnote \
         literal `{MARK_UNAVAILABLE_FOOTNOTE}`.\n\nbody:\n{body}",
    );

    // Reconciliation FAIL would have routed `Err(ReportError::Reconciliation
    // { .. })` out of `generate(...)`; reaching this point implies a clean
    // PASS reconciliation.  Mirror the front-matter contract for clarity.
    assert!(
        full.contains("reconciliation: PASS"),
        "front-matter should record a PASS reconciliation",
    );
}
