---
slug: ui-rethink-phase-d-trail-followup
status: proposed
owner: architect
updated: 2026-05-20
version: 0.1.1
predecessor: ui-rethink-phase-d-trail v0.1.0
---

# UI rethink Phase D+ — Trail follow-up (v0.1.1 patch)

> Patch release closing the five items deferred at Phase D v0.1.0 ship
> (presenter deck approved 2026-05-20 via "Autoapprove all"). The
> Phase D v0.1.0 surface remains shipped and unchanged; this brief
> only adds the live-update subscription bridge, three new snapshot
> baselines, two benchmarks, and the K7 live-counter probe — none of
> which mutate any anchored body, any of the 22 body-SHA-256 anchors,
> or any Phase A/B/C/D-shipped UI surface.
>
> **Scope source-of-truth:** presenter deck § 5 + § 8 of
> [`spec/ui-rethink-phase-d-trail/presentations/ui-rethink-phase-d-trail-2026-05-20.md`](../ui-rethink-phase-d-trail/presentations/ui-rethink-phase-d-trail-2026-05-20.md)
> and tester-report § 11 of
> [`spec/ui-rethink-phase-d-trail/reports/test-final-2026-05-20.md`](../ui-rethink-phase-d-trail/reports/test-final-2026-05-20.md).
>
> **Implementation contract:** this brief.
>
> **ADR carry-forward:** Phase D's amendment in
> [`spec/architecture/adr/0031-audit-tick-consumer-envelope.md`](../architecture/adr/0031-audit-tick-consumer-envelope.md)
> § "Phase D amendment (2026-05-20)" still applies verbatim — the
> follow-up adds **zero** new architecture edges. The trail-mirror's
> existing `reflection → audit (via AuditTick stream)` edge is the
> only data plane in scope.

## Why

Phase D v0.1.0 shipped 22/22 anchors PASS, 937/937 lib tests PASS,
6/6 layout-invariants PASS, and 3/3 trail-reconstruction integration
tests PASS — the trail screen, the universal Live chevron, the
side-drawer, and the SQL-backfill path are all live. Five items were
explicitly deferred as Phase D+ (v0.1.1) follow-up scope; the
operator's "Autoapprove all" tick on the presenter deck
(2026-05-20) accepted that cut. This brief closes them.

The deferred items break naturally into three buckets:

1. **Live-update bridge** (R1 — T-D-N26). The `TrailMirror` task is
   already spawned at `crates/agent/src/main.rs:179-185` and emits
   `TrailMirrorTick::TrailReady` / `TrailUpdated` on a
   `tokio::sync::broadcast` sender. The cockpit's iced
   `Subscription` does not yet consume that stream. v0.1.0's trail
   screen runs via on-click SQL backfill (R6.3) — fully functional
   but missing the steady-state push-update path the broadcast bus
   was built for. The placeholder is at
   `crates/ui/src/state.rs:1362` (`Message::TrailMirrorTick(SmolStr)`)
   and `:1836` (update arm — currently a no-op stub).

2. **Snapshot coverage** (R2 — T-D-N27 / T-F3). Three new `insta`
   snapshot baselines extend forward coverage of the Phase D
   surfaces. **They are NEW baselines, not changes to any of the 22
   anchored body-SHAs** — anchor risk is zero by construction
   (R7.1).

3. **Performance + paper-mode probe** (R3 + R4 + R5 — T-D-N29 / T-F6
   / T-F7 / T-F9). H5 backfill-latency bench (≥10⁵ audit rows,
   p99 < 50 ms), H3 idle-CPU floor (≤13.6% under the new
   Subscription + universal chevron), and K7 paper-mode counter
   assertion (`reflection_audit_tick_seen_total{variant="ForecastEmitted"}` ≥ 1)
   close out the falsifiable performance hypotheses and the
   structurally-verified-but-never-fired live counter.

The K7 live-counter probe (R5) carries a **hard infrastructure
dependency**: it requires a BS-1 (or BS-2) TCN checkpoint **plus**
an armed live data feed (paper-mode binary running). The wiring is
structurally complete (presenter deck § 4 K7 line) — the missing
piece is the deployment environment. Surfaced as Q1 below for the
operator-decide.

## Requirements

### R1 — Iced `Subscription` bridge (closes T-D-N26)

**R1.1** — New free function in `crates/ui/src/live.rs` (or a new
sibling module — architect-decide):

```rust
pub fn trail_mirror_subscription(
    handle: TrailMirrorHandle,
) -> iced::Subscription<Message>
```

The function constructs an iced `Recipe` (precedent:
`live::BusRecipe` at `crates/ui/src/live.rs:112-130`) that wraps
`handle.tick_tx.subscribe()` as a `BoxStream` of `Message::TrailMirrorTick`
variants. The recipe's `hash()` impl uses `TypeId::of::<Self>()` +
a static discriminant (mirrors `BusRecipe::hash`) so iced
doesn't duplicate the recipe across redraws.

**R1.2** — Extend the `Message::TrailMirrorTick` payload to carry
the **structured** mirror tick, not just a `SmolStr` placeholder.
Current shape (`crates/ui/src/state.rs:1362`):

```rust
TrailMirrorTick(SmolStr),  // v0.1.0 placeholder
```

Phase D+ shape (proposed):

```rust
TrailMirrorTick(reflection::trail_mirror::TrailMirrorTick),
```

The enum already exists at `crates/reflection/src/trail_mirror.rs:103-110`
with two variants (`TrailReady(Box<ReconstructedTrail>)` /
`TrailUpdated(String)`). The `TrailReady` arm hydrates
`model.trail_screen_state.reconstructed_trail` (new field — see
R1.4); the `TrailUpdated` arm flags the cached audit_id as
"stale" so the next render re-fetches from the mirror via
`TrailMirrorRequest::Open`.

**Open Q1 (operator-decide):** payload shape — see
"Open questions" Q2 below; the architect-decide between (a)
re-export `reflection::trail_mirror::TrailMirrorTick` directly into
the `ui` crate or (b) wrap it in a UI-local `TrailMirrorUiTick`
struct is **NOT** an operator question — flag it for the
architect's M-T1 pass.

**R1.3** — Wire `trail_mirror_subscription(handle)` into
`Cockpit::subscription` at the cockpit-binary level. The
load-bearing site is `crates/ui/src/bin/cockpit_live.rs:864-887`
(the existing `fn subscription` body). The `TrailMirrorHandle` is
currently constructed in `crates/agent/src/main.rs:180-185` and
bound to `_trail_mirror_handle` (intentionally underscored at
v0.1.0). To reach the cockpit's `Subscription`, the handle must
flow from `agent::main` into the `Cockpit` struct (or into a
sibling field next to `bus: Arc<EventBus>` already held by the
binary).

**Open Q2 (operator-decide via architect-spike):** plumbing path
— see "Open questions" Q3 below.

**R1.4** — Extend `Cockpit::trail_screen_state` with two new
fields:
- `reconstructed_trail: Option<reflection::trail_mirror::ReconstructedTrail>`
  — populated by `Message::TrailMirrorTick(TrailReady(boxed))` arm;
  consumed by `screens::trail::view` trail-mode body (R2.3 / R3 of
  the predecessor).
- `pending_trail_audit_id: Option<SmolStr>` — set when the operator
  clicks the chevron (compound dispatch `OpenTrailFor` →
  `SelectTrailRow`); cleared when the mirror responds with
  `TrailReady`. While `Some`, the trail-mode body renders a
  `frame::loading_body` placeholder (R3.4 empty-stage degradation
  reused).

These fields default to `None`. They are pure UI-side state; no
audit-side schema change.

**R1.5** — Cadence: **every mirror tick** (no UI-side throttling at
v0.1.1). The mirror's broadcast capacity is 16 (see
`trail_mirror.rs:207` `broadcast::channel(16)`); under sustained
load the `RecvError::Lagged(n)` path is already wired
(`AuditTickStream::next` at `crates/audit/src/tick.rs:172-191`).
The cockpit drops on lag — same policy as v0.1.0.

**Open Q3 (operator-decide via H3 falsification):** if the H3 idle-
CPU floor falsifies under "every tick" cadence (R4), the fallback
is a 4 Hz throttle (250 ms coalescing window) — mirrors the
`ThrottledSpinner` 10 fps precedent at
`crates/ui/src/widgets/throttled_spinner.rs`. **Default → (a)
every tick**; throttle only if T-F6 falsifies H3.

### R2 — 3 new `insta` snapshot baselines (closes T-D-N27 / T-F3)

**R2.1** — `trail__steady_state` — list mode, byte-identical to
legacy `audit::view`. This is the R2.2 / R7.1 byte-identity gate
for the trail screen's default mode. Authored in
`crates/ui/tests/visual_snapshots.rs` (the existing snapshot host).
Acceptance: `cargo insta accept` commits the baseline; subsequent
runs match SSIM ≥ 0.99 per the `visual_diff::matches_screenshot`
helper convention.

**R2.2** — `trail__side_drawer_open` — trail mode with the
side-drawer showing a Forecast-node payload. Fixture: deterministic
`ReconstructedTrail` with a fully-populated `forecast` stage (R4.2
of the predecessor). Authored in the same harness.

**R2.3** — `live__recent_activity_with_chevron` — Live screen with
the universal chevron rendered on every recent-activity row
(R5.1 of the predecessor). Fixture: 5-row `agent_feed::ready_body`
state.

**R2.4** — Non-regression: the 3 baselines are **NEW** files under
`crates/ui/tests/snapshots/` (or wherever the host stores
baselines). They do not alter any of the 22 anchored body-SHA-256
files in `spec/anchors.toml`. R7.1 / H2 gates remain non-negotiable
and unchanged.

### R3 — H5 backfill-latency benchmark (closes T-D-N29 / T-F9)

**R3.1** — New bench file `crates/reflection/benches/trail_mirror.rs`
using `criterion` (workspace dep per `Cargo.toml`).

**R3.2** — Fixture: seed an **in-memory** SQLite ledger
(`audit::Ledger::open_in_memory` or `:memory:` URL) with **10⁵**
synthetic `journal_transactions` rows split across the four trail
tables (`journal_transactions`, `strategy_signals`,
`forecast_events`, plus the `forecast_correlation_id` / `signal_id`
/ `fill_id` chains the trail joins on). Use a deterministic seed
(`ChaCha20Rng::seed_from_u64(0xD0_05D5_C0_FFEE)` precedent — pick
a unique seed for this bench).

**R3.3** — Benchmark body: time **100 random `Open` requests**
through the `TrailMirror::handle_request` path
(`crates/reflection/src/trail_mirror.rs:279-298`). Each request
runs the full LRU-miss → `audit::query::trail_for_fill_id`
backfill chain (the LRU is cleared between iterations to force the
SQL path).

**R3.4** — Acceptance gate: p99 < 50 ms (H5 falsification gate per
the predecessor). The criterion harness emits per-sample latencies;
the test asserts `quantile(99) < Duration::from_millis(50)`.

**R3.5** — H5 falsification routing: if p99 ≥ 50 ms, the deferred
**R6.3 pre-fetch redesign** triggers and becomes a Phase D++ scope
expansion. Flag as a sub-scope decision in the test report
(architect routes to a `v0.1.2` follow-up if needed).

### R4 — H3 idle-CPU floor under Phase D+ surface (closes T-F6)

**R4.1** — Reproduce `cockpit-performance v1.0.0`'s idle-CPU
benchmark with the Phase D+ surface armed:
- Trail-mirror task spawned (R1.3 + agent main wiring already in
  place).
- iced `Subscription` includes the new `trail_mirror_subscription`
  recipe (R1.1).
- Live screen rendering the universal chevron on every recent-
  activity row (Phase D v0.1.0 already on disk).

**R4.2** — Budget: idle CPU ≤ **13.6%** (Phase D v0.1.0 floor is
13.1%; budget is 13.1% + 0.5% Phase D headroom — see R7.4 of the
predecessor and presenter deck § 4 K2).

**R4.3** — Acceptance: bench script outputs a 60-second sustained
idle-CPU sample; assertion is the median of N=3 runs ≤ 13.6%. If
two of three runs ≥ 13.7%, H3 falsifies — fallback per R1.5 Open
Q3 (throttle the Subscription to 4 Hz coalescing).

**R4.4** — Tooling: **Open Q4 (operator-decide)** — see "Open
questions" Q4 below. The presenter deck (§ 5 T-F6) says "no bench
tooling available in this sandbox". The analyst proposes **macOS
`top -l 1 -n 0 -pid <pid> -stats cpu`** sampling at 1 Hz × 60
samples; alternative `tokio-console` or `cargo-flamegraph` if
the operator has a preference. **Default → (a)** simple `top`
sampling — minimal external dependency, mirrors the original
v1.0.0 perf bench shape per
[`spec/cockpit-performance-and-input-responsiveness/feature.md`](../cockpit-performance-and-input-responsiveness/feature.md).

### R5 — K7 paper-mode `ForecastEmitted` counter probe (closes T-F7)

**R5.1** — Run paper-mode agent (`cargo run --bin agent --
--mode paper --config <cfg>`) with:
- `[strategies.tcn_overlay_momentum] enabled = true` (the config
  knob landed at `crates/agent/src/config.rs` per T-D-N19 of the
  predecessor).
- `[reflection] audit_tick_consumer_enabled = true` (the v0.1.0
  audit-tick-consumer-envelope feature flag).
- A **BS-1 TCN checkpoint** loaded (this is the hard blocker — see
  Q1 below).
- A live data feed (Binance WS or paper-replay equivalent).

**R5.2** — After **≥ 60 s** sustained run, query the counter
`reflection_audit_tick_seen_total{variant="ForecastEmitted"}` from
the Prometheus / `metrics` exporter (the counter is already wired
at `crates/reflection/src/audit_tick_consumer.rs:57-61` per the
predecessor's v0.1.0 audit-tick-consumer feature).

**R5.3** — Acceptance: counter value ≥ 1. Asserts the full
production wiring end-to-end:
`TcnSyncForecaster::with_ledger` (T-D-N20) →
`build_registry_with_ledger` (T-D-N22) →
`post_forecast_event` + `tick::emit_public` from the two
forecast emit sites (T-D-N5; `crates/forecast/src/tcn.rs:861-879`,
`:985-1007`) → broadcast bus → `AuditTickStream` → counter
increment.

**R5.4** — Negative-path graceful behaviour: if the BS-1 checkpoint
is **missing**, the `TcnSyncForecaster::load_bs1` call returns
`Err`; `build_registry_with_ledger` emits `tracing::warn!` and
skips the TCN strategy registration (presenter deck § 4 K7 line).
This is **not** a silent failure — but it **does** mean the
counter assertion will read 0 in any environment without a
checkpoint. Surfaced as Q1.

**R5.5** — Hard blocker: see Open question Q1 (BS-1 checkpoint +
live feed availability).

## Open questions for operator

> Surface these for explicit operator-decide. Defaults documented
> per the analyst's research below; the operator may accept by
> taking no action ("Autoapprove all" precedent — architect proceeds
> with defaults) or override on any specific Q.

### Q1 — BS-1 checkpoint + paper-mode feed availability (HARD BLOCKER for R5)

**Question:** Is a BS-1 (or BS-2) TCN checkpoint file present in
the deployment environment that this v0.1.1 will run in? Is a paper-
mode binary running with a live (Binance WS) or replay data feed
producing bars at sufficient rate that `TcnOverlayMomentumStrategy`
will emit at least one forecast in 60 s?

**Analyst research:**
- Checkpoint provenance is locked by
  [`spec/architecture/adr/0029-tcn-checkpoint-provenance.md`](../architecture/adr/0029-tcn-checkpoint-provenance.md).
- `TcnSyncForecaster::load_bs1` / `load_bs2`
  (`crates/strategy/src/tcn_overlay_momentum.rs:170-184`) require
  a checkpoint binary file at the path configured under
  `[strategies.tcn_overlay_momentum.bs1_checkpoint_path]`.
- The presenter deck § 4 K7 explicitly carries this as the deferred
  reason: "wiring confirmed complete; live counter deferred —
  requires BS-1 checkpoint + live feed".
- v25-tcn-alpha-investigation v0.1.0 (shipped 2026-05-20) ran the
  BS-1 / BS-2 checkpoints through forecast-distribution +
  Sharpe-comparison bins (see anchors
  `forecast-distribution-bs1-realdata` /
  `forecast-distribution-bs2-realdata` — locked in
  `spec/anchors.toml`). The **checkpoint files exist** on the
  workstation that produced those anchors.

**Routing if "yes":** R5 ships as scoped. Tester runs the 60-s
paper-mode smoke; counter assertion is the M-FINAL T-F7 gate.

**Routing if "no":** R5 demotes to a **structural-only**
acceptance (statically re-verify the emit-site wiring + serde
round-trip per the v0.1.0 tester report § 3.3 T-F7). The K7 live-
counter assertion stays deferred to a future v0.1.2 (or
infrastructure-cycle) brief. Phase D+ v0.1.1 still ships the
other four items (R1-R4).

**Default → operator-decide required.** No safe analyst default —
the analyst cannot verify deployment artifact availability.

### Q2 — `TrailMirrorTick` payload shape across the crate boundary (architect-decide)

**Question:** Should the cockpit-side `Message::TrailMirrorTick`
carry (a) a direct re-export of `reflection::trail_mirror::TrailMirrorTick`
or (b) a UI-local `TrailMirrorUiTick` wrapper struct that
narrows the type?

**Analyst note:** This is **not** an operator question — flagging
it for architect M-T1. The `ui` crate currently depends on
`reflection` indirectly (via `agent`); the architect must confirm
a direct `ui → reflection` dependency is acceptable per
[`spec/architecture/01-data-flow.md`](../architecture/01-data-flow.md)
edge invariants. **No operator action required.**

### Q3 — TrailMirrorHandle plumbing path

**Question:** Where does the `TrailMirrorHandle` live so that
`Cockpit::subscription` can reach it?

**Analyst research:** The handle is constructed in `crates/agent/src/main.rs:180-185`
as `_trail_mirror_handle: Option<TrailMirrorHandle>`. The iced
`Cockpit` struct is created in `crates/ui/src/bin/cockpit_live.rs`
(`Cockpit` field at the application initialiser around line
~860-900). The plumbing options:

- **(a)** Add a `trail_mirror_handle: Option<TrailMirrorHandle>`
  field to the `Cockpit` struct itself (sits next to
  `bus: Arc<EventBus>` already plumbed via the v0.1.0 cockpit-live
  bootstrap). Pros: simplest; reuses the existing `bus`-style
  plumbing convention. Cons: bloats the `Cockpit` struct with
  binary-only state.
- **(b)** Add a sibling field at the binary-app level (outside
  `Cockpit`) and pass the handle through closure-capture into the
  subscription builder. Pros: keeps `Cockpit` slim. Cons: less
  uniform with the existing `bus` precedent.
- **(c)** Move the `TrailMirrorHandle` construction out of `agent::main`
  and into the cockpit-binary's bootstrap (re-subscribing to the
  same tick-bus sender that already lives in scope). Pros: keeps
  agent-runtime / cockpit-binary concerns separated. Cons: requires
  the cockpit-binary bootstrap to know about the trail-mirror
  module — minor reflection-crate coupling.

**Default → (a)** field on `Cockpit`. Mirrors the existing
`bus: Arc<EventBus>` plumbing convention exactly (the bus is
similarly constructed pre-cockpit and threaded into `Cockpit::new`).
Architect M-T1 confirms or overrides. **No operator action required
unless (c) is preferred** — and (c) implies moving Phase D's
already-shipped `_trail_mirror_handle` spawn site in `agent::main`,
which is a non-trivial refactor to a v0.1.0-shipped surface.

### Q4 — Idle-CPU bench tooling (R4)

**Question:** Use macOS `top -l 1 -n 0 -pid <pid> -stats cpu` at
1 Hz × 60 samples (analyst-recommended), or another tool?

**Analyst research:**
- v0.1.0 presenter deck § 5 T-F6 said "no bench tooling available
  in this sandbox".
- The original cockpit-performance v1.0.0 idle-CPU floor was
  measured pre-this-sandbox (operator workstation, 2026-05-15);
  the methodology is documented at
  [`spec/cockpit-performance-and-input-responsiveness/feature.md`](../cockpit-performance-and-input-responsiveness/feature.md)
  M0 + M2 sections (per the trace.toml row
  `REQ-COCKPIT-PERF-001`).
- macOS `top` is universally available; output parsing is one-liner
  `awk` (precedent: any system-monitoring shell script).

**Default → (a) `top` sampling, 1 Hz × 60 samples × N=3 runs;
report median CPU%.** Architect M-T1 confirms; tester implements
in a new `scripts/bench_idle_cpu.sh` (if architect agrees, this
becomes a small new project script).

### Q5 — Throttle the Subscription if H3 falsifies

**Question:** If R4 measures idle CPU > 13.6% with the
Subscription at "every tick" cadence (R1.5), do we (a) throttle
to 4 Hz coalescing and re-bench, (b) gate the Subscription behind
a `--feature live-trail-updates` Cargo flag, or (c) revert R1.5
to v0.1.0 SQL-backfill-only?

**Default → (a) throttle to 4 Hz.** Mirrors the ThrottledSpinner
10 fps precedent at `crates/ui/src/widgets/throttled_spinner.rs`
that shipped under `cockpit-performance-and-input-responsiveness
v0.1.0` (idle CPU 66.9% → 2.2-13.1%). Option (b) is a permanent
opt-out — operationally awkward. Option (c) abandons R1
altogether — too aggressive without first measuring.

## K-risk register

### K1 — Subscription bridge under-delivery (TrailMirrorTick lossy)

**Risk:** The `tokio::sync::broadcast` channel underlying
`TrailMirrorHandle::tick_tx` has capacity 16
(`trail_mirror.rs:207`). Under a forecast burst (e.g. 100 emits in
a 16-tick window because the cockpit thread blocked on a render
pass) the channel drops oldest with `RecvError::Lagged(n)`. The
iced `Subscription` recipe must handle this gracefully — log the
gap and move on (already the v0.1.0 `AuditTickStream` policy at
`crates/audit/src/tick.rs:172-191`).
**Severity:** LOW.
**Mitigation:** Architect M-T1 confirms the `Recipe::stream`
body wraps `broadcast::Receiver` with the same
`tokio_stream::wrappers::BroadcastStream` shape used in
`live.rs::BusRecipe` (precedent already drops on lag with
`tracing::warn!`). Tester gate: smoke a burst of 100 mock ticks
and assert the cockpit recovers (no panic, counter increments).

### K2 — Subscription idle-CPU regression falsifies H3

**Risk:** Even at idle, the iced Subscription may wake the
runtime once per redraw to poll the BroadcastStream; if iced
already drives a 60 fps redraw (typical), the BroadcastStream is
polled 60 times/s. Combined with the existing
`ThrottledSpinner` + `ServerTimeRecipe` (1 Hz) and the
`live::subscription` bus channels, total CPU may exceed the
13.6% budget.
**Severity:** MEDIUM.
**Mitigation:** R1.5 falls back to a 4 Hz coalescing throttle
(Q5 default = (a)). H3 falsification gate (R4.3) is the load-
bearing test. If the throttle itself falsifies — escalate to
architect (revisit Q5 (b) or (c)).

### K3 — H5 backfill p99 falsifies under 10⁵ rows

**Risk:** SQLite `:memory:` is significantly faster than the
on-disk WAL-mode ledger (the production shape). 100k rows
in-memory may pass p99 < 50 ms; on-disk may not.
**Severity:** MEDIUM.
**Mitigation:** R3.2 specifies `:memory:` for the bench (matches
the H5 hypothesis verbatim). If H5 passes on `:memory:` but
production ops observe slow first-open in the wild, that's a
follow-up bench scoped to a v0.1.2 (R6.3 pre-fetch redesign per
the predecessor's K4 mitigation). Tester documents the
`:memory:` vs. on-disk delta in the M-FINAL report.

### K4 — Snapshot baseline non-determinism (insta)

**Risk:** The 3 new baselines (R2.1-R2.3) render fixtures that may
include time-dependent fields (`HH:MM:SS.μμμ` in trail nodes per
R3.2 of the predecessor). Non-deterministic time → snapshot churn
on every test run.
**Severity:** LOW.
**Mitigation:** Reuse the existing `crates/ui/src/test_support.rs`
deterministic-clock pattern (per
[`spec/ui-quality-gate-overhaul/feature.md`](../ui-quality-gate-overhaul/feature.md)
M1-B). Fixtures must seed all timestamps from a fixed clock; the
`scripts/check_no_clocks_in_ui_tests.sh` guard catches violations.

### K5 — K7 counter assertion fails in deployment-less environment

**Risk:** If Q1 resolves "no" (no BS-1 checkpoint + feed available),
R5 demotes to structural-only — but the M-FINAL T-F7 gate is
flagged as "PASS-WITH-DEFERRED" again, repeating the v0.1.0
pattern. Operator may reasonably ask "why are we deferring twice?".
**Severity:** MEDIUM (process risk, not code risk).
**Mitigation:** Make Q1 explicit and route the answer **before**
the architect's M-T1 pass. If "no", the test report frames T-F7
as **infrastructure-blocked**, not deferred — a different gate
class. Operator may then route this to a deployment-cycle ticket
rather than a v0.1.2 feature brief.

### K6 — TrailMirrorTick payload-change ripple (R1.2)

**Risk:** Changing `Message::TrailMirrorTick(SmolStr)` to
`Message::TrailMirrorTick(reflection::trail_mirror::TrailMirrorTick)`
forces the `ui` crate to pick up a `reflection` dependency it
doesn't have today (it currently depends on `agent` which depends
on `reflection`).
**Severity:** LOW.
**Mitigation:** Architect M-T1 confirms the
`spec/architecture/01-data-flow.md` edge table permits
`ui → reflection`. The Phase D ADR amendment (ADR-0031 §
"Architecture invariants") already explicitly disallows
`ui → audit` direct edges; `ui → reflection` is a different
edge and not currently disallowed (reflection is the trail-
mirror's home crate per `decomp.md §3` of the predecessor).
Fallback per Q2 (b): wrap in a UI-local `TrailMirrorUiTick`
struct that flattens the enum.

### K7 — Bench file location convention (R3.1)

**Risk:** `crates/reflection/benches/trail_mirror.rs` is the
proposed location, but `reflection` may not currently have a
`benches/` directory wired in `Cargo.toml`.
**Severity:** LOW.
**Mitigation:** Architect M-T1 confirms the bench convention.
Precedent: any crate with criterion benches in the workspace
(typically `crates/backtest/benches/*` or `crates/forecast/benches/*`).
Bench discovery is via `[[bench]]` entries in `Cargo.toml`.

## H-hypothesis register

### H1 — Live Subscription closes the steady-state push path

**Claim:** The R1 Subscription bridge delivers `TrailReady` /
`TrailUpdated` ticks to `Cockpit::update` such that an open trail
view receives push updates without re-firing the on-click SQL
backfill.

**Falsification:** Tester writes a UI integration test that opens
a trail (chevron click), then synthesises a downstream
`AuditEvent::ForecastEmitted` tick into the broadcast bus, and
asserts that `model.trail_screen_state.reconstructed_trail`
updates within one Iced frame. Falsifies if the assertion times
out (> 100 ms).

### H2 — Phase D+ does not touch any of the 22 anchored bodies

**Claim:** The 22 body-SHA-256 anchors in `spec/anchors.toml`
remain byte-identical after Phase D+ lands. No anchored backtest
report renderer touches `trail_mirror`, `screens/trail`, or any
of the 3 new snapshot baselines.

**Falsification:** `scripts/verify_anchors.sh` → any anchor
mismatch falsifies H2. Hard gate.

### H3 — Idle CPU stays ≤ 13.6% under the Subscription bridge

**Claim:** With the Subscription bridge armed + universal chevron
+ trail-mirror task spawned, the cockpit's idle-CPU sample stays
under 13.6% (13.1% Phase D floor + 0.5% Phase D+ headroom).

**Falsification:** R4.3 measurement — median of N=3 60s samples
> 13.6% falsifies H3 and triggers Q5 fallback (a) throttle to
4 Hz.

### H4 — Snapshot baselines remain deterministic across N=10 runs

**Claim:** The 3 new insta baselines (R2.1-R2.3) produce
byte-identical pixels across N=10 consecutive `cargo test
--lib visual_snapshots` runs (SSIM = 1.0).

**Falsification:** Tester runs the snapshot test 10× consecutively
and observes any pixel diff (SSIM < 1.0) — falsifies H4 and
triggers K4 mitigation (deterministic-clock audit).

### H5 — Backfill p99 < 50 ms at 10⁵ rows (carry-forward from predecessor)

**Claim:** Single-row trail reconstruction (4 indexed point-lookups
via `audit::query::trail_for_fill_id`) completes in p99 < 50 ms
on a 10⁵-row in-memory SQLite fixture.

**Falsification:** R3.4 — bench output p99 ≥ 50 ms falsifies H5
and triggers R3.5 (R6.3 pre-fetch redesign → Phase D++ scope).

### H6 — K7 production wiring fires ≥ 1 ForecastEmitted in 60 s

**Claim:** With BS-1 checkpoint loaded + paper-mode feed running +
`tcn_overlay_momentum.enabled = true`, the production
`TcnSyncForecaster` emits ≥ 1 `AuditEvent::ForecastEmitted` tick
in 60 seconds of sustained operation, observed via the
`reflection_audit_tick_seen_total{variant="ForecastEmitted"}`
counter.

**Falsification:** Counter reads 0 after 60 s → falsifies H6 →
escalate to substantive regression (per orchestrator hand-off
note in the predecessor's tester report § 11 T-F7). **NOT** a
deferred item — a regression. If the BS-1 checkpoint is missing,
H6 is **untested** (different from "falsified") — gate routes per
Q1 resolution.

## Non-regression contract

1. **22 body-SHA-256 anchors stay byte-identical** (H2). Hard gate.
2. **All Phase A/B/C/D-shipped surfaces unchanged.** Lab + chart +
   Train panel + Lab Run button + 3-group sidebar + Live screen +
   Strategy registry + Settings rollup + Trail screen list mode
   + Trail screen trail mode + universal chevron all stay byte-
   identical.
3. **`cockpit-smoke` PASS 0 panics** (or M1-C `layout_invariants`
   proxy 6/6 PASS).
4. **`cargo fmt --check` + `cargo clippy --workspace -- -D
   warnings` exit 0.**
5. **`cargo test --workspace --lib` 100% PASS** (937 baseline +
   any new unit tests from R1).
6. **`spec-lint` Phase D+ contribution = 0** (carry-forward
   baseline; spec-hygiene cleanup from presenter deck § 8 may
   reduce the baseline but is out-of-scope here).
7. **No new external crate deps.** The `reflection` benches use
   `criterion` which is already a workspace dep. No other new
   crate is introduced.
8. **No new Lumen tokens.** The 3 snapshot baselines render
   existing Phase D widgets unchanged.
9. **`audit-tick-consumer-envelope` invariants preserved** —
   subscriber-side only; no producer-side `Ledger` change.
10. **Backtest determinism preserved** — backtests construct
    `Ledger::open` (no tick_bus); trail-mirror is paper-mode-only
    by construction (already enforced at v0.1.0 main.rs:180).

## Acceptance criteria

### M0 — Analyst synthesis (this pass)

- [x] R1-R5 anchored to predecessor presenter deck § 5 + tester
      report § 11.
- [x] Q1-Q5 surfaced with analyst-recommended defaults; Q1 explicitly
      flagged as hard blocker for R5.
- [x] K1-K7 risk register.
- [x] H1-H6 falsifiable hypotheses.
- [x] Non-regression contract enumerated (22 anchors + 9 carry-
      forward gates).
- [x] Trace row `REQ-UI-RETHINK-PHASE-D-FOLLOWUP-001` opened in
      `draft` state.
- [x] Backlog Active entry inserted above
      `v25-tcn-alpha-investigation`.

### M-T1 — Architect decomposition (next)

- [ ] Architect resolves Q2 (payload-shape) + Q3 (plumbing path)
      + Q4 (bench tooling) + Q5 (throttle policy) — these are
      architect-decide questions; Q1 stays operator-decide.
- [ ] Architect decomposes R1-R5 into ordered T-D-N rows with
      acceptance gates per task (anticipated Waves: A — payload
      + Recipe; B — Cockpit wiring; C — snapshots; D — bench;
      E — paper-mode probe).
- [ ] Architect confirms `ui → reflection` edge is permitted by
      `spec/architecture/01-data-flow.md` (or routes through a
      thin UI-local wrapper per Q2 (b)).
- [ ] Spec-lint clean (deferred to M-FINAL tester sweep).

### M-FINAL — Tester sweep (terminal)

- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D
      warnings` exit 0.
- [ ] `cargo test --workspace --lib` 100% PASS.
- [ ] 3 new snapshot baselines (`trail__steady_state`,
      `trail__side_drawer_open`, `live__recent_activity_with_chevron`)
      committed under `crates/ui/tests/snapshots/` and PASS on
      a clean run.
- [ ] `scripts/verify_anchors.sh` → 22/22 PASS — non-negotiable
      (H2 gate; carry-forward from predecessor).
- [ ] `cockpit-smoke` → 0 panic lines (or M1-C
      `layout_invariants` 6/6 proxy).
- [ ] `crates/reflection/benches/trail_mirror.rs` bench output
      asserts p99 < 50 ms (H5 gate).
- [ ] Idle-CPU floor median(N=3) ≤ 13.6% under Phase D+ surface
      (H3 gate; R4.3).
- [ ] **R5 / T-F7 paper-mode probe** — gate per Q1 resolution:
      if Q1=yes, counter ≥ 1 in 60 s; if Q1=no, structural-only
      re-verification + infrastructure-blocked routing.
- [ ] H1 Subscription integration test PASS.
- [ ] H4 snapshot determinism N=10 PASS.
- [ ] Author
      `spec/ui-rethink-phase-d-trail-followup/reports/test-final-<YYYY-MM-DD>.md`.

## Trace

Trace row `REQ-UI-RETHINK-PHASE-D-FOLLOWUP-001` opened in `draft`
state by this analyst pass (2026-05-20). Predecessor row
`REQ-UI-RETHINK-PHASE-D-001` stays `shipped`. `crates`, `tests`,
`anchors` columns to be filled by architect / developer / tester
respectively.

## Changelog

- 2026-05-20 (analyst): brief opened in response to operator's
  Phase D v0.1.0 ship + "five deferred items" follow-up scope
  (presenter deck § 8). R1-R5 anchored to presenter deck § 5 +
  tester report § 11. Q1-Q5 surfaced with defaults; Q1 explicitly
  flagged as hard infrastructure blocker (BS-1 checkpoint + live
  feed). K1-K7 risk register; K5 (deployment-less K7 assertion)
  surfaced as process risk to be routed via Q1 explicit answer.
  H1-H6 falsifiable hypotheses. Status: `draft`. Predecessor:
  `ui-rethink-phase-d-trail v0.1.0` shipped 2026-05-20. Awaiting
  operator-decide on Q1 (R5 hard blocker) and architect M-T1 on
  Q2-Q5.
