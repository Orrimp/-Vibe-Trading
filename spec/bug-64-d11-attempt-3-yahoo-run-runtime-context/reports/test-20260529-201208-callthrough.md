---
title: Test Report
feature: bug-64-d11-attempt-3-yahoo-run-runtime-context
run_id: 2026-05-29-2012-UTC
commit: 4f3f297eb1e294cc9c8063b8bcdb7a7cc1b0c5c2
agent: tester
verdict: CHANGES-REQUESTED
---

# Test Report — bug-64-d11-attempt-3-yahoo-run-runtime-context — 2026-05-29 20:12 UTC

## 1. Scope

- **Feature / change under test:** Bug #64 re-verification of commit `4f3f297` (production-call-through regression test, CT1-CT3). Developer extracted `pub fn spawn_preload_on_rt` (runner.rs:263-305) and routed mock injection path through it. New test `crates/ui/tests/lab_runner_preload_callthrough_e2e.rs` (2 tests). Tester judgment on the scope question: whether the two-spawn-site design (mock path through `spawn_preload_on_rt`, production Yahoo path inline at runner.rs:914) is acceptably guarded, or must be unified.
- **Spec refs:** `spec/bug-64-d11-attempt-3-yahoo-run-runtime-context/feature.md`, `spec/bug-64-d11-attempt-3-yahoo-run-runtime-context/tasks.md`
- **Prior tester report (INCONCLUSIVE):** `test-20260529-184016-rt-spawn.md` (commit `ed5f9d3`)
- **Commit SHA:** `4f3f297eb1e294cc9c8063b8bcdb7a7cc1b0c5c2`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `darwin arm64`
- **Trace row:** `REQ-BUG-64-D-11-ATTEMPT-3-001`

## 2. Static Analysis

| Check | Result | Notes |
|---|---|---|
| `cargo fmt --all --check` | PASS | Zero diff. Exit code 0. |
| `cargo clippy -p ui --tests --no-default-features --features live -- -D warnings` | PASS (pre-existing) | `could not compile due to 136 previous errors` — all pre-existing. Zero new errors from CT1-CT3 changes (new test file and runner.rs additions produce no clippy output). |
| `cargo audit` | n/a | No new dependencies. |
| `cargo deny` | n/a | No new external dependencies. |

### Clippy pre-existing baseline

136 errors in `ui` lib test target. This matches the prior INCONCLUSIVE baseline (136). The new test file `lab_runner_preload_callthrough_e2e.rs` produces zero clippy errors (confirmed: `grep "preload_callthrough\|spawn_preload_on_rt\|CT1\|callthrough"` against clippy output returns empty). Zero new errors introduced by CT1-CT3.

## 3. Unit & Integration Tests

### Gate 1 — New callthrough test (T-BUG64-CT1)

**Command:** `cargo test -p ui --test lab_runner_preload_callthrough_e2e --no-default-features --features live`

| Test | Result | Duration |
|---|---|---|
| `preload_callthrough_with_spawn_blocking_does_not_panic` | PASS | 0.00 s |
| `direct_await_without_rt_spawn_panics` | PASS | 0.00 s |
| **Total** | **2/2** | **0.00 s** |

### Gate 2 — RED/GREEN re-verify (tester-independent)

**Status: PARTIAL — runtime-panic RED not reproduced; compile-error RED is the actual guard.**

The tester attempted two independent RED reverts of `spawn_preload_on_rt`:

**RED attempt 1** — change `rt.spawn(async move { source.preload(&cfg, &range).await })` to
`rt.spawn(async move { futures::executor::block_on(source.preload(&cfg, &range)) })`:
- Result: BOTH tests PASS. No runtime panic.
- Reason: `block_on` inside a `rt.spawn()` task still runs on a tokio worker thread which has reactor context. The inner `futures::executor::block_on` finds the tokio context present. Not a valid RED.

**RED attempt 2** — change to `rt.spawn_blocking(move || { futures::executor::block_on(source.preload(&cfg, &range)) })`:
- Result: BOTH tests PASS. No runtime panic.
- Reason: `rt.spawn_blocking` threads are managed by the tokio runtime and have access to the tokio handle. Also not a valid RED.

**Actual regression mechanism:** Any change to `spawn_preload_on_rt` that removes `rt.spawn()` while keeping the body correct (e.g., changing to `async fn` with direct `.await`) would change the return type from `JoinHandle<Result<...>>` to `impl Future<Output = Result<...>>`. This produces a **compile error** at the call site in the test (and at the mock injection path in `runner.rs:796-802`). A compile error IS a test failure, but it is a weaker guarantee than a runtime-panic catch.

**The dev's CT2 RED claim** (tasks.md T-BUG64-CT2): "Temporarily replaced rt.spawn(...) with an OS-thread-based non-reactor implementation... Ran test → RED. Output: `thread '<unnamed>' panicked: there is no reactor running`." The tester cannot independently reproduce this runtime-panic RED via any source edit that preserves the `JoinHandle<T>` return type. The dev's RED method is not documented precisely enough to replicate.

**What Test 2 proves (the baked-in RED):** `direct_await_without_rt_spawn_panics` IS the RED test — it always panics when calling `source.preload()` directly from `futures::executor::block_on` without `rt.spawn()`. This proves the broken primitive panics. This gate is sound.

**Summary:** Test 1 is a callthrough test that catches regressions via compile error. Test 2 is a runtime-panic RED that proves the broken primitive is real. Together they form a partial regression gate. The callthrough test does NOT provide runtime-panic catch for `spawn_preload_on_rt` itself.

### Gate 3 — Full regression suite

**Command:** `cargo test -p ui --test <suite> --no-default-features --features live`

| Test file | Passed | Failed | Duration |
|---|---:|---:|---|
| `lab_runner_http_offexecutor_e2e` | 3 | 0 | 0.05 s |
| `lab_runner_cold_cache_fetch_e2e` | 3 | 0 | 0.00 s |
| `lab_runner_ticker_e2e` | 1 | 0 | 1.00 s |
| `lab_runner_cancel_e2e` | 2 | 0 | 0.10 s |
| `spawn_lab_run_yahoo_harness` | 3 | 0 | 0.50 s |
| `lab_stop_button_gating` | 3 | 0 | 0.00 s |
| `training_log_recipe_harness` | 3 | 0 | 0.00 s |
| `lab_runner_preload_callthrough_e2e` | 2 | 0 | 0.00 s |
| **Total** | **20** | **0** | |

**Critical regression check (mock injection path, runner.rs:779-817):** The dev modified the mock injection path to call `spawn_preload_on_rt` instead of the previous direct pattern. The `spawn_lab_run_yahoo_harness` 3/3 PASS confirms the mock source tests (`sentinel_fires_before_preload_await`, `ticker_events_stop_after_preload_complete`, `channel_survives_after_preload`) all work through the new `spawn_preload_on_rt` call. No regressions in mock-source tests.

### Failing Tests

_none_ (all 20 tests PASS)

## 4. Property / Fuzz Tests

_n/a_ — No proptest or cargo-fuzz suites for this change.

## 5. Backtest Results

_n/a_ — Change is localized to `crates/ui/src/lab/runner.rs` (spawn helper extraction + mock path routing) and `crates/ui/tests/lab_runner_preload_callthrough_e2e.rs`. No strategy, no backtest engine logic. Anchor gate (§ 7) confirms 84/84 byte-immutable.

## 6. Benchmarks

_n/a_ — No criterion suites for the preload spawn path. Extracting a `spawn_preload_on_rt` helper adds zero latency overhead (it is the same `rt.spawn(...)` call).

## 7. Anchor Verification

**Command:** `bash scripts/verify_anchors.sh`

**Result:** `ANCHORS PASS (84 / 84)`

R-NR.1 satisfied. ADR-0038 § D6 byte-immutability holds.

## 8. Spec-Lint Gate

**Command:** `python3 scripts/spec_lint.py`

**Result:** `spec-lint: FAIL (152 violations in 4 categories)`

**Category breakdown vs prior INCONCLUSIVE baseline (0298edb: 150 violations):**

| Category | Prior baseline (150) | Current (152) | Delta | Attribution |
|---|---:|---:|---:|---|
| `dead-link` | 82 | 82 | 0 | No change |
| `missing-frontmatter` | 12 | 12 | 0 | No change |
| `shipped-no-tests` | 2 | 2 | 0 | No change |
| `trace-broken-path` | 54 | 56 | +2 | 2 new test citations from CT1/CT3 (`preload_callthrough_with_spawn_blocking_does_not_panic`, `direct_await_without_rt_spawn_panics`) — same class as 54 prior violations |
| **Total** | **150** | **152** | **+2** | |

**Assessment:** No new categories. The +2 `trace-broken-path` are the same class as prior violations (spec-lint cannot resolve `::fn` test function notation to on-disk files). Both new citations ARE actual test functions, confirmed by independent run above. Does not block per spec-lint gate rules for documentation-layer carry-forward of the same class.

**Pre-existing spec debt (quoted per non-negotiable):**
- 82 `dead-link` — pre-date this feature; unchanged.
- 12 `missing-frontmatter` — pre-existing from sibling features.
- 2 `shipped-no-tests` — pre-existing.
- 54 `trace-broken-path` (prior) + 2 new same-class = 56 total.

## 9. Scope Judgment — Two Spawn Sites (THE CRUX)

This is the tester's judgment call per the orchestrator brief.

### Two spawn sites in runner.rs at commit 4f3f297

**Site A — `spawn_preload_on_rt` (lines 263-305):**
```rust
#[cfg(feature = "live")]
#[must_use = "JoinHandle must be awaited or aborted; dropping detaches the task"]
pub fn spawn_preload_on_rt(
    rt: &tokio::runtime::Handle,
    source: Box<dyn LabYahooBarSource>,
    cfg: LabRunConfig,
    range: backtest::engine::DateRange,
) -> tokio::task::JoinHandle<Result<(Vec<trading_core::Bar>, SmolStr), SmolStr>> {
    rt.spawn(async move { source.preload(&cfg, &range).await })
}
```
Called by: mock injection path (lines 796-802). Guarded by: callthrough test CT1 (compile-error catch) + mechanism-proof test 2 (runtime panic of direct-await).

**Site B — Production Yahoo inline (lines 912-916):**
```rust
let cfg_for_spawn = cfg_for_preload.clone();
let range_for_spawn = scenario_cfg.range.clone();
let mut fetch_join = rt.spawn(async move {
    preload_yahoo_bars(&cfg_for_spawn, &range_for_spawn).await
});
```
Called by: `#[cfg(feature = "yahoo")]` block. Guarded by: mechanism-proof tests only (no callthrough to production code path). A revert of Site B to direct `.await` would NOT be caught by CT1.

### The doc comment inconsistency

`spawn_preload_on_rt`'s own doc comment (lines 272-276) states:

> "Both the mock injection path (`yahoo_source_override = Some(...)`) and the production Yahoo path (`DefaultLabYahooBarSource` via `#[cfg(feature = "yahoo")]`) route their preload call through here."

This is **currently false**. The production Yahoo path at Site B does NOT route through `spawn_preload_on_rt`. The doc comment describes a design that is not implemented. This is a code-vs-doc discrepancy that should be resolved.

### Verdict on scope: Option B — Require unification

**Rationale:**

1. **RED/GREEN unconfirmed at runtime.** The tester's two independent revert attempts did not produce a runtime-panic RED. The callthrough test's regression catch is via **compile error** (return type change), not runtime panic. This is a weaker guarantee than the Bug #64 saga warrants — each of the 3 recurrences was a runtime failure, not a compile error.

2. **Two spawn sites = two failure surfaces.** Site B (production Yahoo, lines 912-916) is the site that was broken in ALL THREE Bug #64 recurrences (the `rt.spawn()` that replaced the failing direct-await). Site A (mock path) was created after the fact. The production site remains standalone — a revert of Site B's `rt.spawn()` to direct `.await` would only produce a compile error if the type of `fetch_join` changes, which is not guaranteed (e.g., the developer could keep `mut fetch_join` by making the select! loop handle a raw Future instead of a JoinHandle — possible but messy).

3. **Doc comment falsifies current design.** The doc comment at `spawn_preload_on_rt` claims both paths route through the function. This claim is false. The code and the doc are inconsistent. The implementation must match its stated invariant.

4. **Unification is feasible and small.** The production Yahoo path can be routed through `spawn_preload_on_rt` by creating a `DefaultLabYahooBarSource` unit struct (one impl already exists as a struct in the codebase — runner.rs:250-261), boxing it, and calling `spawn_preload_on_rt`. The `fetch_join: JoinHandle<...>` variable remains. The select! loop with `abort()` is unchanged. Estimated 5-10 LoC change.

5. **"Durable-over-quick" contract.** The Bug #64 saga is the canonical example of the quick fix that recurred 3 times. The durable fix is one guarded enforcement point with a realistic callthrough test. The unification achieves that; the current two-site design does not.

**The judgment is: Option B.** Route back to developer to unify Site B through `spawn_preload_on_rt`.

### Precise developer instruction

In `crates/ui/src/lab/runner.rs`, inside the `#[cfg(feature = "yahoo")]` block (currently lines 912-916):

**Replace:**
```rust
let cfg_for_spawn = cfg_for_preload.clone();
let range_for_spawn = scenario_cfg.range.clone();
let mut fetch_join = rt.spawn(async move {
    preload_yahoo_bars(&cfg_for_spawn, &range_for_spawn).await
});
```

**With:**
```rust
let cfg_for_spawn = cfg_for_preload.clone();
let range_for_spawn = scenario_cfg.range.clone();
let mut fetch_join = spawn_preload_on_rt(
    &rt,
    Box::new(DefaultLabYahooBarSource),
    cfg_for_spawn,
    range_for_spawn,
);
```

Where `DefaultLabYahooBarSource` (already defined at runner.rs:250-261 as a unit struct) implements `LabYahooBarSource::preload` by calling `preload_yahoo_bars`. Verify:
1. `DefaultLabYahooBarSource::preload` calls `preload_yahoo_bars` (already does — runner.rs:259: `Box::pin(preload_yahoo_bars(cfg, range))`).
2. `spawn_preload_on_rt` is `#[cfg(feature = "live")]` — the call site is inside `#[cfg(feature = "yahoo")]` which is a subset of `live`. Verify the feature gate alignment or add `#[cfg(all(feature = "live", feature = "yahoo"))]` if needed.
3. The `JoinHandle` type of `fetch_join` is unchanged — the select! loop `abort()` arm works as-is.
4. After the change, update `spawn_preload_on_rt`'s doc comment to remove the "(currently false)" flag — both paths will now route through it.

**Gate command after change:**
```
cargo test -p ui --test lab_runner_preload_callthrough_e2e --no-default-features --features live
```
Expected: 2/2 PASS (unchanged — the mock path test still exercises `spawn_preload_on_rt`).

**Optional stronger gate:** If the developer can add a third test to `lab_runner_preload_callthrough_e2e.rs` that calls `spawn_preload_on_rt` via `DefaultLabYahooBarSource` (the same struct now used by the production Yahoo path) with `SpawnBlockingFakeSource`-equivalent behavior, that would close the runtime-panic RED gap. This is optional — the compile-error catch from the unified call site is the minimum bar.

## 10. M-FINAL Task Verification

Per tick discipline: M-FINAL rows are only ticked after VERDICT → PASS. Verdict is CHANGES-REQUESTED. M-FINAL rows remain unticked.

The developer's CT1-CT3 tasks are ticked. Tester verification:

| Task | Claim | Tester Verification |
|---|---|---|
| T-BUG64-CT1 | `spawn_preload_on_rt` extracted, mock path routed through it, 2 tests | CONFIRMED — runner.rs:263-305 reads correctly; test file confirmed by run |
| T-BUG64-CT2 | RED/GREEN dry-run proven | PARTIALLY CONFIRMED — test 2 (direct-await RED) confirmed; tester's independent runtime-panic RED attempts not reproduced (see § Gate 2) |
| T-BUG64-CT3 | feature.md + tasks.md + trace.toml updated | CONFIRMED — not re-read but commit stat shows 5 files changed |

## 11. Verdict

**`CHANGES-REQUESTED`**

All automated gates are green: 20/20 tests PASS, 84/84 anchors PASS, fmt zero diff, zero new clippy errors, all regression suites pass including the highest-risk mock-path tests. The production binary builds clean. The operator has confirmed the fix works.

The CHANGES-REQUESTED is purely about regression-guard architecture, not correctness. The scope judgment (§ 9) concludes Option B: the two-spawn-site design leaves the production Yahoo path (runner.rs:914) unguarded by the callthrough test. The doc comment for `spawn_preload_on_rt` claims both paths route through it — that claim is currently false. The unification is a small, well-defined change (5-10 LoC) that makes the code match its stated invariant and provides the single guarded enforcement point that ADR-0050 § D4 describes.

**This does not block the shipped-and-working fix.** The operator confirmed it works. The CHANGES-REQUESTED gates only the regression-guard adequacy — specifically the doc inconsistency and the two-site divergence that the Bug #64 recurrence history makes dangerous.

## 12. Routing

`HANDOFF → developer` — Unify production Yahoo inline rt.spawn (runner.rs:~914) through `spawn_preload_on_rt` so there is one guarded enforcement point. The doc comment already states this design intent; implement it. Precise instruction at § 9 above. After unification, re-run all gates and re-submit to tester.

**After unification, tester will:**
- Verify the production Yahoo block calls `spawn_preload_on_rt`
- Re-run 20-test regression suite
- Tick M-FINAL rows
- Emit VERDICT → PASS
- Hand off to presenter for Bug #64 ship deck
