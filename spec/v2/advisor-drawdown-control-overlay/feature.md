---
slug: advisor-drawdown-control-overlay
status: tester-done
owner: tester
updated: 2026-06-30
version: v2.0.0-phase-2c
---

# Feature: Drawdown-Control Overlay (P1-3)

Single-line: "Static CPPI drawdown controller that de-risks exposure proportionally
as equity approaches 80% of the starting €200, with a load-bearing HWM restart."

**Operator promise (D8, 2026-06-30):** Never lose more than 20% of the starting €200.

## Context

The research synthesis (`research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md` §6 P1-B)
identifies drawdown control with HWM restart as the most deployable overlay:

> On BTC Jan-2020 – Sep-2022 with 0.1% per-trade costs, drawdown modulation **with HWM
> restart** cut max-DD **72%→20%** AND held **Sharpe 1.521**, while the **same controller
> without restart** collapsed to **Sharpe −0.043** (lock-out-then-churn bleeds).

Three independent derivations converge on the same cushion-multiplier family (research [13], [31], [12]).

## D8: Operator decision (2026-06-30)

**Static CPPI @ 20% drawdown floor.**

- Floor = `initial_equity × 0.80`. Floor NEVER moves (static, not ratcheting/TIPP).
- TIPP / ratcheting floor deferred to v0.2.
- HWM restart is LOAD-BEARING (see § Design).

## Design

### Cushion-multiplier formula (normalised)

The architecture spec formula `M(k) = (d_max − d(k)) / (1 − d(k))` is normalised
by dividing by `d_max` so the boundary condition `M(0) = 1` holds (full exposure at ATH):

```text
d(k)  = 1 − equity_k / hwm_k              (current drawdown from HWM)
M(k)  = (d_max − d(k)) / (d_max × (1 − d(k)))   (normalised cushion multiplier)
```

Boundary conditions:
- `d(k) = 0`    → M = 1.0  (at HWM, full exposure)
- `d(k) = d_max`→ M = 0    (at floor, exposure shut)
- `d(k) > d_max`→ M = 0    (clamped)
- `d(k) < 0`   → M = 1.0  (clamped; equity above HWM, guarded upstream)

Default: `d_max = 0.20` (D8).

### HWM restart (LOAD-BEARING — ADR-0080 D2)

When equity sets a new all-time high, the HWM is reset to the new high.

**Without restart:** the Hsieh (2022) BTC benchmark shows Sharpe collapsed to −0.043
because the controller locks out (M=0) during the drawdown, then when equity recovers
the HWM is still at the old peak — so re-entry is delayed and the recovery rally is missed.

**With restart:** when equity exceeds the old HWM, the HWM resets, M recovers to 1.0,
and the portfolio re-enters fully. The static floor (`initial × 0.80`) NEVER moves.

### Composition (budget-cap invariant)

The overlay implements `Strategy::quantity_scale` to return M(k) as an `f64`.
The sizing pipeline (`FixedFractionSizer`) queries this at order-construction time
and computes: `qty = M(k) × fraction × equity / price`, then clamps to
`min(qty, budget_cap / price)`. The overlay NEVER bypasses the budget cap —
the clamp is applied AFTER the multiplier.

Mirroring the `VolTargetingOverlay` pattern (ADR-0038 § D5).

### Telemetry

`DrawdownControlOverlay::telemetry()` returns `DrawdownTelemetry { cushion_multiplier,
drawdown_from_hwm, hwm, floor }` for operator visibility. All `Decimal`.

### Sibling vol estimator

`crates/strategy/src/vol_estimator.rs` (P1-5 / ADR-0079) is the shared σ̂ module
consumed by both overlays. The drawdown overlay does not consume it in this increment
(the CPPI formula does not require σ̂ — it only needs equity and HWM). If the two
overlays are combined in a future increment, vol_estimator is the natural σ̂ source.

## Implementation

Files added:
- `crates/strategy/src/drawdown_control_overlay.rs` — `DrawdownControlConfig`, `DrawdownControlOverlay<S>`, `compute_cushion_multiplier`, `DrawdownTelemetry`.
- `crates/strategy/tests/drawdown_control_overlay_end_to_end.rs` — 6 integration tests incl. the mandatory load-bearing divergence gate.

`lib.rs` changes:
- `pub mod drawdown_control_overlay;` registered.
- `pub use drawdown_control_overlay::{DrawdownControlConfig, DrawdownControlOverlay, DrawdownTelemetry, compute_cushion_multiplier};` exported.

### Test inventory

| Test | Location | What it proves |
|---|---|---|
| `overlay_equity_diverges_from_baseline_on_drawdown_scenario` | e2e | MANDATORY load-bearing gate (≥1bp divergence from baseline — catches no-op) |
| `hwm_restart_proof_benchmark_sequence` | e2e | With/without restart difference on BTC-style sequence |
| `floor_never_moves_static_cppi_d8` | e2e | D8 static floor invariant (not TIPP) |
| `budget_cap_invariant_quantity_scale_max_one` | e2e | `quantity_scale ∈ [0,1]` always |
| `quantity_scale_before_update_returns_default_one` | e2e | Default 1.0 before first `update_equity` |
| `multiplier_at_10pct_drawdown_is_correct` | e2e | Formula pin at d_k=0.10 |
| `multiplier_at_zero_drawdown_is_one` | unit | Boundary M(0)=1 |
| `multiplier_at_floor_drawdown_is_zero` | unit | Boundary M(d_max)=0 |
| `multiplier_beyond_floor_is_zero` | unit | Clamp at d_k > d_max |
| `multiplier_halfway_matches_normalised_formula` | unit | Formula mid-point |
| `multiplier_at_negative_drawdown_is_clamped_to_one` | unit | Guard d_k<0 |
| `floor_never_moves_even_when_hwm_doubles` | unit | D8 invariant |
| `hwm_restart_preserves_upside_in_second_drawdown` | unit | HWM restart logic |
| `quantity_scale_is_always_in_zero_to_one` | unit | Budget cap invariant |
| `update_equity_hwm_ratchets_on_new_high` | unit | HWM ratchet |
| `update_equity_no_restart_hwm_stays_fixed` | unit | No-restart mode |
| `bars_total_counter_increments` | unit | Telemetry counter |
| `telemetry_reflects_current_state` | unit | Telemetry accuracy |

### Determinism

- No `SystemTime` / `Instant` / `chrono` in any path.
- No `f64` in money/position arithmetic — HWM, floor, equity, M(k) all `Decimal`.
- `f64` only at the `quantity_scale` trait boundary (Decimal→f64, precision loss negligible
  since M ∈ [0,1]).
- No RNG.

### Anchor safety

The overlay runs on the advisor path (`write_report=false` in bake-off). It is anchor-safe
by construction — the overlay's output (scaled positions) never flows into any anchored
report body. 119/119 anchors unchanged.

## Honest operator promise

> "The drawdown-control overlay never lets the account fall below 80% of the starting
> budget. Every euro of upside exposure is proportional to the remaining cushion. When
> the cushion is gone, the overlay shuts new buying to zero. But every floor guarantee
> is probabilistic on a gapping asset — a crypto jump larger than the cushion between
> bars can still breach the floor. We disclose this."

ADR: `spec/architecture/adr/0080-drawdown-control-overlay.md`
