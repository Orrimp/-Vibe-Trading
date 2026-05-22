---
slug: v3-volatility-forecaster
scenario: vol-verdict-bs1-realdata
generated: 2026-05-22T08:10:05Z
wall_clock_s: 0.6
host: M022517718D
git_commit: af64141392096269f7d4a90dfbd4df79e3a4d16f
checkpoint_revision: 991324772ba077355731c2f551e3412430070b76468f6044261161a9160c0c71
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
verdict: V3
---
# Vol-forecast V-verdict report — BS-1 (real Binance hourly OHLCV, GARCH(1,1))

## Checkpoint

| Field               | Value                                          |
|---------------------|------------------------------------------------|
| Anchor scenario     | garch-bs1                                       |
| checkpoint_revision | 991324772ba077355731c2f551e3412430070b76468f6044261161a9160c0c71 |
| target_kind         | Parkinson                                             |
| target_horizon_bars | 24                                              |
| evaluation_span     | 2023-01-01T00:00:00Z .. 2024-01-01T00:00:00Z   |
| n_symbols           | 10                                              |
| n_predictions_total | 76800                                          |

## Per-symbol QLIKE table

| symbol   | n_pred | qlike_garch | qlike_const | improvement_pct | mean_sigma_hat | mean_sigma_real | calib_ratio | std_sigma_hat | std_sigma_real |
|----------|--------|-------------|-------------|-----------------|----------------|-----------------|-------------|---------------|----------------|
| ADAUSDT  | 7680   | 0.426992    | 0.518094    | 17.584064        | 0.006768       | 0.006869        | 0.985247    | 0.003620      | 0.003679       |
| AVAXUSDT | 7680   | 0.566233    | 13.653577    | 95.852861        | 0.018693       | 0.008101        | 2.307620    | 0.233192      | 0.004742       |
| BNBUSDT  | 7680   | 0.549405    | 0.615777    | 10.778597        | 0.004617       | 0.004573        | 1.009649    | 0.002683      | 0.002620       |
| BTCUSDT  | 7680   | 0.769428    | 1.901616    | 59.538193        | 0.004067       | 0.004219        | 0.963859    | 0.002665      | 0.002186       |
| DOGEUSDT | 7680   | 0.920710    | 18.945262    | 95.140156        | 0.071541       | 0.006981        | 10.247541    | 1.931281      | 0.004234       |
| DOTUSDT  | 7680   | 0.553155    | 17.334936    | 96.809018        | 0.065635       | 0.006501        | 10.096677    | 1.243604      | 0.003301       |
| ETHUSDT  | 7680   | 0.584227    | 0.713697    | 18.140665        | 0.004459       | 0.004542        | 0.981762    | 0.002581      | 0.002058       |
| LINKUSDT | 7680   | 0.437470    | 0.478717    | 8.616203        | 0.007382       | 0.007645        | 0.965601    | 0.003695      | 0.003652       |
| SOLUSDT  | 7680   | 0.363077    | 0.440710    | 17.615355        | 0.009221       | 0.009517        | 0.968941    | 0.003845      | 0.004774       |
| XRPUSDT  | 7680   | 0.956240    | 0.685794    | -39.435460        | 0.006818       | 0.006853        | 0.995011    | 0.004319      | 0.004847       |

## Aggregate statistics

| Field                       | Value      |
|-----------------------------|------------|
| qlike_garch_mean            | 0.612694   |
| qlike_constant_mean         | 5.528818   |
| qlike_garch_max             | 0.956240   |
| qlike_garch_min             | 0.363077   |
| qlike_dispersion            | 2.633711   |
| mean_calibration_ratio      | 2.952191   |
| n_symbols_improving_≥10pct  | 8          |

## Verdict

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Case              | V3                                             |
| Trigger evidence  | mean_calibration_ratio = mean_over_symbols(mean(σ̂)/mean(σ_realized)) = 2.952191 outside [0.7, 1.4] |
| Routes to         | v3-garch-calibration-tune |

## Notes

- Read-only against `crates/forecast/checkpoints/anchors/garch-bs1-991324772ba077355731c2f551e3412430070b76468f6044261161a9160c0c71.json`.
- QLIKE per Patton 2011 *Volatility forecast comparison using
  imperfect volatility proxies* — robust to noise in the Parkinson
  σ_realized proxy; preferred over MSE for vol forecasts.
- Parkinson realized-vol target: `σ̂_P² = (1/(4·ln 2)) · mean over k of (ln(high_k/low_k))²`.
- V-verdict algorithm: see [ADR-0038 § D1](../architecture/adr/0038-vol-forecast-verdict-shape.md#d1-v-verdict-priority-tree-parallel-to-adr-0033--d3-not-extension).
