---
slug: advisor-cost-model-opt-in
status: shipped
owner: operator
version: 2.0.0
updated: 2026-07-01
---

# P1-6 Cost-Model Hardening + Venue-Trust Map

State-aware (vol-scaled) spread as a new `SlippageModel::VolScaledSpread` variant,
**opt-in-forever** per operator-ratified D6. The default `SlippageModel::Linear { bps: 8 }`
is byte-identical to today; anchors 119/119 unchanged.

**Design reference:** [`v2-architecture.md`](../v2-architecture.md) §1 P1-6 +
§2 R3 (blast-radius note) + §6.0 D6 (operator-ratified opt-in-forever).
Research:
[`research/backtesting/application-cost-and-impact-modeling.md`](../../../research/backtesting/application-cost-and-impact-modeling.md)
§6 E/B (vol-scaled spread recipe);
[`research/crypto-market-structure/application-data-integrity.md`](../../../research/crypto-market-structure/application-data-integrity.md)
§6 A/B (venue trust map source).

## What was built

### 1. `SlippageModel::VolScaledSpread` variant (ADR-0081)

New variant in `crates/cost/src/slippage.rs`. Formula:

```text
effective_bps = base_bps + vol_multiplier · σ̂_ewma(bar_returns) · 10_000
```

where `σ̂_ewma` is an EWMA of the trailing `sigma_window` log-returns.
`effective_bps` is capped at `MAX_SLIPPAGE_BPS = 1_000` (10%).

**Chosen defaults (operator-ratified ADR-0081 § D2):**

| Parameter | Default | Rationale |
|-----------|---------|-----------|
| `base_bps` | `8` | Matches the Linear default (ADR-0045 D1 pin) |
| `vol_multiplier` | `2.0` | Spreads widen 2–3× in high-vol regimes `backtesting[47]`; midpoint = 2.0 |
| `sigma_window` | `20` | ~1 trading month of hourly bars; responsive without overreacting |
| `sigma_lambda` | `0.94` | RiskMetrics λ (≈11.3-day half-life); faster than the 126-day sizing default to respond to short-term liquidity stress |

**σ̂ source (dep-cycle avoidance — ADR-0081 § D1):** The `cost` crate does NOT
depend on `crates/strategy` (that would create a cycle: `strategy` dev-depends
on `cost`). The identical 5-line EWMA recurrence is inlined in
`apply_slippage_vol_scaled_bps`. The formula matches `strategy::vol_estimator::ewma_realized_vol`.

**`DEFAULT_VOL_SCALED_SPREAD` const** is exported for callers wanting the canonical
opt-in setup without specifying every field.

### 2. Venue-trust map (display-only)

`spec/dev-notes/venue-trust-map-2026-07-01.md` — codifies which crypto venues
to trust for which metrics, grounded in
`application-data-integrity.md §6 A/B`:

- **Price:** deep major-venue USD/USDT (Binance, Coinbase, Kraken);
- **OI:** Kraken/HTX only (Bybit/OKX/Binance-inverse fabricate OI);
- **Volume:** distrust unregulated CEXs (>70% wash-traded);
- **Depth/book:** assume ~31% spoofable on all venues including Coinbase.

### 3. Fee-sensitivity sweep helper

`fee_sensitivity_report(base_bps, sigma_hat, vol_scale_factors)` in `crates/cost/src/slippage.rs`
returns `Vec<(f64, f64)>` (multiplier → effective spread in bps). Report-only;
no gate band change. Enables the "spec-curve for costs" review from `backtesting[24][47]`.

## D6 opt-in-forever contract (LOAD-BEARING)

`SlippageModel::default() = Linear { bps: 8 }` — NEVER CHANGES.
`VolScaledSpread` is opt-in ONLY; the anchored CLI backtest paths
all run with `write_report=true` on `Linear { bps: 8 }` → anchors 119/119
by construction.

ADR-0081 records this decision.

## Anchor safety

Verified before AND after: `bash scripts/verify_anchors.sh` → **119/119**.
`VolScaledSpread` is unreachable from any anchored CLI path by construction.
