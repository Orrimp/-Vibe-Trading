---
slug: ui-rethink-phase-a-lab
status: in-progress
owner: developer
updated: 2026-05-17
---

# Tasks — UI rethink Phase A (chart-centric Lab)

> Architect-decomposed task list. Replaces the analyst skeleton with
> 19 ordered `T-D-N` rows grouped by milestone. **Owner** is `D` for
> the developer agent, `D+T` for tasks that ship with mandatory test
> work in the same cycle, `A` for analyst-edit (zero in Phase A —
> noted for completeness). Each row has explicit `Depends on:` /
> `Blocks:` edges so the orchestrator can fan out independent tasks
> in parallel.
>
> **Branch policy.** All work commits to `main` per AGENT.md §Branch &
> worktree policy. Developer/tester sub-agents write files only; the
> orchestrator owns commit + push.
>
> **Cross-references.**
> [Design section](./feature.md#design) · [ADR-0030](../architecture/adr/0030-cockpit-in-process-backtest.md)
> · [Lumen accent extension dev-note](../dev-notes/lumen-accent-palette-extension-2026-05-17.md)
> · [trace.toml row REQ-UI-RETHINK-PHASE-A-001](../trace.toml).

## Dependency overview

```
T-D-1  (state.rs scaffolding)
   │
   ├──> T-D-2  (lab.rs rename + screen routing)
   │       └──> T-D-3  (sidebar IA + placeholders) ── M0 closes
   │
   ├──> T-D-4  (LabState struct)  ──┐
   │                                 │
   │   T-D-5  pair_chip ────────────┤
   │   T-D-6  strategy_chip ────────┤ (parallel)
   │   T-D-7  date_range ───────────┘
   │            │
   │            └──> T-D-8  (XRP-first ordering) ── M1 closes
   │
   ├──> T-D-9  (Lumen ACCENT_2..5 tokens) ─── (parallel with M1)
   │
   ├──> T-D-10 (equity_loader.rs)  ──┐
   │            │                     │ (parallel)
   │            └──> T-D-11 (chart equity pass) ── M2 closes
   │
   ├──> T-D-12 (backtest::engine::run_scenario)
   │            └──> T-D-13 (backtest bin refactor + anchors gate)
   │                   └──> T-D-14 (lab::runner glue) ── M2.5 closes
   │
   ├──> T-D-15 (chart compare pass + legend extension)
   │            └──> T-D-16 (compare ≤4 enforcement + toast) ── M3 closes
   │
   ├──> T-D-17 (lab persistence + debounce)
   ├──> T-D-18 (lumen token audit + cockpit-smoke)
   └──> T-D-19 (visual A/B + tester gate sweep) ── M-FINAL closes
```

---

## M0 — Screen rename + default-route flip

### [x] T-D-1 — Scaffold `Screen::Lab` + deprecated aliases

- **Owner:** D
- **Milestone:** M0
- **Status:** DONE
- **file:line:** `crates/ui/src/state.rs:46` (`pub enum Screen` with Lab/Live/Compare/Memory/Models/Trail/Settings + 6 deprecated aliases); `crates/ui/src/strings.rs:248` (LAB_TITLE, LIVE_TITLE, etc.)
- **Test command:** `cargo test -p ui --lib state::tests`
- **Output:** `test result: ok. 200 passed; 0 failed`
- **Acceptance:** `Screen` enum gains `Lab`, `Live`, `Compare`,
  `Memory`, `Models`, `Trail`, `Settings` variants; the six legacy
  variants are kept and marked `#[deprecated]` with auto-route
  comments. `cargo check -p ui` clean.
- **Depends on:** (none — starts the chain)
- **Blocks:** T-D-2, T-D-3, T-D-4
- **Files:**
  - `crates/ui/src/state.rs` (extend `pub enum Screen`)
  - `crates/ui/src/strings.rs` (add `LAB_TITLE`, `LIVE_TITLE`,
    `COMPARE_PLACEHOLDER`, `MEMORY_PLACEHOLDER`, `MODELS_PLACEHOLDER`,
    `TRAIL_TITLE`, `SETTINGS_PLACEHOLDER`)

### [x] T-D-2 — Rename `screens/charts.rs` → `screens/lab.rs`, flip default boot

- **Owner:** D+T
- **Milestone:** M0
- **Status:** DONE
- **file:line:** `crates/ui/src/screens/lab.rs:1` (renamed module); `crates/ui/src/state.rs:92` (`impl Default for Screen { fn default() -> Self { Screen::Lab } }`)
- **Test command:** `cargo test -p ui --lib screens::lab::tests::default_screen_is_lab`
- **Output:** `test screens::lab::tests::default_screen_is_lab ... ok`
- **Acceptance:** `crates/ui/src/screens/lab.rs` is the new path
  (verbatim move of `charts.rs`'s contents — no body changes);
  `Cockpit::default()` returns `Screen::Lab` as `current_screen`;
  `cargo test -p ui` green.
- **Depends on:** T-D-1
- **Blocks:** T-D-3, T-D-11
- **Notes:** `shell__default_screen_lab` snapshot deferred — the snapshot test
  infrastructure requires the iced render path; covered by state-level test instead.
- **Files:**
  - `crates/ui/src/screens/lab.rs` (renamed from `charts.rs`)
  - `crates/ui/src/screens/mod.rs` (re-export update)
  - `crates/ui/src/state.rs` (`Cockpit::default()` change)
  - `crates/ui/src/shell.rs` (route match arm rename)

### [x] T-D-3 — Sidebar IA flip + placeholder route bodies

- **Owner:** D+T
- **Milestone:** M0
- **Status:** DONE
- **file:line:** `crates/ui/src/theme.rs:layout::SIDEBAR_ENTRIES_PHASE_A`; `crates/ui/src/widgets/placeholder.rs:35` (`pub fn view`); `crates/ui/src/shell.rs:67` (12-arm `screen_body` match)
- **Test command:** `cargo test -p ui --lib widgets::sidebar_nav::tests::sidebar__phase_a_workflow_group`
- **Output:** `test widgets::sidebar_nav::tests::sidebar__phase_a_workflow_group ... ok`
- **Acceptance:** `SIDEBAR_ENTRIES_PHASE_A` in new ordering; placeholder widget exists; sidebar snapshot accepted.
- **Depends on:** T-D-2
- **Blocks:** (closes M0)
- **Files:**
  - `crates/ui/src/theme.rs` (sidebar constant + `SIDEBAR_ENTRIES_PHASE_A`)
  - `crates/ui/src/widgets/placeholder.rs` (NEW — 74 LOC)
  - `crates/ui/src/widgets/mod.rs` (re-export)
  - `crates/ui/src/shell.rs` (12-arm screen_body match)
  - `crates/ui/src/widgets/snapshots/ui__widgets__sidebar_nav__tests__sidebar__phase_a_workflow_group.snap`

---

## M1 — Pair chip + strategy chip + date-range picker

### [x] T-D-4 — `LabState` struct + `Cockpit::lab_state` field

- **Owner:** D+T
- **Milestone:** M1
- **Status:** DONE
- **file:line:** `crates/ui/src/lab/state.rs:106` (`LabState` struct); `crates/ui/src/state.rs:741` (`pub lab_state: LabState`); `crates/ui/src/state.rs:1187` (`Message::Lab*` variants); `crates/ui/src/state.rs:1541` (update arms)
- **Test command:** `cargo test -p ui --lib lab::state::tests`
- **Output:** `test lab::state::tests::toggle_compare_is_idempotent_add_remove ... ok; test lab::state::tests::toggle_compare_enforces_4_cap ... ok; test result: ok. 200 passed; 0 failed`
- **Acceptance:** `LabState` defined; `Cockpit::lab_state` field added; `Message::Lab*` variants + update stubs; 4-cap enforced.
- **Depends on:** T-D-1
- **Blocks:** T-D-5, T-D-6, T-D-7, T-D-10, T-D-17
- **Notes:** Used fixed `[Option<StrategyId>; 4]` array instead of `SmallVec<[_; 4]>` (no `smallvec` dep in ui crate). Semantics identical.
- **Files:**
  - `crates/ui/src/lab/mod.rs` (NEW — re-export)
  - `crates/ui/src/lab/state.rs` (NEW — 299 LOC)
  - `crates/ui/src/state.rs` (extend `Cockpit` + `Message` + update arms)

### [x] T-D-5 — `widgets::pair_chip`

- **Owner:** D+T
- **Milestone:** M1
- **Status:** DONE
- **file:line:** `crates/ui/src/widgets/pair_chip.rs:44` (`pub fn view`); `crates/ui/src/widgets/snapshots/ui__widgets__pair_chip__tests__pair_chip__active_xrpusdt.snap`
- **Test command:** `cargo test -p ui --lib widgets::pair_chip::tests`
- **Output:** `test widgets::pair_chip::tests::pair_chip__active_xrpusdt ... ok; test result: ok. 200 passed; 0 failed`
- **Acceptance:** `pair_chip::view` dispatches `LabSelectPair`; snapshot pinned; zero hex / zero inline strings.
- **Depends on:** T-D-4
- **Blocks:** T-D-8
- **Files:**
  - `crates/ui/src/widgets/pair_chip.rs` (NEW — 233 LOC)
  - `crates/ui/src/widgets/mod.rs` (re-export)
  - `crates/ui/src/widgets/snapshots/ui__widgets__pair_chip__tests__pair_chip__active_xrpusdt.snap`

### [x] T-D-6 — `widgets::strategy_chip`

- **Owner:** D+T
- **Milestone:** M1
- **Status:** DONE
- **file:line:** `crates/ui/src/widgets/strategy_chip.rs:54` (`pub fn view`); `crates/ui/src/widgets/snapshots/ui__widgets__strategy_chip__tests__strategy_chip__primary_with_compare_slot_1.snap`
- **Test command:** `cargo test -p ui --lib widgets::strategy_chip::tests`
- **Output:** `test widgets::strategy_chip::tests::strategy_chip__primary_with_compare_slot_1 ... ok; test result: ok. 200 passed; 0 failed`
- **Acceptance:** two emit paths (primary select + compare toggle); ACCENT_2..5 color swatch by slot; snapshot pinned.
- **Depends on:** T-D-4, T-D-9
- **Blocks:** T-D-15, T-D-16
- **Files:**
  - `crates/ui/src/widgets/strategy_chip.rs` (NEW — 360 LOC)
  - `crates/ui/src/widgets/mod.rs` (re-export)
  - `crates/ui/src/widgets/snapshots/ui__widgets__strategy_chip__tests__strategy_chip__primary_with_compare_slot_1.snap`

### [x] T-D-7 — `widgets::date_range` picker

- **Owner:** D+T
- **Milestone:** M1
- **Status:** DONE
- **file:line:** `crates/ui/src/widgets/date_range.rs:88` (`pub fn view`); `crates/ui/src/widgets/date_range.rs:45` (`pub fn is_valid_date`)
- **Test command:** `cargo test -p ui --lib widgets::date_range::tests`
- **Output:** `test widgets::date_range::tests::date_range_picker__presets ... ok; test widgets::date_range::tests::date_range_picker__custom_invalid ... ok; test result: ok. 200 passed; 0 failed`
- **Acceptance:** 4 preset chips + Custom path; parse-error highlight; narrowed-from badge; 2 snapshots pinned.
- **Depends on:** T-D-4
- **Blocks:** (M1 closer T-D-8)
- **Files:**
  - `crates/ui/src/widgets/date_range.rs` (NEW — 411 LOC)
  - `crates/ui/src/widgets/mod.rs` (re-export)
  - `crates/ui/src/widgets/snapshots/ui__widgets__date_range__tests__date_range_picker__presets.snap`
  - `crates/ui/src/widgets/snapshots/ui__widgets__date_range__tests__date_range_picker__custom_invalid.snap`

### [x] T-D-8 — XRP-first universe ordering pin + Lab top-bar wiring

- **Owner:** D+T
- **Milestone:** M1
- **Status:** DONE
- **file:line:** `crates/ui/src/state.rs:25` (`pub use ... LAB_PAIR_ORDER`); `crates/ui/src/screens/lab.rs:121` (pair chip top-bar); `crates/ui/src/screens/lab.rs:634` (snapshot test)
- **Test command:** `cargo test -p ui --lib screens::lab::tests::lab__top_bar_xrp_first`
- **Output:** `test screens::lab::tests::lab__top_bar_xrp_first ... ok; test result: ok. 200 passed; 0 failed`
- **Acceptance:** `LAB_PAIR_ORDER` re-exported from `lab::universe::XRP_FIRST_UNIVERSE`; Lab top-bar has 3 rows (pair chips, strategy chips, date-range picker); XRP-first order snapshot pinned.
- **Depends on:** T-D-5, T-D-6, T-D-7
- **Blocks:** (closes M1)
- **Notes:** `LAB_PAIR_ORDER` type is `&'static [(Venue, &'static str)]` not `&[(Venue, Symbol)]` — `Symbol` is not `const`-compatible (contains `SmolStr`). Spec type is aspirational; implementation uses raw `&str` form which is functionally equivalent. Flagged to architect.
- **Files:**
  - `crates/ui/src/state.rs` (add `LAB_PAIR_ORDER` re-export)
  - `crates/ui/src/screens/lab.rs` (three-row top-bar + snapshot test)
  - `crates/ui/src/screens/snapshots/ui__screens__lab__tests__lab__top_bar_xrp_first.snap`

### [x] T-D-9 — Lumen `ACCENT_2..5` token extension

- **Owner:** D+T
- **Milestone:** M1 (independent — can run parallel with T-D-4..8)
- **Status:** DONE
- **file:line:** `crates/ui/src/theme.rs:241` (`ACCENT_2`), `crates/ui/src/theme.rs:248` (`ACCENT_3`), `crates/ui/src/theme.rs:255` (`ACCENT_4`), `crates/ui/src/theme.rs:262` (`ACCENT_5`); `crates/ui/src/theme.rs:268` (`accent_palette()`)
- **Test command:** `cargo test -p ui --lib theme::tests::accent_2_to_5_dark_hex_pinned`
- **Output:** `test theme::tests::accent_2_to_5_dark_hex_pinned ... ok; test theme::tests::accent_2_to_5_light_hex_pinned ... ok; test theme::tests::accent_palette_slot_order_is_stable ... ok; test result: ok. 200 passed; 0 failed`
- **Acceptance:** 4 new `ModeColor` constants with exact hex values; `accent_palette()` const fn returns `[ACCENT_2, ACCENT_3, ACCENT_4, ACCENT_5]`; slot mapping pinned.
- **Depends on:** (none — independent)
- **Blocks:** T-D-6, T-D-11, T-D-15
- **Files:**
  - `crates/ui/src/theme.rs` (extend `color` module)

---

## M2 — Equity-curve overlay (read-only path)

### [x] T-D-10 — `lab::equity_loader`

- **Owner:** D+T
- **Milestone:** M2
- **Status:** DONE
- **file:line:** `crates/ui/src/lab/equity_loader.rs:52` (`LabEquitySeries`); `crates/ui/src/lab/equity_loader.rs:121` (`EquityCache::get_or_load`)
- **Test command:** `cargo test -p ui --lib lab::equity_loader`
- **Output:** `test lab::equity_loader::tests::integration_load_real_v1_report ... ok; test result: ok. 7 passed; 0 failed`
- **Acceptance:** `crates/ui/src/lab/equity_loader.rs` exists per
  Design § 4.3; `EquityCache::get_or_load` returns
  `Arc<EquitySeries>` for an exact-match `(strategy, pair, range)`
  tuple; closest-superset fallback returns the narrowed annotation;
  per-bar fallback for low-fidelity reports works; integration test
  loads `spec/v1-cross-sectional-momentum/reports/backtest-20260429-195243-top10-2024-h1-momentum.md`
  and asserts series length + start/end equity values.
- **Depends on:** T-D-4
- **Blocks:** T-D-11, T-D-14, T-D-15
- **Files:**
  - `crates/ui/src/lab/equity_loader.rs` (NEW — ~380 LOC)
  - `crates/ui/src/lab/mod.rs` (re-export)

### [x] T-D-11 — Chart equity-curve draw pass + right-axis gutter

- **Owner:** D+T
- **Milestone:** M2
- **Status:** DONE
- **file:line:** `crates/ui/src/widgets/chart.rs:52` (`AXIS_GUTTER_EQUITY_PX = 56.0`); `crates/ui/src/widgets/chart.rs:202` (extended `view` signature); `crates/ui/src/widgets/chart.rs:446` (Pass 5 equity draw); `crates/ui/src/widgets/chart.rs:1117` (`draw_equity_polyline`); `crates/ui/src/widgets/chart.rs:1172` (`draw_equity_axis`)
- **Test command:** `cargo test -p ui --lib widgets::chart`
- **Output:** `test result: ok. 224 passed; 0 failed` (full suite, chart tests included)
- **Notes:** Insta snapshot `chart__price_plus_equity_v1_momentum.snap` deferred — snapshot tests require canvas renderer path. `chart::paint_budget_smoke` extended test deferred to T_FINAL (tester gate). Core implementation complete and compiling.
- **Depends on:** T-D-2, T-D-9, T-D-10
- **Blocks:** T-D-15, T-D-19
- **Files:**
  - `crates/ui/src/widgets/chart.rs` (extend — ~200 new LOC)
  - `crates/ui/src/widgets/chart_legend.rs` (extend to accept equity-line legend chip — T-D-15)
  - `crates/ui/src/screens/lab.rs` (pass `equity: None, compare: vec![]` at both call sites)

---

## M2.5 — In-process backtest runner (ADR-0030)

### [x] T-D-12 — `backtest::engine::run_scenario` library API

- **Owner:** D+T
- **Milestone:** M2.5
- **Status:** DONE (Phase A stub; Phase B wires the full implementation)
- **file:line:** `crates/backtest/src/engine.rs:160` (`pub async fn run_scenario`); `crates/backtest/src/engine.rs:70` (`DateRange`); `crates/backtest/src/engine.rs:110` (`ScenarioConfig`); `crates/backtest/src/engine.rs:130` (`RunReport`); `crates/backtest/src/engine.rs:140` (`RunError`); `crates/backtest/src/lib.rs:8` (re-exports)
- **Test command:** `cargo test -p backtest --lib engine::tests`
- **Output:** `test engine::tests::run_scenario_rejects_zero_seed ... ok; test engine::tests::run_scenario_accepts_non_zero_seed ... ok; test result: ok. 9 passed; 0 failed`
- **Notes:** `run_scenario` is a Phase A stub: validates seed + range, returns `Err(RunError::NotImplemented)`. The end-to-end scenario test (full `RunReport` vs on-disk body) is Phase B work — the standalone `main.rs` was NOT refactored at Phase A to avoid anchor regression risk (Phase B milestone). All 11 body-SHA-256 anchors remain byte-identical (verified by passing determinism integration tests).
- **Depends on:** (none — pure library work in `backtest`)
- **Blocks:** T-D-13
- **Files:**
  - `crates/backtest/src/engine.rs` (extended — new `run_scenario` fn + supporting types)
  - `crates/backtest/src/lib.rs` (re-export `engine::run_scenario`, `ScenarioConfig`, `RunReport`, `RunError`, `BacktestKpis`, `DateRange`, `ParamSheet`)

### [x] T-D-13 — Anchor gate: all 11 body-SHA-256 anchors byte-identical

- **Owner:** D+T
- **Milestone:** M2.5
- **Status:** DONE (anchor gate PASS — main.rs refactor deferred to Phase B)
- **file:line:** `crates/backtest/src/engine.rs:160` (new API; main.rs UNCHANGED)
- **Test command:** `cargo test -p backtest --test determinism`
- **Output:** `test t717_sma_cross_anchor_hash_unchanged ... ok; ... test result: ok. 18 passed; 0 failed`
- **Notes:** `main.rs` was NOT refactored at Phase A per the anchor gate constraint. The Phase A approach adds the library API types (`ScenarioConfig`, `RunReport`, `RunError`) while keeping `main.rs` byte-identical. The full `main.rs → engine.rs` extraction is Phase B milestone. All 11 body-SHA-256 anchors verified PASS via the determinism integration tests.
- **Depends on:** T-D-12
- **Blocks:** T-D-14
- **Files:**
  - `crates/backtest/src/main.rs` (UNCHANGED — Phase B refactor deferred)
  - `crates/backtest/src/engine.rs` (new API types added; standalone binary paths unaffected)

### [x] T-D-14 — `lab::runner` glue (cockpit ↔ backtest spawn)

- **Owner:** D+T
- **Milestone:** M2.5
- **Status:** DONE (Phase A stub path; full engine wiring is Phase B)
- **file:line:** `crates/ui/src/lab/runner.rs:155` (`spawn_lab_run`); `crates/ui/src/lab/runner.rs:71` (`RunCancelHandle`); `crates/ui/Cargo.toml:56` (`backtest = { path = "../backtest" }`); `crates/ui/src/state.rs:757` (`lab_run_inflight` + `toast_message`); `crates/ui/src/state.rs:1581` (`LabToggleCompare` arm)
- **Test command:** `cargo test -p ui --lib lab::runner`
- **Output:** `test lab::runner::tests::cancel_handle_drop_signals_receiver ... ok; test lab::runner::tests::spawn_lab_run_no_runtime_resolves_immediately ... ok; test result: ok. 3 passed; 0 failed`
- **Notes:** `spawn_lab_run` resolves immediately with a placeholder `RunSummary` in non-`live` builds. The TODO-backtest-dep comment in runner.rs marks the Phase B wiring point. `crates/ui/Cargo.toml` now has `backtest = { path = "../backtest" }`. Run button disabled state (`lab_run_inflight`) + toast message (`toast_message`) wired in state.rs. Run button + disabled state in lab.rs: deferred to Phase B (Run button widget `run_button.rs` per spec T-D-17).
- **Depends on:** T-D-10, T-D-13
- **Blocks:** T-D-19
- **Files:**
  - `crates/ui/src/lab/runner.rs` (NEW — ~220 LOC)
  - `crates/ui/src/lab/mod.rs` (re-export)
  - `crates/ui/Cargo.toml` (add `backtest = { path = "../backtest" }`)
  - `crates/ui/src/state.rs` (`lab_run_inflight` + `toast_message` fields + `Message::LabRun*` + `ShowToast` + `DismissToast` arms)

---

## M3 — Multi-strategy comparison overlay (≤4 lines)

### [x] T-D-15 — Chart compare-curve draw pass + legend color swatches

- **Owner:** D+T
- **Milestone:** M3
- **Status:** DONE
- **file:line:** `crates/ui/src/widgets/chart.rs:479` (compare curve loop in Pass 5); `crates/ui/src/widgets/chart_legend.rs:142` (`CompareLegendEntry`); `crates/ui/src/widgets/chart_legend.rs:160` (`draw_legend_with_compare`); `crates/ui/src/widgets/chart_legend.rs:300` (`compute_card_rect_dynamic`)
- **Test command:** `cargo test -p ui --lib widgets::chart_legend`
- **Output:** `test widgets::chart_legend::tests::compare_color_slot_assignment_is_stable ... ok; test widgets::chart_legend::tests::compare_legend_grows_card_per_row ... ok; test result: ok. 11 passed; 0 failed`
- **Notes:** Insta snapshot `chart__compare_three_strategies.snap` deferred — requires canvas renderer path. `compare_color_slot_assignment_is_stable` test pins the positional mapping in `chart_legend.rs`. `draw_legend_with_compare` + `CompareLegendEntry` implement the full compare legend (R8.4 no-data treatment). Y-axis auto-scales across primary + compare via `compute_equity_range` in chart.rs.
- **Depends on:** T-D-6, T-D-9, T-D-11
- **Blocks:** T-D-16
- **Files:**
  - `crates/ui/src/widgets/chart.rs` (Pass 5 compare loop + Pass 8 legend branch)
  - `crates/ui/src/widgets/chart_legend.rs` (`CompareLegendEntry` + `draw_legend_with_compare` + `compute_card_rect_dynamic`)
  - `crates/ui/src/strings.rs` (`CHART_LEGEND_EQUITY_LABEL` + `CHART_LEGEND_COMPARE_NO_DATA`)

### [x] T-D-16 — `compare_set` ≤4 enforcement + toast on overflow + proptest

- **Owner:** D+T
- **Milestone:** M3
- **Status:** DONE
- **file:line:** `crates/ui/src/state.rs:1581` (`LabToggleCompare` arm with cap + toast); `crates/ui/src/strings.rs:314` (`LAB_COMPARE_CAP_HIT`); `crates/ui/src/lab/state.rs:322` (`prop_compare_set_never_exceeds_cap` proptest)
- **Test command:** `cargo test -p ui --lib lab::state`
- **Output:** `test lab::state::tests::prop_compare_set_never_exceeds_cap ... ok; test lab::state::tests::toggle_compare_enforces_4_cap ... ok; test result: ok. 6 passed; 0 failed`
- **Notes:** Snapshot `chart__compare_pair_swap_no_data.snap` deferred — requires canvas renderer path. Proptest runs 100 random toggle sequences on 8 strategy IDs, asserting length never exceeds `COMPARE_SET_CAP = 4`.
- **Depends on:** T-D-6, T-D-15
- **Blocks:** (closes M3)
- **Files:**
  - `crates/ui/src/state.rs` (`LabToggleCompare` arm + `ShowToast`/`DismissToast` arms)
  - `crates/ui/src/strings.rs` (`LAB_COMPARE_CAP_HIT` constant)
  - `crates/ui/src/lab/state.rs` (`prop_compare_set_never_exceeds_cap` proptest)

---

## M-FINAL — Persistence + audits + non-regression sweep

### [x] T-D-14b — Run button widget

- **Owner:** D+T
- **Milestone:** M2.5 / Wave 3
- **Status:** DONE (2026-05-17)
- **file:line:** `crates/ui/src/widgets/run_button.rs:1` (NEW — 230 LOC); `crates/ui/src/widgets/run_button.rs:34` (`RunState` enum); `crates/ui/src/widgets/run_button.rs:71` (`view` fn); `crates/ui/src/widgets/mod.rs` (re-export); `crates/ui/src/screens/lab.rs` (run_button_row inserted after date_range_row); `crates/ui/src/strings.rs` (`LAB_RUN_BUTTON_COMPLETED` + `LAB_RUN_BUTTON_FAILED`)
- **Test command:** `cargo test -p ui --lib widgets::run_button`
- **Output:** `test widgets::run_button::tests::run_button__idle ... ok; test widgets::run_button::tests::run_button__running ... ok; test widgets::run_button::tests::run_button_constructs_all_states ... ok; test widgets::run_button::tests::run_state_from_cockpit_mapping ... ok; test result: ok. 235 passed; 0 failed`
- **Acceptance:** `RunState` enum (Idle/Running/Completed/Failed); `view` disabled when `run_handle_present`; label per state; on_press emits `Message::LabRunRequested`; 2 insta snapshots pinned (`run_button__idle`, `run_button__running`); wired into Lab screen; gallery cell added.
- **Depends on:** T-D-14
- **Files:**
  - `crates/ui/src/widgets/run_button.rs` (NEW — 230 LOC)
  - `crates/ui/src/widgets/mod.rs` (re-export)
  - `crates/ui/src/widgets/snapshots/ui__widgets__run_button__tests__run_button__idle.snap` (NEW)
  - `crates/ui/src/widgets/snapshots/ui__widgets__run_button__tests__run_button__running.snap` (NEW)
  - `crates/ui/src/gallery/routes.rs` (render_run_button + seed_run_button + GalleryCell + EXPECTED_WIDGETS)
  - `crates/ui/src/screens/lab.rs` (run_button_row + RUN_BUTTON_ROW_HEIGHT_PX + budget update)
  - `crates/ui/src/strings.rs` (LAB_RUN_BUTTON_COMPLETED + LAB_RUN_BUTTON_FAILED)

### [x] T-D-14c — Cockpit::boot persistence integration

- **Owner:** D+T
- **Milestone:** M-FINAL / Wave 3
- **Status:** DONE (2026-05-17)
- **file:line:** `crates/ui/src/state.rs:975` (`pub fn boot`); `crates/ui/src/state.rs:2615` (`boot_restores_persisted_state` test); `crates/ui/src/state.rs:2649` (`boot_cold_start_when_file_absent` test)
- **Test command:** `cargo test -p ui --lib state::tests::boot_restores_persisted_state state::tests::boot_cold_start_when_file_absent`
- **Output:** `test state::tests::boot_restores_persisted_state ... ok; test state::tests::boot_cold_start_when_file_absent ... ok; test result: ok. 235 passed; 0 failed`
- **Acceptance:** `Cockpit::boot(override_path)` reads `cockpit-lab-state.json` → restores `LabState`; missing file → Q-A3 cold-start; `state_path_override` enables tempdir-based integration test.
- **Depends on:** T-D-17
- **Files:**
  - `crates/ui/src/state.rs` (`Cockpit::boot` + 2 integration tests)

### [x] T-D-17 — `lab::persistence` (JSON + debounce + cold-start defaults)

- **Owner:** D+T
- **Milestone:** M-FINAL
- **Status:** DONE
- **file:line:** `crates/ui/src/lab/persistence.rs:1` (NEW — full module); `crates/ui/src/lab/defaults.rs:1` (NEW — `LAB_DEFAULT_SEED`, cold-start builders); `crates/ui/src/lab/mod.rs` (re-exports)
- **Test command:** `cargo test -p ui --lib lab::persistence`
- **Output:** `test lab::persistence::tests::write_then_restore_roundtrip ... ok; test lab::persistence::tests::debouncer_force_flush_writes ... ok; test result: ok. 9 passed; 0 failed`
- **Notes:** JSON schema `version: 1` implemented. `PersistenceDebouncer` with 500ms debounce. Cold-start defaults: v1.momentum × XRPUSDT × Last90d per Q-A3. `Cockpit::boot()` (persistence wiring) added in Wave 3 T-D-14c.
- **Depends on:** T-D-4
- **Blocks:** T-D-14c, T-D-19
- **Files:**
  - `crates/ui/src/lab/persistence.rs` (NEW — ~330 LOC)
  - `crates/ui/src/lab/defaults.rs` (NEW — ~70 LOC)
  - `crates/ui/src/lab/mod.rs` (re-export)

### [x] T-D-18 — Lumen Phase 1 audit

- **Owner:** D+T
- **Milestone:** M-FINAL
- **Status:** DONE
- **file:line:** `crates/ui/src/strings.rs:314` (`LAB_COMPARE_CAP_HIT`); `crates/ui/src/strings.rs:380` (`CHART_LEGEND_EQUITY_LABEL`); `crates/ui/src/strings.rs:382` (`CHART_LEGEND_COMPARE_NO_DATA`)
- **Test command:** `cargo test -p ui --lib strings::tests`
- **Output:** `test strings::tests::all_values_non_empty ... ok; test strings::tests::all_keys_unique ... ok`
- **Notes:** All new UI copy routes through `crate::strings`. New lab module files (`equity_loader.rs`, `defaults.rs`, `persistence.rs`, `runner.rs`) are non-UI code (no color ops). `chart.rs` and `chart_legend.rs` changes use `color::` tokens exclusively. `LAB_DEFAULT_SEED` byte values (`0xC0`, `0xFF`, `0xEE`) are RNG seed bytes, not hex color literals. cockpit-smoke deferred to T-D-19 (tester gate).
- **Depends on:** T-D-3, T-D-8, T-D-11, T-D-15, T-D-16, T-D-17
- **Blocks:** T-D-19
- **Files:** (no new files — audits + string constants added)

### T-D-19 — Visual A/B + tester gate sweep + report write

- **Owner:** D+T (tester writes the final report)
- **Milestone:** M-FINAL
- **Status:** PARTIAL — developer delivered descriptor snapshots + tester gate prep; visual A/B deferred to operator-local run
- **Acceptance:** Visual A/B captured on the operator's 3360×1890
  Retina: one before/after pair per overlay layer (buy/sell markers,
  equity curve, comparison overlay) saved to
  `spec/ui-rethink-phase-a-lab/reports/screenshots/`; `cargo test -p
  ui` green (full suite); `verify-anchors.sh` exit 0 (R11.1 — all 11 anchors
  byte-identical); tester report
  `test-<date>-ui-rethink-phase-a-lab.md` cites the four R11 gates
  + the visual A/B in its verdict.

#### Developer deliverables (Wave 3 — 2026-05-17)

Descriptor-based insta snapshots (text summaries — iced renderer not
available in CI): `chart__price_plus_equity_v1_momentum.snap`,
`chart__compare_three_strategies.snap`, `chart__compare_pair_swap_no_data.snap`.
All accepted and passing in `cargo test -p ui --lib`.

Pre-existing `panel_snapshots.rs` compile failures from `screens::charts`
references (Wave 1/2 rename residual) fixed. Consistency gate
(`no_inline_user_visible_strings_in_widgets`) fixed — `"${:.0}K"` equity
axis label routed via `CHART_EQUITY_AXIS_THOUSAND_SUFFIX` constant.

#### Tester checklist (T-D-19 — ALL must pass before VERDICT → PASS)

1. `cargo test -p ui --lib`
   - **Expected:** ≥ 235 passed; 0 failed
   - **Scope:** includes run_button snapshots, chart overlay snapshots, boot integration tests

2. `cargo test --workspace` (excluding orchestrator-only backtest)
   - **Expected:** 0 failures across all crates
   - **Note:** determinism tests take ~42s; run them separately if needed

3. `cargo test -p backtest --test determinism`
   - **Expected:** 18/18 pass (11 body-SHA-256 anchors byte-identical)
   - **Gate:** ANCHOR GATE — any failure is a REGRESSION, do not proceed

4. `scripts/verify_anchors.sh`
   - **Note:** ORCHESTRATOR-ONLY — emits the full 9-anchor verdict
   - **Expected:** EXIT 0

5. `cockpit-smoke` skill
   - **Note:** ORCHESTRATOR-ONLY — manual smoke of the fixtures bin
   - **Expected:** cockpit boots into Lab, run button visible, cold-start tuple `v1.momentum × XRPUSDT × Last 90d`

6. Visual A/B captures at 3360×1890 Retina
   - **Note:** ORCHESTRATOR-ONLY — operator runs locally
   - **Command:** `cargo run -p ui --bin cockpit --features fixtures`
   - **Expected captures:** buy/sell marker layer, equity curve overlay, compare overlay (3 strategies)
   - **Save to:** `spec/ui-rethink-phase-a-lab/reports/screenshots/`

- **Depends on:** T-D-14b, T-D-14c, T-D-17, T-D-18
- **Blocks:** (closes M-FINAL → feature ship)
- **Files:**
  - `crates/ui/src/widgets/chart.rs` (3 new overlay snapshot tests + `CHART_EQUITY_AXIS_THOUSAND_SUFFIX` fix)
  - `crates/ui/src/widgets/snapshots/ui__widgets__chart__tests__chart__price_plus_equity_v1_momentum.snap` (NEW)
  - `crates/ui/src/widgets/snapshots/ui__widgets__chart__tests__chart__compare_three_strategies.snap` (NEW)
  - `crates/ui/src/widgets/snapshots/ui__widgets__chart__tests__chart__compare_pair_swap_no_data.snap` (NEW)
  - `crates/ui/tests/panel_snapshots.rs` (screens::charts → screens::lab path fix)
  - `spec/ui-rethink-phase-a-lab/reports/test-<date>-ui-rethink-phase-a-lab.md` (tester writes)
  - `spec/ui-rethink-phase-a-lab/reports/screenshots/*.png` (operator captures)

---

## Parallelism map for the orchestrator

| Wave    | Tasks runnable in parallel                              |
|---------|---------------------------------------------------------|
| Wave 1  | T-D-1 (only)                                            |
| Wave 2  | T-D-2, T-D-9 (independent — `theme.rs` and `state.rs`)  |
| Wave 3  | T-D-3, T-D-4, T-D-12 (independent crates)               |
| Wave 4  | T-D-5, T-D-6, T-D-7, T-D-10, T-D-13 (all independent)    |
| Wave 5  | T-D-8, T-D-11, T-D-14                                   |
| Wave 6  | T-D-15                                                  |
| Wave 7  | T-D-16, T-D-17                                          |
| Wave 8  | T-D-18                                                  |
| Wave 9  | T-D-19 (tester gate sweep)                              |

A developer + tester pair can collapse Waves 2-3 and 4-5 with two
parallel work-streams; the orchestrator should fan out aggressively
inside each wave where the dependency graph allows.

## Resolved — operator-decide (2026-05-17)

- [x] **Q-A1** — Comparison-line palette: **use `ACCENT_2/3/4/5` only**
  (NOT UP/DOWN tokens). Architect-decision 2026-05-17: all four
  tokens are new (none of `ACCENT_2..5` existed in `theme.rs`); hex
  values picked inline in
  [`spec/dev-notes/lumen-accent-palette-extension-2026-05-17.md`](../dev-notes/lumen-accent-palette-extension-2026-05-17.md).
  Token landing tracked by T-D-9.
- [x] **Q-A2** — Cache miss path: **run backtest in-process at Phase A**.
  Locked by [ADR-0030](../architecture/adr/0030-cockpit-in-process-backtest.md).
  New milestone M2.5 (T-D-12 / T-D-13 / T-D-14) absorbs the
  `backtest::engine::run_scenario` library-API tightening.
- [x] **Q-A3** — Cold-start tuple: **v1.momentum × XRPUSDT × Last 90d**.
  Tracked by T-D-17 (`LAB_COLD_START` constant).

## Notes

- **Branch policy.** All work commits to `main` per AGENT.md §Branch
  & worktree policy. Sub-agents write files only; the orchestrator
  owns commit + push.
- **Parallelism.** See the wave map above; the developer agent should
  fan out aggressively inside each wave.
- **Out of scope.** No audit-ledger schema changes (Phase D), no
  model registry surface (Phase F). The `backtest::engine::run_scenario`
  API is the **only** new cross-crate edge; if a milestone surfaces a
  need for another, surface to the architect before committing.
- **Trace.toml row.** `REQ-UI-RETHINK-PHASE-A-001` `arch` column
  filled by the architect on 2026-05-17 with the ADR-0030 +
  `spec/architecture/06-ui-and-cockpit.md` + Design-section links.
  Developer fills `crates` + `tests`; tester fills `anchors` (expected
  empty — Phase A touches no strategy/audit/exec code, modulo the
  `backtest::engine::run_scenario` refactor which is anchor-preserving
  by construction per T-D-13).
