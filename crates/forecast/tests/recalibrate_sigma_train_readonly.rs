//! Read-only guard tests for the `recalibrate_sigma_train` bin (T-D-N5).
//!
//! Tests:
//! (a) `test_help_no_forbidden_flags` — `--help` output must NOT contain
//!     `retrain`, `update`, `write-checkpoint`, `update-sigma`.
//! (b) `test_originals_untouched_by_run` — mtime of the original
//!     `.metadata.json` and `.safetensors` files must be unchanged after
//!     a `--help` invocation.
//!
//! These tests run with the `candle` feature (so the binary exists).
//! They do NOT run a full forward pass — that's covered by the manual T-D-N3
//! acceptance run.
//!
//! # Cross-references
//!
//! - ADR-0035 D2 — hard invariant: original files stay byte-identical.
//! - T-D-N5 (decomp.md Wave A) — read-only enforcement gate.

use std::process::Command;

/// (c) `--help` output must NOT contain forbidden flags.
///
/// Forbidden: `retrain`, `update`, `write-checkpoint`, `update-sigma`.
/// Required: `--scenario`, `--data-root`, `--out-dir`, `--anchor-dir`.
#[test]
fn test_help_no_forbidden_flags() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "forecast",
            "--features",
            "candle",
            "--bin",
            "recalibrate_sigma_train",
            "--",
            "--help",
        ])
        .output()
        .expect("failed to spawn cargo run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    // Forbidden flags (K5 hard contract — ADR-0035 D2 + CLI surface).
    assert!(
        !combined.to_lowercase().contains("retrain"),
        "help output must not mention 'retrain'; got: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("update-sigma"),
        "help output must not mention 'update-sigma'; got: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("write-checkpoint"),
        "help output must not mention 'write-checkpoint'; got: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("update-original"),
        "help output must not mention 'update-original'; got: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("write-safetensors"),
        "help output must not mention 'write-safetensors'; got: {combined}"
    );

    // Required flags (D-AR-1.b from decomp.md).
    assert!(
        combined.contains("--scenario"),
        "help must contain --scenario; got: {combined}"
    );
    assert!(
        combined.contains("--data-root"),
        "help must contain --data-root; got: {combined}"
    );
    assert!(
        combined.contains("--out-dir"),
        "help must contain --out-dir; got: {combined}"
    );
    assert!(
        combined.contains("--anchor-dir"),
        "help must contain --anchor-dir; got: {combined}"
    );

    // Smoke: help must not be empty.
    assert!(
        combined.contains("recalibrate_sigma_train"),
        "help must contain the binary name; got: {combined}"
    );
}

/// (b) The four anchored checkpoint files are **byte-identical** across a `--help` invocation.
///
/// **This covers `--help` only, and now says so in its name.** Real runs are covered by
/// `crates/forecast/tests/anchored_report_reproduction.rs`, whose runner asserts `git status` over
/// the checkpoint directory across every run it makes.
///
/// Rewritten 2026-10-01, bug-log `#135`. The previous version could not fail, for three independent
/// reasons: `--help` exits in clap before any filesystem call; the sentinel literals were
/// **workspace**-relative while `cargo` runs an integration test from the **package** root, so all
/// four resolved to nothing; and a missing file mapped to `None`, which compares equal to `None`.
/// Any one alone was sufficient, which is why fixing one would not have revealed the others.
#[test]
fn test_originals_untouched_by_help_invocation() {
    let anchors_dir = workspace_root().join("crates/forecast/checkpoints/anchors");

    // Record mtimes before.
    let sentinel_paths: Vec<std::path::PathBuf> = vec![
        anchors_dir.join("tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.json"),
        anchors_dir.join("tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.json"),
        anchors_dir.join("tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.safetensors"),
        anchors_dir.join("tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.safetensors"),
    ];

    let bytes_before: Vec<Vec<u8>> = sentinel_paths.iter().map(|p| sentinel_bytes(p)).collect();

    // Run --help (should not touch any checkpoint files).
    let _ = Command::new("cargo")
        .args([
            "run",
            "-p",
            "forecast",
            "--features",
            "candle",
            "--bin",
            "recalibrate_sigma_train",
            "--",
            "--help",
        ])
        .output()
        .expect("failed to spawn cargo run");

    // Record mtimes after.
    let bytes_after: Vec<Vec<u8>> = sentinel_paths.iter().map(|p| sentinel_bytes(p)).collect();

    for (i, (path, (before, after))) in sentinel_paths
        .iter()
        .zip(bytes_before.iter().zip(bytes_after.iter()))
        .enumerate()
    {
        assert_eq!(
            before,
            after,
            "sentinel file #{i} ({}) CHANGED BYTES during the --help invocation",
            path.display()
        );
    }
}

/// `#134` tripwire, cheap and always running: the derivation report must be handed the path the run
/// **read**, never the path it **wrote**.
///
/// `recalibrate_sigma_train.rs:457` renders `"- Read-only against \`{…}\` original safetensors."`
/// into the HASHED body. Until 2026-10-01 it was fed `overlay_path` — the
/// `.metadata.recalibrated.json` this run creates three lines earlier (`:631`) — so both anchored
/// bodies claimed read-only status for the only file the run wrote, and called a JSON metadata
/// overlay *safetensors*.
///
/// This walks the call site rather than the rendered body because a rendered body costs eight
/// minutes; `body_is_anchor_dir_invariant` below is the real proof and this is the tripwire that
/// fires in a millisecond. It walks a DIFFERENT file than its own, which is why it needs no
/// `concat!` self-match guard (bug-log `#133`) — do not move it into the bin.
#[test]
fn report_is_handed_the_read_path_not_the_write_path() {
    let src = std::fs::read_to_string(
        workspace_root().join("crates/forecast/src/bin/recalibrate_sigma_train.rs"),
    )
    .expect("read recalibrate_sigma_train.rs");

    // Non-vacuity first: if the call site moved or was renamed, FAIL — do not pass quietly over a
    // file this test no longer understands (bug-log #113 req 6, and #135 mechanism 2's lesson).
    let call = src.split("let report = render_report(").nth(1).expect(
        "no `let report = render_report(` call site — this test no longer knows what it is \
         asserting, which is a failure, not a pass",
    );
    let args = call
        .split(");")
        .next()
        .expect("unterminated render_report( call");

    assert!(
        !args.contains("overlay_path"),
        "#134 regression: render_report is handed `overlay_path`, which is \
         `args.anchor_dir.join(…)` — the file the run WRITES. It reaches the hashed body through \
         the \"Read-only against …\" note, which then names the only artefact the run created.\n\
         Call-site args were:\n{args}"
    );
    assert!(
        !args.contains("anchor_dir"),
        "#134 regression: a `--anchor-dir`-derived value reaches the hashed body. That makes the \
         body record WHERE the run wrote instead of WHAT it measured, and re-blocks both \
         reproduction gates (#128e), because no tempdir then reproduces the anchor.\n\
         Call-site args were:\n{args}"
    );
    assert!(
        args.contains(".safetensors"),
        "#134 regression: render_report is no longer handed a `.safetensors` path. The note is \
         supposed to name the weights it read.\nCall-site args were:\n{args}"
    );
}

/// `#134`/`#128e` — the hashed body does not depend on `--anchor-dir`.
///
/// This is the invariant that both unblocks the two reproduction gates and proves the fix: two runs
/// into two **different** anchor-dirs must produce the same body-SHA. Before 2026-10-01 they could
/// not — the directory was interpolated into the Notes line — so the only `--anchor-dir` that
/// reproduced the anchor was the committed checkpoint dir, and the only available gate would have
/// had to rewrite the repo to run.
///
/// It also carries the read-only assertion `#135` had nowhere to put: the committed checkpoint
/// directory is compared **byte for byte** across two REAL runs, not across a `--help`.
///
/// Artefacts are written to `target/134-anchor-dir-invariance/` and deliberately **left there**:
/// when a body-SHA gate goes red, the body you need to diff is the one thing a tempdir has just
/// deleted.
///
/// ~16 min for bs1, ~20 min for bs2 (the anchored bodies' own `wall_clock_s`, doubled).
#[test]
#[ignore = "2 full forward passes per scenario, ~36 min for both: run with --ignored"]
fn body_is_anchor_dir_invariant() {
    let ws = workspace_root();
    assert!(
        ws.join("data/binance/REVISION.toml").is_file(),
        "UNMEASURED, not passed: needs data/binance/REVISION.toml and it is absent."
    );

    let status = std::process::Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "forecast",
            "--bin",
            "recalibrate_sigma_train",
            "--features",
            "candle",
        ])
        .current_dir(&ws)
        .status()
        .expect("spawn cargo build");
    assert!(status.success(), "building recalibrate_sigma_train failed");
    let bin = ws.join("target/release/recalibrate_sigma_train");

    let sentinels: Vec<std::path::PathBuf> =
        std::fs::read_dir(ws.join("crates/forecast/checkpoints/anchors"))
            .expect("read anchors dir")
            .flatten()
            .map(|e| e.path())
            .collect();
    assert!(
        !sentinels.is_empty(),
        "UNMEASURED: the checkpoint dir is empty, so the read-only half asserts nothing"
    );
    let mut sentinels: Vec<_> = sentinels;
    sentinels.sort();
    let before: Vec<Vec<u8>> = sentinels.iter().map(|p| sentinel_bytes(p)).collect();

    for scenario in ["bs1", "bs2"] {
        let mut shas = Vec::new();
        for leg in ["a", "b"] {
            let root = ws
                .join("target/134-anchor-dir-invariance")
                .join(scenario)
                .join(leg);
            let (anchor_dir, out_dir) = (root.join("anchors"), root.join("reports"));
            // A different anchor-dir per leg is the WHOLE point; equal dirs would make this pass
            // for the wrong reason.
            std::fs::create_dir_all(&anchor_dir).expect("create anchor-dir");
            std::fs::create_dir_all(&out_dir).expect("create out-dir");

            let out = std::process::Command::new(&bin)
                .args(["--scenario", scenario])
                .arg("--anchor-dir")
                .arg(&anchor_dir)
                .arg("--out-dir")
                .arg(&out_dir)
                .current_dir(&ws)
                .output()
                .expect("spawn recalibrate_sigma_train");
            assert!(
                out.status.success(),
                "{scenario}/{leg}: exit {:?}\nstdout: {}\nstderr: {}",
                out.status,
                String::from_utf8_lossy(&out.stdout).trim(),
                String::from_utf8_lossy(&out.stderr).trim()
            );

            let md = std::fs::read_dir(&out_dir)
                .expect("read out-dir")
                .flatten()
                .map(|e| e.path())
                .find(|p| p.extension().is_some_and(|x| x == "md"))
                .unwrap_or_else(|| panic!("{scenario}/{leg}: no .md written"));
            let hash = std::process::Command::new("python3")
                .arg(ws.join("scripts/hash_report.py"))
                .arg(&md)
                .output()
                .expect("spawn hash_report.py");
            let text = String::from_utf8_lossy(&hash.stdout);
            let sha = text
                .split_whitespace()
                .next()
                .expect("hash_report.py printed nothing")
                .to_string();
            eprintln!("[#134] {scenario}/{leg}: {sha}  {}", md.display());
            shas.push(sha);
        }
        assert_eq!(
            shas[0], shas[1],
            "#134/#128e: `{scenario}`'s hashed body DEPENDS on --anchor-dir. Two runs into two \
             different anchor-dirs produced {} and {}. The body is recording WHERE the run wrote \
             instead of WHAT it measured, so no tempdir reproduces the anchor and the row cannot \
             be gated without mutating the repo.",
            shas[0], shas[1]
        );
    }

    let after: Vec<Vec<u8>> = sentinels.iter().map(|p| sentinel_bytes(p)).collect();
    assert_eq!(
        before, after,
        "the committed checkpoint directory CHANGED across four real runs. With --anchor-dir \
         pointed at a tempdir the bin must not write there at all — this is the read-only \
         assertion the --help guard could never make (bug-log #135)."
    );
}

// ── `#135` plumbing ──────────────────────────────────────────────────────────
//
// Identical in all three read-only guards (`recalibrate_sigma_train_readonly.rs`,
// `threshold_sweep_readonly.rs`, `forecast_distribution_bin_readonly.rs`). Kept local and
// deliberately identical rather than shared through a crate: it is a file read, not a measurement
// harness, so the Dev-Note hazard about re-implementing a harness twice does not apply — but if you
// change one, change all three.

/// The workspace root. `cargo` runs an integration test from the PACKAGE root, so a
/// workspace-relative literal silently resolves to nothing — bug-log `#135` mechanism 2, which made
/// all four sentinels missing on every run of this file for months.
fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("locate workspace root")
        .to_path_buf()
}

/// Read a sentinel, PANICKING when it is absent.
///
/// bug-log `#135` mechanism 3: the previous version mapped a missing file to `None` and then
/// compared `None` to `None`, so the test was green with the checkpoint directory deleted. An
/// absent sentinel is a finding, never a silent equality.
fn sentinel_bytes(path: &std::path::Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| {
        panic!(
            "sentinel {} is MISSING or unreadable ({e}). This is UNMEASURED, not a pass: the \
             read-only contract cannot be checked against a file that is not there (bug-log #135).",
            path.display()
        )
    })
}
