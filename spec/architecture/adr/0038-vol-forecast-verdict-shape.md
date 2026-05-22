---
adr: 0038
title: v3 — Vol-forecast V-verdict report shape + GARCH(1,1) baseline contract (parallel to ADR-0033, not extension)
status: accepted
date: 2026-05-22
supersedes: none
superseded-by: none
---

# ADR-0038: v3 vol-forecast V-verdict report shape & GARCH(1,1) baseline contract

## Context

[ADR-0033](0033-tcn-alpha-investigation-report-shape.md) § D3 codified
the F-verdict algorithm (F1/F2/F3/F4) for the v2.5 TCN
alpha-investigation. The retrospective
([`spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`](../../dev-notes/v25-dl-journey-retrospective-2026-05-22.md)
§ Lessons learned, item #2) locked **F-verdict immutability** as the
load-bearing invariant for cross-paradigm evidence comparability — the
F-verdict thresholds (1e-6, std/sigma_train > 0.1, etc.) cannot drift
across follow-on ships because they anchor the comparable measurement
bar the v25-tcn / v25-patchtst chain converged on.

[`spec/v3-volatility-forecaster/feature.md`](../../v3-volatility-forecaster/feature.md)
(operator-decide bundle "Autoapprove all" landed 2026-05-22) asks the
architect to lock the **vol-forecast verdict shape**. The v0.1.0 ship
uses **GARCH(1,1)-only** (Q2=(a); cheap-first per retrospective lesson
#1) over the **Parkinson realized-vol target** (Q1=(b)), with the
primary anchor target being the **vol-targeting overlay on v1
momentum** (Q3=(d) with R6.a primary). The verdict must:

1. Route the operator's follow-on funding decision **without
   hand-eyeballing a per-symbol QLIKE table** — the algorithm has to
   be code-checkable and reproducible (ADR-0033 § D3 precedent).
2. Stay **mutually exclusive across V1-V5 + V_ALPHA** (ADR-0033 § D3
   precedent — a fixture grid asserts exclusivity).
3. Stay **parallel to ADR-0033 § D3, not an extension** — the
   F-verdict algorithm in ADR-0033 § D3 stays IMMUTABLE per
   retrospective lesson #2. This is Q4=(b) operator default.
4. Carry the **GARCH(1,1) baseline contract** so that a future
   vol-forecaster ship (v0.1.1 DL refinement; v0.2.0 multi-horizon)
   inherits the comparable measurement bar.

Three orthogonal decisions to lock here, cited from feature.md:

1. **V-verdict priority tree** — V1-V5 (model-quality) + V_ALPHA
   (strategy-side gate) shape, mutual exclusivity, evidence string
   format. Mirrors ADR-0033 § D3 priority-tree structure but with
   vol-specific inputs (QLIKE, calibration ratio, per-symbol
   dispersion — NOT `frac_inside_epsilon` / gate-survival).
2. **Report body shape** — frontmatter (advisory) vs. body (hashed),
   per-symbol QLIKE table format, calibration scatter representation,
   verdict section placement, follow-on routing language. Both
   report families must follow the ADR-0032 § D4 precedent
   (run-varying fields in frontmatter only).
3. **GARCH(1,1) baseline contract** — per-symbol fit shape, parameter
   ranges + convergence tolerance + max iters, JSON checkpoint
   schema, determinism contract. The MLE implementation choice
   (hand-rolled vs. external crate) is also locked here.

## Decision

### D1. V-verdict priority tree (parallel to ADR-0033 § D3, not extension)

The V-verdict is **a sibling of the F-verdict**, evaluated by an
independent algorithm over the M-V-VERDICT report's per-symbol QLIKE +
calibration statistics. The two verdicts share no code path; F-verdict
remains exactly as locked in ADR-0033 § D3.

#### D1.a — Per-symbol inputs (collected over the BS-1 evaluation span)

For each of the 10 symbols, the V-verdict bin computes:

```rust
// crates/forecast/src/bin/vol_verdict.rs
struct PerSymbolStats {
    symbol: String,            // e.g. "BTCUSDT"
    n_predictions: u64,        // count of (sigma_hat, sigma_realized) pairs
    qlike_garch: f64,          // QLIKE(GARCH σ̂, Parkinson σ_realized) over BS-1
    qlike_constant: f64,       // QLIKE(unconditional_var, Parkinson σ_realized)
    mean_sigma_hat: f64,       // mean predicted σ over BS-1
    mean_sigma_realized: f64,  // mean realized (Parkinson) σ over BS-1
    std_sigma_hat: f64,        // sample stdev of predicted σ
    std_sigma_realized: f64,   // sample stdev of realized σ
    // Calibration ratio: mean_sigma_hat / mean_sigma_realized.
    // A well-calibrated forecaster has this in [0.7, 1.4].
}
```

QLIKE per Patton 2011 *Volatility forecast comparison using imperfect
volatility proxies*, robust to noise in the Parkinson proxy:

```text
QLIKE(σ̂, σ_realized) = mean over t,s of:
    (σ_realized² / σ̂²) - ln(σ_realized² / σ̂²) - 1
```

QLIKE is non-negative; lower is better; zero iff σ̂ ≡ σ_realized.

**Cross-symbol aggregates:**

```rust
struct AggregateStats {
    qlike_garch_mean: f64,     // mean over the 10 symbols of qlike_garch
    qlike_constant_mean: f64,  // mean over the 10 symbols of qlike_constant
    qlike_garch_max: f64,      // worst (highest) per-symbol QLIKE_GARCH
    qlike_garch_min: f64,      // best (lowest)  per-symbol QLIKE_GARCH
    // Per-symbol dispersion of QLIKE_GARCH:
    //   qlike_dispersion = qlike_garch_max / qlike_garch_min
    qlike_dispersion: f64,
}
```

All ten fields above are emitted in the report body's tables (D2.a).

#### D1.b — Per-feature verdict function (V1..V5)

```rust
fn classify_v(agg: &AggregateStats, per_symbol: &[PerSymbolStats]) -> Verdict {
    // V1 — Constant collapse.
    //
    // The GARCH(1,1) fitter output is numerically constant (σ̂ ≡
    // unconditional_var across all t,s on every symbol). "Constant"
    // is operationalised as
    //   std_sigma_hat / mean_sigma_hat < 1e-3
    // on EVERY symbol — i.e. the coefficient-of-variation of the
    // predicted σ is below 0.1% on every pair. This is tighter than
    // the F1 1e-6 threshold because vol predictions are bounded
    // away from zero (σ ≥ ω/(1 - α - β) under GARCH stationarity);
    // the "constant collapse" signature is "no time-variation,"
    // not "numerically zero."
    if per_symbol.iter().all(|s| s.std_sigma_hat / s.mean_sigma_hat.max(1e-12) < 1e-3) {
        return Verdict::V1 {
            evidence: format!(
                "max CoV(σ̂) = {:.6} < 1e-3 across all 10 symbols (worst-symbol = {})",
                per_symbol.iter()
                    .map(|s| s.std_sigma_hat / s.mean_sigma_hat.max(1e-12))
                    .fold(0.0_f64, f64::max),
                per_symbol.iter()
                    .max_by(|a, b| (a.std_sigma_hat / a.mean_sigma_hat.max(1e-12))
                        .partial_cmp(&(b.std_sigma_hat / b.mean_sigma_hat.max(1e-12)))
                        .unwrap_or(std::cmp::Ordering::Equal))
                    .map(|s| s.symbol.as_str())
                    .unwrap_or("?"),
            ),
            follow_on: "v3-garch-refit-diagnose",
        };
    }

    // V2 — Per-symbol mis-fit (heteroscedastic dispersion across the
    // universe). The QLIKE varies more than 3× across the 10
    // symbols, signalling that the universal GARCH(1,1) initial
    // conditions / convergence path are not robust across the price
    // regimes of (e.g. BTC vs DOGE). Operationalisation:
    //   qlike_dispersion = qlike_garch_max / qlike_garch_min > 3.0
    if agg.qlike_dispersion > 3.0 {
        return Verdict::V2 {
            evidence: format!(
                "qlike_dispersion = qlike_garch_max / qlike_garch_min = {:.6} > 3.0 \
                 (max = {:.6}, min = {:.6})",
                agg.qlike_dispersion, agg.qlike_garch_max, agg.qlike_garch_min,
            ),
            follow_on: "v3-garch-per-symbol-hyperparam-search",
        };
    }

    // V3 — Calibration drift. The predicted σ is systematically
    // biased away from realized σ. Operationalisation:
    //   mean_ratio = mean_sigma_hat / mean_sigma_realized, computed
    //                per-symbol and averaged across the 10 symbols.
    //   V3 fires iff mean_ratio is outside [0.7, 1.4] (i.e. the
    //   GARCH baseline systematically over- or under-predicts).
    let mean_calibration_ratio: f64 = per_symbol.iter()
        .map(|s| s.mean_sigma_hat / s.mean_sigma_realized.max(1e-12))
        .sum::<f64>() / (per_symbol.len() as f64).max(1.0);
    if mean_calibration_ratio < 0.7 || mean_calibration_ratio > 1.4 {
        return Verdict::V3 {
            evidence: format!(
                "mean_calibration_ratio = mean_over_symbols(mean(σ̂)/mean(σ_realized)) = {:.6} \
                 outside [0.7, 1.4]",
                mean_calibration_ratio,
            ),
            follow_on: "v3-garch-calibration-tune",
        };
    }

    // V4 — No improvement over constant-σ baseline.
    //
    // Equivalent to H4 falsification: the GARCH(1,1) baseline does
    // NOT meaningfully beat the constant-σ (unconditional variance)
    // baseline. Operationalisation:
    //   per-symbol QLIKE improvement over constant-σ is:
    //     (qlike_constant - qlike_garch) / qlike_constant.max(1e-12)
    //   V4 fires iff fewer than 7 of 10 symbols show ≥ 10%
    //   improvement.
    let n_improving = per_symbol.iter()
        .filter(|s| (s.qlike_constant - s.qlike_garch) / s.qlike_constant.max(1e-12) >= 0.10)
        .count();
    if n_improving < 7 {
        return Verdict::V4 {
            evidence: format!(
                "n_symbols_improving_≥10pct_over_constant_sigma = {} < 7 of 10",
                n_improving,
            ),
            follow_on: "v3-data-vol-investigation",
        };
    }

    // V5 — GARCH baseline is healthy.
    //
    // Fallback case: V1-V4 are all false. The GARCH(1,1) baseline
    // emits time-varying σ̂ that beats the constant-σ baseline by
    // ≥10% on ≥7 of 10 symbols, the per-symbol dispersion stays
    // within the 3× bound, and calibration ratio is inside
    // [0.7, 1.4]. Routes the strategy-side V_ALPHA gate.
    Verdict::V5 {
        evidence: format!(
            "n_improving = {} ≥ 7; qlike_dispersion = {:.6} ≤ 3.0; \
             mean_calibration_ratio = {:.6} ∈ [0.7, 1.4]",
            n_improving, agg.qlike_dispersion, mean_calibration_ratio,
        ),
        follow_on: "v_alpha_strategy_gate",
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Verdict {
    V1 { evidence: String, follow_on: &'static str },
    V2 { evidence: String, follow_on: &'static str },
    V3 { evidence: String, follow_on: &'static str },
    V4 { evidence: String, follow_on: &'static str },
    V5 { evidence: String, follow_on: &'static str },
}
```

**Mutual exclusivity** — V1 → V2 → V3 → V4 → V5 fallthrough in
priority order. The first triggering case returns. A unit test in
`crates/forecast/tests/vol_verdict_mutual_exclusivity.rs` asserts
mutual exclusivity over a hand-built fixture grid + a property test
(random fixture → exactly one verdict returned). Same shape as
`crates/forecast/tests/forecast_distribution_verdict.rs` (ADR-0033
§ D3.d precedent).

**Threshold derivation notes:**

- **V1 1e-3 CoV(σ̂)**: tighter than the F1 1e-6 because the GARCH
  output is bounded below by `ω/(1-α-β)` (unconditional variance ≥
  ~5e-5 at hourly cadence on top-10 USDT pairs per published
  crypto-vol benchmarks). CoV < 1e-3 means the predicted σ moves
  less than 0.1% of its mean — unambiguous "the fit collapsed to
  unconditional variance." If a future ship legitimately produces
  CoV in the 1e-4 to 1e-3 range, V1's threshold revisits in a
  superseding ADR.
- **V2 3.0 dispersion**: per the survey + Catania-Grassi 2017 +
  Petrozziello 2022, published per-symbol GARCH(1,1) QLIKE values on
  crypto hourly OHLCV typically span a 1.5-2.5× range across
  BTC/ETH/major altcoins. 3× is the "obvious mis-fit" threshold —
  signals that the universal initial conditions need a per-symbol
  hyperparameter search (V2's follow-on).
- **V3 [0.7, 1.4] calibration band**: well-calibrated vol forecasters
  per published benchmarks sit in [0.8, 1.2]; [0.7, 1.4] is a 2×
  tolerance band (40% bias) that flags systematic over- or
  under-prediction without firing on noise.
- **V4 ≥10% on ≥7/10 symbols**: matches the H4 falsification
  threshold (feature.md § H4). 10% QLIKE improvement is the
  published "GARCH beats constant" floor on crypto-hourly
  benchmarks; 7/10 is a strict majority — if 4+ symbols fail to
  show 10% improvement, the universe-level signal is broken.

#### D1.c — V_ALPHA strategy-side gate (parallel to F4's M-SHARPE)

V_ALPHA is **NOT** part of the V1..V5 priority tree. It is a
**separate strategy-side gate** that runs against the M-SHARPE
report (Sharpe-comparison bin) — exactly the same architectural shape
as F4's M-SHARPE in ADR-0033 § D3.c (the M-SHARPE Sharpe-delta
verdict is a sibling of the F-verdict, not a 5th branch).

```rust
// crates/forecast/src/bin/sharpe_comparison.rs
// (vol-target-bs1 dispatch extension)
fn classify_v_alpha(
    sharpe_baseline: f64,         // un-targeted v1 momentum (top10-2023-1h-momentum)
    sharpe_vol_target: f64,       // top10-2023-fy-vol-target-overlay-realdata
    sharpe_vol_target_net: f64,   // net of turnover (gating metric)
) -> TVerdict {
    let gross_delta = sharpe_vol_target - sharpe_baseline;
    let net_delta = sharpe_vol_target_net - sharpe_baseline;
    if net_delta >= 0.10 {
        TVerdict::TVolAlphaUnlocked { gross_delta, net_delta }
    } else if net_delta >= 0.05 {
        TVerdict::TVolMarginal { gross_delta, net_delta }
    } else {
        TVerdict::TVolNoAlpha { gross_delta, net_delta }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum TVerdict {
    TVolAlphaUnlocked { gross_delta: f64, net_delta: f64 },
    TVolMarginal      { gross_delta: f64, net_delta: f64 },
    TVolNoAlpha       { gross_delta: f64, net_delta: f64 },
}
```

**Net-of-turnover is the gating metric** — per K-vol-1 (rebalancing
turnover eats lift). Gross Sharpe-delta is reported side-by-side for
diagnostic visibility.

**Joint advisory verdict** (recorded at M-FINAL in feature.md §
Verification):

| V-verdict | T-classifier | Joint advisory verdict | Operator routing |
|-----------|--------------|------------------------|------------------|
| V5        | T-VOL-ALPHA-UNLOCKED | ALPHA-UNLOCKED        | Ship; promote C2 + C5. |
| V5        | T-VOL-MARGINAL       | MARGINAL              | Spawn `v3-vol-target-tuning`. |
| V5        | T-VOL-NO-ALPHA       | NO-ALPHA              | Analyst spawn for C1 retirement; route budget to C2. |
| V1/V2/V3  | (any)                | MODEL-BROKEN          | Follow V-verdict's `follow_on` field. |
| V4        | (any)                | DATA-PATHOLOGY        | Spawn `v3-data-vol-investigation`; foreclose on H1/H2 jointly. |

The joint table is the **only** place V-verdict and T-classifier
combine. They never combine inside the verdict bins themselves; that
keeps each bin's output anchor-deterministic in isolation.

### D2. Report body shape — frontmatter vs. body discipline

Both report families (V-verdict + Sharpe-comparison-vol-target) follow
ADR-0032 § D4 + ADR-0033 § D2 precedents exactly: run-varying fields
in YAML frontmatter (excluded from body hash via
`scripts/hash_report.py`); deterministic content in the body.

#### D2.a — `vol-verdict-bs1-realdata-YYYYMMDD.md`

**Frontmatter (advisory, NOT hashed):**

```yaml
---
slug: v3-volatility-forecaster
scenario: vol-verdict-bs1-realdata
generated: 2026-MM-DDTHH:MM:SSZ                 # ISO-8601, second precision
wall_clock_s: 12.3                              # f64, one decimal
host: <hostname>                                # advisory only
git_commit: <40 hex>                            # advisory only
checkpoint_revision: <64 hex>                   # GARCH per-symbol JSON aggregate SHA
data_revision_sha: 3a8b96c4…                    # 64 hex from data/binance/REVISION.toml
verdict: V5                                     # OR V1/V2/V3/V4 — mirror of body
---
```

**Body (deterministic, hashed by anchor):**

```markdown
# Vol-forecast V-verdict report — BS-1 (real Binance hourly OHLCV, GARCH(1,1))

## Checkpoint

| Field              | Value                                          |
|--------------------|------------------------------------------------|
| Anchor scenario    | garch-bs1                                      |
| checkpoint_revision| <64 hex>                                       |
| target_kind        | Parkinson                                      |
| target_horizon_bars| 24                                             |
| evaluation_span    | 2023-01-01T00:00:00Z .. 2024-01-01T00:00:00Z   |
| n_symbols          | 10                                             |
| n_predictions_total| 87590                                          |

## Per-symbol QLIKE table

10 rows, fixed-precision floats. Order: ADA, AVAX, BNB, BTC, DOGE,
DOT, ETH, LINK, SOL, XRP (alphabetical USDT-quote universe).

| symbol  | n_pred | qlike_garch | qlike_const | improvement_pct | mean_sigma_hat | mean_sigma_real | calib_ratio | std_sigma_hat | std_sigma_real |
|---------|--------|-------------|-------------|-----------------|----------------|-----------------|-------------|---------------|----------------|
| ADAUSDT | 8759   | 0.123456    | 0.234567    | 47.350123       | 0.012345       | 0.013456        | 0.917437    | 0.001234      | 0.001345       |
| AVAXUSDT| 8759   | …           | …           | …               | …              | …               | …           | …             | …              |
| (… all 10 rows …) |
| XRPUSDT | 8759   | …           | …           | …               | …              | …               | …           | …             | …              |

## Aggregate statistics

| Field                       | Value      |
|-----------------------------|------------|
| qlike_garch_mean            | 0.150000   |
| qlike_constant_mean         | 0.280000   |
| qlike_garch_max             | 0.220000   |
| qlike_garch_min             | 0.090000   |
| qlike_dispersion            | 2.444444   |
| mean_calibration_ratio      | 0.984321   |
| n_symbols_improving_≥10pct  | 9          |

## Verdict

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Case              | V5                                             |
| Trigger evidence  | n_improving = 9 ≥ 7; qlike_dispersion = 2.444444 ≤ 3.0; mean_calibration_ratio = 0.984321 ∈ [0.7, 1.4] |
| Routes to         | V_ALPHA strategy-side gate (Sharpe-comparison bin) |

## Notes

- Read-only against `crates/forecast/checkpoints/anchors/garch-bs1-<sha>.json`.
- QLIKE per Patton 2011 *Volatility forecast comparison using
  imperfect volatility proxies* — robust to noise in the Parkinson
  σ_realized proxy; preferred over MSE for vol forecasts.
- Parkinson realized-vol target: `σ̂_P² = (1/(4·ln 2)) · mean over k of (ln(high_k/low_k))²`.
- V-verdict algorithm: see [ADR-0038 § D1](#d1-v-verdict-priority-tree-parallel-to-adr-0033--d3-not-extension).
```

**Floating-point canonicalisation** (locked here to forestall K3 drift;
same precision discipline as ADR-0033 § D2.a):

| Field family                  | Format                            |
|-------------------------------|-----------------------------------|
| qlike_*, calib_ratio, mean_sigma_*  | `format!("{:.6}", x)` (6 decimals) |
| std_sigma_*                   | `format!("{:.6}", x)` (6 decimals) |
| improvement_pct               | `format!("{:.6}", x)` (6 decimals) |
| qlike_dispersion              | `format!("{:.6}", x)` (6 decimals) |
| n_pred, n_symbols_improving   | `format!("{}", x)` (integer)       |

**Symbol-row order** is alphabetical USDT-quote (locked here to
forestall hash-map iteration drift): ADAUSDT, AVAXUSDT, BNBUSDT,
BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT, LINKUSDT, SOLUSDT, XRPUSDT.

#### D2.b — `sharpe-comparison-vol-target-bs1-realdata-YYYYMMDD.md`

Sibling of the v25-tcn-alpha-investigation `sharpe-comparison-realdata`
report (ADR-0033 § D2.b). The dispatch extension to
`crates/forecast/src/bin/sharpe_comparison.rs` adds two new sources:

```yaml
sources:
  - spec/backtest-real-binance-data/reports/backtest-…-top10-2023-1h-momentum-realdata.md  # un-targeted v1 baseline
  - spec/v3-volatility-forecaster/reports/backtest-…-top10-2023-fy-vol-target-overlay-realdata.md
```

The Sharpe-delta table mirrors ADR-0033 § D2.b's columns; the
T-classifier verdict (T-VOL-ALPHA-UNLOCKED / T-VOL-MARGINAL /
T-VOL-NO-ALPHA) appears in the Verdict section with both gross + net
columns reported side-by-side. The new bin determinism contract is
identical to ADR-0033 § D2.b (2-run byte-identity gate at M-FINAL).

### D3. GARCH(1,1) baseline contract

**Implementation choice — hand-rolled MLE, NOT `rust-quant`:**

The architect decides to **hand-roll the GARCH(1,1) MLE** in
~80-120 LoC of pure Rust under `crates/forecast/src/garch.rs`,
rejecting the `rust-quant` v0.0.10 alternative for these reasons:

1. **No new external crate dependency** — analyst-default per
   feature.md § R10. The hand-rolled fitter has zero `Cargo.toml`
   churn; `rust-quant` adds a transitive-dep surface
   (`rust-quant` → `nalgebra` → `simba` → …) that complicates the
   single-binary discipline (CLAUDE.md § Library compatibility
   checklist).
2. **API surface fit** — `rust-quant`'s GARCH module is built for
   per-fit hyperparameter exposure (initial values, bounds,
   convergence options) but no clean "load checkpoint + recurrence
   step" entry point. Our consumer (`GarchVolForecaster::forecast_vol`)
   needs the recurrence step at sub-microsecond cost; wrapping
   `rust-quant`'s API to fit that shape costs roughly the same LoC
   as hand-rolling.
3. **Maintained status** — `rust-quant` v0.0.10 is a 0.0.x
   pre-stable crate; per CLAUDE.md compatibility checklist, the
   "maintained" gate prefers either ≤18-month-old releases of
   well-known crates OR hand-rolled. GARCH(1,1) MLE is textbook
   1986 mathematics; hand-rolling is the lower-risk path.
4. **Determinism contract** — hand-rolled lets us pin the
   quasi-Newton optimisation steps + termination conditions in
   source, which is load-bearing for the
   `garch_fit_determinism` 2-run byte-identity unit test (R11.4).
   A 3rd-party crate's optimiser may pick up internal changes
   between minor versions.

**Hyperparameters (locked here):**

| Parameter | Value     | Rationale |
|-----------|-----------|-----------|
| ω initial | 1e-6      | Per Bollerslev 1986; small positive; converges fast on crypto hourly. |
| α initial | 0.10      | Typical crypto hourly fit per Catania-Grassi 2017. |
| β initial | 0.85      | Typical crypto hourly fit; half-life ~24-72 hours. |
| Convergence tol | 1e-8 | Tighter than published 1e-6 default — ensures determinism. |
| Max iters | 500       | Bollerslev 1986 convergence in <100 iter; 500 is 5× safety margin. |
| Optimiser | L-BFGS (hand-rolled, single-precision gradient) | Quasi-Newton; documented in source comments. |
| Constraint | α + β < 1 (stationarity) | Re-projected after each step; aborts run if α+β diverges. |
| Stationarity floor | (ω, α, β) > 1e-10 | Avoids log(0) in the likelihood. |

**Per-symbol fit:** 10 independent fits (no pooled fit in v0.1.0;
deferred to v0.1.1 per feature.md § R2). Each fit runs sub-second
wall-clock; total ~5-10 seconds for the 10-symbol universe.

**JSON checkpoint schema** (`crates/forecast/checkpoints/anchors/garch-bs1-<sha>.json`):

```json
{
  "schema_version": 1,
  "target_kind": "Parkinson",
  "target_horizon_bars": 24,
  "train_span_start": "2023-01-01T00:00:00Z",
  "train_span_end":   "2024-01-01T00:00:00Z",
  "data_revision_sha": "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7",
  "params": {
    "ADAUSDT":  { "omega": 1.234e-06, "alpha": 0.087654, "beta": 0.852341, "unconditional_var": 0.000023, "log_likelihood": 12345.67, "n_iters": 87,  "converged": true },
    "AVAXUSDT": { "omega": …,         "alpha": …,        "beta": …,        "unconditional_var": …,        "log_likelihood": …,       "n_iters": …,   "converged": true },
    "...":      { ... },
    "XRPUSDT":  { ... }
  }
}
```

**Aggregate SHA derivation** (locks `checkpoint_revision` per
ADR-0029 § canonical-arch-descriptor additive extension):

```text
checkpoint_revision = SHA-256(
    "garch-bs1\n" ||
    "schema_version=1\n" ||
    "target_kind=Parkinson\n" ||
    "target_horizon_bars=24\n" ||
    "train_span=2023-01-01T00:00:00Z..2024-01-01T00:00:00Z\n" ||
    "data_revision_sha=3a8b96c4…\n" ||
    canonical_params_block      // JSON canonicalised: keys alpha-sorted, floats %.6e
)
```

The `canonical_params_block` ASCII representation is locked to
`%.6e` floats, alphabetical symbol keys, alphabetical inner-key
order (`alpha, beta, converged, log_likelihood, n_iters, omega,
unconditional_var`). This sidesteps IEEE-754 round-trip drift
across symbols (the JSON file itself uses `%.9e` for human
readability; the canonicalised hash input is `%.6e` because the
fit precision is ~5-6 significant figures).

**Determinism contract:** two sequential GARCH fits on identical
input bars produce byte-identical JSON files (and identical
`checkpoint_revision`). Tested at M-FINAL via
`crates/forecast/tests/garch_fit_determinism.rs` (R11.4).

**Forecast recurrence step** (sub-microsecond per call):

```rust
// crates/forecast/src/garch.rs
impl GarchModel {
    /// One GARCH(1,1) recurrence: σ²_t = ω + α·r²_{t-1} + β·σ²_{t-1}.
    /// Returns predicted σ for horizon 1 (caller multiplies by sqrt(H)
    /// for multi-horizon, or recurses for term-structure).
    pub fn forecast_step(&self, r_prev: f64, sigma_prev: f64) -> f64 {
        let sigma2 = self.omega
            + self.alpha * r_prev * r_prev
            + self.beta  * sigma_prev * sigma_prev;
        sigma2.max(self.omega).sqrt() // floor at ω prevents underflow
    }
}
```

### D4. Replay-cache namespace additive extension

Per feature.md § R4 + tasks.md T-AR-3, `crates/replay-cache` extends
with the new namespace `"vol_forecast"`:

- The existing `"forecast"` namespace stays **byte-identical** —
  cache keys for v2.5 TCN / v2.5a PatchTST entries unchanged.
- New cache keys live under `"vol_forecast"` keyed by
  `(symbol, timestamp, checkpoint_revision)`. Cache value:
  `{ sigma_hat: f64, horizon_bars: u32, model_revision: Sha256 }`.
- Implementation: enum variant extension in
  `crates/replay-cache/src/lib.rs::CacheNamespace` (additive;
  serialisation of existing variants unchanged).

### D5. Strategy-side composition (v0.1.0); risk-engine deferred

Per feature.md § R12 K-vol-2 lock + the analyst's audit
(`crates/cost/src/risk_state.rs` does NOT exist on disk; closest
surface is `crates/cost/src/budget.rs`), this ADR locks:

- **v0.1.0 ships strategy-side composition only** —
  `VolTargetingOverlay<S: Strategy>` wraps the inner v1 momentum
  strategy. No risk-engine refactor; no `crates/cost/` modification.
- **Risk-engine integration deferred to v0.1.1** — if v0.1.0
  finishes T-VOL-ALPHA-UNLOCKED, the v0.1.1 spawn covers the
  kill-switch risk-engine wire-up + `crates/cost/src/budget.rs`
  vol-input surface.
- **The Q3=(d) kill-switch builder still ships in v0.1.0** — but
  as a `Strategy` wrapper (`VolKillSwitchOverlay<S>`), not a
  risk-engine hook. The kill-switch fires inside `on_bar()`, not
  inside the cost-crate event loop. This is anchor-additive only
  — the existing strategy surface gains a new builder; nothing
  in `crates/cost/` changes.

### D6. Anchor + version naming

- **New anchors** (under version `v3.0.0-volatility` per Q5=(a)):
  - `vol-verdict-bs1-realdata` (M-V-VERDICT)
  - `top10-2023-fy-vol-target-overlay-realdata` (M-SHARPE primary)
  - `sharpe-comparison-vol-target-bs1-realdata` (M-SHARPE comparison)
- **Existing 30 anchors stay byte-identical** — this ship is
  anchor-additive only. The kill-switch backtest scenario
  (`top10-2023-fy-vol-killswitch-overlay-realdata`) ships **without
  an anchor in v0.1.0** per Q-anchors-sub = 3 default — added in
  v0.1.1 if byte-deterministic.

Anchor count progression:
- Pre-feature: 30 (current baseline post v25a-patchtst-overlay ship,
  confirmed via `scripts/verify_anchors.sh` → `ANCHORS PASS (30/30)`
  on 2026-05-22).
- Post M-V-VERDICT: 31 (+ vol-verdict-bs1-realdata).
- Post M-SHARPE: 33 (+ vol-target-overlay-realdata + sharpe-comparison).

### D6.b — Wiring-bug-fix re-emission protocol (amendment, 2026-05-22)

Adopted under [v3-volatility-forecaster-noop-fix](../../v3-volatility-forecaster-noop-fix/feature.md) v0.1.0 (P0). The original D6 contract reads "existing anchors stay byte-identical." That spirit is **don't silently mutate historical evidence**. When the recorded body reflects a demonstrated wiring bug (the contract being witnessed is materially different from what was intended), re-emission is legitimate **under the following protocol**:

1. **Enumerate affected anchors** with current SHA-256 in the feature brief's § Investigation findings. The architect confirms the enumeration is exhaustive at M-T1 (e.g. via cross-grep of the report-body sources for the load-bearing observable; see [v3-volatility-forecaster-noop-fix decomp.md § T-AR-5](../../v3-volatility-forecaster-noop-fix/decomp.md) for the worked example — 4 candidates audited, 1 ruled out as GARCH-only).
2. **Cite the bug site with `file:line`** in the feature brief's § Smoking gun. The dev-note captures the diagnostic chain that surfaced the bug (cf. [v3-vol-overlay-noop-discovery-2026-05-22.md](../../dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md) — caveman probe + byte-identical surfacing).
3. **Include the would-have-caught test** as a feature requirement (e.g. R2 in the worked example). The test MUST be run against the **pre-fix** code BEFORE the fix lands; the architect captures the literal pre-fix FAIL output as evidence the gate is meaningful (cf. [v3-volatility-forecaster-noop-fix decomp.md § T-AR-4 forensic gate](../../v3-volatility-forecaster-noop-fix/decomp.md)).
4. **Architect signs off on the re-emission delta**. The new SHAs land in `spec/anchors.toml` **in-place under the existing namespaces** (Q2=(a) default — never bifurcate the namespace; never silently delete a row). A comment block above the affected rows cites the fix-feature slug + the dev-note slug.
5. **Negative invariant**: the unchanged rows MUST stay byte-identical. Tester M-FINAL captures the diff at `spec/<fix-feature>/reports/test-final-<date>.md` showing every changed row + the count of unchanged rows. The wave-B verify_anchors.sh output (`ANCHORS PASS (N / N)`) is the gate; a regression to `FAIL` halts the ship.

**Allowed re-emission scope**: only rows whose body cites the load-bearing observable that the bug perturbed. Rows that cite orthogonal observables (e.g. GARCH-only model diagnostics for an overlay-wiring fix) stay byte-identical and are part of the negative invariant.

**Not in scope of this protocol**: silent mutations (forbidden by D6 spirit), namespace bifurcation (a `*-postfix` namespace was rejected at Q2 — bifurcation invites future readers to consume stale bodies), row deletion (forbidden — historical evidence stays linked even after re-emission via the dev-note + feature.md cross-references).

**Live-exec parity follow-on**: the v0.1.0 vol-target wire-up landed at the **backtest-only** sizing-pipeline site (`crates/backtest/src/scenarios/garch_vol_target_overlay.rs`). Live execution (when wired in `crates/exec/` post-v0.1.1) MUST add an equivalent `Strategy::quantity_scale` query at the live order-construction site. Parity gap is flagged in [v3-volatility-forecaster-noop-fix decomp.md § T-AR-2](../../v3-volatility-forecaster-noop-fix/decomp.md) and tracked as a v0.1.1 follow-on item.

**Precedent**: this is the **first** invocation of D6.b. Future wiring-bug discoveries inherit the 5-step protocol verbatim. If the protocol itself needs revision (e.g. multi-overlay wire-up bugs requiring batched re-emissions), the revision lands as **D6.c** (additive amendment subsection, not in-place mutation of D6.b).

## Alternatives considered

1. **Extend ADR-0033 § D3 with vol-classifier branches.** Rejected
   per Q4=(a) reject + retrospective lesson #2: ADR-0033 § D3 is
   IMMUTABLE for return-target forecasters. The F-verdict thresholds
   anchor the comparable measurement bar across v25-tcn /
   v25-patchtst evidence; mutating ADR-0033 to host V1-V5 branches
   breaks that immutability property for zero architectural benefit.

2. **Use `rust-quant` v0.0.10 GARCH fitter.** Rejected per D3 above:
   four independent reasons (new dep, API fit, maintained status,
   determinism contract). Hand-rolled is the lower-risk path for an
   ~80-120 LoC textbook implementation.

3. **Embed V-verdict thresholds in TOML config.** Rejected per the
   ADR-0033 § Alternatives precedent: thresholds are load-bearing
   for the algorithm; a TOML config invites operators to tune them
   silently between runs and defeats the anchor contract. Future
   tuning happens in a superseding ADR with an explicit follow-on
   feature.

4. **MSE / MAE-of-log instead of QLIKE for V-verdict loss.**
   Rejected per Patton 2011: QLIKE is the unique vol-forecast loss
   that is **invariant to noise in the volatility proxy** (Parkinson
   is a noisy estimator of true realized vol). MSE on σ over a noisy
   proxy under-weights large-σ regimes; MAE-of-log over-weights
   small-σ regimes. QLIKE is the textbook choice for noisy-proxy
   evaluation.

5. **Per-checkpoint joint verdict (BS-1 + BS-2 mirroring ADR-0033 §
   D3.c).** Deferred to v0.1.1. v0.1.0 ships BS-1 V-verdict only;
   per Q6=(a) BS-1 train + BS-2 val convention, BS-2 vol-target
   backtest scenario is out-of-scope per feature.md § Out of scope.
   When v0.1.1 lands the BS-2 vol-target scenario, a follow-on ADR
   amendment specifies the joint V-verdict shape (likely mirroring
   ADR-0033 § D3.c verbatim).

6. **Pool GARCH fits across symbols.** Rejected for v0.1.0 per
   feature.md § R2: pooled fits are a v0.1.1 optimisation; the
   per-symbol fit is the cleaner baseline + V2 mis-fit verdict
   surfaces per-symbol divergence faster than a pooled fit would.

7. **Walk-forward GARCH refit (re-fit every N bars).** Deferred to
   v0.1.1 per Q6=(b) reject. Single BS-1 fit on the full 2023 span
   is the cleanest comparison vs un-targeted v1 momentum baseline
   (mirrors the v2.5 fixed-checkpoint convention).

## Consequences

**New files (this ADR scope):**
- This file: `spec/architecture/adr/0038-vol-forecast-verdict-shape.md`
- `crates/forecast/src/garch.rs` (~120 LoC; per D3 hand-rolled MLE)
- `crates/forecast/src/vol.rs` (~80 LoC; `VolForecastProvider` trait + types per feature.md § R4)
- `crates/forecast/src/bin/vol_verdict.rs` (~280 LoC; mirrors `forecast_distribution.rs` shape)
- `crates/forecast/src/bin/train_garch.rs` (~100 LoC; per-symbol MLE fit driver)
- `crates/strategy/src/vol_targeting_overlay.rs` (~120 LoC; R6.a primary)
- `crates/strategy/src/vol_killswitch_overlay.rs` (~80 LoC; R6.b secondary)
- `crates/strategy/src/vol_meanreversion.rs` (~100 LoC; R6.c tertiary)
- `crates/backtest/src/scenarios/garch_vol_target_overlay.rs` (~250 LoC; sibling of `tcn_overlay_weights.rs`)
- `crates/forecast/tests/garch_fit_determinism.rs` (~80 LoC; R11.4)
- `crates/forecast/tests/vol_verdict_mutual_exclusivity.rs` (~120 LoC; R11.5)
- `crates/forecast/tests/parkinson_target_derivation.rs` (~60 LoC; R11.4 vol-target fixture)
- `crates/strategy/tests/vol_targeting_overlay.rs` (~80 LoC; R11.6)
- `crates/forecast/tests/tcn_byte_identity.rs` (~30 LoC; R11.7 K-vol-3 guard)
- `crates/forecast/tests/patchtst_byte_identity.rs` (~30 LoC; R11.8 K-vol-3 guard)

**Modified files:**
- `crates/forecast/src/features.rs` (additive: `VolTargetKind` enum + Parkinson target derivation alongside existing `target_logret`; existing TCN/PatchTST callers untouched).
- `crates/forecast/src/bin/sharpe_comparison.rs` (additive: `--scenario vol-target-bs1` dispatch arm; existing TCN/PatchTST dispatch byte-identical).
- `crates/forecast/src/lib.rs` (additive: `mod garch; mod vol;`).
- `crates/strategy/src/lib.rs` (additive: 3 new builders `with_garch_vol_strategy`, `with_garch_vol_overlay_momentum`, `with_garch_vol_kill_switch`).
- `crates/backtest/src/main.rs` (additive: `ScenarioStrategy::GarchVolTargetOverlayMomentum` variant + `top10-2023-fy-vol-target-overlay-realdata` scenario match arm).
- `crates/backtest/src/scenarios/mod.rs` (additive: `pub mod garch_vol_target_overlay;`).
- `crates/replay-cache/src/lib.rs` (additive: `CacheNamespace::VolForecast` variant; existing variants byte-identical).
- `spec/architecture/adr/README.md` — registry row added for ADR-0038.
- `spec/anchors.toml` — 3 new anchor rows under `v3.0.0-volatility`.
- `spec/trace.toml` — `REQ-V3-VOL-FORECASTER-001` `arch` / `crates` / `tests` / `anchors` columns extended.
- `spec/v3-volatility-forecaster/feature.md` — § Design block added at M-T1 close; changelog entry.
- `spec/v3-volatility-forecaster/tasks.md` — T-D-N* rows for Wave A-E.

**Cross-phase implications:**
- v0.1.1 (if v0.1.0 finishes T-VOL-MARGINAL): DL refinement spawn
  inherits the V-verdict report shape verbatim; substitute
  `garch-bs1` with `vol-{tcn,lstm,patchtst}-bs1` in the bin paths.
  Adds V5-DL-IMPROVES / V4-DL-NO-IMPROVE branches via a superseding
  ADR; the V1/V2/V3 priority branches apply as-is (they classify
  the underlying baseline, not the DL refinement specifically).
- v3-regime-classifier (C2, sibling analyst pass): inherits the
  report-shape discipline (frontmatter vs body); the verdict
  algorithm is task-specific (regime confusion matrix vs vol
  calibration) so it gets its own ADR.
- v3-llm-overlay (C5, sibling analyst pass): same — task-specific
  verdict; same body-shape discipline.

**Enforced by:**
- `cargo test -p forecast --test vol_verdict_mutual_exclusivity` —
  one fixture per V-label + mutual-exclusivity property test
  (mirrors `forecast_distribution_verdict.rs`).
- `cargo test -p forecast --test garch_fit_determinism` — 2-run
  byte-identity of per-symbol `(ω, α, β)` JSON outputs.
- `cargo test -p forecast --test parkinson_target_derivation` —
  Parkinson formula on a hand-built fixture matches the
  closed-form value.
- `cargo test -p strategy --test vol_targeting_overlay` — overlay
  wraps inner strategy correctly + scale clamp invariants hold.
- `cargo test -p forecast --test tcn_byte_identity` — K-vol-3
  scope-creep guard (`git diff HEAD -- crates/forecast/src/tcn.rs`
  is empty modulo comment-only after the vol ship).
- `cargo test -p forecast --test patchtst_byte_identity` — same
  for `crates/forecast/src/patchtst.rs`.
- `bash scripts/verify_anchors.sh` — must report `33/33` post
  M-FINAL; pre-M-FINAL must report `30/30` (current baseline as of
  2026-05-22, confirmed by architect).
- 2-run byte-identity determinism gate on the new
  `vol-verdict-bs1-realdata-*.md` report (R11.9).
- 2-run byte-identity determinism gate on the new
  `top10-2023-fy-vol-target-overlay-realdata-*.md` report (R11.10).

**What breaks if this is violated:**
- A V-verdict authored without the algorithm (e.g. hand-written in a
  report body) → mutual-exclusivity check fails or evidence string
  doesn't match the values, the orchestrator cannot route. Caught
  by `vol_verdict_mutual_exclusivity`.
- A GARCH fitter that uses different optimiser internals (e.g.
  swapping L-BFGS for Nelder-Mead) → JSON params drift between
  runs, `garch_fit_determinism` fails, anchor lock fails.
- Floating-point format drift in the per-symbol QLIKE table (e.g.
  `%.5f` instead of `%.6f`) → body SHA flips on a second run,
  vol-verdict anchor lock fails.
- A developer adds a write site outside `--out-dir` in the
  V-verdict bin → mirrors the
  `forecast_distribution_bin_readonly` precedent (architect
  recommends a similar read-only-contract test at developer level,
  optional in v0.1.0).

**What this enables:**
- Operator gets a code-checkable V-verdict that routes follow-on
  funding decisions without eyeballing per-symbol QLIKE tables.
- v0.1.1 DL refinement (if spawned) reuses the V-verdict report
  shape verbatim, dropping its authoring cost to ~1 day.
- v0.2.0 multi-horizon vol curve + walk-forward refit inherits the
  V-verdict algorithm as the baseline measurement bar.
- Joint advisory verdict (V × T) table at D1.c gives the presenter
  a code-checkable routing tree from M-FINAL evidence → operator
  decision.

## References

- [ADR-0028](0028-v25-dl-forecast-overlay-candle.md) — candle ML
  framework; covers DL vol refinement under Q2 ≠ (a); N/A under
  Q2=(a) GARCH-only.
- [ADR-0029](0029-tcn-checkpoint-provenance.md) — canonical-arch
  descriptor; extended additively for GARCH per-symbol JSON
  checkpoint.
- [ADR-0032](0032-backtest-realdata-path-and-revision-pin.md) —
  realdata path + frontmatter-vs-body discipline (the precedent
  this ADR's D2 follows).
- [ADR-0033](0033-tcn-alpha-investigation-report-shape.md) §§
  D2/D3/D3.c/D3.d — IMMUTABLE F-verdict; this ADR is PARALLEL
  per Q4=(b), not extension.
- [ADR-0035](0035-tcn-sigma-train-recalibration.md) — N/A under
  Q2=(a) GARCH-only (GARCH has no σ_train concept).
- [`spec/v3-volatility-forecaster/feature.md`](../../v3-volatility-forecaster/feature.md)
  R1-R12, H1-H4, K-vol-1..6, Q1-Q6 + Q-anchors-sub + Q3-sub
  operator-decide bundle (autoapproved 2026-05-22).
- [`spec/dev-notes/strategy-reformulation-survey-2026-05-22.md`](../../dev-notes/strategy-reformulation-survey-2026-05-22.md)
  § Candidate 1 — survey-time cost / EV / reuse scoping.
- [`spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`](../../dev-notes/v25-dl-journey-retrospective-2026-05-22.md)
  § Lessons learned (cheap-first; F-verdict immutability; σ_train
  load-bearing) — guardrails this ADR honors.
- Bollerslev 1986 — *Generalized Autoregressive Conditional
  Heteroskedasticity* — Journal of Econometrics 31(3) — GARCH(1,1)
  foundational.
- Parkinson 1980 — *The extreme value method for estimating the
  variance of the rate of return* — Journal of Business 53(1) —
  Parkinson estimator + 5-7× sample efficiency lift.
- Patton 2011 — *Volatility forecast comparison using imperfect
  volatility proxies* — Journal of Econometrics 160(1) — QLIKE
  loss definition + proxy-noise invariance.
- Moreira & Muir 2017 — *Volatility-Managed Portfolios* — Journal
  of Finance 72(4) — vol-targeting on momentum precedent (the
  textbook prior for H2 alpha-unlock).
- Catania-Grassi 2017 — crypto-hourly GARCH(1,1) β ≈ 0.85
  empirical fit; half-life ~24-72 hours.

## Changelog

- 2026-05-22 (architect): initial accept. Locks six orthogonal
  decisions (D1 V1-V5 + V_ALPHA priority tree; D2 report body shape
  + canonicalisation; D3 hand-rolled GARCH(1,1) MLE contract
  including JSON checkpoint schema + aggregate SHA derivation; D4
  replay-cache namespace additive extension; D5 strategy-side
  composition v0.1.0 + risk-engine deferred to v0.1.1; D6 anchor
  + version naming). Covers T-AR-1 + T-AR-2 + T-AR-3 + T-AR-5
  + T-AR-6 from `spec/v3-volatility-forecaster/tasks.md`. PARALLEL
  to ADR-0033 § D3, NOT extension (Q4=(b) operator default;
  retrospective lesson #2 honored). Cross-refs
  `REQ-V3-VOL-FORECASTER-001` in `spec/trace.toml`.
