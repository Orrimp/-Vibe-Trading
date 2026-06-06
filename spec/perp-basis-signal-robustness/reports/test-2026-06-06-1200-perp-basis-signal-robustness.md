---
title: Test Report
feature: perp-basis-signal-robustness
run_id: 2026-06-06-1200-UTC
commit: 38bd87b
agent: tester
verdict: PASS
---

# Test Report — perp-basis-signal-robustness — 2026-06-06 12:00 UTC

## 1. Scope

- **Feature / change under test:** Perp-spot basis-reversal signal arm (M-DEV-0..9 Pass 1 + Pass 2 + M-DEV-8 fee-sweep). Adds `ScoreSource::BasisReversal` to the cross-sectional momentum strategy (crates/strategy), a basis data loader mirroring `funding_data.rs` (crates/backtest/src/basis_data.rs), a `--taker-fee-bps` fee axis on the sweep binary, `SweepScoreSource::BasisReversal` + `BASIS_TIER1_GRID` + `load_basis_path_gen` wiring in `param_robustness_sweep.rs`, 6 day-1 falsifiers (crates/backtest/tests/basis_divergence_e2e.rs), the `perp-basis-signal-robustness` handler in `scripts/verify_anchors.sh`, and the 8 anchored θ×fee surfaces (2023+2024 × {0,2,5,10} bps).
- **Spec refs:** `spec/perp-basis-signal-robustness/feature.md`, `spec/perp-basis-signal-robustness/tasks.md`, `spec/anchors.toml` (anchors #100–#107)
- **Commit SHA:** `38bd87b` (HEAD — M-DEV-8 fee-sweep verdict commit)
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** Darwin 25.5.0 / arm64 (Apple Silicon M-series canonical box)

## 2. Static Analysis

| Check | Result | Notes |
|---|---|---|
| `cargo fmt --check` (backtest + strategy + data) | PASS | No format diffs on touched crates |
| `cargo clippy -p backtest -p strategy -p data --features "backtest/realdata backtest/candle" --bins --tests -- -D warnings` | PASS | Zero warnings. crates/ui excluded (pre-existing pedantic lints, out of scope) |
| `cargo build -p backtest --features "candle realdata" --bin param_robustness_sweep` | PASS | Compiled in 13.82s |
| `cargo audit` | NOT RUN (pre-existing baseline; no new dependencies added) | |
| spec-lint | `spec-lint: FAIL (94 violations in 2 categories)` — 87 dead-link + 7 trace-broken-path. **No new regressions vs prior tester baseline** (horizon-retest: 94 violations / 2 categories). The 8 `unreferenced-anchor` violations that appeared after locking anchors were resolved by filling the trace.toml `anchors`, `crates`, and `tests` fields for REQ-PERP-BASIS-SIGNAL-ROBUSTNESS-001. |

Pre-existing spec debt (carried forward, not new, does NOT block PASS):
- dead-link (87): ADR-0027 Kronos references, chart/cockpit /tmp/ artifacts, archive links. Same as prior baseline.
- trace-broken-path (7): REQ-LAB-YAHOO-REALDATA-V0-1-4-001, REQ-VISUAL-FAIL-HTML-REPORTER-001 (×3), REQ-UI-CONTRAST-ASSERTER-001, REQ-QUEUE-STALENESS-RECONCILIATION-001, REQ-OPERATOR-LEDGER-SCHEMA-LINT-001. Same as prior baseline.

## 3. Unit & Integration Tests

**Pre-flight stray check:** `git status --porcelain --untracked-files=all` before any verification — only `data/yahoo/REVISION.toml` modified (pre-existing stray, not ours). No rogue report files. Targeted test scope only (per polluter warning — `determinism.rs` tests use temp dirs and do NOT pollute spec/).

### Day-1 falsifiers — `cargo test -p backtest --features "candle realdata" --test basis_divergence_e2e`

| Test | Result | Description |
|---|---|---|
| `r_br_baseline_equity_divergence` | PASS | Basis arm equity diverges from un-tilted baseline by ≥ 1 bp (CLAUDE.md non-negotiable overlay-divergence gate). Universe: BBUSDT (very negative basis −0.015) selected by BasisReversal; AAUSDT (strong +5%/bar uptrend) selected by VolAdjustedReturn. K=1 guarantees selection divergence. |
| `r_br_baseline_divergence_red_on_revert` | PASS | Two identical-signal strategies (both VolAdjustedReturn, no basis) produce Δ=0, proving #1 would FAIL if basis were not load-bearing. |
| `r_br_basis_non_no_op` | PASS | Constant basis (zero cross-sectional dispersion) → two BasisReversal runs produce identical equity; real-basis run (disparate signal) produces different equity → basis is load-bearing, not decorative. |
| `r_br_sign_assertion_integration` | PASS | Correct-sign basis (BBUSDT negative, AAUSDT positive) selects flat BBUSDT; flipped-sign basis (inputs swapped) selects trending AAUSDT → different equity. Sign convention active at integration level. |
| `r_br_no_look_ahead_integration` | PASS | Future-shifted basis (+8 bars look-ahead) produces different equity from causal basis → as-of join is causal (basis_close[t-1] at open of t). |
| `basis_two_run_byte_identity` | PASS | Two sweeps at same ensemble_seed produce byte-identical formatted summaries (ADR-0051 § D6.9 — no non-determinism in basis co-resampling, ring buffer, or reduction order). |

**Full run result:** `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; finished in 0.02s`

### RED-on-revert proof for the load-bearing SIGN guard (R-BR.2)

**Method:** Mutated `Some(-mean)` → `Some(mean)` at `crates/strategy/src/cross_sectional/momentum.rs:360` (the one place the sign lives). Re-ran strategy-level unit tests:

```
cargo test -p strategy --lib "cross_sectional::momentum::tests::r_br2"
```

**Result:** Both r_br2 tests went RED with explicit guard messages:

- `r_br2_sign_assertion_longs_low_basis_name` FAILED: _"R-BR.2 SIGN VIOLATION: basis-reversal strategy with K=1 MUST select ETHUSDT (low/negative basis = reversal-favored leg) but got: ["BTCUSDT"]. This means basis_reversal_score returns +mean instead of −mean — the sign is FLIPPED and the strategy is a basis-MOMENTUM payer..."_
- `r_br2_basis_reversal_score_low_basis_outscores_high_basis` FAILED: _"R-BR.2 SIGN VIOLATION: basis_reversal_score(ETHUSDT, low_basis=-0.005)=-0.005 must be > basis_reversal_score(BTCUSDT, high_basis=0.02)=0.02. The sign `-mean` means the lowest-basis name has the highest score. If this fails, basis_reversal_score is returning +mean (the basis-MOMENTUM bug)."_

Sign restored immediately (`git diff crates/` empty after restore, verified). The integration test `r_br_sign_assertion_integration` uses input-data flip (correct vs flipped basis maps) and passed through the mutation — this is correct by design: the integration test proves sign-sensitivity via data, while the unit tests prove the code-level sign guard. Both layers guard the convention.

**Source state after revert:** `git diff crates/` produces no output — source tree is byte-identical to HEAD.

| Crate | Passed | Failed | Ignored | Duration |
|---|---:|---:|---:|---:|
| `backtest` (basis_divergence_e2e) | 6 | 0 | 0 | 0.02s |
| **Total** | **6** | **0** | **0** | |

## 4. Property / Fuzz Tests

_n/a_ — no proptest or cargo-fuzz suites in the changed crates for this feature. The `basis_two_run_byte_identity` falsifier serves as a determinism oracle.

## 5. Backtest Results — Fee-Sweep θ-Surface Verdicts (M-DEV-8)

**Universe:** 10 large-cap perpetuals (BTCUSDT, ETHUSDT, BNBUSDT, XRPUSDT, ADAUSDT, DOGEUSDT, SOLUSDT, DOTUSDT, LTCUSDT, LINKUSDT) — OHLCV pin `3a8b96c4…`, basis pin `aa72409a…`
**Periods:** 2023-FY (in-sample) and 2024-FY (gating/tail-negative regime)
**Generator:** block-bootstrap-real, shared-index mode, N=200 paths, block_length auto (L=204 for 2023, L=200 for 2024)
**Grid:** BASIS_TIER1_GRID, 6 cells LOCKED — lookback_bars ∈ {24, 60, 168}, k_long ∈ {1, 3, 5}, rebalance ∈ {480, 1440} min, drift=0.10
**Fee ladder:** taker_fee ∈ {0, 2, 5, 10} bps; slippage = 2bps throughout
**Decision rule:** frozen § 0 weakest-link composite (p5_sharpe < 0 → FRAGILE; all 5 PRIMARY signals evaluated)
**BH control:** p50 Sharpe +1.735 / prob_loss 4.5% / P(Sharpe>1) 77.5% / p95_maxdd 51.15% (2023-FY); p50 Sharpe +1.105 / prob_loss 16.5% / P(Sharpe>1) 53.5% / p95_maxdd 64.83% (2024-FY)

### Fee-sweep surface verdicts — ALL FAMILY-UNIFORM-FRAGILE

| Fee | Year | Best cell (g2, L=168) p50/p5 | Worst cell (g4, L=60,K=1) p50/p5 | Family verdict |
|---|---|---|---|---|
| 0 bps | 2023 | p50=+0.049, p5=−0.043 | p50=+0.020, p5=−0.231 | FAMILY-UNIFORM-FRAGILE |
| 0 bps | 2024 | p50=+0.051, p5=−0.010 | p50=+0.027, p5=−0.001 | FAMILY-UNIFORM-FRAGILE |
| 2 bps | 2023 | p50=+0.048, p5=−0.063 | p50=+0.020, p5=−0.231 | FAMILY-UNIFORM-FRAGILE |
| 2 bps | 2024 | p50=+0.050, p5=−0.010 | p50=+0.026, p5=−0.001 | FAMILY-UNIFORM-FRAGILE |
| 5 bps | 2023 | p50=+0.047, p5=−0.064 | p50=+0.019, p5=−0.231 | FAMILY-UNIFORM-FRAGILE |
| 5 bps | 2024 | p50=+0.049, p5=−0.011 | p50=+0.026, p5=−0.002 | FAMILY-UNIFORM-FRAGILE |
| 10 bps | 2023 | p50=+0.045, p5=−0.081 | p50=+0.019, p5=−0.232 | FAMILY-UNIFORM-FRAGILE |
| 10 bps | 2024 | p50=+0.047, p5=−0.015 | p50=+0.026, p5=−0.001 | FAMILY-UNIFORM-FRAGILE |

**Frozen § 0 verdict: FAMILY-UNIFORM-FRAGILE at every fee level including 0 bps gross.**

P(Sharpe>1) = 0.000 in every cell on every surface. The p50 Sharpe barely moves across the fee sweep (the fee ladder is largely irrelevant — fees are NOT the structural killer). The median active Sharpe (+0.047 to +0.051 at best) is 34×–37× below the BH control's +1.735 median (2023-FY). P(Sharpe>1) = 0.000 vs BH's 77.5%. Prob_loss ranges 6–35% vs BH's 4.5%.

### Key structural read (science verdict)

The **long-only** v0.1.0 basis-reversal arm captures only the long-low-basis leg of a reversal spread while carrying full market beta. The strategy is swamped by the BH control's +1.74 passive Sharpe bar. The long-short spread mechanism (long low-basis, short high-basis) is the natural vehicle for this signal — but that is a market-neutral v0.2.0 question deferred to the analyst.

**Program-level note:** Long-only basis-reversal FRAGILE even gross → the derivatives-positioning family closes on the long-only verdict. The open question is the market-neutral long/short v0.2.0 spread — analyst's call.

### Pre-flight void-if-fail headers

Each surface report contains `generator: block-bootstrap-real` and `bootstrap_mode: shared-index` in the ensemble parameters table. Confirmed present in all 8 reports.

### Regressions vs Baseline

No regression. The 99 pre-existing anchors are byte-identical (99/99 → 107/107 PASS). The basis work is strictly additive.

## 6. Benchmarks

_n/a_ — No hot-path changes in latency-sensitive paths. The fee axis is a parameter substitution (u32 comparison, zero algorithmic cost). The `basis_reversal_score` ring-buffer implementation mirrors `carry_score` (already benched). Wall-clock per-surface: 28–30s Apple-Silicon M-series (6 cells × N=200 paths, confirmed during M-DEV-8 run).

## 7. Anchor Gate

**Pre-anchor state:** `verify_anchors.sh` → 99/99 PASS (confirmed before any basis anchor locking).

**SHA-256 body hashes computed and locked:**

| Anchor # | Scenario | Body SHA-256 |
|---|---|---|
| 100 | v1-basis-reversal-fee00bps-theta-surface-2023-block-bootstrap-real-fy | `1cd5abbb5c4325a63e1c358c746f7e7b70481ab0429dbf78ad89a1296cebdc87` |
| 101 | v1-basis-reversal-fee00bps-theta-surface-2024-block-bootstrap-real-fy | `e8552e4557197caf42f7d479c7c55862eb3e8c99dcfd41f5fe5c09b0eaed6562` |
| 102 | v1-basis-reversal-fee02bps-theta-surface-2023-block-bootstrap-real-fy | `9ed405f54715fc97573c2f31c27c717d8d032dbc66ad4287eada727d2ca37d6f` |
| 103 | v1-basis-reversal-fee02bps-theta-surface-2024-block-bootstrap-real-fy | `87ba979816fc0b5dc63cede6ebf42cd979bf868faf901f8346dcde61d6a98b42` |
| 104 | v1-basis-reversal-fee05bps-theta-surface-2023-block-bootstrap-real-fy | `a29d9f954e8c698da7fa233f2302412fae8f9941c19711b933e24d755afc8b06` |
| 105 | v1-basis-reversal-fee05bps-theta-surface-2024-block-bootstrap-real-fy | `dfe12905e0f41d0275ad0f8672ad1f2eb59eddc5696747a93aa0304098018c97` |
| 106 | v1-basis-reversal-fee10bps-theta-surface-2023-block-bootstrap-real-fy | `c1ae40887dfcce853f55abeff3bf8de8a5d743258a3d0d6d7a46f69dc289c3ef` |
| 107 | v1-basis-reversal-fee10bps-theta-surface-2024-block-bootstrap-real-fy | `ab4b53a90ed5d9d1533b9445dcbeb521cae27fd260efa55d016556cacafb847d` |

**Post-lock verification:** `verify_anchors.sh` → 107/107 PASS. Transition: 99 → 107.

## 8. Environment / Infrastructure Issues

_none_ — Clean run. Only pre-existing stray `data/yahoo/REVISION.toml` in working tree (not ours). No rogue report files generated.

## 9. Tasks Ticked

Tasks verified on disk and ticked in `spec/perp-basis-signal-robustness/tasks.md`:

- **M-DEV-0** [x] (pre-existing, confirmed 99/99 pre-flight)
- **M-DEV-1** [x] (pre-existing, basis_data.rs confirmed on disk at crates/backtest/src/basis_data.rs)
- **M-DEV-2** [x] (pre-existing, basis_as_of + build_basis_at_return confirmed)
- **M-DEV-3** [x] (pre-existing, ScoreSource::BasisReversal + sign + falsifiers confirmed)
- **M-DEV-4** [x] (TICKED by tester — --taker-fee-bps/--slippage-bps flags confirmed in param_robustness_sweep.rs at line ~1380/1388)
- **M-DEV-5** [x] (TICKED by tester — SweepScoreSource::BasisReversal + BASIS_TIER1_GRID + GridKind::BasisTier1 + load_basis_path_gen all confirmed)
- **M-DEV-6** [x] (TICKED by tester — basis_divergence_e2e.rs exists, all 6 falsifiers GREEN)
- **M-DEV-7** [x] (TICKED by tester — basis_two_run_byte_identity GREEN, two-run identity confirmed)
- **M-DEV-8** [x] (TICKED by tester — 8 surface reports confirmed on disk, all FAMILY-UNIFORM-FRAGILE)
- **M-DEV-9** [x] (TICKED by tester — perp-basis-signal-robustness elif handler confirmed in verify_anchors.sh at line ~170)
- **M-TEST** [x] (TICKED by tester — this report)

## 10. Verdict

**`PASS`**

The implementation is sound. All 6 day-1 falsifiers are GREEN. The load-bearing sign guard (`Some(-mean)` at momentum.rs:360) is confirmed RED-on-revert at the strategy-unit level (both `r_br2_*` tests panic with exact diagnostic messages on the sign flip). The 8 anchored θ×fee surfaces are locked cleanly (verify_anchors.sh 107/107 PASS; transition 99→107 with all 99 pre-existing anchors byte-identical). Clippy and fmt pass on all touched crates. Source tree byte-identical to HEAD after the sign-revert probe (`git diff crates/` empty). Spec-lint at 94 violations / 2 categories — no new regressions vs baseline.

The science result — FAMILY-UNIFORM-FRAGILE at ALL fee levels including 0 bps gross — is the expected, pre-registered decision-grade outcome (R-BR.LOAD). A robust implementation that correctly reports a fragile signal is a PASS. The long-only vehicle fails the BH control bar by 34×–37× on median Sharpe; P(Sharpe>1) = 0.000 throughout.

## 11. Routing

`VERDICT → PASS` — ready for presenter / analyst handoff.

Analyst open item: the market-neutral long/short v0.2.0 spread (long low-basis, short high-basis) is the natural next question given the raw IC signal (−0.08 to −0.11) is real. The long-only arm is the structural mismatch, not the signal. This is the analyst's call per program routing.
