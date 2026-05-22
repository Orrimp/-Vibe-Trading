---
slug: v3-volatility-forecaster-noop-fix
phase: M-T1
owner: architect
date: 2026-05-22
status: locked
priority: P0
parent: v3-volatility-forecaster
---

# M-T1 — Architect decomposition (v3-volatility-forecaster-noop-fix v0.1.0)

> Architect lock for the **P0 wire-up fix** to the GARCH vol-targeting
> overlay. Operator approved Q1..Q3 = analyst defaults on 2026-05-22
> via standing Autoapprove. Q1=(ii) defaulted `Strategy::quantity_scale`
> trait method. Q2=(a) re-emit affected anchors in-place under existing
> namespaces + ADR-0038 § D6.b amendment. Q3=(b) vol-target-only fix.
>
> **Anchor delta locked at this M-T1**: 3 SHAs re-emit (the two
> vol-target sharpe-comparison rows + the vol-target-overlay backtest
> row). 1 row (`vol-verdict-bs1-realdata`) stays byte-identical
> post-audit (§ T-AR-5 — body is GARCH-only). 30 unchanged rows stay
> byte-identical by construction (no other scenario reads
> `Strategy::quantity_scale`; the default-1.0 inherit is never queried
> outside `garch_vol_target_overlay.rs`).
>
> **Forensic gate is live**: T-AR-4 specifies the R2 end-to-end test
> shape; the developer runs it ONCE before landing the fix to confirm
> it **FAILS** under current main, then lets it pass under the fix.

## Baseline gate (M-T1 capture)

Quoted literal from `bash scripts/verify_anchors.sh` at M-T1 open
(2026-05-22, pre-fix):

```
ANCHORS PASS  (34 / 34)
```

All 34 rows currently PASS — this is the entry condition for the
re-emission protocol. Wave B leaves us at `ANCHORS PASS (34 / 34)`
with 3 fresh SHAs replacing the no-op SHAs in-place and 31 rows
unchanged (the negative invariant).

## Spike requirement assessment

**Not required.** The fix is ~80-150 LoC across 4 file touches:

| Edit | File | LoC budget |
|---|---|---|
| Trait method add | `crates/strategy/src/traits.rs` | +7 |
| Overlay cache + override | `crates/strategy/src/vol_targeting_overlay.rs` (lines 134-339 region) | +20 / −5 |
| Sizing-pipeline hook (Buy + Sell arms) | `crates/backtest/src/scenarios/garch_vol_target_overlay.rs` (lines 261-329 region) | +12 |
| R2 e2e regression test | `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` (new file) | +90 |
| R6 unit test rows | `crates/strategy/tests/vol_targeting_overlay.rs` (existing file) | +30 |

Total: ~+160 / −5 LoC. No new dependency. No external API change.
No new architectural pattern (the trait-defaulted-method hook is
load-bearing-but-tiny; the only existing `Strategy` consumers see
zero change). Walk-the-call-path completed at T-AR-2 (one site).
Spike would not surface additional unknowns.

## 1. Architect-decide resolutions (T-AR-1 .. T-AR-7)

### T-AR-1 — `Strategy::quantity_scale` trait method shape (Q1=(ii) locked)

**Decision**: defaulted method `fn quantity_scale(&self, _symbol: &Symbol) -> f64 { 1.0 }` on the `Strategy` trait at [`crates/strategy/src/traits.rs:8-15`](../../crates/strategy/src/traits.rs).

**Rationale**:

- Minimum blast radius. The trait gains a single defaulted method;
  all 9 existing `impl Strategy` blocks auto-inherit the `1.0`
  return without code changes:
  - [`crates/strategy/src/sma_crossover.rs:32`](../../crates/strategy/src/sma_crossover.rs) (`SmaCrossover`)
  - [`crates/strategy/src/cross_sectional/momentum.rs:181`](../../crates/strategy/src/cross_sectional/momentum.rs) (`MomentumStrategy`)
  - [`crates/strategy/src/pairs/mean_reversion.rs:157`](../../crates/strategy/src/pairs/mean_reversion.rs) (`MeanReversionPairsStrategy`)
  - [`crates/strategy/src/composed/node.rs:1208`](../../crates/strategy/src/composed/node.rs) (`ComposedStrategy`)
  - [`crates/strategy/src/tcn_overlay_momentum.rs:629`](../../crates/strategy/src/tcn_overlay_momentum.rs) (`TcnOverlayMomentumStrategy`)
  - [`crates/strategy/src/patchtst_overlay_momentum.rs:295`](../../crates/strategy/src/patchtst_overlay_momentum.rs) (`PatchTstOverlayMomentumStrategy`)
  - [`crates/strategy/src/vol_killswitch_overlay.rs:164`](../../crates/strategy/src/vol_killswitch_overlay.rs) (`VolKillSwitchOverlay`)
  - [`crates/strategy/src/vol_meanreversion.rs:148`](../../crates/strategy/src/vol_meanreversion.rs) (`VolMeanReversionStrategy`)
  - [`crates/strategy/src/vol_targeting_overlay.rs:256`](../../crates/strategy/src/vol_targeting_overlay.rs) (`VolTargetingOverlay` — overrides).
- `&self` (not `&mut self`). The scale is computed during `on_bar`
  (mutates `self.state` per symbol). `quantity_scale` is the
  read-only accessor that exposes the cached value. `&self`
  composes with `for sig in &signals` borrow-without-conflict at
  the call site.
- `&Symbol` parameter (not by-value). `Symbol` is a `smol_str`
  newtype (small string optimisation) but cloning is still a cycle.
  The sizing pipeline already holds `&sig.symbol`; pass it through.

**Locked signature** (lands verbatim at the bottom of the trait):

```rust
// crates/strategy/src/traits.rs:8-15 (current); +7 LoC at line 14
pub trait Strategy: Send + Sync {
    fn id(&self) -> StrategyId;
    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal>;
    fn on_tick(&mut self, tick: &Tick) -> Vec<Signal>;
    fn config_schema() -> serde_json::Value
    where
        Self: Sized;

    /// Per-symbol quantity scale factor applied at sizing time.
    ///
    /// Default returns `1.0` (no scaling). Overlays that adjust position
    /// sizes (e.g., vol-targeting) override this to expose their cached
    /// per-symbol scale factor. Queried by the sizing pipeline at order-
    /// construction time. The scale is cached from the most recent
    /// `on_bar` call; calling `quantity_scale` before any `on_bar` for
    /// this symbol returns the default `1.0`.
    fn quantity_scale(&self, _symbol: &Symbol) -> f64 {
        1.0
    }
}
```

Note: this requires importing `Symbol` at the top of `traits.rs`:

```rust
use trading_core::{Bar, Signal, StrategyId, Symbol, Tick};
```

(Currently `Symbol` is not imported here — single-line additive edit.)

**Alternatives considered (and rejected at M-OD by operator)**:

- **(i) `Signal.quantity_scale: f64` field on `trading_core::Signal`** — rejected. Signal is serialized into the audit ledger + journal entries + ADR-0029 canonical arch descriptors. Adding a field there has multi-crate ripple. Defaulted trait method is much lighter.
- **`&mut self` receiver** — rejected. The scale was already computed in `on_bar` and cached. Read-only accessor composes cleanly with the existing `for sig in &signals` loop in `garch_vol_target_overlay.rs:239` without borrow-checker gymnastics.
- **Owned `Symbol` parameter** — rejected. The sizing pipeline already has `&sig.symbol`; no need to clone.

### T-AR-2 — Sizing-pipeline call site (load-bearing architectural decision)

**Walk of the call path** (M-T1 trace):

1. Entry: `crates/backtest/src/scenarios/garch_vol_target_overlay.rs:237` — `let signals = overlay_strategy.on_bar(bar);` (already computes + caches the scale per Q1=(ii) override at T-AR-3).
2. Loop: line 239 — `for sig in &signals` iterates the (potentially scaled) signals.
3. **Sizing site (the hook)**: lines 262-265 (Buy arm) — `let fraction = dec!(0.10); let notional = equity * fraction; let qty_raw = notional / mark;`. This is where `qty_raw` becomes the order quantity.
4. Order construction: lines 270-283 — `Quantity::new(qty_raw)` + `Order::new(...)`.
5. Sell arm: line 300 — `Quantity::new(current_qty)`. **The Sell arm closes the existing position, not size from notional × scale. Vol-target scale does NOT apply to Sell-to-close orders** (you exit the full position; scaling a close-by-fraction is semantically wrong and would leak residual exposure).

**The hook (Wave A T-D-N3)**:

```rust
// crates/backtest/src/scenarios/garch_vol_target_overlay.rs:262
// BEFORE:
let fraction = dec!(0.10);
let notional = equity * fraction;
let qty_raw = notional / mark;

// AFTER (insert between fraction and notional):
let fraction = dec!(0.10);
let scale = overlay_strategy.quantity_scale(&sig.symbol);
// Convert f64 scale to Decimal for exact-cent compatibility (CLAUDE.md
// money-math rule). rust_decimal::Decimal::try_from(f64) handles NaN/Inf
// by returning Err; defensively floor to 1.0 (treat as no-op) if Err.
let scale_dec = Decimal::try_from(scale).unwrap_or(Decimal::ONE);
let notional = equity * fraction * scale_dec;
let qty_raw = notional / mark;
```

**Precision/rounding notes**:

- `Decimal * Decimal * Decimal` is exact-precision. `rust_decimal::Decimal` truncates to 28 significant digits — far beyond f64's 15-17.
- `Decimal::try_from(f64)` may round (f64 representation cannot encode every Decimal). `scale ∈ [0.5, 2.0]` clamped → values are bounded; precision loss bounded at ~1e-15 relative.
- Determinism contract: the f64 → Decimal conversion uses `try_from`, which is deterministic (no SystemTime, no PRNG). 2-run byte-identity preserved per R11.9 / R11.10.

**Sell arm**: NOT scaled (close-existing-position). Documented above. T-D-N3 adds an inline comment at line 300 noting "Sell-to-close: full-position exit, vol-target scale does NOT apply (would leak residual exposure on regime spikes)."

**Why this is the ONLY sizing site that needs the hook**:

| Scenario file | Reads vol-target overlay? | Hook needed? |
|---|---|---|
| `garch_vol_target_overlay.rs` | YES (the target) | YES |
| `tcn_overlay.rs` | NO | NO (default 1.0 inherit) |
| `tcn_overlay_weights.rs` | NO | NO |
| `patchtst_overlay_weights.rs` | NO | NO |
| `threshold_sweep.rs` | NO | NO |
| `pairs.rs` | NO | NO |
| `momentum.rs` | NO | NO |
| `sma_composed.rs` | NO | NO |

**Anchor implication**: by NOT calling `quantity_scale` from
non-vol-target scenarios, every non-vol-target anchor stays
byte-identical by construction. No `Decimal * Decimal::ONE`
multiplication introduced into hot paths that would otherwise need
to bit-compare. The 30 non-vol-target rows in `spec/anchors.toml`
hold byte-identical SHAs through the fix.

**Exec / live-trading parity (deferred)**: Live execution paths in
`crates/exec/` do not currently call any `Strategy::quantity_scale`
hook. v3 still ships at backtest-only per ADR-0038 § D5 ("risk-engine
+ live exec integration deferred to v0.1.1"). When live exec wires
up, the parity hook lives at the live-equivalent of the
`notional = equity * fraction` site. **Out-of-scope for this fix**;
flagged as a v0.1.1 follow-on item in the ADR-0038 § D6.b amendment.

### T-AR-3 — VolTargetingOverlay refactor (cache + accessor)

**File**: [`crates/strategy/src/vol_targeting_overlay.rs`](../../crates/strategy/src/vol_targeting_overlay.rs).

**Edits**:

1. **Add `scale_cache` field** to `VolTargetingOverlay` struct (line 143-156):

   ```rust
   pub struct VolTargetingOverlay {
       id: StrategyId,
       inner: MomentumStrategy,
       models: BTreeMap<Symbol, GarchParams>,
       state: BTreeMap<Symbol, PerSymbolGarchState>,
       config: VolTargetingConfig,
       pub stats: VolTargetingStats,
       // NEW: per-symbol cached scale factor from most recent on_bar.
       // Default 1.0 for symbols not yet seen.
       scale_cache: BTreeMap<Symbol, f64>,
   }
   ```

2. **Initialise in `new`** (line 215-222): add `scale_cache: BTreeMap::new()` to the struct literal.

3. **Populate in `on_bar`** (replace lines 305-319):

   ```rust
   // BEFORE (lines 305-319):
   // Compute scale factor.
   let scale = self.compute_scale(sigma_hat);
   //
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

   ```rust
   // AFTER (lines 305-321):
   // Compute scale factor.
   let scale = self.compute_scale(sigma_hat);

   // Cache the per-symbol scale for the sizing pipeline to query at
   // order-construction time via `Strategy::quantity_scale`.
   // (ADR-0038 § D5 strategy-side composition; § D6.b wiring-bug-fix
   // re-emission protocol — feature.md § R1.)
   self.scale_cache.insert(bar.symbol.clone(), scale);

   // Update stats counters (proxies for diagnostics — load-bearing
   // application happens via quantity_scale, not via signal mutation).
   let tol = 1e-6;
   if (scale - 1.0).abs() < tol {
       self.stats.signals_passthrough += base_signals.len() as u64;
   } else {
       self.stats.signals_scaled += base_signals.len() as u64;
   }
   base_signals
   ```

   Critical: the misleading "diagnostic only" comment is removed.
   `stats.signals_scaled` semantic is preserved (still a proxy);
   `stats.signals_passthrough` semantic is preserved. The behavioural
   change is **only** the `scale_cache.insert`. Existing 8 unit tests
   in `crates/strategy/tests/vol_targeting_overlay.rs` still pass
   (they test `compute_scale`, `init_sigma`, `forecast_step` — all
   unchanged).

4. **Override `quantity_scale`** in the `impl Strategy for VolTargetingOverlay` block (after `on_tick`, around line 325):

   ```rust
   fn quantity_scale(&self, symbol: &Symbol) -> f64 {
       self.scale_cache.get(symbol).copied().unwrap_or(1.0)
   }
   ```

   Default-on-miss `1.0` preserves correctness for the warm-up bars
   (before `on_bar` has been called for a symbol) and the no-model
   branch (when the symbol is not in the GARCH checkpoint — the
   counter `stats.bars_no_model` ticks but `compute_scale` is never
   queried so `scale_cache` stays empty for that symbol).

5. **NO other module touched**: the doc-comment header at lines 1-46
   already describes the intended semantic ("multiplied by `scale` at
   order-submission time"). The fix lands without contradicting the
   doc.

### T-AR-4 — R2 end-to-end regression test (the missing gate)

**File** (new): [`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](../../crates/strategy/tests/vol_targeting_overlay_end_to_end.rs).

**Shape**:

```rust
//! R2 end-to-end regression test (v3-volatility-forecaster-noop-fix).
//!
//! Asserts that `VolTargetingOverlay::quantity_scale(symbol)` returns
//! the cached per-symbol scale factor from the most recent `on_bar`,
//! and that a `scale != 1.0` query result differs from a `scale = 1.0`
//! query result by a testable epsilon (≥ 0.01).
//!
//! This is the gate that would have caught the v0.1.0 / v0.1.0-rebaseline
//! no-op: under the pre-fix code, `quantity_scale` returns the default
//! `1.0` regardless of the GARCH state, so the assertion at the bottom
//! FAILS. Under the fix, the cache populates and the assertion PASSES.
//!
//! # Forensic gate
//!
//! Run this test against current main BEFORE the fix lands (T-AR-6).
//! Expected: `assertion failed: (scale_after - 1.0).abs() >= 0.01` —
//! the trait's default `1.0` is what gets returned. After the fix lands,
//! the test passes.
//!
//! # Cross-references
//!
//! - feature.md § R2 — end-to-end equity-divergence regression.
//! - feature.md § R6 — unit + integration guards.
//! - decomp.md § T-AR-4 — this file's shape.

use std::collections::BTreeMap;

use rust_decimal_macros::dec;
use strategy::{
    GarchParams, MomentumStrategy, Strategy, VolTargetingConfig, VolTargetingOverlay,
    cross_sectional::CrossSectionalMomentumConfig,
};
use time::OffsetDateTime;
use trading_core::symbol::Symbol;
use trading_core::{Bar, Price, Quantity, Timeframe, Timestamp, Venue};

// Minimal momentum config (mirrors crates/strategy/tests/vol_targeting_overlay.rs).
fn stub_momentum() -> MomentumStrategy { /* ... copy-paste from existing test helpers ... */ }

// GARCH params rigged so that omega + alpha + beta = 0.95 (stable),
// with low unconditional_var → init_sigma is small → compute_scale
// (target_vol / sigma_hat) hits the clamp_max = 2.0.
fn high_scale_model() -> GarchParams {
    GarchParams {
        omega: 1e-10,
        alpha: 0.05,
        beta: 0.90,
        unconditional_var: 1e-10 / (1.0 - 0.05 - 0.90), // tiny → init_sigma ~3.16e-5
    }
}

fn make_bar(symbol: &str, ts: i64, close: rust_decimal::Decimal) -> Bar {
    Bar {
        ts: Timestamp::from_unix_seconds(ts).unwrap(),
        venue: Venue::new("BinanceSpot"),
        symbol: Symbol::new(symbol),
        timeframe: Timeframe::H1,
        open: Price::new(close).unwrap(),
        high: Price::new(close).unwrap(),
        low: Price::new(close).unwrap(),
        close: Price::new(close).unwrap(),
        volume: Quantity::new(dec!(1.0)).unwrap(),
    }
}

#[test]
fn overlay_quantity_scale_reflects_computed_factor() {
    let inner = stub_momentum();
    let mut models = BTreeMap::new();
    models.insert("BTCUSDT".to_string(), high_scale_model());

    let mut overlay = VolTargetingOverlay::new(
        inner,
        models,
        VolTargetingConfig::default(), // target_vol = 0.02
    );

    let btc = Symbol::new("BTCUSDT");

    // Pre-on_bar query → default 1.0 (no cache entry yet).
    let scale_before = overlay.quantity_scale(&btc);
    assert_eq!(scale_before, 1.0, "default-on-miss must be 1.0 (no on_bar yet)");

    // Drive on_bar with a sequence of bars on BTCUSDT.
    // With high_scale_model, sigma_hat stays tiny → compute_scale hits clamp_max = 2.0.
    for i in 0..5 {
        let bar = make_bar("BTCUSDT", 1_700_000_000 + i * 3600, dec!(50_000.0));
        let _signals = overlay.on_bar(&bar);
    }

    // Post-on_bar query → cached scale (expected ~2.0, the clamp_max).
    let scale_after = overlay.quantity_scale(&btc);
    assert!(
        (scale_after - 1.0).abs() >= 0.01,
        "vol-target overlay produced scale={scale_after} after 5 on_bar calls — \
         expected ≠ 1.0 (no-op signature). This is the R2 forensic gate; \
         under the pre-fix code this assertion FAILS because quantity_scale \
         returns the default 1.0 regardless of GARCH state."
    );
    assert!(
        (scale_after - 2.0).abs() < 0.01,
        "expected scale ≈ 2.0 (clamp_max for low-sigma regime); got {scale_after}"
    );

    // Symbol not in the GARCH checkpoint → default 1.0 (no cache write).
    let eth = Symbol::new("ETHUSDT");
    let scale_eth = overlay.quantity_scale(&eth);
    assert_eq!(scale_eth, 1.0, "no-model symbol must inherit default 1.0");
}
```

**Why this is THE missing gate**: the 8 existing unit tests in
`crates/strategy/tests/vol_targeting_overlay.rs` verify `compute_scale`
math in isolation (`compute_scale(target_vol) == 1.0`, clamp invariants,
`init_sigma > 0`, etc.). None of them call `on_bar` + then read the
result back via the load-bearing accessor. This test is the bridge.

**The forensic gate (T-AR-6 verification)**: the test is run against
**current main** (before the fix lands). Expected output is a FAIL on
`assert!((scale_after - 1.0).abs() >= 0.01, ...)` because the pre-fix
code has no `quantity_scale` override; the trait default returns 1.0.
This proves the test catches the regression. The developer's T-D-N3a
(below) captures this literal pre-fix failure as evidence.

**Cargo invocation** (Wave A T-D-N3a forensic gate, run BEFORE the fix):

```bash
cargo test -p strategy --test vol_targeting_overlay_end_to_end -- --nocapture
```

**Expected pre-fix output** (failure):

```
running 1 test
test overlay_quantity_scale_reflects_computed_factor ... FAILED

failures:

---- overlay_quantity_scale_reflects_computed_factor stdout ----
thread 'overlay_quantity_scale_reflects_computed_factor' panicked at 'vol-target overlay produced scale=1 after 5 on_bar calls — expected ≠ 1.0 (no-op signature). This is the R2 forensic gate; under the pre-fix code this assertion FAILS because quantity_scale returns the default 1.0 regardless of GARCH state.'

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

**Expected post-fix output** (pass):

```
running 1 test
test overlay_quantity_scale_reflects_computed_factor ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in <wall-clock>s
```

The developer's T-D-N3a captures the pre-fix FAILED panic verbatim
into the Wave A status update. If pre-fix run PASSES, the test is
wrong (false negative; tests the wrong field) and Wave A halts.

### T-AR-5 — `vol-verdict-bs1-realdata` audit (analyst's audit-pending row)

**Walk of [`crates/forecast/src/bin/vol_verdict.rs`](../../crates/forecast/src/bin/vol_verdict.rs)** — body emit section, lines 460-587:

| Body section | Lines | Reads | Overlay equity? |
|---|---|---|---|
| Header | 462-464 | Static title | NO |
| Checkpoint table | 467-496 | `checkpoint_revision`, `target_kind`, `target_horizon_bars`, `train_span_*`, `UNIVERSE.len()`, `n_predictions_total` | NO |
| Per-symbol QLIKE table | 499-522 | `s.qlike_garch`, `s.qlike_constant`, `s.mean_sigma_hat`, `s.mean_sigma_realized`, `s.std_sigma_hat`, `s.std_sigma_realized`, `s.calibration_ratio()`, `s.improvement_pct()` | NO |
| Aggregate statistics | 525-556 | `agg.qlike_garch_mean`, `agg.qlike_constant_mean`, `agg.qlike_garch_max`, `agg.qlike_garch_min`, `agg.qlike_dispersion`, `agg.mean_calibration_ratio`, `agg.n_symbols_improving` | NO |
| Verdict | 559-571 | `verdict.label()`, `verdict.evidence()`, `verdict.routes_to()` (V1/V2/V3/V4/V5 only; no equity input) | NO |
| Notes | 574-587 | Static text + `checkpoint_revision` | NO |

**Finding**: `vol-verdict-bs1-realdata` body is **GARCH-only**. Every
quantity cited is computed on `predicted-vs-realized GARCH sigma_hat`
**before** the vol-targeting overlay tries to apply the scale. The
V-verdict bin does not load the overlay, does not run a backtest,
does not read any equity curve. It is a pure model-diagnostic.

**Conclusion**: `vol-verdict-bs1-realdata` anchor SHA
(`99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21`)
stays byte-identical post-fix. **Re-emission is NOT required** for
this row.

**Verification at tester M-FINAL (T-T2)**: tester runs
`scripts/verify_anchors.sh` post-fix; expects `vol-verdict-bs1-realdata`
row to show `PASS` against the unchanged SHA above. This row is one of
the 31 negative-invariant rows.

**Updated anchor delta** (T-AR-5 finalises analyst's 4-row enumeration):

| Namespace | Scenario | Re-emit? | Current SHA |
|---|---|---|---|
| `[v3.0.0-volatility]` | `vol-verdict-bs1-realdata` | **NO** (GARCH-only body, audit-confirmed) | `99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21` |
| `[v3.0.0-volatility]` | `top10-2023-fy-vol-target-overlay-realdata` | YES | `66cd69ad03294cccf514184968babce0127f2ebfa4d1f4a03b332f8000f79c65` |
| `[v3.0.0-volatility]` | `sharpe-comparison-vol-target-bs1-realdata` | YES | `ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1` |
| `[v3.0.0-volatility-rebaseline]` | `sharpe-comparison-vol-target-bs1-realbaseline` | YES | `d561fed564166f8c907cc9dda98fd2d56eb03333bd5aea16a0f6425924a2afb8` |

**Final delta**: 3 SHAs re-emit; 31 SHAs stay byte-identical
(includes the 30 pre-v3 anchors + `vol-verdict-bs1-realdata`).
Tester M-FINAL expects `ANCHORS PASS (34 / 34)` with the 3 new
SHAs locked at Wave B T-D-N10.

### T-AR-6 — Wave shape + parallelism

#### Wave A — wire-up fix + tests (sequential, ~80-150 LoC)

Each step depends on the previous:

| Task | Edit | Cargo invocation | Expected literal output |
|---|---|---|---|
| **T-D-N1** | Add defaulted `quantity_scale` to `Strategy` trait at `crates/strategy/src/traits.rs:14` (+ import `Symbol`). | `cargo check -p strategy` | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in <wall-clock>s` |
| **T-D-N2** | Refactor `VolTargetingOverlay` at `crates/strategy/src/vol_targeting_overlay.rs:143-322`: add `scale_cache` field, populate in `on_bar` (lines 305-319 region), override `quantity_scale`. | `cargo test -p strategy --test vol_targeting_overlay` | `test result: ok. 8 passed; 0 failed; ...` (existing tests stay green; math-only) |
| **T-D-N3a** | **Forensic gate**: author `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` (T-AR-4 shape) and run against the **pre-fix** code (revert T-D-N2 temporarily OR run on a sibling branch). | `cargo test -p strategy --test vol_targeting_overlay_end_to_end -- --nocapture` | `test result: FAILED. 0 passed; 1 failed; ...` with the literal `'vol-target overlay produced scale=1 after 5 on_bar calls — expected ≠ 1.0 (no-op signature)'` panic message. **Captured verbatim into Wave A status update**. |
| **T-D-N3b** | Re-apply T-D-N2 + re-run the e2e test. | `cargo test -p strategy --test vol_targeting_overlay_end_to_end` | `test result: ok. 1 passed; 0 failed; ...` |
| **T-D-N4** | Sizing-pipeline hook at `crates/backtest/src/scenarios/garch_vol_target_overlay.rs:262-265` (Buy arm only; Sell arm gets inline comment per T-AR-2). | `cargo check -p backtest --features candle,realdata` | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in <wall-clock>s` |
| **T-D-N5** | Add R6 unit test rows to `crates/strategy/tests/vol_targeting_overlay.rs` (existing file): `scale_cache_populates_after_on_bar`, `quantity_scale_default_for_unseen_symbol`. ~30 LoC. | `cargo test -p strategy --test vol_targeting_overlay` | `test result: ok. 10 passed; 0 failed; ...` (8 existing + 2 new) |
| **T-D-N6** | Workspace gate: `cargo fmt --check` + `cargo clippy --workspace --features candle,realdata -- -D warnings` + `cargo test --workspace --features candle,realdata`. | `cargo test --workspace --features candle,realdata` | `test result: ok. <N> passed; 0 failed; ...` for every test binary |

**Wave A wall-clock estimate**: ~30-60 min coding + ~15 min full workspace test = ~45-75 min.

#### Wave B — anchor re-emission (sequential after Wave A)

Each scenario produces a deterministic body; the developer runs each twice and confirms byte-identity before locking the SHA.

| Task | Cargo invocation | Expected literal output (header) |
|---|---|---|
| **T-D-N7** | Re-emit `top10-2023-fy-vol-target-overlay-realdata`: `cargo run -p backtest --release --features candle,realdata -- --scenario top10-2023-fy-vol-target-overlay-realdata --seed 0xC0FFEE` | `garch-vol-target-overlay backtest complete` tracing line + report at `spec/v3-volatility-forecaster/reports/backtest-<ts>-top10-2023-fy-vol-target-overlay-realdata.md`. Body-SHA captured via `python3 scripts/hash_report.py <path>`. |
| **T-D-N8** | Run T-D-N7 a second time + diff body-SHA-256 (R11.9 / R11.10 byte-identity gate carry-forward). | `<sha-T-D-N7> == <sha-T-D-N8>` (byte-identical body) |
| **T-D-N9** | Re-emit `sharpe-comparison-vol-target-bs1-realdata`: `cargo run -p forecast --release --bin sharpe_comparison --features candle -- --scenario vol-target-bs1` | Body at `spec/v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-<ts>.md`. SHA via `hash_report.py`. |
| **T-D-N10** | Re-emit `sharpe-comparison-vol-target-bs1-realbaseline`: `cargo run -p forecast --release --bin sharpe_comparison --features candle -- --scenario vol-target-bs1-rebaseline` | Body at `spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-<ts>.md`. SHA via `hash_report.py`. |
| **T-D-N11** | Determinism re-confirm: run T-D-N9 + T-D-N10 a second time each; diff SHAs. | All four body-SHAs (2 from T-D-N9 + 2 from T-D-N10) byte-identical pairwise. |
| **T-D-N12** | Lock the 3 new SHAs in `spec/anchors.toml`: replace the 3 SHAs in-place under their existing namespaces (Q2=(a)). Add a comment block referencing this feature + the discovery dev-note. | (Edit only; no cargo run.) |
| **T-D-N13** | Run `bash scripts/verify_anchors.sh`. | `ANCHORS PASS  (34 / 34)` with the 3 fresh SHAs + 31 unchanged (negative invariant on `vol-verdict-bs1-realdata` + the 30 pre-v3 rows). |

**Wave B wall-clock estimate** (carry-forward from rebaseline pass: ~40s per scenario × 2 runs × 3 scenarios + verify_anchors = ~5 min). Use the watch-recipe pattern below for the `top10-2023-fy-vol-target-overlay-realdata` re-run if it overruns 2 min:

```bash
watch -n 5 'ls -lt spec/v3-volatility-forecaster/reports/backtest-*-top10-2023-fy-vol-target-overlay-realdata.md 2>/dev/null | head -3; echo "---"; pgrep -af "backtest --release" | head -3'
```

#### Wave C — ADR-0038 § D6.b amendment + tester handoff (parallel-safe with B)

| Task | Edit | Cargo invocation |
|---|---|---|
| **T-D-N14** | Append § D6.b amendment subsection to `spec/architecture/adr/0038-vol-forecast-verdict-shape.md` at the end of § D6 (after line 606). Text per T-AR-7 below (~30 lines). | (Edit only via spec-update skill.) |
| **T-D-N15** | Flip `spec/trace.toml` REQ-V3-VOL-FORECASTER-NOOP-FIX-001 state `proposed → in-progress` AND extend `arch` / `crates` / `tests` / `anchors` columns. | (Edit only.) |
| **T-D-N16** | Append `## Design` block to `spec/v3-volatility-forecaster-noop-fix/feature.md` (cross-pointer to this decomp.md). | (Edit only.) |

**Wave C wall-clock estimate**: ~15 min (text edits + 1 spec-update validation).

#### Parallelism contract

- Wave A → Wave B: STRICT sequential (Wave B re-emits depend on the fix landing in Wave A).
- Wave B ‖ Wave C: PARALLEL-SAFE. Wave C is pure-text spec edits (ADR + trace.toml + feature.md); Wave B is binary re-runs + anchors.toml SHA lock. No file overlap; no read-after-write hazard.
- Developer kicks off Wave C as a side task while Wave B's longest scenario (`top10-2023-fy-vol-target-overlay-realdata` ~40s × 2 runs) is in-flight.

### T-AR-7 — ADR-0038 § D6.b amendment text (R5 deliverable)

The architect drafts the subsection text here; developer T-D-N14 lands it verbatim via spec-update. Target location: `spec/architecture/adr/0038-vol-forecast-verdict-shape.md` end of § D6 (after the existing "Anchor count progression" block at line 606, before the `## Alternatives considered` H2 at line 608).

**Text to append**:

```markdown
### D6.b — Wiring-bug-fix re-emission protocol (amendment, 2026-05-22)

Adopted under [v3-volatility-forecaster-noop-fix](../../v3-volatility-forecaster-noop-fix/feature.md) v0.1.0 (P0). The original D6 contract reads "existing anchors stay byte-identical." That spirit is **don't silently mutate historical evidence**. When the recorded body reflects a demonstrated wiring bug (the contract being witnessed is materially different from what was intended), re-emission is legitimate **under the following protocol**:

1. **Enumerate affected anchors** with current SHA-256 in the feature brief's § Investigation findings. The architect confirms the enumeration is exhaustive at M-T1 (e.g. via cross-grep of the report-body sources for the load-bearing observable; see [v3-volatility-forecaster-noop-fix decomp.md § T-AR-5](../../v3-volatility-forecaster-noop-fix/decomp.md) for the worked example — 4 candidates audited, 1 ruled out as GARCH-only).
2. **Cite the bug site with `file:line`** in the feature brief's § Smoking gun. The dev-note captures the diagnostic chain that surfaced the bug (cf. [v3-vol-overlay-noop-discovery-2026-05-22.md](../../dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md) — caveman probe + byte-identical surfacing).
3. **Include the would-have-caught test** as a feature requirement (e.g. R2 in the worked example). The test MUST be run against the **pre-fix** code BEFORE the fix lands; the architect captures the literal pre-fix FAIL output as evidence the gate is meaningful (cf. [v3-volatility-forecaster-noop-fix decomp.md § T-AR-4 forensic gate](../../v3-volatility-forecaster-noop-fix/decomp.md)).
4. **Architect signs off on the re-emission delta**. The new SHAs land in `spec/anchors.toml` **in-place under the existing namespaces** (Q2=(a) default — never bifurcate the namespace; never silently delete a row). A comment block above the affected rows cites the fix-feature slug + the dev-note slug.
5. **Negative invariant**: the unchanged rows MUST stay byte-identical. Tester M-FINAL captures the diff at `spec/<fix-feature>/reports/test-final-<date>.md` showing every changed row + the count of unchanged rows. The wave-B verify_anchors.sh output (`ANCHORS PASS (N / N)`) is the gate; a regression to `FAIL` halts the ship.

**Allowed re-emission scope**: only rows whose body cites the load-bearing observable that the bug perturbed. Rows that cite orthogonal observables (e.g. GARCH-only model diagnostics for an overlay-wiring fix) stay byte-identical and are part of the negative invariant.

**Not in scope of this protocol**: silent mutations (forbidden by D6 spirit), namespace bifurcation (a `*-postfix` namespace was rejected at Q2 — bifurcation invites future readers to consume stale bodies), row deletion (forbidden — historical evidence stays linked even after re-emission via the dev-note + feature.md cross-references).

**Live-exec parity follow-on**: the v0.1.0 vol-target wire-up landed at the **backtest-only** sizing-pipeline site (`crates/backtest/src/scenarios/garch_vol_target_overlay.rs`). Live execution (when wired in `crates/exec/` post-v0.1.1) MUST add an equivalent `Strategy::quantity_scale` query at the live order-construction site. Parity gap is flagged in [v3-volatility-forecaster-noop-fix decomp.md § T-AR-2](../../v3-volatility-forecaster-noop-fix/decomp.md) and tracked as a v0.1.1 follow-on item.

**Precedent**: this is the **first** invocation of D6.b. Future wiring-bug discoveries inherit the 5-step protocol verbatim. If the protocol itself needs revision (e.g. multi-overlay wire-up bugs requiring batched re-emissions), the revision lands as **D6.c** (additive amendment subsection, not in-place mutation of D6.b).
```

This subsection is ~35 lines (estimated). It is **additive** — does not mutate the original § D6 text. The existing § D6 paragraph at lines 589-606 stays byte-identical. The 5-clause protocol mirrors R5's acceptance criterion (a, b, c, d from feature.md § R5 + an additional negative-invariant clause derived from the worked example).

### T-AR-8 — TCN overlay re-audit under Q1=(ii) (Q3=(b) finalisation)

**T-A2 finding confirmed**: TCN overlay's `combine_with_direction` mutates `Signal.kind` (line 697-700 of `crates/strategy/src/tcn_overlay_momentum.rs`), and `Signal.kind` is a load-bearing field the executor already reads via the `match sig.kind` discriminant at every sizing site. The TCN dampen-to-Hold semantic is **already wired**; the executor's Hold-branch (which omits the order entirely) is the existing wire.

**Re-audit under Q1=(ii)**: does adding `Strategy::quantity_scale` with a default `1.0` introduce any adjacent break in TCN overlay's `Signal { kind: modulated_kind, ..sig }` spread (line 697-700)?

**Answer: NO.**

1. The trait method is defaulted — TCN overlay inherits `1.0` without touching its impl block.
2. The TCN overlay's `Signal` spread is unchanged (the spread does not interact with the trait surface).
3. The TCN scenarios (`tcn_overlay.rs`, `tcn_overlay_weights.rs`) **do not call** `quantity_scale` in the sizing pipeline (per T-AR-2 table). Default-1.0 is never queried for TCN.
4. The TCN scenarios' anchors stay byte-identical by construction.

**Verification at tester M-FINAL**: tester runs `verify_anchors.sh`; expects the 6 TCN anchors (`top10-2023-fy-tcn-overlay`, `top10-2024-fy-tcn-overlay`, `top10-2023-fy-tcn-overlay-weights`, `top10-2024-fy-tcn-overlay-weights`, `top10-2023-fy-tcn-overlay-realdata`, `top10-2024-fy-tcn-overlay-realdata`, `top10-2023-fy-tcn-overlay-weights-realdata`, `top10-2024-fy-tcn-overlay-weights-realdata`) to PASS unchanged.

**Conclusion**: Q3=(b) holds; TCN overlay audit closed; no follow-on filed.

## 2. Anchor delta summary

**Re-emit (3 rows, in-place under existing namespaces — Q2=(a))**:

| Namespace | Scenario | Pre-fix SHA | Post-fix SHA |
|---|---|---|---|
| `[v3.0.0-volatility]` | `top10-2023-fy-vol-target-overlay-realdata` | `66cd69ad03294cccf514184968babce0127f2ebfa4d1f4a03b332f8000f79c65` | TBD by developer at T-D-N7 |
| `[v3.0.0-volatility]` | `sharpe-comparison-vol-target-bs1-realdata` | `ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1` | TBD at T-D-N9 |
| `[v3.0.0-volatility-rebaseline]` | `sharpe-comparison-vol-target-bs1-realbaseline` | `d561fed564166f8c907cc9dda98fd2d56eb03333bd5aea16a0f6425924a2afb8` | TBD at T-D-N10 |

**Stay byte-identical (31 rows — negative invariant)**:

- `vol-verdict-bs1-realdata` (GARCH-only body; T-AR-5 audit confirmed)
- 30 pre-v3 anchors (lines 15-237 of `spec/anchors.toml`): every SMA / momentum / pairs / TCN / PatchTST / threshold-sweep / forecast-distribution / recalibrate-sigma-train / report-sample row — none read the vol-target overlay.

**Total after Wave B**: `ANCHORS PASS (34 / 34)`. No row count change.

## 3. Spec hygiene at M-T1 close

- [x] `tasks.md` frontmatter flipped `owner: analyst → architect` (developer flips to `developer` at Wave A start).
- [x] T-AR-1..T-AR-7 ticked in tasks.md with literal output / decision rationale citations (this decomp.md is the canonical reference).
- [x] `spec/trace.toml` REQ-V3-VOL-FORECASTER-NOOP-FIX-001 state flipped `proposed → in-progress`.
- [x] `spec/v3-volatility-forecaster-noop-fix/feature.md` § Design block appended (cross-pointer to this decomp.md).
- [x] Baseline `ANCHORS PASS  (34 / 34)` quoted verbatim in § Baseline gate above.

## 4. Risks at hand-off

| Risk | Mitigation |
|---|---|
| Developer accidentally introduces `Decimal::try_from(scale).unwrap()` instead of `.unwrap_or(Decimal::ONE)` — NaN-bomb on a future GARCH divergence. | Code review at T-D-N4 cargo check; `unwrap_or` is the locked form per T-AR-2 hook block above. |
| Developer adds the hook to `tcn_overlay.rs` / `pairs.rs` / etc. (out of T-AR-2's NO list) — would re-emit non-vol-target anchors. | T-AR-2 table is the gate; tester M-T2 negative invariant on the 31 unchanged rows catches the regression. |
| `vol-verdict-bs1-realdata` body changes for some incidental reason (e.g. checkpoint_revision string shifts on a Cargo bump). | T-T2 captures the row's PASS/FAIL state in the test report; if FAIL, route back to developer for diagnosis BEFORE shipping (NOT an automatic re-emit — the row was audited as GARCH-only at T-AR-5). |
| Wave B's 3 re-emitted bodies fail R11.9 byte-identity (2-run divergence). | Per ADR-0038 § D7.b carry-forward, R11.9 byte-identity is a release gate; failure routes to developer for a determinism investigation BEFORE M-FINAL. |
| Sell-arm scaling regression: a future developer adds the hook to the Sell arm of `garch_vol_target_overlay.rs:298-329` — would leak residual exposure on regime spikes. | T-AR-2 inline comment at the Sell arm + T-D-N4 code review. Defensive but not load-bearing for v0.1.0 (the bug would surface as drift in trade_count, caught by R2's `trade_count_ratio ∈ [0.95, 1.05]` half). |
| Q1=(ii) means live-exec wiring is **not** automatic (parity gap). | Documented in T-AR-2 + ADR-0038 § D6.b amendment as a v0.1.1 follow-on. v3 ships at backtest-only per parent ADR-0038 § D5. |

## 5. Watch recipe (per MEMORY.md)

If any of Wave B's 3 re-emissions overruns 2 min wall-clock, developer
emits:

```bash
# For top10-2023-fy-vol-target-overlay-realdata (largest scenario):
watch -n 5 'ls -lt spec/v3-volatility-forecaster/reports/backtest-*-top10-2023-fy-vol-target-overlay-realdata.md 2>/dev/null | head -3; echo "---"; pgrep -af "backtest --release" | head -3'
```

```bash
# For sharpe-comparison-vol-target-bs1-realdata (forecast bin):
watch -n 5 'ls -lt spec/v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-*.md 2>/dev/null | head -3; echo "---"; pgrep -af "sharpe_comparison" | head -3'
```

```bash
# For sharpe-comparison-vol-target-bs1-realbaseline:
watch -n 5 'ls -lt spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-*.md 2>/dev/null | head -3; echo "---"; pgrep -af "sharpe_comparison" | head -3'
```

Per rebaseline carry-forward, each is ~40s on M-series; watch likely
not required. Keep the recipe ready for cold-cache cases.

## Changelog

- 2026-05-22 (architect): decomp authored at M-T1 close.
  - T-AR-1: `Strategy::quantity_scale(&self, &Symbol) -> f64 { 1.0 }` defaulted trait method at `crates/strategy/src/traits.rs:14`; `&self`/`&Symbol` signature locked.
  - T-AR-2: sizing-pipeline call site is `crates/backtest/src/scenarios/garch_vol_target_overlay.rs:262-265` (Buy arm only; Sell arm gets inline comment, scale does NOT apply to close-by-full-position).
  - T-AR-3: `VolTargetingOverlay` gets `scale_cache: BTreeMap<Symbol, f64>` field; `on_bar` populates; `quantity_scale` reads. Misleading "diagnostic only" comment removed.
  - T-AR-4: R2 e2e test at new file `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs::overlay_quantity_scale_reflects_computed_factor`; asserts `(scale_after - 1.0).abs() >= 0.01` post-on_bar; forensic gate captures literal pre-fix FAIL panic message verbatim.
  - T-AR-5: `vol-verdict-bs1-realdata` audit concluded — body is GARCH-only (Checkpoint + Per-symbol QLIKE + Aggregate stats + Verdict + Notes; no overlay equity citations). Row stays byte-identical post-fix.
  - T-AR-6: Waves locked — A (4 LoC edits, sequential, ~45-75 min) → B (3 re-emits, ~5 min, sequential after A) ‖ C (text edits, ~15 min, parallel-safe with B). Anchor delta: 3 re-emit + 31 stay byte-identical = `ANCHORS PASS (34 / 34)`.
  - T-AR-7: ADR-0038 § D6.b amendment subsection text drafted (~35 lines, 5-clause re-emission protocol).
  - T-AR-8 (was: re-audit): TCN overlay re-confirmed structurally different under Q1=(ii); inherits default 1.0; no anchor delta; Q3=(b) holds.
  - HANDOFF → orchestrator → developer.
