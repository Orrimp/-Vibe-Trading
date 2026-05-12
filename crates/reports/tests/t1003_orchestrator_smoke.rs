#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1003 — orchestrator integration smoke tests.
//!
//! Inline smoke that exercises the new mark-to-market unrealized-P&L
//! path inside `reports::generate(...)` per architect's Design § Q6
//! (warn + zero + body footnote) and § Q4 (anchors stay byte-identical
//! on empty-positions fixtures).
//!
//! The dedicated V1/V2/V6/V7/V8 tests (T1005, T1006, T1007) are owned
//! by Wave 2 and live in their own integration targets.  This file
//! contains the three minimal smokes mandated by T1003's task body:
//!
//! 1. `t1003_orchestrator_with_zero_open_positions_keeps_anchor_byte_identical`
//!    — runs `generate(...)` against the unmodified `build_ledger_7d`
//!    fixture; the body SHA-256 MUST equal the locked `report-sample-7d`
//!    anchor.  Falsifies any regression where the new open-positions
//!    code path leaks bytes (e.g. an empty footnote line, a stray
//!    whitespace shift) on the empty-positions branch — the load-bearing
//!    Q4 invariant of this feature.
//! 2. `t1003_orchestrator_with_open_positions_computes_unrealized`
//!    — runs `generate(...)` against the T1004 fixture
//!    (`build_ledger_with_open_positions_7d`) with a `FrozenMarkSource`
//!    that resolves both BTCUSDT@70_000 and ETHUSDT@3_500 at
//!    `period_end`; asserts `unrealized != 0` end-to-end and that the
//!    R11 reconciliation row reports the hand-computed `+200.00 USDT`.
//! 3. `t1003_orchestrator_handles_mark_miss` — same fixture but a
//!    `FrozenMarkSource` that omits both symbols → both lookups return
//!    `MarkError::OutOfRange` → `unrealized == 0` and the R11 footnote
//!    is present in the body.

use std::path::Path;

use reports::{render::reconciliation::MARK_UNAVAILABLE_FOOTNOTE, FrozenMarkSource, ReportWindow};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[path = "fixtures/build_ledger_7d.rs"]
mod build_ledger_7d;

#[path = "fixtures/build_ledger_with_open_positions_7d.rs"]
mod build_ledger_with_open_positions_7d;

use crate::build_ledger_7d::{
    build_ledger_7d, fixture_period_start as fixture_7d_period_start, FIXTURE_SEED,
};
use crate::build_ledger_with_open_positions_7d::{
    build_ledger_with_open_positions_7d, fixture_period_end as fixture_open_period_end,
    fixture_period_start as fixture_open_period_start,
};

/// Locked body-SHA256 for `report-sample-7d` (mirrors the entry in
/// `spec/anchors.toml` and `tests/report_scenarios.rs::EXPECTED_SHA_7D`).
/// The Q4 invariant — "anchors stay byte-identical" — is the load-bearing
/// claim of T1003; this constant pins it locally so the smoke fails
/// loudly rather than via the slower workspace anchor sweep.
// T1810 / T1813 — re-anchored post-reflection-memory renderer rewrite.
// T1935 / T1936 (v2-llm-strategy, pass 6) — re-anchored post-System-Health
// renderer rewrite (Q11 denominator $135 → $200 + Q5d Cache hit ratio row).
// Same value pinned in `crates/reports/tests/report_scenarios.rs::EXPECTED_SHA_7D`.
const EXPECTED_SHA_7D: &str = "520b1f2968ad52d5981a1cdb3749235416c77c058364bd8c11ebd7d2468f46a3";

/// Slice off the front-matter and return the body bytes — same
/// convention as `scripts/hash_report.py` and the anchor regression
/// gate.
fn body_after_fence(full: &str) -> &str {
    let Some(rest) = full.strip_prefix("---\n") else {
        return full;
    };
    let close_marker = "\n---\n";
    rest.find(close_marker)
        .map_or("", |pos| &rest[pos + close_marker.len()..])
}

fn body_sha256_hex(body: &str) -> String {
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    hex::encode(h.finalize())
}

/// Run `reports::generate` against a prepared fixture + frozen marks
/// and return the body bytes (post-front-matter, hashable slice).
async fn render_with_marks(
    db_path: &Path,
    out_path: &Path,
    period_start: trading_core::Timestamp,
    marks_csv: &str,
    seed: u64,
) -> String {
    let frozen = FrozenMarkSource::from_csv_str(marks_csv).expect("frozen marks parse");
    reports::generate(
        ReportWindow::Since(period_start),
        db_path,
        &frozen,
        out_path,
        Some(seed),
    )
    .await
    .expect("generate should succeed on a balanced fixture");
    let full = std::fs::read_to_string(out_path).expect("read rendered report");
    body_after_fence(&full).to_string()
}

/// Run `reports::generate` against the open-positions fixture and use
/// `Since(period_start)` so the report window matches the fixture's
/// fixed timestamps.  Returns the FULL body string.
async fn render_full_body(
    db_path: &Path,
    out_path: &Path,
    period_start: trading_core::Timestamp,
    marks_csv: &str,
) -> String {
    render_with_marks(
        db_path,
        out_path,
        period_start,
        marks_csv,
        crate::build_ledger_with_open_positions_7d::FIXTURE_SEED,
    )
    .await
}

// ── 1. Zero open positions → anchors byte-identical ───────────────────────────

#[tokio::test]
async fn t1003_orchestrator_with_zero_open_positions_keeps_anchor_byte_identical() {
    // Q4 invariant: `build_ledger_7d` has 12 perfectly symmetric (Buy,
    // Sell) pairs → `open_positions_at(period_end) = []` → the new
    // T1003 loop is a no-op → body bytes match the pre-T1003 path
    // exactly.  Falsified iff the orchestrator's new code path leaks a
    // stray whitespace, footnote line, or signed-zero on the
    // no-open-positions branch.
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit-7d.db");
    let _ = build_ledger_7d(&db_path).await.expect("build 7d fixture");

    let out_path = dir.path().join("report.md");
    let body = render_with_marks(
        &db_path,
        &out_path,
        fixture_7d_period_start(),
        // Empty mark source — same as `report_scenarios.rs`.  No open
        // positions in the fixture, so the loop never calls `close_at`.
        "symbol,close_time,close\n",
        FIXTURE_SEED,
    )
    .await;

    let sha = body_sha256_hex(&body);
    assert_eq!(
        sha, EXPECTED_SHA_7D,
        "T1003 anchor regression: body-SHA256 drifted on the \
         empty-positions branch.  The orchestrator's new \
         open-positions loop must emit ZERO new bytes when \
         `open_positions = []`. \n\nactual: {sha}\nexpected: \
         {EXPECTED_SHA_7D}\n\nlikely cause: footnote leak (empty \
         string concatenation), whitespace drift in render, or a \
         signed-zero `Decimal` regression.  Route HANDOFF → \
         architect with the body diff per task body.",
    );
    // Sanity: the footnote MUST NOT appear when no marks were missed.
    assert!(
        !body.contains(MARK_UNAVAILABLE_FOOTNOTE),
        "footnote must not appear when no open positions exist",
    );
}

// ── 2. Open positions + resolving marks → unrealized = +200 USDT ──────────────

#[tokio::test]
async fn t1003_orchestrator_with_open_positions_computes_unrealized() {
    // T1004 fixture: 2 dangling Buys at day 6 hour 20 →
    //   BTCUSDT @ avg_cost=60_000, qty=0.01
    //   ETHUSDT @ avg_cost=3_000,  qty=0.20
    // Hand-computed expected at marks (BTC@70_000, ETH@3_500):
    //   BTC: 0.01 × (70_000 − 60_000) = +100.00
    //   ETH: 0.20 × ( 3_500 −  3_000) = +100.00
    //   Σ unrealized = +200.00 USDT (matches Design § Q5 worked example).
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit-with-open-positions.db");
    let _ = build_ledger_with_open_positions_7d(&db_path)
        .await
        .expect("build open-positions fixture");

    // Compute unix-millis for period_start / period_end so we can write
    // a `FrozenMarkSource` CSV that matches the loader's schema (millis,
    // not RFC-3339).  The fixture's `frozen_marks_csv()` uses an
    // RFC-3339 + `ts` schema that pre-dates the FrozenMarkSource
    // contract; we synthesize the proper schema here so T1003's smoke
    // is independent of T1006's V2 fixture layout.
    let start_ms = fixture_open_period_start().unix_millis();
    let end_ms = fixture_open_period_end().unix_millis();
    let marks_csv = format!(
        "symbol,close_time,close\n\
         BTCUSDT,{start_ms},60000\n\
         BTCUSDT,{end_ms},70000\n\
         ETHUSDT,{start_ms},3000\n\
         ETHUSDT,{end_ms},3500\n",
    );

    let out_path = dir.path().join("report.md");
    let body = render_full_body(&db_path, &out_path, fixture_open_period_start(), &marks_csv).await;

    // The R11 reconciliation appendix carries the unrealized cell.
    // Render order: `headline_return = realized + unrealized` is row 1.
    // For build_ledger_with_open_positions_7d the symmetric (Buy, Sell)
    // pairs net to ZERO realized P&L (every Buy is matched by a Sell at
    // the same price), so `realized = 0` and the headline row reduces
    // to `0 + 200 = 200`.  Both report and ledger sides equal +200.
    assert!(
        body.contains("## Reconciliation"),
        "body should carry the R11 reconciliation appendix",
    );
    // Asserting the literal "200" in the unrealized cell would couple
    // the test to render-side formatting (decimal scale, sign glyph).
    // Instead, parse the headline row's Ledger-side value (column 3).
    let headline_row = body
        .lines()
        .find(|l| l.contains("headline_return = realized + unrealized"))
        .expect("headline reconciliation row present");
    // Row format: `| identity | report | ledger | Δ | Pass? |`
    let cells: Vec<&str> = headline_row.split('|').map(str::trim).collect();
    // Layout: [empty, identity, report, ledger, delta, pass, empty]
    assert!(
        cells.len() >= 6,
        "headline row must have ≥5 cells; got {cells:?}",
    );
    let ledger_cell = cells[3];
    let ledger_val: rust_decimal::Decimal =
        ledger_cell.parse().expect("ledger cell parses as Decimal");
    assert_eq!(
        ledger_val,
        rust_decimal_macros::dec!(200),
        "T1003 unrealized arithmetic: expected +200 USDT (BTC \
         +100 + ETH +100); got {ledger_val} from row `{headline_row}`",
    );
    // No mark misses → no footnote.
    assert!(
        !body.contains(MARK_UNAVAILABLE_FOOTNOTE),
        "footnote must not appear when every open-position mark resolves",
    );
}

// ── 3. Mark miss → unrealized = 0 + footnote present ──────────────────────────

#[tokio::test]
async fn t1003_orchestrator_handles_mark_miss() {
    // Q6 contract: when `MarkSource::close_at` returns `OutOfRange` for
    // an open position, the orchestrator (a) logs a warning, (b)
    // contributes Decimal::ZERO for that position, (c) toggles a
    // body-side footnote on the R11 appendix.  The arithmetic stays
    // invariant under mark-source health (determinism foot-gun avoided).
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit-mark-miss.db");
    let _ = build_ledger_with_open_positions_7d(&db_path)
        .await
        .expect("build open-positions fixture");

    // Empty mark source → BOTH BTCUSDT and ETHUSDT lookups return
    // `MarkError::OutOfRange { .. }` → both contribute 0 →
    // `mark_misses == 2` → footnote toggled on.
    let out_path = dir.path().join("report.md");
    let body = render_full_body(
        &db_path,
        &out_path,
        fixture_open_period_start(),
        "symbol,close_time,close\n",
    )
    .await;

    // Same headline row parse as test 2, but expecting Ledger-side == 0.
    let headline_row = body
        .lines()
        .find(|l| l.contains("headline_return = realized + unrealized"))
        .expect("headline reconciliation row present");
    let cells: Vec<&str> = headline_row.split('|').map(str::trim).collect();
    let ledger_cell = cells[3];
    let ledger_val: rust_decimal::Decimal =
        ledger_cell.parse().expect("ledger cell parses as Decimal");
    assert_eq!(
        ledger_val,
        rust_decimal::Decimal::ZERO,
        "T1003 mark-miss contract: every position contributes 0 → \
         unrealized == 0; got {ledger_val}",
    );

    // Footnote MUST be present in the body when at least one mark missed.
    assert!(
        body.contains(MARK_UNAVAILABLE_FOOTNOTE),
        "T1003 mark-miss contract: body must contain the deterministic \
         footnote `{MARK_UNAVAILABLE_FOOTNOTE}` when ≥1 position's mark \
         resolves to OutOfRange.\n\nbody:\n{body}",
    );
}
