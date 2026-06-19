//! trail-side-drawer-render — render-layer proof that the Trail screen's
//! side-drawer ACTUALLY PAINTS when the operator opens it.
//!
//! ## Why this file exists
//!
//! The shipped Phase D trail drawer (`widgets::trail_drawer::view`) sized BOTH
//! its content column and outer `Container` to `Length::Fixed(RIGHT_RAIL_WIDTH_PX)`.
//! That token migrated to `0.0` (the CLOSED-state right-rail width) when the
//! Phase F Assistant slot landed — but the drawer is ONLY ever built in the
//! OPEN state (`screens::trail::view` only constructs it inside
//! `if let Some(node_kind) = drawer_selected`). Result: a 0-px-wide,
//! invisible drawer. Every existing proxy stayed green:
//!   - `trail_node` unit tests render the node stack, not the drawer.
//!   - the `trail__side_drawer_open` visual baseline captured the INVISIBLE
//!     drawer as "correct" (a self-fulfilling 0-px snapshot).
//!   - `update`-arm tests assert `drawer_selected_node = Some(_)` — state, not pixels.
//!
//! This guard renders the REAL `screens::trail::view` HEADLESS with the drawer
//! open on a POPULATED `reconstructed_trail`, and asserts on the rendered
//! PIXELS that a wide `PANEL_RAISED` drawer paints in the far-right region.
//! The negative control (drawer closed) must leave that region ~empty, so the
//! assertion is not a tautology.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `render_snapshots.rs` / `reports_populated_curve_render.rs`, real-
//! renderer pixel assertions are macOS-canonical (cosmic-text font
//! rasterisation is per-OS). The file compiles to nothing on Linux/Windows.
//! Thresholds are coarse (presence/absence of the drawer fill), robust within
//! macOS across font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::Duration;

use smol_str::SmolStr;
use ui::state::{Cockpit, ReconstructedTrailUi, Screen, TrailScreenState, TrailStageUi};
use ui::test_support::program_from_cockpit;
use ui::widgets::trail_node::{TrailNode, TrailNodeKind};

const W: u32 = 1920;
const H: u32 = 1080;

/// Build a Trail-screen cockpit in TRAIL mode on a populated
/// `reconstructed_trail`. `drawer` selects which node's drawer (if any) is
/// open. `with_raw` controls whether each stage carries a `raw_payload` (the
/// per-kind drawer body) so we can prove the body renders real content rather
/// than the LLM "(no transcript)" placeholder.
fn trail_cockpit(drawer: Option<TrailNodeKind>, with_raw: bool) -> Cockpit {
    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Trail;

    let raw = |s: &str| -> Option<String> { with_raw.then(|| s.to_string()) };

    let forecast = TrailStageUi {
        timestamp: Some("12:34:54.001".to_string()),
        actor: Some("tcn:abc12345".to_string()),
        headline: Some("Bullish p=0.72 horizon=15m".to_string()),
        raw_payload: raw(
            "{\n  \"forecast_id\": \"fc001\",\n  \"direction\": \"up\",\n  \
             \"confidence\": 0.72,\n  \"model_revision\": \"tcn-d1c3696d\",\n  \
             \"cache_hit\": false\n}",
        ),
    };
    let signal = TrailStageUi {
        timestamp: Some("12:34:55.123".to_string()),
        actor: Some("strategy:sma_crossover".to_string()),
        headline: Some("Buy signal triggered (SMA crossover)".to_string()),
        raw_payload: raw("{\n  \"signal_id\": \"sig001\",\n  \"side\": \"buy\",\n  \
             \"intended_qty\": 0.05,\n  \"intended_price\": 42000.0,\n  \
             \"was_clamped\": false\n}"),
    };
    let fill = TrailStageUi {
        timestamp: Some("12:34:56.789".to_string()),
        actor: Some("strategy:sma_crossover".to_string()),
        headline: Some("Buy 0.05 BTCUSDT @ 42000.00".to_string()),
        raw_payload: raw("{\n  \"fill_id\": \"abc123\",\n  \"qty\": 0.05,\n  \
             \"price\": 42000.0,\n  \"venue\": \"binance\"\n}"),
    };
    let debate = TrailStageUi::default();

    let nodes = vec![
        TrailNode {
            kind: TrailNodeKind::Forecast,
            timestamp: forecast.timestamp.clone(),
            actor: forecast.actor.clone(),
            headline: forecast.headline.clone(),
        },
        TrailNode {
            kind: TrailNodeKind::LlmDebate,
            timestamp: None,
            actor: None,
            headline: None,
        },
        TrailNode {
            kind: TrailNodeKind::Signal,
            timestamp: signal.timestamp.clone(),
            actor: signal.actor.clone(),
            headline: signal.headline.clone(),
        },
        TrailNode {
            kind: TrailNodeKind::Fill,
            timestamp: fill.timestamp.clone(),
            actor: fill.actor.clone(),
            headline: fill.headline.clone(),
        },
    ];

    let trail = ReconstructedTrailUi {
        audit_id: SmolStr::new("fixture-audit-id-001"),
        fill,
        signal,
        forecast,
        debate,
        nodes,
    };

    cockpit.trail_screen_state = TrailScreenState {
        selected_audit_id: Some(SmolStr::new("fixture-audit-id-001")),
        drawer_selected_node: drawer,
        reconstructed_trail: Some(trail),
        pending_trail_audit_id: None,
    };
    cockpit
}

/// Render the full cockpit shell (routing through the REAL
/// `screens::trail::view`) at the typical 1920×1080 slot and return the
/// physical-pixel RGBA buffer.
fn render_rgba(cockpit: Cockpit) -> Vec<u8> {
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    let screenshot = iced_test::screenshot(&program, &theme, (W, H), 1.0, Duration::ZERO);
    screenshot.rgba.to_vec()
}

fn save_png(rgba: &[u8], name: &str) {
    let path = format!("/tmp/ui-audit/trail-drawer/{name}.png");
    let _ = std::fs::create_dir_all("/tmp/ui-audit/trail-drawer");
    if let Some(img) = image::RgbaImage::from_raw(W, H, rgba.to_vec()) {
        let _ = img.save(&path);
    }
}

/// Count `PANEL_RAISED`-fill pixels in the LOWER-right region of the frame
/// (x ∈ [0.74·W, W), y ∈ [0.45·H, 0.95·H)). The 320-px drawer paints a solid
/// `PANEL_RAISED` (#2A3038, dark) panel there when OPEN — it is `height: Fill`,
/// so it extends FAR below the four node rows + status bar (which all end by
/// ~0.36·H). When CLOSED, this lower-right window is the shell's empty content
/// background (darker `CANVAS`). Restricting to the lower band is what isolates
/// the drawer from the full-width node stack (the node rows are `PANEL_RAISED`
/// too, but live only in the TOP band). We match the drawer fill hue with a
/// small tolerance to stay robust to anti-aliased edges + cosmic-text jitter.
fn drawer_fill_pixels(rgba: &[u8]) -> u64 {
    // PANEL_RAISED dark = rgb(0x2A, 0x30, 0x38) = (42, 48, 56).
    let (tr, tg, tb) = (42i32, 48i32, 56i32);
    let x0 = (W as f32 * 0.74) as u32;
    let y0 = (H as f32 * 0.45) as u32;
    let y1 = (H as f32 * 0.95) as u32;
    let mut hits = 0u64;
    for y in y0..y1 {
        for x in x0..W {
            let idx = ((y as usize * W as usize) + x as usize) * 4;
            let r = i32::from(rgba[idx]);
            let g = i32::from(rgba[idx + 1]);
            let b = i32::from(rgba[idx + 2]);
            if (r - tr).abs() <= 6 && (g - tg).abs() <= 6 && (b - tb).abs() <= 6 {
                hits += 1;
            }
        }
    }
    hits
}

/// **The render-layer guard (positive).** With the drawer OPEN on a populated
/// trail, the far-right region must paint a wide `PANEL_RAISED` drawer —
/// tens of thousands of fill px (320 px wide × hundreds of px tall).
#[test]
fn trail_drawer_open_paints_right_rail() {
    for kind in [
        TrailNodeKind::Forecast,
        TrailNodeKind::Signal,
        TrailNodeKind::Fill,
    ] {
        let cockpit = trail_cockpit(Some(kind), true);
        let rgba = render_rgba(cockpit);
        save_png(&rgba, &format!("open_{kind:?}").to_lowercase());
        let hits = drawer_fill_pixels(&rgba);
        assert!(
            hits > 20_000,
            "drawer OPEN to {kind:?} must paint a wide PANEL_RAISED drawer in the \
             far-right region (expected >20000 fill px, got {hits}). \
             PNG: /tmp/ui-audit/trail-drawer/open_{}.png",
            format!("{kind:?}").to_lowercase()
        );
    }
}

/// **The negative control.** With the drawer CLOSED, the far-right region is
/// the shell background — far fewer `PANEL_RAISED` fill px than the open state.
/// Proves the positive assertion genuinely discriminates the drawer fill.
#[test]
fn trail_drawer_closed_leaves_right_rail_empty() {
    let cockpit = trail_cockpit(None, true);
    let rgba = render_rgba(cockpit);
    save_png(&rgba, "closed");
    let hits = drawer_fill_pixels(&rgba);
    assert!(
        hits < 5_000,
        "drawer CLOSED must leave the far-right region ~empty of PANEL_RAISED \
         fill (expected <5000 px, got {hits}). PNG: /tmp/ui-audit/trail-drawer/closed.png"
    );
}

/// Separation sanity: the OPEN drawer fill must dwarf the CLOSED background —
/// a single assertion that locks the discriminating gap in one place.
#[test]
fn trail_drawer_open_dwarfs_closed() {
    let open = drawer_fill_pixels(&render_rgba(trail_cockpit(
        Some(TrailNodeKind::Forecast),
        true,
    )));
    let closed = drawer_fill_pixels(&render_rgba(trail_cockpit(None, true)));
    assert!(
        open > closed * 5,
        "open drawer fill ({open}) must dwarf closed background ({closed}) by >5×"
    );
}
