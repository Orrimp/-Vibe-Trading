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
use trading_core::Symbol;

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

// ── F3 guided input — coin universe + lookback + budget ───────────────────────

/// The coin universe the guided-input coin picker offers — the symbols that
/// actually exist in the pinned Binance corpus (`data/binance/<SYM>/…`),
/// XRP-first to match the Lab's operator-locked scan order (product § journey
/// step 1: "pick a coin (e.g. XRPUSD)").
///
/// `&'static [&'static str]` (not `Symbol` — `Symbol` holds a `SmolStr`, which
/// is not `const`-constructible). The screen maps each to a `Symbol` at render
/// time. Kept here (next to the state it drives) rather than in the Lab's
/// `universe` module because the bake-off corpus set is its own contract — the
/// Lab universe is venue-tagged `(Venue, &str)` tuples, this is the flat coin
/// set the single-coin advisor ranks over.
pub const BAKEOFF_COIN_UNIVERSE: &[&str] = &[
    "XRPUSDT", "ETHUSDT", "BTCUSDT", "ADAUSDT", "AVAXUSDT", "BNBUSDT", "DOGEUSDT", "DOTUSDT",
    "LINKUSDT", "SOLUSDT",
];

/// The default coin the guided input starts on (product default + the v0.1.0
/// trigger's `BTCUSDT`). Kept in sync with `runner::DEFAULT_BAKEOFF_COIN`.
pub const DEFAULT_BAKEOFF_COIN: &str = "BTCUSDT";

/// The default budget the guided input starts on — €200 (product § journey
/// step 1: "a budget (e.g. €200)"). Stored as the raw input string so the
/// numeric field round-trips the operator's keystrokes verbatim.
pub const DEFAULT_BUDGET_INPUT: &str = "200";

/// Milliseconds in one calendar day — the relative-lookback arithmetic unit.
const MS_PER_DAY: i64 = 86_400_000;

/// A human lookback choice the guided input offers (product § journey step 1:
/// a configurable lookback "2 weeks → ~4 years"). Each maps to a
/// `backtest::engine::DateRange` **in the UI** (the relative ones to a
/// `Custom { now - N days, now }` window; the fixed 2024 presets pass through)
/// — the backtest crate is never edited.
///
/// A closed UI-side enum so the picker never matches on an engine type and the
/// chip labels are driven by a `ui`-owned discriminant (the same mirror
/// discipline the rest of this module follows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderboardLookback {
    /// ~2 weeks (14 days) to today.
    TwoWeeks,
    /// ~1 month (30 days) to today.
    OneMonth,
    /// ~3 months (90 days) to today.
    ThreeMonths,
    /// ~6 months (182 days) to today.
    SixMonths,
    /// ~1 year (365 days) to today.
    OneYear,
    /// ~2 years (730 days) to today.
    TwoYears,
    /// ~4 years (1460 days) to today.
    FourYears,
    /// Fixed preset — first half of 2024 (the v0.1.0 default; full corpus
    /// coverage, so the leaderboard always populates).
    H1_2024,
    /// Fixed preset — second half of 2024.
    H2_2024,
}

impl LeaderboardLookback {
    /// The lookback choices in the order the picker renders them — relative
    /// windows first (2 weeks → 4 years, the product's headline range), the
    /// two fixed corpus-covered 2024 presets last.
    pub const ALL: &'static [LeaderboardLookback] = &[
        LeaderboardLookback::TwoWeeks,
        LeaderboardLookback::OneMonth,
        LeaderboardLookback::ThreeMonths,
        LeaderboardLookback::SixMonths,
        LeaderboardLookback::OneYear,
        LeaderboardLookback::TwoYears,
        LeaderboardLookback::FourYears,
        LeaderboardLookback::H1_2024,
        LeaderboardLookback::H2_2024,
    ];

    /// The relative-window span in days, or `None` for the fixed 2024 presets.
    #[must_use]
    pub fn relative_days(self) -> Option<i64> {
        match self {
            LeaderboardLookback::TwoWeeks => Some(14),
            LeaderboardLookback::OneMonth => Some(30),
            LeaderboardLookback::ThreeMonths => Some(90),
            LeaderboardLookback::SixMonths => Some(182),
            LeaderboardLookback::OneYear => Some(365),
            LeaderboardLookback::TwoYears => Some(730),
            LeaderboardLookback::FourYears => Some(1460),
            LeaderboardLookback::H1_2024 | LeaderboardLookback::H2_2024 => None,
        }
    }

    /// Map this lookback to a `backtest::engine::DateRange`, computing the
    /// relative windows against `now_ms` (wall-clock UTC epoch-millis, passed
    /// in so the mapping stays pure + testable).
    ///
    /// Relative → `Custom { start_ms: now - N*86_400_000, end_ms: now }`;
    /// fixed presets pass through to the engine's named variants. The backtest
    /// crate is untouched — this is a pure UI-side enum→`DateRange` mapping.
    #[must_use]
    pub fn to_date_range(self, now_ms: i64) -> backtest::engine::DateRange {
        use backtest::engine::DateRange;
        // Exhaustive `match self` (no catch-all) — a new lookback variant fails
        // to compile until it's mapped here, so the UI→engine mapping can never
        // silently fall through. The relative arms share the same `Custom`
        // window shape via `relative_days()`; the two fixed presets pass through.
        match self.relative_days() {
            Some(days) => DateRange::Custom {
                start_ms: now_ms - days * MS_PER_DAY,
                end_ms: now_ms,
            },
            // `relative_days()` is `None` ⇒ a fixed preset (H1/H2 only).
            None => match self {
                LeaderboardLookback::H2_2024 => DateRange::H2_2024,
                // Every remaining variant with `relative_days() == None` is
                // `H1_2024` (the only other fixed preset) — mapped here.
                _ => DateRange::H1_2024,
            },
        }
    }
}

/// Parse a budget input string into a non-negative `Decimal` of euros.
///
/// Returns `Some(amount)` only when `s` parses to a finite `Decimal ≥ 0`
/// (a budget cannot be negative). Empty / non-numeric / negative returns
/// `None` → the screen shows the placeholder + the run uses the default. Pure;
/// no I/O. Accepts a leading `€` and surrounding whitespace so paste-from-copy
/// ("€200") round-trips.
#[must_use]
pub fn parse_budget(s: &str) -> Option<Decimal> {
    let trimmed = s.trim().trim_start_matches('\u{20ac}').trim();
    if trimmed.is_empty() {
        return None;
    }
    let amount = trimmed.parse::<Decimal>().ok()?;
    if amount.is_sign_negative() {
        None
    } else {
        Some(amount)
    }
}

/// Per-session Leaderboard-screen state. Sibling of `ReportsScreenState`.
///
/// `Default` = `result: PanelState::Empty` (no bake-off run yet — the cold
/// "press Run bake-off" surface), `running: false`, and the F3 guided-input
/// defaults (`BTCUSDT` / €200 / 2024 H1). A `Run bake-off` action flips
/// `result` to `Loading` (and `running` to `true`); the async completion lands
/// `Ready(mirror)` or `Error(msg)`.
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

    // ── F3 guided input ──────────────────────────────────────────────────────
    /// The coin the operator chose to rank strategies on (default `BTCUSDT`).
    /// Drives the bake-off `BakeoffRequest::symbol` and the budget-context
    /// header copy.
    pub coin: Symbol,
    /// The raw budget input string (round-trips the operator's keystrokes).
    /// Parsed to a `Decimal` via [`parse_budget`] for display; `None` parse →
    /// the default is shown. The bake-off RANKING does not use the budget
    /// (ranking is budget-independent); it carries forward to F4 (sizing) +
    /// F5 (paper-trade) and is SHOWN in the header for context.
    pub budget_input: String,
    /// The lookback window the operator chose (default `H1_2024`). Mapped to a
    /// `backtest::engine::DateRange` at dispatch time via
    /// [`LeaderboardLookback::to_date_range`].
    pub lookback: LeaderboardLookback,
}

impl Default for LeaderboardScreenState {
    fn default() -> Self {
        Self {
            // Cold start is the honest Empty state (no run yet), NOT Loading —
            // Loading is reserved for an in-flight bake-off so the spinner only
            // shows when work is actually happening.
            result: PanelState::Empty,
            running: false,
            // F3 guided-input defaults — the most-used starting point (product
            // § journey step 1: BTCUSDT / €200 / a corpus-covered window).
            coin: Symbol::new(DEFAULT_BAKEOFF_COIN),
            budget_input: DEFAULT_BUDGET_INPUT.to_string(),
            lookback: LeaderboardLookback::H1_2024,
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

    /// The parsed budget (euros), or `None` when the input is blank /
    /// unparseable / negative. The header + the down-stream sizing read this;
    /// the bake-off ranking does not.
    #[must_use]
    pub fn budget_eur(&self) -> Option<Decimal> {
        parse_budget(&self.budget_input)
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
    fn default_guided_input_is_btc_200_h1() {
        // F3 — the most-used starting point (product § journey step 1).
        let s = LeaderboardScreenState::default();
        assert_eq!(s.coin.0.as_str(), "BTCUSDT", "default coin is BTCUSDT");
        assert_eq!(s.budget_input, "200", "default budget input is 200");
        assert_eq!(
            s.budget_eur(),
            Some(dec!(200)),
            "default budget parses to €200"
        );
        assert_eq!(
            s.lookback,
            LeaderboardLookback::H1_2024,
            "default lookback is the corpus-covered H1 2024"
        );
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

    // ── F3 guided input — lookback + budget ──────────────────────────────────

    #[test]
    fn coin_universe_is_corpus_covered_and_xrp_first() {
        // Every coin must exist in the pinned Binance corpus AND match the
        // operator-locked XRP-first scan order.
        assert_eq!(BAKEOFF_COIN_UNIVERSE.len(), 10);
        assert_eq!(BAKEOFF_COIN_UNIVERSE[0], "XRPUSDT", "XRP-first");
        assert_eq!(BAKEOFF_COIN_UNIVERSE[2], "BTCUSDT");
        assert!(
            BAKEOFF_COIN_UNIVERSE.contains(&DEFAULT_BAKEOFF_COIN),
            "the default coin must be in the offered universe"
        );
    }

    #[test]
    fn relative_lookback_maps_to_custom_window_from_now() {
        // A relative lookback → Custom { now - N days, now } (the UI mapping;
        // the backtest crate is untouched).
        const NOW: i64 = 1_900_000_000_000; // a fixed wall-clock for the test
        let two_weeks = LeaderboardLookback::TwoWeeks.to_date_range(NOW);
        match two_weeks {
            backtest::engine::DateRange::Custom { start_ms, end_ms } => {
                assert_eq!(end_ms, NOW, "the window ends at now");
                assert_eq!(
                    end_ms - start_ms,
                    14 * 86_400_000,
                    "2-weeks spans exactly 14 days of millis"
                );
            }
            other => panic!("relative lookback must map to Custom, got {other:?}"),
        }

        // 4-years is the product's widest headline window.
        let four_years = LeaderboardLookback::FourYears.to_date_range(NOW);
        match four_years {
            backtest::engine::DateRange::Custom { start_ms, end_ms } => {
                assert_eq!(end_ms - start_ms, 1460 * 86_400_000);
            }
            other => panic!("4-years must map to Custom, got {other:?}"),
        }
    }

    #[test]
    fn fixed_preset_lookback_passes_through() {
        const NOW: i64 = 1_900_000_000_000;
        assert!(matches!(
            LeaderboardLookback::H1_2024.to_date_range(NOW),
            backtest::engine::DateRange::H1_2024
        ));
        assert!(matches!(
            LeaderboardLookback::H2_2024.to_date_range(NOW),
            backtest::engine::DateRange::H2_2024
        ));
    }

    #[test]
    fn all_lookbacks_render_in_order() {
        // The picker offers all nine choices, relative first then the 2024
        // presets last (the order the chips render).
        let all = LeaderboardLookback::ALL;
        assert_eq!(all.len(), 9);
        assert_eq!(all[0], LeaderboardLookback::TwoWeeks);
        assert_eq!(all[6], LeaderboardLookback::FourYears);
        assert_eq!(all[7], LeaderboardLookback::H1_2024);
        assert_eq!(all[8], LeaderboardLookback::H2_2024);
    }

    #[test]
    fn parse_budget_accepts_plain_euro_and_whitespace() {
        assert_eq!(parse_budget("200"), Some(dec!(200)));
        assert_eq!(parse_budget("  200  "), Some(dec!(200)));
        assert_eq!(parse_budget("\u{20ac}200"), Some(dec!(200)), "leading € ok");
        assert_eq!(parse_budget("199.50"), Some(dec!(199.50)));
        assert_eq!(
            parse_budget("0"),
            Some(dec!(0)),
            "zero is a valid (empty) budget"
        );
    }

    #[test]
    fn parse_budget_rejects_blank_negative_and_garbage() {
        assert_eq!(parse_budget(""), None);
        assert_eq!(parse_budget("   "), None);
        assert_eq!(parse_budget("-50"), None, "a budget cannot be negative");
        assert_eq!(parse_budget("abc"), None);
        assert_eq!(
            parse_budget("1,000"),
            None,
            "no thousands sep in the raw field"
        );
    }
}
