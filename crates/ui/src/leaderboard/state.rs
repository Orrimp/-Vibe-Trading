//! advisor-leaderboard-screen v0.1.0 — Leaderboard-screen per-session state.
//!
//! Step 3 of the single-coin investment-advisor journey (product § journey:
//! pick coin + budget → bake off → **rank & pick** → plan → watch). This
//! module holds the bake-off result the screen renders, mirrored into a
//! pure-`ui` shape exactly as `lab::runner::RunReportMirror` mirrors
//! `backtest::RunReport`.
//!
//! ## The mirror discipline (INVARIANT)
//!
//! `ui` must NOT import `strategy` / `exec` / `forecast` / `llm`. The
//! bake-off result type (`backtest::BakeoffReport`) is consumed through the
//! **existing `backtest` dep** — the same sanctioned seam the Lab runner uses
//! for `RunReport`. We mirror it into [`BakeoffReportMirror`] / [`LeaderRow`]
//! /[`RecommendationMirror`] at the dispatch boundary so the screen renders
//! plain `String` / `Decimal` / `f64` fields and never threads an engine type
//! through `view`. This keeps the render code trivially `ui`-pure and makes
//! the leaderboard rows unit-constructible in fixtures + render tests without
//! standing up the whole engine.
//!
//! [`LeaderboardScreenState`] mirrors `ReportsScreenState` /
//! `BaselineScreenState` one-for-one — the established list-detail / result
//! screen shape, with the result behind a [`PanelState`] (Loading / Empty /
//! Error / Ready) so there is never a blank screen.

use rust_decimal::Decimal;
use smol_str::SmolStr;

use crate::state::PanelState;

/// One leaderboard row — a single candidate strategy's bake-off outcome,
/// mirrored from `backtest::CandidateResult` into render-ready fields.
///
/// Free of every engine type: the `strategy` id is a plain `SmolStr`
/// (display string), the KPIs are `Decimal` / `f64` / `usize`. The equity
/// curve is intentionally NOT mirrored here — the leaderboard table renders
/// scalars only; the per-candidate sparkline is a later polish (F-followup).
#[derive(Debug, Clone, PartialEq)]
pub struct LeaderRow {
    /// Strategy display id, e.g. `"v0.sma"` or `"v0.buyhold"`.
    pub strategy: SmolStr,
    /// `true` for the buy-and-hold benchmark arm — drives the "(benchmark)"
    /// row label so the operator always sees the passive baseline plainly.
    pub is_benchmark: bool,
    /// Annualised Sharpe (hourly). The primary ranking metric.
    pub sharpe: f64,
    /// Total return fraction (`0.1` = +10 %). Rendered as a sentiment-coloured
    /// percentage.
    pub total_return_pct: Decimal,
    /// Max drawdown fraction (`0.0` = no drawdown). Rendered as a `DOWN_500`
    /// percentage.
    pub max_drawdown: Decimal,
    /// Number of executed trades (buys + sells).
    pub trade_count: usize,
    /// Robustness flag mirrored as a display string, `None` when the gate was
    /// not run (`RobustnessMode::Skip`). One of `"robust"` / `"marginal"` /
    /// `"fragile"` / `"not checked"`.
    pub robustness: Option<RobustnessLabel>,
}

/// Robustness verdict mirrored from `backtest::RobustnessFlag` into a closed
/// UI-side enum (so the screen never matches on an engine type, and the
/// FRAGILE-warning copy is driven by a `ui`-owned discriminant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobustnessLabel {
    /// Survived resampling — p5 Sharpe ≥ 0 with margin.
    Robust,
    /// Borderline under resampling.
    Marginal,
    /// p5 Sharpe < 0 under block-bootstrap resampling — cannot be crowned
    /// unless every candidate is fragile (the credibility gate).
    Fragile,
    /// The robustness gate was intentionally not run for this candidate.
    NotChecked,
}

/// Which honesty branch the recommendation fired — mirrored from
/// `backtest::RecommendationOutcome`. Drives the headline sentence the screen
/// renders (the UI owns the copy; the engine owns the data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeKind {
    /// An active strategy was crowned and beat the benchmark.
    ActiveWins,
    /// Buy-and-hold was crowned ("nothing beat simply holding").
    BenchmarkWins,
    /// Every candidate looked fragile under resampling.
    AllFragile,
}

/// A single supporting reason, mirrored from `backtest::ReasonCode` into a
/// closed UI-side enum the screen maps to one line of sub-copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasonLabel {
    /// Crowned on Sharpe among non-fragile arms.
    HighestRobustSharpe,
    /// Winner Sharpe beat the benchmark's Sharpe.
    BeatBenchmarkSharpe,
    /// No active arm beat buy-and-hold.
    BenchmarkUndefeated,
    /// The robustness gate found nothing robust.
    AllCandidatesFragile,
    /// A Sharpe tie was resolved by the higher total return.
    TieBrokenByReturn,
    /// A Sharpe + return tie was resolved by the lower max drawdown.
    TieBrokenByDrawdown,
}

/// The structured recommendation, mirrored from `backtest::Recommendation`.
///
/// The screen turns this into a plain-language headline + supporting reasons.
/// No pre-rendered string crosses the seam — the copy (and the mandatory
/// not-advice disclaimer) live in `crate::strings`.
#[derive(Debug, Clone, PartialEq)]
pub struct RecommendationMirror {
    /// Which honesty branch fired (drives the headline).
    pub outcome: OutcomeKind,
    /// The crowned strategy display id.
    pub winner: SmolStr,
    /// The crowned strategy's robustness label (echoed for the "…and it's
    /// robust/fragile" clause), `None` when the gate was not run.
    pub winner_robustness: Option<RobustnessLabel>,
    /// The supporting reason codes, in deterministic order.
    pub reasons: Vec<ReasonLabel>,
}

/// The full bake-off result the leaderboard screen renders, mirrored from
/// `backtest::BakeoffReport` into render-ready shape.
///
/// `ranked` is the best-first display order (indices into `rows`); `crowned`
/// is the index of the recommendation. The `coin` + `range_label` echo the
/// request so the screen can say "…over this window for BTCUSDT".
#[derive(Debug, Clone, PartialEq)]
pub struct BakeoffReportMirror {
    /// The coin every candidate was run on, e.g. `"BTCUSDT"` (echoed for copy).
    pub coin: SmolStr,
    /// Human-readable lookback label, e.g. `"2024 H1"` (echoed for copy).
    pub range_label: SmolStr,
    /// Per-candidate rows, in *insertion* order (= field order, benchmark last).
    pub rows: Vec<LeaderRow>,
    /// Indices into `rows`, best-first, per the ranking comparator.
    pub ranked: Vec<usize>,
    /// Index of the crowned pick (`None` only when there are zero rows).
    pub crowned: Option<usize>,
    /// The structured recommendation.
    pub recommendation: RecommendationMirror,
}

impl BakeoffReportMirror {
    /// Build the mirror from a `backtest::BakeoffReport`.
    ///
    /// This is the ONLY place an engine `BakeoffReport` is read; everything
    /// downstream (state, view, fixtures, render tests) works on the mirror.
    /// Pure + total (no I/O, no panic).
    #[must_use]
    pub fn from_report(report: &backtest::BakeoffReport) -> Self {
        let rows = report
            .candidates
            .iter()
            .map(|c| LeaderRow {
                strategy: SmolStr::new(c.strategy.0.as_str()),
                is_benchmark: c.is_benchmark,
                sharpe: c.kpis.sharpe,
                total_return_pct: c.kpis.total_return_pct,
                max_drawdown: c.kpis.max_drawdown,
                trade_count: c.kpis.trade_count,
                robustness: c.robustness.map(robustness_label),
            })
            .collect();

        let r = &report.rationale;
        let recommendation = RecommendationMirror {
            outcome: outcome_kind(r.outcome),
            winner: SmolStr::new(r.winner.0.as_str()),
            winner_robustness: r.winner_robustness.map(robustness_label),
            reasons: r.reasons.iter().copied().map(reason_label).collect(),
        };

        Self {
            coin: SmolStr::new(report.request.symbol.0.as_str()),
            range_label: SmolStr::new(range_label_for(&report.request.range)),
            rows,
            ranked: report.ranked.clone(),
            crowned: report.crowned,
            recommendation,
        }
    }

    /// The crowned row, if any (the `crowned` index resolved against `rows`).
    #[must_use]
    pub fn crowned_row(&self) -> Option<&LeaderRow> {
        self.rows.get(self.crowned?)
    }
}

/// Map a `backtest::RobustnessFlag` to the UI-side label.
fn robustness_label(flag: backtest::RobustnessFlag) -> RobustnessLabel {
    match flag {
        backtest::RobustnessFlag::Robust => RobustnessLabel::Robust,
        backtest::RobustnessFlag::Marginal => RobustnessLabel::Marginal,
        backtest::RobustnessFlag::Fragile => RobustnessLabel::Fragile,
        backtest::RobustnessFlag::Skipped => RobustnessLabel::NotChecked,
    }
}

/// Map a `backtest::RecommendationOutcome` to the UI-side kind.
fn outcome_kind(outcome: backtest::RecommendationOutcome) -> OutcomeKind {
    match outcome {
        backtest::RecommendationOutcome::ActiveWins => OutcomeKind::ActiveWins,
        backtest::RecommendationOutcome::BenchmarkWins => OutcomeKind::BenchmarkWins,
        backtest::RecommendationOutcome::AllFragile => OutcomeKind::AllFragile,
    }
}

/// Map a `backtest::ReasonCode` to the UI-side label.
fn reason_label(code: backtest::ReasonCode) -> ReasonLabel {
    match code {
        backtest::ReasonCode::HighestRobustSharpe => ReasonLabel::HighestRobustSharpe,
        backtest::ReasonCode::BeatBenchmarkSharpe => ReasonLabel::BeatBenchmarkSharpe,
        backtest::ReasonCode::BenchmarkUndefeated => ReasonLabel::BenchmarkUndefeated,
        backtest::ReasonCode::AllCandidatesFragile => ReasonLabel::AllCandidatesFragile,
        backtest::ReasonCode::TieBrokenByReturn => ReasonLabel::TieBrokenByReturn,
        backtest::ReasonCode::TieBrokenByDrawdown => ReasonLabel::TieBrokenByDrawdown,
    }
}

/// Human-readable label for a `backtest::DateRange` (echoed into the headline
/// so the operator sees the window plainly, e.g. "2024 H1"). Kept here (not in
/// `strings`) because it is a 1:1 enum→label mapping, not operator copy.
fn range_label_for(range: &backtest::engine::DateRange) -> &'static str {
    use backtest::engine::DateRange;
    match range {
        DateRange::Last30d => "last 30 days",
        DateRange::Last90d => "last 90 days",
        DateRange::H1_2024 => "2024 H1",
        DateRange::H2_2024 => "2024 H2",
        DateRange::Custom { .. } => "custom window",
    }
}

/// Per-session Leaderboard-screen state. Sibling of `ReportsScreenState`.
///
/// `Default` = `result: PanelState::Empty` (no bake-off run yet — the cold
/// "press Run bake-off" surface) and `running: false`. A `Run bake-off`
/// action flips `result` to `Loading` (and `running` to `true`); the async
/// completion lands `Ready(mirror)` or `Error(msg)`.
#[derive(Debug, Clone)]
pub struct LeaderboardScreenState {
    /// The bake-off result.
    ///
    /// - `Empty` — cold start, no run yet (the "press Run bake-off" prompt).
    /// - `Loading` — a bake-off is in flight (spinner + "running…" copy).
    /// - `Ready(mirror)` — the ranked leaderboard + recommendation.
    /// - `Error(msg)` — the run failed (operator-friendly reason + retry).
    pub result: PanelState<BakeoffReportMirror>,
    /// Whether a bake-off is currently in flight (mirrors the Lab
    /// `lab_run_inflight` token). Guards against double-dispatch — the Run
    /// button is disabled while `true`.
    pub running: bool,
}

impl Default for LeaderboardScreenState {
    fn default() -> Self {
        Self {
            // Cold start is the honest Empty state (no run yet), NOT Loading —
            // Loading is reserved for an in-flight bake-off so the spinner only
            // shows when work is actually happening.
            result: PanelState::Empty,
            running: false,
        }
    }
}

impl LeaderboardScreenState {
    /// Mark a bake-off as started — flips `result` to `Loading` + `running` to
    /// `true`. Called from the `Message::BakeoffRunRequested` update arm.
    pub fn begin_run(&mut self) {
        self.result = PanelState::Loading;
        self.running = true;
    }

    /// Land a completed bake-off result. `Ok(mirror)` → `Ready`; `Err(msg)` →
    /// `Error`. Always clears `running`. Called from the
    /// `Message::BakeoffRunCompleted` update arm.
    pub fn finish_run(&mut self, outcome: Result<BakeoffReportMirror, SmolStr>) {
        self.running = false;
        self.result = match outcome {
            Ok(mirror) if mirror.rows.is_empty() => PanelState::Empty,
            Ok(mirror) => PanelState::Ready(mirror),
            Err(msg) => PanelState::Error(msg),
        };
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn row(strategy: &str, is_benchmark: bool, sharpe: f64) -> LeaderRow {
        LeaderRow {
            strategy: SmolStr::new(strategy),
            is_benchmark,
            sharpe,
            total_return_pct: dec!(0.05),
            max_drawdown: dec!(0.10),
            trade_count: 12,
            robustness: None,
        }
    }

    fn ready_mirror() -> BakeoffReportMirror {
        BakeoffReportMirror {
            coin: SmolStr::new("BTCUSDT"),
            range_label: SmolStr::new("2024 H1"),
            rows: vec![row("v0.sma", false, 1.2), row("v0.buyhold", true, 0.4)],
            ranked: vec![0, 1],
            crowned: Some(0),
            recommendation: RecommendationMirror {
                outcome: OutcomeKind::ActiveWins,
                winner: SmolStr::new("v0.sma"),
                winner_robustness: None,
                reasons: vec![ReasonLabel::HighestRobustSharpe],
            },
        }
    }

    #[test]
    fn default_is_empty_not_running() {
        let s = LeaderboardScreenState::default();
        assert!(
            matches!(s.result, PanelState::Empty),
            "cold start must be Empty (the press-Run prompt), not Loading"
        );
        assert!(!s.running, "no run in flight at cold start");
    }

    #[test]
    fn begin_run_sets_loading_and_running() {
        let mut s = LeaderboardScreenState::default();
        s.begin_run();
        assert!(matches!(s.result, PanelState::Loading));
        assert!(s.running);
    }

    #[test]
    fn finish_run_ok_lands_ready_and_clears_running() {
        let mut s = LeaderboardScreenState::default();
        s.begin_run();
        s.finish_run(Ok(ready_mirror()));
        assert!(matches!(s.result, PanelState::Ready(_)));
        assert!(!s.running, "running must clear on completion");
    }

    #[test]
    fn finish_run_empty_rows_lands_empty_not_ready() {
        let mut s = LeaderboardScreenState::default();
        s.begin_run();
        let mut m = ready_mirror();
        m.rows.clear();
        m.ranked.clear();
        m.crowned = None;
        s.finish_run(Ok(m));
        assert!(
            matches!(s.result, PanelState::Empty),
            "a zero-row result is the Empty surface, never a blank Ready table"
        );
    }

    #[test]
    fn finish_run_err_lands_error_and_clears_running() {
        let mut s = LeaderboardScreenState::default();
        s.begin_run();
        s.finish_run(Err(SmolStr::new("corpus missing")));
        match &s.result {
            PanelState::Error(e) => assert_eq!(e.as_str(), "corpus missing"),
            other => panic!("expected Error, got {}", other.variant_name()),
        }
        assert!(!s.running);
    }

    #[test]
    fn crowned_row_resolves_index() {
        let m = ready_mirror();
        assert_eq!(m.crowned_row().map(|r| r.strategy.as_str()), Some("v0.sma"));
        let mut none = m;
        none.crowned = None;
        assert!(none.crowned_row().is_none());
    }
}
