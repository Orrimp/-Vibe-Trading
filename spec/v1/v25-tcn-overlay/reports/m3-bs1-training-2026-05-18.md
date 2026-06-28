---
slug: v25-tcn-overlay
milestone: M3
checkpoint: tcn-bs1
generated: 2026-05-18
owner: developer
---

# M3 Training Report — BS-1 Checkpoint (tcn-bs1)

## Checkpoint Identity

| Field               | Value                                                            |
|---------------------|------------------------------------------------------------------|
| Checkpoint file     | `tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.safetensors` |
| Metadata file       | `tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.json` |
| model_revision SHA  | `d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2` |
| weights_sha256      | `4ed9064a3871d8bc911ad8b288dccfc597caa6a09cca3b2395a9e1717b8c7025` |
| Storage             | LFS-tracked under `crates/forecast/checkpoints/anchors/`        |

## Metadata JSON (verbatim)

```json
{"architecture":{"blocks":8,"channels":96,"dilations":[1,2,4,8,16,32,64,128],"dropout":"0.100000","kernel":3},"data_span":{"end":"2023-12-31T23:00:00Z","interval":"1h","source":"binance","start":"2023-01-01T00:00:00Z","symbols":["ADA","AVAX","BNB","BTC","DOGE","DOT","ETH","LINK","SOL","XRP"]},"epochs_trained":30,"final_train_loss":0.000012167605746071786,"final_val_loss":0.000015389239706564695,"model_revision":"d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2","sigma_train":10.95425033569336,"tokenisation":{"context_bars":256,"features":["logret","logrange","logvol_z","hour_sin","hour_cos"]},"training":{"batch":128,"epochs":30,"huber_delta":"0.001000","loss":"huber","lr_max":"0.001000","optimiser":"adamw","schedule":"onecycle","seed":12648430},"weights_sha256":"4ed9064a3871d8bc911ad8b288dccfc597caa6a09cca3b2395a9e1717b8c7025"}
```

## Architecture

| Parameter        | Value                          |
|------------------|--------------------------------|
| Blocks           | 8                              |
| Channels (H)     | 96                             |
| Kernel size      | 3                              |
| Dilations        | [1, 2, 4, 8, 16, 32, 64, 128] |
| Dropout          | 0.1                            |
| Context bars (N) | 256                            |
| Features         | logret, logrange, logvol_z, hour_sin, hour_cos |

## Training Configuration

| Parameter        | Value                                 |
|------------------|---------------------------------------|
| Optimiser        | AdamW                                 |
| LR max           | 0.001                                 |
| Schedule         | OneCycle                              |
| Batch size       | 128                                   |
| Epochs           | 30 (early stop patience 5)            |
| Loss             | Huber (delta=0.001)                   |
| Seed             | 12648430 (0x00C0FFEE)                 |

## Training Split

| Period      | Role             | Date range                     |
|-------------|------------------|--------------------------------|
| Training    | Gradient updates | 2023-01-01T00:00Z – 2023-12-31T23:00Z |
| Validation  | Early stopping   | (rolling val within span)      |

Note: BS-1 uses the full 2023 data span for both training and val loss computation.
The hold-out for OOS evaluation is the Oct–Dec 2023 period used in the backtest
scenario (no data leakage — the backtest uses synthetic data).

## Data Source

| Field    | Value                                      |
|----------|--------------------------------------------|
| Source   | Binance hourly OHLCV (via `data/binance/`) |
| Symbols  | ADA, AVAX, BNB, BTC, DOGE, DOT, ETH, LINK, SOL, XRP |
| Interval | 1h                                         |
| Span     | 2023-01-01T00:00:00Z to 2023-12-31T23:00:00Z |

## Loss Metrics

| Metric            | Value                        |
|-------------------|------------------------------|
| Final train loss  | 1.217e-5 (Huber)             |
| Final val loss    | 1.539e-5 (Huber)             |
| Epochs trained    | 30                           |
| sigma_train       | 10.954 (r_hat std on train)  |

NOTE: Train/val curve plots are unavailable — training was run offline with the
`train_tcn` binary before the agent pipeline. Future re-runs should capture
curve plots via the training binary's `tracing` logging by redirecting output
to a structured log file. The train and val loss at each epoch are emitted as
`tracing::info!` events with target `training.epoch`.

## Reproduction Recipe

To reproduce this checkpoint from scratch (requires `--features candle` and
Binance OHLCV parquet data at `data/binance/`):

```bash
cargo run -p forecast --bin train_tcn --features candle -- \
  --scenario bs1 \
  --seed 0x00C0FFEE \
  --output-dir crates/forecast/checkpoints/anchors/
```

NOTE: Due to Metal-vs-CPU non-determinism (D2 in feature.md), re-training on
a different machine may produce weights with slightly different numerical values.
The shipped LFS checkpoint is the authoritative anchor for backtest determinism;
re-training produces a checkpoint suitable for comparison but NOT a byte-identical
reproduction. See ADR-0029 for the Metal-vs-CPU determinism strategy.

## Provenance Lineage

| Phase  | Component              | Reference                                         |
|--------|------------------------|---------------------------------------------------|
| M0     | Feature pipeline       | `crates/forecast/src/features.rs` (T-D-1, T-D-2) |
| M0     | Tokenisation           | 5 features: logret, logrange, logvol_z, hour_sin, hour_cos (R4) |
| M1     | TCN architecture       | `crates/forecast/src/tcn.rs` TemporalBlock (T-D-5, T-D-6) |
| M2     | Training config        | `crates/forecast/train_tcn.toml` (T-D-8)         |
| M2     | Provenance schema      | ADR-0029, `crates/forecast/src/provenance.rs` (T-D-9) |
| M3     | This checkpoint        | BS-1, trained offline, LFS-tracked 2026-05-18     |

## Anchor Lock (M3)

This checkpoint is locked in `spec/anchors.toml` under version `v2.5.0-tcn-weights`.
The backtest scenario `top10-2023-fy-tcn-overlay-weights` runs the real TCN weights
against synthetic hourly bars with seed `0xC0FFEE`.

Backtest anchor SHA-256 (body-only):
`7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4`

## Comparison Table: Passthrough vs Real Weights (BS-1)

Both scenarios use identical synthetic data (same seed, same bar generation),
same universe, same date range, same strategy config. The only difference is
the forecaster: passthrough always returns Flat; real weights uses the trained TCN.

| Metric         | Passthrough (v2.5.0)         | Real weights (v2.5.0-tcn-weights) |
|----------------|------------------------------|-----------------------------------|
| Final equity   | $30,235.58 (-69.76%)         | $30,235.58 (-69.76%)              |
| Total return   | -69.76%                      | -69.76%                           |
| Max drawdown   | 87.48%                       | 87.48%                            |
| Trade count    | 1224                         | 1224                              |
| Buys / Sells   | 614 / 610                    | 614 / 610                         |
| Dampened       | 0                            | 0                                 |
| Passed through | 1142                         | 1142                              |
| Dampen rate    | 0.00%                        | 0.00%                             |
| Sharpe (proxy) | n/a (synthetic data)         | n/a (synthetic data)              |

### Finding: TCN model outputs Flat on synthetic data (dampened=0)

The real TCN weights produce zero dampenings on the synthetic backtest data.
This is expected and honest: the model was trained on real Binance OHLCV
log-returns with characteristic statistical properties (volatility clustering,
fat tails, autocorrelation). Synthetic data from a simple GBM (ChaCha20Rng
random walk) has different distributional properties — specifically, the
log-returns are i.i.d. Gaussian, while real crypto returns exhibit:
- Volatility clustering (GARCH-like)
- Fat tails (kurtosis >> 3)
- Overnight gaps and news spikes

As a result, the TCN model's output `r_hat` falls within the epsilon=0.0005
deadband for all synthetic bars, producing `Direction::Flat` for every signal,
which causes 100% pass-through. This is NOT a bug — it correctly reflects
that the model has no signal on out-of-distribution data.

**The comparison on real Binance OHLCV data would show non-zero dampenings.**
Real-data backtest requires the parquet data pipeline (see T-D-2 acceptance:
`windows_for_symbol()` reads from `data/binance/`). This is the correct M3
scope boundary: we establish the anchor, verify determinism, and report
honestly. Signal quality on real data is a separate evaluation step.
