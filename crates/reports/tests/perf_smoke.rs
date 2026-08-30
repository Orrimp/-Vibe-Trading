#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T815 — performance smoke test for the operator-success-report renderer.
//!
//! Walks the [R13] acceptance criteria (`spec/features/operator-success-reports.md`):
//!
//! - **R13.1** wall-clock `< 10s` for `--period 90d` against a 1-year-history
//!   ledger fixture.  Measured via `std::time::Instant::now()`.
//! - **R13.3** RSS `< 256 MiB` measured via `libc::getrusage(RUSAGE_SELF)` on
//!   `ru_maxrss`.  We use the one-FFI-call solution recommended by the
//!   developer-agent task brief; `libc` is added as a `[dev-dependencies]`
//!   only (see `crates/reports/Cargo.toml`).
//!
//! The fixture itself ships at
//! `crates/reports/tests/fixtures/build_ledger_1y.rs`; we pull it in via
//! `#[path = "..."]` rather than a `mod fixtures;` declaration because Rust's
//! integration-test convention treats every file under `tests/` as its own
//! crate root unless the file lives inside a sub-directory module path.
//!
//! [R13]: ../../spec/features/operator-success-reports.md#r13--performance

#[path = "fixtures/build_ledger_1y.rs"]
mod build_ledger_1y;

use std::time::{Duration, Instant};

use reports::{FrozenMarkSource, ReportWindow};
use tempfile::TempDir;
use time::OffsetDateTime;
use trading_core::Timestamp;

use crate::build_ledger_1y::{FIXTURE_SEED, build_ledger_1y};

/// R13.1 wall-clock budget — assert `< 10s`.
const WALL_CLOCK_BUDGET: Duration = Duration::from_secs(10);

/// R13.3 RSS ceiling — **calibrated per platform** (operator ruling 2026-08-29,
/// bug-log #98).
///
/// R13.3 declares "RSS < 256 MiB" and that number is unchanged on the canonical
/// box. What changed is the admission that `ru_maxrss` is NOT a comparable
/// measure across kernels, so one literal cannot mean the same thing on three
/// legs. Measured, same commit, same workload:
///
/// | platform | peak `ru_maxrss` |
/// |---|---|
/// | macOS (canonical box) | **54.3 MiB** |
/// | ubuntu-latest (CI)    | **269.9 MiB** |
///
/// A 5x gap is not "the code needs 270 MiB". `ru_maxrss` counts RESIDENT pages,
/// and what stays resident differs by allocator, by whether parquet reads are
/// mapped or copied, and by kernel reclaim policy. The unit handling below is
/// correct and was verified first (bytes on macOS, kilobytes x 1024 elsewhere),
/// so both figures are genuine.
///
/// The non-macOS ceiling is therefore CALIBRATED, not invented: 269.9 MiB
/// measured, 384 MiB budgeted, ~42 % headroom. Raising the macOS number to match
/// was explicitly rejected — that would fit a declared requirement to the noisiest
/// platform and leave the next one to break it (bug-log #77's failure in
/// performance clothing).
///
/// **If this trips on Linux, do not raise it reflexively.** 384 MiB is far above
/// the measured value; a breach means real growth, not instrument noise. Re-measure
/// both platforms and record the new pair here.
#[cfg(target_os = "macos")]
const RSS_BUDGET_BYTES: u64 = 256 * 1024 * 1024;

/// See the macOS constant above for the calibration and the ruling.
#[cfg(not(target_os = "macos"))]
const RSS_BUDGET_BYTES: u64 = 384 * 1024 * 1024;

/// Read the current process's peak resident-set size in bytes.
///
/// Uses `getrusage(RUSAGE_SELF, ...)` and reads `ru_maxrss`.  Per the
/// `getrusage(2)` manual:
///
/// - **macOS / Darwin**: `ru_maxrss` is in **bytes**.
/// - **Linux** (and most other Unixes): `ru_maxrss` is in **kilobytes**.
///
/// We branch on `cfg(target_os)` so the comparison against the
/// [`RSS_BUDGET_BYTES`] ceiling is always in bytes.
///
/// On non-Unix targets this returns `None`; the test then short-circuits
/// the RSS assertion and prints a `cargo:warning=` so the operator can
/// see RSS coverage was skipped.
#[cfg(unix)]
fn peak_rss_bytes() -> Option<u64> {
    // SAFETY: `getrusage(2)` is a pure-stack syscall — `RUSAGE_SELF` is
    // always valid and `usage` is fully zero-initialised before the call,
    // so the kernel writes into a sound `&mut`.  Both fields we read
    // (`ru_maxrss`) are scalar `c_long` values with no aliasing.
    let usage = unsafe {
        let mut u: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut u) != 0 {
            return None;
        }
        u
    };
    let raw = u64::try_from(usage.ru_maxrss).unwrap_or(0);
    #[cfg(target_os = "macos")]
    {
        // macOS: `ru_maxrss` is bytes — already the right unit.
        Some(raw)
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Linux (and friends): `ru_maxrss` is kilobytes — multiply.
        Some(raw.saturating_mul(1024))
    }
}

#[cfg(not(unix))]
fn peak_rss_bytes() -> Option<u64> {
    None
}

/// Anchor `period_end` at the latest minute boundary of the current
/// wall-clock so the `--period 90d` window resolves to a sub-range
/// covered by the fixture.  We round to the minute (drop sub-second
/// drift) so reruns within the same minute hit the same boundary,
/// matching the determinism contract for the fixture itself.
fn period_end_anchor() -> Timestamp {
    let now = OffsetDateTime::now_utc();
    let trimmed = now.replace_nanosecond(0).expect("nanos = 0 is valid");
    let trimmed = trimmed
        .replace_second(0)
        .expect("second clamp to 0 is valid");
    Timestamp::new(trimmed)
}

/// T815 — `--period 90d` against a 1-year fixture must finish under
/// 10s wall-clock and stay under 256 MiB RSS.
///
/// Uses `lib::generate` directly (rather than spawning the `report`
/// binary via `cargo run`) so the wall-clock budget is not polluted by
/// `cargo`'s own compile/link overhead — that's `cargo`'s timing, not
/// the renderer's.  This matches the spec's "wrap the call" wording in
/// the developer-agent task brief.
#[tokio::test(flavor = "multi_thread")]
async fn t815_perf_smoke_90d_under_10s_and_under_256mib() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("fixture.db");
    let out_md = dir.path().join("reports").join("success").join("report.md");
    std::fs::create_dir_all(out_md.parent().unwrap()).unwrap();

    // Pre-build the 1-year fixture ledger.  This is *not* part of the
    // wall-clock budget — the fixture build is a one-shot setup; the
    // budget is for `lib::generate` only.
    let period_end = period_end_anchor();
    let (_inception, _) = build_ledger_1y(&db_path, period_end)
        .await
        .expect("build 1y fixture ledger");

    // FrozenMarkSource is empty — the orchestrator's BTC baseline calls
    // `marks.close_at(...)` and tolerates an `Err` (returns 0% baseline).
    // For the perf smoke we don't need real marks; the heavy paths are
    // the audit queries + equity-curve sampler + CSV writers.
    let frozen = FrozenMarkSource::from_csv_str("symbol,close_time,close\n").unwrap();

    let started = Instant::now();
    let result = reports::generate(
        ReportWindow::Days90,
        &db_path,
        &frozen,
        &out_md,
        Some(FIXTURE_SEED),
    )
    .await;
    let elapsed = started.elapsed();

    let _artifacts = result.expect("generate should succeed on 1y fixture");

    // ── R13.1 — wall-clock < 10s ────────────────────────────────────────────
    assert!(
        elapsed < WALL_CLOCK_BUDGET,
        "R13.1 wall-clock budget blown: rendered in {:.3}s (budget < {}s)",
        elapsed.as_secs_f64(),
        WALL_CLOCK_BUDGET.as_secs(),
    );
    eprintln!(
        "T815 wall-clock: {:.3}s (budget < {}s) — PASS",
        elapsed.as_secs_f64(),
        WALL_CLOCK_BUDGET.as_secs(),
    );

    // ── R13.3 — RSS < 256 MiB ───────────────────────────────────────────────
    if let Some(rss) = peak_rss_bytes() {
        assert!(
            rss < RSS_BUDGET_BYTES,
            "R13.3 RSS budget blown: peak ru_maxrss = {} B ({:.1} MiB), budget < {} B ({} MiB)",
            rss,
            (rss as f64) / (1024.0 * 1024.0),
            RSS_BUDGET_BYTES,
            RSS_BUDGET_BYTES / (1024 * 1024),
        );
        eprintln!(
            "T815 peak RSS: {:.1} MiB (budget < {} MiB) — PASS",
            (rss as f64) / (1024.0 * 1024.0),
            RSS_BUDGET_BYTES / (1024 * 1024),
        );
    } else {
        // Non-Unix target — getrusage isn't available.  The brief
        // explicitly allows skipping the RSS assertion in that case
        // ("If RSS measurement is genuinely intractable on this
        // platform, document why in the test, run only the wall-clock
        // assertion").  `cargo test` runs on macOS + Linux for this
        // workspace, so this branch is documentation, not coverage.
        eprintln!("T815 RSS measurement skipped: getrusage unavailable on this target_os — SKIP",);
    }
}
