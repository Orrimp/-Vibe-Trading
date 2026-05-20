---
slug: ui-rethink-phase-d-trail-followup
status: proposed
owner: architect
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

## M-T1 — Architect decomposition (next, after operator-decide)

Architect-decide questions resolved at M-T1:

- [ ] T-T1-1 — Q2: `TrailMirrorTick` payload shape across the
  crate boundary. Default architect-pick: (a) direct re-export
  if `ui → reflection` edge is permitted; else (b) UI-local
  wrapper struct.
- [ ] T-T1-2 — Q3: `TrailMirrorHandle` plumbing path. Default
  architect-pick: (a) field on `Cockpit` struct.
- [ ] T-T1-3 — Q4: idle-CPU bench tooling. Default architect-pick:
  (a) macOS `top -l 1 -n 0 -pid <pid> -stats cpu`, 1 Hz × 60
  samples × N=3 runs.
- [ ] T-T1-4 — Decompose R1-R5 into ordered T-D-N rows. Anticipated
  Waves:
  - **Wave A** — payload + Recipe (R1.1, R1.2). Renames the UI-
    side `Message::TrailMirrorTick` payload; lands the new
    `trail_mirror_subscription` Recipe in `crates/ui/src/live.rs`
    (or sibling).
  - **Wave B** — Cockpit wiring (R1.3, R1.4, R1.5). Threads the
    `TrailMirrorHandle` into `Cockpit::new` and the cockpit-binary
    bootstrap; wires the Subscription into `Cockpit::subscription`
    batch; extends `trail_screen_state` with the two new fields.
  - **Wave C** — Snapshots (R2.1-R2.3). Authors 3 new fixtures in
    `crates/ui/tests/visual_snapshots.rs` (or wherever the host
    stores baselines); commits the 3 baselines after a
    `cargo insta accept` pass.
  - **Wave D** — Bench (R3.1-R3.4). Creates
    `crates/reflection/benches/trail_mirror.rs`; seeds 10⁵-row
    fixture; asserts p99 < 50 ms.
  - **Wave E** — Paper-mode probe (R5). Tester-driven; gated by
    Q1 resolution. If Q1=yes, tester runs the 60-s smoke;
    structural-only re-verify if Q1=no.

- [ ] T-T1-5 — Confirm `ui → reflection` edge is permitted per
  `spec/architecture/01-data-flow.md` edge invariants. If not,
  fall back to Q2 (b) UI-local wrapper struct.

- [ ] T-T1-6 — Spec-lint clean (deferred to M-FINAL tester sweep).

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
