//! advisor-forward-plan v0.1.0 (roadmap F6) — render-layer proof of the
//! POPULATED forward buy/sell PLAN in the cockpit.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, or a no-panic boot is NOT proof the plan draws. That trap
//! shipped multiple blind cockpit bugs (the Live-view saga, the trail 0-px
//! side-drawer, the Reports empty-curve). This guard renders the REAL
//! `screens::forward_plan::view` HEADLESS with a POPULATED `ForwardPlanView`
//! and asserts on the rendered PIXELS that the stance badge + the IF/THEN rule
//! text + the €200 projected-sizing number + the not-a-prediction framing
//! actually paint — with TWO negative controls:
//!   - the BUY-AND-HOLD degenerate plan (the same KIND of object, but visibly
//!     DIFFERENT — no IF/THEN accent in the rules band), proving the populated
//!     guard is not a tautology;
//!   - the EMPTY "no plan yet" state (paints ~no plan — a single prompt line).
//!
//! Three guards (populated active + buy-and-hold negative control + empty,
//! per CLAUDE.md + ADR-0062 § D8.1 / T-U3.1):
//!
//! - [`forward_plan_populated_paints_stance_rules_and_sizing`] — the active
//!   SMA fixture paints (a) `ACCENT` teal in the RULES band (the IF/THEN
//!   keywords), and (b) a healthy amount of foreground text (the stance,
//!   rules, sizing, horizon blocks plus the framing banner and the
//!   disclaimers actually drew, not a blank pane). Writes the operator-facing
//!   PNG to `/tmp/forward_plan_populated_render.png`.
//! - [`forward_plan_buy_and_hold_is_the_negative_control`] — the buy-and-hold
//!   degenerate fixture paints the plan (a lot of foreground — it IS the same
//!   kind of object) but paints STRICTLY LESS `ACCENT` teal in the RULES band
//!   (no IF/THEN keywords — there is no sell trigger / re-evaluation), proving
//!   the populated guard is not a tautology. Writes
//!   `/tmp/forward_plan_buy_and_hold_render.png`.
//! - [`forward_plan_empty_paints_no_plan`] — the second negative control: the
//!   `Empty` "run a bake-off first" prompt paints far less foreground than the
//!   populated plan and ~no `ACCENT` teal anywhere. Writes
//!   `/tmp/forward_plan_empty_render.png`.
//!
//! ## bug-log #200 (LR-200b)
//!
//! [`forward_plan_promoted_strip_names_the_landed_grids_coin`] builds every
//! frame through `update`: a grid swept on one coin, "Tune…" on another, "Use
//! this config", the tuned plan. The promote-provenance strip names the landed
//! grid's coin: an ETH grid re-opened on SOL and a SOL grid re-opened on ETH
//! differ only inside the strip's band, and an ETH grid re-opened on SOL paints
//! the same pixels as one re-opened on ETH. Writes
//! `/tmp/forward_plan_200_*_render.png`.
//!
//! ## bug-log #222 (LR-222)
//!
//! [`promoted_header_is_not_the_crowned_header_on_one_bakeoff`] renders a crowned
//! plan beside its bake-off and the SAME plan view after a promotion (only the
//! provenance differs), and asserts inside the header text column that the
//! headline and the caption each differ, that the header depends on provenance only
//! (a second crowned and a second promoted cockpit draw the same column), that the
//! promoted header is no taller, and that it shows from the `Loading` frame on.
//! Writes `/tmp/forward_plan_222_*_render.png`.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file
//! compiles to nothing on Linux/Windows. Pixel thresholds are deliberately
//! coarse (presence/absence of a hue, not byte-exact), robust within macOS
//! across font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState};
use ui::test_support::forward_plan_screen_program;

/// Render the bare Forward-plan screen body at the `typical` 1920×1080 slot and
/// return the physical-pixel RGBA buffer + dimensions.
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

// ── Region bands ──────────────────────────────────────────────────────────────
//
// The plan body stacks, top-to-bottom, at the 1920×1080 / scale-1.0 slot:
//
//   y   0– 90  header: page headline + caption.
//   y 100–175  the not-a-prediction FRAMING banner (warn-bordered card).
//   y 175–300  the "Right now" stance block (badge + as-of + latest signal).
//   y 300–460  the "Standing rules" block (IF/THEN lines + cadence) — the
//              IF/THEN keywords paint ACCENT teal HERE (active plans only;
//              the buy-and-hold plan paints NO accent in this band).
//   y 460+     the sizing block + horizon block + disclaimer.
//
// The RULES band is the load-bearing discriminator: an active plan paints the
// IF/THEN keyword accent there; the buy-and-hold plan does not. Because the
// exact y-extents shift a little with content, the RULES band is scoped
// generously and the whole-screen foreground carries the populated-vs-empty
// discrimination.

/// Top of the RULES band — where the IF/THEN keyword accent lives on an active
/// plan. Below the header + framing banner + stance block.
const RULES_TOP: u32 = 300;
/// Bottom of the RULES band — above the sizing block.
const RULES_BOTTOM: u32 = 470;

/// `true` for an `ACCENT`-teal (#6FB6AE — R111 G182 B174) pixel — green & blue
/// high and close, red clearly lower (the exact predicate the leaderboard +
/// Reports curve guards use for #6FB6AE).
fn is_accent_teal(r: i32, g: i32, b: i32) -> bool {
    g > 120 && b > 120 && (g - b).abs() < 40 && (g - r) > 25
}

/// Count `ACCENT`-teal pixels in the `[y0, y1)` row band.
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

/// The IF/THEN keyword `ACCENT` teal in the RULES band. On an active plan this
/// is non-trivial (the IF/THEN keywords on the entry + exit lines); on the
/// buy-and-hold plan it is ~zero (no IF/THEN — there is no sell trigger).
fn rules_band_accent(w: u32, rgba: &[u8]) -> u64 {
    accent_teal_in_band(w, rgba, RULES_TOP, RULES_BOTTOM)
}

/// `ACCENT` teal across the whole frame — used by the empty-state guard (the
/// empty prompt has no plan, so ~no accent anywhere).
fn whole_screen_accent(w: u32, h: u32, rgba: &[u8]) -> u64 {
    accent_teal_in_band(w, rgba, 0, h)
}

/// Count general foreground (text / marker) pixels in the `[y0, y1)` band —
/// anything that crosses a luma floor the near-black `CANVAS`/`PANEL`/
/// `PANEL_RAISED` tiers never reach. Monotonic in how much content drew.
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

/// General foreground across the whole frame.
fn foreground_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, 0, h)
}

/// **The render-layer guard.** A populated active-strategy `ForwardPlanView`
/// MUST paint, in the cockpit Forward-plan screen:
/// - `ACCENT` teal in the RULES band (the IF/THEN keywords — the conditional
///   framing actually drew);
/// - a healthy amount of foreground text (the framing banner + the four blocks
///   + the disclaimers drew, not a blank pane).
///
/// Writes the operator-facing PNG to `/tmp/forward_plan_populated_render.png`.
#[test]
fn forward_plan_populated_paints_stance_rules_and_sizing() {
    let view = ui::fixtures::fake_forward_plan();
    assert!(
        !view.is_buy_and_hold(),
        "the populated fixture must be an ACTIVE plan (IF/THEN rules), not buy-and-hold"
    );

    let cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(view));
    let (w, h, rgba) = render_forward_plan_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/forward_plan_populated_render.png");
    }

    let rules_accent = rules_band_accent(w, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The IF/THEN keywords (entry + exit lines) paint ACCENT teal in the RULES
    // band — proof the conditional rule structure actually rendered.
    assert!(
        rules_accent > 60,
        "the IF/THEN keyword ACCENT must paint in the RULES band (expected >60 \
         teal px, got {rules_accent}). If this fails the standing-rules block \
         did not render the conditional structure. \
         PNG: /tmp/forward_plan_populated_render.png"
    );
    // The full plan (banner + stance + rules + sizing + horizon + disclaimers)
    // is a lot of text.
    assert!(
        fg > 7000,
        "the populated plan must paint a lot of foreground text (expected >7000 \
         px, got {fg}). If this is low the screen rendered a blank/empty pane \
         despite Ready data. PNG: /tmp/forward_plan_populated_render.png"
    );
}

/// **Negative control #1 — the buy-and-hold degenerate plan.** The SAME harness
/// with `fake_forward_plan_buy_and_hold()` paints the plan (a lot of foreground
/// — it IS the same KIND of object) but paints STRICTLY LESS `ACCENT` teal in
/// the RULES band than the active plan (no IF/THEN keywords — there is no sell
/// trigger / re-evaluation). Proves the populated guard genuinely discriminates
/// the conditional active plan from the degenerate one (it is not satisfied by
/// chrome / the framing banner / the disclaimer).
#[test]
fn forward_plan_buy_and_hold_is_the_negative_control() {
    let bh = ui::fixtures::fake_forward_plan_buy_and_hold();
    assert!(
        bh.is_buy_and_hold(),
        "the negative-control fixture is buy-and-hold"
    );
    let active = ui::fixtures::fake_forward_plan();

    let bh_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(bh));
    let active_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(active));

    let (wb, hb, rb) = render_forward_plan_rgba(bh_cockpit);
    let (wa, _ha, ra) = render_forward_plan_rgba(active_cockpit);

    if let Some(img) = image::RgbaImage::from_raw(wb, hb, rb.clone()) {
        let _ = img.save("/tmp/forward_plan_buy_and_hold_render.png");
    }

    // It IS the same kind of object — it paints a substantial plan.
    let fg_bh = foreground_pixels(wb, hb, &rb);
    assert!(
        fg_bh > 4000,
        "the buy-and-hold plan must still paint a substantial plan (expected \
         >4000 foreground px, got {fg_bh}) — it is the same KIND of object. \
         PNG: /tmp/forward_plan_buy_and_hold_render.png"
    );

    // But it has NO IF/THEN keywords — strictly less RULES-band accent than the
    // active plan. This is the anti-tautology discriminator.
    let rules_accent_bh = rules_band_accent(wb, &rb);
    let rules_accent_active = rules_band_accent(wa, &ra);
    assert!(
        rules_accent_active > rules_accent_bh + 40,
        "the active plan must paint strictly more IF/THEN accent in the RULES \
         band than the buy-and-hold plan (active {rules_accent_active} vs \
         buy-and-hold {rules_accent_bh}). If they are equal the populated guard \
         is a tautology (the IF/THEN structure is not discriminating). \
         PNG: /tmp/forward_plan_buy_and_hold_render.png"
    );
}

/// **Negative control #2 — the empty / no-pick state.** The `Empty` "run a
/// bake-off first" prompt is a single muted line, NO plan. So it paints far
/// less foreground than the populated plan and ~no `ACCENT` teal anywhere.
/// Proves the populated guard genuinely discriminates (it is not satisfied by
/// the screen header / chrome).
#[test]
fn forward_plan_empty_paints_no_plan() {
    let empty_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Empty);
    let populated_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(
        ui::fixtures::fake_forward_plan(),
    ));

    let (we, he, re) = render_forward_plan_rgba(empty_cockpit);
    let (wp, hp, rp) = render_forward_plan_rgba(populated_cockpit);

    if let Some(img) = image::RgbaImage::from_raw(we, he, re.clone()) {
        let _ = img.save("/tmp/forward_plan_empty_render.png");
    }

    // ~No ACCENT teal anywhere — the empty prompt has no plan (no stance badge,
    // no IF/THEN keywords). A small floor allows stray AA.
    let empty_accent = whole_screen_accent(we, he, &re);
    assert!(
        empty_accent < 120,
        "the Empty state must NOT paint plan ACCENT (expected <120 stray teal \
         px, got {empty_accent}). If high, the populated guard is a tautology. \
         PNG: /tmp/forward_plan_empty_render.png"
    );

    // Strictly less foreground than the populated plan (the prompt is one line;
    // the plan is six stacked blocks).
    let fg_empty = foreground_pixels(we, he, &re);
    let fg_pop = foreground_pixels(wp, hp, &rp);
    assert!(
        fg_pop > fg_empty + 4000,
        "the populated plan must paint far more foreground than the empty \
         prompt (populated {fg_pop} vs empty {fg_empty}). \
         PNG: /tmp/forward_plan_empty_render.png"
    );
}

// ── advisor-param-promotion (ADR-0070 § D6/§ T12 Proof 2) — promoted plan ──────
//
// When the operator clicks "Use this config" on a PROMOTABLE Tune row, the
// forward plan is preseeded from THAT tuned config + `pending_forward_promotion`
// flips, so the plan leads with the "you tuned this … not a guarantee, not
// advice" provenance strip (a WARN_500-bordered card just below the headline) —
// distinct from the crowned "best of the bake-off" framing. The plan body shows
// the TUNED rules (fast 10 / slow 20), not the shipped 20/50.
//
// The provenance strip sits in the HEADER band (below the H1 headline + caption,
// above the not-a-prediction framing banner). The negative control is the SAME
// active plan WITHOUT promotion (no strip): it paints strictly LESS foreground in
// that header band.

/// Top of the PROVENANCE-STRIP band — below the H1 headline + caption.
const PROVENANCE_TOP: u32 = 92;
/// Bottom of the PROVENANCE-STRIP band — above the not-a-prediction framing
/// banner (the promoted plan adds ~one extra card here; the non-promoted plan
/// has only the framing banner, which starts lower).
const PROVENANCE_BOTTOM: u32 = 175;

/// Foreground in the provenance-strip band — the "you tuned this …" copy. High on
/// a promoted plan (the strip rendered); low on a non-promoted plan (no strip,
/// just whitespace between caption and the framing banner).
fn provenance_band_foreground(w: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, PROVENANCE_TOP, PROVENANCE_BOTTOM)
}

/// **Proof 2 — the promoted forward plan.** A `Cockpit` preseeded from a PROMOTION
/// (a `pending_forward_promotion` with TUNED SMA
/// params, fast 10 / slow 20) MUST paint, in the cockpit Forward-plan screen:
/// - the "you tuned this … not a guarantee, not advice" PROVENANCE strip in the
///   header band (the only live promote-vs-crown signal);
/// - the TUNED IF/THEN rules in the rules band (ACCENT keywords — the conditional
///   structure of the tuned config rendered);
/// - a healthy amount of foreground overall.
///
/// Writes `/tmp/forward_plan_promoted_render.png`.
#[test]
fn forward_plan_promoted_paints_provenance_and_tuned_rules() {
    let cockpit = ui::fixtures::fake_cockpit_forward_plan_promoted();
    assert!(
        cockpit.pending_forward_promotion.is_some(),
        "the promoted fixture must carry a pending_forward_promotion (the provenance source)"
    );

    let (w, h, rgba) = render_forward_plan_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/forward_plan_promoted_render.png");
    }

    let provenance_fg = provenance_band_foreground(w, &rgba);
    let rules_accent = rules_band_accent(w, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The "you tuned this …" provenance strip rendered in the header band.
    assert!(
        provenance_fg > 600,
        "the promote-provenance strip ('You tuned this … not a guarantee, not \
         advice') must paint in the header band (expected >600 fg px, got \
         {provenance_fg}). If low, the provenance header did not render. \
         PNG: /tmp/forward_plan_promoted_render.png"
    );
    // The TUNED rules rendered (IF/THEN accent — the conditional structure).
    assert!(
        rules_accent > 60,
        "the tuned plan's IF/THEN keywords must paint ACCENT teal in the RULES \
         band (expected >60 teal px, got {rules_accent}). \
         PNG: /tmp/forward_plan_promoted_render.png"
    );
    // The full promoted plan (strip + framing + four blocks + disclaimers).
    assert!(
        fg > 7000,
        "the promoted plan must paint a lot of foreground text (expected >7000 \
         px, got {fg}). PNG: /tmp/forward_plan_promoted_render.png"
    );
}

/// **Proof 2 negative control — no provenance strip without promotion.** The SAME
/// active SMA plan WITHOUT a `pending_forward_promotion` (the crowned-pick path)
/// paints STRICTLY LESS foreground in the provenance-strip band — there is no
/// "you tuned this" header, just whitespace between the caption and the framing
/// banner. Proves the promoted guard is not a tautology (the strip is the live
/// promote-vs-crown signal, not chrome).
#[test]
fn forward_plan_crowned_has_no_provenance_strip() {
    // Promoted: the strip is present.
    let promoted = ui::fixtures::fake_cockpit_forward_plan_promoted();
    // Crowned (no promotion): the same active SMA plan, no pending_forward_promotion.
    let crowned = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(
        ui::fixtures::fake_forward_plan(),
    ));
    assert!(
        crowned.pending_forward_promotion.is_none(),
        "the crowned-pick control must NOT carry a pending_forward_promotion"
    );

    let (wp, _hp, rp) = render_forward_plan_rgba(promoted);
    let (wc, hc, rc) = render_forward_plan_rgba(crowned);

    if let Some(img) = image::RgbaImage::from_raw(wc, hc, rc.clone()) {
        let _ = img.save("/tmp/forward_plan_crowned_control_render.png");
    }

    let promoted_strip = provenance_band_foreground(wp, &rp);
    let crowned_strip = provenance_band_foreground(wc, &rc);

    assert!(
        promoted_strip > crowned_strip + 300,
        "the promoted plan must paint clearly MORE foreground in the provenance \
         band than the crowned-pick plan (promoted {promoted_strip} vs crowned \
         {crowned_strip}) — proof the 'you tuned this' header is the live \
         promote-vs-crown signal, not shared chrome. \
         PNGs: /tmp/forward_plan_{{promoted,crowned_control}}_render.png"
    );
}

/// **RSI-reversion render guard.** Verifies that the FAITHFUL RSI rule copy
/// (flip-to-false exit at RSI > 30, NO overbought-70 claim, plus the
/// compound-condition caveat) actually renders in the cockpit Forward-plan
/// screen:
///
/// - `ACCENT` teal in the RULES band (IF/THEN keywords on entry + exit lines);
/// - a healthy foreground count (all blocks drew).
/// - Strictly less RULES-band accent than the SMA active fixture (same proof
///   structure as the BuyAndHold negative control, just to avoid tautology with
///   a different strategy family than the main populated guard).
///
/// Writes the operator-facing PNG to `/tmp/forward_plan_rsi_render.png`.
#[test]
fn forward_plan_rsi_reversion_paints_faithful_rules() {
    let rsi_view = ui::fixtures::fake_forward_plan_rsi();
    assert!(
        !rsi_view.is_buy_and_hold(),
        "the RSI fixture must be an active plan (IF/THEN rules)"
    );

    let cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(rsi_view));
    let (w, h, rgba) = render_forward_plan_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/forward_plan_rsi_render.png");
    }

    let rules_accent = rules_band_accent(w, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The IF/THEN keywords (RSI entry + flip-to-false exit) must paint ACCENT
    // teal in the RULES band — proof the faithful non-SMA copy rendered.
    assert!(
        rules_accent > 60,
        "the RSI plan IF/THEN keywords must paint ACCENT teal in the RULES band \
         (expected >60 teal px, got {rules_accent}). If this fails the RSI rule \
         copy did not render the conditional structure. \
         PNG: /tmp/forward_plan_rsi_render.png"
    );
    // The full plan must be a lot of text (all six blocks + framing).
    assert!(
        fg > 7000,
        "the RSI plan must paint a lot of foreground text (expected >7000 px, \
         got {fg}). If this is low the screen rendered a blank/empty pane. \
         PNG: /tmp/forward_plan_rsi_render.png"
    );
}

// ── F8 ensemble forward-plan render guard (advisor-ensemble, ADR-0063) ────────
//
// When the crowned pick is an ensemble (signal vote), the forward plan must
// describe the VOTE faithfully — the vote-method headline rule (an IF/THEN
// line, so it paints the conditional ACCENT like the singles) + the live tally
// + the honest caveat — NOT a fabricated single-indicator rule. The buy-and-hold
// degenerate plan is the negative control (same harness, no IF/THEN accent).

/// **The F8 ensemble-plan render guard.** A populated ENSEMBLE `ForwardPlanView`
/// (the `v0.8.vote.majority` 2-of-3 vote, currently LONG) MUST paint, in the
/// cockpit Forward-plan screen:
/// - `ACCENT` teal in the RULES band (the vote rule's IF/THEN keywords — the
///   conditional structure of the vote rendered, NOT a single-indicator rule);
/// - a healthy amount of foreground text (the framing + the stance + the vote
///   rule + tally + caveat + sizing + horizon + disclaimers drew).
///
/// Writes the operator-facing PNG to `/tmp/forward_f8_ensemble_plan_render.png`.
#[test]
fn forward_plan_f8_ensemble_paints_vote_rule_and_tally() {
    let view = ui::fixtures::fake_forward_plan_ensemble();
    assert!(view.is_ensemble(), "the fixture must be an ensemble plan");
    assert!(
        !view.is_buy_and_hold(),
        "an ensemble is not the buy-and-hold degenerate plan"
    );

    let cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(view));
    let (w, h, rgba) = render_forward_plan_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/forward_f8_ensemble_plan_render.png");
    }

    let rules_accent = rules_band_accent(w, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The vote rule's IF/THEN keywords paint ACCENT teal in the RULES band —
    // proof the ensemble's conditional vote structure rendered (faithful, not a
    // fabricated single rule).
    assert!(
        rules_accent > 60,
        "the ensemble vote rule's IF/THEN keywords must paint ACCENT teal in the \
         RULES band (expected >60 teal px, got {rules_accent}). If this fails the \
         ensemble plan did not render the conditional vote structure. \
         PNG: /tmp/forward_f8_ensemble_plan_render.png"
    );
    // The full ensemble plan (framing + stance + vote rule + tally + caveat +
    // sizing + horizon + disclaimers) is a lot of text.
    assert!(
        fg > 7000,
        "the ensemble plan must paint a lot of foreground text (expected >7000 \
         px, got {fg}). If this is low the screen rendered a blank/empty pane \
         despite Ready data. PNG: /tmp/forward_f8_ensemble_plan_render.png"
    );
}

/// **F8 negative control — the buy-and-hold degenerate plan.** The SAME harness
/// with `fake_forward_plan_buy_and_hold()` paints a substantial plan (it IS the
/// same KIND of object) but STRICTLY LESS `ACCENT` teal in the RULES band than
/// the ensemble plan (no IF/THEN vote rule — buy-and-hold has no consensus / no
/// re-evaluation). Proves the ensemble guard genuinely discriminates the
/// conditional vote plan from the degenerate one (it is not satisfied by chrome
/// / the framing banner / the disclaimer). Writes the buy-and-hold negative
/// control PNG to `/tmp/forward_f8_ensemble_plan_buyhold_control.png`.
#[test]
fn forward_plan_f8_buy_and_hold_is_the_negative_control() {
    let bh = ui::fixtures::fake_forward_plan_buy_and_hold();
    assert!(
        bh.is_buy_and_hold(),
        "the negative-control fixture is buy-and-hold"
    );
    let ensemble = ui::fixtures::fake_forward_plan_ensemble();

    let bh_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(bh));
    let ens_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(ensemble));

    let (wb, hb, rb) = render_forward_plan_rgba(bh_cockpit);
    let (we, _he, re) = render_forward_plan_rgba(ens_cockpit);

    if let Some(img) = image::RgbaImage::from_raw(wb, hb, rb.clone()) {
        let _ = img.save("/tmp/forward_f8_ensemble_plan_buyhold_control.png");
    }

    // It IS the same kind of object — it paints a substantial plan.
    let fg_bh = foreground_pixels(wb, hb, &rb);
    assert!(
        fg_bh > 4000,
        "the buy-and-hold plan must still paint a substantial plan (expected \
         >4000 foreground px, got {fg_bh}) — it is the same KIND of object. \
         PNG: /tmp/forward_f8_ensemble_plan_buyhold_control.png"
    );

    // But it has NO IF/THEN vote rule — strictly less RULES-band accent than the
    // ensemble plan. This is the anti-tautology discriminator.
    let rules_accent_bh = rules_band_accent(wb, &rb);
    let rules_accent_ens = rules_band_accent(we, &re);
    assert!(
        rules_accent_ens > rules_accent_bh + 40,
        "the ensemble plan must paint strictly more IF/THEN accent in the RULES \
         band than the buy-and-hold plan (ensemble {rules_accent_ens} vs \
         buy-and-hold {rules_accent_bh}). If they are equal the ensemble guard \
         is a tautology (the vote rule is not discriminating). \
         PNG: /tmp/forward_f8_ensemble_plan_buyhold_control.png"
    );
}

// ── bug-log #200 (LR-200b) — the strip names the landed grid's coin ─────────

/// From `Cockpit::new()`, through `ui::state::update` only: the Lab's "Tune…" on
/// `swept`, Run, the sweep lands `fake_sweep_report_mirror()` swept on `swept`,
/// "Tune…" on `reopen` (the grid stays), "Use this config" (SMA fast 10 /
/// slow 20), and the tuned plan arrives.
fn promoted_after_reopen(swept: &str, reopen: &str) -> Cockpit {
    use ui::state::{Message, update};
    let open = |cockpit: &mut Cockpit, coin: &str| {
        update(
            cockpit,
            Message::OpenTuneEditor {
                family: ui::tune::TuneFamily::Sma,
                coin: trading_core::Symbol::new(coin),
                lookback: ui::leaderboard::LeaderboardLookback::H1_2024,
            },
        );
    };
    let mut grid = ui::fixtures::fake_sweep_report_mirror();
    grid.coin = smol_str::SmolStr::new(swept);

    let mut cockpit = Cockpit::new();
    open(&mut cockpit, swept);
    update(&mut cockpit, Message::SweepRunRequested);
    assert!(
        cockpit.tune_screen_state.running,
        "precondition: Run started the {swept} sweep"
    );
    update(&mut cockpit, Message::SweepRunCompleted(Ok(grid.clone())));
    open(&mut cockpit, reopen);
    assert_eq!(
        cockpit.tune_coin.0.as_str(),
        reopen,
        "precondition: the second Tune… moved the Tune coin"
    );
    assert!(
        matches!(&cockpit.tune_screen_state.result, PanelState::Ready(m) if *m == grid),
        "precondition (K1): the {swept} grid is Ready on screen after Tune… on {reopen}"
    );
    update(
        &mut cockpit,
        Message::PromoteSweptConfig(ui::tune::PromoteParams::Sma {
            fast_len: 10,
            slow_len: 20,
        }),
    );
    assert!(
        cockpit.pending_forward_promotion.is_some(),
        "precondition: Use this config stored a promotion"
    );
    let plan = ui::fixtures::fake_forward_plan();
    update(&mut cockpit, Message::ForwardPlanReceived(plan.clone()));
    assert!(
        matches!(&cockpit.forward_plan_screen_state.plan, PanelState::Ready(v) if *v == plan),
        "precondition: the tuned plan is on screen"
    );
    cockpit
}

/// **LR-200b (bug-log #200, AD-10, ADR-0070 § D6).** The promote-provenance
/// strip names the landed grid's coin. Three promotions, each after "Tune…" was
/// re-opened: P_eth (an ETH grid, re-opened on SOL), P_sol (a SOL grid,
/// re-opened on ETH), P_eth2 (an ETH grid, re-opened on ETH).
///
/// 1. P_eth paints the strip (the file's own > 600 floor);
/// 2. P_eth and P_sol differ, and every differing row is inside the strip's band
///    `[PROVENANCE_TOP, PROVENANCE_BOTTOM)`: the coin is the only difference;
/// 3. P_eth and P_eth2 are byte-identical: the re-open coin is not shown.
///
/// Writes `/tmp/forward_plan_200_{eth_reopened_sol,sol_reopened_eth,eth}_render.png`.
#[test]
fn forward_plan_promoted_strip_names_the_landed_grids_coin() {
    let mut frames = Vec::new();
    for (name, swept, reopen) in [
        ("eth_reopened_sol", "ETHUSDT", "SOLUSDT"),
        ("sol_reopened_eth", "SOLUSDT", "ETHUSDT"),
        ("eth", "ETHUSDT", "ETHUSDT"),
    ] {
        let (w, h, rgba) = render_forward_plan_rgba(promoted_after_reopen(swept, reopen));
        if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
            let _ = img.save(format!("/tmp/forward_plan_200_{name}_render.png"));
        }
        frames.push((w, h, rgba));
    }
    let (w, h) = (frames[0].0, frames[0].1);
    assert!(
        frames.iter().all(|(fw, fh, _)| (*fw, *fh) == (w, h)),
        "every frame renders at the same size"
    );
    let [p_eth, p_sol, p_eth2] = [0, 1, 2].map(|i| frames[i].2.as_slice());

    // 1. The strip paints.
    let strip = provenance_band_foreground(w, p_eth);
    assert!(
        strip > 600,
        "LR-200b (1): the promote-provenance strip must paint (expected >600 fg px, got \
         {strip}). PNG: /tmp/forward_plan_200_eth_reopened_sol_render.png"
    );

    // 2. The two grid coins differ, only inside the strip's band.
    let row = w as usize * 4;
    let differing: Vec<u32> = p_eth
        .chunks(row)
        .zip(p_sol.chunks(row))
        .enumerate()
        .filter(|(_, (ra, rb))| ra != rb)
        .map(|(y, _)| u32::try_from(y).expect("a row index fits in u32"))
        .collect();
    eprintln!(
        "#200 pixel margin LR-200b: strip fg {strip}; ETH vs SOL grid: {} rows differ ({:?}..={:?})",
        differing.len(),
        differing.first(),
        differing.last()
    );
    assert!(
        !differing.is_empty()
            && differing
                .iter()
                .all(|y| (PROVENANCE_TOP..PROVENANCE_BOTTOM).contains(y)),
        "LR-200b (2) #200: the strip must name the landed grid's coin, so promotions of an ETH \
         and a SOL grid differ, and only inside [{PROVENANCE_TOP}, {PROVENANCE_BOTTOM}); got \
         {} rows ({:?}..={:?}). PNGs: /tmp/forward_plan_200_{{eth_reopened_sol,sol_reopened_eth}}_render.png",
        differing.len(),
        differing.first(),
        differing.last()
    );

    // 3. The re-open coin is not what the strip names.
    assert!(
        p_eth == p_eth2,
        "LR-200b (3) #200: an ETH grid re-opened on SOL must paint the same strip as one \
         re-opened on ETH. PNGs: /tmp/forward_plan_200_{{eth_reopened_sol,eth}}_render.png"
    );
}

// ── bug-log #222 (LR-222, AD-10, ADR-0070 § D6 as amended) — the header names the provenance ──

/// Right edge of the header TEXT column: the 16-px column padding plus the fixed
/// 720-px caption (`screens::forward_plan::header_text`), with a margin. The export
/// button is right-aligned near x ≈ 1760, outside it, so the column sees the
/// headline and the caption only.
const HEADER_TEXT_RIGHT: u32 = 760;

/// `true` for a `WARN_500`-amber pixel (dark `#E0B45C`): the predicate
/// `forward_plan_short_render.rs` uses. Both the D6 strip and the framing banner
/// carry a 1-px `WARN_500` border.
fn is_warn_amber(r: i32, g: i32, b: i32) -> bool {
    r > 170 && g > 140 && b < 140 && (r - g).abs() < 60 && (g - b) > 40
}

/// Foreground (the file's luma > 80) pixels of row `y` inside the header text column.
fn column_row_fg(w: u32, rgba: &[u8], y: u32) -> u64 {
    let mut hits = 0u64;
    for x in 0..HEADER_TEXT_RIGHT.min(w) {
        let idx = ((y as usize * w as usize) + x as usize) * 4;
        let (r, g, b) = (
            i32::from(rgba[idx]),
            i32::from(rgba[idx + 1]),
            i32::from(rgba[idx + 2]),
        );
        if (r * 2 + g * 3 + b) / 6 > 80 {
            hits += 1;
        }
    }
    hits
}

/// The RGBA bytes of `[y0, y1)` × `[0, HEADER_TEXT_RIGHT)`.
fn column_bytes(w: u32, rgba: &[u8], y0: u32, y1: u32) -> Vec<u8> {
    let right = HEADER_TEXT_RIGHT.min(w) as usize * 4;
    (y0..y1)
        .flat_map(|y| {
            let start = y as usize * w as usize * 4;
            rgba[start..start + right].iter().copied()
        })
        .collect()
}

/// The first row in `[from, to)` whose column foreground is non-zero (`ink`) or zero.
fn first_row(w: u32, rgba: &[u8], from: u32, to: u32, ink: bool) -> Option<u32> {
    (from..to).find(|&y| (column_row_fg(w, rgba, y) > 0) == ink)
}

/// The first row of the whole frame with at least 500 `WARN_500` pixels: the top
/// border of the first warn card under the header (the D6 strip on a promoted plan,
/// the framing banner on a crowned one).
fn first_amber_row(w: u32, h: u32, rgba: &[u8]) -> Option<u32> {
    (0..h).find(|&y| {
        (0..w)
            .filter(|&x| {
                let idx = ((y as usize * w as usize) + x as usize) * 4;
                is_warn_amber(
                    i32::from(rgba[idx]),
                    i32::from(rgba[idx + 1]),
                    i32::from(rgba[idx + 2]),
                )
            })
            .count()
            >= 500
    })
}

/// Promote a BTCUSDT SMA grid (`Sma{10,20}`) on top of `cockpit`, through `update`
/// only: "Tune…", Run, the grid lands, "Use this config". Leaves the plan `Loading`
/// (the first frame after the press).
fn promote_through_update(cockpit: &mut Cockpit) {
    use ui::state::{Message, update};
    update(
        cockpit,
        Message::OpenTuneEditor {
            family: ui::tune::TuneFamily::Sma,
            coin: trading_core::Symbol::new("BTCUSDT"),
            lookback: ui::leaderboard::LeaderboardLookback::H1_2024,
        },
    );
    update(cockpit, Message::SweepRunRequested);
    let mut grid = ui::fixtures::fake_sweep_report_mirror();
    grid.coin = smol_str::SmolStr::new("BTCUSDT");
    update(cockpit, Message::SweepRunCompleted(Ok(grid)));
    update(
        cockpit,
        Message::PromoteSweptConfig(ui::tune::PromoteParams::Sma {
            fast_len: 10,
            slow_len: 20,
        }),
    );
}

/// **LR-222 (bug-log #222, AD-10, ADR-0070 § D6 as amended).** The forward plan's
/// headline and caption follow the plan's provenance. Frames, all on the stock
/// bake-off (BTCUSDT, crowned `v0.sma`):
/// - C: the crowned plan beside the bake-off that crowned it
///   (`fake_cockpit_forward_plan_crowned`);
/// - P: C, then a promotion through `update`, then the tuned plan landed: the shape
///   production lands for a promotion (`v0.5.sma`, the tuned 10/20 rules, no
///   confidence since bug-log #213; the same view `fake_cockpit_forward_plan_promoted`
///   carries), with the stock bake-off still on the leaderboard. A promotion beside
///   a Ready bake-off is the state #222 and #223 were filed against. (The
///   identical-view #161 pair, a promotion over the crowned view, is pinned at the
///   predicate level instead: U1 (b) and `forward_plan_provenance.rs` E1 (d). It is
///   reachable only through the late-plan race filed in bug-log #244.)
/// - P_load: P before the tuned view lands (plan `Loading`, the first frame after
///   "Use this config");
/// - P2: `fake_cockpit_forward_plan_promoted()` (the same tuned view, no bake-off);
/// - C2: the crowned cockpit on the five-arm bake-off.
///
/// Inside the header text column (`[0, HEADER_TEXT_RIGHT)` × `[0, PROVENANCE_TOP)`),
/// with sub-bands derived from C only (`h0` C's first ink row, `split` C's first
/// blank row after it, `c0` C's first ink row after that, the caption's top):
/// 1. fg(C) > 0 and 2·fg(P) ≥ fg(C);
/// 2. (a) C and P differ in the headline band `[h0, split)`, (b) in the caption band
///    `[c0, PROVENANCE_TOP)`, and (c) P's headline is one ink run ending above `c0`
///    (no P ink from its end down to `c0`) and P's ink in the caption band spans
///    exactly C's caption rows (`c0` to C's last caption row), so (b) is the
///    caption's difference, not a headline that grew into the caption band;
/// 3. P == P2 and C == C2, byte for byte: the header depends on provenance only;
/// 4. P's first ≥ 500-amber row is at or above C's: the promoted header is no
///    taller (the D6 strip and the framing banner both start at header bottom + L);
/// 5. P_load == P: the header is right from the first frame.
///
/// Writes `/tmp/forward_plan_222_{crowned,promoted,promoted_loading,promoted_no_bakeoff,crowned_five_arm}_render.png`.
#[test]
fn promoted_header_is_not_the_crowned_header_on_one_bakeoff() {
    use ui::state::{Message, update};
    let mut fails: Vec<String> = Vec::new();

    let c =
        ui::fixtures::fake_cockpit_forward_plan_crowned(ui::fixtures::fake_bakeoff_report_mirror());
    let p2 = ui::fixtures::fake_cockpit_forward_plan_promoted();
    let PanelState::Ready(tuned_view) = p2.forward_plan_screen_state.plan.clone() else {
        panic!("precondition: the promoted fixture carries a Ready tuned plan");
    };
    if !(tuned_view.confidence.is_none() && tuned_view.strategy.as_str() == "v0.5.sma") {
        fails.push(
            "precondition: the tuned view is production-shaped (v0.5.sma, no confidence)"
                .to_string(),
        );
    }
    let mut p_load = c.clone();
    promote_through_update(&mut p_load);
    if !(p_load.pending_forward_promotion.is_some()
        && matches!(p_load.forward_plan_screen_state.plan, PanelState::Loading))
    {
        fails.push("precondition: the promotion is pending and the plan is Loading".to_string());
    }
    let mut p = p_load.clone();
    update(&mut p, Message::ForwardPlanReceived(tuned_view.clone()));
    if !(matches!(&p.forward_plan_screen_state.plan, PanelState::Ready(v) if *v == tuned_view)
        && matches!(
            (&p.leaderboard_screen_state.result, &c.leaderboard_screen_state.result),
            (PanelState::Ready(a), PanelState::Ready(b)) if a == b
        ))
    {
        fails.push(
            "precondition: P carries the tuned plan beside C's bake-off, promoted".to_string(),
        );
    }
    let c2 = ui::fixtures::fake_cockpit_forward_plan_crowned(
        ui::fixtures::fake_bakeoff_report_mirror_five_arm(),
    );

    let frames = [
        ("crowned", c),
        ("promoted", p),
        ("promoted_loading", p_load),
        ("promoted_no_bakeoff", p2),
        ("crowned_five_arm", c2),
    ]
    .map(|(name, cockpit)| {
        let (w, h, rgba) = render_forward_plan_rgba(cockpit);
        if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
            let _ = img.save(format!("/tmp/forward_plan_222_{name}_render.png"));
        }
        (w, h, rgba)
    });
    let (w, h) = (frames[0].0, frames[0].1);
    assert!(
        frames.iter().all(|(fw, fh, _)| (*fw, *fh) == (w, h)),
        "every frame renders at the same size"
    );
    let [c, p, p_load, p2, c2] = [0, 1, 2, 3, 4].map(|i| frames[i].2.as_slice());

    // Sub-bands, derived from C alone.
    let h0 = first_row(w, c, 0, PROVENANCE_TOP, true).expect("C's header draws ink");
    let split = first_row(w, c, h0, PROVENANCE_TOP, false).expect("C's headline ends");
    let c0 = first_row(w, c, split, PROVENANCE_TOP, true).expect("C's caption draws ink");
    let p_h0 = first_row(w, p, 0, PROVENANCE_TOP, true);
    let p_split = p_h0.and_then(|y| first_row(w, p, y, PROVENANCE_TOP, false));
    let caption_last = (c0..PROVENANCE_TOP)
        .rev()
        .find(|&y| column_row_fg(w, c, y) > 0);
    let caption_lines = (c0..PROVENANCE_TOP)
        .filter(|&y| column_row_fg(w, c, y) > 0 && (y == c0 || column_row_fg(w, c, y - 1) == 0))
        .count();
    let p_caption_last = (c0..PROVENANCE_TOP)
        .rev()
        .find(|&y| column_row_fg(w, p, y) > 0);
    let p_caption_lines = (c0..PROVENANCE_TOP)
        .filter(|&y| column_row_fg(w, p, y) > 0 && (y == c0 || column_row_fg(w, p, y - 1) == 0))
        .count();
    let amber_c = first_amber_row(w, h, c);
    let amber_p = first_amber_row(w, h, p);
    let fg = |rgba: &[u8]| -> u64 { (0..PROVENANCE_TOP).map(|y| column_row_fg(w, rgba, y)).sum() };
    let (fg_c, fg_p) = (fg(c), fg(p));
    eprintln!(
        "#222 pixel margin LR-222: column fg C {fg_c} P {fg_p}; C h0 {h0} split {split} c0 {c0} \
         caption last ink row {caption_last:?} ink runs {caption_lines}; P h0 {p_h0:?} \
         headline end {p_split:?} caption last ink row {p_caption_last:?} ink runs \
         {p_caption_lines}; first >=500-amber row C {amber_c:?} P {amber_p:?}"
    );

    // 1.
    if !(fg_c > 0 && 2 * fg_p >= fg_c) {
        fails.push(format!(
            "(1) fg(C) {fg_c} > 0 and 2 x fg(P) {fg_p} >= fg(C)"
        ));
    }
    // 2.
    if column_bytes(w, c, h0, split) == column_bytes(w, p, h0, split) {
        fails.push(format!(
            "(2a) #222: the promoted headline must differ from the crowned one in [{h0}, {split})"
        ));
    }
    if column_bytes(w, c, c0, PROVENANCE_TOP) == column_bytes(w, p, c0, PROVENANCE_TOP) {
        fails.push(format!(
            "(2b) #222: the promoted caption must differ from the crowned one in [{c0}, \
             {PROVENANCE_TOP})"
        ));
    }
    let p_gap_ink: u64 = p_split.map_or(u64::MAX, |end| {
        (end..c0.max(end)).map(|y| column_row_fg(w, p, y)).sum()
    });
    let p_caption_first = first_row(w, p, c0, PROVENANCE_TOP, true);
    if p_split.is_none_or(|y| y > c0)
        || p_gap_ink > 0
        || p_caption_first != Some(c0)
        || p_caption_last != caption_last
    {
        fails.push(format!(
            "(2c) P's headline must be one ink run above the caption and P's caption must span \
             C's caption rows: headline end {p_split:?}, ink between it and c0 {c0}: {p_gap_ink} \
             px, P's caption rows {p_caption_first:?}..={p_caption_last:?}, C's \
             {c0}..={caption_last:?}"
        ));
    }
    // 3.
    if column_bytes(w, p, 0, PROVENANCE_TOP) != column_bytes(w, p2, 0, PROVENANCE_TOP) {
        fails.push(
            "(3) P and P2 (another promoted plan, no bake-off) must draw the same header"
                .to_string(),
        );
    }
    if column_bytes(w, c, 0, PROVENANCE_TOP) != column_bytes(w, c2, 0, PROVENANCE_TOP) {
        fails.push(
            "(3) C and C2 (another crowned plan, another bake-off) must draw the same header"
                .to_string(),
        );
    }
    // 4.
    match (amber_p, amber_c) {
        (Some(yp), Some(yc)) if yp <= yc => {}
        (yp, yc) => fails.push(format!(
            "(4) the promoted header must be no taller than the crowned one: P's first \
             >=500-amber row {yp:?} must be <= C's {yc:?}"
        )),
    }
    // 5.
    if column_bytes(w, p_load, 0, PROVENANCE_TOP) != column_bytes(w, p, 0, PROVENANCE_TOP) {
        fails.push(
            "(5) the header over the Loading plan (P_load) must equal P's: the provenance \
             shows from the first frame"
                .to_string(),
        );
    }
    assert!(
        fails.is_empty(),
        "LR-222: {} failure(s):\n  {}\nPNGs: /tmp/forward_plan_222_{{crowned,promoted,\
         promoted_loading,promoted_no_bakeoff,crowned_five_arm}}_render.png",
        fails.len(),
        fails.join("\n  ")
    );
}
