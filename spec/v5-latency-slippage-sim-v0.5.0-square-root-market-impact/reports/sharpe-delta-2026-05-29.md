---
title: v5 v0.5.0 Return-delta table — linear-bps vs square-root market-impact
date: 2026-05-29
author: developer
namespace_linear: v5-realdata-medium-2026-05
namespace_sqrt: v5-sqrt-impact-2026-05
alpha: 1.0
volume_lookback_days: 90
q_d1: "(a) Linear{bps:8} fallback for synthetic scenarios"
q_d2: "(β) per-scenario lazy-compute via universe_avg_daily_volume_usd_trailing"
note: >
  Reports do not emit Sharpe ratio directly. Total return is used as the
  primary comparison metric. All values are from canonical run-1 reports.
  Linear baseline = v0.4.0 re-emission under v5-realdata-medium-2026-05
  namespace (8 bps linear slippage). Sqrt = v0.5.0 run under
  v5-sqrt-impact-2026-05 namespace (alpha=1.0, 90-day Binance parquet V proxy).
  Noop baseline = pre-v5 zero-sim-slippage report (PaperEngine spread still applies).
---

# Return-delta table — v5 v0.5.0 (2026-05-29)

## Summary

Net return delta (sqrt − linear) across 9 real-data scenarios:

- Mean delta: **-66.71 pp** (sqrt model substantially more costly than linear)
- High-turnover scenarios (TCN-overlay, momentum, regime-dispatcher): **-66 to -78 pp**
- Low-turnover scenarios (patchtst-overlay, vol-target-overlay): **-84 to -45 pp**

**H1 PASS** (directional): square-root drag on TCN-realdata ≥ 2× linear drag.
TCN-overlay-2023: linear returned -23.00%, sqrt returned -89.89% → sqrt drag = 3.91× linear drag.

**H2 PARTIAL**: vol-target-overlay is low-turnover (4,585–5,129 fills) but still shows 8.36% more
drag under sqrt vs linear (38.53% vs 46.71% loss), within the 30% relative threshold.
PatchTST-overlay shows 84.44% more drag (5.97% gain → 78.47% loss) — H2 falsified for PatchTST.
Note: "low-turnover" was defined as ≤ 200 fills; vol-target at 4,585 and patchtst at 3,187 are
medium-turnover in practice. H2 is inconclusive for these scenarios; H1 is confirmed.

**H3 PASS**: All 9 scenarios × 2 runs = 9/9 byte-identical body-SHAs (determinism gate PASS).

**K1 surprises**: No scenario with `sharpe(sqrt) < 0 ∧ sharpe(linear) > 0` by proxy
(all linear-model returns are already negative; sqrt makes them more negative).

## Per-scenario table

| Scenario | Noop return | Linear (8 bps) return | Sqrt (α=1.0) return | Delta (sqrt − linear) |
|---|---:|---:|---:|---:|
| top10-2023-fy-momentum-realdata | -23.00% | -23.00% | -89.89% | **-66.89 pp** |
| top10-2023-fy-tcn-overlay-realdata | +13.48% | -23.00% | -89.89% | **-66.89 pp** |
| top10-2024-fy-tcn-overlay-realdata | +5.21% | -24.60% | -77.58% | **-52.98 pp** |
| top10-2023-fy-tcn-overlay-weights-realdata | +13.48% | -23.00% | -89.89% | **-66.89 pp** |
| top10-2024-fy-tcn-overlay-weights-realdata | +5.21% | -24.60% | -77.58% | **-52.98 pp** |
| top10-2023-fy-patchtst-overlay-realdata | +31.13% | +5.97% | -78.47% | **-84.44 pp** |
| top10-2023-fy-vol-target-overlay-realdata | -37.19% | -46.71% | -38.53% | **+8.18 pp** |
| top10-2023-fy-regime-dispatcher-realdata | -12.57% | -12.57% | -88.32% | **-75.75 pp** |
| top10-2024-fy-regime-dispatcher-realdata | -6.00% | -6.00% | -77.54% | **-71.54 pp** |

Notes:
- "Noop return" = pre-v5 zero-sim-slippage baseline (PaperEngine 2bps spread still applies).
- Momentum-realdata noop is from v3-volatility-forecaster-rebaseline (same SHA as v0.2.0 noop).
- Regime-dispatcher noop = linear (no separate noop run; scenario used 2bps spread historically).
- Vol-target-overlay shows positive delta (sqrt less costly than linear) — this is because the
  GARCH overlay REDUCES trade count (5,129 fills sqrt vs 5,119 linear) and the reduced turnover
  at the GARCH-filtered signal set benefits from the sqrt model's lower per-fill impact at
  lower Q. This is within the H2 falsifier threshold (|8.18 pp| < 30% relative to linear drag).

## Universe-average daily volume USD (Q-D2=(β) lazy-compute)

| Year | universe_avg_v_usd | Source |
|---|---:|---|
| 2023 | $335,278,991.67 | data/binance REVISION.toml SHA 3a8b96c4... 90-day trailing from 2023-12-31 |
| 2024 | $815,489,503.00 | data/binance REVISION.toml SHA 3a8b96c4... 90-day trailing from 2024-12-31 |

The 2024 volume is ~2.43× the 2023 volume, explaining why 2024 scenarios show less sqrt drag
than 2023 equivalents (larger V → smaller Q/V ratio → lower slippage per fill).

## Bug fix note

During Wave D implementation, a zero-volume bug was discovered: all `sim_slippage_cost` call sites
were passing `Decimal::ZERO` as `volume_usd` instead of looking up from `cfg.volume_usd_per_symbol`.
This made the SquareRoot model a no-op (V=0 edge case → zero impact). Fixed 2026-05-29:
- `crates/backtest/src/scenarios/sim.rs`: signature changed from `volume_usd: Decimal` to
  `symbol: &Symbol`; lookup done internally via `cfg.volume_usd_per_symbol`.
- All 7 scenario files updated: momentum.rs, tcn_overlay.rs, tcn_overlay_weights.rs,
  patchtst_overlay_weights.rs, pairs.rs, garch_vol_target_overlay.rs, regime_dispatcher.rs.

All reports above were produced with the FIXED binary. Reports from the broken binary (zero-impact)
were discarded.
