# v2.5 DL forecaster — pre-analyst reading list (2026-05-16)

> **Purpose.** Operator-driven literature spike per the Kronos-pivot lesson:
> committing to a model approach without doing the reading produces
> Kronos-style cycles. The orchestrator (this file's author) curates
> pointers; the operator does the synthesis and picks the direction. **No
> recommendation in this dev-note.** When the operator picks, then spawn the
> v2.5 analyst (task #25) with the chosen direction in the brief.

## Operating constraints (locked)

| Constraint | Value |
|------------|-------|
| ML framework | `candle` (pure-Rust, per CLAUDE.md) — Metal backend on Apple Silicon |
| Realistic model size | ~5-10M params max (Metal backend less mature than CUDA) |
| Data | Hourly OHLCV, 10 USDT pairs, 2023+2024 (bootstrapped by task #28) |
| Inference target | Single-bar next-bar (matches v1 momentum cadence) — analyst can revisit |
| Determinism contract | Strict-replay via `crates/replay-cache/` (already built) |
| Output | `ForecastOverlay` value (Direction + confidence) — model-agnostic |

## Three model families to consider

The candidate space is bounded by "small enough to train on M-series" and
"deterministic enough to audit." Three families fit.

### A. Temporal Convolutional Network (TCN)

Dilated causal 1D convolutions. Fewer parameters than a Transformer for the
same receptive field. Deterministic inference (no autoregressive sampling).

| Source | Type | One-line description |
|--------|------|---------------------|
| Bai, Kolter, Koltun 2018 — *An Empirical Evaluation of Generic Convolutional and Recurrent Networks for Sequence Modeling* | paper | Canonical TCN paper. Compares against LSTM/GRU on standard sequence tasks. |
| [locuslab/TCN](https://github.com/locuslab/TCN) | repo | Reference PyTorch implementation from the paper. ~200 LOC core. |
| [Keras-TCN](https://github.com/philipperemy/keras-tcn) | repo | Well-documented Keras impl. Useful as architecture reference. |

What you'd learn building it: dilated convolutions, residual blocks, receptive-field math, MSE/MAE training on regression.

### B. Patch-based Transformer (PatchTST family)

Recent (2023-2024) Transformer architectures designed specifically for
time-series forecasting. Token = patch of contiguous bars instead of single
timesteps.

| Source | Type | One-line description |
|--------|------|---------------------|
| Nie et al 2023 — *A Time Series is Worth 64 Words: Long-term Forecasting with Transformers* (PatchTST) | paper | ICLR 2023. Patches + channel-independence; competitive with much larger models. |
| [yuqinie98/PatchTST](https://github.com/yuqinie98/PatchTST) | repo | Official impl. Small enough to fit our params budget. |
| Liu et al 2024 — *iTransformer: Inverted Transformers Are Effective for Time Series Forecasting* | paper | ICLR 2024. Tokens-as-variates inversion; surprisingly small + competitive. |
| [thuml/iTransformer](https://github.com/thuml/iTransformer) | repo | Official impl. |

What you'd learn building it: attention from scratch in `candle`, patching strategy, channel-independence vs channel-mixed.

### C. Vanilla small decoder-only Transformer

The "Kronos shape" but without Kronos's specifics: train a small (5-10M
param) decoder-only Transformer on quantised OHLCV tokens from scratch.

| Source | Type | One-line description |
|--------|------|---------------------|
| Radford et al 2019 (GPT-2) — *Language Models are Unsupervised Multitask Learners* | paper | The decoder-only Transformer recipe. Skip if you know it. |
| [huggingface/candle/candle-transformers](https://github.com/huggingface/candle/tree/main/candle-transformers) | repo | candle's own transformer building blocks — exact crate you'd consume. |
| Karpathy's [nanoGPT](https://github.com/karpathy/nanoGPT) | repo | The smallest-clearest decoder-only Transformer impl. ~300 LOC. Good mental model. |

What you'd learn: tokenisation strategy for OHLCV, full Transformer training loop, autoregressive sampling.

## Tokenisation / discretisation strategy

Orthogonal to the model family choice — affects all three.

| Approach | Source | Notes |
|----------|--------|-------|
| Continuous regression (predict next OHLCV as floats) | Standard TCN/Transformer regression | Simplest; loss is MSE/MAE. No tokenisation. |
| Quantile binning (predict P(next return in bin_k)) | Standard quantile classification | Discrete; loss is cross-entropy. Maps cleanly to confidence in `ForecastOverlay`. |
| VQ-VAE tokens (learned discrete codebook over OHLCV patches) | Kronos's actual approach; see [van den Oord 2017 — Neural Discrete Representation Learning](https://arxiv.org/abs/1711.00937) | Most powerful, most complex. Adds an encoder pretraining step. |

## Crypto-specific work — relevant or not

Most academic time-series forecasting work uses standardised benchmarks (M4,
ETT, electricity, traffic). Almost no published work uses crypto OHLCV
directly. The crypto-specific ML literature is mostly on order books
(DeepLOB family) which is a different problem.

**Implication:** any model choice will be "applied to crypto for the first
time in this project's history" — there is no pre-existing crypto-OHLCV
benchmark to anchor to. Success criterion (vs v1 momentum baseline) is
therefore unavoidably a project-internal measurement, not an external
benchmark match.

## candle ecosystem maturity check

- `candle-core` + `candle-nn` — production-ready for the operations needed
  (Linear, Conv1d, MultiHeadAttention, LayerNorm). Metal backend is real.
- `candle-transformers` — high-level Transformer blocks. Mostly LLM-oriented
  but the building blocks are reusable.
- Time-series-specific candle examples — sparse. You'll be re-implementing
  the model from scratch in candle, not adapting a published candle impl.
  That's the point of the "learn by building" frame.

## Questions to answer from the reading

When you're done reading, you should be able to answer (these flow directly
into the v2.5 analyst brief):

1. **Model family** — TCN, PatchTST/iTransformer, or vanilla decoder-only Transformer?
2. **Tokenisation** — continuous regression / quantile bins / VQ-VAE?
3. **Receptive field / context window** — how many bars of history per inference?
4. **Output shape** — Direction (Up/Down/Flat) + confidence directly, OR predict next OHLCV and post-process to Direction?
5. **Loss function** — MSE / MAE / CE / hybrid?
6. **Training loop shape** — full backprop on M-series? Gradient accumulation? Batch size?
7. **Checkpoint provenance** — what gets hashed into `model_revision` (architecture + weights + training data span + seed)?

When the answers are concrete enough that you could argue them aloud,
**then** spawn the analyst (task #25).

## What this file deliberately doesn't do

- No recommended model.
- No "best for crypto" claim.
- No code.

The point is to make the operator's choice **traceable** — the analyst
brief that lands after this reading will cite specific sources for each
design decision, so we can later look back and say "the design choice was
because X paper said Y" rather than "the analyst recommended it."
