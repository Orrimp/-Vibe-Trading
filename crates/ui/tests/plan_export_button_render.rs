//! advisor-handoff-export P5 (ADR-0088 § D9 / Q-HE-5) — rendered-PIXEL proof of
//! the "Export this plan" button on the SUGGEST / `Screen::ForwardPlan` header.
//!
//! ## Why this file exists (CLAUDE.md non-negotiable)
//!
//! The export button is a NEW visible control on a journey screen, so per
//! CLAUDE.md ("verify at the rendered-PIXEL layer") + MEMORY.md ("verify UI at
//! the render layer") it gets a real headless render — NOT a unit test, a text
//! snapshot, or a no-panic boot. This guard renders the REAL
//! `screens::forward_plan::view` and asserts, at the pixel layer:
//!
//! - [`export_button_paints_in_ready_header`] — a Ready crowned plan, beside the
//!   bake-off that crowned it, paints the `ACCENT`-filled button (a solid teal
//!   block) in the HEADER band. Writes `/tmp/plan_export_button_ready_render.png`.
//! - [`export_button_absent_when_not_ready`] — the Q-HE-6 negative control: the
//!   `Empty` "no plan yet" state paints ~no `ACCENT` in the header band (the
//!   button is ABSENT — no empty-export affordance) and STRICTLY less than the
//!   Ready state. Proves the guard tracks the plan state, not a tautology.
//!   Writes `/tmp/plan_export_button_empty_render.png`.
//! - [`export_button_shows_only_beside_the_bakeoff_that_crowned_the_plan`] — LR-223
//!   (bug-log #223, ADR-0088 § D5 as amended): the button shows exactly when
//!   `screens::forward_plan::crowned_export_pair` is `Some`. On one bake-off, the
//!   crowned plan beside it paints the button; a promoted plan beside the same
//!   bake-off, the crowned plan during a re-run, the same plan beside a newer bake-off on another
//!   coin, and a Ready plan with no bake-off at all (the state this file's positive
//!   frame used before the fix) paint none.
//!
//! ## What the pixels key on
//!
//! The button is `ACCENT`-filled (`color::ACCENT`, the same primary-action idiom
//! as the leaderboard "Run bake-off" button). In dark mode `ACCENT` is
//! `#6FB6AE` — the exact teal the other forward-plan / leaderboard / Reports
//! render guards key on. Within the HEADER band `[0, 92)` the ONLY `ACCENT`
//! source is the button fill: the headline (`FG_1`) + caption (`FG_3`) are not
//! teal, and the LONG stance badge's accent text lives lower (y ≈ 175+). So a
//! large header-band teal count ⇔ the button rendered.
//!
//! ## Dual-theme note
//!
//! The button uses the dual-mode `color::ACCENT` / `color::FG_ON_ACCENT`
//! `ModeColor` tokens — the SAME mechanism every other widget on this
//! already-dual-theme screen uses — so it is correct under `--theme light`
//! (`ACCENT` = `#3F968D`) by construction. This harness renders dark because
//! `test_support::ForwardPlanScreenApp` is dark-canonical (ADR-0057: tiny-skia
//! pixel determinism is macOS + dark).
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like the sibling forward-plan render guards, real-renderer pixel assertions
//! are macOS-canonical (cosmic-text rasterisation is per-OS); this file compiles
//! to nothing on Linux/Windows. Thresholds are coarse (presence/absence of a
//! hue, not byte-exact).

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState};
use ui::test_support::forward_plan_screen_program;

/// Render the bare Forward-plan screen body at the `typical` 1920×1080 slot and
/// return the physical-pixel RGBA buffer + dimensions (dark theme — the harness
/// is dark-canonical, ADR-0057).
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

// ── Header band ────────────────────────────────────────────────────────────────
//
// The header row (headline + caption on the left, the export button on the
// right) is the FIRST element in the padded plan column, above the
// not-a-prediction framing banner (y ≈ 112+). The export button sits inside
// `[0, 92)`; the promoted-plan render guard already fixes 92 as the header /
// provenance boundary (`forward_plan_populated_render.rs::PROVENANCE_TOP`).

/// Bottom of the HEADER band — where the export button lives (above the framing
/// banner). Matches the sibling guard's header/provenance boundary.
const HEADER_BOTTOM: u32 = 92;

/// `true` for an `ACCENT`-teal pixel — green & blue high and close, red clearly
/// lower (the exact predicate the forward-plan / leaderboard / Reports guards use
/// for `ACCENT`). Matches dark `#6FB6AE` AND light `#3F968D` (both are teal by
/// this predicate), so the button fill is counted in either theme.
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

/// The `ACCENT` teal in the HEADER band — the export button fill. Large on a
/// Ready plan (the button rendered); ~zero on the Empty state (no button).
fn header_band_accent(w: u32, rgba: &[u8]) -> u64 {
    accent_teal_in_band(w, rgba, 0, HEADER_BOTTOM)
}

/// **The render-layer guard (positive).** A Ready crowned plan, beside the bake-off
/// that crowned it (bug-log #223), MUST paint the `ACCENT`-filled "Export this plan"
/// button in the HEADER band. Writes the operator-facing PNG to
/// `/tmp/plan_export_button_ready_render.png`.
#[test]
fn export_button_paints_in_ready_header() {
    let cockpit =
        ui::fixtures::fake_cockpit_forward_plan_crowned(ui::fixtures::fake_bakeoff_report_mirror());
    let (w, h, rgba) = render_forward_plan_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/plan_export_button_ready_render.png");
    }

    let header_accent = header_band_accent(w, &rgba);
    assert!(
        header_accent > 800,
        "the ACCENT-filled 'Export this plan' button must paint in the HEADER \
         band on a Ready plan (expected >800 teal px, got {header_accent}). If \
         this fails the export affordance did not render. \
         PNG: /tmp/plan_export_button_ready_render.png"
    );
    // Sanity: the screen otherwise drew (not a blank frame).
    assert!(h > 0 && w > 0, "the screen must have rendered");
}

/// **The Q-HE-6 negative control.** The `Empty` "no plan yet" state paints ~no
/// `ACCENT` in the HEADER band — the button is ABSENT (no empty-export
/// affordance) — and STRICTLY less than the Ready state. Proves the guard tracks
/// the plan state (it is not satisfied by the headline / caption / chrome).
/// Writes `/tmp/plan_export_button_empty_render.png`.
#[test]
fn export_button_absent_when_not_ready() {
    let empty = ui::fixtures::fake_cockpit_forward_plan(PanelState::Empty);
    let ready =
        ui::fixtures::fake_cockpit_forward_plan_crowned(ui::fixtures::fake_bakeoff_report_mirror());

    let (we, he, re) = render_forward_plan_rgba(empty);
    let (wr, _hr, rr) = render_forward_plan_rgba(ready);

    if let Some(img) = image::RgbaImage::from_raw(we, he, re.clone()) {
        let _ = img.save("/tmp/plan_export_button_empty_render.png");
    }

    let empty_accent = header_band_accent(we, &re);
    let ready_accent = header_band_accent(wr, &rr);

    // ABSENT: ~no ACCENT teal in the Empty header (a small floor allows stray AA).
    assert!(
        empty_accent < 150,
        "the Empty state must NOT paint the export button (expected <150 stray \
         teal px in the header, got {empty_accent}). If high, the button is \
         rendering when there is no crowned plan (Q-HE-6 violation). \
         PNG: /tmp/plan_export_button_empty_render.png"
    );
    // DISCRIMINATES: the Ready header paints clearly MORE accent than Empty.
    assert!(
        ready_accent > empty_accent + 600,
        "the Ready header must paint clearly more ACCENT than the Empty header \
         (ready {ready_accent} vs empty {empty_accent}) — proof the button is the \
         live Ready-only affordance, not shared chrome. \
         PNGs: /tmp/plan_export_button_{{ready,empty}}_render.png"
    );
}

// ── bug-log #223 (LR-223, AD-10, ADR-0088 § D5 as amended) ──────────────────────

/// Promote a BTCUSDT SMA grid (`Sma{10,20}`) on top of `cockpit`, through `update`
/// only: "Tune…", Run, the grid lands, "Use this config" (the plan goes `Loading`).
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

/// **LR-223 (bug-log #223, AD-10, ADR-0088 § D5 as amended).** The export button
/// shows exactly beside the bake-off that crowned the plan. Frames on the stock
/// bake-off (BTCUSDT, crowned `v0.sma`):
/// - C: the crowned plan beside it (`fake_cockpit_forward_plan_crowned`);
/// - P: C, then a promotion through `update`, then the tuned plan production lands for
///   it (`v0.5.sma`, the tuned rules, no confidence since bug-log #213: the view
///   `fake_cockpit_forward_plan_promoted` carries), with the stock bake-off still
///   Ready on the leaderboard: #223's title state;
/// - R: C after `BakeoffRunRequested` (a re-run in flight, the leaderboard `Loading`);
/// - X: C after a newer bake-off landed that differs from the stock one only in its
///   coin (`ETHUSDT`);
/// - N: `fake_cockpit_forward_plan(Ready(fake_forward_plan()))`, a Ready plan with
///   no bake-off: #223's title state, and this file's positive frame before the fix.
///
/// Header-band `ACCENT`: C > 800 (the file's floor); each of P, R, X and N < 150
/// (the file's absent floor); C > each + 600.
///
/// Writes `/tmp/plan_export_button_{crowned,promoted,rerun,other_coin,no_bakeoff}_render.png`.
#[test]
fn export_button_shows_only_beside_the_bakeoff_that_crowned_the_plan() {
    use ui::state::{Message, update};
    let mut fails: Vec<String> = Vec::new();

    let stock = ui::fixtures::fake_bakeoff_report_mirror();
    let c = ui::fixtures::fake_cockpit_forward_plan_crowned(stock.clone());
    let PanelState::Ready(view) = c.forward_plan_screen_state.plan.clone() else {
        panic!("precondition: the stock bake-off crowns a plan");
    };
    let same_plan = |cockpit: &Cockpit| matches!(&cockpit.forward_plan_screen_state.plan, PanelState::Ready(v) if *v == view);

    let PanelState::Ready(tuned) = ui::fixtures::fake_cockpit_forward_plan_promoted()
        .forward_plan_screen_state
        .plan
    else {
        panic!("precondition: the promoted fixture carries a Ready tuned plan");
    };
    let mut p = c.clone();
    promote_through_update(&mut p);
    update(&mut p, Message::ForwardPlanReceived(tuned.clone()));
    if !(p.pending_forward_promotion.is_some()
        && tuned.confidence.is_none()
        && matches!(&p.forward_plan_screen_state.plan, PanelState::Ready(v) if *v == tuned)
        && matches!(&p.leaderboard_screen_state.result, PanelState::Ready(m) if *m == stock))
    {
        fails.push(
            "precondition: P is promoted, carries the tuned plan, beside the stock bake-off"
                .to_string(),
        );
    }

    let mut r = c.clone();
    update(&mut r, Message::BakeoffRunRequested);
    if !(matches!(r.leaderboard_screen_state.result, PanelState::Loading) && same_plan(&r)) {
        fails.push("precondition: R's leaderboard is Loading under C's plan view".to_string());
    }

    let mut eth = stock.clone();
    eth.coin = smol_str::SmolStr::new("ETHUSDT");
    let mut x = c.clone();
    update(&mut x, Message::BakeoffRunCompleted(Ok(eth.clone())));
    if !(matches!(&x.leaderboard_screen_state.result, PanelState::Ready(m) if *m == eth)
        && same_plan(&x))
    {
        fails.push("precondition: X carries the ETHUSDT bake-off under C's plan view".to_string());
    }

    let n = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(
        ui::fixtures::fake_forward_plan(),
    ));
    if !matches!(n.leaderboard_screen_state.result, PanelState::Empty) {
        fails.push("precondition: N has no bake-off".to_string());
    }

    let accents = [
        ("crowned", c),
        ("promoted", p),
        ("rerun", r),
        ("other_coin", x),
        ("no_bakeoff", n),
    ]
    .map(|(name, cockpit)| {
        let (w, h, rgba) = render_forward_plan_rgba(cockpit);
        if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
            let _ = img.save(format!("/tmp/plan_export_button_{name}_render.png"));
        }
        (name, header_band_accent(w, &rgba))
    });
    eprintln!("#223 pixel margin LR-223: header-band ACCENT {accents:?}");

    let crowned = accents[0].1;
    if crowned <= 800 {
        fails.push(format!(
            "(C) the crowned plan beside its bake-off must paint the button: {crowned} > 800"
        ));
    }
    for (name, accent) in &accents[1..] {
        if *accent >= 150 || crowned <= accent + 600 {
            fails.push(format!(
                "({name}) #223: no button may paint here: {accent} < 150 and crowned {crowned} \
                 > {accent} + 600"
            ));
        }
    }
    assert!(
        fails.is_empty(),
        "LR-223: {} failure(s):\n  {}\nPNGs: /tmp/plan_export_button_{{crowned,promoted,rerun,\
         other_coin,no_bakeoff}}_render.png",
        fails.len(),
        fails.join("\n  ")
    );
}
