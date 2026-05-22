---
slug: v3-volatility-forecaster
status: in-progress
owner: developer
updated: 2026-05-22
version: 0.1.0
parent: (none — new strategy lane; first ship in post-v2.5 reformulation)
predecessor: v25a-patchtst-overlay v0.1.0 (RETIRED-evidence-source)
---

# v3 — Volatility forecaster (predict σ, not μ)

> **First-of-three analyst pass** in the operator's 2026-05-22 hybrid
> sequencing decision (Q-PICK = C1+C2+C5; Q-BUDGET ~6-8 weeks total cap;
> Q-SEQ = build C1 first; C2/C5 analyst passes run in parallel as
> spec-only design exploration with no code commitment until C1's
> verdict). This brief is C1 — the **vol forecaster** — and is the only
> one of the three picks where code lands during the cap window.
>
> **Sibling analyst passes (no code yet):**
> - `spec/v3-regime-classifier/feature.md` (C2 — to be authored in parallel).
> - `spec/v3-llm-overlay/feature.md` (C5 — to be authored in parallel).
>
> **Predecessor evidence (closed 2026-05-22):**
> The v2.5 DL forecast overlay umbrella retired with joint F4-F4-F4
> across TCN BS-1/BS-2 @ 1h (+0.018 / +0.045 Sharpe-delta) and PatchTST
> BS-1 @ 24h (+0.006) — see
> [`spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`](../dev-notes/v25-dl-journey-retrospective-2026-05-22.md)
> for the full chain. Predicting **μ** (next-bar log-return) over the
> 5-feature OHLCV window does not extract +0.10 Sharpe-delta on the v1
> cross-sectional momentum baseline at hourly cadence. The freed budget
> (~3-5 weeks of compute + analyst/architect/dev/tester bandwidth) plus
> operator's fresh 6-8 week cap pivots to **vol forecasting** as the
> highest-cost-effectiveness candidate per the
> [strategy-reformulation survey](../dev-notes/strategy-reformulation-survey-2026-05-22.md)
> § Tabulated summary (HIGH reuse + MEDIUM-HIGH prior + small compute +
> HIGH independence from v2.5 F4).

## Why

### The orthogonal task

The v2.5 evidence chain establishes that on hourly crypto OHLCV the
**next-bar direction** task is structurally hard. It establishes
**nothing** about the **next-period spread** task — those are
information-theoretically distinct hypotheses.

- **Direction (retired):** sign and magnitude of `(close_{t+h} /
  close_t).ln()`. The v2.5 paradigm-and-horizon exhaustion across TCN
  + PatchTST at 1h + 24h converges on F4.
- **Spread (this feature):** `σ_{t..t+h} = std(log_returns_{t..t+h})`
  or a range-based proxy (Parkinson / Garman-Klass). Vol clustering
  on crypto hourly bars is empirically strong (autocorrelation of
  `|r|` > 0.3 at lag 1-24; established in 2 decades of quant
  literature on equities + commodities + crypto).

The hypothesis: **vol is predictable on hourly crypto OHLCV at +5% QLIKE
over GARCH(1,1) baseline**, and a vol-targeting overlay on v1 momentum
extracts **+0.10 Sharpe-delta** vs the un-targeted v1 baseline. The
former is a calibration metric (textbook-precedent confidence HIGH); the
latter is the alpha-unlock gate (textbook-precedent confidence
MEDIUM-HIGH — vol-targeting on momentum is industry-standard in equity
factor research with reported Sharpe lifts of 0.1-0.3, but hourly crypto
transaction-cost drag is the load-bearing empirical unknown).

### Why this differs from v2.5 F4

| Axis | v2.5 (retired) | v3 (this feature) |
|------|----------------|-------------------|
| Task | predict μ (sign + magnitude of next-bar return) | predict σ (next-period spread of returns) |
| Loss surface | Huber on log-returns; signal-to-noise ratio < 1 | QLIKE / MSE on log-σ; signal-to-noise ratio > 5 (per published crypto-vol benchmarks) |
| Baseline | constant-zero (signal absent ⇒ p95 < ε) | GARCH(1,1) (signal present; the question is whether DL improves on the parametric floor) |
| Consumer | signal-level overlay (`combine()` of direction + confidence) | **risk-level overlay** (position-size multiplier) and/or kill-switch trigger — a different composition layer than v2.5 |
| Verdict | F-verdict per ADR-0033 § D3 (priority tree over `frac_inside_epsilon` / gate-survival / std/σ_train) | **V-verdict (NEW)** per the proposed ADR-0038 — priority tree over QLIKE-vs-GARCH / per-symbol calibration / heteroscedasticity-after-fit |

The retrospective's § "What the next research direction COULD usefully
chase" explicitly named "volatility forecasting (predict σ not μ)" as
the head item. The survey ranked C1 as **highest EV per wall-clock
week** in the post-F4 candidate set.

### Strategy lane > forecast overlay

The v2.5 umbrella was framed as the **forecast overlay** lane — a
narrow, deeply-anchored strategy slot whose primary job was to multiply
direction confidence into the v1 momentum baseline. The 30-anchor
investment in that lane is preserved as evidence; the **lane is
retired**.

This feature opens a **new lane**: a vol forecaster whose downstream
consumer is **not** a multiplicative direction signal. The natural
consumers are (per Q3 below):

1. **Vol-targeting overlay** — scales per-symbol position size by
   `1/σ̂` clipped to a band (e.g. [0.5×, 2×]). High-vol symbols get
   smaller positions; low-vol symbols get larger. Sharpe rises when
   drawdown bands narrow under high-vol regimes that previously over-
   contributed risk.
2. **Risk-engine kill-switch** — when σ̂ crosses a regime-shift
   threshold (e.g. 2-3× per-symbol historical median), the strategy
   flat-lines exposure on that symbol until σ̂ falls back inside the
   band.
3. **Strategy primitive** — a standalone strategy that emits position
   size from `1 - σ̂/σ_realized` (long when realized > predicted, flat
   when predicted ≥ realized).

This is the **risk-level** vs **signal-level** distinction the survey
flagged as the only sticky ADR amendment. The architect locks the
overlay shape at M-T1; the analyst's default is **(d) all 3 as opt-in
builders** — see Q3.

## Quantitative-finance context

- **Hourly crypto vol clustering:** empirically strong. Squared-return
  autocorrelation `Corr(r²_t, r²_{t-1})` on top-10 USDT pairs in
  2023-2024 is typically 0.15-0.40 at lag 1-24 hours; persistence
  decays slowly (half-life ~24-72 hours per the GARCH(1,1) β ≈ 0.85
  empirical fit on similar windows in published literature).
- **Range-based estimators on hourly bars:** Parkinson estimator
  `σ̂_P = sqrt((1/(4*ln 2)) * (ln(high/low))²)` is unbiased under the
  Brownian-motion assumption (Parkinson 1980); empirically 5-7× more
  efficient than realized-vol-from-close on the same data — load-
  bearing because **we already have OHLC** in the realdata pipeline;
  no new data sourcing is required for a strong baseline.
- **GARCH(1,1) on crypto:** widely benchmarked; pure-Rust impls exist
  (`rust-quant` v0.0.10 has GARCH-family fitters; alternatively
  ~50 LoC of hand-rolled MLE). Wall-clock per fit on 8760 bars is
  seconds.
- **Vol-targeting Sharpe lift:** equity factor research (Moreira &
  Muir 2017 *Volatility-Managed Portfolios* — Journal of Finance)
  reports Sharpe improvements of 0.15-0.40 on momentum + value factor
  portfolios via constant-vol scaling. Crypto-specific evidence is
  thinner (limited published benchmarks) — this feature provides the
  empirical answer.
- **Transaction-cost drag:** vol-targeting introduces **per-symbol
  rebalancing turnover** whenever σ̂ moves. The Sharpe lift only
  realises if rebalancing turnover stays bounded; this is the
  load-bearing empirical question for crypto-at-hourly-cadence.

## Carry-forward invariants

Per
[`spec/architecture/12-forecast-overlay.md`](../architecture/12-forecast-overlay.md)
and shared infrastructure already on disk:

- **Same data:** 10 USDT pairs
  (ADA/AVAX/BNB/BTC/DOGE/DOT/ETH/LINK/SOL/XRP), hourly OHLCV,
  2023 + 2024 full year, bootstrapped via
  [`crates/data/src/bin/fetch_binance_klines.rs`](../../crates/data/src/bin/fetch_binance_klines.rs).
  No new data sourcing required for the analyst-default vol-target
  shape (Parkinson + GARCH operate on high/low/close already in the
  parquet store). Optional Q-DATA expansion: 1-minute bars for true
  realized-vol — deferred unless operator escalates.
- **Same backtest scenarios:** BS-1 (2023 full-year top-10 USDT
  hourly), BS-2 (2024 full-year top-10 USDT hourly). The vol forecaster
  evaluates on the same scenario surface as v2.5 for apples-to-apples
  Sharpe-delta vs v1 momentum baseline.
- **`ForecastProvider` trait reuse — proposed extension:** the
  existing trait emits a `ForecastResponse { overlay:
  ForecastOverlay { direction, confidence }, … }`. The analyst
  proposes (architect-confirm at M-T1) an **additive enum variant**
  shape — either (a) a sibling `VolForecastProvider` trait with the
  same async surface but a `VolResponse { sigma_hat, confidence }`
  return type, or (b) extend `ForecastOverlay` with an optional
  `vol_hint: Option<VolHint>` field carrying σ̂ alongside the
  existing direction. Both are anchor-byte-safe by construction
  (additive). Default (a) — sibling trait — because the consumer
  shape is risk-level, not signal-level, so the dispatch path is
  cleaner separated. See Q4.
- **Same audit shape:** `JournalEntry { kind: "vol_forecast_emitted", … }`
  with `model_revision` SHA pinned per ADR-0029 (canonical-arch
  descriptor extended additively).
- **Same hardware constraint:** Apple Silicon M-series via candle
  Metal backend (if DL refinement lands). GARCH(1,1) baseline runs on
  CPU in seconds.
- **ML framework:** candle for DL refinement (per ADR-0028, covers
  vol forecasters explicitly); pure-Rust GARCH via `rust-quant` or
  hand-rolled.
- **σ_train contract:** ADR-0035 § D1 post-training σ_train pattern
  applies **if and only if** the DL refinement uses a confidence-gate
  shape analogous to v2.5. Vol forecasters typically do **not** need
  σ_train gating in the same way (the model output `σ̂` IS the
  scale; there is no division-by-σ_train step). Architect locks at
  M-T1 — see Q4. If σ_train is not load-bearing, R10's σ_train test
  is moot for this feature.
- **F-verdict reuse vs new V-verdict:** ADR-0033 § D3 was IMMUTABLE
  for return-target forecasters. Vol forecasters need a different
  verdict shape (calibration: predicted vs realized; QLIKE / MSE / MAE
  of log-σ). Surface as Q5: extend ADR-0033 § D3 with a vol-classifier
  branch OR write new ADR-0038. Analyst default: **(b) new ADR-0038
  V-verdict** — keeps ADR-0033 immutable per the retrospective's
  lesson #2 ("F-verdict immutability locked a comparable measurement
  bar"); ADR-0038 mirrors the priority-tree shape for the new task.

## Requirements (R1-R12)

> **MVP scope** — GARCH(1,1) baseline + (conditional on Q2) small DL
> refinement + V-verdict calibration report + vol-targeting overlay on
> v1 momentum + Sharpe-comparison report + sibling strategy/risk
> integration + 3-4 anchors under `v3.0.0-volatility`. Reach goals
> (multi-horizon vol curve, kill-switch builder, BS-2 anchor) defer to
> v0.1.1. The v0.1.0 ship is operator-decide-bounded by Q1-Q6 below;
> all defaults are analyst-recommended; "autoapprove" activates them.

### R1 — Vol target derivation (closes Q1)

The training and evaluation pipeline produces a per-window vol target.
Implementation extends
[`crates/forecast/src/features.rs:489,627-628`](../../crates/forecast/src/features.rs)
with an additional target derivation alongside the existing
log-return target:

- **Analyst-default (Q1=(b)):** **Parkinson estimator** on the next
  `target_horizon_bars` bars:

  ```rust
  // For window ending at bar t, target = Parkinson σ over bars t+1..t+H.
  let mut sum_sq = 0.0_f64;
  for k in 1..=H {
      let bar = &bars[t + k];
      let ln_hl = (bar.high / bar.low).ln();
      sum_sq += ln_hl * ln_hl;
  }
  let parkinson_sigma = ((1.0 / (4.0 * f64::ln(2.0))) * (sum_sq / H as f64)).sqrt();
  ```

  - Pros: reuses existing high/low columns (zero new data sourcing);
    5-7× more sample-efficient than close-to-close realized-vol;
    well-studied in 40+ years of derivatives literature.
  - Cons: assumes Brownian motion (zero-drift); biased downward in
    high-momentum regimes — acceptable for hourly crypto where drift
    is small relative to hourly std.
- **Q1=(a) alternative — realized-vol from close-to-close returns:**
  `σ̂ = std({r_{t+1..t+H}})` where `r_k = ln(close_k / close_{k-1})`.
  Simpler; well-understood; matches the v2.5 5-feature input's
  `logret` channel. Less sample-efficient than (b); needs H ≥ 24 for
  stable estimates on hourly bars.
- **Q1=(c) alternative — multi-horizon vol curve:** emit `{σ̂_1h,
  σ̂_4h, σ̂_24h}` as 3 separate scalar outputs. **Analyst rejects
  for v0.1.0** — triples the output head + complicates calibration
  evaluation; defer to v0.1.1 if v0.1.0 finishes ALPHA-UNLOCKED.

The `target_horizon_bars` field stays load-bearing (1 / 4 / 24 bar
horizons all valid under either Q1=(a) or Q1=(b)). Default
`target_horizon_bars = 24` per Q1 + Q6 — mirrors the PatchTST 24h
convention and gives stable Parkinson estimates over a full session.

### R2 — GARCH(1,1) baseline (closes Q2 baseline branch)

A pure-Rust GARCH(1,1) fitter lands as either (a) a new bin
`crates/forecast/src/bin/train_garch.rs` or (b) inside the existing
forecast crate as `crates/forecast/src/garch.rs`. Architect-decide at
M-T1.

- **Model:** classical GARCH(1,1):

  ```text
  σ²_t = ω + α * r²_{t-1} + β * σ²_{t-1}
  ```

  Three parameters per symbol (ω, α, β) fit via maximum likelihood
  (closed-form gradient available; convergence in <100 iterations
  per symbol with quasi-Newton).
- **Per-symbol fit:** the GARCH parameters are **per-symbol** (10
  fits total for the top-10 universe). Pooled-fit alternative
  rejected in v0.1.0 — adds scope without clear EV.
- **Hyperparameters:** ω initial 1e-6; α initial 0.1; β initial 0.85;
  convergence tolerance 1e-8; max iterations 500. Architect locks
  at M-T1.
- **Crate choice (architect-decide at M-T1):**
  - `rust-quant` v0.0.10 has GARCH-family fitters per the survey;
    license check + integration cost at M-T1.
  - Hand-rolled MLE in ~80-120 LoC of pure Rust; zero new
    dependency. Analyst-default if `rust-quant`'s API surface
    isn't a clean fit.
- **Outputs:** for each symbol, persist `(omega, alpha, beta,
  unconditional_var)` to
  `crates/forecast/checkpoints/anchors/garch-bs1-<sha>.json`. The
  per-symbol params are tiny (~120 bytes total); no safetensors
  involved; the JSON file itself is the checkpoint.
- **Determinism:** identical fit across 2 runs given identical seeds
  + identical input data (tester verifies at M-FINAL).

### R3 — Conditional DL refinement (closes Q2 DL branch)

**Conditional on Q2 = (a) ensemble or (b) DL-refinement only.** Under
Q2 = (c) GARCH-only-MVP, R3 is empty and v0.1.0 ships GARCH-only.

If DL refinement lands, the model targets `log(σ̂_realized)` (log
keeps the output unbounded and well-conditioned; matches the
literature). Three architecture options surfaced as Q3:

- **Q3=(a) — small TCN-shape (~100k params):** mirror of the v2.5
  TCN scaffold at smaller scale. 4 blocks, 32 channels, dilation
  [1,2,4,8]. Receptive field ~256 bars (~10 days). Reuses
  `crates/forecast/src/tcn.rs` patterns verbatim with halved depth +
  channels.
- **Q3=(b) — LSTM (~50k params):** classical recurrent baseline.
  Single layer, hidden 64, sequence length 168 (1 week of hourly
  context). Less paradigm-overlap with v2.5; cheap to train.
- **Q3=(c) — PatchTST-shape (~150k params):** paradigm carry-forward
  with smaller config (d_model=64, n_heads=2, n_layers=2). Reuses
  the M-T1-shipped PatchTST module from `crates/forecast/src/patchtst.rs`
  with smaller config.
- **Q3=(d) — GARCH-only (no DL refinement):** v0.1.0 ships GARCH
  baseline + vol-targeting overlay + V-verdict report; defer DL
  refinement to v0.1.1 if GARCH alone clears H2. **Analyst-
  recommended default** per cheap-first lesson #1 (retrospective).

The **cheap-first hypothesis** (retrospective lesson #1) argues
strongly for Q3=(d) — ship GARCH-only in v0.1.0 and let the empirical
GARCH-vs-vol-targeting Sharpe result decide whether DL refinement is
worth the marginal training cost. Q3=(d) is also the only branch that
keeps v0.1.0 ship inside ~3-4 weeks; (a)/(b)/(c) push to 4-6 weeks
total (still inside the operator's 6-8 week cap but eats parallel
budget for C2/C5 analyst passes).

### R4 — Vol forecaster trait + impl (closes Q4)

A new file `crates/forecast/src/vol.rs` lands as a sibling to
`tcn.rs` / `patchtst.rs`:

- **Trait shape (Q4=(a) analyst default — sibling trait):**

  ```rust
  #[async_trait]
  pub trait VolForecastProvider: Send + Sync {
      async fn forecast_vol(
          &self,
          req: VolRequest,
      ) -> Result<VolResponse, ForecastError>;
  }

  pub struct VolRequest {
      pub symbol: Symbol,
      pub timestamp: Timestamp,
      pub context_bars: Vec<OhlcvBar>,  // matches v2.5 context shape
  }

  pub struct VolResponse {
      pub sigma_hat: f64,         // predicted σ for horizon H
      pub horizon_bars: u32,      // H (configurable per fit)
      pub confidence: Confidence, // calibration-derived (R5)
      pub model_revision: Sha256, // per ADR-0029
  }
  ```

- **Q4=(b) alternative — extend `ForecastOverlay`:** add
  `vol_hint: Option<VolHint>` field. Analyst rejects — couples the
  vol path to the retired direction overlay path; consumers would
  have to thread the optional vol through `combine()` which is
  unused-for-vol.
- **GARCH impl:** `GarchVolForecaster::load_anchor(scenario)` loads
  per-symbol `(ω, α, β)` from the JSON checkpoint; `forecast_vol()`
  runs one GARCH(1,1) recurrence step given the symbol's last `(r_t,
  σ_t)`. Pure computation; sub-microsecond per call.
- **DL impl (conditional on Q3 ≠ (d)):** `DlVolForecaster` over the
  chosen architecture from R3.
- **Replay-cache wiring:** vol forecast determinism is load-bearing
  for backtest reproducibility. `crates/replay-cache` extends with a
  namespace `"vol_forecast"` (additive; the existing `"forecast"`
  namespace stays byte-identical). Architect-confirm at M-T1.

### R5 — V-verdict algorithm + ADR-0038 (closes Q5)

> **ADR territory.** Q5 picks between extending ADR-0033 § D3 with a
> vol-classifier branch OR writing new ADR-0038. Analyst default
> **(b) ADR-0038 NEW**.

A new ADR `spec/architecture/adr/0038-vol-forecast-verdict-shape.md`
codifies the V-verdict algorithm — a priority tree over calibration
metrics, mirroring ADR-0033 § D3's shape but with vol-specific
inputs.

**Proposed V-verdict shape (architect locks at M-T1; ships as
ADR-0038):**

```text
                    Per-symbol vol forecast over BS-1 (2023):
                        σ̂_t,s vs σ_realized,t,s

  V1 — Constant collapse        (σ̂ ≡ unconditional_var across all t,s)
  V2 — Per-symbol mis-fit       (QLIKE varies > 3× across symbols
                                  ⇒ universal hyperparameters wrong)
  V3 — Calibration drift        (mean(σ̂)/mean(σ_realized) outside
                                  [0.7, 1.4] ⇒ systematic bias)
  V4 — No improvement over GARCH (QLIKE_DL > QLIKE_GARCH by < 5%
                                  ⇒ DL adds no value; ship GARCH-only)
  V5 — DL improves over GARCH    (QLIKE_DL < QLIKE_GARCH by ≥ 5%
                                  ⇒ ship DL refinement)
  V_ALPHA — Vol-target Sharpe-delta ≥ +0.10 (the strategy-side gate
                                              parallel to F4's M-SHARPE)
```

- **QLIKE** (Patton 2011 *Volatility forecast comparison using
  imperfect volatility proxies*) is the standard vol-forecast loss:
  `QLIKE(σ̂, σ_realized) = σ_realized/σ̂ - ln(σ_realized/σ̂) - 1`.
  Robust to noise in the realized-vol proxy; preferred over MSE
  for vol forecasts.
- **Per-symbol breakdown** is load-bearing (vol forecasts that work
  on BTC may fail on lower-cap pairs; the V2 trigger surfaces this).
- **GARCH baseline as floor:** V4 / V5 fire iff the DL refinement
  shipped (R3 active). Under Q3=(d) GARCH-only, only V1/V2/V3 + V_ALPHA
  fire.
- **Mutual exclusivity:** architect codifies a unit test asserting
  V1-V5 are mutually exclusive (precedent from ADR-0033 § D3 unit
  test).

**Bin: `crates/forecast/src/bin/vol_verdict.rs`** — sibling of
`forecast_distribution.rs`. Emits `vol-verdict-bs1-realdata-<date>.md`
under `spec/v3-volatility-forecaster/reports/`. Report body shape per
ADR-0038 § D2 (per-symbol QLIKE table + calibration scatter + V-verdict
section + follow-on routing per V1-V5).

### R6 — Vol-targeting overlay strategy (closes Q3 consumer)

> **Analyst default Q3=(d) — all 3 builders shipped as opt-in:**
> vol-targeting overlay, kill-switch overlay, standalone strategy.
> Operator can ship 1 / 2 / 3 of these via the Q3 sub-choices.

#### R6.a — Vol-targeting overlay on v1 momentum (primary deliverable)

A new file `crates/strategy/src/vol_targeting_overlay.rs` lands as a
sibling to `momentum.rs`:

```rust
pub struct VolTargetingOverlay<S: Strategy> {
    inner: S,                          // wrapped v1 momentum
    vol_provider: Arc<dyn VolForecastProvider>,
    target_vol: f64,                   // e.g. 0.02 (daily-equivalent)
    scale_clamp: (f64, f64),           // e.g. (0.5, 2.0)
}

impl<S: Strategy> Strategy for VolTargetingOverlay<S> {
    fn on_bar(&mut self, bar: &Bar, state: &mut State) -> Vec<Order> {
        let base_orders = self.inner.on_bar(bar, state);
        let sigma_hat = block_on(self.vol_provider.forecast_vol(...));
        let raw_scale = self.target_vol / sigma_hat;
        let clamped = raw_scale.clamp(self.scale_clamp.0, self.scale_clamp.1);
        base_orders.into_iter().map(|o| o.scale_quantity(clamped)).collect()
    }
}
```

- **Composition:** vol-targeting wraps the v1 momentum baseline
  without modifying it. Existing `MomentumStrategy` builder stays
  byte-identical (load-bearing for the 4 existing v1 anchors).
- **Scale clamp:** [0.5, 2.0] default per the survey; rebalancing
  turnover bounded by the clamp width. Operator-decide at Q3-sub.
- **target_vol parameter:** default `0.02` (daily-equivalent
  log-return std target). Architect-confirm at M-T1; operator-
  decide via Q3-sub.

#### R6.b — Vol-kill-switch overlay (secondary deliverable)

```rust
pub struct VolKillSwitchOverlay<S: Strategy> {
    inner: S,
    vol_provider: Arc<dyn VolForecastProvider>,
    threshold: f64,                    // e.g. 3.0× per-symbol median σ̂
    cooldown_bars: u32,                // e.g. 4 (4h cooldown after re-entry)
}
```

When σ̂ crosses `threshold × historical_median(σ̂)`, the strategy
flat-lines exposure on that symbol until σ̂ falls back inside the
band for `cooldown_bars`.

#### R6.c — Standalone vol-meanreversion strategy (tertiary deliverable)

A standalone `VolMeanReversionStrategy` that goes long when
`σ̂_realized > σ̂_predicted` (vol surprise; expect reversion) and
flat when `σ̂_predicted > σ̂_realized` (well-anticipated vol; no edge).
Defers cleanly to v0.1.1 if Q3-sub picks vol-targeting only.

### R7 — Backtest scenario integration (closes Q-anchors)

Two new scenarios mirror the v2.5 TCN convention per ADR-0032:

- **`top10-2023-fy-vol-target-overlay-realdata`** — BS-1 backtest of
  the v1 momentum baseline + `VolTargetingOverlay` per R6.a.
- **(Optional, Q-anchors=(a))**
  **`top10-2023-fy-vol-killswitch-overlay-realdata`** — BS-1 backtest
  of the v1 momentum baseline + `VolKillSwitchOverlay` per R6.b.

Emits a `top10-2023-fy-vol-target-overlay-realdata-<date>.md` report
under `spec/v3-volatility-forecaster/reports/`. Report body bytes
deterministic across 2-run byte-identity for anchor-lock at M-FINAL.

### R8 — Sharpe-comparison report

Extends the existing
[`crates/forecast/src/bin/sharpe_comparison.rs`](../../crates/forecast/src/bin/sharpe_comparison.rs)
with a `--scenario vol-target-bs1` dispatch (additive enum variant).
Emits `sharpe-comparison-vol-target-bs1-realdata-<date>.md` under
`spec/v3-volatility-forecaster/reports/`.

**T-classifier verdict** (analyst proposes; architect locks at M-T1):

- `T-VOL-ALPHA-UNLOCKED` — Sharpe-delta ≥ +0.10 vs un-targeted v1
  baseline.
- `T-VOL-MARGINAL` — Sharpe-delta in [+0.05, +0.10).
- `T-VOL-NO-ALPHA` — Sharpe-delta < +0.05 OR drawdown widens vs
  baseline.

Sharpe-delta is computed both **gross** (no fee adjustment, apples-to-
apples with v1 momentum baseline anchor) and **net of vol-targeting
turnover** (per-symbol rebalancing cost at venue-bid-ask). The two
results are reported side-by-side; net-of-turnover is the gating
metric.

### R9 — Watch recipe for long-running training (per MEMORY.md)

If Q3 ≠ (d) and a DL training run kicks off, the developer at M-D
MUST emit a copy-pasteable `watch -n 60 '<probe>'` block per the
operator's `MEMORY.md` directive:

```bash
# Vol DL training progress (replace <PID> with cargo PID via `pgrep -f train_vol_dl`)
watch -n 60 'tail -30 /tmp/train_vol_dl-bs1.log && \
             echo "---" && \
             ps -p <PID> -o pcpu,pmem,etime,command | tail -2 && \
             echo "---" && \
             ls -lh crates/forecast/checkpoints/anchors/vol-*-bs1-*.safetensors 2>/dev/null || echo "(checkpoint not yet written)"'
```

GARCH(1,1) fit is sub-second per symbol; the watch recipe is only
load-bearing if R3 (DL refinement) ships.

### R10 — Non-regression contract (load-bearing)

This ship is **anchor-additive**. The **30 v2.5-chain anchors stay
byte-identical**:

- 22 pre-recalibrate anchors (19 pre-investigation + 3
  alpha-investigation).
- 4 v2.6.1-alpha-investigation-recalibrated anchors.
- 2 v2.6.2-threshold-tuning anchors.
- 2 v2.5a.0-patchtst anchors.

Specifically:

- The existing 30 body-SHAs stay byte-identical. `verify_anchors.sh`
  reports `28 PASS + 2 known-FAIL` PRE-lock (current baseline post
  v25a-patchtst-overlay ship; 30 anchors total with 2 pre-existing
  glob-collision known-FAILs). POST-lock reports
  `28 + N_new PASS + 2 known-FAIL` where N_new is the count of new
  anchors picked under Q-anchors (3 or 4 — see § Q-anchors).
- The existing `tcn-bs{1,2}-*.{safetensors,metadata.json,metadata.recalibrated.json}`
  files stay byte-identical.
- The existing `patchtst-bs1-*.{safetensors,metadata.json}` files
  stay byte-identical.
- The 8 existing TCN strategy builders + 2 PatchTST strategy builders
  stay byte-identical. New `with_vol_*` builders are ADDITIVE.
- The existing `forecast_distribution` bin + `sharpe_comparison` bin
  stay backward-compatible (additive vol dispatch is the only
  change; existing dispatch byte-identical).
- The existing `recalibrate_sigma_train` bin stays byte-identical
  (unused by GARCH; potentially used by DL refinement under Q3 ≠
  (d) — see R3 — but the bin is model-agnostic per ADR-0035 § D1).
- No iced bump (operator-locked per CLAUDE.md).
- No new external crate dependency unless Q-crate picks
  `rust-quant` for GARCH; analyst-default is hand-rolled (zero new
  dep).

### R11 — Verification gates

Tester confirms at M-FINAL:

1. `cargo fmt --check` + `cargo clippy --workspace -- -D warnings`
   PASS.
2. `cargo clippy -p forecast --features candle -- -D warnings` PASS
   (only if R3 ships DL refinement).
3. `cargo test --workspace --lib` PASS, 0 failures.
4. `cargo test -p forecast --test garch_fit_determinism` PASS — 2-run
   byte-identity of per-symbol `(ω, α, β)` JSON outputs.
5. `cargo test -p forecast --test vol_verdict_mutual_exclusivity`
   PASS — V1-V5 priority tree mutually exclusive (per ADR-0038 § D3
   precedent inherited from ADR-0033 § D3).
6. `cargo test -p strategy --test vol_targeting_overlay` PASS —
   overlay wraps inner strategy correctly + scale clamp invariants
   hold.
7. `cargo test -p forecast --test tcn_byte_identity` PASS — K-vol-3
   scope-creep guard (TCN files stay byte-identical).
8. `cargo test -p forecast --test patchtst_byte_identity` PASS —
   K-vol-3 extension (PatchTST files stay byte-identical).
9. 2-run byte-identity determinism gate on the new
   `vol-verdict-bs1-realdata-*.md` report.
10. 2-run byte-identity determinism gate on the new
    `top10-2023-fy-vol-target-overlay-realdata-*.md` report.
11. `bash scripts/verify_anchors.sh` reports `28 PASS + 2 known-FAIL`
    PRE; `28 + N_new PASS + 2 known-FAIL` POST. The 30 originals
    stay byte-identical.
12. `uv run scripts/spec_lint.py` matches the baseline (0 new
    categories).

### R12 — Risk-engine integration (load-bearing per architect lock)

> **K-vol-2 critical.** The survey flagged the risk-level overlay vs
> signal-level overlay as the only sticky ADR amendment. Architect at
> M-T1 locks where vol forecasts compose:
>
> - **Option A — Strategy-side composition** (`VolTargetingOverlay`
>   wraps the inner strategy; vol scaling happens inside `on_bar`).
>   Analyst default. Cleanest separation; no risk-engine refactor;
>   reuses existing `Strategy` trait.
> - **Option B — Risk-engine input** (vol forecast feeds the
>   risk-engine telemetry surface — the analyst notes the brief's
>   reference to `crates/cost/src/risk_state.rs` is **stale**: the
>   actual cost-crate surface is `crates/cost/src/budget.rs`. The
>   risk-engine pathway exists in `crates/risk/` if it has landed;
>   architect verifies at M-T1).
> - **Option C — Both** — vol forecast feeds both strategy-side
>   composition AND risk-engine telemetry. Operator-decide expansion;
>   defer the second consumer to v0.1.1.

Architect at M-T1 verifies the actual paths (the analyst brief
references `risk_state.rs` which does NOT exist on disk at
2026-05-22; closest is `crates/cost/src/budget.rs`). The
non-regression contract holds regardless: any path that touches
cost / risk crates is additive-only.

## Hypothesis register (H1-H4)

> Each hypothesis is testable; the tester gate closes / falsifies it.
> Analyst proposes; architect locks at M-T1.

### H1 — Small DL vol forecaster beats GARCH(1,1) baseline by ≥5% on QLIKE

**Statement.** Under Q3 ≠ (d), the chosen DL architecture trained on
the same span as the GARCH baseline produces a per-symbol QLIKE that
is ≥5% lower (better) than GARCH(1,1) on at least 7 of 10 symbols on
the BS-1 holdout split.

**Test.** R5 V-verdict bin emits per-symbol QLIKE table; V5 fires
iff the 5% improvement is met.

**Confidence at brief time:** **MEDIUM.** Crypto vol literature on
DL-vs-GARCH is mixed; some papers (Liu 2019, Petrozziello 2022) show
2-8% QLIKE improvement, others show DL adds noise without
information. Under Q3 = (d) GARCH-only-MVP, H1 is **moot** (the v0.1.0
ship skips DL refinement; v0.1.1 can run H1 as a follow-on).

### H2 — Vol-targeting overlay on v1 momentum extracts +0.10 Sharpe-delta vs un-targeted v1 baseline

**Statement.** The v1 cross-sectional momentum baseline + R6.a
`VolTargetingOverlay` over the GARCH (or DL — Q3-dependent) vol
forecaster scores a Sharpe-delta ≥ +0.10 vs the existing v1 baseline
anchor (`top10-2023-1h-momentum`) on BS-1 realdata, net of
vol-targeting rebalancing turnover cost.

**Test.** R8 Sharpe-comparison bin emits T-classifier verdict per the
+0.10 gate.

**Confidence at brief time:** **MEDIUM-HIGH.** Per the survey's
analysis: textbook vol-targeting on momentum delivers 0.1-0.3 Sharpe
lift on equity factor portfolios (Moreira & Muir 2017). Crypto-at-
hourly-cadence is under-studied; the load-bearing empirical question
is whether the per-symbol rebalancing turnover at hourly cadence eats
the lift. The clamp [0.5×, 2×] bounds rebalancing magnitude; analyst-
prior is alpha SURVIVES the turnover net cost.

**This is the alpha-unlock hypothesis for v0.1.0.**

### H3 — 4-6 week ship is feasible (cheap-first hypothesis)

**Statement.** The MVP under Q3 = (d) GARCH-only-MVP ships within
3-4 weeks of M-OD operator approval. Under Q3 ≠ (d), DL refinement
extends to 4-6 weeks total. Both fit inside the operator's 6-8 week
cumulative cap across C1+C2+C5.

| Wave | Q3=(d) GARCH-only | Q3≠(d) GARCH+DL |
|------|-------------------|-----------------|
| M-OD operator-decide Q1-Q6 | minutes (autoapprove) | minutes |
| M-T1 architect lock + ADR-0038 V-verdict | 4-8 hr | 6-12 hr (DL bits add) |
| Wave A — GARCH fitter + vol target derivation + vol forecaster trait + 4-5 unit tests | 3-5 days | 3-5 days |
| Wave B — Per-symbol GARCH fit (10 symbols × seconds) | 30 min | 30 min |
| Wave C — DL training (Q3 ≠ (d) only) | 0 (skipped) | 1-3 days (small model on Metal) |
| Wave D — V-verdict + Sharpe-comparison + vol-targeting overlay + backtest scenario | 2-3 days | 2-3 days |
| Wave E — tester gate (M-FINAL) | 0.5-1 day | 0.5-1 day |
| Wave F — presenter deck (M-PRESENTER) | 0.5 day | 0.5 day |
| **Total wall-clock** | **~2-3 weeks (best case); ~3-4 weeks with one retry** | **~4-6 weeks** |

**Test.** Calendar tracking from M-OD approval to presenter ship
date. H3 is **confirmed** iff total ≤ 4 weeks under Q3=(d), or ≤ 6
weeks under Q3 ≠ (d).

**Confidence at brief time:** **HIGH** under Q3=(d) (GARCH is
textbook; no novel ML; no Metal compute risk). **MEDIUM** under Q3 ≠
(d) — same Apple Silicon constraints as v2.5, but model is ~10×
smaller than PatchTST so training wall-clock should be hours-to-day,
not week.

### H4 — Hourly crypto vol IS predictable

**Statement.** Both GARCH(1,1) and (under Q3 ≠ (d)) the DL refinement
produce per-symbol QLIKE values that meaningfully improve over a
constant-σ baseline (V_NO_PRED below). Equivalently, the V-verdict is
NOT V1 (constant collapse).

**Test.** R5 V-verdict bin emits per-symbol QLIKE vs constant-σ
baseline; H4 confirmed iff every symbol shows ≥10% QLIKE improvement.

**Confidence at brief time:** **HIGH.** Vol clustering on crypto is
empirically well-established (squared-return autocorrelation > 0.15
at lag 1-24h on hourly OHLCV across all top-10 USDT pairs per public
benchmarks). GARCH(1,1) is the textbook capture for this clustering.
**If H4 falsifies, the cheap-first investigation has surfaced a deeper
data-pathology hypothesis** (no vol clustering on this universe) that
forecloses on H1 AND H2 jointly — the analyst's prior is this won't
happen, but the V-verdict makes the falsification fast and cheap.

## Risk register (K-vol-1..K-vol-6)

| Risk | Mitigation |
|------|------------|
| **K-vol-1 — Vol-targeting Sharpe lift eaten by rebalancing turnover.** Per-symbol position-size scaling fires every bar (potentially) on 10 symbols × 8760 bars/year = up to 87,600 rebalancing decisions. Bid-ask spread + slippage at venue (Binance spot: ~5-10 bps round-trip on top pairs) compounds over the year. | R6.a scale clamp [0.5×, 2×] bounds rebalancing magnitude; **gross + net Sharpe** reported side-by-side; T-VOL-NO-ALPHA fires if net Sharpe lift < +0.05. Architect at M-T1 considers turnover-threshold gating (skip rebalancing when scale_change < e.g. 5%). |
| **K-vol-2 — Strategy-side vs risk-engine composition ADR amendment is sticky.** Per survey: vol-targeting is naturally risk-level (scaling position size) rather than signal-level (modulating direction); the architectural choice affects all future strategies, not just this one. | Analyst-default Q3-sub picks strategy-side composition (`VolTargetingOverlay`) — minimises ADR surface. Architect at M-T1 locks via ADR-0038 § Dx or a separate ADR; the analyst's recommendation is "ship strategy-side in v0.1.0; defer risk-engine integration to v0.1.1 if the strategy-side overlay clears H2." This keeps the load-bearing decision reversible. |
| **K-vol-3 — Scope creep into v2.5 forecast crate.** Developer tempted to refactor `forecast::lib` to share more code between TCN / PatchTST / vol forecasters. | Hard analyst boundary: v0.1.0 ships `vol.rs` + `garch.rs` as siblings to `tcn.rs` / `patchtst.rs` — zero refactor of existing files. Architect formalises at M-T1 as unit tests: `git diff HEAD -- crates/forecast/src/{tcn,patchtst}.rs` is empty after the vol ship (R11.7 + R11.8). |
| **K-vol-4 — H4 falsifies (no vol clustering signal on this universe).** Unlikely per established literature, but possible if e.g. the realdata pipeline has a bar-aggregation bug that destroys autocorrelation. | The V-verdict surfaces this fast (V1 constant-collapse on GARCH baseline). If H4 falsifies, the analyst spawns a `v3-data-vol-investigation` follow-on; the vol forecaster work itself is paused. Cheap-to-falsify per the retrospective's lesson #1 (cheap-first investigation order). |
| **K-vol-5 — V-verdict shape disagreement** between analyst proposal (R5) and architect lock (ADR-0038 at M-T1). The analyst proposes V1-V5 + V_ALPHA; architect may collapse or expand the priority tree. | Architect-decide territory; analyst's proposal is **recommended, not mandated**. The lesson from ADR-0033 immutability is that the verdict shape locked at architect-time stays stable across follow-on ships; the analyst flags this and defers the lock. |
| **K-vol-6 — Q3=(d) GARCH-only-MVP under-delivers** (operator wanted DL refinement; the cheap-first analyst-default disappoints). | This is an explicit operator-decide branch (Q3=(d) vs others). Surface as Q3 with analyst default = (d); if operator picks (a)/(b)/(c) DL refinement, R3 lands inside the same 6-8 week cap. The cheap-first read is honest: GARCH is likely sufficient for the H2 alpha-unlock test (textbook precedent), and DL refinement is a v0.1.1 optimisation if v0.1.0 clears the bar. |

## Non-regression contract

This section consolidates the load-bearing invariants the tester
confirms at M-FINAL:

1. **30 anchored body-SHAs byte-identical.** `bash
   scripts/verify_anchors.sh` reports `28 PASS + 2 pre-existing
   glob-collision FAIL` PRE-lock (current baseline post v25a-patchtst
   ship — 30 anchors total with 2 known-FAILs).
   POST-lock the count grows by N_new ∈ {3, 4} per Q-anchors.
2. **Original TCN `.safetensors` + `.metadata.*` files byte-identical.**
3. **Original PatchTST `.safetensors` + `.metadata.json` files
   byte-identical.**
4. **`tcn.rs` body byte-identical.** `git diff HEAD --
   crates/forecast/src/tcn.rs` is empty (modulo comment-only).
5. **`patchtst.rs` body byte-identical.** `git diff HEAD --
   crates/forecast/src/patchtst.rs` is empty (modulo comment-only).
6. **Existing TCN strategy builders byte-identical.**
7. **Existing PatchTST strategy builders byte-identical.**
8. **Existing forecast_distribution bin TCN/PatchTST dispatch
   byte-identical.**
9. **No new external crate dependencies** unless operator picks
   `rust-quant` GARCH via Q-crate. Analyst-default = hand-rolled.
10. **No iced bump.** Operator-locked per CLAUDE.md.
11. **F-verdict algorithm IMMUTABLE.** ADR-0033 § D3 stays unchanged.
    Vol forecasters use the **new V-verdict** per ADR-0038 (Q5
    default), not an extension of ADR-0033.
12. **ADR-0035 σ_train contract carries to DL refinement if Q3 ≠ (d).**
    Under Q3=(d) GARCH-only, σ_train is N/A.
13. **ADR-0029 canonical-arch-descriptor extended additively** for
    GARCH + (conditional) DL vol forecasters. The v2.5 TCN/PatchTST
    `model_revision` SHAs are unchanged.

## Acceptance per milestone

The feature is **done** when all milestones land their gates.

### M-OD — Operator-decide (Q1-Q6 resolved)

> **Soft blocker.** All 6 questions carry analyst-recommended
> defaults. "Autoapprove" activates all defaults. Operator may
> override individual questions; analyst recommends the bundled
> defaults.

1. Q1-Q6 answered by operator (or "autoapprove").
2. Frontmatter flips `status: draft → proposed`, `owner: analyst →
   architect`.

### M-T1 — Architect lock

1. § Design block appended to `feature.md`.
2. `spec/v3-volatility-forecaster/decomp.md` complete with T-D / T-T
   row decomposition into waves A-F.
3. **ADR-0038** written:
   `spec/architecture/adr/0038-vol-forecast-verdict-shape.md`.
   Codifies (D1) V-verdict priority tree (V1-V5 + V_ALPHA); (D2)
   report shape (per-symbol QLIKE table + calibration scatter +
   verdict section + follow-on routing); (D3) mutual-exclusivity
   test contract; (D4) GARCH(1,1) baseline contract (per-symbol fit,
   parameter ranges, convergence tolerance, JSON checkpoint shape);
   (D5) strategy-side vs risk-engine composition decision (vol
   forecaster composes at strategy-side via `VolTargetingOverlay`;
   risk-engine integration deferred).
4. K-vol-3 byte-identity unit tests designed (tcn + patchtst sibling
   files).
5. K-vol-5 V-verdict shape locked.
6. Frontmatter flips `status: proposed → in-progress`, `owner:
   architect → developer`.

### M-D — Developer Waves A-D

1. Wave A: `crates/forecast/src/vol.rs` (trait + types) +
   `crates/forecast/src/garch.rs` (GARCH fitter) + `vol_target
   derivation` extension to `features.rs` + 4-5 unit tests.
2. Wave B: Per-symbol GARCH(1,1) fit on BS-1 span (30 min wall-clock
   total; sub-second per symbol).
3. Wave C: (Q3 ≠ (d) only) DL training run.
4. Wave D: `vol_verdict.rs` bin + `sharpe_comparison` additive
   dispatch + `vol_targeting_overlay.rs` strategy +
   `top10-2023-fy-vol-target-overlay-realdata` backtest scenario.

### M-V-VERDICT — V-verdict report

1. `vol_verdict --scenario garch-bs1` runs against the per-symbol
   GARCH checkpoint.
2. `vol-verdict-bs1-realdata-<date>.md` emitted.
3. V-verdict (V1-V5) per the new ADR-0038 § D1 priority tree
   recorded in the report body.

### M-SHARPE — Real-Binance backtest

1. `backtest --scenario top10-2023-fy-vol-target-overlay-realdata`
   runs.
2. `top10-2023-fy-vol-target-overlay-realdata-<date>.md` emitted.
3. T-classifier verdict (T-VOL-ALPHA-UNLOCKED / T-VOL-MARGINAL /
   T-VOL-NO-ALPHA) recorded — both gross + net of turnover.
4. `sharpe-comparison-vol-target-bs1-realdata-<date>.md` emitted.

### M-FINAL — Tester gate

R11's 12 gates land green. Joint advisory verdict (V-verdict +
T-classifier) recorded in `feature.md § Verification`.

### M-PRESENTER — Operator approval

1. Presenter deck under
   `spec/v3-volatility-forecaster/presentations/v3-volatility-forecaster-<YYYY-MM-DD>.md`
   carrying joint advisory verdict + recommended next routing:
   - **T-VOL-ALPHA-UNLOCKED** → ship; promote C2 + C5 from
     analyst-only to active development; the hybrid sequence
     succeeded.
   - **T-VOL-MARGINAL** → spawn `v3-vol-target-tuning` feature
     (parallel to v25-tcn-threshold-tuning) for clamp / threshold
     sweep.
   - **T-VOL-NO-ALPHA** → analyst spawn for the C1 retirement
     decision; consider routing budget to C2 (regime) which now has
     a parallel-authored analyst pass ready.
2. Operator ticks approval. Frontmatter flips `status: in-progress
   → shipped`.
3. Trace row `REQ-V3-VOL-FORECASTER-001` flips state.
4. Backlog entry moved Active → Recent.

## Open questions (Q1-Q6 — operator-decide)

> **All 6 questions carry analyst-recommended defaults. "Autoapprove"
> activates all defaults.** The default bundle is internally
> consistent (Q1=(b) Parkinson + Q2=(c) GARCH-only-MVP + Q3=(d)
> all-3-builders + Q4=(a) sibling trait + Q5=(b) ADR-0038 NEW + Q6=(a)
> BS-1 train-span reinforce each other for the cheap-first 3-4 week
> ship).

### Q1 — What to predict

Which vol target?

- **(a)** **1h realized vol** — `σ̂ = std({r_{t+1..t+H}})` over a
  rolling window. Simplest; matches the v2.5 5-feature `logret`
  channel. Needs H ≥ 24 for stable estimates on hourly bars.
- **(b)** **Parkinson estimator over the next H bars** — reuses
  existing high/low columns; 5-7× more sample-efficient than (a) per
  Parkinson 1980; well-studied in derivatives literature.
  **Analyst-recommended default.**
- **(c)** **GARCH(1,1) baseline + small DL refinement** — predict
  σ²_t directly from the recursive GARCH formula; DL refines the
  parametric prior. Tighter coupling between R2 + R3.
- **(d)** **Multi-horizon vol curve** — emit `{σ̂_1h, σ̂_4h,
  σ̂_24h}` as 3 scalar outputs. **Analyst rejects for v0.1.0** —
  triples the output head + complicates V-verdict; defer to v0.1.1.

**Analyst default: (b) Parkinson.** Cheapest path to a strong
calibration baseline; reuses existing data columns; no new sourcing.

### Q2 — Architecture choice

What model family?

- **(a)** **GARCH(1,1) only (no DL)** — pure classical baseline;
  per-symbol fit in seconds. Cheapest possible path; **Analyst-
  recommended default for v0.1.0 MVP per cheap-first hypothesis**
  (retrospective lesson #1).
- **(b)** **Small TCN-shape (~100k params)** — paradigm carry-forward
  at smaller scale.
- **(c)** **LSTM (~50k params)** — classical recurrent baseline; less
  paradigm-overlap with v2.5 retired direction TCN.
- **(d)** **PatchTST-shape (~150k params)** — reuses M-T1-shipped
  PatchTST module at smaller config.
- **(e)** **Ensemble (a)+(b) or (a)+(c)** — GARCH parametric prior +
  DL refinement. **Analyst considers strong alternative**; defers
  to v0.1.1 if v0.1.0 GARCH-only finishes T-VOL-MARGINAL.

**Analyst default: (a) GARCH-only.** Per cheap-first lesson #1: ship
the textbook baseline; let the empirical Sharpe result decide
whether DL refinement is worth the marginal training cost. (a) is the
only branch that keeps v0.1.0 inside ~3 weeks; (b)/(c)/(d) extend to
4-6 weeks (still inside the 6-8 week cap but eats parallel budget
for C2/C5 analyst passes).

### Q3 — Consumer shape

Where does the vol forecast plug in?

- **(a)** **Standalone Strategy** — `VolMeanReversionStrategy` emits
  position size from `1 - σ̂_realized / σ̂_predicted`. New strategy
  primitive.
- **(b)** **Overlay on v1 momentum** — `VolTargetingOverlay` scales
  per-symbol position by `target_vol / σ̂` clipped to a band. The
  textbook precedent (Moreira & Muir 2017); most-likely-positive
  alpha-unlock test.
- **(c)** **Risk-engine input** — vol forecast feeds the risk-engine
  telemetry; kill-switch fires above threshold. Architectural change
  to the risk path.
- **(d)** **All 3 as opt-in builders** — strategy code ships all
  three; the backtest scenario picks one. **Analyst-recommended
  default.** Marginal LoC cost is small; operator-decide on Q3-sub
  picks the primary anchor target.

**Analyst default: (d) all 3, with primary anchor target = (b) vol-
targeting overlay** (the textbook H2 alpha-unlock test). Q3-sub
operator-decide picks the per-symbol band ([0.5×, 2×] default),
target_vol parameter (0.02 daily-equivalent default), and
kill-switch threshold (3× median default).

### Q4 — Verdict shape (ADR amendment)

How does the vol-forecast verdict relate to the immutable ADR-0033
F-verdict?

- **(a)** **Extend ADR-0033 § D3** with a vol-classifier branch.
  Adds V-verdict variants alongside F-verdict variants in a single
  priority tree. **Analyst rejects** — ADR-0033 is IMMUTABLE per the
  retrospective's lesson #2; mutating it breaks the "comparable
  measurement bar" property across v2.5 + v3 evidence.
- **(b)** **Write new ADR-0038** with parallel V-verdict shape
  (V1-V5 priority tree). Mirrors the ADR-0033 § D3 structure for the
  new task; keeps F-verdict immutable. **Analyst-recommended
  default.**
- **(c)** **Embed verdict shape in feature.md § R5 without an ADR.**
  Cheap; minimises ADR surface; but loses the cross-feature
  immutability anchor. Analyst-considered-borderline.

**Analyst default: (b) ADR-0038 NEW.** Keeps ADR-0033 immutable;
mirrors the priority-tree shape; provides a stable verdict for future
vol-forecaster ships (e.g. v0.1.1 DL refinement, v0.2.0 multi-horizon).

### Q5 — Anchor strategy + version pin

Anchor naming + version?

- **(a)** **Anchor under version `v3.0.0-volatility`** with naming
  `{report-family}-vol-target-bs1-realdata` (e.g.
  `vol-verdict-bs1-realdata`,
  `top10-2023-fy-vol-target-overlay-realdata`). Existing 30 anchors
  byte-identical. **Analyst-recommended default.**
- **(b)** Anchor under `v2.7.0-volatility` to keep numbering close to
  the v2.5 chain. **Analyst rejects** — vol forecasting is a new
  strategy lane, not a v2.x continuation; v3.0.0 signals the lane
  shift cleanly per the operator's framing.
- **(c)** Wait until C1+C2+C5 all ship and anchor jointly. **Analyst
  rejects** — leaves v0.1.0 un-anchored; no determinism gate.

**Analyst default: (a) `v3.0.0-volatility`.** Mirrors the
`v2.5a.0-patchtst` naming convention; signals the strategy-lane
shift; anchor-additive by construction.

**Q-anchors sub-question:** how many new anchors land in v0.1.0?

- **N_new = 3:** `vol-verdict-bs1-realdata` +
  `top10-2023-fy-vol-target-overlay-realdata` +
  `sharpe-comparison-vol-target-bs1-realdata`. Minimum viable
  anchor set.
- **N_new = 4:** add `top10-2023-fy-vol-killswitch-overlay-realdata`
  if Q3 also picks the kill-switch overlay as a v0.1.0 deliverable.

**Analyst default: N_new = 3.** Kill-switch defers to v0.1.1 unless
operator overrides at Q3-sub.

### Q6 — Training-data span

Which calendar span for the GARCH fit (and DL refinement if Q2 ≠ (a))?

- **(a)** **BS-1 train + BS-2 val** — mirror v2.5 convention: fit
  GARCH on 2023-01-01..2023-12-31; validate on 2024-01-01..2024-12-31.
  **Analyst-recommended default.** Apples-to-apples vs v2.5 evidence;
  honors the per-checkpoint convention.
- **(b)** **Walk-forward fit** — re-fit GARCH every N bars (e.g.
  weekly); model parameters drift over time. **Analyst considers
  strong alternative**; defers to v0.1.1 — adds scope; the v0.1.0
  BS-1 fixed-fit is the cleanest baseline.
- **(c)** **Full 2-year span fit** — single GARCH fit on
  2023-01-01..2024-12-31; evaluate on the same. **Analyst rejects** —
  no OOS holdout; circular.

**Analyst default: (a) BS-1 train + BS-2 val.** Maps directly onto
the v2.5 scenario surface; OOS holdout matches Strategy lifecycle
gate (Sharpe > 1.0 on 2y OOS data per product.md).

## Cost estimate (per scope branch)

| Scope branch (operator-decide) | Wall-clock | Owner |
|--------------------------------|------------|-------|
| Author this brief (R1-R12 + H1-H4 + K-vol-1..6 + Q1-Q6) | done 2026-05-22 | analyst (this brief) |
| **Q2 = (a) — GARCH-only MVP (analyst-recommended default)** | | |
| Operator-decide Q1-Q6 | minutes (autoapprove) | operator |
| Architect lock + ADR-0038 + decomp.md | 4-8 hr | architect |
| Wave A — vol.rs + garch.rs + features.rs ext + 4-5 unit tests | 3-5 days | developer |
| Wave B — per-symbol GARCH fit (10 × seconds) | 30 min | developer |
| Wave C — skipped (no DL in this branch) | 0 | n/a |
| Wave D — vol_verdict + sharpe_comparison ext + vol_targeting_overlay + backtest scenario | 2-3 days | developer |
| Tester gate + Presenter deck | 1-2 days | tester + presenter |
| **Q2=(a) total** | **~2-3 weeks (best case); ~3-4 weeks with one retry** | |
| **Q2 = (b)/(c)/(d) — GARCH + DL refinement** | | |
| As Q2=(a) + DL training (1-3 days) + ~1 day extra Wave A | +2-4 days | developer |
| **Q2=(b/c/d) total** | **~4-6 weeks (best case); ~5-7 weeks with retry** | |
| **Q2 = (e) — Ensemble GARCH + DL** | | |
| Same as Q2=(b/c/d) + ensemble glue (~0.5 day) | ~+0.5 day | developer |
| **Q2=(e) total** | **~4-6 weeks** | |

**Analyst recommendation: Q2 = (a) GARCH-only-MVP.** Per cheap-first
hypothesis (retrospective lesson #1): ship the textbook baseline;
let the empirical Sharpe result decide whether DL refinement is worth
the marginal training cost. The +0.10 Sharpe-delta H2 unlock test
runs identically under Q2=(a) — the GARCH baseline IS likely
sufficient for vol-targeting to clear the bar. v0.1.1 adds DL
refinement as a follow-on if v0.1.0 finishes T-VOL-MARGINAL.

## Out of scope

- **No BS-2 vol-target backtest scenario in v0.1.0** — defer to
  v0.1.1. Rationale: H2 falsification on BS-1 alone is sufficient to
  route; H2 confirmation on BS-1 makes BS-2 a follow-on commitment
  (mirrors the v25a-patchtst-overlay Q2=(a) decision).
- **No multi-horizon vol curve in v0.1.0** — Q1 default = (b)
  Parkinson at single horizon. v0.1.1 may extend to `{σ̂_1h, σ̂_4h,
  σ̂_24h}` if v0.1.0 finishes T-VOL-MARGINAL.
- **No DL refinement in v0.1.0** — Q2 default = (a) GARCH-only. v0.1.1
  adds Q2=(b/c/d/e) if v0.1.0 finishes T-VOL-MARGINAL or if the
  V-verdict is V4 (DL adds no value over GARCH ⇒ moot) vs V5 (DL
  improves over GARCH ⇒ ship DL).
- **No risk-engine integration in v0.1.0** — Q3 default = (b)
  strategy-side composition (`VolTargetingOverlay`). v0.1.1 integrates
  with the risk-engine telemetry surface if operator picks Q3=(c).
- **No walk-forward GARCH refit in v0.1.0** — Q6 default = (a)
  fixed BS-1 fit. v0.1.1 adds rolling refit if v0.1.0 finishes
  T-VOL-MARGINAL.
- **No 1-minute bar sourcing in v0.1.0** — Q-DATA from the survey
  defers; the Parkinson estimator on hourly OHLC is sufficient.
- **No multi-venue vol forecaster** — Binance-only universe carries
  forward from v2.5; cross-venue vol divergence is a v0.2.0 question.

## References

- [Strategy reformulation survey 2026-05-22](../dev-notes/strategy-reformulation-survey-2026-05-22.md)
  § Candidate 1 (volatility forecasting) — survey-time cost / EV /
  reuse scoping that this brief inherits.
- [v2.5 DL journey retrospective 2026-05-22](../dev-notes/v25-dl-journey-retrospective-2026-05-22.md)
  § Lessons learned + § What the next research direction COULD
  usefully chase — the evidence chain that motivated the pivot.
- [ADR-0028 candle ML framework](../architecture/adr/0028-v25-dl-forecast-overlay-candle.md)
  — covers DL vol refinement under Q2 ≠ (a).
- [ADR-0029 TCN checkpoint provenance](../architecture/adr/0029-tcn-checkpoint-provenance.md)
  — canonical-arch descriptor extended additively for GARCH +
  DL vol forecasters.
- [ADR-0032 backtest realdata path](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)
  — vol-target backtest scenarios inherit realdata path.
- [ADR-0033 F-verdict algorithm](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md)
  — IMMUTABLE; vol forecasters use parallel V-verdict per new
  ADR-0038 (Q4=(b) default).
- [ADR-0035 σ_train recalibration](../architecture/adr/0035-tcn-sigma-train-recalibration.md)
  — applies to DL vol refinement under Q2 ≠ (a); N/A under
  Q2=(a) GARCH-only.
- Parkinson 1980 — *The extreme value method for estimating the
  variance of the rate of return* — Journal of Business 53(1).
- Moreira & Muir 2017 — *Volatility-Managed Portfolios* — Journal
  of Finance 72(4) — vol-targeting on momentum precedent.
- Patton 2011 — *Volatility forecast comparison using imperfect
  volatility proxies* — Journal of Econometrics 160(1) — QLIKE
  loss definition.
- Survey/companion analyst passes (to be authored 2026-05-22 in
  parallel):
  - `spec/v3-regime-classifier/feature.md` (C2)
  - `spec/v3-llm-overlay/feature.md` (C5)

## Design (architect M-T1 — 2026-05-22)

> **M-T1 architect lock closed 2026-05-22.** Full decomposition lives
> at [`spec/v3-volatility-forecaster/decomp.md`](decomp.md); ADR-0038
> codifies the V-verdict shape + GARCH(1,1) baseline contract at
> [`spec/architecture/adr/0038-vol-forecast-verdict-shape.md`](../architecture/adr/0038-vol-forecast-verdict-shape.md).
> The section below summarises the architect-decide resolutions; the
> primary sources are decomp.md + ADR-0038.

### Baseline anchor gate (pre-feature)

`bash scripts/verify_anchors.sh` reports `ANCHORS PASS  (30 / 30)`
on 2026-05-22 (quoted literal output line from the architect's run).
The 30 SHAs stay byte-identical through this ship; N_new=3 added at
M-FINAL per Q5=(a) + Q-anchors-sub=3.

### Architect-decide resolutions (T-AR-1..T-AR-10)

| # | Decision | Source |
|---|----------|--------|
| T-AR-1 | **Hand-rolled GARCH(1,1) MLE** in `crates/forecast/src/garch.rs` (~120 LoC, zero new dep); `rust-quant` v0.0.10 rejected per 4 reasons (no-new-dep + API fit + maintained status + determinism contract) | ADR-0038 § D3; decomp.md § T-AR-1 |
| T-AR-2 | **ADR-0038 NEW** — V1→V2→V3→V4→V5 + V_ALPHA priority tree; parallel to ADR-0033 § D3 (not extension); ADR-0033 stays IMMUTABLE | ADR-0038 § D1 |
| T-AR-3 | **Parkinson target derivation** extends `features.rs:642-656` additively; `VolTargetKind` enum + Optional fields; existing TCN/PatchTST callers byte-identical | decomp.md § T-AR-3 |
| T-AR-4 | **All 3 consumer builders** ship in v0.1.0 per Q3=(d) (`with_garch_vol_strategy` / `with_garch_vol_overlay_momentum` / `with_garch_vol_kill_switch`); primary anchor target = R6.a vol-targeting overlay; kill-switch backtest scenario deferred to v0.1.1 | decomp.md § T-AR-4; ADR-0038 § D5 |
| T-AR-5 | **V-verdict bin** at `crates/forecast/src/bin/vol_verdict.rs` (sibling of `forecast_distribution.rs`, ~280 LoC) | decomp.md § T-AR-5; ADR-0038 § D2.a |
| T-AR-6 | **Backtest scenario** `top10-2023-fy-vol-target-overlay-realdata` lands via new scenarios/garch_vol_target_overlay.rs + additive `ScenarioStrategy::GarchVolTargetOverlayMomentum` variant | decomp.md § T-AR-6 |
| T-AR-7 | **Sharpe-comparison extension** — additive `ScenarioFamily` enum + `--scenario vol-target-bs1` dispatch arm; existing TCN/PatchTST dispatch byte-identical | decomp.md § T-AR-7; ADR-0038 § D2.b |
| T-AR-8 | **5-wave shape**: A ∥ B (parallel-eligible) → C (V-verdict bin) → D (3 builders + scenario + sharpe-ext) → E (tester + presenter). Wave C was former DL-training slot; Q2=(a) GARCH-only drops the slot. | decomp.md § 3 |
| T-AR-9 | **Training cost negligible** (~5-10s wall-clock total for 10 per-symbol GARCH fits). No watch recipe needed (R9 moot under Q2=(a)) | decomp.md § T-AR-9 |
| T-AR-10 | **Wave map + parallelism + rollback shape per wave** documented in decomp.md § 5 (every wave's diff is additive against the previous wave's main) | decomp.md § 5 |

### Risk-engine integration (K-vol-2 lock)

ADR-0038 § D5 locks **strategy-side composition only for v0.1.0**.
Risk-engine integration deferred to v0.1.1 (`crates/cost/src/risk_state.rs`
does NOT exist on disk — closest is `crates/cost/src/budget.rs`;
analyst brief reference was stale). The Q3=(d) kill-switch builder
still ships in v0.1.0 — but as a `Strategy` wrapper
(`VolKillSwitchOverlay<S>`), not a risk-engine hook. The kill-switch
fires inside `on_bar()`, not inside the cost-crate event loop. This
keeps the v0.1.0 ship anchor-additive against `crates/cost/`
(zero modification).

### Replay-cache namespace (D4)

`CacheNamespace::VolForecast` variant lands additively in
`crates/replay-cache/src/lib.rs`. Existing `"forecast"` namespace
byte-identical (v2.5 TCN / v2.5a PatchTST cache entries unchanged).

### Anchor naming (Q5=(a) + Q-anchors-sub=3)

3 new anchors under version `v3.0.0-volatility`:

1. `vol-verdict-bs1-realdata` (M-V-VERDICT)
2. `top10-2023-fy-vol-target-overlay-realdata` (M-SHARPE primary)
3. `sharpe-comparison-vol-target-bs1-realdata` (M-SHARPE comparison)

Kill-switch backtest scenario ships without an anchor in v0.1.0;
added in v0.1.1 if byte-deterministic.

### Joint advisory verdict (V × T) — recorded at M-FINAL

| V-verdict | T-classifier | Joint advisory verdict | Operator routing |
|-----------|--------------|------------------------|------------------|
| V5 | T-VOL-ALPHA-UNLOCKED | **ALPHA-UNLOCKED** | Ship; promote C2 + C5. |
| V5 | T-VOL-MARGINAL | **MARGINAL** | Spawn `v3-vol-target-tuning`. |
| V5 | T-VOL-NO-ALPHA | **NO-ALPHA** | Analyst spawn for C1 retirement; route budget to C2. |
| V1/V2/V3 | (any) | **MODEL-BROKEN** | Follow V-verdict's `follow_on`. |
| V4 | (any) | **DATA-PATHOLOGY** | Spawn `v3-data-vol-investigation`; foreclose on H1/H2 jointly. |

(Source: ADR-0038 § D1.c.)

### HANDOFF status

- **M-T1 closed 2026-05-22.** Architect lock complete.
- **HANDOFF → developer** for Wave A start. See
  [`tasks.md`](tasks.md) T-D-N1..T-D-N28 for the ordered T-D row
  breakdown with file:line + cargo + literal-output honest-tick
  contract.

## Changelog

- 2026-05-22 (analyst): authored v0.1.0 brief — R1-R12 / H1-H4 /
  K-vol-1..6 / Q1-Q6 / non-regression contract / acceptance per
  milestone / cost estimate per scope branch / out-of-scope guardrails.
  Predecessor evidence chain (v25a-patchtst-overlay v0.1.0 RETIRED
  2026-05-22) cited as motivating; survey C1 row carried forward
  with concrete R1-R12 / H1-H4 expansions. Operator's hybrid-sequence
  Q-PICK=C1+C2+C5 (this is C1; first to build) + Q-BUDGET ~6-8 weeks
  + Q-SEQ HYBRID + Q-PROCESS 3-parallel-analyst-passes reflected in
  Why + cost estimate. Trace row `REQ-V3-VOL-FORECASTER-001` opened
  `draft`; backlog Active entry added. HANDOFF → operator-decide
  (Q1-Q6) → architect for M-T1 / ADR-0038.
- 2026-05-22 (operator): autoapprove-all on Q1-Q6 + Q-anchors-sub
  + Q3-sub. Bundle locked: Q1=(b) Parkinson + Q2=(a) GARCH-only-MVP
  + Q3=(d) all-3-builders + Q4=(b) ADR-0038 NEW + Q5=(a)
  v3.0.0-volatility + Q6=(a) BS-1 train + BS-2 val + Q-anchors-sub=3
  + Q3-sub clamp [0.5,2.0] / target_vol=0.02 / kill-switch-mult=3.0.
  HANDOFF → architect for M-T1.
- 2026-05-22 (architect): M-T1 closed.
  [`decomp.md`](decomp.md) authored (T-AR-1..T-AR-10 resolved with
  file:line citations + Wave A-E ordered + rollback shape per wave
  + NO spike required).
  [`ADR-0038`](../architecture/adr/0038-vol-forecast-verdict-shape.md)
  authored (V-verdict V1-V5 + V_ALPHA priority tree; hand-rolled
  GARCH(1,1) MLE contract; JSON checkpoint schema; replay-cache
  namespace extension; strategy-side composition lock; anchor
  naming under v3.0.0-volatility N_new=3). PARALLEL to ADR-0033
  § D3, NOT extension (Q4=(b) operator default; retrospective
  lesson #2 honored). Frontmatter `status: proposed → in-progress`,
  `owner: architect → developer`. tasks.md T-AR-1..T-AR-10 ticked;
  T-D-N1..T-D-N28 Wave A-E rows queued. Trace row state flipped
  `proposed → in-progress`; ADR-0038 + decomp.md added to `arch`
  column. ADR-0038 registered in
  [`spec/architecture/adr/README.md`](../architecture/adr/README.md)
  registry table. Baseline anchor gate confirmed PASS pre-handoff:
  `ANCHORS PASS  (30 / 30)`. HANDOFF → developer for Wave A start.
