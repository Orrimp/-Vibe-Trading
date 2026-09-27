//! θ-surface reproduction gates — the first re-run coverage for the 34 anchored
//! parameter-robustness surfaces (bug-log `#93`, `#111`).
//!
//! ## What this closes
//!
//! `scripts/verify_anchors.sh` hashes the COMMITTED report bodies and never re-runs, so it
//! cannot see code-vs-evidence drift. Until now the 34 θ-surfaces — the single largest block
//! of the anchored corpus — had **no** re-run gate of any kind: `param_sweep_e2e.rs:870` pins
//! the scenario NAME, which catches a rename orphaning an anchor but says nothing about the
//! body. Story 1-26 re-priced all 34 on 2026-09-25; these gates are what notice if the code
//! walks away from them again.
//!
//! ## The condition, declared
//!
//! bug-log `#112`: a body-SHA comparison means nothing without its condition, and the
//! condition here is **(binary, features, CWD, out-dir, invocation)**:
//!
//! - `param_robustness_sweep`, **release**, `--features candle,realdata` (the binary refuses
//!   `--generator block-bootstrap-real` without `realdata` and says so);
//! - CWD = the workspace root, because the corpora resolve relative to it;
//! - the full invocation per surface comes from `scripts/relock/surfaces.tsv`, whose 34 rows
//!   are themselves derived from `GridKind` and gate-tested by `relock_manifest.rs` — so a
//!   grid change cannot silently diverge from what this gate runs;
//! - **`--out-dir` to a tempdir, always.** The binary's default out-dir points INSIDE the
//!   anchored corpus (story 1-26 AC3), and `verify_anchors.sh` resolves each row to the
//!   NEWEST match — so a gate that omitted it could plant a drifted body and flip the corpus
//!   gate as a side effect of running a test (bug-log `#113`).
//!
//! ## What this does NOT prove
//!
//! It proves the 34 bodies are reproducible from this machine's corpora. It does **not** make
//! them third-party reproducible: the three corpora (`data/binance`, `-funding`, `-basis`) are
//! gitignored, ~11 MB of parquet with only `REVISION.toml` tracked. On a fresh clone these
//! gates report **UNMEASURED**, never green — see `corpus_gated_theta_tests_are_declared`.

#![cfg(feature = "realdata")]
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

/// The two surfaces whose cost is an order of magnitude above the rest.
/// Measured 2026-09-25 during the 1-26 regeneration, at `RAYON_NUM_THREADS=8`.
const EXPENSIVE: [(&str, u32); 2] = [
    ("v1-momentum-theta-surface-2023-block-bootstrap-real-fy", 1461),
    ("v1-mr-theta-surface-2023-block-bootstrap-real-fy", 2258),
];

const MANIFEST: &str = "scripts/relock/surfaces.tsv";
const EXPECTED_SURFACES: usize = 34;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("locate workspace root")
        .to_path_buf()
}

/// `(scenario, argv)` for every surface in the manifest, in file order.
fn manifest_rows() -> Vec<(String, Vec<String>)> {
    let text = std::fs::read_to_string(workspace_root().join(MANIFEST))
        .unwrap_or_else(|e| panic!("read {MANIFEST}: {e}"));
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let mut it = l.split('\t');
            let scenario = it.next()?.to_string();
            let _bin = it.next()?;
            let args = it.next()?.split_whitespace().map(str::to_string).collect();
            Some((scenario, args))
        })
        .collect()
}

/// The anchored SHA for `scenario`, read out of `evidence/anchors.toml` by text scan.
///
/// Deliberately not a TOML parse: the same choice `strategy_anchors_unchanged.rs` documents —
/// fewer moving parts between the gate and the file everyone else treats as the source of
/// truth. Panics when the scenario has no row, because a surface in the manifest without an
/// anchor is a finding, not a skip.
fn anchored_sha(scenario: &str) -> String {
    let text = std::fs::read_to_string(workspace_root().join("evidence/anchors.toml"))
        .expect("read evidence/anchors.toml");
    let needle = format!("scenario = \"{scenario}\"");
    let tail = text
        .split(&needle)
        .nth(1)
        .unwrap_or_else(|| panic!("no anchors.toml row for {scenario}"));
    let sha_line = tail
        .lines()
        .find(|l| l.trim_start().starts_with("sha256"))
        .unwrap_or_else(|| panic!("no sha256 after the row for {scenario}"));
    sha_line
        .split('"')
        .nth(1)
        .unwrap_or_else(|| panic!("malformed sha256 line for {scenario}"))
        .to_string()
}

fn corpora_present() -> bool {
    let ws = workspace_root();
    ["data/binance", "data/binance-funding", "data/binance-basis"]
        .iter()
        .all(|d| ws.join(d).join("REVISION.toml").is_file())
}

/// Build the release sweep binary with the features its generator requires.
fn ensure_sweep_binary() -> PathBuf {
    let ws = workspace_root();
    let status = std::process::Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "backtest",
            "--bin",
            "param_robustness_sweep",
            "--features",
            "candle,realdata",
        ])
        .current_dir(&ws)
        .status()
        .expect("cargo build failed");
    assert!(status.success(), "building param_robustness_sweep failed");
    let bin = ws.join("target/release/param_robustness_sweep");
    assert!(bin.is_file(), "missing {}", bin.display());
    bin
}

/// Run one surface and return its body-SHA, or the binary's own refusal.
fn run_surface(bin: &Path, scenario: &str, args: &[String]) -> Result<String, String> {
    let ws = workspace_root();
    let out = tempfile::tempdir().expect("create out-dir tempdir");
    let output = std::process::Command::new(bin)
        .arg("--out-dir")
        .arg(out.path())
        .args(args)
        .current_dir(&ws)
        .output()
        .expect("spawn param_robustness_sweep");

    if !output.status.success() {
        let so = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let se = String::from_utf8_lossy(&output.stderr).trim().to_string();
        // bug-log #124: two empty streams plus a non-zero exit is a KILL, not a refusal.
        let hint = if so.is_empty() && se.is_empty() {
            " — nothing on either stream, so it was almost certainly killed (signal / OOM) \
             rather than refusing the run; re-run before treating this as drift"
        } else {
            ""
        };
        return Err(format!("{scenario}: exit {:?}{hint}\n{so}\n{se}", output.status));
    }

    let md = std::fs::read_dir(out.path())
        .expect("read out-dir")
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "md"))
        .ok_or_else(|| format!("{scenario}: no .md written"))?;

    let hash = std::process::Command::new("python3")
        .arg(ws.join("scripts/hash_report.py"))
        .arg(&md)
        .output()
        .expect("spawn hash_report.py");
    let text = String::from_utf8_lossy(&hash.stdout);
    Ok(text
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("{scenario}: hash_report.py printed nothing"))?
        .to_string())
}

/// Compare a set of surfaces against their anchors and report EVERY mismatch at once.
fn assert_surfaces_reproduce(which: &[(String, Vec<String>)]) {
    assert!(
        corpora_present(),
        "UNMEASURED, not passed: the θ-surface corpora are absent \
         (data/binance{{,-funding,-basis}}/REVISION.toml). Reporting a skip as a pass is \
         bug-log #113 requirement 6."
    );
    assert!(!which.is_empty(), "empty surface set — the manifest scan found nothing");

    let bin = ensure_sweep_binary();
    let mut drifted = Vec::new();
    let mut broke = Vec::new();

    for (scenario, args) in which {
        match run_surface(&bin, scenario, args) {
            Ok(got) => {
                let want = anchored_sha(scenario);
                if got != want {
                    drifted.push(format!("  {scenario}\n    expected {want}\n    got      {got}"));
                }
            }
            Err(why) => broke.push(format!("  {why}")),
        }
    }

    assert!(
        drifted.is_empty() && broke.is_empty(),
        "θ-surface reproduction FAILED over {} surface(s).\n\n\
         DRIFTED ({}):\n{}\n\nDID NOT RUN ({}):\n{}\n\n\
         Drift here is code-vs-evidence drift — invisible to verify_anchors.sh (bug-log #93). \
         Do NOT re-pin these anchors to the produced values; that converts a caught drift into \
         a silent one (bug-log #77). The resolution is an ADR-0038 § D6.b re-emission, worked \
         twice in docs/dev-notes/1-2{{6,7}}-d6b-re-emission-*.md.",
        which.len(),
        drifted.len(),
        if drifted.is_empty() { "  (none)".into() } else { drifted.join("\n") },
        broke.len(),
        if broke.is_empty() { "  (none)".into() } else { broke.join("\n") },
    );
}

// ── The gates ────────────────────────────────────────────────────────────────

/// R-THETA-1 — the 32 cheap surfaces reproduce their anchors. ~11 min.
#[test]
#[ignore = "corpus-gated + ~11 min: run with --ignored (see corpus_gated_theta_tests_are_declared)"]
fn theta_surfaces_cheap_reproduce_anchors() {
    let expensive: Vec<&str> = EXPENSIVE.iter().map(|(s, _)| *s).collect();
    let rows: Vec<_> = manifest_rows()
        .into_iter()
        .filter(|(s, _)| !expensive.contains(&s.as_str()))
        .collect();
    assert_eq!(
        rows.len(),
        EXPECTED_SURFACES - EXPENSIVE.len(),
        "expected {} cheap surfaces",
        EXPECTED_SURFACES - EXPENSIVE.len()
    );
    assert_surfaces_reproduce(&rows);
}

/// R-THETA-2 — the two expensive surfaces. **~62 min**, separate so the cheap block is usable.
#[test]
#[ignore = "corpus-gated + ~62 min for two surfaces (1461 s + 2258 s measured): run with --ignored"]
fn theta_surfaces_expensive_reproduce_anchors() {
    let expensive: Vec<&str> = EXPENSIVE.iter().map(|(s, _)| *s).collect();
    let rows: Vec<_> = manifest_rows()
        .into_iter()
        .filter(|(s, _)| expensive.contains(&s.as_str()))
        .collect();
    assert_eq!(rows.len(), EXPENSIVE.len(), "expected the 2 expensive surfaces");
    assert_surfaces_reproduce(&rows);
}

/// The companion that ALWAYS runs — corpus-independent, cheap, and the reason the two gates
/// above are allowed to be `#[ignore]`d.
///
/// Modelled on `dvol_bakeoff_path_gate.rs::corpus_gated_tests_are_declared_and_counted`, whose
/// docstring states the principle: *"It deliberately does NOT fail when the corpus is absent …
/// It fails when the skips become **invisible**."* A gate nobody can see skipping is
/// indistinguishable from one that passed.
#[test]
fn corpus_gated_theta_tests_are_declared() {
    let rows = manifest_rows();
    assert_eq!(
        rows.len(),
        EXPECTED_SURFACES,
        "{MANIFEST} has {} rows, expected {EXPECTED_SURFACES}. If the grid genuinely changed, \
         relock_manifest.rs is the gate that says so — fix the count here in the same commit.",
        rows.len()
    );

    // Every surface must have an anchor, or the gate above would silently cover fewer.
    for (scenario, args) in &rows {
        let sha = anchored_sha(scenario);
        assert_eq!(sha.len(), 64, "{scenario}: malformed anchor sha");
        assert!(!args.is_empty(), "{scenario}: manifest row has no argv");
    }

    let expensive: Vec<&str> = EXPENSIVE.iter().map(|(s, _)| *s).collect();
    for (s, _) in EXPENSIVE {
        assert!(
            rows.iter().any(|(r, _)| r == s),
            "EXPENSIVE names {s}, which is not in the manifest — the cost split is stale"
        );
    }
    let cheap = rows.len() - expensive.len();

    if corpora_present() {
        eprintln!(
            "[theta] ⚠ the corpora ARE on this machine, so the θ reproduction gates SHOULD be \
             run before any verdict:\n\
             \x20   cargo test -p backtest --features candle,realdata \
             --test theta_surface_reproduction -- --ignored\n\
             \x20   {cheap} cheap surfaces ≈ 11 min · {} expensive ≈ 62 min",
            expensive.len()
        );
    } else {
        eprintln!(
            "[skip] θ reproduction gates UNMEASURED on this machine: the corpora are gitignored \
             (~11 MB of parquet, only REVISION.toml tracked), so {} surfaces have no re-run \
             coverage here. This is not a pass.",
            rows.len()
        );
    }
}
