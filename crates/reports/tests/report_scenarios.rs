#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T816 — `report-sample-7d` + `report-sample-90d` scenario tests.
//!
//! Drives both report scenarios end-to-end through `lib::generate`,
//! captures their body-SHA256, and asserts:
//!
//! 1. **Determinism (V4 / R10.3)** — same fixture + same seed run twice
//!    sequentially produces byte-identical body bytes (post-front-matter
//!    fence, hashed exactly the way `scripts/hash_report.py` and the
//!    9-anchor regression gate hash backtest reports).
//! 2. **Anchor lock** — the captured body-SHA256 matches the expected
//!    hex constant.  When the SHA legitimately rotates (e.g. a render
//!    change) the constant updates here AND the entry in
//!    `spec/anchors.toml` rotates with the same value — the test owns
//!    both gates.
//! 3. **Cron-friendliness (V10)** — three concurrent renders against
//!    the same fixture targeting the same canonical output path each
//!    exit `Ok(_)` and produce byte-identical bodies.  The atomic-write
//!    helper guarantees the canonical path is always either absent or
//!    a complete file (the test polls the path while the renders run
//!    and asserts no file ever observes a partial body).
//!
//! The reports land under `spec/v1/operator-success-reports/reports/success-<run_id>-<scenario>.md`
//! so `scripts/verify_anchors.sh` (after its glob extension to `success-*-`)
//! can pick them up alongside the 9 backtest anchors → 11/11.
//!
//! ## Why `ReportWindow::Since(...)` instead of `Days7` / `Days90`
//!
//! `lib::generate` uses `OffsetDateTime::now_utc()` to derive
//! `period_end`, then resolves `period_start = period_end -
//! window_duration`.  Two runs at different wall-clocks therefore
//! produce DIFFERENT `period_start` strings even at the same `--period`,
//! and any timestamp emitted into the body (R8 lifecycle event ts via
//! `format_event`) would drift with wall-clock — defeating the
//! "stable SHA across days" anchor contract.  Using
//! `ReportWindow::Since(<fixed>)` pins `period_start` to a known
//! RFC-3339 string; `period_end = now` still drifts, but the only
//! body-side consumer of `period_end` is the equity-curve sampler,
//! whose output passes through R3's fixed-width sparkline encoder
//! (60 cells regardless of input length) — so the body-SHA stays
//! stable.  The slug rendered into R4's `Period` column reads
//! `since:2026-...` instead of `7d` / `90d`, but that string is
//! **fixed** under the anchor — the spec's `report-sample-7d` /
//! `report-sample-90d` names are scenario identifiers, not literal
//! `--period` arguments.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use reports::{FrozenMarkSource, ReportWindow};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[path = "fixtures/build_ledger_7d.rs"]
mod build_ledger_7d;
#[path = "fixtures/build_ledger_90d.rs"]
mod build_ledger_90d;

use crate::build_ledger_7d::{
    FIXTURE_SEED, build_ledger_7d, fixture_period_end as fixture_7d_period_end,
    fixture_period_start as fixture_7d_period_start,
};
use crate::build_ledger_90d::{
    build_ledger_90d, fixture_period_end as fixture_90d_period_end,
    fixture_period_start as fixture_90d_period_start,
};

/// Locked body-SHA256 for `report-sample-7d`.  Captured at first
/// successful local run on 2026-05-01; mirrored in `spec/anchors.toml`
/// under the matching `[[anchors]]` entry.
///
/// **Rotation policy:** if a render-side change legitimately shifts
/// the body, this constant moves AND the anchors.toml entry moves —
/// in the same commit.  Architect approval required (per
/// `spec/anchors.toml` ownership note).
// T1810 / T1813 — re-captured post-reflection-memory renderer rewrite
// (R5.4 re-anchor procedure).  The reflection-memory empty-state body
// supersedes the v1+ placeholder body; the new SHA was captured at the
// developer's first deterministic local run on 2026-05-08 against the
// FIXTURE_SEED = 0xC0FFEE fixtures.  spec/anchors.toml lines 67–75
// receive the same value at T_FINAL_REFLECTION_MEMORY (tester only).
//
// T1935 / T1936 (v2-llm-strategy, pass 6) — re-captured post-System-
// Health rewrite (Q11 denominator `$135 → $200` + Q5d `Cache hit
// ratio` row). New SHAs captured at the developer's first deterministic
// local run on 2026-05-12 against the FIXTURE_SEED = 0xC0FFEE
// fixtures. spec/anchors.toml lines 67–75 receive these values at
// T_FINAL_V2_LLM_STRATEGY (tester only).
const EXPECTED_SHA_7D: &str = "520b1f2968ad52d5981a1cdb3749235416c77c058364bd8c11ebd7d2468f46a3";

/// Locked body-SHA256 for `report-sample-90d`.  Same ownership notes
/// as [`EXPECTED_SHA_7D`].
const EXPECTED_SHA_90D: &str = "c656414ebf6f526372c27ae2d537301c68a0bc71d896f5a7cbc65a02edd60333";

/// Slice off the front-matter and return the body bytes the
/// `scripts/hash_report.py` regex considers the "body".
///
/// `hash_report.py` strips `^---\n.*?\n---\n` (DOTALL) from the file —
/// note the regex stops AFTER the closing `---\n`, so the body slice
/// starts at the immediately-following byte (typically `\n## ...`).
/// The earlier draft of this helper trimmed an extra `\n` boundary to
/// match the t814_body_no_volatile_metadata `body_after_fence` helper;
/// the 9-anchor regression gate uses the python convention, so this
/// test must too — diverging would silently miss real anchor drift.
fn body_after_fence(full: &str) -> &str {
    // Find the closing fence: the first `\n---\n` after the opening
    // `---\n`.  Body starts immediately after that closing newline.
    let Some(rest) = full.strip_prefix("---\n") else {
        return full;
    };
    let close_marker = "\n---\n";
    rest.find(close_marker)
        .map_or("", |pos| &rest[pos + close_marker.len()..])
}

fn body_sha256_hex(body: &str) -> String {
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    hex::encode(h.finalize())
}

/// Scenario render — invokes `lib::generate` against the prepared
/// fixture and returns the body bytes (hashable slice).
async fn render_scenario(
    db_path: &Path,
    out_path: &Path,
    period_start: trading_core::Timestamp,
) -> String {
    let frozen = FrozenMarkSource::from_csv_str("symbol,close_time,close\n").unwrap();
    reports::generate(
        ReportWindow::Since(period_start),
        db_path,
        &frozen,
        out_path,
        Some(FIXTURE_SEED),
    )
    .await
    .expect("render should succeed on a balanced fixture");
    let full = std::fs::read_to_string(out_path).expect("read rendered report");
    body_after_fence(&full).to_string()
}

// NOTE (2026-07-12): the pre-reorg `workspace_success_dir()` published the
// "lock" copy into `spec/operator-success-reports/reports/` — a path the
// 2026-06-28 v1 reorg retired. Post-reorg, `verify_anchors.sh` hashes the
// COMMITTED `spec/v1/operator-success-reports/reports/` bodies (byte-stable),
// and the locked in-test SHAs below independently prove the fresh render
// matches. Publishing into `spec/` is therefore vestigial — it only littered
// an untracked root dir on every full test run (caught by spec-lint
// orphan-feature after the first CI shakeout). The publish now targets the
// test's own TempDir purely for local inspection.

/// Publish the canonical `success-*-<scenario>.md` copy of a freshly
/// rendered scenario report into `spec/v1/operator-success-reports/reports/`.
/// This is the file `verify_anchors.sh` (post glob-extension) hashes —
/// the test itself runs against `tempfile::TempDir` paths to keep the
/// fixture surface ephemeral, but the locked SHA only matters when the
/// gate can find a real report on disk to hash against.
fn publish_success_copy(dest_root: &Path, src_full_md: &Path, scenario: &str) -> PathBuf {
    let dest_dir = dest_root.join("published");
    std::fs::create_dir_all(&dest_dir).expect("create TempDir publish dir");
    // Single canonical filename per scenario (no timestamp suffix) — the
    // verify_anchors.sh `ls -1 | sort | tail -1` step picks any matching
    // file regardless of how many we publish, but a stable filename
    // avoids accumulating stale copies on every test re-run.
    let dest = dest_dir.join(format!("success-fixed-{scenario}.md"));
    std::fs::copy(src_full_md, &dest).expect("publish success copy");
    dest
}

// ── 7-day scenario ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn t816_report_sample_7d_determinism_and_anchor_lock() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit-7d.db");
    let _ = build_ledger_7d(&db_path).await.expect("build 7d fixture");
    assert_eq!(
        fixture_7d_period_start().inner(),
        time::OffsetDateTime::parse(
            build_ledger_7d::PERIOD_START_RFC3339,
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap(),
        "fixture period_start parses round-trip clean",
    );
    let _ = fixture_7d_period_end();

    let out_a = dir.path().join("a.md");
    let out_b = dir.path().join("b.md");

    // Two sequential renders — same seed, same fixture.
    let body_a = render_scenario(&db_path, &out_a, fixture_7d_period_start()).await;
    let body_b = render_scenario(&db_path, &out_b, fixture_7d_period_start()).await;
    // Publish the `out_a` rendering to `spec/v1/operator-success-reports/reports/`
    // so the verify-anchors gate can pick it up via the success-* glob (the
    // body bytes match `out_b` byte-for-byte by V4 — either copy works).
    let _published = publish_success_copy(dir.path(), &out_a, "report-sample-7d");

    let sha_a = body_sha256_hex(&body_a);
    let sha_b = body_sha256_hex(&body_b);

    eprintln!("T816 report-sample-7d body SHA-256: {sha_a}");
    assert_eq!(
        sha_a, sha_b,
        "V4 determinism violation: same fixture + same seed produced \
         two distinct bodies.\n\nbody A:\n{body_a}\n\nbody B:\n{body_b}",
    );
    // Sanity — body should contain the fixture's hand-curated lifecycle event.
    assert!(
        body_a.contains("[Load] strategy_id=strat_alpha"),
        "body should surface the strat_alpha Load event in R8"
    );
    assert!(
        body_a.contains("strategy_id=strat_beta"),
        "body should surface strat_beta in either R5 or R8"
    );
    // Anchor gate — captured SHA must match the locked constant.  When
    // first running this test, the assertion will fail and the actual
    // SHA will be printed above.  Update both [`EXPECTED_SHA_7D`] AND
    // `spec/anchors.toml` in the same commit.
    assert_eq!(
        sha_a, EXPECTED_SHA_7D,
        "report-sample-7d body-SHA256 drifted.  \
         Update both EXPECTED_SHA_7D and spec/anchors.toml if the rotation is intentional.",
    );
}

// ── 90-day scenario ────────────────────────────────────────────────────────────

#[tokio::test]
async fn t816_report_sample_90d_determinism_and_anchor_lock() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit-90d.db");
    let _ = build_ledger_90d(&db_path).await.expect("build 90d fixture");
    let _ = fixture_90d_period_end();

    let out_a = dir.path().join("a.md");
    let out_b = dir.path().join("b.md");

    let body_a = render_scenario(&db_path, &out_a, fixture_90d_period_start()).await;
    let body_b = render_scenario(&db_path, &out_b, fixture_90d_period_start()).await;
    let _published = publish_success_copy(dir.path(), &out_a, "report-sample-90d");

    let sha_a = body_sha256_hex(&body_a);
    let sha_b = body_sha256_hex(&body_b);

    eprintln!("T816 report-sample-90d body SHA-256: {sha_a}");
    assert_eq!(
        sha_a, sha_b,
        "V4 determinism violation: same fixture + same seed produced \
         two distinct bodies.\n\nbody A:\n{body_a}\n\nbody B:\n{body_b}",
    );
    assert!(
        body_a.contains("[Swap] strategy_id=strat_alpha"),
        "body should surface the strat_alpha Swap event in R8"
    );
    assert!(
        body_a.contains("pairs_zeta"),
        "body should surface pairs_zeta in R5 or R8"
    );
    assert_eq!(
        sha_a, EXPECTED_SHA_90D,
        "report-sample-90d body-SHA256 drifted.  \
         Update both EXPECTED_SHA_90D and spec/anchors.toml if the rotation is intentional.",
    );
}

// ── V10 — Cron-friendliness smoke (3× parallel renders) ───────────────────────

/// V10 — three concurrent renders against the same fixture must:
///
/// 1. All exit `Ok(_)` (no race-induced rename failure / EBUSY / etc.).
/// 2. Produce byte-identical bodies (the seed pins the body shape).
/// 3. Never expose a half-written file at the canonical path — the
///    `atomic_write` helper writes to `<path>.tmp.<pid>.<n>` then
///    `std::fs::rename`s, which is atomic on macOS / Linux for paths
///    on the same filesystem.  The test polls the canonical path
///    while the renders execute and verifies any file it observes
///    parses as a complete report (front-matter + closing fence).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t816_v10_cron_friendly_3x_parallel_renders_atomic() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit-v10.db");
    let _ = build_ledger_7d(&db_path).await.expect("build 7d fixture");

    let canonical = dir.path().join("v10-out").join("report.md");
    std::fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    let period_start = fixture_7d_period_start();

    // Spawn a background poller that watches the canonical path while
    // the three renders race.  Any time the file exists, it must parse
    // as a complete report (front-matter fence + closing fence + body).
    let stop = Arc::new(AtomicBool::new(false));
    let canonical_for_poll = canonical.clone();
    let stop_for_poll = stop.clone();
    let poller = tokio::spawn(async move {
        let mut bad_observation: Option<String> = None;
        while !stop_for_poll.load(Ordering::Relaxed) {
            if let Ok(buf) = std::fs::read_to_string(&canonical_for_poll) {
                if !buf.starts_with("---\n") {
                    bad_observation = Some("missing opening fence".into());
                    break;
                }
                // Closing fence is `\n---\n` (no mandatory blank line —
                // hash_report.py only requires the latter `---\n`).
                let Some(after_open) = buf.strip_prefix("---\n") else {
                    bad_observation = Some("front-matter prefix vanished".into());
                    break;
                };
                let Some(close_pos) = after_open.find("\n---\n") else {
                    bad_observation = Some("missing closing fence".into());
                    break;
                };
                let body_start = close_pos + "\n---\n".len();
                if body_start >= after_open.len() {
                    bad_observation = Some("body slice empty".into());
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        bad_observation
    });

    // Three concurrent renders — all targeting the same canonical path.
    let mut tasks = Vec::with_capacity(3);
    for _ in 0..3 {
        let db = db_path.clone();
        let out = canonical.clone();
        tasks.push(tokio::spawn(async move {
            let frozen = FrozenMarkSource::from_csv_str("symbol,close_time,close\n").unwrap();
            reports::generate(
                ReportWindow::Since(period_start),
                &db,
                &frozen,
                &out,
                Some(FIXTURE_SEED),
            )
            .await
        }));
    }

    for (i, t) in tasks.into_iter().enumerate() {
        let result = t.await.expect("task join");
        result.unwrap_or_else(|e| panic!("V10 render #{i} returned Err: {e}"));
    }

    // Stop the poller; any partial-file observation it caught
    // would have been surfaced by now.
    stop.store(true, Ordering::Relaxed);
    let observation = poller.await.expect("poller join");
    if let Some(reason) = observation {
        panic!("V10 violation: canonical path observed in partial state — {reason}");
    }

    // Final canonical body must parse cleanly + match the locked SHA.
    let full = std::fs::read_to_string(&canonical).unwrap();
    let body = body_after_fence(&full);
    assert!(!body.is_empty(), "canonical body must be non-empty");
    let sha = body_sha256_hex(body);
    assert_eq!(
        sha, EXPECTED_SHA_7D,
        "V10 final-write body SHA must match the locked 7d anchor"
    );

    // Sanity: the parent directory must contain only the canonical file
    // (no orphaned `.tmp.<pid>.<n>` left behind after a successful run).
    let entries = std::fs::read_dir(canonical.parent().unwrap()).unwrap();
    let names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let leftover_tmp: Vec<&String> = names.iter().filter(|n| n.contains(".tmp.")).collect();
    assert!(
        leftover_tmp.is_empty(),
        "V10 hygiene: unexpected tempfile remnants in canonical dir: {leftover_tmp:?}"
    );
}

// ── Cron-friendliness smoke via the `report` BIN (3× parallel processes) ───────

/// Same V10 idea as the lib-level test above, but spawns the `report`
/// binary 3× in parallel from the same CWD against the same fixture.
/// Asserts: all three exit 0; all three canonical outputs (each
/// process gets its own `--output` since they share a CWD) carry a
/// matching body-SHA256; no partial files appear at any output path.
///
/// The brief calls this the "test script runs the binary 3× in parallel
/// from the same CWD" sub-test (T816 V10).  We invoke via
/// `cargo run --quiet --bin report` to skip a precompile of the bin —
/// `cargo` short-circuits when the build artifact is current.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t816_v10_cron_friendly_3x_parallel_bin_processes() {
    // Skip when CARGO is not available (the test is `tokio::test` so
    // the integration harness always has it; this guard is a belt-and-
    // braces against unusual CI shells).
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit-bin.db");
    let _ = build_ledger_7d(&db_path).await.expect("build 7d fixture");

    let workspace_root = workspace_root();
    let period_arg = format!("since:{}", build_ledger_7d::PERIOD_START_RFC3339);
    let seed_arg = format!("0x{FIXTURE_SEED:X}");

    let mut handles: Vec<std::thread::JoinHandle<(i32, PathBuf)>> = Vec::with_capacity(3);
    for n in 0..3 {
        let out = dir.path().join(format!("bin-out-{n}.md"));
        let cargo_cmd = cargo.clone();
        let db = db_path.clone();
        let workspace_root = workspace_root.clone();
        let period_arg = period_arg.clone();
        let seed_arg = seed_arg.clone();
        handles.push(std::thread::spawn(move || {
            let status = std::process::Command::new(&cargo_cmd)
                .args([
                    "run",
                    "--quiet",
                    "-p",
                    "reports",
                    "--bin",
                    "report",
                    "--",
                    "--period",
                    &period_arg,
                    "--ledger",
                    db.to_str().unwrap(),
                    "--output",
                    out.to_str().unwrap(),
                    "--seed",
                    &seed_arg,
                ])
                .current_dir(&workspace_root)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("spawn `cargo run -p reports --bin report`");
            (status.code().unwrap_or(-1), out)
        }));
    }

    let mut shas: Vec<String> = Vec::with_capacity(3);
    for (i, h) in handles.into_iter().enumerate() {
        let (code, out) = h.join().expect("thread join");
        assert_eq!(
            code, 0,
            "V10 bin proc #{i} exited with non-zero code {code}"
        );
        let full = std::fs::read_to_string(&out)
            .unwrap_or_else(|e| panic!("read output {out:?} for proc #{i}: {e}"));
        let body = body_after_fence(&full).to_string();
        shas.push(body_sha256_hex(&body));
    }
    assert!(
        shas.windows(2).all(|w| w[0] == w[1]),
        "V10 bin SHAs differ across 3 parallel processes: {shas:?}"
    );
    assert_eq!(
        shas[0], EXPECTED_SHA_7D,
        "V10 bin SHAs must match the locked 7d anchor"
    );
}

/// Walk up from `CARGO_MANIFEST_DIR` until we find the workspace root
/// (the directory containing the top-level `Cargo.toml` with `[workspace]`).
/// Used so the spawned `cargo run` command resolves the workspace
/// regardless of the test's effective CWD.
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/reports during `cargo test`.
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Walk up: crates/reports → crates → workspace root.
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map_or(crate_dir.clone(), Path::to_path_buf)
}
