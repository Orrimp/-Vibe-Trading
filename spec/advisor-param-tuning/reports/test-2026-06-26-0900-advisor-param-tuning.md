---
title: Test Report
feature: advisor-param-tuning
run_id: 2026-06-26-0900-UTC
commit: 2080b217a985bd63298b0d7b627c0e0850ca4b41
agent: tester
verdict: PASS
---

# Test Report — advisor-param-tuning — 2026-06-26 09:00 UTC

## 1. Scope

- **Feature / change under test:** ADR-0069 — gate-tied param sweep editor. Adds a `TuneScreen` to the cockpit UI with SMA/MACD/RSI/Bollinger grid editors, a sweep engine (`bakeoff::sweep`) that runs a grid of backtest cells in parallel, a robustness distribution gate, and composed-family TOML identity guards (so non-SMA crowned picks can be later promoted). Includes the promote-ready state mirror.
- **Spec refs:** `spec/advisor-param-tuning/feature.md`
- **Commit SHA:** `2080b217a985bd63298b0d7b627c0e0850ca4b41`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `Darwin arm64`

## 2. Static Analysis

| Check               | Result | Notes                                                    |
|---------------------|--------|----------------------------------------------------------|
| `cargo fmt --check` | PASS   | Exit 0 — workspace clean                                 |
| `cargo clippy`      | PASS   | `cargo clippy --workspace --all-targets --features ui/live -- -D warnings` exit 0; zero warnings emitted |
| `cargo audit`       | n/a    | Not run this cycle (no dependency changes in scope)      |
| `cargo deny`        | n/a    | Not run this cycle                                       |

spec-lint: PASS (0 violations) — `python3 scripts/spec_lint.py` exit 0.

## 3. Unit & Integration Tests

| Suite | Test binary / target | Passed | Failed | Ignored | Duration |
|-------|---------------------|-------:|-------:|--------:|---------:|
| `backtest` — `compute_robustness_distribution_matches_flag` | `tests/compute_robustness_distribution_matches_flag.rs` | 8 | 0 | 0 | 0.06s |
| `backtest` — `param_sweep_divergence_end_to_end` | `tests/param_sweep_divergence_end_to_end.rs` | 12 | 0 | 0 | 0.66s |
| `backtest` — `bakeoff::sweep::tests` (lib, incl. ignored) | `src/bakeoff/sweep.rs` | 29 | 0 | 0 | 0.00s |
| `ui` — lib tune | `src/lib.rs` (filtered to `tune` module) | 44 | 0 | 0 | 0.00s |
| `ui` — `param_sweep_render` (render) | `tests/param_sweep_render.rs` | 9 | 0 | 0 | 42.76s |
| **Total** | | **102** | **0** | **0** | |

### Test detail — compute_robustness_distribution_matches_flag (8/8)

```
compute_robustness_distribution_none_for_empty_curve          ... ok
compute_robustness_distribution_none_for_short_curve          ... ok
compute_robustness_distribution_summary_fields_sane           ... ok
compute_robustness_distribution_is_deterministic              ... ok
compute_robustness_distribution_verdict_matches_flag_growing  ... ok
compute_robustness_distribution_verdict_matches_flag_declining ... ok
compute_robustness_distribution_verdict_matches_flag_flat     ... ok
compute_robustness_distribution_verdict_matches_flag_battery  ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; finished in 0.06s
```

### Test detail — param_sweep_divergence_end_to_end (12/12)

```
t3_sweep_cancelled_returns_cancelled_error                         ... ok
t3_benchmark_is_populated                                          ... ok
t3_sweep_reports_invalid_cells                                     ... ok
t4_concrete_pin_fast10_slow20_differs_from_baseline                ... ok
t7_rsi_sweep_cells_diverge_from_baseline                           ... ok
t7_macd_sweep_cells_diverge_from_baseline                          ... ok
t7_bbands_sweep_cells_diverge_from_baseline                        ... ok
t4_swept_cells_diverge_from_baseline                               ... ok
t4_swept_cells_are_not_all_identical                               ... ok
t4_identical_params_produce_identical_equity_the_positive_control  ... ok
t3_sweep_returns_correct_cell_count_on_synthetic                   ... ok
t3_sweep_grid_truncates_at_cap                                     ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; finished in 0.66s
```

### Test detail — bakeoff::sweep::tests lib (29/29, --include-ignored)

All 29 unit tests in `src/bakeoff/sweep.rs` passed, including the 3 composed-TOML identity guards:

```
build_swept_config_sma_rejects_invalid_params        ... ok
build_swept_config_bbands_rejects_zero_k             ... ok
build_swept_config_macd_rejects_fast_ge_slow         ... ok
build_swept_config_rsi_rejects_oversold_above_49     ... ok
bbands_toml_k_decimal_2_normalizes_to_2              ... ok
macd_grid_drops_fast_ge_slow                         ... ok
build_swept_config_sma_threads_params                ... ok
macd_grid_default_enumerate_valid_all_pairs_have_fast_lt_slow ... ok
sma_grid_respects_400_upper_bound                    ... ok
sweep_axis_values_basic                              ... ok
bollinger_grid_default_enumerate_valid               ... ok
sma_grid_drops_invalid_fast_ge_slow                  ... ok
rsi_grid_default_enumerate_valid                     ... ok
sweep_axis_values_single_point                       ... ok
sweep_axis_values_step_one                           ... ok
sweep_axis_values_zero_step_treated_as_one           ... ok
sweep_drops_invalid_sma_cells                        ... ok
bbands_toml_generates_parseable_string               ... ok
rsi_toml_generates_parseable_string                  ... ok
sweep_grid_truncates_at_cap                          ... ok
macd_toml_generates_parseable_string                 ... ok
build_swept_config_bbands_sets_composed_toml_override ... ok
build_swept_config_rsi_sets_composed_toml_override   ... ok
swept_params_is_sma_shipped_default                  ... ok
build_swept_config_macd_sets_composed_toml_override  ... ok
swept_params_sma_label                               ... ok
bbands_toml_shipped_params_round_trip                ... ok
rsi_toml_shipped_params_round_trip                   ... ok
macd_toml_shipped_params_round_trip                  ... ok

test result: ok. 29 passed; 0 failed; 0 ignored; 141 filtered out; finished in 0.00s
```

### Test detail — ui lib tune (44/44)

```
tune::screen_state::tests::all_families_are_runnable                        ... ok
screens::tune::tests::every_family_has_a_label                              ... ok
tune::screen_state::tests::axis_kind_maps_to_owning_family                  ... ok
tune::screen_state::tests::begin_run_sets_loading_and_running               ... ok
screens::tune::tests::fmt_prob_clamps_and_rounds                            ... ok
tune::screen_state::tests::default_family_is_sma_and_runnable               ... ok
tune::runner::tests::config_from_state_maps_relative_lookback_to_custom_window ... ok
tune::screen_state::tests::bollinger_toggle_k_via_state                     ... ok
tune::screen_state::tests::empty_grid_blocks_run                            ... ok
tune::screen_state::tests::default_grid_is_runnable_and_under_cap           ... ok
tune::runner::tests::config_from_state_carries_family_coin_and_ranges       ... ok
tune::screen_state::tests::blank_field_blocks_run                           ... ok
tune::runner::tests::config_from_state_macd_maps_form_to_real_grid          ... ok
tune::screen_state::tests::edit_round_trips_verbatim                        ... ok
tune::screen_state::tests::finish_run_err_lands_error                       ... ok
tune::runner::tests::config_from_state_rsi_maps_form_to_real_grid           ... ok
tune::screen_state::tests::bollinger_zero_k_blocks_run                      ... ok
tune::screen_state::tests::invalid_cells_dropped_when_fast_ge_slow          ... ok
tune::screen_state::tests::macd_edit_routes_by_family_via_state             ... ok
tune::screen_state::tests::over_cap_grid_truncates                          ... ok
tune::screen_state::tests::bollinger_selected_k_decimals_match_presets      ... ok
tune::screen_state::tests::macd_drops_fast_ge_slow_like_engine              ... ok
tune::screen_state::tests::macd_selecting_family_drives_estimate            ... ok
tune::screen_state::tests::preset_apply_seeds_axis                          ... ok
tune::screen_state::tests::rsi_drops_oversold_ge_50                         ... ok
tune::runner::tests::config_from_state_bollinger_maps_form_to_real_grid     ... ok
tune::runner::tests::config_from_state_bollinger_empty_k_falls_back_to_shipped ... ok
tune::screen_state::tests::macd_default_form_matches_engine_default_and_runs ... ok
tune::screen_state::tests::finish_run_ok_lands_ready                        ... ok
tune::screen_state::tests::finish_run_empty_cells_lands_empty               ... ok
tune::screen_state::tests::select_family_keeps_result                       ... ok
tune::screen_state::tests::bollinger_default_form_matches_engine_default    ... ok
tune::screen_state::tests::rsi_default_form_matches_engine_default          ... ok
tune::state::tests::from_report_distribution_fields_mapped                  ... ok
tune::state::tests::from_report_maps_correct_cell_count                     ... ok
tune::state::tests::from_report_populates_promote_params_for_every_cell     ... ok
tune::state::tests::from_report_truncation_flag_echoed                      ... ok
tune::state::tests::from_report_benchmark_kpis_echoed                       ... ok
tune::state::tests::from_report_promotable_false_iff_fragile                ... ok
tune::state::tests::from_report_echoes_request_metadata                     ... ok
tune::state::tests::from_report_verdict_labels_correct                      ... ok
tune::state::tests::from_report_baseline_is_shipped_config                  ... ok
tune::state::tests::promote_params_from_swept_maps_every_family             ... ok
screens::tune::tests::view_constructs_both_modes_empty_and_ready            ... ok

test result: ok. 44 passed; 0 failed; 0 ignored; 531 filtered out; finished in 0.00s
```

### Test detail — param_sweep_render (9/9, render)

```
sweep_sma_form_has_no_third_axis                   ... ok
sweep_macd_form_paints_third_axis                  ... ok
sweep_empty_paints_no_grid                         ... ok
sweep_macd_populated_paints_grid_and_fragile_badge ... ok
sweep_progress_determinate_paints                  ... ok
sweep_populated_paints_grid_and_fragile_badge      ... ok
sweep_populated_paints_strictly_more_than_empty    ... ok
sweep_fragile_promote_disabled_accent_discriminator ... ok
sweep_promotable_use_config_is_enabled_accent_button ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; finished in 42.76s
```

Render test ran to completion (42.76s) with no CoreText deadlock.

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest/fuzz suites for this feature.

## 5. Backtest Results

_n/a_ — the sweep engine is tested via the divergence end-to-end suite (T4/T7 scenarios), which asserts that different parameter grids produce different equity curves. No additional strategy-level backtest metrics are required for this plumbing/UI feature.

## 6. Benchmarks

_n/a_ — the sweep grid is an interactive operator tool with no latency-critical path. Sweep execution time is dominated by per-cell backtest runtime, which is governed by the existing backtest benchmarks.

## 7. Environment / Infrastructure Issues

The `param_sweep_render` test binary was run in isolation (one binary per cargo invocation, with `pkill -9` prior to the run). It completed in 42.76s with no CoreText font-mutex deadlock. Multiple cargo processes for `crates/backtest` competed for the file-lock artifact directory during parallel runs; they serialized correctly with no failures.

## 8. Verdict

**PASS**

All 102 tests across 5 suites pass. Static analysis (clippy, fmt) is clean. Anchor gate holds at 119/119. spec-lint reports 0 violations. Render tests produced genuine `test result: ok` output. The divergence suites (T4 SMA, T7 composed) confirm that non-default parameter grids produce divergent equity from the baseline. The robustness distribution gate is bit-identical across runs. Composed-TOML identity guards (T7 BBands/RSI/MACD) confirm the TOML round-trip needed for the promotion path. No regressions detected.

## 9. Routing

`VERDICT → PASS` — ready to ship.

---

## Shared Gates (cited in all three reports)

| Gate | Result |
|------|--------|
| `cargo fmt --check` (workspace) | PASS — exit 0 |
| `cargo clippy --workspace --all-targets --features ui/live -- -D warnings` | PASS — exit 0 |
| `bash scripts/verify_anchors.sh` | PASS — 119/119 |
| `python3 scripts/spec_lint.py` | PASS — 0 violations |
