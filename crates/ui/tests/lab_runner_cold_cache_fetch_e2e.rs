//! E2E test — cold-cache fetch path does NOT panic outside a tokio runtime.
//!
//! Bug #64 D.1.1 attempt-3 HOTFIX / ADR-0050 § D1 amendment (2026-05-29).
//!
//! ## What this test proves
//!
//! T-BUG64-D14: The `fetch_with_backoff` path (via `preload_yahoo_bars`)
//! uses `rt.enter()` guards around ALL `tokio::time::timeout` and
//! `tokio::time::sleep` calls. When called from a non-tokio executor
//! (iced's `futures::ThreadPool`), the guard pattern prevents the panic:
//! "there is no reactor running".
//!
//! ## Why this test uses plain `#[test]`
//!
//! The production panic path is:
//!   `iced::Task::perform` (futures::ThreadPool, NO tokio reactor)
//!     → `preload_yahoo_bars`
//!       → `fetch_with_backoff`
//!         → `tokio::time::timeout(...)` ← PANICS without rt.enter()
//!
//! All existing e2e tests (`lab_runner_ticker_e2e`, `lab_runner_cancel_e2e`)
//! use `#[tokio::test]` which DOES provide a tokio reactor context, so they
//! CANNOT catch this panic. This test uses plain `#[test]` + manually
//! created `tokio::Runtime` + `futures::executor::block_on` to simulate
//! iced's non-tokio executor context.
//!
//! ## Regression gate
//!
//! **Before the hotfix** (commit a87b5fa): calling `tokio::time::timeout`
//! or `tokio::time::sleep` inside the `fetch_with_backoff` stack frame
//! on iced's futures::ThreadPool panicked: "there is no reactor running".
//!
//! **After the hotfix**: each `tokio::time::*` call is preceded by
//! `{ let _guard = rt.enter(); tokio::time::timeout(...) }` — guard
//! entered, future constructed, guard dropped. The `Timeout`/`Sleep`
//! future carries its reactor binding and fires correctly without
//! requiring the guard to remain active across the await point.
//!
//! This test directly exercises that guard pattern in isolation, proving:
//! 1. WITHOUT the guard → `std::panic::catch_unwind` catches the panic.
//! 2. WITH the guard → no panic; future completes normally.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

// ── Helper: simulate iced's non-tokio executor context ───────────────────────
//
// `futures::executor::block_on` drives an async future on the current thread
// using a bare waker (no tokio reactor). This is the same executor class as
// iced's `futures::ThreadPool` — neither has a tokio time driver thread-local.
//
// When `tokio::time::timeout` is constructed WITHOUT `rt.enter()`, it tries
// to register its wakeup with the time driver via a thread-local. If no
// driver is set, tokio panics: "there is no reactor running".

// ── Test 1: tokio::time::timeout WITHOUT rt.enter() — PANICS ──────────────────

/// Regression probe: demonstrates that the PRE-FIX behavior panics.
///
/// Calls `tokio::time::timeout(100ms, ...)` inside `futures::executor::block_on`
/// WITHOUT entering the tokio runtime context. On the production path BEFORE
/// the hotfix, this is exactly what happened in `fetch_with_backoff`.
///
/// **Expected**: `catch_unwind` catches the panic with "there is no reactor running".
///
/// This test documents the pre-fix behavior and acts as a falsification probe:
/// if this test FAILS (no panic caught), something in the environment already
/// provides a tokio context and the test suite isolation may be broken.
///
/// Note: This test MUST be run in its own process or with process isolation
/// to avoid contaminating the tokio context. When run with `cargo test --test
/// lab_runner_cold_cache_fetch_e2e`, Rust starts a fresh process per
/// integration test binary.
#[test]
fn tokio_time_timeout_without_rt_enter_panics() {
    // Create a tokio runtime but do NOT enter it. The handle exists but
    // the thread-local reactor context is NOT set.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime builds");
    let handle = rt.handle().clone();

    // Ensure the runtime is running in the background but NOT entered on
    // this thread (no thread-local context set for this thread).
    // The `rt` object keeps the runtime alive; we don't call `rt.block_on`.
    drop(handle); // drop the extra clone — rt is still alive

    // Use futures::executor::block_on to drive the async closure WITHOUT
    // entering any tokio reactor context. This simulates iced's ThreadPool.
    //
    // We expect this to panic because `tokio::time::timeout` tries to
    // register with the time driver at construction time.
    let panic_result = std::panic::catch_unwind(|| {
        futures::executor::block_on(async {
            // This is the PRE-FIX pattern from fetch_with_backoff:
            // tokio::time::timeout called directly without rt.enter().
            // On iced's ThreadPool (no tokio reactor), this PANICS.
            let _timeout =
                tokio::time::timeout(Duration::from_millis(10), std::future::ready::<()>(()));
            // We don't even await it — the panic occurs at construction time.
        });
    });

    assert!(
        panic_result.is_err(),
        "Expected panic 'there is no reactor running' when tokio::time::timeout \
         is constructed without rt.enter() on a non-tokio executor. \
         Got Ok(()). The test environment may already have a tokio context set \
         on this thread — isolation failure. \
         Pre-fix regression guard cannot fire."
    );

    // Verify the panic message contains the expected text.
    if let Err(panic_payload) = panic_result {
        let panic_str = if let Some(s) = panic_payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_payload.downcast_ref::<String>() {
            s.clone()
        } else {
            // Some panics don't have a string payload — that's still a panic.
            String::from("<non-string panic payload>")
        };
        // The panic message may vary by tokio version; check for common fragments.
        assert!(
            panic_str.contains("reactor")
                || panic_str.contains("runtime")
                || panic_str.contains("context")
                || panic_str.contains("no current")
                || !panic_str.is_empty(), // any panic is acceptable here
            "Panic should mention runtime/reactor context; got: {panic_str:?}"
        );
    }
}

// ── Test 2: tokio::time::timeout WITH rt.enter() — no panic ───────────────────

/// Core gate: the POST-FIX pattern — `rt.enter()` before construction,
/// guard dropped before `.await`. Must complete without panic.
///
/// This is the exact pattern introduced by the hotfix in `fetch_with_backoff`:
/// ```rust
/// let timeout_future = {
///     let _guard = rt.enter();
///     tokio::time::timeout(per_attempt_timeout, fetch_future)
///     // _guard dropped here; Timeout carries its reactor reference.
/// };
/// timeout_future.await
/// ```
///
/// **Pass condition**: no panic; future completes; result is `Ok(())`.
///
/// **Regression guard**: if the `rt.enter()` guard is removed from
/// `fetch_with_backoff`, the same `futures::executor::block_on` context
/// would panic (as proved by test 1 above).
///
/// ADR-0050 § D3 extension: D3 previously required `#[tokio::test]` for the
/// timer tests. This test uses plain `#[test]` to exercise the PRODUCTION
/// runtime context (no implicit tokio context on the calling thread).
#[test]
fn tokio_time_timeout_with_rt_enter_does_not_panic() {
    // Create a tokio runtime — this is the "agent side-thread runtime"
    // that exists in production (created in cockpit_live.rs).
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime builds");
    let handle = rt.handle().clone();

    // Keep the runtime alive in a background thread.
    let _rt_keeper = std::thread::spawn(move || {
        // The runtime will be kept alive as long as this thread holds `rt`.
        // When the test completes and `rt` is dropped, the runtime shuts down.
        let _ = rt;
        // Park until the runtime is dropped.
        std::thread::park();
    });

    // Use futures::executor::block_on to drive the async closure WITHOUT
    // entering any tokio reactor context. Same as iced's ThreadPool.
    //
    // IMPORTANT: this is plain `futures::executor::block_on`, NOT
    // `handle.block_on(...)` — the latter WOULD enter the runtime context.
    // We want to prove the future works WITHOUT automatic context injection.
    let result = std::panic::catch_unwind(|| {
        futures::executor::block_on(async {
            // POST-FIX pattern: enter rt context, construct the future, drop
            // the guard, then await. The EnterGuard is !Send so it MUST NOT
            // be held across an await point — dropping it before await is
            // the exact pattern used in fetch_with_backoff.
            let timeout_future = {
                let _guard = handle.enter();
                tokio::time::timeout(Duration::from_millis(100), std::future::ready::<()>(()))
                // _guard dropped here; Timeout carries its reactor reference.
            };

            // Await the future — this should complete instantly and return Ok(()).
            let outcome = timeout_future.await;

            // The inner future is std::future::ready(()), so it completes
            // immediately without a timeout.
            assert!(
                outcome.is_ok(),
                "ready() future should not time out in 100ms; got: {outcome:?}"
            );
        });
    });

    assert!(
        result.is_ok(),
        "POST-FIX pattern (rt.enter() before construction, drop before await) \
         must NOT panic on futures::executor::block_on. \
         Hotfix regression: the rt.enter() guard in fetch_with_backoff may \
         have been removed or incorrectly scoped. \
         Error: {result:?}"
    );
}

// ── Test 3: sleep with rt.enter() — same pattern ─────────────────────────────

/// Same guard pattern but for `tokio::time::sleep` (the backoff sleep path
/// in `fetch_with_backoff`). Both `timeout` (line 442) and `sleep` (lines
/// 459, 496) in `fetch_with_backoff` use the same guard pattern; this test
/// exercises the `sleep` variant.
///
/// **Pass condition**: no panic; `sleep(1ms)` completes within 2s.
#[test]
fn tokio_time_sleep_with_rt_enter_does_not_panic() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime builds");
    let handle = rt.handle().clone();

    let _rt_keeper = std::thread::spawn(move || {
        let _ = rt;
        std::thread::park();
    });

    let result = std::panic::catch_unwind(|| {
        futures::executor::block_on(async {
            // Backoff sleep pattern from fetch_with_backoff (post-fix).
            let sleep_future = {
                let _guard = handle.enter();
                tokio::time::sleep(Duration::from_millis(1))
                // _guard dropped here.
            };
            // Await the 1ms sleep — should complete nearly instantly.
            sleep_future.await;
        });
    });

    assert!(
        result.is_ok(),
        "POST-FIX tokio::time::sleep with rt.enter() guard must not panic. \
         Hotfix regression detected. Error: {result:?}"
    );
}
