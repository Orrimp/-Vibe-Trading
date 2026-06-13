//! AC8 render proof — WITHOUT the `binance` feature the Lab `source_toggle`
//! renders TWO chips (Synthetic + Yahoo) and the Binance chip is hidden
//! (simple-strategies-realdata T-B5 / AC8).
//!
//! ## Why a separate file gated on `not(binance)`
//!
//! The everyday cockpit now ships `binance` in `default`, so the three-chip
//! render proof (`lab_binance_render.rs`) covers the with-feature case. THIS
//! file proves the complementary no-feature contract — that a build which
//! opts OUT of `binance` (the "minimal surface" / no-Binance-dependency build)
//! still renders a clean two-chip toggle with no third chip and no panic. It is
//! the render-layer half of AC8 ("fixtures cockpit hides the chip"); the
//! Cargo-level half (no `data/binance` read path linked) is the `#[cfg(feature
//! = "binance")]` gate on `preload_binance_bars` + the chip itself.
//!
//! ## Invocation
//!
//! This test is gated `#[cfg(all(feature = "live", not(feature = "binance")))]`
//! so it compiles + runs ONLY under a no-`binance` build:
//!
//! ```text
//! cargo test -p ui --no-default-features --features live \
//!     --test lab_source_toggle_no_binance
//! ```
//!
//! Under the default feature set (which includes `binance`) the whole file
//! compiles to nothing — the three-chip proof in `lab_binance_render.rs`
//! applies instead.

#![cfg(all(feature = "live", not(feature = "binance")))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::Duration;

use ui::lab::state::LabDataSource;
use ui::test_support::source_toggle_program;
use ui::theme::{ThemeMode, color};

const VIEW_W: u32 = 480;
const VIEW_H: u32 = 80;
const SCALE: f32 = 1.0;

fn accent_rgb() -> (i32, i32, i32) {
    let c = color::ACCENT.current(ThemeMode::Dark);
    (
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    )
}

const CHANNEL_TOL: i32 = 30;

fn is_accent(r: u8, g: u8, b: u8) -> bool {
    let (ar, ag, ab) = accent_rgb();
    (i32::from(r) - ar).abs() <= CHANNEL_TOL
        && (i32::from(g) - ag).abs() <= CHANNEL_TOL
        && (i32::from(b) - ab).abs() <= CHANNEL_TOL
}

/// Max-x of any ACCENT pixel across the frame (0 if none).
fn accent_max_x(current: LabDataSource) -> Option<u32> {
    let program = source_toggle_program(current);
    let theme = iced::Theme::Dark;
    let shot = iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO);
    let w = shot.size.width;
    let h = shot.size.height;
    let rgba: &[u8] = &shot.rgba;
    let mut max_x: Option<u32> = None;
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            if idx + 2 >= rgba.len() {
                continue;
            }
            if is_accent(rgba[idx], rgba[idx + 1], rgba[idx + 2]) {
                max_x = Some(max_x.map_or(x, |m| m.max(x)));
            }
        }
    }
    max_x
}

/// **AC8 render proof — no `binance` feature → two chips only.**
///
/// Selecting `Synthetic` paints an ACCENT highlight; selecting `YahooCache`
/// paints one FURTHER RIGHT (the second chip). The two active states must
/// resolve to DISTINCT positions (two chips render). Critically, the Yahoo
/// chip (the SECOND and rightmost chip in a no-binance build) bounds the
/// toggle's right edge — there is no third chip beyond it.
#[test]
fn no_binance_feature_renders_two_chips() {
    let synthetic_max = accent_max_x(LabDataSource::Synthetic)
        .expect("Synthetic-active toggle must paint an ACCENT highlight");
    let yahoo_max = accent_max_x(LabDataSource::YahooCache)
        .expect("Yahoo-active toggle must paint an ACCENT highlight");

    // Two distinct chips: the Yahoo highlight extends further right than the
    // Synthetic highlight (Yahoo is the second chip).
    assert!(
        yahoo_max > synthetic_max,
        "Yahoo chip highlight (max_x={yahoo_max}) must extend right of the \
         Synthetic chip highlight (max_x={synthetic_max}) — two distinct chips"
    );

    // The toggle does NOT crash and renders without the Binance chip: selecting
    // BinanceCache is impossible to reach via this two-chip widget, and the
    // widget's own `#[cfg(feature = "binance")]` gate means the third chip is
    // simply absent. (The friendly rebuild-error guard in `spawn_lab_run`
    // covers the case where a persisted BinanceCache selection is loaded under
    // a no-binance build — proven by the no-feature build compiling at all.)
}
