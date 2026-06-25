---
slug: leaderboard-timeframe-capital
status: dev-done
owner: developer
updated: 2026-06-25
---

# Tasks — leaderboard-timeframe-capital

## Engine layer

- [x] T1: Add `timeframe: Horizon` to `BakeoffRequest`
  - file: `crates/backtest/src/bakeoff/mod.rs` (struct BakeoffRequest)
  - test: `cargo test -p backtest --test bakeoff_e2e -- leaderboard_tuning_divergence`
  - output: `test leaderboard_tuning_divergence::t_capital_div_2x_capital_doubles_absolute_equity ... ok`

- [x] T2: Add `initial_capital: Decimal` to `BakeoffRequest`
  - file: `crates/backtest/src/bakeoff/mod.rs` (struct BakeoffRequest)
  - test: `cargo test -p backtest --test bakeoff_e2e -- leaderboard_tuning_divergence`
  - output: same run as T1

- [x] T3: Resample 1h bars in `run_bakeoff` before candidate loop
  - file: `crates/backtest/src/bakeoff/mod.rs` (run_bakeoff, lines ~647-655)
  - test: `cargo test -p backtest --test bakeoff_e2e -- t_timeframe_div_resampler_reduces_bar_count_4to1`
  - output: `test leaderboard_tuning_divergence::t_timeframe_div_resampler_reduces_bar_count_4to1 ... ok`

- [x] T4: Add `initial_capital: Option<Decimal>` to `ScenarioConfig`; wire in `run_scenario`
  - file: `crates/backtest/src/engine.rs` (struct ScenarioConfig + run_scenario)
  - test: `cargo test -p backtest --test bakeoff_e2e -- t_capital_div`
  - output: `test leaderboard_tuning_divergence::t_capital_div_2x_capital_doubles_absolute_equity ... ok`

## UI state layer

- [x] T5: Add `BakeoffTimeframe` enum + `to_horizon()` + `chip_label()` to state.rs
  - file: `crates/ui/src/leaderboard/state.rs` (enum BakeoffTimeframe)
  - test: `cargo test -p ui --lib -- leaderboard::state::tests::bakeoff_timeframe`
  - output: `test leaderboard::state::tests::bakeoff_timeframe_all_has_three_entries ... ok`

- [x] T6: Add `start_capital_input: String` field + `parse_start_capital` + `start_capital()` to LeaderboardScreenState
  - file: `crates/ui/src/leaderboard/state.rs`
  - test: `cargo test -p ui --lib -- leaderboard::state::tests::start_capital`
  - output: `test leaderboard::state::tests::start_capital_falls_back_to_100k_on_bad_input ... ok`

- [x] T7: Add `BakeoffSelectTimeframe` + `BakeoffSetStartCapital` messages + handlers
  - file: `crates/ui/src/state.rs`
  - test: `cargo test -p ui --lib -- bakeoff`
  - output: `test result: ok. 8 passed; 0 failed; 0 ignored`

## UI widget layer

- [x] T8: Update `bakeoff_input::view` signature + add timeframe chip row + capital field
  - file: `crates/ui/src/widgets/bakeoff_input.rs`
  - test: `cargo test -p ui --lib -- widgets::bakeoff_input`
  - output: `test result: ok. 5 passed; 0 failed; 0 ignored`

- [x] T9: Update call sites in `screens/leaderboard.rs` + `gallery/routes.rs`
  - files: both updated to pass `timeframe` + `start_capital_input`
  - test: `cargo build -p ui` → `Finished`

## Strings

- [x] T10: Add `LEADERBOARD_TIMEFRAME_LABEL`, `LEADERBOARD_CAPITAL_LABEL`, `LEADERBOARD_CAPITAL_PLACEHOLDER`, `LEADERBOARD_CAPITAL_HINT` to strings.rs
  - file: `crates/ui/src/strings.rs`
  - test: `cargo clippy --workspace --all-targets -- -D warnings` → `Finished`

## Bug fixes bundled

- [x] T11: Fix `display_label` + `is_short_capable_id` for `v0.`-prefixed short arm ids
  - file: `crates/ui/src/screens/leaderboard.rs`
  - test: `cargo test -p ui --lib -- screens::leaderboard::tests::v0_prefixed_short_arm_ids`
  - output: `test screens::leaderboard::tests::v0_prefixed_short_arm_ids_map_to_friendly_labels ... ok`

## Day-1 divergence tests (CLAUDE.md non-negotiable)

- [x] T12: Capital divergence e2e (`t_capital_div_2x_capital_doubles_absolute_equity`)
  - file: `crates/backtest/tests/bakeoff_e2e.rs`
  - test: `cargo test -p backtest --test bakeoff_e2e -- t_capital_div`
  - output: `test leaderboard_tuning_divergence::t_capital_div_2x_capital_doubles_absolute_equity ... ok`

- [x] T13: Timeframe resampler unit test (`t_timeframe_div_resampler_reduces_bar_count_4to1`)
  - file: `crates/backtest/tests/bakeoff_e2e.rs`
  - test: `cargo test -p backtest --test bakeoff_e2e -- t_timeframe_div_resampler`
  - output: `test leaderboard_tuning_divergence::t_timeframe_div_resampler_reduces_bar_count_4to1 ... ok`

- [x] T14: Timeframe bars-override wiring test (`t_timeframe_bars_resampled_bars_produce_different_equity`)
  - file: `crates/backtest/tests/bakeoff_e2e.rs`
  - test: `cargo test -p backtest --test bakeoff_e2e -- t_timeframe_bars`
  - output: `test leaderboard_tuning_divergence::t_timeframe_bars_resampled_bars_produce_different_equity ... ok`

- [x] T15: Config-from-state wiring tests (6 tests in `bakeoff_config_from_state.rs`)
  - file: `crates/ui/tests/bakeoff_config_from_state.rs`
  - test: `cargo test -p ui --test bakeoff_config_from_state`
  - output: `test result: ok. 6 passed; 0 failed; 0 ignored`

## Gates

- [x] T16: `cargo clippy --workspace --all-targets -- -D warnings` → PASS
  - output: `Finished dev profile`

- [x] T17: `bash scripts/verify_anchors.sh` → 119/119
  - output: `ANCHORS PASS  (119 / 119)`
