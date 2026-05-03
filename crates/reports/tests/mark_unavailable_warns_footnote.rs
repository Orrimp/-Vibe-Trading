#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1006 — V6 (negative path) **body footnote literal** half: asserts
//! the rendered report body contains the architect-locked
//! `MARK_UNAVAILABLE_FOOTNOTE` literal verbatim, plus a forward-compat
//! guard `assert_eq!` on the constant itself.
//!
//! Lives in its own integration-test binary (separate from
//! `mark_unavailable_warns_capture.rs`) so cargo's per-binary process
//! isolation guarantees the warn-capture sibling cannot pollute the
//! `tracing::Dispatch` thread-local cache before the capture layer is
//! installed. See the V6 flake analysis in
//! `spec/reports/test-2026-05-02-2113-real-mtm-unrealized-pnl-final.md`
//! § 3 for the root-cause writeup that motivated this split.

use reports::{render::reconciliation::MARK_UNAVAILABLE_FOOTNOTE, FrozenMarkSource, ReportWindow};
use tempfile::TempDir;

#[path = "fixtures/build_ledger_with_open_positions_7d.rs"]
mod build_ledger_with_open_positions_7d;

use crate::build_ledger_with_open_positions_7d::{
    build_ledger_with_open_positions_7d, fixture_period_end, fixture_period_start, FIXTURE_SEED,
};

// ── frozen marks: BTCUSDT covered, ETHUSDT omitted ────────────────────────────

/// Build a `FrozenMarkSource` CSV body with BTC marks present and ETH
/// marks deliberately omitted at every timestamp — exercises Q6's
/// mark-source miss branch for the ETHUSDT open position only.  The
/// loader expects header `symbol,close_time,close` with `close_time` in
/// unix millis; same shape used by the T1003 orchestrator smoke.
fn marks_csv_btc_only(start_ms: i64, end_ms: i64) -> String {
    format!(
        "symbol,close_time,close\n\
         BTCUSDT,{start_ms},60000\n\
         BTCUSDT,{end_ms},70000\n",
    )
}

// ── V6 — body footnote literal ───────────────────────────────────────────────

#[tokio::test]
async fn t1006_v6_footnote_present_when_miss() {
    // Independent of the tracing-capture machinery — drives the
    // orchestrator under the standard tokio test runtime and asserts
    // the rendered body contains the architect-locked literal footnote
    // string verbatim.  Falsifies any drift in `MARK_UNAVAILABLE_FOOTNOTE`
    // (which T1003 pinned as the source of truth at
    // `crates/reports/src/render/reconciliation.rs:21`).

    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit-v6-footnote.db");
    let _ = build_ledger_with_open_positions_7d(&db_path)
        .await
        .expect("build T1004 fixture");

    let start_ms = fixture_period_start().unix_millis();
    let end_ms = fixture_period_end().unix_millis();
    let csv_body = marks_csv_btc_only(start_ms, end_ms);
    let frozen = FrozenMarkSource::from_csv_str(&csv_body).expect("frozen marks parse");

    let out_path = dir.path().join("report.md");
    reports::generate(
        ReportWindow::Since(fixture_period_start()),
        &db_path,
        &frozen,
        &out_path,
        Some(FIXTURE_SEED),
    )
    .await
    .expect("generate must return Ok on a mark miss (Q6: warn + zero, no propagation)");

    let full = std::fs::read_to_string(&out_path).expect("read rendered report");

    // The architect-locked literal string MUST appear in the body
    // verbatim.  The constant is pinned at
    // `crates/reports/src/render/reconciliation.rs:21` and is exactly
    // `*one or more open-position marks were unavailable at period_end;
    //  see logs*`.  Any divergence in the literal breaks the V6 contract.
    assert!(
        full.contains(MARK_UNAVAILABLE_FOOTNOTE),
        "T1006 V6 contract: body must contain the deterministic \
         footnote literal `{MARK_UNAVAILABLE_FOOTNOTE}` when at least \
         one open position's mark is unavailable.\n\nfull report:\n{full}",
    );

    // Belt-and-suspenders: the footnote literal must match
    // architect's exact wording.  Keep this assertion as a guard
    // against accidental edits to the constant on the source side
    // (the constant lives in `crates/reports/src/render/reconciliation.rs`
    // — owned by T1003; T1006 must not silently accept a drift).
    assert_eq!(
        MARK_UNAVAILABLE_FOOTNOTE,
        "*one or more open-position marks were unavailable at period_end; see logs*",
        "T1006 V6 forward-compat guard: MARK_UNAVAILABLE_FOOTNOTE must \
         remain the architect-locked Q6 literal.  Any change requires \
         architect approval (changes the rendered body shape).",
    );
}
