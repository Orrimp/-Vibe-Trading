//! Regression test: Yahoo report-emit helper shape contract — D-V0.1.3-8.
//!
//! This test asserts that:
//!
//! 1. The emitted BTC Yahoo report body contains ZERO `rev=` substrings.
//! 2. The emitted BTC Yahoo report front-matter contains exactly ONE
//!    `revision_sha:` line matching a 64-char hex string.
//! 3. The `report::yahoo::YahooReportContext::data_source()` never includes
//!    `rev=` — the single constructor of Yahoo data-source strings.
//!
//! Together these guard H3 (future emitter retrofit-free) and the durable
//! body→frontmatter contract from D-V0.1.3-1.  A future `run_yahoo_*`
//! binary that hand-formats the data_source string or bypasses
//! `emit_sma_report` would cause this test to either fail or require
//! a deliberate SHA update — making the regression visible at CI.
//!
//! Tests (a) and (b) invoke `run_yahoo_sma` via `std::process::Command`
//! and require the `yahoo` feature + the on-disk parquet cache
//! (`data/yahoo/BTC-USD/1d/2024/`).  They are gracefully skipped when
//! the cache is absent.  Test (c) is a unit test on the public API —
//! no binary or cache required.

#![allow(clippy::unwrap_used)]

use std::path::Path;
#[cfg(feature = "yahoo")]
use std::process::Command;

// ── Helpers ────────────────────────────────────────────────────────────────────
//
// The three helpers below (`binary_path`, `split_frontmatter`, `read_report`)
// are only referenced from `#[cfg(feature = "yahoo")]` tests (a) and (b).
// Without the yahoo feature enabled they are dead code by design — they exist
// as documentation of the binary-invoke contract and are retained so the
// gated tests compile without restructuring.  Suppress the dead_code lint
// rather than removing the helpers (which would break a `--features yahoo`
// build) or #[cfg]-gating them (which would hide the API from non-gated IDEs).

#[allow(dead_code)]
fn workspace_root() -> std::path::PathBuf {
    let mut probe = std::env::current_dir().expect("cwd");
    for _ in 0..8 {
        if probe.join("Cargo.lock").is_file() {
            return probe;
        }
        if let Some(p) = probe.parent() {
            probe = p.to_path_buf();
        } else {
            break;
        }
    }
    std::env::current_dir().expect("cwd")
}

#[allow(dead_code)]
fn binary_path() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_run_yahoo_sma") {
        let path = std::path::PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    let root = workspace_root();
    let candidate = root.join("target/debug/run_yahoo_sma");
    if candidate.is_file() {
        return Some(candidate);
    }
    None
}

/// Strip YAML front-matter (`---\n...\n---\n`) and return (frontmatter, body).
#[allow(dead_code)]
fn split_frontmatter(text: &str) -> (String, String) {
    if !text.starts_with("---\n") {
        return (String::new(), text.to_string());
    }
    let after_open = &text[4..];
    if let Some(pos) = after_open.find("\n---\n") {
        let fm = after_open[..pos].to_string();
        let body = after_open[pos + 5..].to_string();
        (fm, body)
    } else {
        (String::new(), text.to_string())
    }
}

#[allow(dead_code)]
fn read_report(dir: &Path) -> String {
    let report = std::fs::read_dir(dir)
        .expect("read tempdir")
        .filter_map(|e| e.ok())
        .find(|e| {
            e.file_name()
                .to_string_lossy()
                .ends_with("-btc-yahoo-2024-1d-sma-cross.md")
        })
        .expect("no BTC Yahoo report generated in tempdir");
    std::fs::read_to_string(report.path()).unwrap_or_else(|e| panic!("failed to read report: {e}"))
}

// ── (c) Unit-level API contract — no binary or cache required ─────────────────

/// The data_source() function must NEVER produce a `rev=` substring.
/// This is a compile-time / unit-level guard on the public helper API.
#[test]
fn data_source_never_contains_rev_substring() {
    use backtest::report::yahoo::YahooReportContext;

    let tickers = ["BTC-USD", "ETH-USD", "BNB-USD", "SOL-USD", "LINK-USD"];
    for ticker in tickers {
        let ctx = YahooReportContext {
            ticker,
            interval: "1d",
            year: 2024,
            revision_sha: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        };
        let ds = ctx.data_source();
        assert!(
            !ds.contains("rev="),
            "YahooReportContext::data_source() MUST NOT contain 'rev='; \
             got '{ds}' for ticker {ticker}"
        );
        assert!(
            ds.starts_with("yahoo-cache:"),
            "data_source must start with 'yahoo-cache:'; got '{ds}'"
        );
        assert!(
            ds.ends_with("/2024"),
            "data_source must end with '/2024'; got '{ds}'"
        );
    }
}

// ── (a) Emitted body has zero `rev=` substrings ────────────────────────────────

/// Verify that a freshly-emitted BTC Yahoo report body contains NO `rev=`.
///
/// This is the D-V0.1.3-1 body-shape contract gate.  Any future emitter that
/// regresses by calling `report::sma::write` directly (bypassing the yahoo
/// helper) with a hand-formatted `data_source` containing `rev=` will fail
/// this test.
#[cfg(feature = "yahoo")]
#[test]
fn emitted_btc_report_body_has_no_rev_substring() {
    let Some(bin) = binary_path() else {
        eprintln!(
            "SKIP: run_yahoo_sma binary not found — \
             build with --features yahoo first"
        );
        return;
    };

    let root = workspace_root();
    let cache_root = root.join("data/yahoo");
    if !cache_root.join("BTC-USD/1d/2024/01.parquet").is_file() {
        eprintln!("SKIP: data/yahoo/BTC-USD/1d/2024/ not present");
        return;
    }

    let reports_dir = tempfile::tempdir().expect("tempdir");

    let output = Command::new(&bin)
        .arg("--cache-root")
        .arg(&cache_root)
        .arg("--reports-dir")
        .arg(reports_dir.path())
        .output()
        .expect("failed to run run_yahoo_sma");

    assert!(
        output.status.success(),
        "run_yahoo_sma exited with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let text = read_report(reports_dir.path());
    let (_, body) = split_frontmatter(&text);

    assert!(
        !body.contains("rev="),
        "Report body MUST NOT contain 'rev=' (D-V0.1.3-1 body-shape contract).\n\
         Found 'rev=' in body:\n{body}"
    );
}

// ── (b) Front-matter has exactly one `revision_sha:` with 64-char hex ─────────

/// Verify that a freshly-emitted BTC Yahoo report front-matter contains
/// exactly one `revision_sha:` line matching a 64-char hex string.
///
/// This guards the D-V0.1.3-1 frontmatter injection contract.  If the
/// `revision_sha:` line is absent or malformed, this test fails loudly —
/// surfacing the regression before any anchor check can catch it.
#[cfg(feature = "yahoo")]
#[test]
fn emitted_btc_report_frontmatter_has_revision_sha() {
    let Some(bin) = binary_path() else {
        eprintln!(
            "SKIP: run_yahoo_sma binary not found — \
             build with --features yahoo first"
        );
        return;
    };

    let root = workspace_root();
    let cache_root = root.join("data/yahoo");
    if !cache_root.join("BTC-USD/1d/2024/01.parquet").is_file() {
        eprintln!("SKIP: data/yahoo/BTC-USD/1d/2024/ not present");
        return;
    }

    let reports_dir = tempfile::tempdir().expect("tempdir");

    let output = Command::new(&bin)
        .arg("--cache-root")
        .arg(&cache_root)
        .arg("--reports-dir")
        .arg(reports_dir.path())
        .output()
        .expect("failed to run run_yahoo_sma");

    assert!(
        output.status.success(),
        "run_yahoo_sma exited with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let text = read_report(reports_dir.path());
    let (frontmatter, _) = split_frontmatter(&text);

    // Count `revision_sha:` lines in frontmatter.
    let rev_lines: Vec<&str> = frontmatter
        .lines()
        .filter(|l| l.trim_start().starts_with("revision_sha:"))
        .collect();

    assert_eq!(
        rev_lines.len(),
        1,
        "Front-matter must contain exactly 1 'revision_sha:' line; \
         found {} in:\n{frontmatter}",
        rev_lines.len()
    );

    // Extract the SHA value after `revision_sha: `.
    let line = rev_lines[0];
    let sha_part = line
        .split_once("revision_sha:")
        .map(|(_, v)| v.trim())
        .expect("revision_sha: line malformed");

    assert_eq!(
        sha_part.len(),
        64,
        "revision_sha value must be exactly 64 hex chars; got '{}' (len={})",
        sha_part,
        sha_part.len()
    );
    assert!(
        sha_part.chars().all(|c| c.is_ascii_hexdigit()),
        "revision_sha value must be all hex chars; got '{sha_part}'"
    );
}
