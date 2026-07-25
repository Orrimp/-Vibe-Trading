---
title: v2.5 DL forecast overlay journey — retrospective
date: 2026-05-22
authors: [orchestrator, operator]
status: archive
tags: [retrospective, v2.5, dl, tcn, patchtst, retired, evidence-archive]
---

# v2.5 DL forecast overlay journey — retrospective

> **What this is.** A what-not-to-do reference + evidence archive for the
> v2.5 DL forecast overlay umbrella, which was retired 2026-05-22 after
> 7 feature ships established that v2.5-era DL approaches (TCN +
> PatchTST) do not extract +0.10 Sharpe-delta alpha on the v1
> cross-sectional momentum baseline using real Binance hourly OHLCV.
> Preserved so a future engineer / agent doesn't re-derive the same
> conclusion from scratch and so future research can branch off the
> joint F4-F4-F4 evidence chain.

## Headline verdict

**Joint F4 across 3 model checkpoints / 2 model families / 2 horizons.**

| Ship | Date | Model | Horizon | F-verdict | Sharpe-delta vs v1 baseline |
|------|------|-------|---------|-----------|------------------------------|
| `v25-tcn-overlay v2.5.0` | (pre-session anchor) | TCN BS-1 | 1h | F4 | +0.018 (T-MARGINAL) |
| `v25-tcn-overlay v2.5.0` | (pre-session anchor) | TCN BS-2 | 1h | F4 | +0.045 (T-MARGINAL) |
| `v25a-patchtst-overlay v0.1.0` | 2026-05-22 | PatchTST BS-1 | **24h** | F4 | **+0.006 (lowest)** |

**Read**: PatchTST @ 24h scored lower than TCN @ 1h. H1 (24h-horizon
unlocks signal where 1h failed) FALSIFIED. The retired-at-1h hypothesis
plus the falsified-at-24h hypothesis jointly imply v2.5-era DL paradigms
do not predict next-1h or next-24h log-returns well enough on hourly
crypto OHLCV to clear the +0.10 alpha-unlock threshold.

## The 7-ship journey

```
v25-tcn-overlay v2.5.0 (pre-session)
        │
        ├─ shipped TCN BS-1 + BS-2 anchored checkpoints; 4 anchors locked
        │
        ▼
backtest-real-binance-data v0.1.0 (pre-session)
        │
        ├─ ran TCN overlay on real Binance hourly OHLCV
        ├─ reported `dampened=0` across all 4 -realdata scenarios
        ├─ 4 v2.6.0-realdata anchors locked
        │
        ▼
v25-tcn-alpha-investigation v0.1.0 (shipped 2026-05-19)
        │
        ├─ forensic read-only investigation; F-verdict algorithm (ADR-0033)
        ├─ F4 joint verdict on BS-1 + BS-2
        ├─ σ_train calibration anomaly surfaced (std ~500× smaller than σ_train)
        ├─ 3 anchors locked (forecast-distribution-bs{1,2}-realdata + sharpe-comparison-realdata)
        │
        ▼
v25-tcn-recalibrate v0.1.0 (shipped 2026-05-20)
        │
        ├─ σ_train bug eliminated: BS-1 10.95 → 0.018, BS-2 6.92 → 0.012
        ├─ Root cause: in-loop accumulator in train_tcn.rs:606,676-678,733-741
        │  never reset between epochs → final scalar dominated by pre-convergence
        │  trajectory variance
        ├─ ADR-0035 codified the cross-phase post-training σ_train pattern
        ├─ Gate-survival jumped 0% → 40-89% under recalibrated σ_train
        ├─ F-verdict STAYED F4 (frac_inside_epsilon < 0.5 threshold per ADR-0033 § D3)
        ├─ 4 new anchors locked (forecast-distribution-bs{1,2}-realdata-recalibrated
        │  + recalibrate-sigma-train-bs{1,2})
        │
        ▼
v25-tcn-threshold-tuning v0.1.0 (shipped 2026-05-20)
        │
        ├─ cheap τ × ε sweep: 9 × 5 = 45 cells per checkpoint × 2 = 90 backtests
        ├─ JOINT T-MARGINAL verdict (no (τ, ε) tuple unlocked alpha)
        ├─ BS-1 best: τ=0.1 / ε=0.001 / +0.018 Sharpe-delta
        ├─ BS-2 best: τ=0.1 / ε=0.001 / +0.045 Sharpe-delta
        ├─ Both below +0.10 T-ALPHA-UNLOCKED threshold
        ├─ 4-way rayon::par_iter; 4 additive `_tuned` builders shipped
        ├─ 2 new anchors locked (threshold-sweep-bs{1,2}-realdata-recalibrated)
        │
        ▼
v25-tcn-horizon-bump-or-retire v0.1.0 (shipped 2026-05-21)
        │
        ├─ Policy/decision feature (no code change)
        ├─ Operator decided Q1=(b) RETIRE v2.5 TCN at 1h horizon
        ├─ Rationale: 3 substantive ships exhausted 1h-horizon TCN hypothesis
        ├─ Pivot multi-week budget to v2.5a PatchTST
        ├─ 28 originals byte-identical; 0 new anchors
        │
        ▼
v25a-patchtst-overlay v0.1.0 (shipped 2026-05-22)
        │
        ├─ PatchTST per Nie et al 2022; ~431k params; candle pure-Rust
        ├─ d_model=128, n_heads=4, n_layers=3, patch_len=16, stride=8
        ├─ context_len=336, target_horizon_bars=24, dropout=0.2
        ├─ Training: 30 epochs / 7h 45min wall-clock / final train_loss 2.6e-5
        ├─ ADR-0035 § D1 post-training σ_train pattern from start: σ=0.007053
        ├─ ADR-0036 PatchTST training contract codified
        ├─ F-verdict F4 — Sharpe-delta +0.006 (LOWER than retired TCN @ 1h)
        ├─ Gate-survival 55.8% at τ=0.6 (gate works; model doesn't predict)
        ├─ H1 (24h horizon unlocks signal) FALSIFIED
        ├─ 2 new anchors locked (forecast-distribution-patchtst-bs1-realdata +
        │  top10-2023-fy-patchtst-overlay-realdata under v2.5a.0-patchtst)
        │
        ▼
Strategic retirement 2026-05-22
        │
        ├─ Operator decided routing (a) at v25a presenter approval
        ├─ v25-dl-forecast-overlay umbrella → deprecated
        ├─ v25b-transformer-overlay → deprecated (never shipped)
        ├─ v26-forecast-bakeoff → deprecated (never shipped; premise moot)
        ├─ ~3-5 week compute budget freed for non-DL or strategy-side work
```

## Anchor evolution

| Phase | Anchor count | Cumulative | Locked under |
|-------|--------------|------------|---------------|
| Pre-session baseline | 22 | 22 | mixed v2.x |
| `v25-tcn-alpha-investigation` ship | +3 | 25 | (legacy version) |
| `v25-tcn-recalibrate` ship | +4 (actually 3 derivation reports landed as anchors at recalibrate; threshold-tuning added 2 more under different version) — at recalibrate ship-time: 25 + 3 = 28 — actual 22-anchor base + 4 forecast-distribution-recalibrated + 2 sharpe pre-existing... see anchors.toml for exact ordering | 28 | `v2.6.1-alpha-investigation-recalibrated` |
| `v25-tcn-threshold-tuning` | +2 | 28 (some re-locked, see anchors.toml) | `v2.6.2-threshold-tuning` |
| `v25a-patchtst-overlay` | +2 | **30** (final) | `v2.5a.0-patchtst` |

**Hard invariant preserved across all 7 ships**: every pre-feature anchor stays byte-identical. The `tcn_byte_identity` test + the `patchtst_overlay_neutrality` K4 test enforce this structurally. No code in `crates/forecast/src/tcn.rs` was modified after v25-tcn-overlay v2.5.0 shipped; PatchTST landed as additive enum variants in dispatch bins + sibling files everywhere.

## ADRs locked across the journey

| ADR | Title | Status | Purpose |
|-----|-------|--------|---------|
| `0028` | v25-dl-forecast-overlay-candle | accepted | candle ML framework choice (covers all 4 phases) |
| `0029` | tcn-checkpoint-provenance | accepted | metadata canonicaliser; extended additively for PatchTST |
| `0032` | backtest-realdata-path-and-revision-pin | accepted | realdata anchor-version-pin |
| `0033` | tcn-alpha-investigation-report-shape | **immutable** | F-verdict algorithm § D3 — F1/F2/F3/F4 priority tree |
| `0035` | tcn-sigma-train-recalibration | accepted | cross-phase post-training σ_train pattern (TCN + PatchTST + future) |
| `0036` | patchtst-training-contract | accepted | PatchTST topology + training scaffold + invariants |

ADR-0033 is **immutable** — it provides the canonical F-verdict algorithm that both v2.5 TCN and v2.5a PatchTST forecast-distribution reports used. Any future forecaster ship can inherit it verbatim.

ADR-0035 is **cross-phase** — codifies the post-training σ_train derivation pattern (NOT the deprecated in-loop accumulator pattern in the original TCN training scaffold) so future model families don't re-introduce the same calibration bug.

## Lessons learned

### 1. Cheap-first investigation order beats jumping to retrain

The recalibrate → threshold-tuning sequence cost ~10 wall-clock hours total but eliminated 2 confounders (σ_train calibration bug + gating-too-tight hypothesis) before the operator committed to the multi-week retrain budget. If the original v25-tcn-overlay had retrained at 24h horizon based on the `dampened=0` finding alone, the σ_train bug would have been baked in AND the retrain would have failed to disambiguate "wrong horizon" from "wrong gating" from "wrong calibration". The cheap-first order produced clean evidence.

### 2. F-verdict immutability locked a comparable measurement bar

Because ADR-0033 § D3 stays immutable, the F-verdict from TCN alpha-investigation (2026-05-19) is directly comparable to the F-verdict from PatchTST forecast-distribution (2026-05-22) — same priority tree, same thresholds, same report shape. The "F4 across 3 checkpoints" headline only has evidential force because the measurement bar didn't drift. Future model families should reuse ADR-0033 verbatim.

### 3. σ_train derivation is load-bearing for gating semantics

The 608×/580× inflation bug in `train_tcn.rs:606,676-678,733-741` (in-loop accumulator never reset) silently disabled the entire confidence gate in production. Threshold-tuning would have been a no-op without recalibrate first; without recalibrate, the F-verdict classifier might have been misled (F2 hypothesis would have superficially looked plausible because the inflated σ_train made std/σ_train tiny — but in the wrong direction, so F2 didn't trigger). The post-training pattern in ADR-0035 § D1 is the correct shape; the deprecated in-loop accumulator should be removed from any cookbook used for future training scaffolds.

### 4. Architecture-paradigm tests beat hyperparameter sweeps when the prior is mixed

The thinking around threshold-tuning was: "is gating wrong, or is signal absent?" The cheap τ × ε sweep answered "gating works fine; signal is genuinely weak". After that, paradigm change (TCN → PatchTST) had a higher information value than horizon-bump-on-TCN (which would have been more parameters of the same paradigm). PatchTST @ 24h provided the cross-paradigm evidence cheaply: convolutional vs patch-attention both F4 implies the problem is not the architecture-family axis.

### 5. PatchTST trained ~10× faster than projected

Architect estimated 3-5 days for the PatchTST training run on Apple Silicon Metal; actual was **7h 45min** for 30 epochs. The earlier "3-5 days" framing was based on v2.5 TCN training-loop wall-clock numbers (which had been multi-day per checkpoint). For future training-time estimates on similar hardware + framework: a ~430k-param model with 76k 24h-target training windows runs at ~16 min/epoch (with overlapping targets). Scale roughly linearly with parameter count × windows × epochs.

### 6. Operator routing-at-presenter is the right decision layer

The F-verdict + σ_train + threshold-tuning + retire-vs-bump + paradigm-test cascade required 3 substantive operator routing decisions (recalibrate Q1=Yes BS-1+feed; threshold-tuning routing (c) both follow-ons; horizon-bump-or-retire Q1=(b) retire+pivot; v25a routing (a) retire entire umbrella). Each was decision-grade evidence the analyst couldn't autoapprove. Multi-week budgets need explicit human judgment.

## Out-of-scope work flagged but never funded

1. **PatchTST BS-2 checkpoint** — would have tested "did we just get unlucky with BS-1?" Deferred to v0.1.1; ultimately retired with the umbrella.
2. **PatchTST hyperparameter sweep** — patch_len × stride × d_model × n_heads × n_layers. Deferred to v0.1.1; retired with the umbrella.
3. **Crypto-specific features** — realized-vol bands, funding-rate, OI. The 5-feature input (logret/logrange/logvol_z/hour_sin/hour_cos) was carried forward across all v2.5 ships unchanged. Future strategy-side reformulation can revisit if useful.
4. **Walk-forward retraining** — v2.6 bake-off was the canonical walk-forward home; retired without shipping. If a future strategy-side approach (volatility forecasting, regime classification) revisits, walk-forward is the right scope home.
5. **v2.5b vanilla decoder-only Transformer** — the autoregressive Kronos-shape successor. Deferred from F4-F4 prior (joint evidence said low EV).
6. **v25b-llm-arbiter** (Queue § Process / tooling) — depended on v2.5/v2.5a/v2.5b shipping. Now moot.

## What the next research direction should NOT chase

Per joint F4-F4-F4 evidence:

- **Same 5-feature input + 1h or 24h next-bar log-return target** — already exhausted. Any v2.x DL forecaster using this signal+target combination on hourly crypto OHLCV is unlikely to clear +0.10 Sharpe-delta vs v1 momentum baseline.
- **TCN/PatchTST/iTransformer on this task framing** — paradigm-family axis explored. iTransformer was rejected at v25a Q1 due to narrow 5-feature input; the analyst's reasoning applies.
- **More epochs / more parameters / different optimiser** — gating-too-tight (F3) was ruled out by threshold-tuning; calibration (F2) was ruled out by recalibrate; collapse-to-zero (F1) was ruled out by alpha-investigation. The model genuinely doesn't predict returns; throwing more compute won't fix that.

## What the next research direction COULD usefully chase

- **Different target** — volatility forecasting (predict σ not μ); regime classification (trending vs mean-reverting); microstructure (predict order-flow imbalance instead of price)
- **Different horizon shape** — 168h trend signal; intra-session vs inter-session decomposition; weekly seasonality
- **Different feature set** — crypto-specific (funding rate, OI, perp basis); cross-venue (Binance + Coinbase + Kraken price difference); on-chain (mempool, transfer volume)
- **Strategy-side reformulation** — accept that 1h/24h log-return prediction on this overlay shape is the wrong task. Reformulate signal-overlay-composition entirely.
- **Reflection-memory consumption** — the v2 LLM analyst now has Memory + Models screens (Phase F). Could the reflection-memory itself + LLM debate provide a forecast-equivalent signal without DL?
- **Non-DL approaches** — regime-classifier (HMM, kernel methods, statistical filters) on this same data may extract the +0.10 Sharpe-delta that DL didn't.

## Operator's research-budget pivot decision (2026-05-22)

Operator routing (a) at v25a presenter:
> "Retire entire 4-phase DL forecast overlay project. Highest-EV given
> joint F4 evidence. v2.5 TCN failed at 1h. v2.5a PatchTST failed at
> 24h. v2.5b vanilla Transformer is unlikely to outperform given 2
> paradigms already failed. Free up ~3-5 weeks of compute budget. Mark
> v25-dl-forecast-overlay as terminal. Pivot research budget to
> strategy-side reformulation or other work."

## References

- All shipped feature folders under `spec/v25-tcn-*/` and `spec/v25a-patchtst-overlay/`
- Retired feature folders preserved under `spec/v25-dl-forecast-overlay/`, `spec/v25b-transformer-overlay/`, `spec/v26-forecast-bakeoff/` (status: deprecated)
- ADRs: `_bmad-output/planning-artifacts/architecture/decisions/{0028,0029,0032,0033,0035,0036}*.md`
- Presenter decks (canonical narrative reads): `spec/v25a-patchtst-overlay/presentations/v25a-patchtst-overlay-2026-05-22.md`, `spec/v25-tcn-threshold-tuning/presentations/`, `spec/v25-tcn-recalibrate/presentations/`, `spec/v25-tcn-alpha-investigation/presentations/`
- Anchor manifest: `spec/anchors.toml` rows under versions `v2.6.0-realdata`, `v2.6.1-alpha-investigation-recalibrated`, `v2.6.2-threshold-tuning`, `v2.5a.0-patchtst`

## Pre-pivot breadcrumb chain

- Original Kronos approach (dropped 2026-05-16): `docs/dev-notes/kronos-evaluation-2026-05-10.md` (superseded by ADR-0028)
- Pivot to candle-trained TCN/PatchTST/Transformer: ADR-0028
- 4-phase DL umbrella: `spec/v25-dl-forecast-overlay/feature.md` (now retired 2026-05-22)
- **This retrospective**: the v2.5 chapter closes here.
