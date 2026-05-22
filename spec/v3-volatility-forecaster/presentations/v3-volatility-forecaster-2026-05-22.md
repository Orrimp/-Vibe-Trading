---
slug: v3-volatility-forecaster
mode: release
status: draft
audience: human-operator
updated: 2026-05-22
generated: 2026-05-22T00:00:00Z
version: 0.1.0
commit: 625fb336e7faeb96aa1040343dab34e02115b72f
predecessor: v25a-patchtst-overlay v0.1.0 (RETIRED — joint F4/F4/F4)
parent: (none — first ship in post-v2.5 strategy reformulation; C1 of hybrid C1+C2+C5)
---

# v3 volatility forecaster — release deck

## Operator headline

The C1 vol-forecaster shipped clean code on first try: 33/33 anchors, 992
tests, 0 failures, GARCH(1,1) MLE in ~120 LoC of pure Rust, no new external
dependency. **But the joint advisory verdict is `V3 × T-VOL-NO-ALPHA →
MODEL-BROKEN / NO-ALPHA`**: GARCH systematically under-predicts realized
volatility by ~3× across the universe (mean_calibration_ratio = 2.952191,
outside the [0.7, 1.4] envelope) — driven by non-convergence on AVAX / DOGE
/ DOT at the 500-iteration ceiling — and the vol-target overlay delivers
only +0.029868 net Sharpe-delta vs the v1 momentum baseline, below the
+0.05 T-VOL-MARGINAL floor. **One load-bearing caveat**: that Sharpe
comparison is **synthetic-vs-real** — the v1 baseline runs on synthetic GBM
bars, the overlay on real Binance hourly OHLCV. The NO-ALPHA reading may
be a data-mismatch artifact.

**The operator decides one of (a)/(b)/(c)/(d) below.** Presenter's
provisional recommendation is **(b) RE-BASELINE FIRST** — ~1 day to
disambiguate the data-mismatch caveat before committing multi-week budget
to retirement (a), debug (c), or refit (d).

## Operator routing options (the deck's primary ask)

**One decision. Variable budget implications. Standing "Autoapprove all"
applies to MVP defaults; this is a strategic budget call and the
presenter explicitly does NOT autoapprove.**

| Path | Action | Budget | When to pick |
|------|--------|--------|--------------|
| **(a) RETIRE C1** | Accept joint verdict at face value. Free C1 budget. Promote C2 (regime-classifier) or C5 (LLM-forecaster) from Queue → Active per HYBRID sequencing. | 0 (frees ~3-4 weeks for C2/C5) | If you believe the synthetic-vs-real caveat is small and the V3 + NO-ALPHA verdicts hold under real-vs-real. Mirrors the v2.5 DL retire pattern (see `spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`). |
| **(b) RE-BASELINE FIRST** ⟵ **presenter-recommended** | Re-run the sharpe comparison with the REAL v1 baseline (un-targeted real-data momentum). No retrain; one bin invocation + ~40s backtest + sharpe-comparison re-run. | ~1 day | When the largest single source of uncertainty in the joint verdict is the data caveat. Cheap, decides between (a) and (c)/(d) on harder evidence. |
| **(c) DEBUG V3** | Spawn `v3-garch-calibration-tune` for per-symbol hyperparameter search (ω, α, β ranges; max_iters > 500; tighter convergence tol; or Garman-Klass fallback for non-convergent symbols). | ~2-3 weeks | If V3 is the dominant signal and you want the model fixed before deciding on alpha. ADR-0038 has the routing slot pre-allocated. |
| **(d) v0.1.1 GARCH refit + return** | Same fitter; iterate hyperparameters in place. Keep the v0.1.0 workspace structure; bump version → v0.1.1. | ~2-3 days | If you want a quick refit attempt scoped tight enough to ship-or-skip on a single iteration. |

### Decision tree

```
Operator-decide:
  └── confident V3 + NO-ALPHA hold under real-vs-real baseline? 
       ├── YES → (a) RETIRE C1 → promote C2 or C5
       └── UNCERTAIN → (b) RE-BASELINE FIRST (~1 day)
                       ├── re-baseline still T-VOL-NO-ALPHA + V3 → (a)
                       ├── re-baseline flips to T-VOL-MARGINAL → (d) v0.1.1 refit
                       └── re-baseline flips to T-VOL-ALPHA-UNLOCKED → (c) DEBUG V3 (V3 still fires; fix calibration before banking alpha)
```

### Presenter recommendation: (b)

The synthetic-vs-real data mismatch is the **largest single source of
uncertainty** in the joint advisory verdict. Spending ~1 day on (b) is
the highest-EV play before committing multi-week budget to either (c)
multi-week debug or (a) programme-retirement. If (b) confirms NO-ALPHA
on a real-vs-real baseline, proceed to (a) with full confidence and
free ~3-4 weeks for C2 or C5. If (b) flips the T-classifier, the
routing changes accordingly per the tree above.

## Evidence summary

Three anchored body-SHA-256 reports, one-liner each:

- **Vol-verdict** —
  [`reports/vol-verdict-bs1-realdata-20260522.md`](../reports/vol-verdict-bs1-realdata-20260522.md)
  body-SHA `99c21892…`. Per-symbol QLIKE + calibration ratios across
  10 symbols. 7 calibrate cleanly within [0.96, 1.01]; **3 catastrophic
  overflowers** drive the V3 trigger:
  - DOGEUSDT — calib_ratio **10.247541** (severe overflow; GARCH
    non-convergence at 500 iters)
  - DOTUSDT — calib_ratio **10.096677** (severe overflow; same root)
  - AVAXUSDT — calib_ratio **2.307620** (overflow; same root)
- **Backtest** —
  [`reports/backtest-20260522-082914-top10-2023-fy-vol-target-overlay-realdata.md`](../reports/backtest-20260522-082914-top10-2023-fy-vol-target-overlay-realdata.md)
  body-SHA `66cd69ad…`. Vol-target overlay (real Binance 2023):
  **+13.48% total return** vs v1 baseline's **-43.72%**; max drawdown
  **73.73%** (vs 87.48%); **6203 trades** (+1394 vs baseline); $17.4k
  fees on $100k notional; dampen rate 100% (every trade scaled by
  GARCH σ̂).
- **Sharpe-comparison** —
  [`reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md`](../reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md)
  body-SHA `ef048366…`. Sharpe baseline **-0.026770** (synthetic GBM)
  vs overlay **0.003098** (real Binance); **net delta +0.029868** <
  +0.05 → **T-VOL-NO-ALPHA**. Data caveat embedded verbatim — see
  next subsection.

### Data caveat (synthetic-vs-real)

From the sharpe-comparison report body, verbatim within fair-use:

> "Baseline uses synthetic GBM bars; overlay uses real Binance 2023
> data."

The baseline (top10-2023-1h-momentum) ran the un-targeted v1
cross-sectional momentum strategy through the **passthrough forecaster
on synthetic Geometric Brownian Motion bars** (no `candle` dependency,
no real Binance fixture). The overlay (top10-2023-fy-vol-target-overlay-realdata)
ran the GARCH vol-targeting overlay on **real Binance hourly OHLCV**
(`data_revision_sha: 3a8b96c4…`). The net Sharpe-delta of +0.029868
therefore **conflates two effects**: (1) the vol-targeting overlay's
true lift, and (2) the structural Sharpe difference between synthetic
GBM and real Binance returns. **Routing (b) above resolves this
ambiguity at ~1-day cost.**

### Why V3 fires — the GARCH-non-convergence story

The hand-rolled L-BFGS MLE in `crates/forecast/src/garch.rs` hits its
500-iteration ceiling for 3 of the 10 universe symbols (AVAX / DOGE /
DOT). At the ceiling, `unconditional_var = ω / (1 − α − β)` blows up
because α + β ≈ 1 (very-near-non-stationary fit), driving σ̂ overflow
and the calibration ratio off the chart for those three symbols. The
remaining 7 symbols (ADA / BNB / BTC / ETH / LINK / SOL / XRP)
calibrate inside [0.96, 1.01] — **GARCH is working correctly for 70%
of the universe**; the V3 verdict is dominated by a tight failure
cluster, not a systemic GARCH break.

This is the load-bearing detail for routing (c) / (d): per-symbol
hyperparameter search (or a Garman-Klass fallback) on AVAX / DOGE /
DOT is a tractable, scoped fix. The model is not categorically broken;
3 of 10 symbols are mis-calibrated under the analyst-recommended
default fitter config.

## What landed

- **Hand-rolled GARCH(1,1) MLE** at
  [`crates/forecast/src/garch.rs`](../../../crates/forecast/src/garch.rs)
  (~120 LoC) — closed-form likelihood + L-BFGS optimiser + stationarity
  constraint α+β<1. `rust-quant v0.0.10` rejected per ADR-0038 § D3
  (4 reasons: CLAUDE.md library-compat checklist, API fit,
  maintained-status, determinism contract). **Zero new external
  dependency.** Hyperparameter lock: ω init=1e-6; α init=0.10; β
  init=0.85; convergence tol=1e-8; max_iters=500.
- **`VolForecastProvider` trait + `vol.rs` types** at
  [`crates/forecast/src/vol.rs`](../../../crates/forecast/src/vol.rs)
  (~80 LoC) — async trait sibling to `ForecastProvider` per ADR-0038
  § D1.a. `ForecastProvider` (direction-target) **byte-identical** —
  K-vol-3 guard via `tcn_byte_identity` + `patchtst_byte_identity`
  tests.
- **GARCH training bin** at
  [`crates/forecast/src/bin/train_garch.rs`](../../../crates/forecast/src/bin/train_garch.rs)
  — per-symbol MLE driver; emits
  `crates/forecast/checkpoints/anchors/garch-bs1-<sha>.json`.
  Wall-clock for 10 symbols × ~8760 hourly bars: **5-10 seconds**.
- **Parkinson target derivation** at
  [`crates/forecast/src/features.rs:642-656`](../../../crates/forecast/src/features.rs)
  — additive `VolTargetKind` enum + `target_parkinson_vol: Option<f32>`
  scalar per ADR-0038 § D3. Single-horizon scalar (24-bar default).
  Existing TCN/PatchTST callers pass `vol_target_kind: None` —
  iteration order and `target_logret` byte-identical (R11.7 / R11.8
  guard).
- **V-verdict bin** at
  [`crates/forecast/src/bin/vol_verdict.rs`](../../../crates/forecast/src/bin/vol_verdict.rs)
  (~280 LoC) — sibling of `forecast_distribution.rs`; emits the
  `vol-verdict-bs1-realdata` report under
  `spec/v3-volatility-forecaster/reports/`. Read-only contract.
- **3 strategy builders** (Q3=(d) all-3) —
  [`crates/strategy/src/vol_targeting_overlay.rs`](../../../crates/strategy/src/vol_targeting_overlay.rs)
  (R6.a primary; ~500 LoC w/ tests),
  [`crates/strategy/src/vol_killswitch_overlay.rs`](../../../crates/strategy/src/vol_killswitch_overlay.rs)
  (R6.b; ~330 LoC),
  [`crates/strategy/src/vol_meanreversion.rs`](../../../crates/strategy/src/vol_meanreversion.rs)
  (R6.c; ~265 LoC). 3 new builder fns in
  [`crates/strategy/src/lib.rs:53-111`](../../../crates/strategy/src/lib.rs).
  **8 existing strategy builders byte-identical.**
- **Backtest scenario** at
  [`crates/backtest/src/scenarios/garch_vol_target_overlay.rs`](../../../crates/backtest/src/scenarios/garch_vol_target_overlay.rs)
  + `GarchVolTargetOverlayMomentum` variant in
  [`crates/backtest/src/main.rs`](../../../crates/backtest/src/main.rs).
  Strategy config
  [`crates/strategy/config/vol_target_overlay_momentum.toml`](../../../crates/strategy/config/vol_target_overlay_momentum.toml)
  pinning `target_vol=0.02`, `scale_clamp=[0.5, 2.0]`,
  `momentum_config_id="top10_momentum"`.
- **`sharpe_comparison` extended** at
  [`crates/forecast/src/bin/sharpe_comparison.rs`](../../../crates/forecast/src/bin/sharpe_comparison.rs)
  — additive `ScenarioFamily` enum (Tcn/VolTarget) + `--scenario
  vol-target-bs1` dispatch arm + `render_vol_target` T-classifier
  logic. Existing TCN/PatchTST anchored output byte-identical.
- **`replay-cache` extended** at
  [`crates/replay-cache/src/lib.rs`](../../../crates/replay-cache/src/lib.rs)
  — additive `CacheNamespace::VolForecast` variant. Existing
  `Forecast` namespace byte-identical.
- **ADR-0038** at
  [`spec/architecture/adr/0038-vol-forecast-verdict-shape.md`](../../architecture/adr/0038-vol-forecast-verdict-shape.md)
  (status: accepted) — V1-V5 priority tree + V_ALPHA strategy-gate
  sibling; T-classifier thresholds (T-VOL-ALPHA-UNLOCKED / -MARGINAL /
  -NO-ALPHA); GARCH(1,1) JSON checkpoint schema; anchor + version
  naming `v3.0.0-volatility`. **ADR-0033 (F-verdict) stays IMMUTABLE**
  per retrospective lesson #2.
- **Trace row** `REQ-V3-VOL-FORECASTER-001` opened (analyst T-A5),
  carried through `proposed → in-progress → shipped` states.

**File scope** (per `decomp.md § 2`): **14 new files, 8 modified
files**. All modifications are strictly additive (enum variants,
trait impls, dispatch arms); zero refactor of pre-existing code.

## What you can do now

| Action | Command |
|--------|---------|
| Re-verify all 33 anchors (30 originals + 3 new vol-forecaster rows) | `bash scripts/verify_anchors.sh` |
| Re-run the V-verdict report (GARCH per-symbol QLIKE) | `cargo run -p forecast --release --features candle --bin vol_verdict -- --scenario bs1` |
| Re-run the vol-target backtest (real Binance 2023 hourly) | `cargo run -p backtest --release --features candle,realdata --bin backtest -- --scenario top10-2023-fy-vol-target-overlay-realdata --seed 0xC0FFEE` |
| Re-run the sharpe-comparison (T-classifier) | `cargo run -p forecast --release --features candle --bin sharpe_comparison -- --scenario vol-target-bs1` |
| **Routing (b) — re-baseline against REAL v1 momentum** | `cargo run -p backtest --release --features candle,realdata --bin backtest -- --scenario top10-2023-fy-momentum-realdata --seed 0xC0FFEE` then re-run sharpe-comparison against the real-data baseline (architect spec on follow-up) |
| Adopt vol-targeting overlay (advisory; additive builder) | `MomentumStrategy::with_garch_vol_overlay_momentum(base, ledger)` in the trading host |
| Approve + pick routing | tick a box below; orchestrator opens the picked path |

## Live demo — anchor gate 33 / 33 (verbatim tail)

```
$ bash scripts/verify_anchors.sh 2>&1 | tail -5
PASS  vol-verdict-bs1-realdata              99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21
PASS  top10-2023-fy-vol-target-overlay-realdata  66cd69ad03294cccf514184968babce0127f2ebfa4d1f4a03b332f8000f79c65
PASS  sharpe-comparison-vol-target-bs1-realdata  ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1
---
ANCHORS PASS  (33 / 33)
```

**30 originals byte-identical** (R10 non-regression contract held);
**3 new** under `[v3.0.0-volatility]` locked by tester T-T2.

## Verification matrix (R11 gates)

| V-id | Gate | Status | Evidence |
|------|------|--------|----------|
| V-R11.1 | `cargo fmt --check` clean | VERIFIED | No output, exit 0 (orchestrator pre-cleared 71-hunk fmt drift before re-gate). |
| V-R11.2 | `cargo clippy --workspace --features candle -- -D warnings` | VERIFIED | `Finished dev profile … in 9.80s`; 0 errors / 0 warnings. |
| V-R11.3 | `cargo test --workspace --lib --features candle` | VERIFIED | **992 passed / 0 failed / 2 ignored** across 17 suites. |
| V-R11.4 | `garch_fit_determinism` (2-run byte-identity) | VERIFIED | `test result: ok. 1 passed`. |
| V-R11.5 | `vol_verdict_mutual_exclusivity` (V1-V5 priority tree) | VERIFIED | All V-label fixtures + property test PASS. |
| V-R11.6 | `vol_targeting_overlay` (overlay wrap correctness) | VERIFIED | 8 passed; scale clamp + zero-sigma defensive guard. |
| V-R11.7 | `tcn_byte_identity` (K-vol-3 scope-creep guard) | VERIFIED | `git diff HEAD -- crates/forecast/src/tcn.rs` empty. |
| V-R11.8 | `patchtst_byte_identity` (K-vol-3 scope-creep guard) | VERIFIED | `git diff HEAD -- crates/forecast/src/patchtst.rs` empty. |
| V-R11.9 | 2-run byte-identity — `vol-verdict-bs1-realdata` | VERIFIED | body-SHA `99c21892…` matches across runs. |
| V-R11.10 | 2-run byte-identity — `top10-2023-fy-vol-target-overlay-realdata` | VERIFIED | body-SHA `66cd69ad…` matches across `082901`/`082914` files. |
| V-R11.11 | `verify_anchors.sh` 30 PRE / 33 POST | VERIFIED | `ANCHORS PASS (33 / 33)` (see live demo). |
| V-R11.12 | Joint advisory verdict recorded in `feature.md § Verification` | VERIFIED | V3 × T-VOL-NO-ALPHA → MODEL-BROKEN / NO-ALPHA + data caveat. |
| V-R10 | 30 pre-existing anchors byte-identical | VERIFIED | All 30 SHAs match pre-feature values (R10 contract). |

## Numbers that matter

- **V-verdict** — **V3** (mean_calibration_ratio = 2.952191; outside
  [0.7, 1.4]).
- **T-classifier** — **T-VOL-NO-ALPHA** (net Sharpe-delta = 0.029868
  < +0.05 floor).
- **Joint advisory** — **MODEL-BROKEN / NO-ALPHA** (per ADR-0038
  § D1.c: any V1/V2/V3 collapses to MODEL-BROKEN regardless of T).
- **Per-symbol calibration spread** — 7 symbols inside [0.96, 1.01];
  3 overflowers (AVAX 2.31, DOT 10.10, DOGE 10.25).
- **Sharpe-delta** — **+0.029868** (overlay 0.003098 − baseline
  −0.026770). Below T-VOL-MARGINAL floor (+0.05) by **−0.020**;
  below T-VOL-ALPHA-UNLOCKED floor (+0.10) by **−0.070**.
- **Backtest total return** — **+13.48%** (overlay) vs **−43.72%**
  (baseline, bankrupt at $0). Note: total-return delta is dominated by
  the synthetic-vs-real data mismatch, not by overlay lift.
- **Max drawdown** — **73.73%** (overlay) vs **87.48%** (baseline).
- **Trades / fees** — 6203 trades / $17.4k fees on $100k notional
  (overlay).
- **GARCH wall-clock** — **5-10 seconds** for 10 symbols × ~8760 bars
  (T-AR-9). Backtest itself **~40 s** (longest step in v0.1.0).
- **Tests** — **992 passed / 0 failed / 2 ignored** across 17 suites.
- **Anchors** — 30 → **33** (3 new under `v3.0.0-volatility`):
  - `vol-verdict-bs1-realdata` → `99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21`
  - `top10-2023-fy-vol-target-overlay-realdata` → `66cd69ad03294cccf514184968babce0127f2ebfa4d1f4a03b332f8000f79c65`
  - `sharpe-comparison-vol-target-bs1-realdata` → `ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1`
- **Code scope** — 14 new files / 8 modified files; **zero new
  external dependency**.
- **GARCH MLE LoC** — ~120 (hand-rolled; rust-quant v0.0.10 rejected).
- **Cheap-first dividend** — Q2=(a) GARCH-only-MVP shipped <1 day
  Wave A→E vs analyst's 3-4 week best-case estimate (per
  `tasks.md § Notes`).

## Open decisions

**One load-bearing decision** — pick **(a) / (b) / (c) / (d)** from
the routing-options table above. Costs by branch:

- **(a) RETIRE C1** → 0 wall-clock; frees ~3-4 weeks for whichever
  of C2 (regime-classifier) or C5 (LLM-forecaster) you promote from
  Queue § Strategy.
- **(b) RE-BASELINE FIRST** ⟵ presenter recommendation → ~1 day; one
  bin invocation + ~40s backtest + sharpe-comparison re-run on the
  REAL v1 momentum baseline. Decides between (a) / (c) / (d) on
  harder evidence. Standing "Autoapprove all" can accelerate (b) to
  same-day execution by the orchestrator.
- **(c) DEBUG V3** → ~2-3 weeks; spawn `v3-garch-calibration-tune`
  feature; per-symbol hyperparameter search. ADR-0038 routing slot
  pre-allocated.
- **(d) v0.1.1 GARCH refit + return** → ~2-3 days; in-place
  iteration on the existing fitter; bump version → v0.1.1.

**Secondary decision if (a)**: choose C2 or C5 promotion candidate.
Both have analyst-only spec drafts ready 2026-05-22 (see
`spec/v3-regime-classifier/feature.md` and
`spec/v3-llm-forecaster/feature.md`). HYBRID-sequencing budget caps
at ~3-4 weeks remaining post-C1.

## Hypothesis register status

| H | Statement | Status | Evidence |
|---|-----------|--------|----------|
| **H1** | DL beats GARCH ≥5% QLIKE on the vol-forecast target. | **DEFERRED** | Q2=(a) GARCH-only-MVP shipped without DL refinement. Open until v0.1.1 / v3.x if operator routes to (c) or (d). |
| **H2** | Vol-targeting Sharpe-delta ≥ +0.10 vs un-targeted baseline. | **FALSIFIED (with caveat)** | Net delta = +0.029868 < +0.10. Caveat: synthetic-vs-real baseline mismatch may artifact the reading. Routing (b) disambiguates. |
| **H3** | 3-4 week cheap-first ship feasibility. | **CONFIRMED — substantially under-budget** | Wave A→E completed <1 day end-to-end (T-AR-9 wall-clock prediction held). |
| **H4** | Hourly crypto vol IS predictable (vs constant-vol benchmark). | **PARTIALLY CONFIRMED** | 8 of 10 symbols improve ≥10% QLIKE vs constant-vol baseline (qlike_garch_mean 0.612 vs qlike_constant_mean 5.529). H4 holds for 7 well-calibrated symbols; fails for AVAX/DOGE/DOT under the current fitter config. |

H2's caveated falsification is the load-bearing reason routing (b)
makes sense before committing to (a) retire.

## Deferred / out of scope (v0.1.1+)

- **DL refinement** (TCN / LSTM / PatchTST vol-targeted variants) —
  deferred per Q2=(a). Routes to v0.1.1 only if (b) re-baseline
  flips T-classifier to T-VOL-MARGINAL or T-VOL-ALPHA-UNLOCKED **and**
  V3 is fixed by (c)/(d).
- **Risk-engine integration** (Q3=(c)) — deferred to v0.1.1 per
  ADR-0038 § D5. v0.1.0 ships strategy-side composition only.
- **Kill-switch backtest scenario** — scenario implemented and
  unit-tested but no anchor at v0.1.0 (Q-anchors-sub=3). Deferrable
  to v0.1.1 once the body-determinism story is settled.
- **Garman-Klass realized-vol estimator** — Q1=(b) Parkinson selected
  at MVP; Garman-Klass is a routing-(c) fallback for non-convergent
  symbols.
- **`vol_meanreversion` strategy** (R6.c) — landed as a tertiary
  builder + unit tests; not backtested in v0.1.0 (out of scope per
  Q3=(d) primary-anchor target = vol-target overlay).
- **BS-2 (2024) validation span** — out of scope at v0.1.0 (Q6=(a)
  BS-1 train only). Open for v0.1.1 sanity if (d) routes there.

## Rollback

This feature is **additive only**.

| Wave | Rollback action | Cost |
|------|-----------------|------|
| A (`garch.rs` + `vol.rs` + `train_garch.rs` + 8 tests) | `git revert <wave-A-shas>`. K-vol-3 holds throughout; existing TCN/PatchTST untouched. | ~1 minute |
| B (`features.rs:642-656` additive Parkinson block) | `git revert <wave-B-shas>`. Existing callers pass `vol_target_kind: None` and stay byte-identical. | ~30 seconds |
| C (`vol_verdict.rs` bin + report) | `git revert <wave-C-shas>` + `rm` 1 report artifact. | ~1 minute |
| D (3 strategy builders + backtest scenario + sharpe-comparison extension + 2 reports) | `git revert <wave-D-shas>` + `rm` the 2 report artifacts. | ~3 minutes |
| Anchor lock | revert the 3 new rows at `spec/anchors.toml [v3.0.0-volatility]`. 30 originals stay byte-identical. | ~1 minute |
| ADR-0038 | `git revert` the ADR commit. | ~30 seconds |
| Full feature | `git revert` the wave commits + `rm` 3 report artifacts + 1 GARCH checkpoint. | ~10 minutes total |

The non-negotiable safety net: **30 original anchors byte-identical**
(R10 confirmed via V-R11.11). **`tcn.rs` / `patchtst.rs` byte-identical**
(V-R11.7 / V-R11.8 confirmed). **8 existing strategy builders
byte-identical** (Wave D additive). Rollback never touches a locked
artifact.

## Closing gates

Both mechanical gates run on this presentation file:

```
$ bash scripts/check_presentation.sh spec/v3-volatility-forecaster/presentations/v3-volatility-forecaster-2026-05-22.md
<PASS line embedded in the handoff envelope below>
```

```
$ python3.14 scripts/spec_lint.py 2>&1 | head -1
spec-lint: FAIL (85 violations in 1 categories)
```

**Baseline match: 85 / 1**, identical to the tester's recorded post-T-T2
baseline (`reports/test-final-2026-05-22.md § 8`). **No new categories
or count growth introduced by this presentation file.** The +3 dead-link
delta vs `audit-2026-05-22.md` (82 → 85) was already present at the
tester PASS gate (developer ADR-0038 self-ref + Wave C vol_verdict
report link; analyst v3-llm-forecaster spec-only links) — pre-existing
artifact debt, not a presenter-introduced regression.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Routing pick (operator selects exactly one)

- [ ] (a) RETIRE C1 — accept joint verdict; promote C2 _or_ C5 from
  Queue → Active (note which one below)
- [ ] (b) RE-BASELINE FIRST — re-run sharpe-comparison vs REAL v1
  momentum baseline before further routing (presenter-recommended)
- [ ] (c) DEBUG V3 — spawn `v3-garch-calibration-tune` for per-symbol
  hyperparameter search
- [ ] (d) v0.1.1 GARCH refit + return — in-place fitter iteration;
  bump version → v0.1.1

### Notes / feedback

_(operator fills in routing choice details, C2-vs-C5 pick if (a), or
rejection reason if rejected)_

## Sources cited

- [`feature.md`](../feature.md) — feature brief v0.1.0; R1-R12 + H1-H4
  + K-vol-1..6 + Q1-Q6 + § Verification (V3 + T-VOL-NO-ALPHA + data
  caveat + routing implications).
- [`tasks.md`](../tasks.md) — T-A1..T-A7 + T-OD1..T-OD-Q-anchors-sub
  + T-AR-1..T-AR-10 + T-D-N1..T-D-N28 + T-T1..T-T3 + T-P1
  (this row).
- [`decomp.md`](../decomp.md) — architect M-T1 decomposition (Waves
  A / B / C / D / E).
- [`reports/test-final-2026-05-22.md`](../reports/test-final-2026-05-22.md)
  — tester M-FINAL re-gate `VERDICT → PASS`; 992/0/2 tests; 33/33
  anchors; spec-lint 85/1 (no new categories).
- [`reports/vol-verdict-bs1-realdata-20260522.md`](../reports/vol-verdict-bs1-realdata-20260522.md)
  — V3 evidence + per-symbol QLIKE + calibration table (body-SHA
  `99c21892…`).
- [`reports/backtest-20260522-082914-top10-2023-fy-vol-target-overlay-realdata.md`](../reports/backtest-20260522-082914-top10-2023-fy-vol-target-overlay-realdata.md)
  — vol-target overlay backtest (body-SHA `66cd69ad…`).
- [`reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md`](../reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md)
  — Sharpe-delta + T-classifier + synthetic-vs-real caveat (body-SHA
  `ef048366…`).
- [ADR-0038](../../architecture/adr/0038-vol-forecast-verdict-shape.md)
  — V-verdict shape + T-classifier thresholds + GARCH(1,1) checkpoint
  schema (this feature's NEW ADR; ADR-0033 stays IMMUTABLE per
  retrospective lesson #2).
- [Predecessor presenter deck (v25a-patchtst-overlay 2026-05-22)](../../v25a-patchtst-overlay/presentations/v25a-patchtst-overlay-2026-05-22.md)
  — style template + retire-pattern reference for routing (a).
- [`spec/dev-notes/strategy-reformulation-survey-2026-05-22.md`](../../dev-notes/strategy-reformulation-survey-2026-05-22.md)
  § Candidate 1 / 2 / 5 — C1 (this feature) / C2 (regime-classifier) /
  C5 (LLM-forecaster) survey-time EV ranking; load-bearing for the (a)
  promotion decision.
- [`spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`](../../dev-notes/v25-dl-journey-retrospective-2026-05-22.md)
  — v2.5 DL programme retirement pattern; the precedent for (a) "we
  just did this once already".
- `spec/anchors.toml [v3.0.0-volatility]` — 3 new anchor rows.
- `spec/trace.toml` — `REQ-V3-VOL-FORECASTER-001` carried through
  `proposed → in-progress → shipped`.
- Code sites:
  - [`crates/forecast/src/garch.rs`](../../../crates/forecast/src/garch.rs) — GARCH(1,1) MLE (~120 LoC).
  - [`crates/forecast/src/vol.rs`](../../../crates/forecast/src/vol.rs) — `VolForecastProvider` trait (~80 LoC).
  - [`crates/forecast/src/bin/vol_verdict.rs`](../../../crates/forecast/src/bin/vol_verdict.rs) — V-verdict bin (~280 LoC).
  - [`crates/forecast/src/bin/train_garch.rs`](../../../crates/forecast/src/bin/train_garch.rs) — per-symbol MLE driver.
  - [`crates/forecast/src/bin/sharpe_comparison.rs`](../../../crates/forecast/src/bin/sharpe_comparison.rs) — T-classifier dispatch.
  - [`crates/forecast/src/features.rs:642-656`](../../../crates/forecast/src/features.rs) — Parkinson target derivation.
  - [`crates/strategy/src/vol_targeting_overlay.rs`](../../../crates/strategy/src/vol_targeting_overlay.rs) — R6.a primary builder (~500 LoC w/ tests).
  - [`crates/strategy/src/vol_killswitch_overlay.rs`](../../../crates/strategy/src/vol_killswitch_overlay.rs) — R6.b secondary (~330 LoC).
  - [`crates/strategy/src/vol_meanreversion.rs`](../../../crates/strategy/src/vol_meanreversion.rs) — R6.c tertiary (~265 LoC).
  - [`crates/backtest/src/scenarios/garch_vol_target_overlay.rs`](../../../crates/backtest/src/scenarios/garch_vol_target_overlay.rs) — backtest scenario.

## Changelog

- 2026-05-22 (presenter): release deck. v3 volatility forecaster
  v0.1.0 shipped clean code on first try (Wave A→E < 1 day vs
  3-4 week analyst estimate); 992 tests / 0 failures / 33 anchors
  PASS / 0 new spec-lint categories. **Joint advisory verdict
  V3 × T-VOL-NO-ALPHA → MODEL-BROKEN / NO-ALPHA** recorded per
  ADR-0038 § D1.b + § D1.c. V3 driven by GARCH non-convergence on
  AVAX / DOGE / DOT (calib_ratio 2.3 / 10.1 / 10.2 vs envelope
  [0.7, 1.4]); 7 of 10 symbols calibrate cleanly. T-VOL-NO-ALPHA
  confounded by synthetic-vs-real baseline mismatch — load-bearing
  caveat for routing (b). 4-way operator-decide routing surfaced:
  (a) RETIRE C1, (b) RE-BASELINE FIRST [recommended], (c) DEBUG V3,
  (d) v0.1.1 GARCH refit. Mechanical pre-tick + spec-lint gates
  passed at baseline 85 / 1 (no new categories or count growth
  introduced).
