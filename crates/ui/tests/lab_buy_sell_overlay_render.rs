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
use ui::theme::layout::LEGEND_CARD_WIDTH_PX;
use ui::theme::{ThemeMode, color, space};

const VIEW_W: u32 = 1280;
const VIEW_H: u32 = 980;
const SCALE: f32 = 1.0;

// Vertical bands (from the 1280×980 render of the full Lab body): the price
// chart's marker zone and the per-bar-volume strip. Generous so font/AA jitter
// in either widget never drops the band.
const CHART_BAND_Y: (u32, u32) = (545, 650);
const HISTO_BAND_Y: (u32, u32) = (815, 890);

/// The chart legend's column span `[left, right]` in a Lab frame, DERIVED from
/// the frame's plot right edge and the legend's layout constants. The legend
/// card ("Buy"/"Sell" triangle swatches in `UP_500`/`DOWN_500`, the colours of
/// the fill triangles) is anchored top-right INSIDE the plot rect:
/// `space::M` in from the plot's right edge and `LEGEND_CARD_WIDTH_PX` wide
/// (`chart_legend::compute_card_rect_dynamic`, no compare rows in the Lab).
/// So it moves left with the plot's right edge when the chart reserves the
/// equity gutter, and it is excluded from the chart-band clusters by this
/// geometry, not by a fixed column cutoff (in a gutter frame the swatches sat
/// left of the old fixed cutoff and were counted as a third triangle).
fn legend_x_span(plot_right: u32) -> (u32, u32) {
    let right = plot_right as f32 - space::M as f32;
    let left = right - LEGEND_CARD_WIDTH_PX;
    (left.floor() as u32, right.ceil() as u32)
}

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

/// Whether column `x` holds a `target`-coloured pixel within rows `[y0, y1)`.
fn column_hits(rgba: &[u8], w: u32, x: u32, (y0, y1): (u32, u32), target: (i32, i32, i32)) -> bool {
    (y0..y1).any(|y| {
        let idx = ((y * w + x) * 4) as usize;
        let p = (
            i32::from(rgba[idx]),
            i32::from(rgba[idx + 1]),
            i32::from(rgba[idx + 2]),
        );
        close(p, target, 10)
    })
}

/// x-centroids of horizontally-separated clusters of `target` color within the
/// row band `[y0,y1)`, columns split into clusters by gaps > 12 px. Columns in
/// the inclusive span `exclude` (the chart legend) are skipped.
fn cluster_xs(
    rgba: &[u8],
    w: u32,
    rows: (u32, u32),
    target: (i32, i32, i32),
    exclude: Option<(u32, u32)>,
) -> Vec<u32> {
    let cols: Vec<u32> = (0..w)
        .filter(|x| !exclude.is_some_and(|(lo, hi)| (lo..=hi).contains(x)))
        .filter(|&x| column_hits(rgba, w, x, rows, target))
        .collect();
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
    let cockpit = charts_screen_cockpit();
    let fills = seeded_fills(&cockpit);
    let (w, rgba) = render(cockpit);

    if std::env::var_os("UI_DUMP_PNG").is_some()
        && let Some(img) = image::RgbaImage::from_raw(w, VIEW_H, rgba.clone())
    {
        let _ = img.save("/tmp/lab_buy_sell_overlay_aligned.png");
    }

    let frame = measure_lab_frame("the standard Lab scene", w, &rgba);
    assert_volume_bars_under_triangles("the standard Lab scene", &frame, &fills);
}

/// The fills the Lab draws, DERIVED from the scene's own seed — the
/// cockpit's `chart_markers` (`charts_screen_cockpit` seeds them with
/// `ui::fixtures::synthetic_fills_for`) for the active BTCUSDT pair, over the
/// seeded BTCUSDT bars: per side, the index of the bar each fill falls in
/// (the first bar with `open <= ts <= close`, `compute_volume_bins`' rule),
/// left to right.
struct SeededFills {
    buy_bars: Vec<usize>,
    sell_bars: Vec<usize>,
    bar_count: usize,
}

fn seeded_fills(cockpit: &Cockpit) -> SeededFills {
    let symbol = Symbol::new("BTCUSDT");
    let bars = btc_bars();
    let PanelState::Ready(markers) = &cockpit.chart_markers else {
        panic!("the scene must seed chart markers")
    };
    let (mut buy_bars, mut sell_bars) = (Vec::new(), Vec::new());
    for fill in markers.iter().filter(|f| f.symbol == symbol) {
        let ts = fill.venue_ts.unix_millis();
        let idx = bars
            .iter()
            .position(|b| b.open_ts.unix_millis() <= ts && ts <= b.close_ts.unix_millis())
            .expect("every seeded fill lies on a seeded bar");
        match fill.side {
            Side::Buy => buy_bars.push(idx),
            Side::Sell => sell_bars.push(idx),
        }
    }
    buy_bars.sort_unstable();
    sell_bars.sort_unstable();
    for (label, idxs) in [("buy", &buy_bars), ("sell", &sell_bars)] {
        assert!(
            !idxs.is_empty() && idxs.windows(2).all(|p| p[0] < p[1]),
            "precondition: each side has fills, one per bar, so one triangle and one volume \
             bar per fill ({label} bars {idxs:?})"
        );
    }
    SeededFills {
        buy_bars,
        sell_bars,
        bar_count: bars.len(),
    }
}

/// One rendered Lab frame's measured marker geometry.
struct LabFrame {
    /// The plot rect's right edge: the price line's last vertex column.
    plot_right: u32,
    /// The legend's derived column span, excluded from the chart band.
    legend: (u32, u32),
    tri_buy: Vec<u32>,
    tri_sell: Vec<u32>,
    his_buy: Vec<u32>,
    his_sell: Vec<u32>,
}

/// Measure the x-centroid of each buy/sell triangle (chart band, legend
/// excluded by `legend_x_span`) and each buy/sell volume bar (histogram band,
/// which holds no legend). Positive control on the exclusion: the derived
/// legend span must hold the legend's `UP_500` and `DOWN_500` swatches within
/// the chart band, so the box excluded is the legend, wherever the gutter
/// put it.
fn measure_lab_frame(scene: &str, w: u32, rgba: &[u8]) -> LabFrame {
    let up = rgb_of(color::UP_500.current(ThemeMode::Dark));
    let down = rgb_of(color::DOWN_500.current(ThemeMode::Dark));
    let plot_right = rightmost_accent_column(rgba, w, CHART_CANVAS_ROWS)
        .unwrap_or_else(|| panic!("{scene}: the price line must draw"));
    let legend = legend_x_span(plot_right);
    for (name, target) in [("Buy", up), ("Sell", down)] {
        assert!(
            (legend.0..=legend.1).any(|x| column_hits(rgba, w, x, CHART_BAND_Y, target)),
            "{scene}: the derived legend span {legend:?} (plot right edge {plot_right}) must \
             hold the legend's {name} swatch in rows {CHART_BAND_Y:?}"
        );
    }
    LabFrame {
        plot_right,
        legend,
        tri_buy: cluster_xs(rgba, w, CHART_BAND_Y, up, Some(legend)),
        tri_sell: cluster_xs(rgba, w, CHART_BAND_Y, down, Some(legend)),
        his_buy: cluster_xs(rgba, w, HISTO_BAND_Y, up, None),
        his_sell: cluster_xs(rgba, w, HISTO_BAND_Y, down, None),
    }
}

/// Largest |triangle x − volume-bar x| over the left-to-right pairs (no
/// assertion; for the margin line).
fn worst_residual(frame: &LabFrame) -> i32 {
    [
        (&frame.tri_buy, &frame.his_buy),
        (&frame.tri_sell, &frame.his_sell),
    ]
    .iter()
    .flat_map(|(tri, his)| tri.iter().zip(his.iter()))
    .map(|(&tx, &hx)| (tx as i32 - hx as i32).abs())
    .max()
    .unwrap_or(0)
}

/// Assert both widgets painted exactly the seeded fills — as many triangles
/// and as many volume bars per side as `fills` holds (the proof is
/// meaningless if either is blank, the Reports-curve trap; and a count above
/// the fills means something else, such as a legend swatch, was counted) —
/// and that each volume bar sits under its triangle within `ALIGN_TOL_PX`.
/// `scene` names the frame in the failure message. Returns the largest
/// residual.
fn assert_volume_bars_under_triangles(scene: &str, frame: &LabFrame, fills: &SeededFills) -> i32 {
    let LabFrame {
        plot_right,
        legend,
        tri_buy,
        tri_sell,
        his_buy,
        his_sell,
    } = frame;
    let want = (fills.buy_bars.len(), fills.sell_bars.len());
    assert_eq!(
        (tri_buy.len(), tri_sell.len()),
        want,
        "{scene}: the chart must paint one triangle per seeded fill, (buys, sells) = {want:?} \
         (buy={tri_buy:?}, sell={tri_sell:?}; legend {legend:?} excluded, plot right edge \
         {plot_right})"
    );
    assert_eq!(
        (his_buy.len(), his_sell.len()),
        want,
        "{scene}: the histogram must paint one volume bar per seeded fill, (buys, sells) = \
         {want:?} (buy={his_buy:?}, sell={his_sell:?})"
    );

    // Each histogram bar sits under its triangle (paired left-to-right).
    for (label, tri, his) in [("BUY", tri_buy, his_buy), ("SELL", tri_sell, his_sell)] {
        for (i, (&tx, &hx)) in tri.iter().zip(his.iter()).enumerate() {
            let delta = (tx as i32 - hx as i32).abs();
            assert!(
                delta <= ALIGN_TOL_PX,
                "{scene}: {label}[{i}] volume bar (x={hx}) is NOT under its triangle (x={tx}): \
                 |delta|={delta} px > {ALIGN_TOL_PX} px tolerance. The buy/sell volume \
                 overlay is detached from the triangles — lab-buy-sell-overlay-align \
                 regression. (chart buys={tri_buy:?} sells={tri_sell:?}; \
                 histo buys={his_buy:?} sells={his_sell:?})"
            );
        }
    }
    worst_residual(frame)
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
    let his_sell = cluster_xs(&rgba, w, HISTO_BAND_Y, down, None);
    assert!(
        his_sell.is_empty(),
        "long-only scene must paint NO sell (DOWN_500) volume bars in the \
         histogram band, got clusters at {his_sell:?}"
    );
}

// ── bug-log #207 — the full Lab: the equity gutter is reserved iff it draws ──
//
// LR-207d: the operator's screen. The Lab is pointed at the v1 momentum
// strategy on BTCUSDT (defaults: Last90d, Synthetic) and its equity cache is
// pre-warmed from a temp root (`EquityCache::get_or_load`), so `lab::view`'s
// `route_equity_overlay` is a cache hit and never reads the real
// `lab-runs/`/`evidence/` roots: the frame depends on the fixture only.
//   A — the committed start/end-only momentum report (copied byte-for-byte):
//       its ordinal `0`/`1` samples miss the seeded bar window.
//   B — an in-window per-bar report (`ui::test_support::lab_runs_report_at`).
// A must keep the chart full width (no empty gutter) with the histogram still
// under the triangles; B reserves the gutter. The right-edge shift A − B is
// derived from the gutter constants.

use std::path::Path;

use trading_core::StrategyId;
use ui::lab::equity_loader::{Fidelity, LabTuple, default_evidence_root};
use ui::test_support::{EQUITY_GUTTER_WIDENING_PX, lab_runs_report_at};

/// The committed start/end-only momentum report, relative to `evidence/`.
const COMMITTED_START_END_ONLY: &str =
    "v1/v1-cross-sectional-momentum/reports/backtest-20260429-195243-top10-2024-h1-momentum.md";

/// Rows of the 1280×980 Lab frame searched for the chart's right edge: the
/// chart canvas (rows ≈ 563..695, below the KPI strip and above the
/// position-size strip), calibrated by `diag_207_lab_bands` and read off the
/// PNGs. The legend is excluded by geometry, not by rows: here it overlaps
/// the plot's rows, but it is inset from the plot's right edge and its `ACCENT`
/// "Price" stub sits at its left, while the price line's last vertex sits ON
/// the right edge, right of the legend box.
const CHART_CANVAS_ROWS: (u32, u32) = (560, 700);

/// The Lab scene at a strategy: `charts_screen_cockpit` (60 seeded BTCUSDT
/// bars, 4 alternating fills at bars 12/24/36/48) pointed at v1 momentum on
/// BTCUSDT, its equity cache pre-warmed from `root`. Returns the cockpit and
/// the cached series.
fn lab_207_scene(root: &Path) -> (Cockpit, ui::lab::equity_loader::LabEquitySeries) {
    let mut cockpit = charts_screen_cockpit();
    let strategy = StrategyId::new("v1.momentum");
    let symbol = Symbol::new("BTCUSDT");
    cockpit.lab_state.strategy = Some(strategy.clone());
    cockpit.lab_state.pair = Some((Venue::Binance, symbol.clone()));
    // The tuple exactly as `lab::view` builds it from `lab_state`.
    let tuple = LabTuple::new(
        &strategy,
        Venue::Binance,
        &symbol,
        cockpit.lab_state.range.clone(),
        cockpit.lab_state.data_source,
    );
    let series = cockpit
        .equity_cache
        .borrow_mut()
        .get_or_load(&tuple, root)
        .expect("the fixture report resolves for the Lab tuple")
        .as_ref()
        .clone();
    (cockpit, series)
}

/// The 60 seeded BTCUSDT bars, from the same builder (and seed and count)
/// `charts_screen_cockpit` seeds the chart with.
fn btc_bars() -> Vec<trading_core::Bar> {
    let symbol = Symbol::new("BTCUSDT");
    let seed = ui::fixtures::seed_for(Venue::Binance, &symbol);
    ui::fixtures::synthetic_candles(seed, Venue::Binance, symbol, 60)
}

/// The seeded BTCUSDT bar window `[first open, last close]` in ms.
fn btc_bar_window_ms() -> (i64, i64) {
    let bars = btc_bars();
    (
        bars.first().expect("bars").open_ts.unix_millis(),
        bars.last().expect("bars").close_ts.unix_millis(),
    )
}

/// Write the scene-A root: the committed report copied byte-for-byte.
fn write_committed_root(root: &Path) {
    let src = default_evidence_root().join(COMMITTED_START_END_ONLY);
    assert!(
        src.is_file(),
        "the committed report must exist: {}",
        src.display()
    );
    let dir = root.join("v1-cross-sectional-momentum").join("reports");
    std::fs::create_dir_all(&dir).expect("create reports dir");
    let dst = dir.join(src.file_name().expect("file name"));
    std::fs::copy(&src, &dst).expect("copy the committed report");
    assert_eq!(
        std::fs::read(&src).expect("read source"),
        std::fs::read(&dst).expect("read copy"),
        "the copy must be byte-identical"
    );
}

/// Write the scene-B root: an in-window per-bar report for BTCUSDT.
fn write_in_window_root(root: &Path) {
    let dir = root.join("v1-cross-sectional-momentum").join("reports");
    std::fs::create_dir_all(&dir).expect("create reports dir");
    std::fs::write(
        dir.join("backtest-20260601-120000-top10-2024-h1-momentum.md"),
        lab_runs_report_at("top10-2024-h1-momentum", "BTCUSDT", 100_000),
    )
    .expect("write the in-window report");
}

/// Rightmost column within rows `[y0, y1)` holding an `ACCENT` pixel (the
/// price line's last vertex sits on the plot's right edge; an in-window equity
/// curve, also `ACCENT`, ends before it).
fn rightmost_accent_column(rgba: &[u8], w: u32, (y0, y1): (u32, u32)) -> Option<u32> {
    let accent = rgb_of(color::ACCENT.current(ThemeMode::Dark));
    (0..w).rev().find(|&x| {
        (y0..y1).any(|y| {
            let idx = ((y * w + x) * 4) as usize;
            let p = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            close(p, accent, 10)
        })
    })
}

/// Write `rgba` to `path` when `UI_DUMP_PNG` is set (this file's convention).
fn dump_png(w: u32, rgba: &[u8], path: &str) {
    if std::env::var_os("UI_DUMP_PNG").is_some()
        && let Some(img) = image::RgbaImage::from_raw(w, VIEW_H, rgba.to_vec())
    {
        let _ = img.save(path);
    }
}

/// Diagnostic (run with `--nocapture`): the row extents of the UP/DOWN marker
/// colours and the rightmost `ACCENT` column per 20-row band for scenes A and
/// B, so `CHART_BAND_Y`, `HISTO_BAND_Y` and `CHART_CANVAS_ROWS` are
/// placed by measurement. Not an assertion gate.
#[test]
fn diag_207_lab_bands() {
    let up = rgb_of(color::UP_500.current(ThemeMode::Dark));
    let down = rgb_of(color::DOWN_500.current(ThemeMode::Dark));
    for (name, write) in [
        ("A committed", write_committed_root as fn(&Path)),
        ("B in-window", write_in_window_root as fn(&Path)),
    ] {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path());
        let (cockpit, _) = lab_207_scene(tmp.path());
        let (w, rgba) = render(cockpit);
        let legend = rightmost_accent_column(&rgba, w, CHART_CANVAS_ROWS).map(legend_x_span);
        eprintln!("[diag #207] {name}: derived legend span {legend:?}");
        let rows_with = |target: (i32, i32, i32)| -> Vec<u32> {
            (0..VIEW_H)
                .filter(|&y| {
                    (0..w)
                        .filter(|x| !legend.is_some_and(|(lo, hi)| (lo..=hi).contains(x)))
                        .any(|x| {
                            let idx = ((y * w + x) * 4) as usize;
                            let p = (
                                i32::from(rgba[idx]),
                                i32::from(rgba[idx + 1]),
                                i32::from(rgba[idx + 2]),
                            );
                            close(p, target, 10)
                        })
                })
                .collect()
        };
        let edges: Vec<String> = (0..VIEW_H)
            .step_by(20)
            .filter_map(|y0| {
                rightmost_accent_column(&rgba, w, (y0, (y0 + 20).min(VIEW_H)))
                    .map(|x| format!("{y0}:{x}"))
            })
            .collect();
        eprintln!("[diag #207] {name}: UP rows {:?}", rows_with(up));
        eprintln!("[diag #207] {name}: DOWN rows {:?}", rows_with(down));
        eprintln!("[diag #207] {name}: rightmost ACCENT by 20-row band {edges:?}");
        eprintln!(
            "[diag #207] {name}: CHART_CANVAS_ROWS {CHART_CANVAS_ROWS:?} -> {:?}",
            rightmost_accent_column(&rgba, w, CHART_CANVAS_ROWS)
        );
    }
}

/// **LR-207d — the operator's Lab screen (bug-log #207).** Scene A (the
/// committed start/end-only report, out of the bar window) keeps the chart
/// full width and the volume bars under their triangles; scene B (an
/// in-window per-bar report) reserves the equity gutter. The chart's right
/// edge in A minus B is the derived gutter widening
/// (`EQUITY_GUTTER_WIDENING_PX` = `AXIS_GUTTER_EQUITY_PX` 56.0 −
/// `AXIS_GUTTER_RIGHT_PX` 16.0 = 40 px) within `[40 − 6, 40 + 4]`. Under the
/// pre-#207 rule A reserves the gutter too (shift 0). When only one of the
/// chart and the histogram follows the rule, the plot widths differ by 40 px
/// and the right-hand fills drift from their volume bars (the `SELL` fill at
/// bar 48 by 20 px > `ALIGN_TOL_PX`, measured with the histogram leg reverted).
///
/// The histogram's own shift is pinned too, DERIVED from the geometry: the
/// histogram centres volume bar `i` of `n` at `chart::x_for_index(i, n, plot)`
/// = `plot.x + i / (n − 1) · plot.width`, and B's plot rect has the same left
/// edge (`AXIS_GUTTER_PRICE_PX` in both) and is `EQUITY_GUTTER_WIDENING_PX`
/// narrower. So the volume bar of a fill on bar `i` sits
/// `i / (n − 1) · EQUITY_GUTTER_WIDENING_PX` further right in A than in B:
/// for the seeded fills on bars 12, 24, 36 and 48 of 60, 8.1, 16.3, 24.4 and
/// 32.5 px, each within `HISTO_SHIFT_TOL_PX`. A histogram that ignores the
/// rule (shift 0) misses every one by more than the tolerance, independent of
/// the triangle/volume residual.
/// PNGs (with `UI_DUMP_PNG`): `/tmp/lab_207_full_out_of_window.png`,
/// `/tmp/lab_207_full_in_window.png`.
#[test]
fn lab_start_end_only_overlay_keeps_the_chart_full_width_and_aligned() {
    let (win_start, win_end) = btc_bar_window_ms();

    let tmp_a = tempfile::tempdir().expect("tempdir");
    write_committed_root(tmp_a.path());
    let (cockpit_a, series_a) = lab_207_scene(tmp_a.path());
    assert_eq!(series_a.fidelity, Fidelity::StartEndOnly);
    assert!(
        series_a
            .samples
            .iter()
            .all(|(ts, _)| *ts < win_start || *ts > win_end),
        "precondition A: every sample misses the bar window [{win_start}, {win_end}]"
    );

    let tmp_b = tempfile::tempdir().expect("tempdir");
    write_in_window_root(tmp_b.path());
    let (cockpit_b, series_b) = lab_207_scene(tmp_b.path());
    assert_eq!(series_b.fidelity, Fidelity::PerBar);
    assert!(
        series_b
            .samples
            .iter()
            .all(|(ts, _)| (win_start..=win_end).contains(ts)),
        "precondition B: every sample lies in the bar window [{win_start}, {win_end}]"
    );

    let fills = seeded_fills(&cockpit_a);
    let fills_b = seeded_fills(&cockpit_b);
    assert_eq!(
        (&fills.buy_bars, &fills.sell_bars),
        (&fills_b.buy_bars, &fills_b.sell_bars),
        "precondition: both scenes seed the same fills"
    );

    let (w_a, rgba_a) = render(cockpit_a);
    let (w_b, rgba_b) = render(cockpit_b);
    dump_png(w_a, &rgba_a, "/tmp/lab_207_full_out_of_window.png");
    dump_png(w_b, &rgba_b, "/tmp/lab_207_full_in_window.png");

    let frame_a = measure_lab_frame("#207 scene A (out of window)", w_a, &rgba_a);
    let frame_b = measure_lab_frame("#207 scene B (in window)", w_b, &rgba_b);

    // Chart: the plot's right edge moves by the derived gutter widening.
    let (edge_a, edge_b) = (frame_a.plot_right, frame_b.plot_right);
    let shift = i64::from(edge_a) - i64::from(edge_b);
    let expected = EQUITY_GUTTER_WIDENING_PX.round() as i64;
    let (lo, hi) = (expected - 6, expected + 4);

    // Histogram: each fill's volume bar moves by its derived share of it.
    let span = (fills.bar_count - 1) as f32;
    let histo_shifts: Vec<(String, f32, i64)> = [
        ("BUY", &fills.buy_bars, &frame_a.his_buy, &frame_b.his_buy),
        (
            "SELL",
            &fills.sell_bars,
            &frame_a.his_sell,
            &frame_b.his_sell,
        ),
    ]
    .into_iter()
    .flat_map(|(label, bars, his_a, his_b)| {
        bars.iter()
            .zip(his_a.iter().zip(his_b.iter()))
            .enumerate()
            .map(move |(i, (&bar, (&xa, &xb)))| {
                let want = bar as f32 / span * EQUITY_GUTTER_WIDENING_PX;
                (
                    format!("{label}[{i}] bar {bar}"),
                    want,
                    i64::from(xa) - i64::from(xb),
                )
            })
    })
    .collect();
    let histo_worst = histo_shifts
        .iter()
        .map(|(_, want, got)| (*got as f32 - want).abs())
        .fold(0.0_f32, f32::max);

    eprintln!(
        "#207 pixel margin LR-207d: right edge A {edge_a} - B {edge_b} = {shift} px (expected \
         {expected}, band [{lo}, {hi}]); worst triangle/volume residual A {} px, B {} px \
         (threshold <= {ALIGN_TOL_PX})",
        worst_residual(&frame_a),
        worst_residual(&frame_b)
    );
    eprintln!(
        "#207 pixel margin LR-207d histogram shift A - B: {histo_shifts:?} (label, derived, \
         measured); worst |measured - derived| {histo_worst:.1} px (threshold <= \
         {HISTO_SHIFT_TOL_PX})"
    );

    let worst_a =
        assert_volume_bars_under_triangles("#207 scene A (out of window)", &frame_a, &fills);
    let worst_b = assert_volume_bars_under_triangles("#207 scene B (in window)", &frame_b, &fills);
    assert_eq!(
        histo_shifts.len(),
        fills.buy_bars.len() + fills.sell_bars.len(),
        "every seeded fill's volume bar has a measured A - B shift"
    );
    assert!(
        histo_worst <= HISTO_SHIFT_TOL_PX,
        "#207: the histogram must follow the chart's gutter: each volume bar's A - B shift must \
         be its derived i / (n - 1) * {EQUITY_GUTTER_WIDENING_PX} px within \
         {HISTO_SHIFT_TOL_PX} px, got {histo_shifts:?} (label, derived, measured)"
    );
    assert!(
        (lo..=hi).contains(&shift),
        "#207: the out-of-window overlay must leave the chart full width and the in-window one \
         must reserve the gutter: right edge A {edge_a}, B {edge_b}, shift {shift} px, expected \
         {expected} within [{lo}, {hi}] (worst residuals A {worst_a}, B {worst_b})"
    );
}

/// Tolerance on LR-207d's derived per-bar histogram shift (logical px): the
/// volume-bar centroids are integer column means, and the shifts derived from
/// the gutter constants are 8.1 to 32.5 px.
const HISTO_SHIFT_TOL_PX: f32 = 4.0;
