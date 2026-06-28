//! Viewport-matrix helper — ui-test-harness-viewport-matrix v0.1.0.
//!
//! Exposes the operator-locked three-slot SLOTS constant (D-VPM-1) and
//! two entry-point functions (D-VPM-2):
//!
//! - `snapshot_widget_at_slot` — render one fixture at one slot and
//!   byte-compare against the committed baseline PNG.
//! - `snapshot_widget_at_viewports` — fan-out: call
//!   `snapshot_widget_at_slot` for every entry in SLOTS.
//!
//! ## Baseline path convention (D-VPM-5)
//!
//! Top-level fixtures:
//! ```text
//! crates/ui/tests/visual-baselines/<fixture_name>__<slot_name>.png
//! ```
//!
//! Nested (e.g. render_snapshots):
//! ```text
//! crates/ui/tests/visual-baselines/<subdir>/<fixture_name>__<slot_name>.png
//! ```
//!
//! `render_snapshots.rs` panel names keep the `_dark_` theme-infix convention
//! already used by the bootstrap Charts triple; the caller passes the full
//! `fixture_name` (including the `_dark_` segment) so this helper stays
//! convention-agnostic.
//!
//! ## First-run semantics
//!
//! On a missing baseline the underlying `visual_diff::matches_screenshot`
//! auto-writes the actual PNG as the new baseline and returns `Ok(())`.
//! Subsequent runs byte-compare; any mismatch panics with the multi-line
//! path-triple message and emits an HTML artifact via the sibling
//! visual-fail-html-reporter helper.
//!
//! ## Determinism
//!
//! Every call to `snapshot_widget_at_slot` sets `CHART_FORCE_UTC_ENV`
//! before invoking `iced_test::screenshot`, mirroring the contract in
//! `visual_snapshots.rs::run_slot` lines 96-99. The fixture builder
//! closure is called fresh per slot so the program is never shared
//! across renders (avoids hidden mutable-state bugs).

#![allow(clippy::expect_used, clippy::unwrap_used, dead_code)]

use std::time::Duration;

use super::visual_diff::matches_screenshot;

/// Operator-locked viewport slot table (D-VPM-1, inheriting from
/// `ui-test-harness-bootstrap` Q10 operator-lock verbatim).
///
/// Tuple shape: `(slot_name, (logical_w, logical_h), scale_factor)`.
///
/// MUST stay in sync with `spec/v1/ui-test-harness-viewport-matrix/feature.md
/// § Design D-VPM-1`.
pub const SLOTS: &[(&str, (u32, u32), f32)] = &[
    ("floor", (1280, 720), 1.0),
    ("typical", (1920, 1080), 1.0),
    ("operator", (3360, 1890), 2.0),
];

/// Look up a single slot row by name.
///
/// Returns `((logical_w, logical_h), scale_factor)`.
///
/// Panics if `slot_name` is not in `SLOTS` — the helper is test-only
/// and a typo on the call site is always a test-author bug.
pub fn slot(slot_name: &str) -> ((u32, u32), f32) {
    SLOTS
        .iter()
        .find(|(s, _, _)| *s == slot_name)
        .map(|(_, dims, scale)| (*dims, *scale))
        .unwrap_or_else(|| panic!("unknown SLOTS row: {slot_name}"))
}

/// Drive `iced_test::screenshot` for `slot_name` against the program
/// produced by `build_program`, then route the resulting `Screenshot`
/// through `fixtures::visual_diff::matches_screenshot`.
///
/// # Arguments
///
/// - `fixture_name` — base filename component (e.g. `"trail__steady_state"`).
/// - `slot_name` — one of `"floor"`, `"typical"`, `"operator"`.
/// - `baseline_subdir` — optional subdirectory under `visual-baselines/`
///   (e.g. `Some("render_snapshots")`); `None` means top-level.
/// - `build_program` — closure that constructs a type implementing
///   `iced_test::program::Program`. Called once per invocation so slots
///   are independent (each gets a fresh program instance).
///
/// # Baseline path
///
/// - top-level: `crates/ui/tests/visual-baselines/<fixture_name>__<slot_name>.png`
/// - nested:    `crates/ui/tests/visual-baselines/<subdir>/<fixture_name>__<slot_name>.png`
///
/// # Panics
///
/// Panics with a multi-line operator-friendly message when the baseline
/// and actual PNGs differ (after writing diff artifacts to
/// `target/visual-diff/`).
/// Process-wide UTC-forcing initialiser for the viewport-matrix helper.
///
/// Uses `std::sync::Once` so parallel test threads calling
/// `snapshot_widget_at_slot` concurrently only execute the inner
/// `force_chart_utc_for_tests()` store once, yet every thread is
/// guaranteed to see the flag set before proceeding.  This replaces the
/// old `unsafe { std::env::set_var(…) }` call which was NOT thread-safe
/// (root cause of the 2026-06-13 / 2026-06-15 flaky failures).
static INIT_UTC: std::sync::Once = std::sync::Once::new();

pub fn snapshot_widget_at_slot<P, B>(
    fixture_name: &str,
    slot_name: &str,
    baseline_subdir: Option<&str>,
    build_program: B,
) where
    P: iced_test::program::Program<Theme = iced::Theme> + 'static,
    B: FnOnce() -> P,
{
    // v1.11 chart-x-axis-local-time: integration tests link against the
    // library compiled WITHOUT `cfg(test)`, so the `cfg(test)` UTC
    // override in `widgets::chart::local_offset_or_utc` does not fire
    // here.  The atomic flag ensures UTC rendering without a data race —
    // safe to call from parallel test threads.  See
    // `ui::force_chart_utc_for_tests` for the full contract.
    INIT_UTC.call_once(ui::force_chart_utc_for_tests);

    let ((w, h), scale) = slot(slot_name);

    let program = build_program();
    let theme = iced::Theme::Dark;

    let screenshot = iced_test::screenshot(&program, &theme, (w, h), scale, Duration::ZERO);

    // Resolve baseline path under CARGO_MANIFEST_DIR (set to crates/ui/
    // by Cargo for integration tests). Defence-in-depth against CWD
    // convention shifts.
    let baseline = match baseline_subdir {
        Some(subdir) => format!(
            "{}/tests/visual-baselines/{subdir}/{fixture_name}__{slot_name}.png",
            env!("CARGO_MANIFEST_DIR")
        ),
        None => format!(
            "{}/tests/visual-baselines/{fixture_name}__{slot_name}.png",
            env!("CARGO_MANIFEST_DIR")
        ),
    };

    let test_name = match baseline_subdir {
        Some(subdir) => format!("{subdir}__{fixture_name}__{slot_name}"),
        None => format!("{fixture_name}__{slot_name}"),
    };

    matches_screenshot(&screenshot, &baseline, &test_name).unwrap_or_else(|err| {
        panic!(
            "visual snapshot mismatch for fixture `{fixture_name}` slot `{slot_name}`:\n{err}\n\n\
             Review the baseline / actual / diff triple, then either:\n  \
             (a) accept the change: delete the baseline + rerun (helper auto-rewrites), or\n  \
             (b) reject the change: fix the producing widget code."
        )
    });
}

/// Fan-out helper: invoke `snapshot_widget_at_slot` for every entry in
/// `SLOTS`.
///
/// The closure `build_program` is re-invoked for each slot so each
/// render starts from a fresh program instance (no shared mutable state
/// across viewports).
///
/// # Typical usage
///
/// ```ignore
/// viewport_matrix::snapshot_widget_at_viewports(
///     "memory__cold_boot_empty",
///     None,
///     || ui::test_support::program_from_cockpit(fixtures::memory__cold_boot_empty_cockpit()),
/// );
/// ```
pub fn snapshot_widget_at_viewports<P, B>(
    fixture_name: &str,
    baseline_subdir: Option<&str>,
    build_program: B,
) where
    P: iced_test::program::Program<Theme = iced::Theme> + 'static,
    B: Fn() -> P,
{
    for (slot_name, _, _) in SLOTS {
        // Wrap in a closure that delegates to the Fn so the per-slot
        // call gets an FnOnce it can consume.
        snapshot_widget_at_slot::<P, _>(fixture_name, slot_name, baseline_subdir, || {
            build_program()
        });
    }
}
