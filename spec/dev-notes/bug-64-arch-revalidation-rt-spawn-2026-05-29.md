---
slug: bug-64-arch-revalidation-rt-spawn
status: draft
owner: architect
updated: 2026-05-29
---

# Bug #64 — Architect RE-validation: rt.spawn() durable fix (2026-05-29)

READ-ONLY architect pass. My prior Q1 assertion was FALSIFIED twice. This
note owns that error, re-derives the mechanism from first principles
(grounded in source + docs, not memory), validates the proposed
`rt.spawn(...).await` fix against the 4 lane questions, and emits corrected
ADR-0050 D-clauses for the dev to paste. ZERO edits to `crates/`.

Lane: Bug #64 D.1.1 attempt-3, recurrence #3. Supersedes the Q1 reasoning in
`bug-64-arch-validation-2026-05-29.md § 2 Q1` and the hotfix Changelog
reasoning in ADR-0050 (2026-05-29 hotfix entry).

## § 1. Own the falsified assertion

### What I asserted (wrong, twice)

`bug-64-arch-validation-2026-05-29.md § 2 Q1` (commit `4473bd2`) said:

> "the `provider.get_quote_history_interval(...).await` inside
> `preload_yahoo_bars → fetch_with_backoff → fetch_and_cache` is fine:
> `reqwest::Client` ... uses `tokio` internally and its tasks register with
> the reactor on spawn. So the preload IO works; only the
> `tokio::time::interval` TIMER in the Task::perform closure breaks."

This was backwards. The mental model "reqwest spawns its own tasks onto
tokio, so it carries its own reactor context" is FALSE for the part that
panicked.

### Why reqwest needs the reactor MORE, not less

Source confirms (`crates/data/src/yahoo.rs:364-413`): `fetch_and_cache`
constructs `yahoo_finance_api::YahooConnector::new()` — which builds a
`reqwest::Client` — and then `.await`s `get_quote_history_interval(...)`.
reqwest (0.12) uses hyper + hyper-util. hyper-util's default DNS resolver is
`GaiResolver`, which performs blocking `getaddrinfo` by calling
`tokio::task::spawn_blocking`. `spawn_blocking` REQUIRES an active tokio
runtime handle on the current thread — it panics "there is no reactor
running" when none is reachable.

The panic the operator hit, verbatim:

```
thread '<unnamed>' panicked at hyper-util-0.1.20/src/client/legacy/connect/dns.rs:119:24:
there is no reactor running, must be called from the context of a Tokio 1.x runtime
```

`dns.rs:119` is exactly the `spawn_blocking` for `getaddrinfo`.

The crucial asymmetry I missed:

- reqwest does NOT pre-own a runtime. It assumes it is *called from within*
  a runtime context (the standard tokio convention). It calls
  `tokio::spawn` / `tokio::task::spawn_blocking` / registers I/O with the
  reactor **on whatever thread is polling the future**.
- When the whole future is polled by iced's `futures::ThreadPool` executor
  (no tokio thread-local), reqwest's *own internals* — not just our
  `tokio::time::*` calls — panic the moment they reach the DNS
  `spawn_blocking`. This is one layer DEEPER than our code.

So reqwest needs the reactor strictly MORE than a bare timer does: a timer
binds at construction and survives the guard drop (D1's K8 pattern); reqwest
calls `spawn_blocking` lazily, *inside the awaited future*, long after any
construction-scoped `rt.enter()` guard has been dropped at the first
`.await`.

### Why the hotfix's piecemeal guards could never have worked

The hotfix (`61abef6`) wrapped each `tokio::time::timeout/sleep` line in
`{ let _guard = rt.enter(); tokio::time::timeout(fut) }` and dropped the
guard before `.await` (correct, because `EnterGuard` is `!Send` and cannot
cross an await in a `Send` future). That fixed recurrence #2 (the explicit
timer at runner.rs:395) but it is structurally incapable of covering
reqwest's DNS, because:

1. The `fetch_future = src.fetch_and_cache(...)` is constructed OUTSIDE any
   guard.
2. The guard only spans the `tokio::time::timeout(per_attempt_timeout,
   fetch_future)` *constructor*, then is dropped.
3. The `.await` on the resulting `Timeout` polls `fetch_future`, which
   eventually calls reqwest → `GaiResolver::spawn_blocking` — on a thread
   with NO guard in scope.

`rt.enter()` sets a thread-local that lasts only for the synchronous scope
of the `EnterGuard`. It does NOT travel with the future across `.await`
boundaries. You cannot guard a multi-await third-party future by wrapping
its constructor.

### Why the existing test passed while production panicked

`crates/ui/tests/lab_runner_cold_cache_fetch_e2e.rs` (the hotfix's own D3
regression gate) only exercises `tokio::time::timeout`/`sleep` wrapping
`std::future::ready::<()>(())` (lines 103, 197, 250). It NEVER constructs a
`reqwest::Client` and NEVER hits DNS. So it validated the timer-guard
pattern in isolation and reported green — while the real `fetch_and_cache`
path panicked one layer down. The test's gap is the proximate reason
recurrence #3 shipped. § 5 closes it.

## § 2. Validate the rt.spawn() fix against the 4 questions

The durable fix is to stop guarding and instead run the entire fetch *on*
the tokio runtime, where reactor context is guaranteed:

```rust
let bars = rt.spawn(async move {
    fetch_with_backoff(&src, &ticker, interval, start_ms, end_ms).await
}).await??;
```

### Q1 — Is rt.spawn() the right primitive?

**YES. Validated, and it is already proven in production in this exact file.**

Evidence (`crates/ui/src/lab/runner.rs:949-993`): the *engine* call already
uses precisely this shape from inside the same `iced::Task::perform`
closure:

```rust
let join = rt.spawn(async move {
    match backtest::engine::run_scenario(scenario_cfg, cancel, progress_tx).await { ... }
});
let result = match join.await { Ok(r) => r, Err(e) => Err(...) };
```

The engine future is heavily tokio-dependent (progress channel recv, bar
loop, cancel polling) and has worked in production. This is empirical proof
that `rt.spawn(...).await` from iced's `futures::ThreadPool` executor is
sound. The preload simply needs the same treatment the engine already gets.

Why it works (mechanism, grounded in the std `Future`/`Waker` contract +
tokio docs):

- `Handle::spawn` runs the future on the runtime's **worker threads**, which
  always carry reactor + time-driver context, regardless of where `spawn()`
  was called. (tokio `Handle::spawn` docs: "spawns the given future onto the
  runtime's executor ... the thread pool is then responsible for polling the
  future until it completes.") So reqwest's `spawn_blocking` finds a runtime.
- `JoinHandle<T>` is a plain `impl Future<Output = Result<T, JoinError>>`.
  Polling it registers the *caller's* `Waker` (here: iced's executor waker)
  to be woken when the spawned task finishes. The wakeup is fired from the
  tokio worker thread via `waker.wake()`. Per the `std::task::Waker`
  contract, `wake()` is executor-agnostic — it does not require the *poller*
  to have tokio context. This is categorically different from
  `tokio::time::Sleep`, which registers with the time driver **at the
  polling thread** and therefore DOES require tokio context where it is
  polled.

So: the spawned task gets reactor context (fixes reqwest); the `JoinHandle`
poll/wake works on iced's executor (no context needed there). The asymmetry
is exactly what makes spawning correct-by-construction where guarding is not.

**Alternatives considered and rejected:**

- `rt.enter()` held across the whole await — REJECTED. `EnterGuard` is
  `!Send`; holding it across `.await` makes the closure `!Send`, which fails
  `iced::Task::perform`'s `Send + 'static` bound. Even if it compiled (e.g.
  on a current-thread runtime), the guard's thread-local does not migrate
  when a multi-thread executor moves the future between threads at await
  points. Fragile and non-portable.
- `rt.block_on()` on a dedicated thread — REJECTED. Blocking a thread inside
  an `iced::Task::perform` future stalls that future and can deadlock the
  iced executor pool; it also forfeits the cooperative `select!` with the
  ticker/cancel. `spawn` + `.await` is strictly better.

**Conclusion: `rt.spawn(...).await` is the correct primitive.** Use `.await`
on the `JoinHandle` and surface both error layers (`JoinError` + the inner
`Result`) — see Q4 for how this interacts with the existing select! loop.

### Q2 — Interaction with cancellation (the third select! arm)

The current attempt-3 loop (`runner.rs:873-923`) races `preload_future`
against `cancel.cancelled()` and `ticker.tick()` in a `biased` select!.
The fix must NOT lose that. The right shape: spawn the fetch, then race the
**JoinHandle** (not the raw fetch future) in the same select!.

```rust
let mut fetch_join = rt.spawn(async move {
    preload_yahoo_bars_owned(cfg_owned, range_owned).await   // owned args, see Q4/§6
});
let preload_result = loop {
    tokio::task::yield_now().await;          // D-R1.4 retained
    tokio::select! {
        biased;
        joined = &mut fetch_join => {
            break joined.map_err(|e| SmolStr::new(format!("preload task join error: {e}")))?;
            //     ^ JoinError → Err(SmolStr); inner Result is the loop value
        }
        _ = cancel.cancelled() => {
            fetch_join.abort();              // <-- REQUIRED: stop the spawned task
            if let Some(h) = yahoo_activity_handle { h.fail("operator cancelled"); }
            return Err(SmolStr::new("operator cancelled during preload"));
        }
        _ = ticker.tick() => { /* emit elapsed progress (unchanged) */ }
    }
};
```

**The critical correctness point**: once the fetch is a *spawned task*, the
cancel arm firing only stops the `select!`; it does NOT stop the spawned
task. The dev MUST call `fetch_join.abort()` in the cancel arm. Dropping the
`JoinHandle` alone does NOT cancel a tokio task — it detaches it (the task
keeps running, leaking the in-flight HTTP request until it completes). Two
acceptable approaches:

- **(Recommended) `JoinHandle::abort()`** in the cancel arm. Simple, local,
  no plumbing. The in-flight reqwest call is dropped at its next await point.
- Pass the `CancellationToken` (clone) INTO the spawned future and
  `select!` on `cancel.cancelled()` inside it too. More thorough (cancels
  mid-DNS) but more plumbing. Not required for the ≤500 ms cancel SLA — abort
  is sufficient because reqwest yields frequently.

**Recommendation: `fetch_join.abort()` in the cancel arm.** Document that
`abort()` is best-effort (the task stops at its next yield point) but is
well within the ≤500 ms Stop SLA.

### Q3 — What about the ticker (runner.rs:820-828)?

**The ticker stays where it is — its `rt.enter()` guard is correct and
sufficient. Do NOT move it onto rt.spawn().**

Rationale:

- The ticker uses ONLY `tokio::time::interval` — a timer. Per ADR-0050 § D1
  (the K8 pattern), `tokio::time::Interval` binds to the time driver at
  construction inside the `rt.enter()` scope and the constructed `Sleep`
  futures carry that binding across the drop. Its `.tick().await` is polled
  on iced's executor and the wakeup comes from the time driver thread — same
  executor-agnostic-waker mechanism as a `JoinHandle`. There is NO
  `spawn_blocking` and NO third-party reactor dependency in a bare interval.
- The ticker MUST stay in the `iced::Task::perform` closure (not spawned)
  because it is the loop driver that races the fetch JoinHandle and emits
  progress via `progress_tx`. Moving it onto rt.spawn would defeat the
  purpose (you'd have to bridge progress back out).
- This is the SAME conclusion the working code already encodes — recurrence
  #1 (runner.rs:744) was fixed by the `rt.enter()` guard at lines 820-825 and
  has not regressed. Only the *reqwest path* (multi-await, third-party
  `spawn_blocking`) needs spawning; the *timer path* (construction-bound,
  first-party) is correctly served by the guard.

**Decision: ticker = `rt.enter()` guard (keep). reqwest fetch = `rt.spawn()`
(change).** The discriminator is "does the future call `tokio::*` lazily
across await points (spawn it) or only bind at construction (guard it)?"
Codified as the corrected D1 in § 3.

### Q4 — Move semantics / Send bounds (full treatment in § 6)

`rt.spawn(fut)` requires `fut: Future + Send + 'static`. The current
`preload_yahoo_bars(&cfg, &range, &rt)` borrows all three args, so it cannot
be spawned as-is. `YahooBarSource` itself IS `Send + Sync` (holds only
`PathBuf` + `OnceLock<String>` — verified `yahoo.rs:212-216`), so the
blocker is purely the borrows, resolved by passing owned values into the
spawned future. Detail in § 6.

## § 3. Corrected ADR-0050 D-clauses (paste-ready for the dev)

The dev applies these as a **Changelog amendment** to
`spec/architecture/adr/0050-iced-tokio-runtime-context-and-cancellation.md`
AND updates the README registry row + frontmatter `updated:` IN THE SAME
COMMIT (architect.md atomic-register; the hotfix entry already amended the
row, so append to it). The corrected D1 supersedes the body D1 and the
2026-05-29 hotfix Changelog reasoning.

### D1 (corrected 2026-05-29 — supersedes prior D1 + hotfix entry)

> **D1 (corrected):** Off-runtime async work reachable from
> `iced::Task::perform` (or a `Recipe::stream()` body) that touches tokio —
> DIRECTLY (`tokio::time::*`, `tokio::select!`) OR TRANSITIVELY (reqwest /
> hyper DNS via `tokio::task::spawn_blocking`, or any crate that calls into
> `tokio::*` lazily across its own await points) — MUST be moved onto the
> tokio runtime via `rt.spawn(async { ... }).await`, NOT merely wrapped in
> `rt.enter()`.
>
> `rt.enter()` only sets the thread-local runtime context for the current
> *synchronous* scope and is dropped at the first `.await` boundary (it must
> be — `EnterGuard` is `!Send` and cannot cross an await in a `Send`
> future). It is therefore sufficient ONLY for APIs that bind to the runtime
> at CONSTRUCTION time and carry that binding thereafter — i.e. bare
> `tokio::time::Interval`/`Sleep`/`Timeout` constructors (the K8 pattern).
> It is NOT sufficient for multi-await futures that call into tokio LAZILY
> (reqwest's DNS `spawn_blocking` fires deep inside the awaited future, long
> after the construction-scoped guard is dropped — this is what panicked at
> `hyper-util .../connect/dns.rs:119` on Bug #64 recurrence #3).
>
> **Decision rule:** if a future only *constructs* a tokio timer in a guarded
> scope and then awaits it → `rt.enter()` guard is correct (keep the ticker
> at runner.rs:820-825 as-is). If a future *lazily* calls `tokio::*`
> (including transitively via a third-party crate) at any point reachable
> from its `.await` → it MUST be spawned. Spawning is correct-by-construction
> because the spawned task runs on tokio worker threads which always carry
> reactor + time-driver context, and the returned `JoinHandle` is an
> executor-agnostic `Future` awaitable from iced's `futures::ThreadPool`
> (proven in production at runner.rs:949-993, the engine call).

### D4 (new 2026-05-29)

> **D4:** Any code path that constructs a `reqwest::Client` (or any HTTP
> client built on hyper/hyper-util), or issues an HTTP request, from a
> context reachable by `iced::Task::perform` or `Recipe::stream()` MUST run
> that request inside a future spawned via `rt.spawn(...)`. Wrapping the
> request in `rt.enter()` is INSUFFICIENT — the DNS resolver
> (`hyper_util ... GaiResolver`) calls `tokio::task::spawn_blocking` lazily
> during the awaited request, requiring an active runtime on the polling
> thread, which iced's executor does not provide. This applies transitively
> to wrapper crates (`yahoo_finance_api`, etc.).

### D3 (amended 2026-05-29 — close the test gap)

> **D3 (amended):** The D3 timer-fired-in-bounded-window contract is
> RETAINED. ADDITIONALLY: any code path that issues an HTTP request (or calls
> a transitive `spawn_blocking`) reachable from `iced::Task::perform` MUST
> have an e2e test that exercises that request path under a NON-tokio
> executor (`futures::executor::block_on`, the same executor class as iced's
> `futures::ThreadPool`), NOT merely a timer. Asserting only on
> `tokio::time::*` wrapping `std::future::ready(())` is INSUFFICIENT — it
> masks transitive `spawn_blocking` panics (this is the exact gap in
> `lab_runner_cold_cache_fetch_e2e.rs` that let recurrence #3 ship: it tested
> timers but never constructed a reqwest client). The HTTP path test MUST
> either (a) drive a real HTTP request against a localhost fixture server, or
> (b) inject a fake source whose `preload` itself performs a real
> `reqwest`/`spawn_blocking` operation, OR (c) be a real-network integration
> test gated behind a feature/`#[ignore]`. A pure-timer test does not
> satisfy D3 for HTTP paths. See § 5 of
> `bug-64-arch-revalidation-rt-spawn-2026-05-29.md` for the recommended
> design.

### D2 — unchanged

D2 (`CancellationToken`) already landed correctly (`crates/backtest/src/
cancel.rs` verified: `cancelled()` is present and used). No change. NOTE for
the dev: D2's `cancelled()` future is runtime-agnostic to *await* but it is
now awaited inside the `select!` on iced's executor alongside the spawned
JoinHandle — both are executor-agnostic `Future`s, so this composes. The new
obligation is `fetch_join.abort()` in the cancel arm (§ 4), which is a D2
*usage* refinement, not a D2 primitive change.

## § 4. Cancellation interaction design (spawn + CancellationToken)

Three-arm `biased` select! over (1) spawned-fetch JoinHandle, (2)
`cancel.cancelled()`, (3) ticker. Full sketch in § 2/Q2. The load-bearing
rules:

1. **biased order: fetch > cancel > ticker** (unchanged from attempt-3).
   Preserves the "preload wins at the completion boundary, no ticker leak"
   contract (Surface 1 Test 3).
2. **Cancel arm MUST call `fetch_join.abort()`** before returning the
   cancelled error. Without it, the detached spawned task keeps running the
   HTTP request to completion (resource leak, and the cache write may still
   land after the operator cancelled). `abort()` is best-effort: the task
   stops at its next await/yield point; reqwest yields frequently, so the
   ≤500 ms Stop SLA holds.
3. **JoinError handling**: `&mut fetch_join` resolves to
   `Result<InnerResult, JoinError>`. A `JoinError` means the task panicked or
   was aborted. Map it to `Err(SmolStr)` ("preload task join error: {e}") so
   a panic in the fetch surfaces as a Run failure banner, not a silent hang.
   Do NOT `unwrap()` the JoinHandle (no `.unwrap()` in lib code per CLAUDE.md).
4. **Activity handle**: the `yahoo_activity_handle.fail(...)` / `drop` (End
   Success) bookkeeping stays in the `iced::Task::perform` closure (NOT in
   the spawned future), unchanged — the `ActivityHandle` is `!Send` and must
   not cross the spawn boundary. Confirmed: the comment at runner.rs:836
   already notes `ActivityHandle` is `!Send`; keep it on the closure side.
5. **`progress_tx` ownership**: the ticker arm emits progress, so
   `progress_tx` stays on the closure side. The spawned fetch does NOT need
   `progress_tx` (preload emits no per-bar progress — only the sentinel at
   t=0, already emitted before the loop). So `progress_tx` is NOT moved into
   the spawned fetch; no clone needed for the preload phase. (The engine
   phase at runner.rs:949 still moves `progress_tx` into its own spawn,
   AFTER the preload loop — sequencing unchanged.)

## § 5. Test design — exercise the reqwest path off-runtime hermetically

The gap that let #3 through: the regression gate tested timers, never HTTP.
Close it with a layered approach. The dev SHOULD implement (A) as the primary
hermetic gate and (C) as a belt-and-suspenders real-network smoke.

### (A) (Recommended) Localhost HTTP fixture under plain `#[test]`

Hermetic, no external network, exercises a REAL `reqwest`/`spawn_blocking`
DNS+connect path:

- Spin a tiny localhost HTTP server on `127.0.0.1:0` (ephemeral port) inside
  the test's tokio runtime — a one-route `tiny_http` or a `tokio`/`hyper`
  one-shot responder returning a canned Yahoo-shaped JSON (or even a 200 with
  a minimal body; the assertion is "no panic + future resolves", not parse
  correctness for the panic gate).
- Drive the *production code path* (or a thin wrapper that constructs a
  `reqwest::Client` and issues a GET to the fixture URL) via
  `futures::executor::block_on` — the SAME non-tokio executor class as iced's
  `futures::ThreadPool` — while the tokio runtime exists only as the `Handle`
  passed in. This reproduces the exact production topology: non-tokio poller,
  tokio runtime available only via `rt.spawn`.
- Assert: WITHOUT `rt.spawn` (direct `.await` of the reqwest future on
  `block_on`) → `catch_unwind` catches the "no reactor running" panic at the
  DNS resolver. WITH `rt.spawn(...).await` → no panic; future resolves.
- This is the HTTP analogue of the existing
  `lab_runner_cold_cache_fetch_e2e.rs` timer tests — same harness shape,
  real HTTP instead of `std::future::ready(())`. Name it e.g.
  `lab_runner_cold_cache_http_offexecutor_e2e.rs`.

`127.0.0.1` skips DNS in some resolvers; to FORCE the `GaiResolver`
`spawn_blocking` path that actually panicked, use `localhost` (a name that
resolves via `getaddrinfo`) rather than the literal IP. The dev should
confirm by running the WITHOUT-spawn variant first and seeing the panic — if
it doesn't panic with the literal IP, switch to `localhost`.

### (B) Fake source that performs a real spawn_blocking

A `MockLabYahooBarSource` whose `preload` calls
`tokio::task::spawn_blocking(|| std::net::ToSocketAddrs::to_socket_addrs(&"localhost:80"))`
(or any genuine `spawn_blocking`) reproduces the panic without an HTTP
server. Lighter than (A) but less faithful (no real reqwest stack). Use as a
fast unit-level guard if (A) is deemed too heavy for CI; (A) is preferred for
fidelity.

### (C) Real-network integration test, feature-gated / `#[ignore]`

A `#[ignore]`d (or `--features yahoo-online`-gated) test that runs the actual
`DefaultLabYahooBarSource::preload` against live Yahoo from a
`futures::executor::block_on` context, asserting no panic and bars returned.
Run manually / nightly, not in the hermetic CI gate. This is the only test
that exercises the literal production path end to end.

**Recommendation: ship (A) as the D3-HTTP gate (hermetic, deterministic,
fast) + (C) as a manual/`#[ignore]` real-path smoke.** (A) is the durable
regression guard; (C) catches Yahoo-API-shape drift.

## § 6. Move-semantics / Send blockers for the spawn boundary

`rt.spawn(fut)` requires `fut: Future<Output: Send> + Send + 'static`.

Verified facts:

- `YahooBarSource` (`crates/data/src/yahoo.rs:212-216`) holds `cache_root:
  PathBuf` + `revision_sha: OnceLock<String>`. Both `Send + Sync`. The struct
  is `Send + Sync`. ✓ No blocker — it can be MOVED (owned) into the spawned
  future. It does NOT need `Arc`.
- The blocker is purely BORROWS: current `preload_yahoo_bars(cfg: &..,
  range: &.., rt: &Handle)` and `fetch_with_backoff(src: &.., ticker: &str,
  ..)` take references. A spawned `'static` future cannot hold borrows of
  closure-local values.
- Resolution: introduce an owned-args entry path for the spawned future.
  Either (i) add an owned-args wrapper `preload_yahoo_bars_owned(cfg:
  LabRunConfig, range: DateRange)` that constructs `YahooBarSource` and calls
  `fetch_with_backoff` with owned `String` ticker, or (ii) clone/move the
  needed values into the `async move {}` block before `rt.spawn`. `cfg` is
  already cloned to `cfg_for_preload` (runner.rs:714); clone once more (or
  move it) into the spawned block. `range` (`DateRange`) is `Copy`/`Clone` —
  move a copy. The Yahoo `ticker` is derived inside `preload_yahoo_bars` from
  `cfg.symbol` (a `SmolStr`, `Clone + Send + 'static`) — fine.
- **`rt` inside the spawned future**: do NOT pass `rt` into the spawned
  fetch for the purpose of `rt.enter()` guards — once spawned, the task runs
  on a tokio worker with reactor context, so the timer guards inside
  `fetch_with_backoff` become UNNECESSARY (they are harmless if left, but the
  dev MAY simplify `fetch_with_backoff` to drop the per-line `rt.enter()`
  guards since the whole function now runs on-runtime). ARCHITECT NOTE: the
  dev SHOULD remove the now-redundant guards from `fetch_with_backoff` to
  avoid leaving a misleading "this needs a guard" signal — but this is a
  cleanup, not a correctness requirement. Flag, don't block.
- `EnterGuard` !Send: confirmed (tokio docs). It must never cross `.await`.
  Spawning sidesteps this entirely (no guard needed in the spawned task).
- `JoinError`: `Send + 'static`. ✓ crosses back fine.
- `Vec<trading_core::Bar>` (the success payload): `Send`. ✓

**No hard Send blockers.** The only work is converting the borrow-based
preload entry to an owned-args spawned future. ~10-15 LoC.

## § 7. Open questions for the dev

1. **Spawn granularity**: spawn `preload_yahoo_bars` as a whole (cache load +
   fetch), OR only spawn the `fetch_with_backoff` sub-call and keep the
   synchronous `load_cached` on the closure side? Architect bias: spawn the
   WHOLE `preload_yahoo_bars` (owned-args wrapper) — `load_cached` is a
   blocking parquet disk read and is happier on a worker thread anyway, and
   one spawn boundary is simpler than two. Confirm there's no `!Send` in the
   parquet read path (likely fine; `polars`/`arrow` reads are `Send`).
2. **Remove redundant `fetch_with_backoff` guards?** Per § 6, the per-line
   `rt.enter()` guards become dead weight once the function runs on-runtime.
   Remove for clarity (recommended) or leave (harmless)? If removed, update
   the ADR-0050 D1 reference-implementation table row for `fetch_with_backoff`
   and the function's doc comment (runner.rs:395-408) — keep the ADR honest.
3. **`abort()` vs token-into-task** for cancellation (§ 4 rule 2): architect
   bias is `abort()` (sufficient for ≤500 ms SLA). Confirm the existing
   `lab_runner_cancel_e2e` cancel test still passes with the spawned-fetch
   topology, and add a cancel-during-spawned-fetch assertion if the existing
   one only covered the inline preload.
4. **Localhost vs IP in the § 5(A) fixture**: confirm whether `127.0.0.1`
   short-circuits the `GaiResolver`. If it does, use `localhost` to force the
   `spawn_blocking` DNS path that actually panicked (the whole point of the
   gate).

## Constraints honored

- READ-ONLY pass — ZERO edits to `crates/`. All claims derived from direct
  source read (`runner.rs`, `cancel.rs`, `yahoo.rs`, `cold_cache_fetch_e2e`,
  workspace `Cargo.toml`) + tokio/tokio-util docs (WebFetch) + the verbatim
  operator panic.
- NO destructive git, NO commits. Single dev-note written. Orchestrator
  commits.
- Single-binary; no Docker; edition 2024.

## Changelog

- 2026-05-29 (architect): created. RE-validation of Bug #64 D.1.1
  recurrence #3. Owns the falsified Q1 assertion (reqwest needs the reactor
  MORE, not less — transitive DNS `spawn_blocking` panics deeper than our
  code). Validates `rt.spawn(...).await` as the durable fix (proven in
  production at runner.rs:949-993, the engine call). Corrected ADR-0050 D1
  (spawn, don't guard, for lazy/transitive tokio futures) + new D4 (HTTP
  must be spawned) + amended D3 (HTTP path test, not just timers). Cancel
  design: `fetch_join.abort()` in the cancel arm. Test design: localhost
  HTTP fixture under `futures::executor::block_on` (hermetic D3-HTTP gate) +
  feature-gated real-network smoke. No hard Send blockers (`YahooBarSource`
  is `Send + Sync`; only borrows need converting to owned-args).
