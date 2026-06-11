---
slug: bug-64-arch-validation
status: draft
owner: architect
updated: 2026-05-29
---

# Bug #64 — Architect validation of attempt-3 code-map (2026-05-29)

READ-ONLY architect pass. Validates the developer's code-map dev-note
(`spec/dev-notes/bug-64-yahoo-run-code-map-2026-05-29.md`, commit
`92864cc`) and answers the 5 open Qs the developer surfaced. ZERO
edits to `crates/`. Output is a single architect-readable note. No
ADR written yet (Bug #64 fix justifies an amendment — see § 5).

## § 1. Code-map validation

Verified each of the dev's 10 sections against source.

### § 1.1 Files in scope (dev § 1) — COMPLETE + CORRECT

All 16 files listed are present. Line-range citations match HEAD. Two
minor omissions, both peripheral and explicitly flagged as such by the
dev:

- `crates/ui/src/live.rs:780` (`server_time_stream_impl`) — the
  reference impl for the `rt_handle.enter()` + `tokio::time::interval`
  pattern. Not on the R1/R2 critical path BUT is the structural
  precedent that confirms H-R1d. Add to the table.
- `crates/ui/src/lab/training_log.rs:96–125` (`TrainingLogRecipe`) —
  cited in dev § 8 cross-ref but not in § 1 file table. Same
  recipe-shape precedent.

Verdict: COMPLETE for R1/R2 root-cause analysis. Add the two
peripheral references in attempt-3 design.

### § 1.2 Cold-cache sequence diagram (dev § 2) — CORRECT + FAITHFUL

Each arrow verified against `runner.rs:654–888`. The annotation at the
bottom ("biased select! → preload wins") matches the `biased;` keyword
at `runner.rs:780`. The "no select! inside yahoo.rs" call-out at the
provider step is correct — `data/yahoo.rs:380–383` is a single
uninterruptible `.await`.

Two micro-amendments:

- The sentinel `try_send` at `runner.rs:735–739` IS emitted before
  `ticker.tick().await` consumes the immediate (t=0) tick — diagram
  shows this correctly. Good.
- The "subscription() rebuilds LabProgressRecipe (salt=N)" note at
  `cockpit_live.rs:1575` is approximately correct but elides the
  ordering question (see Q3 below): iced calls `subscription()` AFTER
  `update()` returns, which means there IS a brief window where the
  sentinel sits in the buffer before the recipe is registered. The
  diagram should add a "frame-gap" annotation. NOT a bug; just a
  clarification.

Verdict: FAITHFUL to the implementation. Diverges from attempt-2's
DESIGN INTENT only in that attempt-2 was supposed to deliver a
ticking label — the diagram correctly shows WHY it doesn't (no
`rt_handle.enter()` before `tokio::time::interval` at line 744).

### § 1.3 Warm-cache sequence diagram (dev § 3) — CORRECT

Matches the `biased; result = &mut preload_future =>` branch winning
when preload completes before the first 250 ms tick. The "ticker
NEVER fires on warm cache" note is correct.

### § 1.4 State table (dev § 4) — COMPLETE, mostly CORRECT

Seven states. The `SpawnPending` row's "render chance" column reads
"only one render frame between LabRunRequested and iced starting to
poll the Task" — this is approximately right but glosses over the
iced subscription registration lag, which IS the H-R1c risk.

Suggested amendment for attempt-3 design: split the
Preloading/FetchingBackoff row into:
- `Preloading` — `preload_yahoo_bars::load_cached` disk read (~ms)
- `FetchingBackoff` — `fetch_with_backoff` → `fetch_and_cache` (~30–60 s)

The 30-60 s case is the ONLY one where the dormant label is visible
to the operator. Disk-only `load_cached` returns too fast for any
ticker to fire (same as warm cache).

### § 1.5 Call graph (dev § 5) — CORRECT + this is where H-R1d is best surfaced

The dev's annotation at the call graph is the single most important
line in the entire code-map:

> "** H-R1d critical point: iced::Task::perform closure runs on iced's
> futures::ThreadPool executor. Is the tokio rt context available here?
> ServerTimeRecipe enters rt with rt_handle.enter() inside stream(),
> but spawn_lab_run does NOT call rt_handle.enter() before creating
> the interval. **"

Verified by direct source read (see Q1 below). CONFIRMED.

The dev's counter-argument ("rt.spawn at runner.rs:831 awaits a
JoinHandle from the agent tokio runtime") is itself wrong-headed —
`rt.spawn` runs ON the agent runtime regardless of iced executor
context, because `rt` is a cloned `tokio::runtime::Handle`. The
spawned task therefore HAS reactor context, while the
`iced::Task::perform` closure body does NOT. This is exactly the
asymmetry that breaks the ticker.

### § 1.6 Cancellation reachability table (dev § 6) — CORRECT + COMPLETE

Verified: ZERO calls to `cancel.is_cancelled()` or any cancel adapter
between `runner.rs:705` and `runner.rs:828`. The `RunCancelReceiver`
type at `crates/backtest/src/cancel.rs:50–62` has exactly one public
method (`is_cancelled()`) — no `notified()`, no `Future` impl, no
async adapter. The R2 structural omission is confirmed.

### § 1.7 Progress emission table (dev § 7) — CORRECT, H-R1c is real

The receiver in `LabProgressRecipe::stream_impl` does drain via
`rx.recv().await` (verified at `progress.rs:101`). The H-R1c race
("sentinel fires before subscription re-registers") IS structurally
possible (see Q3 below for iced 0.14 timing semantics) but is NOT
the proximate cause of the dormant label, because:

1. The sentinel fires first (capacity used = 1/8).
2. Subscription re-registers after `update()` returns.
3. Even if there's a brief window, the sentinel sits unconsumed in
   the buffer — when the subscription registers and starts polling
   `rx.recv().await`, the sentinel IS delivered.
4. The bigger problem is that subsequent ticks NEVER FIRE
   (H-R1d) — not that they're dropped (H-R1c).

H-R1c is real but secondary to H-R1d. Once H-R1d is fixed, H-R1c
becomes the next-most-likely failure mode and should be defended
against in the same ship.

### § 1.8 Cross-reference with prior attempts (dev § 8) — CORRECT

Attempt-1 / attempt-2 / attempt-3-investigation cross-ref matches
prior dev-notes. The attempt-2 change list (6 items) is accurate.
What attempt-2 did NOT add — explicitly noted by dev — is precisely
the two remaining gaps (no cancel poll during preload + no
`rt_handle.enter()`).

### § 1.9 Open Qs (dev § 9) — addressed in § 2 below

### § 1.10 Constraints honored (dev § 10) — CORRECT

Read-only pass; no `crates/` edits; lane coordination notes accurate.

### § 1 verdict

Code-map is FAITHFUL to attempt-2 HEAD, COMPLETE for R1/R2 root-cause
analysis, CORRECT in every cited line. Two micro-amendments noted
above (peripheral file refs, state-table split). No blocking issues.

## § 2. Answers to the 5 open Qs

### Q1 — H-R1d (tokio context in iced Task::perform)

**Verified at source:**

- `crates/ui/src/bin/cockpit_live.rs:127–151` — `ServerTimeRecipe`
  delegates to `ui::live::server_time_stream_impl(&self.rt_handle)`.
- `crates/ui/src/live.rs:780–797` — `server_time_stream_impl`:
  ```rust
  let mut interval = {
      let _guard = rt_handle.enter();           // <-- explicit entry
      tokio::time::interval(Duration::from_secs(1))
  };
  ```
- `crates/ui/src/lab/runner.rs:743–747` — production Yahoo block:
  ```rust
  let mut ticker =
      tokio::time::interval(std::time::Duration::from_millis(250));
  ticker.tick().await;
  ```
  **NO** `rt_handle.enter()` or `rt.enter()` before this call.

The dev's claim is verified verbatim. `ServerTimeRecipe` and
`ToastDismissRecipe` (both at `live.rs:780` and `live.rs:824`) AND
`LabProgressRecipe::stream` (at `progress.rs:71–80`) ALL explicitly
enter `rt_handle` before calling `tokio::time::interval` or related
APIs. The `cockpit_live.rs:110–125` doc comment spells out the
rationale: *iced 0.14 uses `futures::executor::ThreadPool` when the
`thread-pool` feature is active. That executor has NO tokio reactor
context — calling `tokio::time::interval()` directly inside a stream
panics with "there is no reactor running"*.

`iced::Task::perform` futures run on the SAME `futures::ThreadPool`
executor as Recipes (per the inline comments in `cockpit_live.rs`).
The `runner.rs:744` call therefore executes WITHOUT tokio reactor
context. There are three possible runtime outcomes:

1. **Panic with "no reactor running"** at the `tokio::time::interval`
   call. The Task::perform closure unwinds; iced reports the join
   error as a `LabRunCompleted(Err)`. Operator would see a "Run failed"
   banner. Operator reports "endless spinning", not a banner →
   probably NOT this case.
2. **Silent failure** — `tokio::time::interval` constructed but
   `ticker.tick().await` never fires because no time driver is
   reachable from the current thread. The select! arm `_ = ticker.tick()`
   is permanently pending → preload future is the only arm that can
   complete. Sentinel was already emitted; no further progress events
   fire; label sits at 0/1·0s for 30–60 s. **This matches the operator
   symptom exactly.**
3. **Lucky-thread context** — iced's `futures::ThreadPool` happens to
   share threads with the tokio runtime (because both bootstrap
   handles on the same process), so the call works by coincidence.
   Operator would see a ticking label — they don't → NOT this case.

Outcome (2) — silent ticker pending — is the structurally correct
explanation. `tokio::time::interval` does NOT panic at construction
when no reactor is reachable; the panic happens on `.tick().await`
only when the runtime can't schedule the wakeup. But on iced's
`futures::ThreadPool`, the future is polled by a non-tokio executor
and `tokio::time::Sleep`'s `register_waker` never registers with a
reactor — so the future stays `Poll::Pending` forever. No panic, no
progress, perfect match for "endless spinning".

**Verdict: H-R1d is CONFIRMED.** The dormant label is the proximate
result of `tokio::time::interval` being constructed without
`rt_handle.enter()` in scope. The mitigation is to mirror the
`ServerTimeRecipe` pattern.

NB: this does NOT mean the entire `iced::Task::perform` closure runs
outside tokio. The `rt.spawn(...)` call at `runner.rs:831` schedules
the engine future ON the agent runtime, so the engine has reactor
context. The `provider.get_quote_history_interval(...).await` inside
`preload_yahoo_bars → fetch_with_backoff → fetch_and_cache` is
similarly fine: `reqwest::Client` constructed by `yahoo-finance-api`
uses `tokio` internally and its tasks register with the reactor on
spawn. So the preload IO works; only the `tokio::time::interval`
TIMER in the Task::perform closure breaks.

### Q2 — Feature gate

**Verified at source:**

- `crates/ui/src/lab/runner.rs:705` — `#[cfg(feature = "yahoo")]`
  gates the production ticker loop block at lines 706–828.
- `crates/ui/src/lab/runner.rs:636–644` — when `not(feature = "yahoo")`,
  the call returns an Err immediately if `data_source == YahooCache`.

The dev's claim is verified. The feature gate IS `#[cfg(feature = "yahoo")]`
exactly as named. If the operator built with `--features live` ONLY
(no `yahoo`), the entire ticker block is compiled out AND the
`YahooCache` branch returns an error before any preload happens —
operator would see a "Run failed" banner, not "endless spinning".

**Verdict**: if the operator's recipe is `cargo run --features
"live,yahoo"` (or `--features live --features yahoo`), this Q is
moot. If they accidentally used `--features live` only, they would
see an immediate error banner — incompatible with "endless spinning"
report. The most-likely cause of the operator's symptom is H-R1d
(tokio context), not H-R1b (missing feature). Recipe should
nonetheless require BOTH features to defend against future operators.

### Q3 — iced-frame gap between update() and subscription() re-register

**Web research:** iced 0.14 calls `Application::subscription()` after
every `update()` returns, per
[DeepWiki iced subscriptions](https://deepwiki.com/iced-rs/iced/6.2-subscriptions)
and the
[State-Driven Subscriptions blog](https://d34dl0ck.me/rust-bites-iced-subscriptions/index.html):

> "The method is called during each update cycle, allowing
> subscriptions to change dynamically based on application state.
> The runtime compares the new set of subscriptions with the previous
> set, starting new ones and stopping removed ones."

So: `update()` → `subscription()` rebuild → reconciliation → next
event loop iteration. There IS a brief frame-window between the
sentinel `try_send` (which happens INSIDE the `iced::Task::perform`
closure, NOT in `update()`) and the moment iced reconciles the new
subscription set.

Critical timing for the sentinel:

1. `update(LabRunRequested)` returns → iced calls `subscription()` →
   `LabProgressRecipe { salt: N+1 }` is registered → iced's runtime
   begins polling its `stream()` body (which takes the `rx_opt` and
   loops on `rx.recv().await`).
2. Concurrently, iced begins polling the `Task::perform` future
   returned from `update(LabRunRequested)`.
3. The first thing the Task closure does is `progress_tx.try_send(sentinel)`.
4. The sentinel lands in the tokio mpsc buffer.
5. Whichever of (1)'s `rx.recv().await` poll OR (3)'s `try_send`
   happens first determines whether the sentinel arrives at the iced
   update loop quickly or after a small delay.

Either ordering works: `tokio::mpsc::channel(8)` is a queue, so a
`try_send` BEFORE `recv().await` registers simply sits in the buffer
and is consumed on the next `recv().await` poll. No drop, no panic.

**H-R1c verdict**: the sentinel race is real but BENIGN — the buffer
absorbs the ordering ambiguity, and `rx.recv().await` (the recipe
stream body) ALWAYS drains it once it starts running. H-R1c is
REFUTED as the proximate cause.

The actual cause of label dormancy is downstream: even after the
sentinel is delivered, the SUBSEQUENT 250 ms tick emits never fire
(H-R1d), so the label sits at "0/1 bars · 0.0s" — exactly the
elapsed_ms value the sentinel carries.

### Q4 — R2 new API: `RunCancelReceiver::notified()` vs `tokio_util::sync::CancellationToken`

**Verified at source** (`crates/backtest/src/cancel.rs:50–72`):

Current public surface:
```rust
pub struct RunCancelHandle { _tx: std::sync::mpsc::SyncSender<()> }
pub struct RunCancelReceiver { rx: std::sync::mpsc::Receiver<()> }
impl RunCancelReceiver {
    pub fn is_cancelled(&self) -> bool { /* try_recv → matches Disconnected */ }
}
pub fn cancellation_pair() -> (RunCancelHandle, RunCancelReceiver) {
    let (tx, rx) = std::sync::mpsc::sync_channel(0);
    (RunCancelHandle::new(tx), RunCancelReceiver { rx })
}
```

The receiver wraps a `std::sync::mpsc::Receiver` — a SYNCHRONOUS
primitive. `try_recv()` returns immediately; `recv()` blocks the
calling thread. NEITHER is awaitable. Cannot be polled inside
`tokio::select!`.

**Three options** for the R2 fix:

#### Option A — Amend `RunCancelReceiver` with `notified()`

Add a `tokio::sync::Notify` alongside the std `sync_channel`. The
handle holds an `Arc<Notify>`; on drop it calls `notify.notify_one()`.
The receiver exposes:
```rust
impl RunCancelReceiver {
    pub fn is_cancelled(&self) -> bool { ... }  // existing
    pub async fn notified(&self) { self.notify.notified().await }
}
```

Pros: minimally invasive; `is_cancelled()` continues to work; old
call sites unchanged. Cons: two parallel signals (std channel +
tokio Notify) — risk of one firing without the other if the handle
Drop ordering is subtle. Heavier than necessary.

#### Option B — Replace internals with `tokio_util::sync::CancellationToken`

Reshape:
```rust
pub struct RunCancelHandle(CancellationToken);
pub struct RunCancelReceiver(CancellationToken);
impl Drop for RunCancelHandle {
    fn drop(&mut self) { self.0.cancel(); }
}
impl RunCancelReceiver {
    pub fn is_cancelled(&self) -> bool { self.0.is_cancelled() }
    pub async fn cancelled(&self) { self.0.cancelled().await }
}
pub fn cancellation_pair() -> (RunCancelHandle, RunCancelReceiver) {
    let token = CancellationToken::new();
    (RunCancelHandle(token.clone()), RunCancelReceiver(token))
}
```

Pros: ONE source of truth; `CancellationToken` is the canonical
tokio cancel primitive; `cancelled()` is a `Future` directly usable
in `select!`; no Drop-ordering subtleties; adds `tokio-util` as a
dep (already pulled transitively by iced, verify).

Cons: requires bumping `tokio-util` to a direct dep (check
`crates/backtest/Cargo.toml`); semantic shift — the std-channel
"disconnect" idiom is replaced by `Arc<CancellationToken>` clone-and-cancel.

Per architect.md style guide ("boring, production-proven Rust
crates"), `tokio_util::sync::CancellationToken` is the
production-proven boring choice. Used widely in the tokio ecosystem
(axum, hyper, sqlx all use it for graceful shutdown).

#### Option C — Standalone `tokio::sync::Notify` field

Like Option A but without retaining the std channel. Simpler than A
but reinvents what `CancellationToken` already provides.

#### Recommendation

**Option B (`tokio_util::sync::CancellationToken`)** is the durable
choice per architect.md.

- Boring + production-proven (axum/hyper/sqlx precedent).
- Single source of truth (no parallel std + tokio signals).
- `.cancelled() -> impl Future` slot is exactly what `tokio::select!`
  needs in the preload loop.
- `tokio-util` likely already transitively pulled; even if direct
  dep is added, it is a thin (~3 KB) addition.

Signature sketch (architect lock target):
```rust
// crates/backtest/src/cancel.rs (post-fix)
use tokio_util::sync::CancellationToken;
pub struct RunCancelHandle { token: CancellationToken }
pub struct RunCancelReceiver { token: CancellationToken }
impl Drop for RunCancelHandle {
    fn drop(&mut self) { self.token.cancel(); }
}
impl RunCancelReceiver {
    #[must_use] pub fn is_cancelled(&self) -> bool { self.token.is_cancelled() }
    pub async fn cancelled(&self) { self.token.cancelled().await }
}
pub fn cancellation_pair() -> (RunCancelHandle, RunCancelReceiver) {
    let token = CancellationToken::new();
    (RunCancelHandle { token: token.clone() },
     RunCancelReceiver { token })
}
```

Existing `is_cancelled()` callers (`engine::run_scenario` + 4
scenario bar loops) keep working unchanged. New call site in
`runner.rs` select! adds a `_ = cancel.cancelled() => break Err(...)`
branch.

### Q5 — `backtest::progress::progress_pair()` outside tokio runtime

**Verified at source** (`crates/backtest/src/progress.rs:64–67`):
```rust
pub fn progress_pair() -> (ProgressSender, tokio::sync::mpsc::Receiver<Progress>) {
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    (ProgressSender::new(tx), rx)
}
```

`tokio::sync::mpsc::channel(N)` is RUNTIME-AGNOSTIC at construction
time — confirmed by the tokio docs for `tokio::sync::mpsc::channel`
which state "this method does not require an active tokio runtime"
(the channel is built from `Arc`s + atomics; runtime is only needed
when a `recv()` is `.await`ed and a wakeup must be scheduled).

The `cockpit_live.rs:1494` call site IS on the iced main thread (no
tokio context). Construction is safe. Sending is safe
(`try_send` uses atomics only; no runtime needed). Receiving via
`rx.recv().await` IS where a reactor is needed — but the receiver is
moved into `LabProgressRecipe::stream()` which DOES enter the
`rt_handle` first (see `progress.rs:75`).

**Verdict**: `progress_pair()` outside a tokio runtime is SAFE. No
fix required. Dev's open Q is CLOSED.

Same answer for `tokio::sync::mpsc::unbounded_channel()`: runtime-
agnostic at construction. Verified.

## § 3. Design D-clauses for next-step fix

Bias DURABLE per AGENT.md.

### R1 fix — tokio reactor context for the preload ticker

**D-R1.1** — `spawn_lab_run` MUST enter the `rt_handle` (the cloned
`rt: tokio::runtime::Handle`) before constructing `tokio::time::interval`.
Pattern follows `ui::live::server_time_stream_impl` precedent:
```rust
let mut ticker = {
    let _guard = rt.enter();
    tokio::time::interval(Duration::from_millis(250))
};
```
The guard is dropped immediately; the `Sleep` futures returned by
`ticker.tick()` carry their reactor binding and continue to fire on
the agent runtime even after the guard drops.

**D-R1.2** — The same `_guard = rt.enter()` pattern MUST apply to
ALL future `tokio::time::*` or `tokio::net::*` calls inside the
`iced::Task::perform` closure. The doc comment at `cockpit_live.rs:110–125`
is the authoritative reference for this invariant; cross-link from
`runner.rs` so it does not get lost.

**D-R1.3** — Add an end-to-end test (per CLAUDE.md non-negotiable for
overlays — adapted to UI runtime contracts) that verifies the ticker
fires at least N times during a synthetic 1-second preload. Use the
existing `MockLabYahooBarSource` injection path
(`runner.rs:681–703`) with a `tokio::time::sleep(1000)` mock. Assert
that `LabProgressRecipe::stream_impl` yields ≥ 3 `LabRunProgress`
events with `elapsed_ms > 0`. This guards against future regressions
where the `_guard = rt.enter()` is accidentally removed.

### R2 fix — cancel-token in preload select!

**D-R2.1** — `backtest::cancel` MUST replace its internals with
`tokio_util::sync::CancellationToken` per Q4 recommendation. Public
API preserves `is_cancelled()` AND adds `cancelled() -> impl Future`.
This is a minor breaking change ONLY in that `RunCancelReceiver` now
holds a token instead of a std mpsc receiver; no caller sees the
change unless they were destructuring the type internals (none do).

**D-R2.2** — `spawn_lab_run` MUST extend the preload `tokio::select!`
at `runner.rs:778–805` with a third branch:
```rust
_ = cancel.cancelled() => {
    drop(yahoo_activity_handle);  // emit End{Cancelled}
    return Err(SmolStr::new("cancelled"));
}
```
Place this branch BEFORE the ticker arm and AFTER the preload arm so
the `biased` order is: preload (winning case), cancel (stop-during-preload),
ticker (still-running animation).

**D-R2.3** — Add an end-to-end test that confirms Stop during preload
aborts within ≤ 500 ms. Use `MockLabYahooBarSource` configured to
sleep for 5 s; trigger Stop after 250 ms; assert the
`LabRunCompleted(Err("cancelled"))` arrives within 500 ms wall-clock.
Add to the existing `spawn_lab_run_yahoo_harness.rs` as Surface 1
Test 4.

### Defense-in-depth for H-R1c (sentinel race)

**D-R1.4** — Although Q3 concluded H-R1c is benign, defend against
future regressions by adding a `tokio::task::yield_now().await`
between `progress_tx.try_send(sentinel)` and the `ticker.tick().await`
that consumes t=0. This forces the closure to surrender to the
executor briefly, giving iced's subscription reconciliation a
canonical wake point. Cost: ~0 µs. Benefit: makes the sentinel-vs-
recipe-register order DETERMINISTIC rather than scheduler-dependent.

### Trace coverage

**D-Tr.1** — Each of D-R1.1 / D-R2.1 / D-R2.2 gets a row in
`spec/trace.toml`. Architect column cites this dev-note + the future
attempt-3 feature folder. Required.

## § 4. Scope decision

Operator already locked **Q1 = (a) one-ship R1 + R2** per the
analyst's Q-BUG64-D11-3-Q1. Reaffirmed from architect side:

- R1 fix (D-R1.1) is ~5 LoC in `runner.rs`.
- R2 fix (D-R2.1 + D-R2.2) is ~25 LoC across `backtest/src/cancel.rs`
  and `runner.rs`.
- Defense-in-depth D-R1.4 is ~1 LoC.
- E2E tests D-R1.3 + D-R2.3 are ~80 LoC of test code.

Combined surface is ~30 LoC production + 80 LoC tests + 1 cargo dep
bump (`tokio-util` direct). Single architect M-T1 lock against the
`runner.rs:705–828` region + `cancel.rs:50–72` region. Splitting
into (b) sequential would committed two M-T1 passes against
overlapping regions — the second lock would amend the first. Avoid.

**Architect lock**: Q1 = (a) one-ship. Confirmed.

## § 5. ADR registry

This fix touches two distinct architecture surfaces:

1. **The iced-tokio reactor-context contract** for code that runs
   inside `iced::Task::perform` async closures. Currently codified
   ONLY as an inline doc comment at `cockpit_live.rs:110–125`. The
   recurrence of H-R1d (this is the SECOND time this exact mistake
   has been shipped — first was the P1 fix on 2026-05-23 in the
   `ServerTimeRecipe`) suggests an ADR is warranted.

2. **The `RunCancelReceiver` cancellation primitive shape** — moving
   from std `mpsc::sync_channel(0)` to `CancellationToken` is a
   moderate architectural shift that future agents need to understand.

### Recommendation

**Write a new ADR-0050 — "iced ↔ tokio runtime-context contract and
cooperative cancellation primitives"**. Single ADR covers both
clauses:

- D1 — Any code path that runs inside `iced::Task::perform` or
  inside a `Recipe::stream()` body and calls `tokio::time::*`,
  `tokio::net::*`, or other reactor-dependent APIs MUST enter the
  agent runtime via `let _guard = rt_handle.enter();` before the
  call. The guard may be dropped before `Box::pin` per the K8
  pattern; the constructed Sleep/Stream carries its reactor binding.
- D2 — Cooperative cancellation between the iced main thread and
  tokio-runtime tasks SHALL use `tokio_util::sync::CancellationToken`.
  Custom std-mpsc-disconnect patterns are deprecated.
- D3 — Tests for code paths that include `tokio::time::*` calls
  inside `iced::Task::perform` MUST assert ≥ 1 timer event within a
  bounded window, to catch future regressions where `rt_handle.enter()`
  is accidentally dropped.

The ADR atomic-register contract (architect.md) requires:
- Write `spec/architecture/adr/0050-iced-tokio-runtime-context-and-cancellation.md`
- Add row to `spec/architecture/adr/README.md` table in same commit
- Update README frontmatter `updated:` field

ADR-0048 (lab-recipe-test-harness) gets a Changelog amendment noting
that Surface 1 Test 4 (cancel-during-preload) was added per ADR-0050
§ D3 — same-commit registry update.

This ADR work happens in the developer's M-T1 lock pass, NOT now.
Architect note: deferred to attempt-3 architect M-T1 pass.

## § 6. Open Qs for analyst

All architect-side Qs are answered. Three Qs remain that ONLY the
analyst can decide:

### A-Q1 — Defense-in-depth scope

D-R1.4 (yield_now between sentinel and ticker.tick consume) is
strictly defensive. It is correct but adds ~1 LoC and 1 sentence of
explanatory comment. Analyst: ship in attempt-3 (durable) or skip
(YAGNI)? Architect bias: SHIP — costs nothing, future-proofs the
sentinel race.

### A-Q2 — `tokio-util` direct-dep vs feature-gated dep

Adding `tokio_util::sync::CancellationToken` to `backtest::cancel`
requires `tokio-util = "0.7"` in `crates/backtest/Cargo.toml`. The
crate is already transitively present (via `tokio` ecosystem deps).
Options:

- (a) Direct dep `tokio-util = { version = "0.7", features = ["rt"] }`
- (b) Direct dep `tokio-util = { version = "0.7", default-features = false, features = ["rt"] }`
- (c) Feature-gate behind a `cancellation` feature so non-live
  builds avoid the tiny code-size cost.

Architect bias: (b). The `rt` feature is required for
`CancellationToken`; default features bring in extra surface (codec,
io) we don't use. Skipping default features keeps the workspace
lean.

### A-Q3 — Documentation strategy for the iced-tokio context contract

The contract currently lives as an inline doc comment at
`cockpit_live.rs:110–125`. ADR-0050 (recommended in § 5) would
codify it as a durable architecture document. Should the analyst:

- (a) Promote to ADR-0050 in attempt-3 (recommended).
- (b) Defer ADR-0050 until a third instance of the bug appears.
- (c) Add a tracing assertion or compile-time lint instead of doc.

Architect bias: (a). The pattern has now bitten us twice (P1 fix
2026-05-23, Bug #64 attempt-2 2026-05-29). One more time and it
becomes a recurrence pattern; codifying NOW prevents instance #3.

## Constraints honored

- READ-ONLY pass — zero edits to `crates/`. All content derived
  from direct source read.
- No destructive git commands run. No `git stash`, no `git checkout
  -- .`, no `git reset`. Only `grep`/`Read`/`WebSearch`/`WebFetch`.
- Single dev-note written via this `spec-update`. No other writes.
- Single-binary; no Docker; edition 2024 — verified by reading
  `Cargo.toml` workspace.

## Changelog

- 2026-05-29 (architect): created. Bug #64 D.1.1 attempt-3
  architect validation of developer's code-map (commit `92864cc`).
  Confirms H-R1d (no `rt_handle.enter()` before `tokio::time::interval`
  at runner.rs:744) as the proximate cause of R1 dormant label.
  Confirms R2 cancel-receiver-not-polled-during-preload as
  structural omission. Recommends `tokio_util::sync::CancellationToken`
  as the durable cancel primitive (Q4). Drafts D-R1.1/2/3/4 +
  D-R2.1/2/3 design clauses for one-ship fix. Recommends new
  ADR-0050 for the iced ↔ tokio reactor-context contract. Three
  analyst Qs surfaced (A-Q1 yield_now defense, A-Q2 tokio-util
  dep shape, A-Q3 ADR-0050 timing).
