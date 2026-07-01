---
adr: 0081
title: Cost-model opt-in VolScaledSpread variant (default unchanged)
status: accepted
date: 2026-07-01
supersedes: none
superseded-by: none
---

# ADR-0081: Cost-model opt-in `VolScaledSpread` variant (default unchanged)

## Context

P1-6 (v2 Phase 2D) adds a state-aware, vol-scaled spread model to
`crates/cost/src/slippage.rs`. The architectural constraint (R3, §6.0 D6,
operator-ratified 2026-06-28) is that ANY change to the *default* cost path
would break all 119 anchored backtest-report body SHAs (CX-7). Changing the
default for v2 was explicitly rejected:

> "Opt-in-forever for v2. Revisit a default bump only if a coin is found where
> flat-bps materially mis-costs a crownable arm."
> — v2-architecture.md §2 R3

The research rationale (`backtesting[47]`, `crypto-market-structure[90]`):
crypto spreads widen 2–3× in high-volatility/stress regimes, and visible
order-book depth overstates true liquidity (~31% spoofable on Coinbase).
A credible backtest should never assume mid-price fills in stress regimes.

## Decision

Add `SlippageModel::VolScaledSpread { base_bps, vol_multiplier, sigma_window, sigma_lambda }`
as a **new, opt-in-forever variant** in `crates/cost/src/slippage.rs`.

**`SlippageModel::default()` REMAINS `Linear { bps: 8 }` — IMMUTABLE.**

### D1 — σ̂ source: inline EWMA, not `strategy::vol_estimator`

The `cost` crate does NOT depend on `crates/strategy`. `strategy` already
dev-depends on `cost`; adding `cost → strategy` creates a cycle. The
identical 5-line EWMA recurrence (`σ²_t = (1-λ)·r²_t + λ·σ²_{t-1}`) is
inlined in `apply_slippage_vol_scaled_bps`. This is an **explicit,
documented divergence** from the D5 "shared vol-estimator" rule; the
divergence is forced by the dep-graph constraint, not a design preference.

### D2 — Chosen defaults (operator-ratified)

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| `base_bps` | `8` | Matches the Linear default (ADR-0045 D1 pin) — no surprise floor increase |
| `vol_multiplier` | `2.0` | Research midpoint of the 2–3× widen observed in high-vol regimes (`backtesting[47]`) |
| `sigma_window` | `20` | ~1 trading month of hourly bars; responsive without overreacting to transient spikes |
| `sigma_lambda` | `0.94` | RiskMetrics λ (≈11.3-day half-life); faster-reacting than the 126-day sizing default, appropriate for liquidity-stress detection |

**Export:** `DEFAULT_VOL_SCALED_SPREAD` const provides these defaults in one binding.

### D3 — Full dispatcher extended

`apply_slippage_model_with_returns(signal_price, side, notional, model, volume_usd, bar_log_returns)`
is the new primary dispatcher. `apply_slippage_model` remains as a backward-compat
wrapper that passes `&[]` for `bar_log_returns` (VolScaledSpread falls back to
`base_bps` only — correct warm-up behaviour).

### D4 — Fee-sensitivity sweep helper

`fee_sensitivity_report(base_bps, sigma_hat, &[factors]) -> Vec<(f64, f64)>` provides
the "spec-curve for costs" review: re-rank verdicts across a grid of cost assumptions
per `backtesting[24][47]`. Report-only; no gate band touched.

### D5 — Decimal/f64 boundary (ADR-0003 preserved)

Log-returns and EWMA variance are `f64` (statistical/dimensionless). The conversion
back to basis points (u32) caps at `MAX_SLIPPAGE_BPS` before the `Decimal` fill-price
multiply. The `base_bps` floor and fill-price computation remain fully `Decimal`.

### D6 — Opt-in-forever contract (BINDING)

The 119 anchored body-SHAs in `spec/anchors.toml` are byte-immutable (ADR-0038 §D6).
`VolScaledSpread` is never reachable from any anchored CLI path:
- The anchored CLI (`param_robustness_sweep` bin et al.) constructs
  `LatencySlippageSimConfig::default()` → `SlippageModel::Linear { bps: 8 }`.
- The advisor bake-off uses `write_report=false` → no anchor SHA produced.
- The only callers that would use `VolScaledSpread` are opt-in, operator-configured.

**Enforcement:** the `default_is_linear_bps_8` unit test in `slippage.rs` asserts
`SlippageModel::default() == Linear { bps: 8 }` and must never be deleted.

## Alternatives considered

- **Default bump (rejected).** Changing the default would re-emit all 119 anchored
  reports under ADR-0038 §D6b — a multi-day migration for marginal gain at €200 scale.
  The research confirms impact ≈ 0 for retail size (`backtesting[13][41][95]`).
- **Pull `strategy::vol_estimator` (rejected).** Would create a dep cycle.
  See D1 above.
- **σ̂ as a new `apply_slippage_model_with_vol` param of type `Decimal` (rejected).**
  Callers don't have σ̂ pre-computed; forcing them to pass it externalises an EWMA
  computation that belongs inside the model. The `bar_log_returns: &[f64]` slice is
  the natural input — the model owns its own estimator.

## Consequences

- `SlippageModel` is no longer `Eq` (because `f64` fields in `VolScaledSpread` are
  not `Eq`). The `PartialEq` derive is preserved. Any code asserting `==` on two
  `VolScaledSpread` instances should use field-wise comparison or compare bps outputs.
- The `apply_slippage_model` API is backward-compatible (same signature, new wrapper
  over `apply_slippage_model_with_returns`). All existing call sites compile unchanged.
- Serde: `VolScaledSpread { base_bps, vol_multiplier, sigma_window, sigma_lambda }`
  serialises with `kind = "vol_scaled_spread"` (snake_case tag). Existing configs
  deserialise as `Linear` (no `kind` key) or with `kind = "linear"` — unchanged.

## Changelog

- 2026-07-01 (developer): initial accept; P1-6 Phase 2D.
