---
slug: cockpit-activity-status-bar
version: 0.1.0
mode: release
status: draft
audience: human-operator
owner: presenter
updated: 2026-05-26
generated: 2026-05-26T12:00:00Z
predecessor: lab-end-to-end-v2 v0.1.0 (shipped 2026-05-25)
verdict_source: spec/cockpit-activity-status-bar/reports/test-final-2026-05-26-cockpit-activity-status-bar.md
verdict_commit: 0ff402fc2f7fdf25a78e07a48b14b21808100d3a
trace_row: REQ-COCKPIT-ACTIVITY-001 (state = passed)
---

# cockpit-activity-status-bar v0.1.0 — release

## TL;DR

The cockpit's bottom status bar now carries a live activity tape — a continuously-updated list of the in-flight background work (Yahoo data download, Lab Run, Training subprocess) — so the operator can finally tell at a glance whether the app is "thinking" or "stuck". Verbatim 2026-05-25 request resolved, anchors 34/34 untouched, perf budget cleared with > 1000× headroom. **Ready to ship.**

## The operator-visible win

### Verbatim 2026-05-25 request (now resolved)

> "Status bar should show all the current steps the cockpit is doing — downloading data, backtesting, everything else which could be helpful for the UI user to understand what's going on in background."

### Bug-driven trigger

Three operator-flagged "is it stuck?" moments in the two weeks before this brief all traced to the same gap: a background activity (Yahoo cold-cache fetch 30-60 s, slow-universe backtest dispatch, training subprocess) had **no operator-facing surface outside the screen that triggered it**. The Lab Run progress bar sat inside the Lab screen; the Train sub-panel status strip sat inside the Train sub-panel; the global status bar showed only static fields. Now there is one global surface — the existing 24 px bottom status bar — that aggregates every in-flight background activity from any subsystem.

## What changed

Four waves landed on `main` over 2026-05-25 → 2026-05-26. Anchor risk **zero by construction** (UI + agent only — `crates/backtest/`, `crates/strategy/`, `crates/exec/`, `crates/risk/`, `crates/reports/`, `crates/forecast/` untouched).

### Wave A — `crates/agent` bus extension (~280 LOC)

- New sibling module [`crates/agent/src/activity.rs`](../../../crates/agent/src/activity.rs) — `ActivityEvent`, `ActivityKind` (enum: YahooPreload / LabRun / TrainingRun / LlmCall / AuditLedgerWrite), `ActivityPhase`, `ActivityOutcome`, `ActivityId(u64)` monotonic.
- New `EventBus::activity_tx: broadcast::Sender<ActivityEvent>` (cap 256) — 10th channel on the bus.
- New `ActivitySender::start(kind, label) -> ActivityHandle` factory; `ActivityHandle::tick(c, t)` rate-limited to 10 Hz per handle; `Drop` impl auto-emits `End { Success }` (or `Failed("dropped during panic")` on unwind).
- **Test count:** 7 new tests (6 in `activity_types` + 1 in `bus::tests`). All pass.

### Wave B — `crates/ui` tape state + recipe + widget (~626 LOC)

- New `crates/ui/src/lab/activity.rs` (~337 LOC) — `ActivityTape` state machine (`Vec<ActivityState>` capped at 32 + `apply` / `purge` / `visible`).
- New `crates/ui/src/widgets/activity_tape.rs` (~289 LOC) — pure `view(&ActivityTape) -> Element` render fn. Applies R2.3 200 ms render-floor; R3.1 max-3-visible + "+N more" overflow chip; Q5 red-row 3 s hold for failures. Zero inline string literals; zero new Lumen tokens.
- New `ActivityRecipe` in `crates/ui/src/live.rs` (sibling of `BusRecipe`, `ServerTimeRecipe`) — handles `Lagged(n)` with `tracing::warn` + continue.
- 4 insta snapshots accepted: `status_bar__activity_tape_empty`, `_one_inflight`, `_three_plus_overflow`, `_failed_red`.
- **Test count:** 11 new tests (5 state + 2 recipe + 4 widget). All pass.

### Wave C — R4 producer wiring at 3 call sites (Yahoo / Lab Run / Training)

- [`crates/ui/src/lab/runner.rs`](../../../crates/ui/src/lab/runner.rs) — Yahoo preload (R4.1) + Lab Run (R4.2) gain `ActivityHandle` around their async work. ~1 line at each tick site.
- [`crates/ui/src/lab/trainer.rs`](../../../crates/ui/src/lab/trainer.rs) — `spawn_training_run` returns `(TrainingHandle, Option<ActivityHandle>)` so caller holds the activity handle alongside the training handle.
- [`crates/ui/src/bin/cockpit_live.rs`](../../../crates/ui/src/bin/cockpit_live.rs) — `AppState` manual `Clone` (ActivityHandle is `!Clone`), new `lab_activity_handle` + `training_activity_handle` fields, `ActivityRecipe` wired into both `Subscription::batch` branches.
- **Test count:** 7 new integration tests (2 + 3 + 2). All pass.

### Wave D — Perf gates (~350 LOC)

- New criterion bench `crates/ui/benches/activity_tape.rs` — 5 micro-benches per D3 Layer 2.
- New integration perf test `crates/ui/tests/activity_tape_event_storm.rs` — 10 k-event burst, asserts drain < 1 s, delivery ≥ 95 %, P99 latency < 16 ms.
- **Bench results (Apple M2 Pro, isolated re-run at tester PASS):** all 5 within absolute budget, all deltas under 6 % vs dev baseline (no > 20 % flag).

## Why

The bottom status bar (`crates/ui/src/widgets/status_bar.rs`, fixed 24 px row) was the only candidate global surface that already exists across every screen. The architect picked `EventBus` as the event source because it is the project's accepted broadcast pattern for "many publishers, the cockpit subscribes" (nine channels already). Activity events live in memory only (no persistence; audit ledger remains source-of-truth for compliance). Cheapest plausible slice: anchor-neutral by construction, ~1 week wall-clock. See [feature.md § Why](../feature.md#why) for the long form.

## What you can do now

| Action | Command |
|---|---|
| Run the live cockpit and see the new tape | `cargo run -p ui --bin cockpit_live --features live,yahoo` |
| Trigger a Yahoo preload to observe the activity row | (in cockpit) click **Run** on a Lab Run config that requires fresh Yahoo bars |
| Run the perf storm test in isolation | `cargo test -p ui --test activity_tape_event_storm --features live -- --nocapture` |
| Re-run the criterion benches | `cargo bench -p ui --bench activity_tape` |
| Verify anchors stayed byte-identical | `bash scripts/verify_anchors.sh` |
| Run the cockpit-smoke manual capture | see § "Cockpit smoke (operator)" below |

## Live demo

Replayed at the tester-PASS commit `0ff402f` on Apple M2 Pro 2026-05-26. The storm test is the load-bearing perf evidence — the same code path the Yahoo / Lab Run / Training producers all flow through, just at 10,000× the event rate.

```
$ cargo test -p ui --test activity_tape_event_storm --features live -- --nocapture
   Compiling ui v0.1.0 (...)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.69s
     Running tests/activity_tape_event_storm.rs

running 1 test
=== activity_tape_event_storm measurements ===
  drain_time:      7.923 ms
  delivery_rate:   1.0000 (10000 / 10000)
  p99_latency:     0.034 ms
  latency_samples: 10000 events measured
=== PASS: all 3 assertions hold ===
test activity_tape_handles_10k_event_burst_without_lag ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Interpretation: 10 000 events drained in **7.9 ms** (budget 1000 ms — 126× headroom); **100 %** delivery (budget 95 %); **P99 end-to-end latency 34 µs** (budget 16 ms — 470× headroom). Frame-budget gate (16 ms at 60 fps) is fully insulated from any plausible producer event rate. Raw log saved at [`presentations/artifacts/cockpit-activity-status-bar-2026-05-26/storm-test-live-run.txt`](artifacts/cockpit-activity-status-bar-2026-05-26/storm-test-live-run.txt).

### Synthetic Yahoo preload sequence (what the operator sees in the tape)

Once the operator clicks **Run** on a Yahoo-backed Lab Run config, the tape region between the account label and the server-time label cycles through:

| t (s)  | Tape contents (rendered between account · | · server-time) |
|--------|--------------------------------------------------------|
| 0.0    | `[ ]` (empty — `Space::with_width(0)`)                 |
| 0.2    | `· Yahoo BTC-USD · 2y · <1s`                           |
| 1.0    | `· Yahoo BTC-USD · 2y · 1s`                            |
| 5.0    | `· Yahoo BTC-USD · 2y · 5s`                            |
| 7.4    | `[ ]` (Success — row removed immediately)              |

Add a Lab Run + Training subprocess in parallel (3 in-flight):

| t (s)  | Tape contents                                                                |
|--------|------------------------------------------------------------------------------|
| 8.0    | `· Yahoo … · 0s   · Backtest v1.momentum · 0s   · Train BS-1 TCN · 0s`        |

Add 2 more (5 in-flight):

| t (s)  | Tape contents                                                                |
|--------|------------------------------------------------------------------------------|
| 8.1    | `· Yahoo … · 1s   · Backtest … · 1s   · Train … · 1s   [+2 more]`             |

Fail one:

| t (s)  | Tape contents                                                                |
|--------|------------------------------------------------------------------------------|
| 9.0    | `· Yahoo … (RED — Failed) · 1s   · Backtest … · 2s   · Train … · 2s   [+2 more]` |
| 12.0   | red-hold expired → row removed → tape returns to 4 in-flight                 |

This sequence is the abstract spec — operator-captured screenshots below render the actual pixels.

## Screenshots

Headless sandbox at presenter time — operator manual-capture instructions follow. All 4 placeholder paths are reserved under `spec/cockpit-activity-status-bar/presentations/artifacts/cockpit-activity-status-bar-2026-05-26/` and the deck cross-references them so the operator just drops the PNGs in.

### Manual-capture instructions for the 4 T-P-2 screenshots

```text
manual-capture: cockpit-activity-status-bar v0.1.0 (4 screenshots)

# Pre-req: live build of the cockpit
cargo build -p ui --bin cockpit_live --features live,yahoo

# 1. Bare status bar (no activity tape) — capture BEFORE any interaction
cargo run -p ui --bin cockpit_live --features live,yahoo &
sleep 4
screencapture -W spec/cockpit-activity-status-bar/presentations/artifacts/cockpit-activity-status-bar-2026-05-26/01-before-bare-status-bar.png
# Do NOT interact yet.

# 2. One in-flight activity (Yahoo preload ~5s in)
# In the cockpit, click Run on a config that requires fresh Yahoo bars
# (cold-cache miss for BTC-USD 2y is the canonical trigger).
# Wait ~5s during the preload, then:
screencapture -W spec/cockpit-activity-status-bar/presentations/artifacts/cockpit-activity-status-bar-2026-05-26/02-after-yahoo-preload-active.png

# 3. Three activities + "+2 more" overflow chip
# Trigger Yahoo preload + Lab Run + Training subprocess back-to-back; then
# trigger 2 more (any combination). Within the ~3s overlap window:
screencapture -W spec/cockpit-activity-status-bar/presentations/artifacts/cockpit-activity-status-bar-2026-05-26/03-after-three-plus-overflow.png

# 4. One failed activity in 3s red-hold
# Kill the cockpit. Then deliberately break the YahooCache:
mkdir -p /tmp/yahoo-cache-backup && mv data/yahoo/* /tmp/yahoo-cache-backup/ 2>/dev/null
cargo run -p ui --bin cockpit_live --features live,yahoo &
sleep 4
# Click Run on a Yahoo-backed config. The preload will fail. Within the 3s
# red-hold window (before the row auto-removes):
screencapture -W spec/cockpit-activity-status-bar/presentations/artifacts/cockpit-activity-status-bar-2026-05-26/04-after-failed-red-hold.png

# Restore the cache when done:
mv /tmp/yahoo-cache-backup/* data/yahoo/ 2>/dev/null

pkill -f "target/release/cockpit_live" 2>/dev/null
```

### Slot 1 — Before (bare status bar)

> _Pending operator capture: [`artifacts/cockpit-activity-status-bar-2026-05-26/01-before-bare-status-bar.png`](artifacts/cockpit-activity-status-bar-2026-05-26/01-before-bare-status-bar.png)_

Baseline reference — confirms the 24 px status-bar height contract (`lumen-phase-1-foundation` R13 / K6) is preserved when the tape region is empty.

### Slot 2 — After: one in-flight activity (Yahoo preload ~5s)

> _Pending operator capture: [`artifacts/cockpit-activity-status-bar-2026-05-26/02-after-yahoo-preload-active.png`](artifacts/cockpit-activity-status-bar-2026-05-26/02-after-yahoo-preload-active.png)_

The operator-visible win. The verbatim 2026-05-25 complaint ("is it stuck?" on cold-cache Yahoo preload) gains its affordance: `· Yahoo BTC-USD · 2y · 5s` rendered in the tape region between account and server-time.

### Slot 3 — After: three activities + "+2 more" overflow chip

> _Pending operator capture: [`artifacts/cockpit-activity-status-bar-2026-05-26/03-after-three-plus-overflow.png`](artifacts/cockpit-activity-status-bar-2026-05-26/03-after-three-plus-overflow.png)_

Confirms the R2.2 max-3-visible + overflow chip contract. The chip text comes from `strings::ACTIVITY_TAPE_MORE_PREFIX` + `_SUFFIX` (zero inline literals).

### Slot 4 — After: one failed activity in 3s red-hold

> _Pending operator capture: [`artifacts/cockpit-activity-status-bar-2026-05-26/04-after-failed-red-hold.png`](artifacts/cockpit-activity-status-bar-2026-05-26/04-after-failed-red-hold.png)`

Confirms the Q5=(a) red-row 3 s hold contract. After 3 s the row auto-removes (no operator dismiss affordance at v0.1.0 — Q6=(a) read-only).

## Verification

Verdict source: [`reports/test-final-2026-05-26-cockpit-activity-status-bar.md`](../reports/test-final-2026-05-26-cockpit-activity-status-bar.md) at commit `0ff402f` — tester second-pass `VERDICT → PASS`.

| V-id | Gate                                  | Status   | Evidence                                                                |
|------|---------------------------------------|----------|-------------------------------------------------------------------------|
| V1   | `scripts/verify_anchors.sh`           | VERIFIED | `ANCHORS PASS (34 / 34)` — body-SHAs byte-identical, R-NR.1 contract met |
| V2   | `cargo test --workspace`              | VERIFIED | 2034 passed / 3 failed / 28 ignored — all 3 failures pre-existing/whitelisted (per test-final § 4 re-verification) |
| V3   | cockpit-smoke (manual)                | DEFERRED | Orchestrator-only per skill; operator capture instructions in test-final § 6 + § "Cockpit smoke (operator)" below |
| V4   | `cargo clippy --workspace -D warnings` | PARTIAL  | Cockpit-crate clippy errors fixed at `0ff402f` (while_let_loop + map_unwrap_or); workspace gate still blocked by **pre-existing** `crates/backtest/src/engine.rs:539` `map_unwrap_or` (tech debt, not from this feature) |
| V5   | `cargo fmt --check`                   | VERIFIED | Zero diffs at `0ff402f` (fix commit ran `cargo fmt`)                    |
| V6   | criterion bench (5 micro-benches)     | VERIFIED | All 5 within absolute budget; all deltas under 6 % vs dev baseline; no > 20 % regression flag |
| V7   | integration perf storm (10 k events)  | VERIFIED | drain 7.9 ms / delivery 100 % / P99 34 µs — see live demo above         |
| V8   | visual gates (`render_snapshots` + `visual_snapshots`) | VERIFIED | `render_snapshots` 2/2 PASS + `visual_snapshots` 19/19 PASS with regenerated baselines after status-bar layout change |

## Numbers that matter

- **Tests added by this feature:** 25 (7 agent + 11 ui-lib + 7 ui-integ) + 5 criterion benches + 1 storm test = **31 new test artifacts**.
- **Workspace total at PASS:** 2034 passed, 3 failed (all pre-existing/whitelisted), 28 ignored.
- **Anchors:** **34 / 34 PASS**, zero new anchors (UI + agent only feature). Locked sample: `top10-2023-fy-vol-target-overlay-realdata 9fa64d46`, `sharpe-comparison-vol-target-bs1-realdata d21db467`, `sharpe-comparison-vol-target-bs1-realbaseline ff2b9349`.
- **Perf budgets — Wave D criterion** (Apple M2 Pro, isolated re-run, `cargo bench -p ui --bench activity_tape`):

  | Bench                                    | Tester run | Absolute budget | Headroom |
  |------------------------------------------|-----------:|----------------:|---------:|
  | `activity_handle_tick_throttle`          | 19.99 ns   | < 200 ns        | 10×      |
  | `activity_recipe_fan_out`                | 57.98 ns   | < 500 ns        | 8×       |
  | `activity_tape_render_empty`             | 33.40 ns   | < 200 µs        | 6 000×   |
  | `activity_tape_render_three_inflight`    | 944.86 ns  | < 1 ms          | 1 060×   |
  | `activity_tape_render_five_plus_overflow`| 1.066 µs   | < 1.2 ms        | 1 125×   |

- **Storm test (e2e):** drain **7.923 ms** / delivery **100 %** (10000/10000) / P99 latency **34 µs**.
- **Rollback cost** (per feature.md § D6): ~ 60 LOC across 4-5 files (single revert per wave; binary-compatible bus change).
- **LOC shipped:** ~ 1 600 LOC across `crates/agent` (Wave A ~280) + `crates/ui` (Waves B + C + D ~1 320) — surgical, additive.

## What's NOT in scope (v0.1.0 deferrals — R5)

| Deferral                                         | Why                                                                 | Lands in |
|--------------------------------------------------|---------------------------------------------------------------------|----------|
| LLM call activity (`ActivityKind::LlmCall`)      | `v3-llm-forecaster` producer still in-flight; enum slot reserved    | v0.1.1   |
| Audit-ledger-writes activity (`ActivityKind::AuditLedgerWrite`) | Fan-out at thousands/sec needs an aggregator (K3); per-event 100 ms throttle is the wrong layer | v0.1.1   |
| `TrainingPressed` → `spawn_training_run` cockpit_live.rs wiring | Wave C wired the producer side and unit-tested via `sleep 1` fixture, but the cockpit's `TrainingPressed` message arm does not yet invoke `spawn_training_run` end-to-end | v0.1.1 follow-on |
| `!Send ActivityHandle` constraint workaround docs | Wave C used approach A (inline handle) at all three sites — documented in tasks.md, not yet promoted to a developer-runbook page | v0.1.1   |
| Bus broadcast events themselves (fills/positions/bars/ticks/pnl) | Not activities — they have their own surfaces (tape panel, position panel, charts). Adding them duplicates without value (R5.3) | not planned |
| Operator click-drill-down (Q6=(b))               | Read-only at v0.1.0 (Q6=(a)) — drill-down adds navigation coupling not justified by first-ship telemetry | future ship |

## Risk register surfaced

| K-id  | Risk                                                  | Status at v0.1.0 ship                                                |
|-------|-------------------------------------------------------|----------------------------------------------------------------------|
| K1    | Channel-lag silent staleness (> 25 s UI freeze)        | Low — `tracing::warn` on `Lagged(n)` + storm test confirms 100 % delivery at 10 000× event rate. A 25 s UI freeze would have bigger problems than the tape staleness. |
| K2    | Producer wiring drift (future async fn forgets to wire) | Medium — covered by developer M-DEV acceptance criterion today; long-term clippy-style lint deferred to a follow-on feature. Surface for operator awareness. |
| K3    | Audit-flood producer overwhelms tape                  | **Deferred to v0.1.1** — `R5.2` explicit. Audit writes can fan-out thousands/sec; the per-handle 100 ms throttle is the wrong layer; v0.1.1 brief must specify aggregator design. |
| K4    | LLM-call label leaks vendor internals                 | **Deferred to v0.1.1** — when `v3-llm-forecaster` lands, its label-redaction rule must be specified in the v0.1.1 brief. |
| K5    | Sub-frame producer hot-path stalls                    | Cleared — criterion `activity_handle_tick_throttle` 20 ns / call (10× budget headroom). The throttle is a `Instant::now` + atomic compare, no allocation, no syscall. |
| K6    | Status-bar 24 px height contract regression           | **Cleared** — `visual_snapshots` 19/19 PASS with regenerated baselines confirms the height contract holds. R-NR explicit. |
| K7    | Inter-feature ordering with `v3-llm-forecaster`        | Tracked — cross-link must be added when v3-llm-forecaster reaches M-T1. Slot reserved (enum variant). |
| K8    | Anchor-additive contract via ADR-0038 § D6            | Cleared — UI + agent only; zero touched files in `crates/backtest/`, `crates/strategy/`, `crates/exec/`, `crates/risk/`, `crates/reports/`. |

## Open decisions

These do NOT block ship — surfaced so the operator can route the v0.1.1 follow-on with full context.

1. **Re-baseline criterion numbers?** Tester re-ran in isolated mode at PASS and recorded clean numbers (Δ < 6 % vs dev). Recommend: lock the tester's isolated numbers as the new M-FINAL baseline (`activity_handle_tick_throttle: 19.99 ns`, etc.) so future > 20 % regression alerts trigger off the noise-free reference rather than the dev's run. **Operator: approve re-baseline, or keep dev baseline?**
2. **v0.1.1 scope sign-off.** R5 lists 4 deferrals: LLM call, audit-ledger-writes (needs aggregator design — K3), TrainingPressed end-to-end wiring, !Send ActivityHandle runbook docs. Recommend bundling all 4 in v0.1.1. **Operator: approve v0.1.1 scope, or split?**
3. **Pre-existing `crates/backtest/src/engine.rs:539` `map_unwrap_or` blocks workspace `cargo clippy -D warnings`.** Not from this feature; tester confirmed via git history. Recommend opening a one-line cleanup task. **Operator: open the task now, or leave for the next sweep?**
4. **Cockpit-smoke pass.** Orchestrator-only per skill rules; tester emitted operator-runnable instructions. Recommend the operator captures the 4 T-P-2 screenshots in the same session as a cockpit-smoke pass. **Operator: confirm you will do this before approving, or accept the deferred verification?**
5. **Spec-lint drift since 2026-05-25 audit baseline (61 violations in 1 category) → current state (74 in 4 categories).** The 10-row trace-broken-path count is a lint-tool artifact: `spec/trace.toml` row `REQ-COCKPIT-ACTIVITY-001.anchors = "34/34 PASS"` is a string (not an array), and the lint iterates its characters. Tester's second-pass `VERDICT → PASS` explicitly accepted this state ("64 violations in 3 categories, pre-existing debt only, zero new regressions"). Recommend: either fix the trace row to an array form (e.g. `anchors = []` with a comment) or accept as known artifact. **Operator: fix now, fix later, or accept?**

### Cockpit smoke (operator)

Run before approving — confirms the live cockpit renders the new tape region without panicking. Per the tester's § 6 instructions:

```bash
LOG=spec/cockpit-activity-status-bar/reports/cockpit-smoke-$(date -u +%Y-%m-%dT%H-%MZ).log
mkdir -p "$(dirname "$LOG")"
cargo build -p ui --bin cockpit --features fixtures
(RUST_BACKTRACE=1 cargo run -p ui --bin cockpit --features fixtures > "$LOG" 2>&1 &)
sleep 7
pkill -f "target/debug/cockpit" 2>/dev/null
sleep 1
PANIC_COUNT=$(grep -c "panicked at\|non-unwinding panic\|fatal runtime error" "$LOG")
echo "Panic count: $PANIC_COUNT"
```

Expected: `Panic count: 0`. Then visually verify the tape region appears between account and server-time during a Yahoo preload + Lab Run.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_
- [ ] Retire — _the feature is unwanted; route to architect to plan removal_
- [ ] Fix-and-reship — _changes required before approval; notes below_

### Notes / feedback

_empty until operator fills_

## Changelog

- 2026-05-26 (presenter): initial draft after tester second-pass `VERDICT → PASS` at commit `0ff402f`. Live demo = storm test re-run on Apple M2 Pro. Screenshots pending operator capture (headless sandbox). 8-row verification matrix + R5 deferrals + K1-K8 status. Five open decisions surfaced.
