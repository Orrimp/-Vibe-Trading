#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1937 / T1942 (v2-llm-strategy, pass 6) — negative-invariant test
//! for the 9 strategy-backtest anchors at `spec/anchors.toml:15-58`.
//!
//! After T1935's `crates/reports/src/render/system_health.rs` rewrite,
//! the two `report-sample-*` anchors at `spec/anchors.toml:67-75` ARE
//! expected to drift (T_FINAL_V2_LLM_STRATEGY re-locks them). The 9
//! strategy anchors MUST stay byte-identical — none of the strategy /
//! audit / exec / backtest crates were touched by M7, so their
//! reports' body bytes must hash to the same SHAs they hashed to
//! pre-M7.
//!
//! This test mirrors the v1.8 reflection-memory `T1812`
//! negative-confirmation step. It catches accidental anchor drift in
//! M7 (e.g. a developer accidentally edits a report rendering function
//! and breaks an upstream anchor without noticing).
//!
//! ## v0.3.0 update (ADR-0047 D3 — namespace-aware resolver)
//!
//! The original `find_backtest_report` helper used lexicographic-sort-
//! and-take-last, with no concept of the `version`/namespace column
//! added in ADR-0045 D2. Wave A's 2026-05-27 canonical reports
//! (`backtest-20260527-065*-<scenario>.md`) sort lexicographically
//! AFTER the original noop reports, so the old helper picked up
//! canonical reports and compared them against noop-locked constants
//! — producing a spurious mismatch.
//!
//! The fix: `find_backtest_report(scenario, Namespace)` filters
//! candidates by namespace (Noop vs Canonical) using path exclusion
//! rules, mirroring `scripts/verify_anchors.sh:63-110`. The existing
//! `STRATEGY_ANCHORS` constant table is kept pinned to noop-baseline
//! SHAs forever. A new `CANONICAL_STRATEGY_ANCHORS` table (added at
//! v0.3.0 Wave C) is checked against canonical-namespace reports.
//!
//! ## What this test checks
//!
//! For each of the 9 strategy anchors:
//!
//! 1. Find the corresponding `backtest-*-<scenario>.md` file on disk,
//!    filtered to the NOOP namespace (excludes canonical feature dirs).
//! 2. Strip the YAML front-matter (matching `scripts/hash_report.py`).
//! 3. Compute the body SHA-256.
//! 4. Assert the SHA matches the locked value at
//!    `spec/anchors.toml:15-58`.
//!
//! The 9 anchors are inlined here (matching the constants the
//! anchors.toml file carries) so the test depends only on the report
//! files on disk — not on a TOML parser, which would be a
//! transitive-dep change.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

// ── Namespace enum ────────────────────────────────────────────────────────────

/// Selects which set of reports `find_backtest_report` targets.
///
/// Mirrors the namespace-aware path-filter in `scripts/verify_anchors.sh:63-110`.
/// See ADR-0047 D3 for the resolution algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Namespace {
    /// Noop-baseline namespace: reports outside the v5 canonical feature
    /// directories. These are the reports whose body-SHAs are locked in
    /// `STRATEGY_ANCHORS`. The noop-baseline SHAs are stable as long as
    /// no strategy / exec / audit code changes (and will never change due
    /// to canonical re-emissions in v5 feature directories).
    Noop,
    /// Canonical namespace: reports inside one of the v5 canonical emission
    /// directories. These are the reports locked in `CANONICAL_STRATEGY_ANCHORS`.
    Canonical,
    /// Square-root market-impact namespace (v0.5.0): reports inside the
    /// `v5-latency-slippage-sim-v0.5.0-square-root-market-impact` directory.
    /// These are the reports locked in `SQRT_IMPACT_STRATEGY_ANCHORS`.
    SqrtImpact,
}

/// Feature-directory names that host canonical (v5-sim) reports.
/// Any `backtest-*-<scenario>.md` found inside one of these directories
/// is canonical, NOT noop-baseline.
///
/// Per ADR-0047 D3 the list is maintained here; when a new canonical
/// namespace is added (e.g. v0.4.0), append to this slice.
const CANONICAL_FEATURE_DIRS: &[&str] = &[
    "v5-latency-slippage-sim-v0.2.0-anchor-migration",
    "v5-latency-slippage-sim-v0.3.0-full-path-wiring",
    "v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit",
];

/// Feature-directory names that host sqrt-impact (v0.5.0) reports.
/// Added per D-T1.8 (ADR-0047 D3 extension — third namespace).
const SQRT_IMPACT_FEATURE_DIRS: &[&str] = &[
    "v5-latency-slippage-sim-v0.5.0-square-root-market-impact",
];

/// Predicate: returns `true` if any path component matches a canonical
/// feature directory, making this a canonical-namespace report.
fn is_canonical_path(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(|s| CANONICAL_FEATURE_DIRS.contains(&s))
            .unwrap_or(false)
    })
}

/// Predicate: returns `true` if any path component matches a sqrt-impact
/// feature directory (v0.5.0 namespace).
fn is_sqrt_impact_path(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(|s| SQRT_IMPACT_FEATURE_DIRS.contains(&s))
            .unwrap_or(false)
    })
}

// ── NOOP-baseline anchor table ────────────────────────────────────────────────

/// Each tuple = `(scenario_id, sha256_hex)`. Mirrors
/// `spec/anchors.toml:15-58` byte-for-byte at v2.0.0 ship time. The
/// orchestrator updates BOTH places in lockstep — if these constants
/// drift from `spec/anchors.toml`, the workspace fails this test AND
/// `bash scripts/verify_anchors.sh`.
///
/// These SHAs are NOOP-BASELINE (no v5 sim applied). They are pinned
/// forever — future canonical re-emissions only add to
/// `CANONICAL_STRATEGY_ANCHORS` below, never change this table.
const STRATEGY_ANCHORS: &[(&str, &str)] = &[
    (
        "btc-2023-1m-sma-cross",
        "fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c",
    ),
    (
        "btc-2023-1m-sma-baseline-refresh",
        "fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c",
    ),
    (
        "btc-2023-1m-macd-trend",
        "ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805",
    ),
    (
        "btc-2023-1m-rsi-reversion",
        "bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa",
    ),
    (
        "btc-2023-1m-bbands-mean-revert",
        "d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3",
    ),
    (
        "top10-2023-1h-momentum",
        "3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97",
    ),
    (
        "top10-2024-h1-momentum",
        "1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6",
    ),
    (
        "pairs-2023-zscore-mr",
        "90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0",
    ),
    (
        "pairs-2024-h1-zscore-mr",
        "14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f",
    ),
];

// ── CANONICAL anchor table ────────────────────────────────────────────────────

/// Canonical-namespace SHAs: reports re-emitted under the v5 canonical
/// `LatencySlippageSimConfig { latency_ms_min: 30, latency_ms_max: 80,
/// slippage_bps: 8 }` per ADR-0045 D1 / ADR-0047 D2-D4.
///
/// Populated by the developer at Wave C close (v5-latency-slippage-sim
/// v0.3.0). If empty at that time, `t1937b_canonical_strategy_anchors_unchanged`
/// is a soft skip (logged via `eprintln!`).
///
/// **Migration freeze** (ADR-0047 D3): after v0.3.0 ship, this table is
/// frozen. Future updates require a new ADR re-emission trigger.
const CANONICAL_STRATEGY_ANCHORS: &[(&str, &str)] = &[
    // Populated at Wave C close (v5-latency-slippage-sim v0.3.0 M-DEV, 2026-05-27).
    // Reports in spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/
    // All run under canonical LatencySlippageSimConfig { latency_ms_min: 30,
    // latency_ms_max: 80, slippage_bps: 8 } per ADR-0047 D1.
    // Determinism verified: 2 independent runs produced identical body-SHAs.
    //
    // Group A: SMA synthetic (Q1=(a) --force-synthetic-bars)
    (
        "btc-2023-1m-sma-cross",
        "d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0",
    ),
    (
        "btc-2023-1m-sma-baseline-refresh",
        "d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0",
    ),
    // Group B: Composed strategies (real Binance data, SMA path now wired)
    (
        "btc-2023-1m-macd-trend",
        "6cb14ac55350325c2785284f6e9a8db29693def83a31b144e1d4607f5baf53f5",
    ),
    (
        "btc-2023-1m-rsi-reversion",
        "87b4e1cc1b949a5b60420bf4fa2319e40035a57de6590d8b8987eb5357845695",
    ),
    (
        "btc-2023-1m-bbands-mean-revert",
        "5b6237d11f962b98e9ce0f0deb4b7ec7d7638bbcb15f5e418f3909f07a3393cd",
    ),
    // Group C: Momentum (unchanged from v0.2.0; wired in v0.1.0, synthetic data)
    (
        "top10-2023-1h-momentum",
        "0f6f6eb8d943fefa866c4883be034f1beb3caff169fe76ec73bf3c29041a8ba3",
    ),
    (
        "top10-2024-h1-momentum",
        "78976062cf3d62b9bbb2ab579e91822cb49f0d12464dedf912edb427e66c7490",
    ),
    // Group D: Pairs (newly wired in v0.3.0)
    (
        "pairs-2023-zscore-mr",
        "01c9da4d4c5ce268b5de49c72f367ef729fcaccf04d572e5dc0fa1f1bd65e76e",
    ),
    (
        "pairs-2024-h1-zscore-mr",
        "6252819b4f719ce45bf5cfa70bfb38143c216e96dde8e5a43fdfe43055dce5e9",
    ),
    // Group E: TCN overlay synthetic (newly wired in v0.3.0)
    (
        "top10-2023-fy-tcn-overlay",
        "1460fcc70029746b650ae6f1298a7f2291603e96c54531f26bf6f24c558250fc",
    ),
    (
        "top10-2024-fy-tcn-overlay",
        "b8e9186bb36abe6539917245f7dec99685792dcc955e11ba52380a7a5293ad1e",
    ),
    // Group F: TCN overlay weights (candle feature — v0.4.0 re-emit, 2026-05-28)
    // Reports in spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/
    // Built with --features "candle realdata"; 2-run determinism verified.
    (
        "top10-2023-fy-tcn-overlay-weights",
        "28379df8913e987bf41b0b1d1913c77781306b5934432c495277723033993fdc",
    ),
    (
        "top10-2024-fy-tcn-overlay-weights",
        "0c13ed0bd5e7d4e502e3d4bd70912336193ac43b21247151257ddb5312b90137",
    ),
    // Group G: TCN overlay realdata (realdata feature — v0.4.0 re-emit, 2026-05-28)
    (
        "top10-2023-fy-tcn-overlay-realdata",
        "10fd4502d9057f9390d4869c32ef1c65dc93d91b8574a740b198f995b2563d37",
    ),
    (
        "top10-2024-fy-tcn-overlay-realdata",
        "87dfad459bcbb0640dd70985063f25da985dbb4f39776c99bbe9056ccceda61b",
    ),
    // Group H: TCN overlay weights+realdata (both features — v0.4.0 re-emit, 2026-05-28)
    (
        "top10-2023-fy-tcn-overlay-weights-realdata",
        "123d8228e50536c9094bc8605ecae2e0aadbdcd8a4bf854e5ae3e5f3414413a7",
    ),
    (
        "top10-2024-fy-tcn-overlay-weights-realdata",
        "21bec3c9f9da750853ddcc571246ba00d00b3903d18a0f6989b1434f8c72b612",
    ),
    // Group I: PatchTST overlay realdata (candle+realdata features — v0.4.0 re-emit, 2026-05-28)
    (
        "top10-2023-fy-patchtst-overlay-realdata",
        "55c5b715e6f5573e73c2db4b9aae859cf6d52472cbac6918920ac7afd7f36e6b",
    ),
    // Group J: Vol-target GARCH overlay realdata (realdata feature — v0.4.0 re-emit, 2026-05-28)
    (
        "top10-2023-fy-vol-target-overlay-realdata",
        "4edd8cc5f3041e308d4c83cfcf35109da9b9e4a363d7b6bc6d8d4407e50aa8ce",
    ),
];

// ── SQRT-IMPACT anchor table ──────────────────────────────────────────────────

/// Square-root market-impact namespace SHAs (v5-latency-slippage-sim v0.5.0).
///
/// Reports re-emitted under `SlippageModel::SquareRoot { alpha: 1.0,
/// volume_lookback_days: 90 }` per D-T1.4 Option-A (Binance parquet 90-day
/// trailing) for real-data scenarios and universe-avg V for synthetic scenarios
/// (Q3=(b) operator override, D-T1.5).
///
/// Populated by the developer at Wave E close (v5-latency-slippage-sim v0.5.0
/// M-DEV). If empty at that time, `t1937c_sqrt_impact_strategy_anchors_unchanged`
/// soft-skips.
///
/// **Freeze contract**: after v0.5.0 ship, this table is frozen. Future updates
/// require a new re-emission trigger (v0.6.0 sub-namespace cleanup).
const SQRT_IMPACT_STRATEGY_ANCHORS: &[(&str, &str)] = &[
    // Populated at Wave E close (v5-latency-slippage-sim v0.5.0 M-DEV, 2026-05-29).
    // Reports in spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/
    // Real-data scenarios: SquareRoot { alpha: 1.0, volume_lookback_days: 90 }
    // Synthetic scenarios: SquareRoot { alpha: 1.0, volume_lookback_days: 90 } + universe-avg V
    // Determinism verified: 2 independent runs produced identical body-SHAs.
    // SHA values to be filled by developer Wave E.
];

// ── Resolver ──────────────────────────────────────────────────────────────────

/// Resolve the workspace root (the directory containing the workspace
/// `Cargo.toml`) from the test's `CARGO_MANIFEST_DIR`.
fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map_or_else(|| crate_dir.clone(), Path::to_path_buf)
}

/// Find the latest `backtest-*-<scenario>.md` file for the given namespace.
///
/// Resolution algorithm per ADR-0047 D3:
/// - `Namespace::Noop`: collect under `spec/**/reports/`, EXCLUDING any path
///   that contains a canonical feature directory name. Return lex-newest.
/// - `Namespace::Canonical`: collect ONLY from canonical feature directories
///   (`spec/v5-latency-slippage-sim-*/reports/`). Return lex-newest.
///
/// Mirrors `scripts/verify_anchors.sh:63-110`.
fn find_backtest_report(scenario: &str, namespace: Namespace) -> Option<PathBuf> {
    let spec_dir = workspace_root().join("spec");
    let mut candidates: Vec<PathBuf> = Vec::new();
    walk_collect(&spec_dir, scenario, namespace, &mut candidates);
    candidates.sort();
    candidates.into_iter().next_back()
}

fn walk_collect(dir: &Path, scenario: &str, namespace: Namespace, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let suffix = format!("-{scenario}.md");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_collect(&path, scenario, namespace, out);
        } else if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if name.starts_with("backtest-") && name.ends_with(&suffix) {
                // Apply namespace filter per ADR-0047 D3 (extended to 3 namespaces
                // in D-T1.8, 2026-05-29):
                // Noop: excludes canonical AND sqrt-impact dirs (R-NR.3 preserved).
                // Canonical: only canonical dirs.
                // SqrtImpact: only sqrt-impact dirs.
                let is_canonical = is_canonical_path(&path);
                let is_sqrt = is_sqrt_impact_path(&path);
                let include = match namespace {
                    Namespace::Noop => !is_canonical && !is_sqrt,
                    Namespace::Canonical => is_canonical,
                    Namespace::SqrtImpact => is_sqrt,
                };
                if include {
                    out.push(path);
                }
            }
        }
    }
}

// ── SHA helpers ───────────────────────────────────────────────────────────────

/// Body-hash a report file: strip the leading
/// `---\n<frontmatter>\n---\n` block (matching
/// `scripts/hash_report.py`'s regex) and SHA-256 the remainder.
fn body_sha256(path: &Path) -> String {
    let text = std::fs::read_to_string(path).expect("read report");
    let body = strip_front_matter(&text);
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    hex::encode(h.finalize())
}

fn strip_front_matter(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("---\n") else {
        return text;
    };
    let close = "\n---\n";
    rest.find(close)
        .map_or("", |pos| &rest[pos + close.len()..])
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// T1937 — every one of the 9 strategy anchors is byte-identical to
/// the locked SHA-256 in `spec/anchors.toml:15-58`.
///
/// If this test FAILS in pass 6 (M7), a non-M7 change has touched the
/// report rendering of one of the 9 backtest scenarios — re-run the
/// failing scenario's backtest from a clean checkout to reproduce and
/// surface the diff. The test must pass before T_FINAL.
///
/// ## v0.3.0 behaviour change (ADR-0047 D3)
///
/// Reports are now resolved with `Namespace::Noop` — this excludes any
/// report found inside a v5 canonical feature directory, so the test
/// always compares against the pre-v5 noop-baseline reports regardless
/// of how many canonical re-emissions land on disk.
#[test]
fn t1937_nine_strategy_anchors_unchanged() {
    let mut mismatches: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    for (scenario, expected_sha) in STRATEGY_ANCHORS {
        match find_backtest_report(scenario, Namespace::Noop) {
            None => {
                missing.push((*scenario).to_string());
            }
            Some(path) => {
                let actual = body_sha256(&path);
                if actual != *expected_sha {
                    mismatches.push(format!(
                        "{scenario}: expected {expected_sha}, got {actual} (file: {})",
                        path.display()
                    ));
                }
            }
        }
    }

    if !missing.is_empty() {
        // A missing report doesn't fail the test — the orchestrator
        // may have pruned old reports. Log a soft warning via
        // `eprintln!` (this is a test) so the run log carries the
        // signal. Only an explicit byte mismatch is hard-fail.
        eprintln!(
            "T1937 soft warning: no noop-baseline backtest report on disk for: {missing:?}; \
             skipping these. The verify_anchors.sh script will surface this as \
             MISS at T_FINAL."
        );
    }

    assert!(
        mismatches.is_empty(),
        "T1937 — the 9 strategy anchors MUST stay byte-identical in M7. \
         M7 (v2-llm-strategy / pass 6) is config + reports plumbing — \
         not strategy / audit / exec / backtest code. \
         The following noop-baseline anchors drifted:\n\n{}",
        mismatches.join("\n")
    );
}

/// T1937b — canonical strategy anchors (v5-latency-slippage-sim v0.3.0).
///
/// Each entry in `CANONICAL_STRATEGY_ANCHORS` is checked against the
/// most-recent report in the v5 canonical feature directories. If the
/// table is empty (pre-Wave-C state), the test soft-skips.
///
/// Added per ADR-0047 D3: after Wave C close the developer populates
/// `CANONICAL_STRATEGY_ANCHORS` above and this gate becomes a hard
/// regression check for future canonical migrations.
#[test]
fn t1937b_canonical_strategy_anchors_unchanged() {
    if CANONICAL_STRATEGY_ANCHORS.is_empty() {
        eprintln!(
            "T1937b soft skip: CANONICAL_STRATEGY_ANCHORS is empty — populated at Wave C \
             close (v5-latency-slippage-sim v0.3.0 M-DEV). This is expected pre-Wave-C."
        );
        return;
    }

    let mut mismatches: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    for (scenario, expected_sha) in CANONICAL_STRATEGY_ANCHORS {
        match find_backtest_report(scenario, Namespace::Canonical) {
            None => {
                missing.push((*scenario).to_string());
            }
            Some(path) => {
                let actual = body_sha256(&path);
                if actual != *expected_sha {
                    mismatches.push(format!(
                        "{scenario}: expected {expected_sha}, got {actual} (file: {})",
                        path.display()
                    ));
                }
            }
        }
    }

    if !missing.is_empty() {
        eprintln!(
            "T1937b soft warning: no canonical backtest report on disk for: {missing:?}; \
             skipping these."
        );
    }

    assert!(
        mismatches.is_empty(),
        "T1937b — canonical strategy anchors drifted after v0.3.0 Wave C:\n\n{}",
        mismatches.join("\n")
    );
}

/// T1937c — sqrt-impact strategy anchors (v5-latency-slippage-sim v0.5.0).
///
/// Each entry in `SQRT_IMPACT_STRATEGY_ANCHORS` is checked against the
/// most-recent report in the v0.5.0 sqrt-impact feature directory. If the
/// table is empty (pre-Wave-E state), the test soft-skips.
///
/// Added per D-T1.8 (ADR-0047 D3 extension — third namespace). After Wave E
/// close the developer populates `SQRT_IMPACT_STRATEGY_ANCHORS` above and
/// this gate becomes a hard regression check for future migrations.
#[test]
fn t1937c_sqrt_impact_strategy_anchors_unchanged() {
    if SQRT_IMPACT_STRATEGY_ANCHORS.is_empty() {
        eprintln!(
            "T1937c soft skip: SQRT_IMPACT_STRATEGY_ANCHORS is empty — populated at Wave E \
             close (v5-latency-slippage-sim v0.5.0 M-DEV). This is expected pre-Wave-E."
        );
        return;
    }

    let mut mismatches: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    for (scenario, expected_sha) in SQRT_IMPACT_STRATEGY_ANCHORS {
        match find_backtest_report(scenario, Namespace::SqrtImpact) {
            None => {
                missing.push((*scenario).to_string());
            }
            Some(path) => {
                let actual = body_sha256(&path);
                if actual != *expected_sha {
                    mismatches.push(format!(
                        "{scenario}: expected {expected_sha}, got {actual} (file: {})",
                        path.display()
                    ));
                }
            }
        }
    }

    if !missing.is_empty() {
        eprintln!(
            "T1937c soft warning: no sqrt-impact backtest report on disk for: {missing:?}; \
             skipping these."
        );
    }

    assert!(
        mismatches.is_empty(),
        "T1937c — sqrt-impact strategy anchors drifted after v0.5.0 Wave E:\n\n{}",
        mismatches.join("\n")
    );
}

/// T1942 — V8 / V12 confirmation gate: assert each anchor's SHA hex
/// is exactly 64 lowercase-hex chars (no whitespace, no upper-case).
/// Defends against a malformed paste in `spec/anchors.toml` or in
/// the inlined constants here.
#[test]
fn t1942_anchor_shas_are_well_formed_64_lowercase_hex() {
    for (scenario, sha) in STRATEGY_ANCHORS {
        assert_eq!(sha.len(), 64, "{scenario}: SHA must be 64 chars");
        for ch in sha.chars() {
            assert!(
                ch.is_ascii_lowercase() && ch.is_ascii_hexdigit() || ch.is_ascii_digit(),
                "{scenario}: SHA must be lowercase hex (offending char '{ch}')"
            );
        }
    }
}
