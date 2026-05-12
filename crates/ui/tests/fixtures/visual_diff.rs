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
//! - First run: baseline doesn't exist → write the actual rgba as the
//!   baseline PNG and return `Ok(())`. Operator visually reviews,
//!   commits.
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

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use image::{ImageBuffer, RgbImage, Rgba, RgbaImage};

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
/// `baseline_path` is relative to the workspace root or absolute. On
/// first run the baseline is written from `screenshot.rgba`; the test
/// passes so the operator can review-and-commit. On subsequent runs
/// the actual rgba is byte-compared against the decoded baseline; a
/// mismatch writes the perceptual-diff PNG via
/// `image_compare::rgb_hybrid_compare` and returns `Err(Mismatch)`.
///
/// The function does NOT panic — it returns `Result` so the integration
/// test can choose to panic with `.expect(...)` (preserving an
/// operator-friendly multi-line failure message) or to assert in a
/// `catch_unwind` (for the V9 self-test).
pub fn matches_screenshot(
    screenshot: &iced::window::Screenshot,
    baseline_path: &str,
    test_name: &str,
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

    if !baseline.exists() {
        // First-run: persist the baseline so subsequent runs have
        // something to compare. Operator reviews the PNG before
        // committing (H2 falsifier in feature.md).
        if let Some(parent) = baseline.parent() {
            fs::create_dir_all(parent).map_err(VisualDiffError::Io)?;
        }
        actual.save(baseline).map_err(VisualDiffError::Image)?;
        return Ok(());
    }

    // Decode the baseline PNG. Use `image::open` and convert to
    // RGBA8 — that matches the rgba layout iced emits.
    let baseline_img = image::open(baseline).map_err(VisualDiffError::Image)?;
    let baseline_rgba: RgbaImage = baseline_img.to_rgba8();

    if baseline_rgba.dimensions() != actual.dimensions() {
        write_diff_artifacts(&actual, &baseline_rgba, test_name)?;
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
}
