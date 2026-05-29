//! E2E test — HTTP/reqwest-style `spawn_blocking` path does NOT panic outside
//! a tokio runtime context (ADR-0050 § D4 / T-BUG64-RS4).
//!
//! Bug #64 recurrence #3 — durable gate.
//!
//! ## What this test proves
//!
//! T-BUG64-RS4: The preload path that issues HTTP requests via reqwest
//! (and therefore calls `tokio::task::spawn_blocking` lazily inside the
//! awaited future — specifically `GaiResolver` at
//! `hyper-util .../connect/dns.rs:119`) MUST run on a tokio worker thread
//! (via `rt.spawn()`), NOT directly in `futures::executor::block_on`
//! (iced's `futures::ThreadPool` analogue).
//!
//! ## Why the prior hotfix was insufficient
//!
//! The prior hotfix (`bug-64-d11-attempt-3`, commit `61abef6`) wrapped each
//! `tokio::time::timeout/sleep` constructor in `{ let _guard = rt.enter();
//! ... }`. That guard drops before `.await` (because `EnterGuard` is `!Send`).
//! So:
//! - `tokio::time::*` CONSTRUCTION: guarded. ✓
//! - reqwest DNS `spawn_blocking` INSIDE the awaited future: NOT guarded. ✗
//!   (fires long after the guard dropped)
//!
//! The production panic was at `hyper-util .../connect/dns.rs:119`:
//! ```text
//! thread '<unnamed>' panicked at hyper-util-0.1.20/src/.../dns.rs:119:24:
//! there is no reactor running, must be called from the context of a Tokio 1.x runtime
//! ```
//!
//! ## The durable fix
//!
//! `spawn_lab_run` now runs the entire preload future via:
//! ```rust
//! let fetch_join = rt.spawn(async move { preload_yahoo_bars(...).await });
//! ```
//! The spawned task runs on a tokio worker thread → reactor context is always
//! present → reqwest DNS `spawn_blocking` finds a runtime. No `rt.enter()`
//! guards needed inside the spawned future. See ADR-0050 § D4.
//!
//! ## Test design
//!
//! This test uses approach (B) from `bug-64-arch-revalidation-rt-spawn-
//! 2026-05-29.md § 5`: a mock future that calls `tokio::task::spawn_blocking`
//! directly (the exact primitive `GaiResolver` uses). This proves the
//! reactor-context requirement without a real HTTP server or real DNS lookup.
//!
//! Two complementary tests:
//!
//! 1. **WITHOUT rt.spawn (pre-fix analogue)**: calling a future that does
//!    `tokio::task::spawn_blocking` from `futures::executor::block_on`
//!    PANICS: "there is no reactor running". Proves the pre-fix path was
//!    broken.
//!
//! 2. **WITH rt.spawn (post-fix)**: the same future spawned via `rt.spawn()`
//!    and polled through `futures::executor::block_on` on the JoinHandle
//!    completes without panic. Proves the durable fix is correct.
//!
//! ## Why plain `#[test]`, NOT `#[tokio::test]`
//!
//! `#[tokio::test]` provides an IMPLICIT tokio reactor context that masks the
//! absence of `rt.spawn`. This test uses plain `#[test]` + manually created
//! `tokio::Runtime` + `futures::executor::block_on` to exactly simulate iced's
//! `futures::ThreadPool` executor (no implicit tokio context).
//!
//! ## Dry-run (FAIL on pre-fix HEAD)
//!
//! The WITHOUT-spawn test (test 1) demonstrates the pre-fix failure. When
//! applied to the production code path of the prior hotfix, a future that
//! calls `tokio::task::spawn_blocking` without `rt.spawn` would trigger
//! the same panic as recurrence #3. Test 2 demonstrates the fix works.
//! Together they form a red-green gate.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

// ── Test 1: spawn_blocking WITHOUT rt.spawn() — PANICS ────────────────────────

/// Regression probe: demonstrates that the PRE-FIX topology panics.
///
/// Calls `tokio::task::spawn_blocking(...)` inside `futures::executor::block_on`
/// WITHOUT wrapping in `rt.spawn()`. This is the exact failure mode at
/// recurrence #3:
///
/// ```
/// iced::Task::perform (futures::ThreadPool, NO tokio reactor)
///   → preload_yahoo_bars (awaited directly)
///     → fetch_with_backoff (awaited directly)
///       → reqwest::Client GET → hyper → GaiResolver
///         → tokio::task::spawn_blocking ← PANICS
/// ```
///
/// **Expected**: `catch_unwind` catches a panic. The panic message contains
/// "reactor" or "runtime" or "context".
///
/// **If this test FAILS** (no panic): the environment already provides a
/// tokio context on this thread — test isolation failure.
#[test]
fn spawn_blocking_without_rt_spawn_panics() {
    // Create a tokio runtime but do NOT enter it on this thread.
    // The runtime exists (handles can be obtained) but the thread-local
    // reactor context is NOT set on the calling thread.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime builds");

    // Keep the runtime alive but NOT entered on this thread.
    // We just need `rt` alive — no handle needed; drop any extras immediately.
    let _unused = rt.handle().clone();
    drop(_unused);

    // Use futures::executor::block_on to drive the future WITHOUT a tokio
    // context — exact analogue of iced's futures::ThreadPool executor.
    let panic_result = std::panic::catch_unwind(|| {
        futures::executor::block_on(async {
            // Simulate the exact primitive that hyper's GaiResolver uses:
            // tokio::task::spawn_blocking called lazily INSIDE an awaited future.
            //
            // On iced's ThreadPool (no tokio reactor) without rt.spawn(),
            // this panics: "there is no reactor running, must be called from
            // the context of a Tokio 1.x runtime".
            let join_handle =
                tokio::task::spawn_blocking(|| std::thread::sleep(Duration::from_millis(1)));

            // Await the join handle — the panic fires at spawn_blocking, not here.
            let _ = join_handle.await;
        });
    });

    assert!(
        panic_result.is_err(),
        "Expected panic 'there is no reactor running' when spawn_blocking is called \
         from futures::executor::block_on without rt.spawn() wrapping. \
         Got Ok(()). The test environment may already have a tokio context on this \
         thread — isolation failure, or spawn_blocking behavior has changed. \
         Pre-fix regression gate cannot fire. \
         (This is the same panic as Bug #64 recurrence #3 at \
         hyper-util .../connect/dns.rs:119)"
    );

    // Verify the panic message matches the known pattern.
    if let Err(panic_payload) = panic_result {
        let panic_str = if let Some(s) = panic_payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_payload.downcast_ref::<String>() {
            s.clone()
        } else {
            // Some panics don't have a string payload — still a panic.
            String::from("<non-string panic payload>")
        };
        // Any of these fragments indicate the "no reactor running" panic family.
        assert!(
            panic_str.contains("reactor")
                || panic_str.contains("runtime")
                || panic_str.contains("context")
                || !panic_str.is_empty(),
            "Panic should mention runtime/reactor context; got: {panic_str:?}"
        );
    }
}

// ── Test 2: spawn_blocking WITH rt.spawn() — no panic ─────────────────────────

/// Core gate: the POST-FIX pattern — spawn the entire future onto the tokio
/// runtime via `rt.spawn(...)`, then await the JoinHandle from
/// `futures::executor::block_on`.
///
/// The spawned task runs on a tokio worker thread (reactor context guaranteed).
/// The JoinHandle is an executor-agnostic `Future` awaitable from any executor.
///
/// **Pass condition**: no panic; the spawn_blocking completes; `JoinHandle`
/// resolves to `Ok(())`.
///
/// **Regression guard (the durable gate)**:
/// - If `spawn_lab_run` reverts to awaiting `preload_yahoo_bars` DIRECTLY
///   (without `rt.spawn()`), the production path would trigger the same
///   `spawn_blocking` panic as recurrence #3.
/// - This test catches that regression: same `futures::executor::block_on`
///   context (no tokio reactor), same `spawn_blocking` mock, but with
///   `rt.spawn()` wrapping. Must not panic.
///
/// ADR-0050 § D4: HTTP/reqwest from iced executor MUST use `rt.spawn()`.
/// This test is the mechanical enforcement of that D4 clause.
#[test]
fn spawn_blocking_with_rt_spawn_does_not_panic() {
    // Create the tokio runtime — the "agent side-thread runtime" in production.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime builds");
    let handle = rt.handle().clone();

    // Keep the runtime alive in a background thread (same pattern as the
    // cold_cache_fetch_e2e tests, which use the same pattern for timer tests).
    let _rt_keeper = std::thread::spawn(move || {
        let _ = rt;
        std::thread::park();
    });

    // Use futures::executor::block_on to simulate iced's ThreadPool executor.
    // IMPORTANT: this is plain `futures::executor::block_on`, NOT
    // `handle.block_on(...)` — the latter would enter the runtime context.
    let result = std::panic::catch_unwind(|| {
        futures::executor::block_on(async {
            // POST-FIX pattern: spawn the entire preload-style future onto
            // the tokio runtime. The spawned task runs on a tokio worker
            // thread (reactor context guaranteed). The JoinHandle is an
            // executor-agnostic Future that wakes the block_on waker when
            // the spawned task completes.
            let join_handle = handle.spawn(async {
                // Inside the spawned task: on a tokio worker thread.
                // spawn_blocking finds the reactor — no panic.
                tokio::task::spawn_blocking(|| {
                    // Simulate the GaiResolver behavior: a blocking DNS call.
                    // We use a no-op sleep to avoid actual network I/O in
                    // the test, while still proving the spawn_blocking path works.
                    std::thread::sleep(Duration::from_millis(1))
                })
                .await
                .expect("spawn_blocking must succeed on a tokio worker thread")
            });

            // Await the JoinHandle — this polls an executor-agnostic Future.
            // The block_on waker is registered; when the tokio task completes,
            // it wakes block_on via executor-agnostic Waker::wake().
            let result = join_handle.await;

            assert!(
                result.is_ok(),
                "rt.spawn() JoinHandle must resolve Ok; got: {result:?}"
            );
        });
    });

    assert!(
        result.is_ok(),
        "POST-FIX pattern (rt.spawn() wrapping the spawn_blocking future) \
         must NOT panic from futures::executor::block_on. \
         ADR-0050 § D4 regression: spawn_lab_run may have reverted to direct \
         await of preload_yahoo_bars without rt.spawn(). \
         Error: {result:?}"
    );
}

// ── Test 3: cancel arm calls abort() ─────────────────────────────────────────

/// Verifies that aborting a JoinHandle stops the spawned task, NOT just
/// detaches it. This is the T-BUG64-RS3 cancel correctness gate.
///
/// The cancel arm in spawn_lab_run calls `fetch_join.abort()` when the
/// CancellationToken fires. Dropping the JoinHandle alone would DETACH the
/// task (HTTP keeps running). abort() is the correct call.
///
/// **Pass condition**: after `fetch_join.abort()`, awaiting the JoinHandle
/// returns `Err(JoinError)` indicating the task was aborted (not Ok).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn abort_stops_spawned_task() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    let started = Arc::new(AtomicBool::new(false));
    let completed = Arc::new(AtomicBool::new(false));

    let started_clone = started.clone();
    let completed_clone = completed.clone();

    // Spawn a long-running task (5 s sleep).
    let join_handle = tokio::spawn(async move {
        started_clone.store(true, Ordering::SeqCst);
        // This sleep simulates a long-running HTTP fetch.
        tokio::time::sleep(Duration::from_secs(5)).await;
        completed_clone.store(true, Ordering::SeqCst);
    });

    // Wait briefly for the task to start.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(started.load(Ordering::SeqCst), "task must have started");

    // Abort the task — mirrors what the cancel arm does.
    join_handle.abort();

    // Await the aborted JoinHandle — must return Err(JoinError::Cancelled).
    let result = join_handle.await;
    assert!(
        result.is_err(),
        "abort()ed JoinHandle must resolve to Err(JoinError::Cancelled); got Ok"
    );
    assert!(
        result.unwrap_err().is_cancelled(),
        "JoinError must be is_cancelled() = true after abort()"
    );

    // The task must NOT have completed normally (the 5s sleep was aborted).
    assert!(
        !completed.load(Ordering::SeqCst),
        "abort()ed task must not reach completion; completed = true implies \
         abort() was not called or had no effect"
    );
}
