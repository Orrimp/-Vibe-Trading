//! Charts-screen visual snapshots — ui-test-harness-bootstrap v0.1.
//!
//! Three discrete `#[test] fn`s — one per viewport slot (Q10
//! operator-locked) — each driving `iced_test::screenshot(...)` against
//! the test-only cockpit factory and comparing the resulting PNG bytes
//! against a baseline at
//! `crates/ui/tests/visual-baselines/charts_screen_dark_<slot>.png`.
//!
//! ## First-run semantics
//!
//! The visual-diff helper auto-writes the baseline on first run (when
//! it doesn't exist), so a fresh checkout produces:
//!
//! ```text
//! crates/ui/tests/visual-baselines/charts_screen_dark_floor.png    (1280 × 720)
//! crates/ui/tests/visual-baselines/charts_screen_dark_typical.png  (1920 × 1080)
//! crates/ui/tests/visual-baselines/charts_screen_dark_operator.png (6720 × 3780 — physical 2.0x)
//! ```
//!
//! These three PNGs ship committed. Subsequent runs byte-compare
//! against them; any mismatch writes a perceptual-diff PNG under
//! `target/visual-diff/` and fails the test with the path triple
//! cited in the panic message.
//!
//! ## Determinism (R4 / H1)
//!
//! The fixture path is clock-free (`charts_screen_with_hovered_marker`
//! seeds fixed `Timestamp`s only), the `local_offset_or_utc()`
//! override returns `UtcOffset::UTC` under `#[cfg(test)]`, and the
//! `Duration::ZERO` argument means no async tasks pump between paint
//! cycles. Two consecutive `cargo test -p ui --test visual_snapshots`
//! runs MUST produce zero diff bytes (T4032 / H1 falsifier — run by
//! the orchestrator, not the developer).
//!
//! ## V1 + V2 coverage
//!
//! - V1 — `cargo test -p ui --test visual_snapshots` exits 0 with the
//!   three slot-named `#[test] fn`s green.
//! - V2 — second consecutive run produces zero `target/visual-diff/`
//!   PNGs and zero `git status` modifications to
//!   `crates/ui/tests/visual-baselines/`. (Orchestrator-only check.)

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::Duration;

#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::charts_screen_with_hovered_marker;
use fixtures::visual_diff::matches_screenshot;
use ui::test_support::program_from_cockpit;

/// Slot → (logical width, logical height, scale_factor) — operator-
/// locked Q10. Adding a fourth slot is one row plus one `#[test] fn`.
///
/// - `floor`: min_size — the 1280×720 Q10 floor.
/// - `typical`: T3022 default — 1920×1080 desktop.
/// - `operator`: actual hardware — 3360×1890 logical at 2.0x scale
///   (6720×3780 physical, ≈ 76 MB rgba).
const SLOTS: &[(&str, (u32, u32), f32)] = &[
    ("floor", (1280, 720), 1.0),
    ("typical", (1920, 1080), 1.0),
    ("operator", (3360, 1890), 2.0),
];

/// Drive `iced_test::screenshot` for `slot_name`, then route the
/// resulting `iced::window::Screenshot` through the
/// `matches_screenshot` helper. Panics with a multi-line cite-the-
/// paths message on mismatch — see `fixtures::visual_diff` for the
/// failure forensic flow.
fn run_slot(slot_name: &str) {
    // v1.11 chart-x-axis-local-time: integration tests link against
    // the library compiled WITHOUT `cfg(test)`, so the `cfg(test)`
    // UTC override in `widgets::chart::local_offset_or_utc` does not
    // fire here. The env-var gate (`UI_CHART_FORCE_UTC`) preserves
    // snapshot determinism across host time zones — see the function's
    // doc comment for the full contract.
    // SAFETY: `set_var` is unsafe in edition 2024; this is a test-only
    // single-threaded init before iced_test::screenshot — no other
    // thread observes the env at this point.
    unsafe { std::env::set_var(ui::strings::CHART_FORCE_UTC_ENV, "1") };

    let (_, (w, h), scale) = SLOTS
        .iter()
        .find(|(s, _, _)| *s == slot_name)
        .copied()
        .unwrap_or_else(|| panic!("unknown SLOTS row: {slot_name}"));

    let cockpit = charts_screen_with_hovered_marker();
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;

    let screenshot = iced_test::screenshot(
        &program,
        &theme,
        (w, h),
        scale,
        // H2 — `Duration::ZERO` produces a fully-rendered frame for a
        // pure function-of-state cockpit (no async boot tasks). If
        // the orchestrator's H2 review finds a pre-data placeholder,
        // bump this to `Duration::from_millis(50..200)` per
        // feature.md's H2 falsifier.
        Duration::ZERO,
    );

    // Cargo runs integration tests with CWD = the package root
    // (`crates/ui/`), so the baseline path is relative to that.
    // `CARGO_MANIFEST_DIR` is set to the same dir at compile time —
    // we use it here for explicit defence-in-depth in case Cargo's
    // CWD convention ever shifts.
    let baseline = format!(
        "{}/tests/visual-baselines/charts_screen_dark_{slot_name}.png",
        env!("CARGO_MANIFEST_DIR")
    );
    let test_name = format!("charts_screen_dark_{slot_name}");

    matches_screenshot(&screenshot, &baseline, &test_name).unwrap_or_else(|err| {
        panic!(
            "visual snapshot mismatch for slot `{slot_name}`:\n{err}\n\n\
             Review the baseline / actual / diff triple, then either:\n  \
             (a) accept the change: delete the baseline + rerun (helper auto-rewrites), or\n  \
             (b) reject the change: fix the producing widget code."
        )
    });
}

/// Floor viewport — 1280 × 720 @ 1.0x. The Q10 floor — equivalent to
/// the cockpit's `min_size`. A failure here means the layout doesn't
/// survive the cockpit's smallest supported window.
#[test]
fn charts_screen_dark_floor() {
    run_slot("floor");
}

/// Typical viewport — 1920 × 1080 @ 1.0x. The T3022 default. A
/// failure here is the most-common regression mode (operators run at
/// this resolution by default).
#[test]
fn charts_screen_dark_typical() {
    run_slot("typical");
}

/// Operator viewport — 3360 × 1890 @ 2.0x (6720 × 3780 physical).
/// The actual cockpit hardware from chart-canvas-overhaul v1.10.0.
/// A failure here is the one that closes V15 of chart-canvas-overhaul
/// (per operator decision D4) — the regression the original cycle
/// missed because no test exercised this viewport.
#[test]
fn charts_screen_dark_operator() {
    run_slot("operator");
}

/// V9 — the perceptual-diff helper materialises a diff PNG on
/// mismatch (R6.1 — R6.4). Drives the helper with two known-
/// different `RgbImage` buffers (solid red vs. solid green 8x8) and
/// asserts:
///
/// 1. `matches_rgb_buffers` returns `Err(VisualDiffError::Mismatch)`.
/// 2. `target/visual-diff/visual_diff_helper_writes_diff_png_on_mismatch.png`
///    exists post-call.
///
/// This is the self-test the operator-locked Q8 lock requires — see
/// feature.md V9. Sandbox-safe: no cockpit launch, no screencapture.
#[test]
fn visual_diff_helper_writes_diff_png_on_mismatch() {
    use image::{ImageBuffer, Rgb, RgbImage};

    let test_name = "visual_diff_helper_writes_diff_png_on_mismatch";

    // Two 8x8 RGB buffers — baseline solid red, actual solid green.
    // image-compare's hybrid SSIM+RMS picks up the chrominance delta
    // and produces a non-uniform diff PNG.
    let baseline: RgbImage = ImageBuffer::from_pixel(8, 8, Rgb([255, 0, 0]));
    let actual: RgbImage = ImageBuffer::from_pixel(8, 8, Rgb([0, 255, 0]));

    // Use CARGO_TARGET_DIR if set, else the workspace's `target/`.
    let diff_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("target")
        })
        .join("visual-diff");
    let diff_path = diff_dir.join(format!("{test_name}.png"));

    // Best-effort cleanup so a prior run can't false-pass the
    // post-condition.
    let _ = std::fs::remove_file(&diff_path);

    let result = fixtures::visual_diff::matches_rgb_buffers(&baseline, &actual, test_name);
    assert!(
        matches!(
            result,
            Err(fixtures::visual_diff::VisualDiffError::Mismatch { .. })
        ),
        "two-color buffers must report Mismatch; got {result:?}"
    );
    assert!(
        diff_path.exists(),
        "diff PNG must exist at {} after Mismatch",
        diff_path.display()
    );
}
