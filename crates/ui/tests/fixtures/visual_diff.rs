//! Visual-diff helper — wraps a baseline-vs.-actual byte comparison
//! around an [`iced::window::Screenshot`] and emits a perceptual diff
//! PNG to `target/visual-diff/<test_name>.png` on mismatch.
//!
//! ## Why this helper exists
//!
//! The architect's brief (Q2 resolution) referenced
//! `iced_test::Snapshot::matches_image(path)` as the canonical
//! baseline-comparison call. The shipped `iced_test = "0.14.0"`
//! `screenshot(&program, &theme, viewport, scale_factor, duration)`
//! free function actually returns `iced::window::Screenshot` — the
//! `Snapshot` type lives behind the `Simulator` path which doesn't
//! accept a viewport+scale_factor pair. So we implement the
//! baseline-vs.-actual comparison directly against
//! `Screenshot { rgba, size, scale_factor }`:
//!
//! - No baseline on disk: a FAILURE, not a pass. `UI_BASELINE_CAPTURE=1` is
//!   what writes one — see [`baseline_action`]. Until 2026-11-09 a missing
//!   baseline was written silently and the test reported ok, so deleting a
//!   PNG — the documented way to accept a change — was also the way to make
//!   that gate unfalsifiable for a run (bug-log `#187`).
//! - Subsequent runs: byte-compare actual rgba against the baseline
//!   PNG's decoded rgba. On match: `Ok(())`. On mismatch: write
//!   `target/visual-diff/<test_name>.png` via
//!   `image_compare::rgb_hybrid_compare`, also persist
//!   `target/visual-diff/<test_name>-actual.png` so the operator can
//!   open all three (baseline / actual / diff) in Preview/Finder, and
//!   return `Err`.
//!
//! ## R6 contract
//!
//! Per feature.md R6 — `image-compare` is `[dev-dependencies]` only,
//! never reachable from production code. The diff PNG is forensic
//! only — the comparison still hard-fails on any byte mismatch.

#![allow(clippy::expect_used, clippy::unwrap_used, dead_code)]

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use image::{ImageBuffer, RgbImage, Rgba, RgbaImage};

// visual-fail-html-reporter v0.1.0 (T-VFH-D5) — wire-up at FAIL branches.
use super::visual_fail_html::{VisualFailContext, emit_visual_fail_html};

/// Where the diff PNGs land — sibling of `target/<profile>/` so the
/// operator can find them under one path regardless of build profile.
fn visual_diff_dir() -> PathBuf {
    // `CARGO_TARGET_DIR` honours the workspace's target override; fall
    // back to `target/` relative to the workspace root when unset.
    let target_root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Walk up from CARGO_MANIFEST_DIR to find the workspace
            // root's `target` dir. `crates/ui` → `..` → `..` lands at
            // workspace root.
            let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            manifest_dir.join("..").join("..").join("target")
        });
    target_root.join("visual-diff")
}

/// Compare a freshly-captured `Screenshot` against a baseline PNG.
///
/// `baseline_path` is relative to the workspace root or absolute. The actual
/// rgba is byte-compared against the decoded baseline; a mismatch writes the
/// perceptual-diff PNG via `image_compare::rgb_hybrid_compare` and returns
/// `Err(Mismatch)`. A baseline that is not on disk returns
/// `Err(MissingBaseline)` — unless `UI_BASELINE_CAPTURE=1` asked for a
/// capture, in which case the frame is written and `Ok(())` returned.
///
/// The function does NOT panic — it returns `Result` so the integration
/// test can choose to panic with `.expect(...)` (preserving an
/// operator-friendly multi-line failure message) or to assert in a
/// `catch_unwind` (for the V9 self-test).
/// Environment variable that turns a capture on.
///
/// Unset — the normal case, and CI — means a baseline that is not on disk is a
/// failure. Set to exactly `1`, every baseline the run touches is rewritten
/// from what rendered: that is the one-pass re-baseline ADR-0093 describes,
/// and it no longer requires deleting the files first.
pub const CAPTURE_ENV: &str = "UI_BASELINE_CAPTURE";

/// Does this raw environment value ask for a capture? Only an explicit `1`.
///
/// Deliberately narrow: `true`, `yes` and a bare empty value do NOT capture,
/// because a variable left set by a previous shell command must not silently
/// rewrite 56 baselines.
pub fn capture_requested(raw: Option<&OsStr>) -> bool {
    raw.and_then(OsStr::to_str) == Some("1")
}

/// What to do about the baseline file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselineAction {
    /// Byte-compare the render against the file on disk.
    Compare,
    /// Write the render to disk as the baseline.
    Capture,
    /// Nothing to compare against, and no capture was asked for.
    Missing,
}

/// The whole decision, as a pure function of its two inputs, so it can be
/// tested on both sides of both dimensions without touching a filesystem or a
/// process environment.
///
/// The `(false, false)` row is the one this function exists for: a render with
/// nothing to compare it against is not evidence of anything, and reporting it
/// as a pass is how a deleted baseline stayed invisible (bug-log `#187`).
pub fn baseline_action(baseline_exists: bool, capture_requested: bool) -> BaselineAction {
    match (baseline_exists, capture_requested) {
        (_, true) => BaselineAction::Capture,
        (true, false) => BaselineAction::Compare,
        (false, false) => BaselineAction::Missing,
    }
}

pub fn matches_screenshot(
    screenshot: &iced::window::Screenshot,
    baseline_path: &str,
    test_name: &str,
) -> Result<(), VisualDiffError> {
    matches_screenshot_with_capture(
        screenshot,
        baseline_path,
        test_name,
        capture_requested(std::env::var_os(CAPTURE_ENV).as_deref()),
    )
}

/// [`matches_screenshot`] with the capture decision passed in rather than read
/// from the environment — the form the self-tests drive, since a test that
/// mutated `UI_BASELINE_CAPTURE` would be mutating it for every other test
/// rendering in the same process.
pub fn matches_screenshot_with_capture(
    screenshot: &iced::window::Screenshot,
    baseline_path: &str,
    test_name: &str,
    capture: bool,
) -> Result<(), VisualDiffError> {
    let baseline = Path::new(baseline_path);

    // Convert rgba `Bytes` to a fully-owned `RgbaImage`. iced's
    // `Screenshot.size` is physical pixels — that's what we want to
    // persist (the scale_factor is encoded by the PNG dimensions).
    let width = screenshot.size.width;
    let height = screenshot.size.height;
    let rgba_vec: Vec<u8> = screenshot.rgba.to_vec();
    let expected_len = (width as usize) * (height as usize) * 4;
    if rgba_vec.len() != expected_len {
        return Err(VisualDiffError::Rgba {
            expected: expected_len,
            actual: rgba_vec.len(),
        });
    }

    let actual: RgbaImage = ImageBuffer::from_raw(width, height, rgba_vec)
        .ok_or(VisualDiffError::RgbaFromRaw { width, height })?;

    match baseline_action(baseline.exists(), capture) {
        BaselineAction::Capture => {
            if let Some(parent) = baseline.parent() {
                fs::create_dir_all(parent).map_err(VisualDiffError::Io)?;
            }
            actual.save(baseline).map_err(VisualDiffError::Image)?;
            return Ok(());
        }
        BaselineAction::Missing => {
            return Err(VisualDiffError::MissingBaseline {
                path: baseline.to_path_buf(),
            });
        }
        BaselineAction::Compare => {}
    }

    // Decode the baseline PNG. Use `image::open` and convert to
    // RGBA8 — that matches the rgba layout iced emits.
    let baseline_img = image::open(baseline).map_err(VisualDiffError::Image)?;
    let baseline_rgba: RgbaImage = baseline_img.to_rgba8();

    if baseline_rgba.dimensions() != actual.dimensions() {
        write_diff_artifacts(&actual, &baseline_rgba, test_name)?;
        // T-VFH-D5 call site 1: DimensionMismatch — no perceptual diff.
        let emit_result = emit_visual_fail_html(VisualFailContext {
            test_name,
            assertion_location: concat!(file!(), ":", line!()),
            assertion_body: &format!(
                "dimension mismatch — baseline {}×{}, actual {}×{}",
                baseline_rgba.width(),
                baseline_rgba.height(),
                width,
                height,
            ),
            baseline_png_path: baseline,
            actual_png_path: &actual_path(test_name),
            diff_png_path: None,
            optional_vlm_verdict: None,
        });
        if let Err(ref err) = emit_result {
            eprintln!("warning: visual-fail HTML emission failed: {err}");
        }
        return Err(VisualDiffError::DimensionMismatch {
            baseline_w: baseline_rgba.width(),
            baseline_h: baseline_rgba.height(),
            actual_w: width,
            actual_h: height,
        });
    }

    // Byte-compare the rgba slices. `image::ImageBuffer::into_raw`
    // returns `Vec<u8>`; we go through `as_raw` to keep the buffers
    // alive for the diff path.
    if baseline_rgba.as_raw() == actual.as_raw() {
        return Ok(());
    }

    write_diff_artifacts(&actual, &baseline_rgba, test_name)?;
    // T-VFH-D5 call site 2: Mismatch — diff PNG available.
    let emit_result = emit_visual_fail_html(VisualFailContext {
        test_name,
        assertion_location: concat!(file!(), ":", line!()),
        assertion_body: &format!(
            "visual baseline mismatch:\n  baseline: {}\n    actual: {}\n      diff: {}",
            baseline.display(),
            actual_path(test_name).display(),
            diff_path(test_name).display(),
        ),
        baseline_png_path: baseline,
        actual_png_path: &actual_path(test_name),
        diff_png_path: Some(&diff_path(test_name)),
        optional_vlm_verdict: None,
    });
    if let Err(ref err) = emit_result {
        eprintln!("warning: visual-fail HTML emission failed: {err}");
    }
    Err(VisualDiffError::Mismatch {
        baseline: baseline.to_path_buf(),
        diff: diff_path(test_name),
        actual: actual_path(test_name),
    })
}

/// Public diff helper for the V9 self-test. Compares two arbitrary
/// `RgbImage` buffers (no Screenshot indirection) and writes the diff
/// PNG. Returns `Ok(())` on byte-identity, `Err(Mismatch)` otherwise.
///
/// Mirrors the failure path of `matches_screenshot` so the V9 test
/// exercises the same `image_compare::rgb_hybrid_compare` integration.
pub fn matches_rgb_buffers(
    baseline: &RgbImage,
    actual: &RgbImage,
    test_name: &str,
) -> Result<(), VisualDiffError> {
    if baseline.dimensions() != actual.dimensions() {
        write_rgb_diff_artifacts(actual, baseline, test_name)?;
        return Err(VisualDiffError::DimensionMismatch {
            baseline_w: baseline.width(),
            baseline_h: baseline.height(),
            actual_w: actual.width(),
            actual_h: actual.height(),
        });
    }
    if baseline.as_raw() == actual.as_raw() {
        return Ok(());
    }
    write_rgb_diff_artifacts(actual, baseline, test_name)?;
    // T-VFH-D5 call site 3: matches_rgb_buffers Mismatch — diff PNG available.
    // The baseline here is an in-memory buffer (no source file), so we save
    // it to a temp path that the HTML emitter can read.
    let baseline_tmp_path = visual_diff_dir().join(format!("{test_name}-baseline-tmp.png"));
    let has_baseline_tmp = baseline.save(&baseline_tmp_path).is_ok();
    if has_baseline_tmp {
        let emit_result = emit_visual_fail_html(VisualFailContext {
            test_name,
            assertion_location: concat!(file!(), ":", line!()),
            assertion_body: &format!(
                "visual baseline mismatch (in-memory buffer for {test_name}):\n    actual: {}\n      diff: {}",
                actual_path(test_name).display(),
                diff_path(test_name).display(),
            ),
            baseline_png_path: &baseline_tmp_path,
            actual_png_path: &actual_path(test_name),
            diff_png_path: Some(&diff_path(test_name)),
            optional_vlm_verdict: None,
        });
        if let Err(ref err) = emit_result {
            eprintln!("warning: visual-fail HTML emission failed: {err}");
        }
        // Best-effort cleanup of the tmp baseline copy.
        let _ = fs::remove_file(&baseline_tmp_path);
    }
    Err(VisualDiffError::Mismatch {
        baseline: PathBuf::from(format!("(in-memory buffer for {test_name})")),
        diff: diff_path(test_name),
        actual: actual_path(test_name),
    })
}

fn diff_path(test_name: &str) -> PathBuf {
    visual_diff_dir().join(format!("{test_name}.png"))
}

fn actual_path(test_name: &str) -> PathBuf {
    visual_diff_dir().join(format!("{test_name}-actual.png"))
}

fn write_diff_artifacts(
    actual: &RgbaImage,
    baseline: &RgbaImage,
    test_name: &str,
) -> Result<(), VisualDiffError> {
    let diff_dir = visual_diff_dir();
    fs::create_dir_all(&diff_dir).map_err(VisualDiffError::Io)?;

    // Persist the actual PNG alongside the diff so the operator can
    // open all three in one Finder window (baseline / actual / diff).
    actual
        .save(actual_path(test_name))
        .map_err(VisualDiffError::Image)?;

    // image-compare needs RgbImage — strip the alpha channel.
    let baseline_rgb = rgba_to_rgb(baseline);
    let actual_rgb = rgba_to_rgb(actual);
    write_rgb_diff_artifacts(&actual_rgb, &baseline_rgb, test_name)
}

fn write_rgb_diff_artifacts(
    actual: &RgbImage,
    baseline: &RgbImage,
    test_name: &str,
) -> Result<(), VisualDiffError> {
    let diff_dir = visual_diff_dir();
    fs::create_dir_all(&diff_dir).map_err(VisualDiffError::Io)?;

    // image-compare 0.4 — `rgb_hybrid_compare` returns a similarity
    // score plus a per-pixel similarity map; we convert the map to an
    // 8-bit greyscale PNG where dark = high delta. Bright spots
    // localise the regression.
    let similarity =
        image_compare::rgb_hybrid_compare(baseline, actual).map_err(VisualDiffError::Compare)?;

    // `similarity.image.to_color_map()` returns a u8 luminance map.
    let diff_img = similarity.image.to_color_map();
    diff_img
        .save(diff_path(test_name))
        .map_err(VisualDiffError::Image)?;
    Ok(())
}

fn rgba_to_rgb(src: &RgbaImage) -> RgbImage {
    let (w, h) = src.dimensions();
    let mut out: RgbImage = ImageBuffer::new(w, h);
    for (x, y, Rgba([r, g, b, _])) in src.enumerate_pixels() {
        out.put_pixel(x, y, image::Rgb([*r, *g, *b]));
    }
    out
}

/// Error variants surfaced by [`matches_screenshot`]. Each carries
/// enough context for the integration test's `.expect(...)` panic
/// message to cite the baseline / diff / actual paths.
#[derive(Debug)]
pub enum VisualDiffError {
    /// Baseline-vs.-actual byte mismatch. The triple of paths is
    /// printed so the operator can `open` them in Finder.
    Mismatch {
        baseline: PathBuf,
        diff: PathBuf,
        actual: PathBuf,
    },
    /// No baseline on disk and no capture requested. Before `#187` this was
    /// a silent pass that wrote whatever had just rendered.
    MissingBaseline { path: PathBuf },
    /// Dimensions don't match — usually a viewport/scale_factor drift.
    DimensionMismatch {
        baseline_w: u32,
        baseline_h: u32,
        actual_w: u32,
        actual_h: u32,
    },
    /// Screenshot rgba length didn't match width*height*4.
    Rgba { expected: usize, actual: usize },
    /// ImageBuffer::from_raw refused the slice (alignment / length).
    RgbaFromRaw { width: u32, height: u32 },
    /// Underlying `image` crate error (decode / encode).
    Image(image::ImageError),
    /// Filesystem error (create_dir_all / save).
    Io(io::Error),
    /// `image_compare::rgb_hybrid_compare` refused the inputs.
    Compare(image_compare::CompareError),
}

impl std::fmt::Display for VisualDiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisualDiffError::Mismatch {
                baseline,
                diff,
                actual,
            } => write!(
                f,
                "visual baseline mismatch:\n  baseline: {}\n    actual: {}\n      diff: {}",
                baseline.display(),
                actual.display(),
                diff.display()
            ),
            VisualDiffError::MissingBaseline { path } => write!(
                f,
                "no baseline at {} — a render with nothing to compare against is not a \
                 pass. If this frame is the one that should be recorded, re-run with \
                 `{CAPTURE_ENV}=1` to capture it.",
                path.display(),
            ),
            VisualDiffError::DimensionMismatch {
                baseline_w,
                baseline_h,
                actual_w,
                actual_h,
            } => write!(
                f,
                "dimension mismatch — baseline {}x{}, actual {}x{}",
                baseline_w, baseline_h, actual_w, actual_h
            ),
            VisualDiffError::Rgba { expected, actual } => write!(
                f,
                "screenshot rgba length wrong — expected {} bytes, got {}",
                expected, actual
            ),
            VisualDiffError::RgbaFromRaw { width, height } => write!(
                f,
                "ImageBuffer::from_raw refused {}x{} rgba slice",
                width, height
            ),
            VisualDiffError::Image(err) => write!(f, "image crate error: {err}"),
            VisualDiffError::Io(err) => write!(f, "I/O error: {err}"),
            VisualDiffError::Compare(err) => write!(f, "image_compare error: {err}"),
        }
    }
}

impl std::error::Error for VisualDiffError {}

/// Refuse to byte-compare a render that was not drawn with the embedded UI font.
///
/// A baseline is reproducible only if every glyph in it comes from a font the repo
/// ships. `iced_test::screenshot` builds its renderer from
/// `Program::settings().default_font`, so a program built without
/// `.default_font(ui::theme::font::UI)` renders through the OS font database and
/// its baseline drifts with the OS (`docs/dev-notes/visual-baseline-drift-2026-07-27.md`).
/// Every byte-compare entry point calls this BEFORE rendering, so such a baseline
/// can never even be captured.
#[allow(dead_code)] // each including test file uses a different subset of fixtures
pub fn assert_embedded_default_font<P: iced_test::program::Program>(program: &P) {
    let font = iced_test::program::Program::settings(program).default_font;
    assert!(
        font == ui::theme::font::UI,
        "this program renders with {font:?}, not the embedded UI font: build it with \
         `.default_font(ui::theme::font::embedded())`, which LOADS the face as well as \
         naming it (ADR-0093)"
    );
}

#[cfg(test)]
mod tests {
    //! V9 self-test — the diff helper materialises a PNG on mismatch.
    //!
    //! Lives in a `#[cfg(test)]` module inside the helper file so the
    //! test compiles only when the surrounding integration test
    //! includes this module (per Cargo test layout, `cfg(test)` inside
    //! a `tests/fixtures/...` file is gated by the parent target's
    //! test compilation).

    use super::*;
    use image::{Rgb, RgbImage};

    /// Two known-different RGB buffers must produce
    /// `target/visual-diff/<test_name>.png` AND
    /// `target/visual-diff/<test_name>-actual.png`.
    pub fn run_visual_diff_helper_writes_diff_png_on_mismatch() {
        let test_name = "visual_diff_helper_writes_diff_png_on_mismatch";

        // Two 8x8 RGB buffers — baseline solid red, actual solid green.
        // image-compare's hybrid SSIM+RMS picks up the chrominance
        // delta and produces a bright diff PNG.
        let baseline: RgbImage = ImageBuffer::from_pixel(8, 8, Rgb([255, 0, 0]));
        let actual: RgbImage = ImageBuffer::from_pixel(8, 8, Rgb([0, 255, 0]));

        // Best-effort cleanup so the first run of this test isn't
        // tricked by a pre-existing diff PNG.
        let _ = fs::remove_file(diff_path(test_name));
        let _ = fs::remove_file(actual_path(test_name));

        let result = matches_rgb_buffers(&baseline, &actual, test_name);
        assert!(
            matches!(result, Err(VisualDiffError::Mismatch { .. })),
            "two-color buffers must report Mismatch; got {result:?}"
        );
        assert!(
            diff_path(test_name).exists(),
            "diff PNG must exist at {}",
            diff_path(test_name).display()
        );
    }

    // ── #187: a missing baseline is a failure, and that is tested on both
    // sides of both dimensions ───────────────────────────────────────────────

    /// The decision table in full. Four inputs, three distinct outcomes: a
    /// fixture that only ever asked `baseline_action(true, false)` would pin
    /// nothing, because every wrong implementation also returns `Compare`
    /// there.
    #[test]
    fn a_render_with_no_baseline_to_compare_against_is_not_a_pass() {
        assert_eq!(baseline_action(true, false), BaselineAction::Compare);
        assert_eq!(baseline_action(false, false), BaselineAction::Missing);
        assert_eq!(baseline_action(false, true), BaselineAction::Capture);
        assert_eq!(baseline_action(true, true), BaselineAction::Capture);
    }

    /// Only an explicit `1`. A variable left over as `0`, `true` or empty must
    /// not rewrite 56 baselines on the next run.
    #[test]
    fn only_an_explicit_1_asks_for_a_capture() {
        assert!(capture_requested(Some(OsStr::new("1"))));
        assert!(!capture_requested(None));
        assert!(!capture_requested(Some(OsStr::new("0"))));
        assert!(!capture_requested(Some(OsStr::new(""))));
        assert!(!capture_requested(Some(OsStr::new("true"))));
        assert!(!capture_requested(Some(OsStr::new("yes"))));
    }

    /// A 2x2 red frame, so the two end-to-end cases below cost no renderer.
    fn probe_screenshot() -> iced::window::Screenshot {
        let rgba: Vec<u8> = [255u8, 0, 0, 255].repeat(4);
        iced::window::Screenshot::new(rgba, iced::Size::new(2, 2), 1.0)
    }

    fn probe_path(leaf: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ui-baseline-probe-{}-{leaf}.png",
            std::process::id()
        ))
    }

    /// The failing side, through the real entry point: no file, no capture →
    /// `MissingBaseline`, and nothing written. The second assertion is the
    /// one that would have caught `#187`: the old code's failure mode was not
    /// a wrong verdict, it was a file appearing.
    #[test]
    fn the_compare_path_refuses_an_absent_baseline_and_writes_nothing() {
        let path = probe_path("refuses");
        let _ = fs::remove_file(&path);

        let result = matches_screenshot_with_capture(
            &probe_screenshot(),
            path.to_str().expect("probe path is utf-8"),
            "probe_refuses",
            false,
        );

        assert!(
            matches!(result, Err(VisualDiffError::MissingBaseline { .. })),
            "an absent baseline must be reported, not minted; got {result:?}"
        );
        assert!(
            !path.exists(),
            "the refusing path must not leave a baseline behind at {}",
            path.display()
        );
    }

    /// The capturing side: same inputs, capture asked for → the frame lands on
    /// disk and compares clean on the next call.
    #[test]
    fn the_capture_path_writes_the_frame_and_the_next_compare_matches_it() {
        let path = probe_path("captures");
        let _ = fs::remove_file(&path);
        let as_str = path.to_str().expect("probe path is utf-8").to_owned();

        matches_screenshot_with_capture(&probe_screenshot(), &as_str, "probe_captures", true)
            .expect("a requested capture must succeed");
        assert!(path.exists(), "capture must write {}", path.display());

        matches_screenshot_with_capture(&probe_screenshot(), &as_str, "probe_captures", false)
            .expect("the captured frame must compare clean against itself");

        let _ = fs::remove_file(&path);
    }
}
