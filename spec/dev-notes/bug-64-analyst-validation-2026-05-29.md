---
slug: bug-64-analyst-validation
status: draft
owner: analyst
updated: 2026-05-29
---

# Bug #64 — analyst validation of the Yahoo+Run code-map (2026-05-29)

READ-ONLY validation of the developer's code-map at
[`bug-64-yahoo-run-code-map-2026-05-29.md`](bug-64-yahoo-run-code-map-2026-05-29.md)
(commit `92864cc`) against the analyst's prior investigation at
[`bug-64-d11-attempt-3-investigation-2026-05-29.md`](bug-64-d11-attempt-3-investigation-2026-05-29.md)
(commit `43cb32d`). Bug log history at
[`spec/bug-log.md:89–161`](../bug-log.md) cross-referenced.

Zero `crates/` edits. Output is a hypothesis re-rank, an attempt-history
cross-check, an operator-failure-report state mapping, a one-ship vs
sequential re-evaluation, and a refreshed operator-decide Q set for the
architect M-T1.

---

## § 1. Cross-check: does the code-map agree with the analyst's hypothesis ranking?

The analyst's prior 5-hypothesis tree for R1:

| Hyp | Description | Prior rank | Code-map evidence | New rank |
|---|---|---|---|---|
| **H-R1a** | Stale binary / wrong build output | HIGH | Code-map cannot validate operator-side; remains an OPERATOR-side probe. § 9 Q5 in code-map does not move this. | HIGH (unchanged) |
| **H-R1b** | Missing `--features yahoo` flag | HIGH | Code-map § 1 confirms the production ticker block lives behind `#[cfg(feature = "yahoo")]` (`runner.rs:705`). Without the flag, the test-injection branch (`runner.rs:681-703`) runs — which has NO ticker, just a sentinel + `source.preload().await`. The label would static-pin at 0/1·0s — matches operator report. | HIGH (unchanged) |
| **H-R1c** | Silent `try_send` drop / Recipe not draining | MEDIUM | Code-map § 7 surfaces a TIMING RACE between sentinel emit (synchronous, before any await in the Task closure) and iced's subscription rebuild after `salt += 1`. The sentinel is placed in the buffer BEFORE the Recipe is registered with the new salt — if the rebuild lags one iced frame, the sentinel is buffered but no Recipe is polling. | MEDIUM (unchanged but better-characterised) |
| **H-R1d** | `tokio::time::interval` outside tokio reactor context | LOW (analyst) | **Code-map § 5 surfaced strong asymmetric evidence.** The `ServerTimeRecipe` and `ToastDismissRecipe` BOTH explicitly call `rt_handle.enter()` inside `stream()` (`cockpit_live.rs:120-125, 167-169`) to avoid "no reactor running" panic on the iced ThreadPool. `spawn_lab_run` does NOT call `rt_handle.enter()` before `tokio::time::interval(250ms)` at `runner.rs:744`. The pattern is structurally identical to what the ServerTimeRecipe explicitly guards against. | **HIGH** (re-ranked) |
| **H-R1e** | Widget reads stale state | LOW | Code-map § 4 confirms widget reads `lab_state.run_progress` which is set in `state.rs:2185` via `Message::LabRunProgress` arm. Path is structurally sound — no evidence of stale-state issue. | LOW (unchanged, near-eliminated) |

### New top-3

1. **H-R1d** — tokio reactor context absence — promoted from LOW to HIGH.
   The code-map evidence is asymmetric: TWO other recipes carry an
   explicit `rt_handle.enter()` guard for the SAME tokio API call shape;
   `spawn_lab_run` does NOT. The asymmetry is the kind of structural
   omission that ships dormant and surfaces only on the cold-cache
   path (warm cache returns before the first 250 ms ticker tick fires —
   see code-map § 3 — so the absence is invisible on warm cache).
2. **H-R1a** — stale binary — retained as HIGH because it is the
   cheapest operator-side probe and is still consistent with the report.
3. **H-R1b** — missing yahoo feature — retained as HIGH for the same
   probe reason.

### Does H-R1d outrank H-R1a (stale binary)?

**No.** H-R1a is a 5-minute operator-side probe (`ls -la
target/release/cockpit_live; cargo build --release ...`). H-R1d is a
2-line code fix (`let _guard = rt_handle.enter();` before the
interval). H-R1a is cheaper to RULE OUT than H-R1d is to fix, so the
diagnosis ordering is unchanged. But the **likely cause** has shifted:
if the operator confirms a fresh binary with `--features yahoo`,
H-R1d becomes the residual #1 candidate and the architect M-T1 should
plan for it.

### Does H-R1d outrank H-R1c (try_send drop)?

**Likely yes.** The H-R1c timing-race is a SECONDARY risk: even if the
Recipe rebuild lags one frame, the ticker fires every 250 ms — so the
2nd, 3rd, …, Nth tick will arrive at a registered Recipe. The label
would still tick (just with a small first-event delay). The operator
reports "endless spinning" — consistent with NO ticks ever, which
H-R1d explains and H-R1c does not. The `TrainingLogRecipe` (Wave A)
proves the salt-bump pattern works for OTHER per-run channels, so the
Recipe machinery itself is sound.

---

## § 2. Cross-reference with attempt-1 and attempt-2 history

Bug log entries at `spec/bug-log.md:89-161` give the historical context.

### Attempt-1 (commit `5f9f920`, reverted at `05937e4`)

**What it tried**: D.1.1 sentinel ticker + D.2.1 post-completion linger.
Per `bug-log.md:121-129`, the implementation had three regressions on
operator visual-verify:

1. No label visible at all.
2. Progress bar stuck at ~30% (iced indeterminate fallback).
3. Stop button broken after Run.

The proximate cause (per `runner.rs:720-725` comment + `bug-log.md:137-139`)
was that attempt-1 called `preload_yahoo_bars(...)` INSIDE the select! loop
body — creating a FRESH future per iteration. Each new future started
from scratch (re-connect to Yahoo), so preload never made progress.

**Important note**: attempt-1's regressions were diagnosed POST-revert
as a SHAPE bug (`pin!` missing), NOT as a tokio-context bug. The
attempt-1 dev did not investigate H-R1d because the immediate
infinite-loop symptom obscured it.

### Attempt-2 (current HEAD; the code-map reads this)

Attempt-2 added per code-map § 8:
1. `std::pin::pin!(preload_yahoo_bars(...))` — fixes attempt-1's
   fresh-future-per-iteration bug.
2. `biased` keyword — preload wins over ticker at completion.
3. `ticker.tick().await` BEFORE the loop — consumes the t=0 immediate
   tick.
4. `drop(ticker)` — explicit cleanup.
5. Activity handle wiring.
6. Harness gating (test-injection branch).

**What attempt-2 did NOT address** (per code-map § 8):
- No cancel check during preload (R2 — confirmed structural omission).
- No `rt_handle.enter()` before `tokio::time::interval` (H-R1d).

### Did attempt-2 structurally address the original Bug #64 root cause?

The ORIGINAL Bug #64 root cause (2026-05-25, per `bug-log.md:95-103`) was
that scenarios used a sparse poll boundary (every 32/128 bars) and
short Yahoo daily runs (30 bars) had only ONE emit at bar 0. The fix
forced an emit at the final bar. **That fix is intact** — neither
attempt-1 nor attempt-2 broke it.

The D.1.1 sub-fix targets a DIFFERENT symptom: the 30-60 s preload
window has NO emits at all (the sentinel fires once, then dead silence
until the engine starts). Attempt-2's ticker is structurally correct as
a CONCEPT but, per H-R1d, may not RUN AT ALL on the production iced
ThreadPool executor without an `rt_handle.enter()` guard.

**Code-map verdict cross-check**: § 5 of the code map already calls
this out — "spawn_lab_run does NOT call rt_handle.enter() before
creating the interval. This is the H-R1d concern raised by the analyst."
The code-map dev's counter-argument (`rt.spawn(...)` at line 831 awaits
a JoinHandle, so SOMETHING in the executor must support tokio) is weak:
`rt.spawn(...)` is the AGENT runtime, not the iced executor, and
awaiting a JoinHandle from the agent runtime works regardless of
whether the iced executor has a tokio reactor context — the JoinHandle
is itself a future that only requires the WAKER to be invoked. The
operator's "endless spinning" report is the smoking gun: not a panic,
not a hang at startup — just NO TICKS during preload. That is exactly
what an inert `tokio::time::interval` looks like when constructed
without a reactor (depending on tokio version, it may silently never
fire instead of panicking).

---

## § 3. Operator failure-report parsing

Operator: "I still can no stop the running and see no progress. Just
endless spinning."

Mapping to code-map § 4 states:

| Operator phrase | Code-map state | Mechanism |
|---|---|---|
| "endless spinning" | `Preloading` (sub-state of run_progress = Some{0/1·0ms}) | The label is stuck on the sentinel value because no subsequent `LabRunProgress` ticks arrive. |
| "no progress" | Same as above — the label STAYS at `0 / 1 bars · 0.0s` for the duration of the cold-cache fetch | Either (a) ticker arm of select! never fires (H-R1d), or (b) ticks fire but try_send drops them silently (H-R1c). |
| "no stop" | The select! loop (`runner.rs:778-805`) has only TWO branches — `&mut preload_future` and `ticker.tick()`. There is NO third branch for cancel. | Confirmed by code-map § 6: ZERO calls to `cancel.is_cancelled()` reachable between `runner.rs:705` and `runner.rs:830`. Stop drops the handle, `is_cancelled()` flips true, but nothing reads it during preload. |

### If H-R1d is the cause: predicted UI state

- Run button click → `Message::LabRunRequested` fires.
- `LabState::run_progress` = `Some(Progress{0, 1, 0ms})` (sentinel) — set
  via the sentinel `try_send` at `runner.rs:735-739` if the Recipe
  IS draining; if H-R1c also bites, then `None` instead.
- Sentinel arrival depends on whether `LabProgressRecipe` is registered
  in time — race per H-R1c.
- **`tokio::time::interval` never ticks** (H-R1d) → the select! loop
  blocks on `&mut preload_future` ONLY. No ticker arm ever fires. No
  subsequent `LabRunProgress` messages.
- Run button stays disabled (because `lab_run_inflight = true`).
- Stop button click → drops `run_cancel`, sets `is_cancelled()` true, but
  no code reads it. The preload future runs to completion (or 60 s
  per-attempt timeout × 5 retries = up to 5 min).
- After preload finally returns, the engine bar loop runs and DOES poll
  cancel — at which point the previously-issued Stop finally takes
  effect (5 min too late).

**This matches the operator's report exactly.** "Endless spinning" = no
ticks. "Can't stop" = Stop press is silently swallowed during preload.

---

## § 4. Sequential vs one-ship — re-evaluation

Original Q-BUG64-D11-3-Q1 default was (a) one-ship R1+R2, biased on
durability per the 2026-05-28 contract.

Given the code-map evidence and the H-R1d re-rank:

### R1 fix scope (if H-R1d is the actual cause)

- **Smallest fix**: 2 lines in `runner.rs` — wrap the
  `tokio::time::interval` and select! loop body in
  `let _guard = rt_handle.enter();`. This mirrors what `ServerTimeRecipe`
  does at `cockpit_live.rs:120-125`. The `rt_handle` is already plumbed
  into `spawn_lab_run` (the `let rt = handle.clone();` at `runner.rs:646`).
  Estimated 10-15 LoC including the comment block.
- **Alternative**: move the ticker into a separate `iced::Subscription`
  via a new `LabPreloadTickRecipe` that follows the ServerTimeRecipe
  pattern. ~80 LoC across `progress.rs` + `state.rs` + `cockpit_live.rs`.
  More aligned with iced's intended pattern but introduces another
  Recipe.

### R2 fix scope

- **Smallest fix**: extend `RunCancelReceiver` with a `notified()`
  future, then add `_ = cancel.notified() => break Err("cancelled");`
  to the select! loop. ~20-30 LoC across `backtest::cancel` + `runner.rs`.
  Note: std `mpsc::Receiver` is NOT async-poll-friendly; this requires
  either (a) swap to `tokio::sync::oneshot::Receiver` (also Drop-fires),
  (b) swap to `tokio_util::sync::CancellationToken`, or (c) add an
  `Arc<Notify>` alongside the existing channel.
- **Heavier fix**: spawn-and-abort pattern. ~50 LoC across 3 files.

### Re-evaluation verdict

**Q1=(a) one-ship is STILL the durable answer**, but the cost ratio
shifts in favour of it:

- If H-R1d is the cause, R1 fix is the SMALL one (10-15 LoC enter-guard).
- R2 fix is MEDIUM (~30-50 LoC with the right primitive swap).
- Both touch the same select! loop region (`runner.rs:705-828`).
- Two separate architect M-T1 passes against the same 25-line region is
  wasteful; one ship that lands the enter-guard + the cancel-notified
  arm is the durable choice.
- Estimated one-ship cycle: 1 day analyst-architect-developer-tester.

Fallback if budget tightens: Q1=(b) — R1 first (just the enter-guard,
zero cancel changes), defer R2 to a separate attempt-3b. But the
expected v0.X+1 follow-on commitment for R2 makes this strictly more
expensive in total.

---

## § 5. UX trade-offs for Q3 cancellation options

| Option | UX behaviour | Operator experience during cold-cache 30-60 s preload | Verdict |
|---|---|---|---|
| **(a) cancel-token in select!** | Stop click → preload future yields immediately at the select! point; closure returns Err("cancelled"); LabRunCompleted(Err) flows through normal path. | Stop press is instant (≤ 250 ms — bounded by the ticker period if the cancel future and the preload future race on the same poll). Run button re-enables. UX feels like a normal app. | DURABLE — recommend |
| **(b) spawn-and-abort** | Stop click → `handle.abort()` on the preload JoinHandle. The aborted task drops at an internal `.await`. yahoo_activity_handle's Drop fires the "End {Failed}" event. | Stop press is instant but cleanup is NON-cooperative — the in-flight HTTP request to Yahoo is dropped mid-stream, partial cache writes possible (`fetch_and_cache` writes to disk inside `provider.get_quote_history_interval`). The next Run on the same ticker may see a corrupted parquet file. | Functional but RISKY — fallback |
| **(c) ignore-Stop during preload** | Stop button is grayed out for the 30-60 s preload window. Operator must wait. | Operator clicks Stop → nothing happens → reads tooltip "wait for preload" → frustrated. Permanent UX regression for the cold-cache path. | NOT recommend — cheap-but-typing-faster trap |

### Recommendation (confirmed unchanged)

**Q3=(a) cancel-token wrap** is the only durable choice for an
interactive cockpit. The operator's failure report explicitly cites
the Stop button as broken — fixing it with option (c) would be a
literal "the Stop button is intentionally broken during preload"
spec note, which is exactly the kind of UX scar a research cockpit
should avoid.

**Caveat on (a) implementation**: the cleanest primitive swap is
`tokio_util::sync::CancellationToken`, which provides a `cancelled()`
future natively and is Send + Sync. The std `mpsc::sync_channel(0)`
disconnect pattern was chosen 2026-05-22 for the simplicity reason
that ALL cancel checks were synchronous `is_cancelled()` calls at
bar-loop boundaries. Now that we need an async `.await` at the
preload select!, the primitive is wrong-shape — argue for a swap to
`CancellationToken` in the M-T1 brief.

---

## § 6. v0.1.5 lab-yahoo escalation re-evaluation

Prior K2 said: if H-R1c (silent tick drop) is the actual cause, scope
extends into LabProgressRecipe wiring — escalate to v0.1.5 lab-yahoo
feature rather than a Bug #64 hotfix.

### If H-R1d is the cause instead

**Escalation is NOT needed.** H-R1d is a strict 10-15 LoC change in
`spawn_lab_run` (add `let _guard = rt_handle.enter();` at the right
place). The `rt_handle` is already plumbed in. No new Recipe, no
LabProgressRecipe mutation, no AppState field changes.

**The fix is Bug #64 hotfix-sized.** No v0.1.5 escalation required.

### Residual escalation triggers

The hotfix scope expands ONLY IF:
1. The architect M-T1 probe shows that even with `rt_handle.enter()`,
   the ticker still doesn't fire — meaning H-R1d was wrong AND H-R1c
   (or some other recipe-wiring issue) is the residual cause. Then
   escalate.
2. The R2 fix turns out to require a NEW Recipe (because
   `RunCancelReceiver` can't be cleanly extended). The
   `CancellationToken` swap is a strict API-additive change, no Recipe
   needed — so this trigger is unlikely.

---

## § 7. Recipe/Subscription pattern reuse

Code-map § 8 noted: "Wave A TrainingLogRecipe pattern is structurally
identical to LabProgressRecipe. No new Recipe needed — reuse existing."

### Validation

The analyst's prior dev-note already confirmed this conclusion: no new
Recipe needed (see `bug-64-d11-attempt-3-investigation-2026-05-29.md:233-250`).
The Wave A pattern reuse holds.

### But: is LabProgressRecipe the right SHAPE if H-R1d is the cause?

The LabProgressRecipe drains a `tokio::sync::mpsc::Receiver<Progress>`
via `rx.recv().await` inside its `stream_impl()` — this runs INSIDE
the iced subscription executor. The receiver is at the iced-executor
boundary. Whether the iced executor has tokio context inside
`Recipe::stream()` is a SEPARATE concern from whether it has tokio
context inside `iced::Task::perform` closures.

**Evidence the Recipe-side IS tokio-context-aware**: `ServerTimeRecipe`
and `ToastDismissRecipe` BOTH carry `rt_handle` and call `enter()` in
`stream()`. This means the iced ThreadPool DOES NOT auto-provide tokio
context — but each Recipe can enter the agent runtime explicitly. The
**`LabProgressRecipe` ALSO needs the same `rt_handle.enter()`** if it
does any `tokio::time::*` calls inside its stream. Currently it only
does `rx.recv().await` on a tokio mpsc receiver — which DOES require
the tokio context to register the waker correctly.

This is a SECOND H-R1d-shaped concern, hiding in the LabProgressRecipe.
Let me check the code: `progress.rs:96-112` (per code-map) shows
`Box::pin(async_stream::stream! { ... rx.recv().await ... })` — no
`rt_handle.enter()` guard. **If H-R1d is the production-side cause for
the ticker, the same omission may be silently breaking the Recipe's
ability to drain `rx`.** This would be a third concurrent symptom.

**Counter-evidence**: the Wave A TrainingLogRecipe DOES tick reliably
in the Train panel (per the prior analyst note `H-R1c discussion`).
TrainingLogRecipe uses `std::sync::mpsc::Receiver` + `spawn_blocking`
(per `lab/training_log.rs:1-125` — code-map § 1) — a DIFFERENT pattern.
So Wave A's success does NOT prove the `tokio::sync::mpsc`-based
LabProgressRecipe pattern works.

### Architect probe recommendation

The architect M-T1 should ALSO confirm whether the LabProgressRecipe's
`rx.recv().await` on a tokio mpsc actually drains messages on the
iced executor — independent of the runner.rs fix. If both are broken,
the fix is two `rt_handle.enter()` guards in two files. If only the
runner.rs one is broken, the fix is one guard.

---

## § 8. Operator-decide Qs (refreshed)

### Q-BUG64-D11-3-Q1 (refreshed) — scope: one-ship R1+R2 vs sequential?

Unchanged from prior recommendation: **(a) one-ship (Recommended)**.

The code-map evidence (H-R1d as a small fix) makes (a) STRICTLY
cheaper than the sequential path because the R1 fix shrinks. The
fallback (b) sequential is now strictly more wasteful and SHOULD NOT
be picked unless a budget-tightening reason exists.

### Q-BUG64-D11-3-Q2 (refreshed) — progress-label mechanism

The original Q2 considered whether to keep the existing
tokio::select! + ticker shape vs replace with `iced::time::every` vs
single-Message. Per code-map § 5 evidence:

**(a) Keep existing shape + add `rt_handle.enter()` guard (Recommended)**.
The select!+ticker shape is correct architecturally. The bug is the
missing tokio-context guard, which mirrors what `ServerTimeRecipe`
explicitly does. Cost: 2-line fix (`let _guard = rt_handle.enter();`)
plus a comment block. Zero subscription-pattern change. Aligned with
the cockpit's existing recipe pattern.

**(b) Replace with `iced::time::every` Subscription** — fallback if
(a) doesn't work in M-T1 probe. ~40 LoC, introduces a SECOND progress
mechanism (DRY violation). The fact that the cockpit's other tickers
(ServerTime, ToastDismiss) use Recipe + rt_handle.enter() rather than
iced::time::every is itself evidence that the analyst chose
Recipe+enter() before for similar reasons.

**(c) Single-Message after preload** — drop the ticker entirely.
Cheapest, regresses UX. Recommend NOT — operator already pushed back
on dormant label.

### Q-BUG64-D11-3-Q3 (refreshed) — cancellation primitive

Unchanged conceptually: **(a) cancel-token wrap (Recommended)**, with
implementation refinement:

The PRIMITIVE choice for (a) is the new sub-question:
- **(a.i) Swap `RunCancelReceiver` to `tokio_util::sync::CancellationToken`** —
  cleanest. Drop-in replacement at the cancel.rs API boundary. The
  `CancellationToken::cancelled()` future is what the select! needs.
  Existing `is_cancelled()` call sites in scenarios can stay (token
  has the same method name). ~30 LoC.
- **(a.ii) Add `Arc<Notify>` alongside the existing std mpsc** — keep
  the std primitive for backwards compat, add `notified()` via Notify.
  Two parallel signals to manage; more error-prone. ~40 LoC.
- **(a.iii) Swap to `tokio::sync::oneshot::Receiver`** — same Drop
  semantics. Receiver is .awaitable. ~35 LoC. Slightly awkward because
  `is_cancelled()` becomes "have we received OR is sender dropped?".

**Recommend (a.i)** — `CancellationToken` is the purpose-built primitive.

### NEW Q-BUG64-D11-3-Q4 — does LabProgressRecipe also need `rt_handle.enter()`?

Surfaced by code-map § 7 evidence. The LabProgressRecipe's
`stream_impl()` calls `rx.recv().await` on a tokio mpsc receiver —
which requires tokio context for waker registration.

- **(a) Add `rt_handle.enter()` to LabProgressRecipe::stream_impl
  defensively** (Recommended). Mirrors ServerTimeRecipe / ToastDismissRecipe.
  Costs ~10 LoC including comment. Closes the H-R1d-shaped concern on
  the Recipe side at the same time. No risk of double-fixing.
- **(b) Leave LabProgressRecipe alone** — fallback. If the spawn_lab_run
  fix alone closes R1, leaving the Recipe path is fine. But re-opens
  the same hidden risk for any future Recipe that drains a tokio
  channel.

Architect M-T1 should ALSO answer: does the LabProgressRecipe currently
work (per Surface 1 / Surface 2 harness gates)? If yes, the answer is
"the harness tests must be entering tokio context somehow" — and the
architect should explicitly identify that mechanism so production
parity is locked.

### NEW Q-BUG64-D11-3-Q5 — keep Recipe/Subscription pattern or move to `iced::time::every`?

The architecture decision: iced 0.14 ships `iced::time::every` as an
out-of-the-box subscription that fires `Message::Tick(Instant)` every
N ms. This is a SIMPLER pattern than Recipe + `rt_handle.enter()` +
`tokio::time::interval`.

- **(a) Keep Recipe + enter-guard pattern (Recommended)**. Aligned
  with the cockpit's existing 5 Recipes. Each Recipe owns its own
  data shape. No new subscription pattern proliferation.
- **(b) Migrate to `iced::time::every`** — fallback. SIMPLER but
  requires per-tick state in `LabState` (e.g. `preload_started_at`)
  and re-renders compute elapsed locally. Decouples the label from
  the backtest channel.

Recommend (a). The Recipe pattern is the durable choice per the prior
analyst dev-note; the operator already invested in it. Migration to
`iced::time::every` is a v0.X+1 architecture decision, not a Bug #64
hotfix.

---

## § 9. Risks

### K1 — Auto-fetch-fallback (2026-05-25) still holds

Confirmed: none of the proposed Q-set options change the
`preload_yahoo_bars` fallback semantics on `CacheMiss | RevisionMissing`.

### K2 — Recipe-rewrite scope creep — RE-EVALUATED

Prior risk: if H-R1c is the cause, scope balloons into
LabProgressRecipe. **Per § 6 above, if H-R1d is the cause (now the
top R1 candidate), the scope STAYS small.** K2 risk is downgraded.

### K3 — Wave A TrainingLogRecipe pattern reuse — UPDATED

Wave A uses a DIFFERENT receiver primitive (std mpsc + spawn_blocking)
than LabProgressRecipe (tokio mpsc). So Wave A's success does NOT
guarantee the tokio-mpsc path works on the iced ThreadPool. New Q4
above surfaces this explicitly. K3 risk is upgraded slightly.

### K4 — v2.1-redactor `cockpit_live.rs` subscriber-init lane

Unchanged. Architect should coordinate rebase as before.

### K5 — Stale-binary mis-diagnosis (H-R1a) — UNCHANGED

Still the cheapest probe; still mandatory before any code-change ship.
Bake into the operator recipe as step 1.

### K6 (NEW) — `CancellationToken` primitive swap blast radius

The `RunCancelReceiver` is currently consumed in 4 scenarios
(`momentum.rs`, `pairs.rs`, `sma_composed_run.rs`, `tcn_overlay.rs`)
and in `engine.rs`. Swapping to `CancellationToken` requires touching
6 files. The change is mechanical (API surface preserved) but it
broadens the blast radius for the M-T1 review.

**Mitigation**: scope the primitive swap as its own task in the
M-T1 brief (M-T1 task A), separate from the ticker enter-guard fix
(M-T1 task B). If task A turns out heavier than expected, task B
can ship alone as attempt-3a and task A re-emerges as attempt-3b.
This is the natural fallback if Q1=(a) one-ship runs into budget.

---

## § 10. Assumptions

Carried forward from the prior dev-note plus new:

- The cockpit_live binary's `rt_handle` is the agent runtime built at
  `main()` and is multi-threaded with reactor enabled. Confirmed by
  reading `cockpit_live.rs:262-265` (bootstrap_rt) and the side-thread
  agent_runtime construction.
- `iced::Task::perform` closures run on iced's `futures::ThreadPool`
  executor with NO tokio reactor context by default. Confirmed by
  reading the `ServerTimeRecipe` comment block at
  `cockpit_live.rs:104-126`.
- The harness test-injection branch at `runner.rs:681-703` runs under
  `#[tokio::test]` in the harness file, which DOES provide a tokio
  reactor. The harness therefore CANNOT exercise the H-R1d concern;
  the production path is the only place where the missing
  `rt_handle.enter()` would manifest.
- `tokio_util::sync::CancellationToken` is a viable in-workspace dep
  (architect M-T1 to confirm `tokio-util` is already in Cargo.toml).
- The operator's cold-cache fetch hits the `#[cfg(feature = "yahoo")]`
  production block, NOT the harness branch. The operator's `cargo run`
  line is presumed to include `--features "live yahoo"` (else H-R1b
  would have surfaced earlier).

---

## § 11. Recommended next step

Spawn architect M-T1 with this dev-note + the code-map as inputs. The
M-T1 brief should:

1. Lock Q1 = (a) one-ship — R1 + R2 together.
2. Lock Q2 = (a) keep existing select!+ticker, add
   `let _guard = rt_handle.enter();` guard.
3. Lock Q3 = (a) cancel-token wrap, sub-option (a.i)
   `tokio_util::sync::CancellationToken` swap.
4. Decide Q4 — add the enter-guard to LabProgressRecipe::stream_impl
   defensively, OR leave-and-monitor.
5. Decide Q5 — keep Recipe pattern (recommend) vs migrate to
   iced::time::every (fallback only if Q2=(a) doesn't probe out).
6. Coordinate with v2.1-redactor lane on `cockpit_live.rs` rebase.

**Stale-binary check FIRST** — before any code change ships, the
operator-side recipe must confirm:
- `cargo clean && cargo build --release -p ui --bin cockpit_live --features live,yahoo`
- `ls -la target/release/cockpit_live` shows a fresh mtime.
- A trivial probe (`tracing::warn!("BUG-64-D11-3 marker active")`
  somewhere obvious) confirms the new binary is what's running.

If stale-binary IS the cause (H-R1a), the analyst-architect-developer
cycle is closed by a recipe update — no code change ships.

---

## Changelog

- 2026-05-29 (analyst): created. Validation pass of the developer's
  code-map dev-note (`92864cc`) cross-referenced with the prior
  analyst investigation (`43cb32d`) and bug-log.md attempt-1/2 history.
  Re-ranked H-R1d from LOW to HIGH as the new top R1 candidate based
  on the code-map's asymmetric-evidence finding (ServerTimeRecipe and
  ToastDismissRecipe carry explicit `rt_handle.enter()` guards;
  spawn_lab_run does NOT). Surfaced Q4 (LabProgressRecipe may need
  the same guard) and Q5 (Recipe vs iced::time::every architecture
  decision). Confirmed Q1=(a) one-ship and Q3=(a) cancel-token wrap
  with primitive-swap sub-option (a.i) CancellationToken. NO code edits.
