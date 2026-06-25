//! advisor-param-tuning (ADR-0069) — the gate-tied hyperparameter sweep editor
//! ("Tune") screen body.
//!
//! A power-user drill-down off a Leaderboard row's "Tune…" affordance: pick a
//! strategy family + a parameter grid, sweep it, and see each config scored
//! through the SAME frozen robustness gate the bake-off uses. An overfit config
//! that looks great in-sample but falls apart under resampling renders FRAGILE
//! and is promotion-blocked (the anti-overfit honesty lock).
//!
//! Layout (single column, top-to-bottom):
//!
//! ```text
//! ┌─ Tune parameters ───────────────────────────────────────────────┐
//! │ <caption>                                      [ Run sweep ]     │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ Choose a parameter grid                                          │
//! │  Strategy family  [SMA crossover*] [MACD] [RSI] [Bollinger]      │  ← chips
//! │  Fast window (shipped 20)   min[10] max[30] step[5]  [narrow][…] │
//! │  Slow window (shipped 50)   min[30] max[70] step[10] [narrow][…] │
//! │  25 configs → ~25000 bootstrap runs                              │  ← readout
//! ├─────────────────────────────────────────────────────────────────┤
//! │ vs just holding BTCUSDT: +3.60% return, Sharpe 0.41.            │  ← benchmark
//! │ Showing 24 of 30 configs — narrow your ranges…  (when truncated) │  ← banner
//! │  Config        Verdict  Return  Sharpe p5/p50/p95  P(loss) … Use │
//! │  fast=10,sl=20 robust   +7.38%  0.60/1.20/1.70     12%  …   Use  │
//! │  fast=15,sl=30 fragile  +9.10% -0.50/2.50/3.00     50%  … Locked │  ← FRAGILE
//! │  fast=20,sl=50 shipped  +5.10%  0.40/0.90/1.30     18%  …   Use  │  ← baseline
//! │  <distribution caption>                                          │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ Tuning is paper/sim research, not advice. A fragile config…      │  ← footer
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! - **Result behind a `PanelState`** (Loading / Empty / Error / Ready) — no
//!   blank screen. `Empty` is the cold "set ranges and press Run sweep" prompt.
//! - **FRAGILE is the prominent, promotion-blocking state** — the leaderboard's
//!   `DOWN_50`-tinted pill with the `DOWN_500` "fragile" label (the same pixels
//!   the leaderboard guard checks), and its "Use this config" affordance is
//!   disabled + greyed ("Fragile cannot be crowned").
//! - **The bootstrap distribution is shown, not just the point estimate** — the
//!   Sharpe column shows p5 / p50 / p95, and P(loss) / P(Sharpe>1) / Max-DD p95
//!   get their own columns. The spread is the honesty affordance.
//! - **The shipped config is always a labelled baseline row**; **buy-and-hold**
//!   KPIs are a header strip ("vs just holding {coin}").
//! - **A persistent NOT-ADVICE + FRAGILE-is-overfit footer** sits at the bottom.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.
//! **No new theme token, no new widget.**

// Per-module clippy allow-pattern (mirrors `screens/leaderboard.rs:53`): the
// `space::* as u16` / `as f32` layout casts are bounded + safe; `view`/helpers
// take `mode` by value (the `Copy` `ThemeMode`).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::needless_pass_by_value
)]

use iced::widget::{Button, Column, Container, Row, Scrollable, Space, Text, button, text_input};
use iced::{Border, Length};

use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    TUNE_AXIS_FAST_LABEL, TUNE_AXIS_MAX, TUNE_AXIS_MIN, TUNE_AXIS_SLOW_LABEL, TUNE_AXIS_STEP,
    TUNE_BASELINE_TAG, TUNE_BENCHMARK_STRIP_FMT, TUNE_CAPTION, TUNE_COL_CONFIG, TUNE_COL_MAXDD_P95,
    TUNE_COL_PROB_LOSS, TUNE_COL_PROB_SHARPE, TUNE_COL_RETURN, TUNE_COL_SHARPE_SPREAD,
    TUNE_COL_USE, TUNE_COL_VERDICT, TUNE_DISCLAIMER, TUNE_DISTRIBUTION_CAPTION, TUNE_EMPTY_PROMPT,
    TUNE_ERROR_PREFIX, TUNE_FAMILY_BOLLINGER, TUNE_FAMILY_LABEL, TUNE_FAMILY_MACD,
    TUNE_FAMILY_PENDING_NOTE, TUNE_FAMILY_RSI, TUNE_FAMILY_SMA, TUNE_FORM_TITLE,
    TUNE_FRAGILE_PROMOTE_NOTE, TUNE_GRID_READOUT_BLANK, TUNE_GRID_READOUT_EMPTY,
    TUNE_GRID_READOUT_FMT, TUNE_HEADLINE, TUNE_LOADING, TUNE_PRESET_NARROW, TUNE_PRESET_SHIPPED,
    TUNE_PRESET_WIDE, TUNE_PROGRESS_FMT, TUNE_RUN_BUTTON, TUNE_RUN_BUTTON_RUNNING,
    TUNE_TRUNCATION_FMT, TUNE_USE_CONFIG, TUNE_USE_CONFIG_FRAGILE, TUNE_VERDICT_FRAGILE,
    TUNE_VERDICT_MARGINAL, TUNE_VERDICT_ROBUST,
};
use crate::theme::{ThemeMode, color, radius, space, text};
use crate::tune::screen_state::{
    AxisField, AxisInput, AxisPreset, SmaAxisKind, TuneFamily, TuneScreenState,
};
use crate::tune::state::{SweepCellRow, SweepReportMirror, SweepVerdictLabel};
use crate::widgets::frame;
use crate::widgets::num::{fmt_pct_signed, format_pct_max_dd, format_sharpe};

// ── Column widths (the result-grid proportions) ───────────────────────────────
//
// A wide config column on the left, fixed-width right-aligned numeric columns,
// and a verdict + action column. Local `f32` layout constants (the same kind as
// `leaderboard::W_NUM`), not design tokens.

/// Config (params) column width — fits "fast=20, slow=50" + the shipped tag.
const W_CONFIG: f32 = 150.0;
/// Verdict pill column width.
const W_VERDICT: f32 = 84.0;
/// Single numeric cell width (Return / P(loss) / P(Sharpe>1) / Max-DD p95).
const W_NUM: f32 = 92.0;
/// The Sharpe-spread column (p5 / p50 / p95) is wider — three numbers.
const W_SPREAD: f32 = 150.0;
/// The "Use this config" action column width.
const W_USE: f32 = 130.0;
/// A bounded axis-field input width (min / max / step boxes).
const W_AXIS_FIELD: f32 = 64.0;

/// Render the Tune screen body.
///
/// Called by `shell::screen_body` when `current_screen == Screen::Tune`.
///
/// Layout, top-to-bottom: the headline/caption + Run button header, the range
/// form (family picker + SMA axes + presets + the live grid readout), then the
/// result body (the `PanelState` grid / prompt / spinner / error), then the
/// persistent honesty footer.
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let st = &model.tune_screen_state;
    let coin = model.tune_coin.0.as_str();

    // Header: headline + caption on the left, the Run button on the right.
    let header = Row::new()
        .spacing(space::L)
        .align_y(iced::alignment::Vertical::Center)
        .push(header_text(mode))
        .push(Space::new().width(Length::Fill))
        .push(run_button(st, mode));

    let form = form_panel(st, mode);

    let mut column = Column::new()
        .padding(space::L as u16)
        .spacing(space::L)
        .push(header)
        .push(form);

    // The determinate sweep progress bar — shown only while a sweep is in flight.
    if let Some(bar) = progress_strip(st, mode) {
        column = column.push(bar);
    }

    column
        .push(result_body(st, coin, mode))
        .push(disclaimer(coin, mode))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Headline (`H1`) over caption (`BODY`, muted) — the screen's plain-language
/// "what this is" + the honest "fragile = overfit" frame.
fn header_text(mode: ThemeMode) -> crate::Element<'static> {
    Column::new()
        .spacing(space::XXS)
        .push(
            Text::new(TUNE_HEADLINE)
                .size(text::H1)
                .color(color::FG_1.current(mode)),
        )
        .push(
            Text::new(TUNE_CAPTION)
                .size(text::BODY)
                .color(color::FG_3.current(mode))
                .width(Length::Fixed(720.0)),
        )
        .into()
}

/// The "Run sweep" action button — `ACCENT`-filled (the right default action).
/// Disabled (no `on_press` + muted fill) while a sweep is in flight OR the grid
/// is invalid / the family is not runnable, with the label swapped to
/// "Running…" while in flight so the disabled state is legible beyond colour.
fn run_button(st: &TuneScreenState, mode: ThemeMode) -> crate::Element<'static> {
    let (label, fg, bg, on_press) = if st.running {
        (
            TUNE_RUN_BUTTON_RUNNING,
            color::FG_3.current(mode),
            color::PANEL_RAISED.current(mode),
            None,
        )
    } else if st.can_run() {
        (
            TUNE_RUN_BUTTON,
            color::FG_ON_ACCENT.current(mode),
            color::ACCENT.current(mode),
            Some(Message::SweepRunRequested),
        )
    } else {
        // Not runnable (empty/blank grid, or a not-yet-supported family) — muted,
        // no on_press, label still legible.
        (
            TUNE_RUN_BUTTON,
            color::FG_3.current(mode),
            color::PANEL_RAISED.current(mode),
            None,
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

// ── The range form ────────────────────────────────────────────────────────────

/// The range-form panel — a titled `frame::panel` holding the family picker, the
/// SMA axes (+ presets), the live grid readout, and (for a not-yet-runnable
/// family) the honest "coming soon" note.
fn form_panel(st: &TuneScreenState, mode: ThemeMode) -> crate::Element<'_> {
    let family_block = Column::new()
        .spacing(space::XS)
        .push(field_label(TUNE_FAMILY_LABEL, mode))
        .push(family_row(st.family, mode));

    let mut body = Column::new().spacing(space::M).push(family_block);

    if st.family.is_runnable() {
        // SMA axes (the only runnable family in v0.1).
        body = body
            .push(axis_block(
                TUNE_AXIS_FAST_LABEL,
                SmaAxisKind::Fast,
                &st.sma_grid.fast,
                mode,
            ))
            .push(axis_block(
                TUNE_AXIS_SLOW_LABEL,
                SmaAxisKind::Slow,
                &st.sma_grid.slow,
                mode,
            ))
            .push(grid_readout(st, mode));
    } else {
        // Pending family — the honest "not sweepable yet" note (no axes).
        body = body.push(
            Text::new(TUNE_FAMILY_PENDING_NOTE)
                .size(text::SMALL)
                .color(color::WARN_500.current(mode))
                .width(Length::Fill),
        );
    }

    frame::panel(TUNE_FORM_TITLE, body.into(), mode)
}

/// A field label — `MICRO` muted (the column-header convention).
fn field_label(label: &'static str, mode: ThemeMode) -> crate::Element<'static> {
    Text::new(label)
        .size(text::MICRO)
        .color(color::FG_3.current(mode))
        .into()
}

/// The family-chip row over `TuneFamily::ALL` (SMA / MACD / RSI / Bollinger).
/// The active family gets the `ACCENT` chip treatment; SMA is the only runnable
/// one in v0.1 but every family is shown (the picker IS the menu).
fn family_row(selected: TuneFamily, mode: ThemeMode) -> crate::Element<'static> {
    let mut row = Row::new().spacing(space::S);
    for &fam in TuneFamily::ALL {
        row = row.push(family_chip(fam, fam == selected, mode));
    }
    row.width(Length::Fill).into()
}

/// A single family chip — the `pair_chip` / `coin_chip` shape (active = solid
/// `ACCENT` fill + `FG_ON_ACCENT`; inactive = `PANEL` + `BORDER_1` + `FG_2`).
/// Dispatches `Message::SweepSelectFamily`.
fn family_chip(fam: TuneFamily, active: bool, mode: ThemeMode) -> crate::Element<'static> {
    let fg = if active {
        color::FG_ON_ACCENT.current(mode)
    } else {
        color::FG_2.current(mode)
    };
    let border_color = if active {
        color::ACCENT.current(mode)
    } else {
        color::BORDER_1.current(mode)
    };
    let bg_color = if active {
        color::ACCENT.current(mode)
    } else {
        color::PANEL.current(mode)
    };

    let label = Text::new(family_label(fam)).size(text::SMALL).color(fg);
    let chip = Container::new(label)
        .padding([space::XS as u16, space::S as u16])
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(bg_color.into()),
            border: Border {
                color: border_color,
                width: if active { 1.5 } else { 1.0 },
                radius: radius::R4.into(),
            },
            ..Default::default()
        });

    Button::new(chip)
        .on_press(Message::SweepSelectFamily(fam))
        .padding(0)
        .style(|_t: &iced::Theme, _s: button::Status| button::Style {
            background: None,
            ..Default::default()
        })
        .width(Length::Shrink)
        .into()
}

/// Map a family to its chip copy (all in `strings`).
fn family_label(fam: TuneFamily) -> &'static str {
    match fam {
        TuneFamily::Sma => TUNE_FAMILY_SMA,
        TuneFamily::Macd => TUNE_FAMILY_MACD,
        TuneFamily::Rsi => TUNE_FAMILY_RSI,
        TuneFamily::Bollinger => TUNE_FAMILY_BOLLINGER,
    }
}

/// One axis row: a label + the {min, max, step} fields + the preset chips.
fn axis_block<'a>(
    label: &'static str,
    axis: SmaAxisKind,
    input: &'a AxisInput,
    mode: ThemeMode,
) -> crate::Element<'a> {
    let fields = Row::new()
        .spacing(space::S)
        .align_y(iced::alignment::Vertical::Bottom)
        .push(axis_field(
            TUNE_AXIS_MIN,
            &input.min,
            axis,
            AxisField::Min,
            mode,
        ))
        .push(axis_field(
            TUNE_AXIS_MAX,
            &input.max,
            axis,
            AxisField::Max,
            mode,
        ))
        .push(axis_field(
            TUNE_AXIS_STEP,
            &input.step,
            axis,
            AxisField::Step,
            mode,
        ))
        .push(Space::new().width(Length::Fixed(space::M as f32)))
        .push(preset_chips(axis, mode));

    Column::new()
        .spacing(space::XS)
        .push(field_label(label, mode))
        .push(fields)
        .into()
}

/// One labelled axis field — a tiny caption over a fixed-width numeric input.
/// Dispatches `Message::SweepAxisEdit` on every keystroke (round-tripped).
fn axis_field<'a>(
    caption: &'static str,
    value: &'a str,
    axis: SmaAxisKind,
    field: AxisField,
    mode: ThemeMode,
) -> crate::Element<'a> {
    let input = text_input(caption, value)
        .on_input(move |v| Message::SweepAxisEdit {
            axis,
            field,
            value: v,
        })
        .size(text::SMALL)
        .padding([space::XXS as u16, space::XS as u16])
        .width(Length::Fixed(W_AXIS_FIELD))
        .style(
            move |_t: &iced::Theme, _s: text_input::Status| text_input::Style {
                background: color::PANEL.current(mode).into(),
                border: Border {
                    color: color::BORDER_1.current(mode),
                    width: 1.0,
                    radius: radius::R4.into(),
                },
                icon: color::FG_3.current(mode),
                placeholder: color::FG_3.current(mode),
                value: color::FG_1.current(mode),
                selection: color::ACCENT_SOFT.current(mode),
            },
        );

    Column::new()
        .spacing(space::XXS)
        .push(
            Text::new(caption)
                .size(text::MICRO)
                .color(color::FG_3.current(mode)),
        )
        .push(input)
        .into()
}

/// The narrow / shipped / wide preset chips for one axis.
fn preset_chips(axis: SmaAxisKind, mode: ThemeMode) -> crate::Element<'static> {
    let mut row = Row::new().spacing(space::XS);
    for &preset in AxisPreset::ALL {
        row = row.push(preset_chip(axis, preset, mode));
    }
    row.into()
}

/// A single preset chip — a quiet GHOST chip (`PANEL` fill + `BORDER_1`) so it
/// reads as a secondary one-click affordance next to the fields. Dispatches
/// `Message::SweepAxisPreset`.
fn preset_chip(axis: SmaAxisKind, preset: AxisPreset, mode: ThemeMode) -> crate::Element<'static> {
    let label = match preset {
        AxisPreset::Narrow => TUNE_PRESET_NARROW,
        AxisPreset::Shipped => TUNE_PRESET_SHIPPED,
        AxisPreset::Wide => TUNE_PRESET_WIDE,
    };
    let chip = Container::new(
        Text::new(label)
            .size(text::MICRO)
            .color(color::FG_2.current(mode)),
    )
    .padding([space::XXS as u16, space::XS as u16])
    .style(move |_t: &iced::Theme| iced::widget::container::Style {
        background: Some(color::PANEL.current(mode).into()),
        border: Border {
            color: color::BORDER_1.current(mode),
            width: 1.0,
            radius: radius::R4.into(),
        },
        ..Default::default()
    });

    Button::new(chip)
        .on_press(Message::SweepAxisPreset { axis, preset })
        .padding(0)
        .style(|_t: &iced::Theme, _s: button::Status| button::Style {
            background: None,
            ..Default::default()
        })
        .width(Length::Shrink)
        .into()
}

/// The live grid-size readout — "N configs → ~M bootstrap runs", or an honest
/// prompt when the grid is blank/empty. The cap drives the readout so the cost
/// is visible BEFORE pressing Run. Reads `backtest::MAX_SWEEP_CONFIGS` via the
/// state's `grid_estimate`.
fn grid_readout(st: &TuneScreenState, mode: ThemeMode) -> crate::Element<'static> {
    let est = st.grid_estimate();
    let (copy, c) = if est.has_blank_field {
        (
            TUNE_GRID_READOUT_BLANK.to_string(),
            color::FG_3.current(mode),
        )
    } else if est.runnable == 0 {
        (
            TUNE_GRID_READOUT_EMPTY.to_string(),
            color::WARN_500.current(mode),
        )
    } else {
        // ~runs = runnable cells × 1000 bootstrap paths (the gate setting).
        let runs = est.runnable.saturating_mul(1000);
        (
            TUNE_GRID_READOUT_FMT
                .replace("{n}", &est.runnable.to_string())
                .replace("{runs}", &crate::widgets::num::format_count(runs as u64)),
            color::FG_2.current(mode),
        )
    };
    Text::new(copy).size(text::SMALL).color(c).into()
}

// ── Progress ──────────────────────────────────────────────────────────────────

/// The determinate sweep progress bar — `None` when no sweep is running.
/// Mirrors the leaderboard `progress_strip`: once the first `SweepProgress`
/// event lands it fills `done / total` and names the scoring config; before any
/// event arrives it shows the indeterminate sentinel with the loading copy.
fn progress_strip(st: &TuneScreenState, mode: ThemeMode) -> Option<crate::Element<'static>> {
    if !st.running {
        return None;
    }
    match &st.progress {
        Some(p) if p.total > 0 => {
            let n = u32::from(p.done) + 1;
            let label = TUNE_PROGRESS_FMT
                .replace("{current}", p.current_id.as_str())
                .replace("{n}", &n.to_string())
                .replace("{total}", &p.total.to_string());
            let fill = f32::from(p.done) / f32::from(p.total);
            Some(crate::widgets::progress_bar::view_block::<Message>(
                Some(fill),
                &label,
                mode,
            ))
        }
        _ => Some(crate::widgets::progress_bar::view_block::<Message>(
            None,
            TUNE_LOADING,
            mode,
        )),
    }
}

/// `true` when a real cell-level progress event has arrived — the determinate
/// bar above is then showing real progress, so the result body stays calm.
fn has_live_progress(st: &TuneScreenState) -> bool {
    matches!(&st.progress, Some(p) if p.total > 0)
}

// ── Result body (PanelState dispatch) ─────────────────────────────────────────

/// Dispatch on the result `PanelState` — every arm renders something.
fn result_body<'a>(st: &'a TuneScreenState, coin: &'a str, mode: ThemeMode) -> crate::Element<'a> {
    match &st.result {
        PanelState::Loading => {
            let body: crate::Element<'_> = if has_live_progress(st) {
                Space::new()
                    .width(Length::Shrink)
                    .height(Length::Fixed(space::XL as f32))
                    .into()
            } else {
                Column::new()
                    .push(Space::new().height(Length::Fixed(space::XL as f32)))
                    .push(frame::loading_with_spinner(TUNE_LOADING, mode))
                    .into()
            };
            Container::new(body)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
        PanelState::Empty => prompt(TUNE_EMPTY_PROMPT, mode),
        PanelState::Error(detail) => error_pane(detail, mode),
        PanelState::Ready(report) => ready_pane(report, coin, mode),
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
                    .color(color::FG_3.current(mode))
                    .width(Length::Fixed(720.0)),
            ),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// The error surface — the prefix + the engine's detail (never a bare "no data").
fn error_pane(detail: &str, mode: ThemeMode) -> crate::Element<'_> {
    let prefix = Text::new(TUNE_ERROR_PREFIX)
        .size(text::H3)
        .color(color::WARN_500.current(mode));
    let detail_text = Text::new(detail)
        .size(text::BODY)
        .color(color::FG_2.current(mode));
    let body = Column::new()
        .spacing(space::S)
        .push(Space::new().height(Length::Fixed(space::M as f32)))
        .push(prefix)
        .push(detail_text);
    Container::new(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The happy path — the benchmark strip + the truncation banner (when truncated)
/// + the result grid + the distribution caption, in a scrollable column.
fn ready_pane<'a>(
    report: &'a SweepReportMirror,
    coin: &'a str,
    mode: ThemeMode,
) -> crate::Element<'a> {
    let mut stack = Column::new()
        .spacing(space::M)
        .push(benchmark_strip(report, mode));

    if report.truncated {
        stack = stack.push(truncation_banner(report, mode));
    }

    stack = stack
        .push(result_grid(report, mode))
        .push(distribution_caption(mode))
        .width(Length::Fill);

    // `coin` echoed in the strip; nothing else needs it here.
    let _ = coin;

    Scrollable::new(stack)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The buy-and-hold benchmark header strip ("vs just holding {coin}: …") — the
/// "benchmark is always in view" discipline. `FG_2` so it reads as context.
fn benchmark_strip(report: &SweepReportMirror, mode: ThemeMode) -> crate::Element<'_> {
    let bench = &report.benchmark_kpis;
    let copy = TUNE_BENCHMARK_STRIP_FMT
        .replace("{coin}", report.coin.as_str())
        .replace("{return}", &fmt_pct_signed(bench.total_return_pct))
        .replace("{sharpe}", &format_sharpe(sharpe_to_decimal(bench.sharpe)));
    Text::new(copy)
        .size(text::H3)
        .color(color::FG_2.current(mode))
        .into()
}

/// The honest truncation banner — the grid exceeded `MAX_SWEEP_CONFIGS`.
/// `WARN_500`-tinted (paired with the word) so it reads as a real notice.
fn truncation_banner(report: &SweepReportMirror, mode: ThemeMode) -> crate::Element<'_> {
    let copy = TUNE_TRUNCATION_FMT
        .replace("{shown}", &report.grid_size.to_string())
        .replace("{requested}", &report.requested_count.to_string());
    Container::new(
        Text::new(copy)
            .size(text::SMALL)
            .color(color::WARN_500.current(mode))
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(space::S as u16)
    .style(move |_t: &iced::Theme| iced::widget::container::Style {
        background: Some(color::WARN_50.current(mode).into()),
        border: Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: radius::R4.into(),
        },
        text_color: Some(color::WARN_500.current(mode)),
        ..Default::default()
    })
    .into()
}

/// The distribution caption under the grid — "the distribution is what the gate
/// judges" (the anti-overfit affordance made literal). Muted `FG_3`.
fn distribution_caption(mode: ThemeMode) -> crate::Element<'static> {
    Text::new(TUNE_DISTRIBUTION_CAPTION)
        .size(text::SMALL)
        .color(color::FG_3.current(mode))
        .width(Length::Fill)
        .into()
}

// ── The result grid ───────────────────────────────────────────────────────────

/// The result grid — a header row, one row per swept cell, then the
/// shipped-baseline row (tagged), wrapped in the standard `PANEL` surface.
///
/// The cells are rendered in the mirror's order (the engine's axis-major grid
/// order); fragile cells carry the prominent badge wherever they sit.
fn result_grid(report: &SweepReportMirror, mode: ThemeMode) -> crate::Element<'_> {
    let mut col = Column::new().spacing(space::XXS).push(header_row(mode));

    for cell in &report.cells {
        col = col.push(data_row(cell, false, mode));
    }
    // The shipped-config baseline row — always present, tagged, so the operator
    // sees whether any swept neighbour actually beats the default.
    col = col.push(data_row(&report.baseline, true, mode));

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

/// The grid header row — column labels in `MICRO` muted. Numeric columns
/// right-aligned to match the data rows.
fn header_row(mode: ThemeMode) -> crate::Element<'static> {
    Row::new()
        .spacing(space::S)
        .padding([space::XS as u16, space::S as u16])
        .push(col_head(TUNE_COL_CONFIG, W_CONFIG, false, mode))
        .push(col_head(TUNE_COL_VERDICT, W_VERDICT, false, mode))
        .push(col_head(TUNE_COL_RETURN, W_NUM, true, mode))
        .push(col_head(TUNE_COL_SHARPE_SPREAD, W_SPREAD, true, mode))
        .push(col_head(TUNE_COL_PROB_LOSS, W_NUM, true, mode))
        .push(col_head(TUNE_COL_PROB_SHARPE, W_NUM, true, mode))
        .push(col_head(TUNE_COL_MAXDD_P95, W_NUM, true, mode))
        .push(col_head(TUNE_COL_USE, W_USE, false, mode))
        .into()
}

/// A fixed-width column header. `right` right-aligns (numeric columns).
fn col_head(label: &str, width: f32, right: bool, mode: ThemeMode) -> crate::Element<'static> {
    let mut t = Text::new(label.to_string())
        .size(text::MICRO)
        .color(color::FG_3.current(mode))
        .width(Length::Fixed(width));
    if right {
        t = t.align_x(iced::alignment::Horizontal::Right);
    }
    t.into()
}

/// One swept-cell data row. The config column carries the params label + the
/// verdict context; FRAGILE cells get the prominent badge. The distribution
/// columns (p5/p50/p95 Sharpe, P(loss), P(Sharpe>1), Max-DD p95) are the
/// honesty affordance. The "Use this config" action is disabled+greyed on
/// fragile rows (the promotion lock).
fn data_row(cell: &SweepCellRow, is_baseline: bool, mode: ThemeMode) -> crate::Element<'_> {
    // Config cell — params label + (for the baseline) the shipped tag.
    let mut config = Row::new()
        .spacing(space::XS)
        .align_y(iced::alignment::Vertical::Center)
        .push(
            Text::new(cell.params_label.to_string())
                .size(text::BODY)
                .color(color::FG_1.current(mode)),
        );
    if is_baseline {
        config = config.push(tag(TUNE_BASELINE_TAG, color::FG_3.current(mode)));
    }
    let config_cell = Container::new(config).width(Length::Fixed(W_CONFIG));

    // Verdict pill — FRAGILE prominent (DOWN_50/DOWN_500), robust/marginal quiet.
    let verdict_cell =
        Container::new(verdict_pill(cell.verdict, mode)).width(Length::Fixed(W_VERDICT));

    // Return (in-sample point estimate) — signed, sentiment colour.
    let (ret_text, ret_color) = signed_pct(cell.in_sample_return, mode);
    let return_cell = num_cell(ret_text, ret_color, W_NUM);

    // The Sharpe spread (p5 / p50 / p95) — the load-bearing distribution column.
    let spread = format!(
        "{} / {} / {}",
        format_sharpe(sharpe_to_decimal(cell.distribution.sharpe_p5)),
        format_sharpe(sharpe_to_decimal(cell.distribution.sharpe_p50)),
        format_sharpe(sharpe_to_decimal(cell.distribution.sharpe_p95)),
    );
    // p5 < 0 is the tail-loses-money signal — colour the whole spread by the p5
    // sign (pos/neg only), pairing colour with the always-present number.
    let spread_color = if cell.distribution.sharpe_p5 < 0.0 {
        color::DOWN_500.current(mode)
    } else {
        color::FG_1.current(mode)
    };
    let spread_cell = num_cell(spread, spread_color, W_SPREAD);

    // P(loss) — a higher value is worse; tint warn above a coarse floor.
    let prob_loss_cell = num_cell(
        fmt_prob(cell.distribution.prob_loss),
        prob_loss_color(cell.distribution.prob_loss, mode),
        W_NUM,
    );
    // P(Sharpe>1) — higher is better (a credibility signal); neutral colour.
    let prob_sharpe_cell = num_cell(
        fmt_prob(cell.distribution.prob_sharpe_gt1),
        color::FG_1.current(mode),
        W_NUM,
    );
    // Max-DD p95 (tail drawdown) — always shown as a DOWN_500 magnitude.
    let (dd_text, dd_color) = format_pct_max_dd(maxdd_to_pct(cell.distribution.maxdd_p95), mode);
    let maxdd_cell = num_cell(dd_text, dd_color, W_NUM);

    // The "Use this config" action — enabled accent on promotable rows, greyed
    // + disabled on fragile rows (the promotion lock).
    let use_cell = use_config_cell(cell, mode);

    let row = Row::new()
        .spacing(space::S)
        .padding([space::XS as u16, space::S as u16])
        .align_y(iced::alignment::Vertical::Center)
        .push(config_cell)
        .push(verdict_cell)
        .push(return_cell)
        .push(spread_cell)
        .push(prob_loss_cell)
        .push(prob_sharpe_cell)
        .push(maxdd_cell)
        .push(use_cell);

    // Fragile rows get a quiet DOWN_50 wash so the row reads as the notable,
    // ineligible case at a glance (paired with the badge + the locked action —
    // colour is never the only signal).
    if matches!(cell.verdict, SweepVerdictLabel::Fragile) {
        Container::new(row)
            .width(Length::Fill)
            .style(move |_t: &iced::Theme| iced::widget::container::Style {
                background: Some(color::DOWN_50.current(mode).into()),
                border: Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: radius::R3.into(),
                },
                ..Default::default()
            })
            .into()
    } else {
        Container::new(row).width(Length::Fill).into()
    }
}

/// The "Use this config" action cell — enabled `ACCENT` affordance on a
/// promotable row, disabled+greyed on a fragile row (the promotion lock). A
/// fragile row additionally carries the inline "would be overfitting" note.
///
/// Promotion WIRING is out of scope for v0.1 (the affordance carries no message
/// yet); the disabled-on-fragile treatment is what ships now so the honesty is
/// visible from day 1. The enabled affordance is a visual pill (a v0.2 wires the
/// carry-forward) that still reads as the strictly-more-accent, eligible state.
fn use_config_cell(cell: &SweepCellRow, mode: ThemeMode) -> crate::Element<'_> {
    if cell.promotable {
        // Enabled accent affordance — a soft accent pill with an accent label.
        let pill = Container::new(
            Text::new(TUNE_USE_CONFIG)
                .size(text::SMALL)
                .color(color::ACCENT.current(mode)),
        )
        .padding([space::XXS as u16, space::S as u16])
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::ACCENT_SOFT.current(mode).into()),
            border: Border {
                color: color::ACCENT.current(mode),
                width: 1.0,
                radius: radius::R3.into(),
            },
            text_color: Some(color::ACCENT.current(mode)),
            ..Default::default()
        });
        Container::new(pill).width(Length::Fixed(W_USE)).into()
    } else {
        // Disabled greyed lock + the inline "overfitting" note (stacked).
        let locked = Container::new(
            Text::new(TUNE_USE_CONFIG_FRAGILE)
                .size(text::SMALL)
                .color(color::FG_3.current(mode)),
        )
        .padding([space::XXS as u16, space::S as u16])
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R3.into(),
            },
            text_color: Some(color::FG_3.current(mode)),
            ..Default::default()
        });
        Container::new(
            Column::new().spacing(space::XXS).push(locked).push(
                Text::new(TUNE_FRAGILE_PROMOTE_NOTE)
                    .size(text::MICRO)
                    .color(color::FG_3.current(mode)),
            ),
        )
        .width(Length::Fixed(W_USE))
        .into()
    }
}

/// The verdict pill. FRAGILE is the prominent, promotion-blocking state: a soft
/// `DOWN_50` backdrop + a saturated `DOWN_500` label (the leaderboard's
/// `fragile_badge` treatment, the exact pixels its render guard checks). Robust
/// = a quiet `UP`-tinted label; Marginal = a neutral muted label. The word is
/// always present so colour is never the only signal.
fn verdict_pill(verdict: SweepVerdictLabel, mode: ThemeMode) -> crate::Element<'static> {
    match verdict {
        SweepVerdictLabel::Fragile => Container::new(
            Text::new(TUNE_VERDICT_FRAGILE)
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
        .into(),
        SweepVerdictLabel::Robust => Text::new(TUNE_VERDICT_ROBUST)
            .size(text::SMALL)
            .color(color::UP_500.current(mode))
            .into(),
        SweepVerdictLabel::Marginal => Text::new(TUNE_VERDICT_MARGINAL)
            .size(text::SMALL)
            .color(color::FG_3.current(mode))
            .into(),
        // NotChecked never occurs for a sweep cell (the gate always runs), but
        // the match is exhaustive — render nothing rather than panic.
        SweepVerdictLabel::NotChecked => Space::new()
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into(),
    }
}

/// A compact inline tag (`SMALL`, coloured), each carrying a word.
fn tag(label: &str, fg: iced::Color) -> crate::Element<'static> {
    Text::new(label.to_string())
        .size(text::SMALL)
        .color(fg)
        .into()
}

/// A right-aligned numeric cell at a fixed width.
fn num_cell(value: String, value_color: iced::Color, width: f32) -> crate::Element<'static> {
    Text::new(value)
        .size(text::BODY)
        .color(value_color)
        .width(Length::Fixed(width))
        .align_x(iced::alignment::Horizontal::Right)
        .into()
}

// ── Disclaimer ────────────────────────────────────────────────────────────────

/// The persistent NOT-ADVICE + FRAGILE-is-overfit honesty footer (ADR-0069 § 7).
/// `MICRO` muted so it is always present but never shouts; `{coin}` filled here.
fn disclaimer(coin: &str, mode: ThemeMode) -> crate::Element<'static> {
    Text::new(TUNE_DISCLAIMER.replace("{coin}", coin))
        .size(text::MICRO)
        .color(color::FG_3.current(mode))
        .width(Length::Fill)
        .into()
}

// ── Small numeric helpers ──────────────────────────────────────────────────────

/// Format a probability fraction (`0.12` = 12%) as a whole-percent string. The
/// gate signals (P(loss), P(Sharpe>1)) are coarse credibility numbers, so a
/// whole-percent display is honest (no false precision).
fn fmt_prob(p: f64) -> String {
    let pct = (p.clamp(0.0, 1.0) * 100.0).round();
    format!("{pct:.0}%")
}

/// Colour for the P(loss) cell — `DOWN_500` above a coarse 33% floor (a config
/// that loses money in a third of resamples is a real caution), else neutral.
fn prob_loss_color(p: f64, mode: ThemeMode) -> iced::Color {
    if p > 0.33 {
        color::DOWN_500.current(mode)
    } else {
        color::FG_1.current(mode)
    }
}

/// Convert a max-DD fraction (`0.3` = 30%) to percent-points for
/// `format_pct_max_dd` (which expects percent input, e.g. `30.0`).
fn maxdd_to_pct(fraction: f64) -> rust_decimal::Decimal {
    rust_decimal::Decimal::try_from(fraction * 100.0).unwrap_or(rust_decimal::Decimal::ZERO)
}

/// Format a fraction (`0.0738` = +7.38%) as a signed, sentiment-coloured pct.
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

/// Convert an `f64` Sharpe to `Decimal` for `format_sharpe` (display-only).
fn sharpe_to_decimal(sharpe: f64) -> rust_decimal::Decimal {
    rust_decimal::Decimal::try_from(sharpe).unwrap_or(rust_decimal::Decimal::ZERO)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// Every family maps to a non-empty chip label (no blank chip).
    #[test]
    fn every_family_has_a_label() {
        for &fam in TuneFamily::ALL {
            assert!(
                !family_label(fam).is_empty(),
                "family {fam:?} needs a label"
            );
        }
    }

    /// `view` constructs without panic at both theme modes for the Empty + Ready
    /// states (smoke — the render-layer proof is the screenshot harness).
    #[test]
    fn view_constructs_both_modes_empty_and_ready() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            // Empty.
            let empty = crate::fixtures::fake_cockpit_tune(PanelState::Empty);
            let _ = view(&empty, mode);
            // Ready (populated grid with a fragile cell).
            let mirror = crate::fixtures::fake_sweep_report_mirror();
            let ready = crate::fixtures::fake_cockpit_tune(PanelState::Ready(mirror));
            let _ = view(&ready, mode);
        }
    }

    /// The probability formatter clamps + rounds to whole percent.
    #[test]
    fn fmt_prob_clamps_and_rounds() {
        assert_eq!(fmt_prob(0.0), "0%");
        assert_eq!(fmt_prob(0.5), "50%");
        assert_eq!(fmt_prob(1.0), "100%");
        assert_eq!(fmt_prob(1.5), "100%"); // clamped
        assert_eq!(fmt_prob(0.126), "13%"); // rounded
    }
}
