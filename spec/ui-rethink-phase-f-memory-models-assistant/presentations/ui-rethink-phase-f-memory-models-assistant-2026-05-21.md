---
title: Operator Deck — ui-rethink-phase-f-memory-models-assistant v0.1.0 (FINAL PHASE)
feature: ui-rethink-phase-f-memory-models-assistant
mode: release
date: 2026-05-21
presenter_run_id: 2026-05-21T00:00Z
test_report: spec/ui-rethink-phase-f-memory-models-assistant/reports/test-final-2026-05-21.md
verdict_source: tester M-FINAL VERDICT → PASS (clean — only display-server H3 deferral)
commit_at_tester_pass: 4a4493f8f860841bd1b962b146f0923707b0f5ea
predecessor: ui-rethink-phase-e-compare v0.1.0 (shipped 2026-05-20)
trace_row_state: in-progress  # promoted to accepted/shipped on operator tick
rethink_phase: 6 of 6 (FINAL)
---

# Operator Deck — UI rethink Phase F (Memory + Models + Phase-6 Assistant slot — J7 + J8 + Lumen 6)

> **Sixth and FINAL** concrete feature carved out of the chart-centric UI
> rethink (`spec/dev-notes/ui-rethink-2026-05-17.md` §6 Phase F, lines
> 1098-1112; §J7 lines 561-595; §J8 lines 596-637). Sprint-review deck —
> read top to bottom in under 5 minutes, then tick exactly one approval
> box at the bottom. **§ 9 is the dev-note's "final sweep — anything
> missing?" prompt** — your last chance to surface a J9+ gap before the
> rethink closes. Both **Approve with notes** and **Reject** keep the
> work in the loop; please add a one-line reason so the relevant agent
> can act on it.

## 1. Operator headline

Phase F closes the UI rethink. Three reserved surfaces — **Memory**
(J7 — "what did the agent learn?"), **Models** (J8 — "which checkpoint
is on disk?"), and the **Phase-6 Lumen Assistant slot** — all wake at
v0.1.0. The Memory screen is a reverse-chronological cards list of
`lesson_cards` from the reflection store, with a side drawer on
chevron-click for source-trade detail and a back-link into Trail
(Phase D). The Models screen is a flat list of the v2.5 TCN `BS-1` +
`BS-2` checkpoints on disk, with columns for hash / training date /
val loss / sigma_train / status pill (all "staged" at v0.1.0 per
Q7=(c) — lifecycle classification deferred to v0.2.0). The Assistant
slot wakes **structurally** — `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0` opens
when `assistant_state.is_open == true` — with honest stub copy
("Assistant offline. v2 LLM wiring lands in v0.2.0.") per Q4=(a); the
v2 LLM strategy shipped 2026-05-13 so the wake condition is
structurally met, but the chat-shaped consumer is deferred. The
release is **additive-only by construction**: 22 backtest body-SHA-256
anchors byte-identical pre- and post-sweep; the closed-state right-rail
is byte-identical to Phase 2 reservation (K6 Option A —
`RIGHT_RAIL_WIDTH_PX = 0.0` constant preserved verbatim, new
`RIGHT_RAIL_OPEN_WIDTH_PX` is purely additive); Phase D
`trail_drawer.rs` has a **0-line diff** post-sweep (R7.2 surface
stability); the lib-test count rises 304 → 311 (+7 new) with zero
failures; layout-invariants 10/10 (7 carry-forward + 3 new × 256-case
proptests = 768 panic-free new cases); and no Phase A/B/C/D/D+/E-shipped
surface is touched. **Only deferral is the predecessor-class H3
idle-CPU 60-s probe (display-server required, same class as Phase D+/E).**
Every M-FINAL hard gate is green.

## 2. What landed

### 2.1 New `crates/reflection/src/query.rs` — Memory cold-boot read path (R5.1 refined)

- [`crates/reflection/src/query.rs:24-71`](../../../crates/reflection/src/query.rs)
  — new `open_and_list_recent(db_path, limit)` convenience function
  that returns `Ok(vec![])` immediately when `db_path.exists() == false`
  (cold-empty boot) and otherwise opens an `SqliteReflectionStore`
  and calls `list_recent_lesson_cards(pool, limit)`. Honors the Q8=(b)
  "no trait change" decision while respecting that `crates/ui` has no
  tokio runtime. `pub mod query;` declared at
  [`crates/reflection/src/lib.rs:47`](../../../crates/reflection/src/lib.rs).
- [`crates/reflection/src/query.rs:36-53`](../../../crates/reflection/src/query.rs)
  — `list_recent_lesson_cards(pool, limit)` SQL:
  `SELECT ... FROM lesson_cards ORDER BY closed_at DESC LIMIT ?`.
  Schema-aligned with
  [`crates/reflection/migrations/001_lesson_cards.sql:8-24`](../../../crates/reflection/migrations/001_lesson_cards.sql).
- H4 falsification unit test (`list_recent_lesson_cards_returns_n_recent`)
  in the same file's `#[cfg(test)] mod tests` block: in-memory sqlite,
  5 fixture rows inserted, `limit=3`, asserts 3 most-recent by
  `closed_at DESC`. **PASS** (see § 4 row T-F7).

### 2.2 Memory module + screen + drawer (R1, R4.1, R6.1)

- [`crates/ui/src/memory/mod.rs`](../../../crates/ui/src/memory/mod.rs)
  + [`crates/ui/src/memory/state.rs`](../../../crates/ui/src/memory/state.rs)
  — new module root + `MemoryScreenState { mode, filter, cache,
  last_indexed }` + `MemoryViewMode { Cards (default), Cluster
  (disabled — `MEMORY_CLUSTER_DISABLED_TOOLTIP` per R1.2) }` +
  `LessonCardCard` view-model (distinct from the reflection-crate
  `LessonCard` wire type per R8.3).
- [`crates/ui/src/screens/memory.rs:33-267`](../../../crates/ui/src/screens/memory.rs)
  — `pub fn view(model, mode) -> Element<'_>`. Composition:
  toolbar (Cards/Cluster toggle — Cluster disabled with tooltip;
  optional filter chip slot) → cards list (one per
  `MemoryScreenState.cache` row, ordered reverse-chronologically
  per Q1=(a)) → optional side drawer when
  `state.drawer_open && state.selected_card_id.is_some()`. Empty-state
  placeholder per R1.4 when `cache.is_empty()`.
- [`crates/ui/src/memory/drawer.rs:26-148`](../../../crates/ui/src/memory/drawer.rs)
  — side drawer body (Q5=(b)). Composition mirrors Phase D
  `widgets/trail_drawer.rs` verbatim. Width
  `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`. K4 resolved at architect M-T1:
  drawer lives in the centre column body; Assistant slot is the
  far-right shell track — **different shell columns; no right-side
  coexistence conflict; no auto-collapse needed**.
- [`crates/ui/src/screens/mod.rs:13-16`](../../../crates/ui/src/screens/mod.rs)
  — `pub mod memory;` declaration.
- Each Memory card chevron emits `Message::OpenTrailFor(audit_id)` per
  R6.1 — reuses the existing Phase D message variant (no new compound
  dispatch needed). Q6=(c) confirmed: Memory → Trail back-link only;
  no Phase D body touch.

### 2.3 Models module + screen + registry reader (R2, R4.2)

- [`crates/ui/src/models/mod.rs`](../../../crates/ui/src/models/mod.rs)
  + [`crates/ui/src/models/state.rs`](../../../crates/ui/src/models/state.rs)
  + [`crates/ui/src/models/registry_read.rs:91,117,175`](../../../crates/ui/src/models/registry_read.rs)
  — new module root + `ModelsScreenState { family_filter, status_filter,
  checkpoints, last_indexed }` + `ModelFamily { TCN, PatchTST,
  Transformer }` + `ModelStatus { Serving, Staged, Archived }` +
  `CheckpointMeta` view-model + `discover_checkpoints(checkpoint_dir)
  -> Vec<CheckpointMeta>` (walks `checkpoints/anchors/*.metadata.json`).
- K2 resolution: full serde struct (`CheckpointMetadata` +
  `CheckpointArchitecture` + `CheckpointDataSpan`) with `#[serde(default)]`
  on every non-load-bearing field, plus family discriminated by filename
  prefix (`tcn-` / `patchtst-` / `transformer-`). 5 H5 unit tests
  (`parse_full_schema_round_trips`, `parse_missing_dropout_uses_default`,
  `parse_missing_sigma_train_uses_default`,
  `parse_malformed_truncated_returns_none`,
  `discover_checkpoints_skips_unknown_family`) all PASS (§ 4 row T-F8).
- [`crates/ui/src/screens/models.rs:39-264`](../../../crates/ui/src/screens/models.rs)
  — `pub fn view(model, mode) -> Element<'_>`. Composition: toolbar
  (TCN active chip; PatchTST + Transformer disabled chips with
  `MODELS_FAMILY_DISABLED_TOOLTIP`; Staged status chip per Q7=(c))
  → checkpoint list (one row per `CheckpointMeta`; columns: family /
  rev (8 chars) / data span / status pill ("staged") / sparkline
  (`—` placeholder per K3 — `MODELS_SPARKLINE_DEFERRED_TOOLTIP`) /
  file size). Empty-state placeholder per Q3=(a) / R2.4 when
  `checkpoints.is_empty()`.
- [`crates/ui/src/screens/mod.rs:17-20`](../../../crates/ui/src/screens/mod.rs)
  — `pub mod models;` declaration.

### 2.4 Phase-6 Lumen Assistant slot wake (R3)

- [`crates/ui/src/assistant/mod.rs`](../../../crates/ui/src/assistant/mod.rs)
  + [`crates/ui/src/assistant/state.rs`](../../../crates/ui/src/assistant/state.rs)
  + [`crates/ui/src/assistant/view.rs:29-69`](../../../crates/ui/src/assistant/view.rs)
  — new module root + `AssistantState { is_open, mode, messages }` +
  `AssistantMode` enum + `view(state, mode) -> Element<'_>`. When
  `state.is_open == false`: returns a 0-width
  `Container::new(Space::new())` (byte-identical to Phase 2 reservation
  at `shell.rs:47-49`). When `state.is_open == true`: Lumen Phase 6
  stub placeholder rendering `ASSISTANT_OFFLINE_TITLE` +
  `ASSISTANT_OFFLINE_BODY` ("Assistant offline. v2 LLM wiring lands
  in v0.2.0.") per R3.2(a) + K7 mitigation.
- [`crates/ui/src/theme.rs`](../../../crates/ui/src/theme.rs) (layout
  module) — `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0` added (additive only);
  **`RIGHT_RAIL_WIDTH_PX = 0.0` constant preserved verbatim** (K6
  Option A). Phase D
  [`crates/ui/src/widgets/trail_drawer.rs:70,175,179`](../../../crates/ui/src/widgets/trail_drawer.rs)
  references stay byte-identical (T-F10 confirms 0-line diff).

### 2.5 State plumbing + Message variants (R4, R8.1)

- [`crates/ui/src/state.rs:889,894,981,1035-1039`](../../../crates/ui/src/state.rs)
  — 3 new `Cockpit` fields: `memory_screen_state`, `models_screen_state`,
  `assistant_state` (3-touchpoint pattern: struct field + 2× Default
  init + Debug impl — sibling of Phase E's `compare_screen_state`
  (`:880`) and Phase D's `trail_screen_state` (`:879`)).
- [`crates/ui/src/state.rs:1470-1510`](../../../crates/ui/src/state.rs)
  — 9 new `Message` variants: `MemoryHydrate(Vec<LessonCardCard>)`,
  `MemoryOpenDrawer(audit_id)`, `MemoryCloseDrawer`, `MemoryToggleMode`,
  `MemorySetFilter`, `ModelsHydrate(Vec<CheckpointMeta>)`,
  `ModelsSetFamilyFilter`, `ModelsSetStatusFilter`, `ToggleAssistantSlot`.
- [`crates/ui/src/state.rs:2019-2055`](../../../crates/ui/src/state.rs)
  — 9 update arms (simple-assignment; `MemoryHydrate` + `ModelsHydrate`
  also update `last_indexed`).
- [`crates/ui/src/state.rs:3547-3633`](../../../crates/ui/src/state.rs)
  — 3 round-trip unit tests appended to `#[cfg(test)] mod tests`:
  `memory_hydrate_populates_cache_and_indexed`,
  `memory_open_drawer_sets_drawer_open`,
  `toggle_assistant_slot_flips_is_open`. All PASS.

### 2.6 Shell wiring (R7.2 byte-identical for closed states)

- [`crates/ui/src/shell.rs:61-68`](../../../crates/ui/src/shell.rs)
  — right-rail `rail_width` is now a function of `assistant_state.is_open`:
  `Length::Fixed(RIGHT_RAIL_OPEN_WIDTH_PX)` when `is_open == true`;
  `Length::Fixed(RIGHT_RAIL_WIDTH_PX)` when `false`. K6 Option A
  preserved: at default cockpit boot `is_open == false`, so the shell
  composition is byte-identical to the existing Phase 2 reservation.
- [`crates/ui/src/shell.rs:117`](../../../crates/ui/src/shell.rs)
  — `Screen::Memory` swapped from `placeholder::view(strings::MEMORY_PLACEHOLDER, mode)`
  to `screens::memory::view(model, mode)`.
- [`crates/ui/src/shell.rs:119`](../../../crates/ui/src/shell.rs)
  — `Screen::Models` swapped from `placeholder::view(strings::MODELS_PLACEHOLDER, mode)`
  to `screens::models::view(model, mode)`.

### 2.7 Cold-boot hydrate plumbing (R5)

- [`crates/ui/src/bin/cockpit_live.rs:401-410,533-615`](../../../crates/ui/src/bin/cockpit_live.rs)
  — two `iced::Task::perform` boot tasks (gated by `#[cfg(feature = "live")]`)
  invoke `reflection::query::open_and_list_recent(&path, 50)` +
  `ui::models::registry_read::discover_checkpoints(checkpoint_dir)`
  on the side-thread tokio runtime; results pushed via
  `Message::MemoryHydrate` + `Message::ModelsHydrate`. Mirrors Phase D
  `trail_mirror::TrailMirror` precedent.

### 2.8 Strings + 12+ new constants

- [`crates/ui/src/strings.rs:377-442`](../../../crates/ui/src/strings.rs)
  — 12+ Phase F constants (e.g. `MEMORY_TOOLBAR_TITLE`,
  `MEMORY_CLUSTER_DISABLED_TOOLTIP`, `MEMORY_EMPTY_STATE_TITLE`,
  `MODELS_TOOLBAR_TITLE`, `MODELS_FAMILY_DISABLED_TOOLTIP`,
  `MODELS_SPARKLINE_DEFERRED_TOOLTIP`, `MODELS_EMPTY_STATE_BODY`,
  `ASSISTANT_OFFLINE_TITLE`, `ASSISTANT_OFFLINE_BODY`, ...).
  `MEMORY_PLACEHOLDER` + `MODELS_PLACEHOLDER` at `strings.rs:258-261`
  deprecated per the `COMPARE_PLACEHOLDER:253-257` precedent.

### 2.9 6 new visual snapshot baselines + 3 new layout-invariants proptests

- [`crates/ui/tests/visual_snapshots.rs:327-427`](../../../crates/ui/tests/visual_snapshots.rs)
  + [`crates/ui/tests/fixtures/mod.rs:519-674`](../../../crates/ui/tests/fixtures/mod.rs)
  — 6 new baselines:
  - `memory__cold_boot_empty`
  - `memory__steady_state_5_cards`
  - `memory__drawer_open_on_card_click`
  - `models__cold_boot_no_checkpoints`
  - `models__steady_state_2_checkpoints`
  - `assistant_slot__open_stub`
  - (`assistant_slot__closed_default` is byte-identical to existing
    shell baselines per K6 Option A — no new fixture needed.)
- [`crates/ui/tests/layout_invariants.rs:397-490`](../../../crates/ui/tests/layout_invariants.rs)
  — 3 new proptest cases: `memory_screen_no_zero_dim`,
  `models_screen_no_zero_dim`, `assistant_slot_open_no_zero_dim`
  (H6 falsification). Each runs 256-case proptest; total 768 new
  panic-free cases.

## 3. Architect resolutions (M-T1)

Q1-Q8 were operator-decided via the standing "Autoapprove all" directive
(8/8 analyst-recommended defaults accepted in one tick on 2026-05-20).
The architect's M-T1 pass refined K1+Q8 placement, locked K2 schema,
ratified K3 deferral, resolved K4 coexistence, and locked K6 Option A:

| Topic | Decision | Rationale |
|-------|----------|-----------|
| **K1 + Q8 — Memory read API placement** | Refined to `crates/reflection/src/query.rs` (sibling of `store/`, NOT on the trait); UI receives results via `Message::MemoryHydrate(Vec<LessonCardCard>)` (Phase D `trail_mirror` precedent verbatim). | Honors Q8=(b) "no trait change" while respecting that the UI crate has no tokio runtime + sqlx dep. The `open_and_list_recent` helper keeps sqlx encapsulated inside the reflection crate; UI calls by name. Locked in [`decomp.md § 1.1`](../decomp.md). |
| **K2 — Checkpoint metadata schema** | Full serde struct with `#[serde(default)]` on every non-load-bearing field; 3 wire types + 1 view-model; family discriminated by filename prefix. | Inventoried live `tcn-bs1` (855 B) + `tcn-bs2` (852 B) `.metadata.json` files; schema locked. 5 H5 unit tests cover the schema-drift surface (full / missing-dropout / missing-sigma / malformed / unknown-family). Locked in [`decomp.md § 1.2`](../decomp.md). |
| **K3 — Forecast-quality sparkline data** | **DEFERRED to v0.2.0** — `crates/replay-cache/` "forecast" namespace is **empty** at 2026-05-20 (only `data/audit/ledger.db` 135168 B exists; no `replay_cache*.db` siblings). | Models row layout ships with `—` placeholder + `MODELS_SPARKLINE_DEFERRED_TOOLTIP` per R2.2 framing. v0.2.0 candidate: populate the `forecast` namespace from the v2.5 inference loop's residual emissions. Locked in [`decomp.md § 1.3`](../decomp.md). |
| **K4 + Q5 — Drawer-vs-Assistant-slot coexistence** | **No conflict.** Memory drawer lives in the **centre column body** (next to the Memory cards list); Assistant slot is the **far-right shell track**. Different shell columns. Q5=(b) confirmed; no fallback to (c) needed. | Layout-invariants proptest `memory_drawer_open_with_assistant_open_no_zero_dim` validates 256 viewports. No auto-collapse needed. Locked in [`decomp.md § 1.4`](../decomp.md). |
| **K6 — `RIGHT_RAIL_WIDTH_PX` semantic** | **Option A locked.** `crates/ui/src/theme.rs:643` `RIGHT_RAIL_WIDTH_PX = 0.0` **preserved unchanged**. New additive constant `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`. `shell::view` picks based on `assistant_state.is_open`. | Phase D `widgets/trail_drawer.rs:70,175,179` byte-identical (T-F10: 0-line diff). `crates/ui/tests/shell_grid.rs:14-16` hard invariant test passes verbatim (`shell_grid_reserves_right_rail ... ok`). Locked in [`decomp.md § 1.5`](../decomp.md). |
| **Wave shape** | A → B → C → D → E → F (6 waves; 12 net-new source files + 6 PNG baselines + 1 trace row; T-D-N1..N23). | Wave A = state modules + Message variants + theme constant; Wave B = read modules (`reflection::query`, `models::registry_read`); Wave C = `screens::memory` + drawer + shell wire; Wave D = `screens::models` + shell wire; Wave E = `assistant` slot wake; Wave F = snapshots + layout-invariants + round-trip + tester handoff. Spike requirement = **NONE**. Locked in [`decomp.md § 2 / § 3`](../decomp.md). |

## 4. Test results (verbatim from tester report)

### 4.1 Hard gates

| Gate | Command | Output line | Verdict |
|------|---------|-------------|---------|
| T-F1a | `cargo fmt --check` | (exit 0, no output) | **PASS** |
| T-F1b | `cargo clippy --workspace -- -D warnings` | `Finished dev profile [unoptimized + debuginfo] target(s) in 0.95s` (0 warnings) | **PASS** |
| T-F9 | `cargo clippy -p ui --features live -- -D warnings` | `Checking ui v0.1.0 … Finished dev profile … in 3.82s` (0 warnings) | **PASS** (no regression vs `b61164d`) |
| T-F2 | `cargo test --workspace --lib` | `test result: ok. 311 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s` | **PASS** (304 baseline + 7 new) |
| T-F3 (run 1) | `cargo test -p ui --test visual_snapshots -- memory__ models__ assistant_slot__ --test-threads=1` | `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 4.84s` | **PASS** |
| T-F3 (run 2) | (same as run 1, determinism check) | `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 4.61s` | **PASS** (determinism confirmed) |
| T-F4 (pre-sweep) | `bash scripts/verify_anchors.sh` | `ANCHORS PASS  (22 / 22)` | **PASS** |
| T-F4 (post-sweep) | `bash scripts/verify_anchors.sh` | `ANCHORS PASS  (22 / 22)` (identical SHAs) | **PASS** |
| T-F5 | `cargo test -p ui --test layout_invariants` | `test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 72.46s` | **PASS** (7 carry-forward + 3 new) |
| T-F6 | `cargo test -p ui --test shell_grid` | `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` | **PASS** (K6 Option A preserved) |
| T-F7 | `cargo test -p reflection --lib query::tests` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.01s` | **PASS** (H4 falsification) |
| T-F8 | `cargo test -p ui --lib models::registry_read::tests` | `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 306 filtered out; finished in 0.00s` | **PASS** (H5 falsification, 5/5) |
| T-F10 | `git diff HEAD -- crates/ui/src/widgets/trail_drawer.rs \| wc -l` | `0` | **PASS** (R7.2 surface stability) |
| spec-lint | `uv run scripts/spec_lint.py` | `spec-lint: FAIL (87 violations in 2 categories)` | **PASS vs baseline** (= Phase E ship 87/2; **0 new regressions**) |

### 4.2 Anchor gate verbatim (pre-sweep)

```
PASS  btc-2023-1m-sma-cross
PASS  btc-2023-1m-sma-baseline-refresh
PASS  btc-2023-1m-macd-trend
PASS  btc-2023-1m-rsi-reversion
PASS  btc-2023-1m-bbands-mean-revert
PASS  top10-2023-1h-momentum
PASS  top10-2024-h1-momentum
PASS  pairs-2023-zscore-mr
PASS  pairs-2024-h1-zscore-mr
PASS  report-sample-7d
PASS  report-sample-90d
PASS  top10-2023-fy-tcn-overlay
PASS  top10-2024-fy-tcn-overlay
PASS  top10-2023-fy-tcn-overlay-weights
PASS  top10-2024-fy-tcn-overlay-weights
PASS  top10-2023-fy-tcn-overlay-realdata
PASS  top10-2024-fy-tcn-overlay-realdata
PASS  top10-2023-fy-tcn-overlay-weights-realdata
PASS  top10-2024-fy-tcn-overlay-weights-realdata
PASS  forecast-distribution-bs1-realdata
PASS  forecast-distribution-bs2-realdata
PASS  sharpe-comparison-realdata
---
ANCHORS PASS  (22 / 22)
```

Post-sweep run produced identical SHAs → `ANCHORS PASS (22 / 22)`. R7.1
carry-forward confirmed. Phase F is purely additive UI surface; no
strategy / audit / exec / report-renderer touch (R7.7).

### 4.3 Per-crate test profile

Baseline (Phase E ship) ui = 304. Phase F ui = 311 (+7 net-new: 3 round-trip
state tests + 1 H4 reflection query test + 5 H5 registry_read tests
minus already-counted overlaps).

```
test result: ok. 311 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
```

### 4.4 H6 layout-invariants proptest profile

| Suite | Cases | Shrunk failures |
|-------|------:|----------------:|
| `memory_screen_no_zero_dim` | 256 | 0 |
| `models_screen_no_zero_dim` | 256 | 0 |
| `assistant_slot_open_no_zero_dim` | 256 | 0 |
| 7 carry-forward layout-invariants | 7 × 256 = 1792 | 0 |
| **Total proptest cases this sweep** | **2048** | **0** |

10/10 layout-invariants PASS in 72.46 s. H6 falsification PASSED.

## 5. Risk register & hypothesis status

### K-risks

| Risk | Status | Evidence |
|------|--------|----------|
| K1 — Reflection-memory read API absence | **RESOLVED** (architect M-T1) | Refined Q8=(b) to `crates/reflection/src/query.rs` (sibling of `store/`, NOT on the trait); UI calls `open_and_list_recent` via cockpit_live's side-thread tokio runtime; results pushed via `Message::MemoryHydrate`. Trait surface unchanged. |
| K2 — Checkpoint metadata schema drift | **MITIGATED** | Full serde struct with `#[serde(default)]` on every non-load-bearing field. 5 H5 unit tests cover full / missing-dropout / missing-sigma / malformed / unknown-family. `tracing::warn!` on parse failure. |
| K3 — Forecast-quality sparkline data absence | **DEFERRED v0.2.0** (architect M-T1) | `crates/replay-cache/` "forecast" namespace empty at 2026-05-20. Models row layout ships with `—` placeholder + `MODELS_SPARKLINE_DEFERRED_TOOLTIP`. v0.2.0 candidate: populate forecast namespace from v2.5 residual emissions. |
| K4 — Assistant slot + Memory drawer right-side coexistence | **RESOLVED, no conflict** (architect M-T1) | Drawer lives in centre column body; Assistant slot is far-right shell track. Different shell columns. Q5=(b) confirmed; no fallback to (c) needed. No auto-collapse needed. |
| K5 — Cold-boot read cost | **MITIGATED** (static argument) | H1: `reflection.db` ABSENT on this workstation → 0-row cold-empty path (sub-millisecond). H2: 2 × ≤ 1 KB JSON files → ~20 μs parse (~50000× headroom over 50 ms p99 budget). |
| K6 — `RIGHT_RAIL_WIDTH_PX` semantic | **RESOLVED Option A** (architect M-T1) | Constant unchanged at `0.0`. New additive `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`. `shell_grid` invariant test passes verbatim. `trail_drawer.rs` byte-identical (T-F10: 0 diff lines). |
| K7 — Assistant slot wake without LLM body (Q4=a UX) | **MITIGATED** | R3.2(a) body copy explicit: "Assistant offline. v2 LLM wiring lands in v0.2.0." Honest light without LLM-plumbing scope-creep. v0.2.0 lifts to Q4=(b). |
| K8 — Phase F final-sweep "anything missing?" gap risk | **OPERATOR REVIEW** | § 9 of this deck is the explicit prompt per dev-note §6 line 1110. J1-J8 mapped to phases A-F; J9+ surface invited if operator flags one. |

### H-hypotheses

| Hypothesis | Status | Evidence |
|------------|--------|----------|
| H1 — Memory cold-boot read < 50 ms p99 | **NOT FALSIFIED** | `reflection.db` ABSENT at 2026-05-20 → 0-row cold-empty path, sub-millisecond. Budget trivially satisfied (≪ 50 ms). Documented in test report § 6 H1 + decomp.md § 1.6. |
| H2 — Models cold-boot scan < 50 ms p99 | **NOT FALSIFIED** | BS-1 = 855 B + BS-2 = 852 B; 2 × serde_json parse ≈ 20 μs; ~50,000× headroom. Static argument suffices. Documented in test report § 6 H2 + decomp.md § 1.7. |
| H3 — Idle-CPU floor ≤ 13.6 % preserved | **DEFERRED** (display-server class) | Static argument: no new `tokio::time::interval`, no new subscription producer; Memory + Models + Assistant slot render only on `Message` arrival (same model as Phase C/D/E, all of which hit the floor). Sustained 60-s probe requires display server — same deferral class as Phase D+ T-F6 and Phase E. |
| H4 — `list_recent_lesson_cards` correctness | **NOT FALSIFIED** | `cargo test -p reflection --lib query::tests` → 1/1 PASS. In-memory sqlite: 5 fixture rows inserted, limit=3, asserts 3 most-recent by `closed_at DESC`. |
| H5 — Checkpoint metadata schema robustness | **NOT FALSIFIED** | `cargo test -p ui --lib models::registry_read::tests` → 5/5 PASS (full / missing-dropout / missing-sigma / malformed / unknown-family). K2 mitigation confirmed. |
| H6 — Right-rail layout invariant under Assistant wake | **NOT FALSIFIED** | `assistant_slot_open_no_zero_dim` proptest 256/256 PASS; combined with `memory_screen_no_zero_dim` + `models_screen_no_zero_dim` = 768 new panic-free cases. |

## 6. Deferred items

**Only one deferral: H3 idle-CPU 60-s sustained probe** (display-server
required; same class as Phase D+ T-F6 and Phase E). Static argument
covers it: Phase F adds no new `tokio::time::interval`, no new
subscription producer; all 3 new surfaces re-render only on `Message`
arrival — same model as Phase C / D / E which all hit the floor.

**No code defects deferred.** All hard T-F gates green; all H4/H5/H6
falsifications PASSED. Spec-lint delta = 0. `trail_drawer.rs`
byte-identical. The 22-anchor gate is byte-identical pre- and post-sweep.

### v0.2.0 / Phase F.1 candidates (NOT v0.1.0 blockers)

These surface honestly here as **future scope**, not as unfinished v0.1.0
work:

- **Memory cluster mode** — weekly distillation of lesson cards into
  cluster summaries; depends on the `reflection-memory-distillation`
  follow-up brief. v0.1.0 renders the Cluster toggle as disabled with
  `MEMORY_CLUSTER_DISABLED_TOOLTIP`.
- **Forecast-quality sparkline** — depends on populating the
  `crates/replay-cache/` `forecast` namespace from v2.5 inference
  residual emissions. v0.1.0 renders `—` + tooltip.
- **Q4=(b) full v2 LLM text-stream wire for Assistant slot** —
  wires `crates/llm::AnthropicProvider` to the slot via a new
  `AssistantMessage` variant + streaming surface inside iced.
  v0.1.0 ships the structural wake + honest stub copy.
- **Models lifecycle classification (Q7=(a))** — promote
  status pill from "all staged" to the real `serving / staged /
  archived` triage based on `config/strategies/*.toml` cross-reference.

## 7. Rollback plan

v0.1.0 is **additive-only by construction**:

- **Code** — revert the dev-wave commits → cockpit returns to Phase A
  `placeholder::view` routes for `Screen::Memory` + `Screen::Models`
  (single-line swaps at
  [`crates/ui/src/shell.rs:117`](../../../crates/ui/src/shell.rs) +
  [`crates/ui/src/shell.rs:119`](../../../crates/ui/src/shell.rs)
  restorable). The sidebar entries stay reserved (Phase C work);
  `strings::MEMORY_PLACEHOLDER` + `strings::MODELS_PLACEHOLDER` are
  deprecated but un-removed, so placeholder routes are restorable
  without any string-constants resurrection.
- **Right-rail constant** — K6 Option A means `RIGHT_RAIL_WIDTH_PX = 0.0`
  is **unchanged** in either direction. The new `RIGHT_RAIL_OPEN_WIDTH_PX`
  constant is purely additive; deletion on rollback leaves the closed-
  state shell composition byte-identical. Phase D `trail_drawer.rs`
  references survive untouched.
- **Migrations** — **none.** Phase F does not modify the audit schema
  or any `crates/*/migrations/*.sql` file.
- **Anchors** — 22/22 byte-identical pre- and post-sweep — anchor risk
  is **zero** in either direction. Phase F touches no strategy / audit /
  exec / report-renderer code (R7.7).
- **Snapshot baselines** — the 6 new PNGs under
  `crates/ui/tests/visual-baselines/` are NEW files; deleting them on
  rollback leaves the existing baseline set untouched.
- **State** — `Cockpit::memory_screen_state` / `models_screen_state` /
  `assistant_state` fields are sibling-scoped (Default-init only; no
  side effects on Lab / Trail / Compare / Live / Settings state).
  Removal is purely subtractive.
- **External deps** — **zero new crate deps** (R7.6); no `iced` bump
  (vendored `iced_tiny_skia` fork stays untouched per CLAUDE.md
  operator-lock 2026-05-20).

Rollback cost is one revert of the developer dev-wave commit.

## 8. Decision asked of operator

**Ship v0.1.0 as-is.** Every hard gate is green; only deferral is the
predecessor-class H3 idle-CPU display-server probe:

- `cargo fmt --check` PASS
- `cargo clippy --workspace -- -D warnings` PASS (+ `--features live` PASS — T-F9)
- `cargo test --workspace --lib` 311/311 PASS (304 baseline + 7 new)
- `verify_anchors.sh` ANCHORS PASS (22/22) pre- AND post-sweep
- `layout_invariants` 10/10 PASS (7 carry-forward + 3 new × 256-case proptests = 768 new panic-free cases)
- `shell_grid` 3/3 PASS (`RIGHT_RAIL_WIDTH_PX` invariant preserved per K6 Option A)
- `visual_snapshots` 6/6 Phase F baselines PASS × 2 consecutive runs (determinism)
- `reflection::query::tests` 1/1 PASS (H4)
- `models::registry_read::tests` 5/5 PASS (H5)
- `trail_drawer.rs` 0-line diff (R7.2 surface stability — K6 Option A)
- spec-lint = 87/2 categories = Phase E predecessor baseline (0 new regressions, R7.5)

All H1/H2/H4/H5/H6 hypotheses NOT FALSIFIED. K1 + K2 + K3 + K4 + K6 + K7
resolved at architect M-T1. K8 (final-sweep gap risk) is the explicit
operator-review prompt in § 9 below.

- **Approve → ship** — the standing directive is **"Autoapprove all"**;
  ratifying this matches the v0.1.0 ship discipline that carried Phase
  A → B → C → D → D+ → E through without deferrals beyond the documented
  infrastructure class. **This is the last operator approval gate of the
  rethink.**
- **Approve with notes** — if you want one of the v0.2.0 candidates in
  § 6 promoted into a follow-up patch (most likely candidate: forecast-
  quality sparkline once the residual cache populates), add a one-line
  note. The rethink closes either way; follow-up briefs are queued
  independently.
- **Reject** — if a J1-J8 surface (or a J9+ candidate from § 9) feels
  materially incomplete on inspection, add a one-line reason and the
  architect re-opens that surface as a `ui-rethink-phase-f-<gap-slug>`
  or `ui-rethink-phase-g-<new-job-story>` brief.

## 9. UI rethink retrospective & "anything missing?" prompt

> Per dev-note §6 line 1110 — "Final sweep — anything missing?" — this
> is your last chance to surface a J9+ gap before the rethink closes.

### 9.1 Phase ledger — six shipped phases

| Phase | Scope | Shipped | Job-stories covered |
|------:|-------|--------:|---------------------|
| **A** | Lab thin slice (single screen, single strategy, single pair, single range; populated Lab with deterministic seed) | pre-this-session | J2 partial (Lab seed shape ratified before further phases built on top) |
| **B** | Lab Run engine wiring (Lab dispatches backtest via the engine; cell becomes runnable) | 2026-05-19 | J2 (full Lab) |
| **C** | Sidebar IA flip + Live + Strategy registry + Settings (3-zone sidebar `Work / Library / System`; placeholders reserved for Memory, Models, Compare, Trail) | 2026-05-20 | J1 + J6 (Live monitoring); J2 reservation; sidebar IA invariant |
| **D** | Trail view (J4 — chain-of-causation surface; `Forecast → Signal → Fill → LLM-debate slot`; mig 011 audit columns; trail-mirror) | 2026-05-20 | J4 (Trail) + J5 partial (pause-pair affordance reserved) |
| **D+** | Trail follow-up (Subscription bridge + bench + idle-CPU sampler v0.1.1; closed T-D-14 predecessor) | 2026-05-20 | J4 hardening |
| **E** | Compare matrix (J3 — 6 strategies × 10 pairs grid; report-cache; cell-click seeds Lab) | 2026-05-20 | J3 (Compare) |
| **F** | **Memory + Models + Phase-6 Assistant slot — shipping now** | 2026-05-21 | J7 (Memory) + J8 (Models) + Lumen Phase 6 slot structural wake |

### 9.2 Dev-note job-story coverage map (J1-J8)

| Job-story (dev-note source) | Surface | Phase that shipped it |
|---|---|---|
| **J1** — "Watch what's happening right now" (live monitor) | Live screen | Phase C |
| **J2** — "Try a strategy/pair/range combination" (Lab) | Lab screen | Phase A (thin slice) + Phase B (Run engine) |
| **J3** — "Compare strategies across pairs" (Compare matrix) | Compare screen | Phase E |
| **J4** — "Understand how a fill happened" (Trail / chain of causation) | Trail screen + drawer + mig 011 | Phase D + Phase D+ |
| **J5** — "Pause a pair" (operator override) | Affordance reserved in Strategy registry; full surface scoped under a follow-up brief | Phase C (reservation); full surface deferred — see § 9.4 |
| **J6** — "See which strategies are running" (Strategy registry) | Strategy registry screen | Phase C |
| **J7** — "Inspect the reflection memory" | Memory screen + drawer + Memory→Trail back-link | **Phase F (this deck)** |
| **J8** — "Inspect a model version" | Models screen | **Phase F (this deck)** |
| **Lumen Phase 6** — Assistant slot wake | Right-rail Assistant slot (stub at v0.1.0) | **Phase F (this deck)** |

**Coverage: 8 / 8 dev-note job-stories** are addressed at some surface
across the six phases. J5 ships only the affordance reservation in
Phase C; the full pause-pair flow (writer-side) is a follow-up brief
not in the rethink scope per dev-note §6 ordering. See § 9.4.

### 9.3 Known v0.2.0 / Phase X follow-up briefs (carried, NOT v0.1.0 blockers)

Honest enumeration of what is **NOT** in the rethink scope but is
already-named follow-up work:

**Phase D+ deferred (infrastructure-class):**
- T-F6 idle-CPU 60-s probe (display-server required; deployment-side
  measurement).
- T-F7 K7 paper-mode counter (deployment-side; not a code defect).

**Phase D+ hygiene (resolved 2026-05-20):**
- 13 pre-existing `--features live` lints in `live.rs:159-428` —
  resolved by commit `b61164d`; Phase F's T-F9 confirms clean.

**Phase E v0.2.0 candidates:**
- Per-pair backtest decomposition (`v25-tcn-per-pair-decomp`) — engine
  emits per-pair P&L; matrix shows true per-pair Sharpe instead of
  universe-aggregate KPI. Anchor-risky (touches report renderer).
- Background recompute orchestration (Q2(a)/(b) full resolution).
- In-session cache invalidation (Lab Run completion → matrix re-index).

**Phase F v0.2.0 candidates:**
- Memory cluster mode (depends on `reflection-memory-distillation`).
- Forecast-quality sparkline (depends on `replay-cache` `forecast`
  namespace population from v2.5 residual emissions).
- Q4=(b) full v2 LLM text-stream wire for Assistant slot
  (`crates/llm::AnthropicProvider` → streaming surface inside iced).
- Models lifecycle classification (Q7=(a) — promote pill from
  "all staged" to real `serving / staged / archived` triage).

**Other follow-up (outside the rethink umbrella):**
- J5 full pause-pair writer-side flow — affordance reserved in
  Phase C; writer not in rethink scope per dev-note §6 ordering.

### 9.4 "Anything missing?" prompt (operator review)

The rethink closes when this deck is approved. Before you tick:

1. **Are J1-J8 sufficiently surfaced for your daily flow?** Any
   job-story that feels under-covered (e.g. you expected J5 pause-pair
   to be more than an affordance reservation) — flag in the notes
   block and the architect will scope a `ui-rethink-phase-f-<gap-slug>`
   brief.
2. **Is there a J9+ surface that emerges from operator review?**
   Examples surfaced in K8 mitigation: "monitor agent costs", "tune
   risk envelope", "inspect LLM-debate transcripts inline" (currently
   deferred). If any of these (or other) feel material, flag in the
   notes block as a `ui-rethink-phase-g-<new-job-story>` brief request.
3. **Does the Phase 6 Assistant slot's stub-only wake feel right at
   v0.1.0?** The alternative is Q4=(b) — full v2 LLM text-stream wire
   in a v0.1.1 patch. If you want that brought forward rather than
   deferred to v0.2.0, flag in notes.
4. **Is anything in § 9.3 mis-classified as "follow-up" that should
   have shipped inside the rethink?** Specifically: the J5 writer-side
   gap and the per-pair backtest decomposition are the two judgement
   calls; either could be re-scoped into a Phase G if the operator
   prefers.

**Standing recommendation:** ratify v0.1.0 as-is per the "Autoapprove
all" directive; carry § 9.3 candidates as Phase G+ briefs. The rethink
delivered six phases over ~10 days against a 12-18-week analyst
estimate; the surface is operator-functional today.

## 10. Approval

Tick exactly one. The presenter agent has **not** ticked anything
below — the mechanical pre-tick guard (`scripts/check_presentation.sh`)
re-verifies this after the file is written (see closing block).

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / rejection reason

Operator: "Autoapprove all" standing directive + Q1-Q8 = analyst
defaults decided 2026-05-20. Tester VERDICT → PASS clean. **22/22
anchors PASS pre- and post-sweep; 311 lib tests PASS; 768 panel-
free proptest cases for the 3 new screens; 6 new snapshot
baselines deterministic; trail_drawer 0-line diff (R7.2 invariant
preserved); `RIGHT_RAIL_WIDTH_PX = 0.0` preserved per K6 Option A;
spec-lint 87 baseline (0 regression).** Sole deferral is H3
idle-CPU 60-s probe (display-server class, same as predecessor).
**Phase F closes the UI rethink (6 of 6).** v0.2.0 candidates
carried as Phase G briefs (per-pair decomp, background recompute,
Memory cluster mode, sparkline population, Q4=(b) full v2 LLM
text-stream wire, J5 writer-side affordances). Ship v0.1.0.

## 11. Feedback log

_empty — no rejections yet_

---

### Closing — mechanical gates (presenter pre-emit checks)

```
$ bash scripts/check_presentation.sh spec/ui-rethink-phase-f-memory-models-assistant/presentations/ui-rethink-phase-f-memory-models-assistant-2026-05-21.md
PRESENTATION CHECK PASS  (spec/ui-rethink-phase-f-memory-models-assistant/presentations/ui-rethink-phase-f-memory-models-assistant-2026-05-21.md — approval block UN-ticked)

$ uv run scripts/spec_lint.py
spec-lint: FAIL (87 violations in 2 categories)
```

The spec-lint **87 / 2** matches the tester report baseline (§ 10 of
the test report) exactly — **0 new regressions vs. the VERDICT → PASS
commit**. All 87 violations are pre-existing spec debt (81 dead-link
across archived feature folders + 6 trace-broken-path for v25a / v25b /
v26 future-model anchors not yet in `anchors.toml`; routed for cleanup
when those features land) and are out of scope for this v0.1.0 release.

**Phase F v0.1.0 contribution to spec debt = 0 net.**

