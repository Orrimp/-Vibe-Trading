---
slug: v25-tcn-horizon-bump-or-retire
status: shipped
owner: operator
updated: 2026-05-21
version: 0.1.0
predecessor: v25-tcn-threshold-tuning v0.1.0
parent: v25-tcn-overlay v2.5.0 (in-progress)
---

# v2.5 — TCN horizon-bump or retire (scope-decision-grade)

> **Scope-decision-grade analyst pass.** This brief does NOT lock a
> retrain or a retirement up front — it surfaces a multi-week,
> multi-route operator decision with cost / risk / reversibility
> framing per route, then routes to the operator-decide gate. The
> primary Q1 (scope a / b / c / d) has **NO safe analyst default**;
> the operator MUST answer Q1 before architect / developer spawns.
>
> Predecessor stack:
> [`v25-tcn-threshold-tuning v0.1.0`](../v25-tcn-threshold-tuning/feature.md)
> shipped 2026-05-21 with joint **T-MARGINAL + T-MARGINAL** verdict
> (BS-1 +0.018 / BS-2 +0.045 Sharpe-delta at τ=0.1/ε=0.001; both well
> below the +0.10 `T-ALPHA-UNLOCKED` threshold).
> [`v25-tcn-recalibrate v0.1.0`](../v25-tcn-recalibrate/feature.md)
> shipped 2026-05-21 eliminating the σ_train 608× / 580× inflation;
> gate-survival jumped 0% → 40-89% but the F-verdict stayed F4 under
> the immutable
> [ADR-0033 § D3](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d3-f-verdict-decision-algorithm)
> priority tree.
> [`v25-tcn-alpha-investigation v0.1.0`](../v25-tcn-alpha-investigation/feature.md)
> opened the F-verdict + 4-bucket failure-mode taxonomy.
> [`v25-tcn-overlay v2.5.0`](../v25-tcn-overlay/feature.md) is the
> in-progress parent feature; this feature is the operator-decided
> next-routing for it.

## Why

Three independent diagnostic ships have now told the same story:

1. **F4** — alpha-investigation: the v2.5 TCN at 1h horizon emits
   forecasts that are uncorrelated with realised next-bar returns on
   real Binance OHLCV.
2. **σ_train fixed, F4 stays** — recalibrate: the 608× / 580× σ_train
   inflation was eliminated metadata-only; gate-survival jumped 0% →
   40-89%, but the F-verdict legitimately stays F4 under the immutable
   ADR-0033 § D3 priority tree (`frac_inside_epsilon` 0.031 / 0.057 ≪
   0.5 F3 threshold).
3. **τ × ε tuning, marginal at best** — threshold-tuning: 90 backtests
   across the 9 × 5 (τ, ε) grid. Best cell (τ=0.1, ε=0.001) on BS-1
   gave +0.018 Sharpe-delta, on BS-2 +0.045 — both `T-MARGINAL`, both
   far below the +0.10 alpha-unlock threshold from
   [`v25-tcn-overlay`](../v25-tcn-overlay/feature.md) § success
   criterion. The +0.018 / +0.045 may be a noise-versus-noise signal —
   no robust statistical bound was computed in the sweep.

The remaining diagnostic hypothesis is the only one not yet
falsified: **the 1h forecast horizon is too noisy on hourly crypto
bars to give the TCN a learnable signal**. The v2.5 TCN architecture
(8 residual blocks × dilations {1..128} × 96 channels, RF=1021 bars =
~42 days) trains on next-1h log-return targets
([`crates/forecast/src/features.rs:627-628`](../../crates/forecast/src/features.rs)
— `target_logret = (close_t1 / close_t).ln() as f32`). Hourly crypto
log-return std is ~0.005-0.015 — well within the same order of
magnitude as round-trip transaction costs (~0.001-0.002 on liquid
USDT pairs) and 5-10× smaller than the Bitcoin daily log-return std
of ~0.02-0.04 across the 2023-2024 span. The signal-to-noise floor on
1h targets is genuinely poor.

The operator-decided routing (c) from the
[recalibrate presenter deck](../v25-tcn-recalibrate/presentations/v25-tcn-recalibrate-2026-05-21.md)
designated this feature as the multi-week fallback after the cheap
threshold-tuning sweep failed to unlock alpha. We are now at that
fallback. **But there are four mutually-exclusive paths forward**,
each with different cost / risk / reversibility, and the operator
must pick before architect can lock a design.

### Quantitative-finance context

The horizon-bump hypothesis rests on three observations:

1. **Hourly log-return std vs target horizon.** Crypto OHLCV at
   hourly cadence has return std ~0.005-0.015 over the BS-1 (2023 FY,
   ~8,760 bars) and BS-2 (2023 + Q1 2024, ~10,944 bars) spans. At 24h
   horizon, the target return std is ~0.025-0.040 (roughly √24 ≈ 4.9×
   the 1h std under i.i.d. assumption; empirically more like 3-4× due
   to negative serial autocorrelation in crypto returns). The
   signal-to-noise ratio for a directional prediction improves with
   horizon **iff** the model can learn a slow-moving trend the 1h
   target cannot resolve.
2. **Receptive field is already huge.** The current TCN RF = 1021
   bars ≈ 42 days. At hourly bars, the model sees 42 days of context
   when predicting the next 1 hour — a 1000:1 lookback ratio. At 24h
   horizon, the lookback ratio drops to ~42:1 — still well within
   normal time-series practice. RF is NOT the binding constraint.
3. **Effective sample count drops with horizon.** Training on 24h
   non-overlapping targets reduces the effective sample size from
   ~8,760 (1h overlapping) to ~365 (24h non-overlapping) per symbol
   per year. With 10 symbols, 24h non-overlap gives ~3,650 effective
   training samples — well under the typical 50,000 sample-count
   threshold for DL on time-series. **Overlapping 24h targets**
   (still emit a target every 1h, but the target is `(close_{t+24} /
   close_t).ln()`) keep ~87,000 effective samples per checkpoint
   span; the 24h target windows overlap by 23/24, which is acceptable
   but creates autocorrelated targets that need careful loss
   weighting. See H2 / Q2 for the design choice.

The retire-promote alternative rests on different observations:

1. **PatchTST / iTransformer is a different paradigm.** TCN exploits
   local-to-distant via dilated causal convolutions (the v2.5
   inductive bias). PatchTST chunks contiguous bars into "patches"
   and runs full self-attention across patches (the v2.5a inductive
   bias). On crypto OHLCV with intraday structure (Asia / Europe /
   US trading sessions, weekend dampening), PatchTST's patch-attention
   may capture session-level structure that TCN's smooth dilated
   convolution misses. **Empirical bake-off > literature claim** per
   [`v25-dl-forecast-overlay`](../v25-dl-forecast-overlay/feature.md).
2. **Shared infrastructure compounds.** Per the umbrella roadmap, the
   `ForecastProvider` trait, training loop, audit emission, and
   replay-cache wiring are model-agnostic. Phase 2 (PatchTST)
   marginal cost is ~30-50% of phase 1 (TCN) because the scaffold is
   built. Per the
   [`v25a-patchtst-overlay`](../v25a-patchtst-overlay/feature.md)
   stub, this is queued for activation after phase 1 ships — which it
   has, in the "F4 verdict but useful infrastructure" sense.
3. **v2.6 bake-off picks the canonical.** Per the
   [`v26-forecast-bakeoff`](../v26-forecast-bakeoff/feature.md) stub,
   after PatchTST + Transformer ship, all three model families face
   identical evaluation on BS-1 / BS-2 and the operator picks the
   canonical v2.5 forecaster. **Retiring v2.5 TCN now removes one
   contender from the bake-off**, which is also information-bearing:
   the bake-off result is whatever PatchTST + Transformer say,
   without TCN noise.

The cheapest decision-theoretic path is **scope (d) defer-on-live**:
ship the T-MARGINAL tuned cell from the threshold-tuning feature as
an advisory default, gather live-trade Sharpe data for N months, and
decide retrain / retire based on that signal — but it's
latency-bound on real-money flow and only viable if the operator has
the capital deployment to make it meaningful. Scope (d) is the
"don't spend compute until we know it's worth spending" path.

## Requirements (R1-R8 — scope-dependent)

> **All requirements below are scope-dependent.** Until Q1 resolves,
> we do not know which subset of R1-R8 ships. Each R is annotated with
> the scope it activates under.

### R1 — Horizon-bump retrain scaffold (scope (a) or (c) only)

If Q1 = (a) or (c), the developer extends the existing TCN training
scaffold to support a multi-horizon target:

- New CLI flag on `crates/forecast/src/bin/train_tcn.rs`:
  `--target-horizon-bars <N>` (default unchanged = 1; new value per
  Q2). Architect-decide at M-T1 whether to add a `[targets]` section
  to the TOML config or keep it CLI-only.
- Extend
  [`crates/forecast/src/features.rs:627-628`](../../crates/forecast/src/features.rs)
  to compute `target_logret = (close_{t+N} / close_t).ln()` for the
  configured N. The default N=1 stays byte-identical
  (`(close_t1 / close_t).ln()`).
- Bound check: `t+N` must not overflow `self.bars.len()`. The
  iterator's `max_cursor` must be reduced by `N-1` to prevent
  out-of-bounds.
- The receptive field RF=1021 bars is unchanged; the model topology
  is unchanged (8 residual blocks × dilation {1..128} × 96 channels);
  the loss is unchanged (Huber δ=0.001); the OneCycle AdamW schedule
  is unchanged. Only the target derivation changes.
- The σ_train computation at
  [`train_tcn.rs:733-741`](../../crates/forecast/src/bin/train_tcn.rs)
  stays bugged-as-shipped (per-batch accumulation across all epochs).
  At ship time we honour the
  [ADR-0035](../architecture/adr/0035-tcn-sigma-train-recalibration.md)
  contract: σ_train is metadata-only, post-training-frozen-forward-
  pass; a separate `recalibrate_sigma_train` invocation against the
  new checkpoint emits the corrected scalar. Reuses the existing
  recalibrate bin from
  [`crates/forecast/src/bin/recalibrate_sigma_train.rs`](../../crates/forecast/src/bin/recalibrate_sigma_train.rs)
  with no code change (the bin is horizon-agnostic — it computes std
  over the converged-model forward pass on the training span).
- **Operator decision needed?** Q2 — horizon target (24h / 48h /
  168h / daily-bars), Q3 — single or multiple checkpoints, Q4 — TCN
  topology change yes/no.

### R2 — Retrain + new checkpoint files (scope (a) or (c) only)

Run `train_tcn` with `--target-horizon-bars <Q2-value>` against:

- Q5 = (a): `2023-01-01..2023-12-31` (mirror BS-1 span) — emits
  `tcn-bs1-h<Q2>-<sha>.safetensors` + `.metadata.json`.
- Q5 = (b) (optional): `2023-01-01..2024-03-31` (mirror BS-2 span) —
  emits `tcn-bs2-h<Q2>-<sha>.safetensors` + `.metadata.json`.

Filename convention: `tcn-bs{1,2}-h{Q2}-<sha>.{safetensors,metadata.json}`
under `crates/forecast/checkpoints/anchors/`. The `<sha>` is the
`model_revision` SHA per
[ADR-0029](../architecture/adr/0029-tcn-checkpoint-provenance.md) —
deterministic over (weights, canonical-architecture-descriptor,
training-data-revision-SHA, seed). The new horizon parameter enters
the canonical-architecture-descriptor so it contributes to the SHA;
two checkpoints with different horizons have different SHAs by
construction (K2 invariant).

Wall-clock estimate per checkpoint (analyst-honest, based on the
predecessor `v25-tcn-overlay` reporting ~4-5 days per 1h checkpoint
on Apple Silicon Metal):

- **24h horizon, overlapping targets** (~87k samples): ~5-7 days per
  checkpoint. Roughly the same training duration as 1h because the
  sample count is similar; the loss surface may converge faster (24h
  targets are higher-variance but more autocorrelated) or slower
  (more epochs to drive Huber loss down) — empirical until
  developer runs it.
- **24h horizon, non-overlapping targets** (~3,650 samples): ~1-2
  days per checkpoint, but at risk of underfit on the small sample
  count.
- **48h or 168h horizons**: comparable wall-clock to 24h since the
  bottleneck is GPU step count (epochs × batches), not the target
  derivation.

Reproducibility budget: ~5-7 days × N checkpoints (Q3 = (a): 1; (b):
2; (c): 1 followed by 1 if first succeeds) = **5-21 days wall-clock
sequentially**. Apple Silicon Metal cannot meaningfully parallelise
two training runs on one machine without trashing both — they run
sequentially. **This is the load-bearing cost driver for scope (a)**.

- **Operator decision needed?** Q2 (horizon), Q3 (checkpoint count),
  Q5 (data span).

### R3 — Retraining-cost honesty (scope (a) or (c) only)

The developer MUST emit a `watch -n 60 '<probe>'` block at training
start time per the operator's `MEMORY.md` directive ("watch recipe
for long-running tasks"). Suggested probe:

```bash
# Train run progress (replace <PID> with the actual cargo process)
watch -n 60 'tail -20 /tmp/train_tcn-h24-bs1.log && \
             echo "---" && \
             ps -p <PID> -o pcpu,pmem,etime,command | tail -2'
```

The developer MUST emit `train_events` rows per
[`spec/architecture/adr/0034-cockpit-training-control.md`](../architecture/adr/0034-cockpit-training-control.md)
so the cockpit's training-control panel surfaces per-epoch progress.
The cockpit is the canonical live-progress UI per the
[`cockpit-training-control`](../cockpit-training-control/feature.md)
ship 2026-05-21.

**Cost-blow-up tripwire**: if a single epoch exceeds 24 wall-clock
hours on Apple Silicon Metal, the developer MUST escalate to the
operator before continuing. This is a hard tripwire on K1 (compute
cost over-runs the +0.10 alpha threshold). Architect formalises the
tripwire at M-T1.

- **Operator decision needed?** No — analyst-locks the cost
  tripwire.

### R4 — PatchTST analyst-pass (scope (b) or (c) only)

If Q1 = (b) or (c), this feature spawns a follow-on analyst pass for
[`v25a-patchtst-overlay`](../v25a-patchtst-overlay/feature.md) (the
phase-2 stub of the
[4-phase DL roadmap](../v25-dl-forecast-overlay/feature.md)).
The PatchTST stub today is `status: roadmap, owner: pending-analyst`
with only a Why + carry-forward invariants block. The analyst pass
would:

- Read PatchTST paper (Nie et al 2023) + iTransformer (Liu et al
  2024) + survey the candle / candle-nn Transformer primitives.
- Author R1-R<n> requirements + H1-H<n> hypotheses + K1-K<n> risks
  for the PatchTST model topology (patch size, embedding dim, head
  count, depth, dropout) + training scaffold (loss, optimiser,
  schedule, batch size, epoch count) + overlay composition (same
  signal-level pattern as TCN per
  [architecture/12-forecast-overlay.md](../architecture/12-forecast-overlay.md)).
- Author Q1-Q<n> operator-decide questions with analyst defaults.
- Promote the PatchTST stub Queue → Active.

Wall-clock estimate per the v25a-patchtst-overlay stub: **~4-6
weeks** from analyst-pass through ship (architect + developer +
tester + presenter; phase-2 model training is ~7-10 days per
checkpoint).

- **Operator decision needed?** No — analyst-locks the scope (b)
  trigger. The follow-on PatchTST feature is itself
  operator-decided downstream.

### R5 — Retire v2.5 TCN decision-record (scope (b) only, optional under (c))

If Q1 = (b) or (c)-deferred, this feature emits a single decision
record that:

- Marks `v25-tcn-overlay` as `status: retired-research-mode-only`
  in its frontmatter. The shipped BS-1 / BS-2 checkpoints + 26
  anchors stay byte-identical. The `with_tcn_bs{1,2}_ledger` strategy
  builders stay shipped (existing callers continue to work; the
  default `dec!(0.6)` confidence threshold + ε=0.0005 deadband stay).
- Authors **ADR-0036** at
  `spec/architecture/adr/0036-v25-tcn-retire-decision.md`. ADR
  carries: rationale (F4-verdict + σ_train recalibration +
  T-MARGINAL sweep all in evidence), retirement scope (v25-tcn-overlay
  research-mode; v25a-patchtst + v25b-transformer + v26-bakeoff
  proceed), reversibility (the v25-tcn-overlay stack is preserved
  on disk and re-activatable by re-promoting it Active in the
  backlog — no code is deleted).
- Does NOT delete any code. The retire is a documentation /
  routing-status change, not a deletion.
- Adds a backlog § Recent entry recording the retirement.

- **Operator decision needed?** Q6 — retire scope (research-mode vs
  delete-all-code).

### R6 — Defer-on-live decision-record (scope (d) only)

If Q1 = (d), this feature emits a decision record only:

- Authors **ADR-0036-alt** (alternative ADR-0036 body for scope (d))
  at `spec/architecture/adr/0036-v25-tcn-defer-on-live.md`. ADR
  carries: rationale (T-MARGINAL is small but positive; live-trading
  data is the cheaper next signal than a multi-week retrain),
  defer-duration (operator-decide, suggested 30-90 days of live
  trading on the threshold-tuned cell), decision criteria
  (live-trade Sharpe over the deferral window: >= +0.10 → promote
  tuned default; < 0 → retire; in between → re-evaluate).
- Adds a follow-on Queue entry `v25-tcn-live-trade-eval` with the
  defer-duration + decision criteria.
- Does NOT spawn any code change. The
  `with_tcn_bs{1,2}_ledger_tuned(τ, ε)` builders from the
  threshold-tuning ship are already available for live deployment.
- **Constraint**: scope (d) requires the operator to have actual
  capital deployed against the tuned cell. If no capital is deployed,
  scope (d) is degenerate — there's no live data to evaluate; the
  decision indefinitely defers.

- **Operator decision needed?** Q7 — defer-duration + decision
  criteria. Q1 = (d) requires operator-attest that capital will be
  deployed.

### R7 — Anchor strategy (anchor-additive only across all scopes)

This feature is **anchor-additive** regardless of which scope ships.
All 28 existing anchors stay byte-identical:

- 22 pre-recalibrate anchors (19 pre-investigation + 3
  alpha-investigation).
- 4 v2.6.1-alpha-investigation-recalibrated anchors (recalibrate
  ship).
- 2 v2.6.2-threshold-tuning anchors (threshold-tuning ship).

New anchors under a new version string **`v2.7.0-horizon-bump`** (if
scope a / c) or **no new anchors** (if scope b / d) — depending on
which deliverables fire:

**Scope (a) — horizon-bump retrain only**:
- `forecast-distribution-bs1-h<Q2>-realdata` (Q3 = (a)) — body-SHA of
  the new horizon-bumped BS-1 forecast-distribution report.
- `forecast-distribution-bs2-h<Q2>-realdata` (Q3 = (b) only) — same
  for BS-2.
- `top10-2023-fy-tcn-overlay-h<Q2>-realdata` (Q3 = (a) + backtest) —
  body-SHA of the new horizon-bumped BS-1 backtest report.
- `top10-2024-fy-tcn-overlay-h<Q2>-realdata` (Q3 = (b) only) — same
  for BS-2.

**Scope (b) — retire**: 0 new anchors in THIS feature; the PatchTST
follow-on ships its own anchors under `v2.7.0-patchtst` per the v25a
stub.

**Scope (c) — both**: union of (a) + (b) anchors.

**Scope (d) — defer**: 0 new anchors.

Anchor count progression:
- Pre-feature: 28.
- Post-feature (scope a, Q3=(a)): 30 (1 forecast-dist + 1 backtest).
- Post-feature (scope a, Q3=(b)): 32 (2 forecast-dist + 2 backtest).
- Post-feature (scope a, Q3=(c)): 30 then potentially 32 if extension fires.
- Post-feature (scope b): 28 (no new anchors).
- Post-feature (scope c): same as (a) + 0 for the (b) routing.
- Post-feature (scope d): 28 (no new anchors).

`bash scripts/verify_anchors.sh` reports `ANCHORS PASS (28/28)` PRE
and `<NEW>/<NEW>` POST, with all 28 originals byte-identical.

- **Operator decision needed?** Q7 — anchor naming + version pin.
  Analyst default = `v2.7.0-horizon-bump`.

### R8 — Non-regression contract (load-bearing across all scopes)

**Critical, load-bearing**: this feature MUST NOT touch the existing
28 anchors. Specifically:

- The existing `tcn-bs{1,2}-<sha>.safetensors` files are NOT modified.
- The existing `tcn-bs{1,2}-<sha>.metadata.json` files are NOT modified.
- The existing `tcn-bs{1,2}-<sha>.metadata.recalibrated.json` overlay
  files are NOT modified.
- The 22 pre-recalibrate anchors stay byte-identical.
- The 4 v2.6.1-alpha-investigation-recalibrated anchors stay
  byte-identical.
- The 2 v2.6.2-threshold-tuning anchors stay byte-identical.
- The existing `with_tcn_bs{1,2}` + `with_tcn_bs{1,2}_ledger` +
  `with_tcn_bs{1,2}_tuned` + `with_tcn_bs{1,2}_ledger_tuned` strategy
  builders stay byte-identical. New horizon-bump builders (e.g.
  `with_tcn_bs1_h24_ledger`) are ADDITIVE under scope (a) / (c).
- For scope (a) / (c): the new `--target-horizon-bars` CLI flag on
  `train_tcn` defaults to `1` (unchanged behaviour); the feature path
  is opt-in. A unit test asserts default-invocation produces a
  checkpoint with `target_horizon_bars=1` and matches the existing
  `tcn-bs{1,2}` `model_revision` SHA byte-for-byte under
  ADR-0029-canonical-arch-descriptor (modulo any inevitable RNG
  determinism quirks; architect formalises at M-T1).

`bash scripts/verify_anchors.sh` PRE-lock reports `26 PASS` plus
the 2 pre-existing glob-collision FAILs
(`forecast-distribution-bs{1,2}-realdata`) inherited from the
v25-tcn-recalibrate ship (documented at `spec/v25-tcn-threshold-tuning/feature.md
§ Anchor progression`); POST-lock the count grows by the
scope-(a)/(c) additive anchors. The 28 originals stay byte-identical
(26 PASS + 2 known-FAIL).

- **Operator decision needed?** No — analyst-locks. Tester verifies
  at M-FINAL.

## Hypothesis register (H1-H3)

> Each hypothesis is testable; the tester gate is what closes /
> falsifies it. Listed in dependency order. Only relevant under the
> scope that exercises the hypothesis (annotated).

### H1 — 24h horizon unlocks signal on hourly bars (scope a / c)

**Statement.** Retraining the v2.5 TCN topology with `target_logret
= (close_{t+24} / close_t).ln()` (24-hour overlapping or
non-overlapping target — Q2 sub-decision) on the same BS-1 training
span produces a forecaster whose F-verdict at M-FORECAST-DIST is
**F1** or **F3** (not F4). Equivalently: post-recalibration of
σ_train, gate-survival is non-trivial AND `frac_inside_epsilon` ≤
0.5 (the F3 trigger). Equivalently: the joint T-verdict on a
follow-on threshold-tuning sweep of the new checkpoint is
`T-ALPHA-UNLOCKED` (Sharpe-delta ≥ +0.10 vs v1 momentum baseline).

**Test.** R2 trains the new checkpoint; R7 emits the
forecast-distribution report under the recalibrated σ_train
overlay; F-verdict is read from the body. A follow-on
threshold-tuning sweep (separate feature, queued post-M-FORECAST-
DIST) tests the Sharpe-delta. H1 is **confirmed** iff F-verdict
is not F4 AND a τ × ε cell unlocks ≥ +0.10 Sharpe-delta.

**Confidence at brief time**: MEDIUM-LOW. The arithmetic argument
for horizon improvement is sound (24h log-return std is 3-4× the
1h std, so SNR vs transaction-cost noise improves), but the
empirical evidence on whether crypto OHLCV has a 24h-horizon
learnable trend is mixed. Daily-bar momentum strategies on crypto
(e.g. the v1 momentum baseline operating on 20-bar lookback)
extract small alpha on the same universe — suggesting *some*
daily-cadence signal exists. Whether the TCN can extract it at a
24h forward-prediction (rather than a 20-bar backward-lookback) is
the open question. **The threshold-tuning T-MARGINAL +0.018 /
+0.045 result is weak prior evidence that the model has very
little signal at 1h; it does not bound the 24h-horizon prior.**

### H2 — Longer horizons don't overfit to fewer effective samples (scope a / c)

**Statement.** Training with overlapping 24h targets (one target
emitted every 1h, target = `(close_{t+24} / close_t).ln()`) keeps
~87k effective samples per BS-1 span — comparable to the 1h
shipping checkpoint. The model does NOT underfit (Huber loss
converges to a non-degenerate value) AND does NOT overfit
(train/val gap stays bounded; val-Huber doesn't diverge after a
few epochs).

**Test.** R2's training run emits `train_events` rows per epoch
with `train_loss` + `val_loss`. The tester gate at M-T1 (architect
formalises) asserts (a) train_loss strictly decreasing across
epochs, (b) val_loss within 2× train_loss for the final 5 epochs,
(c) val_loss not Inf / NaN, (d) σ_train post-recalibration in the
range [0.020, 0.060] (3-5× the 1h σ_train of 0.018 / 0.012 —
consistent with √24-scaling adjusted for autocorrelation in
overlapping targets).

**Confidence at brief time**: MEDIUM-HIGH. Overlapping targets are
the standard practice in DL time-series forecasting; the
autocorrelation in adjacent targets is well-known (~0.95 between
target_t and target_{t+1}) but doesn't cause catastrophic overfit
at the v2.5 TCN's parameter count (~4.4M). The risk is val-loss
divergence after ~10 epochs as the model memorises the
autocorrelated targets; the OneCycle AdamW schedule's cosine
decay typically prevents this. Architect-confirm at M-T1.

### H3 — PatchTST or Transformer dominates TCN regardless of horizon (scope b / c)

**Statement.** When the v25a-patchtst-overlay phase 2 ships (per the
4-phase DL roadmap), its forecast-distribution + Sharpe-delta on
BS-1 / BS-2 exceeds the v2.5 TCN's by a margin large enough to
justify retiring TCN. Equivalently: PatchTST's joint T-verdict on
the same (τ, ε) sweep is `T-ALPHA-UNLOCKED` while v2.5 TCN's stayed
`T-MARGINAL`.

**Test.** The v2.6 bake-off feature (queued at
[`v26-forecast-bakeoff`](../v26-forecast-bakeoff/feature.md)) is
the canonical test surface; H3 is implicit in the bake-off design.
This feature does NOT directly test H3; it surfaces it as a
hypothesis to motivate scope (b) over scope (a).

**Confidence at brief time**: LOW-MEDIUM. PatchTST's published
benchmarks are on M4 / ETT / electricity — none of which are
crypto OHLCV. The TCN's F4-verdict on 1h horizon is partial evidence
that the data distribution is hard for any forecaster, not just for
TCN-shape models. **The conservative read is: PatchTST may also
land F4 at 1h horizon**, and the bake-off result may be "all three
families are T-MARGINAL at best." Scope (b) is informational
regardless of whether H3 confirms — the bake-off result is
load-bearing either way.

## Risk register (K1-K7)

| Risk | Mitigation |
|------|------------|
| **K1 — Scope (a) retrain blows the compute budget** (e.g. >7 days per checkpoint on Apple Silicon Metal; total >21 days for the 2-checkpoint case). | R3 cost tripwire (single epoch > 24h wall-clock = escalate). Q3 single-checkpoint default keeps the bounded case to ~5-7 days. Architect formalises a developer-decide gate at M-T1: if epoch N takes > 3× the median of epochs 1..N-1, the developer pauses + emits a diagnostic dump + escalates. |
| **K2 — Scope (a) horizon-bump retrain comes back F4 again** (the 24h-horizon doesn't help; the model has no learnable signal at any horizon on this data). | This is an experimental outcome, not a feature failure. Per Q6, the F4-on-24h verdict routes immediately to scope (b) retire — the operator has now ruled out 1h AND 24h horizons. The follow-on `v26-forecast-bakeoff` carries the canonical retirement gate. |
| **K3 — Scope (b) retire is premature** (PatchTST also F4s; we needed the v2.5 TCN as a bake-off baseline). | R5's retire decision-record keeps the v25-tcn-overlay stack on disk + re-activatable. The retirement is a documentation status flip, NOT a code deletion. Reversible by re-promoting Queue → Active in the backlog. |
| **K4 — Scope (d) defer-on-live degenerates** (no capital deployed; no live signal accumulates). | Q1 = (d) requires operator-attest that capital is deployed. If no capital, scope (d) is a no-op and the operator should pick (a) / (b) / (c). |
| **K5 — Scope creep into v2.5 TCN topology changes** (developer tempted to "while we're retraining, let's also bump channel count or add an attention head"). | Hard analyst boundary: scope (a) changes ONLY the target derivation (`target_logret` formula). The 8 residual blocks × dilation {1..128} × 96 channels topology stays byte-identical (`crates/forecast/src/tcn.rs:302-311`). Architect codifies as a unit test at M-T1 (`tcn::CONTEXT_LEN == 256`, `tcn::CHANNELS == 96`, `tcn::N_BLOCKS == 8`, `tcn::DILATIONS == [1,2,4,8,16,32,64,128]`). |
| **K6 — Existing 28 anchors flip on the new horizon-bump build** (some additive code path leaks into a default training run; the v2.6.0-realdata or v2.6.2-threshold-tuning anchors land different SHAs after the feature merges). | R8 non-regression contract: a CI gate at M-FINAL runs `verify_anchors.sh` PRE and POST and asserts the 28 originals are byte-identical. The new `--target-horizon-bars` CLI defaults to 1 → existing-behavior byte-identical (architect formalises at M-T1; developer ships a unit test). |
| **K7 — Operator over-indexes on (a) and burns 21 days that could have gone to (b)**. | Q1 surfaces this as the load-bearing operator-decide. Analyst's analyst-honest framing: scope (a) is a high-information / high-cost experiment; scope (b) is a lower-information / medium-cost commitment; scope (c) is the highest-information / highest-cost path; scope (d) is the cheapest but requires live capital. The operator picks based on their risk tolerance + capital deployment plans. |

## Non-regression contract

This section consolidates the load-bearing invariants that the tester
gate confirms at M-FINAL (regardless of scope):

1. **28 anchored body-SHAs byte-identical.** `bash scripts/verify_anchors.sh`
   reports `26 PASS + 2 pre-existing glob-collision FAILs` PRE-lock
   (inherited from v25-tcn-recalibrate; documented at
   `spec/v25-tcn-threshold-tuning/feature.md § Anchor progression`)
   and `<NEW>/<NEW>` POST-lock with the same 2 known-FAIL baseline.
   All 28 originals (26 PASS + 2 known-FAIL) stay byte-identical.
2. **Original `.safetensors` files byte-identical.** `git diff HEAD --
   crates/forecast/checkpoints/anchors/*.safetensors` is empty.
3. **Original `.metadata.json` files byte-identical.** Same diff.
4. **Original `.metadata.recalibrated.json` overlay files byte-identical.**
   Same diff.
5. **Existing strategy builders byte-identical.**
   `with_tcn_bs{1,2}` + `with_tcn_bs{1,2}_ledger` +
   `with_tcn_bs{1,2}_tuned` + `with_tcn_bs{1,2}_ledger_tuned` stay
   byte-identical (any new horizon-bump builders are additive).
6. **Default `train_tcn` invocation byte-identical** (scope a / c
   only). A `cargo run -p forecast --bin train_tcn -- <existing args>`
   invocation without `--target-horizon-bars` produces a checkpoint
   whose `target_horizon_bars=1` metadata field matches the existing
   `tcn-bs{1,2}` metadata. Architect formalises at M-T1.
7. **No new external crate dependencies.** Workspace `Cargo.toml`
   diff is limited to existing crates (per CLAUDE.md "no new external
   crate deps" constraint).
8. **No iced bump.** Operator-locked per CLAUDE.md.
9. **F-verdict algorithm immutable.** ADR-0033 § D3 stays unchanged
   across this feature. New checkpoints' forecast-distribution reports
   follow the same `## Verdict` algorithm.
10. **ADR-0029 metadata canonicaliser unchanged.** New `_h24` checkpoint
    metadata uses the same JSON-canonical bytes as existing metadata.

## Acceptance per milestone (scope-dependent)

The feature is **done** when all milestones land their gates **for
the chosen scope**.

### M-OD — Operator-decide (Q1-Q7 resolved)

> **Hard blocker before any architect / developer work.** Q1 has NO
> safe analyst default — operator MUST answer Q1 before the feature
> can proceed past M-OD.

1. Q1 (primary scope) answered by operator.
2. Q2-Q7 answered by operator (analyst defaults applied if operator
   says "autoapprove" on the non-Q1 questions).
3. Frontmatter flips `status: draft → proposed`, `owner: analyst →
   architect`.

### M-T1 — Architect lock (scope-dependent)

For scope (a) / (c):
1. § Design block appended to `feature.md` (between § Out of scope
   and § Changelog).
2. `spec/v25-tcn-horizon-bump-or-retire/decomp.md` complete with T-D
   / T-T row decomposition.
3. ADR-0037 (if scope-(a)/(c) requires a new ADR for the horizon-bump
   training contract) written OR explicitly skipped.
4. Cost tripwire (R3) formalised with a developer-callable
   `assert_epoch_budget` invariant.
5. K5 topology-immutability unit test designed.
6. Frontmatter flips `status: proposed → in-progress`, `owner:
   architect → developer`.

For scope (b):
1. ADR-0036 (retire decision-record) drafted.
2. v25-tcn-overlay frontmatter prepared for `retired-research-mode-only`
   flip.
3. PatchTST follow-on analyst spawn queued.

For scope (d):
1. ADR-0036-alt (defer-on-live) drafted.
2. v25-tcn-live-trade-eval follow-on Queue entry drafted with
   defer-duration + decision criteria.

### M-RETRAIN — Multi-day retrain run (scope (a) / (c) only)

1. `train_tcn --target-horizon-bars <Q2>` completes for each Q3
   checkpoint without exceeding the R3 cost tripwire.
2. New `tcn-bs{1,2}-h<Q2>-<sha>.safetensors` + `.metadata.json` files
   on disk under `crates/forecast/checkpoints/anchors/`.
3. `train_events` rows emitted per epoch with deterministic
   `model_revision` SHA at training-complete.
4. σ_train recalibration follow-on invocation emits
   `.metadata.recalibrated.json` overlay for each new checkpoint.

### M-FORECAST-DIST — Re-run forecast distribution (scope (a) / (c) only)

1. `forecast_distribution --metadata-path <recalibrated-overlay>`
   runs on each new checkpoint.
2. New `forecast-distribution-bs{1,2}-h<Q2>-realdata-recalibrated-<date>.md`
   reports emitted under `spec/v25-tcn-horizon-bump-or-retire/reports/`.
3. F-verdict per the immutable ADR-0033 § D3 algorithm recorded in
   each report body.

### M-SHARPE — Real-Binance backtest (scope (a) / (c) only)

1. `backtest --scenario top10-2023-fy-tcn-overlay-h<Q2>-realdata`
   runs on each new checkpoint.
2. New backtest report on disk under `spec/v25-tcn-horizon-bump-or-retire/reports/`.
3. Sharpe-delta vs v1 momentum baseline computed; T-classifier
   advisory verdict (`T-ALPHA-UNLOCKED` / `T-MARGINAL` / `T-NO-ALPHA`)
   recorded.

### M-FINAL — Tester gate (all scopes)

1. `cargo fmt --check` + `cargo clippy --workspace -- -D warnings` PASS.
2. `cargo clippy -p forecast --features candle -- -D warnings` PASS.
3. `cargo test --workspace --lib` PASS, 0 failures.
4. New integration tests PASS (architect decomposes at M-T1).
5. `bash scripts/verify_anchors.sh` reports `26 PASS + 2 known-FAIL`
   PRE + same baseline plus scope-(a)/(c) additive anchors POST.
6. 2-run byte-identity determinism gate on any new reports (scope a / c).
7. `uv run scripts/spec_lint.py` matches the baseline (0 new
   categories).
8. Joint verdict recorded in `feature.md § Verification`.

### M-PRESENTER — Operator approval

1. Presenter deck under
   `spec/v25-tcn-horizon-bump-or-retire/presentations/v25-tcn-horizon-bump-or-retire-<YYYY-MM-DD>.md`
   carrying the joint verdict + the recommended next routing.
2. Operator ticks approval. Frontmatter flips `status: in-progress →
   shipped`.
3. Trace row `REQ-V25-TCN-HORIZON-BUMP-OR-RETIRE-001` flips state.
4. Backlog entry moved Active → Recent.

## Open questions (Q1-Q7 — operator-decide)

> **Q1 is HARD BLOCKER — no safe analyst default.** The operator
> MUST answer Q1 before architect / developer work can spawn. Q2-Q7
> carry analyst-recommended defaults; "autoapprove" applies to Q2-Q7
> only.

### Q1 — Primary scope (HARD BLOCKER — no safe default)

**The load-bearing operator-decide for this entire feature.** The
analyst surfaces four mutually-exclusive paths with cost / risk /
reversibility per path. There is NO safe analyst default; the
operator's risk tolerance + capital-deployment plans dominate.

- **(a)** Horizon-bump retrain only. Retrain v2.5 TCN at a longer
  horizon (Q2 = 24h / 48h / 168h). Single-deliverable scope: new
  checkpoint(s) + forecast-distribution report + Sharpe-delta
  backtest. Cost: **5-21 days wall-clock** depending on Q3
  (1 or 2 checkpoints). Risk: K1 (compute over-runs) + K2 (F4 stays
  even at 24h). Reversibility: HIGH — existing 28 anchors stay
  byte-identical; new anchors are additive.

  **When (a) wins**: operator wants a high-information experiment
  (horizon hypothesis is genuinely untested); operator has the
  compute time + tolerance for a 1-3 week wait; operator wants to
  keep the v2.6 bake-off honest (TCN as a baseline against
  PatchTST + Transformer).

- **(b)** Retire v2.5 TCN; promote v2.5a PatchTST. Skip the retrain;
  the threshold-tuning T-MARGINAL result + the σ_train recalibration
  story together are sufficient evidence that 1h-horizon TCN can't
  extract alpha. Promote
  [`v25a-patchtst-overlay`](../v25a-patchtst-overlay/feature.md)
  Queue → Active and spawn its analyst pass. Cost: **~4-6 weeks
  wall-clock** for the PatchTST ship (per v25a stub). Risk: K3
  (premature retire). Reversibility: HIGH — retire is a status
  flip, not code deletion; v25-tcn-overlay re-activatable.

  **When (b) wins**: operator's read is "1h horizon already proves
  there's no signal at TCN-shape inductive bias; the right next
  experiment is a different paradigm (PatchTST), not a longer
  TCN"; operator wants v2.6 bake-off to start sooner; operator
  doesn't have 1-3 weeks of compute time to spare.

- **(c)** Both in parallel. Run scope (a) as a cheap diagnostic
  (does TCN have ANY horizon-sensitive signal? at any of
  {24h, 48h, 168h}?) AND simultaneously promote scope (b)
  PatchTST analyst-pass. The two run independently — scope (a)
  on Apple Silicon Metal compute, scope (b) on analyst-write +
  architect-design human bandwidth. Cost: **~6-9 weeks wall-clock**
  (overlap minimises but doesn't eliminate the cost). Risk: K7
  (over-spending on both bets). Reversibility: HIGH.

  **When (c) wins**: operator wants to dominate the v2.6 bake-off
  with all-evidence (TCN-at-1h vs TCN-at-24h vs PatchTST vs
  Transformer); operator has full bandwidth (compute + human) for
  the longer schedule; operator's risk tolerance is "spend now to
  decide later with more data."

- **(d)** Defer entire decision. Ship the threshold-tuning ship's
  T-MARGINAL cell (τ=0.1, ε=0.001) as an advisory default flip via
  the `_tuned` builders that already exist. Gather live-trade
  Sharpe data for N months (Q7 = 30 / 60 / 90 days). Decide
  retrain / retire based on the live-trade signal. Cost: **zero
  compute, zero developer-day** — latency-bound on real-money
  flow. Risk: K4 (degenerate if no capital deployed). Reversibility:
  HIGH.

  **When (d) wins**: operator has actual capital deployed to the
  tuned cell; operator's read is "+0.018 / +0.045 Sharpe-delta is
  small but worth real-money testing"; operator wants to defer the
  expensive bets until live data is in hand.

**Analyst recommendation: NONE. Operator-decide load-bearing.**

If forced to pick a default (which the analyst declines), the
analyst would weight (b) over (a) over (c) over (d) — but ONLY
because the analyst's prior is that v2.5 TCN at 1h has had ample
diagnostic work and the remaining marginal information from a 24h
retrain is unlikely to flip the verdict to T-ALPHA-UNLOCKED. The
operator's prior may differ; the decision-relevant context is the
operator's, not the analyst's.

### Q2 — Horizon target (scope a / c only)

If Q1 = (a) or (c), which horizon to retrain at?

- **(a)** 24h target on hourly bars (`target_logret =
  (close_{t+24} / close_t).ln()`). Minimal scaffold change (only
  the target derivation flips). Theoretically motivated by 24h log-
  return std being 3-4× the 1h std (better SNR vs transaction-cost
  noise). **Analyst-recommended default.**
- **(b)** 48h target on hourly bars. More aggressive horizon
  expansion; SNR vs transaction-cost noise improves further but
  the model's effective sample count drops further if
  non-overlapping. Untested in any predecessor.
- **(c)** 168h target on hourly bars (1 week). Maximum horizon
  expansion within the existing data span; effective sample count
  drops significantly if non-overlapping; theoretically motivated by
  weekly-cyclic structure in crypto (weekend dampening).
- **(d)** 1d-OHLCV resolution change (move from hourly to daily
  bars entirely). Largest scaffold change: the data loader needs a
  daily-bar mode; the feature derivation needs daily-cadence stats;
  the model RF=1021 bars now spans 1021 days (~2.8 years) which
  exceeds the available data span. **Analyst rejects** unless
  operator explicitly wants this.

**Analyst default: (a) 24h on hourly bars.** Minimal scaffold change;
the 24h-vs-1h SNR improvement is the biggest single jump in the
horizon ladder. If (a) also lands F4, the cheaper next step is to
skip (b) / (c) and route to scope (b) retire — we've then ruled out
1h AND 24h, which is enough evidence that TCN-shape isn't right
for this data.

**Sub-Q2a — overlapping vs non-overlapping 24h targets**: under Q2 =
(a), do we emit a target every 1h (overlapping) or every 24h
(non-overlapping)?

- Overlapping: ~87k effective samples per BS-1 span; targets are
  autocorrelated (~0.95 between t and t+1); training stability is
  the open empirical question.
- Non-overlapping: ~365 effective samples per BS-1 span per symbol
  × 10 symbols = ~3,650 total; targets are independent; high risk
  of underfit on the small sample count.

Analyst default: **overlapping** — the autocorrelation in
overlapping targets is well-known in time-series DL and doesn't
cause catastrophic overfit at the v2.5 TCN's ~4.4M parameter count.
Architect-confirm at M-T1.

### Q3 — Single checkpoint or multiple (scope a / c only)

If Q1 = (a) or (c), how many new checkpoints to retrain?

- **(a)** One BS-1 checkpoint at the chosen Q2 horizon. Cheap test
  of the horizon hypothesis. If H1 falsifies on BS-1 alone, route
  to scope (b) retire (we don't need BS-2 to confirm). **Analyst-
  recommended default.**
- **(b)** Both BS-1 + BS-2 checkpoints at the chosen Q2 horizon
  (mirror the existing BS-1 / BS-2 split). Higher cost (~2× wall-
  clock) but lets the operator see horizon-bumped Sharpe on both
  train spans (2023 vs 2023 + Q1 2024). Useful if H1 partially
  confirms on BS-1 only and we want to disambiguate train-span
  effects from horizon effects.
- **(c)** One checkpoint at one horizon (BS-1 + 24h) now; expand to
  BS-2 + 24h or BS-1 + 48h later if first succeeds. **Hybrid path
  with operator-gated milestones.** Decoupled from the M-RETRAIN
  milestone; lower commitment per spawn.

**Analyst default: (a)** — one BS-1 + 24h. Cheapest path to
falsify H1; if H1 partial-confirms on BS-1, operator decides next
spawn based on results. Honors the "spend the budget on the
highest-information experiment first" principle.

### Q4 — Keep TCN topology (scope a / c only)

If Q1 = (a) or (c), keep the current TCN topology (8 residual blocks
× dilation {1..128} × 96 channels, RF=1021) or change it for the
horizon-bumped training?

- **(a)** Keep topology byte-identical. Tests the **horizon
  hypothesis cleanly** — the only variable changing is the target
  derivation. **Analyst-recommended default.** K5 enforces.
- **(b)** Increase RF for 24h-horizon prediction. The existing
  RF=1021 bars = ~42 days is already much larger than the 24h
  prediction horizon; arguably the RF is fine. But for 168h
  (Q2=c), RF would only span ~6× the horizon — marginal. Architect
  could add a 9th residual block (dilation 256) to push RF to 2045
  bars = ~85 days. Adds parameters + training time.
- **(c)** Prune topology to compute-budget. Drop to 6 blocks
  (RF=509) or 4 blocks (RF=255). Cheaper training but smaller
  receptive field; risk of underfit at 24h horizon.

**Analyst default: (a) keep topology.** The horizon-bump is an
experiment about target-derivation, not architecture; conflating
the two muddies the H1 test. If H1 confirms on (a) and the operator
wants to push further, a follow-on `v25-tcn-rf-extension` feature
can sweep RF. Out of scope here.

### Q5 — Data span (scope a / c only)

If Q1 = (a) or (c), which training-data span to use?

- **(a)** `2023-01-01..2023-12-31` (mirror BS-1 train span). The
  existing BS-1 checkpoint uses this span; the horizon-bumped
  BS-1-h24 checkpoint should use the same for clean comparison.
  **Analyst-recommended default.** Honors the existing
  per-checkpoint convention.
- **(b)** `2023-01-01..2024-03-31` (mirror BS-2 train span). Useful
  if Q3 = (b) (both checkpoints). For Q3 = (a), this is wrong (it
  mixes BS-1 and BS-2 evaluation conventions).
- **(c)** Longer span — e.g. `2023-01-01..2024-12-31` (full
  available data). Maximises sample count but conflates train-data-
  span with horizon-bump effect.

**Analyst default: (a) BS-1 train span** for Q3 = (a) checkpoints,
**(a) + (b) for Q3 = (b) checkpoints**. Match the per-checkpoint
convention from the predecessor recalibrate / threshold-tuning
ships.

### Q6 — Retire decision threshold (scope a / c only)

If scope (a) horizon-bump comes back F4 at M-FORECAST-DIST, what's
the next step?

- **(a)** Retire v2.5 TCN immediately (route to scope b in a
  follow-on feature). We've now ruled out 1h AND Q2-chosen horizon;
  the cheapest next experiment is PatchTST. **Analyst-recommended
  default.**
- **(b)** Try the next horizon up (Q2 = (b) 48h or (c) 168h) before
  retiring. Higher cost (~5-7 more days wall-clock); higher
  confidence at retire time. Useful if operator's prior is "horizon
  is genuinely the issue and we need to find the right one."
- **(c)** Defer to v2.6 bake-off. Let PatchTST + Transformer ship;
  evaluate TCN-at-1h vs TCN-at-Q2 vs PatchTST vs Transformer
  head-to-head; let the bake-off pick. Most thorough; longest
  schedule.

**Analyst default: (a) retire immediately.** Information theory:
two F4 verdicts at two different horizons is sufficient evidence
that TCN-shape isn't right for this data; further horizon-sweep
is throwing good compute after bad. The bake-off retirement gate
provides the final canonical retire if needed.

### Q7 — Anchor strategy + version pin

If new anchors land, version + naming convention?

- **(a)** New anchors under version `v2.7.0-horizon-bump` with
  naming `{report-family}-h<Q2>-realdata` (e.g.
  `forecast-distribution-bs1-h24-realdata`). Existing 28 anchors
  byte-identical. **Analyst-recommended default.**
- **(b)** Version `v2.7.0-horizon-bump-recalibrated` (since the new
  checkpoint goes through σ_train recalibration). Slightly more
  verbose; signals the post-training recalibration step explicitly.
- **(c)** No version pin — fold into `v2.6.2-threshold-tuning`.
  **Analyst rejects** — this is a new model version (different
  `model_revision` SHA), not a tuning sweep over an existing model.

**Analyst default: (a)** `v2.7.0-horizon-bump`.

## Cost estimate (per scope)

| Scope | Wall-clock | Owner |
|-------|------------|-------|
| Author this brief (R1-R8 + H1-H3 + K1-K7 + Q1-Q7) | 30-60 min | analyst (this brief) |
| **Scope (a) — horizon-bump retrain only** | | |
| Operator-decide Q1-Q7 | minutes (autoapprove Q2-Q7) | operator |
| Architect lock + ADR-0037 + decomp.md | 2-4 hr | architect |
| Implement `--target-horizon-bars` + horizon-aware target loop | 2-4 hr | developer |
| Training run (Q3 = (a), 1 checkpoint at Q2 = 24h, ~5-7 days/checkpoint) | **5-7 days wall-clock** | orchestrator-monitor |
| σ_train recalibration of new checkpoint | 10-15 min | orchestrator |
| Re-run forecast_distribution + backtest | 1-2 hr | orchestrator |
| Tester gate + Presenter deck | 1-2 hr | tester + presenter |
| **Scope (a) total** | **~7-10 days wall-clock** | |
| **Scope (b) — retire-promote-PatchTST** | | |
| Operator-decide Q1 | minutes | operator |
| Architect drafts ADR-0036 + retire decision record | 1-2 hr | architect |
| Flip v25-tcn-overlay frontmatter; queue PatchTST analyst | 30 min | orchestrator |
| PatchTST analyst pass + architect + developer + tester + presenter | **~4-6 weeks wall-clock** (per v25a stub) | downstream |
| **Scope (b) total** | **~4-6 weeks wall-clock** (this feature: <1 day; PatchTST follow-on dominates) | |
| **Scope (c) — both in parallel** | | |
| Operator-decide Q1 + (a)'s Q2-Q7 | minutes | operator |
| (a) and (b) in parallel as above | union of (a) compute + (b) human bandwidth | |
| **Scope (c) total** | **~6-9 weeks wall-clock** | |
| **Scope (d) — defer-on-live** | | |
| Operator-decide Q1 + Q7 (defer-duration) | minutes | operator |
| Architect drafts ADR-0036-alt | 1-2 hr | architect |
| Queue v25-tcn-live-trade-eval | 30 min | orchestrator |
| Live-trade window | **30-90 days wall-clock** (capital-deployment-bound) | downstream |
| **Scope (d) total** | **~30-90 days wall-clock** (this feature: <1 day; live window dominates) | |

**Operator's decision-relevant framing**:

- Scope (a) is the most-information-per-compute-day path **iff** H1
  has a non-trivial prior probability.
- Scope (b) is the least-this-feature-effort path; the cost shifts
  to the PatchTST follow-on.
- Scope (c) is the highest-total-cost, highest-total-information
  path.
- Scope (d) is the cheapest **iff** capital is deployed.

## Out of scope

- **No retraining at 1h horizon.** That's already shipped (BS-1 +
  BS-2 anchored checkpoints from `v25-tcn-overlay` v2.5.0).
- **No σ_train computation-bug fix in `train_tcn.rs:733-741`.**
  The bug stays as-is; the post-training σ_train recalibration via
  the existing `recalibrate_sigma_train` bin is the contracted path
  per [ADR-0035](../architecture/adr/0035-tcn-sigma-train-recalibration.md).
  Out-of-scope to fix the training-time computation; if scope (a)
  ships, the new horizon-bumped checkpoint goes through the same
  metadata-overlay recalibration path.
- **No (τ, ε) sweep on the new horizon-bumped checkpoint** within
  this feature. If H1 partial-confirms (F-verdict not F4 but Sharpe
  delta uncertain), a follow-on threshold-tuning sweep on the
  horizon-bumped checkpoint is a separate spawn.
- **No mutation of the existing 28 anchors.** R7 + R8 are the
  load-bearing non-regression contract.
- **No ADR-0033 amendment.** F-verdict algorithm stays IMMUTABLE
  across this feature regardless of scope.
- **No deletion of v25-tcn-overlay code.** Scope (b) retire is a
  documentation status flip; the checkpoint files, strategy builders,
  and forecast_distribution / sharpe_comparison bins stay shipped
  and re-activatable.
- **No PatchTST design work.** Scope (b) / (c) spawns a follow-on
  PatchTST analyst pass; the design happens there, not here.
- **No new external crate dependencies.** Per CLAUDE.md.
- **No iced bump.** Per CLAUDE.md.

## Sources cited

- [`spec/v25-tcn-threshold-tuning/feature.md`](../v25-tcn-threshold-tuning/feature.md)
  — predecessor feature brief (v0.1.0, shipped 2026-05-21); joint
  T-MARGINAL + T-MARGINAL verdict; R3 routing table; H1 falsification
  result (no cell unlocked +0.10 Sharpe).
- [`spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md`](../v25-tcn-threshold-tuning/reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md),
  [`reports/threshold-sweep-bs2-realdata-recalibrated-20260521.md`](../v25-tcn-threshold-tuning/reports/threshold-sweep-bs2-realdata-recalibrated-20260521.md)
  — 90-backtest sweep evidence; BS-1 +0.018 at (τ=0.1, ε=0.001),
  BS-2 +0.045 at same cell.
- [`spec/v25-tcn-recalibrate/feature.md`](../v25-tcn-recalibrate/feature.md)
  — predecessor feature brief (v0.1.0, shipped 2026-05-21); σ_train
  608× / 580× inflation eliminated; gate-survival jump 0% → 40-89%;
  F-verdict stays F4 under immutable ADR-0033 § D3.
- [`spec/v25-tcn-recalibrate/presentations/v25-tcn-recalibrate-2026-05-21.md`](../v25-tcn-recalibrate/presentations/v25-tcn-recalibrate-2026-05-21.md)
  — presenter deck; routing (c) chosen by operator; cheap τ-sweep
  first, multi-week retrain queued as fallback.
- [`spec/v25-tcn-alpha-investigation/feature.md`](../v25-tcn-alpha-investigation/feature.md)
  — F-verdict + 4-bucket failure-mode taxonomy (R4); routes F4 to
  this feature.
- [`spec/v25-tcn-overlay/feature.md`](../v25-tcn-overlay/feature.md)
  — parent feature (in-progress); D5 thresholds (τ=0.6, ε=0.0005);
  success criterion (+0.10 Sharpe lift); BS-1 / BS-2 train spans.
- [`spec/v25-dl-forecast-overlay/feature.md`](../v25-dl-forecast-overlay/feature.md)
  — 4-phase DL roadmap; phase 1 (TCN, this feature's subject),
  phase 2 (PatchTST), phase 3 (Transformer), phase 4 (bake-off).
- [`spec/v25a-patchtst-overlay/feature.md`](../v25a-patchtst-overlay/feature.md)
  — phase 2 stub; status `roadmap`, owner `pending-analyst`; scope (b)
  / (c) promote this Queue → Active.
- [`spec/v25b-transformer-overlay/feature.md`](../v25b-transformer-overlay/feature.md)
  — phase 3 stub.
- [`spec/v26-forecast-bakeoff/feature.md`](../v26-forecast-bakeoff/feature.md)
  — phase 4 stub; canonical retirement gate for all three model
  families.
- [`spec/backlog.md`](../backlog.md) § Strategy — Queue entry for
  this feature (currently ACTIVATION TRIGGERED 2026-05-21).
- [ADR-0028](../architecture/adr/0028-v25-dl-forecast-overlay-candle.md)
  — model-agnostic candle-direction decision; covers all 4 phases.
- [ADR-0029](../architecture/adr/0029-tcn-checkpoint-provenance.md)
  — TCN checkpoint provenance; `model_revision` SHA derivation
  (the new horizon parameter enters the canonical-arch-descriptor).
- [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)
  — backtest realdata path; this feature's backtests inherit.
- [ADR-0033](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md)
  § D3 — F-verdict algorithm (IMMUTABLE across this feature).
- [ADR-0034](../architecture/adr/0034-cockpit-training-control.md)
  — cockpit training control; `train_events` row emission for
  scope (a) / (c) training runs.
- [ADR-0035](../architecture/adr/0035-tcn-sigma-train-recalibration.md)
  — post-training σ_train recalibration via metadata overlay;
  D4 σ_train-not-in-safetensors invariant carries forward to
  horizon-bumped checkpoints.
- [`crates/forecast/src/bin/train_tcn.rs`](../../crates/forecast/src/bin/train_tcn.rs)
  — training scaffold; the `--target-horizon-bars` CLI flag lands
  here (scope a / c).
- [`crates/forecast/src/features.rs:627-628`](../../crates/forecast/src/features.rs)
  — target derivation; the `(close_t1 / close_t).ln()` formula
  extends to `(close_{t+N} / close_t).ln()` for the horizon-bumped
  case.
- [`crates/forecast/src/tcn.rs`](../../crates/forecast/src/tcn.rs)
  `:99` (CONTEXT_LEN=256), `:1090-1099` (RF=1021 receptive field
  arithmetic), `:302-311` (1×1 conv head topology), `:534` (σ_train
  read site).
- [`crates/forecast/src/bin/recalibrate_sigma_train.rs`](../../crates/forecast/src/bin/recalibrate_sigma_train.rs)
  — post-training σ_train recalibration bin; horizon-agnostic
  (reuses verbatim for any new checkpoint).
- [`crates/forecast/src/bin/forecast_distribution.rs`](../../crates/forecast/src/bin/forecast_distribution.rs)
  — F-verdict reporting bin; reuses verbatim for any new checkpoint
  via the `--metadata-path` flag.
- [`crates/strategy/src/tcn_overlay_momentum.rs`](../../crates/strategy/src/tcn_overlay_momentum.rs)
  — overlay composition; the existing 8 strategy builders
  (`with_tcn_bs{1,2}` × {ledger, tuned, ledger_tuned}) stay
  byte-identical under R8. New horizon-bump builders are additive
  under scope (a) / (c).
- [`crates/backtest/src/scenarios/tcn_overlay_weights.rs`](../../crates/backtest/src/scenarios/tcn_overlay_weights.rs)
  — backtest scenario; runs against any TCN checkpoint via the
  strategy builder.
- Apple Silicon Metal training-time empirical record: per the
  predecessor v25-tcn-overlay reporting, 1h-horizon BS-1 training
  was ~4-5 days wall-clock; horizon-bump scope (a) estimate
  ~5-7 days uses this as the baseline + small multiplier for
  target-derivation overhead. **Honest variance**: ±2 days. The
  R3 cost tripwire bounds the worst case.

## Changelog

- 2026-05-21 (analyst): initial brief authored. R1-R8 requirements,
  H1-H3 hypothesis register, K1-K7 risk register, Q1-Q7 operator-
  decide questions. **Q1 is HARD BLOCKER — no safe analyst default**;
  Q2-Q7 carry analyst-recommended defaults. Predecessor:
  `v25-tcn-threshold-tuning v0.1.0`. Parent: `v25-tcn-overlay v2.5.0
  (in-progress)`. Trace row `REQ-V25-TCN-HORIZON-BUMP-OR-RETIRE-001`
  opened `draft` state. Promoted from Queue § Strategy → Active in
  `spec/backlog.md` per the threshold-tuning ship's joint T-MARGINAL
  verdict + operator routing (c) directive. Cost estimate ranges:
  scope (a) ~7-10 days; scope (b) ~4-6 weeks; scope (c) ~6-9 weeks;
  scope (d) ~30-90 days. HANDOFF → operator-decide (Q1-Q7) →
  architect.
