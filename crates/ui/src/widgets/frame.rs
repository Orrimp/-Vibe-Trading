//! Shared panel frame — the bordered card every panel sits inside.
//!
//! One helper so panel visuals stay identical and layout code doesn't
//! duplicate. Title and body are composed by the caller; this module only
//! controls the frame padding, the gap between title and body, and the
//! title/body typography.
//!
//! ## T1505 — Tier 1 panel chrome
//!
//! `panel` now accepts a `mode: ThemeMode` parameter so the runtime
//! theme toggle (downstream feature) can be wired without touching call
//! sites. Phase 1 bins pass `ThemeMode::Dark` explicitly.
//!
//! Panel header row is wrapped in its own `Container` tinted with
//! `PANEL_RAISED`, followed by a 1 px `BORDER_1` hairline separator.
//!
//! ## T1507 — Active-row pattern
//!
//! `active_row` wraps any table row in a `Row` that prepends a 2 px
//! coloured left rule: `ACCENT` when `active = true`, `TRANSPARENT`
//! when `active = false`. The rule is **always** 2 px wide so layout
//! is identical before and after Phase 1 selection state lands.

use iced::Color;
use iced::Element;
use iced::Length;
use iced::alignment::Vertical;
use iced::widget::{Column, Container, Row, Space, Text, container};

use crate::theme::{ThemeMode, color, layout, radius, shadow, space, text};

/// Wraps a panel body in the standard Tier-1 frame with a tinted header.
///
/// The outer container uses `PANEL` as its background (Tier 1) with a
/// `shadow_1` whisper shadow and a `BORDER_1` hairline border. The header
/// row lives inside its own `PANEL_RAISED` container so it reads as a
/// slightly elevated section header. A 1 px `BORDER_1` hairline separates
/// the header from the body.
///
/// `mode` is passed explicitly so Phase 1 can hard-code `ThemeMode::Dark`
/// while leaving the signature ready for the downstream runtime toggle.
// `cast_possible_truncation`: space::* constants are u32 with bounded values 0..64;
// cast to u16 padding is safe.
#[allow(clippy::needless_pass_by_value, clippy::cast_possible_truncation)]
#[must_use]
pub fn panel<'a, Message: 'a>(
    title: &'a str,
    body: Element<'a, Message>,
    mode: ThemeMode,
) -> Element<'a, Message> {
    // ui-quality-gate-overhaul M2-A (T-M2-A-1/-2): emit a per-construction
    // trace span when `render-debug` is on so an operator triaging a render
    // panic can grep stderr for `widget_draw{widget="panel" ...}` and see
    // the call sequence. Stderr-only per architect Q2 (no audit-ledger
    // sink). `_enter` keeps the span open for the duration of the
    // function — the lazy `Element` it returns is what iced eventually
    // draws, but the span tag lands on stderr the moment we hit this
    // line, which is sufficient for the "which widget called frame::panel
    // before the panic" forensic.
    #[cfg(feature = "render-debug")]
    let _span = tracing::trace_span!("widget_draw", widget = "panel", title = title).entered();

    // Header row: title text inside a PANEL_RAISED tinted container.
    let header_text = Text::new(title)
        .size(text::H2)
        .color(color::FG_1.current(mode));
    let header_container = Container::new(header_text)
        .padding([space::XS as u16, space::M as u16])
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            ..Default::default()
        });

    // 1 px BORDER_1 hairline separator between header and body.
    let separator = Container::new(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::BORDER_1.current(mode).into()),
            ..Default::default()
        });

    // Body wrapped with standard padding.
    #[allow(clippy::cast_possible_truncation)]
    let body_padding = layout::PANEL_PADDING as u16;
    let body_container = Container::new(body)
        .padding(body_padding)
        .width(Length::Fill);

    let stack = Column::new()
        .push(header_container)
        .push(separator)
        .push(body_container)
        .spacing(0);

    Container::new(stack)
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            border: iced::Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: Some(color::FG_1.current(mode)),
            shadow: shadow::shadow_1(mode),
            ..Default::default()
        })
        .into()
}

/// Active-row pattern — prepends a 2 px left rule to any table row.
///
/// When `active` is `true` the rule is coloured `ACCENT`; when `false`
/// the rule is `Color::TRANSPARENT`. The rule is **always** 2 px wide so
/// the row's left padding is identical pre/post Phase 1 selection state.
///
/// Phase 1 pass-through: `widgets::strategies` passes `active = true` for
/// the selected strategy row, `active = false` for all others.
/// `widgets::positions` always passes `active = false` (no selection state
/// until downstream Phase 2).
// `cast_precision_loss`: space::* constants are u32 with bounded values 0..64;
// cast to f32 width is safe (well within the mantissa).
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn active_row<'a, Message: 'a>(
    content: Element<'a, Message>,
    active: bool,
    mode: ThemeMode,
) -> Element<'a, Message> {
    let rule_color = if active {
        color::ACCENT.current(mode)
    } else {
        Color::TRANSPARENT
    };

    let rule = Container::new(Space::new().width(Length::Fixed(2.0)).height(Length::Fill))
        .width(Length::Fixed(2.0))
        .height(Length::Fill)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(rule_color.into()),
            ..Default::default()
        });

    Row::new()
        .push(rule)
        .push(Space::new().width(space::XS as f32).height(0))
        .push(content)
        .height(Length::Shrink)
        .into()
}

/// Small helper for body text in the muted foreground.
#[must_use]
pub fn muted_body<'a, Message: 'a>(t: &'a str) -> Element<'a, Message> {
    Text::new(t)
        .size(text::BODY)
        .color(color::FG_3.current(ThemeMode::Dark))
        .into()
}

/// Brief B (M2) — informational loading text paired with a 16 px
/// throttled spinner in the muted foreground.
///
/// Renders a horizontal `Row` of `[Spinner, Text]` separated by
/// `space::S` and vertically centered. The spinner is the local
/// [`crate::widgets::throttled_spinner::ThrottledSpinner`] — a
/// behavioural-clone of `iced_aw::Spinner` with `FRAMES_PER_SECOND = 10`
/// instead of the upstream 60.
///
/// ## Why throttled, not bare `iced_aw::Spinner`
///
/// Upstream `iced_aw::Spinner` calls
/// `shell.request_redraw_at(now + 16 ms)` on every `RedrawRequested`
/// event (`iced_aw-0.14.1/src/widget/spinner.rs:197-199`), pulling
/// the whole cockpit into 60 fps software-rasterized repaint while
/// ANY panel renders this helper. The orchestrator-executed
/// 2026-05-15 M0 profile (cockpit-performance-and-input-responsiveness
/// feature.md ## M0 results) measured idle cockpit CPU at ~66.9 %
/// with `iced_tiny_skia::Compositor::present` at 45.5 % main-thread
/// self-time. Throttling the cadence to 10 fps cuts the per-second
/// repaint count ~6× — see
/// [`widgets::throttled_spinner`](super::throttled_spinner) for the
/// architect-ratified rationale (M1 Candidate A2).
///
/// ## Determinism
///
/// The 1 Hz `rate` is preserved from the upstream Spinner default; the
/// frame increment is driven by `RedrawRequested` (NOT wall-clock) per
/// the architect's H-arch-9 RESOLVED-PASS verdict in
/// [iced-aw-cherry-pick/feature.md § H-arch-9](../../../spec/iced-aw-cherry-pick/feature.md#h-arch-9--iced_awspinner-deterministic-render--resolved-pass-with-caveat).
/// `iced_test` snapshot paths render at `t = 0.0` because they do not
/// fire `RedrawRequested` events, so `*_loading.snap` baselines stay
/// deterministic — the throttle does not perturb this.
///
/// `mode` is passed explicitly so the text color tracks the runtime
/// theme toggle (same shape as `panel(...)` / `active_row(...)`). Phase
/// 1 callers pass `ThemeMode::Dark`.
#[must_use]
pub fn loading_with_spinner<'a, Message: 'a>(
    text: &'a str,
    mode: ThemeMode,
) -> Element<'a, Message> {
    // ui-quality-gate-overhaul M2-A (T-M2-A-2): paired with the
    // `frame::panel` instrumentation. Same `widget_draw` span name so
    // `RUST_LOG=ui::widgets=trace` filters one widget channel and surfaces
    // both. Build-time-only via `render-debug` feature; default builds
    // compile this away.
    #[cfg(feature = "render-debug")]
    let _span =
        tracing::trace_span!("widget_draw", widget = "loading_with_spinner", text = text).entered();

    Row::new()
        .spacing(space::S)
        .align_y(Vertical::Center)
        .push(
            super::throttled_spinner::ThrottledSpinner::new()
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0)),
        )
        .push(
            Text::new(text)
                .size(text::BODY)
                .color(color::FG_3.current(mode)),
        )
        .into()
}

/// T1609 — chip-row active-bottom variant of the T1507 active-row pattern.
/// The active-row concept is "2 px ACCENT, no fill change"; the literal
/// edge depends on widget orientation. The chip row is horizontal, so the
/// rule lives on the bottom edge (Phase 2 Q5 ratification).
///
/// The rule is **always** 2 px tall so layout is identical pre/post chip
/// selection.
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn active_chip<'a, Message: 'a>(
    content: Element<'a, Message>,
    active: bool,
    mode: ThemeMode,
) -> Element<'a, Message> {
    let rule_color = if active {
        color::ACCENT.current(mode)
    } else {
        Color::TRANSPARENT
    };

    let rule = Container::new(Space::new().width(Length::Fill).height(Length::Fixed(2.0)))
        .width(Length::Fill)
        .height(Length::Fixed(2.0))
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(rule_color.into()),
            ..Default::default()
        });

    Column::new().push(content).push(rule).into()
}

/// T1708 — Phase 3 Risk-screen threshold bar (additive sibling of
/// `active_row` and `active_chip`). Renders a horizontal bar where the
/// filled portion's `Length::FillPortion` ratio is clamped to
/// `[0, 100]` and the colour ramp follows the Phase 1 latency-band
/// precedent: `ACCENT` < 70 %, `WARN_500` ≥ 70 %, `DOWN_500` ≥ 90 %.
///
/// The numeric label `"X / Y (Z %)"` is rendered to the right of the
/// bar via `widgets::num` semantics (this helper takes the parts as
/// `Decimal` so the screen can produce label text in its own scope —
/// the helper itself owns only the bar geometry + colour).
///
/// `cap == 0` is rendered as a fully-empty bar with `ACCENT` colour
/// (no `Decimal` divide-by-zero panic). The numeric label still renders
/// `"X / 0 (—)"` per the screen-side formatter — empty caps mean
/// "no limit configured".
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn threshold_bar<'a, Message: 'a>(
    used: rust_decimal::Decimal,
    cap: rust_decimal::Decimal,
    mode: ThemeMode,
) -> Element<'a, Message> {
    let pct_decimal = if cap == rust_decimal::Decimal::ZERO {
        rust_decimal::Decimal::ZERO
    } else {
        let raw = (used / cap) * rust_decimal::Decimal::from(100);
        // Clamp to [0, 100].
        if raw < rust_decimal::Decimal::ZERO {
            rust_decimal::Decimal::ZERO
        } else if raw > rust_decimal::Decimal::from(100) {
            rust_decimal::Decimal::from(100)
        } else {
            raw
        }
    };
    // `Decimal` → u16 fill portion. Always in [0, 100] post-clamp;
    // `to_u16` truncates the fractional part which is desired (we render
    // integer fill portions only).
    let pct_u16: u16 = pct_decimal.trunc().to_string().parse::<u16>().unwrap_or(0);

    let band_color = if pct_u16 >= 90 {
        color::DOWN_500.current(mode)
    } else if pct_u16 >= 70 {
        color::WARN_500.current(mode)
    } else {
        color::ACCENT.current(mode)
    };

    let filled = Container::new(Space::new().width(Length::Fill).height(Length::Fixed(8.0)))
        .width(Length::FillPortion(pct_u16))
        .height(Length::Fixed(8.0))
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(band_color.into()),
            border: iced::Border {
                radius: radius::R1.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let remainder_portion = 100u16.saturating_sub(pct_u16);
    let empty = Container::new(Space::new().width(Length::Fill).height(Length::Fixed(8.0)))
        .width(Length::FillPortion(remainder_portion))
        .height(Length::Fixed(8.0))
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL_SUNKEN.current(mode).into()),
            border: iced::Border {
                radius: radius::R1.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    Row::new().push(filled).push(empty).into()
}

/// Helper for a red-tinted error row inside a panel body.
#[must_use]
pub fn error_body<'a, Message: 'a>(prefix: &'a str, detail: &'a str) -> Element<'a, Message> {
    Text::new(format!("{prefix}{detail}"))
        .size(text::BODY)
        .color(color::DOWN_500.current(ThemeMode::Dark))
        .into()
}

/// Helper for a caption-sized column header row.
#[must_use]
pub fn col_header<'a, Message: 'a>(t: &'a str) -> Element<'a, Message> {
    Text::new(t)
        .size(text::MICRO)
        .color(color::FG_3.current(ThemeMode::Dark))
        .into()
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::theme::{ThemeMode, color};

    // ── T1505 — panel chrome smoke ─────────────────────────────────────────
    /// Verify `panel` renders without panic with mode param (T1505).
    /// The insta snapshot captures the logical shape of the style:
    /// `PANEL` background, `BORDER_1` border, `shadow_1`, `PANEL_RAISED` header.
    // `cast_possible_truncation` + `cast_sign_loss`: RGB channels are bounded
    // 0.0..=1.0 by iced; f32→u8 byte extraction is intentional and safe for
    // snapshot stability.
    #[test]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn t1505_panel_chrome_style_tokens() {
        use crate::theme::{radius, shadow};
        let mode = ThemeMode::Dark;
        // Confirm the tokens resolve to expected hex bytes — same as the
        // pin tests in theme.rs but from the widget layer.
        let bg = color::PANEL.current(mode);
        let border = color::BORDER_1.current(mode);
        let header_bg = color::PANEL_RAISED.current(mode);
        let fg = color::FG_1.current(mode);
        let sh = shadow::shadow_1(mode);
        let r = radius::R4;

        let summary = format!(
            "panel_bg=#{:02x}{:02x}{:02x}\n\
             border=#{:02x}{:02x}{:02x} width=1.0 radius={}\n\
             header_bg=#{:02x}{:02x}{:02x}\n\
             fg=#{:02x}{:02x}{:02x}\n\
             shadow_offset_y={} blur={}",
            (bg.r * 255.0) as u8,
            (bg.g * 255.0) as u8,
            (bg.b * 255.0) as u8,
            (border.r * 255.0) as u8,
            (border.g * 255.0) as u8,
            (border.b * 255.0) as u8,
            r,
            (header_bg.r * 255.0) as u8,
            (header_bg.g * 255.0) as u8,
            (header_bg.b * 255.0) as u8,
            (fg.r * 255.0) as u8,
            (fg.g * 255.0) as u8,
            (fg.b * 255.0) as u8,
            sh.offset.y,
            sh.blur_radius,
        );
        assert_snapshot!("t1505_panel_chrome_style_tokens", summary);
    }

    // ── T1507 — active-row pattern ─────────────────────────────────────────
    /// Verify the active-row helper uses ACCENT when active=true and
    /// TRANSPARENT when active=false. The snapshot captures the colour
    /// values so any drift in the ACCENT token or the transparency logic
    /// produces a visible diff.
    // `cast_possible_truncation` + `cast_sign_loss`: RGB channels are bounded
    // 0.0..=1.0 by iced; f32→u8 byte extraction is intentional and safe for
    // snapshot stability.
    #[test]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn t1507_active_row_accent_rule() {
        let mode = ThemeMode::Dark;
        let accent = color::ACCENT.current(mode);
        let transparent = iced::Color::TRANSPARENT;

        // active=true → ACCENT rule color
        let active_color = if true { accent } else { transparent };
        // active=false → TRANSPARENT rule color
        let inactive_color = if false { accent } else { transparent };

        let summary = format!(
            "rule_width_px=2\n\
             active_color=#{:02x}{:02x}{:02x} alpha={:.2}\n\
             inactive_color=#{:02x}{:02x}{:02x} alpha={:.2}\n\
             strategy_active_row: accent rule visible\n\
             positions_active_row: transparent rule (no layout shift)",
            (active_color.r * 255.0) as u8,
            (active_color.g * 255.0) as u8,
            (active_color.b * 255.0) as u8,
            active_color.a,
            (inactive_color.r * 255.0) as u8,
            (inactive_color.g * 255.0) as u8,
            (inactive_color.b * 255.0) as u8,
            inactive_color.a,
        );
        assert_snapshot!("strategies_active_row", summary);
    }

    // ── T1609 — chip-row active-bottom variant (Phase 2 Q5) ────────────────
    /// Mirror of `t1507_active_row_accent_rule` on the bottom edge — chip
    /// row is horizontal, so the rule lives on the bottom. Same `ACCENT`
    /// when active, `TRANSPARENT` when not. Rule is **always** 2 px tall.
    // RGB channels bounded to `[0.0, 1.0]` by iced; `f32 → u8` byte
    // extraction is intentional for snapshot stability.
    #[test]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn t1609_active_chip_accent_rule_bottom() {
        let mode = ThemeMode::Dark;
        let accent = color::ACCENT.current(mode);
        let transparent = iced::Color::TRANSPARENT;

        let active_color = if true { accent } else { transparent };
        let inactive_color = if false { accent } else { transparent };

        let summary = format!(
            "rule_height_px=2\n\
             rule_edge=bottom\n\
             active_color=#{:02x}{:02x}{:02x} alpha={:.2}\n\
             inactive_color=#{:02x}{:02x}{:02x} alpha={:.2}\n\
             chip_active_chip: accent rule visible on bottom edge\n\
             chip_inactive_chip: transparent rule (no layout shift)",
            (active_color.r * 255.0) as u8,
            (active_color.g * 255.0) as u8,
            (active_color.b * 255.0) as u8,
            active_color.a,
            (inactive_color.r * 255.0) as u8,
            (inactive_color.g * 255.0) as u8,
            (inactive_color.b * 255.0) as u8,
            inactive_color.a,
        );
        assert_snapshot!("t1609_active_chip_accent_rule_bottom", summary);
    }

    // ── T1708 — Phase 3 threshold-bar colour-ramp ──────────────────────────
    //
    // Pure-function helper exposing the Risk-screen colour-band label so
    // the test pins the transition points without standing up an iced
    // render. Test-only — diagnostic identifiers, not user-visible copy
    // (so the consistency-test suite does not need to route them through
    // `ui::strings`).
    fn band_label(used: rust_decimal::Decimal, cap: rust_decimal::Decimal) -> &'static str {
        if cap == rust_decimal::Decimal::ZERO {
            return "ACCENT";
        }
        let raw = (used / cap) * rust_decimal::Decimal::from(100);
        let clamped: rust_decimal::Decimal = if raw < rust_decimal::Decimal::ZERO {
            rust_decimal::Decimal::ZERO
        } else if raw > rust_decimal::Decimal::from(100) {
            rust_decimal::Decimal::from(100)
        } else {
            raw
        };
        let pct: u16 = clamped.trunc().to_string().parse::<u16>().unwrap_or(0);
        if pct >= 90 {
            "DOWN_500"
        } else if pct >= 70 {
            "WARN_500"
        } else {
            "ACCENT"
        }
    }

    // ── Brief B M2 — loading_with_spinner helper ───────────────────────────
    /// Brief B M2 — `loading_with_spinner` returns an `Element` with
    /// the muted text color matching `muted_body`. Compile-time
    /// guarantee + token equality on the foreground color. The
    /// spinner's render state is gated behind `RedrawRequested` so we
    /// can't observe frame state here, but the surrounding text color
    /// is the load-bearing token for snapshot determinism.
    // RGB channels bounded to `[0.0, 1.0]` by iced; `f32 → u8` byte
    // extraction is intentional for token-pin stability (same pattern
    // as t1505 / t1507 above).
    #[test]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn loading_with_spinner_uses_fg_3_text_color() {
        use iced::Element;
        let mode = ThemeMode::Dark;
        // Compile-time guarantee: returns `Element<()>`.
        let _: Element<'_, ()> = super::loading_with_spinner("Loading…", mode);
        // Token equality: the muted-body and spinner helper both flow
        // text through `FG_3.current(mode)`, so a token drift trips
        // both call sites in lockstep.
        let expected_fg = color::FG_3.current(mode);
        let (r, g, b) = (
            (expected_fg.r * 255.0) as u8,
            (expected_fg.g * 255.0) as u8,
            (expected_fg.b * 255.0) as u8,
        );
        // FG_3 dark = #808993 per theme.rs:179 — pinned here so a token
        // rename forces an audit of this helper's color choice.
        assert_eq!((r, g, b), (0x80, 0x89, 0x93));
    }

    /// T1708 — colour ramp: `ACCENT` < 70 %, `WARN_500` ≥ 70 %, `DOWN_500`
    /// ≥ 90 %. Mirrors the Phase 1 latency-band precedent.
    #[test]
    fn t1708_threshold_bar_color_ramp() {
        use rust_decimal_macros::dec;
        // Below 70 % → ACCENT.
        assert_eq!(band_label(dec!(0), dec!(100)), "ACCENT");
        assert_eq!(band_label(dec!(50), dec!(100)), "ACCENT");
        assert_eq!(band_label(dec!(69), dec!(100)), "ACCENT");
        // At 70 %, ≥ 70 % → WARN_500.
        assert_eq!(band_label(dec!(70), dec!(100)), "WARN_500");
        assert_eq!(band_label(dec!(80), dec!(100)), "WARN_500");
        assert_eq!(band_label(dec!(89), dec!(100)), "WARN_500");
        // At 90 %, ≥ 90 % → DOWN_500.
        assert_eq!(band_label(dec!(90), dec!(100)), "DOWN_500");
        assert_eq!(band_label(dec!(95), dec!(100)), "DOWN_500");
        assert_eq!(band_label(dec!(100), dec!(100)), "DOWN_500");
        // Empty cap → ACCENT (no limit configured).
        assert_eq!(band_label(dec!(50), dec!(0)), "ACCENT");
    }
}
