---
adr: 0050
title: iced-tokio runtime-context contract and cooperative cancellation primitives
status: accepted
date: 2026-05-29
supersedes: none
superseded-by: none
updated: 2026-05-29
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

### D1 — rt_handle.enter() invariant (corrected 2026-05-29 — supersedes prior D1 + hotfix entry)

**D1 (corrected):** Off-runtime async work reachable from
`iced::Task::perform` (or a `Recipe::stream()` body) that touches tokio —
DIRECTLY (`tokio::time::*`, `tokio::select!`) OR TRANSITIVELY (reqwest /
hyper DNS via `tokio::task::spawn_blocking`, or any crate that calls into
`tokio::*` lazily across its own await points) — MUST be moved onto the
tokio runtime via `rt.spawn(async { ... }).await`, NOT merely wrapped in
`rt.enter()`.

`rt.enter()` only sets the thread-local runtime context for the current
*synchronous* scope and is dropped at the first `.await` boundary (it must
be — `EnterGuard` is `!Send` and cannot cross an await in a `Send`
future). It is therefore sufficient ONLY for APIs that bind to the runtime
at CONSTRUCTION time and carry that binding thereafter — i.e. bare
`tokio::time::Interval`/`Sleep`/`Timeout` constructors (the K8 pattern).
It is NOT sufficient for multi-await futures that call into tokio LAZILY
(reqwest's DNS `spawn_blocking` fires deep inside the awaited future, long
after the construction-scoped guard is dropped — this is what panicked at
`hyper-util .../connect/dns.rs:119` on Bug #64 recurrence #3).

**Decision rule:** if a future only *constructs* a tokio timer in a guarded
scope and then awaits it → `rt.enter()` guard is correct (keep the ticker
at `runner.rs` as-is). If a future *lazily* calls `tokio::*`
(including transitively via a third-party crate) at any point reachable
from its `.await` → it MUST be spawned. Spawning is correct-by-construction
because the spawned task runs on tokio worker threads which always carry
reactor + time-driver context, and the returned `JoinHandle` is an
executor-agnostic `Future` awaitable from iced's `futures::ThreadPool`
(proven in production at `runner.rs:949-993`, the engine call).

```rust
// ✓ CORRECT for timer-only (K8 pattern — binds at construction):
let mut ticker = {
    let _guard = rt.enter();            // enter tokio reactor context
    tokio::time::interval(Duration::from_millis(250))
    // _guard dropped here; the constructed Sleep futures carry their
    // reactor binding and continue to fire on the agent runtime.
};

// ✓ CORRECT for transitive tokio (reqwest, hyper, etc.) — must spawn:
let result = rt.spawn(async move {
    some_http_call_that_uses_spawn_blocking().await
}).await??;

// ✗ WRONG — timer silently never fires (no panic; just Poll::Pending forever)
let mut ticker = tokio::time::interval(Duration::from_millis(250));

// ✗ WRONG — rt.enter() guard insufficient for transitive spawn_blocking
// (guard drops before .await; DNS spawn_blocking fires without reactor)
let _guard = rt.enter();
some_http_call().await  // GaiResolver panics: "no reactor running"
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

### D3 — Test contract (amended 2026-05-29 — close the HTTP test gap)

**D3 (amended):** The D3 timer-fired-in-bounded-window contract is
RETAINED. ADDITIONALLY: any code path that issues an HTTP request (or calls
a transitive `spawn_blocking`) reachable from `iced::Task::perform` MUST
have an e2e test that exercises that request path under a NON-tokio
executor (`futures::executor::block_on`, the same executor class as iced's
`futures::ThreadPool`), NOT merely a timer. Asserting only on
`tokio::time::*` wrapping `std::future::ready(())` is INSUFFICIENT — it
masks transitive `spawn_blocking` panics (this is the exact gap in
`lab_runner_cold_cache_fetch_e2e.rs` that let recurrence #3 ship: it tested
timers but never exercised a real `spawn_blocking`).

The HTTP path test MUST either (a) drive a real HTTP request against a
localhost fixture server forcing `GaiResolver::spawn_blocking`, (b) inject
a fake source whose preload performs a real `tokio::task::spawn_blocking`
operation (the exact primitive), or (c) be a real-network integration test
gated behind `#[ignore]`. A pure-timer test does not satisfy D3 for HTTP
paths.

**Timer-fired-in-bounded-window (original D3):**
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

**HTTP/spawn_blocking path gate:**
Reference test: `crates/ui/tests/lab_runner_http_offexecutor_e2e.rs`.
Three tests: (1) proves `spawn_blocking` WITHOUT `rt.spawn()` panics from
`futures::executor::block_on` (falsification probe — reproduces recurrence
#3 topology), (2) proves WITH `rt.spawn()` no panic (core gate), (3) proves
`abort()` stops spawned tasks (D-RS3 cancel correctness gate).

### D4 — HTTP/reqwest from iced executor MUST use rt.spawn() (new 2026-05-29)

Any code path that constructs a `reqwest::Client` (or any HTTP client built
on hyper/hyper-util), or issues an HTTP request, from a context reachable
by `iced::Task::perform` or `Recipe::stream()` MUST run that request inside
a future spawned via `rt.spawn(...)`. Wrapping the request in `rt.enter()`
is INSUFFICIENT — the DNS resolver (`hyper_util .../GaiResolver`) calls
`tokio::task::spawn_blocking` lazily during the awaited request, requiring
an active runtime on the polling thread, which iced's executor does not
provide. This applies transitively to wrapper crates (`yahoo_finance_api`,
etc.).

The D4 obligation follows directly from the corrected D1 decision rule:
HTTP futures call `tokio::*` lazily across await points → MUST be spawned,
not guarded. See `bug-64-arch-revalidation-rt-spawn-2026-05-29.md § 2-4`
for the full derivation.

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

| Role | File | Key lines / notes |
|------|------|-----------|
| Reference recipe (D1 K8 pattern) | `crates/ui/src/live.rs` | `server_time_stream_impl` — `let _guard = rt_handle.enter()` |
| Authoritative doc comment (D1) | `crates/ui/src/bin/cockpit_live.rs` | lines 104-126 |
| Cancel primitive (D2) | `crates/backtest/src/cancel.rs` | `RunCancelHandle` + `RunCancelReceiver` |
| Preload loop (D1+D2+D4) | `crates/ui/src/lab/runner.rs` | `spawn_lab_run` — `rt.spawn(preload_yahoo_bars)` + ticker `rt.enter()` guard |
| Engine spawn (D4 proven-in-production) | `crates/ui/src/lab/runner.rs` | `rt.spawn(run_scenario)` — engine call, same pattern |
| D3 timer test | `crates/ui/tests/lab_runner_ticker_e2e.rs` | `ticker_fires_at_least_3_times_in_1s_window` |
| D3 cancel test | `crates/ui/tests/lab_runner_cancel_e2e.rs` | `stop_during_preload_exits_within_500ms` |
| D3/D4 HTTP/spawn_blocking test | `crates/ui/tests/lab_runner_http_offexecutor_e2e.rs` | `spawn_blocking_without_rt_spawn_panics` + `spawn_blocking_with_rt_spawn_does_not_panic` + `abort_stops_spawned_task` |

## Changelog

- 2026-05-29 (architect+developer): codified on 3rd recurrence per
  twice-bitten threshold; see `docs/dev-notes/bug-64-arch-validation-2026-05-29.md § 5`.
  D1: rt_handle.enter() invariant before any tokio reactor API in
  iced::Task::perform closures. D2: tokio_util::sync::CancellationToken as
  canonical primitive. D3: timer-fired-in-bounded-window test contract.
  Fix lands in same commit: `crates/backtest/src/cancel.rs` (CancellationToken
  primitive swap) + `crates/ui/src/lab/runner.rs` (D-R1.1 rt.enter() guard +
  D-R2.2 cancel select! arm + D-R1.4 yield_now defense).

- 2026-05-29 (hotfix): Architect Q1 assertion that `fetch_with_backoff` works
  without `rt.enter()` because reqwest spawns internally was FALSIFIED by
  operator re-verify. `tokio::time::sleep` / `tokio::time::timeout` inside the
  CALLING stack frame DOES require runtime context regardless of what reqwest
  does internally. Operator hit "there is no reactor running" panic at
  `crates/ui/src/lab/runner.rs:395` (the `tokio::time::timeout` call inside
  `fetch_with_backoff` during cold-cache Yahoo fetch).

  **D1 invariant now applies to ALL `tokio::time::*` and `tokio::select!` calls
  reachable from `iced::Task::perform`, no exceptions.** Previously the D1
  site audit table incorrectly listed `runner.rs:395/405/436` as "ASSESSED —
  architect confirmed IO works; no additional guard needed". That assessment
  was wrong.

  **D3 test contract AMENDMENT**: D3 tests MUST run under plain `#[test]` (NOT
  `#[tokio::test]`) to exercise the PRODUCTION runtime context. `#[tokio::test]`
  provides an implicit tokio reactor context which masks the absence of
  `rt.enter()` guards — this is why the existing `lab_runner_ticker_e2e` and
  `lab_runner_cancel_e2e` tests (under `#[tokio::test]`) PASSED while production
  PANICKED. New gate: `crates/ui/tests/lab_runner_cold_cache_fetch_e2e.rs`
  uses plain `#[test]` + `futures::executor::block_on` to simulate iced's
  non-tokio executor context.

  Fix: `fetch_with_backoff` now accepts `rt: &tokio::runtime::Handle`. All
  three `tokio::time::*` calls use the guard-construct-drop-then-await pattern:
  `{ let _guard = rt.enter(); tokio::time::timeout/sleep(...) }`. The
  `EnterGuard` is `!Send` and MUST NOT be held across `.await` points; dropping
  before await is the canonical pattern (matches D-R1.1 fix at runner.rs:756
  for `tokio::time::interval`). See bug-64-d11-attempt-3 hotfix commit for
  full diff.

- 2026-05-29 (rt.spawn revalidation — recurrence #3 durable fix):
  The hotfix above was ALSO insufficient. Operator cold-cache re-verify hit
  the SAME panic at `hyper-util .../connect/dns.rs:119` ("no reactor
  running") — now in reqwest's DNS resolver, not in our `tokio::time::*`
  calls. Root cause: the `rt.enter()` guard drops before `.await`, so
  reqwest's `GaiResolver::spawn_blocking` fires with no reactor on the
  polling thread. The prior hotfix fixed the explicit timers but missed the
  TRANSITIVE `spawn_blocking` one level deeper.

  **D1 model corrected** — `rt.enter()` guards are insufficient for lazy /
  transitive tokio futures (including reqwest). `rt.spawn(async { ... })`
  is the invariant for any future that calls `tokio::*` across its own
  await points. See corrected D1 body above.

  **D4 added** — HTTP/reqwest from iced executor MUST use `rt.spawn()`.
  See D4 body above.

  **D3 amended** — HTTP path test (not just timers) required as regression
  gate. New: `crates/ui/tests/lab_runner_http_offexecutor_e2e.rs`.

  **Fix**: `spawn_lab_run` now runs the entire `preload_yahoo_bars` call
  via `rt.spawn(async move { preload_yahoo_bars(cfg, range).await })`.
  The cancel arm calls `fetch_join.abort()` (not just drops the handle).
  The per-line `rt.enter()` guards in `fetch_with_backoff` are removed
  (redundant and misleading once the task runs on-runtime).
  Reference: `bug-64-arch-revalidation-rt-spawn-2026-05-29.md`.
