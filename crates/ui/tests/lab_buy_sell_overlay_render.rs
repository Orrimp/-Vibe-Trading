//! Render-layer proof (project law `feedback_verify_ui_at_render_layer` +
//! `spec/dev-notes/iced-ui-render-verification.md`): the Lab screen's per-bar
//! buy/sell **volume bars** line up vertically UNDER the price chart's buy/sell
//! **triangle markers** for the SAME fills.
//!
//! ## The bug (lab-buy-sell-overlay-align, 2026-06-24)
//!
//! The operator: *"the Buy and Sell overlays are not over the triangles but the
//! overlay is in the bottom of the view."* The "overlay at the bottom" is the
//! `volume_histogram` strip (green buy bars up, clay sell bars down). It painted
//! into `canvas_chart::inner_rect` (symmetric `space::S` gutter, left origin 8)
//! while the chart paints into `chart_inner_rect` (a 48-px LEFT price-axis gutter
//! → left origin 56, plus a right gutter). So bar `i`'s triangle and bar `i`'s
//! volume bar landed at DIFFERENT x — confirmed at the pixel layer: the leftmost
//! fill was ~27 px off in a 1280-px render. The fix gives the histogram the
//! chart's HORIZONTAL plot geometry + the chart's `x_for_index` per-bar mapping
//! (`volume_histogram::view_aligned`), so each volume bar sits under its
//! triangle.
//!
//! ## What this proves
//!
//! Rasterizes the REAL full Lab body (`shell::view` → `lab::view` → the real
//! `chart` + `volume_histogram` widgets) via `iced_test::screenshot`, then
//! measures the x-centroid of each buy/sell triangle (chart band) and each
//! buy/sell volume bar (histogram band) and asserts they align within a tight
//! tolerance — a RED-before / GREEN-after guard. NEGATIVE control: a long-only
//! (buys-only) seed paints buy bars but no sell bars in the histogram band.

#![cfg(target_os = "macos")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::Duration;

use trading_core::{Side, Symbol, Venue};
use ui::state::{Cockpit, PanelState};
use ui::test_support::{charts_screen_cockpit, program_from_cockpit};
use ui::theme::{ThemeMode, color};

const VIEW_W: u32 = 1280;
const VIEW_H: u32 = 980;
const SCALE: f32 = 1.0;

// Vertical bands (from the 1280×980 render of the full Lab body): the price
// chart's marker zone and the per-bar-volume strip. Generous so font/AA jitter
// in either widget never drops the band.
const CHART_BAND_Y: (u32, u32) = (545, 650);
const HISTO_BAND_Y: (u32, u32) = (815, 890);
/// The legend ("Buy"/"Sell" glyphs) lives at the chart's top-right; exclude it
/// from the chart-band clusters so it isn't mistaken for a triangle.
const LEGEND_X_CUTOFF: u32 = 1080;

fn rgb_of(c: iced::Color) -> (i32, i32, i32) {
    (
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    )
}
fn close(a: (i32, i32, i32), b: (i32, i32, i32), tol: i32) -> bool {
    (a.0 - b.0).abs() <= tol && (a.1 - b.1).abs() <= tol && (a.2 - b.2).abs() <= tol
}

/// x-centroids of horizontally-separated clusters of `target` color within the
/// row band `[y0,y1)`, columns split into clusters by gaps > 12 px.
fn cluster_xs(
    rgba: &[u8],
    w: u32,
    (y0, y1): (u32, u32),
    target: (i32, i32, i32),
    x_cutoff: u32,
) -> Vec<u32> {
    let tol = 10;
    let mut cols: Vec<u32> = Vec::new();
    for x in 0..w.min(x_cutoff) {
        let mut hit = false;
        for y in y0..y1 {
            let idx = ((y * w + x) * 4) as usize;
            let p = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if close(p, target, tol) {
                hit = true;
                break;
            }
        }
        if hit {
            cols.push(x);
        }
    }
    let mut clusters: Vec<u32> = Vec::new();
    let mut run: Vec<u32> = Vec::new();
    for &x in &cols {
        if run.last().is_some_and(|&last| x - last > 12) {
            clusters.push(run.iter().sum::<u32>() / run.len() as u32);
            run.clear();
        }
        run.push(x);
    }
    if !run.is_empty() {
        clusters.push(run.iter().sum::<u32>() / run.len() as u32);
    }
    clusters
}

fn render(cockpit: Cockpit) -> (u32, Vec<u8>) {
    ui::widgets::chart::force_chart_utc_for_tests();
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    let shot = iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO);
    (shot.size.width, shot.rgba.to_vec())
}

/// Max |triangle_x − histo_x| over the paired buy/sell clusters. ≤ this many
/// logical px counts as "the volume bar sits under its triangle". Before the
/// fix the leftmost fill was ~27 px off; after, all residuals are < 16 px
/// (only the sub-bar-width timestamp-vs-index difference remains).
const ALIGN_TOL_PX: i32 = 16;

#[test]
fn buy_sell_volume_bars_align_under_triangles() {
    // POSITIVE: the standard populated Lab scene (60 bars, 4 alternating
    // buy/sell fills for BTCUSDT) — the exact state the operator sees.
    let (w, rgba) = render(charts_screen_cockpit());

    if std::env::var_os("UI_DUMP_PNG").is_some()
        && let Some(img) = image::RgbaImage::from_raw(w, VIEW_H, rgba.clone())
    {
        let _ = img.save("/tmp/lab_buy_sell_overlay_aligned.png");
    }

    let up = rgb_of(color::UP_500.current(ThemeMode::Dark));
    let down = rgb_of(color::DOWN_500.current(ThemeMode::Dark));

    let tri_buy = cluster_xs(&rgba, w, CHART_BAND_Y, up, LEGEND_X_CUTOFF);
    let tri_sell = cluster_xs(&rgba, w, CHART_BAND_Y, down, LEGEND_X_CUTOFF);
    let his_buy = cluster_xs(&rgba, w, HISTO_BAND_Y, up, LEGEND_X_CUTOFF);
    let his_sell = cluster_xs(&rgba, w, HISTO_BAND_Y, down, LEGEND_X_CUTOFF);

    // Sanity: both widgets actually painted the fills (the proof is meaningless
    // if either is blank — the Reports-curve trap). Two buys + one sell are the
    // alternating-fill fixture's minimum visible markers.
    assert!(
        tri_buy.len() > 1 && !tri_sell.is_empty(),
        "chart must paint buy+sell triangles (buy={tri_buy:?}, sell={tri_sell:?})"
    );
    assert!(
        his_buy.len() > 1 && !his_sell.is_empty(),
        "histogram must paint buy+sell volume bars (buy={his_buy:?}, sell={his_sell:?})"
    );

    // Each histogram bar sits under its triangle (paired left-to-right).
    for (label, tri, his) in [("BUY", &tri_buy, &his_buy), ("SELL", &tri_sell, &his_sell)] {
        for (i, (&tx, &hx)) in tri.iter().zip(his.iter()).enumerate() {
            let delta = (tx as i32 - hx as i32).abs();
            assert!(
                delta <= ALIGN_TOL_PX,
                "{label}[{i}] volume bar (x={hx}) is NOT under its triangle (x={tx}): \
                 |delta|={delta} px > {ALIGN_TOL_PX} px tolerance. The buy/sell volume \
                 overlay is detached from the triangles — lab-buy-sell-overlay-align \
                 regression. (chart buys={tri_buy:?} sells={tri_sell:?}; \
                 histo buys={his_buy:?} sells={his_sell:?})"
            );
        }
    }
}

/// NEGATIVE control — a long-only (buys-only) scene paints buy volume bars but
/// NO sell volume bars in the histogram band. Proves the classifier above
/// discriminates buy vs sell (it isn't matching the same pixels for both), so
/// the alignment assertion is meaningful.
#[test]
fn long_only_paints_no_sell_volume_bars() {
    let mut cockpit = charts_screen_cockpit();
    // Replace the alternating buy/sell markers with buys only, spread across
    // the same bar window so they render along the x-axis.
    let sym = Symbol::new("BTCUSDT");
    let base = ui::fixtures::synthetic_fills_for(Venue::Binance, &sym, 4);
    let buys_only: Vec<_> = base
        .into_iter()
        .map(|mut f| {
            f.side = Side::Buy;
            f
        })
        .collect();
    // `lab::view` derives the volume bins from `chart_markers` at compose time,
    // so the seeded buys-only markers flow straight through to the histogram.
    cockpit.chart_markers = PanelState::Ready(buys_only);

    let (w, rgba) = render(cockpit);
    let down = rgb_of(color::DOWN_500.current(ThemeMode::Dark));
    let his_sell = cluster_xs(&rgba, w, HISTO_BAND_Y, down, LEGEND_X_CUTOFF);
    assert!(
        his_sell.is_empty(),
        "long-only scene must paint NO sell (DOWN_500) volume bars in the \
         histogram band, got clusters at {his_sell:?}"
    );
}
