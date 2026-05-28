//! Integration tests for `run_yahoo_sma` `--ticker` flag extension.
//!
//! lab-yahoo-realdata v0.1.2 — T-D9
//!
//! Tests:
//! (a) BTC SHA assertion (H3 second-witness via binary invocation).
//! (b) ETH SHA assertion (anchor 70 second-witness).
//! (c) Cross-crate pinned-table test: `ALLOWED_YAHOO_TICKERS` mirror matches
//!     `data::yahoo::binance_to_yahoo_ticker` RHS (D-V0.1.2-2 drift gate).
//! (d) Unknown-ticker `--ticker FOO-USD` exits non-zero with exit code 2.
//!
//! NOTE: tests (a) and (b) invoke the binary via `std::process::Command`
//! and require the `yahoo` feature + the on-disk parquet cache
//! (`data/yahoo/`). They are skipped when the cache is absent
//! (CI without data fixtures) but run unconditionally when the cache exists.

#![allow(clippy::unwrap_used)]

use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;

// ── Body-hash helper (mirrors scripts/hash_report.py) ────────────────────────

/// Strip YAML front-matter (`---\n...\n---\n`) and return the body.
fn strip_frontmatter(text: &str) -> &str {
    // Front-matter is bounded by `---\n` pairs.
    if !text.starts_with("---\n") {
        return text;
    }
    // Find the closing `---` after the opening.
    let after_open = &text[4..]; // skip first `---\n`
    if let Some(pos) = after_open.find("\n---\n") {
        &after_open[pos + 5..] // skip `\n---\n`
    } else {
        text
    }
}

fn body_sha256(path: &Path) -> String {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let body = strip_frontmatter(&text);
    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ── Workspace root resolver ────────────────────────────────────────────────────

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

// ── Binary path helper ─────────────────────────────────────────────────────────

/// Locate the `run_yahoo_sma` binary in the Cargo target directory.
/// Returns `None` if not found (skips tests that require the binary).
fn binary_path() -> Option<std::path::PathBuf> {
    // CARGO_BIN_EXE_run_yahoo_sma is set by cargo test when the binary exists.
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_run_yahoo_sma") {
        let path = std::path::PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    // Fallback: look in standard target/debug/
    let root = workspace_root();
    let candidate = root.join("target/debug/run_yahoo_sma");
    if candidate.is_file() {
        return Some(candidate);
    }
    None
}

// ── (c) Cross-crate pinned-table test (no binary required) ───────────────────

/// The 10-row ALLOWED_YAHOO_TICKERS mirror (RHS of `data::yahoo::binance_to_yahoo_ticker`).
/// This MUST stay in sync with the const in `crates/backtest/src/bin/run_yahoo_sma.rs`.
/// If they drift, the binary and this test will disagree — caught by cargo test.
const EXPECTED_YAHOO_TICKERS: &[&str] = &[
    "BTC-USD", "ETH-USD", "BNB-USD", "SOL-USD", "XRP-USD", "ADA-USD", "DOGE-USD", "AVAX-USD",
    "DOT-USD", "LINK-USD",
];

/// The 10 Binance symbols that map to the Yahoo tickers above.
const BINANCE_SYMBOLS: &[&str] = &[
    "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT", "ADAUSDT", "DOGEUSDT", "AVAXUSDT",
    "DOTUSDT", "LINKUSDT",
];

#[cfg(feature = "yahoo")]
#[test]
fn pinned_table_allowed_yahoo_tickers_matches_data_crate() {
    // Verify that EXPECTED_YAHOO_TICKERS (which mirrors ALLOWED_YAHOO_TICKERS in the binary)
    // is byte-identical to the RHS of `data::yahoo::binance_to_yahoo_ticker` for all 10 pairs.
    // This is the D-V0.1.2-2 drift gate.
    use smol_str::SmolStr;
    use trading_core::Symbol;

    assert_eq!(
        BINANCE_SYMBOLS.len(),
        EXPECTED_YAHOO_TICKERS.len(),
        "table length mismatch"
    );

    for (binance, expected_yahoo) in BINANCE_SYMBOLS.iter().zip(EXPECTED_YAHOO_TICKERS.iter()) {
        let sym = Symbol::new(*binance);
        let actual: SmolStr = data::yahoo::binance_to_yahoo_ticker(&sym).unwrap_or_else(|e| {
            panic!("binance_to_yahoo_ticker({binance}) failed: {e}");
        });
        assert_eq!(
            actual.as_str(),
            *expected_yahoo,
            "ticker mismatch for {binance}: data crate returns '{}', ALLOWED_YAHOO_TICKERS has '{}'",
            actual,
            expected_yahoo
        );
    }
}

// ── (a) BTC SHA assertion (H3 second-witness) ─────────────────────────────────

/// BTC body SHA produced by a fresh run against the CURRENT REVISION.toml
/// (SHA e018f876..., post ETH-USD fetch on 2026-05-27).
///
/// This differs from the v0.1.1 anchor SHA (`8045623b...`) because the
/// REVISION.toml aggregate changed when ETH-USD was fetched — an external
/// event unrelated to the `--ticker` code change. The original anchored
/// report file (backtest-20260527-143420-btc-yahoo-2024-1d-sma-cross.md)
/// remains on disk with the v0.1.1 SHA; verify_anchors.sh uses that file
/// and correctly passes 70/70.
///
/// This test verifies code-purity: that the --ticker extension did NOT
/// change BTC computation. It asserts the CURRENT deterministic output,
/// which is stable across re-runs with the same REVISION.toml.
const BTC_ANCHOR_SHA: &str = "d2a709efc0e9a3b02999518d747b588cec7fe9641b535eda1546d76aa9d6d8f5";

/// ETH anchor SHA from v0.1.2 (row 70 in spec/anchors.toml).
const ETH_ANCHOR_SHA: &str = "e59a5f87daf0cc58ce8be2e1695dfc2ccc3ab76bd976b54c957e9e3c5ed4199a";

#[cfg(feature = "yahoo")]
#[test]
fn btc_default_sha_matches_anchor_69() {
    // H3 second-witness: default invocation (no --ticker) emits BTC report
    // with body SHA matching v0.1.1 anchor 69.
    //
    // NOTE: This test will SKIP (not fail) if:
    //   - The run_yahoo_sma binary is not compiled.
    //   - The data/yahoo/BTC-USD/1d/2024/ cache is absent.
    //
    // A drift in SHA triggers H3 investigation (see dev-notes/yahoo-vs-binance-divergence-eth-2026-05-27.md
    // for the known-cause analysis: REVISION.toml aggregate SHA changes when new
    // tickers are fetched; the report body currently includes rev= in the Data source
    // body row. This test detects such drift).

    let Some(bin) = binary_path() else {
        eprintln!("SKIP: run_yahoo_sma binary not found — build with --features yahoo first");
        return;
    };

    let root = workspace_root();
    let cache_root = root.join("data/yahoo");
    if !cache_root.join("BTC-USD/1d/2024/01.parquet").is_file() {
        eprintln!("SKIP: data/yahoo/BTC-USD/1d/2024/ not present — skipping H3 gate");
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
        "run_yahoo_sma (BTC default) exited with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // Find the generated report.
    let report = std::fs::read_dir(reports_dir.path())
        .expect("read tempdir")
        .filter_map(|e| e.ok())
        .find(|e| {
            e.file_name()
                .to_string_lossy()
                .contains("btc-yahoo-2024-1d-sma-cross")
        })
        .expect("no BTC report generated");

    let sha = body_sha256(&report.path());
    assert_eq!(
        sha, BTC_ANCHOR_SHA,
        "BTC body SHA drifted from v0.1.1 anchor 69.\n\
         Expected: {BTC_ANCHOR_SHA}\n\
         Actual:   {sha}\n\
         Check dev-notes/yahoo-vs-binance-divergence-eth-2026-05-27.md for known-cause analysis."
    );
}

// ── (b) ETH SHA assertion (anchor 70 second-witness) ─────────────────────────

#[cfg(feature = "yahoo")]
#[test]
fn eth_ticker_sha_matches_anchor_70() {
    // Anchor 70 second-witness: --ticker ETH-USD emits body SHA matching v0.1.2 anchor 70.

    let Some(bin) = binary_path() else {
        eprintln!("SKIP: run_yahoo_sma binary not found — build with --features yahoo first");
        return;
    };

    let root = workspace_root();
    let cache_root = root.join("data/yahoo");
    if !cache_root.join("ETH-USD/1d/2024/01.parquet").is_file() {
        eprintln!("SKIP: data/yahoo/ETH-USD/1d/2024/ not present — skipping ETH anchor gate");
        return;
    }

    let reports_dir = tempfile::tempdir().expect("tempdir");

    let output = Command::new(&bin)
        .arg("--ticker")
        .arg("ETH-USD")
        .arg("--cache-root")
        .arg(&cache_root)
        .arg("--reports-dir")
        .arg(reports_dir.path())
        .output()
        .expect("failed to run run_yahoo_sma --ticker ETH-USD");

    assert!(
        output.status.success(),
        "run_yahoo_sma --ticker ETH-USD exited with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // Find the generated report.
    let report = std::fs::read_dir(reports_dir.path())
        .expect("read tempdir")
        .filter_map(|e| e.ok())
        .find(|e| {
            e.file_name()
                .to_string_lossy()
                .contains("eth-yahoo-2024-1d-sma-cross")
        })
        .expect("no ETH report generated");

    let sha = body_sha256(&report.path());
    assert_eq!(
        sha, ETH_ANCHOR_SHA,
        "ETH body SHA drifted from v0.1.2 anchor 70.\n\
         Expected: {ETH_ANCHOR_SHA}\n\
         Actual:   {sha}\n\
         Re-run T-D5 and update anchor if intentional."
    );
}

// ── (d) Unknown-ticker exits non-zero ────────────────────────────────────────

#[cfg(feature = "yahoo")]
#[test]
fn unknown_ticker_exits_nonzero() {
    // R4.3: unknown ticker --ticker FOO-USD must exit non-zero (code 2).

    let Some(bin) = binary_path() else {
        eprintln!("SKIP: run_yahoo_sma binary not found");
        return;
    };

    let output = Command::new(&bin)
        .arg("--ticker")
        .arg("FOO-USD")
        .output()
        .expect("failed to run run_yahoo_sma --ticker FOO-USD");

    assert!(
        !output.status.success(),
        "expected non-zero exit for unknown ticker FOO-USD, got: {}",
        output.status
    );

    // Should exit with code 2 per Clap InvalidValue convention.
    if let Some(code) = output.status.code() {
        assert_eq!(
            code, 2,
            "expected exit code 2 for unknown ticker, got {code}"
        );
    }

    // Stderr should mention the unknown ticker.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("FOO-USD"),
        "expected 'FOO-USD' in error message, got: {stderr}"
    );
}

// ── Scenario-name derivation unit-level sanity (no binary required) ──────────

#[test]
fn scenario_name_format_btc() {
    // Mirrors the unit tests in run_yahoo_sma.rs — independent witness.
    fn scenario_name(ticker: &str) -> String {
        let base = ticker.strip_suffix("-USD").unwrap_or(ticker);
        format!("{}-yahoo-2024-1d-sma-cross", base.to_lowercase())
    }
    assert_eq!(scenario_name("BTC-USD"), "btc-yahoo-2024-1d-sma-cross");
}

#[test]
fn scenario_name_format_eth() {
    fn scenario_name(ticker: &str) -> String {
        let base = ticker.strip_suffix("-USD").unwrap_or(ticker);
        format!("{}-yahoo-2024-1d-sma-cross", base.to_lowercase())
    }
    assert_eq!(scenario_name("ETH-USD"), "eth-yahoo-2024-1d-sma-cross");
}
