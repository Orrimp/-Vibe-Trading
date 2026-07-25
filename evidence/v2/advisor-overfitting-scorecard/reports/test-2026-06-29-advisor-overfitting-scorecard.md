---
title: Test Report
feature: advisor-overfitting-scorecard
run_id: 2026-06-29-1320-UTC
commit: d3a9a4a
agent: tester
verdict: PASS
---

# Test Report — advisor-overfitting-scorecard — 2026-06-29 13:20 UTC

## 1. Scope

- **Feature / change under test:** P0-1 Overfitting Scorecard — report-only
  credibility layer (N_eff / DSR / MinBTL) surfaced next to every bake-off
  recommendation. Additive to the FROZEN gate — never a veto.
  Backend: `crates/backtest/src/bakeoff/scorecard.rs` (880 lines, 16 unit
  tests). Carrier: `Recommendation.scorecard` field in `bakeoff/mod.rs`.
  UI: `ScorecardView` mirror in `leaderboard/state.rs` +
  `scorecard_block` in `screens/leaderboard.rs` + 13 `LEADERBOARD_SCORECARD_*`
  string constants. Render-verified via `leaderboard_scorecard_render.rs`.
  Pre-existing-failure sweep: `d3a9a4a` fixes advisor_field arm-count
  (17→19 for DVOL+macro) and clippy-1.94 `allow` annotations.
- **Spec refs:** `spec/v2/advisor-overfitting-scorecard/feature.md`,
  `spec/v2/advisor-overfitting-scorecard/tasks.md`,
  `spec/v2/v2-architecture.md` §1 P0-1 + §6.0
- **Commit SHA:** `d3a9a4a` (HEAD; parent chain: `ac7c779` UI, `9c3c002` backend)
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `Darwin arm64`

---

## 2. Static Analysis

| Check                        | Result | Notes                                    |
|------------------------------|--------|------------------------------------------|
| `cargo fmt --check`          | PASS   | No diff; workspace clean.                |
| `cargo clippy -p backtest --tests -- -D warnings` | PASS | 0 warnings. `Finished dev profile` only. |
| `cargo clippy -p ui --tests --features fixtures -- -D warnings` | PASS | 0 warnings. `Finished dev profile` only. |
| `cargo audit`                | N/A    | Not run this cycle (no new deps added).  |
| `cargo deny`                 | N/A    | Not run this cycle (no new deps added).  |

### Raw gate outputs

```
# cargo fmt --check
(no output — exit 0)

# cargo clippy -p backtest --tests -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.20s

# cargo clippy -p ui --tests --features fixtures -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.68s
```

---

## 3. Unit & Integration Tests

| Crate / suite                         | Passed | Failed | Ignored | Duration |
|---------------------------------------|-------:|-------:|--------:|---------:|
| `backtest` (lib + bins)               | 178    | 0      | 8       | 0.65s    |
| `ui` lib                              | 579    | 0      | 0       | 0.68s    |
| `ui` test `leaderboard_scorecard_render` (macOS render) | 2 | 0 | 0 | 94.20s |
| **Total**                             | **759** | **0** | **8** |          |

### Failing Tests

_none_

### Ignored tests (expected, pre-existing)

Backtest crate (8 tests ignored, all pre-existing):

- `bakeoff::sweep::tests::bbands_toml_shipped_params_round_trip` — requires `config/strategies/btc_bbands_mean_revert.toml` at CWD
- `bakeoff::sweep::tests::macd_toml_shipped_params_round_trip` — same
- `bakeoff::sweep::tests::rsi_toml_shipped_params_round_trip` — same
- `engine::tests::run_scenario_momentum_dispatch_returns_ok` — requires `config/strategies/*.toml`
- `scenarios::sma_composed_run::tests::run_bbands_mean_revert_deterministic` — same
- `scenarios::sma_composed_run::tests::run_macd_trend_deterministic` — same
- `scenarios::sma_composed_run::tests::run_rsi_reversion_deterministic` — same
- `paths::tests::resolves_via_workspace_marker_walk_up` — uses `std::env::set_current_dir` (process-global; races under parallel `cargo test --workspace`; annotated with tracking note)

All 8 are pre-existing ignores unrelated to this feature.

### Scorecard-specific test results (16 tests, all PASS)

```
test bakeoff::scorecard::tests::compute_scorecard_degenerate_empty ... ok
test bakeoff::scorecard::tests::compute_scorecard_pbo_always_none ... ok
test bakeoff::scorecard::tests::dsr_normal_returns_clears_at_n88 ... ok
test bakeoff::scorecard::tests::dsr_research_worked_example_fails_at_n100 ... ok
test bakeoff::scorecard::tests::dsr_research_worked_example_passes_at_n46 ... ok
test bakeoff::scorecard::tests::min_btl_formula_matches_2lnn_over_sr2 ... ok
test bakeoff::scorecard::tests::compute_scorecard_single_candidate ... ok
test bakeoff::scorecard::tests::min_btl_n_eq_24_sr_eq_1 ... ok
test bakeoff::scorecard::tests::min_btl_zero_for_n_le_1 ... ok
test bakeoff::scorecard::tests::n_eff_empty_returns_zero ... ok
test bakeoff::scorecard::tests::n_eff_perfectly_correlated_returns_one ... ok
test bakeoff::scorecard::tests::n_eff_single_candidate ... ok
test bakeoff::scorecard::tests::n_eff_uncorrelated_field_approaches_m ... ok
test bakeoff::scorecard::tests::normal_cdf_symmetry_and_boundary ... ok
test bakeoff::scorecard::tests::normal_inv_cdf_roundtrip ... ok
test bakeoff::scorecard::tests::scorecard_does_not_change_ranking ... ok   ← FROZEN-gate identity
```

The FROZEN-gate identity test `scorecard_does_not_change_ranking` confirms
`rank_candidates` produces byte-identical output (`crowned` / `outcome` /
`order`) before and after the scorecard is computed. The scorecard is additive.

### ScorecardView mirror tests (from ui lib, 2 tests, all PASS)

```
state::tests::scorecard_view_mirrors_a_populated_scorecard ... ok
state::tests::scorecard_view_is_none_for_degenerate_empty_field ... ok
```

---

## 4. Property / Fuzz Tests

_n/a_ — no proptest or cargo-fuzz suites present for this feature.

---

## 5. Backtest Results

_n/a_ — This change is **report-only**. The scorecard is carried on
`Recommendation` and is not on the anchored CLI report path
(`write_report = false` in the advisor bakeoff path). The `rank_candidates`
gate-identity test directly proves no equity change occurs. No bakeoff
anchored output was touched.

---

## 6. Benchmarks

_n/a_ — No hot-path changes. The scorecard is computed once per bakeoff run
(closed-form pure functions on a vector of ≤24 f64 values). No criterion
suite exists for this module and none is required.

---

## 7. Render-Layer Verification (CLAUDE.md non-negotiable)

Gate: `cargo test -p ui --test leaderboard_scorecard_render --features fixtures`

```
running 2 tests
test scorecard_block_paints_and_exceeds_no_scorecard has been running for over 60 seconds
test scorecard_block_present_in_benchmark_wins_modal_case has been running for over 60 seconds
test scorecard_block_present_in_benchmark_wins_modal_case ... ok
test scorecard_block_paints_and_exceeds_no_scorecard ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 94.20s
```

**PNG read at `/tmp/leaderboard_scorecard_render.png` — tester confirms:**

The rendered PNG shows a dark-theme leaderboard at 1920×1080. From top to
bottom: the "Strategy bake-off" header + subtext; "Plan your bake-off" config
panel with coin selector (BTCUSDT highlighted), lookback/budget controls; a
"Recommendation" panel reading "No active strategy cleared the robustness bar
on BTCUSDT — simply holding (buy-and-hold) is the least-bad choice on this
window"; a 2-row ranked table (v0.buyhold starred as best, v0.sma below it);
then the new **"How much to trust this"** block painted clearly below the
table, containing:

- Introductory line: "An honesty check on the search behind the pick — it
  never changes the result."
- **Strategies tried:** "2 — about 2 truly independent"
- **Deflated confidence:** "38%" with gloss "Chance the edge is real after
  accounting for how many we tried."
- **Minimum history needed:** "about 1.1 years of data" with gloss "Trust the
  result only with at least this much history behind it."
- **Beats holding after the search?** "X Not clearly — holding is the honest
  call" (the modal BenchmarkWins case, `crown_clears_dsr == false`).

The block is visually distinct, uses the panel chrome, and all four facts
plus their plain-language glosses are legible. The negative-control test
(`scorecard_block_paints_and_exceeds_no_scorecard`) confirms the with-scorecard
frame paints strictly more foreground (>1200 px delta) than the same frame
with `scorecard = None`.

---

## 8. Anchor Verification Gate (verify-anchors — MANDATORY)

```bash
bash scripts/verify_anchors.sh
```

```
PASS  v2-mn-funding-fee00bps-theta-surface-2023-block-bootstrap-real-fy  16633a63...
PASS  v2-mn-funding-fee00bps-theta-surface-2024-block-bootstrap-real-fy  b3726a28...
PASS  v2-mn-funding-fee05bps-theta-surface-2023-block-bootstrap-real-fy  38ccc463...
PASS  v2-mn-funding-fee05bps-theta-surface-2024-block-bootstrap-real-fy  2e2ba8b6...
PASS  v2-mn-basisperp-fee00bps-theta-surface-2023-block-bootstrap-real-fy  1af13f14...
PASS  v2-mn-basisperp-fee00bps-theta-surface-2024-block-bootstrap-real-fy  058820ff...
PASS  v2-mn-basisperp-fee05bps-theta-surface-2023-block-bootstrap-real-fy  aedbc28a...
PASS  v2-mn-basisperp-fee05bps-theta-surface-2024-block-bootstrap-real-fy  23f03994...
---
ANCHORS PASS  (119 / 119)
```

All 119 anchors intact. No anchored report files were touched by the scorecard
increment. The advisor bakeoff path runs `write_report = false` — the scorecard
is anchor-safe by construction (v2-architecture.md §0).

---

## 9. Spec-Lint Gate

```bash
python3 scripts/spec_lint.py
```

```
spec-lint: PASS (0 violations)
```

No new violations introduced.

---

## 10. Cockpit-Smoke Gate

Per `.claude/skills/cockpit-smoke/SKILL.md`: this skill is **Orchestrator-only**
(sub-agents may not invoke `cargo run --bin cockpit`). The ui-designer emitted
the smoke log at `spec/v2/advisor-overfitting-scorecard/reports/cockpit-smoke-2026-06-29T08-41Z.log`
(0 panic lines found via `grep -c "panicked at\|non-unwinding panic\|fatal runtime error"` → 0).
The tester defers to this log per the skill's capability boundary.

---

## 11. Pre-existing Failures (Out of Scope — DO NOT Reroute)

`cargo test -p ui --test bakeoff_progress_render --features fixtures` — 2/3
tests FAIL with "ACCENT_2 fill expected >1500 px, got 0". This is a pre-
existing regression introduced in `afa5bfb feat(advisor-bakeoff-progress)`,
classified via git-stash 2026-06-29. It is **unrelated to the scorecard
feature** and is documented here as out-of-scope tech debt. It does NOT
block this PASS verdict.

---

## 12. Verdict

**`PASS`**

All mandatory gates pass:

1. `cargo build --workspace` — CLEAN (3m 01s, 0 errors).
2. `cargo test -p backtest` — 178 PASS / 0 FAIL / 8 ignored (all 8 pre-existing config-file ignores). All 16 scorecard unit tests pass, including the FROZEN-gate identity test `scorecard_does_not_change_ranking`.
3. `cargo test -p ui --lib` — 579 PASS / 0 FAIL. ScorecardView mirror tests pass.
4. `cargo test -p ui --test leaderboard_scorecard_render --features fixtures` — 2 PASS / 0 FAIL. Tester read the PNG and confirmed the "How much to trust this" block paints with all four facts + glosses on screen.
5. `cargo clippy -p backtest --tests -- -D warnings` — PASS (0 warnings).
6. `cargo clippy -p ui --tests --features fixtures -- -D warnings` — PASS (0 warnings).
7. `cargo fmt --check` — PASS (0 diff).
8. `bash scripts/verify_anchors.sh` — PASS (119/119). Gate byte-frozen throughout.
9. `python3 scripts/spec_lint.py` — PASS (0 violations).
10. Cockpit-smoke — Orchestrator-only; ui-designer log shows 0 panics.

The scorecard is report-only, additive, and passes the FROZEN-gate identity
test. The `bakeoff_progress_render` pre-existing failure is out-of-scope and
does not affect this verdict.

---

## 13. Routing

`VERDICT → PASS` — ready to ship.

---

## 14. Trace Column Update

Per tester ownership: `REQ-V2-ANALYSIS-001` `tests` column updated to include
`spec/v2/advisor-overfitting-scorecard/reports/test-2026-06-29-advisor-overfitting-scorecard.md`.

Anchor citations: this feature touched no anchored scenario (advisor bakeoff
runs `write_report = false`); `anchors = []` on `REQ-V2-ANALYSIS-001` is
correct by design (EXPECTED NONE — spec/architecture §0).
