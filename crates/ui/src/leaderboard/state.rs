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
    /// Annualised Sortino (hourly). Mirrored from `CandidateKpis.sortino` for
    /// the F9 narration facts; NOT displayed in the leaderboard table columns.
    pub sortino: f64,
    /// Calmar ratio. Mirrored from `CandidateKpis.calmar` for the F9 narration
    /// facts; NOT displayed in the leaderboard table columns.
    pub calmar: f64,
    /// Total return fraction (`0.1` = +10 %). Rendered as a sentiment-coloured
    /// percentage.
    pub total_return_pct: Decimal,
    /// Max drawdown fraction (`0.0` = no drawdown). Rendered as a `DOWN_500`
    /// percentage.
    pub max_drawdown: Decimal,
    /// Number of executed trades (buys + sells).
    pub trade_count: usize,
    /// Capital turnover ratio (P1-1 / advisor-turnover-and-tail-metrics, REPORT-ONLY).
    ///
    /// `Σ(fill.price × fill.qty) / mean_equity` — "how many times did the
    /// strategy churn its capital?"  Mirrored from `CandidateKpis.turnover`.
    /// `0.0` for idle / buy-and-hold with no fills.
    ///
    /// **NOT displayed in the leaderboard table columns yet** (carried for
    /// narration, exactly as `sortino`/`calmar` are today — the ui-designer
    /// surfaces it in a later increment).
    pub turnover: Decimal,
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

// ── P0-1 overfitting scorecard (ADR-0075) — the "show your work" readout ──────

/// The overfitting scorecard, mirrored from `backtest::bakeoff::Scorecard` into
/// a pure-`ui` shape (advisor-overfitting-scorecard, P0-1 / ADR-0075).
///
/// **Plain fields only — NO `backtest::Scorecard` crosses into the widgets.**
/// Every field is a `usize` / `f64` / `bool` (the same value-only discipline
/// [`LeaderRow`] follows): the engine type is read ONCE in
/// [`BakeoffReportMirror::from_report`] and projected here, so the render code
/// never names an engine struct. `pbo` is intentionally omitted — it is always
/// `None` in v2 (deferred to the Tune/sweep surface, §6.0 D1), so there is
/// nothing to display.
///
/// # REPORT-ONLY (§6.0 D3 / ADR-0075)
///
/// This is a credibility/honesty readout, never a verdict. `crown_clears_dsr`
/// is informational — it does NOT (and must not) change the crown, the rank, or
/// the FROZEN robustness gate. The screen labels it "informational, not a gate"
/// so the operator can never mistake it for the pick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScorecardView {
    /// Raw number of candidates tried (every arm ranked, including the
    /// buy-and-hold benchmark). The literature's "number of trials" N.
    pub n_candidates: usize,
    /// Effective (correlation-adjusted) trial count — `ρ̄ + (1 − ρ̄) · M`.
    /// Always `1.0 ≤ n_eff ≤ n_candidates`. Shown rounded as "about N
    /// truly independent".
    pub n_eff: f64,
    /// Deflated Sharpe Ratio — probability in `[0, 1]` that the crown's true
    /// edge exceeds zero AFTER correcting for how many strategies were tried.
    /// Rendered as a percentage.
    pub deflated_sharpe: f64,
    /// Minimum backtest length (years) needed to trust the crown —
    /// `2 · ln(n_eff) / SR_target²`. `0.0` when `n_eff ≤ 1`.
    pub min_btl_years: f64,
    /// Informational flag: `deflated_sharpe ≥ 0.95`. **REPORT-ONLY** — never a
    /// veto. Drives the plain "Beats holding after the search?" yes/no.
    pub crown_clears_dsr: bool,
}

impl ScorecardView {
    /// Mirror a `backtest::bakeoff::Scorecard` into the pure-`ui` view, or
    /// `None` for a **degenerate** scorecard (`n_candidates == 0` — the
    /// zero-field returned by `compute_scorecard` on empty inputs). The screen
    /// renders no "show your work" block for a `None`, so a bake-off that
    /// produced no real scorecard never paints a misleading all-zero readout.
    ///
    /// This is the only place a `backtest::Scorecard` is read on the `ui` side;
    /// it is reached exclusively from [`BakeoffReportMirror::from_report`]
    /// (the single mirror boundary). Pure + total — no I/O, no panic. Crosses
    /// the seam as plain `usize` / `f64` / `bool` (zero new `ui` dep edge).
    #[must_use]
    pub fn from_scorecard(sc: &backtest::bakeoff::Scorecard) -> Option<Self> {
        if sc.n_candidates == 0 {
            return None;
        }
        Some(Self {
            n_candidates: sc.n_candidates,
            n_eff: sc.n_eff,
            deflated_sharpe: sc.deflated_sharpe,
            min_btl_years: sc.min_btl_years,
            crown_clears_dsr: sc.crown_clears_dsr,
        })
    }
}

// ── F9 LLM "why this one" narration (ADR-0064) ────────────────────────────────

/// The narration's lifecycle on the leaderboard recommendation block (F9,
/// ADR-0064 § D4).
///
/// **String/enum only — NO `llm`/`agent`/engine type crosses `view`** (the
/// [`RecommendationMirror`] discipline). The render code matches on this closed
/// `ui` enum exactly as it matches [`OutcomeKind`] / [`RobustnessLabel`] /
/// [`ReasonLabel`]; the ONLY place an `agent`/`llm` narration type is named on
/// the `ui` side is the one `#[cfg(feature = "live")]` recipe/adapter that maps
/// the received `agent::NarrationOutcome` → [`NarrationOutcome`] (the
/// `forward_plan/adapter.rs` boundary — one edit site if a name drifts).
///
/// The templated copy (`headline_copy` + `reason_copy`) is the FLOOR in every
/// arm except [`Ready`](NarrationState::Ready) — there is never a blank or
/// half-answer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NarrationState {
    /// No "Explain" requested yet — show the templated copy + the Explain
    /// control. The default.
    #[default]
    NotRequested,
    /// The narration is being generated — show the templated copy + a spinner.
    InFlight,
    /// A faithful narration passed the agent-side post-check — show this prose
    /// (with the `disclaimer()` framing around it).
    Ready(SmolStr),
    /// The narration was unavailable / errored / over budget / failed the
    /// post-check — show the templated copy (the honest fallback). Silent.
    FellBack,
}

impl NarrationState {
    /// `true` once an "Explain" has been requested (`InFlight` / `Ready` /
    /// `FellBack`) — drives whether the Explain control is still offered (it
    /// shows only in `NotRequested`).
    #[must_use]
    pub fn is_requested(&self) -> bool {
        !matches!(self, NarrationState::NotRequested)
    }
}

/// The `core`-clean narration result that crosses the agent→iced seam, mirrored
/// into a pure-`ui` enum (ADR-0064 § D2). This is the `NarrationOutcome`
/// analogue of [`ForwardPlanView`](crate::forward_plan::ForwardPlanView): the
/// developer's `agent::NarrationOutcome { Ready(SmolStr) | FellBack }` is mapped
/// into THIS `ui`-owned type at the single `#[cfg(feature = "live")]` adapter,
/// so the `Message::BakeoffNarrationCompleted` payload is `ui`-pure and the
/// message enum compiles in the default (non-`live`) fixtures build.
///
/// Carries a plain [`SmolStr`] — NO `llm` type, NO `ChatResponse`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NarrationOutcome {
    /// A faithful narration passed the agent-side post-check.
    Ready(SmolStr),
    /// Every failure mode (provider disabled / network / timeout /
    /// `BudgetExceeded` / `ReplayMiss` / empty response / post-check reject) —
    /// the honest fallback to the templated copy.
    FellBack,
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
    /// The overfitting scorecard (P0-1 / ADR-0075), mirrored from
    /// `Recommendation.scorecard`. `None` for a degenerate (empty-field)
    /// scorecard so the "show your work" block paints nothing rather than an
    /// all-zero readout. **REPORT-ONLY** — display-only honesty readout, never
    /// touches the crown / rank / gate.
    pub scorecard: Option<ScorecardView>,
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
                sortino: c.kpis.sortino,
                calmar: c.kpis.calmar,
                total_return_pct: c.kpis.total_return_pct,
                max_drawdown: c.kpis.max_drawdown,
                trade_count: c.kpis.trade_count,
                // P1-1: mirror turnover (report-only; not displayed in table columns yet).
                turnover: c.kpis.turnover,
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
            // P0-1 (ADR-0075): mirror the report-only scorecard. `None` for a
            // degenerate (empty-field) scorecard. Crosses as plain f64/usize/bool.
            scorecard: ScorecardView::from_scorecard(&r.scorecard),
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

/// The default bake-off start capital input — 100,000 USDT, the legacy engine
/// default. Stored as a raw input string so the numeric field round-trips the
/// operator's keystrokes verbatim (same pattern as `DEFAULT_BUDGET_INPUT`).
/// Parsed to a `Decimal` via [`parse_start_capital`]; an empty / non-numeric /
/// non-positive input falls back to this default in the engine.
pub const DEFAULT_START_CAPITAL_INPUT: &str = "100000";

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

    /// Map this lookback to the **Lab screen's** `crate::lab::state::DateRange`,
    /// so clicking a leaderboard row to inspect a strategy in the Lab carries the
    /// SAME window the bake-off ranked it on (advisor-leaderboard-inspect-in-lab).
    ///
    /// The two fixed presets map to the Lab's named `Preset`s (so the Lab's
    /// date-range picker shows the matching chip); the relative windows map to a
    /// `Custom` ISO-date pair (`now - N days` → `now`, `YYYY-MM-DD`). The Lab run
    /// path (`cockpit_live.rs` → `lab_config_to_scenario`) accepts date-only
    /// `Custom:<start>:<end>` labels, so a `Custom` window runs faithfully — the
    /// byte-identical window the leaderboard scored.
    ///
    /// `now_ms` is wall-clock UTC epoch-millis, passed in (like `to_date_range`)
    /// so the mapping stays pure + testable. A non-representable `now_ms`
    /// (out of `OffsetDateTime`'s range — never reached for real clocks) falls
    /// back to the `H1_2024` corpus preset so the Lab always opens runnable.
    #[must_use]
    pub fn to_lab_date_range(self, now_ms: i64) -> crate::lab::state::DateRange {
        use crate::lab::state::{DateRange as LabRange, Preset};
        match self.relative_days() {
            Some(days) => {
                // `YYYY-MM-DD` ISO date for `now` and `now - N days`. The Lab's
                // `lab_config_to_scenario` parser accepts the date-only form.
                let fmt = time::macros::format_description!("[year]-[month]-[day]");
                let to_iso = |epoch_ms: i64| -> Option<SmolStr> {
                    let secs = epoch_ms.div_euclid(1_000);
                    time::OffsetDateTime::from_unix_timestamp(secs)
                        .ok()
                        .and_then(|dt| dt.format(&fmt).ok())
                        .map(SmolStr::new)
                };
                match (to_iso(now_ms - days * MS_PER_DAY), to_iso(now_ms)) {
                    (Some(start_raw), Some(end_raw)) => LabRange::Custom { start_raw, end_raw },
                    // Unreachable for real clocks; keep the Lab runnable rather
                    // than threading a fallible result through the click handler.
                    _ => LabRange::Preset(Preset::H1_2024),
                }
            }
            // Fixed presets → the Lab's matching named preset.
            None => match self {
                LeaderboardLookback::H2_2024 => LabRange::Preset(Preset::H2_2024),
                _ => LabRange::Preset(Preset::H1_2024),
            },
        }
    }
}

/// The bar-size ("timeframe") the bake-off resamples to before ranking — the
/// new "Tune" knob introduced by the leaderboard-tuning feature.
///
/// A closed UI-side enum (same mirror discipline as `LeaderboardLookback`):
/// the screen never matches on an engine type; `to_horizon()` converts to the
/// `backtest::resample::Horizon` at the dispatch boundary.
///
/// **`OneHour` is the default** — identity pass-through, byte-identical to the
/// prior behaviour so existing tests + anchors are unaffected. `FourHours` and
/// `OneDay` fold bars 4:1 / 24:1 before ranking; a different bar size CAN
/// change the crowning result (the leaderboard chip is honest about this —
/// "changes ranking").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BakeoffTimeframe {
    /// 1-hour bars — the legacy default (identity pass-through, no fold).
    #[default]
    OneHour,
    /// 4-hour bars — fold 4 × 1h bars into 1.
    FourHours,
    /// Daily bars — fold 24 × 1h bars into 1.
    OneDay,
}

impl BakeoffTimeframe {
    /// All three timeframe choices in the order the chip picker renders them.
    pub const ALL: &'static [BakeoffTimeframe] = &[
        BakeoffTimeframe::OneHour,
        BakeoffTimeframe::FourHours,
        BakeoffTimeframe::OneDay,
    ];

    /// Convert to the engine's `Horizon` enum at the dispatch boundary.
    ///
    /// Called by `bakeoff_config_from_state` — the only site where a
    /// UI-side enum converts to an engine type (the mirror discipline).
    #[must_use]
    pub fn to_horizon(self) -> backtest::resample::Horizon {
        match self {
            BakeoffTimeframe::OneHour => backtest::resample::Horizon::OneHour,
            BakeoffTimeframe::FourHours => backtest::resample::Horizon::FourHours,
            BakeoffTimeframe::OneDay => backtest::resample::Horizon::OneDay,
        }
    }

    /// Short display label for the chip: `"H1"` / `"H4"` / `"D1"`.
    #[must_use]
    pub fn chip_label(self) -> &'static str {
        match self {
            BakeoffTimeframe::OneHour => "H1",
            BakeoffTimeframe::FourHours => "H4",
            BakeoffTimeframe::OneDay => "D1",
        }
    }
}

/// Parse a start-capital input string into a positive `Decimal`.
///
/// Returns `Some(amount)` only when `s` parses to a finite `Decimal > 0`
/// (capital must be strictly positive to run a bake-off). Empty / non-numeric /
/// non-positive returns `None` → the engine uses the legacy default (`100_000`
/// USDT). Pure; no I/O. Accepts leading/trailing whitespace and an optional
/// leading `$` / `€` so paste-from-copy round-trips.
#[must_use]
pub fn parse_start_capital(s: &str) -> Option<Decimal> {
    let trimmed = s
        .trim()
        .trim_start_matches('$')
        .trim_start_matches('\u{20ac}')
        .trim();
    if trimmed.is_empty() {
        return None;
    }
    let amount = trimmed.parse::<Decimal>().ok()?;
    if amount.is_sign_positive() && !amount.is_zero() {
        Some(amount)
    } else {
        None
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
    /// The latest candidate-level bake-off progress (mirrors the Lab
    /// `LabState::run_progress`). `Some` once the first `BakeoffProgress` event
    /// arrives over the channel; drives the DETERMINATE progress bar ("Running
    /// {id} — {n} of {total}", filled `done / total`). `None` before the first
    /// event (the bar falls back to the indeterminate spinner) and after the run
    /// completes (cleared in `finish_run`). Stored as the raw `backtest`
    /// candidate-progress type — `backtest` is a hard `ui` dep (the same seam
    /// the Lab uses for `progress::Progress`), and the type is `Clone` so
    /// `LeaderboardScreenState` stays `Clone` for the render-test harness.
    pub progress: Option<backtest::progress::BakeoffProgress>,

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
    /// The bar-size the operator chose for the bake-off (default `H1` — the
    /// legacy identity pass-through). Mapped to a `backtest::resample::Horizon`
    /// at dispatch time via [`BakeoffTimeframe::to_horizon`].
    ///
    /// **Affects ranking**: a different bar size (H4 / D1) folds the 1h bars
    /// before the strategies run, which CAN change which arm is crowned. The UI
    /// is honest about this (the chip row says "changes ranking").
    pub timeframe: BakeoffTimeframe,
    /// The raw start-capital input string (round-trips the operator's keystrokes,
    /// same pattern as `budget_input`). Parsed to a `Decimal` via
    /// [`parse_start_capital`]; `None` parse → the engine uses the `100_000` USDT
    /// legacy default.
    ///
    /// **Does NOT affect ranking** — all arms in the bake-off run with the SAME
    /// start capital, so the relative ranking (Sharpe / Sortino / Calmar / return
    /// %) is unchanged. It scales the ABSOLUTE equity curve values and the forward
    /// sizing estimate. The UI is honest about this (the input note says "does not
    /// affect ranking").
    pub start_capital_input: String,

    // ── F9 LLM "why this one" narration (ADR-0064) ───────────────────────────
    /// The opt-in LLM-narration lifecycle on the recommendation block (F9).
    /// Defaults to [`NarrationState::NotRequested`] and is INDEPENDENT of
    /// `result` — the structured bake-off renders immediately on
    /// `BakeoffRunCompleted`; pressing "Explain" flips this to `InFlight` and
    /// the second async step lands `Ready`/`FellBack` in place. String/enum only
    /// — no `llm`/`agent` type crosses `view`.
    pub narration: NarrationState,
}

impl Default for LeaderboardScreenState {
    fn default() -> Self {
        Self {
            // Cold start is the honest Empty state (no run yet), NOT Loading —
            // Loading is reserved for an in-flight bake-off so the spinner only
            // shows when work is actually happening.
            result: PanelState::Empty,
            running: false,
            // No bake-off in flight at cold start → no progress yet.
            progress: None,
            // F3 guided-input defaults — the most-used starting point (product
            // § journey step 1: BTCUSDT / €200 / a corpus-covered window).
            coin: Symbol::new(DEFAULT_BAKEOFF_COIN),
            budget_input: DEFAULT_BUDGET_INPUT.to_string(),
            lookback: LeaderboardLookback::H1_2024,
            // Tuning knobs — defaults preserve byte-identical prior behaviour:
            // H1 = identity pass-through (no bar fold); 100_000 = legacy capital.
            timeframe: BakeoffTimeframe::OneHour,
            start_capital_input: DEFAULT_START_CAPITAL_INPUT.to_string(),
            // F9 — no narration requested until the operator presses "Explain".
            narration: NarrationState::NotRequested,
        }
    }
}

impl LeaderboardScreenState {
    /// Mark a bake-off as started — flips `result` to `Loading` + `running` to
    /// `true`. Called from the `Message::BakeoffRunRequested` update arm. Also
    /// resets the F9 narration to `NotRequested` so a prior run's "Explain"
    /// prose never carries over into a fresh bake-off.
    pub fn begin_run(&mut self) {
        self.result = PanelState::Loading;
        self.running = true;
        self.narration = NarrationState::NotRequested;
        // Clear any prior run's progress so the new run starts from the
        // indeterminate spinner until its first `BakeoffProgress` event arrives
        // (a stale "7 of 7" from the last run must never show on the new one).
        self.progress = None;
    }

    /// Land a candidate-level progress update (mirrors `LabState::run_progress`
    /// being set on `LabRunProgress`). Called from the `Message::BakeoffProgress`
    /// update arm. Drives the determinate progress bar; ignored after the run
    /// completes (the binary stops the recipe + clears `progress` in `finish_run`).
    pub fn set_progress(&mut self, progress: backtest::progress::BakeoffProgress) {
        self.progress = Some(progress);
    }

    /// Land a completed bake-off result. `Ok(mirror)` → `Ready`; `Err(msg)` →
    /// `Error`. Always clears `running`. Called from the
    /// `Message::BakeoffRunCompleted` update arm. The F9 narration stays
    /// `NotRequested` (it was reset in `begin_run`) — the structured result is
    /// complete and honest on its own; the operator opts into the narration.
    pub fn finish_run(&mut self, outcome: Result<BakeoffReportMirror, SmolStr>) {
        self.running = false;
        // The run is over — clear progress so a stale "{n} of {total}" never
        // lingers under the (now Ready/Error) result.
        self.progress = None;
        self.result = match outcome {
            Ok(mirror) if mirror.rows.is_empty() => PanelState::Empty,
            Ok(mirror) => PanelState::Ready(mirror),
            Err(msg) => PanelState::Error(msg),
        };
    }

    /// Mark the F9 narration as in flight — flips `narration` to `InFlight`.
    /// Called from the `Message::BakeoffNarrationRequested` update arm (after
    /// guarding against a re-request). The templated copy stays visible the
    /// whole time (the floor), so the block never goes blank.
    pub fn begin_narration(&mut self) {
        self.narration = NarrationState::InFlight;
    }

    /// Land a completed F9 narration in place — maps the `core`-clean
    /// [`NarrationOutcome`] into the render-side [`NarrationState`]:
    /// `Ready(prose) → Ready(prose)`, `FellBack → FellBack` (the honest
    /// fallback to the templated copy). Called from the
    /// `Message::BakeoffNarrationCompleted` update arm. Never touches `result`
    /// — the structured bake-off is independent of the narration.
    pub fn set_narration(&mut self, outcome: NarrationOutcome) {
        self.narration = match outcome {
            NarrationOutcome::Ready(prose) => NarrationState::Ready(prose),
            NarrationOutcome::FellBack => NarrationState::FellBack,
        };
    }

    /// The parsed budget (euros), or `None` when the input is blank /
    /// unparseable / negative. The header + the down-stream sizing read this;
    /// the bake-off ranking does not.
    #[must_use]
    pub fn budget_eur(&self) -> Option<Decimal> {
        parse_budget(&self.budget_input)
    }

    /// The parsed bake-off start capital, or the legacy default (`100_000` USDT)
    /// when the input is blank / unparseable / non-positive.
    ///
    /// Called by `bakeoff_config_from_state` to populate `BakeoffRequest::
    /// initial_capital`. Does NOT affect ranking (all arms run with the same
    /// capital, so risk-adjusted KPIs are unchanged); it scales absolute
    /// equity-curve values + the forward sizing estimate.
    #[must_use]
    pub fn start_capital(&self) -> Decimal {
        parse_start_capital(&self.start_capital_input)
            .unwrap_or_else(|| rust_decimal_macros::dec!(100_000))
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
            sortino: 0.0,
            calmar: 0.0,
            total_return_pct: dec!(0.05),
            max_drawdown: dec!(0.10),
            trade_count: 12,
            turnover: Decimal::ZERO,
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
            scorecard: Some(ScorecardView {
                n_candidates: 2,
                n_eff: 1.8,
                deflated_sharpe: 0.71,
                min_btl_years: 1.2,
                crown_clears_dsr: false,
            }),
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

    // ── P0-1 scorecard mirror (ADR-0075) ─────────────────────────────────────

    #[test]
    fn scorecard_view_mirrors_a_populated_scorecard() {
        let sc = backtest::bakeoff::Scorecard {
            n_candidates: 13,
            n_eff: 8.4,
            deflated_sharpe: 0.62,
            min_btl_years: 6.4,
            pbo: None,
            crown_clears_dsr: false,
        };
        let view = ScorecardView::from_scorecard(&sc).expect("populated → Some");
        assert_eq!(view.n_candidates, 13);
        assert!((view.n_eff - 8.4).abs() < 1e-9);
        assert!((view.deflated_sharpe - 0.62).abs() < 1e-9);
        assert!((view.min_btl_years - 6.4).abs() < 1e-9);
        assert!(!view.crown_clears_dsr);
    }

    #[test]
    fn scorecard_view_is_none_for_degenerate_empty_field() {
        // The zero scorecard `compute_scorecard` returns on empty inputs
        // (`n_candidates == 0`) must mirror to `None`, so the "show your work"
        // block paints nothing rather than an all-zero readout.
        let degenerate = backtest::bakeoff::Scorecard {
            n_candidates: 0,
            n_eff: 0.0,
            deflated_sharpe: 0.0,
            min_btl_years: 0.0,
            pbo: None,
            crown_clears_dsr: false,
        };
        assert!(ScorecardView::from_scorecard(&degenerate).is_none());
    }

    // ── F9 narration state transitions (ADR-0064) ────────────────────────────

    #[test]
    fn default_narration_is_not_requested() {
        let s = LeaderboardScreenState::default();
        assert_eq!(
            s.narration,
            NarrationState::NotRequested,
            "no narration until the operator presses Explain"
        );
        assert!(
            !s.narration.is_requested(),
            "NotRequested is not yet requested"
        );
    }

    #[test]
    fn begin_narration_sets_in_flight() {
        let mut s = LeaderboardScreenState::default();
        s.begin_narration();
        assert_eq!(s.narration, NarrationState::InFlight);
        assert!(s.narration.is_requested(), "InFlight counts as requested");
    }

    #[test]
    fn set_narration_ready_lands_prose() {
        let mut s = LeaderboardScreenState::default();
        s.begin_narration();
        s.set_narration(NarrationOutcome::Ready(SmolStr::new("because reasons")));
        match &s.narration {
            NarrationState::Ready(prose) => assert_eq!(prose.as_str(), "because reasons"),
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn set_narration_fell_back_lands_fallback() {
        // Every failure mode maps to FellBack — the honest floor (templated copy
        // renders), NOT an error surface.
        let mut s = LeaderboardScreenState::default();
        s.begin_narration();
        s.set_narration(NarrationOutcome::FellBack);
        assert_eq!(s.narration, NarrationState::FellBack);
        assert!(s.narration.is_requested());
    }

    #[test]
    fn begin_run_resets_a_prior_narration() {
        // A fresh bake-off must NOT carry over the prior run's "Explain" prose —
        // begin_run resets the narration so the new result starts clean.
        let mut s = LeaderboardScreenState::default();
        s.begin_narration();
        s.set_narration(NarrationOutcome::Ready(SmolStr::new("stale prose")));
        assert!(matches!(s.narration, NarrationState::Ready(_)));

        s.begin_run();
        assert_eq!(
            s.narration,
            NarrationState::NotRequested,
            "a new bake-off resets the narration to NotRequested"
        );
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

    // ── Tuning knobs: timeframe + start capital ──────────────────────────────

    #[test]
    fn default_timeframe_is_h1_and_capital_is_100k() {
        let s = LeaderboardScreenState::default();
        assert_eq!(
            s.timeframe,
            BakeoffTimeframe::OneHour,
            "default timeframe is H1 (identity, preserves prior behaviour)"
        );
        assert_eq!(
            s.start_capital_input, "100000",
            "default start capital input is 100000"
        );
        assert_eq!(
            s.start_capital(),
            dec!(100_000),
            "default start capital parses to 100_000"
        );
    }

    #[test]
    fn bakeoff_timeframe_chip_labels() {
        assert_eq!(BakeoffTimeframe::OneHour.chip_label(), "H1");
        assert_eq!(BakeoffTimeframe::FourHours.chip_label(), "H4");
        assert_eq!(BakeoffTimeframe::OneDay.chip_label(), "D1");
    }

    #[test]
    fn bakeoff_timeframe_to_horizon_roundtrips() {
        assert!(matches!(
            BakeoffTimeframe::OneHour.to_horizon(),
            backtest::resample::Horizon::OneHour
        ));
        assert!(matches!(
            BakeoffTimeframe::FourHours.to_horizon(),
            backtest::resample::Horizon::FourHours
        ));
        assert!(matches!(
            BakeoffTimeframe::OneDay.to_horizon(),
            backtest::resample::Horizon::OneDay
        ));
    }

    #[test]
    fn bakeoff_timeframe_all_has_three_entries() {
        assert_eq!(BakeoffTimeframe::ALL.len(), 3);
        assert_eq!(BakeoffTimeframe::ALL[0], BakeoffTimeframe::OneHour);
        assert_eq!(BakeoffTimeframe::ALL[1], BakeoffTimeframe::FourHours);
        assert_eq!(BakeoffTimeframe::ALL[2], BakeoffTimeframe::OneDay);
    }

    #[test]
    fn parse_start_capital_accepts_positive_values() {
        assert_eq!(parse_start_capital("100000"), Some(dec!(100_000)));
        assert_eq!(parse_start_capital("  50000  "), Some(dec!(50_000)));
        assert_eq!(parse_start_capital("$100000"), Some(dec!(100_000)));
        assert_eq!(parse_start_capital("\u{20ac}200"), Some(dec!(200)));
        assert_eq!(parse_start_capital("1.50"), Some(dec!(1.50)));
    }

    #[test]
    fn parse_start_capital_rejects_non_positive_blank_and_garbage() {
        assert_eq!(parse_start_capital(""), None, "empty string → None");
        assert_eq!(parse_start_capital("   "), None, "whitespace only → None");
        assert_eq!(parse_start_capital("0"), None, "zero is not valid capital");
        assert_eq!(
            parse_start_capital("-100"),
            None,
            "negative is not valid capital"
        );
        assert_eq!(parse_start_capital("abc"), None, "non-numeric → None");
        assert_eq!(
            parse_start_capital("1,000"),
            None,
            "thousands-sep not supported"
        );
    }

    #[test]
    fn start_capital_falls_back_to_100k_on_bad_input() {
        // Blank → fallback
        let s = LeaderboardScreenState {
            start_capital_input: String::new(),
            ..LeaderboardScreenState::default()
        };
        assert_eq!(
            s.start_capital(),
            dec!(100_000),
            "blank input falls back to 100_000 USDT"
        );
        // Garbage → fallback
        let s2 = LeaderboardScreenState {
            start_capital_input: "abc".to_string(),
            ..LeaderboardScreenState::default()
        };
        assert_eq!(
            s2.start_capital(),
            dec!(100_000),
            "garbage input falls back to 100_000 USDT"
        );
        // Negative → fallback
        let s3 = LeaderboardScreenState {
            start_capital_input: "-500".to_string(),
            ..LeaderboardScreenState::default()
        };
        assert_eq!(
            s3.start_capital(),
            dec!(100_000),
            "negative input falls back to 100_000 USDT"
        );
    }

    #[test]
    fn start_capital_uses_parsed_value_when_valid() {
        let s = LeaderboardScreenState {
            start_capital_input: "200000".to_string(),
            ..LeaderboardScreenState::default()
        };
        assert_eq!(s.start_capital(), dec!(200_000));

        let s2 = LeaderboardScreenState {
            start_capital_input: "500".to_string(),
            ..LeaderboardScreenState::default()
        };
        assert_eq!(s2.start_capital(), dec!(500));
    }
}
