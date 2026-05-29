---
adr: 0050
title: iced-tokio runtime-context contract and cooperative cancellation primitives
status: accepted
date: 2026-05-29
supersedes: none
superseded-by: none
---

# ADR-0050: iced ↔ tokio runtime-context contract and cooperative cancellation primitives

## Context

This ADR is codified on the **third recurrence** of the iced-tokio
reactor-context-absence bug ("twice-bitten" threshold per architect.md):

1. **2026-05-23 P1 fix** — `ServerTimeRecipe::stream()` lacked
   `rt_handle.enter()` before `tokio::time::interval(1s)`. Fixed by
   explicitly entering the handle in `server_time_stream_impl` + adding a
   doc comment to `cockpit_live.rs:104-126`.
2. **Bug #64 D.1.1 attempt-2 (2026-05-29)** — `spawn_lab_run`'s
   `iced::Task::perform` closure constructed `tokio::time::interval(250ms)`
   at `runner.rs:744` WITHOUT `rt_handle.enter()`. Symptom: progress label
   dormant ("endless spinning 0/1 bars · 0.0s") during 30-60 s cold-cache
   Yahoo fetch. Root cause confirmed by architect validation
   (`bug-64-arch-validation-2026-05-29.md § 2 Q1`).
3. **Bug #64 D.1.1 attempt-2 (2026-05-29)** — `RunCancelReceiver` exposed
   only `is_cancelled()` (synchronous `std::sync::mpsc::try_recv`). No
   `Future` was available for `tokio::select!`. Zero cancel checks existed
   during the cold-cache preload window (`runner.rs:705-828`). Symptom:
   Stop button non-functional during preload.

The inline doc comment at `cockpit_live.rs:104-126` was the ONLY
codification of the iced ↔ tokio reactor-context rule. It was not
sufficient to prevent recurrence. An ADR is the durable home.

## Decision

### D1 — rt_handle.enter() invariant

Any code path that runs inside `iced::Task::perform` async closures OR
inside `Recipe::stream()` bodies (Subscriptions) and calls
`tokio::time::*`, `tokio::net::*`, or any other tokio reactor-dependent
API MUST enter the agent runtime via `let _guard = rt_handle.enter()` BEFORE
the call.

```rust
// ✓ CORRECT (mirrors ServerTimeRecipe and runner.rs post-fix)
let mut ticker = {
    let _guard = rt.enter();            // enter tokio reactor context
    tokio::time::interval(Duration::from_millis(250))
    // _guard dropped here; the constructed Sleep futures carry their
    // reactor binding and continue to fire on the agent runtime.
};

// ✗ WRONG — timer silently never fires (no panic; just Poll::Pending forever)
let mut ticker = tokio::time::interval(Duration::from_millis(250));
```

**Why this matters**: iced 0.14 uses `futures::executor::ThreadPool` (when
the `thread-pool` feature is active). That executor has NO tokio reactor
context thread-local. `tokio::time::Sleep` registers its wakeup with the
time driver at construction time; if no time driver is reachable via the
thread-local, the future is permanently `Poll::Pending` — no panic, just
silent hang. This is invisible to the operator until they see "endless
spinning".

The guard may be dropped before `Box::pin` (K8 pattern): the constructed
`Interval`/`Sleep` captures a reference to the time driver at construction
and does NOT require the thread-local on subsequent `tick()` calls.

### D2 — CancellationToken as canonical cancellation primitive

Cooperative cancellation between the iced main thread and tokio-runtime
tasks SHALL use `tokio_util::sync::CancellationToken`. The previous
`std::sync::mpsc::sync_channel(0)` disconnect-idiom is deprecated.

```rust
// ✓ CORRECT (post-ADR-0050)
pub struct RunCancelHandle { token: CancellationToken }
pub struct RunCancelReceiver { token: CancellationToken }

impl Drop for RunCancelHandle {
    fn drop(&mut self) { self.token.cancel(); }
}

impl RunCancelReceiver {
    pub fn is_cancelled(&self) -> bool { self.token.is_cancelled() }
    pub async fn cancelled(&self) { self.token.cancelled().await }
}
```

The `cancelled() -> impl Future` method is directly usable in
`tokio::select!` as a third arm (D-R2.2):

```rust
tokio::select! {
    biased;
    result = &mut preload_future => { break result; }
    _ = cancel.cancelled() => { return Err(SmolStr::new("cancelled")); }
    _ = ticker.tick() => { /* emit elapsed */ }
}
```

`CancellationToken` is the production-proven primitive used by axum, hyper,
and sqlx for graceful shutdown. It provides ONE source of truth (no parallel
std-channel + tokio-Notify signals), atomic cancel semantics, and direct
`Future` interop with `tokio::select!`.

### D3 — Timer-fired-in-bounded-window test contract

Every `iced::Task::perform` closure that constructs a `tokio::time::Interval`
(or any tokio timer) MUST have an e2e test asserting the timer fires ≥ N times
within a bounded wall-clock window. The test MUST replicate the production
`rt.enter()` guard to validate that the guard is present and functional.

**Minimum contract**: ≥ 3 timer events within a 4× the interval window (e.g.
`tokio::time::interval(250ms)` → ≥ 3 events in 1 s).

Reference test: `crates/ui/tests/lab_runner_ticker_e2e.rs::ticker_fires_at_least_3_times_in_1s_window`.

This test acts as a mechanical regression gate for D1: if `rt.enter()` is
accidentally removed, the timer is permanently pending and the assertion
`ticker_events.len() >= 3` fails immediately.

## Alternatives considered

- **Inline doc comment only** (status quo before ADR-0050) — rejected because
  two separate recurrences prove the doc comment is insufficient. ADR is the
  durable home per architect.md atomic-register contract.
- **Compile-time lint (proc macro) to detect `tokio::time::*` without enter guard** —
  rejected at v0.1.0 as disproportionate complexity (bespoke proc macro).
  D3 test-based gate is sufficient and lower maintenance cost.
- **Migrate all timers to `iced::time::every()`** — rejected per Q5 analyst
  ratification: the Recipe pattern is preferred; `iced::time::every()` does not
  compose with the `select!`-based preload pattern.
- **Keep `std::sync::mpsc::sync_channel(0)` with a `notified()` bridge** —
  rejected (Option A in architect validation Q4) because it introduces two
  parallel signals (std channel + tokio Notify) with subtle Drop ordering
  risks. `CancellationToken` is one source of truth.

## Consequences

### What breaks if D1 is violated

`tokio::time::interval` / `tokio::time::sleep` / `tokio::time::timeout`
constructed inside `iced::Task::perform` or `Recipe::stream()` without
`rt_handle.enter()` will be permanently `Poll::Pending`. No panic.
Operator sees: label stuck at "0/1 bars · 0.0s" forever.

Caught by: `crates/ui/tests/lab_runner_ticker_e2e.rs::ticker_fires_at_least_3_times_in_1s_window`.

### What breaks if D2 is violated

Stop button dispatched during `tokio::select!` preload loop will not be
observed until the preload future resolves (potentially 30-60 s on cold cache).
Operator sees: Stop button non-functional during cold-cache window.

Caught by: `crates/ui/tests/lab_runner_cancel_e2e.rs::stop_during_preload_exits_within_500ms`.

### What breaks if D3 is violated

Regression to D1 goes undetected until operator reports "endless spinning".
The D3 gate is the only automated regression guard for the timer-context
invariant.

### Reference implementations

| Role | File | Key lines |
|------|------|-----------|
| Reference recipe (D1) | `crates/ui/src/live.rs` | `server_time_stream_impl` — `let _guard = rt_handle.enter()` |
| Authoritative doc comment (D1) | `crates/ui/src/bin/cockpit_live.rs` | lines 104-126 |
| Cancel primitive (D2) | `crates/backtest/src/cancel.rs` | `RunCancelHandle` + `RunCancelReceiver` |
| Preload loop (D1+D2) | `crates/ui/src/lab/runner.rs` | `spawn_lab_run` — lines 741-850 |
| D3 timer test | `crates/ui/tests/lab_runner_ticker_e2e.rs` | `ticker_fires_at_least_3_times_in_1s_window` |
| D3 cancel test | `crates/ui/tests/lab_runner_cancel_e2e.rs` | `stop_during_preload_exits_within_500ms` |

## Changelog

- 2026-05-29 (architect+developer): codified on 3rd recurrence per
  twice-bitten threshold; see `spec/dev-notes/bug-64-arch-validation-2026-05-29.md § 5`.
  D1: rt_handle.enter() invariant before any tokio reactor API in
  iced::Task::perform closures. D2: tokio_util::sync::CancellationToken as
  canonical primitive. D3: timer-fired-in-bounded-window test contract.
  Fix lands in same commit: `crates/backtest/src/cancel.rs` (CancellationToken
  primitive swap) + `crates/ui/src/lab/runner.rs` (D-R1.1 rt.enter() guard +
  D-R2.2 cancel select! arm + D-R1.4 yield_now defense).
