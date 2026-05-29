---
slug: bug-64-d11-attempt-3-yahoo-run-runtime-context
version: 0.1.0
status: in-progress
owner: tester
updated: 2026-05-29
related:
  - spec/dev-notes/bug-64-yahoo-run-code-map-2026-05-29.md
  - spec/dev-notes/bug-64-arch-validation-2026-05-29.md
  - spec/dev-notes/bug-64-analyst-validation-2026-05-29.md
  - spec/dev-notes/bug-64-d11-attempt-3-investigation-2026-05-29.md
  - spec/dev-notes/operator-side-pending-ledger.md
---

# Bug #64 D.1.1 attempt-3 — Yahoo+Run runtime context + cancellation

> **Bug-fix feature.** Closes the 3rd recurrence of the iced-tokio
> runtime-context-absence bug + the structural cancellation omission
> during Yahoo cold-cache preload. Both root causes confirmed in
> lockstep by the developer code-map + architect validation +
> analyst validation 2026-05-29.

## § Problem statement

Operator reported 2026-05-29: Lab → Yahoo → SOL → Run produces
**"endless spinning, no progress visible, cannot stop the running
task."** Two distinct regressions in attempt-2:

**R1 — Progress label dormant.** Loading label does not tick
during the 30-60 s cold-cache Yahoo auto-fetch window. Label sits
permanently at `0/1 · 0.0s`.

**R2 — Stop button broken.** Stop dispatch during the cold-cache
window does nothing; operator must wait for fetch to complete OR
force-quit the cockpit.

## § Root cause analysis

Read the 3 dev-notes in `related:` for the full investigation.
Summary:

**R1 root cause (H-R1d, 3rd recurrence)**:
- `crates/ui/src/lab/runner.rs:744` calls
  `tokio::time::interval(250ms)` inside an `iced::Task::perform`
  async closure WITHOUT a tokio runtime context guard.
- Every working recipe in the codebase
  (`ServerTimeRecipe::server_time_stream_impl`,
  `ToastDismissRecipe::toast_dismiss_stream_impl`,
  `LabProgressRecipe::stream_impl`) explicitly calls
  `let _guard = rt_handle.enter();` BEFORE invoking
  `tokio::time::*` APIs.
- See `crates/ui/src/bin/cockpit_live.rs:104-126` doc comment for
  the P1 fix rationale (this bug was fixed once before on
  2026-05-23). This is the 3rd occurrence.
- Symptom-match: silent ticker pending forever, label sits at
  `0/1 · 0.0s` — exact operator failure description.

**R2 root cause (structural omission)**:
- `crates/backtest/src/cancel.rs::RunCancelReceiver` exposes only
  synchronous `is_cancelled()`. No `notified() -> impl Future`
  method.
- `cancel_rx.is_cancelled()` is called at 4 locations, all INSIDE
  `backtest::engine::run_scenario`, which is called at
  `runner.rs:837` — AFTER the select! preload loop exits.
- During the entire cold-cache window (`runner.rs:778-827`), ZERO
  cancel checks exist on the preload future.
- Fix requires either (a) new `notified()` method on
  `RunCancelReceiver` OR (b) primitive swap to
  `tokio_util::sync::CancellationToken`.

## § Design (locked by architect M-T1 at commit 4473bd2)

8 design clauses (full design in
[`bug-64-arch-validation-2026-05-29.md` § 3](../dev-notes/bug-64-arch-validation-2026-05-29.md)):

- **D-R1.1**: `let _guard = rt_handle.enter();` at the top of the
  iced::Task::perform async closure in `spawn_lab_run`, BEFORE
  `tokio::time::interval(250ms)` is constructed at
  `crates/ui/src/lab/runner.rs:744`.
- **D-R1.2**: Extend the rt_handle invariant to ALL reactor APIs
  called inside `iced::Task::perform` closures across `crates/ui/`.
  Audit + fix any other site. Defensive guard for
  `LabProgressRecipe::stream_impl` if found missing.
- **D-R1.3**: E2E test — ticker MUST fire ≥ 3 times during a 1 s
  bounded preload window. New test under
  `crates/ui/tests/lab_runner_ticker_e2e.rs` (or sibling).
- **D-R2.1**: Adopt `tokio_util::sync::CancellationToken` in
  `crates/backtest/src/cancel.rs`. Either replace
  `RunCancelReceiver` internals with `CancellationToken` OR add a
  `notified() -> impl Future` method that bridges to a
  `Notify`-backed signaller. Architect-recommended:
  CancellationToken primitive swap.
- **D-R2.2**: Add a third arm to the existing
  `tokio::select!` at `crates/ui/src/lab/runner.rs:705-828`
  preload loop that listens on `cancel.cancelled()` (the new
  CancellationToken future). When fires, exit the preload loop
  with an `Err(SmolStr::new("operator cancelled"))` (or similar).
- **D-R2.3**: E2E test — cancel-during-preload MUST exit within
  ≤ 500 ms wall-clock of Stop being dispatched. New test in
  `crates/ui/tests/lab_runner_cancel_e2e.rs` (or sibling).
- **D-R1.4** (operator-decide, A-Q1): `tokio::task::yield_now()`
  defensive yield at top of the preload loop. Architect bias YES
  for defense-in-depth. Cheap (~3 LoC).
- **D-Tr.1**: trace.toml row creation for
  `REQ-BUG-64-D-11-ATTEMPT-3-001` with arch + crates + tests +
  state columns wired.

## § ADR-0050 (NEW, atomic-register obligation)

Per architect.md atomic-register contract: developer MUST author
ADR-0050 in the same commit as the fix.

**ADR-0050 title**: "iced ↔ tokio runtime-context contract and
cooperative cancellation primitives"

- **D1**: rt_handle.enter() invariant. Mandatory before any
  tokio reactor API in iced::Task::perform closures. Codified
  this contract because this is the 3rd recurrence (fixed on
  2026-05-23, 2026-05-2X attempt-2, now 2026-05-29 attempt-3).
- **D2**: tokio_util::sync::CancellationToken as canonical
  cancellation primitive. Replaces ad-hoc bool flags.
- **D3**: Timer-fired-in-bounded-window test contract. Every
  iced::Task::perform closure that constructs a tokio timer MUST
  have an e2e test asserting the timer fires ≥ N times in a
  bounded window.

**Atomic-register obligation**:
1. Write `spec/architecture/adr/0050-iced-tokio-runtime-context.md`.
2. Append a row to `spec/architecture/adr/README.md` table.
3. Bump `spec/architecture/adr/README.md` frontmatter
   `updated:` field.
4. All in the SAME commit as the fix.

ADR-0048 § Changelog also gets an amendment row.

## § Constraints

- R-NR.1 — Anchored backtest reports byte-immutable. Verify
  `bash scripts/verify_anchors.sh` → 84/84 PASS after fix.
- R-NR.2 — All v0.1.0/v0.2.0/Wave A regression tests stay PASS:
  - `cargo test -p ui --test spawn_lab_run_yahoo_harness
    --no-default-features --features live` → 3/3 PASS
  - `cargo test -p ui --test lab_stop_button_gating
    --no-default-features --features live` → 3/3 PASS
  - `cargo test -p ui --test training_log_recipe_harness
    --no-default-features --features live` → 3/3 PASS
- R-NR.3 — No production code changes outside `crates/ui/` and
  `crates/backtest/src/cancel.rs`. The fix is localized.
- R-NR.4 — One-ship Q1=(a) — R1 + R2 land in one PR + one ADR.
- R-NR.5 — ADR-0050 atomic-register per the 2026-05-29 contract.

## § Hypotheses

- **H1**: rt_handle.enter() guard at runner.rs:744 + sibling sites
  causes the ticker to fire ≥ 3 times during a 1 s preload window.
  TEST: D-R1.3 e2e.
- **H2**: CancellationToken in cancel.rs + select! third arm at
  runner.rs:705-828 causes Stop dispatch to exit preload within
  500 ms. TEST: D-R2.3 e2e.
- **H3**: No regression to existing harness tests (3/3 each of
  spawn_lab_run_yahoo_harness, lab_stop_button_gating, Wave A
  training_log_recipe_harness).

## § Operator-decide questions

All resolved by architect + analyst validation. Tracked here:

- **Q1=(a)** one-ship R1+R2 (analyst Q-BUG64-D11-3-Q1
  ratification).
- **Q2=(a)** keep select!+ticker + add rt_handle.enter() guard.
- **Q3=(a)+(a.i)** cancel-token wrap with CancellationToken
  primitive swap.
- **Q4 (CLOSED)** = D-R1.2 (LabProgressRecipe defensive guard).
- **Q5 (CLOSED)** = keep Recipe pattern (analyst + architect both
  vote KEEP over iced::time::every migration).
- **A-Q1 (CLOSED)** = D-R1.4 ship yield_now() (architect YES,
  cheap).
- **A-Q2 (CLOSED)** = tokio-util dep shape
  `default-features = false, features = ["rt"]`.
- **A-Q3 (CLOSED)** = codify ADR-0050 NOW (twice-bitten + this
  3rd recurrence; codify-on-3 threshold met).

## § Verdict tree (4-cell)

|  | Code work succeeds | Code work fails / blocks |
|---|---|---|
| **Operator re-verify confirms fix** | `PASS` — v0.1.0 ships; Bug #64 closes; ADR-0050 codified. | `INCONCLUSIVE` — code looks right but operator can't reproduce success; possible environment issue (binary cache, feature flag); route to operator-side recipe update. |
| **Operator re-verify still fails** | `REGRESSION` — fix didn't land; need attempt-4 root-cause. | `FAIL` — same as before + dev work blocked; defer or escalate. |

## § Hotfix (2026-05-29)

Operator cold-cache re-verify hit a NEW panic at `crates/ui/src/lab/runner.rs:395`:
`"there is no reactor running, must be called from the context of a Tokio 1.x runtime"`.

**Root cause**: The architect's Q1 assessment in `bug-64-arch-validation-2026-05-29.md`
stated that `tokio::time::timeout/sleep` in `fetch_with_backoff` (lines 395/405/436)
"work without `rt.enter()` because reqwest spawns internally". That assessment was
FALSIFIED. `tokio::time::timeout` / `tokio::time::sleep` need the reactor at
CONSTRUCTION TIME in the calling stack frame, not just inside reqwest's internal
spawns.

**Why the existing e2e tests didn't catch it**: `lab_runner_ticker_e2e` and
`lab_runner_cancel_e2e` both use `#[tokio::test]` which provides an implicit
tokio reactor context. The production path (`iced::Task::perform` on
`futures::ThreadPool`) has NO reactor context. Tests passed; production panicked.

**Fix (T-BUG64-D13)**: Added `rt: &tokio::runtime::Handle` parameter to
`preload_yahoo_bars` and `fetch_with_backoff`. Inside `fetch_with_backoff`,
each `tokio::time::*` call uses the guard-construct-drop pattern:
```rust
let timeout_future = {
    let _guard = rt.enter();  // enter context, construct future
    tokio::time::timeout(per_attempt_timeout, fetch_future)
    // _guard dropped here — EnterGuard is !Send, MUST drop before .await
};
timeout_future.await
```
`DefaultLabYahooBarSource` struct gained `pub rt: tokio::runtime::Handle` to
carry the handle through the `LabYahooBarSource` trait boundary.

**New e2e test (T-BUG64-D14)**: `crates/ui/tests/lab_runner_cold_cache_fetch_e2e.rs`
uses plain `#[test]` (NOT `#[tokio::test]`) + `futures::executor::block_on` to
simulate iced's non-tokio executor. 3 tests: (1) proves `tokio::time::timeout`
WITHOUT `rt.enter()` panics (falsification probe), (2) proves WITH guard no panic
(core gate), (3) same for `tokio::time::sleep` (backoff path).

**ADR-0050 § Changelog amended (T-BUG64-D16)**: D1 invariant extended to ALL
`tokio::time::*` calls reachable from `iced::Task::perform`, no exceptions.
D3 test contract amended: timer tests MUST use plain `#[test]` not
`#[tokio::test]` to avoid masking the absence of `rt.enter()` guards.

## § Hotfix-2 (rt.spawn — recurrence #3, 2026-05-29)

Operator cold-cache re-verify panicked AGAIN at
`hyper-util-0.1.20/src/client/legacy/connect/dns.rs:119:24:
there is no reactor running, must be called from the context of a Tokio 1.x runtime`.

**Why the hotfix-1 fix was also insufficient**: `rt.enter()` guards set a
thread-local that is dropped at the first `.await` boundary. reqwest's
`GaiResolver` DNS resolver calls `tokio::task::spawn_blocking` lazily
INSIDE the awaited HTTP future, long after all construction-scoped guards
have dropped. The hotfix-1 fixed explicit `tokio::time::*` calls at
construction time (correct for the K8 pattern) but was structurally
incapable of covering reqwest DNS — which fires at a different point in the
call graph.

**Architect re-validation**: `bug-64-arch-revalidation-rt-spawn-2026-05-29.md`
(commit `3329350`) owns the falsified assertion, derives the mechanism from
tokio source, and validates `rt.spawn(...).await` as the durable fix.

**The fix (T-BUG64-RS1..RS6)**:

- **RS1**: Spawn the entire `preload_yahoo_bars` call onto `rt` via
  `rt.spawn(async move { preload_yahoo_bars(cfg, range).await })`.
  The spawned task runs on tokio worker threads → reactor always present.
  Mirror of the existing proven pattern at `runner.rs` engine call.

- **RS2**: Remove the now-redundant `rt.enter()` guards from
  `fetch_with_backoff`. Remove `rt: &Handle` parameter from
  `fetch_with_backoff` and `preload_yahoo_bars`. Remove `rt` field from
  `DefaultLabYahooBarSource` (now a unit struct). The guards were both
  insufficient (didn't cover DNS) and redundant (task runs on-runtime).

- **RS3**: Cancel arm MUST call `fetch_join.abort()` on the `JoinHandle`.
  Dropping a JoinHandle only DETACHES the task — the HTTP request keeps
  running. `abort()` is best-effort (stops at next yield point) and
  well within the ≤500 ms Stop SLA.

- **RS4**: New HTTP-path off-executor test at
  `crates/ui/tests/lab_runner_http_offexecutor_e2e.rs`. Three tests:
  (1) proves `spawn_blocking` without `rt.spawn()` panics from
  `futures::executor::block_on` (falsification probe), (2) proves with
  `rt.spawn()` no panic (durable gate), (3) proves `abort()` stops
  spawned tasks (cancel correctness gate).

- **RS5**: ADR-0050 amended: D1 corrected (spawn vs guard decision rule),
  D4 added (HTTP/reqwest must be spawned), D3 amended (HTTP test required).

- **RS6**: This hotfix-2 section + tasks.md T-BUG64-RS1..RS6 + trace.toml.

**Why RS4 is the durable gate**: the prior test (`lab_runner_cold_cache_
fetch_e2e.rs`) only exercised `tokio::time::timeout`/`sleep` wrapping
`std::future::ready(())` — it never hit DNS or `spawn_blocking`. RS4 uses
`tokio::task::spawn_blocking` directly (the exact primitive that panicked)
through `futures::executor::block_on` (the exact executor class). This test
FAILS on pre-fix HEAD and PASSES after.

## § Changelog

- 2026-05-29 (orchestrator): feature folder created from both
  validators (architect 4473bd2 + analyst ccf39b9). Synthesis of
  the locked 8 D-clauses + ADR-0050 obligation. HANDOFF →
  developer.
- 2026-05-29 (developer): T-BUG64-D1..D12 complete. HANDOFF → tester.
- 2026-05-29 (hotfix developer): T-BUG64-D13..D18 complete.
  Falsified architect Q1 assertion re fetch_with_backoff.
  Added rt handle threading + guard pattern in fetch_with_backoff.
  New cold-cache e2e test (plain #[test] — production context).
  ADR-0050 § Changelog amended. HANDOFF → tester (re-verify).
- 2026-05-29 (rt.spawn developer): T-BUG64-RS1..RS6 complete.
  Architect re-validated at commit 3329350. Hotfix-2 also insufficient.
  Durable fix: rt.spawn() for entire preload; abort() in cancel arm;
  new HTTP off-executor test; ADR-0050 D1/D3/D4 amended.
  HANDOFF → tester (re-verify).
- 2026-05-29 (developer, INCONCLUSIVE closure): T-BUG64-CT1 + CT2
  production-call-through regression guard added. Tester returned
  INCONCLUSIVE (Gate 2) because RS4 tests are mechanism-proof only
  and do NOT call through production runner.rs code. Added
  `spawn_preload_on_rt` (pub fn, runner.rs) that wraps
  `source.preload(...).await` in `rt.spawn()` — both the mock
  injection path and tests call this function. New test file
  `lab_runner_preload_callthrough_e2e.rs` (2 tests, plain #[test])
  directly calls `spawn_preload_on_rt` with `SpawnBlockingFakeSource`
  from `futures::executor::block_on`. CT2 dry-run confirmed:
  removing `rt.spawn()` from `spawn_preload_on_rt` causes
  "there is no reactor running" panic (RED); restoring it passes (GREEN).
  HANDOFF → tester (re-verify Gate 2).

## Implementation

Implemented by developer 2026-05-29. All 12 tasks complete.

### Tier 1 — R1 fix (rt_handle context)

**D-R1.1** (`runner.rs:752-760`): Added `let _guard = rt.enter()` guard
inside a block around `tokio::time::interval(250ms)`. Mirrors the exact
pattern from `ServerTimeRecipe` at `live.rs:784-787`. The guard is dropped
immediately after the `Interval` is constructed; the constructed `Sleep`
futures carry their reactor binding.

**D-R1.2** (audit complete): Grep of all `tokio::time::*`/`tokio::spawn`/
`tokio::select` sites in `crates/ui/src/`:
- `live.rs:786, 831` — inside Recipe `stream_impl` bodies WITH `rt_handle.enter()`. OK.
- `training_subscription.rs:104` — inside `Recipe::stream()` WITH `let _guard = self.rt_handle.enter()`. OK.
- `cockpit_live.rs:489, 501` — inside `rt.block_on(...)`. Reactor available. OK.
- `runner.rs:395, 405, 436` — `tokio::time::timeout/sleep` inside `fetch_with_backoff`. Architect confirmed these work due to reqwest's internal tokio spawning; preload IO has worked in all attempts. No additional guard needed.
- `runner.rs:757` — **FIXED** by D-R1.1.
No additional fixes required.

**D-R1.4** (`runner.rs:807`): Added `tokio::task::yield_now().await` at top
of each preload loop iteration (defense-in-depth per architect A-Q1=YES).

### Tier 2 — R2 fix (cancellation)

**D-R2.1** (`crates/backtest/src/cancel.rs`): Full primitive swap from
`std::sync::mpsc::sync_channel(0)` to `tokio_util::sync::CancellationToken`.
Public API backward-compatible: `is_cancelled()` continues to work. New:
`cancelled() -> impl Future` usable in `tokio::select!`. `RunCancelHandle::Drop`
calls `self.token.cancel()`. `crates/backtest/Cargo.toml` updated with
`tokio-util = { workspace = true }` (workspace pin was already `0.7` with
`["rt"]` feature which includes `sync::CancellationToken`).

**D-R2.2** (`runner.rs:814-828`): Added third arm to the preload `select!`
loop: `_ = cancel.cancelled() => { ... return Err(SmolStr::new("operator cancelled during preload")); }`.
Activity handle `fail()` call emits End{Cancelled} via RAII. Biased order:
preload > cancel > ticker.

### Tier 3 — ADR-0050 atomic-register

**T-BUG64-D8**: `spec/architecture/adr/0050-iced-tokio-runtime-context-and-cancellation.md`
authored with D1 (rt_handle.enter() invariant), D2 (CancellationToken canonical
primitive), D3 (timer-fired-in-bounded-window test contract) + Changelog.

**T-BUG64-D9**: `spec/architecture/adr/README.md` — ADR-0050 row appended to
registry table; frontmatter `updated:` bumped to 2026-05-29 (note: the
frontmatter update was applied in the same session). `spec/architecture/adr/0048-lab-recipe-test-harness.md` — Changelog amended with ride-along note.

### Tier 4 — Spec hygiene

**T-BUG64-D10**: `REQ-BUG-64-D-11-ATTEMPT-3-001` row added to `spec/trace.toml`.

**T-BUG64-D11**: `spec/dev-notes/operator-side-pending-ledger.md` Bug #64 row
updated FAILED → fix-in-flight with feature folder link.

### D-R1.2 site audit table

| File:site | Context | Status |
|---|---|---|
| `runner.rs:757` (post-fix) | `iced::Task::perform` closure | FIXED by D-R1.1 — `rt.enter()` guard added |
| `live.rs:784-787` | `ServerTimeRecipe::stream_impl` | OK — explicit `_guard = rt_handle.enter()` |
| `live.rs:827-832` | `ToastDismissRecipe::stream_impl` | OK — explicit `_guard = rt_handle.enter()` |
| `training_subscription.rs:104` | `TrainingEventsPoller::stream` | OK — `let _guard = self.rt_handle.enter()` |
| `cockpit_live.rs:489, 501` | `rt.block_on(async { tokio::spawn(...) })` | OK — inside block_on, reactor present |
| `runner.rs:395, 405, 436` | `fetch_with_backoff` (inside Task::perform) | ASSESSED — architect confirmed IO works; no additional guard needed |
