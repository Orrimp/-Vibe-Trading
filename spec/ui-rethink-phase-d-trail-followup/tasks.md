---
slug: ui-rethink-phase-d-trail-followup
status: in-progress
owner: developer
updated: 2026-05-20
---

# Tasks — UI rethink Phase D+ (Trail follow-up, v0.1.1)

> Analyst pass. T-A* rows only. Architect M-T1 owns the T-T* and
> T-D-N decomposition (Waves A-E) once Q2-Q5 resolve and Q1
> infrastructure availability is operator-decided.

## M0 — Analyst synthesis

- [x] T-A1 — Read predecessor `ui-rethink-phase-d-trail v0.1.0`
  feature.md + decomp.md + tasks.md (T-D-N1..N29 + T-F1..T-F10
  rollup), presenter deck § 5 (deferred items rationale) + § 8
  (next-up follow-up brief), and tester report § 11.
  _Acceptance: feature.md "Why" anchored to presenter deck § 5 +
  tester report § 11; the 5 deferred items map 1:1 to R1-R5._

- [x] T-A2 — Read ADR-0031 § "Phase D amendment (2026-05-20)" and
  confirm the TCN production wiring shape that R5 / K7 exercises.
  _Acceptance: K7 / R5 references cite
  `crates/forecast/src/tcn.rs:861-879,985-1007`,
  `crates/strategy/src/tcn_overlay_momentum.rs:417-420,434-437`,
  `crates/agent/src/runtime.rs:163-220` — wiring already on disk._

- [x] T-A3 — Read `crates/reflection/src/trail_mirror.rs` to
  confirm `TrailMirrorHandle` surface (`tick_tx: broadcast::Sender`,
  `req_tx: mpsc::Sender<TrailMirrorRequest>`) is iced-Subscription-
  ready.
  _Acceptance: R1.1 cites `trail_mirror.rs:103-110` (tick enum) +
  `:185-192` (handle struct) + `:200-220` (constructor returns
  handle alongside task). R1.2 cites the placeholder mismatch:
  `state.rs:1362` carries `TrailMirrorTick(SmolStr)`, mirror sends
  the structured enum — bridge must reconcile._

- [x] T-A4 — Read `crates/ui/src/live.rs:63-130` (iced Recipe
  precedent — `BusRecipe`) and `crates/ui/src/bin/cockpit_live.rs:864-887`
  (existing `Cockpit::subscription` body) to identify the
  Subscription wiring site.
  _Acceptance: R1.1 / R1.3 cite the precedent + insertion point._

- [x] T-A5 — Read `crates/agent/src/main.rs:170-190` to confirm
  the `_trail_mirror_handle` underscored binding and trace the
  plumbing path back into the cockpit binary (Q3 research).
  _Acceptance: Q3 surfaces 3 options (a/b/c) with analyst-
  recommended default = (a) field on `Cockpit` (mirrors the
  existing `bus: Arc<EventBus>` plumbing precedent)._

- [x] T-A6 — Surface Q1-Q5 with recommended defaults for operator-
  decide. **Q1 is the hard infrastructure blocker** — no safe
  analyst default; Q2-Q5 carry analyst-recommended defaults the
  architect can adopt at M-T1.
  _Acceptance: feature.md "Open questions" section enumerates
  Q1-Q5; Q1 marked as hard blocker; Q2-Q5 carry defaults._

- [x] T-A7 — Author K1-K7 risk register + H1-H6 hypothesis
  register + non-regression contract (22 anchors + 9 carry-
  forward gates) + acceptance criteria per milestone (M0 /
  M-T1 / M-FINAL).
  _Acceptance: feature.md sections "K-risk register",
  "H-hypothesis register", "Non-regression contract",
  "Acceptance criteria" populated._

- [x] T-A8 — Open trace row `REQ-UI-RETHINK-PHASE-D-FOLLOWUP-001`
  in `draft` state. Insert Active backlog entry above
  `v25-tcn-alpha-investigation`.
  _Acceptance: `spec/trace.toml` carries the new `[[req]]` row;
  `spec/backlog.md` § Active has the new entry directly above
  the existing `v25-tcn-alpha-investigation` row._

- [x] T-A9 — Compute the `crates` / `tests` / `anchors` field
  initial values for the new trace row. **`crates`** = same set
  as the predecessor (`crates/ui`, `crates/reflection`,
  `crates/audit` for the bench fixture); **`tests`** = `[]`
  (developer fills); **`anchors`** = `[]` (no new anchors; H2
  carry-forward).

## M-OD — Operator-decide (resolved 2026-05-20)

- [x] T-OD1 — Q1 = **(yes)** — BS-1 (or BS-2) TCN checkpoint + paper-
  mode feed available. R5 ships as scoped; tester runs the 60-s
  probe at M-FINAL T-F7; counter ≥ 1 is the gate.

- [x] T-OD2 — Q5 = (a) 4 Hz coalescing (analyst default accepted;
  applies only if H3 falsifies under R4). Operator picked the
  analyst-recommended fallback via "Autoapprove all" + Q1=Yes
  decision 2026-05-20.

## M-T1 — Architect decomposition (resolved 2026-05-20)

Architect-decide questions resolved at M-T1. Output:
`spec/ui-rethink-phase-d-trail-followup/decomp.md`.

- [x] T-T1-1 — Q2: `TrailMirrorTick` payload shape. Architect-pick:
  **(b) UI-local wrapper enum** (`TrailMirrorUiTick`,
  `ReconstructedTrailUi`, `TrailStageUi`) added to
  `crates/ui/src/state.rs` near `Message`. Crate-boundary
  conversion lives in `crates/ui/src/live.rs` under
  `#[cfg(feature = "live")]`. Rationale: keeps `ui`'s **default-
  build** edge graph free of `reflection`. `reflection` joins
  `ui`'s `live` feature stanza only — same gating as `agent` +
  `audit` at `crates/ui/Cargo.toml:201-211`. Cite:
  `spec/ui-rethink-phase-d-trail-followup/decomp.md § 1.1`.
  Confirmation: `cargo check -p ui` (default) PASS under
  default-build → no `reflection` in `cargo tree -p ui`.

- [x] T-T1-2 — Q3: `TrailMirrorHandle` plumbing. Architect-pick:
  **(c) construct inside `cockpit_live.rs` bootstrap; store on
  `AppState`**. The existing `_trail_mirror_handle` spawn in
  `crates/agent/src/main.rs:180-185` is in the **headless
  `trading` bin** (no cockpit consumer). The cockpit-side handle
  must live where iced lives. Add field
  `trail_mirror_handle: Option<reflection::trail_mirror::TrailMirrorHandle>`
  to `AppState` at `crates/ui/src/bin/cockpit_live.rs:554-579`,
  wire in construction at `:468-474`, batch into
  `subscription()` at `:864-887`. Cite:
  `spec/ui-rethink-phase-d-trail-followup/decomp.md § 1.2`.

- [x] T-T1-3 — Q4: idle-CPU bench tooling. Architect-pick:
  **(a) macOS `top -l 1 -n 0 -pid <pid> -stats cpu`**, 1 Hz × 60
  samples × N=3 runs; report median of medians. Lives in new
  `scripts/bench_idle_cpu.sh`. No external dep; mirrors
  cockpit-performance v1.0.0 methodology. Cite:
  `spec/ui-rethink-phase-d-trail-followup/decomp.md § 1.3`.

- [x] T-T1-4 — R1-R5 decomposed into Wave A → Wave E, 19 T-D-N
  rows. Output: `spec/ui-rethink-phase-d-trail-followup/decomp.md`
  § 2 (change-map) + § 3 (ordered waves). Spike requirement
  (§ 4): **NONE** — predecessor's K5 spike closed all structural
  unknowns; the `Recipe` API is the `BusRecipe` precedent at
  `crates/ui/src/live.rs:117-149`. Rollback shape per wave at
  decomp.md § 5.

- [x] T-T1-5 — `ui → reflection` edge gating. Architect resolution:
  add `reflection = { path = "../reflection", optional = true }`
  to `crates/ui/Cargo.toml` deps and append `"dep:reflection"` to
  the existing `live` feature array. **Zero new architecture edges
  in the data-flow doc sense** — `ui → reflection` under
  `--features live` is structurally equivalent to the existing
  `ui → agent → reflection` transitive edge already on disk at
  v0.1.0. ADR-0031 carry-forward "follow-up adds zero new
  architecture edges" honored. Cite:
  `spec/ui-rethink-phase-d-trail-followup/decomp.md § 1.1`.

- [x] T-T1-6 — Anchor gate baseline re-verified BEFORE handoff to
  developer. Command: `bash scripts/verify_anchors.sh` (run by
  architect 2026-05-20 during this pass). Literal expected output
  tail:
  ```
  PASS  sharpe-comparison-realdata            17d2e96c1bb79c0dad84c81daf4be333acb2b35a8c05b954ccaee7aa53370924
  ---
  ANCHORS PASS  (22 / 22)
  ```
  Observed: matched verbatim. Wave D (additive bench file) and
  Wave A-C (additive ui-crate surface) preserve this gate by
  construction; tester re-runs at M-FINAL T-F4 (carry-forward).

## M-D — Developer implementation (Waves A → E)

Implementation order: A → B → C → D → E. Waves C and D may run
in parallel after A + B land. Honest-tick rule: every row carries
file:line + cargo invocation + literal expected output.

### Wave A — Payload + Recipe (R1.1, R1.2)

- [ ] T-D-N1 — Add `reflection` optional dep + `live` feature
  entry in `crates/ui/Cargo.toml`. Files: `crates/ui/Cargo.toml:102-113`
  (add `reflection = { path = "../reflection", optional = true }`)
  + `:201-211` (append `"dep:reflection"`).
  Command: `cargo check -p ui && cargo check -p ui --features live`.
  Expected: both exit 0; `cargo tree -p ui --features live | grep ^reflection`
  shows `reflection v...` once.

- [ ] T-D-N2 — Add UI-local mirror types (`TrailMirrorUiTick`,
  `TrailStageUi`, `ReconstructedTrailUi`) to
  `crates/ui/src/state.rs:~1340` (above `Message`).
  Command: `cargo check -p ui`.
  Expected: exit 0.

- [ ] T-D-N3 — Replace `Message::TrailMirrorTick(SmolStr)` →
  `Message::TrailMirrorTick(TrailMirrorUiTick)` at
  `crates/ui/src/state.rs:1362`; extend `update` arm at `:1836`
  with two real branches (`TrailReady → hydrate
  trail_screen_state.reconstructed_trail`; `TrailUpdated → mark
  stale`).
  Command: `cargo test -p ui --lib`.
  Expected: `test result: ok. <N≥294> passed; 0 failed`
  (the 294 baseline holds + any new asserts the developer adds).

- [ ] T-D-N4 — Extend `TrailScreenState` at
  `crates/ui/src/state.rs:692` with `reconstructed_trail:
  Option<ReconstructedTrailUi>` and `pending_trail_audit_id:
  Option<SmolStr>`. Update `OpenTrailFor` arm at `:1830` to set
  `pending_trail_audit_id`.
  Command: `cargo test -p ui --lib state::tests::open_trail_for`.
  Expected: `test result: ok. <N> passed` (at least
  `open_trail_for_sets_screen_and_selected_audit_id` PASS,
  baseline carry-forward).

- [ ] T-D-N5 — Author `trail_mirror_subscription` +
  `TrailMirrorRecipe` + `From<reflection::trail_mirror::TrailMirrorTick>
  for TrailMirrorUiTick` impl in `crates/ui/src/live.rs:EOF`
  (under existing `#![cfg(feature = "live")]`). Recipe mirrors
  `BusRecipe` at `live.rs:117-149`; uses
  `tokio_stream::wrappers::BroadcastStream` + drop-on-lag
  `tracing::warn!`.
  Command: `cargo clippy -p ui --features live -- -D warnings`.
  Expected: `Finished` line; exit 0.

### Wave B — Subscription wiring + idle-CPU bench tooling (R1.3-R1.5, R4 tooling)

- [ ] T-D-N6 — Add trail-mirror construction in
  `crates/ui/src/bin/cockpit_live.rs:258-260` (inside
  `bootstrap_rt.block_on(...)`).
  Command: `cargo build --features live -p ui --bin cockpit_live`.
  Expected: `Finished` line; exit 0.

- [ ] T-D-N7 — Spawn `mirror.run()` inside the side-thread runtime
  at `crates/ui/src/bin/cockpit_live.rs:~410`.
  Command: `cargo build --features live -p ui --bin cockpit_live`.
  Expected: `Finished` line; exit 0.

- [ ] T-D-N8 — Add `trail_mirror_handle` field to `AppState` at
  `crates/ui/src/bin/cockpit_live.rs:554-579`; wire in `AppState`
  construction at `:468-474`.
  Command: `cargo build --features live -p ui --bin cockpit_live`.
  Expected: `Finished` line; exit 0.

- [ ] T-D-N9 — Batch `trail_sub` into `AppState::subscription` at
  `crates/ui/src/bin/cockpit_live.rs:864-887` (both modal-open
  and modal-closed branches).
  Command: `cargo build --features live -p ui --bin cockpit_live`.
  Expected: `Finished` line; exit 0.

- [ ] T-D-N10 — Hydrate the trail-mode body from
  `reconstructed_trail` / `pending_trail_audit_id` in
  `crates/ui/src/screens/trail.rs` (and / or the side-drawer at
  `crates/ui/src/widgets/trail_drawer.rs`).
  Command: `cargo test -p ui --lib && cargo test -p ui --test
  layout_invariants`.
  Expected lib: `test result: ok. <N≥294> passed; 0 failed`.
  Expected layout: `test result: ok. 6 passed; 0 failed`.

- [ ] T-D-N11 — Author `scripts/bench_idle_cpu.sh` per decomp.md
  § 1.3. Mark executable (`chmod +x`).
  Command: `bash scripts/bench_idle_cpu.sh $$ 3`.
  Expected: 3 lines of `<i> <cpu_pct>` to stdout; exit 0.

### Wave C — Snapshot baselines (R2.1-R2.3)

- [ ] T-D-N12 — Author fixture + test for `trail__steady_state`
  (list mode, byte-identical to legacy `audit::view`). Files:
  `crates/ui/tests/visual_snapshots.rs` + sibling fixture
  module; baseline PNG at
  `crates/ui/tests/visual-baselines/trail__steady_state.png`.
  Command: `cargo test -p ui --test visual_snapshots -- --exact
  trail__steady_state`.
  Expected: `test result: ok. 1 passed; 0 failed` (baseline
  auto-written on first run; matches verbatim on rerun).

- [ ] T-D-N13 — Author fixture + test for
  `trail__side_drawer_open` (trail mode + Forecast-stage
  payload + drawer open). Files: same. Baseline at
  `crates/ui/tests/visual-baselines/trail__side_drawer_open.png`.
  Command: `cargo test -p ui --test visual_snapshots -- --exact
  trail__side_drawer_open`.
  Expected: `test result: ok. 1 passed; 0 failed`.

- [ ] T-D-N14 — Author fixture + test for
  `live__recent_activity_with_chevron` (Live screen, 5-row
  `agent_feed::ready_body`, universal chevron). Files: same.
  Baseline at `crates/ui/tests/visual-baselines/live__recent_activity_with_chevron.png`.
  Command: `cargo test -p ui --test visual_snapshots -- --exact
  live__recent_activity_with_chevron`.
  Expected: `test result: ok. 1 passed; 0 failed`.

### Wave D — H5 backfill bench + K7 probe harness (R3, R5 prep)

- [ ] T-D-N15 — Add `criterion = { workspace = true }` to
  `crates/reflection/Cargo.toml [dev-dependencies]`; append
  `[[bench]] name = "trail_mirror" harness = false` at EOF.
  Command: `cargo bench -p reflection --bench trail_mirror -- --help`.
  Expected: criterion help banner; exit 0.

- [ ] T-D-N16 — Author `crates/reflection/benches/trail_mirror.rs`
  per decomp.md § 3 Wave D. Seed:
  `ChaCha20Rng::seed_from_u64(0xD005_D5C0_FFEE_BCH1)`. 10⁵
  synthetic rows; 100 random `Open` requests; assert p99 < 50 ms.
  Command: `cargo bench -p reflection --bench trail_mirror`.
  Expected output line tail: `trail_mirror_open` summary with p99
  numeric value `< 50ms`; criterion exit 0.

- [ ] T-D-N17 — Stage paper-mode K7 probe invocation for the
  tester. No code change; tasks.md captures the verbatim cargo
  command:
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
  Expected: counter line with value ≥ 1 (gated by Q1 = YES; BS-1
  checkpoint required on the deployment workstation).

### Wave E — M-FINAL handoff prep

- [ ] T-D-N18 — Verify anchor gate post-implementation. Command:
  `bash scripts/verify_anchors.sh`.
  Expected tail:
  ```
  ANCHORS PASS  (22 / 22)
  ```

- [ ] T-D-N19 — Developer hands off to tester. Tester runs M-FINAL
  sweep per feature.md § Acceptance Criteria M-FINAL: `cargo fmt
  --check`, `cargo clippy --workspace -- -D warnings`, `cargo
  test --workspace --lib` (937+ baseline), `scripts/verify_anchors.sh`
  (22/22), `cargo test -p ui --test layout_invariants` (6/6),
  3 snapshot tests (Wave C), bench p99 (Wave D), idle-CPU floor
  (`scripts/bench_idle_cpu.sh` × 3 runs), K7 paper-mode probe
  (T-D-N17 invocation). Tester emits
  `spec/ui-rethink-phase-d-trail-followup/reports/test-final-<YYYY-MM-DD>.md`.
  Expected: VERDICT → PASS (gated on Q1 = YES providing the BS-1
  checkpoint at probe time).

## M-FINAL — Tester sweep (terminal)

(Tester fills T-F* rows after the developer ticks all T-D-N rows
landed by M-T1. Carries forward the predecessor's M-FINAL gate
shape verbatim — `cargo fmt` + `clippy` + `cargo test --workspace
--lib` + `verify_anchors.sh` 22/22 + cockpit-smoke + bench p99 +
idle-CPU floor + paper-mode probe per Q1.)

## Notes

- Predecessor: `ui-rethink-phase-d-trail v0.1.0` (shipped 2026-05-20).
  All of Phase D's surfaces are byte-identical-preserved here.
- Carry-forward ADR: `0031-audit-tick-consumer-envelope.md`
  § "Phase D amendment (2026-05-20)". No new ADR required at
  v0.1.1.
- The trail-mirror's `TrailMirrorHandle` is already constructed
  and `tokio::spawn`-ed at `crates/agent/src/main.rs:180-185`;
  this brief only wires it into the iced Subscription.
- The 3 new snapshot baselines are NEW files; the 22 anchored
  body-SHAs are unaffected.

## Changelog

- 2026-05-20 (analyst): T-A1..T-A9 ticked. M0 analyst-synthesis
  complete; M-OD (operator-decide) and M-T1 (architect-decide)
  rows enumerated awaiting their respective passes. Hard blocker
  is Q1 (BS-1 checkpoint + paper-mode feed availability) — must
  be operator-decided before R5 / T-F7 scope locks. Status:
  `draft`; awaiting operator-decide → architect M-T1.
- 2026-05-20 (architect M-T1): T-T1-1..T-T1-6 ticked.
  Resolutions: Q2=(b) UI-local wrapper enum; Q3=(c) construct
  TrailMirrorHandle in `cockpit_live.rs` bootstrap, store on
  `AppState`; Q4=(a) macOS `top` sampler. Spike requirement
  NONE — predecessor's K5 spike closed all structural unknowns.
  Anchor gate re-verified PASS (22/22) before handoff. Decomp
  artifact: `spec/ui-rethink-phase-d-trail-followup/decomp.md`
  (Waves A→E, 19 T-D-N rows, change-map at § 2, rollback at § 5).
  `reflection` joins `ui` deps as `optional = true`, gated behind
  the existing `live` feature — same shape as `agent`/`audit`.
  Zero new architecture edges in the data-flow-doc sense
  (ADR-0031 carry-forward "follow-up adds zero new architecture
  edges" honored). Status: `in-progress`; owner: `developer`.
  Trace row REQ-UI-RETHINK-PHASE-D-FOLLOWUP-001 flipped
  proposed → in-progress; decomp.md appended to its `arch`
  array. Awaiting developer T-D-N1.
