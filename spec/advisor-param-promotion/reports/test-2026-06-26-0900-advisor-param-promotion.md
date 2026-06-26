---
title: Test Report
feature: advisor-param-promotion
run_id: 2026-06-26-0900-UTC
commit: 2080b217a985bd63298b0d7b627c0e0850ca4b41
agent: tester
verdict: PASS
---

# Test Report — advisor-param-promotion — 2026-06-26 09:00 UTC

## 1. Scope

- **Feature / change under test:** ADR-0070 — "Use this config" button promotes a tuned sweep result into the forward €200 paper-trade. Includes `param_override` plumbing through `AgentConfig`, `AgentPlan`, and `build_registry_for`; promote-button state wiring in UI; forward-plan render showing provenance strip; F5b fidelity (non-SMA crowned picks use their own ComposedStrategy).
- **Spec refs:** `spec/advisor-param-promotion/feature.md`
- **Commit SHA:** `2080b217a985bd63298b0d7b627c0e0850ca4b41`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `Darwin arm64`

## 2. Static Analysis

| Check               | Result | Notes                                                    |
|---------------------|--------|----------------------------------------------------------|
| `cargo fmt --check` | PASS   | Exit 0 — workspace clean                                 |
| `cargo clippy`      | PASS   | `cargo clippy --workspace --all-targets --features ui/live -- -D warnings` exit 0; compiler output: "Finished dev profile in 1.74s" — zero warnings emitted |
| `cargo audit`       | n/a    | Not run this cycle (no dependency changes in scope)      |
| `cargo deny`        | n/a    | Not run this cycle                                       |

spec-lint: PASS (0 violations) — `python3 scripts/spec_lint.py` exit 0.

## 3. Unit & Integration Tests

| Suite | Test binary / target | Passed | Failed | Ignored | Duration |
|-------|---------------------|-------:|-------:|--------:|---------:|
| `agent` — `forward_promotion_divergence` | `tests/forward_promotion_divergence.rs` | 7 | 0 | 0 | 0.00s |
| `agent` — `forward_run_engine_fidelity`  | `tests/forward_run_engine_fidelity.rs`  | 8 | 0 | 0 | 0.00s |
| `ui` — `promote_swept_config` (fixtures) | `tests/promote_swept_config.rs`         | 3 | 0 | 0 | 0.00s |
| `ui` — `param_sweep_render` (render)     | `tests/param_sweep_render.rs`           | 9 | 0 | 0 | 42.76s |
| `ui` — `forward_plan_populated_render` (render) | `tests/forward_plan_populated_render.rs` | 8 | 0 | 0 | 42.64s |
| **Total** | | **35** | **0** | **0** | |

### Test detail — forward_promotion_divergence (7/7)

```
t6c_plan_reflects_tuned_sma_override                ... ok
t6c_plan_none_path_emits_default_lens               ... ok
t6a_sma_param_override_produces_divergent_signals   ... ok
t6b_bbands_agent_toml_byte_equals_sweep_generator   ... ok
t6b_rsi_agent_toml_byte_equals_sweep_generator      ... ok
t6b_macd_agent_toml_byte_equals_sweep_generator     ... ok
t6a_macd_param_override_produces_divergent_signals  ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; finished in 0.00s
```

### Test detail — forward_run_engine_fidelity (8/8)

```
f5b_buyhold_identity_is_always_long_not_sma_crossover        ... ok
f5b_no_forward_returns_default_sma_registry                  ... ok
f5b_unknown_strategy_id_returns_err_not_sma_fallback         ... ok
f5b_buyhold_registry_differs_from_sma_registry_on_same_bars  ... ok
f5b_rsi_identity_is_btc_rsi_reversion_not_sma_crossover      ... ok
f5b_macd_identity_is_btc_macd_trend_not_sma_crossover        ... ok
f5b_bbands_identity_is_btc_bbands_mean_revert_not_sma_crossover ... ok
f5b_macd_registry_differs_from_sma_registry_on_same_bars     ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; finished in 0.00s
```

### Test detail — promote_swept_config (3/3)

```
promote_swept_config_carries_the_swept_window_label    ... ok
promote_swept_config_preseeds_target_and_navigates     ... ok
promote_swept_config_maps_every_family_to_its_forward_id ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; finished in 0.00s
```

### Test detail — param_sweep_render (9/9, render)

```
sweep_sma_form_has_no_third_axis                  ... ok
sweep_macd_form_paints_third_axis                 ... ok
sweep_empty_paints_no_grid                        ... ok
sweep_macd_populated_paints_grid_and_fragile_badge ... ok
sweep_progress_determinate_paints                 ... ok
sweep_populated_paints_grid_and_fragile_badge     ... ok
sweep_populated_paints_strictly_more_than_empty   ... ok
sweep_fragile_promote_disabled_accent_discriminator ... ok
sweep_promotable_use_config_is_enabled_accent_button ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; finished in 42.76s
```

Render test ran to completion (42.76s) with no CoreText deadlock detected.

### Test detail — forward_plan_populated_render (8/8, render)

```
forward_plan_promoted_paints_provenance_and_tuned_rules ... ok
forward_plan_f8_ensemble_paints_vote_rule_and_tally     ... ok
forward_plan_populated_paints_stance_rules_and_sizing   ... ok
forward_plan_rsi_reversion_paints_faithful_rules        ... ok
forward_plan_f8_buy_and_hold_is_the_negative_control    ... ok
forward_plan_crowned_has_no_provenance_strip             ... ok
forward_plan_buy_and_hold_is_the_negative_control       ... ok
forward_plan_empty_paints_no_plan                       ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; finished in 42.64s
```

Render test ran to completion (42.64s) with no CoreText deadlock detected.

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest/fuzz suites for this feature.

## 5. Backtest Results

_n/a_ — this feature is a UI/agent plumbing change (parameter promotion routing). It does not introduce new strategy math. The `forward_promotion_divergence` tests assert that a `param_override` causes different signals vs the un-overridden path, which is the functional equivalent of a divergence gate for this scope.

## 6. Benchmarks

_n/a_ — no hot-path changes; promotion is a one-shot config rewrite at plan-build time.

## 7. Environment / Infrastructure Issues

Render tests (`param_sweep_render`, `forward_plan_populated_render`) were run sequentially, one binary per cargo invocation, with `pkill -9` between runs to prevent CoreText font-mutex deadlock (per `spec/dev-notes/iced-ui-render-verification.md`). Both completed cleanly within ~43s each. No env-deadlock occurred.

## 8. Verdict

**PASS**

All 35 tests across 5 suites pass. Static analysis (clippy, fmt) is clean. Anchor gate holds at 119/119. spec-lint reports 0 violations. Render tests produced genuine `test result: ok` output (not suppressed by deadlock). The divergence and fidelity suites confirm that `param_override` reaches the engine and produces different signals, and that F5b fidelity is preserved for the `None` path. No regressions detected.

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
