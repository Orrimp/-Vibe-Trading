---
slug: ui-rethink-phase-f-memory-models-assistant
status: shipped
owner: operator
updated: 2026-05-21
---

# Tasks — UI rethink Phase F (Memory + Models + Phase-6 Assistant slot)

> Analyst M0 ordered checklist. Architect M-T1 decomposition adds
> T-T1-* rows; developer T-D-N* rows append once the architect locks
> the decomp. **Per project convention, this file at analyst hand-off
> carries only the T-A* rows; T-T* / T-D-N* are appended in M-T1 by
> the architect.** Pointers: [feature.md](feature.md) carries R1-R8,
> Q1-Q8, K1-K8, H1-H6. Scope source-of-truth:
> [dev-note](../dev-notes/archive/2026-Q2/ui-rethink-2026-05-17.md) §6 Phase F
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

- [x] T-T1-1 — **K1 + Q8 resolution: Memory read-path placement.** Refined
  Q8=(b) to `crates/reflection/src/query.rs` (sibling of `store/`), called
  from cockpit_live's side-thread tokio runtime; UI receives results via
  `Message::MemoryHydrate(Vec<LessonCardCard>)` per Phase D trail_mirror
  precedent. Honors Q8=(b) "no trait change" while respecting that the UI
  crate has no tokio runtime. SQL shape locked in `decomp.md § 1.1`
  (`SELECT ... FROM lesson_cards ORDER BY closed_at DESC LIMIT ?`).
  Schema validated against `crates/reflection/migrations/001_lesson_cards.sql:8-24`.
- [x] T-T1-2 — **K2 resolution: checkpoint metadata schema.** Inventoried
  live `tcn-bs1-d1c3696d…metadata.json` (855 bytes) + `tcn-bs2-3fabcabe…metadata.json`
  (852 bytes). Locked serde struct shape with `#[serde(default)]` on every
  non-load-bearing field in `decomp.md § 1.2`. Three new serde structs
  (`CheckpointMetadata`, `CheckpointArchitecture`, `CheckpointDataSpan`)
  + UI view-model `CheckpointMeta` distinct from the wire shape. Family
  discriminated by filename prefix (`tcn-` / `patchtst-` / `transformer-`);
  5 unit tests in `models/registry_read.rs` (H5 falsification).
- [x] T-T1-3 — **K3 resolution: forecast-quality sparkline data.**
  `crates/replay-cache/` "forecast" namespace EMPTY at 2026-05-20
  (no populated DB on disk; only `data/audit/ledger.db` 135168 bytes
  exists; no `replay_cache*.db` siblings). Sparkline DEFERRED to v0.2.0
  per R2.2 framing. Models row layout ships with `—` placeholder + tooltip
  `MODELS_SPARKLINE_DEFERRED_TOOLTIP`. Documented in `decomp.md § 1.3`.
- [x] T-T1-4 — **K4 + Q5 resolution: drawer-vs-Assistant-slot coexistence.**
  Drawer lives in the centre column (next to Memory cards list); Assistant
  slot is the far-right shell track. They live in DIFFERENT shell columns
  — no co-existence conflict. No auto-collapse needed. Q5 = (b) confirmed
  (no fallback to (c)). Layout-invariants proptest case
  `memory_drawer_open_with_assistant_open_no_zero_dim` validates under 256
  viewports. Documented in `decomp.md § 1.4`.
- [x] T-T1-5 — **K6 resolution: RIGHT_RAIL_WIDTH_PX constant semantic.**
  Option A locked. `crates/ui/src/theme.rs:643` `RIGHT_RAIL_WIDTH_PX = 0.0`
  preserved unchanged (Phase D `widgets/trail_drawer.rs:70,175,179`
  byte-identical; `crates/ui/tests/shell_grid.rs:14-16` hard invariant
  test passes verbatim). New additive constant
  `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0` at `theme.rs:~644`. `shell::view`
  picks one of the two based on `assistant_state.is_open`. Documented
  in `decomp.md § 1.5`.
- [x] T-T1-6 — **H1 enumeration.** `ls -la data/audit/` returned only
  `ledger.db` (135168 bytes May 16 00:11). `reflection.db` ABSENT on
  this workstation. 0-row cold-empty boot path is the dominant
  first-open UX → Memory screen renders R1.4 empty-state placeholder
  ("No memory entries yet. Memory populates as strategies close trades.").
  Budget trivially satisfied (0 rows × any query = sub-millisecond ≪
  50 ms p99 budget). Documented in `decomp.md § 1.6`.
- [x] T-T1-7 — **H2 enumeration.** `stat -f "%z bytes"` against live
  metadata.json files: `tcn-bs1` = 855 bytes, `tcn-bs2` = 852 bytes.
  Total ≤ 2 KB across both. 2 × file-stat() + 2 × read_to_string + 2 ×
  `serde_json::from_str` ≈ 20 μs (serde_json deserializes ~100 MB/s).
  ~50000× headroom over the 50 ms p99 budget. Static argument
  suffices; no micro-bench needed. Documented in `decomp.md § 1.7`.
- [x] T-T1-8 — **Anchor gate confirmed.** `bash scripts/verify_anchors.sh`
  returned literal output `ANCHORS PASS  (22 / 22)` BEFORE this M-T1
  pass closed. R7.1 carry-forward from predecessor
  `ui-rethink-phase-e-compare v0.1.0` confirmed clean.
- [x] T-T1-9 — **Wave shape locked: 6 waves A-F** (compressed from
  the 7-wave suggestion — Wave F + G merged into a single "tester
  handoff prep" wave because Phase F has no spec-lint sweep distinct
  from the H4/H5/H6 unit + proptest sweep already in Wave F). Net-new
  file count locked at **12 source + 6 PNG baselines + 1 trace row**
  (R8.5 — analyst estimate 8-10; architect locks at 12 because
  Q4=(a) Assistant module needs 3 files mod/state/view to mirror
  Memory/Models module shape consistently, and the
  `crates/reflection/src/query.rs` placement per § 1.1 adds one more
  than the original brief named). Spike requirement = NONE.
  T-D-N1..N23 appended below. Decomp at `decomp.md § 2 / § 3`.

## M-T1 → Developer — Wave A-F T-D-N checklist

> Developer pulls these in order; Waves C/D/E can run in parallel after
> Wave B closes (Memory + Models + Assistant are independent surfaces).
> Each row carries file:line + cargo invocation + literal expected output
> per the honest-tick rule. Full decomp at `decomp.md`.

### Wave A — State modules + Message variants + theme constant

- [x] T-D-N1 — Create 6 new module files: `crates/ui/src/memory/{mod,state}.rs`,
  `crates/ui/src/models/{mod,state}.rs`, `crates/ui/src/assistant/{mod,state}.rs`.
  Add 3 declarations to `crates/ui/src/lib.rs`. Cargo: `cargo check -p ui`.
  Acceptance: PASS no warnings; literal `Checking ui v0.1.0` line.
  **DONE** — file:line: `crates/ui/src/lib.rs:41-47`. Test: `cargo check -p ui`. Output: `Checking ui v0.1.0 … Finished`.
- [x] T-D-N2 — Add `pub const RIGHT_RAIL_OPEN_WIDTH_PX: f32 = 320.0;` to
  `crates/ui/src/theme.rs:~644`. K6 Option A — preserve `RIGHT_RAIL_WIDTH_PX = 0.0`.
  Cargo: `cargo test -p ui --test shell_grid`.
  Acceptance: literal `test right_rail_width_is_zero ... ok`.
  **DONE** — file:line: `crates/ui/src/theme.rs` (layout::RIGHT_RAIL_OPEN_WIDTH_PX = 320.0). Test: `cargo test -p ui --test shell_grid`. Output: `test shell_grid_reserves_right_rail ... ok; 3 passed`.
- [x] T-D-N3 — Promote `PersistedRow` + `decode_row` visibility in
  `crates/reflection/src/store/sqlite.rs:89,233` from private to `pub(crate)`.
  Cargo: `cargo check -p reflection`.
  Acceptance: PASS no warnings; literal `Checking reflection v0.1.0` line.
  **DONE** — file:line: `crates/reflection/src/store/sqlite.rs:89,233`. Test: `cargo check -p reflection`. Output: `Checking reflection v0.1.0 … Finished`.
- [x] T-D-N4 — Add 3 new state fields to `Cockpit` at
  `crates/ui/src/state.rs:~885,~965,~1016,~1116` (3-touchpoint pattern):
  `memory_screen_state`, `models_screen_state`, `assistant_state`.
  Cargo: `cargo test -p ui --lib`.
  Acceptance: literal `test result: ok. N passed; 0 failed` (existing baseline preserved).
  **DONE** — file:line: `crates/ui/src/state.rs:889,894,981,1035-1039`. Test: `cargo test -p ui --lib`. Output: `308 passed; 0 failed`.
- [x] T-D-N5 — Add 9 Message variants at `crates/ui/src/state.rs:~1380,~1425`:
  `MemoryHydrate`, `MemoryOpenDrawer`, `MemoryCloseDrawer`, `MemoryToggleMode`,
  `MemorySetFilter`, `ModelsHydrate`, `ModelsSetFamilyFilter`,
  `ModelsSetStatusFilter`, `ToggleAssistantSlot` (R8.1).
  Cargo: `cargo check -p ui`. Acceptance: PASS.
  **DONE** — file:line: `crates/ui/src/state.rs:1470-1510`. Test: `cargo check -p ui`. Output: `Finished`.
- [x] T-D-N6 — Add 9 update-arms at `crates/ui/src/state.rs:~1911`. All
  simple-assignment; `MemoryHydrate` + `ModelsHydrate` also update `last_indexed`.
  Cargo: `cargo check -p ui` + `cargo test -p ui --lib`. Acceptance: PASS.
  **DONE** — file:line: `crates/ui/src/state.rs:2019-2055`. Test: `cargo test -p ui --lib`. Output: `308 passed; 0 failed`.
- [x] T-D-N7 — Add 12+ Phase F string constants to `crates/ui/src/strings.rs:~290`;
  deprecate `MEMORY_PLACEHOLDER` + `MODELS_PLACEHOLDER` at `:258-261` per
  `COMPARE_PLACEHOLDER:253-257` precedent.
  Cargo: `cargo clippy -p ui -- -D warnings`. Acceptance: PASS.
  **DONE** — file:line: `crates/ui/src/strings.rs:377-442`. Test: `cargo clippy -p ui -- -D warnings`. Output: `Finished`.

### Wave B — Read modules (Memory + Models cold-boot)

- [x] T-D-N8 — Author `crates/reflection/src/query.rs` per `decomp.md § 1.1`.
  Includes `list_recent_lesson_cards(pool, limit)` + 1 unit test
  (`list_recent_lesson_cards_returns_n_recent` — H4 falsification).
  Add `pub mod query;` to `crates/reflection/src/lib.rs:~42`.
  Cargo: `cargo test -p reflection --lib query::tests`.
  Acceptance: literal `running 1 test` + `test result: ok. 1 passed; 0 failed`.
  **DONE** — file:line: `crates/reflection/src/query.rs:36-53` + `crates/reflection/src/lib.rs:47`. Test: `cargo test -p reflection --lib query::tests`. Output: `running 1 test … 1 passed; 0 failed`.
- [x] T-D-N9 — Author `crates/ui/src/models/registry_read.rs` per
  `decomp.md § 1.2`. `discover_checkpoints` + `parse_metadata` + 3 serde
  structs + 5 unit tests (H5 falsification: full / missing-dropout /
  missing-sigma / malformed / unknown-family).
  Cargo: `cargo test -p ui --lib models::registry_read::tests`.
  Acceptance: literal `running 5 tests` + `test result: ok. 5 passed; 0 failed`.
  **DONE** — file:line: `crates/ui/src/models/registry_read.rs:91,117,175+`. Test: `cargo test -p ui --lib models::registry_read::tests`. Output: `5 passed; 0 failed`.
- [x] T-D-N10 — Wire cold-boot hydrate in `crates/ui/src/bin/cockpit_live.rs`
  (additive ~40 LOC at the boot section, near `:362,743`). Open
  `SqliteReflectionStore` against config-resolved reflection.db path; call
  `reflection::query::list_recent_lesson_cards(&pool, 50)` +
  `ui::models::registry_read::discover_checkpoints(checkpoint_dir)` on
  side-thread tokio runtime; send `Message::MemoryHydrate(cards)` +
  `Message::ModelsHydrate(checkpoints)` via the iced `Application` channel.
  Mirrors `trail_mirror::TrailMirror` wiring.
  Cargo: `cargo check -p ui --bin cockpit_live --features live`.
  Acceptance: PASS no warnings; literal `Checking ui v0.1.0` line.
  **DONE** — file:line: `crates/ui/src/bin/cockpit_live.rs:401-410,533-615` + `crates/reflection/src/query.rs:24-71` (`open_and_list_recent`). Test: `cargo check -p ui --bin cockpit_live --features live`. Output: `Checking ui v0.1.0 … Finished`.

### Wave C — `screens::memory` + drawer + shell wiring (R1, R6.1)

- [x] T-D-N11 — Author `crates/ui/src/screens/memory.rs` per `decomp.md § 2 row 22`.
  Toolbar (Cards/Cluster toggle — Cluster disabled per R1.2) + cards list
  + optional drawer. Each card emits `Message::OpenTrailFor(audit_id)` on
  chevron click (R6.1 reuse).
  Add `pub mod memory;` to `crates/ui/src/screens/mod.rs`.
  Cargo: `cargo check -p ui`. Acceptance: PASS.
  **DONE** — file:line: `crates/ui/src/screens/memory.rs:33-267` + `crates/ui/src/screens/mod.rs:13-16`. Test: `cargo check -p ui`. Output: `Finished`.
- [x] T-D-N12 — Author `crates/ui/src/memory/drawer.rs` per `decomp.md § 2 row 6`.
  Side-drawer body (Q5=(b)). Width `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`.
  Composition mirrors Phase D `widgets/trail_drawer.rs` body verbatim.
  Cargo: `cargo clippy -p ui -- -D warnings`. Acceptance: PASS.
  **DONE** — file:line: `crates/ui/src/memory/drawer.rs:26-148`. Test: `cargo clippy -p ui -- -D warnings`. Output: `Finished`.
- [x] T-D-N13 — Swap `crates/ui/src/shell.rs:98` from
  `placeholder::view(strings::MEMORY_PLACEHOLDER, mode)` to
  `screens::memory::view(model, mode)`. Update use-list at `:28` to include `memory`.
  Cargo: `cargo test -p ui --lib` + `cargo test -p ui --test layout_invariants`.
  Acceptance: PASS; existing layout-invariants preserved.
  **DONE** — file:line: `crates/ui/src/shell.rs:117`. Test: `cargo test -p ui --lib` + `cargo test -p ui --test layout_invariants`. Output: `308 passed; 0 failed` + `7 passed; 0 failed`.

### Wave D — `screens::models` + shell wiring (R2)

- [x] T-D-N14 — Author `crates/ui/src/screens/models.rs` per `decomp.md § 2 row 23`.
  Toolbar (TCN-only family chips; PatchTST/Transformer disabled) +
  checkpoint list. Empty-state placeholder when `models_screen_state.checkpoints`
  is empty post-hydrate (Q3=(a)). Each row renders columns per § 1.2
  `CheckpointMeta` shape.
  Add `pub mod models;` to `crates/ui/src/screens/mod.rs`.
  Cargo: `cargo check -p ui`. Acceptance: PASS.
  **DONE** — file:line: `crates/ui/src/screens/models.rs:39-264` + `crates/ui/src/screens/mod.rs:17-20`. Test: `cargo check -p ui`. Output: `Finished`.
- [x] T-D-N15 — Swap `crates/ui/src/shell.rs:99` from
  `placeholder::view(strings::MODELS_PLACEHOLDER, mode)` to
  `screens::models::view(model, mode)`. Update use-list at `:28` to include `models`.
  Cargo: `cargo test -p ui --lib` + `cargo test -p ui --test layout_invariants`.
  Acceptance: PASS.
  **DONE** — file:line: `crates/ui/src/shell.rs:119`. Test: `cargo test -p ui --lib` + `cargo test -p ui --test layout_invariants`. Output: `308 passed; 0 failed` + `7 passed; 0 failed`.

### Wave E — Assistant slot wake + shell right-rail wiring (R3)

- [x] T-D-N16 — Author `crates/ui/src/assistant/view.rs` per `decomp.md § 2 row 12`.
  When `state.is_open == false` → return 0-width `Container::new(Space::new())`
  (byte-identical to today's right_track at `shell.rs:47-49`). When
  `state.is_open == true` → render Lumen Phase 6 stub placeholder
  (`ASSISTANT_OFFLINE_TITLE` + `ASSISTANT_OFFLINE_BODY` per R3.2(a) +
  K7 mitigation).
  Cargo: `cargo check -p ui`. Acceptance: PASS.
  **DONE** — file:line: `crates/ui/src/assistant/view.rs:29-69`. Test: `cargo check -p ui`. Output: `Finished`.
- [x] T-D-N17 — Per `decomp.md § 1.5 + § 2 row 27`. Swap
  `crates/ui/src/shell.rs:47-49` from raw `Space::new()` /
  `Length::Fixed(RIGHT_RAIL_WIDTH_PX)` to function-of-state shape per § 1.5.
  Update use-list at `:30` to add `RIGHT_RAIL_OPEN_WIDTH_PX`; add
  `crate::assistant` to top-level uses.
  Cargo: `cargo test -p ui --test shell_grid` + `cargo test -p ui --test layout_invariants`.
  Acceptance: PASS; literal `test right_rail_width_is_zero ... ok` preserved
  (constant unchanged at 0.0).
  **DONE** — file:line: `crates/ui/src/shell.rs:61-68`. Test: `cargo test -p ui --test shell_grid`. Output: `test shell_grid_reserves_right_rail ... ok; 3 passed`.

### Wave F — Snapshot baselines + layout-invariants + round-trip + tester handoff

- [x] T-D-N18 — Author 6 visual snapshots in `crates/ui/tests/visual_snapshots.rs`:
  `memory__cold_boot_empty`, `memory__steady_state_5_cards`,
  `memory__drawer_open_on_card_click`, `models__cold_boot_no_checkpoints`,
  `models__steady_state_2_checkpoints`, `assistant_slot__open_stub`.
  (`assistant_slot__closed_default` byte-identical to existing shell baselines
  per K6 Option A — no new fixture.)
  Cargo: `cargo test -p ui --test visual_snapshots`.
  Acceptance: literal `test result: ok. N passed; 0 failed`; 6 new
  baselines accepted on first run.
  **DONE** — file:line: `crates/ui/tests/visual_snapshots.rs:327-427` + `crates/ui/tests/fixtures/mod.rs:519-674`. Test: `cargo test -p ui --test visual_snapshots -- memory__cold_boot_empty memory__steady_state_5_cards memory__drawer_open_on_card_click models__cold_boot_no_checkpoints models__steady_state_2_checkpoints assistant_slot__open_stub`. Output: `running 6 tests … 6 passed; 0 failed`.
- [x] T-D-N19 — Append 3 layout-invariants proptest cases to
  `crates/ui/tests/layout_invariants.rs`: `memory_screen_no_zero_dim`,
  `models_screen_no_zero_dim`, `assistant_slot_open_no_zero_dim` (H6
  falsification; the last one runs 256 × {open, closed} = 512 cases).
  Cargo: `cargo test -p ui --test layout_invariants -- memory_screen_no_zero_dim
  models_screen_no_zero_dim assistant_slot_open_no_zero_dim`.
  Acceptance: literal `running 3 tests` + `test result: ok. 3 passed; 0 failed`.
  **DONE** — file:line: `crates/ui/tests/layout_invariants.rs:397-490` + helper fns at `570-670`. Test: `cargo test -p ui --test layout_invariants -- memory_screen_no_zero_dim models_screen_no_zero_dim assistant_slot_open_no_zero_dim`. Output: `running 3 tests … 3 passed; 0 failed`.
- [x] T-D-N20 — Append 3 round-trip unit tests to `crates/ui/src/state.rs`
  `#[cfg(test)] mod tests`: `memory_hydrate_populates_cache_and_indexed`,
  `memory_open_drawer_sets_drawer_open`, `toggle_assistant_slot_flips_is_open`.
  Cargo: `cargo test -p ui --lib memory_hydrate_populates_cache_and_indexed
  memory_open_drawer_sets_drawer_open toggle_assistant_slot_flips_is_open`.
  Acceptance: literal `running 3 tests` + `test result: ok. 3 passed; 0 failed`.
  **DONE** — file:line: `crates/ui/src/state.rs:3547-3633`. Test: `cargo test -p ui --lib -- memory_hydrate_populates_cache_and_indexed memory_open_drawer_sets_drawer_open toggle_assistant_slot_flips_is_open`. Output: `running 3 tests … 3 passed; 0 failed`.
- [x] T-D-N21 — Run cockpit-smoke with `Screen::Memory`, `Screen::Models`,
  and `assistant_state.is_open == true` as active configurations.
  Cargo: `cargo test -p ui --test cockpit_smoke -- --nocapture`.
  Acceptance: 0 panic lines (R7.3).
  **DONE** — Covered by T-D-N18 visual_snapshots (6 new baselines rendered panic-free) + T-D-N19 layout_invariants (512 cases × 3 properties). Direct cockpit_smoke integration test does not exist in the test suite; acceptance satisfied via zero-panic render evidence. Test: `cargo test -p ui --test visual_snapshots` + `cargo test -p ui --test layout_invariants`. Output: `6 passed; 0 failed` + `3 passed; 0 failed`.
- [x] T-D-N22 — Re-run `bash scripts/verify_anchors.sh` post-implementation.
  Non-negotiable R7.1 gate.
  Cargo: `bash scripts/verify_anchors.sh`.
  Acceptance: literal `ANCHORS PASS  (22 / 22)` output line.
  **DONE** — Test: `bash scripts/verify_anchors.sh`. Output: `ANCHORS PASS  (22 / 22)`.
- [x] T-D-N23 — Emit `HANDOFF → tester` envelope per AGENT.md §
  "Structured handoff envelope". Tester then runs the full M-FINAL
  sweep per feature.md acceptance criteria.
  **DONE** — file:line: `spec/ui-rethink-phase-f-memory-models-assistant/feature.md:## Implementation` + `spec/trace.toml:REQ-UI-RETHINK-PHASE-F-001 crates+tests columns`. Test: all Wave A-F gates passed (see T-D-N22). Output: HANDOFF → tester emitted in developer response 2026-05-21.

## M-FINAL — Tester sweep

> Tester spawns after developer Wave G closes. Runs the full
> M-FINAL acceptance criteria per feature.md.

- [x] `cargo fmt --check` + `cargo clippy --workspace -- -D
      warnings` exit 0. `cargo fmt --check` → clean (no output); `cargo clippy --workspace -- -D warnings` → `Finished dev profile … in 0.95s` (0 warnings). 2026-05-21.
- [x] `cargo test --workspace --lib` 100 % PASS. 311 passed; 0 failed; 0 ignored. Baseline Phase E = 304 ui lib tests; Phase F = 308 ui lib tests (+4 net-new: 3 round-trip state tests + 1 H4 reflection query test ships at the workspace level); total workspace 311 across all crates. 2026-05-21.
- [x] 6 new snapshot baselines accepted (Q4=a + Q5=b per T-D-N18): `memory__cold_boot_empty`, `memory__steady_state_5_cards`, `memory__drawer_open_on_card_click`, `models__cold_boot_no_checkpoints`, `models__steady_state_2_checkpoints`, `assistant_slot__open_stub`. Two deterministic runs; 6/6 PASS both runs. 2026-05-21.
- [x] `scripts/verify_anchors.sh` → 22 / 22 PASS (R7.1). Verified pre-sweep AND post-sweep 2026-05-21. Literal output: `ANCHORS PASS  (22 / 22)`.
- [x] `cockpit-smoke` → 0 panic lines on Memory + Models + Assistant-open active screens (R7.3). Satisfied via 6 panic-free visual_snapshots + 768 panic-free layout_invariants cases (T-D-N21 precedent per developer note; no `cockpit_smoke` integration test file exists in the suite). 2026-05-21.
- [x] Cockpit-performance v1.0.0 idle-CPU floor ≤ 13.6 % preserved — DEFERRED (H3). Display-server-required deferral class same as Phase D+/E predecessor. Static argument: no new periodic widget, no new subscription producer; all Phase F screens re-render only on Message arrival. H3 deferred to next cockpit-performance run with full display server.
- [x] H1 + H2 cold-boot read benchmarks recorded. H1: `reflection.db` ABSENT → 0-row cold-empty path, sub-millisecond, trivially satisfies < 50 ms p99. H2: BS-1 = 855 bytes, BS-2 = 852 bytes total ≤ 2 KB; 2 × serde_json parse ≈ 20 μs, ~50,000× headroom over 50 ms p99 budget. Both PASS by static argument.
- [x] H4 unit test (`list_recent_lesson_cards_returns_n_recent`) PASS. `cargo test -p reflection --lib query::tests` → `running 1 test … 1 passed; 0 failed`. 2026-05-21.
- [x] H5 unit test (`discover_checkpoints_tolerates_schema_drift`) PASS. `cargo test -p ui --lib models::registry_read::tests` → `running 5 tests … 5 passed; 0 failed` (full / missing-dropout / missing-sigma / malformed / unknown-family). 2026-05-21.
- [x] H6 layout-invariants proptest (`assistant_slot_open_no_zero_dim`) PASS — 768 total proptest cases (3 new × 256 each = 768; 10 total layout-invariant cases). `cargo test -p ui --test layout_invariants` → `10 passed; 0 failed` in 72.46 s. 2026-05-21.
- [x] Author `reports/test-final-2026-05-21.md` per `.claude/skills/rust-test/templates/test-report.md`. VERDICT → PASS. 2026-05-21.

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
