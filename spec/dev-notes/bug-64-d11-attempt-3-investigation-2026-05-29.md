---
slug: bug-64-d11-attempt-3-investigation
status: draft
owner: analyst
updated: 2026-05-29
---

# Bug #64 D.1.1 — attempt-3 investigation (2026-05-29)

Operator-reported FAIL at commit `3979437`:

> "I still can no stop the running and see no progress. Just endless spinning."

Two regressions surfaced during the visual-verify of attempt-2:

1. **R1 — Progress label dormant.** During the 30–60 s cold-cache Yahoo
   fetch, the loading label does NOT tick. The original D.1.1 symptom is
   back (or never went away on the real cockpit path).
2. **R2 — Stop button broken.** Pressing Stop during the preload window
   does not abort the in-flight fetch. Operator must wait for either
   network timeout or completion.

This dev-note is a READ-ONLY analyst pass. No code edits. The output is a
hypothesis-ranked failure tree, three operator-decide questions, and a
verdict-tree of next-attempt shapes. If the operator promotes, the
architect picks up at M-T1.

## Scope

- Per Bug #64 ledger: attempt-1 (commit `5f9f920`, reverted at `05937e4`),
  attempt-2 (current HEAD, harness-gated). This is attempt-3 scoping.
- Out of scope: the D.2.1 post-completion-linger sub-fix
  (operator-DROPPED 2026-05-28, harness-conflict — stays won't-fix).
- Out of scope: the v2.1-redactor `crates/ui/src/bin/cockpit_live.rs`
  subscriber-init migration (in-flight, separate lane).
- Out of scope: the v5 tester `crates/backtest` work (different scope).

## Code paths read

| File | Lines | Role |
|---|---|---|
| `crates/ui/src/lab/runner.rs` | 295–367 | `preload_yahoo_bars` — UI ↔ data boundary; auto-fetch fallback on `CacheMiss \| RevisionMissing` |
| `crates/ui/src/lab/runner.rs` | 374–447 | `fetch_with_backoff` — 5-retry, 1s→60s, per-attempt 60 s timeout (Bug #63 fix) |
| `crates/ui/src/lab/runner.rs` | 705–828 | `spawn_lab_run` production `#[cfg(feature = "yahoo")]` block — sentinel emit + pinned preload future + `tokio::select!` ticker (attempt-2 shape) |
| `crates/ui/src/lab/runner.rs` | 831–871 | `rt.spawn(...)` for `backtest::engine::run_scenario(scenario_cfg, cancel, progress_tx)` — receives the cancel receiver |
| `crates/ui/src/lab/progress.rs` | 96–112 | `LabProgressRecipe::stream_impl` — drains `mpsc::Receiver<Progress>` into `Message::LabRunProgress` |
| `crates/ui/src/state.rs` | 1504–1530 | `LabRunRequested` / `LabRunCompleted` / `LabRunStopRequested` / `LabRunProgress` / `LabRunProgressDone` Message variants |
| `crates/ui/src/state.rs` | 2136–2190 | Pure-state update arms — Stop arm intentionally a no-op (binary-side wrapper drops `run_cancel`) |
| `crates/ui/src/bin/cockpit_live.rs` | 1199–1215 | Binary-side wrapper — drops `run_cancel` + clears `lab_progress_rx` on Stop & on any LabRunCompleted |
| `crates/ui/src/bin/cockpit_live.rs` | 1478–1530 | LabRunRequested handler — builds cancel pair, builds progress channel, bumps salt, spawns the Task |
| `crates/ui/src/bin/cockpit_live.rs` | 1549–1583 | `subscription()` — builds the `LabProgressRecipe` when `lab_progress_rx.is_some()` |
| `crates/data/src/yahoo.rs` | 364–413 | `YahooBarSource::fetch_and_cache` — single `await` on `provider.get_quote_history_interval` (the 30–60 s cold-cache blocker) |
| `crates/ui/src/lab/training_log.rs` | 1–125 | Wave A `TrainingLogRecipe` — `std::sync::mpsc` → async stream bridge via `spawn_blocking`; reusable pattern |
| `crates/ui/src/lab/cache_state.rs` | (not directly relevant to R1/R2) | Cache probe; out of scope here |
| `spec/bug-log.md` | 100–161 | Full Bug #64 history (attempts 1+2) |

## R1 — Progress label dormant: hypothesis ranking

The attempt-2 select! loop in `runner.rs:778–805` LOOKS correct:

```rust
let mut preload_future = std::pin::pin!(preload_yahoo_bars(...));
let preload_result = loop {
    tokio::select! {
        biased;
        result = &mut preload_future => break result,
        _ = ticker.tick() => {
            progress_tx.try_send(Progress { current_bar: 0, total_bars: 1, elapsed_ms });
        }
    }
};
```

So why doesn't the label tick? Five hypotheses, ranked by likelihood:

### H-R1a — Operator ran a stale binary (HIGH likelihood)

Most common cause of "fix shipped, operator still sees old behaviour":
the cockpit binary was launched from a prior `cargo build` output. Until
the operator does a fresh `cargo run --release -p ui --bin cockpit_live
--features live,yahoo`, the binary in-process is attempt-1 (or
pre-attempt-1) shape.

**Verify**: ask the operator to confirm a fresh build (`cargo clean` not
required; `cargo build` after the attempt-2 commit landed is enough).
This is the cheapest probe; do it FIRST.

### H-R1b — Wrong feature flag set (HIGH likelihood)

The attempt-2 ticker loop lives inside `#[cfg(feature = "yahoo")]`
(`runner.rs:705`). If the operator's `cargo run` line omits
`--features yahoo`, the entire ticker block is compiled out, and the
fallback branch is the test-injection branch (`runner.rs:681–703`)
which fires the sentinel ONCE then awaits — no ticker. Endless static
0/1 label is the exact symptom.

`cockpit_live` itself requires `--features live`. The lab-yahoo-realdata
v0.1.x features land Yahoo as a separately-gated feature. The combined
flag should be `--features "live yahoo"`.

**Verify**: capture the operator's exact `cargo run` line; confirm
`yahoo` is in the comma- or space-separated list.

### H-R1c — `progress_tx.try_send` is silently dropping ticks (MEDIUM likelihood)

`progress_tx.try_send` is non-blocking by design (the comment at
`runner.rs:792–795` calls this out). The channel buffer is bounded
(default 8 in `backtest::progress::progress_pair()`). If the
`LabProgressRecipe::stream` task is NOT actively draining the receiver,
the first 8 ticks fill the buffer and subsequent ones get dropped on
the floor — the channel never closes (preload future hasn't completed)
so no `LabRunProgressDone` either.

But the recipe SHOULD be draining. The salt bump at
`cockpit_live.rs:1496` should force iced to rebuild the subscription
identity each Run. If for any reason iced de-dups the recipe (e.g., the
hash collides because `TypeId` + salt is insufficient), the
subscription stays bound to the previous run's CLOSED receiver, and
the new receiver is never polled. Tick buffer fills, ticks drop, label
never updates.

**Verify**: add `tracing::trace!` at `progress.rs::stream_impl` entry
and after each `rx.recv().await` (would require a probe edit — propose
in attempt-3, not now). Salt-bump pattern is identical to
`TrainingLogRecipe`, which DOES tick reliably in the Train panel, so
the pattern itself is sound. Risk: per-run lifecycle interaction
between `LabRunCompleted` clearing `lab_progress_rx = None` (line 1214)
and the next `LabRunRequested` setting it `Some`.

### H-R1d — `iced::Task::perform` not yielding between ticks (LOW likelihood)

In iced 0.14, `Task::perform` futures run on the iced executor. The
`tokio::select!` requires the tokio runtime context — but the outer
async closure at `runner.rs:655` is NOT inside `rt.enter()`. The
`rt.spawn` at line 831 happens AFTER preload. So `ticker.tick()`
relies on tokio's timer, which requires a tokio runtime.

Look at `runner.rs:743`: `tokio::time::interval(...)` is called
INSIDE the iced::Task::perform closure WITHOUT a tokio runtime guard.
iced may be using its own executor (smol / async-std style), in which
case `tokio::time::interval` would panic at startup or never fire.

**Counter-argument**: the entire iced::Task::perform closure at
`runner.rs:655` calls multiple `.await` points that include
`join.await` (line 872), and the `rt.spawn` at line 831 returns a
`JoinHandle` that is awaited. If iced were NOT running inside a tokio
runtime, the whole thing would have broken long ago. The cockpit's
`AppState::new` (in `cockpit_live.rs`) almost certainly bootstraps a
tokio runtime and `enter()`s it before launching iced. Worth a
sanity-check in attempt-3 architect probe but unlikely to be the
proximate cause.

### H-R1e — Widget reads stale state (LOW likelihood)

`LabState::run_progress: Option<Progress>` is updated in
`state.rs::Message::LabRunProgress` arm (line 2185). The widget
that renders the label reads this field on each `view()`. If the
widget caches by `Progress::elapsed_ms`'s SmolStr formatting and the
operator's screen rendering pipeline somehow short-circuits the
repaint, the label could appear static.

**Probe**: examine `crates/ui/src/widgets/lab/run_progress.rs` or
similar. Lower priority — iced normally repaints on every Message.

### R1 hypothesis disposition

The ranking suggests an architect-friendly path:

1. First eliminate H-R1a (stale binary) and H-R1b (missing feature flag)
   via a 5-minute operator-side probe. Cheap, no code change.
2. If still failing, escalate to H-R1c via probe edit (tracing in
   `progress.rs::stream_impl`).
3. If H-R1c is clean, fall through to H-R1d / H-R1e.

NB: this matters for SCOPE selection. If the answer is H-R1a or H-R1b,
attempt-3 is closed by a recipe update, not a code change.

## R2 — Stop button broken: hypothesis ranking

R2 is structurally clearer than R1. The cancellation contract is:

1. Operator presses Stop → `Message::LabRunStopRequested`.
2. `cockpit_live.rs:1203` drops `lab_state.run_cancel`.
3. Drop fires the `oneshot::Sender<()>` → `RunCancelReceiver::is_cancelled()`
   flips to true.
4. The cancel receiver is **only inspected inside `backtest::engine::run_scenario`**
   (called at `runner.rs:837`). It is NOT checked during preload.

**Therefore: R2 is a structural omission, not a bug.** During the
30–60 s preload window, no path reads `cancel.is_cancelled()`. The
fetch future runs to its conclusion (or its per-attempt 60 s timeout).

### H-R2a — Cancel receiver never polled during preload (CERTAIN)

Confirmed by reading `runner.rs:705–828`. The select! loop has only
two branches: `&mut preload_future` and `ticker.tick()`. Adding a
third branch that polls the cancel receiver (or wrapping the preload
future in a `tokio::select!` against `cancel.notified()`) would close
this gap.

Three implementation shapes (each has different rework cost):

1. **Cancel-token in select!** — add a third select! branch:
   ```rust
   _ = cancel.notified() => return Err(SmolStr::new("cancelled")),
   ```
   Requires `RunCancelReceiver` to expose a `Future` shape (the
   `backtest::cancel` crate would need an audit). ~15–30 LoC change in
   `runner.rs` + possibly ~10 LoC API tweak in `backtest::cancel`.
   Durable; works for both R2 today AND any future preload extension.

2. **Spawn preload + abort on Stop** — move the preload into
   `rt.spawn(...)`, hold the `JoinHandle`, await it in the iced::Task
   loop. On Stop, the binary side calls `handle.abort()`. Adds a
   `lab_state.preload_join: Option<JoinHandle<...>>` field. ~50 LoC
   spread across `state.rs`, `runner.rs`, `cockpit_live.rs`. Heavier
   but more aligned with how `backtest::engine::run_scenario` itself
   gets cancelled.

3. **Ignore Stop during preload** — cheapest: do nothing. Disable
   the Stop button while `lab_state.preload_inflight` is true; the
   operator must wait. Adds a tiny LabState flag + button-gating
   logic. ~10 LoC. Punts the problem.

### H-R2b — Stop click reaches Message but no-op arm runs (LOW)

The pure-state arm at `state.rs:2179` is intentionally a no-op
(the comment cites T-D3.4 / R6.3). The binary side does the
work at `cockpit_live.rs:1203`. The path is sound; verified by reading.
Not the proximate cause unless the binary wrapper is somehow not being
invoked for `LabRunStopRequested` Messages.

## Reusing the Wave A TrainingLogRecipe pattern

The `TrainingLogRecipe` (lab-recipe-test-harness v0.2.0, Wave A) is
ALREADY structurally identical to `LabProgressRecipe`. The
`crates/ui/src/lab/training_log.rs:103–125` pattern bridges a
`std::sync::mpsc` blocking receiver into the async stream via
`tokio::task::spawn_blocking`. The Lab progress channel is already
`tokio::sync::mpsc` (natively async), so no bridge is needed.

For attempt-3, the relevant Wave A insight is the salt-bump + Drop
semantics for clean per-run subscription rebuild. Both the Lab and
Training recipes already use this pattern correctly. So a hypothetical
`YahooFetchProgressRecipe` would NOT be a new pattern — it would be a
fourth instance of the same recipe shape.

**Verdict**: a separate recipe is NOT needed. The existing
`LabProgressRecipe` is the right vehicle for any new tick stream.
Attempt-3 should NOT introduce a new Recipe.

## Operator-decide questions

All Qs bias DURABLE per the 2026-05-28 contract (see analyst.md §
Defaults). The `(Recommended)` tag goes on the choice whose architect
M-T1 lock carries forward longest, not the cheapest typing.

### Q-BUG64-D11-3-Q1 — scope: fix R1 + R2 in one ship vs sequential?

Options:

- **(a) — One-ship fix, R1 + R2 together (Recommended)** — durable.
  R1 and R2 share the same code region (`runner.rs:705–828` ticker loop).
  Attempting them in separate ships would commit two architect M-T1
  passes against the same ~25-line region, and the second ship would
  almost certainly amend the first's lock. Combined scope: ~30–60 LoC
  in `runner.rs` + possibly ~10 LoC `backtest::cancel` audit + 2 e2e
  tests + harness update. Estimated 1–1.5 d analyst-architect-developer
  cycle. Zero v0.X+1 follow-on.

- **(b) — R1 first, R2 deferred** — fallback if budget tightens.
  Ship the H-R1a + H-R1b verify probe in attempt-3a (~0.25 d). If R1
  is closed by recipe fix, defer R2 to a separate attempt-3b. Adds
  one extra cycle but de-risks the diagnosis path. Adds a v0.X+1
  follow-on commitment for R2.

### Q-BUG64-D11-3-Q2 — progress-label mechanism: Subscription vs Message poll vs single-Message?

Options:

- **(a) — Keep the existing tokio::select! + ticker loop, but verify it (Recommended)** — durable.
  The attempt-2 shape (sentinel + pinned future + ticker in select!) is
  the correct pattern. The bug is almost certainly in the OPERATOR'S
  environment (H-R1a / H-R1b) OR a subtle wiring leak (H-R1c). Fix the
  diagnosis, don't replace the mechanism. The mechanism has 3/3
  harness gates GREEN (`spawn_lab_run_yahoo_harness.rs`). Cost: ~5
  min operator-side probe; if probe fails, ~1 d wiring fix.

- **(b) — Replace with a periodic Message poll (`iced::time::every`)** —
  fallback. Drops the select! ticker. Adds an `iced::time::every`
  subscription that fires Message::PreloadTick every 250 ms while
  `lab_state.preload_inflight = true`. View reads `lab_state.preload_started_at`
  and computes elapsed locally. Decouples the label from the
  `progress_tx` channel entirely. ~40 LoC. Less coupled to backtest
  channel BUT introduces a second progress mechanism — DRY violation
  if the existing one works. Maintenance commitment: tester now
  guards two paths.

- **(c) — Single Message after preload completes** — cheapest, lowest
  utility. Drop the ticker entirely. Show a static "Fetching Yahoo bars
  …" label until `LabRunProgress` arrives from the engine. ~5 LoC
  revert of attempt-2. Lowest cost, no fix for R1, regresses to
  pre-D.1.1 UX. Recommend ONLY if operator deprioritises R1 entirely.

### Q-BUG64-D11-3-Q3 — cancellation: cancel-token in select! vs spawn-and-abort vs ignore-Stop?

Options:

- **(a) — Cancel-token wrap (Recommended)** — durable. Add a third
  branch to the existing `tokio::select!`:
  ```rust
  _ = cancel.notified() => return Err(SmolStr::new("cancelled during preload"))
  ```
  Requires `backtest::cancel::RunCancelReceiver` to expose a `notified()`
  future. ~15 LoC `runner.rs` + ~10 LoC `backtest::cancel`. The cancel
  pattern then composes with ALL future preload extensions (e.g., if
  preload grows multi-ticker bulk fetch in v0.1.5). Aligned with the
  existing backtest::engine cancel contract.

- **(b) — Spawn-and-abort** — fallback. Move preload into
  `rt.spawn(...)`. Hold the JoinHandle on AppState. Binary side
  `handle.abort()` on Stop. Heavier (~50 LoC across 3 files) and
  abort() leaves the tokio task in a "dropped at await" state which
  is less clean than a cooperative cancel. Adds an AppState field
  that needs cleanup on every completion path.

- **(c) — Ignore-Stop during preload** — cheapest, ships fast.
  Gate the Stop button (`disabled = lab_state.preload_inflight`).
  Operator must wait through the per-attempt 60 s timeout if they want
  to abort. ~10 LoC + a button-render change. **Permanent UX
  regression** during cold-cache fetches. Cheap-but-typing-faster
  trap — exactly the kind of choice the 2026-05-28 durability
  contract demands we NOT recommend.

## Verdict tree (4 cells)

Possible attempt-3 outcomes once the architect picks a Q-set and the
developer ships:

| Verdict | What it means | Architect M-T1 lock |
|---|---|---|
| **PASS** | Operator visual-verify: label ticks 0/1 bars · X.Xs every 250 ms during cold-cache fetch AND Stop button aborts the in-flight fetch within ≤ 500 ms. | Lock the cancel-token + ticker shape in `runner.rs:705–828`. Harness Surface 1 gains a 4th test: `cancel_during_preload_aborts_within_500ms`. Bug #64 closes definitively. |
| **SOFT-PASS** | R1 fixed (label ticks) but R2 deferred (Stop still no-op during preload). Operator accepts as polish-only. | Lock R1 fix; document R2 as carry-forward to a v0.1.5 lab-yahoo feature. Update Bug #64 ledger to mark R1 closed, R2 reclassified to a UX-affordance item rather than a regression. |
| **FAIL** | R1 attempt missed the actual proximate cause (e.g., the bug was H-R1c silent-tick-drop and the architect picked the wrong wiring fix). Label still dormant. | Architect re-evaluates — likely a probe-pass spawned to add tracing at `progress.rs::stream_impl`. NO lock change; investigation continues. |
| **REGRESSION** | A new regression is introduced. Most-likely candidates: (1) the test-injection branch's sentinel emit is broken by the harness update, killing Surface 1 Test 1; (2) `lab_progress_rx` cleanup on Stop is off-by-one and the next Run shows stale events; (3) cancel-token plumbing breaks `backtest::engine::run_scenario` interruptibility. | Operator override required. Per CLAUDE.md non-negotiable: no ship on REGRESSION without explicit operator override. Revert path is the same shape as attempt-1 (`git revert <attempt-3 commit>`). |

## Risks

### K1 — Auto-fetch-fallback operator decision (2026-05-25) still holds

The 2026-05-25 operator decision instructs that
`preload_yahoo_bars` falls back to `fetch_with_backoff` on
`CacheMiss \| RevisionMissing`. None of the proposed attempt-3 shapes
change this. The cancel-token wrap (Q3=(a)) ADDS a cancellation
shortcut but does NOT alter the fallback semantics. **Confirmed: the
2026-05-25 decision continues to hold under all Q3 options.**

### K2 — Recipe-rewrite scope creep

If H-R1c (silent tick drop) is the actual cause, the fix may extend
into the LabProgressRecipe wiring on AppState. This is INSIDE the
lab-yahoo-realdata scope but BORDERS the lab-recipe-test-harness
v0.2.0 contract. If attempt-3 needs to mutate the LabProgressRecipe
hash/salt semantics, the architect should consult the harness
contract before landing. **Mitigation**: scope attempt-3 to
runner.rs ticker loop + cancel-token plumbing ONLY. If the recipe
itself is the proximate cause, escalate to a v0.1.5 lab-yahoo
feature with its own M-T1 pass rather than a Bug #64 hotfix.

### K3 — Wave A TrainingLogRecipe pattern reuse

Already addressed above (see § "Reusing the Wave A TrainingLogRecipe
pattern"). Verdict: no new Recipe needed; the existing
`LabProgressRecipe` is structurally adequate.

### K4 — v2.1-redactor cockpit_live.rs subscriber-init lane

The v2.1-redactor dev is editing `crates/ui/src/bin/cockpit_live.rs`
for subscriber-init migration (in-flight). Attempt-3's likely surface
includes `cockpit_live.rs:1203–1215` (Stop / completion clear arms)
and `cockpit_live.rs:1478–1530` (LabRunRequested handler). The
subscriber migration is unlikely to touch these arms but the architect
should coordinate to avoid merge surface conflicts. **Mitigation**:
the developer for attempt-3 SHOULD rebase onto the redactor branch
before opening a PR, OR the redactor lands first and attempt-3 starts
from post-redactor HEAD.

### K5 — Stale binary diagnosis (R1 H-R1a)

The most-likely cause is also the least-glamorous: stale binary. If
the architect skips the recipe-update probe and goes straight to
code-change attempt-3, AND H-R1a was the actual cause, attempt-3
ships a no-op code change. The operator will report "no change" and
attempt-3 will be REVERTED. **Mitigation**: bake the operator-side
binary-freshness check INTO the recipe BEFORE any attempt-3 code
change.

## Assumptions

- The attempt-2 commit shape at `runner.rs:705–828` is the actual
  HEAD-of-main shape; verified by direct read of the file.
- The operator's failure report ("endless spinning") describes the
  cold-cache 30–60 s window, not a post-completion artifact.
- The lab-yahoo-realdata v0.1.x feature gates (`yahoo` cargo feature)
  are still required for the production preload path; assumed unchanged.
- `RunCancelReceiver` from `backtest::cancel` exposes (or can be
  trivially extended to expose) a `notified()` future shape for
  use in `tokio::select!`. The architect M-T1 will confirm.

## Recommended next step

Spawn the architect for an M-T1 pass with this brief as input. The
architect should:

1. Lock Q1 = (a) one-ship — R1 + R2 together.
2. Lock Q2 = (a) keep existing ticker, fix wiring — IF the recipe
   surface a binary-freshness check FIRST.
3. Lock Q3 = (a) cancel-token wrap.
4. Coordinate with v2.1-redactor lane on `cockpit_live.rs` rebase.

If the operator prefers the cheaper path (budget-tight), the
fallback set is Q1=(b) sequential, Q2=(c) single-message, Q3=(c)
ignore-Stop — accepting R2 as permanent UX regression during preload.

## Changelog

- 2026-05-29 (analyst): created. Bug #64 D.1.1 attempt-3 investigation
  pass after operator FAIL at commit `3979437`. Two regressions
  surfaced: progress label dormant (R1) + Stop button broken (R2).
  Five-hypothesis tree for R1, structural cause for R2, three
  operator-decide Qs biased durable. NO code edits; architect picks
  up at M-T1 if operator promotes.
