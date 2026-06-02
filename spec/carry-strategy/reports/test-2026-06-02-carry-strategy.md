---
title: Test Report
feature: carry-strategy
run_id: 2026-06-02-1020-UTC
commit: a2be07e (carry-strategy M-DEV-8: anchored carry-C3 6x200 surfaces)
agent: tester
verdict: PASS
---

# Test Report — carry-strategy — 2026-06-02 10:20 UTC

## 1. Scope

- **Feature / change under test:** carry-strategy Stages 1–4a (M-DEV-0 through M-DEV-8). Full carry (funding) strategy implementation: FundingDataSource loader, as-of forward-fill, shared-index bootstrap co-resample of funding, ScoreSource::FundingCarry signal + R-CARRY.2 sign, funding-cashflow accrual in run_path, carry-C3 6-cell sweep wiring, and the two anchored surfaces (2023 + 2024). M-TEST science gate + anchor lock (#88 + #89).
- **Spec refs:** `spec/carry-strategy/feature.md`, `spec/carry-strategy/tasks.md`
- **Commit SHA:** `a2be07e` (HEAD at M-TEST)
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** Darwin 25.5.0 arm64 (Apple Silicon)

## 2. Static Analysis

| Check              | Result | Notes                       |
|--------------------|--------|-----------------------------|
| `cargo fmt --check`| PASS   | Zero diff on strategy, backtest, data crates |
| `cargo clippy`     | PASS   | 0 errors on `strategy`, `data`, `backtest --features candle realdata` (crates/ui 138 pre-existing pedantic lints excluded per task scope) |
| `cargo audit`      | N/A    | `cargo-audit` not installed on this box; `cargo deny` shows pre-existing advisories/licenses FAIL (not introduced by carry — pre-existing) |
| `cargo deny`       | PRE-EXISTING FAIL | advisories + licenses pre-existing; not a carry regression |

## 3. Unit & Integration Tests

| Crate | Passed | Failed | Ignored | Duration |
|-------|-------:|-------:|--------:|---------:|
| `strategy` (lib) | 136 | 0 | 0 | ~12s |
| `data` (synth::bootstrap) | 15 | 0 | 0 | <1s |
| `backtest` (lib, --features candle realdata) | 76 | 0 | 0 | <1s |
| `backtest` (carry_divergence_e2e) | 6 | 0 | 0 | 0.02s |
| `backtest` (param_sweep_e2e) | 8 | 0 | 0 | 1.72s |
| `backtest` (montecarlo_e2e) | 9 | 0 | 0 | 0.12s |
| **Total** | **250** | **0** | **0** | ~15s |

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest or cargo-fuzz suites in scope for carry.

## 5. Backtest Results

### Carry-C3 θ-Surface — 2023-FY (anchor #88)

**Universe:** 10 USDT pairs (BTCUSDT, ETHUSDT, BNBUSDT, …)
**Period:** 2023-FY (full year)
**Data source:** block-bootstrap-real, shared-index, OHLCV revision `3a8b96c4…`, funding revision `bf1ede44…`
**Fees / slippage:** 2 bps slippage + 4 bps taker fee (6 bps total)
**Grid:** 6 cells g∈{0..5}, N=200 paths each, seed=0xC0FFEE

| g  | l_settle | rebalance | K | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | funding_harvested | verdict |
|----|----------|-----------|---|-----------|------------|------------|-----------|-------------|-----------|-------------------|---------|
| 0  | 9        | 480m      | 3 | -0.1003   | +0.0252    | +0.0512    | 31.5%     | 0.0%        | 90.77%    | +3,098,097        | FRAGILE |
| 1  | 3        | 480m      | 3 | -0.0933   | +0.0154    | +0.0349    | 38.0%     | 0.0%        | 92.19%    | +1,572,693        | FRAGILE |
| 2  | 21       | 480m      | 3 | -0.1323   | +0.0278    | +0.0528    | 22.5%     | 0.0%        | 90.80%    | +1,997,086        | FRAGILE |
| 3  | 9        | 1440m     | 5 | -0.1009   | +0.0222    | +0.0671    | 25.5%     | 0.0%        | 89.89%    | -1,321,996        | FRAGILE |
| 4  | 9        | 480m      | 1 | -0.1921   | +0.0386    | +0.0774    | 14.0%     | 0.0%        | 87.43%    | +4,081,759        | FRAGILE |
| 5  | 3        | 480m      | 5 | -0.0717   | +0.0170    | +0.0473    | 23.5%     | 0.0%        | 91.15%    | -1,626,590        | FRAGILE |

**Buy-and-hold control (same N=200 paths):** p5=+0.124, p50=+1.735, p95=+3.870, P(loss)=4.5%, P(Sharpe>1)=77.5%, p95_maxdd=51.15%

**FAMILY VERDICT: FAMILY-UNIFORM-FRAGILE** — all 6 cells FRAGILE under frozen § 0 bands (p5 Sharpe < 0 for all 6 cells).

**Science gate (frozen § 0 rule application):** Every cell has p5 Sharpe < 0, which is the FRAGILE trigger per the primary band. P(Sharpe>1)=0.000 for all cells. No cell clears the ROBUST threshold. No non-FRAGILE cell → no `→ C5 DEFLATION REQUIRED` flag needed. FP-C3.5 anti-cherry-pick: the surface reports the full grid without crowning an argmax winner.

**Bar comparison (2023):** Best carry cell (g=4, p50=+0.039) vs buy-and-hold (p50=+1.74). Carry falls approximately 1.70 Sharpe units below passive holding. The fee-bleed hypothesis is corroborated: g=4 (K=1, lowest selection churn) achieves the best p50 carry result, but even this structurally-low-turnover configuration cannot overcome the combination of selection volatility and directional price exposure.

### Carry-C3 θ-Surface — 2024-FY (anchor #89)

**Universe:** same 10 USDT pairs
**Period:** 2024-FY (full year, harder tail-negative regime)
**Data source:** same locked grid; OHLCV revision `3a8b96c4…`, funding revision `bf1ede44…`

| g  | l_settle | rebalance | K | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | verdict |
|----|----------|-----------|---|-----------|------------|------------|-----------|-------------|-----------|---------|
| 0  | 9        | 480m      | 3 | -0.0186   | +0.0046    | +0.0383    | 34.5%     | 0.0%        | 80.66%    | FRAGILE |
| 1  | 3        | 480m      | 3 | -0.0336   | -0.0014    | +0.0268    | 54.5%     | 0.0%        | 82.04%    | FRAGILE |
| 2  | 21       | 480m      | 3 | -0.0262   | -0.0001    | +0.0350    | 50.0%     | 0.0%        | 77.05%    | FRAGILE |
| 3  | 9        | 1440m     | 5 | -0.0661   | -0.0094    | +0.0434    | 61.0%     | 0.0%        | 83.86%    | FRAGILE |
| 4  | 9        | 480m      | 1 | +0.0163   | +0.0427    | +0.0942    | 2.5%      | 0.0%        | 63.60%    | FRAGILE |
| 5  | 3        | 480m      | 5 | -0.0569   | -0.0169    | +0.0283    | 72.5%     | 0.0%        | 87.83%    | FRAGILE |

**Buy-and-hold control (same N=200 paths):** p5=-0.682, p50=+1.105, p95=+2.690, P(loss)=16.5%, P(Sharpe>1)=53.5%, p95_maxdd=64.83%

**FAMILY VERDICT: FAMILY-UNIFORM-FRAGILE** — all 6 cells FRAGILE under frozen § 0 bands. The weakest-link for g=4 (the single cell with p5 > 0) is p95_maxdd = 63.6%, which is well above the ~50% ROBUST threshold. P(Sharpe>1)=0.000 confirms FRAGILE. No cell is non-FRAGILE.

**Bar comparison (2024):** Best carry cell (g=4, p50=+0.043) vs buy-and-hold (p50=+1.10). Even in the harder 2024 regime (tail-negative BH control), the BH bar stays approximately 1.06 Sharpe units above the best carry cell.

**Pre-flight void-if-fail confirmation:**
- 2023 report: `generator: block-bootstrap-real` PRESENT; `bootstrap_mode: shared-index` PRESENT. VALID.
- 2024 report: `generator: block-bootstrap-real` PRESENT; `bootstrap_mode: shared-index` PRESENT. VALID.
- Both reports: OHLCV revision `3a8b96c4…` + funding revision `bf1ede44…` in hashed body. VALID.

**Realized-funding column:** Both surfaces contain non-zero `funding_harvested` values per cell (confirming R-CARRY.10b: the cashflow accrual is active). Mixed sign values in some cells (g=3, g=5 in 2023; most cells in 2024) reflect the long-only directional framing (a): when the strategy holds names that become positive-funding over the bootstrap paths, funding is paid rather than earned.

### Science Gate — Falsifier RED-on-revert Verification

All 4 mandatory falsifiers independently verified RED on property revert:

**R-CARRY.2 (sign convention):** Mutated `carry_score` return from `Some(-mean)` to `Some(mean)` (flipped sign). Tests `r_carry2_sign_assertion_longs_negative_funding_name` and `r_carry2_carry_score_negative_funding_outscores_positive` in `crates/strategy/src/cross_sectional/momentum.rs` both FAILED with explicit messages: *"R-CARRY.2 SIGN VIOLATION: carry strategy MUST select ETHUSDT (negative funding) but got: BTCUSDT"* and *"SIGN VIOLATION: carry_score(ETHUSDT)=-0.0001 must be > carry_score(BTCUSDT)=0.0001"*. RESTORED GREEN.

**R-CARRY.6 (no-look-ahead):** Mutated `funding_as_of` from `t <= bar_ts` to `t < bar_ts`. Test `no_look_ahead_falsifier` in `crates/backtest/src/funding_data.rs` FAILED: *"assertion `left == right` failed — left: [Some(0.001)] right: [Some(0.002)]"*. RESTORED GREEN.

**R-CARRY.10a (carry-vs-price divergence):** Collapsed `ScoreSource::FundingCarry` arm to use `score_vol_adjusted_return` (price fallback, never calling `carry_score`). The integration tests `r_carry_10b_integration_cashflow_non_no_op`, `r_carry_2_sign_assertion_integration`, and `r_carry_6_no_look_ahead_integration` all FAILED with "equity_with=100000, equity_zero=100000, diff=0" messages. The funding rings are never populated when `carry_score` is not called, causing a cascade RED across 3 falsifiers simultaneously — confirming the wiring is tight. RESTORED GREEN.

**R-CARRY.10b (cashflow non-no-op):** Zeroed `cash += cashflow` (kept `realized_funding_total += cashflow` for tracking but dropped the actual cash update). Test `r_carry10b_funding_cashflow_non_no_op` in `crates/backtest/src/scenarios/montecarlo.rs` FAILED: *"R-CARRY.10b NON-NO-OP VIOLATION: diff=0.000… the cashflow is computed-and-ignored (the v3-vol-overlay no-op pattern)"*. RESTORED GREEN.

**Two-run byte-identity:**
- `carry_two_run_byte_identity` (in `carry_divergence_e2e.rs`): PASS — two sweeps at seed 0xC0FFEE produce identical formatted summaries.
- `fp_c3_3_two_run_byte_identity` (in `param_sweep_e2e.rs`): PASS — the param sweep two-run identity holds.

### Anchor Verification

**Before lock (baseline):** `bash scripts/verify_anchors.sh` → **87/87 PASS** (all momentum #86 + MR #87 and all prior anchors byte-identical; carry path is additive/off for all pre-existing runs).

**After lock (#88 + #89 added):** `bash scripts/verify_anchors.sh` → **89/89 PASS**.

- Anchor #88 (`v1-carry-theta-surface-2023-block-bootstrap-real-fy`):
  SHA `f03cd7145699f854768e1721ee675d7aa87a10269694f41561c404f2e1b9f2c4`
- Anchor #89 (`v1-carry-theta-surface-2024-block-bootstrap-real-fy`):
  SHA `fd96d5a87fd9ad18c98cf38f5f3c17a55c8a79e92a1a0845a724a507bb51e199`
- Body hash method: `scripts/hash_report.py` (strip YAML front-matter `---\n...\n---\n`, SHA-256 over UTF-8 body bytes) — the canonical method used by all 89 anchors.
- Momentum #86 (`0dd989d9…`) and MR #87 (`a708112e…`): PASS byte-identical. Confirmed additive.

### Family Verdict — Program-Level Read

**Both regimes return FAMILY-UNIFORM-FRAGILE**, completing the three-strategy program:

| Family | Year | Best-cell p50 | BH p50 bar | vs bar | Verdict |
|--------|------|---------------|------------|--------|---------|
| Momentum | 2023 | +0.014 | +1.74 | -1.73 | FAMILY-UNIFORM-FRAGILE |
| Mean-reversion | 2023 | +0.007 | +1.74 | -1.73 | FAMILY-UNIFORM-FRAGILE |
| **Carry** | **2023** | **+0.039** | **+1.74** | **-1.70** | **FAMILY-UNIFORM-FRAGILE** |
| **Carry** | **2024** | **+0.043** | **+1.10** | **-1.06** | **FAMILY-UNIFORM-FRAGILE** |

**Program-level conclusion:** All three tested cross-sectional strategy classes — price-momentum (top-K winners), price-mean-reversion (bottom-K losers), and funding-carry (long the most-negative-funding names) — are dominated by passive equal-weight buy-and-hold on the 10-USDT-pair Binance universe. The carry strategy was the pre-registered rotation target with the best a-priori structural case (non-trend, genuinely independent return source, naturally low-turnover vs the price families). Even with this structural advantage, carry FAMILY-UNIFORM-FRAGILE on both 2023 (bull) and 2024 (harder tail) regimes.

The honest diagnosis: framing (a) long-only directional carry-tilt holds perp exposure on the negative-funding names, so P&L is dominated by price risk rather than the funding premium. The long-only engine cannot isolate the funding signal from the price beta. The funding cashflow is real and non-zero (realized_funding column shows meaningful accruals), but the directional price exposure overwhelms it. The v0.2.0 durable follow-on (market-neutral long/short harvest, framing (b)) would need the short-side engine, and only warrants building if the funding signal has a directional edge — which this result suggests is unlikely at these parameter ranges.

This result is a **methodology win**: the harness has cheaply ruled out the three most-cited crypto cross-sectional strategy families on this universe, completing the decision-grade go/no-go at fraction of a live-trading cost. The next rotation (value, regime/blended) is data-gated.

### Regressions vs Baseline

None. The carry path is additive (defaults-off); all 87 pre-existing anchors remain byte-identical.

## 6. Benchmarks

_n/a_ — no criterion benchmarks added by carry; hot paths (the funding gather in `bootstrap.rs`) are O(n_bars) per path and confirmed negligible vs wall-clock (30.7s / 28.4s for 6×200 = 1200 paths, comparable to MR's 6×200 wall-clock).

## 7. Environment / Infrastructure Issues

- `cargo audit` not installed on this machine (pre-existing — not a carry regression).
- `cargo deny` shows pre-existing advisories/licenses FAIL (not introduced by carry).
- `data/yahoo/REVISION.toml` has a stale uncommitted working-tree change (operator-noted; out of scope; not touched).
- `crates/ui` has 138 pre-existing pedantic clippy lints (known; out of scope per task instructions).

## 8. Verdict

**`PASS`**

The carry implementation is sound: all 89 anchors PASS (including the 2 newly locked carry surfaces), the 4 mandatory falsifiers are each independently RED-on-revert (sign, no-look-ahead, carry-vs-price divergence, cashflow non-no-op), the two-run byte-identity holds, and both surface reports carry correct provenance (block-bootstrap-real / shared-index / OHLCV `3a8b96c4…` + funding `bf1ede44…`) with real (non-zero) realized-funding columns.

The science finding — FAMILY-UNIFORM-FRAGILE on both 2023 and 2024 regimes — is the correct and expected result from a sound implementation. It is NOT a test regression. The carry strategy correctly implements the pre-registered rotation target design (framing (a) long-only directional carry-tilt per D-CARRY.0), and the harness has faithfully evaluated it against the frozen decision-rule bands.

**spec-lint: FAIL (94 violations, 2 categories: dead-link 87 + trace-broken-path 7).** All violations are pre-existing baseline carry-overs (baseline 2026-06-01: 95 violations; today: 94 — one fewer due to missing-frontmatter now resolved). No new violation categories or counts introduced by carry. Pre-existing spec debt quoted below.

**verify-anchors: 89/89 PASS** (87 pre-existing + #88 carry-2023 + #89 carry-2024).

## 9. Pre-existing Spec Debt (carried into this report per non-negotiable visibility rule)

- **dead-link (87):** All pre-existing. Clusters: ADR-0027 (5 links to archived v25-kronos-forecast-overlay), chart-canvas-overhaul (7 links to ephemeral /tmp screenshots), ui-rethink phase D/E presentations (7 links to untracked visual baselines), v0-paper-sma README (6 stale pre-flat-layout paths), v3-volatility-forecaster anchored report (1 dead link — byte-immutable per ADR-0038 § D6, cannot be edited), others. Owner: analyst/architect/developer per category.
- **trace-broken-path (7):** All pre-existing. REQ-LAB-YAHOO-REALDATA-V0-1-4-001 (renamed slug), REQ-VISUAL-FAIL-HTML-REPORTER-001 (archived doc path + 2 bare function test paths), REQ-UI-CONTRAST-ASSERTER-001 (archived doc path), REQ-QUEUE-STALENESS-RECONCILIATION-001 (script self-test path), REQ-OPERATOR-LEDGER-SCHEMA-LINT-001 (script self-test path).

## 10. Routing

`VERDICT → PASS` — implementation is sound; both carry surfaces anchored (#88 + #89); science finding (FAMILY-UNIFORM-FRAGILE) is the correct output of a correct harness. Ready for presenter.
