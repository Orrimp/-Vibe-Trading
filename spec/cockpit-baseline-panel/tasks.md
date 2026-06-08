---
slug: cockpit-baseline-panel
status: in-progress
owner: architect
updated: 2026-06-08
---

# Tasks — cockpit-baseline-panel

Proportionate S–M read-only UI feature. Surfaces the shipped passive-BH
result as `Screen::Baseline`, reusing `equity_curve` / `kpi_strip` /
`drawdown_band` verbatim. Design + decisions: [feature.md § Design](feature.md).

## Who implements what (parallel: ui-designer ‖ developer)

The loader is **cleanly separable** (pure `std::fs` + `core`, mirrors
`models/registry_read.rs`), so the two implementers run in parallel:

- **developer** — the pure-`ui` data layer: `baseline/loader.rs`
  (CSV → `EquitySeries`) + the embedded §7.1 metrics const + loader/metrics
  unit tests (T2, T3, T8).
- **ui-designer** — the surface: `baseline/state.rs`, `screens/baseline.rs`,
  `strings.rs` block, sidebar IA, panel snapshots, Lumen-consistency (T1,
  T4, T5, T6, T7, T9, T10).

**Shared touchpoint = `state.rs`** (T1): the `Screen::Baseline` variant +
`Cockpit` field + `Message::BaselineSelectYear` arm. Land T1 first (either
agent), then T2/T3 (developer) and T4–T7 (ui-designer) proceed
independently. T8–T10 (tests) close after their subjects land.

> If the orchestrator prefers a single owner, **ui-designer solo** is
> viable (the loader is small) — but the clean loader/screen seam makes
> ui-designer ‖ developer the lower-wall-clock path. Recommended: parallel.

## Tasks

- [ ] **T1 — `Screen::Baseline` + state touchpoints** —
  `crates/ui/src/state.rs` —
  _Add `Screen::Baseline` variant (after `Strategies`, before deprecated
  aliases); add `BaselineYear { Y2023, Y2024 }` (`Default = Y2024`); add
  `baseline_screen_state: BaselineScreenState` to `Cockpit` (enum +
  `Default` + `Debug`); add `Message::BaselineSelectYear(BaselineYear)`
  arm (NO `String` payload) and its `update` handler (sets `active_year`)._
  — _acceptance: `cargo build -p ui` green; `Message::BaselineSelectYear`
  is typed; `current_screen = Screen::Baseline` compiles. (R2, R6, AC1)_

- [ ] **T2 — `baseline/loader.rs` curve loader** (developer) —
  `crates/ui/src/baseline/loader.rs` (+ `baseline/mod.rs`) —
  _Pure-`ui` synchronous loader, no new crate edge. `load_baseline_curve(
  &Path) -> PanelState<EquitySeries>`: read 3-column CSV
  (`bar_index,timestamp_utc,equity_usd`), parse `timestamp_utc` as
  minute-precision Zulu (`[year]-[month]-[day]T[hour]:[minute]Z` via `time`
  format_description — NOT `Rfc3339`), `equity_usd` → `Money<Usdt>` via
  `Decimal` (never `f64`), ignore `bar_index`, preserve file order →
  `EquitySeries::from_points`. Missing file / parse miss / `from_points`
  err → `PanelState::Error(BASELINE_DATA_UNAVAILABLE)`; zero data rows →
  `Empty`. Never panics (mirror `registry_read.rs` K2 +
  `viewer::load_equity_companion`). Isolate the workspace-root path helper
  in one fn so T8 can point it at a bogus path._ — _acceptance: loads both
  committed CSVs to `Ready` with first point `$100,000.00`; missing path →
  `Error`, no panic. (R1, R4, R7, AC1, AC3)_

- [ ] **T3 — embedded §7.1 metrics const** (developer) —
  `crates/ui/src/baseline/loader.rs` —
  _`baseline_metrics(BaselineYear) -> BacktestMetrics` returning the
  realized §7.1 row (NOT bootstrap p50). Values per [feature.md § Design
  table]: 2023 = total/cagr `196.22`, sharpe `1.8417`, maxDD `34.57`,
  trades `0`, win_rate absent; 2024 = `91.04` / `91.04` / `0.8925` /
  `48.95` / `0` / absent. `cagr_present = sharpe_present = true`,
  `win_rate_present = false`. Carry a `// RE-SYNC:` doc-comment naming
  `passive-baseline-characterization.md §7.1` + the exact values, and a
  note that CAGR=total_return is a correct 1-yr-horizon derivation, not a
  copied value._ — _acceptance: returns the locked values for each year;
  `kpi_strip` renders them (Sharpe via `format_sharpe` 4-dp). (R1, A1, A2,
  AC1)_

- [ ] **T4 — `baseline/state.rs`** (ui-designer) —
  `crates/ui/src/baseline/state.rs` —
  _`BaselineScreenState { curve_2023: PanelState<EquitySeries>, curve_2024:
  PanelState<EquitySeries>, active_year: BaselineYear }`. `Default`:
  `active_year = Y2024`, both curves `Loading`. Boot-load both curves via
  `loader::load_baseline_curve` (a helper the bins call at boot, mirroring
  `cockpit.rs` fixture pre-seed). Metrics are NOT stored — pulled from the
  const at view time._ — _acceptance: `Default` compiles; boot helper
  populates both curves; visiting Baseline shows 2024 by default. (R1, R2,
  R4)_

- [ ] **T5 — `screens/baseline.rs` view** (ui-designer) —
  `crates/ui/src/screens/baseline.rs` (+ `screens/mod.rs`) —
  _`view(&Cockpit, ThemeMode)` composing top→bottom: headline
  (`BASELINE_HEADLINE` H2) + year chips `[2023][2024]` (Compare/Lab chip
  pattern, `on_press(Message::BaselineSelectYear(y))`, focusable +
  Enter-activatable) → caption (`BASELINE_CAPTION`) →
  `kpi_strip::view(&Ready(baseline_metrics(active_year)), mode)` →
  `equity_curve::view(&curve[active_year], mode)` →
  `drawdown_band::view(&curve[active_year], mode)` → optional
  `BASELINE_RISK_DETAIL` Sortino/Calmar line (FG_3). Bridge the three
  `ViewerMessage` widgets with `.map(|_| Message::ChartMarkerHoverEnded)`
  per `screens/live.rs:62,67`. Add the `Screen::Baseline =>
  baseline::view(model, mode)` arm to `shell::screen_body`._ — _acceptance:
  Baseline renders curve+band+6-card strip for 2024; toggling to 2023 swaps
  curve+metrics; zero hardcoded strings/colours. (R1, R2, R3, R5, AC1)_

- [ ] **T6 — `BASELINE_*` strings block** (ui-designer) —
  `crates/ui/src/strings.rs` —
  _Add: `BASELINE_SIDEBAR_LABEL`="Baseline", `BASELINE_HEADLINE`="Passive
  baseline", `BASELINE_CAPTION` (equal-weight buy-and-hold across 10
  large-cap pairs, bought once at year-open, never rebalanced; honest
  finding "passive baseline; active ≤ passive in the reachable universe,
  this sample" — **MUST NOT** say "optimal"/"unbeatable"/"none beat it"),
  `BASELINE_YEAR_2023_LABEL`/`BASELINE_YEAR_2024_LABEL`,
  `BASELINE_DATA_UNAVAILABLE` (error copy: what to do next),
  `BASELINE_RISK_DETAIL` (Sortino 2.51/Calmar 5.68 [2023], Sortino
  1.20/Calmar 1.85 [2024])._ — _acceptance: all Baseline copy resolves via
  `strings`; caption passes the no-overclaim assertion. (R3, R5, A3, AC5)_

- [ ] **T7 — sidebar IA** (ui-designer) — `crates/ui/src/theme.rs` —
  _Add `Screen::Baseline` to `SIDEBAR_GROUPS_PHASE_C` **Work** group after
  `Compare`, AND to `SIDEBAR_ENTRIES_PHASE_A` after `Compare` (both must
  stay lock-step or the flatten-invariant test fails — that is the guard).
  Label via `BASELINE_SIDEBAR_LABEL`. Do NOT change the cockpit smoke's
  default screen (D2 — navigable only)._ — _acceptance:
  `sidebar_groups_phase_c__flatten_matches_phase_a` green with Baseline in
  both consts; Baseline reachable from the sidebar. (R6, D2, AC6)_

- [ ] **T8 — loader + Error-state tests** (developer) —
  `crates/ui/src/baseline/loader.rs` (`#[cfg(test)]`) +
  `crates/ui/tests/baseline_error_state.rs` —
  _Unit: parse `…T00:00Z` timestamp shape; load committed CSV → `Ready`
  first-point `$100,000.00`; `baseline_metrics_match_characterization`
  asserts the six embedded scalars == documented §7.1 (the re-sync trip).
  Integration `baseline_error_state_renders_without_panic`: loader at a
  **missing** path → both curves `Error(BASELINE_DATA_UNAVAILABLE)`, and
  `screens::baseline::view` renders in **both themes** with no panic — the
  deterministic stand-in for the fixtures-only smoke path._ — _acceptance:
  all green; missing-CSV path proven non-panicking. (R4, R7, D1 re-sync,
  D2, AC2, AC3)_

- [ ] **T9 — Baseline panel snapshot (both themes)** (ui-designer) —
  `crates/ui/tests/panel_snapshots.rs` —
  _Add a Baseline-screen snapshot per the 267-test convention: set
  `current_screen = Screen::Baseline`, snapshot Dark + Light (Ready state
  with committed curve; and an unavailable/Error variant if cheaply
  reachable). Follow the existing `Screen::X` snapshot pattern
  (`panel_snapshots.rs:2682` etc.)._ — _acceptance: new snapshots accepted;
  suite green in both themes. (R4, R5, AC2, AC6)_

- [ ] **T10 — Lumen-consistency green** (ui-designer) —
  `crates/ui/tests/{consistency,contrast,layout_invariants}.rs` (no new
  invariants expected; just stay green) —
  _Confirm `tests/consistency.rs` / `tests/contrast.rs` /
  `tests/layout_invariants.rs` pass with the new screen: no hardcoded
  colours (tokens only), no hardcoded strings, no new theme token, both
  themes render. New code follows the crate's existing per-module
  `#![allow(...)]` lint convention; does NOT introduce new warnings and
  does NOT touch the pre-existing ~140 pedantic lints._ — _acceptance:
  three consistency suites green; AC7 review confirms pure-`ui`, no new
  edge/widget/token. (R5, AC4, AC7)_

## Validation gate (tester)

After T1–T10 land, the tester runs:

- `rust-build` + `rust-validate` (`cargo clippy -p ui -- -D warnings` for
  **new** code; pre-existing lints out of scope).
- The cockpit fixtures smoke (`headless_emulator_smoke.rs`) — first-frame
  render, **no panic** (Baseline navigable; default screen unchanged).
- The four-state behaviour (T8) + panel snapshots (T9) + Lumen
  consistency (T10) + the caption no-overclaim assertion (T6).
- Confirms the **119/119 regression anchor gate is unchanged** (read-only
  UI feature — no new backtest anchors).

Maps to AC1–AC7 in [feature.md § Acceptance criteria](feature.md).

## Notes

- **No ADR** — every decision is pure-`ui`, additive, within the
  `viewer`/`registry_read` precedent (architect call; see feature.md
  § Design).
- **No new crate edge / widget / theme token** (AC7). `ui → backtest`
  already exists (ADR-0030, Lab Run only) and is **not used** here.
- **Money math is `Decimal`/`Money<Usdt>`** in the loader + metrics const —
  never `f64` (CLAUDE.md non-negotiable).
- **CLAUDE.md baseline-equity-divergence e2e gate does NOT apply** — this
  is a read-only panel, not a strategy overlay / sizing modifier (no
  overlay, no sizing math, no decision variable; see feature.md § Backtest
  Scenarios).
- **Do not touch** anchored reports, `data/yahoo/REVISION.toml`, or the
  BH CSVs (read-only; non-anchored, safe to read).
