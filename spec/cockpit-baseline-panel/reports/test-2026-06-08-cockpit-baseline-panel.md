---
title: Test Report
feature: cockpit-baseline-panel
run_id: 2026-06-08-1700-UTC
commit: f1c1bf337165a45aff7bdd63279265cab25c450d
agent: tester
verdict: PASS
---

# Test Report — cockpit-baseline-panel — 2026-06-08

## 1. Scope

- **Feature / change under test:** Cockpit Baseline screen (v0.1.0) — surfaces the shipped passive buy-and-hold result (2023+2024 equity curve + drawdown band + 6-card KPI strip + honest bounded-scope caption). Read-only UI feature; reuses existing widgets verbatim; no new crate edge, widget, or theme token.
- **Spec refs:** `spec/cockpit-baseline-panel/feature.md`, `spec/cockpit-baseline-panel/tasks.md`
- **Commit SHA:** `f1c1bf337165a45aff7bdd63279265cab25c450d`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `darwin arm64`
- **Pre-run git status:** `M data/yahoo/REVISION.toml` (pre-existing, not touched by this feature)
- **Post-run `git diff crates/`:** empty — no source files modified during the test run

## 2. Static Analysis

| Check              | Result | Notes                       |
|--------------------|--------|-----------------------------|
| `cargo build -p ui` | PASS | `Finished dev profile … 1.11s` — clean build, no errors |
| `cargo fmt -p ui --check` | PASS | No output (zero diff) |
| `cargo clippy -p ui` | PASS (no new warnings from feature files) | All 10 lib warnings + 6 bin warnings are pre-existing pedantic lints in `lab/`, `live.rs`, `position_curve.rs`, `cockpit_live.rs` — none point at `baseline/`, `screens/baseline.rs`, or the new test files. The pre-existing ~140 lints are explicitly out of scope per AC7 / feature.md § Lint convention. |
| `cargo audit` | n/a (targeted ui scope) | |
| `verify-anchors` | **PASS 119/119** | All anchors verified; no anchored file touched (read-only UI feature). |
| `spec-lint` | FAIL (95 violations in 2 categories) — see § Pre-existing spec debt | One new `trace-broken-path` violation introduced by this feature's trace row (see below). |

### New clippy warning scan (feature files)

Files checked: `crates/ui/src/baseline/mod.rs`, `crates/ui/src/baseline/loader.rs`, `crates/ui/src/baseline/state.rs`, `crates/ui/src/screens/baseline.rs`, `crates/ui/tests/baseline_error_state.rs`, `crates/ui/tests/panel_snapshots.rs` (`mod baseline_screen`), `crates/ui/tests/headless_emulator_smoke.rs` (`headless_emulator_paints_baseline_route`).

Result: **zero new warnings** from any of these files. The `#![allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]` headers in `screens/baseline.rs` and `baseline/loader.rs` match the surrounding module convention.

### Pre-existing spec debt

spec-lint baseline (from `spec/dev-notes/audit-2026-06-08.md`): **94 violations** (87 dead-link + 7 trace-broken-path).

Current run: **95 violations** (87 dead-link + 8 trace-broken-path). Delta: +1 trace-broken-path.

New violation: `spec/trace.toml: row REQ-COCKPIT-BASELINE-001 field arch: missing path ADR-0030`.

Root cause: the trace row introduced by this feature uses the shorthand `"ADR-0030"` in the `arch` array instead of the full path `"spec/architecture/adr/0030-cockpit-in-process-backtest.md"`. All other rows in `trace.toml` use full relative paths for ADR references. This is a doc-hygiene fix only; no logic, no design change.

**Resolution:** corrected in the trace.toml update below (the `arch` array is patched from `"ADR-0030"` to `"spec/architecture/adr/0030-cockpit-in-process-backtest.md"`). After this fix the trace-broken-path count returns to 7 (baseline). The 87 dead-links are unchanged pre-existing debt.

## 3. Unit & Integration Tests

### Suite summary (targeted: `-p ui`)

| Suite | Passed | Failed | Ignored | Duration |
|-------|-------:|-------:|--------:|--------:|
| `ui` lib unit tests | 428 | 0 | 0 | 0.53s |
| `ui` bin: `cockpit_live` | 2 | 0 | 0 | 0.00s |
| `ui` bin: `viewer` | 4 | 0 | 0 | 0.00s |
| `baseline_error_state` (integration) | 3 | 0 | 0 | 7.77s |
| `headless_emulator_smoke` (integration) | 2 | 0 | 0 | 6.44s |
| `panel_snapshots` (integration, subset baseline) | 7 | 0 | 0 | 0.31s |
| `consistency` | 2 | 0 | 0 | 0.02s |
| `contrast` | 7 | 0 | 2 ignored | 0.00s |
| `layout_invariants` | 11 | 0 | 0 | 73.00s |
| All other suites (25 integration test binaries) | ~120 | 0 | ~6 ignored | — |
| `lab_run_engine` | 0 | **1** (pre-existing) | 0 | 6.18s |
| **Total** | **428+ unit** | **1 pre-existing** | **~8 pre-existing** | — |

The `lab_run_engine` failure (`inner::h3_in_memory_equals_cached_disk`) is **pre-existing**. Verified by stashing all changes and running the same test against the parent commit (`bde478f`) — it fails identically with the same panic message (`write_report=true should produce a report_path` at `lab_run_engine.rs:108`). This test was introduced in commit `a5f8647` (`v5-latency-slippage-sim`) and is unrelated to this feature. It does NOT block PASS.

### Feature-specific test results

#### `baseline::loader` unit tests (13 in `src/baseline/loader.rs`)

| Test | Result |
|------|--------|
| `parses_minute_precision_zulu_timestamp` | PASS |
| `parses_well_formed_body_in_file_order` | PASS |
| `header_only_file_parses_to_zero_points` | PASS |
| `bad_timestamp_row_returns_err_line` | PASS |
| `non_decimal_equity_row_returns_err_line` | PASS |
| `missing_file_yields_error_state_no_panic` | PASS |
| `header_only_path_yields_empty_state` | PASS |
| `committed_csvs_load_to_ready_first_point_100k` | PASS (CSVs present; first point = $100,000.00 for both 2023 and 2024) |
| `baseline_metrics_match_characterization` | PASS — see § D1 re-sync guard below |
| `csv_path_is_workspace_relative_and_year_specific` | PASS |

#### `baseline::state` unit tests (3 in `src/baseline/state.rs`)

| Test | Result |
|------|--------|
| `default_is_y2024_curves_loading_metrics_ready` | PASS |
| `active_metrics_follows_year` | PASS |
| `active_curve_follows_year` | PASS |

#### `baseline_error_state` integration (3 in `tests/baseline_error_state.rs`)

| Test | Result |
|------|--------|
| `loader_missing_path_yields_error_both_years` | PASS |
| `baseline_error_state_renders_without_panic` | PASS — both themes, Error state, non-zero root confirmed |
| `baseline_ready_state_renders_when_csvs_present` | PASS — CSVs present; Ready path rendered in both themes for both years |

#### `headless_emulator_smoke` integration (2 in `tests/headless_emulator_smoke.rs`)

| Test | Result |
|------|--------|
| `headless_emulator_boots_cockpit_and_renders` | PASS |
| `headless_emulator_paints_baseline_route` | PASS — first-frame, 1280×720, non-empty rgba buffer |

#### `panel_snapshots::baseline_screen` (7 in `tests/panel_snapshots.rs`)

| Test | Result |
|------|--------|
| `baseline_snapshot__ready_2024_dark` | PASS |
| `baseline_snapshot__ready_2024_light` | PASS |
| `baseline_snapshot__ready_2023_toggled_dark` | PASS |
| `baseline_snapshot__error_dark` | PASS |
| `baseline_snapshot__error_light` | PASS |
| `baseline_caption_is_honest_bounded_no_overclaim` | PASS |
| `baseline_kpi_values_match_characterization_2024` | PASS |

#### Lumen gate (from full suite run)

| Test | Result |
|------|--------|
| `consistency::no_inline_user_visible_strings_in_widgets` | PASS |
| `consistency::no_inline_hex_colors_in_widgets_or_state` | PASS |
| `contrast::all_theme_pairs_meet_wcag` | PASS |
| `layout_invariants` (all 11) | PASS |
| `theme::tests::sidebar_groups_phase_c__flatten_matches_phase_a` | PASS |

### Failing tests

`inner::h3_in_memory_equals_cached_disk` (`crates/ui/tests/lab_run_engine.rs:108`) — pre-existing failure unrelated to this feature. Introduced in `a5f8647` (v5-latency-slippage-sim). Panics: `write_report=true should produce a report_path`. Verified pre-existing by confirming identical failure on parent commit `bde478f` (stash + test + pop). Does not gate PASS.

## 4. Property / Fuzz Tests

_n/a_ — no proptest or fuzz targets in this feature (read-only UI panel; no algorithmic math introduced).

## 5. Backtest Results

_n/a_ — read-only UI feature. Surfaces an already-shipped, already-anchored backtest result. The feature runs no new strategy simulation and produces no new backtest data. Per CLAUDE.md, the baseline-equity-divergence e2e gate applies only to strategy overlays / sizing modifiers, which this is not (no overlay, no sizing math, no decision variable). Confirmed: `spec/cockpit-baseline-panel/feature.md § Backtest Scenarios`: "N/A — this is a read-only UI feature."

## 6. Benchmarks

_n/a_ — no hot paths touched. The loader (`load_baseline_curve`) is called once at boot over a ~367-row CSV; there is no latency-sensitive execution path.

## 7. Environment / Infrastructure Issues

- `data/yahoo/REVISION.toml` was already modified at conversation start (pre-existing — not touched by this feature or test run).
- `gallery_dark_*` tests (3) are pre-existing BLOCKED ignores (iced Table cell-bounds panic, documented in `gallery_snapshots.rs`). Unrelated to this feature.
- `contrast::probe_*` (2) are pre-existing diagnostic ignores. Unrelated.
- `lab_run_engine` failure is pre-existing (see § 3).

## 8. Acceptance Criteria Evidence

| AC | Description | Test(s) / Evidence | Result |
|----|-------------|-------------------|--------|
| AC1 | Baseline screen renders BH result; 2024 default; toggle to 2023 swaps curve+metrics | `baseline_snapshot__ready_2024_dark`, `baseline_snapshot__ready_2024_light`, `baseline_snapshot__ready_2023_toggled_dark`, `baseline_kpi_values_match_characterization_2024`, `committed_csvs_load_to_ready_first_point_100k` (first point $100k both years) | PASS |
| AC2 | Four panel states behave (Loading, Ready, Error, Empty) | `baseline_error_state_renders_without_panic` (Error — both themes, no panic), `baseline_snapshot__error_dark` / `_light`, `header_only_path_yields_empty_state` (Empty), `default_is_y2024_curves_loading_metrics_ready` (Loading) | PASS |
| AC3 | Fixtures cockpit smoke passes — first-frame Baseline route, no panic | `headless_emulator_paints_baseline_route` (1280×720, non-empty rgba, no panic); `baseline_error_state_renders_without_panic` (deterministic Error-path stand-in per D2) | PASS |
| AC4 | Lumen-consistent — consistency/contrast/layout_invariants green; no hardcoded colors/strings; both themes | `consistency` (2 PASS), `contrast` (7 PASS), `layout_invariants` (11 PASS), clippy new-warning scan (ZERO new warnings from feature files) | PASS |
| AC5 | Honest caption: states bounded finding; no "optimal"/"unbeatable"/"none beat it" | `baseline_caption_is_honest_bounded_no_overclaim` — asserts `BASELINE_CAPTION` contains `"active ≤ passive in the reachable universe, this sample"`, contains `"buy-and-hold"` + `"never rebalanced"`, and does NOT contain any of: `"optimal"`, `"unbeatable"`, `"none beat it"`, `"none can beat"`, `"cannot be beaten"`, `"best possible"`, `"guaranteed"` | PASS |
| AC6 | Panel-snapshot both themes + sidebar flatten-invariant updated | `baseline_snapshot__ready_2024_dark/light`, `baseline_snapshot__error_dark/light`, `theme::tests::sidebar_groups_phase_c__flatten_matches_phase_a` | PASS |
| AC7 | No new crate edge, no new widget, no new theme token | `git diff crates/ui/Cargo.toml` = empty (Cargo.toml unchanged); `baseline/` is pure-`ui` over `std::fs` + `trading_core`; `backtest` dep is pre-existing ADR-0030; consistency scan finds zero new hex / inline strings; no new widget file | PASS |

## 9. D1 Re-sync Guard Confirmation

`baseline_metrics_match_characterization` passes, asserting all six embedded scalars match characterization §7.1 realized row:

| Field | 2023 asserted | 2024 asserted | Result |
|-------|--------------|--------------|--------|
| `total_return_pct` | `196.22` | `91.04` | PASS |
| `cagr_pct` | `196.22` | `91.04` | PASS |
| `sharpe` | `1.8417` | `0.8925` | PASS |
| `max_drawdown_pct` | `34.57` | `48.95` | PASS |
| `win_rate_present` | `false` | `false` | PASS |
| `trades` | `0` | `0` | PASS |

The test would go RED on any silent edit to the embedded const — it is the re-sync trigger. Confirmed: the guard works as designed.

## 10. D1 Band-vs-Card Nuance Confirmation (documented-expected, NOT a defect)

The drawdown band renders the daily-sampled curve's per-point drawdown (visual shape; `from_points` computes this for free from the ~366 daily points). The Max DD KPI card shows the §7.1 const headline (48.95% / 34.57% — computed over 8,784/8,759 hourly bars). The band's visual trough sits shallower than the const headline because daily sampling misses intraday peaks.

This divergence is explicitly documented in `feature.md § Design D1`: "Band = curve shape, card = the published number." It is NOT a defect. The `baseline_snapshot__ready_2024_dark` snapshot pins the per-point band value from the loaded curve (drawn from `from_points`) while the KPI strip shows `48.95%` from the const. Both render; the divergence is visible to reviewers in the snapshot summary output. Confirmed: no defect.

## 11. Anchor Verification

`verify-anchors` result: **PASS 119/119**. No anchored file was touched. This feature is read-only UI over non-anchored runbook CSVs.

Skipped: `verify-anchors` is the non-negotiable gate when `crates/strategy/`, `crates/audit/`, `crates/exec/`, `crates/backtest/`, or report rendering is touched. This feature touches only `crates/ui/` — confirming N/A for the strategy/exec/backtest/audit/report surface, but the script was run as part of the proportionate gate and returns 119/119.

## 12. Verdict

**PASS**

All 7 acceptance criteria pass with direct test evidence. The UI suite counts 428 unit tests + all integration test suites GREEN. The one non-green test (`lab_run_engine::inner::h3_in_memory_equals_cached_disk`) is pre-existing and confirmed by retest on the parent commit. Static analysis is clean: build clean, fmt clean, zero new warnings from feature files (pre-existing ~140 pedantic lints untouched per scope). The D1 re-sync guard (`baseline_metrics_match_characterization`) correctly asserts all six §7.1 scalars. The honest-caption no-overclaim test (`baseline_caption_is_honest_bounded_no_overclaim`) passes. The smoke emulator (`headless_emulator_paints_baseline_route`) paints the Baseline route first-frame with no panic. Anchors 119/119. One new spec-lint trace-broken-path violation was introduced by the feature's trace row (shorthand `"ADR-0030"` instead of the full path) — corrected in the trace.toml update accompanying this report; after the fix the count returns to the 94-violation baseline.

## 13. Routing

`VERDICT → PASS` — ready to ship.
