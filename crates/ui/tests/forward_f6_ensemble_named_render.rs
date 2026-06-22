//! advisor-forward-plan F6 member-name enrichment — render-layer proof that the
//! ensemble forward plan NAMES its members ("≥ 2 of {MACD trend, RSI reversion,
//! Bollinger reversion} agree…") in the cockpit Forward-plan screen.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model state is NOT proof
//! the named rule draws. The F6 enrichment carries each ensemble member's display
//! label across the agent→ui boundary so the plan can name them instead of saying
//! "3 member strategies" abstractly. This guard renders the REAL
//! `screens::forward_plan::view` HEADLESS with the populated ensemble
//! `ForwardPlanView` (whose `PlanRuleView::Ensemble` carries the member labels)
//! and asserts on the rendered PIXELS that the member-naming rule actually
//! painted — with a NEGATIVE CONTROL (a single-strategy SMA plan, whose RULES
//! band carries far less text because it names no member set).
//!
//! ## How "member names present" is verified at the pixel layer
//!
//! The named ensemble rule ("Holds while at least 2 of {MACD trend, RSI
//! reversion, Bollinger reversion} agree…") is a LONG line — the brace-list adds
//! the three family names. So the RULES band of the named-ensemble plan paints
//! STRICTLY MORE foreground text than (a) the single-strategy SMA plan (which
//! names no member set) and (b) a healthy absolute floor. Pixels cannot read the
//! glyphs, but the member-name brace-list is the dominant extra text in that
//! band, so a strict-exceedance discriminator is a faithful proxy for "the names
//! rendered". The IF/THEN ACCENT keywords are also asserted (the conditional vote
//! structure drew).
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `forward_plan_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical. The file compiles to nothing on Linux/Windows. Thresholds are
//! coarse (presence/relative magnitude of foreground, not byte-exact).

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState};
use ui::test_support::forward_plan_screen_program;

/// Render the bare Forward-plan screen body at the `typical` 1920×1080 slot.
fn render_forward_plan_rgba(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = forward_plan_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    let screenshot = iced_test::screenshot(&program, &theme, (1920, 1080), 1.0, Duration::ZERO);
    (
        screenshot.size.width,
        screenshot.size.height,
        screenshot.rgba.to_vec(),
    )
}

// ── Region band ────────────────────────────────────────────────────────────────
//
// Same layout as `forward_plan_populated_render.rs`: the "Standing rules" block
// (the headline vote rule + tally + caveat + cadence) lives in the RULES band.
// The member-name brace-list is the dominant extra text there.

/// Top of the RULES band — below the header + framing banner + stance block.
const RULES_TOP: u32 = 300;
/// Bottom of the RULES band — generous, to capture the multi-line named rule +
/// tally + caveat (the named ensemble rule wraps to more lines than a single).
const RULES_BOTTOM: u32 = 520;

/// `true` for an `ACCENT`-teal (#6FB6AE — R111 G182 B174) pixel — the IF/THEN
/// keyword accent (same predicate the other forward-plan guards use).
fn is_accent_teal(r: i32, g: i32, b: i32) -> bool {
    g > 120 && b > 120 && (g - b).abs() < 40 && (g - r) > 25
}

/// Count `ACCENT`-teal pixels in the `[y0, y1)` band.
fn accent_teal_in_band(w: u32, rgba: &[u8], y0: u32, y1: u32) -> u64 {
    let mut hits = 0u64;
    for y in y0..y1 {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if is_accent_teal(r, g, b) {
                hits += 1;
            }
        }
    }
    hits
}

/// Count general foreground (text) pixels in the `[y0, y1)` band.
fn foreground_in_band(w: u32, rgba: &[u8], y0: u32, y1: u32) -> u64 {
    let mut hits = 0u64;
    for y in y0..y1 {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            let luma = (r * 2 + g * 3 + b) / 6;
            if luma > 80 {
                hits += 1;
            }
        }
    }
    hits
}

/// IF/THEN keyword accent in the RULES band.
fn rules_band_accent(w: u32, rgba: &[u8]) -> u64 {
    accent_teal_in_band(w, rgba, RULES_TOP, RULES_BOTTOM)
}

/// Foreground (the named rule + tally + caveat text) in the RULES band.
fn rules_band_foreground(w: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, RULES_TOP, RULES_BOTTOM)
}

/// General foreground across the whole frame.
fn foreground_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, 0, h)
}

/// **The F6 member-naming render guard.** The populated ENSEMBLE plan (the
/// 2-of-3 majority vote over MACD / RSI / Bollinger, currently LONG) MUST paint,
/// in the cockpit Forward-plan screen:
/// - `ACCENT` teal in the RULES band (the vote rule's IF/THEN keywords — the
///   conditional vote structure drew);
/// - a healthy amount of RULES-band foreground (the NAMED rule "{MACD trend, RSI
///   reversion, Bollinger reversion}" + tally + caveat — the member names are the
///   dominant extra text there);
/// - a healthy whole-frame foreground (the full plan drew, not a blank pane).
///
/// Writes the operator-facing PNG to `/tmp/forward_f6_ensemble_named_render.png`.
#[test]
fn forward_f6_ensemble_names_its_members() {
    let view = ui::fixtures::fake_forward_plan_ensemble();
    assert!(view.is_ensemble(), "the fixture must be an ensemble plan");
    // The fixture carries the three member labels (the F6 enrichment).
    if let ui::forward_plan::PlanRuleView::Ensemble { members, .. } = &view.rule {
        assert_eq!(members.len(), 3, "the majority vote names 3 members");
        assert!(
            members.iter().any(|m| m.contains("MACD")),
            "the member labels include MACD (got {members:?})"
        );
    } else {
        panic!("the ensemble fixture must carry PlanRuleView::Ensemble");
    }

    let cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(view));
    let (w, h, rgba) = render_forward_plan_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/forward_f6_ensemble_named_render.png");
    }

    let rules_accent = rules_band_accent(w, &rgba);
    let rules_fg = rules_band_foreground(w, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The vote rule's IF/THEN keywords paint ACCENT teal in the RULES band.
    assert!(
        rules_accent > 60,
        "the ensemble vote rule's IF/THEN keywords must paint ACCENT teal in the \
         RULES band (expected >60 teal px, got {rules_accent}). \
         PNG: /tmp/forward_f6_ensemble_named_render.png"
    );
    // The named rule (with the {MACD trend, RSI reversion, Bollinger reversion}
    // brace-list) + tally + caveat is a lot of text in the RULES band.
    assert!(
        rules_fg > 2500,
        "the NAMED ensemble rule + tally + caveat must paint substantial \
         foreground in the RULES band (expected >2500 px, got {rules_fg}). If low, \
         the member names did not render. \
         PNG: /tmp/forward_f6_ensemble_named_render.png"
    );
    // The full plan is a lot of text.
    assert!(
        fg > 7000,
        "the ensemble plan must paint a lot of foreground text (expected >7000 px, \
         got {fg}). PNG: /tmp/forward_f6_ensemble_named_render.png"
    );
}

/// **Anti-tautology discriminator — the named ensemble names MORE than a single.**
/// The named-ensemble plan paints STRICTLY MORE RULES-band foreground than the
/// single-strategy SMA plan (whose rule names no member set — just one entry +
/// one exit line). The member-name brace-list is the dominant difference, so this
/// strict exceedance is a faithful proxy for "the member names rendered" (the
/// single plan cannot produce that text). Ties the two states together so a
/// regression that drops the names (reverting to the abstract "{n} member
/// strategies" count) would shrink the gap and fail.
#[test]
fn named_ensemble_rules_band_exceeds_single_strategy() {
    let ensemble = ui::fixtures::fake_forward_plan_ensemble();
    let single = ui::fixtures::fake_forward_plan(); // SMA single — names no members

    let ens_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(ensemble));
    let sma_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(single));

    let (we, _he, re) = render_forward_plan_rgba(ens_cockpit);
    let (ws, _hs, rs) = render_forward_plan_rgba(sma_cockpit);

    let fg_ens = rules_band_foreground(we, &re);
    let fg_sma = rules_band_foreground(ws, &rs);
    assert!(
        fg_ens > fg_sma + 800,
        "the NAMED ensemble rule must paint strictly more RULES-band foreground \
         than the single SMA plan (the {{MACD trend, RSI reversion, Bollinger \
         reversion}} member-name list is the extra text) — ensemble {fg_ens} vs \
         SMA {fg_sma}. If the gap is small the member names are not rendering. \
         PNG: /tmp/forward_f6_ensemble_named_render.png"
    );
}
