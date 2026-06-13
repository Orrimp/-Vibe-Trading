//! Render-layer proofs for the Binance Lab path (simple-strategies-realdata
//! T-B1 / T-C4 — AC7, project law `feedback_verify_ui_at_render_layer`).
//!
//! Model-state assertions are NECESSARY but NOT SUFFICIENT (the Live-equity
//! saga: every prior fix passed at the model layer while the rendered curve
//! was broken). These tests rasterize the REAL widgets via
//! `iced_test::screenshot` (view → draw → tiny_skia readback) and inspect
//! pixels:
//!
//!   T-B1 — the three-way `source_toggle` renders THREE chips and the active
//!          chip's `ACCENT` highlight is at the correct position (the Binance
//!          chip is third, so its highlight is RIGHT of Synthetic's/Yahoo's).
//!   T-C4 — a Binance-SOURCED run's equity curve (real BTC 2023-H1 bars →
//!          `v0.sma` → `run_scenario` → equity series) rasterizes a visible
//!          `ACCENT` polyline on the real Lab overlay widget.

#![cfg(all(feature = "live", feature = "binance"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::Duration;

use ui::lab::state::LabDataSource;
use ui::test_support::source_toggle_program;
use ui::theme::{ThemeMode, color};

const VIEW_W: u32 = 480;
const VIEW_H: u32 = 80;
const SCALE: f32 = 1.0;

/// `(r,g,b)` 0-255 of an `ACCENT` (dark theme) pixel — the active chip's
/// background fill (and the equity polyline color).
fn accent_rgb() -> (i32, i32, i32) {
    let c = color::ACCENT.current(ThemeMode::Dark);
    (
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    )
}

/// ±tol per-channel ACCENT match (tiny_skia AA the chip border/text edges).
const CHANNEL_TOL: i32 = 30;

fn is_accent(r: u8, g: u8, b: u8) -> bool {
    let (ar, ag, ab) = accent_rgb();
    (i32::from(r) - ar).abs() <= CHANNEL_TOL
        && (i32::from(g) - ag).abs() <= CHANNEL_TOL
        && (i32::from(b) - ab).abs() <= CHANNEL_TOL
}

/// ACCENT pixel count + horizontal bounding box (the active chip's band).
struct AccentBand {
    count: usize,
    min_x: u32,
    max_x: u32,
}

fn accent_band(shot: &iced::window::Screenshot) -> AccentBand {
    let w = shot.size.width;
    let h = shot.size.height;
    let rgba: &[u8] = &shot.rgba;
    let mut count = 0usize;
    let (mut min_x, mut max_x) = (u32::MAX, 0u32);
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            if idx + 2 >= rgba.len() {
                continue;
            }
            if is_accent(rgba[idx], rgba[idx + 1], rgba[idx + 2]) {
                count += 1;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }
    }
    AccentBand {
        count,
        min_x,
        max_x,
    }
}

fn render_toggle(current: LabDataSource) -> iced::window::Screenshot {
    let program = source_toggle_program(current);
    let theme = iced::Theme::Dark;
    iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO)
}

/// Center-x of the active chip's ACCENT band (the highlight midpoint).
fn active_center_x(current: LabDataSource) -> f32 {
    let band = accent_band(&render_toggle(current));
    assert!(
        band.count > 0,
        "the active chip ({current:?}) must paint an ACCENT highlight band"
    );
    (band.min_x as f32 + band.max_x as f32) / 2.0
}

/// **T-B1 — three-way toggle render proof (AC7).** With the `binance` feature
/// the toggle renders THREE chips left-to-right (Synthetic, Yahoo, Binance).
/// Selecting each makes exactly that chip the ACCENT-highlighted one. We prove
/// (a) each state paints a real ACCENT band (the chip drew), and (b) the
/// highlight MARCHES RIGHT as the active source advances — Synthetic's center
/// is left of Yahoo's, which is left of Binance's. A two-chip render (Binance
/// chip missing) or a wrong-active-state would break the ordering.
#[test]
fn three_way_toggle_active_chip_marches_right() {
    let synthetic_x = active_center_x(LabDataSource::Synthetic);
    let yahoo_x = active_center_x(LabDataSource::YahooCache);
    let binance_x = active_center_x(LabDataSource::BinanceCache);

    // Each chip is a distinct position (no two collapse to the same band).
    assert!(
        synthetic_x < yahoo_x,
        "Synthetic chip ({synthetic_x}) must be LEFT of the Yahoo chip ({yahoo_x})"
    );
    assert!(
        yahoo_x < binance_x,
        "Yahoo chip ({yahoo_x}) must be LEFT of the Binance chip ({binance_x}) — \
         the Binance chip is the THIRD chip; if it failed to render (e.g. the \
         feature gate is wrong) this ordering breaks"
    );
}

/// **T-B1 (companion) — the Binance chip is a real, sizeable highlight.**
/// Belt-and-braces: the Binance-active render paints a non-trivial ACCENT band
/// (not a 1-px sliver), proving the third chip genuinely rasterized, not just a
/// border artifact.
#[test]
fn binance_chip_renders_visible_highlight() {
    let band = accent_band(&render_toggle(LabDataSource::BinanceCache));
    assert!(
        band.count >= 50,
        "the active Binance chip must paint a visible ACCENT band (got {} px, \
         expected ≥ 50) — a real filled chip, not an AA sliver",
        band.count
    );
    let span = band.max_x.saturating_sub(band.min_x);
    assert!(
        span >= 20,
        "the Binance chip highlight must span its width (got {span} px, ≥ 20)"
    );
}

// ── T-C4 — Binance-sourced equity curve rasterization ─────────────────────────

use backtest::engine::{DateRange, ScenarioConfig, ScenarioDataSource, run_scenario};
use backtest::progress::ProgressSender;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use trading_core::{StrategyId, Symbol, Venue};
use ui::lab::defaults::LAB_DEFAULT_SEED;
use ui::lab::equity_loader::LabEquitySeries;
use ui::lab::runner::{DefaultLabBinanceBarSource, LabBarSource, LabRunConfig};
use ui::test_support::chart_overlay_program;

const RANGE_2023_H1: DateRange = DateRange::Custom {
    start_ms: 1_672_531_200_000, // 2023-01-01T00:00:00Z
    end_ms: 1_688_169_600_000,   // 2023-07-01T00:00:00Z
};

/// Full-frame curve crop (the overlay widget fills the frame, no sidebar).
const CURVE_VIEW_W: u32 = 1280;
const CURVE_VIEW_H: u32 = 720;

/// `(r,g,b)` of an `ACCENT_2` pixel — the compare-overlay curve color, used to
/// isolate the equity curve from the same-colored ACCENT price line.
fn accent2_rgb() -> (i32, i32, i32) {
    let c = color::ACCENT_2.current(ThemeMode::Dark);
    (
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    )
}

fn dist2(p: (i32, i32, i32), q: (i32, i32, i32)) -> i32 {
    let (dr, dg, db) = (p.0 - q.0, p.1 - q.1, p.2 - q.2);
    dr * dr + dg * dg + db * db
}

/// Count `ACCENT_2` pixels across the whole frame (the compare/equity curve).
const CURVE_MATCH_R2: i32 = 900;

fn count_accent2(shot: &iced::window::Screenshot) -> usize {
    let rgba: &[u8] = &shot.rgba;
    let target = accent2_rgb();
    let accent = accent_rgb();
    let mut n = 0usize;
    let mut i = 0;
    while i + 2 < rgba.len() {
        let p = (
            i32::from(rgba[i]),
            i32::from(rgba[i + 1]),
            i32::from(rgba[i + 2]),
        );
        // Winner-take-all vs the ACCENT price line so an ACCENT_2 count is the
        // overlay curve only (the two share a hue).
        if dist2(p, target) <= CURVE_MATCH_R2 && dist2(p, target) < dist2(p, accent) {
            n += 1;
        }
        i += 4;
    }
    n
}

/// Load real Binance bars and run `v0.sma` to produce a genuine Binance-sourced
/// equity series. Returns `(bars, equity_series)` or `None` if the corpus is
/// absent (test skips — the gitignored corpus may not be present in CI).
fn binance_sourced_run() -> Option<(Vec<trading_core::Bar>, Vec<(i64, Decimal)>)> {
    let cfg = LabRunConfig {
        strategy_id: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        venue: SmolStr::new("Binance"),
        range_label: SmolStr::new("Custom:2023-01-01:2023-07-01"),
        seed: LAB_DEFAULT_SEED,
        write_report: false,
        data_source: LabDataSource::BinanceCache,
        sma_fast_len: None,
        sma_slow_len: None,
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime builds");
    let bars = match rt.block_on(async {
        DefaultLabBinanceBarSource
            .preload(&cfg, &RANGE_2023_H1)
            .await
    }) {
        Ok((bars, _sha)) => bars,
        Err(e) => {
            eprintln!("[skip] Binance corpus absent ({e}); curve raster proof skipped");
            return None;
        }
    };
    let scenario = ScenarioConfig {
        strategy: StrategyId("v0.sma".into()),
        pair: (Venue::Binance, Symbol::new("BTCUSDT")),
        range: RANGE_2023_H1,
        params: None,
        seed: LAB_DEFAULT_SEED,
        write_report: false,
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(bars.clone()),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
    };
    let (_cancel, recv) = ui::lab::runner::cancellation_pair();
    let report = rt
        .block_on(run_scenario(scenario, recv, ProgressSender::disabled()))
        .expect("Binance v0.sma run succeeds");
    let equity: Vec<(i64, Decimal)> = report
        .equity_series
        .iter()
        .map(|(ts, money)| (ts.unix_millis(), money.amount()))
        .collect();
    Some(((*report.bars).clone(), equity))
}

/// **T-C4 — Binance-sourced equity curve rasterizes (AC7, the render gate).**
///
/// A run on REAL Binance 2023-H1 BTC bars produces an equity series; fed to the
/// real Lab overlay widget (the exact `chart::view` draw path the Lab/Compare
/// screens paint), it rasterizes a visible `ACCENT_2` polyline. This closes the
/// "wired but doesn't paint" gap for the Binance path: the curve is the SECOND
/// (compare-color) overlay so it is isolable from the ACCENT price line.
#[test]
fn binance_sourced_equity_curve_rasterizes() {
    let Some((bars, equity)) = binance_sourced_run() else {
        return; // corpus absent — skip
    };

    assert!(
        equity.len() >= 2,
        "Binance run must yield ≥2 equity points to draw a traversing curve (got {})",
        equity.len()
    );

    // Feed the Binance-sourced equity as the COMPARE (ACCENT_2) overlay so it is
    // color-isolable from the ACCENT price line. (The equity values fall inside
    // the bar window by construction — they ARE the per-bar equity of these
    // very bars.)
    let series = LabEquitySeries::from_samples(equity, SmolStr::new("binance-btc-2023h1"))
        .expect("non-empty equity → Some(series)");

    let program = chart_overlay_program(bars, None, vec![series]);
    let theme = iced::Theme::Dark;
    let shot = iced_test::screenshot(
        &program,
        &theme,
        (CURVE_VIEW_W, CURVE_VIEW_H),
        SCALE,
        Duration::ZERO,
    );

    let accent2 = count_accent2(&shot);
    assert!(
        accent2 >= 120,
        "the Binance-sourced equity curve did NOT rasterize: only {accent2} \
         ACCENT_2 pixels (expected ≥ 120 for a traversing polyline). The run \
         produced equity but the canvas painted no visible curve — the \
         'wired but doesn't paint' gap for the Binance path."
    );
}
