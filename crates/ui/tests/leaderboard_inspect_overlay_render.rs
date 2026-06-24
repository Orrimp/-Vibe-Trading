//! Render-layer proof (project law `feedback_verify_ui_at_render_layer` +
//! `spec/dev-notes/iced-ui-render-verification.md`) for
//! advisor-leaderboard-inspect-in-lab.
//!
//! ## What this proves
//!
//! Clicking a leaderboard data row for a NON-SMA pick (`v0.5.macd`) navigates to
//! the Lab PRESEEDED on the chosen coin (ETHUSDT) + window, and the Lab chart
//! then paints the buy/sell **triangles** for that strategy's fills with the
//! per-bar volume **overlay** aligned UNDER them — the same overlay the operator
//! sees in the Lab. This is the render-pixel gate the feature exists to satisfy:
//! a no-panic boot or a passing logic test is NOT proof the chart draws.
//!
//! ## Drive path (the real message, then real render)
//!
//! We drive the actual `Message::InspectStrategyFromLeaderboard` through
//! `ui::state::update` (so the navigation/preseed under test runs end-to-end and
//! lands `selected_symbol = (Binance, ETHUSDT)`), seed ETHUSDT bars + alternating
//! buy/sell fills into `chart_markers` (the exact slice `lab::view` derives both
//! the triangles AND the aligned volume bins from — mirroring the proven
//! `lab_buy_sell_overlay_render.rs` harness), then rasterize the REAL Lab body
//! via `iced_test::screenshot` and measure the painted clusters.
//!
//! Rendering the chart from a seeded `chart_markers` slice (rather than spinning
//! the engine inside a screenshot test) matches the established overlay-render
//! harness; the FIDELITY of the run path itself — that `v0.5.macd` dispatches to
//! the real ComposedStrategy, not an SMA proxy — is proven separately by the
//! `run_scenario` dispatch trace (engine.rs `"v0.5.macd"` arm → `btc_macd_trend`).
//!
//! NEGATIVE control: a long-only (buys-only) inspect paints buy bars but NO sell
//! bars in the histogram band — proving the buy/sell classifier discriminates
//! and the positive assertion is meaningful.

// Gated to macOS only (the iced_test screenshot readback path), matching the
// sibling `lab_buy_sell_overlay_render.rs`. NOT `fixtures`-gated: `ui::fixtures`
// and `ui::test_support` are unconditionally public (lib.rs), so this runs under
// the standard `cargo test -p ui` default-feature gate alongside the other
// render harnesses — no `--features fixtures` needed.
#![cfg(target_os = "macos")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::Duration;

use trading_core::{Side, StrategyId, Symbol, Venue};
use ui::leaderboard::LeaderboardLookback;
use ui::state::{Cockpit, Message, PanelState, Screen, update};
use ui::test_support::{charts_screen_cockpit, program_from_cockpit};
use ui::theme::{ThemeMode, color};

const VIEW_W: u32 = 1280;
const VIEW_H: u32 = 980;
const SCALE: f32 = 1.0;

// Same bands as lab_buy_sell_overlay_render.rs (the full Lab body at 1280×980).
const CHART_BAND_Y: (u32, u32) = (545, 650);
const HISTO_BAND_Y: (u32, u32) = (815, 890);
const LEGEND_X_CUTOFF: u32 = 1080;
const ALIGN_TOL_PX: i32 = 16;

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

/// Build the cockpit an operator reaches by clicking the `v0.5.macd` leaderboard
/// row while ETHUSDT is the chosen coin: start on the Leaderboard, drive the real
/// inspect message, then seed ETHUSDT fills (the chart's marker source) for the
/// requested `sides`.
fn inspected_eth_cockpit(sides: &[Side]) -> Cockpit {
    // Base scene has 60 synthetic bars seeded for BTC/ETH/SOL already.
    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = Screen::Leaderboard;

    // The real navigation+preseed under test: inspect v0.5.macd on ETHUSDT.
    update(
        &mut cockpit,
        Message::InspectStrategyFromLeaderboard {
            strategy: StrategyId::new("v0.5.macd"),
            coin: Symbol::new("ETHUSDT"),
            lookback: LeaderboardLookback::H1_2024,
        },
    );
    // Sanity: the inspect landed us in the Lab on ETHUSDT.
    assert_eq!(cockpit.current_screen, Screen::Lab);
    assert_eq!(
        cockpit.selected_symbol.as_ref().map(|(_, s)| s.0.as_str()),
        Some("ETHUSDT"),
        "inspect must set the active symbol to the chosen coin (drives chart filter)"
    );

    // Seed ETHUSDT fills with the requested side pattern. `lab::view` filters
    // `chart_markers` to the active symbol (ETHUSDT) and derives BOTH the chart
    // triangles and the aligned volume bins from this slice.
    let sym = Symbol::new("ETHUSDT");
    let base = ui::fixtures::synthetic_fills_for(Venue::Binance, &sym, sides.len());
    let fills: Vec<_> = base
        .into_iter()
        .zip(sides.iter().copied())
        .map(|(mut f, side)| {
            f.side = side;
            f
        })
        .collect();
    cockpit.chart_markers = PanelState::Ready(fills);
    cockpit
}

/// POSITIVE: inspecting the `v0.5.macd` row lands in the Lab on ETHUSDT and the
/// chart paints buy+sell triangles with the per-bar volume overlay aligned under
/// them.
#[test]
fn inspect_macd_paints_aligned_buy_sell_overlay() {
    // Alternating buy/sell fills — the standard populated overlay scene.
    let cockpit = inspected_eth_cockpit(&[Side::Buy, Side::Sell, Side::Buy, Side::Sell]);
    let (w, rgba) = render(cockpit);

    if std::env::var_os("UI_DUMP_PNG").is_some()
        && let Some(img) = image::RgbaImage::from_raw(w, VIEW_H, rgba.clone())
    {
        let _ = img.save("/tmp/leaderboard_inspect_macd_overlay.png");
    }

    let up = rgb_of(color::UP_500.current(ThemeMode::Dark));
    let down = rgb_of(color::DOWN_500.current(ThemeMode::Dark));

    let tri_buy = cluster_xs(&rgba, w, CHART_BAND_Y, up, LEGEND_X_CUTOFF);
    let tri_sell = cluster_xs(&rgba, w, CHART_BAND_Y, down, LEGEND_X_CUTOFF);
    let his_buy = cluster_xs(&rgba, w, HISTO_BAND_Y, up, LEGEND_X_CUTOFF);
    let his_sell = cluster_xs(&rgba, w, HISTO_BAND_Y, down, LEGEND_X_CUTOFF);

    // The chart + histogram must actually paint the inspected strategy's fills —
    // the proof is meaningless if the Lab opened blank (the Reports-curve trap).
    assert!(
        tri_buy.len() > 1 && !tri_sell.is_empty(),
        "inspected Lab chart must paint buy+sell triangles for the picked coin \
         (buy={tri_buy:?}, sell={tri_sell:?}) — a blank chart means the inspect \
         preseed did not flow ETHUSDT markers to the chart"
    );
    assert!(
        his_buy.len() > 1 && !his_sell.is_empty(),
        "inspected Lab histogram must paint buy+sell volume bars \
         (buy={his_buy:?}, sell={his_sell:?})"
    );

    // Each histogram bar sits under its triangle (paired left-to-right) — the
    // overlay is aligned for the inspected strategy, not just present.
    for (label, tri, his) in [("BUY", &tri_buy, &his_buy), ("SELL", &tri_sell, &his_sell)] {
        for (i, (&tx, &hx)) in tri.iter().zip(his.iter()).enumerate() {
            let delta = (tx as i32 - hx as i32).abs();
            assert!(
                delta <= ALIGN_TOL_PX,
                "{label}[{i}] volume bar (x={hx}) is NOT under its triangle (x={tx}): \
                 |delta|={delta} px > {ALIGN_TOL_PX} px tolerance (chart buys={tri_buy:?} \
                 sells={tri_sell:?}; histo buys={his_buy:?} sells={his_sell:?})"
            );
        }
    }
}

/// NEGATIVE control — inspecting with a long-only (buys-only) fill set paints buy
/// volume bars but NO sell volume bars in the histogram band. Proves the buy/sell
/// classifier discriminates (it is not matching the same pixels for both), so the
/// positive alignment assertion above is meaningful.
#[test]
fn inspect_long_only_paints_no_sell_volume_bars() {
    let cockpit = inspected_eth_cockpit(&[Side::Buy, Side::Buy, Side::Buy, Side::Buy]);
    let (w, rgba) = render(cockpit);

    let down = rgb_of(color::DOWN_500.current(ThemeMode::Dark));
    let his_sell = cluster_xs(&rgba, w, HISTO_BAND_Y, down, LEGEND_X_CUTOFF);
    assert!(
        his_sell.is_empty(),
        "a long-only inspect must paint NO sell (DOWN_500) volume bars in the \
         histogram band, got clusters at {his_sell:?}"
    );
}
