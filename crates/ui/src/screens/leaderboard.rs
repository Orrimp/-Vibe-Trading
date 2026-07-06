//! advisor-leaderboard-screen v0.1.0 — Leaderboard screen body.
//!
//! Step 3 of the single-coin investment-advisor journey: render a
//! `backtest::bakeoff` result as a ranked, clickable leaderboard. Layout
//! (single column, top-to-bottom):
//!
//! ```text
//! ┌─ Strategy bake-off ─────────────────────────────────────────────┐
//! │ <caption>                                  [ Run bake-off ]      │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ Recommendation                                                   │
//! │   <headline from Recommendation>                                 │
//! │   · <reason>   · <reason>                                        │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  #  Strategy            Return   Sharpe   Max DD   Trades        │
//! │  1  v0.sma  ★ best      +7.38%   1.20     -4.20%   441           │  ← ACCENT row
//! │  2  v0.buyhold baseline +3.60%   0.41     -8.10%   2             │
//! │  …                                                               │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ Not financial advice. Results are simulated…  (persistent)       │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! - **Result behind a `PanelState`** (Loading / Empty / Error / Ready) — no
//!   blank screen. `Empty` is the cold "press Run bake-off" prompt; `Loading`
//!   shows the spinner; `Error` shows the prefix + the engine detail; `Ready`
//!   shows the recommendation + the ranked table.
//! - **The crowned row** reuses the `ACCENT` "best" treatment (a `★ best` tag
//!   + accent text + accent left-rule via `frame::active_row`), the same
//!   discipline the Reports "● curve" / active-row marker uses.
//! - **The benchmark row** is labelled `baseline (buy & hold)` so buy-and-hold
//!   always reads as the reference line the active strategies are measured
//!   against (ADR-0066) — never a failed candidate. When `BenchmarkWins` fires
//!   (the honest modal crypto outcome) the crowned baseline row is the answer:
//!   nothing active cleared the robustness bar, so holding is the least-bad
//!   choice. Its own robustness flag (if Fragile) renders as a quiet
//!   informational note, not the disqualifying badge an active arm gets.
//! - **Numbers are scannable** — the Return / Sharpe / Max DD / Trades columns
//!   are right-aligned; colour is used only for `pos` / `neg` sentiment.
//! - **The recommendation headline** is rendered FROM the structured
//!   `Recommendation` (the UI owns the copy — every string is in
//!   `crate::strings`), with the `reasons` as supporting sub-copy.
//! - **A persistent NOT-ADVICE + simulated disclaimer** (product § D5) sits at
//!   the bottom of every result surface.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.
//! **No new theme token, no new widget.**

// Per-module clippy allow-pattern (mirrors `screens/reports.rs:42`): the
// `space::* as u16` / `as f32` layout casts are bounded + safe; `view`/helpers
// take `mode` by value (the `Copy` `ThemeMode`).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::needless_pass_by_value
)]

use iced::widget::{Button, Column, Container, Row, Scrollable, Space, Text, button};
use iced::{Border, Length};
use trading_core::{StrategyId, Symbol};

use crate::leaderboard::state::{
    BakeoffReportMirror, DataQualityView, LeaderRow, LeaderboardLookback, NarrationState,
    OutcomeKind, ReasonLabel, RecommendationMirror, RobustnessLabel, ScorecardView,
    TailSummaryView,
};
use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    LEADERBOARD_BENCHMARK_FRAGILE_NOTE, LEADERBOARD_BENCHMARK_TAG, LEADERBOARD_BUDGET_CONTEXT_FMT,
    LEADERBOARD_CAPTION, LEADERBOARD_COL_MAX_DD, LEADERBOARD_COL_RANK, LEADERBOARD_COL_RETURN,
    LEADERBOARD_COL_SHARPE, LEADERBOARD_COL_STRATEGY, LEADERBOARD_COL_TRADES,
    LEADERBOARD_COL_TURNOVER, LEADERBOARD_CONTEXT_NO_BUDGET_FMT, LEADERBOARD_CROWN_TAG,
    LEADERBOARD_DATA_QUALITY_CAPTION, LEADERBOARD_DATA_QUALITY_INFORMATIONAL_NOTE,
    LEADERBOARD_DATA_QUALITY_PROVENANCE_LABEL, LEADERBOARD_DATA_QUALITY_SURVIVAL_LABEL,
    LEADERBOARD_DATA_QUALITY_TITLE, LEADERBOARD_DATA_QUALITY_TRUST_LABEL,
    LEADERBOARD_DATA_QUALITY_VENUE_LABEL, LEADERBOARD_DATA_QUALITY_WARNINGS_LABEL,
    LEADERBOARD_DISCLAIMER, LEADERBOARD_EMPTY_PROMPT, LEADERBOARD_ENSEMBLE_ANY1OF4_LABEL,
    LEADERBOARD_ENSEMBLE_K2OF4_LABEL, LEADERBOARD_ENSEMBLE_K3OF4_LABEL,
    LEADERBOARD_ENSEMBLE_MAJORITY_LABEL, LEADERBOARD_ENSEMBLE_SAT_IN_CASH,
    LEADERBOARD_ENSEMBLE_TR_MR_MACD_RSI_LABEL, LEADERBOARD_ENSEMBLE_TR_MR_SMA_BB_LABEL,
    LEADERBOARD_ENSEMBLE_TREND_PAIR_LABEL, LEADERBOARD_ENSEMBLE_UNANIMOUS_LABEL,
    LEADERBOARD_ENSEMBLE_VOTE_TAG, LEADERBOARD_ERROR_PREFIX, LEADERBOARD_EXPLAIN_BUTTON,
    LEADERBOARD_EXPLAIN_FELLBACK, LEADERBOARD_EXPLAIN_INFLIGHT, LEADERBOARD_EXPLAIN_LLM_LABEL,
    LEADERBOARD_FIELD_ARM_COUNT_FMT, LEADERBOARD_FRAGILE_TAG, LEADERBOARD_HEADLINE,
    LEADERBOARD_HEADLINE_ACTIVE_WINS, LEADERBOARD_HEADLINE_ALL_FRAGILE,
    LEADERBOARD_HEADLINE_BENCHMARK_WINS, LEADERBOARD_LOADING, LEADERBOARD_MARGINAL_TAG,
    LEADERBOARD_PROGRESS_FMT, LEADERBOARD_REASON_ALL_FRAGILE,
    LEADERBOARD_REASON_BEAT_BENCHMARK_SHARPE, LEADERBOARD_REASON_BENCHMARK_UNDEFEATED,
    LEADERBOARD_REASON_HIGHEST_ROBUST_SHARPE, LEADERBOARD_REASON_TIE_DRAWDOWN,
    LEADERBOARD_REASON_TIE_RETURN, LEADERBOARD_RECOMMENDATION_TITLE,
    LEADERBOARD_RISK_STORY_CALMAR_HINT, LEADERBOARD_RISK_STORY_CALMAR_LABEL,
    LEADERBOARD_RISK_STORY_CAPTION, LEADERBOARD_RISK_STORY_CVAR_95_LABEL,
    LEADERBOARD_RISK_STORY_CVAR_99_LABEL, LEADERBOARD_RISK_STORY_CVAR_HINT,
    LEADERBOARD_RISK_STORY_INFORMATIONAL_NOTE, LEADERBOARD_RISK_STORY_MEDIAN_HINT,
    LEADERBOARD_RISK_STORY_MEDIAN_LABEL, LEADERBOARD_RISK_STORY_SKEW_HINT,
    LEADERBOARD_RISK_STORY_SKEW_LABEL, LEADERBOARD_RISK_STORY_SORTINO_HINT,
    LEADERBOARD_RISK_STORY_SORTINO_LABEL, LEADERBOARD_RISK_STORY_TITLE, LEADERBOARD_ROBUST_TAG,
    LEADERBOARD_RUN_BUTTON, LEADERBOARD_RUN_BUTTON_RUNNING, LEADERBOARD_SCORECARD_BEATS_HOLD_LABEL,
    LEADERBOARD_SCORECARD_BEATS_HOLD_NO, LEADERBOARD_SCORECARD_BEATS_HOLD_YES,
    LEADERBOARD_SCORECARD_CAPTION, LEADERBOARD_SCORECARD_CONFIDENCE_HINT,
    LEADERBOARD_SCORECARD_CONFIDENCE_LABEL, LEADERBOARD_SCORECARD_HISTORY_FMT,
    LEADERBOARD_SCORECARD_HISTORY_HINT, LEADERBOARD_SCORECARD_HISTORY_LABEL,
    LEADERBOARD_SCORECARD_INFORMATIONAL_NOTE, LEADERBOARD_SCORECARD_TITLE,
    LEADERBOARD_SCORECARD_TRIED_EFFECTIVE_FMT, LEADERBOARD_SCORECARD_TRIED_LABEL,
    LEADERBOARD_SHORT_ALWAYS_SHORT_LABEL, LEADERBOARD_SHORT_BBANDS_LS_LABEL,
    LEADERBOARD_SHORT_FIELD_NOTE, LEADERBOARD_SHORT_MACD_LS_LABEL, LEADERBOARD_SHORT_RSI_LS_LABEL,
    LEADERBOARD_SHORT_SMA_CROSS_LS_LABEL, LEADERBOARD_SHORT_TAG,
    LEADERBOARD_SIGNAL_DONCHIAN_BREAK_LABEL, LEADERBOARD_SIGNAL_DONCHIAN_FLOOR_LABEL,
    LEADERBOARD_SIGNAL_DVOL_REGIME_LABEL, LEADERBOARD_SIGNAL_MACRO_RISKON_LABEL,
    LEADERBOARD_SIGNAL_OBV_LABEL, LEADERBOARD_SIGNAL_ROC_MOMENTUM_LABEL,
    LEADERBOARD_SIGNAL_VOL_BREAKOUT_LABEL, LEADERBOARD_WINNER_FRAGILE_CLAUSE,
    LEADERBOARD_WINNER_ROBUST_CLAUSE, SHORT_UNBOUNDED_LOSS_DISCLAIMER,
};
use crate::theme::{ThemeMode, color, radius, space, text};
use crate::widgets::frame;
use crate::widgets::num::{fmt_pct_signed, format_pct_max_dd, format_sharpe};

// ── Column widths (the leaderboard table proportions) ─────────────────────────
//
// A wide strategy column (id + tags) on the left, fixed-width right-aligned
// numeric columns on the right (the scannability rule). Local `f32` layout
// constants — not design tokens (column widths are a per-table layout choice,
// the same way `reports::PICKER_WIDTH` is a local list-rail width).

/// Rank cell width (`#` column — 1-2 digits).
const W_RANK: f32 = 28.0;
/// Numeric cell width (Return / Sharpe / Max DD / Trades).
const W_NUM: f32 = 110.0;
/// Churn (turnover) cell width — narrower than `W_NUM` because the values are
/// always short ("0.0×" / "1.4×" / "12.7×"). Keeps the table fitting at the
/// 1920-wide leaderboard screenshot viewport.
const W_TURNOVER: f32 = 80.0;

/// Render the Leaderboard screen body.
///
/// Called by `shell::screen_body` when `current_screen == Screen::Leaderboard`.
///
/// Layout, top-to-bottom: the headline/caption + Run button header, the F3
/// guided-input form (coin + budget + lookback), the budget-context line
/// ("Ranking strategies for €200 in XRPUSDT"), then the result body (the
/// `PanelState` table / prompt / spinner / error).
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let st = &model.leaderboard_screen_state;

    // Header: headline + caption on the left, the Run button on the right.
    let header = Row::new()
        .spacing(space::L)
        .align_y(iced::alignment::Vertical::Center)
        .push(header_text(mode))
        .push(Space::new().width(Length::Fill))
        .push(run_button(st.running, mode));

    // F3 guided input — the entry point to the whole journey: pick coin +
    // budget + lookback. Drives the next bake-off (the binary reads this state
    // to build the `BakeoffConfig`).
    // F7: pass the advisor EUR/USD rate so the FX hint is honest.
    // Tuning knobs: timeframe + start capital are wired in here.
    let guided_input = crate::widgets::bakeoff_input::view(
        &st.coin,
        &st.budget_input,
        st.lookback,
        st.timeframe,
        &st.start_capital_input,
        model.advisor_eur_usd_rate,
        mode,
    );

    // Budget-context line — carries the budget forward visually (the ranking
    // itself is budget-independent; this is shown for context per F3).
    let budget_context = budget_context_line(st, mode);

    let body = result_body(st, mode);

    let mut column = Column::new()
        .padding(space::L as u16)
        .spacing(space::L)
        .push(header)
        .push(guided_input);

    // advisor-bakeoff-progress — the DETERMINATE progress bar, BENEATH the
    // "Plan your bake-off" panel + the SAME width (`Length::Fill`, matching the
    // panel's fill width). Shown only while a run is in flight; once the first
    // `BakeoffProgress` event arrives it fills `done / total` and names the
    // running candidate. (The operator's headline ask — real "running X of N".)
    if let Some(bar) = progress_strip(st, mode) {
        column = column.push(bar);
    }

    column
        .push(budget_context)
        .push(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The determinate bake-off progress bar shown beneath the input panel while a
/// run is in flight — `None` when no run is running (so the strip vanishes
/// between runs). Once the first `BakeoffProgress` event lands it shows
/// "Running `{current_id}` — `{done+1}` of `{total}`" filled `done / total`;
/// before any event arrives (the brief pre-first-candidate window) it shows the
/// indeterminate sentinel with the loading copy, so the bar is never blank
/// during a run.
fn progress_strip(
    st: &crate::leaderboard::LeaderboardScreenState,
    mode: ThemeMode,
) -> Option<crate::Element<'static>> {
    if !st.running {
        return None;
    }
    match &st.progress {
        Some(p) if p.total > 0 => {
            // 1-based position: `done` = completed so far, so the running one is
            // `done + 1`. Fill is `done / total` (the fraction COMPLETED).
            let n = u32::from(p.done) + 1;
            let label = LEADERBOARD_PROGRESS_FMT
                .replace("{current}", display_label(p.current_id.as_str()))
                .replace("{n}", &n.to_string())
                .replace("{total}", &p.total.to_string());
            let fill = f32::from(p.done) / f32::from(p.total);
            Some(crate::widgets::progress_bar::view_block::<Message>(
                Some(fill),
                &label,
                mode,
            ))
        }
        // Running but no event yet (`done == 0 && total == 0`) — the indeterminate
        // sentinel with the loading copy, so the strip is never empty mid-run.
        _ => Some(crate::widgets::progress_bar::view_block::<Message>(
            None,
            LEADERBOARD_LOADING,
            mode,
        )),
    }
}

/// `true` when a real candidate-level progress event has arrived (`total > 0`)
/// — the determinate bar beneath the input panel is then showing real progress,
/// so the result body suppresses its fallback spinner. Mirrors the
/// `progress_strip` `total > 0` gate.
fn has_live_progress(st: &crate::leaderboard::LeaderboardScreenState) -> bool {
    matches!(&st.progress, Some(p) if p.total > 0)
}

/// The budget-context block shown under the guided input — the "Ranking
/// strategies for €200 in XRPUSDT." line plus a quiet arm-count note ("13
/// strategies head-to-head…", OQ-2) so the size of the field is self-explanatory
/// (a wider field takes proportionally longer). When the budget field is
/// blank/unparseable the budget clause drops but the coin is still named, so the
/// line never goes empty (no-blank-screen rule). Built from the structured
/// selection + the closed `ui`-side field count (the UI owns the copy; the
/// runtime values stay values).
fn budget_context_line(
    st: &crate::leaderboard::LeaderboardScreenState,
    mode: ThemeMode,
) -> crate::Element<'_> {
    let copy = match st.budget_eur() {
        Some(amount) => LEADERBOARD_BUDGET_CONTEXT_FMT
            .replace("{budget}", &crate::widgets::num::fmt_eur(amount))
            .replace("{coin}", st.coin.0.as_str()),
        None => LEADERBOARD_CONTEXT_NO_BUDGET_FMT.replace("{coin}", st.coin.0.as_str()),
    };
    // The arm-count note — how many strategies the bake-off ranks head-to-head.
    // Sourced from the real `advisor_field()` size (+1 for the appended
    // buy-and-hold benchmark) so it can never drift from the field that runs.
    let arm_count = crate::leaderboard::runner::advisor_field_arm_count();
    let arm_note = LEADERBOARD_FIELD_ARM_COUNT_FMT.replace("{count}", &arm_count.to_string());
    Column::new()
        .spacing(space::XXS)
        .push(
            Text::new(copy)
                .size(text::H3)
                .color(color::FG_2.current(mode)),
        )
        .push(
            Text::new(arm_note)
                .size(text::SMALL)
                .color(color::FG_3.current(mode)),
        )
        .into()
}

/// Headline (`H1`) over caption (`BODY`, muted) — the screen's plain-language
/// "what this is".
fn header_text(mode: ThemeMode) -> crate::Element<'static> {
    Column::new()
        .spacing(space::XXS)
        .push(
            Text::new(LEADERBOARD_HEADLINE)
                .size(text::H1)
                .color(color::FG_1.current(mode)),
        )
        .push(
            Text::new(LEADERBOARD_CAPTION)
                .size(text::BODY)
                .color(color::FG_3.current(mode))
                .width(Length::Fixed(640.0)),
        )
        .into()
}

/// The "Run bake-off" action button — `ACCENT`-filled (the "right" default
/// action). Disabled (no `on_press` + muted fill) while a bake-off is in
/// flight, with the label swapped to "Running…" so the disabled state is
/// legible beyond colour (accessibility minimum).
fn run_button(running: bool, mode: ThemeMode) -> crate::Element<'static> {
    let (label, fg, bg, on_press) = if running {
        (
            LEADERBOARD_RUN_BUTTON_RUNNING,
            color::FG_3.current(mode),
            color::PANEL_RAISED.current(mode),
            None,
        )
    } else {
        (
            LEADERBOARD_RUN_BUTTON,
            color::FG_ON_ACCENT.current(mode),
            color::ACCENT.current(mode),
            Some(Message::BakeoffRunRequested),
        )
    };

    let mut btn = Button::new(Text::new(label).size(text::BODY).color(fg))
        .padding([space::S as u16, space::L as u16])
        .style(move |_t: &iced::Theme, _s: button::Status| button::Style {
            background: Some(bg.into()),
            border: Border {
                color: bg,
                width: 1.0,
                radius: radius::R3.into(),
            },
            text_color: fg,
            ..Default::default()
        });
    if let Some(msg) = on_press {
        btn = btn.on_press(msg);
    }
    btn.into()
}

/// Dispatch on the result `PanelState` — every arm renders something (no blank
/// screen).
fn result_body(
    st: &crate::leaderboard::LeaderboardScreenState,
    mode: ThemeMode,
) -> crate::Element<'_> {
    match &st.result {
        // In-flight bake-off. The DETERMINATE progress bar beneath the input
        // panel (see `progress_strip`) is the real "running X of N" signal once
        // the first `BakeoffProgress` event lands, so the body here falls back to
        // the indeterminate spinner ONLY in the brief pre-first-event window
        // (`progress` still `None` / `total == 0`); after that the body stays
        // quiet (a second spinner/bar would be redundant with the strip above).
        PanelState::Loading => {
            let body: crate::Element<'_> = if has_live_progress(st) {
                // The strip above is showing real progress — keep the result area
                // calm (no duplicate spinner). A short padding holds the layout.
                Space::new()
                    .width(Length::Shrink)
                    .height(Length::Fixed(space::XL as f32))
                    .into()
            } else {
                Column::new()
                    .push(Space::new().height(Length::Fixed(space::XL as f32)))
                    .push(frame::loading_with_spinner(LEADERBOARD_LOADING, mode))
                    .into()
            };
            Container::new(body)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
        // Cold start — the "press Run bake-off" prompt (honest Empty). The copy
        // reflects the CURRENT selection so the operator sees exactly what the
        // next Run will do (F3): "…rank every strategy on {coin} over {lookback}".
        PanelState::Empty => prompt(empty_prompt_copy(st), mode),
        // The run failed — prefix + the engine's detail (never a bare "no data").
        PanelState::Error(detail) => error_pane(detail, mode),
        // The ranked leaderboard + recommendation (+ the F9 narration state).
        // `coin` + `lookback` are threaded down so each data row can build the
        // `InspectStrategyFromLeaderboard` message (click → inspect in the Lab
        // on the SAME coin/window the bake-off ranked).
        PanelState::Ready(report) => ready_pane(report, &st.narration, &st.coin, st.lookback, mode),
    }
}

/// A centred prompt/empty message — never a blank surface. Takes the copy by
/// value (the empty-prompt copy is computed from the live selection).
fn prompt(copy: String, mode: ThemeMode) -> crate::Element<'static> {
    Container::new(
        Column::new()
            .push(Space::new().height(Length::Fixed(space::XL as f32)))
            .push(
                Text::new(copy)
                    .size(text::BODY)
                    .color(color::FG_3.current(mode)),
            ),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Fill the empty-prompt template with the current coin + lookback so the cold
/// surface tells the operator exactly what the next Run will rank (F3).
fn empty_prompt_copy(st: &crate::leaderboard::LeaderboardScreenState) -> String {
    LEADERBOARD_EMPTY_PROMPT
        .replace("{coin}", st.coin.0.as_str())
        .replace(
            "{lookback}",
            crate::widgets::bakeoff_input::lookback_copy(st.lookback),
        )
}

/// The error surface — `LEADERBOARD_ERROR_PREFIX` + the engine's detail, so the
/// operator sees what happened and what to check (the human-friendliness
/// rule), with the disclaimer still present.
fn error_pane(detail: &str, mode: ThemeMode) -> crate::Element<'_> {
    let prefix = Text::new(LEADERBOARD_ERROR_PREFIX)
        .size(text::H3)
        .color(color::WARN_500.current(mode));
    let detail_text = Text::new(detail)
        .size(text::BODY)
        .color(color::FG_2.current(mode));
    let body = Column::new()
        .spacing(space::S)
        .push(Space::new().height(Length::Fixed(space::M as f32)))
        .push(prefix)
        .push(detail_text)
        .push(Space::new().height(Length::Fixed(space::L as f32)))
        .push(disclaimer(mode));
    Container::new(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The happy path — recommendation block, the ranked table, and the persistent
/// disclaimer, stacked vertically in a scrollable column (the table can be
/// taller than the viewport with the full field + opt-in arms).
fn ready_pane<'a>(
    report: &'a BakeoffReportMirror,
    narration: &'a NarrationState,
    coin: &Symbol,
    lookback: LeaderboardLookback,
    mode: ThemeMode,
) -> crate::Element<'a> {
    let recommendation = recommendation_block(report, narration, mode);
    let table = leaderboard_table(report, coin, lookback, mode);

    // advisor-data-quality-surface (P1-7) — the DATA-stage trust/quality
    // readout, rendered FIRST (above the recommendation + table) so the
    // workflow spine reads DATA → ANALYSIS → SUGGEST: "here's what the
    // numbers are built on" before "here's the pick". Always present
    // (`data_quality` is not `Option` — every bake-off has a known symbol).
    // DISPLAY-ONLY: never feeds the crown/rank/gate.
    let data_quality = data_quality_block(&report.data_quality, mode);

    let mut stack = Column::new()
        .spacing(space::L)
        .push(data_quality)
        .push(recommendation)
        .push(table);

    // advisor-overfitting-scorecard (P0-1 / ADR-0075) — the "show your work"
    // honesty readout. Rendered DIRECTLY UNDER the ranked table (so it reads as
    // "here's the pick, and here's how much to trust it") ONLY when the report
    // carries a (non-degenerate) scorecard; absent → no block (the negative
    // control). Placed below the table — not between recommendation and table —
    // so the ranked rows keep their position in the result pane. REPORT-ONLY:
    // display-only, never the verdict.
    if let Some(sc) = &report.scorecard {
        stack = stack.push(scorecard_block(sc, mode));
    }

    // advisor-turnover-and-tail-metrics (P1-2) — the "Risk story" block. Sits
    // BELOW the scorecard so the two honesty layers pair: trust (scorecard) +
    // risk (tail/median). Rendered ONLY when the report carries the crown's
    // tail summary; absent → no block (the negative control). Sortino/Calmar
    // come from the crowned row (`CandidateKpis` already carried them for
    // narration). REPORT-ONLY — display-only, never changes the pick.
    if let Some(tail) = &report.tail
        && let Some(crown) = report.crowned_row()
    {
        stack = stack.push(risk_story_block(tail, crown, mode));
    }

    // advisor-short-selling (T-U2 / T-U4) — when the field contains one or more
    // short-capable arms, carry the short field-note + the load-bearing
    // unbounded-loss disclaimer ABOVE the persistent not-advice disclaimer.
    if field_has_short_arm(report) {
        stack = stack.push(short_field_block(mode));
    }

    let stack = stack.push(disclaimer(mode)).width(Length::Fill);

    Scrollable::new(stack)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// `true` when the bake-off field contains at least one short-capable arm
/// (advisor-short-selling, ADR-0068 § D9). Gates the short field-note + the
/// unbounded-loss disclaimer so a long-only field is byte-identical to pre-feature.
fn field_has_short_arm(report: &BakeoffReportMirror) -> bool {
    report
        .rows
        .iter()
        .any(|r| is_short_capable_id(r.strategy.as_str()))
}

/// The short-field block (advisor-short-selling, T-U2 / T-U4) — the
/// "short-capable arms can bet on a decline; the drawdown can be brutal" note
/// over the load-bearing "a short can lose MORE than your €200" unbounded-loss
/// disclaimer. The unbounded-loss line is `WARN_500`-tinted (paired with the
/// word, so colour is never the only signal) so it reads as a real risk note;
/// the field-note is muted `FG_3`. Rendered only when the field has a short arm.
fn short_field_block(mode: ThemeMode) -> crate::Element<'static> {
    Column::new()
        .spacing(space::XS)
        .push(
            Text::new(LEADERBOARD_SHORT_FIELD_NOTE)
                .size(text::SMALL)
                .color(color::FG_3.current(mode))
                .width(Length::Fill),
        )
        .push(
            Text::new(SHORT_UNBOUNDED_LOSS_DISCLAIMER)
                .size(text::SMALL)
                .color(color::WARN_500.current(mode))
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .into()
}

// ── Recommendation block ──────────────────────────────────────────────────────

/// The recommendation block — a `frame::panel` titled "Recommendation" holding
/// the plain-language headline (rendered FROM the structured `Recommendation`)
/// + the winner-robustness clause + the F9 narration section.
///
/// The headline + the winner-robustness clause are ALWAYS present (the
/// structured floor). Below them, the narration section (F9, ADR-0064 § D7)
/// renders one of four states — the templated reasons are the floor in every
/// arm except `Ready`, so there is NEVER a blank or half-answer:
///
/// - `NotRequested` → templated reasons + the **Explain** control.
/// - `InFlight`     → templated reasons + a spinner/"writing…" affordance.
/// - `Ready(prose)` → the LLM prose, **labelled** as an AI summary of the
///   numbers above (the templated reasons are subsumed by the richer prose).
/// - `FellBack`     → templated reasons + a quiet "couldn't generate" note.
fn recommendation_block<'a>(
    report: &'a BakeoffReportMirror,
    narration: &'a NarrationState,
    mode: ThemeMode,
) -> crate::Element<'a> {
    let rec = &report.recommendation;

    let headline = Text::new(headline_copy(report))
        .size(text::H2)
        .color(color::FG_1.current(mode));

    let mut col = Column::new().spacing(space::S).push(headline);

    // The winner-robustness clause (only when the gate ran for the winner).
    if let Some(clause) = winner_robustness_clause(rec) {
        let (clause_text, clause_color) = clause;
        col = col.push(Text::new(clause_text).size(text::BODY).color(clause_color));
    }

    // ── F9 narration section — replaces the reasons block on `Ready`, augments
    // it otherwise. The templated reasons are the floor in every arm but Ready.
    col = col.push(narration_section(rec, narration, mode));

    frame::panel(LEADERBOARD_RECOMMENDATION_TITLE, col.into(), mode)
}

/// The F9 narration section (ADR-0064 § D7) — dispatches on the closed
/// `ui`-owned [`NarrationState`]. The structured templated reasons are the
/// FLOOR in every arm but `Ready`; on `Ready` the richer LLM prose stands in
/// for them (the same facts, in fluent plain language), labelled as AI-generated
/// and anchored to the numbers above.
fn narration_section<'a>(
    rec: &'a RecommendationMirror,
    narration: &'a NarrationState,
    mode: ThemeMode,
) -> crate::Element<'a> {
    match narration {
        // No "Explain" yet — the templated reasons + the opt-in Explain control.
        NarrationState::NotRequested => Column::new()
            .spacing(space::S)
            .push(templated_reasons(rec, mode))
            .push(explain_control(mode))
            .into(),
        // Generating — the templated reasons stay visible (the floor) + a quiet
        // spinner line so the operator knows work is happening.
        NarrationState::InFlight => Column::new()
            .spacing(space::S)
            .push(templated_reasons(rec, mode))
            .push(frame::loading_with_spinner(
                LEADERBOARD_EXPLAIN_INFLIGHT,
                mode,
            ))
            .into(),
        // A faithful narration arrived — show the LLM prose (labelled), in a
        // subtly-distinct AI-summary card. The persistent `disclaimer()` still
        // surrounds the whole block (it sits below in `ready_pane`).
        NarrationState::Ready(prose) => llm_summary_card(prose.as_str(), mode),
        // The narration fell back (disabled / error / budget / post-check
        // reject) — the templated reasons (the honest floor) + a quiet note.
        NarrationState::FellBack => Column::new()
            .spacing(space::S)
            .push(templated_reasons(rec, mode))
            .push(
                Text::new(LEADERBOARD_EXPLAIN_FELLBACK)
                    .size(text::MICRO)
                    .color(color::FG_3.current(mode)),
            )
            .into(),
    }
}

/// The structured templated reasons — one muted "· reason" line each
/// (deterministic order). The honest floor F9 falls back to; shown in every
/// narration state but `Ready`. Returns an empty (zero-height) element when
/// there are no reasons (the headline already carries the verdict).
fn templated_reasons(rec: &RecommendationMirror, mode: ThemeMode) -> crate::Element<'_> {
    if rec.reasons.is_empty() {
        return Space::new()
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into();
    }
    let mut reasons = Column::new().spacing(space::XXS);
    for reason in &rec.reasons {
        reasons = reasons.push(
            Text::new(format!("\u{00b7} {}", reason_copy(*reason)))
                .size(text::BODY)
                .color(color::FG_3.current(mode)),
        );
    }
    reasons.into()
}

/// The opt-in "Explain" control (ADR-0064 § D3, U1) — a quiet GHOST button (a
/// soft `ACCENT_SOFT` fill + an `ACCENT` label, no heavy chrome) so it reads as
/// a secondary, optional affordance next to the always-honest structured copy,
/// not a primary call-to-action. Posts `Message::BakeoffNarrationRequested`.
/// Shows only in the `NotRequested` state.
fn explain_control(mode: ThemeMode) -> crate::Element<'static> {
    Button::new(
        Text::new(LEADERBOARD_EXPLAIN_BUTTON)
            .size(text::SMALL)
            .color(color::ACCENT.current(mode)),
    )
    .padding([space::XS as u16, space::M as u16])
    .style(move |_t: &iced::Theme, _s: button::Status| button::Style {
        background: Some(color::ACCENT_SOFT.current(mode).into()),
        border: Border {
            color: color::ACCENT.current(mode),
            width: 1.0,
            radius: radius::R3.into(),
        },
        text_color: color::ACCENT.current(mode),
        ..Default::default()
    })
    .on_press(Message::BakeoffNarrationRequested)
    .into()
}

/// The LLM-summary card (ADR-0064 § D7 / R4, U2) — the `Ready` state. A
/// subtly-distinct `ACCENT_SOFT`-tinted bordered card holding:
/// - the AI-summary LABEL (`MICRO`, accent) naming the prose as an AI-generated
///   summary of the numbers above (so the operator always sees the structured
///   result the words describe — never free-floating analysis), and
/// - the LLM prose (`BODY`, foreground).
///
/// The persistent not-advice / simulated-€200 `disclaimer()` still surrounds
/// the whole recommendation block (it sits below in `ready_pane`), so the
/// narration is framed top (label) and bottom (disclaimer).
fn llm_summary_card(prose: &str, mode: ThemeMode) -> crate::Element<'_> {
    let label = Text::new(LEADERBOARD_EXPLAIN_LLM_LABEL)
        .size(text::MICRO)
        .color(color::ACCENT.current(mode))
        .width(Length::Fill);

    let body = Text::new(prose)
        .size(text::BODY)
        .color(color::FG_1.current(mode))
        .width(Length::Fill);

    let inner = Column::new().spacing(space::XS).push(label).push(body);

    Container::new(inner)
        .width(Length::Fill)
        .padding(space::M as u16)
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::ACCENT_SOFT.current(mode).into()),
            border: Border {
                color: color::ACCENT.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

/// Build the headline string from the structured outcome (the UI owns copy).
///
/// `{coin}` / `{winner}` placeholders are filled here — the prose stays in
/// `strings`, the runtime values stay values.
fn headline_copy(report: &BakeoffReportMirror) -> String {
    match report.recommendation.outcome {
        OutcomeKind::BenchmarkWins => {
            LEADERBOARD_HEADLINE_BENCHMARK_WINS.replace("{coin}", report.coin.as_str())
        }
        OutcomeKind::ActiveWins => LEADERBOARD_HEADLINE_ACTIVE_WINS
            .replace("{winner}", report.recommendation.winner.as_str()),
        OutcomeKind::AllFragile => LEADERBOARD_HEADLINE_ALL_FRAGILE.to_string(),
    }
}

/// The winner-robustness clause + its colour, or `None` when the gate did not
/// run for the winner (`RobustnessMode::Skip`).
fn winner_robustness_clause(rec: &RecommendationMirror) -> Option<(&'static str, iced::Color)> {
    // Colour echoes the principle: pos/neg/warn only. Robust → muted (it's a
    // reassurance, not a celebration); fragile → warn (a caution).
    match rec.winner_robustness {
        Some(RobustnessLabel::Robust) => Some((
            LEADERBOARD_WINNER_ROBUST_CLAUSE,
            color::FG_3.current(ThemeMode::Dark),
        )),
        Some(RobustnessLabel::Fragile) => Some((
            LEADERBOARD_WINNER_FRAGILE_CLAUSE,
            color::WARN_500.current(ThemeMode::Dark),
        )),
        // Marginal / NotChecked / None → no clause (the reasons carry nuance).
        _ => None,
    }
}

/// Map a reason code to its one-line copy.
fn reason_copy(reason: ReasonLabel) -> &'static str {
    match reason {
        ReasonLabel::HighestRobustSharpe => LEADERBOARD_REASON_HIGHEST_ROBUST_SHARPE,
        ReasonLabel::BeatBenchmarkSharpe => LEADERBOARD_REASON_BEAT_BENCHMARK_SHARPE,
        ReasonLabel::BenchmarkUndefeated => LEADERBOARD_REASON_BENCHMARK_UNDEFEATED,
        ReasonLabel::AllCandidatesFragile => LEADERBOARD_REASON_ALL_FRAGILE,
        ReasonLabel::TieBrokenByReturn => LEADERBOARD_REASON_TIE_RETURN,
        ReasonLabel::TieBrokenByDrawdown => LEADERBOARD_REASON_TIE_DRAWDOWN,
    }
}

// ── DATA-quality block (advisor-data-quality-surface P1-7) ────────────────────

/// The "Data quality" DATA-stage readout — a `frame::panel`-titled block
/// rendering the DISPLAY-ONLY [`DataQualityView`] in plain language (P1-7 /
/// v2-architecture §1 P1-7).
///
/// Sits ABOVE the recommendation + ranked table (the DATA → ANALYSIS →
/// SUGGEST workflow spine — this is the first honesty layer, describing the
/// INPUT rather than the output). The bottom carries the load-bearing
/// "informational, not a gate" note, matching the scorecard's + Risk story's
/// framing, so the operator can never mistake it for a verdict.
///
/// Facts, each `label : value`:
/// - **Venue** — where the price series is sourced from.
/// - **Provenance** — the one-line sourcing mechanics.
/// - **Trust level** — the venue-trust-map tier badge.
/// - **Survival bias** — ALWAYS present (every universe symbol survived to
///   today by construction).
/// - **Warnings** — rendered ONLY when non-empty (the honest "nothing to
///   flag" case renders no row at all for the default deep-liquidity
///   universe, not a placeholder "no warnings" line).
fn data_quality_block(dq: &DataQualityView, mode: ThemeMode) -> crate::Element<'static> {
    let caption = Text::new(LEADERBOARD_DATA_QUALITY_CAPTION)
        .size(text::SMALL)
        .color(color::FG_3.current(mode))
        .width(Length::Fill);

    let venue = scorecard_fact(
        LEADERBOARD_DATA_QUALITY_VENUE_LABEL,
        dq.venue.clone(),
        None,
        color::FG_1.current(mode),
        mode,
    );

    let provenance = scorecard_fact(
        LEADERBOARD_DATA_QUALITY_PROVENANCE_LABEL,
        dq.provenance.clone(),
        None,
        color::FG_3.current(mode),
        mode,
    );

    // Trust level — neutral colour (NOT pos/neg): `HighReconcilable` is the
    // expected default for the pinned corpus, not a "win"; a lower tier
    // (reserved for future symbols) is a caveat to read, not a loss.
    let trust = scorecard_fact(
        LEADERBOARD_DATA_QUALITY_TRUST_LABEL,
        dq.venue_trust.badge_label().to_string(),
        None,
        color::FG_1.current(mode),
        mode,
    );

    // Survival bias — ALWAYS present. Muted colour: it's a caveat on the
    // universe, not a sentiment-bearing figure.
    let survival = scorecard_fact(
        LEADERBOARD_DATA_QUALITY_SURVIVAL_LABEL,
        dq.survival_note.clone(),
        None,
        color::FG_3.current(mode),
        mode,
    );

    let mut body = Column::new()
        .spacing(space::M)
        .push(caption)
        .push(venue)
        .push(provenance)
        .push(trust)
        .push(survival);

    // Warnings — rendered ONLY when non-empty (the negative control: the
    // default deep-liquidity universe carries zero warnings, so this whole
    // section is absent rather than an empty placeholder row).
    if !dq.warnings.is_empty() {
        let warnings_label = Text::new(LEADERBOARD_DATA_QUALITY_WARNINGS_LABEL)
            .size(text::MICRO)
            .color(color::FG_3.current(mode));
        let mut warnings_col = Column::new().spacing(space::XXS).push(warnings_label);
        for w in &dq.warnings {
            warnings_col = warnings_col.push(
                Text::new(w.copy())
                    .size(text::BODY)
                    .color(color::WARN_500.current(mode))
                    .width(Length::Fill),
            );
        }
        body = body.push(warnings_col);
    }

    // The bottom "informational, not a gate" note — DISPLAY-ONLY (P1-7).
    let info_note = Text::new(LEADERBOARD_DATA_QUALITY_INFORMATIONAL_NOTE)
        .size(text::MICRO)
        .color(color::FG_3.current(mode))
        .width(Length::Fill);
    body = body.push(info_note);

    frame::panel(LEADERBOARD_DATA_QUALITY_TITLE, body.into(), mode)
}

// ── Scorecard block (advisor-overfitting-scorecard P0-1 / ADR-0075) ───────────

/// The "How much to trust this" / "show your work" honesty readout — a
/// `frame::panel`-titled block rendering the REPORT-ONLY [`ScorecardView`] in
/// plain language (ADR-0075 / §6.0 D3).
///
/// Reads as an honesty self-check, NEVER a verdict: the title + caption frame it
/// as "an honesty check on the search", and the "Beats holding after the
/// search?" row carries the load-bearing "informational, not a gate" note so the
/// operator can never mistake it for the pick. When buy-and-hold is crowned (the
/// modal case) the block still reads sensibly — the "Not clearly — holding is the
/// honest call" value is the expected, fine answer, not a failure.
///
/// Four facts, each `label : value` with a one-line plain-language gloss for the
/// terms of art (`DSR`, `MinBTL`) per the no-jargon rule:
/// - **Strategies tried** — `n_candidates` (+ the effective count in plain words).
/// - **Deflated confidence** — `deflated_sharpe` as a %, glossed.
/// - **Minimum history needed** — `min_btl_years` years, glossed.
/// - **Beats holding after the search?** — `crown_clears_dsr` yes/no + the
///   informational-not-a-gate note.
fn scorecard_block(sc: &ScorecardView, mode: ThemeMode) -> crate::Element<'static> {
    let caption = Text::new(LEADERBOARD_SCORECARD_CAPTION)
        .size(text::SMALL)
        .color(color::FG_3.current(mode))
        .width(Length::Fill);

    // Strategies tried — raw N, with the effective count in plain words.
    let tried_value = format!(
        "{} \u{2014} {}",
        sc.n_candidates,
        LEADERBOARD_SCORECARD_TRIED_EFFECTIVE_FMT.replace("{n_eff}", &round_count(sc.n_eff))
    );
    let tried = scorecard_fact(
        LEADERBOARD_SCORECARD_TRIED_LABEL,
        tried_value,
        None,
        color::FG_1.current(mode),
        mode,
    );

    // Deflated confidence — DSR as a percentage, with the plain-language gloss.
    let confidence = scorecard_fact(
        LEADERBOARD_SCORECARD_CONFIDENCE_LABEL,
        fmt_probability_pct(sc.deflated_sharpe),
        Some(LEADERBOARD_SCORECARD_CONFIDENCE_HINT),
        color::FG_1.current(mode),
        mode,
    );

    // Minimum history needed — MinBTL in years, with the gloss.
    let history = scorecard_fact(
        LEADERBOARD_SCORECARD_HISTORY_LABEL,
        LEADERBOARD_SCORECARD_HISTORY_FMT.replace("{years}", &round_years(sc.min_btl_years)),
        Some(LEADERBOARD_SCORECARD_HISTORY_HINT),
        color::FG_1.current(mode),
        mode,
    );

    // Beats holding after the search? — the yes/no + the load-bearing
    // informational-not-a-gate note (REPORT-ONLY). The value is muted FG_1 (NOT
    // pos/neg) — a "no" here is the honest modal case, not a loss, so it must not
    // wear the red sentiment colour. The ✓/✗ glyph carries the signal beyond
    // colour (accessibility — colour is never the only signal).
    let beats_value = if sc.crown_clears_dsr {
        LEADERBOARD_SCORECARD_BEATS_HOLD_YES
    } else {
        LEADERBOARD_SCORECARD_BEATS_HOLD_NO
    };
    let beats = scorecard_fact(
        LEADERBOARD_SCORECARD_BEATS_HOLD_LABEL,
        beats_value.to_string(),
        Some(LEADERBOARD_SCORECARD_INFORMATIONAL_NOTE),
        color::FG_1.current(mode),
        mode,
    );

    let body = Column::new()
        .spacing(space::M)
        .push(caption)
        .push(tried)
        .push(confidence)
        .push(history)
        .push(beats);

    frame::panel(LEADERBOARD_SCORECARD_TITLE, body.into(), mode)
}

/// One scorecard fact — a `label` (muted `MICRO`) over a `value` (`H3`,
/// `value_color`), with an optional one-line plain-language `hint` (muted
/// `SMALL`) beneath. The label-over-value shape keeps each fact scannable as a
/// small stat, and the hint glosses the terms of art (`DSR` / `MinBTL`) inline
/// so there is no undefined jargon (the no-jargon human-friendliness rule).
fn scorecard_fact(
    label: &'static str,
    value: String,
    hint: Option<&'static str>,
    value_color: iced::Color,
    mode: ThemeMode,
) -> crate::Element<'static> {
    let mut col = Column::new()
        .spacing(space::XXS)
        .push(
            Text::new(label)
                .size(text::MICRO)
                .color(color::FG_3.current(mode)),
        )
        .push(Text::new(value).size(text::H3).color(value_color));
    if let Some(h) = hint {
        col = col.push(
            Text::new(h)
                .size(text::SMALL)
                .color(color::FG_3.current(mode))
                .width(Length::Fill),
        );
    }
    col.width(Length::Fill).into()
}

/// Format a `[0, 1]` probability (the `DSR`) as a whole-percent string, e.g.
/// `0.62 → "62%"`. Clamped to `[0, 1]` defensively (the engine already returns
/// in-range, but the UI must never render a nonsense `-3%` / `140%`). Rounded to
/// the nearest whole percent — sub-percent precision is noise for a confidence
/// readout the operator scans at a glance.
fn fmt_probability_pct(p: f64) -> String {
    let clamped = p.clamp(0.0, 1.0);
    // `clamped * 100` is in `[0, 100]`; `.round()` lands on an integer that
    // fits a u32 with room to spare — the cast is lossless and in-range.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pct = (clamped * 100.0).round() as u32;
    format!("{pct}%")
}

/// Round an effective-trial count (`n_eff`) to a friendly whole number for the
/// "about N truly independent" copy. `n_eff` is always `≥ 1.0`; we floor the
/// rounding at 1 so it never reads "about 0".
fn round_count(n_eff: f64) -> String {
    // `n_eff` is bounded `[1.0, n_candidates]` (a small count) → `.round()` fits
    // a u32 trivially; the cast is lossless and in-range.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = (n_eff.round() as u32).max(1);
    n.to_string()
}

/// Round a `MinBTL` year figure to one decimal place for display, e.g.
/// `6.36 → "6.4"`. Clamped at `0.0` (`MinBTL` is `≥ 0` by construction).
fn round_years(years: f64) -> String {
    let y = years.max(0.0);
    format!("{y:.1}")
}

// ── Risk story block (advisor-turnover-and-tail-metrics P1-2) ─────────────────

/// The "Risk story" honesty readout — a `frame::panel`-titled block rendering
/// the REPORT-ONLY [`TailSummaryView`] + the crowned row's Sortino/Calmar in
/// plain language (v2-architecture §1 P1-2).
///
/// Reads as a tail/median honesty self-check, NEVER a verdict: the title +
/// caption frame it as "what the bad days look like, and what the typical
/// outcome is", and the bottom carries the load-bearing "informational, not a
/// gate" note so the operator can never mistake it for the pick.
///
/// Six facts, each `label : value` with a one-line plain-language gloss for
/// the terms of art (`CVaR`, skew, Sortino, Calmar) per the no-jargon rule:
/// - **Typical outcome (median)** — `median_terminal_wealth` in USDT.
/// - **Worst-5 % `CVaR`** + **worst-1 % `CVaR`** — paired (one gloss covers both).
/// - **Surprise shape (skew)** — `skew`, signed (positive = lottery).
/// - **Sortino + Calmar** — the crown's downside-only + drawdown-adjusted
///   Sharpe analogues (already on `CandidateKpis`; surfaced here as part of
///   the "tail/median honesty layer").
fn risk_story_block(
    tail: &TailSummaryView,
    crown: &LeaderRow,
    mode: ThemeMode,
) -> crate::Element<'static> {
    let caption = Text::new(LEADERBOARD_RISK_STORY_CAPTION)
        .size(text::SMALL)
        .color(color::FG_3.current(mode))
        .width(Length::Fill);

    // Typical outcome (median) — the honest "middle outcome" in USDT. Neutral
    // colour (not pos/neg) — a typical outcome can be above or below the
    // initial budget; the sentiment doesn't drive the read.
    let median = risk_story_fact(
        LEADERBOARD_RISK_STORY_MEDIAN_LABEL,
        crate::widgets::num::fmt_usdt(
            rust_decimal::Decimal::try_from(tail.median_terminal_wealth)
                .unwrap_or(rust_decimal::Decimal::ZERO),
        ),
        Some(LEADERBOARD_RISK_STORY_MEDIAN_HINT),
        color::FG_1.current(mode),
        mode,
    );

    // Worst-5 % CVaR + Worst-1 % CVaR — paired rows, one shared CVaR gloss
    // beneath the second so the term-of-art is defined once. Coloured DOWN_500
    // (these are losses by construction — the bad-tail of the distribution).
    let cvar_95 = risk_story_fact(
        LEADERBOARD_RISK_STORY_CVAR_95_LABEL,
        fmt_signed_pct_from_f64(tail.cvar_95),
        None,
        color::DOWN_500.current(mode),
        mode,
    );
    let cvar_99 = risk_story_fact(
        LEADERBOARD_RISK_STORY_CVAR_99_LABEL,
        fmt_signed_pct_from_f64(tail.cvar_99),
        Some(LEADERBOARD_RISK_STORY_CVAR_HINT),
        color::DOWN_500.current(mode),
        mode,
    );

    // Surprise shape (skew) — signed, three decimal places. Neutral colour:
    // positive AND negative skew are both informational, neither is "good" or
    // "bad" — the gloss explains the sign meaning. The sign and magnitude
    // carry the signal (a `+`/`-` prefix is always present beyond colour).
    let skew = risk_story_fact(
        LEADERBOARD_RISK_STORY_SKEW_LABEL,
        format_signed_decimal(tail.skew, 2),
        Some(LEADERBOARD_RISK_STORY_SKEW_HINT),
        color::FG_1.current(mode),
        mode,
    );

    // Sortino + Calmar — the crown's already-carried downside-only +
    // drawdown-adjusted ratios. Neutral colour for both — they're risk-adjusted
    // ratios, not P&L, so the "good/bad" reading depends on context.
    let sortino = risk_story_fact(
        LEADERBOARD_RISK_STORY_SORTINO_LABEL,
        format_signed_decimal(crown.sortino, 2),
        Some(LEADERBOARD_RISK_STORY_SORTINO_HINT),
        color::FG_1.current(mode),
        mode,
    );
    let calmar = risk_story_fact(
        LEADERBOARD_RISK_STORY_CALMAR_LABEL,
        format_signed_decimal(crown.calmar, 2),
        Some(LEADERBOARD_RISK_STORY_CALMAR_HINT),
        color::FG_1.current(mode),
        mode,
    );

    // The bottom "informational, not a gate" note — REPORT-ONLY (§1 P1-2).
    let info_note = Text::new(LEADERBOARD_RISK_STORY_INFORMATIONAL_NOTE)
        .size(text::MICRO)
        .color(color::FG_3.current(mode))
        .width(Length::Fill);

    let body = Column::new()
        .spacing(space::M)
        .push(caption)
        .push(median)
        .push(cvar_95)
        .push(cvar_99)
        .push(skew)
        .push(sortino)
        .push(calmar)
        .push(info_note);

    frame::panel(LEADERBOARD_RISK_STORY_TITLE, body.into(), mode)
}

/// One Risk-story fact — same shape as [`scorecard_fact`] (label `MICRO` muted
/// over a `value` `H3` `value_color` + optional one-line `hint` `SMALL` muted).
/// Composed here so the two blocks are visually peer (both honesty readouts).
fn risk_story_fact(
    label: &'static str,
    value: String,
    hint: Option<&'static str>,
    value_color: iced::Color,
    mode: ThemeMode,
) -> crate::Element<'static> {
    scorecard_fact(label, value, hint, value_color, mode)
}

/// Format a signed `f64` fraction (e.g. `-0.18` = a 18 % loss) as a signed
/// percentage string with explicit `+`/`-` prefix.  Used for the `CVaR` rows —
/// `CVaR` is always `≤ 0` by construction (the worst-tail mean of total
/// returns), so the leading `-` is the operator-visible "you lose this much".
fn fmt_signed_pct_from_f64(fraction: f64) -> String {
    let pct = fraction * 100.0;
    // Display only — round to one decimal place. Bounded `[-1, +infty)` in
    // practice (a 100 % loss caps the worst path); the cast for the sign check
    // is on `f64.is_sign_negative()` (no truncation).
    if fraction.is_nan() {
        // Defensive: NaN should not arrive (engine returns `f64::NAN` only for
        // empty inputs which the bootstrap guards against), but render a
        // visible placeholder rather than `NaN%`.
        return "\u{2014}".to_string();
    }
    if pct == 0.0 || fraction == 0.0 {
        return "0.0%".to_string();
    }
    if fraction.is_sign_negative() {
        format!("{}{:.1}%", crate::strings::MINUS_SIGN_LITERAL, pct.abs())
    } else {
        format!("+{pct:.1}%")
    }
}

/// Format a signed `f64` to `N` decimal places with explicit `+`/`-` prefix
/// (using the unicode `MINUS_SIGN_LITERAL`).  Used for the skew, Sortino, and
/// Calmar values — the sign carries the directional read (positive lottery /
/// negative crash for skew; positive risk-adjusted / negative for the ratios).
fn format_signed_decimal(value: f64, decimals: usize) -> String {
    if value.is_nan() {
        return "\u{2014}".to_string();
    }
    let abs = value.abs();
    let body = format!("{abs:.decimals$}");
    if value == 0.0 {
        body
    } else if value.is_sign_negative() {
        format!("{}{body}", crate::strings::MINUS_SIGN_LITERAL)
    } else {
        format!("+{body}")
    }
}

// ── The ranked table ──────────────────────────────────────────────────────────

/// The leaderboard table — a header row + one row per candidate, in `ranked`
/// (best-first) order. The crowned row gets the `ACCENT` treatment; the
/// benchmark row gets the `benchmark` tag.
fn leaderboard_table<'a>(
    report: &'a BakeoffReportMirror,
    coin: &Symbol,
    lookback: LeaderboardLookback,
    mode: ThemeMode,
) -> crate::Element<'a> {
    let mut col = Column::new().spacing(space::XXS).push(header_row(mode));

    // Iterate in ranked (best-first) order. `rank` is the 1-based display
    // position; `crowned` marks the accent row.
    for (rank, &row_idx) in report.ranked.iter().enumerate() {
        let Some(leader) = report.rows.get(row_idx) else {
            continue; // defensive: ranked indices are always in-range
        };
        let is_crowned = report.crowned == Some(row_idx);
        col = col.push(data_row(rank + 1, leader, is_crowned, coin, lookback, mode));
    }

    // The table needs no titled frame header (it has its own column-header
    // row), so wrap it in the standard PANEL surface directly — `PANEL` bg +
    // `BORDER_1` hairline + `R4` radius, the same chrome `frame::panel` applies
    // (composition with existing tokens; no new widget).
    Container::new(col)
        .width(Length::Fill)
        .padding(space::M as u16)
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL.current(mode).into()),
            border: Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

/// The table header row — column labels in `MICRO` muted (the col-header
/// convention). Numeric columns right-aligned to match the data rows.
///
/// **P1-1 turnover column ("Churn"):** the rightmost numeric column, sized
/// narrower than the other numerics because turnover values are short
/// ("0.0×" / "1.4×" / "12.7×"). Right of Trades because it's the same
/// "trading-activity" half of the row (Trades = count; Churn = capital
/// equivalents) — kept together for scanning.
fn header_row(mode: ThemeMode) -> crate::Element<'static> {
    Row::new()
        .spacing(space::S)
        .padding([space::XS as u16, space::S as u16])
        .push(col_head(LEADERBOARD_COL_RANK, W_RANK, false, mode))
        .push(col_head_fill(LEADERBOARD_COL_STRATEGY, mode))
        .push(col_head(LEADERBOARD_COL_RETURN, W_NUM, true, mode))
        .push(col_head(LEADERBOARD_COL_SHARPE, W_NUM, true, mode))
        .push(col_head(LEADERBOARD_COL_MAX_DD, W_NUM, true, mode))
        .push(col_head(LEADERBOARD_COL_TRADES, W_NUM, true, mode))
        .push(col_head(LEADERBOARD_COL_TURNOVER, W_TURNOVER, true, mode))
        .into()
}

/// A fixed-width column header. `right` right-aligns (numeric columns).
fn col_head(
    label: &'static str,
    width: f32,
    right: bool,
    mode: ThemeMode,
) -> crate::Element<'static> {
    let mut t = Text::new(label)
        .size(text::MICRO)
        .color(color::FG_3.current(mode))
        .width(Length::Fixed(width));
    if right {
        t = t.align_x(iced::alignment::Horizontal::Right);
    }
    t.into()
}

/// The filling (strategy) column header.
fn col_head_fill(label: &'static str, mode: ThemeMode) -> crate::Element<'static> {
    Text::new(label)
        .size(text::MICRO)
        .color(color::FG_3.current(mode))
        .width(Length::Fill)
        .into()
}

/// One candidate data row. The crowned row uses `frame::active_row` (the 2 px
/// `ACCENT` left rule) + `ACCENT` strategy text + a `★ best` tag; non-crowned
/// rows are neutral. The benchmark row carries a `benchmark` tag. Numbers are
/// right-aligned with `pos`/`neg`/`warn`-only colour.
///
/// Splitting this further would obscure the row-layout sequence (rank → strat
/// + tags → numeric cells → click-wrap); the extras are the inline tag flags
///   (crown / vote / short / benchmark / robustness) + the new `turnover_cell`.
#[allow(clippy::too_many_lines)]
fn data_row<'a>(
    rank: usize,
    leader: &'a LeaderRow,
    is_crowned: bool,
    coin: &Symbol,
    lookback: LeaderboardLookback,
    mode: ThemeMode,
) -> crate::Element<'a> {
    // Rank cell.
    let rank_cell = Text::new(format!("{rank}"))
        .size(text::BODY)
        .color(color::FG_3.current(mode))
        .width(Length::Fixed(W_RANK));

    // Strategy cell — display label + inline tags (crown / vote / benchmark /
    // robustness). Ensemble arms render their FRIENDLY label ("Majority vote
    // (2-of-3)") instead of the opaque id so the row reads AS an ensemble.
    let id_color = if is_crowned {
        color::ACCENT.current(mode)
    } else {
        color::FG_1.current(mode)
    };
    let mut strat = Row::new()
        .spacing(space::XS)
        .align_y(iced::alignment::Vertical::Center)
        .push(
            Text::new(display_label(leader.strategy.as_str()))
                .size(text::BODY)
                .color(id_color),
        );
    if is_crowned {
        strat = strat.push(tag(
            LEADERBOARD_CROWN_TAG,
            color::ACCENT.current(mode),
            mode,
        ));
    }
    // The `vote` tag marks an ensemble candidate (so the kind is legible beyond
    // the friendly label, the way `benchmark` marks the passive baseline).
    let is_ensemble = is_ensemble_id(leader.strategy.as_str());
    if is_ensemble {
        strat = strat.push(tag(
            LEADERBOARD_ENSEMBLE_VOTE_TAG,
            color::FG_3.current(mode),
            mode,
        ));
    }
    // The `short` tag marks a short-capable arm (advisor-short-selling, T-U2 /
    // ADR-0068 § D9) so the user sees the short field — pairs with the friendly
    // directional label the way `vote` pairs with an ensemble. Rendered with the
    // `DOWN_500` clay hue (the down-half treatment) so the short field reads as
    // the notable, asymmetric-risk case, with the WORD always present
    // (accessibility — colour is never the only signal).
    if is_short_capable_id(leader.strategy.as_str()) {
        strat = strat.push(tag(
            LEADERBOARD_SHORT_TAG,
            color::DOWN_500.current(mode),
            mode,
        ));
    }
    if leader.is_benchmark {
        strat = strat.push(tag(
            LEADERBOARD_BENCHMARK_TAG,
            color::FG_3.current(mode),
            mode,
        ));
    }
    // The "sat in cash — consensus never reached" note for a ZERO-trade ensemble
    // (B1 / U3): a 4-of-4 unanimous vote whose quorum was never reached stays
    // flat the whole window. Without this the row is a bare Sharpe-0 line
    // indistinguishable from a strategy that traded and lost — honest-but-
    // mis-presented (analyst § 1.4). The note makes the "why it's flat" explicit.
    if is_ensemble && leader.trade_count == 0 {
        strat = strat.push(sat_in_cash_note(mode));
    }
    // The robustness marker. The BENCHMARK (ADR-0066 § D3) is the baseline — its
    // flag is still shown, but informationally (it is exempt from the candidate
    // verdict), so it never paints the prominent "cannot be crowned" Fragile
    // badge an ACTIVE arm gets.
    if let Some(rob_tag) = robustness_tag(leader.robustness, leader.is_benchmark, mode) {
        strat = strat.push(rob_tag);
    }
    let strat_cell = Container::new(strat).width(Length::Fill);

    // Numeric cells (right-aligned, sentiment/neg colour only).
    let (return_text, return_color) = signed_pct(leader.total_return_pct, mode);
    let (dd_text, dd_color) = format_pct_max_dd(leader.max_drawdown * dec_100(), mode);
    let return_cell = num_cell(return_text, return_color);
    let sharpe_cell = num_cell(
        format_sharpe(sharpe_to_decimal(leader.sharpe)),
        color::FG_1.current(mode),
    );
    let dd_cell = num_cell(dd_text, dd_color);
    let trades_cell = num_cell(
        crate::widgets::num::format_count(leader.trade_count as u64),
        color::FG_1.current(mode),
    );
    // P1-1 (advisor-turnover-and-tail-metrics) — the "cost story" column.
    // `0.0` for idle / no-fills (the buy-and-hold benchmark always sits here).
    // The narrower `W_TURNOVER` width keeps the table fitting at 1920px.
    let turnover_cell = turnover_num_cell(leader.turnover, color::FG_1.current(mode));

    let row = Row::new()
        .spacing(space::S)
        .padding([space::XS as u16, space::S as u16])
        .align_y(iced::alignment::Vertical::Center)
        .push(rank_cell)
        .push(strat_cell)
        .push(return_cell)
        .push(sharpe_cell)
        .push(dd_cell)
        .push(trades_cell)
        .push(turnover_cell);

    // Crowned rows get the 2 px ACCENT left rule (the active-row pattern);
    // non-crowned rows pass `active = false` (transparent rule, identical
    // layout) so the table stays aligned.
    let styled_row = frame::active_row(row.into(), is_crowned, mode);

    // advisor-leaderboard-inspect-in-lab — make the whole row clickable: a
    // jump to the Lab preseeded with THIS row's strategy + the leaderboard's
    // chosen coin + lookback. Wrap the already-styled row in a TRANSPARENT,
    // zero-padding button so the table still reads as a table (no button
    // chrome) — the ACCENT left-rule, tags, and column alignment all survive
    // inside. A subtle `ACCENT_SOFT` hover backdrop is the only affordance, so
    // the rows are discoverable as interactive without looking like a row of
    // buttons. The message content is owned (`StrategyId` from the row id +
    // the cloned coin + Copy lookback) so it satisfies the button's `Clone`
    // bound regardless of the borrowed row content.
    let inspect_msg = Message::InspectStrategyFromLeaderboard {
        strategy: StrategyId::new(leader.strategy.as_str()),
        coin: coin.clone(),
        lookback,
    };
    Button::new(styled_row)
        .width(Length::Fill)
        .padding(0)
        .on_press(inspect_msg)
        .style(move |_t: &iced::Theme, status: button::Status| {
            // Default: fully transparent (the row paints its own treatment).
            // Hovered/pressed: a faint ACCENT_SOFT wash signals the row is a
            // click target. No border/radius — keep the table grid intact.
            let background = match status {
                button::Status::Hovered | button::Status::Pressed => {
                    Some(color::ACCENT_SOFT.current(mode).into())
                }
                button::Status::Active | button::Status::Disabled => None,
            };
            button::Style {
                background,
                text_color: color::FG_1.current(mode),
                ..Default::default()
            }
        })
        .into()
}

/// A compact inline tag (`SMALL`, coloured). Used for the crown / vote /
/// benchmark / robust / marginal markers — each carries a word so colour is
/// never the only signal.
fn tag(label: &str, fg: iced::Color, _mode: ThemeMode) -> crate::Element<'static> {
    Text::new(label.to_string())
        .size(text::SMALL)
        .color(fg)
        .into()
}

/// The friendly display label for a strategy id. Ensemble arms (F8 + the
/// advisor-combination-search slate, ADR-0067) carry opaque `v0.8.vote.*` ids;
/// map each of the 8 pre-registered ids to its plain-language vote label so the
/// row reads AS the specific ensemble (method + named members or k-of-n quorum).
/// Every other id renders verbatim. Closed `ui`-side match (no engine string
/// crosses the seam).
fn display_label(strategy: &str) -> &str {
    match strategy {
        // The two F8 arms (ADR-0063).
        "v0.8.vote.majority" => LEADERBOARD_ENSEMBLE_MAJORITY_LABEL,
        "v0.8.vote.unanimous" => LEADERBOARD_ENSEMBLE_UNANIMOUS_LABEL,
        // The 6 combination-search arms (ADR-0067): the 3 decorrelation pairs +
        // the complete k∈{1,2,3}-of-4 ladder.
        "v0.8.vote.trend_pair" => LEADERBOARD_ENSEMBLE_TREND_PAIR_LABEL,
        "v0.8.vote.tr_mr_macd_rsi" => LEADERBOARD_ENSEMBLE_TR_MR_MACD_RSI_LABEL,
        "v0.8.vote.tr_mr_sma_bb" => LEADERBOARD_ENSEMBLE_TR_MR_SMA_BB_LABEL,
        "v0.8.vote.any1of4" => LEADERBOARD_ENSEMBLE_ANY1OF4_LABEL,
        "v0.8.vote.k2of4" => LEADERBOARD_ENSEMBLE_K2OF4_LABEL,
        "v0.8.vote.k3of4" => LEADERBOARD_ENSEMBLE_K3OF4_LABEL,
        // The FIXED 5-arm short slate (advisor-short-selling, ADR-0068 § D9):
        // the 4 symmetric long/short `_ls` variants + the always-short control.
        // Map each opaque id to its friendly directional label so the row reads
        // AS a long/short strategy (NOT a raw `sma_cross_ls` id) — own this
        // ui-side, the lesson from advisor-combination-search.
        //
        // The bakeoff engine emits `v0.`-prefixed ids (e.g. `"v0.sma_cross_ls"`,
        // `"v0.macd_ls"`) but earlier UI code only mapped the bare short forms.
        // Both forms are mapped here so the label survives either id shape —
        // whichever the engine reports, the row reads as a human label.
        "sma_cross_ls" | "v0.sma_cross_ls" => LEADERBOARD_SHORT_SMA_CROSS_LS_LABEL,
        "macd_ls" | "v0.macd_ls" => LEADERBOARD_SHORT_MACD_LS_LABEL,
        "rsi_ls" | "v0.rsi_ls" => LEADERBOARD_SHORT_RSI_LS_LABEL,
        "bbands_ls" | "v0.bbands_ls" => LEADERBOARD_SHORT_BBANDS_LS_LABEL,
        "always_short" | "v0.always_short" => LEADERBOARD_SHORT_ALWAYS_SHORT_LABEL,
        // The FIXED 5-arm signal-library slate (advisor-signal-library-expansion,
        // ADR-0071 § D6): 4 DSL-only breakout/volume/momentum arms + the OBV arm.
        // Each opaque `v0.*` id maps to a friendly label naming the rule + its
        // LOCKED parameterization so the row reads AS the strategy, not a raw id —
        // the same anti-raw-id discipline the ensemble + short slates use.
        //
        // The bake-off emits the `v0.`-prefixed ids (e.g. `"v0.donchian_break"`),
        // but the bare forms are mapped too (mirroring the short-arm handling) so
        // the label survives whichever id shape reaches the row.
        "donchian_break" | "v0.donchian_break" => LEADERBOARD_SIGNAL_DONCHIAN_BREAK_LABEL,
        "donchian_floor" | "v0.donchian_floor" => LEADERBOARD_SIGNAL_DONCHIAN_FLOOR_LABEL,
        "vol_breakout" | "v0.vol_breakout" => LEADERBOARD_SIGNAL_VOL_BREAKOUT_LABEL,
        "roc_momentum" | "v0.roc_momentum" => LEADERBOARD_SIGNAL_ROC_MOMENTUM_LABEL,
        "obv" | "v0.obv" => LEADERBOARD_SIGNAL_OBV_LABEL,
        // ADR-0072 DVOL implied-vol regime probe (BTC+ETH only; filtered at runtime).
        "dvol_regime" | "v0.dvol_regime" => LEADERBOARD_SIGNAL_DVOL_REGIME_LABEL,
        // ADR-0073 cross-asset macro regime probe (requires yahoo-macro corpus).
        "macro_riskon" | "v0.macro_riskon" => LEADERBOARD_SIGNAL_MACRO_RISKON_LABEL,
        other => other,
    }
}

/// `true` for one of the FIXED 5-arm short slate ids (advisor-short-selling,
/// ADR-0068 § D9) — drives the `short` row tag + the short field-note. A closed
/// `ui`-side match keyed on the pre-registered ids (no engine string crosses the
/// seam), the same discipline `is_ensemble_id` uses. The `_ls` suffix covers the
/// four symmetric variants; `always_short` is the explicit control.
///
/// Both the bare form (`"sma_cross_ls"`) and the `v0.`-prefixed form
/// (`"v0.sma_cross_ls"`) are matched — the bakeoff engine emits the `v0.` form,
/// so both must be accepted to prevent the `short` tag from silently dropping on
/// advisor-path rows.
fn is_short_capable_id(strategy: &str) -> bool {
    strategy.ends_with("_ls") || strategy == "always_short" || strategy == "v0.always_short"
}

/// `true` for one of the 8 pre-registered vote-ensemble ids (the 2 F8 arms + the
/// 6 combination-search arms) — drives the `vote` tag. Every `v0.8.vote.*` id is
/// a vote ensemble by construction, so a prefix match is the honest predicate
/// (it also future-proofs against a new slate arm being added to the field
/// without its `vote` tag silently dropping).
fn is_ensemble_id(strategy: &str) -> bool {
    strategy.starts_with("v0.8.vote.")
}

/// The robustness marker for a row, or `None` when the gate did not run.
///
/// **For ACTIVE arms, Fragile is a prominent BADGE** (soft `DOWN_50` backdrop +
/// saturated `DOWN_500` label + `PILL` radius — the status-pill pattern from
/// the design principles) because a Fragile *active* candidate is *ineligible to
/// crown*: it must be unmistakable, not a faint word. Robust / marginal stay
/// plain muted text (a quiet reassurance, not a sentiment signal — the table
/// would be a wall of pills otherwise).
///
/// **The BENCHMARK is the baseline (ADR-0066 § D3), not a candidate** — its own
/// robustness flag is still computed + shown, but it is *exempt from the
/// candidate verdict*, so a Fragile benchmark must NOT wear the prominent
/// "cannot be crowned" badge (that would read as disqualifying, the nihilist
/// framing B1 exists to remove). Instead it renders the quiet, informational
/// "baseline is path-dependent" note — honest that buy-and-hold is itself
/// path-dependent on a single volatile asset, without implying it lost the bar.
/// Robust / marginal on the benchmark stay silent (the `baseline` tag already
/// frames the row; no extra reassurance needed).
fn robustness_tag(
    robustness: Option<RobustnessLabel>,
    is_benchmark: bool,
    mode: ThemeMode,
) -> Option<crate::Element<'static>> {
    if is_benchmark {
        // Baseline: only the Fragile case gets a quiet informational note; the
        // robust/marginal/none cases stay silent (the `baseline` tag suffices).
        return match robustness {
            Some(RobustnessLabel::Fragile) => Some(benchmark_fragile_note(mode)),
            _ => None,
        };
    }
    match robustness {
        Some(RobustnessLabel::Fragile) => Some(fragile_badge(mode)),
        Some(RobustnessLabel::Robust) => {
            Some(tag(LEADERBOARD_ROBUST_TAG, color::FG_3.current(mode), mode))
        }
        Some(RobustnessLabel::Marginal) => Some(tag(
            LEADERBOARD_MARGINAL_TAG,
            color::FG_3.current(mode),
            mode,
        )),
        // NotChecked / None → no tag (the gate was skipped; the row is silent).
        _ => None,
    }
}

/// The quiet, informational "sat in cash — consensus never reached" note for a
/// zero-trade ensemble (U3). Muted `FG_3` `SMALL` text — it explains *why* the
/// row is flat (the vote never reached its quorum), so a 0-trade unanimous
/// ensemble never reads as a strategy that traded and lost. Plain text, NOT a
/// warn/negative pill: not trading when there's no consensus is correct
/// behaviour, not a failure.
fn sat_in_cash_note(mode: ThemeMode) -> crate::Element<'static> {
    tag(
        LEADERBOARD_ENSEMBLE_SAT_IN_CASH,
        color::FG_3.current(mode),
        mode,
    )
}

/// The informational robustness note on the BENCHMARK row (ADR-0066 § D3) — a
/// quiet muted `FG_3` `SMALL` word, deliberately NOT the saturated `DOWN_500`
/// Fragile badge an active arm gets. The benchmark is the baseline (exempt from
/// the candidate verdict), so this reads as context ("the baseline itself is
/// path-dependent on a single volatile asset"), never as "disqualified".
fn benchmark_fragile_note(mode: ThemeMode) -> crate::Element<'static> {
    tag(
        LEADERBOARD_BENCHMARK_FRAGILE_NOTE,
        color::FG_3.current(mode),
        mode,
    )
}

/// The Fragile badge — a soft `DOWN_50`-tinted pill with a saturated
/// `DOWN_500` "fragile" label (the `Negative` status-pill intent from the
/// design principles: soft-tint backdrop carries the visual edge, the label
/// keeps the high-contrast signal). `PILL` radius marks it a category tag, not
/// a button. The word is always present so colour is never the only signal
/// (accessibility). This is the visible "cannot be crowned" marker.
fn fragile_badge(mode: ThemeMode) -> crate::Element<'static> {
    Container::new(
        Text::new(LEADERBOARD_FRAGILE_TAG)
            .size(text::SMALL)
            .color(color::DOWN_500.current(mode)),
    )
    .padding([space::XXS as u16, space::XS as u16])
    .style(move |_t: &iced::Theme| iced::widget::container::Style {
        background: Some(color::DOWN_50.current(mode).into()),
        border: Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: radius::PILL.into(),
        },
        text_color: Some(color::DOWN_500.current(mode)),
        ..Default::default()
    })
    .into()
}

/// A right-aligned numeric cell at the fixed numeric width.
fn num_cell(value: String, value_color: iced::Color) -> crate::Element<'static> {
    Text::new(value)
        .size(text::BODY)
        .color(value_color)
        .width(Length::Fixed(W_NUM))
        .align_x(iced::alignment::Horizontal::Right)
        .into()
}

/// A right-aligned turnover cell at the narrower [`W_TURNOVER`] width.
/// Formats the turnover ratio as `"N.N\u{00d7}"` (e.g. `"1.4×"`), the operator-
/// friendly "this many capital-equivalents traded" framing (P1-1).
fn turnover_num_cell(value: rust_decimal::Decimal, color: iced::Color) -> crate::Element<'static> {
    Text::new(format_turnover_ratio(value))
        .size(text::BODY)
        .color(color)
        .width(Length::Fixed(W_TURNOVER))
        .align_x(iced::alignment::Horizontal::Right)
        .into()
}

/// Format the turnover ratio as a short "N.N×" string. A ratio of `1.0` (the
/// strategy churned its entire equity once) renders as `"1.0×"`; `0.0` (idle /
/// no fills) as `"0.0×"`; `12.7` as `"12.7×"`. One decimal place — sub-tenth
/// precision is noise for a churn readout the operator scans at a glance.
///
/// **Why `×` not `%`:** turnover here is a *ratio*, not a percentage, so the
/// "×" suffix reads directly as "this many capital-equivalents". `350%` would
/// be technically equivalent but confusing once the value exceeds 1.0 (a
/// turnover of 12.7 reads worse as `1270%`).
fn format_turnover_ratio(value: rust_decimal::Decimal) -> String {
    // `Decimal::round_dp(1)` rounds to one decimal place using banker's
    // rounding; safe + lossless for the display string.
    let rounded = value.round_dp(1);
    // `Decimal::to_string()` always renders the full scale, so a rounded `1.0`
    // is the string `"1.0"` directly. Negative values shouldn't happen for
    // turnover (it's `Σ|notional| / mean_equity` — non-negative by
    // construction) but `Decimal::abs()` keeps the display defensive.
    let abs = rounded.abs();
    let raw = abs.to_string();
    // Pad to exactly one fractional digit so "1" → "1.0".
    let padded = if raw.contains('.') {
        let (int, frac) = raw.split_once('.').unwrap_or((&raw, ""));
        if frac.is_empty() {
            format!("{int}.0")
        } else {
            format!("{int}.{}", &frac[..1])
        }
    } else {
        format!("{raw}.0")
    };
    format!("{padded}\u{00d7}")
}

// ── Disclaimer ────────────────────────────────────────────────────────────────

/// The persistent NOT-ADVICE + simulated-results disclaimer (product § D5).
/// `MICRO` muted so it is always present but never shouts. Shown on every
/// result surface (Ready AND Error).
fn disclaimer(mode: ThemeMode) -> crate::Element<'static> {
    Text::new(LEADERBOARD_DISCLAIMER)
        .size(text::MICRO)
        .color(color::FG_3.current(mode))
        .width(Length::Fill)
        .into()
}

// ── Small numeric helpers (over the shared `num` formatters) ──────────────────

/// Format a fraction (`0.0738` = +7.38 %) as a signed, sentiment-coloured
/// percentage. Wraps `num::fmt_pct_signed` for the string + picks the colour
/// (pos/neg/zero) the same way the KPI strip's `format_pct_sentiment` does.
fn signed_pct(fraction: rust_decimal::Decimal, mode: ThemeMode) -> (String, iced::Color) {
    let text = fmt_pct_signed(fraction);
    let c = if fraction.is_zero() {
        color::FG_1.current(mode)
    } else if fraction.is_sign_positive() {
        color::UP_500.current(mode)
    } else {
        color::DOWN_500.current(mode)
    };
    (text, c)
}

/// `Decimal(100)` — for converting a drawdown fraction to percent-points for
/// `format_pct_max_dd` (which expects percent input, e.g. `4.20` for 4.2 %).
fn dec_100() -> rust_decimal::Decimal {
    rust_decimal::Decimal::ONE_HUNDRED
}

/// Convert an `f64` Sharpe to `Decimal` for `format_sharpe` (which renders 4 dp
/// with the unicode minus). Lossy-but-display-only: Sharpe is shown to 4 dp, so
/// the f64→Decimal conversion precision is far beyond the rendered resolution.
fn sharpe_to_decimal(sharpe: f64) -> rust_decimal::Decimal {
    rust_decimal::Decimal::try_from(sharpe).unwrap_or(rust_decimal::Decimal::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// advisor-short-selling (T-U2) — the LOAD-BEARING anti-raw-id guard. The 5
    /// short-slate ids MUST each map to a FRIENDLY directional label, never fall
    /// through to the raw id. This is the exact regression advisor-combination-
    /// search hit: the engine adds the ids but the leaderboard mapping must be
    /// extended ui-side or the rows show raw `sma_cross_ls` etc. A failure here
    /// is "the short arm renders its raw id".
    #[test]
    fn short_arm_ids_map_to_friendly_labels_not_raw_ids() {
        let cases = [
            ("sma_cross_ls", LEADERBOARD_SHORT_SMA_CROSS_LS_LABEL),
            ("macd_ls", LEADERBOARD_SHORT_MACD_LS_LABEL),
            ("rsi_ls", LEADERBOARD_SHORT_RSI_LS_LABEL),
            ("bbands_ls", LEADERBOARD_SHORT_BBANDS_LS_LABEL),
            ("always_short", LEADERBOARD_SHORT_ALWAYS_SHORT_LABEL),
        ];
        for (id, expected_label) in cases {
            let label = display_label(id);
            assert_ne!(
                label, id,
                "short arm `{id}` rendered its RAW id — the display_label mapping \
                 is missing (the advisor-combination-search regression)"
            );
            assert_eq!(
                label, expected_label,
                "short arm `{id}` must map to its friendly directional label"
            );
        }
    }

    /// The bakeoff engine emits `v0.`-prefixed short ids. These MUST also map to
    /// friendly labels — the bare-form test above only verifies the fallback path.
    #[test]
    fn v0_prefixed_short_arm_ids_map_to_friendly_labels() {
        let cases = [
            ("v0.sma_cross_ls", LEADERBOARD_SHORT_SMA_CROSS_LS_LABEL),
            ("v0.macd_ls", LEADERBOARD_SHORT_MACD_LS_LABEL),
            ("v0.rsi_ls", LEADERBOARD_SHORT_RSI_LS_LABEL),
            ("v0.bbands_ls", LEADERBOARD_SHORT_BBANDS_LS_LABEL),
            ("v0.always_short", LEADERBOARD_SHORT_ALWAYS_SHORT_LABEL),
        ];
        for (id, expected_label) in cases {
            let label = display_label(id);
            assert_ne!(
                label, id,
                "v0-prefixed short arm `{id}` rendered its RAW id — \
                 the display_label mapping for the v0. prefix is missing"
            );
            assert_eq!(
                label, expected_label,
                "v0-prefixed short arm `{id}` must map to its friendly directional label"
            );
        }
    }

    /// advisor-signal-library-expansion (T11, ADR-0071 § D6) — the LOAD-BEARING
    /// anti-raw-id guard for the FIXED 5-arm signal-library slate. Each of the 5
    /// new arm ids MUST map to a FRIENDLY label, never fall through to the raw id.
    /// This is the exact regression advisor-combination-search hit (and the short
    /// slate guards against): the engine adds the ids in `default_field()` but the
    /// leaderboard mapping must be extended ui-side or the rows show raw
    /// `v0.donchian_break` etc. A failure here is "the new arm renders its raw id".
    ///
    /// The bake-off emits the `v0.`-prefixed ids, but the bare forms are mapped
    /// too (mirroring the short-arm handling), so both id shapes are asserted.
    #[test]
    fn signal_library_arm_ids_map_to_friendly_labels_not_raw_ids() {
        let cases = [
            // (bare form, v0.-prefixed form, expected friendly label)
            (
                "donchian_break",
                "v0.donchian_break",
                LEADERBOARD_SIGNAL_DONCHIAN_BREAK_LABEL,
            ),
            (
                "donchian_floor",
                "v0.donchian_floor",
                LEADERBOARD_SIGNAL_DONCHIAN_FLOOR_LABEL,
            ),
            (
                "vol_breakout",
                "v0.vol_breakout",
                LEADERBOARD_SIGNAL_VOL_BREAKOUT_LABEL,
            ),
            (
                "roc_momentum",
                "v0.roc_momentum",
                LEADERBOARD_SIGNAL_ROC_MOMENTUM_LABEL,
            ),
            ("obv", "v0.obv", LEADERBOARD_SIGNAL_OBV_LABEL),
            (
                "dvol_regime",
                "v0.dvol_regime",
                LEADERBOARD_SIGNAL_DVOL_REGIME_LABEL,
            ),
            // ADR-0073 cross-asset macro regime probe.
            (
                "macro_riskon",
                "v0.macro_riskon",
                LEADERBOARD_SIGNAL_MACRO_RISKON_LABEL,
            ),
        ];
        for (bare, prefixed, expected_label) in cases {
            for id in [bare, prefixed] {
                let label = display_label(id);
                assert_ne!(
                    label, id,
                    "signal-library arm `{id}` rendered its RAW id — the \
                     display_label mapping is missing (the \
                     advisor-combination-search regression)"
                );
                assert_eq!(
                    label, expected_label,
                    "signal-library arm `{id}` must map to its friendly label"
                );
            }
        }
    }

    /// The signal-library labels (including the ADR-0073 macro arm) are DISTINCT
    /// from one another and from the existing single-arm / ensemble / short labels
    /// — so no two rows collapse to the same display string. Guards against
    /// copy-paste constant mix-up.
    #[test]
    fn signal_library_labels_are_distinct() {
        let new_labels = [
            LEADERBOARD_SIGNAL_DONCHIAN_BREAK_LABEL,
            LEADERBOARD_SIGNAL_DONCHIAN_FLOOR_LABEL,
            LEADERBOARD_SIGNAL_VOL_BREAKOUT_LABEL,
            LEADERBOARD_SIGNAL_ROC_MOMENTUM_LABEL,
            LEADERBOARD_SIGNAL_OBV_LABEL,
            LEADERBOARD_SIGNAL_DVOL_REGIME_LABEL,
            // ADR-0073 macro regime probe.
            LEADERBOARD_SIGNAL_MACRO_RISKON_LABEL,
        ];
        for (i, a) in new_labels.iter().enumerate() {
            for b in &new_labels[i + 1..] {
                assert_ne!(a, b, "two signal-library labels collide: {a:?} == {b:?}");
            }
            // Also distinct from the crowned single + an ensemble + a short label,
            // so a new row never reads as an existing arm.
            assert_ne!(*a, LEADERBOARD_ENSEMBLE_MAJORITY_LABEL);
            assert_ne!(*a, LEADERBOARD_SHORT_SMA_CROSS_LS_LABEL);
        }
    }

    /// The `short` row tag fires for every short-slate id (the `_ls` suffix + the
    /// `always_short` control) and NOT for the long-only / ensemble / benchmark
    /// ids — so the short field is marked, and only the short field.
    #[test]
    fn is_short_capable_id_marks_only_the_short_slate() {
        for id in [
            "sma_cross_ls",
            "macd_ls",
            "rsi_ls",
            "bbands_ls",
            "always_short",
            // v0.-prefixed forms (what the bakeoff engine actually emits):
            "v0.sma_cross_ls",
            "v0.macd_ls",
            "v0.rsi_ls",
            "v0.bbands_ls",
            "v0.always_short",
        ] {
            assert!(is_short_capable_id(id), "`{id}` is a short-slate arm");
        }
        for id in [
            "v0.sma",
            "v0.5.macd",
            "v0.5.rsi",
            "v0.5.bbands",
            "v0.buyhold",
            "v0.8.vote.majority",
            "v0.8.vote.tr_mr_macd_rsi",
        ] {
            assert!(
                !is_short_capable_id(id),
                "`{id}` is NOT a short arm — the `short` tag must not fire"
            );
        }
    }

    // ── P1-1 turnover formatting ────────────────────────────────────────────

    /// The turnover ratio renders as a short "N.N×" string — the operator-
    /// friendly "this many capital-equivalents traded" framing. Always exactly
    /// one fractional digit so the column reads as a uniform grid.
    #[test]
    fn format_turnover_ratio_renders_one_decimal_with_x_suffix() {
        use rust_decimal_macros::dec;
        // Idle / no fills — the buy-and-hold benchmark sits here.
        assert_eq!(format_turnover_ratio(dec!(0)), "0.0\u{00d7}");
        // One full capital churn.
        assert_eq!(format_turnover_ratio(dec!(1.0)), "1.0\u{00d7}");
        // Sub-unit — rounded to one decimal.
        assert_eq!(format_turnover_ratio(dec!(0.42)), "0.4\u{00d7}");
        // Multi-capital churn — the active-trading case.
        assert_eq!(format_turnover_ratio(dec!(12.74)), "12.7\u{00d7}");
        // Pad missing fractional digit ("3" → "3.0×").
        assert_eq!(format_turnover_ratio(dec!(3)), "3.0\u{00d7}");
    }

    // ── P1-2 risk-story formatting ──────────────────────────────────────────

    /// `CVaR` rendering — signed-pct from `f64` fraction, leading `MINUS_SIGN_LITERAL`
    /// (unicode minus) for negatives, `+` for positives, plain `0.0%` for zero.
    /// One decimal place. `NaN` renders as the em-dash (defensive — engine
    /// should not emit `NaN`, but a bare `NaN%` would be operator-hostile if
    /// it did).
    #[test]
    fn fmt_signed_pct_from_f64_renders_signed_one_decimal_with_unicode_minus() {
        use crate::strings::MINUS_SIGN_LITERAL;
        assert_eq!(fmt_signed_pct_from_f64(0.0), "0.0%");
        assert_eq!(fmt_signed_pct_from_f64(0.182), "+18.2%");
        assert_eq!(
            fmt_signed_pct_from_f64(-0.182),
            format!("{MINUS_SIGN_LITERAL}18.2%")
        );
        // CVaR_99 is typically deeper than CVaR_95 — exercise the larger loss.
        assert_eq!(
            fmt_signed_pct_from_f64(-0.301),
            format!("{MINUS_SIGN_LITERAL}30.1%")
        );
        // Defensive NaN guard.
        assert_eq!(fmt_signed_pct_from_f64(f64::NAN), "\u{2014}");
    }

    /// Skew/Sortino/Calmar rendering — `format_signed_decimal` writes `+N.NN`
    /// for positives, the unicode `-N.NN` for negatives, bare `N.NN` for zero.
    /// The sign carries the directional read (positive skew = lottery; negative
    /// skew = crash-prone) so a sign prefix is always present beyond colour
    /// (accessibility — colour is never the only signal).
    #[test]
    fn format_signed_decimal_renders_signed_with_unicode_minus() {
        use crate::strings::MINUS_SIGN_LITERAL;
        // Positive skew (lottery-like).
        assert_eq!(format_signed_decimal(0.42, 2), "+0.42");
        // Negative skew (crash-prone).
        assert_eq!(
            format_signed_decimal(-0.42, 2),
            format!("{MINUS_SIGN_LITERAL}0.42")
        );
        // Zero — no sign.
        assert_eq!(format_signed_decimal(0.0, 2), "0.00");
        // Multi-decimal — Sortino above 1 with two-dp precision.
        assert_eq!(format_signed_decimal(1.95, 2), "+1.95");
        // NaN guard.
        assert_eq!(format_signed_decimal(f64::NAN, 2), "\u{2014}");
    }
}
