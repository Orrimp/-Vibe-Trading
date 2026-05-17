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

### T-D-1 — Scaffold `Screen::Lab` + deprecated aliases

- **Owner:** D
- **Milestone:** M0
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

### T-D-2 — Rename `screens/charts.rs` → `screens/lab.rs`, flip default boot

- **Owner:** D+T
- **Milestone:** M0
- **Acceptance:** `crates/ui/src/screens/lab.rs` is the new path
  (verbatim move of `charts.rs`'s contents — no body changes);
  `Cockpit::default()` returns `Screen::Lab` as `current_screen`;
  insta snapshot `shell__default_screen_lab` records the new boot;
  `cargo test -p ui` green.
- **Depends on:** T-D-1
- **Blocks:** T-D-3, T-D-11
- **Files:**
  - `crates/ui/src/screens/lab.rs` (renamed from `charts.rs`)
  - `crates/ui/src/screens/mod.rs` (re-export update)
  - `crates/ui/src/state.rs` (`Cockpit::default()` change)
  - `crates/ui/src/shell.rs` (route match arm rename)
  - `crates/ui/src/snapshots/` (new `shell__default_screen_lab.snap`)

### T-D-3 — Sidebar IA flip + placeholder route bodies

- **Owner:** D+T
- **Milestone:** M0
- **Acceptance:** `SIDEBAR_ENTRIES_PHASE_5` renamed to
  `SIDEBAR_ENTRIES_PHASE_A` with the new ordering (Lab, Live,
  Compare, divider, Strategies, Memory, Models, Trail, divider,
  Settings); new `widgets::placeholder::view(title, mode)` widget
  renders an empty-state card; placeholder routes resolve without
  panic; insta snapshot `sidebar__phase_a_workflow_group` records
  the new sidebar shape; cockpit-smoke clicks each sidebar entry,
  exit 0.
- **Depends on:** T-D-2
- **Blocks:** (closes M0)
- **Files:**
  - `crates/ui/src/theme/layout.rs` (sidebar constant rename + reorder)
  - `crates/ui/src/widgets/placeholder.rs` (NEW — ~30 LOC)
  - `crates/ui/src/widgets/mod.rs` (re-export)
  - `crates/ui/src/shell.rs` (7-arm screen_body match)
  - `crates/ui/src/snapshots/` (new `sidebar__phase_a_workflow_group.snap`)

---

## M1 — Pair chip + strategy chip + date-range picker

### T-D-4 — `LabState` struct + `Cockpit::lab_state` field

- **Owner:** D+T
- **Milestone:** M1
- **Acceptance:** `crates/ui/src/lab/state.rs` defines `LabState` per
  R6.1 (strategy, pair, range, params, compare_set: SmallVec<[_; 4]>);
  `Cockpit::lab_state: LabState` field added; `Message::Lab*` variants
  (`LabSelectPair`, `LabSelectPrimaryStrategy`, `LabToggleCompare`,
  `LabSelectRange`, `LabRunRequested`, `LabRunCompleted`) added with
  update-handler stubs that mutate the field; a proptest verifies
  `LabToggleCompare` is idempotent + enforces the 4-cap.
- **Depends on:** T-D-1
- **Blocks:** T-D-5, T-D-6, T-D-7, T-D-10, T-D-17
- **Files:**
  - `crates/ui/src/lab/mod.rs` (NEW — re-export)
  - `crates/ui/src/lab/state.rs` (NEW — ~80 LOC)
  - `crates/ui/src/state.rs` (extend `Cockpit` + `Message` + update arms)

### T-D-5 — `widgets::pair_chip`

- **Owner:** D+T
- **Milestone:** M1
- **Acceptance:** `widgets/pair_chip.rs` exists with the `view` fn per
  Design § 2.1; unit test asserts `Message::LabSelectPair` dispatch;
  insta snapshot `pair_chip__active_xrpusdt` records the active chip.
  Zero hex literals (tokens only). Zero string literals (via
  `crate::strings`).
- **Depends on:** T-D-4
- **Blocks:** T-D-8
- **Files:**
  - `crates/ui/src/widgets/pair_chip.rs` (NEW)
  - `crates/ui/src/widgets/mod.rs` (re-export)
  - `crates/ui/src/snapshots/pair_chip__active_xrpusdt.snap`

### T-D-6 — `widgets::strategy_chip`

- **Owner:** D+T
- **Milestone:** M1
- **Acceptance:** `widgets/strategy_chip.rs` exists per Design § 2.2;
  two emit paths (primary select + compare toggle) covered by unit
  tests; color swatch reads positionally from
  `[ACCENT_2, ACCENT_3, ACCENT_4, ACCENT_5]` (depends on T-D-9
  landing the tokens); insta snapshot
  `strategy_chip__primary_with_compare_slot_1.snap`.
- **Depends on:** T-D-4, T-D-9 (token landing for the swatch lookup)
- **Blocks:** T-D-15, T-D-16
- **Files:**
  - `crates/ui/src/widgets/strategy_chip.rs` (NEW)
  - `crates/ui/src/widgets/mod.rs` (re-export)
  - `crates/ui/src/snapshots/strategy_chip__primary_with_compare_slot_1.snap`

### T-D-7 — `widgets::date_range` picker

- **Owner:** D+T
- **Milestone:** M1
- **Acceptance:** `widgets/date_range.rs` exists per Design § 2.3;
  five preset entries + "Custom…" path; parse-error highlight on
  invalid ISO-8601 input; "narrowed-from" badge renders when
  `narrowed_from.is_some()`; insta snapshot
  `date_range_picker__presets.snap` + `date_range_picker__custom_invalid.snap`.
- **Depends on:** T-D-4
- **Blocks:** (M1 closer T-D-8)
- **Files:**
  - `crates/ui/src/widgets/date_range.rs` (NEW)
  - `crates/ui/src/widgets/mod.rs` (re-export)
  - `crates/ui/src/snapshots/date_range_picker__*.snap`

### T-D-8 — XRP-first universe ordering pin + Lab top-bar wiring

- **Owner:** D+T
- **Milestone:** M1
- **Acceptance:** `crates/ui/src/state.rs` gains
  `pub const LAB_PAIR_ORDER: &[(Venue, Symbol)]` in the operator-locked
  order (XRP, ETH, BTC, ADA, AVAX, BNB, DOGE, DOT, LINK, SOL);
  `assert_eq!` test pins the order; `screens/lab.rs` renders the
  three-row top bar (pair chips → strategy chips → date-range picker)
  using T-D-5/6/7 widgets; cockpit-smoke clicks XRP then ETH chips
  and observes `lab_state.pair` mutation.
- **Depends on:** T-D-5, T-D-6, T-D-7
- **Blocks:** (closes M1)
- **Files:**
  - `crates/ui/src/state.rs` (add `LAB_PAIR_ORDER`)
  - `crates/ui/src/screens/lab.rs` (top-bar composition)
  - `crates/ui/src/snapshots/lab__top_bar_xrp_first.snap`

### T-D-9 — Lumen `ACCENT_2..5` token extension

- **Owner:** D+T
- **Milestone:** M1 (independent — can run parallel with T-D-4..8)
- **Acceptance:** `crates/ui/src/theme.rs` `color` module gains four
  new `ModeColor` constants (`ACCENT_2 / ACCENT_3 / ACCENT_4 /
  ACCENT_5`) per Design § 7 with exact hex values from the
  [accent-palette dev-note](../dev-notes/lumen-accent-palette-extension-2026-05-17.md);
  existing Lumen contrast audit script (`scripts/lumen_contrast_audit.sh`)
  re-run, exit 0; an `assert_eq!` test pins the positional slot
  mapping `[ACCENT_2..5]` for the compare slot lookup.
- **Depends on:** (none — independent)
- **Blocks:** T-D-6, T-D-11, T-D-15
- **Files:**
  - `crates/ui/src/theme.rs` (extend `color` module)
  - `crates/ui/src/theme/iced_widget_catalogs.rs` (if catalog audit
    needs registering — verify in implementation)

---

## M2 — Equity-curve overlay (read-only path)

### T-D-10 — `lab::equity_loader`

- **Owner:** D+T
- **Milestone:** M2
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
  - `crates/ui/src/lab/equity_loader.rs` (NEW — ~200 LOC)
  - `crates/ui/src/lab/mod.rs` (re-export)

### T-D-11 — Chart equity-curve draw pass + right-axis gutter

- **Owner:** D+T
- **Milestone:** M2
- **Acceptance:** `widgets::chart::view` signature extends to take
  `equity: Option<EquitySeries>` and `compare: Vec<EquitySeries>`
  (with `compare = vec![]` accepted at all existing call sites
  pixel-identically); when `equity.is_some()`, the right-axis gutter
  appears at `AXIS_GUTTER_PX = 56.0`, ticks render, equity polyline
  draws in `color::ACCENT` (primary slot); buy/sell markers stay
  on top (R2.4); insta snapshot
  `chart__price_plus_equity_v1_momentum.snap` records the
  two-line shape; `chart::paint_budget_smoke` extended to assert
  paint < 16 ms on the 3360×1890 fixture.
- **Depends on:** T-D-2, T-D-9, T-D-10
- **Blocks:** T-D-15, T-D-19
- **Files:**
  - `crates/ui/src/widgets/chart.rs` (extend — ~150 new LOC)
  - `crates/ui/src/widgets/chart_legend.rs` (extend `view` to accept
    equity-line legend chip)
  - `crates/ui/src/screens/lab.rs` (pass `equity` parameter)
  - `crates/ui/src/snapshots/chart__price_plus_equity_v1_momentum.snap`

---

## M2.5 — In-process backtest runner (ADR-0030)

### T-D-12 — `backtest::engine::run_scenario` library API

- **Owner:** D+T
- **Milestone:** M2.5
- **Acceptance:** `crates/backtest/src/engine.rs` gains `pub async fn
  run_scenario(cfg: ScenarioConfig) -> Result<RunReport, RunError>`
  per [ADR-0030](../architecture/adr/0030-cockpit-in-process-backtest.md);
  `ScenarioConfig` + `RunReport` + `RunError` types defined; `seed:
  [u8; 32]` mandatory with `[0u8; 32]` rejection; `cargo test -p
  backtest` green; new unit test runs a small scenario end-to-end
  and asserts the in-memory `RunReport` matches the on-disk Markdown
  report's body bytes when `write_report = true`.
- **Depends on:** (none — pure library work in `backtest`)
- **Blocks:** T-D-13
- **Files:**
  - `crates/backtest/src/engine.rs` (extend — new `run_scenario` fn
    + supporting types)
  - `crates/backtest/src/lib.rs` (re-export `engine::run_scenario`,
    `ScenarioConfig`, `RunReport`, `RunError`)

### T-D-13 — Refactor `backtest` bin to call the new library API

- **Owner:** D+T
- **Milestone:** M2.5
- **Acceptance:** `crates/backtest/src/main.rs` is rebuilt around
  `engine::run_scenario` — CLI arg parsing still produces a
  `ScenarioConfig`, then calls the library; `verify-anchors.sh`
  exit 0 (all 11 body-SHA-256 anchors byte-identical — this is the
  hard determinism gate); existing CLI invocations (`cargo run -p
  backtest --bin backtest -- --scenario sma-2023-1m`) produce
  byte-identical files.
- **Depends on:** T-D-12
- **Blocks:** T-D-14
- **Files:**
  - `crates/backtest/src/main.rs` (refactor — net LOC change should
    be small; the body moves into `engine.rs`)

### T-D-14 — `lab::runner` glue (cockpit ↔ backtest spawn)

- **Owner:** D+T
- **Milestone:** M2.5
- **Acceptance:** `crates/ui/src/lab/runner.rs` exists per Design
  § 4.2; `crates/ui/Cargo.toml` gains `backtest = { path =
  "../backtest" }` dependency; `Message::LabRunRequested` handler
  spawns via `tokio::runtime::Handle` (captured at cockpit boot as
  with `KillSwitch::trip`); `Message::LabRunCompleted` invalidates
  the `EquityCache` and triggers a chart repaint; in-flight run
  cancellation works (clicking Run twice cancels the first); the
  Run button greys out while in-flight; cockpit-smoke test runs a
  fixture scenario and observes the chart updating from cached to
  fresh data.
- **Depends on:** T-D-10, T-D-13
- **Blocks:** T-D-19
- **Files:**
  - `crates/ui/src/lab/runner.rs` (NEW — ~120 LOC)
  - `crates/ui/src/lab/mod.rs` (re-export)
  - `crates/ui/Cargo.toml` (add `backtest` dep)
  - `crates/ui/src/state.rs` (`lab_state.run_inflight` field +
    `Message::LabRun*` update arms)
  - `crates/ui/src/screens/lab.rs` (Run button + disabled state)

---

## M3 — Multi-strategy comparison overlay (≤4 lines)

### T-D-15 — Chart compare-curve draw pass + legend color swatches

- **Owner:** D+T
- **Milestone:** M3
- **Acceptance:** `widgets::chart::view`'s `compare: Vec<EquitySeries>`
  parameter is now populated; each compare entry renders in its
  positional ACCENT_2..5 color; right Y-axis auto-scales across
  primary + compare lines (R8.3); legend extension shows
  `CompareLegendEntry` rows with the matching color swatch + the
  faded "no data" treatment for `CompareStatus::NoDataForPair`
  (R8.4); insta snapshot `chart__compare_three_strategies.snap`
  records three distinct equity lines on a fixture pair; unit test
  `chart::test::compare_color_slot_assignment_is_stable` pins the
  positional mapping.
- **Depends on:** T-D-6, T-D-9, T-D-11
- **Blocks:** T-D-16
- **Files:**
  - `crates/ui/src/widgets/chart.rs` (extend — ~80 new LOC for the
    compare pass + Y-axis auto-scale)
  - `crates/ui/src/widgets/chart_legend.rs` (compare-entry rendering)
  - `crates/ui/src/screens/lab.rs` (pass `compare` parameter)
  - `crates/ui/src/snapshots/chart__compare_three_strategies.snap`

### T-D-16 — `compare_set` ≤4 enforcement + toast on overflow + pair-swap reload

- **Owner:** D+T
- **Milestone:** M3
- **Acceptance:** `Message::LabToggleCompare` enforces the 4-cap
  (the 5th add is no-op + emits `Message::ShowToast(strings::LAB_COMPARE_CAP_HIT)`);
  pair-swap test: with 3 compare strategies active, clicking BTCUSDT
  → each compare entry's loader fires; any strategy without a
  cached BTCUSDT report shows the faded "no data" legend chip
  (verified via insta snapshot
  `chart__compare_pair_swap_no_data.snap`); proptest verifies the
  cap holds under randomised add/remove sequences.
- **Depends on:** T-D-6, T-D-15
- **Blocks:** (closes M3)
- **Files:**
  - `crates/ui/src/state.rs` (`LabToggleCompare` update arm)
  - `crates/ui/src/strings.rs` (add `LAB_COMPARE_CAP_HIT`)
  - `crates/ui/src/snapshots/chart__compare_pair_swap_no_data.snap`

---

## M-FINAL — Persistence + audits + non-regression sweep

### T-D-17 — `lab::persistence` (JSON + debounce + cold-start defaults)

- **Owner:** D+T
- **Milestone:** M-FINAL
- **Acceptance:** `crates/ui/src/lab/persistence.rs` exists per Design
  § 5; `crates/ui/src/lab/defaults.rs` defines `LAB_COLD_START`
  (v1.momentum × XRPUSDT × Last 90d per Q-A3); JSON schema matches
  the documented shape (`version: 1`, discriminated `range` union);
  write debouncer fires 500 ms after last mutation (proptest verifies
  no write storm under 100 chip clicks); corrupted file → warn-log +
  cold-start fallback; integration test: mutate `lab_state`, force
  flush, restart cockpit (simulated via re-`Cockpit::default()` +
  restore call), verify tuple restored.
- **Depends on:** T-D-4
- **Blocks:** T-D-19
- **Files:**
  - `crates/ui/src/lab/persistence.rs` (NEW — ~150 LOC)
  - `crates/ui/src/lab/defaults.rs` (NEW — ~30 LOC)
  - `crates/ui/src/lab/mod.rs` (re-export)
  - `crates/ui/src/state.rs` (`Cockpit::boot()` calls
    `persistence::restore_or_default`)

### T-D-18 — Lumen Phase 1 audit + cockpit-smoke gate

- **Owner:** D+T
- **Milestone:** M-FINAL
- **Acceptance:** `grep '#' crates/ui/src/screens/lab.rs
  crates/ui/src/widgets/pair_chip.rs crates/ui/src/widgets/strategy_chip.rs
  crates/ui/src/widgets/date_range.rs crates/ui/src/lab/*.rs` returns
  zero hex colors; same grep for raw string literals (excluding
  test-only `cfg(test)` blocks) returns zero; `cockpit-smoke` skill
  exit 0 against the M0..M3 acceptance scenarios; `spec-lint` exit 0.
- **Depends on:** T-D-3, T-D-8, T-D-11, T-D-15, T-D-16, T-D-17
- **Blocks:** T-D-19
- **Files:** (no new files — audits only)

### T-D-19 — Visual A/B + tester gate sweep + report write

- **Owner:** D+T (tester writes the final report)
- **Milestone:** M-FINAL
- **Acceptance:** Visual A/B captured on the operator's 3360×1890
  Retina: one before/after pair per overlay layer (buy/sell markers,
  equity curve, comparison overlay) saved to
  `spec/ui-rethink-phase-a-lab/reports/screenshots/`; `cargo test -p
  ui` green (full suite, 267 baseline snapshots + the new Phase A
  ones); `verify-anchors.sh` exit 0 (R11.1 — all 11 anchors
  byte-identical); tester report
  `test-<date>-ui-rethink-phase-a-lab.md` cites the four R11 gates
  + the visual A/B in its verdict.
- **Depends on:** T-D-14, T-D-17, T-D-18
- **Blocks:** (closes M-FINAL → feature ship)
- **Files:**
  - `spec/ui-rethink-phase-a-lab/reports/test-<date>-ui-rethink-phase-a-lab.md`
  - `spec/ui-rethink-phase-a-lab/reports/screenshots/*.png`

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
