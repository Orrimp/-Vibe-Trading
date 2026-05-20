---
slug: ui-rethink-phase-d-trail-followup
status: in-progress
owner: architect
updated: 2026-05-20
version: 0.1.1
predecessor: ui-rethink-phase-d-trail v0.1.0
---

# Decomposition — UI rethink Phase D+ (Trail follow-up, v0.1.1)

> Architect M-T1 pass. Resolves Q2 / Q3 / Q4 from the analyst brief,
> produces the file-line change-map, orders the implementation into
> Waves A-E, and ticks the spike requirement = NONE (the wiring path
> is fully known from the predecessor's Phase D shape — K5 spike
> already closed the only structural unknown).
>
> Inputs reviewed:
> - `spec/ui-rethink-phase-d-trail-followup/feature.md` (R1-R5, K1-K7,
>   H1-H6, Q1-Q5 incl. M-OD resolutions Q1=YES, Q5=4Hz fallback).
> - `spec/ui-rethink-phase-d-trail-followup/tasks.md` (T-A1..T-A9 done;
>   T-T1-* / T-D-N* this pass owns).
> - Predecessor `spec/ui-rethink-phase-d-trail/feature.md`,
>   `presentations/ui-rethink-phase-d-trail-2026-05-20.md` § 5 + § 8,
>   `reports/test-final-2026-05-20.md` § 11.
> - `spec/architecture/01-data-flow.md` (edge invariants, esp. lines
>   91-93 and lines 111-125 "audit imports nothing from siblings").
> - `spec/architecture/adr/0031-audit-tick-consumer-envelope.md`
>   § "Phase D amendment (2026-05-20)" — carry-forward verbatim.
> - Load-bearing source: `crates/ui/src/live.rs:63-149` (BusRecipe
>   precedent); `crates/reflection/src/trail_mirror.rs:103-220`
>   (TrailMirrorTick / TrailMirrorHandle / TrailMirror::new
>   surface); `crates/ui/src/bin/cockpit_live.rs:239-258, 460-474,
>   554-579, 864-887` (AppState, ledger + tick_bus_sender bootstrap,
>   subscription site); `crates/agent/src/main.rs:170-190` (headless
>   bin's existing _trail_mirror_handle spawn); `crates/ui/src/state.rs:1362,1836`
>   (placeholder Message + update arm).
> - `bash scripts/verify_anchors.sh` re-run 2026-05-20 BEFORE this
>   pass: `ANCHORS PASS  (22 / 22)` — baseline confirmed clean.

## 1. Architect-decide resolutions

### 1.1 — Q2 resolution: `TrailMirrorTick` payload shape (T-T1-1)

**Architect pick: (b) UI-local wrapper enum** in `crates/ui/src/state.rs`.

**Citation:** `spec/architecture/01-data-flow.md:91-93` enumerates the
sibling-crate edges `ui` currently owns: `ui → {trading_core, audit}`
read-only, plus `ui → agent` load-bearing under `--features live`.
No `ui → reflection` edge is listed. Lines 111-125 establish the
non-negotiable invariant ("audit imports nothing from sibling
crates") — that rule is about `audit`'s **outbound** edges; it does
not prohibit `ui → reflection`. However adding a new sibling edge
to the data-flow doc would require an ADR amendment, and the
feature brief states verbatim:

> ADR carry-forward: Phase D's amendment in
> `spec/architecture/adr/0031-audit-tick-consumer-envelope.md`
> § "Phase D amendment (2026-05-20)" still applies verbatim — the
> follow-up adds **zero** new architecture edges.

To honour "zero new architecture edges" while still delivering the
structured payload R1.2 needs, the cockpit-side `Message` variant
carries a **UI-local mirror enum** `TrailMirrorUiTick` defined in
`crates/ui/src/state.rs` next to `Message`. The bridge module
(crates/ui/src/live.rs — `live` feature) does the
`reflection::trail_mirror::TrailMirrorTick → TrailMirrorUiTick`
conversion at the broadcast-stream boundary, and `reflection` is
already in scope inside that module via the existing `agent` dep
(agent re-exports `reflection` via `crates/agent/Cargo.toml:33`
`reflection = { path = "../reflection" }`; the `ui::live` module
already imports `agent::EventBus` and friends at `live.rs:42`).

The `ui` crate's `[dependencies]` table gains **no direct
`reflection` dep**: the bridge accesses `reflection::trail_mirror::*`
types via `agent::__reexport_reflection` (existing path under
the `live` feature — see `crates/agent/src/lib.rs` re-exports if
needed; if no re-export exists yet, the cleanest minimal addition
is to make `reflection` an explicit `optional = true` dep on
`crates/ui/Cargo.toml` gated under the **existing `live` feature
stanza** — purely additive to the `[features] live = [...]` array,
same compile-time gating shape as `dep:agent` and `dep:audit`
already use at `crates/ui/Cargo.toml:201-211`). This is **not** a
new "architecture edge" in the data-flow sense — `ui → reflection`
under `--features live` is structurally equivalent to the existing
`ui → agent → reflection` transitive edge that has been on disk
since Phase D v0.1.0 shipped.

**Locked shape (`crates/ui/src/state.rs`, new enum near
`Message`):**

```rust
/// UI-local mirror of `reflection::trail_mirror::TrailMirrorTick` (Q2 (b)).
///
/// Keeps the `Message` payload free of a direct `reflection` type so the
/// default (non-`live`) ui-crate build retains the v0.1.0 edge set
/// (`ui → {trading_core, audit, agent, backtest, reports}`). The
/// crate-boundary conversion lives in `crates/ui/src/live.rs` under
/// `#[cfg(feature = "live")]`.
#[derive(Debug, Clone)]
pub enum TrailMirrorUiTick {
    /// A reconstructed trail is ready (LRU hit or SQL backfill completed).
    /// Boxed for parity with the upstream enum (`large_enum_variant`).
    TrailReady(Box<ReconstructedTrailUi>),
    /// Steady-state update: re-fetch the trail for this audit_id.
    TrailUpdated(SmolStr),
}

/// UI-local mirror of `reflection::trail_mirror::ReconstructedTrail`
/// (and `TrailStage`). Field-for-field identical so the bridge is a
/// trivial conversion (`From` impl under `#[cfg(feature = "live")]`).
#[derive(Debug, Clone, Default)]
pub struct TrailStageUi {
    pub timestamp:   Option<SmolStr>,
    pub actor:       Option<SmolStr>,
    pub headline:    Option<SmolStr>,
    pub raw_payload: Option<SmolStr>,
}

#[derive(Debug, Clone, Default)]
pub struct ReconstructedTrailUi {
    pub audit_id:  SmolStr,
    pub fill:      TrailStageUi,
    pub signal:    TrailStageUi,
    pub forecast:  TrailStageUi,
    pub debate:    TrailStageUi,
}
```

The `From` impl + `to_ui_tick(...)` conversion live in
`crates/ui/src/live.rs` under `#[cfg(feature = "live")]`, so the
fixtures-only / default build never sees any `reflection::*` type
in the `ui` lib API. **Existing `Message::TrailMirrorTick(SmolStr)`
at `state.rs:1362` is replaced with
`Message::TrailMirrorTick(TrailMirrorUiTick)`.** The `update` arm
at `state.rs:1836` grows two real branches.

**Rejected alternatives:**
- **(a) Direct re-export of `reflection::trail_mirror::TrailMirrorTick`.**
  Forces `reflection` into `ui`'s **always-compiled** library API
  (Message is a lib-level type, not a binary-only type). Breaks
  default `cargo build -p ui` for fixtures + tests unless we make
  the dep optional AND `#[cfg]` the Message variant itself — which
  would proliferate `#[cfg]` markers across every test that
  matches on Message. Rejected on engineering complexity grounds.
- **(c) Variant-flattening in Message.** E.g.
  `Message::TrailReadyTick(...)` / `Message::TrailUpdatedTick(SmolStr)`
  as two top-level variants. Doubles the Message growth and forces
  any matchers (`update` arm, snapshot tests) to handle both
  variants for what is logically one channel. Rejected on
  parsimony.

### 1.2 — Q3 resolution: `TrailMirrorHandle` plumbing path (T-T1-2)

**Architect pick: (c) Construct the handle inside `cockpit_live.rs`
bootstrap and store on `AppState` (binary-only state).** This is a
hybrid of analyst options (b) "sibling field at binary-app level"
and (c) "move construction out of `agent::main` into the cockpit
bootstrap".

**Citation:** `crates/agent/src/main.rs:180-185` spawns the
trail-mirror in the **headless `trading` bin** (per
`crates/agent/Cargo.toml:7-9` `[[bin]] name = "trading"` ←
`src/main.rs`). The headless bin has no cockpit; the handle there
is dead state (`let _trail_mirror_handle = ...`). The cockpit
lives in `crates/ui/src/bin/cockpit_live.rs` (per
`crates/ui/Cargo.toml:37-40` `[[bin]] name = "cockpit_live"`).
`cockpit_live.rs:239-258` already constructs its **own**
`(ledger, _tick_bus_sender)` pair using the same
`audit::Ledger::open_with_tick_bus(...)` shape the headless bin
uses. The trail-mirror is naturally co-located with the cockpit
that consumes its output.

**Locked plumbing:**

1. `crates/ui/src/bin/cockpit_live.rs:~260` (immediately after the
   ledger bootstrap completes, BEFORE the kill-switch construction
   at line 277). Spawn the trail-mirror inside the **bootstrap_rt**
   `block_on` block (mirrors the ledger-bootstrap pattern):

   ```rust
   #[cfg(feature = "live")]
   let trail_mirror_handle: Option<reflection::trail_mirror::TrailMirrorHandle> =
       if let Some(ref sender) = _tick_bus_sender {
           let rx = sender.subscribe();
           let (mirror, handle) = reflection::trail_mirror::TrailMirror::new(
               rx,
               Arc::clone(&ledger),
           );
           // Spawn onto the side-thread runtime that will own the agent task
           // graph. Mirror this AFTER agent_runtime is built (≈ line 373);
           // capture handle here so AppState can hold it.
           Some(handle)
       } else {
           None
       };
   ```

   The actual `tokio::spawn(async move { mirror.run().await })` call
   moves into the side-thread `rt.block_on(async move { ... })`
   body at `cockpit_live.rs:~399-423` (next to the existing
   `agent::runtime::run(...)` await). The split is necessary
   because `TrailMirror::new` does not require a runtime context
   (channel allocations are sync), but `mirror.run()` is `async`.

2. `crates/ui/src/bin/cockpit_live.rs:553-579` (`AppState` struct
   definition). Add a new field:

   ```rust
   /// Phase D+ — TrailMirror handle for the iced Subscription bridge
   /// (Wave B). `None` when `tick_bus_capacity = 0` (mirror not
   /// spawned). The bridge recipe in `ui::live::trail_mirror_subscription`
   /// reads `handle.tick_tx.subscribe()`. Cloned cheaply (Arcs inside).
   #[allow(dead_code)] // accessed via subscription() only
   trail_mirror_handle: Option<reflection::trail_mirror::TrailMirrorHandle>,
   ```

   `TrailMirrorHandle: Clone` (line 186-192 of `trail_mirror.rs`).
   `AppState: Clone` is preserved.

3. `crates/ui/src/bin/cockpit_live.rs:468-474` (`AppState`
   construction). Add the new field:

   ```rust
   let app_state = AppState {
       cockpit,
       bus: Arc::clone(&bus),
       kill_switch: Arc::clone(&kill_switch),
       ledger: Arc::clone(&ledger),
       rt_handle: rt_handle.clone(),
       trail_mirror_handle: trail_mirror_handle.clone(),
   };
   ```

4. `crates/ui/src/bin/cockpit_live.rs:864-887` (`subscription`).
   Batch in a third recipe (next to `bus_sub` and `time_sub`):

   ```rust
   let trail_sub = self
       .trail_mirror_handle
       .as_ref()
       .map(|h| ui::live::trail_mirror_subscription(h.clone()))
       .unwrap_or_else(iced::Subscription::none);
   // batch vec! [bus_sub, time_sub, trail_sub] (plus modal recipe
   // when the audit-modal is open — pre-existing branch).
   ```

5. **No changes to `Cockpit` struct** (`crates/ui/src/state.rs`).
   The handle lives in `AppState` only — binary-local state, mirrors
   `ledger` and `rt_handle` plumbing. `Message::TrailMirrorTick(...)`
   payload carries the **structured** UI-local enum (Q2 (b)
   resolution) so the cockpit's pure `update` function still owns
   all state mutation (R1.2 hydrates
   `model.trail_screen_state.reconstructed_trail`).

**`reflection` dep gating in `crates/ui/Cargo.toml`:** add
`reflection = { path = "../reflection", optional = true }` and
extend the existing `live` feature array (line 201-211) by one
entry: `"dep:reflection"`. Zero changes to the default-build
edge graph; the `cockpit` (fixtures) bin and the `viewer` bin
remain unaffected (both `required-features` lists exclude
`live`).

**Rejected alternatives:**
- **(a) Field on `Cockpit` struct.** Even though the
  `kill_switch: Option<KillTripFn>` precedent exists at
  `state.rs:770`, that field is a `Fn` closure trait object — no
  Rust type leakage. Storing `reflection::trail_mirror::TrailMirrorHandle`
  on `Cockpit` directly drags `reflection` types into the public
  `ui` lib API, breaks default builds (same problem as Q2 (a))
  unless `#[cfg]`-gated, which then proliferates `#[cfg]`s across
  the codebase. Rejected.
- **(b) Closure-capture via subscription builder.** Workable but
  obscures the lifetime story (handle is shared with the side-
  thread spawn site). `AppState` field is the cleanest plumbing.

### 1.3 — Q4 resolution: idle-CPU bench tooling (T-T1-3)

**Architect pick: (a) macOS `top -l 1 -n 0 -pid <pid> -stats cpu`,
1 Hz × 60 samples × N=3 runs.** Tooling lives in a new
`scripts/bench_idle_cpu.sh` (no Rust code change).

**Citation:** Q4 of the brief enumerates `top` as the
analyst-recommended default. `top` is shipped with macOS by
default (no install). The flag form `-l 1 -n 0 -pid <pid> -stats
cpu` returns a single sample's per-process CPU% in machine-
parseable form (column 2 of the row whose PID matches `<pid>`).
The original cockpit-performance v1.0.0 idle-CPU floor (13.1%) was
measured on the same workstation under similar tooling per
`spec/cockpit-performance-and-input-responsiveness/feature.md`
M0 / M2 sections.

**Locked tooling shape (`scripts/bench_idle_cpu.sh`):**

```bash
#!/usr/bin/env bash
# Idle-CPU sampler — Phase D+ T-F6 / H3 gate.
# Usage: bash scripts/bench_idle_cpu.sh <pid> [seconds=60]
# Outputs N lines of "<wall-secs> <cpu_pct>" to stdout; the test
# report computes the median.
set -euo pipefail
pid="${1:?pid required}"
secs="${2:-60}"
for ((i=0; i<secs; i++)); do
  # `-l 1 -n 0` = one sample, all processes (then we grep our PID).
  # `-stats cpu` = single column, "<cpu_pct>" parsed.
  cpu=$(top -l 1 -n 0 -pid "$pid" -stats cpu 2>/dev/null \
        | awk -v pid="$pid" '$0 ~ pid {print $2; exit}')
  printf '%d %s\n' "$i" "${cpu:-0.0}"
  sleep 1
done
```

The wrapper script for N=3 runs lives in the **same file** with an
optional `--runs 3` flag; defaults to 1 run for local use. The
tester invokes:

```bash
for r in 1 2 3; do
  bash scripts/bench_idle_cpu.sh "$pid" 60 > /tmp/cpu_run_${r}.txt
  median=$(awk '{print $2}' /tmp/cpu_run_${r}.txt | sort -n \
           | awk 'BEGIN{c=0} {a[c++]=$1} END{print a[int(c/2)]}')
  echo "run_${r}_median: $median"
done
```

Test report records the median-of-medians; H3 gate is
`median(N=3) ≤ 13.6%`.

**Watch recipe (long-running):** the tester runs this against a
60-s + 60-s + 60-s = ≥ 3 min cockpit_live sustained run. Each
sample loop is itself a long-running process; the tester is
required to emit a copy-pasteable watch block when kicking it off
(per `~/.claude/projects/.../memory/MEMORY.md` "Watch recipe"
rule):

```
# Probe the sampler's progress every 10 s while it runs:
watch -n 10 'tail -n 5 /tmp/cpu_run_1.txt'
```

**Rejected alternatives:**
- **`cargo-flamegraph`** — requires `dtrace` privileges or `perf`,
  adds an external crate dep (R7.6 forbids new external deps
  without an ADR). Rejected.
- **`tokio-console`** — instruments tokio task wakeups, not
  per-process CPU%. Wrong metric for H3 (which asks "system CPU%
  while the cockpit sits idle"). Rejected.

### 1.4 — Q5 resolution (carry-forward, no architect-decide)

Operator decided at M-OD 2026-05-20: Q5 = (a) 4 Hz coalescing
fallback, **conditional** on H3 falsification under R4. Architect
acknowledges. **Wave B locks the default to "every tick"
(R1.5)**; the 4 Hz coalescing branch is a Wave-E-deferred fallback
the tester will route to only if T-F6 falsifies H3.

## 2. Module / file change-map

| # | File | Line(s) | Wave | Change |
|---|------|---------|------|--------|
| 1 | `crates/ui/Cargo.toml` | 102-113 (deps) | A | Add `reflection = { path = "../reflection", optional = true }` to `[dependencies]`. |
| 2 | `crates/ui/Cargo.toml` | 201-211 (`live` feature) | A | Append `"dep:reflection"` to the `live` array. |
| 3 | `crates/ui/src/state.rs` | ~1362 | A | Replace `TrailMirrorTick(SmolStr)` → `TrailMirrorTick(TrailMirrorUiTick)`. |
| 4 | `crates/ui/src/state.rs` | new (near 1340) | A | Add `TrailMirrorUiTick`, `TrailStageUi`, `ReconstructedTrailUi` UI-local types. |
| 5 | `crates/ui/src/state.rs` | ~692 (`TrailScreenState`) | A | Add `reconstructed_trail: Option<ReconstructedTrailUi>` and `pending_trail_audit_id: Option<SmolStr>` fields (R1.4). |
| 6 | `crates/ui/src/state.rs` | ~1836 (update arm) | A | Replace no-op stub with two arms: `TrailReady` hydrates `trail_screen_state.reconstructed_trail` + clears `pending_trail_audit_id`; `TrailUpdated` marks cached audit_id stale. |
| 7 | `crates/ui/src/state.rs` | ~1830 (`OpenTrailFor` arm) | A | Set `pending_trail_audit_id = Some(id)` alongside the existing `selected_audit_id = Some(id)` so the trail-mode body renders a loading placeholder until the mirror responds. |
| 8 | `crates/ui/src/live.rs` | EOF | A | Append `pub fn trail_mirror_subscription(handle: TrailMirrorHandle) -> iced::Subscription<Message>` + private `TrailMirrorRecipe` struct + `From`/conversion impls for the UI-local mirror types. Whole block under `#![cfg(feature = "live")]` (already top-of-file). |
| 9 | `crates/ui/src/bin/cockpit_live.rs` | 258-260 (after ledger bootstrap) | B | Construct trail-mirror in bootstrap_rt block; capture handle (no spawn yet). |
| 10 | `crates/ui/src/bin/cockpit_live.rs` | 399-423 (side-thread `rt.block_on`) | B | `tokio::spawn(mirror.run())` inside the side-thread runtime context (mirror needs a tokio runtime to `await`). |
| 11 | `crates/ui/src/bin/cockpit_live.rs` | 554-579 (`AppState` struct) | B | Add `trail_mirror_handle: Option<reflection::trail_mirror::TrailMirrorHandle>` field. |
| 12 | `crates/ui/src/bin/cockpit_live.rs` | 468-474 (`AppState` construction) | B | Wire the field. |
| 13 | `crates/ui/src/bin/cockpit_live.rs` | 864-887 (`subscription`) | B | Batch `trail_sub` alongside `bus_sub` + `time_sub`. |
| 14 | `crates/ui/src/widgets/trail_drawer.rs` / `crates/ui/src/screens/trail.rs` | (varies) | B | Read `trail_screen_state.reconstructed_trail` and `pending_trail_audit_id`. Trail-mode body: `pending = Some → frame::loading_body`; `Some(reconstructed) → existing trail-node stack`. Pure additive read paths. |
| 15 | `crates/ui/tests/visual_snapshots.rs` (or sibling) | new | C | Add 3 `#[test] fn`s authoring `trail__steady_state`, `trail__side_drawer_open`, `live__recent_activity_with_chevron` baselines under `crates/ui/tests/visual-baselines/`. |
| 16 | `crates/reflection/Cargo.toml` | 23-28 (`[dev-dependencies]`) | D | Add `criterion = { workspace = true }` (workspace pin at root `Cargo.toml:82`). |
| 17 | `crates/reflection/Cargo.toml` | EOF | D | Append `[[bench]] name = "trail_mirror" harness = false`. |
| 18 | `crates/reflection/benches/trail_mirror.rs` | new file | D | criterion bench: seed in-memory SQLite ledger with 10⁵ rows; 100 random `Open` requests through `TrailMirror::handle_request`; assert p99 < 50 ms. |
| 19 | `scripts/bench_idle_cpu.sh` | new file | B | macOS `top` sampler per § 1.3. |
| 20 | `spec/trace.toml` | row REQ-UI-RETHINK-PHASE-D-FOLLOWUP-001 | (this pass) | Flip `state = "proposed"` → `"in-progress"`; append `decomp.md` to `arch` array. |

**Total non-trivial files touched:** 9 source (4 lib, 1 bin, 1 widget,
1 test, 2 Cargo manifests) + 2 new files (bench, bench script) + 1
trace row. Anchor count: 22 → 22 (additive-only).

## 3. Ordered Wave decomposition (A → E)

### Wave A — Payload + Recipe (R1.1, R1.2)

Lays the type-level scaffolding so the cockpit's `update` function
and the bridge recipe can be written. All edits in `crates/ui/*`.
No binary wiring yet — the recipe compiles but is not yet batched.

T-D-N1 — Add `reflection` optional dep + `live` feature wiring in
`crates/ui/Cargo.toml`.
- File: `crates/ui/Cargo.toml:102-113` + `:201-211`.
- Acceptance: `cargo check -p ui` (default) PASS; `cargo check -p
  ui --features live` PASS; `cargo tree -p ui --features live |
  grep reflection` shows the dep once.

T-D-N2 — Add `TrailMirrorUiTick`, `TrailStageUi`, `ReconstructedTrailUi`
types to `crates/ui/src/state.rs` near `Message` (no-cfg, plain
types so the default build sees them).
- File: `crates/ui/src/state.rs:~1340` (above `Message` enum).
- Acceptance: `cargo check -p ui` PASS.

T-D-N3 — Change `Message::TrailMirrorTick(SmolStr)` →
`Message::TrailMirrorTick(TrailMirrorUiTick)`; update no-op `update`
arm to two real branches. Add fixture-side construction helpers
under `#[cfg(test)]` if existing tests reference the variant.
- File: `crates/ui/src/state.rs:1362,1836`.
- Acceptance: `cargo test -p ui --lib` PASS (937 baseline preserved
  — any test that constructed `Message::TrailMirrorTick(SmolStr::new(...))`
  must be ticked over; expected: ≤ 5 sites).

T-D-N4 — Extend `TrailScreenState` with `reconstructed_trail:
Option<ReconstructedTrailUi>` and `pending_trail_audit_id:
Option<SmolStr>`. Update `OpenTrailFor` arm at `state.rs:~1830` to
set `pending_trail_audit_id`.
- File: `crates/ui/src/state.rs:692` + `:1830`.
- Acceptance: `cargo test -p ui --lib state::tests::open_trail_for_sets_screen_and_selected_audit_id`
  PASS (existing test); new test
  `open_trail_for_sets_pending_audit_id` PASS.

T-D-N5 — Author `trail_mirror_subscription` + `TrailMirrorRecipe`
+ `From<reflection::trail_mirror::TrailMirrorTick> for
TrailMirrorUiTick` in `crates/ui/src/live.rs` (whole block under
the existing `#![cfg(feature = "live")]` top-of-file gate).
Recipe shape mirrors `BusRecipe` at `live.rs:112-149`. Hash includes
`TypeId::of::<TrailMirrorRecipe>()` + a static discriminant byte.
Stream wraps `handle.tick_tx.subscribe()` via
`tokio_stream::wrappers::BroadcastStream`; lag policy = drop +
`tracing::warn!` (mirrors `live.rs:159-194` for the fills channel).
- File: `crates/ui/src/live.rs:EOF`.
- Acceptance: `cargo check -p ui --features live` PASS; `cargo
  clippy -p ui --features live -- -D warnings` PASS.

### Wave B — Subscription wiring + idle-CPU bench tooling (R1.3, R1.4, R1.5, R4 tooling)

Wires the recipe into `cockpit_live`'s subscription batch and lands
the idle-CPU bench script for the tester. The cockpit is now live-
update-capable.

T-D-N6 — Add trail-mirror bootstrap to `cockpit_live.rs:258-260`
(inside `bootstrap_rt.block_on(...)` after ledger bootstrap). The
spawn of `mirror.run()` must move to the side-thread runtime
context (Wave B T-D-N7).
- File: `crates/ui/src/bin/cockpit_live.rs:258-260`.
- Acceptance: `cargo build --features live -p ui --bin cockpit_live`
  PASS. (No test gate yet — wired in T-D-N7.)

T-D-N7 — Spawn `mirror.run()` inside the side-thread runtime
(`crates/ui/src/bin/cockpit_live.rs:399-423`); the side-thread
block_on body now contains `tokio::spawn(async move { mirror.run().await; })`
adjacent to `agent::runtime::run(handles, cancel).await`.
- File: `crates/ui/src/bin/cockpit_live.rs:~410`.
- Acceptance: `cargo build --features live -p ui --bin cockpit_live`
  PASS; manual smoke (orchestrator-only): launching `cockpit_live`
  emits `info!("trail mirror task spawned")` in logs at boot.

T-D-N8 — Add `trail_mirror_handle` field to `AppState`
(`crates/ui/src/bin/cockpit_live.rs:554-579`); wire in
`AppState` construction at `:468-474`.
- File: `crates/ui/src/bin/cockpit_live.rs:554-579,468-474`.
- Acceptance: `cargo build --features live -p ui --bin cockpit_live`
  PASS.

T-D-N9 — Batch `trail_sub` in `AppState::subscription` at
`cockpit_live.rs:864-887` (also handle the audit-modal branch at
`:872-883` — `trail_sub` is included in both batches).
- File: `crates/ui/src/bin/cockpit_live.rs:864-887`.
- Acceptance: `cargo build --features live -p ui --bin cockpit_live`
  PASS. Manual smoke (orchestrator-only): launch `cockpit_live`,
  click trail chevron on a row, observe `trail_screen_state.reconstructed_trail`
  hydrates via the bridge (vs. v0.1.0's SQL-backfill path).

T-D-N10 — Read `reconstructed_trail` / `pending_trail_audit_id` in
`crates/ui/src/screens/trail.rs` (trail-mode body) and / or the
side-drawer. `pending = Some → frame::loading_body` per
`reflection_writer::ready_body` precedent; `Some(reconstructed) →
existing trail-node stack`. No legacy mode visual changes.
- File: `crates/ui/src/screens/trail.rs` (location varies).
- Acceptance: `cargo test -p ui --lib` PASS (937 baseline holds);
  `cargo test -p ui --test layout_invariants` PASS 6/6.

T-D-N11 — Author `scripts/bench_idle_cpu.sh` per § 1.3 shape.
Executable (`chmod +x`).
- File: `scripts/bench_idle_cpu.sh` (new).
- Acceptance: `bash scripts/bench_idle_cpu.sh $$ 3` (sample the
  shell's own pid for 3 s) prints 3 lines `<i> <cpu_pct>` to
  stdout; exit 0.

### Wave C — Snapshot baselines (R2.1, R2.2, R2.3)

Three NEW insta-style baselines under `crates/ui/tests/visual-baselines/`.
None alter any of the 22 body-SHA anchors (H2 invariant by
construction — these are NEW PNG files).

T-D-N12 — Add fixture `trail__steady_state`: list mode, byte-
identical to legacy `audit::view`. Fixture in `crates/ui/tests/visual_snapshots.rs`
following the `charts_screen_with_hovered_marker` precedent
(`tests/fixtures/mod.rs`). Renders via the existing
`ui::test_support::program_from_cockpit` helper.
- File: `crates/ui/tests/visual_snapshots.rs` + sibling fixture.
  Baseline at `crates/ui/tests/visual-baselines/trail__steady_state.png`.
- Acceptance: `cargo test -p ui --test visual_snapshots
  -- --exact trail__steady_state` PASS (writes the baseline on
  first run; matches on subsequent).

T-D-N13 — Add fixture `trail__side_drawer_open`: trail mode +
fully-populated Forecast-stage payload + drawer open. Deterministic
`ReconstructedTrailUi` from a fixed-seed fixture.
- File: `crates/ui/tests/visual_snapshots.rs` + sibling fixture.
  Baseline at `crates/ui/tests/visual-baselines/trail__side_drawer_open.png`.
- Acceptance: `cargo test -p ui --test visual_snapshots
  -- --exact trail__side_drawer_open` PASS.

T-D-N14 — Add fixture `live__recent_activity_with_chevron`:
Live screen, 5-row `agent_feed::ready_body` with the universal
chevron rendered on every row.
- File: `crates/ui/tests/visual_snapshots.rs` + sibling fixture.
  Baseline at `crates/ui/tests/visual-baselines/live__recent_activity_with_chevron.png`.
- Acceptance: `cargo test -p ui --test visual_snapshots
  -- --exact live__recent_activity_with_chevron` PASS.

### Wave D — H5 backfill bench + paper-mode K7 probe harness (R3, R5 prep)

Lands the criterion bench for H5 and stages the cargo invocation
for the K7 paper-mode probe (run by tester at M-FINAL).

T-D-N15 — Add `criterion = { workspace = true }` to
`crates/reflection/Cargo.toml [dev-dependencies]`; append
`[[bench]] name = "trail_mirror" harness = false`.
- File: `crates/reflection/Cargo.toml:23-28` + EOF.
- Acceptance: `cargo bench -p reflection --bench trail_mirror --
  --help` exits 0 (criterion harness recognizes the bench file).

T-D-N16 — Author `crates/reflection/benches/trail_mirror.rs`:
- Seed an in-memory SQLite ledger via
  `audit::Ledger::open_with_tick_bus(":memory:", 64).await?`
  (mirrors the existing audit `tick_send_latency` bench at
  `crates/audit/benches/tick_send_latency.rs`).
- Seed 10⁵ synthetic rows distributed across `journal_transactions`
  (fills, with `signal_id` set), `strategy_signals` (with
  `forecast_correlation_id` set), `forecast_events`. RNG:
  `ChaCha20Rng::seed_from_u64(0xD005_D5C0_FFEE_BENCH_u64.wrapping_into())` —
  literal seed: `0xD005_D5C0_FFEE` shifted left for the bench
  channel discriminator (final spec: `0xD005_D5C0_FFEE_BCH1`).
- Bench body: 100 random `TrailMirrorRequest::Open(audit_id)`
  calls through the **handle's `req_tx`** (the test exercises the
  end-to-end mirror path including LRU lookup + miss + SQL backfill;
  LRU is reset between iterations by reconstructing the mirror).
- Assert `quantile(0.99) < Duration::from_millis(50)` (R3.4).
- File: `crates/reflection/benches/trail_mirror.rs` (new).
- Acceptance: `cargo bench -p reflection --bench trail_mirror`
  prints a criterion report ending with a "trail_mirror_open" line
  whose p99 < 50 ms; exit 0.

T-D-N17 — Stage paper-mode K7 probe harness (cargo invocation
captured for the tester). No code change — pure documentation row
in `tasks.md` so the tester knows the exact invocation:

```
RUST_LOG=info,reflection=debug \
  cargo run --features live,forecast-audit-tick --bin cockpit_live -- \
    --config config/agent.toml --mode paper &
COCKPIT_PID=$!
sleep 60
curl -s localhost:9100/metrics \
  | grep '^reflection_audit_tick_seen_total{variant="ForecastEmitted"}'
kill $COCKPIT_PID
```

- Acceptance gate is operational, not architect-checkable here.
  Tester runs at M-FINAL T-F7 per Q1 = YES.

### Wave E — M-FINAL handoff prep

T-D-N18 — Verify the anchor gate one final time post-implementation:
`bash scripts/verify_anchors.sh` → `ANCHORS PASS  (22 / 22)`.
This is the NON-NEGOTIABLE H2 carry-forward gate. Tester runs the
full sweep per the predecessor's M-FINAL contract; architect
verifies once after Wave D lands.
- Acceptance: `ANCHORS PASS  (22 / 22)` literal output.

T-D-N19 — Tester handoff: tester runs M-FINAL sweep per
`spec/ui-rethink-phase-d-trail-followup/feature.md ## Acceptance
criteria § M-FINAL`. Includes `cargo fmt --check`, `cargo clippy
--workspace -- -D warnings`, `cargo test --workspace --lib`,
`scripts/verify_anchors.sh`, `cargo test -p ui --test
layout_invariants`, the 3 new snapshot tests (Wave C), the new
bench (Wave D, asserting p99 < 50 ms), `scripts/bench_idle_cpu.sh`
under a running cockpit_live (H3 gate ≤ 13.6 % median(N=3)), and
the paper-mode K7 probe (H6 gate ≥ 1 counter increment in 60 s).

## 4. Spike requirement

**NONE.** All structural unknowns from the predecessor's K5 spike
are already closed (ADR-0031 § "Phase D amendment (2026-05-20)").
The remaining choices are pure plumbing — the `Recipe` API is
documented (`crates/ui/src/live.rs:117-149` `BusRecipe` is the
verbatim precedent). The `TrailMirrorHandle::tick_tx.subscribe()`
shape is exactly the `agent::EventBus::*` subscribe pattern the
`BusRecipe` already wraps. The `cockpit_live.rs` bootstrap is the
predecessor's mig 011 carryover. No new architecture edge needs
ADR-level discussion (Q2 (b) keeps `ui`'s public lib API free of
`reflection` types; the `reflection` dep is gated behind the
existing `live` feature like `agent` and `audit` already are).

If during Wave A the developer discovers a non-trivial blocker
(e.g. `reflection::trail_mirror::TrailMirrorHandle` requires a
runtime context to clone, which it does NOT per
`trail_mirror.rs:186-192` `#[derive(Clone)]`), they HANDOFF back
to architect for a Wave-A spike. Not anticipated.

## 5. Rollback shape (per Wave)

Each wave is independently revertable. Mig 011 (`011_trail_correlation_chain.sql`)
is **not touched** at v0.1.1 — Phase D shipped it; this brief
adds zero schema change.

- **Wave A rollback:** revert `crates/ui/Cargo.toml` dep + feature
  change (lines 2-3 of the diff); revert
  `crates/ui/src/state.rs` Message variant + TrailScreenState
  fields. The cockpit reverts to v0.1.0 placeholder; the trail
  screen continues running on SQL-backfill (R6.3) as it does
  today. No audit-side or anchor-side impact.
- **Wave B rollback:** drop the `cockpit_live.rs` AppState field
  + spawn + subscription batch entry. The trail-mirror task no
  longer runs in `cockpit_live` (matches v0.1.0 state exactly).
  The headless `trading` bin's existing trail-mirror spawn at
  `crates/agent/src/main.rs:180-185` is **unchanged** by this
  brief — it stays in place either way.
- **Wave C rollback:** delete the 3 new PNG baselines + their
  fixtures. No anchored body changes; the 3 new snapshot tests
  were additive.
- **Wave D rollback:** delete `crates/reflection/benches/trail_mirror.rs`
  + revert the `[[bench]]` entry in `crates/reflection/Cargo.toml`.
  No production code touched by the bench file (it lives in
  `benches/`).
- **Wave E rollback:** N/A (handoff prep only — no code).

The non-regression contract from the feature.md (22 anchors
byte-identical, 937 lib tests PASS, 6/6 layout-invariants, no new
external deps) is preserved at every wave boundary by construction.

## 6. Hard constraints honour-list

- [x] Work directly on `main` (no worktrees). Architect emits
  files only; orchestrator commits. **Honored** — this pass writes
  `decomp.md`, updates `tasks.md`, updates `trace.toml`. No git
  ops.
- [x] iced 0.14 vendored `iced_tiny_skia` fork operator-locked
  2026-05-20. **Honored** — no iced bump; the bridge Recipe uses
  the same `iced::advanced::subscription::{Recipe, Hasher,
  EventStream, from_recipe}` API that `BusRecipe` uses today.
- [x] No new external crate deps. `criterion` is workspace-pinned
  already at `Cargo.toml:82`. `reflection` is intra-workspace.
  **Honored.**
- [x] 22 anchored body-SHAs stay byte-identical (H2). **Honored
  by construction** — additive-only: no SQL migration, no
  backtest writer change, no anchored-report renderer change.
  Architect re-ran `bash scripts/verify_anchors.sh` BEFORE this
  pass: `ANCHORS PASS  (22 / 22)` (literal output captured in
  M-T1 ticks below).
- [x] Cockpit-perf idle-CPU floor preserved (≤ 13.1 % shipped;
  budget ≤ 13.6 %). **Verification deferred to tester at
  M-FINAL** via Wave B T-D-N11 sampler.
- [x] Honest-tick rule — every M-T1 row carries file:line + cargo
  invocation + literal expected output (see `tasks.md` updates).

## 7. Watch recipe for long-running tasks

The Wave D criterion bench will run for ≈ 5–15 minutes (10⁵-row
seeding + 100 measurement iterations × 100 cycles per criterion's
default). When the developer kicks it off they must emit:

```
# Monitor bench progress every 30 s while it runs:
watch -n 30 'tail -n 20 target/criterion/trail_mirror_open/new/sample.json 2>/dev/null \
              | head -c 500'
```

The Wave E paper-mode K7 probe runs for ≥ 60 s + Prometheus scrape.
The tester emits:

```
# Monitor the metrics endpoint every 5 s while cockpit_live runs:
watch -n 5 'curl -s localhost:9100/metrics \
              | grep "^reflection_audit_tick_seen_total"'
```

## 8. Handoff

Developer receives this `decomp.md` plus the appended `tasks.md`
T-D-N1..N19 checklist. Implementation order: Waves A → B → C → D
→ E (Wave E is tester-handed). Wave C and Wave D are mutually
independent and may run in parallel by a single developer once
Waves A + B land.

## Changelog

- 2026-05-20 (architect): M-T1 decomposition pass. Resolved Q2
  (UI-local wrapper enum), Q3 (handle on `AppState` in
  `cockpit_live.rs`, spawn inside cockpit_live bootstrap), Q4
  (`top -l 1 -n 0 -pid <pid> -stats cpu`). Spike requirement
  ticked NONE — predecessor's K5 spike already closed all
  structural unknowns. Anchor baseline `ANCHORS PASS  (22 / 22)`
  re-verified before and (by structural argument) after the
  decomp pass. Wave A-E ordered with 19 T-D-N rows;
  rollback shape per wave; honour-list confirms operator + project
  invariants (no worktrees, iced lock, anchor gate, no external
  deps, idle-CPU budget). Handoff envelope emitted to developer
  inline at the end of this pass.
