---
slug: advisor-vol-overlay-reposition
status: tester-done
owner: tester
version: 2.0.0
updated: 2026-06-30
---

# P1-4 Vol-Overlay Reposition — Risk Tool, Not Sharpe Tool

The shipped `VolTargetingOverlay` (`crates/strategy/src/vol_targeting_overlay.rs`)
is repositioned as a **risk-shaping tool, not a Sharpe tool**. This is not a new
overlay — it is an honest reparameterisation of the existing one, informed by the
research finding that crypto's leverage effect is reversed (γ = −0.261, Brini–Lenz
2024 vs equities' +0.115), meaning the Sharpe-gain mechanism does not apply to
crypto. What does apply — universally, across 60+ assets (Harvey et al.) — is
**drawdown and tail reduction**: thinner left tail, shallower max drawdown, lower
vol-of-vol. The overlay now promises only that.

**Design reference:** [`v2-architecture.md`](../v2-architecture.md) §1 P1-4 (binding).
Research: [`research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md`](../../../research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md)
§6 P1-A.

## Operator promise (the framing change)

The overlay is a **de-risking tool**: when volatility spikes, it cuts position size
toward the target. When volatility drops, it holds (does not lever up). The operator
should expect:

- Shallower drawdowns and thinner left tail vs the un-overlaid baseline.
- No reliable Sharpe improvement on crypto (γ = −0.261 reversed leverage effect).
- Per-symbol **return-vol correlation ρ(returns, σ̂)**: the operator reads this to
  verify whether the leverage-effect mechanism is even present on their coin. If ρ > 0
  (the typical crypto regime: FOMO / vol-rises-after-rallies), Sharpe improvement is
  not mechanistically possible; expect only risk shaping.

## What shipped (this dev increment)

### `VolSource` enum (new)

Selects the sigma estimation source:
- `VolSource::Ewma` — slow EWMA from `vol_estimator::ewma_realized_vol` with the
  configured `ewma_lambda` (P1-4 default).
- `VolSource::Garch` — legacy GARCH(1,1) recurrence (ADR-0038 § D5 backward-compat).

### `VolTargetingConfig` — four new fields

| Field | Type | Default (compat) | P1-4 default |
|---|---|---|---|
| `vol_source` | `VolSource` | `Garch` (backward-compat) | `Ewma` |
| `ewma_lambda` | `f64` | `LAMBDA_126D_HOURLY` ≈ 0.999771 | same |
| `no_trade_band` | `f64` | 0.0 (no band; backward-compat) | 0.05 (5%) |
| `derisk_only` | `bool` | `false` (backward-compat) | `true` |

`VolTargetingConfig::p1_4_defaults()` constructor delivers the honest defaults.

**Backward-compatible `Default` trait:** the four original fields are unchanged;
`vol_source = Garch`, `no_trade_band = 0.0`, `derisk_only = false` replicate
pre-P1-4 behaviour so the existing e2e test stays green without modification.

### EWMA vol source — bar cadence choice

The inner `MomentumStrategy` is configured at 1-hour bars. `LAMBDA_126D_HOURLY`
(≈ 0.999771) corresponds to a 126-day half-life at hourly cadence (3024 bars).
This is the slow, cost-survivable choice from Boyd–Candès–Hastie: tight tracking
costs 1105%/yr turnover vs 93% open-loop; slow decay avoids chasing transient spikes.

### No-trade band

`apply_policy(raw_scale, current_scale)` → `(final_scale, band_suppressed, derisk_suppressed)`.

Only resize when `|candidate − current| / current > no_trade_band`. With 5% band:
- A 3% vol change → no resize (held).
- A 12% vol change → resize (allowed).

### De-risk-only

When `derisk_only = true`, the candidate scale is capped at `current_scale.min(1.0)`.
This means the overlay may only ever *reduce* position size on a vol spike. On a vol
drop, it holds (never levers up). The clamp is `1.0` (no leverage on a no-leverage
account).

### `ReturnVolCorrelation` struct (new telemetry)

Accumulates `(return, sigma_hat)` pairs per symbol and computes Pearson ρ
incrementally. Exposed as `overlay.return_vol_correlation: BTreeMap<Symbol, ReturnVolCorrelation>`.

**Interpretation:**
- ρ < 0: leverage effect present (vol rises after down moves — equity pattern). Sharpe
  improvement possible via vol targeting.
- ρ > 0: leverage effect reversed (vol rises after up moves — crypto FOMO). Only risk
  shaping; no Sharpe gain. This is the expected regime on crypto (γ = −0.261).
- ρ ≈ 0: no consistent relationship.

`overlay.return_vol_rho(symbol)` returns `Option<f64>` (None until ≥ 2 observations).
Diagnostic only — never gates anything.

### `PerSymbolEwmaState` struct (new)

Rolling buffer of log-returns per symbol (capped at 10 000 bars ≈ 417 days at hourly
cadence). On each `on_bar` call, the last bar's close is pushed, the EWMA series is
recomputed via `vol_estimator::ewma_realized_vol`, and the last value is cached as
`sigma_hat`. The `PerSymbolGarchState` is kept for the GARCH path.

### Existing e2e — stays green (no defaults change)

`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` uses
`VolTargetingConfig::default()` (Garch source, no band, derisk_only=false). This is
unchanged from pre-P1-4 — the test asserts `quantity_scale ≈ 2.0` (clamp_max) after
5 GARCH bars with a low-unconditional-variance model, which still works exactly as
before. **No divergence assertion needed** because defaults did not change.

### New unit tests (33 added, 233 → 266 total)

| Category | Tests |
|---|---|
| No-trade band | suppresses_small_change, allows_large_change, zero_is_passthrough, derisk_interact_correctly |
| De-risk-only | blocks_upsize, allows_derisking, caps_at_one_not_current, derisk_interact_correctly |
| Return-vol correlation | positive_series → ρ≈1, negative_series → ρ≈−1, zero_corr_series, near_zero_for_uncorrelated, none_with_single_observation, n_obs_counter |
| EWMA vol source | computes_sigma_after_warmup, derisk_only_scale_never_exceeds_one |
| p1_4_defaults | sets_expected_fields |
| Pearson helper | empty/single → None, constant_x → None, identity → 1.0, negated → −1.0 |
| GARCH backward-compat | compute_scale_at_target_vol, clamp_max, clamp_min, forecast_step_positive, forecast_step_floored, init_state |

## Implementation

Files changed:
- `crates/strategy/src/vol_targeting_overlay.rs` — P1-4 reparameterisation.
- `crates/strategy/src/lib.rs` — `pub use drawdown_control_overlay` re-ordering (fmt).
- `crates/strategy/tests/vol_targeting_overlay.rs` — `..Default::default()` fix.
- `crates/strategy/tests/drawdown_control_overlay_end_to_end.rs` — doc comment fix
  (clippy `doc-lazy-continuation`; pre-existing from P1-3 developer).

Chosen defaults:
- **Lambda:** `LAMBDA_126D_HOURLY` ≈ 0.999771 (126-day half-life at hourly cadence;
  the slow, cost-survivable choice from the research).
- **Bar cadence:** Hourly (matching the inner `MomentumStrategy` configuration).
- **No-trade band:** 0.05 (5%) in `p1_4_defaults()` — backward-compat `Default` has 0.0.
- **De-risk-only:** `true` in `p1_4_defaults()` — backward-compat `Default` has `false`.

## Not in this increment

- UI mirror for `ReturnVolCorrelation` (display-only readout in the cockpit — a
  future ui-designer increment via the existing report mirror seam).
- Optional gated model vol forecast behind the `forecast` feature flag (CX-5 —
  strictly opt-in, out of scope for P1-4).
- ADR-0079 (reserved — written/registered atomically when P1-3 drawdown overlay
  lands, as the shared vol-estimator module serves both overlays).
