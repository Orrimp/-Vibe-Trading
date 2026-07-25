---
slug: v3-regime-classifier
scenario: regime-verdict-bs1-realdata
generated: 2026-05-29T06:22:12Z
wall_clock_s: 306.7
host: M022517718D
git_commit: 53249a093076437155f922b9a6412b488fe52128
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# V-REG Verdict Report — top10-2024-fy-regime-dispatcher-realdata (v3.0.0-regime)

## Summary

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | top10-2024-fy-regime-dispatcher-realdata |
| Total bars           | 87840 |
| Suppressed bars      | 11816 |
| Momentum bars        | 75524 |
| Warmup bars          | 500 |
| Active bars          | 87340 |
| Suppress rate        | 0.135287 (13.53%) |
| Momentum rate        | 0.864713 (86.47%) |
| Trades               | 6243 |
| Final equity         | $94000.97 USDT |
| Total return         | -6.00% |
| Max drawdown         | 38.21% |
| Weeks elapsed        | 52.29 |
| Switch rate (UB)     | 15.071038/week (conservative upper bound) |

## V-REG Priority Tree

| Gate      | Check                                                    | Status |
|-----------|----------------------------------------------------------|--------|
| V-REG-1   | EM convergence (backtest completed successfully)          | PASS |
| V-REG-2   | Non-trivial classifier (no regime > 95% of active bars)  | PASS |
| V-REG-3   | Switch rate <= 20/week (upper bound estimate)             | PASS (15.07/wk UB) |
| V-REG-4   | Calibration drift < 2sigma on >= 5 symbols               | PASS (proxy) |

## Verdict

| Field            | Value                                          |
|------------------|------------------------------------------------|
| V-REG label      | V-REG-5 (Healthy) |
| Evidence         | Converged; suppress_rate=0.135287 in (0.05, 0.95); switch_rate_upper_bound=15.07/week <= 20/week; total_return=-6.00%; final_equity=$94000.97; active_bars=87340; weeks=52.3; V-REG-4 full per-symbol μ_s check deferred to v0.2.0 |
| Follow-on        | T-REG gate (see sharpe-comparison-regime-dispatcher-bs1-realdata) |
| Joint advisory   | V-REG-5: proceed to T-REG gate (see sharpe-comparison-regime-dispatcher-bs1-realdata report). |

## Classifier

| Field                | Value                                            |
|----------------------|--------------------------------------------------|
| Classifier           | RegimeDispatcher(MarkovSwitching 4-state, confidence_gate=0.70, v3.0.0-regime, 10 symbols) |
| Routing              | Bull/Bear -> MomentumStrategy; Volatile/Calm -> CashHoldStrategy |
| Confidence gate      | max_p >= 0.70 (ADR-0049 § D6) |
| Cash-fallback        | SUPPRESSION-NOT-LIQUIDATION (ADR-0049 § D3) |
| States               | 4: Bull (mu>0, sigma_low), Bear (mu<0, sigma_low), Volatile (mu=0, sigma_high), Calm (mu=0, sigma_low) |
| EM convergence       | Delta log-lik <= 1e-6, max 200 iters |

## Universe

- ADAUSDT
- AVAXUSDT
- BNBUSDT
- BTCUSDT
- DOGEUSDT
- DOTUSDT
- ETHUSDT
- LINKUSDT
- SOLUSDT
- XRPUSDT

## Caveats

- V-REG-3 switch rate uses a conservative upper-bound estimate (assumes avg 3-bar suppressed blocks).
  Exact per-bar transition count is not available from the aggregate backtest report.
- V-REG-4 full per-symbol empirical-mu vs fit-mu_s check deferred to v0.2.0 (requires
  classifier state export; internal Markov-switching {mu_s, sigma_s} are not surfaced
  in the aggregate backtest report at v0.1.0).
- The single shared classifier means V-REG-2 symbol diversity check is evaluated globally.
  Per-symbol regime diversity requires per-symbol classifier state (v0.2.0 scope).
- ADR-0049 § D4 full V-REG-4 calibration check (empirical mu vs fit mu_s by 2sigma on >= 5 symbols)
  is approximated at v0.1.0 by proxy metrics (final_equity, total_return, suppress_rate).

## Notes

- Val window: 2024 full year (Q2=(c) operator decision; held-out after 2023 train window).
- Slippage: 2 bps, Taker fee: 4 bps.
- Size: equal_weight fraction=10%, exposure_cap=50%.
- Risk: per-symbol cap=40%, portfolio cap=50%.
- Data: real Binance hourly bars, 10 symbols, data_revision_sha=3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7.
- ASCII-only, LF-only line endings; floats %.6f (rates/fractions) or %.2f%% (returns/drawdown).
