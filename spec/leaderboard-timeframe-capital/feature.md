---
slug: leaderboard-timeframe-capital
status: dev-done
owner: developer
updated: 2026-06-25
version: 0.1.0
---

# Leaderboard timeframe + start-capital knobs

Leaderboard epic item #4. Two new "tune" knobs on the bake-off guided-input
panel that let the operator rank strategies across a different bar granularity
and see how absolute equity values scale with a different starting capital.

See CHANGELOG.md for the implementation summary.

## Implementation

All engine, state, UI, and test changes landed in a single development session
(2026-06-24/25). Key changes:

- `crates/backtest/src/bakeoff/mod.rs`: `BakeoffRequest` gains `timeframe:
  Horizon` + `initial_capital: Decimal`; `run_bakeoff` resamples bars once
  before the candidate loop.
- `crates/backtest/src/engine.rs`: `ScenarioConfig` gains `initial_capital:
  Option<Decimal>` (None = 100_000 legacy default, anchor-safe).
- `crates/ui/src/leaderboard/state.rs`: `BakeoffTimeframe` enum + `start_capital_input`
  field + `parse_start_capital` + `start_capital()` method + `DEFAULT_START_CAPITAL_INPUT`.
- `crates/ui/src/state.rs`: `BakeoffSelectTimeframe` + `BakeoffSetStartCapital`
  message variants + update handlers.
- `crates/ui/src/widgets/bakeoff_input.rs`: `view` gains `timeframe` + `start_capital_input`
  params; timeframe chip row + capital text field added.
- `crates/ui/src/screens/leaderboard.rs`: `display_label` + `is_short_capable_id` fixed
  to handle `v0.`-prefixed short arm ids.
- Day-1 divergence tests in `crates/backtest/tests/bakeoff_e2e.rs`:
  `t_capital_div_2x_capital_doubles_absolute_equity`,
  `t_timeframe_div_resampler_reduces_bar_count_4to1`,
  `t_timeframe_bars_resampled_bars_produce_different_equity`.
- Config-from-state wiring tests in `crates/ui/tests/bakeoff_config_from_state.rs`.
