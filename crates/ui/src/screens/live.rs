//! Live trading dashboard screen — Phase C (ui-rethink-phase-c-sidebar-ia).
//!
//! Layout per dev-note §J6 lines 528-542 (R2.2):
//!
//! 1. **Top:** system-health strip — latency row + market-health summary
//!    + server-time badge. Draws from `screens::debug` helpers exposed as
//!    `pub(crate)` so both surfaces stay in sync without code duplication.
//! 2. **Mid-top:** full-width equity curve (`widgets::equity_curve`) bound to
//!    `model.live_equity_curve` — the session-scoped live equity series
//!    accumulated from the per-bar `pnl` feed (cockpit-live-dashboard-wiring
//!    v0.1.0 / D1=(a)). `Loading` until the first bar; grows per bar.
//! 3. **Mid-bottom:** KPI strip (`widgets::kpi_strip`) bound to
//!    `model.live_kpi` (live session Total-return + Max-DD; Sharpe/CAGR/Win
//!    `—`; Trades = the session fill count) + LLM-spend text tile (Design
//!    § A9 / Q4b — a separate Phase-F placeholder, untouched here).
//! 4. **Bottom:** 2-column row — `widgets::positions` LEFT,
//!    `widgets::agent_feed` RIGHT.
//!
//! `widgets::equity_curve` / `widgets::kpi_strip` return
//! `Element<'_, ViewerMessage>`. The `.map(|_| Message::ChartMarkerHoverEnded)`
//! adapter bridges the type gap — a harmless never-fired no-op arm (the live
//! curve/strip emit no interactions), exactly as `screens/baseline.rs` does.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.
//! **No new theme token, no new widget** (AC7).

#![allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]

use iced::Length;
use iced::widget::{Column, Container, Row, Text};

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use trading_core::FxNote;

use crate::state::{Cockpit, LiveEquityWindow, Message, PanelState};
use crate::strings::{
    LIVE_EQUITY_AS_OF_AGE_FMT, LIVE_EQUITY_AS_OF_FMT, LIVE_FORWARD_BUDGET_LABEL,
    LIVE_FORWARD_DISCLAIMER, LIVE_FORWARD_FX_NOTE_FMT, LIVE_FORWARD_PNL_LABEL,
    LIVE_FORWARD_RUNNING_FMT, LIVE_HEADLINE, LIVE_LLM_SPEND_LABEL, LIVE_LLM_SPEND_PLACEHOLDER,
    LIVE_ROLLING_WINDOW_CAPTION, LIVE_SESSION_RETURN_CAPTION, LIVE_SINCE_INCEPTION_CAPTION,
    LIVE_SYSTEM_HEALTH_LABEL, SHORT_UNBOUNDED_LOSS_DISCLAIMER,
};
use crate::theme::{ThemeMode, color, layout, space, text};
use crate::widgets::num::{fmt_eur_plain, fmt_rate, fmt_usdt_plain};
use crate::widgets::{agent_feed, equity_curve, kpi_strip, latency, positions};

/// Render the Live screen body (R2.1 / R2.2).
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    // Headline row.
    let headline = Text::new(LIVE_HEADLINE)
        .size(text::H2)
        .color(color::FG_1.current(mode));

    // ── System health strip (top) ────────────────────────────────────────────
    let health_label = Text::new(LIVE_SYSTEM_HEALTH_LABEL)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let health_strip = Row::new()
        .spacing(space::M)
        .push(health_label)
        .push(latency::view(model))
        .push(crate::screens::debug::server_time_compact(model, mode))
        .push(crate::screens::debug::market_health_compact(model, mode));

    // ── Equity curve (full-width, live) ──────────────────────────────────────
    // cockpit-live-dashboard-wiring — bound to the model-cached, session-scoped
    // live equity series (derived on-append in the `PnlRefreshed` arm, D5).
    // `Loading` until the first bar; grows per bar; `Error`/`Empty` degrade
    // honestly. `.map(|_| Message::ChartMarkerHoverEnded)` bridges
    // ViewerMessage → Message — a never-fired no-op (the live curve emits no
    // interactions), mirroring `screens/baseline.rs`.
    let equity_state = &model.live_equity_curve;
    let equity = equity_curve::view(equity_state, mode).map(|_| Message::ChartMarkerHoverEnded);

    // ── KPI strip + LLM spend tile ──────────────────────────────────────────
    // cockpit-live-dashboard-wiring — bound to the model-cached live KPI strip
    // (Loading until ≥2 points, D2 — a 1-point series carries no return
    // interval). Total-return (session) + Max-DD are live; Sharpe/CAGR/Win-rate
    // render `—`; Trades is the session fill count. From 2 points on a flat
    // feed renders `0.00% / 0.00% / 0`, NOT six dashes (2-15 review H2).
    let kpi_state = &model.live_kpi;
    let kpi = kpi_strip::view(kpi_state, mode).map(|_| Message::ChartMarkerHoverEnded);

    // Design § A9 / Q4b: text-only LLM spend tile (real wiring in Phase F).
    let llm_label = Text::new(LIVE_LLM_SPEND_LABEL)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let llm_value = Text::new(LIVE_LLM_SPEND_PLACEHOLDER)
        .size(text::BODY)
        .color(color::FG_2.current(mode));
    let llm_tile = Column::new()
        .spacing(space::XS)
        .push(llm_label)
        .push(llm_value);

    let kpi_row = Row::new()
        .spacing(layout::PANEL_OUTER_GAP)
        .push(kpi)
        .push(Container::new(llm_tile).width(Length::Fixed(120.0)));

    let session_caption = Text::new(return_scope_caption(model))
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    // 2-15 review M8 — the "as of" marker. Only rendered once at least one
    // snapshot has been delivered (so the fresh-boot Loading screen is
    // unchanged); see `live_equity_as_of_label`.
    let as_of_caption = live_equity_as_of_label(model).map(|label| {
        Text::new(label)
            .size(text::SMALL)
            .color(color::FG_3.current(mode))
    });

    // ── F5 — Forward paper-trade P/L framing ────────────────────────────────
    // Rendered only when `model.forward_budget` is `Some(budget)`.
    //
    // P/L = equity − budget   (USDT; no FX conversion)
    // P/L% = (equity − budget) / budget × 100
    //
    // Color: green (UP_500) when P/L ≥ 0, red (DOWN_500) when P/L < 0.
    // Label: `LIVE_FORWARD_PNL_LABEL` ("P/L").
    // FX note: `LIVE_FORWARD_FX_NOTE` ("€200 ≈ 200 USDT — FX not modelled.")
    // Disclaimer: `LIVE_FORWARD_DISCLAIMER` (not-advice + simulated-budget).
    //
    // When `forward_budget` is `None` (legacy research / soak path), the
    // block is not rendered — pre-F5 byte-identical behaviour preserved.
    // F7: thread the FX note from the Cockpit model into the P/L block.
    // advisor-short-selling (T-U4 / ADR-0068 § D5/D8) — the load-bearing
    // "a short can lose more than your €200" unbounded-loss disclaimer is
    // carried in the forward P/L block whenever the forward run is currently
    // holding a SHORT (a signed `base_qty < 0` in the positions panel). A
    // long-only forward run does not carry it (the disclaimer is short-specific).
    let holding_short = holding_short(model);
    let forward_pnl_block: Option<crate::Element<'_>> =
        model.forward_budget.as_ref().map(|budget| {
            build_forward_pnl_block(
                model,
                budget,
                model.forward_fx.as_ref(),
                holding_short,
                mode,
            )
        });

    // ── 2-column positions / activity row ───────────────────────────────────
    let bottom_row = Row::new()
        .spacing(layout::PANEL_OUTER_GAP)
        .push(positions::view(model))
        .push(agent_feed::view(model));

    // ── Full-screen column ───────────────────────────────────────────────────
    let mut col = Column::new()
        .padding(space::L as u16)
        .spacing(space::M)
        .push(headline)
        .push(health_strip)
        .push(equity)
        .push(kpi_row)
        .push(session_caption);

    if let Some(caption) = as_of_caption {
        col = col.push(caption);
    }

    if let Some(block) = forward_pnl_block {
        col = col.push(block);
    }

    col.push(bottom_row)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The scope caption rendered under the Live KPI strip — **the** statement
/// about what window the Total-return / Max-DD cards actually cover.
///
/// This is the shared seam: `view` renders exactly this string, and the
/// `panel_snapshots.rs` mirror asserts through this function rather than
/// re-deriving the conditional (2-15 review M4's drift lesson applied
/// pre-emptively).
///
/// Precedence, most-honest-first:
///
/// 1. `live_equity_window == Rolling` → [`LIVE_ROLLING_WINDOW_CAPTION`]. The
///    ring evicted its head (or the boot hydrate was capped at the reader's
///    `LIMIT`), so `live_equity_buffer[0]` — the Total-return denominator, and
///    the series the Max-DD peak is scanned over — is neither the session open
///    nor account inception. Both other captions would be false, so this wins
///    over the hydrate switch (2-15 review H3).
/// 2. `live_equity_hydrated` → [`LIVE_SINCE_INCEPTION_CAPTION`]: a durable
///    history was loaded whole and its first row IS the first persisted point.
/// 3. otherwise → [`LIVE_SESSION_RETURN_CAPTION`]: the buffer starts at this
///    session's first bar.
#[must_use]
pub fn return_scope_caption(model: &Cockpit) -> &'static str {
    if model.live_equity_window == LiveEquityWindow::Rolling {
        LIVE_ROLLING_WINDOW_CAPTION
    } else if model.live_equity_hydrated {
        LIVE_SINCE_INCEPTION_CAPTION
    } else {
        LIVE_SESSION_RETURN_CAPTION
    }
}

/// The Live equity "as of" line, or `None` when no snapshot has ever been
/// delivered (a fresh boot renders the Loading bodies and no marker — there is
/// nothing to be stale about yet).
///
/// **Why this exists (2-15 review M8).** If the P&L producer dies *without*
/// closing its channel, no `PnlError` ever arrives: both panels stay `Ready`
/// on the last series forever, and a stopped feed is pixel-identical to a flat
/// market. The health strip does not cover it — its `last_tick` age tracks the
/// MARKET-DATA feed (`Cockpit::last_tick_ts`, set by `Message::Tick`), which
/// keeps ticking merrily while the equity feed is dead. So the equity panels
/// carry their own stamp, from `live_equity_last_as_of` (the wallclock stamp of
/// the last ACCEPTED snapshot — the same key the delivery guard uses).
///
/// When the server clock is known the age is stated outright; otherwise the
/// bare timestamp is rendered next to the health strip's server-time badge, in
/// the same `HH:MM:SS UTC` shape, so the comparison is one glance. A
/// thresholded "STALE" badge is deliberately NOT added here: the threshold
/// would be a guess (bar intervals run from 1 m to 1 d) and thresholded
/// liveness is the health strip's job — this is the fact, not a verdict.
#[must_use]
pub fn live_equity_as_of_label(model: &Cockpit) -> Option<String> {
    let as_of = model.live_equity_last_as_of?;
    let dt = as_of.inner();
    let time = format!("{:02}:{:02}:{:02}", dt.hour(), dt.minute(), dt.second());
    Some(match model.server_time_now {
        Some(now) => {
            let age_s = (now.unix_millis() - as_of.unix_millis()).max(0) / 1_000;
            LIVE_EQUITY_AS_OF_AGE_FMT
                .replace("{time}", &time)
                .replace("{age}", &age_s.to_string())
        }
        None => LIVE_EQUITY_AS_OF_FMT.replace("{time}", &time),
    })
}

/// `true` when the forward paper-trade is currently holding a SHORT — any open
/// position with a signed `base_qty < 0` (advisor-short-selling, ADR-0068 § D8).
/// Drives the unbounded-loss disclaimer in the forward P/L block. Read-only on
/// `&Cockpit`; degrades to `false` for any non-`Ready` positions state.
fn holding_short(model: &Cockpit) -> bool {
    match &model.positions {
        PanelState::Ready(positions) => positions.iter().any(|p| p.base_qty.is_sign_negative()),
        _ => false,
    }
}

/// Build the F5 forward P/L card for `budget`.
///
/// Computes P/L = `total_equity` − `budget` from the latest `PnlSnapshot` in
/// `model.pnl`. Falls back to "—" while the first snapshot is still pending.
///
/// The block is a `Column` with:
/// 1. Running-caption row (strategy id + "simulated budget" label).
/// 2. P/L row: label "P/L" + value (coloured green / red + sign).
///    Budget row: label "Budget" + formatted budget amount.
/// 3. FX note (F7 / ADR-0065): "€X ≈ $Y (at R EUR/USD, source)" when `fx_note`
///    is `Some`; otherwise a fallback from `DEFAULT_EUR_USD_RATE`.
/// 4. Disclaimer (`LIVE_FORWARD_DISCLAIMER`).
/// 5. Unbounded-loss disclaimer (`SHORT_UNBOUNDED_LOSS_DISCLAIMER`) when
///    `holding_short` (advisor-short-selling, T-U4).
// A cohesive card-builder (P/L + budget + FX note + disclaimers); splitting the
// rows into sub-helpers would scatter the one card across the module. The
// advisor-short-selling T-U4 disclaimer push nudged it past the 100-line lint.
#[allow(clippy::too_many_lines)]
fn build_forward_pnl_block<'a>(
    model: &'a Cockpit,
    budget: &trading_core::Money<trading_core::Usdt>,
    fx_note: Option<&FxNote>,
    holding_short: bool,
    mode: ThemeMode,
) -> crate::Element<'a> {
    use iced::widget::{Column, Row, Text};

    // ── Compute P/L ──────────────────────────────────────────────────────────
    let (pnl_text, pnl_pct_text, pnl_color) = if let PanelState::Ready(snap) = &model.pnl {
        let equity_amt = snap.total_equity.amount();
        let budget_amt = budget.amount();
        let pnl_raw = equity_amt - budget_amt;
        let sign = if pnl_raw >= dec!(0) { "+" } else { "" };
        let pnl_str = format!("{sign}{pnl_raw:.2} USDT");
        let pnl_pct_str = if budget_amt.is_zero() {
            String::from("(\u{2014}%)")
        } else {
            let pct = pnl_raw / budget_amt * Decimal::from(100);
            format!("({sign}{pct:.2}%)")
        };
        let col = if pnl_raw >= dec!(0) {
            color::UP_500.current(mode)
        } else {
            color::DOWN_500.current(mode)
        };
        (pnl_str, pnl_pct_str, col)
    } else {
        (
            String::from("\u{2014}"),
            String::new(),
            color::FG_2.current(mode),
        )
    };

    // ── Running caption ───────────────────────────────────────────────────────
    // If we know the strategy from the leaderboard state, show it; otherwise
    // just "Simulated budget — see Leaderboard."
    let caption_str = {
        let sid = if let PanelState::Ready(mirror) = &model.leaderboard_screen_state.result {
            mirror.crowned_row().map_or_else(
                || String::from("selected strategy"),
                |r| r.strategy.as_str().to_owned(),
            )
        } else {
            String::from("selected strategy")
        };
        LIVE_FORWARD_RUNNING_FMT.replace("{strategy}", &sid)
    };

    let running_caption = Text::new(caption_str)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    // ── P/L row ───────────────────────────────────────────────────────────────
    let pnl_label = Text::new(LIVE_FORWARD_PNL_LABEL)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let pnl_value_text = if pnl_pct_text.is_empty() {
        pnl_text.clone()
    } else {
        format!("{pnl_text}  {pnl_pct_text}")
    };
    let pnl_value = Text::new(pnl_value_text).size(text::BODY).color(pnl_color);
    let pnl_col = Column::new()
        .spacing(space::XS)
        .push(pnl_label)
        .push(pnl_value);

    // ── Budget row ────────────────────────────────────────────────────────────
    let budget_label = Text::new(LIVE_FORWARD_BUDGET_LABEL)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let budget_value = Text::new(format!("{:.0} USDT", budget.amount()))
        .size(text::BODY)
        .color(color::FG_2.current(mode));
    let budget_col = Column::new()
        .spacing(space::XS)
        .push(budget_label)
        .push(budget_value);

    // ── Metric row (P/L + Budget side by side) ────────────────────────────────
    let metric_row = Row::new()
        .spacing(layout::PANEL_OUTER_GAP)
        .push(pnl_col)
        .push(budget_col);

    // ── FX note + disclaimer ──────────────────────────────────────────────────
    // F7 (ADR-0065) — the honest EUR→USDT note. Uses the FxNote from the forward
    // run when present; falls back to a default from DEFAULT_EUR_USD_RATE when the
    // run was launched without an FX note (legacy research / soak path).
    let fx_note_text: String = if let Some(note) = fx_note {
        LIVE_FORWARD_FX_NOTE_FMT
            .replace("{eur}", &fmt_eur_plain(note.eur))
            .replace("{usdt}", &fmt_usdt_plain(note.usdt))
            .replace("{rate}", &fmt_rate(note.rate))
            .replace("{source}", note.source.as_str())
    } else {
        use trading_core::{BudgetConversion, DEFAULT_EUR_USD_RATE, FxRate};
        let fx = FxRate::config(DEFAULT_EUR_USD_RATE);
        let conv = BudgetConversion::new(budget.amount(), fx);
        LIVE_FORWARD_FX_NOTE_FMT
            .replace("{eur}", &fmt_eur_plain(conv.eur()))
            .replace("{usdt}", &fmt_usdt_plain(conv.usdt().amount()))
            .replace("{rate}", &fmt_rate(conv.rate().rate()))
            .replace("{source}", conv.rate().source())
    };
    let fx_note_widget = Text::new(fx_note_text)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let disclaimer = Text::new(LIVE_FORWARD_DISCLAIMER)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    let mut col = Column::new()
        .spacing(space::XS)
        .push(running_caption)
        .push(metric_row)
        .push(fx_note_widget)
        .push(disclaimer);

    // advisor-short-selling (T-U4, LOAD-BEARING) — when a short is open, carry
    // the unbounded-loss disclaimer on the Live surface. WARN_500-tinted (paired
    // with the word, so colour is never the only signal) so the "can lose more
    // than your €200" caution reads as a real risk note, not muted fine print.
    if holding_short {
        col = col.push(
            Text::new(SHORT_UNBOUNDED_LOSS_DISCLAIMER)
                .size(text::SMALL)
                .color(color::WARN_500.current(mode))
                .width(iced::Length::Fill),
        );
    }

    col.into()
}
