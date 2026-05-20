---
slug: ui-rethink-phase-f-memory-models-assistant
status: proposed
owner: architect
updated: 2026-05-20
---

# Tasks — UI rethink Phase F (Memory + Models + Phase-6 Assistant slot)

> Analyst M0 ordered checklist. Architect M-T1 decomposition adds
> T-T1-* rows; developer T-D-N* rows append once the architect locks
> the decomp. **Per project convention, this file at analyst hand-off
> carries only the T-A* rows; T-T* / T-D-N* are appended in M-T1 by
> the architect.** Pointers: [feature.md](feature.md) carries R1-R8,
> Q1-Q8, K1-K8, H1-H6. Scope source-of-truth:
> [dev-note](../dev-notes/ui-rethink-2026-05-17.md) §6 Phase F
> (lines 1098-1112), §J7 (lines 561-595, reflection memory), §J8
> (lines 596-637, model versions), §6 Phase ordering (lines 1114-1140).
> This is the **sixth and final** concrete phase of the UI rethink;
> presenter sweep per dev-note §6 line 1110 ("Final sweep — anything
> missing?") closes the rethink unless operator review surfaces a
> J9+ gap.

## M0 — Analyst synthesis

- [x] T-A1 — **Read dev-note source-of-truth.** §6 Phase F (scope
  source-of-truth, lines 1098-1112) + §J7 (Inspect the reflection
  memory — operator job-story, lines 561-595) + §J8 (Inspect a
  model version — operator job-story, lines 596-637) + §6 Phase
  ordering summary (lines 1114-1140 — confirms Phase F is "No
  cliffs … independently shippable and independently reversible").
  _Acceptance: feature.md "Why" + "Requirements" anchored to
  dev-note line numbers; no silent scope drift._

- [x] T-A2 — **Predecessor surface audit (sidebar + placeholders).**
  Confirm Phase C sidebar IA reserves `Screen::Memory` +
  `Screen::Models` in `SIDEBAR_GROUPS_PHASE_C` Library zone
  (`crates/ui/src/theme.rs:741-750`). Confirm `screens::memory` +
  `screens::models` route to `placeholder::view` at
  `crates/ui/src/shell.rs:98-99`. Confirm `strings::MEMORY_PLACEHOLDER`
  (`strings.rs:258-259`) + `strings::MODELS_PLACEHOLDER`
  (`strings.rs:260-261`) + `strings::SIDEBAR_NAV_MEMORY`
  (`strings.rs:272-273`) + `strings::SIDEBAR_NAV_MODELS`
  (`strings.rs:274-275`) already exist.
  _Acceptance: R1.1 / R1.3 / R2.1 / R2.3 cite the existing sidebar +
  placeholder wiring; no Phase A/C body change required._

- [x] T-A3 — **Right-rail Assistant slot reservation audit (Phase 2
  carry-forward).** Confirm `RIGHT_RAIL_WIDTH_PX = 0.0` constant at
  `crates/ui/src/theme.rs:640-643` ("Phase 2 — right-rail Phase 6
  Assistant slot reservation. The shell renders this column with
  `Length::Fixed(0.0)` until the v2-LLM Assistant ships in Phase
  6"). Confirm shell composition at `crates/ui/src/shell.rs:47-49`
  uses the reservation as the right `Container` of the
  `Row[sidebar | centre | right_rail]` shell layout. Confirm Phase
  D trail-drawer references the same constant at
  `crates/ui/src/widgets/trail_drawer.rs:70,175,179` (K6 coupling).
  _Acceptance: R3.1-R3.4 cite the existing right-rail wiring;
  K6 surfaced as the load-bearing cross-feature trap (Phase D
  trail-drawer + Phase F Assistant slot both reference the
  constant)._

- [x] T-A4 — **v2 LLM ship-status check (Q4 grounding).** Confirm
  `v2-llm-strategy v2.0.0` shipped 2026-05-13 — backlog.md:1207-1213.
  Operator-approved via presenter deck at
  `spec/v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md`.
  Wake condition for Lumen Phase 6 Assistant slot ("v2 LLM ships")
  is **met**. Decision shifts from "wait for v2 LLM" to "how much
  of the slot to wake at v0.1.0" — Q4 surfaces (a) stub / (b)
  minimal wire / (c) defer.
  _Acceptance: Q4 in feature.md grounds the recommended default
  in the v2 LLM ship status; backlog reference cited._

- [x] T-A5 — **v2.5 TCN checkpoint presence check (Q3 grounding).**
  Confirm checkpoints on disk at 2026-05-20:
  `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…safetensors`
  + `.metadata.json`, `tcn-bs2-3fabcabe…safetensors` + `.metadata.json`.
  Confirm anchor locks at `spec/anchors.toml:156-161` —
  `forecast-distribution-bs1-realdata` +
  `forecast-distribution-bs2-realdata` under version
  `v2.6.0-realdata`. Dev-note §1172-1177 Q5 investigation already
  resolved ("multi-source, all on disk already") — this brief
  inherits the resolution.
  _Acceptance: feature.md "Why" item 2 cites the checkpoint paths
  + anchor names; Models screen has a real payload at ship; Q3
  default (a) chosen for the fresh-checkout edge case._

- [x] T-A6 — **Reflection-memory crate read-path audit (Q8
  grounding + K1 surfacing).** Confirm `ReflectionStore` trait at
  `crates/reflection/src/store/mod.rs:27-35` exposes only
  `upsert(card) / top_k(query, k) / count()` — there is no
  `list_all` or `list_recent_n` method. The Memory screen at
  R1.2 needs N recent cards (reverse-chronological per Q1=a),
  which requires a new read API. Q8 surfaces three placement
  options: (a) extend the trait / (b) direct SQL in UI module /
  (c) impl-only method on SqliteReflectionStore.
  _Acceptance: K1 + Q8 surfaced in feature.md with analyst-
  recommended (b) — direct SQL keeps the trait surface minimal._

- [x] T-A7 — **Lab seeding / Trail back-link contract audit (R6).**
  Confirm `Message::OpenTrailFor(audit_id)` already exists from
  Phase D (`crates/ui/src/state.rs` — pattern referenced by
  Phase E `OpenLabFromCompare` predecessor). Memory→Trail cross-
  link (Q6=c, analyst-recommended) reuses `OpenTrailFor` directly;
  no new compound dispatch needed. Trail→Memory back-link (Q6=a)
  would touch Phase D body (R7.2 surface stability concern) and is
  rejected in favor of (c).
  _Acceptance: R6.1-R6.3 cite the existing `OpenTrailFor`
  message + frame the Q6 options + recommend (c) additive-only._

- [x] T-A8 — **Anchor-risk pre-flight.** Confirm Phase F touches
  no strategy / audit / exec / report-renderer code; the Memory
  screen reads `lesson_cards` (reflection writer is the producer);
  the Models screen reads checkpoint metadata.json (v2.5 TCN
  training loop is the producer); the Assistant slot at Q4=(a)
  renders static placeholder copy. 22-anchor regression gate
  carry-forward; H2 from Phase D+ → Phase E predecessor applies
  verbatim.
  _Acceptance: R7.1-R7.7 enumerate the 8-item non-regression
  contract; "anchor risk zero" claim defended by construction._

- [x] T-A9 — **Surface Q1-Q8 with recommended defaults** for
  operator-decide:
  - Q1 Memory screen body shape (analyst-recommended: a —
    reverse-chronological list per dev-note §J7 line 569)
  - Q2 Models screen body shape (analyst-recommended: a — flat
    list per dev-note §J8 line 604)
  - Q3 Models screen when no checkpoints (analyst-recommended:
    a — honest placeholder; preserves Phase C IA invariant)
  - Q4 Assistant slot v0 behavior (analyst-recommended: a —
    stub-only wake; honest light without LLM plumbing scope-creep)
  - Q5 Memory entry detail view (analyst-recommended: b — side
    drawer per Phase D precedent; fallback to c if K4 conflict)
  - Q6 Cross-link surfaces (analyst-recommended: c — Memory →
    Trail back-link only; additive, no Phase D body churn)
  - Q7 Models "serving" pill semantics (analyst-recommended:
    c — all "staged" at v0.1.0; lifecycle in v0.2.0)
  - Q8 Memory `list_recent` read API placement (analyst-
    recommended: b — direct SQL in UI module, bypass trait)
  _Acceptance: feature.md "Q-questions (operator-decide)" section
  carries 8 entries each with recommendation + rationale + alt
  options. Q4 carries v2 LLM ship-status grounding._

- [x] T-A10 — **Author K1-K8 risk register.** K4 (Assistant-slot +
  Memory-drawer right-side coexistence) and K6 (RIGHT_RAIL_WIDTH_PX
  constant semantic change — Phase D trail-drawer cross-feature
  coupling) surfaced as the load-bearing cross-feature traps; both
  with analyst-recommended fallback paths (K4 → Q5=c route
  fallback; K6 → Option A additive constant
  `RIGHT_RAIL_OPEN_WIDTH_PX`).
  _Acceptance: feature.md K-section carries 8 entries each with
  severity + mitigation; K4 + K6 each name a concrete fallback
  the architect can lock at M-T1._

- [x] T-A11 — **Author H1-H6 hypothesis register.** Each
  hypothesis must be falsifiable by a named test or measurement:
  - H1 Memory cold-boot read < 50 ms p99 (architect M-T1 enumerates
    `lesson_cards` row count + static argument)
  - H2 Models cold-boot scan < 50 ms p99 (architect M-T1 micro-
    bench against live `crates/forecast/checkpoints/anchors/`)
  - H3 idle-CPU floor ≤ 13.6 % preserved (tester runs cockpit-
    performance v1.0.0 across all 3 new active states)
  - H4 `list_recent_lesson_cards` query correctness (unit test
    with 5 fixture rows)
  - H5 checkpoint metadata schema robustness (unit test with
    full / missing-field / malformed fixtures)
  - H6 right-rail layout invariant under Assistant slot wake
    (proptest 256 cases × {open, closed} = 512 cases)
  _Acceptance: feature.md H-section carries 6 entries; each names
  a falsification path._

- [x] T-A12 — **Author acceptance criteria per milestone** (M0,
  M-OD, M-T1, M-FINAL, M-PRESENTER). M-FINAL includes new snapshot
  baselines: `memory__cold_boot_empty`, `memory__steady_state_5_cards`,
  `memory__drawer_open_on_card_click` (if Q5=b),
  `models__cold_boot_no_checkpoints`, `models__steady_state_2_checkpoints`,
  `assistant_slot__closed_default`, `assistant_slot__open_stub` (if
  Q4=a). M-PRESENTER includes the dev-note §6 line 1110 "anything
  missing?" final sweep per K8 mitigation.
  _Acceptance: feature.md "Acceptance criteria" section structured
  per Phase D / D+ / E precedent + new M-PRESENTER milestone._

- [x] T-A13 — **Open trace row `REQ-UI-RETHINK-PHASE-F-001`** in
  `draft` state. `arch` / `crates` / `tests` / `anchors` columns
  partially filled by analyst (arch = feature.md + tasks.md +
  dev-note pointer; crates = `crates/ui`; tests = empty for
  developer / tester); state = "draft".
  _Acceptance: trace.toml carries the new row immediately after
  REQ-UI-RETHINK-PHASE-E-001 with title quoting the dev-note §6
  Phase F scope + the three deliverables + the v2 LLM ship grounding._

- [x] T-A14 — **Promote backlog entry.** Add
  `ui-rethink-phase-f-memory-models-assistant` to
  `spec/backlog.md` "Active" section directly above
  `v25-tcn-alpha-investigation`, mirroring the Phase E predecessor
  entry format. Carry the v0.1.0 / predecessor / Q1-Q8 / K4 + K6
  load-bearing traps / cost callouts from feature.md. Add the
  analyst-pass changelog comment at the top of backlog.md per
  the Phase E predecessor pattern.
  _Acceptance: backlog.md "Active" section carries the new row
  immediately above `v25-tcn-alpha-investigation`; format
  consistent with the Phase E predecessor entry; analyst-pass
  comment at top of file._

- [x] T-A15 — **Emit analyst HANDOFF envelope** per AGENT.md
  communication contract (`from = "analyst", to = "operator",
  verdict = "READY-FOR-OPERATOR-DECIDE"`). Lists spec files
  written (4: feature.md, tasks.md, trace.toml row, backlog.md
  Active entry) + Q1-Q8 that need operator input + assumptions /
  recommended defaults + v2 LLM ship-status check result (Q4
  grounding).
  _Acceptance: handoff envelope appended to assistant response;
  trace_refs include `REQ-UI-RETHINK-PHASE-F-001`._

## M-OD — Operator-decide (Q1-Q8) — resolved 2026-05-20

> All eight analyst-recommended defaults accepted in one tick via the
> operator's standing "Autoapprove all" directive (confirmed
> 2026-05-20 against the analyst hand-off envelope).

- [x] T-OD1 — Q1 = (a) reverse-chronological Memory list.
- [x] T-OD2 — Q2 = (a) flat checkpoint list for Models.
- [x] T-OD3 — Q3 = (a) honest "no models loaded" placeholder when the
  checkpoint dir is empty (preserves Phase C sidebar IA invariant).
- [x] T-OD4 — Q4 = (a) stub-only Assistant slot wake (Phase 6 Lumen
  slot lights structurally; v2 LLM text-stream wire deferred to v0.2.0
  / Phase F.1). v2-llm-strategy v2.0.0 SHIPPED 2026-05-13 so the wake
  condition is structurally met.
- [x] T-OD5 — Q5 = (b) side drawer for Memory entry detail (mirrors
  Phase D Trail drawer precedent). Architect M-T1 to confirm K4 right-
  side coexistence with the Assistant slot; fallback (c) Screen route
  if conflict is material.
- [x] T-OD6 — Q6 = (c) Memory→Trail back-link only (additive; no
  Phase D body touch).
- [x] T-OD7 — Q7 = (c) all checkpoints render as "staged" at v0.1.0;
  serving-status lifecycle defer to v0.2.0.
- [x] T-OD8 — Q8 = (b) direct SQL in the UI compare module; keep
  `ReflectionStore` trait surface minimal at v0.1.0 (don't extend with
  `list_recent_n`).

## M-T1 — Architect decomposition

> Architect spawns after M-OD closes. Resolves K1 (reflection read
> API), K2 (checkpoint metadata schema), K3 (sparkline data
> presence), K4 (Assistant-slot/Memory-drawer coexistence), K6
> (RIGHT_RAIL constant semantic), and enumerates H1 + H2 against
> the live repo. Authors `decomp.md` per Phase E precedent.
> Decomposes R1-R8 into ordered T-D-N tasks per the suggested
> wave map below; full T-D-N* checklist appended by architect to
> this tasks.md.

- [ ] T-T1-1 — K1 resolution: lock the `list_recent_lesson_cards`
  shape per Q8 operator-decide. If Q8=(b), document the direct-SQL
  query inside `crates/ui/src/memory/store_read.rs` in `decomp.md
  § 1.1`. Validate against `crates/reflection/migrations/001_lesson_cards.sql`
  schema.
- [ ] T-T1-2 — K2 resolution: inventory the
  `crates/forecast/checkpoints/anchors/*.metadata.json` schema by
  parsing the live `tcn-bs1-*.metadata.json` + `tcn-bs2-*.metadata.json`
  files. Lock a serde struct shape with `#[serde(default)]` on
  every non-load-bearing field. Document in `decomp.md § 1.2`.
- [ ] T-T1-3 — K3 resolution: grep `crates/replay-cache/` for the
  `"forecast"` namespace. If populated, sparkline ships at v0.1.0;
  if absent, deferred to v0.2.0 per R2.2 framing. Document the
  finding in `decomp.md § 1.3`.
- [ ] T-T1-4 — K4 + Q5 resolution: lock the Memory-drawer (Q5=b)
  vs Assistant-slot (Q4=a) right-side coexistence rule. Analyst-
  recommended fallback: if conflict material, fall back to Q5=(c)
  (`Screen::MemoryEntry` route — no drawer). Document in
  `decomp.md § 1.4`.
- [ ] T-T1-5 — K6 resolution: lock Option A
  (`RIGHT_RAIL_OPEN_WIDTH_PX` additive constant; preserve
  `RIGHT_RAIL_WIDTH_PX = 0.0` as the closed-state default).
  Verify Phase D trail-drawer references at
  `crates/ui/src/widgets/trail_drawer.rs:70,175,179` continue
  using the unchanged closed-state value. Document in `decomp.md
  § 1.5`.
- [ ] T-T1-6 — H1 enumeration: count `lesson_cards` rows in the
  operator's live `reflection.db` (or the cockpit's connected
  store path); validate < 500 budget per
  `crates/reflection/src/store/sqlite.rs:5-6` annotation. Record
  in `decomp.md § 1.6`.
- [ ] T-T1-7 — H2 enumeration: micro-bench the checkpoint metadata
  scan against the live
  `crates/forecast/checkpoints/anchors/` (2 files at 2026-05-20);
  static argument suffices if files are small. Record in `decomp.md
  § 1.7`.
- [ ] T-T1-8 — Anchor gate confirmed:
  `bash scripts/verify_anchors.sh` returns
  `ANCHORS PASS  (22 / 22)` BEFORE the M-T1 pass closes. R7.1
  carry-forward from predecessor `ui-rethink-phase-e-compare
  v0.1.0` confirmed clean.
- [ ] T-T1-9 — Wave shape locked. Suggested 7 waves (A through G):
  - Wave A — state modules + Message variants (R4, R8.1)
  - Wave B — read modules (R5.1 memory store_read, R5.2 models
    registry_read)
  - Wave C — `screens::memory` + shell wiring (R1)
  - Wave D — `screens::models` + shell wiring (R2)
  - Wave E — `assistant` slot wake + shell wiring (R3) — skip if
    Q4=(c)
  - Wave F — 5-7 snapshot baselines + layout-invariants proptest
    cases + cockpit-smoke pre-run (R7.3, H6)
  - Wave G — anchor gate + spec-lint sweep + tester handoff
    envelope (R7.1, R7.5)
  Architect locks the final wave map + net-new file count (R8.5
  — analyst estimate 8-10) + per-wave T-D-N row count in
  `decomp.md § 2 / § 3`.

## M-FINAL — Tester sweep

> Tester spawns after developer Wave G closes. Runs the full
> M-FINAL acceptance criteria per feature.md.

- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D
      warnings` exit 0.
- [ ] `cargo test --workspace --lib` 100 % PASS.
- [ ] 5-7 new snapshot baselines accepted (per Q4 + Q5 outcomes).
- [ ] `scripts/verify_anchors.sh` → 22 / 22 PASS (R7.1).
- [ ] `cockpit-smoke` → 0 panic lines on Memory + Models +
      Assistant-open active screens (R7.3).
- [ ] Cockpit-performance v1.0.0 idle-CPU floor ≤ 13.6 % preserved
      under each new active screen (R7.4, H3).
- [ ] H1 + H2 cold-boot read benchmarks recorded.
- [ ] H4 unit test (`list_recent_lesson_cards_returns_n_recent`)
      PASS.
- [ ] H5 unit test (`discover_checkpoints_tolerates_schema_drift`)
      PASS.
- [ ] H6 layout-invariants proptest
      (`assistant_slot_open_no_zero_dim`) PASS — 512 cases
      (256 viewport × {open, closed}).
- [ ] Author `reports/test-final-<YYYY-MM-DD>.md` per
      `.claude/skills/rust-test/templates/test-report.md`.

## M-PRESENTER — Final sweep ("anything missing?")

> Presenter spawns after tester VERDICT → PASS. Per dev-note §6
> line 1110, Phase F is the final "anything missing?" sweep —
> presenter deck explicitly enumerates the rethink's 6 phases (A-F)
> and 8 dev-note job-stories (J1-J8) for operator gap-review.

- [ ] Presenter deck enumerates J1-J8 (dev-note §3 IA + §J1-J8
      job-stories) and maps each to the phase / screen that
      shipped it.
- [ ] Presenter deck enumerates phases A-F and confirms each is
      byte-identical at ship (anchor gate carry-forward).
- [ ] Presenter prompts operator on "anything missing?" — any gap
      surfaced becomes a `ui-rethink-phase-f-<gap-slug>` or
      `ui-rethink-phase-g-<new-job>` follow-up brief (K8
      mitigation).
- [ ] Operator-approval via "Autoapprove all" pattern OR explicit
      sign-off on the missing-coverage prompt.

## Notes

- **Analyst hand-off shape**: this tasks.md carries only M0 T-A*
  rows + M-OD / M-T1 / M-FINAL / M-PRESENTER placeholders. The
  architect's M-T1 pass appends T-T1-* + waves A-G with T-D-N* rows.
  Developer must not pull T-D-N rows before architect locks.
- **Predecessor reference**: Phase E's tasks.md
  (`spec/ui-rethink-phase-e-compare/tasks.md`) is the structural
  template; Phase F follows it 1:1 with one additional milestone
  (M-PRESENTER) per the dev-note §6 Phase F "final sweep" framing.
- **No cliffs.** Per dev-note §6 line 1134, Phase F is reversible
  via individual screen-body reverts +
  `Cockpit::{memory,models,assistant}_screen_state` field removals.
  Each of the three deliverables (Memory / Models / Assistant slot)
  is **independently shippable inside Phase F** if the operator's
  M-OD outcomes split scope (e.g. Q4=(c) defers Assistant slot
  entirely → Phase F ships only Memory + Models).
- **Final phase of the rethink.** Per dev-note §6 line 1098 and
  the §6 Phase ordering summary (lines 1114-1124), Phase F is the
  sixth and final concrete phase. Operator-review at M-PRESENTER
  closes the rethink unless a J9+ gap surfaces (K8).
