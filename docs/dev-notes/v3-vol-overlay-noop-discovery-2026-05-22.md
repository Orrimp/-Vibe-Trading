---
slug: v3-vol-overlay-noop-discovery-2026-05-22
status: living
owner: analyst
updated: 2026-05-22
---

# v3 vol-targeting overlay — no-op wiring discovery

> **2026-05-22, post-rebaseline-ship.** The orchestrator's caveman
> probe (manual perturbation injection into `vol_targeting_overlay.rs`)
> revealed that the GARCH vol-targeting overlay is a **no-op** —
> `compute_scale` returns the correct factor, but nothing downstream
> reads it. Both `v3-volatility-forecaster v0.1.0` and
> `v3-volatility-forecaster-rebaseline v0.1.0` are wiring-failure ships,
> not real alpha tests. Captured here so future auditors do not have to
> retrace the diagnostic chain.

## Timeline

| Time (UTC) | Event                                                                                                                                                                                                                                                  |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 2026-05-22 morning | `v3-volatility-forecaster v0.1.0` ships with joint advisory **V3 × T-VOL-NO-ALPHA → MODEL-BROKEN / NO-ALPHA**. 3 anchors under `[v3.0.0-volatility]`. Synthetic-vs-real data caveat noted at § Verification.                                  |
| 2026-05-22 afternoon | `v3-volatility-forecaster-rebaseline v0.1.0` ships with **T-VOL-NO-ALPHA confirmed on real-vs-real evidence** (`net_delta = 0.000000`). 1 anchor under `[v3.0.0-volatility-rebaseline]`. Operator routes (a) RETIRE C1.                                |
| 2026-05-22 ~11:30Z | Operator (orchestrator-mediated) flags the `0.000000` net-delta as suspiciously exact — two genuinely different runs on real data should not produce **byte-identical** Sharpe values. Caveman probe launched.                                          |
| 2026-05-22 ~11:44Z | Caveman probe completes 30-minute foreground run. Three observations land. Code review identifies bug site at `vol_targeting_overlay.rs:309-319`.                                                                                                       |
| 2026-05-22 ~13:00Z | This dev-note + the `v3-volatility-forecaster-noop-fix` feature brief + amendment blocks on parent + rebaseline + REQ row + backlog Active block all land in one analyst pass.                                                                          |

## The caveman probe

The orchestrator hand-patched `crates/strategy/src/vol_targeting_overlay.rs`
to multiply `sigma_hat` by `2.95` inside `on_bar` — the parent's
`mean_calibration_ratio = 2.952191` finding suggested a bias-correction
of that magnitude was the natural perturbation. The intent: if the
overlay is real, a 2.95× perturbation of its input MUST move equity.
If the overlay is a no-op, equity stays byte-identical.

The patched code ran the full `top10-2023-fy-vol-target-overlay-realdata`
backtest (~40s on M-series). Result:

```
EQUITY (caveman-patched, sigma_hat × 2.95):   $113,479.98
RETURN:                                       13.48%
MAX DRAWDOWN:                                 73.73%
TRADES:                                       6203

EQUITY (parent anchor, 66cd69ad…):            $113,479.98
RETURN:                                       13.48%
MAX DRAWDOWN:                                 73.73%
TRADES:                                       6203

EQUITY (rebaseline un-targeted baseline,
        top10-2023-fy-momentum-realdata):     $113,479.98
RETURN:                                       13.48%
MAX DRAWDOWN:                                 73.73%
TRADES:                                       6203
```

All three runs **byte-identical**. The 2.95× perturbation had zero
effect. The vol-target overlay's output equals the un-targeted
baseline's output equals the perturbed-overlay output. This is the
unambiguous signature of a no-op: the overlay's `compute_scale`
output flows nowhere that affects equity.

## Code review — the smoking gun

`crates/strategy/src/vol_targeting_overlay.rs`, lines 305-319 (verbatim):

```rust
        // Compute scale factor.
        let scale = self.compute_scale(sigma_hat);

        // Apply scale to signals.
        let tol = 1e-6;
        if (scale - 1.0).abs() < tol {
            self.stats.signals_passthrough += base_signals.len() as u64;
            base_signals
        } else {
            self.stats.signals_scaled += base_signals.len() as u64;
            // Return the signals with the scale embedded in the strategy_id
            // (diagnostic only — the backtest engine reads quantities from fills,
            // not from signal metadata).
            base_signals
        }
```

The `else` branch:

1. Increments `signals_scaled` counter (a stats proxy, no consequence).
2. Returns `base_signals` **unmodified** — no quantity scaling applied.
3. Carries an inline comment admitting the design intent (embed scale
   in `strategy_id` metadata) was abandoned **without replacement**.
4. The comment correctly identifies the architectural constraint
   ("backtest engine reads quantities from fills, not from signal
   metadata") but does not implement the wire that satisfies it.

`scale` is declared on line 306 and **never read after the `if`
check**. The variable is consumed only to decide which stats counter
to increment.

## Why the gates didn't catch it

Five layers of gating, each blind to the load-bearing property:

| Layer                                | What it tests                                                                       | What it missed                                                                                                                                                                                                                                                |
| ------------------------------------ | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo test` (vol_targeting_overlay) | `compute_scale` clamp invariants: `compute_scale(target_vol) == 1.0`, `compute_scale(tiny_sigma) == clamp_max`, `compute_scale(huge_sigma) == clamp_min`. All 8 unit tests verify the math. | No assertion that `scale ≠ 1.0` results in a different `Signal` (or fill quantity) than `scale = 1.0`. The math is tested in isolation; the application is not tested at all.                                                                                  |
| `clippy / fmt`                       | Code style; dead-code lints fire on unused variables.                                | `scale` IS read (by the `if (scale - 1.0).abs() < tol` check), so the lint passes. The lint does not check that the read has consequences.                                                                                                                |
| Anchor gate (33 → 34 PASS)           | 2-run byte-identity of every report body.                                            | The byte-identity is **exactly the signature of a no-op overlay**. Overlay output == baseline output is the bug's footprint; the anchor gate witnesses but cannot interpret it.                                                                              |
| Architect M-T1 (T-AR-4)              | Scale-clamp invariants; ADR-0038 § D5 strategy-side composition contract.            | The contract specifies the **shape** (overlay wraps inner strategy, composition is strategy-side) but not the **application semantic** (overlay actually changes fill quantities). § D5 is wire-incomplete: it locks composition but not propagation.       |
| Tester M-FINAL (parent + rebaseline) | cargo gates + anchor gate + byte-identity + verdict-cell recording.                  | The 4-cell route table records the verdict cell honestly. The bug is not at the verdict layer; the bug is upstream, in the engine surface the verdict is computed from. Tester is the wrong layer to catch this.                                          |

The 4th and 5th rows are the ones that should have caught it. Both
were checking adjacent properties.

## Why the parent feature shipped anyway

Two contributing factors:

1. **The synthetic-vs-real data caveat hid the signature.** The parent
   feature's net_delta = `0.029868` (synthetic baseline = -0.026770,
   real overlay = 0.003098) was non-zero. Net_delta = 0 in the
   rebaseline pass (`real baseline = 0.003098, real overlay =
   0.003098`) is the exact byte-identical signature, but it was only
   reachable once both sides ran on real data. The rebaseline pass
   surfaced the byte-identity within a few hours of the parent ship.
2. **The V3 verdict dominated the narrative.** The model-broken /
   no-alpha joint advisory framed the result as a GARCH-calibration
   issue ("3/10 symbols non-convergent at 500 MLE iters"). That
   framing is correct on its own merits but was load-bearing for
   accepting the T-VOL-NO-ALPHA reading as plausible — when the
   model is broken, near-zero alpha is the expected outcome. The
   wiring bug was downstream of GARCH and orthogonal to it; nobody
   went looking.

The cascade: synthetic-vs-real caveat → rebaseline pass → exact
byte-identity → caveman probe → smoking gun. ~8 hours end-to-end
from parent ship to discovery.

## What a meaningful test would have looked like

The missing test is **`vol_targeting_overlay::overlay_changes_equity_vs_untargeted_baseline`**
(R2 in the fix feature). Shape:

```rust
#[test]
fn overlay_changes_equity_vs_untargeted_baseline() {
    // Rigged GARCH state: sigma_hat for one symbol biased to drive
    // compute_scale to ~1.7 for N bars.
    let scenario = …;
    let (equity_overlay, trades_overlay) = run_with_vol_target_overlay(&scenario);
    let (equity_baseline, trades_baseline) = run_without_overlay(&scenario);

    let equity_diff_bps = ((equity_overlay - equity_baseline) / equity_baseline).abs() * 10_000.0;
    assert!(
        equity_diff_bps >= 1.0,
        "vol-target overlay produced byte-identical equity to baseline — no-op suspected (Δ={equity_diff_bps} bp)"
    );
    // Sanity: signal timing should be ≈ identical; only sizing differs.
    let trade_count_ratio = trades_overlay as f64 / trades_baseline.max(1) as f64;
    assert!(
        (0.95..=1.05).contains(&trade_count_ratio),
        "trade-count diverged by >5% — signal-timing perturbation suspected (overlay should only scale sizing)"
    );
}
```

This test fails under the pre-fix code (no-op produces
`equity_diff_bps = 0` byte-identically). It passes under the fix
when `compute_scale → 1.7` flows into fill quantity. Both halves
are load-bearing: the first ensures the overlay is doing
something; the second ensures it is doing the right thing
(sizing only, not signal-timing).

The fix feature R6 also adds two narrower guards (unit on the
strategy alone; integration at the engine boundary on fill
quantities directly) so that a future regression at any layer
trips at least one gate before reaching M-FINAL.

## Status of the dependent ships

Both parent and sibling are provisionally invalidated; the
amendment blocks land in this same pass. The (a) RETIRE-C1
routing decision (in the rebaseline pass's presenter deck) is
**invalidated and on hold** until the fix lands and the re-run
produces a real verdict. The V3 calibration finding survives
the fix verbatim (GARCH-only diagnostic, measured before the
overlay applies the scale).

## Lessons (for cross-feature memory)

1. **Test that the overlay changes the output.** Every Strategy-
   composition feature in the v3+ lane needs an end-to-end gate that
   asserts the overlay actually changes the load-bearing observable
   (equity, fill quantity, etc.) vs the un-overlaid baseline. The
   anchor gate alone cannot detect a no-op; it tests determinism,
   not behavior.
2. **The anchor contract's spirit is "don't silently mutate
   historical evidence."** Byte-identity is the property; "the
   recorded body reflects a faithful execution of the named
   contract" is the spirit. When the contract changes (no-op fixed),
   re-emission is legitimate **under a documented protocol** — see
   ADR-0038 § D6 amendment (R5 in the fix feature).
3. **A "diagnostic only" comment in load-bearing code is a code
   smell.** The inline comment at the bug site explicitly admitted
   the scale was diagnostic-only; the abandonment was visible in
   the source but not flagged in any review.
4. **The caveman probe (manual perturbation) is a cheap, high-
   signal forensic tool.** When a result is suspicious, hand-patch
   the most-load-bearing input and re-run; if the output doesn't
   move, the wire is broken. Cost: ~30 minutes wall-clock.

## Cross-references

- Bug site: [`crates/strategy/src/vol_targeting_overlay.rs:305-319`](../../crates/strategy/src/vol_targeting_overlay.rs).
- Existing unit tests (math-only, miss the wire): [`crates/strategy/tests/vol_targeting_overlay.rs`](../../crates/strategy/tests/vol_targeting_overlay.rs).
- TCN overlay (structurally different — kind-replacement, not quantity-scale): [`crates/strategy/src/tcn_overlay_momentum.rs`](../../crates/strategy/src/tcn_overlay_momentum.rs).
- Parent feature (invalidated): [`spec/v3-volatility-forecaster/feature.md`](../../spec/v1/v3-volatility-forecaster/feature.md).
- Sibling rebaseline (invalidated): [`spec/v3-volatility-forecaster-rebaseline/feature.md`](../../spec/v1/v3-volatility-forecaster-rebaseline/feature.md).
- Fix feature: [`spec/v3-volatility-forecaster-noop-fix/feature.md`](../../spec/v1/v3-volatility-forecaster-noop-fix/feature.md).
- ADR-0038 § D5 (strategy-side composition lock) + § D6 (anchor-additive contract): [`_bmad-output/planning-artifacts/architecture/decisions/0038-vol-forecast-verdict-shape.md`](../../_bmad-output/planning-artifacts/architecture/decisions/0038-vol-forecast-verdict-shape.md).

## Changelog

- 2026-05-22 (analyst): dev-note authored alongside the
  v3-volatility-forecaster-noop-fix v0.1.0 feature brief. Captures
  the 8-hour timeline from rebaseline ship → byte-identity surfacing
  → caveman probe → smoking gun → discovery; documents the five
  gate layers that missed it; sketches the meaningful end-to-end
  test (R2 in the fix feature); records the (a) RETIRE-C1 routing
  invalidation.
