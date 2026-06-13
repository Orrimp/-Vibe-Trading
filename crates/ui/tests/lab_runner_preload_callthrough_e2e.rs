//! Production-call-through regression test for Bug #64 rt.spawn() fix.
//!
//! ADR-0050 § D4 / T-BUG64-CT1.
//!
//! ## What this test proves
//!
//! T-BUG64-CT1: The `spawn_preload_on_rt` function (runner.rs) MUST wrap the
//! preload future in `rt.spawn()`. Without that wrapping, any source whose
//! `preload()` method calls `tokio::task::spawn_blocking` (the exact primitive
//! that reqwest/hyper `GaiResolver` uses for DNS at `dns.rs:119`) panics
//! with "there is no reactor running" when called from `futures::executor::
//! block_on` (iced's `futures::ThreadPool` analogue — no tokio reactor on the
//! calling thread).
//!
//! ## Why this test is NOT theater
//!
//! Unlike `lab_runner_http_offexecutor_e2e.rs` (which proves the mechanism by
//! calling `spawn_blocking` directly in a hand-rolled future), THIS test:
//!
//! 1. Calls `ui::lab::runner::spawn_preload_on_rt` — the PRODUCTION function
//!    that enforces ADR-0050 § D4. If this function reverts from `rt.spawn()`
//!    to direct-await, the test panics.
//!
//! 2. Uses `SpawnBlockingFakeSource` — a `LabYahooBarSource` implementation
//!    whose `preload()` method calls `tokio::task::spawn_blocking` exactly as
//!    reqwest/hyper DNS does. No network I/O required.
//!
//! 3. Runs under `futures::executor::block_on` (NO `#[tokio::test]`) to
//!    simulate iced's `futures::ThreadPool` — the reactor-absent context that
//!    caused Bug #64 recurrence #3.
//!
//! ## Regression guard contract
//!
//! If `spawn_preload_on_rt` reverts to:
//! ```rust
//! // BROKEN — do NOT do this:
//! async { source.preload(&cfg, &range).await }
//! ```
//! (i.e., removes the `rt.spawn()` wrapping), the `spawn_blocking` call inside
//! `SpawnBlockingFakeSource::preload()` fires without a reactor context →
//! `tokio::task::spawn_blocking` panics → `catch_unwind` catches it → test FAILS.
//!
//! With the fix (`rt.spawn(async move { source.preload(...).await })`), the
//! future runs on a tokio worker thread where a reactor IS present → no panic →
//! test PASSES.
//!
//! ## Why `#[test]` not `#[tokio::test]`
//!
//! `#[tokio::test]` implicitly provides a tokio reactor on the calling thread,
//! masking the absence of `rt.spawn()`. This test MUST use plain `#[test]` +
//! `futures::executor::block_on` to reproduce the iced executor environment.
//!
//! ## Scope note (updated T-BUG64-UN1)
//!
//! After T-BUG64-UN1, `spawn_preload_on_rt` is the SINGLE `rt.spawn()` enforcement
//! point. Both the mock injection path (`yahoo_source_override = Some(...)`) and the
//! production Yahoo path (`DefaultLabYahooBarSource` via `#[cfg(feature = "yahoo")]`)
//! now route through this function. A type-signature change to `spawn_preload_on_rt`
//! will produce a compile error at BOTH call sites, making this test a compile-error
//! regression gate for the unified enforcement point.
//!
//! ## T-BUG64-UN3 decision (third test)
//!
//! The tester requested an optional third test calling `spawn_preload_on_rt` with
//! `DefaultLabYahooBarSource` (the actual production source). This is NOT hermetically
//! feasible: `DefaultLabYahooBarSource::preload` calls `preload_yahoo_bars` which
//! performs real network and disk I/O (reqwest Yahoo Finance API + parquet cache).
//! No hermetic stub exists for this path. Decision: minimum-bar taken (compile-error
//! catch via unified enforcement point, per tester § 9 Option B).
//!
//! The unification in T-BUG64-UN1 provides the structural guarantee: the production
//! Yahoo call site (inside `#[cfg(feature = "yahoo")]`) now calls
//! `spawn_preload_on_rt(&rt, Box::new(DefaultLabYahooBarSource), ...)`. Any revert
//! that changes `spawn_preload_on_rt`'s return type produces a compile error at both
//! the mock path (which this test exercises) and the production Yahoo path.
//!
//! See tester report `test-20260529-201208-callthrough.md § 9` for the full rationale.
//! See ADR-0050 § D4.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use backtest::engine::DateRange;
use smol_str::SmolStr;
use trading_core::Bar;
use ui::lab::runner::{
    LabBarSource, LabRunConfig, LabYahooBarSource, PreloadFuture, spawn_preload_on_rt,
};
use ui::lab::state::LabDataSource;

// ── SpawnBlockingFakeSource ───────────────────────────────────────────────────

/// A `LabYahooBarSource` that calls `tokio::task::spawn_blocking` inside
/// its `preload()` implementation.
///
/// This is the exact primitive that reqwest/hyper's `GaiResolver` uses for
/// DNS resolution at `hyper-util .../connect/dns.rs:119`. Calling this from
/// a non-tokio executor (no reactor context) panics: "there is no reactor
/// running, must be called from the context of a Tokio 1.x runtime".
///
/// Using this source as the regression probe ensures the test fails if
/// `spawn_preload_on_rt` removes its `rt.spawn()` wrapper.
struct SpawnBlockingFakeSource;

// simple-strategies-realdata T-B3: the `preload` body now lives on the shared
// `LabBarSource` super-trait; `LabYahooBarSource` is a pure marker. The
// regression guard is unchanged — `spawn_preload_on_rt` still receives a
// `Box<dyn LabYahooBarSource>` (coerced into `Box<S: LabBarSource>`) and a
// direct-await revert still fires the spawn_blocking-without-reactor panic.
impl LabBarSource for SpawnBlockingFakeSource {
    fn preload<'a>(&'a self, _cfg: &'a LabRunConfig, _range: &'a DateRange) -> PreloadFuture<'a> {
        Box::pin(async {
            // This is the exact primitive that reqwest/hyper uses for DNS.
            // It requires a tokio reactor on the polling thread.
            // Without rt.spawn() wrapping: panics "there is no reactor running".
            // With rt.spawn() wrapping: runs on a tokio worker thread → OK.
            let _result = tokio::task::spawn_blocking(|| {
                // Simulate a blocking DNS/network operation.
                std::thread::sleep(Duration::from_millis(1));
            })
            .await
            .expect("spawn_blocking must succeed on a tokio worker thread");

            // Return dummy bars (no network/disk I/O).
            Ok::<(Vec<Bar>, SmolStr), SmolStr>((
                Vec::new(),
                SmolStr::new("spawn-blocking-fake-sha-0000000000000000"),
            ))
        })
    }
}

impl LabYahooBarSource for SpawnBlockingFakeSource {}

// ── Helper ────────────────────────────────────────────────────────────────────

fn yahoo_cache_cfg() -> LabRunConfig {
    LabRunConfig {
        strategy_id: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        venue: SmolStr::new("Binance"),
        range_label: SmolStr::new("Last30d"),
        seed: ui::lab::defaults::LAB_DEFAULT_SEED,
        write_report: false,
        data_source: LabDataSource::YahooCache,
        sma_fast_len: None,
        sma_slow_len: None,
    }
}

// ── Test 1: spawn_preload_on_rt — production call-through, no panic ────────────

/// Production call-through gate: `spawn_preload_on_rt` with a source that
/// calls `tokio::task::spawn_blocking` MUST complete without panic when called
/// from `futures::executor::block_on`.
///
/// **Pass condition (current HEAD)**: `spawn_preload_on_rt` wraps the future
/// in `rt.spawn()` → future runs on a tokio worker thread → reactor present
/// → `spawn_blocking` succeeds → `JoinHandle` resolves `Ok(Ok(...))`.
///
/// **Fail condition (pre-fix / regression)**:
/// If `spawn_preload_on_rt` is changed to a direct await:
/// ```rust
/// // BROKEN — removes rt.spawn() invariant:
/// async { source.preload(&cfg, &range).await }
/// ```
/// Then `source.preload()` runs on the `block_on` thread which has NO tokio
/// reactor. `tokio::task::spawn_blocking` panics: "there is no reactor running".
/// `catch_unwind` catches it → test FAILS.
///
/// **ADR-0050 § D4 enforcement**: this test IS the regression gate for
/// `spawn_preload_on_rt`. The tester re-runs this test as part of Gate 2
/// (FAIL-on-pre-fix dry-run) for the Bug #64 INCONCLUSIVE closure.
#[test]
fn preload_callthrough_with_spawn_blocking_does_not_panic() {
    // Build the tokio runtime — the "agent side-thread runtime" that exists
    // in production (created in cockpit_live.rs).
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime builds");
    let handle = rt.handle().clone();

    // Keep the runtime alive in a background thread.
    // We need the runtime to remain alive across the block_on call below.
    let _rt_keeper = std::thread::spawn(move || {
        let _ = rt; // holds the runtime alive
        std::thread::park(); // park until dropped
    });

    let cfg = yahoo_cache_cfg();
    let range = DateRange::Last30d;

    // Use futures::executor::block_on to simulate iced's futures::ThreadPool.
    // IMPORTANT: this is plain `futures::executor::block_on`, NOT
    // `handle.block_on(...)` — the latter would enter the runtime context.
    // We want to prove the future works WITHOUT automatic context injection.
    let result = std::panic::catch_unwind(|| {
        futures::executor::block_on(async {
            // PRODUCTION CALL: spawn_preload_on_rt is the function that
            // enforces ADR-0050 § D4. It calls rt.spawn() internally.
            //
            // If it reverts to direct-await, SpawnBlockingFakeSource::preload()
            // calls spawn_blocking without a reactor → panic.
            let join_handle =
                spawn_preload_on_rt(&handle, Box::new(SpawnBlockingFakeSource), cfg, range);

            // Await the JoinHandle — executor-agnostic Future.
            // block_on waker is registered; tokio task wakes it on completion.
            match join_handle.await {
                Ok(Ok(_bars_and_sha)) => {
                    // Pass: preload completed successfully.
                }
                Ok(Err(e)) => {
                    panic!("SpawnBlockingFakeSource::preload() returned Err: {e}");
                }
                Err(join_err) => {
                    panic!("spawn_preload_on_rt JoinHandle panicked or was aborted: {join_err}");
                }
            }
        });
    });

    assert!(
        result.is_ok(),
        "spawn_preload_on_rt (production call) with SpawnBlockingFakeSource \
         MUST NOT panic from futures::executor::block_on. \
         ADR-0050 § D4 regression: spawn_preload_on_rt may have reverted \
         from rt.spawn() to direct-await, removing the reactor-context guarantee. \
         This is Bug #64 recurrence #3 (hyper DNS spawn_blocking panic). \
         Panic: {result:?}"
    );
}

// ── Test 2: direct-await proof (shows WHAT would break) ──────────────────────

/// Mechanism proof: calling `SpawnBlockingFakeSource::preload()` directly from
/// `futures::executor::block_on` (WITHOUT `rt.spawn()`) PANICS.
///
/// This is the pre-fix / regression analogue. It proves that the
/// `spawn_blocking` in `SpawnBlockingFakeSource` genuinely requires a reactor
/// — making Test 1 a meaningful gate, not a vacuous pass.
///
/// **Expected**: `catch_unwind` catches the panic. Message contains
/// "reactor" or "runtime" or "context" (tokio's standard message family).
///
/// **If this test FAILS** (no panic caught): the test environment already
/// provides a tokio context on this thread — test isolation may be broken.
/// The CT1 test above would also be unreliable.
#[test]
fn direct_await_without_rt_spawn_panics() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime builds");

    // Keep the runtime alive (NOT entered on this thread).
    let _rt_keeper = std::thread::spawn(move || {
        let _ = rt;
        std::thread::park();
    });

    let cfg = yahoo_cache_cfg();
    let range = DateRange::Last30d;
    let source = SpawnBlockingFakeSource;

    // Drive the preload future DIRECTLY from block_on — no rt.spawn() wrapping.
    // This reproduces the pre-fix topology:
    //
    //   futures::executor::block_on  (NO tokio reactor)
    //     → source.preload().await
    //       → tokio::task::spawn_blocking ← PANICS "no reactor running"
    //
    let panic_result = std::panic::catch_unwind(|| {
        futures::executor::block_on(async {
            // Direct await — the broken pre-fix pattern.
            // spawn_blocking fires without a reactor → panic.
            let _ = source.preload(&cfg, &range).await;
        });
    });

    assert!(
        panic_result.is_err(),
        "Expected panic 'there is no reactor running' when SpawnBlockingFakeSource::preload() \
         is called directly from futures::executor::block_on without rt.spawn() wrapping. \
         Got Ok(()). The test environment may already have a tokio context on this thread \
         — isolation failure. If this assertion fires, Test 1 above is also unreliable \
         as a regression gate. \
         (This is Bug #64 recurrence #3: hyper DNS spawn_blocking without reactor context)"
    );
}
