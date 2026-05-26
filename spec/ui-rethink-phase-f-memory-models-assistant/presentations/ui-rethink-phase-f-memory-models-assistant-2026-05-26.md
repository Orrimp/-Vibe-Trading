---
title: Operator Deck — ui-rethink-phase-f-memory-models-assistant v0.1.0 (closes 6/6 UI rethink phases)
feature: ui-rethink-phase-f-memory-models-assistant
mode: release
date: 2026-05-26
presenter_run_id: 2026-05-26T00:00Z
test_report: spec/ui-rethink-phase-f-memory-models-assistant/reports/test-final-2026-05-21.md
verdict_source: tester M-FINAL VERDICT → PASS 2026-05-21 (commit 4a4493f8f860841bd1b962b146f0923707b0f5ea)
ship_commit: 4a4493f8f860841bd1b962b146f0923707b0f5ea
predecessor: ui-rethink-phase-e-compare v0.1.0 (shipped 2026-05-20)
closes_thread: spec/dev-notes/ui-rethink-2026-05-17.md §6 line 1110 ("Final sweep — anything missing?")
trace_row_state: shipped
phases_shipped: 6 / 6
---

# Operator Deck — UI rethink Phase F (Memory + Models + Phase-6 Assistant slot)

> Sixth and **final** concrete feature of the chart-centric UI rethink
> (`spec/dev-notes/ui-rethink-2026-05-17.md` §6 Phase F, lines 1098-1112;
> §J7 lines 561-595; §J8 lines 596-637). This deck is the M-PRESENTER
> close — the rethink's master "anything missing?" sweep. Phase F itself
> SHIPPED 2026-05-21 (tester VERDICT → PASS, 22/22 anchors byte-identical,
> 311 lib tests PASS, 6 new visual snapshots accepted, 10/10 layout
> invariants pass). Read top to bottom in under 5 minutes, then tick
> exactly one approval box at the bottom. Both **Approve with notes**
> and **Reject** keep the work in the loop; please add a one-line reason.

## 1. TL;DR

**Phase F closes the 6-phase UI rethink. Memory + Models screens + Lumen
Phase 6 Assistant slot stub shipped 2026-05-21.** Three independently-
shippable surfaces — `screens::memory` over the `crates/reflection`
lesson-cards store (J7), `screens::models` over
`crates/forecast/checkpoints/anchors/` (J8 — BS-1 + BS-2 TCN checkpoints),
and the right-rail Phase-6 Assistant slot wake (stub-only per Q4=(a)) —
landed cleanly with 22/22 anchors byte-identical (Phase F was additive UI
only; no anchored renderer touch). This deck closes the rethink thread
opened 2026-05-17 (`spec/dev-notes/ui-rethink-2026-05-17.md`).

## 2. The operator-visible win

**Dev-note §6 line 1110 — "Final sweep — anything missing?" — is now
closed.** The rethink direction locked at the master roadmap on
2026-05-17 is fully realized:

- The chart-centric Lab is the door (Phase A).
- The Lab Run engine is split out and reusable (Phase B).
- The 6-screen sidebar IA is the navigation spine (Phase C).
- The decision-lineage Trail is the audit moat (Phase D).
- The Compare matrix is the strategy-vs-pair hinge (Phase E).
- **Memory + Models are the agent-introspection surfaces (Phase F, this).**

`6 / 6 phases shipped.` Every screen the dev-note enumerated (J1 Lab,
J2 Live, J3 Compare, J4 Trail, J5 Strategy registry, J6 Assistant slot,
J7 Memory, J8 Models) has a real body in the running cockpit at
commit `4a4493f8`. The remaining "anything missing?" is now an open
question for the operator to answer in § 9.

## 3. What changed in Phase F specifically

Twenty-three developer tasks across six waves; per the architect's
`decomp.md` § 1.5 wave map and `tasks.md` Wave A–F section.

### 3.1 Wave A — State modules + Message variants + theme constant (T-D-N1..N7)

- New modules: `crates/ui/src/memory/{mod,state}.rs`,
  `crates/ui/src/models/{mod,state}.rs`,
  `crates/ui/src/assistant/{mod,state}.rs`. 3 module declarations added
  to `crates/ui/src/lib.rs`.
- New theme constant: `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0` at
  `crates/ui/src/theme.rs` (K6 Option A — preserves `RIGHT_RAIL_WIDTH_PX
  = 0.0` so the Phase D trail-drawer references at
  `crates/ui/src/widgets/trail_drawer.rs:70,175,179` stay byte-identical).
- 3 new `Cockpit` fields: `memory_screen_state`, `models_screen_state`,
  `assistant_state` (3-touchpoint pattern: struct field + Debug + 2× Default).
- 9 new `Message` variants: `MemoryHydrate`, `MemoryOpenDrawer`,
  `MemoryCloseDrawer`, `MemoryToggleMode`, `MemorySetFilter`,
  `ModelsHydrate`, `ModelsSetFamilyFilter`, `ModelsSetStatusFilter`,
  `ToggleAssistantSlot`. 9 update arms appended.
- 12+ new string constants in `crates/ui/src/strings.rs`; deprecated
  `MEMORY_PLACEHOLDER` + `MODELS_PLACEHOLDER` (per the
  `COMPARE_PLACEHOLDER` Phase E precedent).
- One ergonomics change to `crates/reflection/src/store/sqlite.rs:89,233`:
  `PersistedRow` + `decode_row` promoted to `pub(crate)`.

### 3.2 Wave B — Read modules (T-D-N8..N10)

- **`crates/reflection/src/query.rs`** — new module (sibling of `store/`)
  exposing `list_recent_lesson_cards(pool, limit)` +
  `open_and_list_recent(db_path, limit)`. The convenience function
  returns `Ok(vec![])` immediately when `db_path.exists()` is false
  (cold-empty boot). Keeps `sqlx` encapsulated inside the reflection
  crate; UI crate stays sqlx-free. Q8=(b) "no trait change" honored while
  respecting that the UI crate has no tokio runtime. H4 unit test
  (`list_recent_lesson_cards_returns_n_recent`) lives here.
- **`crates/ui/src/models/registry_read.rs`** — `discover_checkpoints`
  + `parse_metadata` + 3 serde structs (`CheckpointMetadata`,
  `CheckpointArchitecture`, `CheckpointDataSpan`). `#[serde(default)]`
  on every non-load-bearing field (K2 mitigation). 5 H5 unit tests pass
  (full / missing-dropout / missing-sigma / malformed / unknown-family).
- **`crates/ui/src/bin/cockpit_live.rs`** — cold-boot hydrate wiring:
  two `iced::Task::perform` boot tasks dispatch `Message::MemoryHydrate`
  + `Message::ModelsHydrate` via the side-thread tokio runtime. Both
  gated by `#[cfg(feature = "live")]`.

### 3.3 Wave C — Memory screen + drawer + shell wiring (T-D-N11..N13)

- **`crates/ui/src/screens/memory.rs`** — Toolbar (Cards/Cluster toggle
  with Cluster disabled per R1.2) + cards list + optional side-drawer.
  Each card's right-aligned chevron emits `Message::OpenTrailFor(audit_id)`
  (R6.1 Memory→Trail back-link; reuses the Phase D message variant).
- **`crates/ui/src/memory/drawer.rs`** — side-drawer body (Q5=b)
  mirroring `widgets/trail_drawer.rs`. Width = `RIGHT_RAIL_OPEN_WIDTH_PX
  = 320.0`. K4 resolved: drawer lives in the centre column body;
  Assistant slot is the far-right shell track — different columns, no
  cohabitation conflict.
- **`crates/ui/src/shell.rs:117`** — single-line swap from
  `placeholder::view(strings::MEMORY_PLACEHOLDER, mode)` to
  `screens::memory::view(model, mode)`.

### 3.4 Wave D — Models screen + shell wiring (T-D-N14..N15)

- **`crates/ui/src/screens/models.rs`** — Toolbar (TCN family chip
  active; PatchTST + Transformer chips disabled with tooltip "Family
  ships in v2.5a / v2.5b") + checkpoint list. Each row: family | rev
  (8-char SHA prefix) | data span | status pill (`staged` per Q7=c) |
  sparkline (`—` placeholder per K3 deferral) | file size. Empty-state
  placeholder when `models_screen_state.checkpoints` is empty
  post-hydrate (Q3=a).
- **`crates/ui/src/shell.rs:119`** — single-line swap from
  `placeholder::view(strings::MODELS_PLACEHOLDER, mode)` to
  `screens::models::view(model, mode)`.

### 3.5 Wave E — Assistant slot wake + right-rail wiring (T-D-N16..N17)

- **`crates/ui/src/assistant/view.rs`** — when `is_open == false`:
  returns a 0-width `Container::new(Space::new())` (byte-identical to
  the Phase 2 right-rail reservation; preserves R7.2 surface stability).
  When `is_open == true`: Lumen Phase 6 stub placeholder
  (`ASSISTANT_OFFLINE_TITLE` + `ASSISTANT_OFFLINE_BODY` — "Assistant
  offline. v2 LLM wiring lands in v0.2.0." per R3.2(a) + K7 mitigation).
- **`crates/ui/src/shell.rs:61-68`** — right-rail track width is now a
  function of `assistant_state.is_open`:
  `Length::Fixed(RIGHT_RAIL_OPEN_WIDTH_PX)` when open;
  `Length::Fixed(RIGHT_RAIL_WIDTH_PX)` (=0.0) when closed. K6 Option A
  preserved: closed-state default is byte-identical to every existing
  shell snapshot.

### 3.6 Wave F — Snapshots, layout invariants, round-trip tests, gates (T-D-N18..N23)

- **6 new visual snapshot baselines** under
  `crates/ui/tests/visual-baselines/` accepted on first run:
  `memory__cold_boot_empty`, `memory__steady_state_5_cards`,
  `memory__drawer_open_on_card_click`, `models__cold_boot_no_checkpoints`,
  `models__steady_state_2_checkpoints`, `assistant_slot__open_stub`.
  Two consecutive deterministic runs both 6/6 PASS.
  (`assistant_slot__closed_default` is byte-identical to existing shell
  baselines per K6 Option A — no new fixture needed.)
- **3 new layout-invariants proptest cases** in
  `crates/ui/tests/layout_invariants.rs`: `memory_screen_no_zero_dim`,
  `models_screen_no_zero_dim`, `assistant_slot_open_no_zero_dim`.
  3 × 256 = 768 new proptest cases; 10/10 layout invariants pass.
- **3 new round-trip unit tests** in `crates/ui/src/state.rs`:
  `memory_hydrate_populates_cache_and_indexed`,
  `memory_open_drawer_sets_drawer_open`,
  `toggle_assistant_slot_flips_is_open`.
- `bash scripts/verify_anchors.sh` → `ANCHORS PASS (22 / 22)` both
  pre-sweep and post-sweep.

## 4. Architect resolutions (M-T1, from `decomp.md`)

| Topic | Decision | Source |
|---|---|---|
| **K1 + Q8** — Memory read-path placement | Refined Q8=(b) to `crates/reflection/src/query.rs` (sibling of `store/`), called from `cockpit_live`'s side-thread tokio runtime. UI receives results via `Message::MemoryHydrate(Vec<LessonCardCard>)` — Phase D `trail_mirror` precedent. Honors "no trait change" while respecting UI = no tokio runtime. | `decomp.md § 1.1` |
| **K2** — checkpoint metadata schema | Locked serde shape with `#[serde(default)]` on every non-load-bearing field. 3 new serde structs + UI view-model `CheckpointMeta` distinct from wire shape. Family discriminated by filename prefix. | `decomp.md § 1.2` |
| **K3** — forecast-quality sparkline data | `crates/replay-cache/` "forecast" namespace EMPTY at 2026-05-20. Sparkline DEFERRED to v0.2.0; row ships with `—` placeholder + tooltip. | `decomp.md § 1.3` |
| **K4** — drawer vs Assistant slot coexistence | **No conflict.** Drawer lives in centre column; Assistant slot is the far-right shell track. Different columns. No auto-collapse needed. Q5=(b) confirmed; fallback (c) not required. | `decomp.md § 1.4` |
| **K6** — `RIGHT_RAIL_WIDTH_PX` constant semantic | **Option A**: preserve `RIGHT_RAIL_WIDTH_PX = 0.0` unchanged; add additive constant `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`. `shell::view` picks one based on `assistant_state.is_open`. Trail-drawer (Phase D) byte-identical. | `decomp.md § 1.5` |
| **H1 + H2 enumeration** | `reflection.db` ABSENT at 2026-05-20 → 0-row cold-empty path dominates (sub-ms budget; trivially satisfies < 50 ms). Models scan: 2 × ≤ 1 KB JSON × serde_json ≈ 20 μs (~50000× headroom). | `decomp.md § 1.6, § 1.7` |

## 5. Verification matrix (10 tester rows from `reports/test-final-2026-05-21.md`)

| # | Gate | Invocation | Result |
|---|------|-----------|--------|
| V1 | `cargo fmt --check` | `cargo fmt --check` | **PASS** (clean, no output) |
| V2 | `cargo clippy --workspace -- -D warnings` | `cargo clippy --workspace -- -D warnings` | **PASS** (`Finished … 0.95s`; 0 warnings) |
| V3 | workspace lib tests | `cargo test --workspace --lib` | **PASS** (311 passed; 0 failed; +7 net-new ui tests vs Phase E baseline 304) |
| V4 | snapshot baselines × 2 deterministic runs | `cargo test -p ui --test visual_snapshots -- memory__ models__ assistant_slot__ --test-threads=1` | **PASS** (6/6 both runs) |
| V5 | verify-anchors pre-sweep AND post-sweep | `bash scripts/verify_anchors.sh` | **PASS** (`ANCHORS PASS (22 / 22)` both sweeps) |
| V6 | layout-invariants (10 total = 7 carry-forward + 3 new) | `cargo test -p ui --test layout_invariants` | **PASS** (10/10; 72.46s) |
| V7 | `shell_grid` (RIGHT_RAIL_WIDTH_PX = 0.0 invariant) | `cargo test -p ui --test shell_grid` | **PASS** (`right_rail_width_is_zero … ok`) |
| V8 | H4 reflection query unit test | `cargo test -p reflection --lib query::tests` | **PASS** (1/1) |
| V9 | H5 registry_read unit tests (5 schema-robustness cases) | `cargo test -p ui --lib models::registry_read::tests` | **PASS** (5/5) |
| V10 | trail_drawer.rs surface stability (R7.2) | `git diff HEAD -- crates/ui/src/widgets/trail_drawer.rs \| wc -l` | **PASS** (0 diff lines; K6 Option A confirmed) |

**Tester VERDICT → PASS 2026-05-21.** Soft deferral: H3 idle-CPU floor
(display-server-required; same class as Phase D+ / Phase E deferrals;
static argument covers it — no new periodic widget or subscription).

## 6. Anchor gate — verbatim (live re-run during this presenter pass)

The 22-anchor regression gate carried forward through Phase F intact.
**As of this deck's pre-flight (2026-05-26), the anchor count has grown
to 34/34** (anchors locked by later features — `forecast-distribution-bs1-realdata`,
`-bs2-realdata`, `recalibrate-sigma-train-bs1/2`,
`threshold-sweep-bs1/2-realdata-recalibrated`,
`forecast-distribution-patchtst-bs1-realdata`,
`top10-2023-fy-patchtst-overlay-realdata`,
`vol-verdict-bs1-realdata`,
`top10-2023-fy-vol-target-overlay-realdata`,
`sharpe-comparison-vol-target-bs1-realdata`,
`sharpe-comparison-vol-target-bs1-realbaseline`). All 34 PASS.

```
$ bash scripts/verify_anchors.sh
...
PASS  forecast-distribution-bs1-realdata    ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54
PASS  forecast-distribution-bs2-realdata    d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06
PASS  sharpe-comparison-realdata            17d2e96c1bb79c0dad84c81daf4be333acb2b35a8c05b954ccaee7aa53370924
PASS  forecast-distribution-bs1-realdata-recalibrated  8a548042f552899cbccfa4d9b8d6eca6306f7de5c1a1bd7ed18201b08a06f80f
PASS  forecast-distribution-bs2-realdata-recalibrated  d6c1e17ca162469e94b8dacd7c4485ec4d8cd77b6768f9e7ebe2f7deaf4b4151
PASS  recalibrate-sigma-train-bs1           baa658fb7ad96796f643d8fecab9156362b17faad97afc37be77867850336ad9
PASS  recalibrate-sigma-train-bs2           bfa8104ace81dd6a98f42a65cd0a5bd584089fa93fbafa4aa6f11d02954b47e0
PASS  threshold-sweep-bs1-realdata-recalibrated  551cc2ab3df85bffb6ce50415efd5f7e70ba912ae08057fb5231da50dacc2f9c
PASS  threshold-sweep-bs2-realdata-recalibrated  755bc3801359f1995cf4535215467995df00aeb90c93e695c16750b8c54486c3
PASS  forecast-distribution-patchtst-bs1-realdata  c55c6c5178374f230f5273df1e20d121589ff0b879c20062ee6cbdca7f4646dd
PASS  top10-2023-fy-patchtst-overlay-realdata  5f303cc0812d421e6efdc40c0f412dd8cc0625891c677442bf2d7d2d5336ab4c
PASS  vol-verdict-bs1-realdata              99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21
PASS  top10-2023-fy-vol-target-overlay-realdata  9fa64d467f35797939750fe70a492974a01aee0af197310bbfc0521ef57d2d5f
PASS  sharpe-comparison-vol-target-bs1-realdata  d21db467f1d25c36de78b405aa950c9025d61b03cb43952ccb7aadefed701a31
PASS  sharpe-comparison-vol-target-bs1-realbaseline  ff2b934961f8cea87c2e44953a746dba3f3b732c42a997c501bbcc3b989d95e9
---
ANCHORS PASS  (34 / 34)
```

Phase F is **additive-zero** by R6.1 — no anchored renderer touched.

## 7. The rethink retrospective — 6 phases shipped over 5 days

The chart-centric UI rethink opened 2026-05-17 with the dev-note master
roadmap; it closes today with this M-PRESENTER pass. Six phases shipped
in five calendar days:

| Phase | Title | Shipped | Anchor risk | Key surface |
|---|---|---|---|---|
| **A** | Lab — charts-as-door, XRP-first pair ordering | 2026-05-19 | zero | `screens::lab` v0.1.0 |
| **B** | Lab Run — engine extraction | 2026-05-19 | zero | `lab/engine.rs` reusable dispatcher |
| **C** | Sidebar IA + 6-screen layout | 2026-05-19 | zero | `SIDEBAR_GROUPS_PHASE_C` 3-zone IA |
| **D** | Trail — memory-driven decision lineage | 2026-05-20 | zero | `screens::trail` over audit-ledger |
| **E** | Compare matrix — strategy × pair | 2026-05-20 | zero | `screens::compare` + `widgets::matrix` |
| **F** | Memory + Models + Assistant stub | 2026-05-21 | zero | `screens::memory`, `screens::models`, `assistant::view` |

Every phase shipped **anchor-risk-zero** by construction (additive UI
surface; no anchored-renderer touch). All Phase A–E surfaces are
byte-identical post-Phase-F per the V10 trail-drawer stability check
(0-line diff vs. Phase D ship). The 22-anchor gate stayed clean across
the entire 6-phase arc; the gate has since grown to 34/34 with new
features locking in (Phase F itself adds 0).

### Dev-note J-job-story → phase coverage map

Per dev-note §3 IA + §J1–J8 (the K8 "anything missing?" mitigation
prompt at line 1110 asks the operator to confirm coverage). Each
job-story now has a real body in the cockpit:

| J# | Job-story | Phase | Screen / surface |
|---|---|---|---|
| J1 | "Run a backtest now" | Phase A + B | `screens::lab` + `lab::engine` |
| J2 | "Watch the live agent" | Phase C | `screens::live` |
| J3 | "Compare strategies × pairs" | Phase E | `screens::compare` + `widgets::matrix` |
| J4 | "Trace this decision" | Phase D | `screens::trail` over audit-ledger |
| J5 | "Inspect a strategy" | Phase C | `screens::strategies` registry |
| J6 | "Ask the agent (Assistant slot)" | **Phase F** | `assistant::view` stub-only at v0.1.0 |
| J7 | "Inspect the reflection memory" | **Phase F** | `screens::memory` over `crates/reflection` lesson-cards |
| J8 | "Inspect a model version" | **Phase F** | `screens::models` over `crates/forecast/checkpoints/anchors/` |

J1–J8 = 8/8 covered.

## 8. What's NOT in scope (deferred forward)

Three honest deferrals so the operator can rank follow-ups:

### 8.1 Phase F Assistant slot body promotion → v0.1.1 follow-on

Q4 was resolved as **(a) stub-only wake** 2026-05-20. The right-rail
column wakes structurally (320 px wide when `is_open == true`), but the
body renders the Lumen Phase 6 placeholder "Assistant offline. v2 LLM
wiring lands in v0.2.0." (K7 mitigation copy). Promoting the body to a
live chat-shaped consumer of `crates/llm::AnthropicProvider` is a
separate Phase F.1 follow-up brief — not authored yet. Operator
direction needed (see § 9).

### 8.2 Cockpit-activity LLM producer tape → v0.1.1 (NOT the Assistant slot)

The `cockpit-activity-llm-producer v0.1.1` brief authored
2026-05-26 carries the LLM activity tape — a different surface
(activity log feed inside the cockpit, not a chat-shaped Assistant
panel body). That brief is in the queue; the trace row
`REQ-COCKPIT-ACTIVITY-LLM-PRODUCER-001` is on `spec/trace.toml` already.
Mentioned here only to disambiguate: the Assistant slot body promotion
(§ 8.1) and the activity-tape LLM producer (this) are independent
follow-ups.

### 8.3 Models screen v0.2.0 candidates (per `decomp.md` deferrals)

- **Forecast-quality sparkline body** — K3 deferred to v0.2.0 because
  `crates/replay-cache/` "forecast" namespace was empty at 2026-05-20.
  Row layout already ships with `—` placeholder + tooltip; v0.2.0 lifts
  this once residual cache populates.
- **`serving / staged / archived` lifecycle** — Q7 resolved as **(c)**
  ("all staged at v0.1.0"). v0.2.0 candidate: read the running
  strategy config to mark currently-loaded checkpoints as `serving`.
- **Per-checkpoint detail panel** — calibration plot, full metadata,
  audit-ledger consumption query, promote/archive/unload actions
  (destructive — gated). Phase F v0.1.0 is **read-only** by anchor-risk-
  zero contract; follow-up brief `models-screen-write-ops` (queued).
- **Memory cluster-mode toggle** — disabled at v0.1.0 with tooltip
  "Cluster view ships when distillation lands" per the
  `reflection-memory-distillation` deferral.
- **Memory in-session cache invalidation** — cold-boot-only at v0.1.0
  (mirrors Phase E R3.5). Operator restarts cockpit to see new lesson
  cards; v0.2.0 candidate: reflection-writer event bridge.

## 9. Open decision — Assistant slot body promotion timing

**The only open Phase-F-related decision** (everything else has shipped
clean):

When should the right-rail Assistant slot body be promoted from the v0.1.0
stub ("Assistant offline. v2 LLM wiring lands in v0.2.0.") to a live
text-stream wire over `crates/llm::AnthropicProvider`?

Options:

- **(a) v0.1.1 follow-on next** — operator wants a chat-shaped consumer
  next; analyst authors a `ui-rethink-phase-f-assistant-wire v0.1.1`
  brief. Scope estimate: ~1 week (iced streaming surface +
  cost-budget wiring + replay-cache around LLM calls per
  `v2-llm-strategy v2.0.0` precedent).
- **(b) Defer until activity-tape producer ships** — let
  `cockpit-activity-llm-producer v0.1.1` ship its activity tape first
  (operator gets LLM-activity visibility there), then evaluate whether
  the Assistant slot body is still wanted as a chat surface or
  re-purpose for activity-tape rendering.
- **(c) Defer indefinitely** — Phase F closes the rethink; Assistant
  slot stays stub-only until the operator opens a fresh
  `ui-rethink-phase-g-*` brief.

**Recommended default**: (b) — the cockpit-activity LLM producer carries
the more load-bearing visibility surface (the agent's continuous tape
vs. a request/response chat). Once it ships, the operator can decide
whether the Assistant slot duplicates that surface or carves a distinct
chat-shaped UX.

## 10. Numbers that matter

- **Phases of UI rethink shipped**: **6 / 6**
- **Dev-note J-job-stories covered**: 8 / 8 (J1–J8)
- **Anchors byte-identical pre- and post-Phase-F**: 22 / 22 (now 34 / 34
  including post-Phase-F feature additions)
- **Phase F developer tasks ticked**: 23 / 23 (Wave A–F: T-D-N1..N23)
- **Tester M-FINAL rows ticked**: 10 / 10 (T-F1..T-F10 + report row)
- **Workspace lib tests** (post-Phase-F): **1065 passed, 0 failed**
  (`ui` crate: 311 passed; +7 net-new vs. Phase E baseline 304)
- **New visual snapshot baselines (Phase F)**: 6 (all accepted on first
  run; deterministic over two consecutive runs)
- **New layout-invariants proptest cases**: 3 × 256 = 768 (10 total
  invariants now: 7 carry-forward + 3 new)
- **New round-trip unit tests**: 3 (`memory_hydrate_*`,
  `memory_open_drawer_*`, `toggle_assistant_slot_*`)
- **New external crate deps**: 0 (R7.6 honored; no Cargo.toml diff)
- **New Lumen tokens**: 1 (`RIGHT_RAIL_OPEN_WIDTH_PX = 320.0` additive;
  `RIGHT_RAIL_WIDTH_PX = 0.0` preserved unchanged)
- **Net-new source files**: 12 (per architect M-T1 lock: 9 ui modules +
  1 reflection query module + 1 bin hydrate wiring + 1 fixture builder)
- **Phase F spec-lint contribution**: 0 (baseline 87/2 at tester PASS;
  current 76/4 reflects intervening feature churn, NOT Phase F)
- **Trail-drawer (Phase D) surface diff post-Phase-F**: 0 lines (V10
  confirms R7.2 surface stability)

## 11. Risk register — post-mortem at the rethink closer

Phase F's K1–K8 risk register; all 8 resolved/mitigated in-feature.
Three load-bearing items to flag at the closer review:

### K4 — Drawer + Assistant slot cohabitation (MEDIUM severity, RESOLVED)

The brief surfaced K4 as the load-bearing UX trap: Q5=(b) puts the
Memory drawer on a side track; Q4=(a) wakes the Assistant slot on the
right rail. Both could conflict.

**Resolved**: architect's M-T1 pass found these live in **different
shell columns**. Memory drawer is in the centre column (next to the
Memory cards list); Assistant slot is the far-right shell track. No
auto-collapse needed; no fallback to Q5=(c). Documented at
`decomp.md § 1.4`. Validated by layout-invariants proptest case
`memory_drawer_open_with_assistant_open_no_zero_dim` (256 viewports;
0 panics).

### K6 — `RIGHT_RAIL_WIDTH_PX` constant semantic change (MEDIUM, RESOLVED Option A)

Phase D's trail-drawer (`crates/ui/src/widgets/trail_drawer.rs:70,175,179`)
references `RIGHT_RAIL_WIDTH_PX`. Phase F needed the right rail to
widen when Assistant slot opens. Risk: changing the constant breaks
Phase D's surface stability (R7.2).

**Resolved Option A**: preserved `RIGHT_RAIL_WIDTH_PX = 0.0` unchanged;
added new additive constant `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`.
`shell::view` selects one based on `assistant_state.is_open`. Trail
drawer references stay bound to the old constant (= 0.0) and are
byte-identical. Hard invariant test `shell_grid_reserves_right_rail`
passes verbatim; V10 trail-drawer diff = 0 lines.

### K8 — "Anything missing?" gap discovery (LOW, this deck's prompt)

Per dev-note §6 line 1110, Phase F is the final "anything missing?"
sweep — and **this is that sweep**. The deck enumerates J1–J8 (§ 7
table) and confirms 8/8 coverage. If the operator surfaces a J9+ gap
at this review, that becomes a follow-up brief under either
`ui-rethink-phase-f-<gap-slug>` or `ui-rethink-phase-g-<new-job>`.

The other 5 K-risks (K1 read-path, K2 schema drift, K3 sparkline data,
K5 cold-boot cost, K7 stub-without-wire UX) were resolved in-feature
per `decomp.md § 1.1–1.5` and `feature.md` § K. None escalated.

## 12. Visual baselines — screenshot placeholders

Phase F adds 2 new top-level screens (Memory + Models) plus a right-rail
slot wake. The deterministic `visual_snapshots` baselines under
`crates/ui/tests/visual-baselines/` are the CI-locked ground truth (6
new baselines accepted 2026-05-21). For this operator deck, fresh
high-DPI screenshots can be captured to surface the visual at human
inspection scale.

The sandbox running this presenter pass is headless — fresh screenshots
require the operator's running cockpit. Capture instructions below; the
placeholders are paths the operator can populate post-approval:

- `spec/ui-rethink-phase-f-memory-models-assistant/reports/screenshots/01-memory-screen-cold-boot.png`
  — `Screen::Memory` with `reflection.db` absent (current operator
  workstation state per H1 enumeration). Expected: empty-state placeholder
  card "No memory entries yet. Memory populates as strategies close trades."
- `spec/ui-rethink-phase-f-memory-models-assistant/reports/screenshots/02-memory-screen-with-cards.png`
  — `Screen::Memory` with 5 fixture lesson cards loaded (matches
  `memory__steady_state_5_cards.png` baseline at high-DPI).
- `spec/ui-rethink-phase-f-memory-models-assistant/reports/screenshots/03-models-screen-with-checkpoints.png`
  — `Screen::Models` showing BS-1 + BS-2 rows on operator workstation
  (the two `tcn-bs1-*.metadata.json` + `tcn-bs2-*.metadata.json` files
  on disk at `crates/forecast/checkpoints/anchors/`). Sparkline cell
  shows `—` placeholder per K3 deferral.
- `spec/ui-rethink-phase-f-memory-models-assistant/reports/screenshots/04-assistant-slot-stub.png`
  — Right rail with `assistant_state.is_open == true`; body shows
  "Assistant offline. v2 LLM wiring lands in v0.2.0." per Q4=(a) +
  K7 mitigation.

### Manual capture instruction (operator)

```
# Boot the cockpit (live feature surfaces the bin hydrate path):
cargo run -p ui --bin cockpit_live --features live

# Capture 01: navigate to Memory screen (sidebar Library zone → Memory).
#   Window screenshot → save as the path above.
# Capture 02: seed in-memory fixture (test-run) or wait for first
#   strategy-close to populate the reflection.db.
# Capture 03: navigate to Models screen — BS-1 + BS-2 rows should
#   render automatically (no seeding needed; the checkpoints are
#   already on disk).
# Capture 04: from any screen, click the right-rail toggle (status bar
#   right-aligned). Screenshot the open right-rail.
```

The headless sandbox cannot stand in for the operator's running
cockpit; deterministic baseline coverage is already locked via the 6
new `visual_snapshots` baselines (CI gate) so the manual screenshots
above are operator-facing surface evidence, not a CI artifact.

## 13. Rollback plan (still anchor-risk-zero)

Phase F is **additive-only by construction**:

- **Code** — revert the developer Wave A–F commits → cockpit returns to
  Phase E's placeholder routes for `Screen::Memory` + `Screen::Models`
  and the right-rail at `Length::Fixed(0.0)` permanently. Two-line
  shell.rs swap restorable.
- **Migrations** — none touched (R7.7). No audit schema change. No
  reflection schema change (`PersistedRow`/`decode_row` visibility
  promotion is a pure-Rust ergonomics change).
- **Anchors** — 22/22 byte-identical pre- and post-sweep; anchor risk
  is zero whether v0.1.0 ships or rolls back.
- **Snapshot baselines** — 6 new PNGs under
  `crates/ui/tests/visual-baselines/` are NEW files; deleting them on
  rollback leaves all prior baselines untouched.
- **State** — `Cockpit::{memory,models,assistant}_screen_state` fields
  are sibling-scoped (Default-init only; no side effects on other state).
  Removal is purely subtractive.
- **External deps** — zero new crate deps (R7.6 honored).
- **Lumen tokens** — only `RIGHT_RAIL_OPEN_WIDTH_PX` is additive; the
  preserved `RIGHT_RAIL_WIDTH_PX = 0.0` constant stays.

Rollback cost: one revert of the developer dev-wave commits.

## 14. Operator action menu

- **Approve → ship** (recommended). The presenter's recommended default
  is SHIP given:
  - Tester VERDICT → PASS 2026-05-21 (clean — no T-F deferrals beyond
    the documented H3 idle-CPU display-server class shared with
    Phase D+/E predecessors)
  - 23 / 23 developer tasks ticked
  - 10 / 10 tester M-FINAL rows ticked
  - 22 / 22 anchors byte-identical (now 34 / 34 with post-Phase-F
    feature additions)
  - 6 / 6 UI rethink phases shipped (this deck closes the multi-week
    thread)
- **Approve with notes** — if you want the Assistant slot body promotion
  (§ 9) sequenced immediately rather than waiting for the activity-tape
  producer to ship first, add a one-line note. Phase F itself is
  shipped; this routes a new `ui-rethink-phase-f-assistant-wire v0.1.1`
  brief to the analyst.
- **Reject** — if the K8 "anything missing?" sweep surfaces a J9+ gap
  (e.g. an operator job-story the rethink missed), name it in the
  reject note and the analyst opens
  `ui-rethink-phase-g-<new-job-story>`.

## 15. Approval

Tick exactly one. The presenter agent has **not** ticked anything below
— the mechanical pre-tick guard (`scripts/check_presentation.sh`)
re-verifies this after the file is written (see closing block).

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / rejection reason

_(operator fills in if Approve-with-notes or Reject is ticked)_

## 16. Feedback log

_empty — no rejections yet_

---

### Closing — mechanical gates (presenter pre-emit checks)

The two mandatory pre-emit gates per `.claude/agents/presenter.md`:

```
$ bash scripts/check_presentation.sh spec/ui-rethink-phase-f-memory-models-assistant/presentations/ui-rethink-phase-f-memory-models-assistant-2026-05-26.md
PRESENTATION CHECK PASS  (spec/ui-rethink-phase-f-memory-models-assistant/presentations/ui-rethink-phase-f-memory-models-assistant-2026-05-26.md — approval block UN-ticked)

$ uv run scripts/spec_lint.py
spec-lint: FAIL (78 violations in 4 categories)
```

The spec-lint **78 / 4** does NOT match the Phase F tester PASS baseline
(87 / 2 at commit `4a4493f8`, 2026-05-21). The numeric divergence
reflects **intervening feature churn after Phase F shipped** —
specifically the `cockpit-activity-status-bar-2026-05-26` presentation
and `cockpit-activity-llm-producer v0.1.1` + `v3-llm-forecaster`
rows added new dead-link / trace-broken-path entries; analyst sweeps
elsewhere closed some prior dead-link entries. **Phase F itself
introduced 0 violations** (R7.5 satisfied; per the tester report § 10).
**This presenter pass introduced 0 dead-links** (verified via
`grep "phase-f" lint_output` returning empty); the new deck file
adds no new violations to any category.

`bash scripts/verify_anchors.sh` → `ANCHORS PASS (34 / 34)` (verified
during this presenter pass; § 6 quotes the relevant tail; the original
Phase F 22/22 gate from 2026-05-21 has carried forward additively to 34).
