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
//! │  2  v0.buyhold benchmark +3.60%  0.41     -8.10%   2             │
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
//! - **The benchmark row** is labelled `benchmark` so the passive baseline is
//!   always plain.
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

use crate::leaderboard::state::{
    BakeoffReportMirror, LeaderRow, OutcomeKind, ReasonLabel, RecommendationMirror, RobustnessLabel,
};
use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    LEADERBOARD_BENCHMARK_TAG, LEADERBOARD_BUDGET_CONTEXT_FMT, LEADERBOARD_CAPTION,
    LEADERBOARD_COL_MAX_DD, LEADERBOARD_COL_RANK, LEADERBOARD_COL_RETURN, LEADERBOARD_COL_SHARPE,
    LEADERBOARD_COL_STRATEGY, LEADERBOARD_COL_TRADES, LEADERBOARD_CONTEXT_NO_BUDGET_FMT,
    LEADERBOARD_CROWN_TAG, LEADERBOARD_DISCLAIMER, LEADERBOARD_EMPTY_PROMPT,
    LEADERBOARD_ERROR_PREFIX, LEADERBOARD_FRAGILE_TAG, LEADERBOARD_HEADLINE,
    LEADERBOARD_HEADLINE_ACTIVE_WINS, LEADERBOARD_HEADLINE_ALL_FRAGILE,
    LEADERBOARD_HEADLINE_BENCHMARK_WINS, LEADERBOARD_LOADING, LEADERBOARD_MARGINAL_TAG,
    LEADERBOARD_REASON_ALL_FRAGILE, LEADERBOARD_REASON_BEAT_BENCHMARK_SHARPE,
    LEADERBOARD_REASON_BENCHMARK_UNDEFEATED, LEADERBOARD_REASON_HIGHEST_ROBUST_SHARPE,
    LEADERBOARD_REASON_TIE_DRAWDOWN, LEADERBOARD_REASON_TIE_RETURN,
    LEADERBOARD_RECOMMENDATION_TITLE, LEADERBOARD_ROBUST_TAG, LEADERBOARD_RUN_BUTTON,
    LEADERBOARD_RUN_BUTTON_RUNNING, LEADERBOARD_WINNER_FRAGILE_CLAUSE,
    LEADERBOARD_WINNER_ROBUST_CLAUSE,
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
    let guided_input =
        crate::widgets::bakeoff_input::view(&st.coin, &st.budget_input, st.lookback, mode);

    // Budget-context line — carries the budget forward visually (the ranking
    // itself is budget-independent; this is shown for context per F3).
    let budget_context = budget_context_line(st, mode);

    let body = result_body(st, mode);

    Column::new()
        .padding(space::L as u16)
        .spacing(space::L)
        .push(header)
        .push(guided_input)
        .push(budget_context)
        .push(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The budget-context line shown under the guided input — "Ranking strategies
/// for €200 in XRPUSDT." When the budget field is blank/unparseable the budget
/// clause drops but the coin is still named, so the line never goes empty
/// (no-blank-screen rule). Built from the structured selection (the UI owns the
/// copy; the runtime values stay values).
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
    Text::new(copy)
        .size(text::H3)
        .color(color::FG_2.current(mode))
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
        // In-flight bake-off — spinner + "running…" copy.
        PanelState::Loading => Container::new(
            Column::new()
                .push(Space::new().height(Length::Fixed(space::XL as f32)))
                .push(frame::loading_with_spinner(LEADERBOARD_LOADING, mode)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
        // Cold start — the "press Run bake-off" prompt (honest Empty). The copy
        // reflects the CURRENT selection so the operator sees exactly what the
        // next Run will do (F3): "…rank every strategy on {coin} over {lookback}".
        PanelState::Empty => prompt(empty_prompt_copy(st), mode),
        // The run failed — prefix + the engine's detail (never a bare "no data").
        PanelState::Error(detail) => error_pane(detail, mode),
        // The ranked leaderboard + recommendation.
        PanelState::Ready(report) => ready_pane(report, mode),
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
fn ready_pane(report: &BakeoffReportMirror, mode: ThemeMode) -> crate::Element<'_> {
    let recommendation = recommendation_block(report, mode);
    let table = leaderboard_table(report, mode);

    let stack = Column::new()
        .spacing(space::L)
        .push(recommendation)
        .push(table)
        .push(disclaimer(mode))
        .width(Length::Fill);

    Scrollable::new(stack)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Recommendation block ──────────────────────────────────────────────────────

/// The recommendation block — a `frame::panel` titled "Recommendation" holding
/// the plain-language headline (rendered FROM the structured `Recommendation`)
/// + the winner-robustness clause + the supporting reasons as sub-copy.
fn recommendation_block(report: &BakeoffReportMirror, mode: ThemeMode) -> crate::Element<'_> {
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

    // Supporting reasons — one muted line each (deterministic order).
    if !rec.reasons.is_empty() {
        let mut reasons = Column::new().spacing(space::XXS);
        for reason in &rec.reasons {
            reasons = reasons.push(
                Text::new(format!("\u{00b7} {}", reason_copy(*reason)))
                    .size(text::BODY)
                    .color(color::FG_3.current(mode)),
            );
        }
        col = col.push(reasons);
    }

    frame::panel(LEADERBOARD_RECOMMENDATION_TITLE, col.into(), mode)
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

// ── The ranked table ──────────────────────────────────────────────────────────

/// The leaderboard table — a header row + one row per candidate, in `ranked`
/// (best-first) order. The crowned row gets the `ACCENT` treatment; the
/// benchmark row gets the `benchmark` tag.
fn leaderboard_table(report: &BakeoffReportMirror, mode: ThemeMode) -> crate::Element<'_> {
    let mut col = Column::new().spacing(space::XXS).push(header_row(mode));

    // Iterate in ranked (best-first) order. `rank` is the 1-based display
    // position; `crowned` marks the accent row.
    for (rank, &row_idx) in report.ranked.iter().enumerate() {
        let Some(leader) = report.rows.get(row_idx) else {
            continue; // defensive: ranked indices are always in-range
        };
        let is_crowned = report.crowned == Some(row_idx);
        col = col.push(data_row(rank + 1, leader, is_crowned, mode));
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
fn data_row(
    rank: usize,
    leader: &LeaderRow,
    is_crowned: bool,
    mode: ThemeMode,
) -> crate::Element<'_> {
    // Rank cell.
    let rank_cell = Text::new(format!("{rank}"))
        .size(text::BODY)
        .color(color::FG_3.current(mode))
        .width(Length::Fixed(W_RANK));

    // Strategy cell — id + inline tags (crown / benchmark / robustness).
    let id_color = if is_crowned {
        color::ACCENT.current(mode)
    } else {
        color::FG_1.current(mode)
    };
    let mut strat = Row::new()
        .spacing(space::XS)
        .align_y(iced::alignment::Vertical::Center)
        .push(
            Text::new(leader.strategy.as_str())
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
    if leader.is_benchmark {
        strat = strat.push(tag(
            LEADERBOARD_BENCHMARK_TAG,
            color::FG_3.current(mode),
            mode,
        ));
    }
    if let Some(rob_tag) = robustness_tag(leader.robustness, mode) {
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

    let row = Row::new()
        .spacing(space::S)
        .padding([space::XS as u16, space::S as u16])
        .align_y(iced::alignment::Vertical::Center)
        .push(rank_cell)
        .push(strat_cell)
        .push(return_cell)
        .push(sharpe_cell)
        .push(dd_cell)
        .push(trades_cell);

    // Crowned rows get the 2 px ACCENT left rule (the active-row pattern);
    // non-crowned rows pass `active = false` (transparent rule, identical
    // layout) so the table stays aligned.
    frame::active_row(row.into(), is_crowned, mode)
}

/// A compact inline tag (`SMALL`, coloured). Used for the crown / benchmark /
/// robustness markers — each carries a word so colour is never the only signal.
fn tag(label: &str, fg: iced::Color, _mode: ThemeMode) -> crate::Element<'static> {
    Text::new(label.to_string())
        .size(text::SMALL)
        .color(fg)
        .into()
}

/// The robustness tag for a row, or `None` when the gate did not run. Fragile →
/// `warn`; robust/marginal → muted (a reassurance, not a sentiment signal).
fn robustness_tag(
    robustness: Option<RobustnessLabel>,
    mode: ThemeMode,
) -> Option<crate::Element<'static>> {
    match robustness {
        Some(RobustnessLabel::Fragile) => Some(tag(
            LEADERBOARD_FRAGILE_TAG,
            color::WARN_500.current(mode),
            mode,
        )),
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

/// A right-aligned numeric cell at the fixed numeric width.
fn num_cell(value: String, value_color: iced::Color) -> crate::Element<'static> {
    Text::new(value)
        .size(text::BODY)
        .color(value_color)
        .width(Length::Fixed(W_NUM))
        .align_x(iced::alignment::Horizontal::Right)
        .into()
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
