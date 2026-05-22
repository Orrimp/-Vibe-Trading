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
// The Phase D+ snapshot fn names use double-underscore separators that match
// the baseline PNG filenames exactly (e.g. `trail__steady_state.png`).
// Suppressing the lint here is the lowest-noise approach — renaming would
// de-sync fn names from baselines and confuse the operator.
#![allow(non_snake_case)]

use std::time::Duration;

#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::charts_screen_with_hovered_marker;
use fixtures::visual_diff::matches_screenshot;
use ui::test_support::program_from_cockpit;

/// Snapshot slots for the three Phase D+ trail / live baselines
/// (Wave C — T-D-N12, T-D-N13, T-D-N14).  Each entry is a
/// `(fixture_name, logical_w, logical_h, scale)` tuple.  These all
/// use the `typical` viewport (1920×1080 @ 1.0x) — the T3022 default
/// that the operator daily-drives and that the trail-screen ship
/// decision was made against.
const TRAIL_SLOTS: &[(&str, u32, u32, f32)] = &[
    ("trail__steady_state", 1920, 1080, 1.0),
    ("trail__side_drawer_open", 1920, 1080, 1.0),
    ("live__recent_activity_with_chevron", 1920, 1080, 1.0),
];

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

/// Drive `iced_test::screenshot` for a Phase D+ trail/live snapshot slot
/// identified by `fixture_name`, then route through `matches_screenshot`.
///
/// `fixture_name` must be one of the keys in `TRAIL_SLOTS`.  Baseline PNGs
/// live at `crates/ui/tests/visual-baselines/<fixture_name>.png`.  On the
/// first run the baseline is auto-written; subsequent runs byte-compare.
fn run_trail_slot(fixture_name: &str) {
    unsafe { std::env::set_var(ui::strings::CHART_FORCE_UTC_ENV, "1") };

    let (_, w, h, scale) = TRAIL_SLOTS
        .iter()
        .find(|(s, _, _, _)| *s == fixture_name)
        .copied()
        .unwrap_or_else(|| panic!("unknown TRAIL_SLOTS key: {fixture_name}"));

    let cockpit = match fixture_name {
        "trail__steady_state" => fixtures::trail_steady_state_cockpit(),
        "trail__side_drawer_open" => fixtures::trail_side_drawer_open_cockpit(),
        "live__recent_activity_with_chevron" => {
            fixtures::live_recent_activity_with_chevron_cockpit()
        }
        other => panic!("no fixture builder for: {other}"),
    };

    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;

    let screenshot = iced_test::screenshot(&program, &theme, (w, h), scale, Duration::ZERO);

    let baseline = format!(
        "{}/tests/visual-baselines/{fixture_name}.png",
        env!("CARGO_MANIFEST_DIR")
    );

    matches_screenshot(&screenshot, &baseline, fixture_name).unwrap_or_else(|err| {
        panic!(
            "visual snapshot mismatch for `{fixture_name}`:\n{err}\n\n\
             Review the baseline / actual / diff triple, then either:\n  \
             (a) accept: delete baseline + rerun (auto-rewritten), or\n  \
             (b) reject: fix the producing widget code."
        )
    });
}

/// T-D-N12 — Trail screen in list mode (delegates byte-identically to
/// `screens::audit::view` per R2.2).  Baseline auto-written on first run.
#[test]
fn trail__steady_state() {
    run_trail_slot("trail__steady_state");
}

/// T-D-N13 — Trail screen in trail mode: Forecast-stage payload + side-
/// drawer open.  Exercises the full node stack + `trail_drawer::view`.
/// Baseline auto-written on first run.
#[test]
fn trail__side_drawer_open() {
    run_trail_slot("trail__side_drawer_open");
}

/// T-D-N14 — Live screen with 5-row recent-activity tape.  Exercises
/// `screens::live::view` with the universal chevron on every row (R5.1).
/// Baseline auto-written on first run.
#[test]
fn live__recent_activity_with_chevron() {
    run_trail_slot("live__recent_activity_with_chevron");
}

/// Snapshot slots for the Phase E compare baselines
/// (Wave D — T-D-N10, T-D-N11, T-D-N12, T-D-N13).  All use the
/// `typical` viewport (1920×1080 @ 1.0x) — the T3022 default.
const COMPARE_SLOTS: &[(&str, u32, u32, f32)] = &[
    ("compare__cold_boot_all_empty", 1920, 1080, 1.0),
    ("compare__steady_state_populated", 1920, 1080, 1.0),
    ("compare__empty_cell_run_affordance", 1920, 1080, 1.0),
    ("compare__column_header_hover", 1920, 1080, 1.0),
];

/// Drive `iced_test::screenshot` for a Phase E compare snapshot slot
/// identified by `fixture_name`, then route through `matches_screenshot`.
///
/// `fixture_name` must be one of the keys in `COMPARE_SLOTS`. Baseline
/// PNGs live at `crates/ui/tests/visual-baselines/<fixture_name>.png`.
/// On first run the baseline is auto-written; subsequent runs byte-compare.
fn run_compare_slot(fixture_name: &str) {
    unsafe { std::env::set_var(ui::strings::CHART_FORCE_UTC_ENV, "1") };

    let (_, w, h, scale) = COMPARE_SLOTS
        .iter()
        .find(|(s, _, _, _)| *s == fixture_name)
        .copied()
        .unwrap_or_else(|| panic!("unknown COMPARE_SLOTS key: {fixture_name}"));

    let cockpit = match fixture_name {
        "compare__cold_boot_all_empty" => fixtures::compare__cold_boot_all_empty_cockpit(),
        "compare__steady_state_populated" => fixtures::compare__steady_state_populated_cockpit(),
        "compare__empty_cell_run_affordance" => {
            fixtures::compare__empty_cell_run_affordance_cockpit()
        }
        "compare__column_header_hover" => fixtures::compare__column_header_hover_cockpit(),
        other => panic!("no fixture builder for: {other}"),
    };

    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;

    let screenshot = iced_test::screenshot(&program, &theme, (w, h), scale, Duration::ZERO);

    let baseline = format!(
        "{}/tests/visual-baselines/{fixture_name}.png",
        env!("CARGO_MANIFEST_DIR")
    );

    matches_screenshot(&screenshot, &baseline, fixture_name).unwrap_or_else(|err| {
        panic!(
            "visual snapshot mismatch for `{fixture_name}`:\n{err}\n\n\
             Review the baseline / actual / diff triple, then either:\n  \
             (a) accept: delete baseline + rerun (auto-rewritten), or\n  \
             (b) reject: fix the producing widget code."
        )
    });
}

/// T-D-N10 — Compare screen cold-boot: every legal cell shows the "Run"
/// affordance, every non-universe cell shows `—`. K7 subtitle absent.
/// Baseline auto-written on first run.
#[test]
fn compare__cold_boot_all_empty() {
    run_compare_slot("compare__cold_boot_all_empty");
}

/// T-D-N11 — Compare screen steady-state: all 24 populated cells filled
/// per the T-T1-2 census. K7 multi-symbol disclaimer subtitle visible.
/// Baseline auto-written on first run.
#[test]
fn compare__steady_state_populated() {
    run_compare_slot("compare__steady_state_populated");
}

/// T-D-N12 — Compare screen with 20 of 24 cells populated: 4 cells
/// show the "Run" affordance, exercising the `ACCENT_500` hairline
/// button path (R2.3). Baseline auto-written on first run.
#[test]
fn compare__empty_cell_run_affordance() {
    run_compare_slot("compare__empty_cell_run_affordance");
}

/// T-D-N13 — Compare screen column-header hover: column headers are
/// non-interactive (R2.4 v0.1.0). Snapshot confirms no hover tint.
/// Baseline auto-written on first run.
#[test]
fn compare__column_header_hover() {
    run_compare_slot("compare__column_header_hover");
}

// ── Phase F snapshots (ui-rethink-phase-f-memory-models-assistant Wave F T-D-N18) ──

/// Phase F snapshot slot table.
/// All use the `typical` viewport (1920×1080 @ 1.0x) — the T3022 default.
const PHASE_F_SLOTS: &[(&str, u32, u32, f32)] = &[
    ("memory__cold_boot_empty", 1920, 1080, 1.0),
    ("memory__steady_state_5_cards", 1920, 1080, 1.0),
    ("memory__drawer_open_on_card_click", 1920, 1080, 1.0),
    ("models__cold_boot_no_checkpoints", 1920, 1080, 1.0),
    ("models__steady_state_2_checkpoints", 1920, 1080, 1.0),
    ("assistant_slot__open_stub", 1920, 1080, 1.0),
    // v3-llm-forecaster Wave F (T-D-N(F5)) — two new baselines.
    // The `_disabled__placeholder` baseline asserts R9.3 byte-identity:
    // when the v3-llm-forecaster strategy is disabled (default), the
    // Assistant slot body renders the Phase F placeholder verbatim.
    // The `_active__most_recent_trace` baseline locks the new reasoning-
    // trace body composition (R9.2).
    (
        "assistant_slot__llm_forecaster_disabled__placeholder",
        1920,
        1080,
        1.0,
    ),
    (
        "assistant_slot__llm_forecaster_active__most_recent_trace",
        1920,
        1080,
        1.0,
    ),
];

/// Drive a Phase F snapshot slot.
fn run_phase_f_slot(fixture_name: &str) {
    unsafe { std::env::set_var(ui::strings::CHART_FORCE_UTC_ENV, "1") };

    let (_, w, h, scale) = PHASE_F_SLOTS
        .iter()
        .find(|(s, _, _, _)| *s == fixture_name)
        .copied()
        .unwrap_or_else(|| panic!("unknown PHASE_F_SLOTS key: {fixture_name}"));

    let cockpit = match fixture_name {
        "memory__cold_boot_empty" => fixtures::memory__cold_boot_empty_cockpit(),
        "memory__steady_state_5_cards" => fixtures::memory__steady_state_5_cards_cockpit(),
        "memory__drawer_open_on_card_click" => {
            fixtures::memory__drawer_open_on_card_click_cockpit()
        }
        "models__cold_boot_no_checkpoints" => fixtures::models__cold_boot_no_checkpoints_cockpit(),
        "models__steady_state_2_checkpoints" => {
            fixtures::models__steady_state_2_checkpoints_cockpit()
        }
        "assistant_slot__open_stub" => fixtures::assistant_slot__open_stub_cockpit(),
        "assistant_slot__llm_forecaster_disabled__placeholder" => {
            fixtures::assistant_slot__llm_forecaster_disabled__placeholder_cockpit()
        }
        "assistant_slot__llm_forecaster_active__most_recent_trace" => {
            fixtures::assistant_slot__llm_forecaster_active__most_recent_trace_cockpit()
        }
        other => panic!("no fixture builder for: {other}"),
    };

    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;

    let screenshot = iced_test::screenshot(&program, &theme, (w, h), scale, Duration::ZERO);

    let baseline = format!(
        "{}/tests/visual-baselines/{fixture_name}.png",
        env!("CARGO_MANIFEST_DIR")
    );

    matches_screenshot(&screenshot, &baseline, fixture_name).unwrap_or_else(|err| {
        panic!(
            "visual snapshot mismatch for `{fixture_name}`:\n{err}\n\n\
             Review the baseline / actual / diff triple, then either:\n  \
             (a) accept: delete baseline + rerun (auto-rewritten), or\n  \
             (b) reject: fix the producing widget code."
        )
    });
}

/// T-D-N18 (1/6) — Memory screen cold-boot: empty cache renders R1.4 placeholder.
/// Baseline auto-written on first run.
#[test]
fn memory__cold_boot_empty() {
    run_phase_f_slot("memory__cold_boot_empty");
}

/// T-D-N18 (2/6) — Memory screen steady-state: 5 lesson cards (mix of Win /
/// Loss / Scratch). List mode; drawer closed.
/// Baseline auto-written on first run.
#[test]
fn memory__steady_state_5_cards() {
    run_phase_f_slot("memory__steady_state_5_cards");
}

/// T-D-N18 (3/6) — Memory screen with side-drawer open on first card click.
/// Exercises Q5=(b) drawer path.
/// Baseline auto-written on first run.
#[test]
fn memory__drawer_open_on_card_click() {
    run_phase_f_slot("memory__drawer_open_on_card_click");
}

/// T-D-N18 (4/6) — Models screen cold-boot: no checkpoints loaded renders
/// Q3=(a) empty-state placeholder.
/// Baseline auto-written on first run.
#[test]
fn models__cold_boot_no_checkpoints() {
    run_phase_f_slot("models__cold_boot_no_checkpoints");
}

/// T-D-N18 (5/6) — Models screen steady-state: 2 TCN checkpoints loaded
/// (mirrors the live `tcn-bs1` + `tcn-bs2` on disk). Both render as `staged`
/// per Q7=(c).
/// Baseline auto-written on first run.
#[test]
fn models__steady_state_2_checkpoints() {
    run_phase_f_slot("models__steady_state_2_checkpoints");
}

/// T-D-N18 (6/6) — Assistant slot open stub: right-rail renders
/// `ASSISTANT_OFFLINE_TITLE` + `ASSISTANT_OFFLINE_BODY` at Phase 6 wake.
/// K6 Option A: `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0` governs the slot width.
/// Baseline auto-written on first run.
#[test]
fn assistant_slot__open_stub() {
    run_phase_f_slot("assistant_slot__open_stub");
}

/// v3-llm-forecaster Wave F (T-D-N(F5)) — R9.3 byte-identity guard.
///
/// When the v3-llm-forecaster strategy is *disabled* (the default per
/// `LlmForecasterConfig::default().enabled = false`), the cockpit_live
/// boot path leaves `assistant_state.mode = Offline` and the right-
/// rail renders the Phase F placeholder verbatim — byte-identical to
/// the `assistant_slot__open_stub` baseline. Baseline auto-written on
/// first run; subsequent runs assert byte-identity.
///
/// The dedicated baseline (vs reusing `assistant_slot__open_stub`)
/// makes the regression intent explicit: when the v3-llm-forecaster
/// view code mutates (say, adds a new `AssistantMode` variant), this
/// baseline catches any incidental drift on the default-disabled path
/// even when the original Phase F baseline stays green.
#[test]
fn assistant_slot__llm_forecaster_disabled__placeholder() {
    run_phase_f_slot("assistant_slot__llm_forecaster_disabled__placeholder");
}

/// v3-llm-forecaster Wave F (T-D-N(F5)) — active reasoning-trace body.
///
/// `AssistantMode::ReasoningTrace` with one populated `LlmForecastView`
/// in `last_forecast`, one prior forecast in `history`, and one
/// matching `LessonCardCard` hydrated in `memory_screen_state.cache`
/// (the cited-lessons section exercises both matched-card and
/// pending-card branches in a single baseline). Baseline auto-written
/// on first run.
#[test]
fn assistant_slot__llm_forecaster_active__most_recent_trace() {
    run_phase_f_slot("assistant_slot__llm_forecaster_active__most_recent_trace");
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
