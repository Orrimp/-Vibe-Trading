//! Visual-fail HTML artifact emitter — visual-fail-html-reporter v0.1.0.
//!
//! Emits a self-contained `visual-fail-<test_name>-<ts>.html` file on
//! visual assertion FAIL. The HTML inlines baseline, actual, and
//! perceptual-diff PNGs as base64 data URIs alongside the assertion
//! location and body text — the operator opens one file in Safari/Chrome
//! for the full triage view instead of hunting across three PNG files.
//!
//! ## Output path
//!
//! Default: `target/visual-diff/<test_name>-<YYYYMMDDTHHMMSSZ>.html`
//! (gitignored via existing `target/` rule; no `.gitignore` delta).
//!
//! Opt-in spec-persist: set `EMIT_VISUAL_FAIL_TO_SPEC=1` AND
//! `VISUAL_FAIL_SPEC_SLUG=<slug>` to also write a byte-identical copy to
//! `evidence/<slug>/reports/visual-fail-<test_name>-<ts>.html` (env var
//! names kept verbatim across the 2026-07-25 BMAD-migration Phase 3
//! `spec/`→`evidence/` reports-corpus move — only the disk target moved).
//! Default OFF (repo-size guard per K1+K2 falsifiers). If only
//! `EMIT_VISUAL_FAIL_TO_SPEC=1` is set without `VISUAL_FAIL_SPEC_SLUG`,
//! a warning is emitted to stderr and only the `target/` copy is written.
//!
//! ## Failure-mode contract (D-VF-5)
//!
//! If emission fails for any reason, the error is logged via `eprintln!`
//! and `Err(VisualFailHtmlError)` is returned. The CALLER is responsible
//! for continuing with the original `VisualDiffError` return value
//! unchanged. The helper never panics.

#![allow(clippy::expect_used, clippy::unwrap_used, dead_code)]

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64_STANDARD;

// ── Inline CSS (D-VF-1 minimum) ─────────────────────────────────────────────

const STYLE: &str = r#"body { background: #1a1a1a; color: #e0e0e0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 0; padding: 1rem; }
h1 { font-size: 1.5rem; border-bottom: 1px solid #444; padding-bottom: 0.5rem; }
h2 { font-size: 1.1rem; margin-top: 2rem; color: #8ab4f8; }
section { max-width: 100%; }
img { max-width: 100%; height: auto; object-fit: contain; display: block; border: 1px solid #333; }
pre { background: #0f0f0f; padding: 0.75rem; overflow-x: auto; font-size: 0.85rem; white-space: pre-wrap; }
.dim { color: #888; font-size: 0.85rem; margin: 0.25rem 0 0; }"#;

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors returned by [`emit_visual_fail_html`].
#[derive(Debug)]
pub enum VisualFailHtmlError {
    Io(std::io::Error),
    Image(image::ImageError),
}

impl fmt::Display for VisualFailHtmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VisualFailHtmlError::Io(e) => write!(f, "I/O error: {e}"),
            VisualFailHtmlError::Image(e) => write!(f, "image error: {e}"),
        }
    }
}

impl std::error::Error for VisualFailHtmlError {}

// ── Context struct ────────────────────────────────────────────────────────────

/// Context for one visual-fail HTML emission (D-VF-2 signature contract).
///
/// `diff_png_path` is `Option` because `VisualDiffError::DimensionMismatch`
/// has no meaningful perceptual diff (the comparator refuses unequal
/// dimensions). `optional_vlm_verdict` is the v0.2.0+ hook for
/// `ui-vlm-judge`; v0.1.0 always passes `None`.
pub struct VisualFailContext<'a> {
    pub test_name: &'a str,
    /// e.g. `"crates/ui/tests/visual_snapshots.rs:148"`
    pub assertion_location: &'a str,
    /// e.g. the `VisualDiffError::Display` output
    pub assertion_body: &'a str,
    pub baseline_png_path: &'a Path,
    pub actual_png_path: &'a Path,
    /// `None` for `DimensionMismatch` (no perceptual diff generated).
    pub diff_png_path: Option<&'a Path>,
    /// v0.2.0 hook for `ui-vlm-judge`; pass `None` in v0.1.0.
    pub optional_vlm_verdict: Option<&'a str>,
}

// ── Main entry point ──────────────────────────────────────────────────────────

/// Emit a self-contained `visual-fail-<ts>.html` to `target/visual-diff/`.
///
/// Returns the `target/` path written on success, or `Err` on any I/O or
/// PNG-decode failure. Never panics — callers log the error and continue
/// with the original `VisualDiffError` return (D-VF-5 contract).
///
/// When `EMIT_VISUAL_FAIL_TO_SPEC=1` and `VISUAL_FAIL_SPEC_SLUG` are both
/// set, a byte-identical copy is additionally written to
/// `evidence/<slug>/reports/visual-fail-<test_name>-<ts>.html`.
pub fn emit_visual_fail_html(ctx: VisualFailContext<'_>) -> Result<PathBuf, VisualFailHtmlError> {
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let filename = format!("visual-fail-{}-{ts}.html", ctx.test_name);

    // ── Build the HTML payload ────────────────────────────────────────────────
    let html = build_html(&ctx, &ts)?;

    // ── Write to target/visual-diff/ ─────────────────────────────────────────
    let target_dir = visual_diff_dir();
    fs::create_dir_all(&target_dir).map_err(VisualFailHtmlError::Io)?;
    let target_path = target_dir.join(&filename);
    fs::write(&target_path, html.as_bytes()).map_err(VisualFailHtmlError::Io)?;

    // ── Optional spec-persist (Q1 env-var gate) ───────────────────────────────
    // Env var names kept verbatim (EMIT_VISUAL_FAIL_TO_SPEC /
    // VISUAL_FAIL_SPEC_SLUG) across the 2026-07-25 Phase 3 evidence/ move —
    // only the disk target changed.
    let emit_to_spec = std::env::var("EMIT_VISUAL_FAIL_TO_SPEC")
        .map(|v| v == "1")
        .unwrap_or(false);
    if emit_to_spec {
        match std::env::var("VISUAL_FAIL_SPEC_SLUG") {
            Ok(slug) if !slug.is_empty() => {
                let evidence_dir = workspace_root()
                    .join("evidence")
                    .join(&slug)
                    .join("reports");
                if let Err(e) = fs::create_dir_all(&evidence_dir) {
                    eprintln!("warning: visual-fail HTML spec-persist dir create failed: {e}");
                } else {
                    let evidence_path = evidence_dir.join(&filename);
                    if let Err(e) = fs::write(&evidence_path, html.as_bytes()) {
                        eprintln!("warning: visual-fail HTML spec-persist write failed: {e}");
                    }
                }
            }
            Ok(_) | Err(_) => {
                eprintln!(
                    "warning: EMIT_VISUAL_FAIL_TO_SPEC=1 set but VISUAL_FAIL_SPEC_SLUG missing; \
                     spec-persist skipped"
                );
            }
        }
    }

    Ok(target_path)
}

// ── HTML builder ─────────────────────────────────────────────────────────────

fn build_html(ctx: &VisualFailContext<'_>, ts: &str) -> Result<String, VisualFailHtmlError> {
    let baseline_bytes = fs::read(ctx.baseline_png_path).map_err(VisualFailHtmlError::Io)?;
    let actual_bytes = fs::read(ctx.actual_png_path).map_err(VisualFailHtmlError::Io)?;

    let (baseline_w, baseline_h) = png_dimensions(ctx.baseline_png_path)?;
    let (actual_w, actual_h) = png_dimensions(ctx.actual_png_path)?;

    let baseline_b64 = B64_STANDARD.encode(&baseline_bytes);
    let actual_b64 = B64_STANDARD.encode(&actual_bytes);

    let diff_section = match ctx.diff_png_path {
        Some(diff_path) => {
            let diff_bytes = fs::read(diff_path).map_err(VisualFailHtmlError::Io)?;
            let (dw, dh) = png_dimensions(diff_path)?;
            let diff_b64 = B64_STANDARD.encode(&diff_bytes);
            format!(
                r#"  <section class="diff">
    <h2>Perceptual diff (image-compare hybrid SSIM)</h2>
    <img src="data:image/png;base64,{diff_b64}" alt="diff">
    <p class="dim">{dw} &times; {dh} px</p>
  </section>
"#
            )
        }
        None => String::new(),
    };

    let vlm_section = match ctx.optional_vlm_verdict {
        Some(verdict) => format!(
            r#"  <section class="vlm">
    <h2>VLM verdict (shadow mode)</h2>
    <pre>{}</pre>
  </section>
"#,
            html_escape(verdict)
        ),
        None => String::new(),
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en"><head>
  <meta charset="utf-8">
  <title>Visual fail &mdash; {test_name} &mdash; {ts}</title>
  <style>{STYLE}</style>
</head><body>
  <h1>Visual fail &mdash; {test_name} <small>{ts}</small></h1>
  <section class="meta">
    <h2>Assertion</h2>
    <pre>{assertion_location}

{assertion_body}</pre>
  </section>
  <section class="baseline">
    <h2>Baseline (what should render)</h2>
    <img src="data:image/png;base64,{baseline_b64}" alt="baseline">
    <p class="dim">{baseline_w} &times; {baseline_h} px</p>
  </section>
  <section class="actual">
    <h2>Actual (what rendered instead)</h2>
    <img src="data:image/png;base64,{actual_b64}" alt="actual">
    <p class="dim">{actual_w} &times; {actual_h} px</p>
  </section>
{diff_section}{vlm_section}</body></html>
"#,
        test_name = html_escape(ctx.test_name),
        assertion_location = html_escape(ctx.assertion_location),
        assertion_body = html_escape(ctx.assertion_body),
    );

    Ok(html)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn png_dimensions(path: &Path) -> Result<(u32, u32), VisualFailHtmlError> {
    let dims = image::ImageReader::open(path)
        .map_err(VisualFailHtmlError::Io)?
        .into_dimensions()
        .map_err(VisualFailHtmlError::Image)?;
    Ok(dims)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Mirrors `visual_diff_dir()` from `visual_diff.rs` — resolves
/// `target/visual-diff/` from `CARGO_TARGET_DIR` or `CARGO_MANIFEST_DIR`.
fn visual_diff_dir() -> PathBuf {
    let target_root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            manifest_dir.join("..").join("..").join("target")
        });
    target_root.join("visual-diff")
}

/// Derive workspace root from `CARGO_MANIFEST_DIR` (crates/ui → ../..).
///
/// Canonicalizes the result so that `..` components resolve and the path
/// matches what callers compare against (e.g. `TempDir::path()` which is
/// already canonical on macOS / Linux).
fn workspace_root() -> PathBuf {
    let raw = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("..")
        .join("..");
    // Best-effort canonicalize — falls back to the raw path if the dir
    // doesn't exist yet (which shouldn't happen in practice).
    std::fs::canonicalize(&raw).unwrap_or(raw)
}

// ── Self-tests (T-VFH-D6) ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb, RgbImage};
    use std::sync::Mutex;

    // Serialize tests that mutate process-level env vars.  Cargo runs
    // integration-test binary with multiple threads by default; without
    // this guard, Test 1's `remove_var("EMIT_VISUAL_FAIL_TO_SPEC")` could
    // race with Test 2's `set_var("EMIT_VISUAL_FAIL_TO_SPEC", "1")`.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Write a minimal solid-color 8×8 PNG to the given path.
    fn write_tiny_png(path: &Path, r: u8, g: u8, b: u8) {
        let img: RgbImage = ImageBuffer::from_pixel(8, 8, Rgb([r, g, b]));
        img.save(path).expect("synthetic PNG write must succeed");
    }

    /// Test 1 (R4.1 / R4.2): default path — env vars unset → file written
    /// to `target/visual-diff/`.  Asserts HTML contains the base64-encoded
    /// PNG header bytes + assertion text + section `<h2>` headers.
    #[test]
    fn emit_visual_fail_html_default_path_inlines_pngs() {
        let dir = tempfile::TempDir::new().expect("TempDir must succeed");

        let baseline_path = dir.path().join("baseline.png");
        let actual_path = dir.path().join("actual.png");
        let diff_path = dir.path().join("diff.png");

        write_tiny_png(&baseline_path, 255, 0, 0); // red baseline
        write_tiny_png(&actual_path, 0, 255, 0); // green actual
        write_tiny_png(&diff_path, 0, 0, 255); // blue diff

        // Override CARGO_TARGET_DIR to land inside TempDir so the test is
        // hermetic and leaves no state under the workspace's target/.
        let target_override = dir.path().join("target");

        let result = {
            // Acquire ENV_LOCK before mutating process-level env vars to
            // prevent races when Cargo runs tests in parallel threads.
            let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            // SAFETY (env var): guarded by ENV_LOCK; no other thread in this
            // process will read or write these vars while the lock is held.
            unsafe {
                std::env::set_var("CARGO_TARGET_DIR", &target_override);
                std::env::remove_var("EMIT_VISUAL_FAIL_TO_SPEC");
                std::env::remove_var("VISUAL_FAIL_SPEC_SLUG");
            }

            let ctx = VisualFailContext {
                test_name: "test_default_path",
                assertion_location: "crates/ui/tests/fixtures/visual_fail_html.rs:999",
                assertion_body: "PNG byte mismatch: 64 of 64 pixels differ",
                baseline_png_path: &baseline_path,
                actual_png_path: &actual_path,
                diff_png_path: Some(&diff_path),
                optional_vlm_verdict: None,
            };

            let result = emit_visual_fail_html(ctx);

            // Restore env before releasing the lock.
            unsafe {
                std::env::remove_var("CARGO_TARGET_DIR");
            }
            result
        };

        let html_path = result.expect("emit_visual_fail_html must succeed");
        assert!(html_path.exists(), "HTML file must exist at {html_path:?}");

        let html = fs::read_to_string(&html_path).expect("HTML file must be readable");

        // Must contain base64 data URI prefix.
        assert!(
            html.contains("data:image/png;base64,"),
            "HTML must contain base64 data URI"
        );
        // Baseline PNG bytes should appear — read the actual baseline and
        // check that its base64 encoding is in the HTML.
        let baseline_bytes = fs::read(&baseline_path).unwrap();
        let baseline_b64 = B64_STANDARD.encode(&baseline_bytes);
        assert!(
            html.contains(&baseline_b64),
            "HTML must contain base64-encoded baseline PNG"
        );
        // Assertion text must appear (HTML-escaped form).
        assert!(
            html.contains("PNG byte mismatch: 64 of 64 pixels differ"),
            "HTML must contain assertion body"
        );
        assert!(
            html.contains("crates/ui/tests/fixtures/visual_fail_html.rs:999"),
            "HTML must contain assertion location"
        );
        // Section headers must appear.
        assert!(
            html.contains("Baseline (what should render)"),
            "missing baseline h2"
        );
        assert!(
            html.contains("Actual (what rendered instead)"),
            "missing actual h2"
        );
        assert!(
            html.contains("Perceptual diff (image-compare hybrid SSIM)"),
            "missing diff h2"
        );
        // Dimensions in HTML.
        assert!(
            html.contains("8 &times; 8 px"),
            "HTML must contain PNG dimensions"
        );
    }

    /// Test 2 (R4.3): spec-persist path — both env vars set with TempDir
    /// path injected → file written to spec-shaped path AND is byte-identical
    /// to the `target/`-side copy.
    #[test]
    fn emit_visual_fail_html_spec_persist_writes_byte_identical_copy() {
        let dir = tempfile::TempDir::new().expect("TempDir must succeed");

        let baseline_path = dir.path().join("baseline.png");
        let actual_path = dir.path().join("actual.png");

        write_tiny_png(&baseline_path, 128, 0, 0);
        write_tiny_png(&actual_path, 0, 128, 0);

        let target_override = dir.path().join("target");
        // Evidence root is also under TempDir so the test is fully hermetic.
        // We pass an absolute path via VISUAL_FAIL_SPEC_SLUG by pointing
        // workspace_root() to the TempDir. We use a relative slug name and
        // create the expected evidence/<slug>/reports/ structure manually.
        let spec_slug = "test-slug";

        let result = {
            // Acquire ENV_LOCK before mutating process-level env vars.
            let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            // SAFETY (env var): guarded by ENV_LOCK; no other thread in this
            // process will read or write these vars while the lock is held.
            unsafe {
                std::env::set_var("CARGO_TARGET_DIR", &target_override);
                std::env::set_var("EMIT_VISUAL_FAIL_TO_SPEC", "1");
                std::env::set_var("VISUAL_FAIL_SPEC_SLUG", spec_slug);
                // Point CARGO_MANIFEST_DIR to a fake "crates/ui" under TempDir so
                // workspace_root() resolves to TempDir (two ".." hops).
                let fake_manifest_dir = dir.path().join("crates").join("ui");
                fs::create_dir_all(&fake_manifest_dir).unwrap();
                std::env::set_var("CARGO_MANIFEST_DIR", &fake_manifest_dir);
            }

            let ctx = VisualFailContext {
                test_name: "test_spec_persist",
                assertion_location: "crates/ui/tests/fixtures/visual_fail_html.rs:1",
                assertion_body: "spec-persist smoke test",
                baseline_png_path: &baseline_path,
                actual_png_path: &actual_path,
                diff_png_path: None,
                optional_vlm_verdict: None,
            };

            let result = emit_visual_fail_html(ctx);

            // Restore env before releasing the lock.
            unsafe {
                std::env::remove_var("CARGO_TARGET_DIR");
                std::env::remove_var("EMIT_VISUAL_FAIL_TO_SPEC");
                std::env::remove_var("VISUAL_FAIL_SPEC_SLUG");
                std::env::remove_var("CARGO_MANIFEST_DIR");
            }
            result
        };

        let target_path = result.expect("emit_visual_fail_html must succeed");
        assert!(target_path.exists(), "target/ HTML must exist");

        // Find the evidence-side copy — it has the same filename under
        // <TempDir>/evidence/test-slug/reports/.
        let filename = target_path.file_name().unwrap();
        let spec_path = dir
            .path()
            .join("evidence")
            .join(spec_slug)
            .join("reports")
            .join(filename);

        assert!(
            spec_path.exists(),
            "spec-persist HTML must exist at {spec_path:?}"
        );

        // Byte-identical check.
        let target_bytes = fs::read(&target_path).unwrap();
        let spec_bytes = fs::read(&spec_path).unwrap();
        assert_eq!(
            target_bytes, spec_bytes,
            "spec-persist copy must be byte-identical to target/ copy"
        );

        // Basic HTML sanity: no diff section (diff_png_path = None).
        let html = String::from_utf8(target_bytes).unwrap();
        assert!(
            html.contains("data:image/png;base64,"),
            "missing base64 data URI"
        );
        assert!(
            !html.contains("Perceptual diff"),
            "diff section must be absent when diff_png_path = None"
        );
    }
}
