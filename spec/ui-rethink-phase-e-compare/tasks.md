---
slug: ui-rethink-phase-e-compare
status: in-progress
owner: developer
updated: 2026-05-20
---

# Tasks — UI rethink Phase E (Compare matrix)

> Analyst M0 ordered checklist. Architect M-T1 decomposition adds
> T-T1-* rows; developer T-D-N* rows append once the architect locks
> the decomp. **Per project convention, this file at analyst hand-off
> carries only the T-A* rows; T-T* / T-D-N* are appended in M-T1 by
> the architect.** Pointers: [feature.md](feature.md) carries R1-R8,
> Q1-Q8, K1-K8, H1-H5. Scope source-of-truth:
> [dev-note](../dev-notes/ui-rethink-2026-05-17.md) §6 Phase E
> (lines 1082-1096), §J3 (lines 340-390), §3 IA (lines 651-744).

## M0 — Analyst synthesis

- [x] T-A1 — Read dev-note §6 Phase E (scope source-of-truth) +
  §J3 (Compare strategies across pairs — operator job-story) +
  §3 (Per-pair-first navigation pattern — informs the matrix axis
  design) + §1141 addendum (Q8 operator-decided **background**).
  _Acceptance: feature.md "Why" + "Requirements" anchored to
  dev-note line numbers; no silent scope drift._

- [x] T-A2 — **Predecessor surface audit.** Confirm Phase C sidebar
  IA reserves `Screen::Compare` in `SIDEBAR_GROUPS_PHASE_C` Work
  zone (`crates/ui/src/theme.rs:742`). Confirm `screens::compare`
  is currently a placeholder route via `placeholder::view` at
  `crates/ui/src/shell.rs:96`. Confirm `strings::COMPARE_PLACEHOLDER`
  + `strings::SIDEBAR_NAV_COMPARE` already exist
  (`strings.rs:251-266`).
  _Acceptance: R5.1 / R5.2 / R5.3 cite the existing sidebar +
  placeholder wiring; no Phase C body change required._

- [x] T-A3 — **Report-cache shape audit.** Sample existing report
  frontmatter under `spec/<strategy>/reports/backtest-*.md` (e.g.
  `spec/v1-cross-sectional-momentum/reports/backtest-20260429-195148-top10-2023-1h-momentum.md`)
  and confirm the YAML frontmatter is flat key:value with a single
  nested `strategy:` block. Confirm the scenario→universe mapping
  is reconstructable from the scenario name (e.g. `top10-2023-1h-*`
  → 10-symbol universe; `btc-2023-1m-*` → BTCUSDT). Document Q6
  finding: existing reports carry **universe-aggregate KPIs, not
  per-pair**.
  _Acceptance: R3.1-R3.6 anchored to a real report path; K7
  surfaced honestly._

- [x] T-A4 — **Lab seeding contract audit.** Confirm how Lab state
  is pre-filled via `Message` dispatch — `SelectStrategy` +
  `LabSelectPair` + `LabRangeSelected` already exist at
  `crates/ui/src/state.rs:1305,1370,1810`. Identify the compound-
  dispatch precedent (Phase C `OpenStrategyInLab`, Phase D
  `OpenTrailFor`) so R4.1 mirrors it.
  _Acceptance: R4.1-R4.4 cite the existing seeding messages and
  the compound-dispatch precedent; H5 falsifiable test path named._

- [x] T-A5 — **Anchor-risk pre-flight.** Confirm Phase E touches no
  strategy / audit / exec / report-renderer code; the matrix
  consumes existing report files it does not generate. 22-anchor
  regression gate carry-forward; H2 from Phase D+ predecessor
  applies verbatim.
  _Acceptance: R7.1-R7.7 enumerate the 8-item non-regression
  contract; "anchor risk zero" claim defended by construction._

- [x] T-A6 — **Surface Q1-Q8 with recommended defaults** for
  operator-decide:
  - Q1 axis orientation (analyst-recommended: a — strategies as rows)
  - Q2 recompute cadence (analyst-recommended: c — report-cache only)
  - Q3 cell KPI (analyst-recommended: a — Sharpe)
  - Q4 empty cell behavior (analyst-recommended: b — Run affordance)
  - Q5 entry point (analyst-recommended: a — sidebar only)
  - Q6 multi-symbol universe-aggregate semantic (analyst-recommended:
    a + tooltip, ship per-pair-decomp in v0.2.0)
  - Q7 strategy enumeration source (analyst-recommended: a — registry)
  - Q8 pair enumeration source (analyst-recommended: b — universe
    gating)
  _Acceptance: feature.md "Q-questions (operator-decide)" section
  carries 8 entries each with recommendation + rationale + alt
  options._

- [x] T-A7 — **Author K1-K8 risk register.** K6 (Compare/Lab range
  divergence) and K7 (universe-aggregate semantic confusion)
  surfaced as the load-bearing UX traps; both surfaced honestly
  for operator review at M-FINAL.
  _Acceptance: feature.md K-section carries 8 entries each with
  severity + mitigation._

- [x] T-A8 — **Author H1-H5 hypothesis register.** Each hypothesis
  must be falsifiable by a named test or measurement:
  - H1 cache-hit rate ≥ 30 % at first matrix open (architect M-T1
    enumerates against live spec/ tree)
  - H2 6×10 matrix legibility (operator-subjective at presenter deck)
  - H3 idle-CPU floor ≤ 13.6 % preserved
  - H4 cache scan budget < 50 ms p99 (architect M-T1 micro-bench)
  - H5 `OpenLabFromCompare` round-trip atomic (unit test)
  _Acceptance: feature.md H-section carries 5 entries; each names
  a falsification path._

- [x] T-A9 — **Author acceptance criteria per milestone** (M0,
  M-OD, M-T1, M-FINAL). M-FINAL includes new snapshot baselines:
  `compare__cold_boot_all_empty`,
  `compare__steady_state_populated`,
  `compare__empty_cell_run_affordance`,
  `compare__column_header_hover`.
  _Acceptance: feature.md "Acceptance criteria" section structured
  per Phase D / Phase D+ precedent._

- [x] T-A10 — **Open trace row `REQ-UI-RETHINK-PHASE-E-001`** in
  `draft` state. `arch` / `crates` / `tests` / `anchors` columns
  left empty for architect / developer / tester to fill.
  _Acceptance: trace.toml carries the new row with title quoting
  the dev-note §6 Phase E scope; state = "draft"._

- [x] T-A11 — **Promote backlog entry.** Add `ui-rethink-phase-e-compare`
  to `spec/backlog.md` "Active" section directly above
  `v25-tcn-alpha-investigation`, mirroring the predecessor entry
  format. Carry the v0.1.0 / predecessor / Q1-Q8 / cost callouts
  from feature.md.
  _Acceptance: backlog.md "Active" section carries the new row;
  format consistent with the Phase D / Phase D+ predecessor entries._

- [x] T-A12 — **Emit analyst HANDOFF envelope** per AGENT.md
  communication contract (`from = "analyst", to = "operator",
  verdict = "READY-FOR-OPERATOR-DECIDE"`). Lists spec files
  written + Q1-Q8 that need operator input + assumptions /
  recommended defaults.
  _Acceptance: handoff envelope appended to assistant response;
  trace_refs include `REQ-UI-RETHINK-PHASE-E-001`._

## M-OD — Operator-decide (Q1-Q8) — resolved 2026-05-20

> All eight analyst-recommended defaults accepted in one tick via the
> operator's standing "Autoapprove all" directive (confirmed
> 2026-05-20 against the analyst hand-off envelope).

- [x] T-OD1 — Q1 = (a) strategies as rows, pairs as columns.
- [x] T-OD2 — Q2 = (c) report-cache only; no new recompute orchestration
  (manual via Lab Run; v0.2.0 candidate for background poll).
- [x] T-OD3 — Q3 = (a) Sharpe (single number per cell; matches Lab Run
  anchor metric).
- [x] T-OD4 — Q4 = (b) `Run` affordance per empty cell (reuses Phase B
  Lab Run dispatch).
- [x] T-OD5 — Q5 = (a) sidebar entry only (Phase C reserved entry already
  in `SIDEBAR_GROUPS_PHASE_C` Work zone).
- [x] T-OD6 — Q6 = (a) render all cells with universe-aggregate KPI +
  tooltip; per-pair decomp deferred to v0.2.0 (K7 mitigation noted).
- [x] T-OD7 — Q7 = (a) `Cockpit::strategies_config.strategies` registry
  enumeration.
- [x] T-OD8 — Q8 = (b) per-strategy universe with blanked-grey cells
  outside (honest about which cells are legal).

## M-T1 — Architect decomposition — RESOLVED 2026-05-20

> Architect resolved K3 (hand-parse YAML; no `serde_yaml` workspace
> dep, no ADR), enumerated H1 statically (24/60 cells = 40 % first-
> open hit-rate ≥ 30 % threshold), budgeted H4 by static argument
> (≤ 15 ms p99 over 32 reports; 3× under the 50 ms target),
> consolidated Q6 sub-decision (universe-aggregate disclaimer =
> subtitle + per-cell tooltip), and locked the state location
> (sibling field on `Cockpit` at `state.rs:~880`). Five Waves (A-E)
> with 18 T-D-N rows; full decomp at
> [`decomp.md`](decomp.md). Anchor baseline
> `ANCHORS PASS  (22 / 22)` re-verified before this pass.

- [x] T-T1-1 — **K3 resolution.** `cargo tree -e features --workspace
  2>/dev/null | grep -i yaml` returned only `yaml-rust2 v0.8.1`
  (transitive via `insta` dev-dep + `config` runtime dep); `serde_yaml`
  is **not** in the workspace. `grep -rn "serde_yaml\|serde-yaml"
  --include=Cargo.toml` returned empty. Architect-decide: (b) hand-
  parse the flat YAML frontmatter — ~30 LOC, no new external dep, no
  ADR. Documented in `decomp.md` § 1.1 with parser shape + locked
  `parse_frontmatter` contract.
  _Output: `decomp.md § 1.1` locked the parser shape; no Cargo.toml
  diff._

- [x] T-T1-2 — **H1 enumeration.** Counted backtest reports under
  `spec/<strategy>/reports/` via `find spec -type f -name 'backtest-*.md'`
  = 32 total, distributed across 6 spec folders. Mapped scenarios to
  legal cells per Q8=(b) universe gating (BTC-only for v0.sma /
  v0.5.composed; top10 for v1.momentum / v2.5.tcn; (BTC,ETH) for
  v1.5a.pairs; v2.llm not yet registered). **Result: 24 / 60 cells
  populated = 40 % hit-rate** (1+1+10+2+0+10), passes H1 ≥ 30 %
  threshold by a comfortable margin. K7 universe-aggregate semantic
  applies to 20/24 = 83 % of the populated surface (v1.momentum +
  v2.5.tcn rows). Documented in `decomp.md § 1.2`.
  _Output: `decomp.md § 1.2` carries the per-strategy cell census
  table; first-open UX implication = "Run" affordance dominates only
  the v2.llm row at v0.1.0._

- [x] T-T1-3 — **H4 cache-scan budget.** Static argument: 32 reports
  × ~640 B header / file = 20 KB total head-read. Shell-level glob +
  `head -20` × 32 measured at 0.12 s wall (`/usr/bin/time -p sh -c
  'find ... -exec head -20 {} \;'` — 32 fork+exec roundtrips). Pure
  Rust streaming read + hand-parse: ≤ 15 ms (10× faster than
  shell-fork). **Result: ≤ 15 ms p99**, well under the 50 ms H4
  budget (3× headroom). If H4 falsifies at M-FINAL, K5 mitigation
  lifts: `tokio::spawn` cache scan at cockpit boot (~10 LOC, no
  anchor risk). Documented in `decomp.md § 1.3`.
  _Output: `decomp.md § 1.3` carries the static argument; no Rust
  bench needed at architect time (would be circular — code does not
  yet exist)._

- [x] T-T1-4 — **Q6 sub-decision: universe-aggregate disclaimer
  surface.** Architect-decide: subtitle under the matrix toolbar
  (always visible when ≥ 1 multi-symbol cell is in view) + per-cell
  tooltip on hover for every populated multi-symbol cell. Both
  reference a single new string constant
  `strings::COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE`. Rationale: K7
  applies to 83 % of populated cells per T-T1-2; a single corner-
  tooltip insufficient. Documented in `decomp.md § 1.4`.
  _Output: `decomp.md § 1.4` locked the disclaimer shape; new string
  constant added to the Wave A change-map._

- [x] T-T1-5 — **State location.** `pub compare_screen_state:
  CompareScreenState` field on `Cockpit` at
  `crates/ui/src/state.rs:~880` (immediately after
  `pub trail_screen_state: TrailScreenState` at line 879). Mirrors
  the `lab_state` (`:798`) + `trail_screen_state` (`:879`)
  3-touchpoint pattern (struct field declaration + `Default` impl at
  `:1009,1108` + `Debug` impl at `:959`). `CompareScreenState` lives
  in a new `crates/ui/src/compare/state.rs` module. Documented in
  `decomp.md § 1.6`.
  _Output: `decomp.md § 1.6` carries the full locked shape including
  `CachedCell` + `CompareKpiAxis` types + `BTreeMap` rationale
  (deterministic snapshot ordering)._

- [x] T-T1-6 — **Anchor gate confirmed.** `bash scripts/verify_anchors.sh`
  re-run 2026-05-20 BEFORE the M-T1 pass; literal output line:
  `ANCHORS PASS  (22 / 22)`. R7.1 carry-forward from predecessor
  ui-rethink-phase-d-trail-followup v0.1.1 (and v0.1.0 before that)
  confirmed clean. Phase E is purely additive UI by construction; no
  anchored renderer touched (R7.7).
  _Output: `cargo` invocation `bash scripts/verify_anchors.sh` →
  literal `ANCHORS PASS  (22 / 22)` line; trace.toml
  REQ-UI-RETHINK-PHASE-E-001 `anchors = []` carry-forward stays empty._

- [x] T-T1-7 — **Wave shape locked: A → B → C → D → E.** 5 waves;
  net-new file count = 5 (resolves R8.5 — analyst estimated 4-5;
  architect locks at 5: `compare/mod.rs`, `compare/state.rs`,
  `compare/cache.rs`, `widgets/matrix.rs`, `screens/compare.rs`).
  Wave A = data + dispatch scaffolding; Wave B = matrix widget;
  Wave C = screen body + shell wiring (single-line `shell.rs:96`
  swap); Wave D = 4 snapshot baselines + 1 layout-invariants
  proptest case + 1 H5 round-trip unit test + cockpit-smoke pre-run;
  Wave E = anchor gate + tester handoff. Spike requirement = NONE
  (all structural primitives carry-forward from Phase D+).
  _Output: `decomp.md § 1.5` + `§ 3` carry the wave table and the
  ordered T-D-N1..N18 checklist below._

## Wave A — Cache module + state types + Message variants (R3, R4, R6, R8)

- [x] T-D-N1 — Create `crates/ui/src/compare/mod.rs` (`pub mod cache;
  pub mod state;`) + `compare/state.rs` per `decomp.md § 1.6`
  (`CompareScreenState`, `CachedCell`, `CompareKpiAxis`). Add
  `pub mod compare;` to `crates/ui/src/lib.rs` next to `pub mod lab;`.
  - Files: `crates/ui/src/compare/mod.rs` (new), `crates/ui/src/compare/state.rs` (new), `crates/ui/src/lib.rs:1-line declaration`.
  - Cargo: `cargo check -p ui`.
  - Evidence: `crates/ui/src/compare/mod.rs:1-6`, `crates/ui/src/compare/state.rs:1-107`, `crates/ui/src/lib.rs:pub mod compare;` — `Finished` 0 errors 0 warnings.

- [x] T-D-N2 — Author `crates/ui/src/compare/cache.rs` per
  `decomp.md § 1.1`. Includes `parse_frontmatter` (private),
  `scan_spec_tree(spec_root: &Path) -> BTreeMap<...>` (public),
  `lookup_cell(strategy_id, symbol, range) -> Option<CachedCell>`
  (public), and the scenario→universe mapper (R3.2). 5 in-module
  unit tests under `#[cfg(test)] mod tests`: `parses_flat_kv`,
  `parses_strategy_block`, `returns_none_on_malformed`,
  `scenario_top10_maps_to_universe_of_10`,
  `scenario_btc_maps_to_btc_only`.
  - File: `crates/ui/src/compare/cache.rs:1-505` (new).
  - Cargo: `cargo test -p ui --lib compare::cache::tests`.
  - Evidence: `running 5 tests` + `test result: ok. 5 passed; 0 failed`.

- [x] T-D-N3 — Add 3 new `Message` enum variants at
  `crates/ui/src/state.rs`: `OpenLabFromCompare { strategy:
  StrategyId, pair: Option<(Venue, Symbol)>, range: DateRange }`
  (near `:1425` after `OpenTrailFor`); `CompareSelectRange(DateRange)`
  + `CompareSelectKpiAxis(CompareKpiAxis)` (near `:1380` toolbar
  Messages). Add 3 update-arms at `:~1911` after the `OpenTrailFor`
  arm: `OpenLabFromCompare` (compound dispatch — set
  `current_screen = Lab` → `lab_state.strategy = Some(...)` →
  `lab_state.pair = Some(...)` when arg is `Some` → `lab_state.range
  = range`), `CompareSelectRange` (pure assign), `CompareSelectKpiAxis`
  (pure assign). Order matches K4 mitigation (verbatim
  `OpenTrailFor:1902-1910` pattern).
  - File: `crates/ui/src/state.rs` (Message variants + update arms).
  - Cargo: `cargo check -p ui` + `cargo test -p ui --lib`.
  - Evidence: `cargo test -p ui --lib` → 303 passed; 0 failed (baseline 301 + 2 new tests).

- [x] T-D-N4 — Add `pub compare_screen_state: CompareScreenState`
  field to `Cockpit` at `crates/ui/src/state.rs:~880` (immediately
  after `trail_screen_state` at `:879`). Mirror in `Default::default`
  at `:~1009,1108` + `Debug::fmt` at `:~959`. `Cockpit::new()` +
  `Cockpit::new_with_persistence()` both initialize via
  `CompareScreenState::default()`.
  - File: `crates/ui/src/state.rs` (struct field + 3 init sites).
  - Cargo: `cargo test -p ui --lib`.
  - Evidence: `test result: ok. 303 passed; 0 failed`.

- [x] T-D-N5 — Add new strings to `crates/ui/src/strings.rs:~280`
  (Phase E section): `COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE` (§ 1.4),
  `COMPARE_TOOLBAR_RANGE_LABEL`, `COMPARE_TOOLBAR_KPI_LABEL`,
  `COMPARE_CELL_RUN_LABEL`, `COMPARE_CELL_BLANKED_LABEL`. Mark
  `COMPARE_PLACEHOLDER` (line 252) with `#[deprecated(since = "0.3.0",
  note = "Compare now renders the matrix body — Phase F removes this
  constant")]` per the `SETTINGS_PLACEHOLDER:259-263` precedent.
  - File: `crates/ui/src/strings.rs` (5 new consts + deprecated attr).
  - Cargo: `cargo check -p ui` + `cargo clippy -p ui -- -D warnings`.
  - Evidence: `Finished` 0 errors 0 warnings.

## Wave B — `widgets::matrix` widget (R2)

- [x] T-D-N6 — Author `crates/ui/src/widgets/matrix.rs` per
  `decomp.md § 2 row 10`. Public surface: `pub fn view(model:
  &Cockpit, mode: ThemeMode) -> Element<'_>`. Layout primitive:
  iced `Column<Row>` (no new `grid` widget per R2.5). Iterates
  rows over `model.strategies_config.strategies` (Q7=a) × columns
  over `strategy.universe()` (Q8=b). Per-cell match: populated
  (`cache.get(...).is_some()` → KPI text + sparkline + hairline-
  bordered Button on hover); empty-but-legal (centred "Run" Button
  with `ACCENT_500` hairline — Q4=b); blanked (centred `—` label +
  passive hairline — Q8=b). K7 tooltip on every populated cell with
  `cached.is_multi_symbol == true`.
  - Files: `crates/ui/src/widgets/matrix.rs:1-437` (new); `crates/ui/src/widgets/mod.rs` (`pub mod matrix;`).
  - Cargo: `cargo check -p ui`.
  - Evidence: `Finished` 0 errors 0 warnings.

- [x] T-D-N7 — Cell hover style: Lumen `BORDER_HAIRLINE` → `active_row`
  border tint on cell hover (R2.6). Mirrors the Phase C strategy-card
  hover state at `crates/ui/src/widgets/strategy_card.rs`. NO new
  theme tokens (R7.6).
  - File: `crates/ui/src/widgets/matrix.rs:305-326` (style closure on the cell Button).
  - Cargo: `cargo clippy -p ui -- -D warnings`.
  - Evidence: `Finished` 0 warnings.

## Wave C — Screen body + shell wiring (R1, R5)

- [x] T-D-N8 — Author `crates/ui/src/screens/compare.rs`. Toolbar:
  `Row[range_picker | kpi_axis_dropdown | k7_subtitle_when_any_cell_is_multi_symbol]`.
  Body: `widgets::matrix::view(model, mode)`. Public surface:
  `pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_>`.
  - Files: `crates/ui/src/screens/compare.rs:1-217` (new); `crates/ui/src/screens/mod.rs` (`pub mod compare;`).
  - Cargo: `cargo check -p ui`.
  - Evidence: `Finished` 0 errors 0 warnings.

- [x] T-D-N9 — Swap `crates/ui/src/shell.rs:96` from
  `Screen::Compare => placeholder::view(strings::COMPARE_PLACEHOLDER,
  mode)` to `Screen::Compare => screens::compare::view(model, mode)`.
  Add `compare` to the `use crate::screens::{...}` list at `:28`.
  **This is the only line in `shell.rs` Phase E swaps.**
  - File: `crates/ui/src/shell.rs:28,97`.
  - Cargo: `cargo test -p ui --lib` + `cargo test -p ui --test layout_invariants`.
  - Evidence: `test result: ok` on both; 6/6 layout-invariants baseline preserved; 303/303 lib tests.

## Wave D — Snapshot baselines + proptest case + round-trip test (R7, H5)

- [x] T-D-N10 — Author fixture `compare__cold_boot_all_empty`: matrix
  rendered with `compare_screen_state.cache = BTreeMap::new()` —
  every legal cell renders the "Run" affordance + every non-universe
  cell renders the blanked `—`. K7 subtitle absent (no multi-symbol
  cells populated yet).
  - File: `crates/ui/tests/visual_snapshots.rs:compare__cold_boot_all_empty` + `crates/ui/tests/fixtures/mod.rs:compare__cold_boot_all_empty_cockpit()` + baseline `crates/ui/tests/visual-baselines/compare__cold_boot_all_empty.png` (84,356 bytes).
  - Cargo: `cargo test -p ui --test visual_snapshots -- --exact compare__cold_boot_all_empty`.
  - Evidence: `running 1 test` + `test result: ok. 1 passed; 0 failed; finished in 2.42s`.

- [x] T-D-N11 — Author fixture `compare__steady_state_populated`:
  matrix rendered with all 24 populated cells filled per T-T1-2
  enumeration (deterministic values). K7 multi-symbol disclaimer subtitle visible.
  - File: `crates/ui/tests/visual_snapshots.rs:compare__steady_state_populated` + `crates/ui/tests/fixtures/mod.rs:compare__steady_state_populated_cockpit()` + baseline `crates/ui/tests/visual-baselines/compare__steady_state_populated.png` (109,613 bytes).
  - Cargo: `cargo test -p ui --test visual_snapshots -- --exact compare__steady_state_populated`.
  - Evidence: `test result: ok. 1 passed; 0 failed; finished in 2.05s`.

- [x] T-D-N12 — Author fixture `compare__empty_cell_run_affordance`:
  matrix with `compare_screen_state.cache` populated for 20 of 24
  legal cells (so 4 cells show the "Run" affordance — exercises the
  active `ACCENT_500` hairline button per R2.3).
  - File: `crates/ui/tests/visual_snapshots.rs:compare__empty_cell_run_affordance` + `crates/ui/tests/fixtures/mod.rs:compare__empty_cell_run_affordance_cockpit()` + baseline `crates/ui/tests/visual-baselines/compare__empty_cell_run_affordance.png` (94,390 bytes).
  - Cargo: `cargo test -p ui --test visual_snapshots -- --exact compare__empty_cell_run_affordance`.
  - Evidence: `test result: ok. 1 passed; 0 failed; finished in 1.82s`.

- [x] T-D-N13 — Author fixture `compare__column_header_hover`:
  matrix with cursor hovering a column header (e.g. "BTCUSDT").
  Per R2.4 v0.1.0 the column header is **non-interactive** (label
  only) — fixture asserts the column-header hover does NOT render
  the `active_row` border tint (distinct from cell hover at T-D-N7).
  - File: `crates/ui/tests/visual_snapshots.rs:compare__column_header_hover` + `crates/ui/tests/fixtures/mod.rs:compare__column_header_hover_cockpit()` + baseline `crates/ui/tests/visual-baselines/compare__column_header_hover.png` (84,356 bytes, identical to cold_boot — non-interactive header confirmed).
  - Cargo: `cargo test -p ui --test visual_snapshots -- --exact compare__column_header_hover`.
  - Evidence: `test result: ok. 1 passed; 0 failed; finished in 1.94s`.

- [x] T-D-N14 — Add layout-invariants proptest case
  `compare_screen_no_zero_dim` at `crates/ui/tests/layout_invariants.rs`:
  256 viewport-size samples (320×240 → 3840×2160) render
  `screens::compare::view`; assert no panic + every cell area ≥ 1 px
  (R2.5).
  - File: `crates/ui/tests/layout_invariants.rs:compare_screen_no_zero_dim` proptest block + `build_compare_cockpit()` helper.
  - Cargo: `cargo test -p ui --test layout_invariants -- compare_screen_no_zero_dim`.
  - Evidence: `running 1 test` + `test result: ok. 1 passed; 0 failed; finished in 2.44s` (256 proptest cases).

- [x] T-D-N15 — Add H5 round-trip unit test
  `open_lab_from_compare_sets_lab_strategy_pair_and_range` at
  `crates/ui/src/state.rs:~3370` (appended to `#[cfg(test)] mod tests`
  after the existing `trail_drawer_closed_clears_drawer_not_selection` at `:3345`).
  Assertions: post-dispatch `current_screen == Screen::Lab`,
  `lab_state.strategy == Some(strategy)`, `lab_state.pair ==
  Some((venue, symbol))`, `lab_state.range == range`. Mirrors the
  Phase D `open_trail_for_sets_pending_audit_id` shape (`:3259-3290`).
  Plus extension test `open_lab_from_compare_no_pair_leaves_pair_unchanged`.
  - File: `crates/ui/src/state.rs:3370-3440` (2 new tests).
  - Cargo: `cargo test -p ui --lib open_lab_from_compare_sets_lab_strategy_pair_and_range`.
  - Evidence: `running 2 tests` + `test result: ok. 2 passed; 0 failed`.

- [x] T-D-N16 — `cockpit-smoke` pre-run with `Screen::Compare` as
  active. Developer confirms `0 panic lines`; tester re-runs at
  M-FINAL per R7.3.
  - Cargo: `cargo test -p ui --test headless_emulator_smoke`.
  - Evidence: `test result: ok. 1 passed; 0 failed; finished in 1.57s`. 0 panic lines confirmed.

## Wave E — M-FINAL handoff prep

- [x] T-D-N17 — Re-run `scripts/verify_anchors.sh` post-implementation.
  **NON-NEGOTIABLE** R7.1 carry-forward gate. Architect verifies once
  after Wave D lands; tester re-verifies at M-FINAL.
  - Cargo: `bash scripts/verify_anchors.sh`.
  - Evidence: `ANCHORS PASS  (22 / 22)`.

- [ ] T-D-N18 — Emit developer HANDOFF → tester envelope per AGENT.md
  § "Structured handoff envelope". Tester then runs the M-FINAL sweep
  per `spec/ui-rethink-phase-e-compare/feature.md ## Acceptance
  criteria § M-FINAL`: `cargo fmt --check`, `cargo clippy
  --workspace -- -D warnings`, `cargo test --workspace --lib`, the 4
  new visual snapshots (T-D-N10..N13), the new layout-invariants case
  (T-D-N14), the H5 round-trip (T-D-N15), `scripts/verify_anchors.sh`,
  cockpit-performance v1.0.0 idle-CPU floor ≤ 13.6 % preserved, and
  authors `reports/test-final-2026-05-20.md` per
  `.claude/skills/rust-test/templates/test-report.md`.

## M-FINAL — Tester sweep — FAIL (T-F1 fmt gate)

> Tester ran 2026-05-20. 9/10 T-F gates green. T-F1 (`cargo fmt --check`) FAILS — 26 diff
> hunks across 9 Phase E files (all cosmetic). Routing back to developer for `cargo fmt`.
> Full report: `spec/ui-rethink-phase-e-compare/reports/test-final-2026-05-20.md`.

- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D
      warnings` exit 0.
- [ ] `cargo test --workspace --lib` 100 % PASS.
- [ ] 4 new snapshot baselines accepted
      (`compare__cold_boot_all_empty`,
       `compare__steady_state_populated`,
       `compare__empty_cell_run_affordance`,
       `compare__column_header_hover`).
- [ ] `scripts/verify_anchors.sh` → 22 / 22 PASS (R7.1).
- [ ] `cockpit-smoke` → 0 panic lines (R7.3).
- [ ] Cockpit-performance v1.0.0 idle-CPU floor ≤ 13.6 % (R7.4, H3).
- [ ] H1 cache-hit enumeration recorded.
- [ ] H4 cold-boot cache scan p99 recorded.
- [ ] H5 round-trip test
      `open_lab_from_compare_sets_lab_strategy_pair_and_range`
      PASS.
- [ ] Author `reports/test-final-<YYYY-MM-DD>.md` per
      `.claude/skills/rust-test/templates/test-report.md`.

## Notes

- **Analyst hand-off shape**: this tasks.md carries only M0 T-A*
  rows + M-OD / M-T1 / M-FINAL placeholders. The architect's M-T1
  pass appends T-T1-* + waves A-G with T-D-N* rows. Developer
  must not pull T-D-N rows before architect locks.
- **Predecessor reference**: Phase D's tasks.md
  (`spec/ui-rethink-phase-d-trail/tasks.md`) is the structural
  template; Phase E follows it 1:1 except for the K5-spike row
  (no architecture-unknown spike needed — the matrix is purely
  additive UI).
- **No cliffs.** Per dev-note §6 line 1134 ("No cliffs at C, E, F
  — each phase is independently shippable and independently
  reversible"), Phase E is reversible via a single revert of the
  `screens::compare` body + `Cockpit::compare_screen_state` field.
