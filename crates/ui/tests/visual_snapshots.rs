//! Charts-screen + Phase D+ + Phase E + Phase F visual snapshots.
//!
//! ## Viewport matrix (ui-test-harness-viewport-matrix v0.1.0)
//!
//! Every fixture that was previously snapshot-only at the `typical`
//! viewport (1920×1080) now runs at all three operator-locked slots:
//!
//! | Slot     | Logical viewport | scale_factor | Physical pixels       |
//! |----------|------------------|--------------|-----------------------|
//! | floor    | 1280 × 720       | 1.0          | 1280 × 720            |
//! | typical  | 1920 × 1080      | 1.0          | 1920 × 1080           |
//! | operator | 3360 × 1890      | 2.0          | 6720 × 3780           |
//!
//! Each fixture contributes three discrete `#[test] fn`s named
//! `<fixture_name>__<slot_name>` so a CI failure is immediately
//! identifiable by test name.
//!
//! ## Charts triple (bootstrap — unchanged)
//!
//! `charts_screen_dark_floor` / `charts_screen_dark_typical` /
//! `charts_screen_dark_operator` are the bootstrap v0.1.0 baselines and
//! are NOT modified by this brief (R-NR.1 + R4 carry-forward).
//!
//! ## Opt-outs (D-VPM-4)
//!
//! - `visual_diff_helper_writes_diff_png_on_mismatch` (V9 self-test):
//!   no viewport, no expansion.
//!
//! ## First-run semantics
//!
//! On a missing baseline the `viewport_matrix::snapshot_widget_at_slot`
//! helper auto-writes the actual PNG as the new baseline and returns
//! `Ok(())`. Operator visually reviews, then commits.
//!
//! ## Determinism
//!
//! All fixtures seed fixed `Timestamp`s only and pass `Duration::ZERO` to
//! `iced_test::screenshot`. Two consecutive runs MUST produce zero
//! `target/visual-diff/` PNGs and zero `git status` changes.
//!
//! ## Baseline file convention
//!
//! ```text
//! crates/ui/tests/visual-baselines/<fixture_name>__<slot_name>.png
//! ```
//! (double-underscore, no theme infix for Phase D+/E/F fixtures)

// cockpit-cross-platform ADR-0057 D2: visual baselines are macOS-canonical.
// On Linux/Windows cosmic-text resolves body text via PlatformFallback against
// the per-OS system font DB, producing different glyph rasterization — these
// tests would not match the 56 macOS-captured PNGs. Gate the entire file to
// compile only on macOS; on Linux/Windows the file compiles to nothing (tests
// are skipped, never re-baselined). CI needs no --skip filter — the source gate
// IS the filter. See ADR-0057 D2 and docs/runbooks/cockpit-cross-platform.md.
#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used)]
// The snapshot fn names use double-underscore separators that match the
// baseline PNG filenames exactly. Suppressing the lint is the
// lowest-noise approach — renaming would de-sync fn names from baselines.
#![allow(non_snake_case)]

use std::time::Duration;

#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::charts_screen_with_hovered_marker;
use fixtures::viewport_matrix;
use fixtures::visual_diff::matches_screenshot;
use ui::test_support::program_from_cockpit;

// ─── Charts triple (bootstrap — three existing slots, NO expansion) ──────────
//
// These are the bootstrap v0.1.0 baselines. They use the bootstrap-era
// `run_slot` helper below (unchanged) and the `SLOTS` const defined
// there. They stay byte-identical per R-NR.1 + R4.

/// Slot → (logical width, logical height, scale_factor) — operator-
/// locked Q10. Adding a fourth slot is one row plus one `#[test] fn`.
///
/// - `floor`: min_size — the 1280×720 Q10 floor.
/// - `typical`: T3022 default — 1920×1080 desktop.
/// - `operator`: actual hardware — 3360×1890 logical at 2.0x scale
///   (6720×3780 physical, ≈ 76 MB rgba).
const CHARTS_SLOTS: &[(&str, (u32, u32), f32)] = &[
    ("floor", (1280, 720), 1.0),
    ("typical", (1920, 1080), 1.0),
    ("operator", (3360, 1890), 2.0),
];

/// Process-wide UTC-forcing initialiser — called once regardless of how
/// many tests run in parallel. `std::sync::Once` makes the call
/// idempotent and the underlying `SeqCst` atomic makes it thread-safe.
///
/// Replaces the old `unsafe { std::env::set_var(CHART_FORCE_UTC_ENV, "1") }`
/// pattern which was the root cause of the intermittent
/// `charts_screen_dark_*` failures (2026-06-13 / 2026-06-15): parallel
/// threads racing on `set_var` / `var_os` constitute a data race on the
/// process environment — `set_var` is `unsafe` in edition 2024 precisely
/// for this reason.
static INIT_UTC: std::sync::Once = std::sync::Once::new();
fn force_utc_once() {
    INIT_UTC.call_once(ui::force_chart_utc_for_tests);
}

/// Drive `iced_test::screenshot` for `slot_name`, then route the
/// resulting `iced::window::Screenshot` through the
/// `matches_screenshot` helper. Panics with a multi-line cite-the-
/// paths message on mismatch — see `fixtures::visual_diff` for the
/// failure forensic flow.
fn run_slot(slot_name: &str) {
    // v1.11 chart-x-axis-local-time: integration tests link against
    // the library compiled WITHOUT `cfg(test)`, so the `cfg(test)`
    // UTC override in `widgets::chart::local_offset_or_utc` does not
    // fire here.  The atomic flag set below preserves snapshot
    // determinism across host time zones and is thread-safe — see
    // `ui::force_chart_utc_for_tests` for the full contract.
    force_utc_once();

    let (_, (w, h), scale) = CHARTS_SLOTS
        .iter()
        .find(|(s, _, _)| *s == slot_name)
        .copied()
        .unwrap_or_else(|| panic!("unknown CHARTS_SLOTS row: {slot_name}"));

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

// ─── Trail / Live — Phase D+ (viewport matrix expansion) ─────────────────────
//
// Three fixtures previously snapshot only at the `typical` viewport.
// Now expanded to all three slots via `viewport_matrix::snapshot_widget_at_slot`.
//
// Existing typical-slot baseline renamed:
//   trail__steady_state.png            → trail__steady_state__typical.png
//   trail__side_drawer_open.png        → trail__side_drawer_open__typical.png
//   live__recent_activity_with_chevron.png → live__recent_activity_with_chevron__typical.png
// (single rename, zero byte change — the committed PNG is the new __typical member)

#[test]
fn trail__steady_state__floor() {
    viewport_matrix::snapshot_widget_at_slot("trail__steady_state", "floor", None, || {
        program_from_cockpit(fixtures::trail_steady_state_cockpit())
    });
}

#[test]
fn trail__steady_state__typical() {
    viewport_matrix::snapshot_widget_at_slot("trail__steady_state", "typical", None, || {
        program_from_cockpit(fixtures::trail_steady_state_cockpit())
    });
}

#[test]
fn trail__steady_state__operator() {
    viewport_matrix::snapshot_widget_at_slot("trail__steady_state", "operator", None, || {
        program_from_cockpit(fixtures::trail_steady_state_cockpit())
    });
}

#[test]
fn trail__side_drawer_open__floor() {
    viewport_matrix::snapshot_widget_at_slot("trail__side_drawer_open", "floor", None, || {
        program_from_cockpit(fixtures::trail_side_drawer_open_cockpit())
    });
}

#[test]
fn trail__side_drawer_open__typical() {
    viewport_matrix::snapshot_widget_at_slot("trail__side_drawer_open", "typical", None, || {
        program_from_cockpit(fixtures::trail_side_drawer_open_cockpit())
    });
}

#[test]
fn trail__side_drawer_open__operator() {
    viewport_matrix::snapshot_widget_at_slot("trail__side_drawer_open", "operator", None, || {
        program_from_cockpit(fixtures::trail_side_drawer_open_cockpit())
    });
}

#[test]
fn live__recent_activity_with_chevron__floor() {
    viewport_matrix::snapshot_widget_at_slot(
        "live__recent_activity_with_chevron",
        "floor",
        None,
        || program_from_cockpit(fixtures::live_recent_activity_with_chevron_cockpit()),
    );
}

#[test]
fn live__recent_activity_with_chevron__typical() {
    viewport_matrix::snapshot_widget_at_slot(
        "live__recent_activity_with_chevron",
        "typical",
        None,
        || program_from_cockpit(fixtures::live_recent_activity_with_chevron_cockpit()),
    );
}

#[test]
fn live__recent_activity_with_chevron__operator() {
    viewport_matrix::snapshot_widget_at_slot(
        "live__recent_activity_with_chevron",
        "operator",
        None,
        || program_from_cockpit(fixtures::live_recent_activity_with_chevron_cockpit()),
    );
}

// ─── Compare — Phase E (viewport matrix expansion) ───────────────────────────
//
// Four fixtures previously snapshot only at the `typical` viewport.
// Now expanded to all three slots.
//
// Existing typical-slot baseline renamed:
//   compare__cold_boot_all_empty.png       → compare__cold_boot_all_empty__typical.png
//   compare__steady_state_populated.png    → compare__steady_state_populated__typical.png
//   compare__empty_cell_run_affordance.png → compare__empty_cell_run_affordance__typical.png
//   compare__column_header_hover.png       → compare__column_header_hover__typical.png

#[test]
fn compare__cold_boot_all_empty__floor() {
    viewport_matrix::snapshot_widget_at_slot("compare__cold_boot_all_empty", "floor", None, || {
        program_from_cockpit(fixtures::compare__cold_boot_all_empty_cockpit())
    });
}

#[test]
fn compare__cold_boot_all_empty__typical() {
    viewport_matrix::snapshot_widget_at_slot(
        "compare__cold_boot_all_empty",
        "typical",
        None,
        || program_from_cockpit(fixtures::compare__cold_boot_all_empty_cockpit()),
    );
}

#[test]
fn compare__cold_boot_all_empty__operator() {
    viewport_matrix::snapshot_widget_at_slot(
        "compare__cold_boot_all_empty",
        "operator",
        None,
        || program_from_cockpit(fixtures::compare__cold_boot_all_empty_cockpit()),
    );
}

#[test]
fn compare__steady_state_populated__floor() {
    viewport_matrix::snapshot_widget_at_slot(
        "compare__steady_state_populated",
        "floor",
        None,
        || program_from_cockpit(fixtures::compare__steady_state_populated_cockpit()),
    );
}

#[test]
fn compare__steady_state_populated__typical() {
    viewport_matrix::snapshot_widget_at_slot(
        "compare__steady_state_populated",
        "typical",
        None,
        || program_from_cockpit(fixtures::compare__steady_state_populated_cockpit()),
    );
}

#[test]
fn compare__steady_state_populated__operator() {
    viewport_matrix::snapshot_widget_at_slot(
        "compare__steady_state_populated",
        "operator",
        None,
        || program_from_cockpit(fixtures::compare__steady_state_populated_cockpit()),
    );
}

#[test]
fn compare__empty_cell_run_affordance__floor() {
    viewport_matrix::snapshot_widget_at_slot(
        "compare__empty_cell_run_affordance",
        "floor",
        None,
        || program_from_cockpit(fixtures::compare__empty_cell_run_affordance_cockpit()),
    );
}

#[test]
fn compare__empty_cell_run_affordance__typical() {
    viewport_matrix::snapshot_widget_at_slot(
        "compare__empty_cell_run_affordance",
        "typical",
        None,
        || program_from_cockpit(fixtures::compare__empty_cell_run_affordance_cockpit()),
    );
}

#[test]
fn compare__empty_cell_run_affordance__operator() {
    viewport_matrix::snapshot_widget_at_slot(
        "compare__empty_cell_run_affordance",
        "operator",
        None,
        || program_from_cockpit(fixtures::compare__empty_cell_run_affordance_cockpit()),
    );
}

#[test]
fn compare__column_header_hover__floor() {
    viewport_matrix::snapshot_widget_at_slot("compare__column_header_hover", "floor", None, || {
        program_from_cockpit(fixtures::compare__column_header_hover_cockpit())
    });
}

#[test]
fn compare__column_header_hover__typical() {
    viewport_matrix::snapshot_widget_at_slot(
        "compare__column_header_hover",
        "typical",
        None,
        || program_from_cockpit(fixtures::compare__column_header_hover_cockpit()),
    );
}

#[test]
fn compare__column_header_hover__operator() {
    viewport_matrix::snapshot_widget_at_slot(
        "compare__column_header_hover",
        "operator",
        None,
        || program_from_cockpit(fixtures::compare__column_header_hover_cockpit()),
    );
}

// ─── Phase F — Memory / Models / Assistant (viewport matrix expansion) ───────
//
// Eight fixtures previously snapshot only at the `typical` viewport.
// Now expanded to all three slots.
//
// Existing typical-slot baselines renamed (single rename, zero byte change):
//   memory__cold_boot_empty.png                                    → memory__cold_boot_empty__typical.png
//   memory__steady_state_5_cards.png                               → memory__steady_state_5_cards__typical.png
//   memory__drawer_open_on_card_click.png                          → memory__drawer_open_on_card_click__typical.png
//   models__cold_boot_no_checkpoints.png                           → models__cold_boot_no_checkpoints__typical.png
//   models__steady_state_2_checkpoints.png                         → models__steady_state_2_checkpoints__typical.png
//   assistant_slot__open_stub.png                                  → assistant_slot__open_stub__typical.png
//   assistant_slot__llm_forecaster_disabled__placeholder.png       → assistant_slot__llm_forecaster_disabled__placeholder__typical.png
//   assistant_slot__llm_forecaster_active__most_recent_trace.png   → assistant_slot__llm_forecaster_active__most_recent_trace__typical.png

#[test]
fn memory__cold_boot_empty__floor() {
    viewport_matrix::snapshot_widget_at_slot("memory__cold_boot_empty", "floor", None, || {
        program_from_cockpit(fixtures::memory__cold_boot_empty_cockpit())
    });
}

#[test]
fn memory__cold_boot_empty__typical() {
    viewport_matrix::snapshot_widget_at_slot("memory__cold_boot_empty", "typical", None, || {
        program_from_cockpit(fixtures::memory__cold_boot_empty_cockpit())
    });
}

#[test]
fn memory__cold_boot_empty__operator() {
    viewport_matrix::snapshot_widget_at_slot("memory__cold_boot_empty", "operator", None, || {
        program_from_cockpit(fixtures::memory__cold_boot_empty_cockpit())
    });
}

#[test]
fn memory__steady_state_5_cards__floor() {
    viewport_matrix::snapshot_widget_at_slot("memory__steady_state_5_cards", "floor", None, || {
        program_from_cockpit(fixtures::memory__steady_state_5_cards_cockpit())
    });
}

#[test]
fn memory__steady_state_5_cards__typical() {
    viewport_matrix::snapshot_widget_at_slot(
        "memory__steady_state_5_cards",
        "typical",
        None,
        || program_from_cockpit(fixtures::memory__steady_state_5_cards_cockpit()),
    );
}

#[test]
fn memory__steady_state_5_cards__operator() {
    viewport_matrix::snapshot_widget_at_slot(
        "memory__steady_state_5_cards",
        "operator",
        None,
        || program_from_cockpit(fixtures::memory__steady_state_5_cards_cockpit()),
    );
}

#[test]
fn memory__drawer_open_on_card_click__floor() {
    viewport_matrix::snapshot_widget_at_slot(
        "memory__drawer_open_on_card_click",
        "floor",
        None,
        || program_from_cockpit(fixtures::memory__drawer_open_on_card_click_cockpit()),
    );
}

#[test]
fn memory__drawer_open_on_card_click__typical() {
    viewport_matrix::snapshot_widget_at_slot(
        "memory__drawer_open_on_card_click",
        "typical",
        None,
        || program_from_cockpit(fixtures::memory__drawer_open_on_card_click_cockpit()),
    );
}

#[test]
fn memory__drawer_open_on_card_click__operator() {
    viewport_matrix::snapshot_widget_at_slot(
        "memory__drawer_open_on_card_click",
        "operator",
        None,
        || program_from_cockpit(fixtures::memory__drawer_open_on_card_click_cockpit()),
    );
}

#[test]
fn models__cold_boot_no_checkpoints__floor() {
    viewport_matrix::snapshot_widget_at_slot(
        "models__cold_boot_no_checkpoints",
        "floor",
        None,
        || program_from_cockpit(fixtures::models__cold_boot_no_checkpoints_cockpit()),
    );
}

#[test]
fn models__cold_boot_no_checkpoints__typical() {
    viewport_matrix::snapshot_widget_at_slot(
        "models__cold_boot_no_checkpoints",
        "typical",
        None,
        || program_from_cockpit(fixtures::models__cold_boot_no_checkpoints_cockpit()),
    );
}

#[test]
fn models__cold_boot_no_checkpoints__operator() {
    viewport_matrix::snapshot_widget_at_slot(
        "models__cold_boot_no_checkpoints",
        "operator",
        None,
        || program_from_cockpit(fixtures::models__cold_boot_no_checkpoints_cockpit()),
    );
}

#[test]
fn models__steady_state_2_checkpoints__floor() {
    viewport_matrix::snapshot_widget_at_slot(
        "models__steady_state_2_checkpoints",
        "floor",
        None,
        || program_from_cockpit(fixtures::models__steady_state_2_checkpoints_cockpit()),
    );
}

#[test]
fn models__steady_state_2_checkpoints__typical() {
    viewport_matrix::snapshot_widget_at_slot(
        "models__steady_state_2_checkpoints",
        "typical",
        None,
        || program_from_cockpit(fixtures::models__steady_state_2_checkpoints_cockpit()),
    );
}

#[test]
fn models__steady_state_2_checkpoints__operator() {
    viewport_matrix::snapshot_widget_at_slot(
        "models__steady_state_2_checkpoints",
        "operator",
        None,
        || program_from_cockpit(fixtures::models__steady_state_2_checkpoints_cockpit()),
    );
}

#[test]
fn assistant_slot__open_stub__floor() {
    viewport_matrix::snapshot_widget_at_slot("assistant_slot__open_stub", "floor", None, || {
        program_from_cockpit(fixtures::assistant_slot__open_stub_cockpit())
    });
}

#[test]
fn assistant_slot__open_stub__typical() {
    viewport_matrix::snapshot_widget_at_slot("assistant_slot__open_stub", "typical", None, || {
        program_from_cockpit(fixtures::assistant_slot__open_stub_cockpit())
    });
}

#[test]
fn assistant_slot__open_stub__operator() {
    viewport_matrix::snapshot_widget_at_slot("assistant_slot__open_stub", "operator", None, || {
        program_from_cockpit(fixtures::assistant_slot__open_stub_cockpit())
    });
}

#[test]
fn assistant_slot__llm_forecaster_disabled__placeholder__floor() {
    viewport_matrix::snapshot_widget_at_slot(
        "assistant_slot__llm_forecaster_disabled__placeholder",
        "floor",
        None,
        || {
            program_from_cockpit(
                fixtures::assistant_slot__llm_forecaster_disabled__placeholder_cockpit(),
            )
        },
    );
}

#[test]
fn assistant_slot__llm_forecaster_disabled__placeholder__typical() {
    viewport_matrix::snapshot_widget_at_slot(
        "assistant_slot__llm_forecaster_disabled__placeholder",
        "typical",
        None,
        || {
            program_from_cockpit(
                fixtures::assistant_slot__llm_forecaster_disabled__placeholder_cockpit(),
            )
        },
    );
}

#[test]
fn assistant_slot__llm_forecaster_disabled__placeholder__operator() {
    viewport_matrix::snapshot_widget_at_slot(
        "assistant_slot__llm_forecaster_disabled__placeholder",
        "operator",
        None,
        || {
            program_from_cockpit(
                fixtures::assistant_slot__llm_forecaster_disabled__placeholder_cockpit(),
            )
        },
    );
}

#[test]
fn assistant_slot__llm_forecaster_active__most_recent_trace__floor() {
    viewport_matrix::snapshot_widget_at_slot(
        "assistant_slot__llm_forecaster_active__most_recent_trace",
        "floor",
        None,
        || {
            program_from_cockpit(
                fixtures::assistant_slot__llm_forecaster_active__most_recent_trace_cockpit(),
            )
        },
    );
}

#[test]
fn assistant_slot__llm_forecaster_active__most_recent_trace__typical() {
    viewport_matrix::snapshot_widget_at_slot(
        "assistant_slot__llm_forecaster_active__most_recent_trace",
        "typical",
        None,
        || {
            program_from_cockpit(
                fixtures::assistant_slot__llm_forecaster_active__most_recent_trace_cockpit(),
            )
        },
    );
}

#[test]
fn assistant_slot__llm_forecaster_active__most_recent_trace__operator() {
    viewport_matrix::snapshot_widget_at_slot(
        "assistant_slot__llm_forecaster_active__most_recent_trace",
        "operator",
        None,
        || {
            program_from_cockpit(
                fixtures::assistant_slot__llm_forecaster_active__most_recent_trace_cockpit(),
            )
        },
    );
}

// ─── V9 — Opt-out (D-VPM-4) ──────────────────────────────────────────────────
//
// This is a helper self-test that drives the visual-diff helper with
// synthetic 8×8 RGB buffers (no fixture, no viewport). Not a screenshot
// test in the matrix sense — no expansion.

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
