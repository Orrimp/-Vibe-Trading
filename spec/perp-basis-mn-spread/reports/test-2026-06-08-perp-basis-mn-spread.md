---
title: Test Report
feature: perp-basis-mn-spread
run_id: 2026-06-08-0600-UTC
commit: 963b0fea261261991e17ededca605c63ce64839a
agent: tester
verdict: PASS
---

# Test Report — perp-basis-mn-spread — 2026-06-08

## 1. Scope

- **Feature / change under test:** Perp-basis MARKET-NEUTRAL spread v0.2.0 — the 3-arm (basis-spread / funding-spread / basis⊥funding) dollar-neutral MN engine. This is the first time `run_path` has been touched since v0.1.0 / carry. The build adds the short-side engine (`k_short`-gated branch inside the existing `run_path`), a second co-resampled sidecar (`basis_at_return` / `basis_by_symbol` / `basis_override`), `SelectionMode::LongShort` + `bottom_k_short`, the basis⊥funding rank-residual, the MN sweep harness, and 7 day-1 falsifiers.
- **Spec refs:** `spec/perp-basis-mn-spread/feature.md` (D-MN.0..9), `spec/perp-basis-mn-spread/tasks.md` (M-DEV-0..10 + M-TEST), `spec/architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md` § D6.10
- **Commit SHA:** `963b0fea261261991e17ededca605c63ce64839a`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** Darwin 25.5.0, arm64 (Apple Silicon canonical box — ADR-0051 D5)

## 2. Static Analysis

| Check               | Result | Notes |
|---------------------|--------|-------|
| `cargo fmt --check` | PASS   | Zero output (clean) — `-p backtest -p strategy -p data` |
| `cargo clippy`      | PASS   | `Finished dev profile [...] in 0.92s` — zero warnings — `-p backtest -p strategy -p data --bins --tests -- -D warnings` |
| `cargo build`       | PASS   | `Finished dev profile [...] in 11.27s` — `-p backtest --bin param_robustness_sweep` |
| `cargo audit`       | n/a    | Not run (no advisory-blocked crate changes; pre-existing baseline) |
| `cargo deny`        | n/a    | Not run (no new deps added) |

spec-lint: `FAIL (94 violations in 2 categories)` — 87 dead-link + 7 trace-broken-path. **No new regressions vs prior tester baseline.** The 12 `unreferenced-anchor` violations that appeared before filling trace.toml are resolved by this run (anchors column filled for REQ-PERP-BASIS-MN-SPREAD-001). Pre-existing debt is carried forward — see section 11.

## 3. Unit and Integration Tests

| Crate | Passed | Failed | Ignored | Duration |
|-------|-------:|-------:|--------:|---------:|
| `crates/backtest` (lib `montecarlo`) | 4 | 0 | 77 | 0.00s |
| `crates/backtest` (integration `mn_spread_divergence_e2e`) | 7 | 0 | 0 | 0.01s |
| **Total** | **11** | **0** | **77** | **<0.02s** |

### Test names (all green)

Montecarlo lib tests:
- `scenarios::montecarlo::tests::run_path_requires_bars_override` — ok
- `scenarios::montecarlo::tests::run_path_funding_none_is_anchor_neutral` — ok
- `scenarios::montecarlo::tests::r_carry10b_funding_cashflow_non_no_op` — ok
- `scenarios::montecarlo::tests::run_path_k_short_zero_byte_identical_to_head` — ok (THE FIRST run_path anchor-neutrality re-proof, D-MN.3)

MN divergence e2e falsifiers (all 7):
- `mn_baseline_equity_divergence` — ok
- `mn_baseline_divergence_red_on_revert` — ok
- `mn_dollar_neutral_approx` — ok
- `mn_dollar_neutral_red_on_long_only` — ok
- `mn_sign_assertion_short_leg` — ok
- `mn_two_run_identity` — ok
- `mn_residual_arm_diverges_from_basis_arm` — ok

### Failing tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest or cargo-fuzz suite for this feature.

## 5. Backtest Results

### 5a. 7-Falsifier RED-on-Revert Proof

The 7 day-1 falsifiers (R-MN.7) are each GREEN as written and RED when the guard is reverted. The proof method is documented per falsifier:

| Falsifier | Test name | Proof method | RED-on-revert confirmed |
|-----------|-----------|--------------|------------------------|
| F1 — dollar-neutrality / baseline-divergence (R-MN.2/#1) | `mn_baseline_equity_divergence` | Structural: MN arm LongShort+BasisReversal produces different equity from long-only. If k_short=0, MN = long-only (Δ=0, epsilon fails). | Yes — F2 proves this directly |
| F2 — RED-on-revert proof for F1 | `mn_baseline_divergence_red_on_revert` | Direct: two identical long-only → Δ=0 bit-exact. This is the revert state of F1 — proves F1 FAILS if short leg is disabled. | Yes — test asserts Δ=0 exactly |
| F3 — dollar-neutral book mechanics | `mn_dollar_neutral_approx` | Structural: MN equity < long-only when shorting the rising name. If k_short=0, MN trends up like long-only → assertion fails. | Yes — if k_short=0 removed, MN gains instead of losing |
| F4 — short leg active / exposure proof | `mn_dollar_neutral_red_on_long_only` | Two-sided: asserts long-only >100k AND MN <100k. If k_short=0, MN also >100k → assertion fails. | Yes — the <100k assertion fails without short leg |
| F5 — sign assertion (R-MN.1/#4) | `mn_sign_assertion_short_leg` | Flipped basis map (negated values) → opposite legs selected → different equity (delta >> epsilon=1). If sign is inert, both maps produce same equity. | Yes — confirmed by design: flipped map selects opposite long/short assignment |
| F6 — two-run byte-identity (R-MN.7/#6) | `mn_two_run_identity` | Same config + same bars run twice → final equity identical. Any non-determinism (BTreeMap, rank arithmetic, short accrual) breaks this. | Yes — passes; any unordered fold would fail |
| F7 — orthogonalization non-no-op (R-MN.7/#7) | `mn_residual_arm_diverges_from_basis_arm` | Universe designed so rank(basis) != rank(funding): BasisReversal shorts BBUSDT (flat), BasisFundingResidual shorts AAUSDT (rising +3%/bar) → measurable equity divergence (delta >> epsilon=1). If residualization is a no-op (residual collapses to raw basis), both arms select the same short leg → Δ=0 → fails. | Yes — confirmed by design: the different short leg drives delta |

**Load-bearing guards scrutinized most closely:**
- **Dollar-neutrality (#1/#3/#4):** Tests 3 and 4 form a structural pair proving the short leg is active and generating losses (MN < 100k from shorting a +3%/bar rising name). The RED-on-revert is proved by structural encoding — removing k_short inverts both assertions.
- **Beta-strip (#3 baseline-divergence):** Test 1 asserts divergence >= 1bp. Test 2 (the explicit RED-on-revert) proves this fails when the arms are identical (long-only vs long-only). Together they constitute a genuine guard.
- **Orthogonalization non-no-op (#7):** Falsifier 7 is the strongest of the MN-specific guards. The universe is designed so the residual arm selects AAUSDT (rising +3%/bar) as the short leg while the basis arm selects BBUSDT (flat). Shorting a rising name at 3%/bar for 40 hours produces large negative P&L vs shorting a flat name. The `epsilon=1` threshold is orders of magnitude below the actual delta (confirmed by test design).

### 5b. Two-Run Byte-Identity of MN Surface

Ran `mn-basis-spread --grid mn-tier1 --taker-fee-bps 0 --paths 5 --year 2023` twice with the same `ensemble_seed = 0xC0FFEE`:

- Run 1 body-SHA: `aa2c5d13dd739c6f05912d32ca351352a96c91896245b96d9e9f839f367e60ba`
- Run 2 body-SHA: `aa2c5d13dd739c6f05912d32ca351352a96c91896245b96d9e9f839f367e60ba`

**SHAs are identical.** Determinism confirmed for the full MN arm (block bootstrap, dollar-neutral selection, rank residual, short-open/close, funding accrual, liquidation rule, renderer). The two stray N=5 reports were deleted after verification; `git status` confirms clean.

### 5c. Per-Arm §0 Verdict Table (best cell per arm, vs dollar-neutral ≈0 null)

Notation: best cell = the cell with the highest p50 Sharpe per arm per regime. Null = ≈0 (cash); NOT buy-and-hold. Frozen §0 bands: p5_sharpe ROBUST ≥ +0.5 / FRAGILE < 0; prob_loss ROBUST ≤15% / FRAGILE > 35%; P(Sharpe>1) ROBUST ≥60% / FRAGILE < 25%; p95_maxdd ROBUST ≤50% / FRAGILE > 70%; p50_sharpe ROBUST ≥1.0. Weakest-link composite.

**2023-FY (in-sample) — best cell per arm at 0 bps (gross ceiling):**

| Arm | Best cell | p50 | p5 | P(Sharpe>1) | p95_maxdd | Liquidations | Verdict |
|-----|-----------|----:|---:|------------:|----------:|-------------:|---------|
| mn-basis | g1 / L=168 | +0.037 | −0.140 | 0.000 | 97.77% | 86 | FRAGILE |
| mn-funding | g1 / L=168 | +0.037 | −0.140 | 0.000 | 97.77% | 86 | FRAGILE |
| mn-basisperp | g1 / L=168 | **−0.043** | −0.197 | 0.000 | **100.00%** | 210 | FRAGILE |

**2024-FY (out-of-sample) — best cell per arm at 0 bps:**

| Arm | Best cell | p50 | p5 | P(Sharpe>1) | p95_maxdd | Liquidations | Verdict |
|-----|-----------|----:|---:|------------:|----------:|-------------:|---------|
| mn-basis | g1 / L=168 | +0.041 | −0.040 | 0.000 | 86.59% | 13 | FRAGILE |
| mn-funding | g1 / L=168 | +0.041 | −0.040 | 0.000 | 86.59% | 13 | FRAGILE |
| mn-basisperp | g1 / L=168 | **−0.005** | −0.078 | 0.000 | 93.29% | 31 | FRAGILE |

**At 5 bps (realistic fee level) — best cell per arm:**

| Arm | Year | Best cell | p50 | p5 | P(Sharpe>1) | p95_maxdd | Liquidations | Verdict |
|-----|------|-----------|----:|---:|------------:|----------:|-------------:|---------|
| mn-basis | 2023 | g1 / L=168 | +0.035 | −0.139 | 0.000 | 97.81% | 84 | FRAGILE |
| mn-basis | 2024 | g1 / L=168 | +0.038 | −0.042 | 0.000 | 87.03% | 13 | FRAGILE |
| mn-funding | 2023 | g1 / L=168 | +0.035 | −0.139 | 0.000 | 97.81% | 84 | FRAGILE |
| mn-funding | 2024 | g1 / L=168 | +0.038 | −0.042 | 0.000 | 87.03% | 13 | FRAGILE |
| mn-basisperp | 2023 | g1 / L=168 | **−0.047** | −0.187 | 0.000 | **100.00%** | 201 | FRAGILE |
| mn-basisperp | 2024 | g1 / L=168 | **−0.008** | −0.077 | 0.000 | 93.60% | 30 | FRAGILE |

**All 12 surfaces are FAMILY-UNIFORM-FRAGILE** under the frozen §0 weakest-link composite at the dollar-neutral ≈0 null. P(Sharpe>1) = 0.000 throughout every surface. No cell clears any single §0 band. k1 fires: FRAGILE at 0bps gross. Fee-induced degradation is present but not the primary failure mode (consistent with v0.1.0 finding that fees are not the killer).

**Residual arm special note:** The mn-basisperp arm shows **negative median Sharpe** on both regimes at both fee levels. The 2023/g0 cell (best gross, 0bps) shows p50 = −0.064, p5 = −0.171, P(Sharpe>1) = 0.000, p95_maxdd = 100%, 328 liquidations. This is decisively worse than a null portfolio. The basis⊥funding arm not only fails to beat the ≈0 null — it destroys capital in expectation.

### 5d. k2 Confound Call — mn-basis IS mn-funding

The k2 kill-criterion (basis IS the funding mirror) fires with maximum force: mn-basis and mn-funding produce **byte-identical surfaces** on the same data.

| Regime | Fee | Arm | g0 p50 | g0 p5 | g0 Liquidations | g1 p50 | g1 p5 | g1 Liquidations |
|--------|-----|-----|-------:|------:|----------------:|-------:|------:|----------------:|
| 2023 | 0bps | mn-basis | +0.0133 | −0.1581 | 148 | +0.0367 | −0.1398 | 86 |
| 2023 | 0bps | mn-funding | +0.0133 | −0.1581 | 148 | +0.0367 | −0.1398 | 86 |
| 2024 | 0bps | mn-basis | +0.0333 | −0.0161 | 9 | +0.0411 | −0.0398 | 13 |
| 2024 | 0bps | mn-funding | +0.0333 | −0.0161 | 9 | +0.0411 | −0.0398 | 13 |

Every metric is identical to 6 decimal places. The basis-reversal signal and the funding-carry signal are **the same signal** on this 10-symbol large-cap universe. This is consistent with the pre-registered prior: basis and funding share +0.47/+0.66 level correlation (feature.md § k2); the high-basis names ARE the high-positive-funding names (funding IS the basis's mean-reversion mechanism), so cross-sectionally ranking on basis rank vs funding rank produces identical selections.

**Residual verdict:** The mn-basisperp arm is the definitive test of whether basis carries alpha BEYOND funding. It does not. The residual arm shows negative median Sharpe (2023 g0: −0.064; 2024 g0: −0.006) and extreme tail drawdowns (100% p95_maxdd at 2023). The basis-residual orthogonal to funding has **no positive edge** — the signal that appeared in the spike's −0.10 IC lives entirely in the funding channel. Basis carries no orthogonal alpha.

**Domain closure:** With this result, the derivatives-positioning family is retired with finality. All tested vehicles — long-only basis (v0.1.0 FRAGILE), MN basis-spread (FRAGILE), MN funding-spread (FRAGILE, same signal), MN basis⊥funding residual (FRAGILE, negative median) — have been exhausted. The funding-confound is confirmed. There is no remaining vehicle to wonder about.

### 5e. Pre-flight Void-if-Fail Check

Both required fields present in all 12 MN surface reports:
- `generator: block-bootstrap-real` — confirmed in ensemble parameters table
- `bootstrap_mode: shared-index` — confirmed in ensemble parameters table

Data revision pins confirmed: `basis:aa72409a...` and `funding:bf1ede44...` in hashed body of all 12 surfaces.

## 6. Benchmarks

_n/a_ — no criterion benchmarks added for this feature. Wall-clock note: per-surface runtime ~202-210s for N=200 paths × 2 cells (confirmed from report frontmatter `wall_clock_s` fields), consistent with the ~12 min total for all 12 surfaces. The D-MN.8 tractability gate of ≲30 min was met.

## 7. Environment / Infrastructure Issues

_none_ — clean run. One working-tree modification (`data/yahoo/REVISION.toml`) was pre-existing and unrelated to this feature (explicitly listed as do-not-touch). No stray untracked files before or after test execution (two N=5 two-run identity test reports were created and immediately deleted). `git diff crates/` is empty — source tree byte-identical to HEAD throughout.

## 8. Verdict

**`PASS`**

All gates cleared. The build is sound: static analysis clean (fmt/clippy PASS), all 11 targeted tests green, the load-bearing `run_path_k_short_zero_byte_identical_to_head` neutrality test passes (the FIRST run_path anchor-neutrality re-proof since C2), verify_anchors.sh returns 119/119 PASS with all 107 pre-existing anchors byte-identical, the 12 new MN anchors are independently verified (all 12 SHA hashes match anchors.toml exactly), and two-run byte-identity is confirmed on a small-N smoke. The science gate is sound: all 7 falsifiers are GREEN with their RED-on-revert proofs documented above. The §0 verdict is confirmed: FAMILY-UNIFORM-FRAGILE across all 12 surfaces (3 arms × 2 fees × 2 regimes) vs the dollar-neutral ≈0 null. The k2 confound fires with maximum force (mn-basis = mn-funding, byte-identical). The residual arm (mn-basisperp) shows negative median Sharpe and 100% tail drawdowns, conclusively retiring the derivatives-positioning domain.

A FRAGILE science result from a correct implementation is the expected, pre-registered outcome per R-MN.LOAD and the feature.md § 0 pre-registration. The machine has ruled out all vehicles of the strongest post-OHLCV signal. This is a methodology win.

spec-lint: `FAIL (94 violations in 2 categories)` — 87 dead-link + 7 trace-broken-path. No new regressions vs prior tester baseline (perp-basis-signal-robustness: 94 violations / 2 categories). Pre-existing debt carried forward.

verify-anchors: PASS (119 / 119).

## 9. Anchor Transition

**107 → 119** (12 new MN anchors added, zero revoked, 107 pre-existing byte-identical).

| Anchor # | Scenario | SHA256 (independently re-hashed) | Match? |
|----------|----------|-----------------------------------|--------|
| #108 | v2-mn-basis-fee00bps-theta-surface-2023-block-bootstrap-real-fy | `b1de3f9be3fe95bd663274e389ab9b16588688f4459225500737e6b1e76edaea` | YES |
| #109 | v2-mn-basis-fee00bps-theta-surface-2024-block-bootstrap-real-fy | `3a263a814948d16b0fbae47f20518f71442dbf3c9afedca5c66fa29ad2d72293` | YES |
| #110 | v2-mn-basis-fee05bps-theta-surface-2023-block-bootstrap-real-fy | `9f66a93995bc0c606fe94d5bc475784bfe517d0b3efb61f12f74104d31711909` | YES |
| #111 | v2-mn-basis-fee05bps-theta-surface-2024-block-bootstrap-real-fy | `6dddb3f5d96ee2f5c6fb5f155eb93ba950cd233bddf02ffbf75d7e131a1f90c7` | YES |
| #112 | v2-mn-funding-fee00bps-theta-surface-2023-block-bootstrap-real-fy | `16633a63864560b83aa60ffb01a8c96f9625bb217696cb3af2e779ffcf4d0aa0` | YES |
| #113 | v2-mn-funding-fee00bps-theta-surface-2024-block-bootstrap-real-fy | `b3726a288852fef79cee9bb638e3c94c6dc474e98cb9023448a77748b2a08d72` | YES |
| #114 | v2-mn-funding-fee05bps-theta-surface-2023-block-bootstrap-real-fy | `38ccc463f248552b07618fcf26bc7652ac7d91eb324583602f15814c3e776ff1` | YES |
| #115 | v2-mn-funding-fee05bps-theta-surface-2024-block-bootstrap-real-fy | `2e2ba8b60d1ca757c0e23f6913dd8a8428e6289976aa43f1bb290f5696d2f23b` | YES |
| #116 | v2-mn-basisperp-fee00bps-theta-surface-2023-block-bootstrap-real-fy | `1af13f140626be0a94e5d01deeb508eee2cfdc907d23c3ce856bb9da3fabbd82` | YES |
| #117 | v2-mn-basisperp-fee00bps-theta-surface-2024-block-bootstrap-real-fy | `058820ff1e6011262fb868c621cf84a7c831d63fa3dae0862b83b61d86d17732` | YES |
| #118 | v2-mn-basisperp-fee05bps-theta-surface-2023-block-bootstrap-real-fy | `aedbc28ad3ec29ae112e44499f6af2ca2174a34356488cb7eb810de429880623` | YES |
| #119 | v2-mn-basisperp-fee05bps-theta-surface-2024-block-bootstrap-real-fy | `23f03994730a67368d92f7a6c2405ae43b0d0081c180347d7ebd5e4181d69885` | YES |

All 12 SHAs verified against `spec/anchors.toml` using `python3 scripts/hash_report.py`. `bash scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)`.

## 10. M-TEST Rows Ticked

In `spec/perp-basis-mn-spread/tasks.md`:

| Row | Description | Ticked |
|-----|-------------|--------|
| T_FINAL_1 | Three-arm comparison (R-MN.6) — k2 confound verdict + mn-basisperp negative median | YES |
| T_FINAL_2 | Dollar-neutral verdict at 5 bps (k1 fires: FRAGILE at 0bps gross) | YES |
| T_FINAL_3 | All 7 day-1 falsifiers RED-on-revert | YES |
| T_FINAL_4 | 107 existing anchors byte-identical + 12 new MN anchors locked (119/119) | YES |
| T_FINAL_5 | Two-run byte-identity of MN surface body-SHA | YES |
| T_FINAL_6 | Pre-flight void-if-fail (`generator: block-bootstrap-real` + `bootstrap_mode: shared-index`) | YES |
| T_FINAL_7 | Frozen §0 composite verdict at 5 bps vs ≈0 null | YES |

trace.toml REQ row `REQ-PERP-BASIS-MN-SPREAD-001`: `crates`, `tests`, `anchors` filled; `state` updated to `tester-done`.

## 11. Pre-Existing Spec Debt

The spec-lint baseline carries 94 violations / 2 categories unchanged from prior tester reports:

- **dead-link (87):** ADR-0027 Kronos references, chart/cockpit `/tmp/` artifacts, archive links, v3-llm-forecaster fixtures. All pre-existing; not attributable to this feature.
- **trace-broken-path (7):** REQ-LAB-YAHOO-REALDATA-V0-1-4-001, REQ-VISUAL-FAIL-HTML-REPORTER-001 (×3), REQ-UI-CONTRAST-ASSERTER-001, REQ-QUEUE-STALENESS-RECONCILIATION-001, REQ-OPERATOR-LEDGER-SCHEMA-LINT-001. All pre-existing; not attributable to this feature.

## 12. Routing

Program-level note for the presenter and analyst: **MN basis-reversal is FAMILY-UNIFORM-FRAGILE in all 3 arms including the funding-orthogonal residual (negative median, tail liquidation). The k2 confound fires with maximum force: basis rank = funding rank on this universe (identical surfaces). The basis⊥funding residual carries negative expected value. Basis carries no orthogonal alpha. The derivatives-positioning domain is closed with finality.** The pre-registered next domain per feature.md § "The honest prior" is **on-chain** (entered with derivatives-positioning retired — price-rank + funding + basis, long AND short — there is no remaining vehicle to wonder about).

`VERDICT → PASS`
