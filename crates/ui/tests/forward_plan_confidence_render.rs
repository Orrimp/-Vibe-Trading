//! P0-3 "Confidence check, not verdict" — render-layer proof of the CONFIDENCE
//! SUMMARY BLOCK in the cockpit Forward-plan screen (advisor-confidence-not-verdict,
//! ADR-0075).
//!
//! ## Why this file exists
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state,
//! a text `.snap`, or a no-panic boot is NOT proof the confidence block draws.
//! This guard renders the REAL `screens::forward_plan::view` HEADLESS with a
//! POPULATED `ForwardPlanView` carrying a `ConfidenceSummaryView` and asserts on
//! the rendered PIXELS that the four confidence facts + the section header actually
//! paint — with ONE negative control:
//!   - the SAME plan WITHOUT a confidence summary (the block must be ABSENT, i.e.
//!     the plan with confidence paints STRICTLY MORE foreground than without).
//!
//! Three guards:
//! - [`confidence_block_paints_more_foreground_than_without`] — the positive
//!   case: `fake_forward_plan_with_confidence()` paints more total foreground
//!   than `fake_forward_plan()` (same plan family, no confidence). This is the
//!   load-bearing discriminator: the confidence block is EXTRA content not
//!   present on the control.
//! - [`confidence_block_below_horizon_band`] — the block paints foreground in
//!   the LOWER portion of the frame (below the horizon block), meaning the four
//!   fact rows rendered and are not squashed into nothing. The same plan without
//!   confidence paints LESS in that lower band.
//! - `lr213::promoted_plan_draws_no_bakeoff_confidence_block` (bug-log #213,
//!   ADR-0070 D11, cfg `live`): the real promoted chain draws no confidence
//!   block, with HEAD's leaderboard-scorecard rule rebuilt as its negative
//!   control and a threshold derived from the crowned pair (1920×1320).
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `forward_plan_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file compiles
//! to nothing on Linux/Windows. Pixel thresholds are deliberately coarse (presence
//! vs absence of content, not byte-exact), robust within macOS across font-DB jitter.

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

/// Count general foreground (text / marker) pixels in the `[y0, y1)` row band —
/// anything that crosses a luma floor the near-black `CANVAS`/`PANEL`/`PANEL_RAISED`
/// tiers never reach. Monotonic in how much content drew.
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

// The confidence block appears AFTER the horizon block, which ends around y ≈ 600
// at the 1920×1080 / scale-1.0 slot. The lower band (y 600–900) is exclusively
// the confidence-summary block when present, and just the disclaimer footer when
// absent. The block adds four fact rows + a section header — meaningfully more
// foreground than just the footer.
//
// (Exact extents: header + framing ≈ 0–175; stance ≈ 175–300; rules ≈ 300–480;
// sizing ≈ 480–560; horizon ≈ 560–640; confidence ≈ 640–850; disclaimer ≈ 850+)
const CONFIDENCE_BAND_TOP: u32 = 600;
const CONFIDENCE_BAND_BOTTOM: u32 = 900;

/// Foreground in the confidence-block band — the four fact rows + section header.
/// High when the confidence block is present; low when absent (only the disclaimer).
fn confidence_band_foreground(w: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, CONFIDENCE_BAND_TOP, CONFIDENCE_BAND_BOTTOM)
}

/// **Positive case: confidence block renders more foreground than without.**
///
/// A populated `ForwardPlanView` WITH a `ConfidenceSummaryView` (n_candidates=18,
/// deflated_sharpe=0.87, crown_clears_dsr=false, min_btl_years=6.4) MUST paint
/// STRICTLY MORE total foreground than the SAME plan WITHOUT a confidence summary.
/// The delta is the four fact rows + section header of the confidence block.
///
/// Writes operator-facing PNGs to:
/// - `/tmp/forward_plan_confidence_render.png` (with confidence)
/// - `/tmp/forward_plan_no_confidence_render.png` (without — negative control)
#[test]
fn confidence_block_paints_more_foreground_than_without() {
    let with_confidence = ui::fixtures::fake_forward_plan_with_confidence();
    assert!(
        with_confidence.confidence.is_some(),
        "the fixture WITH confidence must carry a populated ConfidenceSummaryView"
    );

    let without_confidence = ui::fixtures::fake_forward_plan();
    assert!(
        without_confidence.confidence.is_none(),
        "the negative-control fixture must NOT carry a confidence summary"
    );

    let cockpit_with = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(with_confidence));
    let cockpit_without =
        ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(without_confidence));

    let (w_with, h_with, rgba_with) = render_forward_plan_rgba(cockpit_with);
    let (w_wo, h_wo, rgba_wo) = render_forward_plan_rgba(cockpit_without);

    // Operator-facing deliverables (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w_with, h_with, rgba_with.clone()) {
        let _ = img.save("/tmp/forward_plan_confidence_render.png");
    }
    if let Some(img) = image::RgbaImage::from_raw(w_wo, h_wo, rgba_wo.clone()) {
        let _ = img.save("/tmp/forward_plan_no_confidence_render.png");
    }

    let fg_with = foreground_pixels(w_with, h_with, &rgba_with);
    let fg_without = foreground_pixels(w_wo, h_wo, &rgba_wo);

    // The plan with confidence must draw MORE total foreground (the block is extra
    // content — a section header + four fact rows that the control does not have).
    assert!(
        fg_with > fg_without + 500,
        "the plan WITH confidence must paint strictly more foreground than the \
         same plan WITHOUT (with {fg_with} vs without {fg_without}, delta must be \
         >500 px). If delta is low the confidence block did not render. \
         PNGs: /tmp/forward_plan_{{confidence,no_confidence}}_render.png"
    );
}

/// **Negative control: the confidence block is in the LOWER band.**
///
/// When the plan carries a confidence summary, the lower band (y 600–900, below
/// the horizon block) must show STRICTLY MORE foreground than the same plan without
/// confidence. The control plan (no confidence) has only the disclaimer footer in
/// that band; the test plan has the full confidence block (header + four facts) above
/// the footer. This is the SPATIAL discriminator: the block is where we expect.
///
/// Writes operator-facing PNGs to the same paths as the first guard (idempotent).
#[test]
fn confidence_block_below_horizon_band() {
    let with_confidence = ui::fixtures::fake_forward_plan_with_confidence();
    let without_confidence = ui::fixtures::fake_forward_plan();

    let cockpit_with = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(with_confidence));
    let cockpit_without =
        ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(without_confidence));

    let (w_with, _h_with, rgba_with) = render_forward_plan_rgba(cockpit_with);
    let (w_wo, _h_wo, rgba_wo) = render_forward_plan_rgba(cockpit_without);

    let conf_fg_with = confidence_band_foreground(w_with, &rgba_with);
    let conf_fg_wo = confidence_band_foreground(w_wo, &rgba_wo);

    // The confidence block (section header + four fact rows) paints far more
    // foreground in the lower band than the disclaimer-only control.
    assert!(
        conf_fg_with > conf_fg_wo + 300,
        "the plan WITH confidence must paint more foreground in the lower band \
         (y {CONFIDENCE_BAND_TOP}–{CONFIDENCE_BAND_BOTTOM}) than the plan without \
         (with {conf_fg_with} vs without {conf_fg_wo}, delta must be >300 px). \
         If delta is low the confidence block is missing from that region. \
         PNGs: /tmp/forward_plan_{{confidence,no_confidence}}_render.png"
    );
}

// ── LR-213 (bug-log #213, AD-10, ADR-0070 § D11) ────────────────────────────

/// **LR-213.** A promoted plan draws no bake-off confidence block, through the
/// real chain: a cockpit driven through `update` (a BTCUSDT bake-off crowned on
/// SMA on the leaderboard, a BTCUSDT SMA grid promoted with `Sma{10,20}`, the
/// case of `forward_launch_config.rs` L1 (a)), then
/// `ui::live::promoted_forward_run_config`, then
/// `agent::build_forward_plan_from_registry`, then `ForwardPlanView::from_plan`,
/// then `ForwardPlanReceived`, then a render at 1920×1320 (`FRAME_HEIGHT`).
///
/// Frames: `fixed` (that chain); `bug` (the same chain with HEAD's rule rebuilt
/// as the negative control: the config carries the on-screen bake-off's
/// scorecard); `crowned` (a crowned chain on the same bake-off, nothing
/// promoted, through `ui::live::crowned_forward_run_config`); `crowned_bare`
/// (that plan with its confidence stripped); `empty` (no plan).
///
/// Every count is the foreground BELOW the header band, `[BODY_TOP, FRAME_HEIGHT)`
/// (bug-log #223): since the fix the export button shows over `crowned` and not over
/// `crowned_bare` (its confidence does not match the bake-off's scorecard), so a
/// whole-frame count would add the button's fill to the block. The header band holds
/// only the headline, the caption and the button; the confidence block is below it.
///
/// 1. `fixed` is populated: fg(fixed) > fg(empty) + 500.
/// 2. fg(bug) − fg(fixed) ≥ ½ × (fg(crowned) − fg(crowned_bare)): the promoted
///    plan lacks what the confidence block draws. The threshold is derived from
///    the crowned pair, not a literal.
/// 3. fg(crowned) > fg(crowned_bare) + 500 (this file's floor for the block).
/// 4. The promoted plan's view carries no confidence. A view check, not a pixel
///    one: (2) is the AD-10 assertion, and it alone would catch a screen that
///    drew a scorecard the view does not carry.
///
/// Writes `/tmp/forward_plan_promoted_{fixed,bug}_confidence_render.png` and
/// `/tmp/forward_plan_crowned{,_bare}_confidence_chain_render.png`. The probe
/// table (P213-1 to P213-16) is in `forward_launch_config.rs`.
#[cfg(feature = "live")]
mod lr213 {
    use super::foreground_in_band;
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;
    use std::time::Duration;
    use trading_core::{Money, Symbol, Usdt};
    use ui::forward_plan::ForwardPlanView;
    use ui::leaderboard::{BakeoffReportMirror, LeaderboardLookback};
    use ui::state::{Cockpit, Message, PanelState, update};
    use ui::test_support::forward_plan_screen_program;
    use ui::tune::{PromoteParams, TuneFamily};

    /// Taller than the file's 1080: the promoted plan's provenance strip pushes
    /// the bug frame's confidence block past y = 1080, and a clipped block would
    /// shrink the margin assertion (2) measures.
    const FRAME_HEIGHT: u32 = 1320;

    /// Top of the plan body: the header band `[0, BODY_TOP)` (headline, caption and,
    /// over a crowned plan, the export button) is left out of every count, so the
    /// subject stays the confidence block (bug-log #223, house rule `#189`). The same
    /// boundary as `forward_plan_populated_render.rs::PROVENANCE_TOP`.
    const BODY_TOP: u32 = 92;

    /// [`super::render_forward_plan_rgba`] at 1920×[`FRAME_HEIGHT`].
    fn render(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
        ui::force_chart_utc_for_tests();
        let program = forward_plan_screen_program(cockpit);
        let shot = iced_test::screenshot(
            &program,
            &iced::Theme::Dark,
            (1920, FRAME_HEIGHT),
            1.0,
            Duration::ZERO,
        );
        (shot.size.width, shot.size.height, shot.rgba.to_vec())
    }

    fn budget() -> Money<Usdt> {
        Money::<Usdt>::from_decimal(dec!(187.42))
    }

    /// The agent's plan for `cfg`, mirrored the way the relay mirrors it.
    fn plan_view(cfg: &agent::ForwardRunConfig) -> ForwardPlanView {
        ui::force_chart_utc_for_tests();
        let price = trading_core::Price::new(dec!(64000)).expect("64000 is a valid price");
        let ts = trading_core::Timestamp::new(
            time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000),
        );
        let plan = agent::build_forward_plan_from_registry(
            &agent::config::Config::default(),
            cfg,
            price,
            ts,
            7,
        )
        .expect("the agent resolves a plan for an SMA config");
        ForwardPlanView::from_plan(&plan)
    }

    /// `cockpit` with `view` landed as the agent's relay lands it.
    fn with_plan(mut cockpit: Cockpit, view: ForwardPlanView) -> Cockpit {
        update(&mut cockpit, Message::ForwardPlanReceived(view));
        cockpit
    }

    /// L1 (a)'s cockpit: the BTCUSDT bake-off on the leaderboard, a BTCUSDT SMA
    /// grid promoted with `Sma{10,20}`, all through `update`.
    fn promoted_cockpit(btc: &BakeoffReportMirror, fails: &mut Vec<String>) -> Cockpit {
        let mut cockpit = Cockpit::new();
        update(&mut cockpit, Message::BakeoffRunCompleted(Ok(btc.clone())));
        update(
            &mut cockpit,
            Message::OpenTuneEditor {
                family: TuneFamily::Sma,
                coin: Symbol::new("BTCUSDT"),
                lookback: LeaderboardLookback::H1_2024,
            },
        );
        update(&mut cockpit, Message::SweepRunRequested);
        let mut grid = ui::fixtures::fake_sweep_report_mirror();
        grid.coin = SmolStr::new("BTCUSDT");
        update(&mut cockpit, Message::SweepRunCompleted(Ok(grid)));
        update(
            &mut cockpit,
            Message::PromoteSweptConfig(PromoteParams::Sma {
                fast_len: 10,
                slow_len: 20,
            }),
        );
        let on_board = matches!(
            &cockpit.leaderboard_screen_state.result,
            PanelState::Ready(m) if m == btc && m.scorecard.is_some()
        );
        let promoted = cockpit
            .pending_forward_promotion
            .as_ref()
            .is_some_and(|p| p.coin.0.as_str() == btc.coin.as_str());
        if !(on_board && promoted) {
            fails.push(
                "precondition: the bake-off (with a scorecard) is on the leaderboard and a \
                 promotion on its coin is pending"
                    .to_string(),
            );
        }
        cockpit
    }

    #[test]
    fn promoted_plan_draws_no_bakeoff_confidence_block() {
        let mut fails = Vec::new();
        let btc = ui::fixtures::fake_bakeoff_report_mirror();

        // fixed / bug: the promoted chain.
        let cockpit = promoted_cockpit(&btc, &mut fails);
        let cfg = ui::live::promoted_forward_run_config(&cockpit, budget())
            .expect("a promotion is pending");
        let fixed_view = plan_view(&cfg);
        let mut bug_cfg = cfg.clone();
        bug_cfg.confidence = btc.scorecard.map(|sc| backtest::bakeoff::ScorecardSummary {
            n_candidates: sc.n_candidates,
            deflated_sharpe: sc.deflated_sharpe,
            crown_clears_dsr: sc.crown_clears_dsr,
            min_btl_years: sc.min_btl_years,
        });
        let bug_view = plan_view(&bug_cfg);
        let fixed_view_confidence = fixed_view.confidence;

        // crowned / crowned_bare: the crowned chain on the same bake-off.
        let mut crowned_cockpit = Cockpit::new();
        update(
            &mut crowned_cockpit,
            Message::BakeoffRunCompleted(Ok(btc.clone())),
        );
        let PanelState::Ready(on_screen) = &crowned_cockpit.leaderboard_screen_state.result else {
            panic!("precondition: the bake-off is on the leaderboard");
        };
        let crowned_cfg = ui::live::crowned_forward_run_config(on_screen, budget())
            .expect("the bake-off crowned a row");
        // The binary sends the launch's budget with every crowned launch, before its
        // plan lands (bug-log #223: the export gate's budget clause reads it).
        update(
            &mut crowned_cockpit,
            Message::ForwardPaperTradeStarted(budget(), None),
        );
        let crowned_view = plan_view(&crowned_cfg);
        let mut crowned_bare_view = crowned_view.clone();
        crowned_bare_view.confidence = None;

        let frames = [
            ("fixed", with_plan(cockpit.clone(), fixed_view)),
            ("bug", with_plan(cockpit, bug_view)),
            ("crowned", with_plan(crowned_cockpit.clone(), crowned_view)),
            (
                "crowned_bare",
                with_plan(crowned_cockpit, crowned_bare_view),
            ),
            (
                "empty",
                ui::fixtures::fake_cockpit_forward_plan(PanelState::Empty),
            ),
        ]
        .map(|(name, c)| {
            let (w, h, rgba) = render(c);
            let path = match name {
                "fixed" | "bug" => Some(format!(
                    "/tmp/forward_plan_promoted_{name}_confidence_render.png"
                )),
                "crowned" | "crowned_bare" => Some(format!(
                    "/tmp/forward_plan_{name}_confidence_chain_render.png"
                )),
                _ => None,
            };
            if let Some(path) = path
                && let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone())
            {
                let _ = img.save(path);
            }
            (name, foreground_in_band(w, &rgba, BODY_TOP, h))
        });
        let fg = |n: &str| -> i64 {
            let f = frames.iter().find(|f| f.0 == n).expect("frame exists").1;
            i64::try_from(f).expect("a pixel count fits in i64")
        };

        let block = fg("crowned") - fg("crowned_bare");
        let missing = fg("bug") - fg("fixed");
        eprintln!(
            "#213 pixel margin LR-213: fg fixed {} bug {} crowned {} crowned_bare {} \
             empty {}; block (crowned - bare) {block}; missing (bug - fixed) {missing}",
            fg("fixed"),
            fg("bug"),
            fg("crowned"),
            fg("crowned_bare"),
            fg("empty")
        );

        if fg("fixed") <= fg("empty") + 500 {
            fails.push(format!(
                "(1) the promoted plan must draw: fg(fixed) {} > fg(empty) {} + 500",
                fg("fixed"),
                fg("empty")
            ));
        }
        if 2 * missing < block {
            fails.push(format!(
                "(2) #213: the promoted plan must lack the confidence block HEAD's rule drew: \
                 fg(bug) - fg(fixed) = {missing} must be >= 1/2 x (fg(crowned) - \
                 fg(crowned_bare)) = {block} / 2"
            ));
        }
        if block <= 500 {
            fails.push(format!(
                "(3) the crowned plan must draw its bake-off's confidence block: fg(crowned) - \
                 fg(crowned_bare) = {block} must be > 500"
            ));
        }
        if let Some(c) = &fixed_view_confidence {
            fails.push(format!(
                "(4) #213: the promoted plan's view carries no confidence, got {c:?}"
            ));
        }
        assert!(
            fails.is_empty(),
            "LR-213: {} failure(s):\n  {}\nPNGs: /tmp/forward_plan_promoted_{{fixed,bug}}_confidence_render.png, \
             /tmp/forward_plan_crowned{{,_bare}}_confidence_chain_render.png",
            fails.len(),
            fails.join("\n  ")
        );
    }
}
