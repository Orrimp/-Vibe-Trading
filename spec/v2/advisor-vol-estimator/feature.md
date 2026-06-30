---
slug: advisor-vol-estimator
status: tester-done
owner: tester
version: 2.0.0
updated: 2026-06-30
---

# P1-5 Shared σ̂ Vol Estimator

The shared multi-horizon realized-vol estimator consumed by BOTH P1-3
(drawdown-control overlay) and P1-4 (vol-targeting overlay reparameterisation).
A **pure, stateless module** — no traits, no I/O, no state — following the
"vol-for-sizing ≠ return-prediction" layering rule.

**Design reference:** [`v2-architecture.md`](../v2-architecture.md) §1 P1-5 +
§6.0 D5 (operator-ratified binding location). Research:
[`research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md`](../../../research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md)
§6 P1-C; crypto cross-check:
[`research/crypto-market-structure/application-volatility-regimes-and-overlays.md`](../../../research/crypto-market-structure/application-volatility-regimes-and-overlays.md)
§6 F.

## Location (binding — D5)

`crates/strategy/src/vol_estimator.rs`

Rationale from §6.0 D5: it is a *sizing input*; both consumers live in
`strategy`; `ui` never touches it (it sees only the overlay's equity output
via the existing mirror); keeping it in `strategy` preserves the
"vol-for-sizing ≠ return-prediction" line. NOT in `crates/forecast`.

## Public API

### `log_returns_from_bars(bars: &[Bar]) -> Vec<f64>`

Extracts `ln(close_t / close_{t-1})` from a `Bar` slice. Returns `n-1`
log-returns from `n` bars. The `Decimal` → `f64` conversion happens here (the
established stats-boundary pattern from `scorecard.rs`). Non-positive closes
yield `0.0` to prevent `ln(0)` blowup.

### `realized_vol_from_returns(returns: &[f64], window: usize) -> f64`

Population standard deviation of the last `window` returns (or all available
returns if `window > len`). The reference/baseline estimator: simple,
equal-weight, no decay. Returns `0.0` for empty input or `window == 0`.

### `ewma_realized_vol(returns: &[f64], lambda: f64) -> Vec<f64>`

Exponentially-weighted moving sigma. Recurrence:

```text
σ²_t = (1 − λ) · r_t² + λ · σ²_{t-1}
```

Initialised from the unconditional (population) variance of the full series.
Same length as `returns`.

### `har_realized_vol(returns: &[f64]) -> Vec<f64>`

HAR-RV (Corsi 2009): equal-weight (1/3, 1/3, 1/3) blend of the daily (1-bar),
weekly (5-bar mean), and monthly (22-bar mean) absolute-return components.
Equal-weight form (not OLS-fitted) per the research doc's "do not over-engineer"
guidance. Same length as `returns`.

## Half-life ↔ λ mapping (documented in module)

```text
Half-life H (bars) and λ:
    λ = exp(ln(0.5) / H)   →   H = ln(0.5) / ln(λ)
```

| Constant | λ | Half-life |
|---|---|---|
| `LAMBDA_126D_DAILY` | `exp(ln(0.5)/126)` ≈ 0.994_514 | ≈ 126 daily bars (architect default, P1-A) |
| `LAMBDA_126D_HOURLY` | `exp(ln(0.5)/3024)` ≈ 0.999_771 | ≈ 3 024 hourly bars (≈ 126 days at 24h/day) |
| `LAMBDA_RISKMETRICS` | 0.94 | ≈ 11.3 daily bars (RiskMetrics; faster/more reactive) |

The slow 126-day default is the architect's preference from P1-A: "loose &
slow & cost-survivable" to avoid overtrading on transient volatility spikes.

## Not in this increment

- Consumer wiring: `vol_targeting_overlay.rs` keeps its GARCH source (P1-4
  will reparameterise it to use `ewma_realized_vol`). `drawdown_control_overlay.rs`
  is not built yet (P1-3).
- Any optional gated-model vol forecast behind the `forecast` feature flag
  (CX-5): that is strictly opt-in / never the default — out of scope for this
  increment.

ADR-0079 (reserved — written/registered atomically when P1-3 drawdown overlay
lands; this module is P1-5 infra it depends on).
