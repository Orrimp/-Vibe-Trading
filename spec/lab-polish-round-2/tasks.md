---
slug: lab-polish-round-2
version: 0.1.0
status: proposed
owner: analyst
updated: 2026-05-25
parent: lab-end-to-end-v2
---

# Lab polish round 2 — task list

Slug: `lab-polish-round-2`

## R1 — Position-curve overlay

### Data layer (backtest crate)

- [x] **T-R1-D1** Add `position_curve: Vec<(i64, Decimal)>` to `SmaComposedRunResult`
  - file:line: `crates/backtest/src/scenarios/sma_composed_run.rs` (struct field + bar-loop emit)
  - test: `cargo test -p backtest`
  - output: `test result: ok. 3 passed; 0 failed`

- [x] **T-R1-D2** Add `position_curve: Vec<(i64, Decimal, Symbol)>` to `MomentumRunResult`
  - file:line: `crates/backtest/src/scenarios/momentum.rs` (struct field + bar-loop emit)
  - test: `cargo test -p backtest`
  - output: `test result: ok. 3 passed; 0 failed`

- [x] **T-R1-D3** Add `position_curve: Vec<(i64, Decimal, Symbol)>` to `PairsRunResult`
  - file:line: `crates/backtest/src/scenarios/pairs.rs` (struct field + bar-loop emit)
  - test: `cargo test -p backtest`
  - output: `test result: ok. 3 passed; 0 failed`

- [x] **T-R1-D4** Add `position_curve: Vec<(i64, Decimal, Symbol)>` to `TcnOverlayRunResult`
  - file:line: `crates/backtest/src/scenarios/tcn_overlay.rs` (struct field + bar-loop emit); `garch_vol_target_overlay.rs` + `tcn_overlay_weights.rs` (placeholder `Vec::new()`)
  - test: `cargo test -p backtest`
  - output: `test result: ok. 3 passed; 0 failed`

- [x] **T-R1-D5** Add `position_curve_raw: Vec<(i64, Decimal, Symbol)>` to `RunReport`; pipe tagged curves through `engine.rs` result-to-report functions
  - file:line: `crates/backtest/src/engine.rs` (RunReport field + result-to-report mapping)
  - test: `cargo test -p backtest`
  - output: `test result: ok. 3 passed; 0 failed`

### UI pipeline (ui crate — runner + mirror)

- [x] **T-R1-U1** Add `position_curve: Vec<(i64, Decimal)>` to `RunSummary`
  - file:line: `crates/ui/src/lab/runner.rs:RunSummary`
  - test: `cargo test -p ui --lib`
  - output: `test result: ok. 377 passed; 0 failed`

- [x] **T-R1-U2** Add `position_curve: Arc<Vec<(i64, Decimal)>>` to `RunReportMirror`
  - file:line: `crates/ui/src/lab/runner.rs:RunReportMirror`
  - test: `cargo test -p ui --lib`
  - output: `test result: ok. 377 passed; 0 failed`

- [x] **T-R1-U3** Filter `position_curve_raw` to active symbol in `spawn_lab_run`
  - file:line: `crates/ui/src/lab/runner.rs:spawn_lab_run` (D-2.5 filter pattern)
  - test: `cargo test -p ui --lib -- widgets::position_curve::tests::position_curve_per_symbol_filter`
  - output: `test position_curve_per_symbol_filter ... ok`

- [x] **T-R1-U4** Update all `RunReportMirror` constructions (`cockpit_live.rs`, `fixtures.rs`, `equity_loader.rs`, `run_delta_badge.rs`, integration tests)
  - file:line: `crates/ui/src/bin/cockpit_live.rs`, `crates/ui/src/fixtures.rs`, `crates/ui/src/lab/equity_loader.rs`, `crates/ui/src/widgets/run_delta_badge.rs`, `crates/ui/tests/lab_run_integration.rs`, `crates/ui/tests/lab_run_real_engine.rs`
  - test: `cargo test -p ui --lib`
  - output: `test result: ok. 377 passed; 0 failed`

### Widget (ui crate — position_curve widget)

- [x] **T-R1-W1** Create `crates/ui/src/widgets/position_curve.rs` — stepped-polyline canvas widget
  - file:line: `crates/ui/src/widgets/position_curve.rs:1`
  - test: `cargo test -p ui --lib -- widgets::position_curve`
  - output: `test result: ok. 5 passed; 0 failed` (4 unit tests + 1 snapshot)

- [x] **T-R1-W2** Register widget in `crates/ui/src/widgets/mod.rs`
  - file:line: `crates/ui/src/widgets/mod.rs:117`
  - test: `cargo test -p ui --lib -- gallery::tests::every_widget_mod_is_listed_in_expected_widgets`
  - output: `test every_widget_mod_is_listed_in_expected_widgets ... ok`

- [x] **T-R1-W3** Add `LAB_POSITION_CURVE_LABEL` string to `crates/ui/src/strings.rs`
  - file:line: `crates/ui/src/strings.rs`
  - test: `cargo test -p ui --lib -- strings`
  - output: `test result: ok. (strings tests pass)`

### Screen wiring (lab.rs)

- [x] **T-R1-S1** Add `POSITION_CURVE_HEIGHT_PX` constant and height-budget accounting in `chart_canvas_height_for_body_with_training`
  - file:line: `crates/ui/src/screens/lab.rs:55` (constant), `:147` (10 gaps), `:160` (`POSITION_CURVE_HEIGHT_PX`)
  - test: `cargo test -p ui --lib -- screens::lab::tests`
  - output: `test result: ok.`

- [x] **T-R1-S2** Wire `position_curve_strip` into `view()` Column layout (between chart and histogram)
  - file:line: `crates/ui/src/screens/lab.rs:619-651` (position_curve_strip construction + column push)
  - test: `cargo test -p ui --lib`
  - output: `test result: ok. 377 passed; 0 failed`

### Gallery

- [x] **T-R1-G1** Add `position_curve` to `gallery/routes.rs` `EXPECTED_WIDGETS` + GalleryCells (2 cells: `with_points` + `empty`)
  - file:line: `crates/ui/src/gallery/routes.rs` (import + seed/render fns + cells + EXPECTED_WIDGETS)
  - test: `cargo test -p ui --lib -- gallery::tests`
  - output: `test result: ok.`

- [x] **T-R1-G2** Add `fake_position_curve_points()` fixture to `crates/ui/src/fixtures.rs`
  - file:line: `crates/ui/src/fixtures.rs` (before `fake_run_report_mirror_pair`)
  - test: `cargo test -p ui --lib -- gallery::tests`
  - output: `test result: ok.`

- [x] **T-R1-G3** Bump `GALLERY_LOGICAL_HEIGHT` in `gallery/mod.rs` from 17_000 to 17_520 (+2 cells)
  - file:line: `crates/ui/src/gallery/mod.rs:74`
  - test: `cargo test -p ui --lib -- gallery::tests::gallery_logical_height_covers_all_cells`
  - output: `test gallery_logical_height_covers_all_cells ... ok`

### Regression gate

- [x] **T-R1-RG** Anchors 34/34 PASS after all data-layer changes
  - command: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (34 / 34)`

## R2 — Strategy parameter editor (SMA fast/slow)

Shipped at commits `c1cddbe` + `ae26281`.

- [x] R2 complete (shipped in prior session)

## R3 — KPI strip densification

Shipped at commit `371d870`.

- [x] R3 complete (shipped in prior session)
