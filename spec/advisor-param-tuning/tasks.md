---
slug: advisor-param-tuning
status: in-progress
owner: developer
updated: 2026-06-25
phase-1-complete: true   # T1–T5 (engine + mirror) shipped; T6–T11 remain
---

# Tasks — advisor-param-tuning (gate-tied hyperparameter sweep editor)

Sequenced so the engine seam + the FAIL-before gates land before any UI, and the
whole vertical slice ships on **SMA first** (the family that already has a runtime
override) before the MACD/RSI/Bollinger string-generation work. Each task lists its
acceptance + verification. ADR-0069 is the design of record.

Owner key: **[dev]** developer (engine + glue), **[ui]** ui-designer (screen body +
render). Dev and ui can parallelise from T8 once the mirror shape (T5) is frozen.

## Phase 1 — engine seam (the gate-tied sweep core), SMA-first

- [x] **T1 [dev] — additive `compute_robustness_distribution` + refactor
  `compute_robustness_flag` to delegate.** In `crates/backtest/src/bakeoff/bootstrap.rs`,
  add `compute_robustness_distribution(&[Decimal], paths, seed) ->
  Option<(DistributionSummary, ParamRobustnessVerdict)>` returning the SAME summary
  `compute_robustness_flag` builds internally; refactor `compute_robustness_flag` to
  call it and drop the summary. _Acceptance: the gate bands + seed rule are untouched._
  _Verify: `compute_robustness_distribution_matches_flag` — for a panel of equity
  curves, the new fn's verdict is bit-identical to `compute_robustness_flag`'s output
  (proves behaviour-preserving + the gate frozen). FAIL-before is N/A (new fn); the
  bit-identity assertion is the gate._
  - file:line: `crates/backtest/src/bakeoff/bootstrap.rs:119` (new `compute_robustness_distribution`) + `bootstrap.rs:177` (delegation in `compute_robustness_flag`)
  - Test command: `cargo test -p backtest --test compute_robustness_distribution_matches_flag`
  - Output: `test result: ok. 8 passed; 0 failed; 0 ignored` (8 bit-identity tests)

- [x] **T2 [dev] — `build_swept_strategy` (SMA arm only) + the `SweepFamily` /
  `SweptParams` / `SweepGrid` types.** New module `crates/backtest/src/bakeoff/sweep.rs`.
  `SweepFamily` (closed enum), `SweptParams` (the concrete per-cell params),
  `SweepGrid` (family {min,max,step} axes + the cap-aware `enumerate()` →
  `Vec<SweptParams>` + a `truncated`/`requested_count` carrier), and
  `build_swept_strategy(family, &SweptParams)` for the **SMA** arm — it threads
  `sma_fast_len/sma_slow_len` into the `ScenarioConfig` (the existing override).
  _Acceptance: an SMA `SweptParams{fast,slow}` produces a `ScenarioConfig` carrying
  those fast/slow overrides; the composed arms `todo!()`/`unimplemented` cleanly (T7)._
  _Verify: `sweep_grid_truncates_at_cap` (>24 cells → exactly 24 + truncated flag +
  requested_count) and `sweep_drops_invalid_sma_cells` (`fast ≥ slow` cells dropped)._
  - file:line: `crates/backtest/src/bakeoff/sweep.rs:1` (new module, types at lines 74-310, `build_swept_config` at 355)
  - Test command: `cargo test -p backtest 'sweep' --lib`
  - Output: `test result: ok. 12 passed; 0 failed; 0 ignored` (12 unit tests)

- [x] **T3 [dev] — `run_param_sweep` orchestrator (SMA-only end-to-end).** In
  `sweep.rs`: the `run_bakeoff`-shaped async fn — preload bars ONCE via
  `resolve_bakeoff_bars` (apples-to-apples), loop the enumerated grid, `run_scenario`
  per cell with `write_report = false`, score via `compute_robustness_distribution`,
  collect `SweepCellResult`, always include the shipped-config `baseline` cell + the
  buy-and-hold `benchmark` KPIs, check `cancel_rx` before each cell, emit
  `SweepProgress` per cell. Returns `SweepReport`. _Acceptance: an SMA sweep over a
  small grid on the synthetic data source returns a `SweepReport` with one cell per
  valid grid point, each carrying a verdict + distribution + KPIs, plus the baseline +
  benchmark._ _Verify: an integration test on `ScenarioDataSource::Synthetic` asserts
  the cell count, that every cell has a populated distribution, and that a cancelled
  run returns `RunError::Cancelled`._
  - file:line: `crates/backtest/src/bakeoff/sweep.rs:481` (`run_param_sweep` function)
  - Test command: `cargo test -p backtest --test param_sweep_divergence_end_to_end`
  - Output: `test result: ok. 9 passed; 0 failed; 0 ignored` (5 T3 tests + 4 T4 tests)

- [x] **T4 [dev] — THE day-1 divergence e2e (SMA).** `crates/backtest/tests/
  param_sweep_divergence_end_to_end.rs` — over a ≥2-cell SMA grid on a ≥1-fill
  fixture: (a) ≥1 swept cell's realized equity diverges from `report.baseline` by
  ≥1 bp at some bar; (b) cells are not all identical to each other; (c) the concrete
  pin `(fast=10, slow=20)` ≠ `(fast=20, slow=50)` baseline. _Acceptance: FAIL-before
  if `build_swept_strategy` returns the shipped config for every cell; PASS-after._
  _Verify: the test itself (this IS the CLAUDE.md non-negotiable gate). Modelled on
  `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`._
  - file:line: `crates/backtest/tests/param_sweep_divergence_end_to_end.rs:1` (new file, primary gate at line 228 `t4_swept_cells_diverge_from_baseline`, concrete pin at line 354 `t4_concrete_pin_fast10_slow20_differs_from_baseline`, FAIL-before control at line 395 `t4_identical_params_produce_identical_equity_the_positive_control`)
  - Test command: `cargo test -p backtest --test param_sweep_divergence_end_to_end`
  - Output: `test result: ok. 9 passed; 0 failed; 0 ignored`

- [x] **T5 [dev] — `SweepReportMirror` + `from_report` (the ONE boundary).** In
  `crates/ui/src/` (new `tune/state.rs` or alongside leaderboard state): the pure-`ui`
  mirror — `SweepReportMirror { family_label, coin, range_label, grid_size,
  truncated, requested_count, cells: Vec<SweepCellRow>, baseline: SweepCellRow,
  benchmark_kpis }` with `SweepCellRow { params_label, verdict: RobustnessLabel,
  in_sample_return, sharpe_p5/p50/p95, prob_loss, prob_sharpe_gt1, maxdd_p95,
  promotable: bool }` and the `from_report(&backtest::SweepReport)` seam (the ONLY
  place a `SweepReport` is read). Reuse the existing `RobustnessLabel`. _Acceptance:
  `cargo tree -p ui` is UNCHANGED (no new crate edge); the mirror is unit-constructible
  in fixtures without the engine._ _Verify: a `from_report` unit test maps a
  hand-built `SweepReport` (mix of Robust/Marginal/Fragile, one truncated) to the
  expected mirror; `promotable == false` iff `verdict == Fragile`._
  - file:line: `crates/ui/src/tune/state.rs:1` (new file); `from_report` at line 168; `promotable` gate at line 193 (`!matches!(verdict, RobustnessLabel::Fragile)`)
  - Test command: `cargo test -p ui tune::state`
  - Output: `test result: ok. 8 passed; 0 failed; 0 ignored`

## Phase 2 — UI (Lab sub-view), can parallelise once T5 lands

- [ ] **T6 [ui] — the Tune screen body + guided range form + `Screen::Tune` route +
  `OpenTuneEditor` drill-down.** `crates/ui/src/screens/tune.rs`: the range form (per
  axis: min/max/step typed fields via `Message::SweepAxisEdit` + narrow/shipped/wide
  preset chips), the live "N configs → ~M runs (~T)" readout (reads
  `backtest::bakeoff::sweep::MAX_SWEEP_CONFIGS`), the result grid (rows = cells;
  columns params / verdict / return / Sharpe p5·p50·p95 / P(loss) / P(Sharpe>1) /
  Max-DD p95), the FRAGILE badge (reuse the leaderboard pill), the
  promotion-disabled-on-fragile "Use this config" affordance, the shipped-baseline
  row tag, the buy-and-hold header strip, the truncation banner, and the persistent
  honesty footer. Add `Screen::Tune` (navigable, not default-routed) + the
  `Message::OpenTuneEditor{family,coin,lookback}` arm from a per-row "Tune…" button
  on the Leaderboard (mirror `InspectStrategyFromLeaderboard`) + a Lab entry point.
  All copy in `crates/ui/src/strings.rs`. _Acceptance: the screen renders the form +
  (when Ready) the grid through `PanelState`; `PanelState::Empty` shows the "set
  ranges and press Run sweep" prompt; the modal-overlay alternative is acceptable._
  _Verify: covered by T9 (render) — no text-snapshot/no-panic-boot is sufficient._

- [ ] **T7 [dev] — `build_swept_strategy` for MACD / RSI / Bollinger (the
  string-generation gap).** Extend `build_swept_strategy` to generate the `signal`
  DSL string per family from the swept params (`macd_hist(f,s,sig) > 0 AND close >
  ema(200)`, `rsi(p) < t AND close > min(low,20)`, `close < bollinger_lower(p,k) AND
  volume > 1.5 * avg(volume,20)`), build the TOML string, and parse it via
  `ComposedStrategyConfig::from_str` → `ComposedStrategy`. Wire these families into
  the Tune form's family picker. _Acceptance: a generated MACD/RSI/Bollinger TOML
  string parses cleanly and runs in `run_param_sweep`; the secondary clauses (trend
  filter / support window / volume confirm) stay fixed per the v0.1 scope cut._
  _Verify: `build_swept_strategy_macd_roundtrips` (+ rsi + bbands) — a generated TOML
  string parses through `from_str` and its AST equals a hand-written fixture for the
  same params; AND the T4 divergence e2e is EXTENDED to assert ≥1 cell of EACH of
  MACD/RSI/Bollinger diverges from its shipped baseline (the no-op guard now covers
  the composed families too)._

## Phase 3 — render-pixel verification (the CLAUDE.md UI gate)

- [ ] **T8 [ui] — Tune-screen fixtures.** In `crates/ui/src/fixtures.rs`: a populated
  `SweepReportMirror` (a mix of Robust + Marginal + ≥1 FRAGILE cell, a shipped-baseline
  row, a buy-and-hold strip), a `Cockpit` routed to `Screen::Tune` with `Ready` /
  `Empty` / mid-`SweepProgress` states, and a non-default range-form selection.
  _Acceptance: the fixtures are deterministic + engine-free._ _Verify: consumed by T9._

- [ ] **T9 [ui] — render-pixel guards.** `crates/ui/tests/param_sweep_render.rs`,
  `#![cfg(target_os = "macos")]` (ADR-0057 § D2), serialising the screenshot harnesses
  to avoid the macOS cosmic-text font-mutex deadlock
  (`spec/dev-notes/iced-ui-render-verification.md`). Guards: (1)
  `sweep_populated_paints_grid_and_fragile_badge` — the grid rows + the FRAGILE badge
  clay in the params column (reuse `fragile_badge_clay`) + the distribution columns +
  the baseline-row tag paint; (2) `sweep_empty_paints_no_grid` — NEGATIVE control:
  Empty paints ~no fragile clay + far less foreground (proves (1) is not a tautology);
  (3) `sweep_fragile_promote_disabled_paints` — a FRAGILE row's greyed "Use this
  config" affordance vs a Robust row's enabled accent affordance (a strictly-more-
  accent discriminator); (4) `sweep_progress_determinate_paints` — mid-sweep partial
  bar (model `bakeoff_progress_render.rs`). Each writes its PNG to `/tmp/…` for the
  operator. _Acceptance: read the PNGs — the grid + FRAGILE badge + distribution
  actually draw, with the negative control discriminating._ _Verify: the four guards._

## Phase 4 — close the loop

- [ ] **T10 [dev] — runner glue + cancellation + the no-`live` fixtures-Err path.**
  `crates/ui/src/tune/runner.rs` (mirror `leaderboard/runner.rs`): `spawn_sweep(rt,
  cfg, cancel, progress_tx, sweep_progress_tx) -> iced::Task<Message>` resolving to
  `Message::SweepRunCompleted(Result<SweepReportMirror, SmolStr>)`, plus
  `sweep_config_from_state` (operator's family + coin + lookback + ranges → `SweepConfig`,
  `data_source = BinanceCache`, `seed = LAB_DEFAULT_SEED`, `paths = 1000`). The
  no-`live` build resolves immediately with the friendly LEADERBOARD_RUN_NEEDS_LIVE-style
  error. _Acceptance: the iced thread never blocks; cancellation trips the run._
  _Verify: a `sweep_config_from_state` unit test (carries the chosen family/coin/
  lookback/ranges) + the no-`live` immediate-Err test._

- [ ] **T11 [dev] — anchors + trace + spec close.** Run `bash scripts/verify_anchors.sh`
  → MUST stay **119/119** (every sweep cell is `write_report = false`). Fill the
  `crates`/`tests` columns of the `REQ-ADVISOR-PARAM-TUNING-001` row in
  `spec/trace.toml` (analyst seeds the row; architect filled `arch` =
  `["spec/architecture/adr/0069-gate-tied-parameter-sweep-seam.md", "spec/architecture.md"]`).
  _Acceptance: 119/119; trace row resolves; `scripts/adr_registry_check.py
  --pre-commit` passes (ADR-0069 registered)._ _Verify: the two scripts._

## Notes

- **Engine gap flagged (ADR-0069 § D3):** MACD/RSI/Bollinger have NO runtime param
  override today — `build_registry_for` hardcodes the TOML filename per id. T7 is the
  string-generation builder that closes it. **Do not** attempt to sweep the composed
  families before T7; ship SMA-only first (T1–T6, T8–T11 work SMA-only) and treat T7
  as the family-completion increment.
- **Do NOT reuse `param_robustness_sweep.rs`.** It is bin-only and built for a
  10-symbol momentum universe / `ThetaCell` grid. The reused pieces are the library
  `classify_verdict` + `compute_robustness_flag` (+ the new `compute_robustness_distribution`)
  and the `run_bakeoff` shape.
- **The gate is frozen.** No edit to `verdict_bands` or the bootstrap seed rule. T1's
  bit-identity test is the guard.
- **Promotion is out of scope for v0.1** — the editor SHOWS the gate verdict and
  disables promotion on FRAGILE rows; carrying a tuned config into F4 (sizing) / F5
  (forward paper) is a v0.2 follow-on.
