#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1006 — V6 (negative path) **warn-capture** half: orchestrator
//! integration with a [`FrozenMarkSource`] that omits `ETHUSDT`
//! exercises the architect Design § Q6 contract on mark-source miss.
//!
//! This file isolates the `tracing::subscriber::with_default(...)`-based
//! warn-capture test into its **own integration-test binary** so cargo's
//! per-binary process isolation guarantees a clean
//! `tracing::Dispatch` cache on the test thread. The companion body
//! footnote literal assertion lives in
//! `mark_unavailable_warns_footnote.rs` (separate binary, separate
//! process, no shared dispatcher cache). See the V6 flake analysis in
//! `evidence/reports/test-2026-05-02-2113-real-mtm-unrealized-pnl-final.md`
//! § 3 for the root-cause writeup that motivated this split.
//!
//! The contract being asserted (architect Design § Q6):
//!
//! 1. `tracing::warn!(symbol, ts, "mark unavailable for open position")`
//!    fires once per missed lookup.
//! 2. The position contributes [`Decimal::ZERO`] (no propagation, no
//!    silent fallback to a stale price).
//! 3. The R11 reconciliation appendix gains a deterministic body
//!    footnote (`MARK_UNAVAILABLE_FOOTNOTE`) signalling that at least
//!    one mark missed.
//! 4. `generate(...)` does NOT return `Err(ReportError::Marks(..))` —
//!    the run completes and the operator gets a partial-but-honest
//!    report.

use std::sync::{Arc, Mutex};

use reports::{FrozenMarkSource, ReportWindow, render::reconciliation::MARK_UNAVAILABLE_FOOTNOTE};
use tempfile::TempDir;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;

#[path = "fixtures/build_ledger_with_open_positions_7d.rs"]
mod build_ledger_with_open_positions_7d;

use crate::build_ledger_with_open_positions_7d::{
    FIXTURE_SEED, build_ledger_with_open_positions_7d, fixture_period_end, fixture_period_start,
};

// ── tracing capture wiring ────────────────────────────────────────────────────

/// One captured `tracing::warn!` event reduced to the fields V6 cares
/// about: the human-readable message body and the `symbol` attribute.
#[derive(Debug, Clone)]
struct CapturedWarn {
    /// `tracing` event message (the literal string passed after the
    /// structured fields, e.g. `"mark unavailable for open position"`).
    message: String,
    /// `symbol = %pos.symbol` field value (rendered via `Display`).
    symbol: Option<String>,
}

/// `tracing::field::Visit` implementation that records the `message`
/// and `symbol` fields off a captured event into a [`CapturedWarn`].
#[derive(Default)]
struct WarnVisitor {
    message: Option<String>,
    symbol: Option<String>,
}

impl Visit for WarnVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => self.message = Some(value.to_string()),
            "symbol" => self.symbol = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        // `tracing::warn!(symbol = %pos.symbol, ...)` records via
        // `record_debug` for non-primitive Display values — the
        // formatter writes the rendered Display form into a Debug
        // adapter inside `tracing`.  Capture both the formatted-debug
        // string and (if it's wrapped in quotes by `Debug` of a
        // newtype) strip the surrounding quotes so the captured value
        // is the bare ticker (`ETHUSDT`) rather than `"ETHUSDT"`.
        let formatted = format!("{value:?}");
        let cleaned = formatted.trim_matches('"').to_string();
        match field.name() {
            "message" => self.message = Some(cleaned),
            "symbol" => self.symbol = Some(cleaned),
            _ => {}
        }
    }
}

/// Custom `tracing_subscriber::Layer` that pushes every WARN-level
/// event whose message contains the architect-locked
/// `"mark unavailable for open position"` substring into a shared
/// `Vec<CapturedWarn>`.  Filters by message body so unrelated WARN
/// events from other code paths don't pollute the V6 assertion.
struct CaptureLayer {
    sink: Arc<Mutex<Vec<CapturedWarn>>>,
}

impl<S> Layer<S> for CaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if *event.metadata().level() != tracing::Level::WARN {
            return;
        }
        let mut visitor = WarnVisitor::default();
        event.record(&mut visitor);
        let Some(message) = visitor.message else {
            return;
        };
        if !message.contains("mark unavailable for open position") {
            return;
        }
        if let Ok(mut sink) = self.sink.lock() {
            sink.push(CapturedWarn {
                message,
                symbol: visitor.symbol,
            });
        }
    }
}

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

// ── V6 — negative: warn fires + zero contribution + Ok return ────────────────

#[test]
fn t1006_v6_mark_miss_warns_and_zeroes() {
    // Drive `reports::generate(...)` against the T1004 fixture under a
    // mark source that only carries BTCUSDT.  The orchestrator's
    // open-positions loop (T1003 / lib.rs:148 onward) calls
    // `MarkSource::close_at(ETHUSDT, period_end)` which returns
    // `MarkError::OutOfRange` → Q6 branch fires:
    //   1. `tracing::warn!(symbol = %pos.symbol, ts = %period_end,
    //      "mark unavailable for open position")` once for ETHUSDT.
    //   2. ETHUSDT contributes Decimal::ZERO to `unrealized`.
    //   3. BTCUSDT resolves to 70_000 → contribution = +100.
    //   4. `mark_misses == 1` → footnote toggled (asserted in V6 sibling).
    //   5. `generate(...)` returns `Ok(ReportArtifacts)` — the run does
    //      NOT propagate the OutOfRange error.
    //
    // Implementation note: this is a `#[test]` (NOT `#[tokio::test]`)
    // because `tracing::subscriber::with_default` installs a thread-
    // scoped dispatch — we need to OWN the tokio runtime build inside
    // the dispatch scope so every poll of the orchestrator future
    // inherits the capturing dispatch.
    //
    // Stabilization note (2026-05-02): this test lives in its own
    // integration-test binary (`mark_unavailable_warns_capture.rs`)
    // separate from `mark_unavailable_warns_footnote.rs`, because
    // cargo runs each `tests/*.rs` binary in its own process. That
    // process isolation guarantees the `tracing::Dispatch` thread-
    // local cache is fresh on the capture thread — eliminating the
    // dispatcher-cache race documented in the V6 flake analysis.

    let sink: Arc<Mutex<Vec<CapturedWarn>>> = Arc::new(Mutex::new(Vec::new()));
    let layer = CaptureLayer {
        sink: Arc::clone(&sink),
    };
    let subscriber = tracing_subscriber::registry().with(layer);

    let (outcome, body) = tracing::subscriber::with_default(subscriber, || {
        // Build a current-thread tokio runtime INSIDE the dispatch
        // scope — every `block_on`-driven poll of the orchestrator
        // future therefore runs under our capture layer.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build tokio current-thread runtime");
        rt.block_on(async {
            let dir = TempDir::new().expect("tempdir");
            let db_path = dir.path().join("audit-v6.db");
            let _ = build_ledger_with_open_positions_7d(&db_path)
                .await
                .expect("build T1004 fixture");

            let start_ms = fixture_period_start().unix_millis();
            let end_ms = fixture_period_end().unix_millis();
            let csv_body = marks_csv_btc_only(start_ms, end_ms);
            let frozen = FrozenMarkSource::from_csv_str(&csv_body).expect("frozen marks parse");

            let out_path = dir.path().join("report.md");
            let outcome = reports::generate(
                ReportWindow::Since(fixture_period_start()),
                &db_path,
                &frozen,
                &out_path,
                Some(FIXTURE_SEED),
            )
            .await;

            // Read the body BEFORE tempdir drops so the post-capture
            // assertions can run against the rendered report.
            let body = std::fs::read_to_string(&out_path).unwrap_or_default();
            (outcome, body)
        })
    });

    let captured: Vec<CapturedWarn> = sink.lock().expect("capture sink mutex").clone();

    // (a) `generate(...)` must NOT propagate `MarkError::OutOfRange`.
    let _artifacts = outcome.expect(
        "T1006 V6 contract: generate(...) must return Ok on a mark miss \
         per Q6; OutOfRange must be swallowed (warn + zero + footnote), \
         not propagated.",
    );

    // (b) Exactly one warn event matching the architect-locked message.
    // The fixture has exactly one ETHUSDT open position at period_end;
    // the marks CSV omits ETHUSDT so the orchestrator loops once into
    // the `Err(MarkError::OutOfRange { .. })` arm and emits exactly one
    // matching warn.  BTCUSDT resolves cleanly → no warn from that arm.
    assert_eq!(
        captured.len(),
        1,
        "T1006 V6 contract: expected exactly ONE \
         `mark unavailable for open position` warn (one per missed \
         open-position mark; the fixture has 1 ETHUSDT open position \
         and the marks CSV omits ETHUSDT); got {} events: {:#?}",
        captured.len(),
        captured,
    );

    // (c) The captured event MUST carry `symbol = ETHUSDT` (BTCUSDT
    // resolves cleanly so it does not warn).
    let only = &captured[0];
    let symbol = only
        .symbol
        .as_deref()
        .expect("captured warn carries a `symbol` field");
    assert!(
        symbol.contains("ETHUSDT"),
        "T1006 V6 contract: captured warn must reference \
         `symbol = ETHUSDT` (the omitted symbol); got `{symbol}` in \
         message `{}`",
        only.message,
    );

    // (d) Sanity — the body should also surface the deterministic
    // footnote (the load-bearing architect-locked literal is asserted
    // verbatim in the V6 sibling test in
    // `mark_unavailable_warns_footnote.rs`; here we just smoke that
    // the body wasn't empty / truncated by a generate error).
    assert!(
        body.contains(MARK_UNAVAILABLE_FOOTNOTE),
        "T1006 V6 sanity: body should carry MARK_UNAVAILABLE_FOOTNOTE \
         when the warn fires",
    );
}
