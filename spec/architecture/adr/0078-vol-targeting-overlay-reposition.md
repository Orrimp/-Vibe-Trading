---
adr: 0078
title: Vol-targeting overlay reposition — risk-shaping not Sharpe; slow EWMA, no-trade band, de-risk-only
status: accepted
date: 2026-06-30
supersedes: none
superseded-by: none
---

# ADR-0078: Vol-targeting overlay reposition (P1-4)

## Context

The existing `VolTargetingOverlay` (`crates/strategy/src/vol_targeting_overlay.rs`,
shipped earlier) reads its σ̂ from a GARCH source and applies a continuous
position-resize on every bar. Two research findings make that posture wrong on
crypto:

1. **Crypto's leverage effect is reversed** (research: γ ≈ **−0.261** on crypto vs
   **+0.115** on equity). The vol-targeting Sharpe gain that works on equities
   relies on the *negative* equity leverage effect (high vol ⇒ low future returns,
   so cutting size during vol spikes preserves return). On crypto the relationship
   is reversed (positive returns drive vol via FOMO), so the *same* mechanism
   cannot mechanistically yield a Sharpe gain — the overlay must be sold as a
   **risk-shaping tool, not a Sharpe tool**.

2. **Continuous re-sizing goes net-negative on turnover.** Without a no-trade
   band, the overlay churns on every bar — paying the cost-drag while delivering
   no risk-adjusted return improvement.

## Decision

Reparameterise the existing overlay (no new file, no new struct rename — keep
the shipped public type). Four additive changes:

1. **Switch σ̂ source to the slow EWMA from `crates/strategy/src/vol_estimator.rs`**
   (ADR-0079, the shared estimator). Default: `LAMBDA_126D_HOURLY ≈ 0.999771`
   (126-day half-life on hourly bars). The previous GARCH source remains
   available via a `VolSource` enum (backward-compatibility) so the existing e2e
   stays byte-identical on `Default`.

2. **No-trade band** (`no_trade_band: Decimal`). Only resize when
   `|new_target − current_size| / current_size > band`. Defaults: `0.0` on
   `Default` (backward-compat); `0.05` (5%) on a new `p1_4_defaults()` ctor.

3. **De-risk-only mode** (`derisk_only: bool`). When `true`, the overlay only
   ever *cuts* position size on a vol spike — never *upsizes* on a vol drop.
   Sells the overlay as risk-shaping, not return-chasing. Defaults: `false` on
   `Default`; `true` on `p1_4_defaults()`.

4. **Per-window return-vol correlation telemetry** (`ReturnVolCorrelation`).
   Computes Pearson ρ between returns and σ̂ over the window — the operator-
   readable answer to "is the Sharpe mechanism even mechanistically present on
   this coin?" Decimal-safe; surfaced on the overlay's state.

### Backward compatibility

The existing `vol_targeting_overlay_end_to_end.rs` integration test stays green
**byte-identical** on `Default` (GARCH source, `no_trade_band = 0.0`,
`derisk_only = false`). Anyone wanting the P1-4 honest defaults uses the new
`p1_4_defaults()` ctor explicitly.

### Anchor safety

The overlay runs on the advisor path with `write_report = false` → anchor-safe
by construction. `verify_anchors.sh` stays 119/119 before and after.

### Frozen-gate identity

The overlay is sizing, not crowning. `rank_candidates` and the FROZEN
`verdict_bands` / `classify_verdict` are byte-untouched. The standing
gate-identity tests (`scorecard_does_not_change_ranking`,
`turnover_does_not_change_ranking`) stay green.

## Status: accepted

- Dev-done at commit (this batch); 266 strategy lib tests pass (33 new
  vol-overlay reposition tests), `vol_targeting_overlay_end_to_end.rs` stays
  green on `Default`, clippy `-D warnings` clean, fmt clean, anchors 119/119,
  spec-lint PASS.
- The companion drawdown-control overlay (ADR-0080) ships in the same batch
  and consumes the same `vol_estimator` module.

## Consequences

- **The vol overlay's promise is now honest** — drawdown / tail / vol-of-vol
  reduction (universal), **never** a Sharpe gain on crypto.
- **Cost-drag bounded** by the no-trade band — the operator chooses how often
  the overlay is allowed to re-size.
- **The Sharpe-mechanism telemetry** (return-vol ρ) gives the operator an
  explicit, per-coin, per-window answer to whether vol-targeting can even
  mechanistically help — eliminates a category of false hope.
- **Backward compatibility preserved** — `Default` matches pre-P1-4 behaviour
  byte-for-byte; `p1_4_defaults()` is the honest opt-in.

## References

- Spec: `spec/v2/advisor-vol-overlay-reposition/feature.md`.
- Architect: `spec/v2/v2-architecture.md` §1 P1-4.
- Research: `research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md` §6 P1-A.
- Consumes: ADR-0079 (`vol_estimator`).
- Composes with: ADR-0080 (drawdown-control overlay, sibling Phase 2C work).
