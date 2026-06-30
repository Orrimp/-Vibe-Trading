---
adr: 0080
title: Drawdown-control overlay with static CPPI and load-bearing HWM restart
status: accepted
date: 2026-06-30
supersedes: none
superseded-by: none
---

# ADR-0080: Drawdown-control overlay with static CPPI and load-bearing HWM restart

## Context

The v2 Phase 2C roadmap (P1-3) adds a drawdown-control overlay to the advisor's
strategy composition surface.  Three prior design inputs converge here:

1. **Research synthesis** (`research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md`
   §6 P1-B, Hsieh 2022 [96]): on BTC Jan-2020 – Sep-2022 with 0.1% per-trade costs,
   a CPPI-style cushion modulator **with HWM restart** cut max-DD 72%→20% and held
   Sharpe 1.521; the **same controller without restart** collapsed to Sharpe −0.043
   (lock-out-then-churn bleeds the cushion).  The restart is load-bearing.

2. **Operator decision D8 (2026-06-30):** Static CPPI at 20% drawdown floor.
   Floor = `initial_equity × 0.80`.  Floor NEVER moves (static, not TIPP/ratcheting).
   TIPP deferred to v0.2.

3. **CLAUDE.md non-negotiable:** Every sizing modifier ships a day-1 baseline-equity-
   divergence end-to-end test proving the overlay is NOT a silent no-op.  The
   v3-vol-overlay-noop precedent (2026-05-22) is the binding incident.

## Decision

**D1 — Module home:** `crates/strategy/src/drawdown_control_overlay.rs`, mirroring the
`VolTargetingOverlay` pattern (`vol_targeting_overlay.rs`).  Exported from `crates/strategy/src/lib.rs`.
Shares the `vol_estimator.rs` (P1-5) sibling module; the CPPI formula does not require σ̂
in this increment, but the two overlays can compose with `vol_estimator` in a future increment.

**D2 — Cushion-multiplier formula (normalised):**

```text
d(k)  = 1 − equity_k / hwm_k                      (drawdown from HWM, in [0,1])
M(k)  = (d_max − d(k)) / (d_max × (1 − d(k)))     (normalised cushion multiplier)
```

The architecture spec's base form `M(k) = (d_max − d(k)) / (1 − d(k))` is normalised
by dividing by `d_max` so the operator contract `M(0) = 1.0` (full exposure at ATH)
holds.  Boundary conditions:

- `d(k) = 0`     → M = 1.0 (full exposure — equity at HWM).
- `d(k) = d_max` → M = 0   (shut — floor reached).
- `d(k) > d_max` → M = 0   (clamped).
- `d(k) < 0`     → M = 1.0 (clamped; equity above HWM — guarded upstream).

Default `d_max = 0.20` per D8.  Implemented in `compute_cushion_multiplier(d_max, d_k)`.

**D3 — HWM restart (LOAD-BEARING):** `DrawdownControlConfig.restart_on_hwm = true`
(default).  When equity sets a new all-time high, `hwm` is reset to the new high.
Setting `restart_on_hwm = false` reproduces the Sharpe −0.04 failure mode and is
available only for testing/comparison; it is NOT the recommended configuration.

**D4 — Static floor (D8):** `floor = initial_equity × (1 − drawdown_floor_pct)`.
The floor is computed once at construction and NEVER changes thereafter.  Moving the
floor with the HWM (TIPP/ratcheting) is deferred to v0.2.

**D5 — Composition via `Strategy::quantity_scale`:** The overlay returns M(k) from
`quantity_scale(&Symbol)` (same interface as `VolTargetingOverlay`, ADR-0038 § D5).
The `FixedFractionSizer::budget_cap` is applied AFTER the multiplier by the sizing
pipeline — the overlay NEVER bypasses the budget cap.  The multiplier is account-level
(same value for all symbols) and is cached in `cached_multiplier: Decimal` after each
`update_equity` call.

**D6 — `update_equity` contract:** In a production loop the caller must call
`DrawdownControlOverlay::update_equity(equity_k)` once per bar with the current account
equity BEFORE calling `on_bar`.  In the bake-off/backtest the engine drives this.

**D7 — Day-1 divergence gate (CLAUDE.md mandatory):**
`crates/strategy/tests/drawdown_control_overlay_end_to_end.rs` contains 6 integration
tests.  The load-bearing test `overlay_equity_diverges_from_baseline_on_drawdown_scenario`
proves the overlay's cumulative exposure diverges from the un-overlaid baseline by ≥ 1 bp
on a scenario with a 20% drawdown.  Red-on-revert: a no-op `quantity_scale` returning 1.0
makes this test FAIL.

**D8 — Anchor safety:** The overlay runs on the advisor bake-off path
(`write_report=false`).  No anchored report body is written → 119/119 anchors unchanged
by construction.

**D9 — Generic inner strategy:** `DrawdownControlOverlay<S: Strategy>` is generic over
the inner strategy, enabling composition with any future strategy without forking the
overlay.  This matches the `VolKillSwitchOverlay` pattern.

## Alternatives considered

- **Linear cushion ratio `M = (equity − floor) / (hwm − floor)`** — equivalent at
  d_k=0 and d_k=d_max; the normalised architecture formula is mildly nonlinear (5/9 vs 0.5
  at the midpoint) but captures the same risk-shaping intent.  The architecture formula
  is chosen for research fidelity [13].
- **TIPP / ratcheting floor** — deferred to v0.2; the static floor is simpler to reason
  about and sufficient for the D8 "never lose more than 20% of the starting €200" promise.
- **Separate `v0.dd_control` bake-off arm** — deferred; the overlay is currently a
  composable `Strategy<S>` wrapper, not a standalone bake-off arm.  Wiring into the
  bake-off as an arm goes through the R1 forward-coverage seam (ADR-0077).

## Consequences

- **Enforced by:** `crates/strategy/tests/drawdown_control_overlay_end_to_end.rs`
  (divergence gate); `tests/drawdown_control_overlay::tests::floor_never_moves_even_when_hwm_doubles`
  (D4 static floor); `tests::quantity_scale_is_always_in_zero_to_one` (D5 budget cap).
- **Budget cap non-negotiable:** if any future code path calls `quantity_scale` and
  multiplies the result by a factor > 1 before the `FixedFractionSizer` clamp, that is a
  violation of D5.  The test `budget_cap_invariant_quantity_scale_max_one` is the gate.
- **`restart_on_hwm = false` is test-only:** the Hsieh benchmark proves the no-restart
  variant has Sharpe −0.04 on BTC.  The default must remain `true`.
- **HWM starts at `initial_equity`:** a caller must not pass an initial_equity of 0 —
  the constructor would produce a zero floor and infinite de-risking.

## Changelog
- 2026-06-30 (developer): initial accept.  Feature `advisor-drawdown-control-overlay`
  (Phase 2C P1-3).  REQ-V2-P1-3-DRAWDOWN-OVERLAY-001 in trace.toml.
