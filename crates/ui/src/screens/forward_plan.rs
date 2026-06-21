//! advisor-forward-plan v0.1.0 (roadmap F6) — the forward buy/sell PLAN body.
//!
//! Step 4 of the single-coin investment-advisor journey: render a
//! `ForwardPlanView` (mirrored from the `core`-typed
//! `agent::config::ForwardPlan`) as a CONDITIONAL, REACTIVE decision plan.
//! Layout (single scrollable column, top-to-bottom):
//!
//! ```text
//! ┌─ Forward plan ──────────────────────────────────────────────────┐
//! │ <headline>                                                       │
//! │ <caption: standing rules, not a forecast>                        │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ ⚠ This is a conditional, rule-based plan — NOT a price          │  ← framing
//! │   prediction or implied return.            (warn-tinted, integral)│     LEADS
//! ├─ Right now ─────────────────────────────────────────────────────┤
//! │   [ Flat — no position ]   ← dated stance badge                  │
//! │   As of the last close 64,000.00 (Jun 19 14:00).                 │
//! │   Latest signal on that bar: hold (no action).                   │
//! ├─ Standing rules ────────────────────────────────────────────────┤
//! │   IF the 12-bar average crosses above the 26-bar average         │
//! │   THEN buy (open a position)                                     │
//! │   IF it crosses back below   THEN sell (close the position)      │
//! │   These rules are re-checked on every new bar for 7 days —       │
//! │   not a day-by-day schedule.                                     │
//! ├─ If it buys next ───────────────────────────────────────────────┤
//! │   On the next buy it would deploy ~0.003125 units at the last    │
//! │   close 64,000.00 …                                              │
//! │   €200 ≈ 200 USDT (FX not modelled). Never more than €200.       │
//! ├─ Horizon ───────────────────────────────────────────────────────┤
//! │   Planned through Jun 26 — the next 7 days. …not a prediction.   │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ Not financial advice. Simulated €200 paper budget… (persistent)  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! **OQ-D — present so it reads as decision-support, NOT a forecast.** The
//! presentation is built on four moves:
//! 1. The **not-a-prediction framing leads** (a warn-tinted banner directly
//!    under the caption, integral to the layout — not a footnote).
//! 2. The stance is a **dated badge** ("As of the last close … (timestamp)")
//!    so it is unmistakably a snapshot, never a live/future claim.
//! 3. The rules are **labelled IF/THEN conditions**, not a timeline — and a
//!    cadence line restates "re-checked each bar … not a schedule".
//! 4. The sizing is **"at the last close"** (an estimate at the last price),
//!    never "you will buy at"; the horizon is "planned through <date> … not a
//!    prediction of where the price will be".
//!
//! - **Result behind a `PanelState`** (Loading / Empty / Error / Ready) — no
//!   blank screen. `Empty` is the "run a bake-off first" tautology guard.
//! - **The buy-and-hold degenerate plan** reads as obviously the same KIND of
//!   object (same sections, same framing) — it just drops the sell-rule line +
//!   the re-evaluation cadence (there is no sell trigger).
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.
//! **No new theme token, no new widget.**

// Per-module clippy allow-pattern (mirrors `screens/leaderboard.rs:48`): the
// `space::* as u16` / `as f32` layout casts are bounded + safe; `view`/helpers
// take `mode` by value (the `Copy` `ThemeMode`).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::needless_pass_by_value
)]

use iced::widget::{Column, Container, Row, Scrollable, Space, Text};
use iced::{Border, Length};

use crate::forward_plan::state::{ForwardPlanView, PlanRuleView, PlanSignalView, PlanStanceView};
use crate::state::{Cockpit, PanelState};
use crate::strings::{
    FORWARD_PLAN_AS_OF_FMT, FORWARD_PLAN_BUDGET_LINE, FORWARD_PLAN_CADENCE_FMT,
    FORWARD_PLAN_CAPTION, FORWARD_PLAN_DISCLAIMER, FORWARD_PLAN_EMPTY_PROMPT,
    FORWARD_PLAN_ERROR_PREFIX, FORWARD_PLAN_HEADLINE, FORWARD_PLAN_HORIZON_FMT,
    FORWARD_PLAN_HORIZON_TITLE, FORWARD_PLAN_LATEST_SIGNAL_FMT, FORWARD_PLAN_LOADING,
    FORWARD_PLAN_NOT_A_PREDICTION, FORWARD_PLAN_RULE_BBANDS_ENTRY_IF_FMT,
    FORWARD_PLAN_RULE_BBANDS_ENTRY_THEN, FORWARD_PLAN_RULE_BBANDS_EXIT_IF,
    FORWARD_PLAN_RULE_BBANDS_EXIT_THEN, FORWARD_PLAN_RULE_BUY_AND_HOLD,
    FORWARD_PLAN_RULE_COMPOUND_CAVEAT, FORWARD_PLAN_RULE_IF, FORWARD_PLAN_RULE_MACD_ENTRY_IF_FMT,
    FORWARD_PLAN_RULE_MACD_ENTRY_THEN, FORWARD_PLAN_RULE_MACD_EXIT_IF,
    FORWARD_PLAN_RULE_MACD_EXIT_THEN, FORWARD_PLAN_RULE_RSI_ENTRY_IF_FMT,
    FORWARD_PLAN_RULE_RSI_ENTRY_THEN, FORWARD_PLAN_RULE_RSI_EXIT_IF_FMT,
    FORWARD_PLAN_RULE_RSI_EXIT_THEN, FORWARD_PLAN_RULE_SMA_ENTRY_IF_FMT,
    FORWARD_PLAN_RULE_SMA_ENTRY_THEN, FORWARD_PLAN_RULE_SMA_EXIT_IF_FMT,
    FORWARD_PLAN_RULE_SMA_EXIT_THEN, FORWARD_PLAN_RULE_THEN, FORWARD_PLAN_RULES_TITLE,
    FORWARD_PLAN_SIGNAL_BUY, FORWARD_PLAN_SIGNAL_HOLD, FORWARD_PLAN_SIGNAL_SELL,
    FORWARD_PLAN_SIZING_BUY_AND_HOLD_FMT, FORWARD_PLAN_SIZING_CAPPED_NOTE,
    FORWARD_PLAN_SIZING_FLAT_FMT, FORWARD_PLAN_SIZING_LONG_FMT, FORWARD_PLAN_SIZING_TITLE,
    FORWARD_PLAN_STANCE_FLAT, FORWARD_PLAN_STANCE_LONG, FORWARD_PLAN_STANCE_TITLE,
};
use crate::theme::{ThemeMode, color, radius, space, text};
use crate::widgets::frame;
use crate::widgets::num::{fmt_price, fmt_qty};

/// Render the Forward-plan screen body.
///
/// Called by `shell::screen_body` when `current_screen == Screen::ForwardPlan`.
/// Layout, top-to-bottom: the headline/caption header, then the result body
/// (the `PanelState` plan / prompt / spinner / error).
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let st = &model.forward_plan_screen_state;

    Column::new()
        .padding(space::L as u16)
        .spacing(space::L)
        .push(header_text(mode))
        .push(result_body(&st.plan, mode))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Headline (`H1`) over caption (`BODY`, muted) — the screen's plain-language
/// "what this is", framed as standing rules (not a forecast) from the first line.
fn header_text(mode: ThemeMode) -> crate::Element<'static> {
    Column::new()
        .spacing(space::XXS)
        .push(
            Text::new(FORWARD_PLAN_HEADLINE)
                .size(text::H1)
                .color(color::FG_1.current(mode)),
        )
        .push(
            Text::new(FORWARD_PLAN_CAPTION)
                .size(text::BODY)
                .color(color::FG_3.current(mode))
                .width(Length::Fixed(720.0)),
        )
        .into()
}

/// Dispatch on the plan `PanelState` — every arm renders something (no blank
/// screen).
fn result_body(plan: &PanelState<ForwardPlanView>, mode: ThemeMode) -> crate::Element<'_> {
    match plan {
        // The agent is resolving the plan from the crowned selection.
        PanelState::Loading => Container::new(
            Column::new()
                .push(Space::new().height(Length::Fixed(space::XL as f32)))
                .push(frame::loading_with_spinner(FORWARD_PLAN_LOADING, mode)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
        // No crowned pick yet — the "run a bake-off first" tautology guard.
        PanelState::Empty => prompt(FORWARD_PLAN_EMPTY_PROMPT, mode),
        // The plan could not be produced — prefix + the detail.
        PanelState::Error(detail) => error_pane(detail, mode),
        // The conditional plan.
        PanelState::Ready(plan) => ready_pane(plan, mode),
    }
}

/// A centred prompt/empty message — never a blank surface.
fn prompt(copy: &'static str, mode: ThemeMode) -> crate::Element<'static> {
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

/// The error surface — prefix + the failure detail, with the disclaimer still
/// present (the human-friendliness rule).
fn error_pane(detail: &str, mode: ThemeMode) -> crate::Element<'_> {
    let body = Column::new()
        .spacing(space::S)
        .push(Space::new().height(Length::Fixed(space::M as f32)))
        .push(
            Text::new(FORWARD_PLAN_ERROR_PREFIX)
                .size(text::H3)
                .color(color::WARN_500.current(mode)),
        )
        .push(
            Text::new(detail.to_string())
                .size(text::BODY)
                .color(color::FG_2.current(mode)),
        )
        .push(Space::new().height(Length::Fixed(space::L as f32)))
        .push(disclaimer(mode));
    Container::new(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The happy path — the not-a-prediction framing banner (leads), the stance
/// block, the standing-rules block, the projected-sizing block, the horizon
/// block, and the persistent disclaimer, in a scrollable column.
fn ready_pane(plan: &ForwardPlanView, mode: ThemeMode) -> crate::Element<'_> {
    let stack = Column::new()
        .spacing(space::L)
        // The not-a-prediction framing LEADS (OQ-D — integral, not a footnote).
        .push(framing_banner(mode))
        .push(stance_block(plan, mode))
        .push(rules_block(plan, mode))
        .push(sizing_block(plan, mode))
        .push(horizon_block(plan, mode))
        .push(disclaimer(mode))
        .width(Length::Fill);

    Scrollable::new(stack)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── The not-a-prediction framing banner (OQ-D — leads the surface) ─────────────

/// The not-a-prediction framing — a `WARN_500`-tinted banner directly under the
/// header, paired with a `⚠` glyph so the caution is legible beyond colour
/// (accessibility). This is the central honesty move: the conditional,
/// not-a-forecast nature is the FIRST thing in the plan body, integral to the
/// layout. Built as a bordered card (composition with existing tokens; no new
/// widget) tinted with the warn hue at low prominence.
fn framing_banner(mode: ThemeMode) -> crate::Element<'static> {
    let text = Text::new(FORWARD_PLAN_NOT_A_PREDICTION)
        .size(text::BODY)
        .color(color::FG_1.current(mode))
        .width(Length::Fill);

    Container::new(text)
        .width(Length::Fill)
        .padding(space::M as u16)
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: Border {
                color: color::WARN_500.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

// ── Stance block (R1 — dated current stance) ──────────────────────────────────

/// The current-stance block — a titled panel holding the dated stance badge
/// (FLAT/LONG), the honest-staleness "as of the last close … (timestamp)" line,
/// and the latest-signal sub-line (for an active strategy).
fn stance_block(plan: &ForwardPlanView, mode: ThemeMode) -> crate::Element<'_> {
    let mut col = Column::new()
        .spacing(space::S)
        .push(stance_badge(plan, mode));

    // "As of the last close {close} ({as_of})." — the staleness stamp.
    let as_of = FORWARD_PLAN_AS_OF_FMT
        .replace("{close}", &fmt_price(plan.last_close))
        .replace("{as_of}", plan.as_of_label.as_str());
    col = col.push(
        Text::new(as_of)
            .size(text::BODY)
            .color(color::FG_2.current(mode)),
    );

    // The latest signal on that bar (active strategies only — buy-and-hold has
    // no re-evaluation, so it carries no latest signal).
    if let Some(signal) = plan.latest_signal {
        let line = FORWARD_PLAN_LATEST_SIGNAL_FMT.replace("{signal}", signal_word(signal));
        col = col.push(
            Text::new(line)
                .size(text::BODY)
                .color(color::FG_3.current(mode)),
        );
    }

    frame::panel(FORWARD_PLAN_STANCE_TITLE, col.into(), mode)
}

/// The dated stance badge — a pill carrying the FLAT/LONG word. FLAT is muted
/// (a neutral waiting state); LONG uses `ACCENT` (in the market). The word is
/// always present so colour is never the only signal (accessibility).
fn stance_badge(plan: &ForwardPlanView, mode: ThemeMode) -> crate::Element<'static> {
    let (label, fg) = match plan.stance {
        PlanStanceView::Flat => (FORWARD_PLAN_STANCE_FLAT, color::FG_1.current(mode)),
        PlanStanceView::Long => (FORWARD_PLAN_STANCE_LONG, color::ACCENT.current(mode)),
    };
    Container::new(Text::new(label).size(text::H3).color(fg))
        .padding([space::XS as u16, space::M as u16])
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R3.into(),
            },
            text_color: Some(fg),
            ..Default::default()
        })
        .into()
}

/// The plain-language word for a latest signal.
fn signal_word(signal: PlanSignalView) -> &'static str {
    match signal {
        PlanSignalView::Buy => FORWARD_PLAN_SIGNAL_BUY,
        PlanSignalView::Sell => FORWARD_PLAN_SIGNAL_SELL,
        PlanSignalView::Hold => FORWARD_PLAN_SIGNAL_HOLD,
    }
}

// ── Standing-rules block (R2 — IF/THEN conditions, NOT a timeline) ─────────────

/// The standing-rules block — a titled panel holding the IF/THEN entry/exit
/// rule lines (the conditional framing, OQ-D) and, for active rules, the
/// re-evaluation cadence line. The buy-and-hold degenerate case renders a
/// single plain "buy now, hold, no sell trigger" line instead (D5) — obviously
/// the same KIND of object, just without the sell half + the cadence.
fn rules_block(plan: &ForwardPlanView, mode: ThemeMode) -> crate::Element<'_> {
    let mut col = Column::new().spacing(space::S);

    if plan.is_buy_and_hold() {
        // The degenerate plan — one standing rule, no IF/THEN, no cadence.
        col = col.push(
            Text::new(FORWARD_PLAN_RULE_BUY_AND_HOLD)
                .size(text::BODY)
                .color(color::FG_1.current(mode))
                .width(Length::Fill),
        );
    } else {
        let (entry, exit, show_compound_caveat) = rule_clauses(plan.rule);
        col = col.push(if_then_line(entry.0, entry.1, mode));
        if let Some((exit_if, exit_then)) = exit {
            col = col.push(if_then_line(exit_if, exit_then, mode));
        }
        // For composed strategies (MACD, RSI, BBands), show an honest caveat
        // that the primary signal shown above may be part of a compound condition.
        if show_compound_caveat {
            col = col.push(
                Text::new(FORWARD_PLAN_RULE_COMPOUND_CAVEAT)
                    .size(text::SMALL)
                    .color(color::FG_3.current(mode))
                    .width(Length::Fill),
            );
        }
        // The reactive cadence — re-checked each bar, NOT a schedule.
        let cadence = FORWARD_PLAN_CADENCE_FMT.replace("{horizon}", &plan.horizon_days.to_string());
        col = col.push(
            Text::new(cadence)
                .size(text::SMALL)
                .color(color::FG_3.current(mode))
                .width(Length::Fill),
        );
    }

    frame::panel(FORWARD_PLAN_RULES_TITLE, col.into(), mode)
}

/// One IF/THEN rule line — the `IF`/`THEN` keywords in `ACCENT` (so the
/// conditional structure pops), the condition + action in body text. Laid out
/// as a labelled condition, deliberately NOT a dated timeline.
fn if_then_line(if_clause: String, then_clause: &str, mode: ThemeMode) -> crate::Element<'static> {
    Column::new()
        .spacing(space::XXS)
        .push(
            Row::new()
                .spacing(space::XS)
                .push(
                    Text::new(FORWARD_PLAN_RULE_IF)
                        .size(text::SMALL)
                        .color(color::ACCENT.current(mode)),
                )
                .push(
                    Text::new(if_clause)
                        .size(text::BODY)
                        .color(color::FG_1.current(mode))
                        .width(Length::Fill),
                ),
        )
        .push(
            Row::new()
                .spacing(space::XS)
                .push(
                    Text::new(FORWARD_PLAN_RULE_THEN)
                        .size(text::SMALL)
                        .color(color::ACCENT.current(mode)),
                )
                .push(
                    Text::new(then_clause.to_string())
                        .size(text::BODY)
                        .color(color::FG_2.current(mode))
                        .width(Length::Fill),
                ),
        )
        .into()
}

/// The `(entry, optional-exit, show_compound_caveat)` IF/THEN clause triple for
/// a rule family.  The IF clauses are formatted with rule parameters (the copy
/// stays in `strings`); THEN clauses are static.
///
/// Returns `(entry, Some(exit), caveat)` for active families where `caveat` is
/// `true` for composed strategies (MACD, RSI, `BBands`) that have a compound AND
/// entry condition — the caveat line makes the primary-signal simplification
/// honest.  `BuyAndHold` never reaches here (handled in [`rules_block`]) — it
/// returns a defensive empty entry.
fn rule_clauses(
    rule: PlanRuleView,
) -> ((String, &'static str), Option<(String, &'static str)>, bool) {
    match rule {
        PlanRuleView::SmaCross { fast_len, slow_len } => {
            let entry_if = FORWARD_PLAN_RULE_SMA_ENTRY_IF_FMT
                .replace("{fast}", &fast_len.to_string())
                .replace("{slow}", &slow_len.to_string());
            let exit_if = FORWARD_PLAN_RULE_SMA_EXIT_IF_FMT
                .replace("{fast}", &fast_len.to_string())
                .replace("{slow}", &slow_len.to_string());
            (
                (entry_if, FORWARD_PLAN_RULE_SMA_ENTRY_THEN),
                Some((exit_if, FORWARD_PLAN_RULE_SMA_EXIT_THEN)),
                // SMA is a single condition — no caveat needed.
                false,
            )
        }
        PlanRuleView::MacdCross { fast, slow, signal } => {
            let entry_if = FORWARD_PLAN_RULE_MACD_ENTRY_IF_FMT
                .replace("{fast}", &fast.to_string())
                .replace("{slow}", &slow.to_string())
                .replace("{signal}", &signal.to_string());
            (
                (entry_if, FORWARD_PLAN_RULE_MACD_ENTRY_THEN),
                Some((
                    FORWARD_PLAN_RULE_MACD_EXIT_IF.to_string(),
                    FORWARD_PLAN_RULE_MACD_EXIT_THEN,
                )),
                // Compound: MACD hist > 0 AND close > EMA(200) — show caveat.
                true,
            )
        }
        PlanRuleView::RsiReversion { len, lower } => {
            let entry_if = FORWARD_PLAN_RULE_RSI_ENTRY_IF_FMT
                .replace("{len}", &len.to_string())
                .replace("{lower}", &lower.to_string());
            // Exit is flip-to-false at the same `lower` threshold — NOT RSI-70.
            let exit_if = FORWARD_PLAN_RULE_RSI_EXIT_IF_FMT.replace("{lower}", &lower.to_string());
            (
                (entry_if, FORWARD_PLAN_RULE_RSI_ENTRY_THEN),
                Some((exit_if, FORWARD_PLAN_RULE_RSI_EXIT_THEN)),
                // Compound: RSI < 30 AND close > min(low,20) — show caveat.
                true,
            )
        }
        PlanRuleView::BollingerReversion { len, k_tenths } => {
            // Render k as N.N from the tenths-int (e.g. 20 → "2.0").
            let k = format!("{}.{}", k_tenths / 10, k_tenths % 10);
            let entry_if = FORWARD_PLAN_RULE_BBANDS_ENTRY_IF_FMT
                .replace("{len}", &len.to_string())
                .replace("{k}", &k);
            // Exit is flip-to-false (price back inside the band) — NOT an
            // upper-band cross.
            (
                (entry_if, FORWARD_PLAN_RULE_BBANDS_ENTRY_THEN),
                Some((
                    FORWARD_PLAN_RULE_BBANDS_EXIT_IF.to_string(),
                    FORWARD_PLAN_RULE_BBANDS_EXIT_THEN,
                )),
                // Compound: close < lower_band AND volume surge — show caveat.
                true,
            )
        }
        // Defensive: BuyAndHold is handled by the caller's `is_buy_and_hold`
        // branch and never reaches here; return an empty entry rather than
        // panicking, keeping this total.
        PlanRuleView::BuyAndHold => (
            (String::new(), FORWARD_PLAN_RULE_SMA_ENTRY_THEN),
            None,
            false,
        ),
    }
}

// ── Sizing block (R3 — budget-aware €200 next-BUY, "at the last close") ────────

/// The projected-sizing block — a titled panel holding the next-BUY sizing line
/// (labelled "at the last close", never "you will buy at"), the €200 ≈ 200 USDT
/// budget + hard-cap line, and the capped note when the F4 cap bound the units.
fn sizing_block(plan: &ForwardPlanView, mode: ThemeMode) -> crate::Element<'_> {
    let units = fmt_qty(plan.projected_units);
    let close = fmt_price(plan.last_close);

    // The sizing line varies by stance / degenerate case (all labelled "at the
    // last close" — an estimate at the last price, not a promised fill).
    let sizing_line = if plan.is_buy_and_hold() {
        FORWARD_PLAN_SIZING_BUY_AND_HOLD_FMT
    } else {
        match plan.stance {
            PlanStanceView::Flat => FORWARD_PLAN_SIZING_FLAT_FMT,
            PlanStanceView::Long => FORWARD_PLAN_SIZING_LONG_FMT,
        }
    }
    .replace("{units}", &units)
    .replace("{close}", &close);

    let mut col = Column::new().spacing(space::S).push(
        Text::new(sizing_line)
            .size(text::BODY)
            .color(color::FG_1.current(mode))
            .width(Length::Fill),
    );

    // The €200 ≈ 200 USDT + hard-cap line — always present (the budget framing).
    col = col.push(
        Text::new(FORWARD_PLAN_BUDGET_LINE)
            .size(text::SMALL)
            .color(color::FG_3.current(mode))
            .width(Length::Fill),
    );

    // Surface the cap explicitly when it actually bit.
    if plan.sizing_capped {
        col = col.push(
            Text::new(FORWARD_PLAN_SIZING_CAPPED_NOTE)
                .size(text::SMALL)
                .color(color::WARN_500.current(mode))
                .width(Length::Fill),
        );
    }

    frame::panel(FORWARD_PLAN_SIZING_TITLE, col.into(), mode)
}

// ── Horizon block (R4 — "planned through <date>") ─────────────────────────────

/// The horizon block — a titled panel holding the "planned through <date> …
/// not a prediction of where the price will be" framing line.
fn horizon_block(plan: &ForwardPlanView, mode: ThemeMode) -> crate::Element<'_> {
    let line = FORWARD_PLAN_HORIZON_FMT
        .replace("{through}", plan.horizon_through_label.as_str())
        .replace("{days}", &plan.horizon_days.to_string());

    frame::panel(
        FORWARD_PLAN_HORIZON_TITLE,
        Text::new(line)
            .size(text::BODY)
            .color(color::FG_2.current(mode))
            .width(Length::Fill)
            .into(),
        mode,
    )
}

// ── Disclaimer ────────────────────────────────────────────────────────────────

/// The persistent NOT-ADVICE + simulated-budget disclaimer (product § D5).
/// `MICRO` muted so it is always present but never shouts. Shown on every
/// result surface (Ready AND Error).
fn disclaimer(mode: ThemeMode) -> crate::Element<'static> {
    Text::new(FORWARD_PLAN_DISCLAIMER)
        .size(text::MICRO)
        .color(color::FG_3.current(mode))
        .width(Length::Fill)
        .into()
}
