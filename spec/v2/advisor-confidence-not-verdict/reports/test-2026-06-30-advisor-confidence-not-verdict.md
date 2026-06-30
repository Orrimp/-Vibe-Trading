---
title: Test Report
feature: advisor-confidence-not-verdict
run_id: 2026-06-30-1430-UTC
commit: bcc4c24c3a3c3508587bfd8ed625989b422db5cc
agent: tester
verdict: PASS
---

# Test Report — advisor-confidence-not-verdict — 2026-06-30 14:30 UTC

## 1. Scope

- **Feature / change under test:** P0-3 "Confidence check, not verdict" — `ScorecardSummary` + `Scorecard::summary()` in `crates/backtest`; `confidence: Option<ScorecardSummary>` wired into `ForwardRunConfig` + `ForwardPlan` in `crates/agent`; `ConfidenceSummaryView` mirror + confidence block UI in `crates/ui`; UI copy relabel (`FORWARD_PLAN_HEADLINE` → "Confidence check"); render-snapshot guard (`crates/ui/tests/forward_plan_confidence_render.rs`).
- **Spec refs:** `spec/v2/advisor-confidence-not-verdict/feature.md`, `spec/v2/advisor-confidence-not-verdict/tasks.md`
- **Commit SHA:** `bcc4c24c3a3c3508587bfd8ed625989b422db5cc`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `Darwin arm64 (macOS 25.5.0)`

## 2. Static Analysis

| Check               | Result | Notes                          |
|---------------------|--------|--------------------------------|
| `cargo fmt --check` | PASS   | No diffs emitted               |
| `cargo clippy -p backtest --tests -- -D warnings` | PASS | 0 warnings |
| `cargo clippy -p ui --tests --features fixtures -- -D warnings` | PASS | 0 warnings |
| `cargo audit`       | n/a    | Not run — no dependency changes in this commit |
| `cargo deny`        | n/a    | Not run — no dependency changes in this commit |

## 3. Unit & Integration Tests

| Crate       | Passed | Failed | Ignored | Duration  |
|-------------|-------:|-------:|--------:|----------:|
| `backtest`  |    195 |      0 |       8 |   0.64 s  |
| `agent`     |    101 |      0 |       0 |  47.33 s  |
| `ui` (lib)  |    583 |      0 |       0 |   0.69 s  |
| `ui` (render integration: `forward_plan_confidence_render`) | 2 | 0 | 0 | 62.64 s |
| **Total**   | **881** | **0** |   **8** |          |

### New P0-3 tests verified passing

- `bakeoff::scorecard::tests::scorecard_summary_positive_case` — PASS
- `bakeoff::scorecard::tests::scorecard_summary_degenerate_yields_none` — PASS
- `forward_plan_confidence_render::confidence_block_paints_more_foreground_than_without` — PASS
- `forward_plan_confidence_render::confidence_block_below_horizon_band` — PASS

### FROZEN gate identity proof

- `bakeoff::scorecard::tests::scorecard_does_not_change_ranking` — PASS (the P0-1 gate-identity test that proves the bakeoff FROZEN path is byte-untouched by the new `summary()` read-path projection).

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — No proptest or cargo-fuzz suites added in this feature.

## 5. Backtest Results

_n/a_ — P0-3 is a pure report-only read-path projection + UI mirror. The advisor bakeoff path uses `write_report=false` by construction; the ranking and verdict paths are byte-untouched (FROZEN gate confirmed). No anchor scenarios are emitted or modified.

## 6. Benchmarks

_n/a_ — No hot-path changes. `ScorecardSummary` is a trivial struct projection over pre-computed fields; latency budget is zero.

## 7. Render-Pixel Verification (CLAUDE.md non-negotiable)

Test: `cargo test -p ui --test forward_plan_confidence_render --features fixtures` — 2 tests, 0 failures, 62.64 s.

**PNG output at `/tmp/forward_plan_confidence_render.png` (with confidence):**

- Headline "Confidence check" paints at top-left (dark background, large white text) — CONFIRMED.
- Caption beneath confirms honest framing: "a confidence check on that pick, not a fresh prediction or a guarantee of future edge" — CONFIRMED.
- Disclaimer banner renders: "This is a conditional, rule-based plan — not a price prediction, and not an implied or expected return..." — CONFIRMED.
- The "How much to trust this pick" section renders below the Horizon block with 4 fact rows:
  - "Strategies tried: 18" — CONFIRMED
  - "Deflated confidence: 87%" — CONFIRMED
  - "Beats holding?: Not yet — edge uncertain after the search" (warning-style text) — CONFIRMED
  - "Minimum history needed: 6.4 yr" — CONFIRMED
- "Informational only — never a rule change. The pick stands regardless." footer text renders — CONFIRMED.
- Fixture values match `fake_forward_plan_with_confidence()` spec: `n_candidates=18`, `deflated_sharpe=0.87`, `crown_clears_dsr=false`, `min_btl_years=6.4` — CONFIRMED.

**PNG output at `/tmp/forward_plan_no_confidence_render.png` (negative control, `confidence: None`):**

- Headline "Confidence check" (same relabel) renders — CONFIRMED.
- The "How much to trust this pick" section and the 4-row confidence block are entirely absent — CONFIRMED. Only the disclaimer footer renders at the bottom.
- This confirms the `if let Some(c) = confidence { ... }` guard fires correctly: zero confidence rows paint when `confidence: None`.

## 8. Anchor Verification

`bash scripts/verify_anchors.sh` → `ANCHORS PASS  (119 / 119)` — all 119 body-SHA anchors match; zero regressions.

This result is expected by construction: P0-3 is a report-only read-path projection; the advisor bakeoff uses `write_report=false`; no anchored report files were created or modified.

## 9. Spec-Lint

`python3 scripts/spec_lint.py` → `spec-lint: PASS (0 violations)`

## 10. Cockpit Smoke (build + 7s no-panic window)

`cargo build -p ui --features fixtures,live` → `Finished` (1.15 s, no errors).

Cockpit binary spawned for 7 s, RUST_BACKTRACE=1. Panic count: **0**. Log: `scratchpad/cockpit-smoke-P0-3.log` (empty = clean).

cockpit-smoke: PASS (0 panics, 7s window)

## 11. Pre-existing Spec Debt

_none_ — spec-lint 0 violations; no pre-existing baseline violations to carry forward.

## 12. Verdict

**`PASS`**

All gates cleared on commit `bcc4c24`:

- `cargo build --workspace` clean (3m 59s, no errors).
- 881 tests pass across `backtest` (195), `agent` (101), `ui --lib` (583), `ui` render integration (2); 0 failures.
- The 2 new scorecard-summary unit tests (`scorecard_summary_positive_case`, `scorecard_summary_degenerate_yields_none`) pass.
- The FROZEN gate identity test (`scorecard_does_not_change_ranking`) still passes — the ranking path is byte-untouched.
- Render-pixel verification (CLAUDE.md non-negotiable): the 4-row confidence block ("Strategies tried / Deflated confidence / Beats holding? / Min history") paints below the Horizon band with the correct fixture values; the negative control (`confidence: None`) renders with the block entirely absent.
- `cargo fmt --check`, both clippy passes, 119/119 anchors, spec-lint 0 violations, cockpit-smoke 0 panics.

P0-3 is a pure read-path projection + UI mirror with no strategy/rank/verdict side effects. Shippable.

## 13. Routing

`VERDICT → PASS` — ready to ship. This closes Phase 2A (3/3 v2 features at tester-done: P0-1 overfitting-scorecard, P1 turnover+tail, P0-3 confidence-not-verdict).

**Presenter handoff:** P0-3 forward-plan confidence-check framing is verified shippable. The screen now reads "Confidence check" (not "Your plan") and surfaces the 4 overfitting facts (candidates tried, deflated confidence, beats-holding status, min history required) below the plan — honest framing that shows the bakeoff's statistical uncertainty without blocking the paper trade.
