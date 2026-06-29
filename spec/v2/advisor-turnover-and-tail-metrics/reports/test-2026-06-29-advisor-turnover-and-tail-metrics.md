---
title: Test Report
feature: advisor-turnover-and-tail-metrics
run_id: 2026-06-29-2039-UTC
commit: 00240ed
agent: tester
verdict: PASS
---

# Test Report — advisor-turnover-and-tail-metrics — 2026-06-29 20:39 UTC

## 1. Scope

- **Feature / change under test:** P1-1 Turnover KPI + P1-2 Coherent Tail / Median Reporting. Two additive report-honesty increments: `CandidateKpis.turnover: Decimal` (cost visibility column) + `DistributionSummary.{cvar_95, cvar_99, median_terminal_wealth, skew}` + `TailSummary` backend struct + `TailSummaryView` UI mirror + "Risk story" `frame::panel` under the scorecard block on the leaderboard screen. REPORT-ONLY: the FROZEN gate (`classify_verdict` / `rank_candidates` / ADR-0066 benchmark exemption) is byte-untouched throughout.
- **Spec refs:** `spec/v2/advisor-turnover-and-tail-metrics/feature.md`, `spec/v2/advisor-turnover-and-tail-metrics/tasks.md`, `spec/v2/v2-architecture.md §1 P1-1 + P1-2 + §6.0`
- **Commit SHA:** `00240ed` (UI agent), `66286e2` (developer/backend)
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `Darwin 25.5.0 arm64 (macOS)`

---

## 2. Static Analysis

| Check               | Result | Notes                                          |
|---------------------|--------|------------------------------------------------|
| `cargo fmt --check` | PASS   | `FMT_CHECK_EXIT: 0`                            |
| `cargo clippy -p backtest --tests` | PASS | `CLIPPY_BACKTEST_EXIT: 0` — "Checking backtest… Finished" |
| `cargo clippy -p ui --tests --features fixtures` | PASS | `CLIPPY_UI_EXIT: 0`  |
| `cargo audit`       | N/A    | Not run this cycle — no new dependencies added |
| `cargo deny`        | N/A    | Not run this cycle — no new dependencies added |

---

## 3. Unit & Integration Tests

| Crate     | Passed | Failed | Ignored | Duration |
|-----------|-------:|-------:|--------:|---------:|
| `backtest` (lib) | 193 | 0 | 8 | 0.64s |
| `ui` (lib) | 583 | 0 | 0 | 0.68s |
| `ui` integration: `leaderboard_risk_story_render` | 2 | 0 | 0 | 96.24s |
| `ui` integration: `leaderboard_scorecard_render` | 2 | 0 | 0 | 84.08s |
| `ui` integration: `bakeoff_progress_render` | 3 | 0 | 0 | 70.47s |
| **Total** | **783** | **0** | **8** | — |

### New tests confirmed passing (P1-1 + P1-2)

**backtest lib — turnover unit tests:**
```
test bakeoff::tests::turnover_idle_zero ... ok
test bakeoff::tests::turnover_one_roundtrip ... ok
test bakeoff::tests::turnover_multi_trade ... ok
test bakeoff::tests::turnover_does_not_change_ranking ... ok
```

**backtest lib — tail metric unit tests:**
```
test stats::tests::cvar_empty_returns_zero ... ok
test stats::tests::cvar_le_var_property ... ok
test stats::tests::cvar_uniform_n20_closed_form ... ok
test stats::tests::cvar_uniform_n100_closed_form ... ok
test stats::tests::cvar_99_equals_min_on_n100 ... ok
test stats::tests::skew_zero_on_symmetric ... ok
test stats::tests::skew_positive_on_right_skewed ... ok
test stats::tests::skew_negative_on_left_skewed ... ok
test stats::tests::skew_degenerate_small_n ... ok
test stats::tests::distribution_summary_p1_2_fields_populated ... ok
test stats::tests::p1_2_fields_additive_gate_unchanged ... ok
```

**ui lib — mirror + format helpers:**
```
test leaderboard::state::tests::tail_summary_view_mirrors_a_populated_tail ... ok
test screens::leaderboard::tests::format_turnover_ratio_renders_one_decimal_with_x_suffix ... ok
test screens::leaderboard::tests::format_signed_decimal_renders_signed_with_unicode_minus ... ok
test screens::leaderboard::tests::fmt_signed_pct_from_f64_renders_signed_one_decimal_with_unicode_minus ... ok
```

**ui integration — render-pixel guards:**
```
test risk_story_block_paints_and_exceeds_no_tail ... ok
test risk_story_block_present_in_benchmark_wins_modal_case ... ok
```

**bakeoff_progress_render — 3/3 (UI agent fixed y-band drift; was 1/3 pre-commit):**
```
test bakeoff_progress_bar_paints_beneath_input_panel ... ok
test not_running_paints_no_progress_strip ... ok
test running_strictly_exceeds_not_running_strip_fill ... ok
```

### Failing Tests

_none_

---

## 4. Property / Fuzz Tests

_n/a_ — no proptest or cargo-fuzz suites added in this feature. The backtest cvar/skew/median unit tests serve as hand-verified closed-form checks over known input vectors.

---

## 5. Backtest Results

_n/a_ — this change is REPORT-ONLY. It extends `DistributionSummary` (reducing over existing `PathMetrics` already computed per candidate) and adds `CandidateKpis.turnover` (derived from `RunReport.fills`). Neither touches the simulation engine, the strategy logic, nor any anchored CLI report path (`write_report=false` for the advisor bakeoff by construction). Anchors verified: 119/119 PASS (see §8). The FROZEN gate test `turnover_does_not_change_ranking` explicitly asserts rank output is byte-identical before/after the turnover field.

---

## 6. Benchmarks

_n/a_ — no hot-path changes. The new computations (`compute_cvar`, `compute_distribution_skew`, median sort) run once per bakeoff crown at report-assembly time, not on the critical simulation or strategy dispatch path.

---

## 7. Visual Render-Pixel Verification (CLAUDE.md non-negotiable)

### Gate 4a — `leaderboard_risk_story_render` (populated + negative control)

Both tests PASS (96.24s, 2/2 ok). The pixel-delta assertion `fg_with > fg_without + 1500` passes, confirming the six-fact "Risk story" panel renders substantially more foreground than the same leaderboard with `tail = None`.

**PNG observed — populated fixture** (`/tmp/leaderboard_risk_story_render.png`):

The rendered leaderboard shows the full screen at 1920×2400 px (tall viewport so the Risk story block clears the scorecard and lands in-frame). Visible elements top-to-bottom:

1. **Strategy bake-off** header + coin/lookback/timeframe/capital controls
2. **Ranking strategies** caption (BTCUSDT, 20 arms head-to-head)
3. **Recommendation** box — "No active strategy cleared the robustness bar on BTCUSDT — simply holding (buy-and-hold) is the least-bad choice on this window."
4. **Ranked table** with columns: #, Strategy, Return, Sharpe, Max drawdown, Trades, **Churn** (new P1-1 column, rightmost — values rendered as `N.N×` format, e.g. `5.0×`, `0.0×`)
5. **"How much to trust this"** scorecard block (P0-1, existing)
6. **"Risk story"** tail/median block (P1-2, new) — six facts clearly painted in panel chrome:
   - **102,300 USDT** — "Typical outcome (median)" (median_terminal_wealth, neutral FG_1)
   - **−18.0 %** — "Average loss in the worst 5 % of paths" (cvar_95, DOWN_500 red)
   - **−31.0 %** — "Average loss in the worst 1 % of paths" (cvar_99, DOWN_500 red) + shared CVaR coherence gloss beneath
   - **+0.42** — "Surprise shape (skew)" (mildly right-skewed, neutral FG_1)
   - **+1.95** — "Downside-only Sharpe (Sortino)" (signed decimal)
   - **+2.32** — "Return vs worst drawdown (Calmar)" (signed decimal)
   - Footer note: "Informational, not a gate — these never change the pick above."
7. **Disclaimer** footer (persistent)

**PNG observed — negative control** (`/tmp/leaderboard_no_risk_story_render.png`):

Same screen with `tail = None`. The Risk story panel is absent. The scorecard block and ranked table render identically to the populated case. The "How much to trust this" block is visibly the last content block above the disclaimer. Pixel count is strictly less than the populated case, confirming the delta > 1500 px assertion.

### Gate 4b — `leaderboard_scorecard_render` (regression check)

2/2 PASS (84.08s). The P0-1 scorecard block paints correctly alongside the new tail block — no regression in the scorecard rendering.

### Gate 4c — `bakeoff_progress_render` (y-band drift fix verification)

3/3 PASS (70.47s). All three bakeoff progress bar tests pass — the UI agent fixed the pre-existing 1/3 y-band drift regression in this commit.

### Benchmark-wins modal case (`risk_story_block_present_in_benchmark_wins_modal_case`)

The fixture carries the honest single-asset hold tail:
- `cvar_95: -0.24` (24% expected loss in worst 5% of paths)
- `cvar_99: -0.41` (41% expected loss in worst 1% of paths — deeper than cvar_95, as required)
- `median_terminal_wealth: 102_300.0` (€102,300 median outcome, positive as required)
- `skew: -0.31` (mildly negative — crash-prone, consistent with a single-asset crypto hold)

The pixel delta assertion `fg_with > fg_without + 1500` passes for this case too, proving the Risk story block renders even when buy-and-hold is crowned.

---

## 8. Anchor Verification Gate (NON-NEGOTIABLE)

```
bash scripts/verify_anchors.sh
```

Result: **`ANCHORS PASS  (119 / 119)`**

All 119 anchors pass. No anchored report body was modified. The advisor bakeoff path uses `write_report=false` by construction — the new `TailSummary` and `turnover` fields are anchor-safe.

---

## 9. Spec-Lint Gate (NON-NEGOTIABLE)

```
python3 scripts/spec_lint.py
```

Result: **`spec-lint: PASS (0 violations)`**

No new violations introduced. No pre-existing baseline debt.

---

## 10. Cockpit Smoke Gate

```
cargo build -p ui --bin cockpit --features fixtures
# Binary: target/debug/cockpit (15 MB, arm64, built 2026-06-29 21:20)
# 7s window smoke run:
PANIC_COUNT: 0
```

Log: `spec/v2/advisor-turnover-and-tail-metrics/reports/cockpit-smoke-2026-06-29T20-39Z.log` (0 lines — normal; macOS GUI binary produces no stderr on clean boot)

Result: **cockpit-smoke: PASS (0 panics, 7s window)**

Reference: P0-1 log is also 0 lines — this is the expected clean-boot state.

---

## 11. FROZEN Gate Identity Check

The `turnover_does_not_change_ranking` test (T8, `crates/backtest/src/bakeoff/mod.rs:1341`) is the explicit proof that adding `pub turnover: Decimal` to `CandidateKpis` does not alter the output of `rank_candidates`. It passes without modification to the frozen gate (`classify_verdict` / `rank_candidates` / ADR-0066 benchmark exemption constants).

```
test bakeoff::tests::turnover_does_not_change_ranking ... ok
```

The analogous P0-1 gate `scorecard_does_not_change_ranking` also continues to pass:
```
test bakeoff::scorecard::tests::scorecard_does_not_change_ranking ... ok
```

---

## 12. Environment / Infrastructure Issues

- The `cargo clippy -p backtest` invocation waited ~7m38s for the artifact lock (the render tests had it). Sequential-cargo discipline maintained; no parallel cargo processes were spawned.
- The cockpit binary was rebuilt from the previous build at 21:20 UTC — already up-to-date with commit `00240ed`.

_No flaky tests, no infra outages, no data gaps._

---

## 13. Trace.toml Anchors Column

Feature crates: `["crates/backtest", "crates/ui"]`. These crates are in the strategy/backtest/exec family covered by the anchor-verification requirement. The feature is REPORT-ONLY with `write_report=false` — no new anchored scenarios were added. The existing 119/119 anchors all PASS. The `anchors` column remains `[]` (expected none, confirmed none).

The `tests` column in `REQ-V2-P1-TURNOVER-TAIL-001` is updated to cite this report path.

---

## 8. Verdict

**`PASS`**

All 10 mandatory gates pass cleanly. The feature is additive and REPORT-ONLY: turnover (P1-1) and coherent tail metrics (P2-2) are visible on the leaderboard screen but never feed the crown/rank/verdict. The FROZEN gate identity is proven by `turnover_does_not_change_ranking` (explicit test, PASS). Anchors 119/119. Spec-lint 0 violations. Cockpit smoke 0 panics. Pixel-layer verification confirms the six-fact "Risk story" panel paints with measurable foreground delta (> 1500 px) over the negative control. The `bakeoff_progress_render` y-band drift (pre-existing 1/3 regression) is fixed by the UI agent in this same commit — all three tests now pass.

---

## 9. Routing

`VERDICT → PASS` — ready to ship. No regressions; no failing tests; no anchor breaks; spec-lint clean. Feature can advance to presenter.
