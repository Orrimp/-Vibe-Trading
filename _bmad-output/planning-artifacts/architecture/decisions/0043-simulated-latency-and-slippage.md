---
adr: 0043
title: Simulated network latency + order-book slippage in backtest
status: accepted
date: 2026-05-26
deciders: analyst (M0) → architect (M-T1)
supersedes: []
superseded_by: []
related: ["ADR-0030 cockpit-in-process-backtest", "ADR-0032 backtest-realdata-path-and-revision-pin", "ADR-0038 spec-anchor-bounded-set-discipline"]
---

# ADR-0043 — Simulated network latency + order-book slippage in backtest

> The operator brief suggested `0040-simulated-latency-and-slippage.md`,
> but 0040 is already taken by `yahoo-realdata-path-and-revision-pin`,
> 0041 by `trader-crate-split`, and 0042 by
> `cockpit-activity-broadcast`. This ADR lands at **0043** — the next
> free slot.

## Context

The current backtest model assumes:

- **Immediate execution at the bar's close** — `MatchingEngine` in
  `crates/exec` produces fills whose `(price, ts_ms)` are deterministic
  functions of the bar + the order. Zero wire latency.
- **Zero order-book slippage** — fills land at the bar-recorded price
  with no walk-the-book cost beyond the existing taker-fee model in
  `crates/cost`.

Both are systematically optimistic. Real venues impose 20-100 ms wire
latency typically; market orders walk the book and fill worse than mid.
A backtest that ignores these two frictions **overestimates strategy
alpha** — the well-known "backtest-vs-live gap" that kills paper-to-
live transitions.

The feature brief at
[`docs/archive/pre-bmad-spec/v5-latency-slippage-sim/feature.md`](../../../../docs/archive/pre-bmad-spec/v5-latency-slippage-sim/feature.md)
introduces deterministic, optional, default-zero simulation of both
frictions. This ADR codifies the 5 sub-decisions (D1-D5) that determine
how the simulator is wired without breaking the 34 SHA-256 anchors in
[`evidence/anchors.toml`](../../../../evidence/anchors.toml).

## Decision

### D1 — Always-on code path with default-zero values

Reject a Cargo feature flag (`#[cfg(feature = "latency_slippage_sim")]`).
The simulator is always compiled in; default config produces noop
behavior byte-identical to the pre-feature code.

**Why**:
- Two code paths (with/without feature) double the CI matrix and
  introduce a structural divergence risk over time.
- The hot-path cost at noop values is one multiplication-by-1.0 + one
  integer-add-of-0. Branch prediction makes this effectively free at
  scale (R-NR.4 perf gate < 1% regression).
- The configuration toggle is a runtime field on `ScenarioConfig`
  (`LatencySlippageSimConfig`) — operator-controlled, not compile-
  time-locked.

**Implementation contract**: every existing call site that constructs
`ScenarioConfig` without specifying `latency_slippage_sim` gets
`LatencySlippageSimConfig::default()` — all zeros. This applies to the
34 anchored scenarios verbatim, preserving byte identity.

### D2 — Seeded RNG sub-stream for latency jitter

Latency is sampled from a uniform distribution
`[latency_ms_min, latency_ms_max]` when the range is non-degenerate. The
sampling RNG MUST be deterministic across runs — wall-clock or
`thread_rng()` would invalidate the anchor invariant.

**Implementation**: a `ChaCha20Rng` sub-stream is constructed by
hashing `(scenario_seed, order_id)` into a 32-byte derived seed:

> **Canonical (M-FINAL deviation):** the shipped implementation is a
> Murmur3-style bit mixer, NOT ChaCha20 — ChaCha20 init was 8–12× over the
> ≤50 ns R7 target. See the § Changelog (2026-05-27) and
> `crates/exec/src/latency.rs`. The `ChaCha20Rng` sketch below is retained as
> the original ADR intent (a deterministic sub-stream keyed on
> `(scenario_seed, order_id)`), which the mixer satisfies.

```rust
fn latency_rng_for_order(scenario_seed: [u8; 32], order_id: OrderId) -> ChaCha20Rng {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"v5-latency-slippage-sim:latency:");
    hasher.update(&scenario_seed);
    hasher.update(&order_id.0.to_le_bytes());
    let derived = hasher.finalize();
    ChaCha20Rng::from_seed(*derived.as_bytes())
}
```

Each order gets its own deterministic RNG; the same `(seed, order_id)`
always produces the same latency value. This preserves replay
determinism even if order ID assignment changes order between runs.

### D3 — Linear bps slippage at v0.1.0; defer square-root to v0.2.0

The slippage model is a linear bps multiplier on the signal price,
sign-applied per `Side`:

```rust
pub fn apply_slippage(
    signal_price: Decimal,
    side: Side,
    _notional: Decimal,  // unused at v0.1.0; reserved for v0.2.0 square-root
    bps: u32,
) -> Decimal {
    if bps == 0 {
        return signal_price;
    }
    let bps_decimal = Decimal::from(bps) / Decimal::from(10_000u32);
    match side {
        Side::Buy => signal_price * (Decimal::ONE + bps_decimal),
        Side::Sell => signal_price * (Decimal::ONE - bps_decimal),
    }
}
```

**Why linear, not square-root**:
- Square-root market impact (`impact ∝ sqrt(notional / depth)`) needs
  an order-book depth estimate that isn't available in our Parquet bar
  data at v0.1.0. Adding it requires an order-book ingest module — out
  of scope.
- Bid-ask additive (`fill = mid ± spread/2`) needs bid-ask spread data
  on bars — also missing.
- Linear bps is the academic baseline; produces the right qualitative
  behavior (larger slippage hurts P&L) without modeling depth.

**Future**: v0.2.0 may swap in square-root. The signature already
includes `notional` as an unused parameter to make that swap a one-
function-body change without rippling through call sites.

### D4 — New audit-event variant; skip-when-zero guard

Add to `crates/audit::AuditEvent`:

```rust
SimulatedExecMetrics {
    order_id: OrderId,
    fill_id: FillId,
    latency_ms_applied: u64,
    slippage_bps_applied: u32,
    slippage_dollars_applied: Decimal,
},
```

**Skip-when-zero guard** (load-bearing for R-NR.1 anchor preservation):
emit this variant ONLY when at least one of `latency_ms_applied > 0` OR
`slippage_bps_applied > 0`. Otherwise write nothing to the audit
ledger.

**Why**:
- Always-emitting the variant would change the audit-ledger SHA on
  every anchored scenario → anchor regression.
- Emitting only when non-zero makes the additive contract: pre-feature
  ledgers stay byte-identical at default-zero config.

Variant is additive on `AuditEvent` — the v0 audit-replay path
(`crates/audit::replay`) handles unknown variants via the existing
`#[serde(other)]` skip-pattern, preserving backward compat per R-NR.6.

### D5 — Backtest-only scope; live mode untouched

`LatencySlippageSimConfig` is read by `crates/backtest::ScenarioConfig`
only. The `crates/agent` (live mode) ingest pipeline does NOT consult
this config — live fills come from the real venue with real latency
and real slippage already imposed.

**Why**:
- Simulating latency/slippage on live mode would double-count real
  frictions.
- Structural separation makes the operator-confusion path
  (K7) compile-time-noisy: if someone tries to apply
  `LatencySlippageSimConfig` to a live `OrderRouter`, they hit a type
  error.

**Implementation guard**: `LatencySlippageSimConfig` is defined in
`crates/backtest` (not `crates/exec` or `crates/cost`). The exec/cost
fns take the parameters via function args, not via a global config —
they're stateless w.r.t. the simulator.

## Consequences

### Positive

1. **Closes the backtest-vs-live gap** — operators can now stress-test
   strategies under realistic execution frictions before promoting to
   paper trading.
2. **Default-zero preserves all 34 anchors** — no migration friction at
   v0.1.0 ship; existing anchored reports remain immutable.
3. **New audit-ledger metric** — `SimulatedExecMetrics` rows give the
   ops dashboard a verifiable record of every simulated friction
   applied, dollar-quantified.
4. **Forward-compatible signature** — `apply_slippage(notional)` is
   already plumbed for v0.2.0 square-root market impact.
5. **Deterministic** — D2 seeded RNG sub-stream preserves replay
   reproducibility; the same `(scenario_seed, order_id)` always
   produces the same latency.

### Negative

1. **New surface for cross-strategy divergence** — every overlay or
   sizing-modifier (vol_targeting, vol_killswitch, future) now has an
   additional dimension to verify at v0.2.0 anchor migration. The
   v0.2.0 brief MUST run the cross-product (overlay × non-zero
   sim-config) e2e tests.
2. **Slight perf cost on the hot path** — even at noop values, the
   apply_latency/apply_slippage functions run on every fill. R-NR.4
   gates this at < 1% regression on the 8760-bar momentum scenario.
3. **Audit-ledger volume grows when enabled** — every fill emits a
   new `SimulatedExecMetrics` row. At a 100-trade/day strategy this
   is negligible; at a 1000-trade/day high-frequency strategy it
   would add ~365k rows/year. Cross-feature interaction with
   cockpit-activity-audit-ledger-producer (v0.1.1) is handled by the
   audit-producer's 100ms aggregation.
4. **Operator can mis-configure** — enabling non-zero values on a
   scenario whose anchored report assumes zeros silently invalidates
   the SHA. The presenter deck (M-PRESENTER) calls this out; the
   v0.2.0 anchor-migration brief codifies the explicit operator
   decision per scenario.

## Alternatives rejected

### Cargo feature flag (`#[cfg(feature = "latency_slippage_sim")]`)

**Rejected.** Two code paths means two CI matrices, two compile
artifacts. Over time the unused path bit-rots. The hot-path cost at
noop values is small enough (D1 justification) to keep one path.

### Externalize to live-only / agent-side post-processing

**Rejected.** Defeats the purpose. The whole point is to introduce
realistic friction INTO the backtest so paper-to-live transitions are
less surprising. Doing it only in live mode keeps the gap open.

### Stochastic mid-bar fills (random sample within the bar's HLC)

**Rejected as out-of-scope at v0.1.0.** This would be a much larger
architectural change — every bar would need to be sampled instead of
closed at the bar end. Defer to a future v0.3.0+ "intrabar fill
sampling" brief if/when needed.

### Audit-event extension via column on existing FillMemo row

**Rejected (D4 detail).** Mutating the FillMemo schema requires an
audit-replay migration. Additive variant via the enum is safer.

### Wall-clock latency sampling

**Rejected outright (D2).** Wall-clock or `thread_rng()` would break
the anchor invariant. The seeded sub-stream is non-negotiable.

## Cross-references

- Feature brief — [`docs/archive/pre-bmad-spec/v5-latency-slippage-sim/feature.md`](../../../../docs/archive/pre-bmad-spec/v5-latency-slippage-sim/feature.md)
- Tasks — [`docs/archive/pre-bmad-spec/v5-latency-slippage-sim/tasks.md`](../../../../docs/archive/pre-bmad-spec/v5-latency-slippage-sim/tasks.md)
- Trace row — `REQ-V5-LATENCY-SLIPPAGE-001` in [`_bmad-output/planning-artifacts/trace.toml`](../../../../_bmad-output/planning-artifacts/trace.toml)
- Pattern reference for e2e divergence test —
  `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`
- Cross-feature precedent (audit additive variant) — ADR-0042
  cockpit-activity-broadcast § D4
- Cross-feature precedent (deterministic RNG sub-stream) —
  `crates/forecast/src/tcn.rs` uses the same `ChaCha20Rng` keying
  pattern for training reproducibility
- CLAUDE.md non-negotiable — every overlay or sizing-modifier ships
  with a baseline-equity-divergence e2e test from day 1; pattern
  reference `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`

## Changelog

- 2026-05-26 (architect, post-analyst): ADR-0043 authored. 5 sub-
  decisions (D1-D5) locked from feature.md § R1-R7 + Q1-Q5 analyst-
  recommended defaults. Operator brief suggested ADR-0040 numbering
  but 0040-0042 are already allocated; this ADR lands at the next
  free slot (0043).
- 2026-05-27 (tester, M-FINAL): Deviation 3 amendment — D2 RNG
  implementation. The ADR draft specified `ChaCha20Rng::from_seed()`
  for latency sub-stream derivation. Developer benchmarks showed
  ChaCha20Rng init at ~400-600 ns/order (8-12x over the ≤50 ns R7
  target). Implemented Murmur3-style bit mixer instead: XOR-combines
  the 32-byte scenario seed (as 4 u64 words) with the order_id, runs
  two rounds of Murmur3 finalizer constants. Result: 2.28-2.50 ns/call
  (20x under target). Determinism contract preserved — same
  `(scenario_seed, order_id)` always produces the same latency value.
  The `latency_rng_for_order` function retaining the `ChaCha20Rng` API
  is available for future multi-sample use cases. Tester verdict:
  ACCEPT. ADR D2 intent (deterministic sub-stream keyed on
  scenario_seed + order_id) is fully satisfied; mixer implementation
  detail is an optimization below the ADR abstraction boundary.
- 2026-05-29 (architect, v0.5.0 M-T1): **D3 amendment — square-root
  market-impact model lands**. Closes D3's deferred `_notional`
  parameter promise. The linear-bps quote `cost = bps · price` upgrades
  to `cost = α · √(Q/V) · 10_000 [bps]` per Almgren & Chriss 2001 +
  Kissell 2014 ch. 3 § "Volume-based impact" (the production-grade
  proxy when L2 depth is unavailable). Locked at:
  - **Model shape**: `SlippageModel` enum in `crates/cost/src/slippage.rs`
    with two variants — `Linear { bps: u32 }` (backward-compat;
    `Default::default()` preserves R-NR.2 byte-identity for the 19
    v0.4.0 friction-real anchor SHAs under `v5-realdata-medium-2026-05`)
    and `SquareRoot { alpha: Decimal, volume_lookback_days: u16 }`.
  - **Operator-locked defaults** (M-OD 2026-05-29 Q1/Q2): α = 1.0
    (Kissell midpoint), volume_lookback_days = 90 (Binance parquet
    trailing daily volume × close; revision-pinned via
    `data/binance/REVISION.toml` SHA `3a8b96…bfc7`).
  - **f64 conversion boundary** (K2 falsifier): one site in
    `apply_slippage_sqrt`. `Decimal::to_f64()` for Q, V, α →
    `f64::sqrt()` (IEEE-754 correctly rounded; AArch64 `fsqrt` on the
    Apple Silicon canonical box) → multiply by α × 10_000 →
    `f64::round_ties_even()` (banker's rounding) → saturating cast to
    `u32` clamped at `MAX_SLIPPAGE_BPS = 1_000`. Back to Decimal for
    the sign × multiplier step (reuses the existing Linear branch
    body). Determinism contract: bit-stable across Apple Silicon
    canonical-box runs (no GPU shader codegen path; this is scalar
    f64 sqrt — different from the v2.5 TCN Metal-vs-CPU precedent).
  - **MAX_SLIPPAGE_BPS = 1_000 (10%)**: fat-tail guard for thin-
    liquidity hours (K3). Operator-override path at M-OD if dry runs
    surface > 5% saturation.
  - **Per-asset V retrieval** (R3 lock): Option A — extend `crates/data`
    with `daily_volume_usd_trailing(parquet_root, symbol, end_date,
    lookback_days)` query. Pure function of `(parquet revision SHA,
    symbol, end_date, lookback)`; cached in-process. No new on-disk
    artifact (Option B `volume_proxy.toml` rejected — would risk
    silent drift from the parquet revision pin).
  - **Synthetic-scenario behavior (Q3 OPERATOR OVERRIDE 2026-05-29)**:
    operator selected (b) MIXED — universe-avg V on synthetic, overriding
    analyst-recommended (a) Linear fallback. The 9 synthetic-data
    scenarios (Group A SMA/Composed × 5, Group D TCN-synthetic × 2,
    Group E TCN-weights × 2) compute V via
    `universe_avg_daily_volume_usd_trailing` (arithmetic mean across
    the 10-USDT-pair Binance universe at the scenario's end_date with
    90-day lookback). **By-design SHA divergence**: the 9 synthetic-
    sqrt rows in `v5-sqrt-impact-2026-05` will NOT be byte-identical
    to their `v5-realdata-medium-2026-05` linear-bps twins.
    Operator-accepted v0.6.0 sub-namespace cleanup commitment (see
    feature.md § D-T1.5 — v0.6.0 will either split `v5-sqrt-impact-2026-05`
    into `realdata` + `synthetic` sub-namespaces or retire the 9
    synthetic-sqrt SHAs and consolidate around 10 real-data sqrt rows
    + 9 linear-synthetic rows).
  - **Namespace cascade** (extends ADR-0045 D2 twin pattern): 71 → 90
    anchors additive. New namespace `v5-sqrt-impact-2026-05` joins
    `noop-baseline` and `v5-realdata-medium-2026-05` as the third
    canonical namespace. **`v5-realdata-medium-2026-05` is now
    permanently the linear-bps oracle** (mirrors how `noop-baseline`
    is the frictionless oracle); the Sharpe-delta table R5 surfaces
    both twin diffs in one 3-column view (noop / linear-bps /
    sqrt-impact).
  - **t1937 namespace-aware resolver extension** (extends ADR-0047 D3):
    `Namespace::SqrtImpact` joins `Noop` + `Canonical`. New
    `SQRT_IMPACT_FEATURE_DIRS` slice + `SQRT_IMPACT_STRATEGY_ANCHORS`
    constant table. New test `t1937c_sqrt_impact_strategy_anchors_unchanged`
    mirrors the `t1937b` precedent verbatim. The Noop predicate now
    excludes paths matching SQRT_IMPACT_FEATURE_DIRS as well — load-
    bearing for R-NR.3 (51 noop SHAs must not alias to sqrt reports).
  - **Forward-compat trail closed**: D3's "Future: v0.2.0 may swap in
    square-root. The signature already includes `notional` as an
    unused parameter to make that swap a one-function-body change
    without rippling through call sites." — v0.5.0 ships exactly that
    swap. `apply_slippage(signal_price, side, _notional, bps)` is
    rewritten to a model-dispatching variant
    `apply_slippage_dispatch(signal_price, side, notional, v_daily_usd,
    model)` that routes to `apply_slippage_linear` or
    `apply_slippage_sqrt`. The `_notional` parameter is now LOAD-
    BEARING for the SquareRoot branch.
  ADR decision rationale (architect M-T1): chose to **amend the §
  Changelog** rather than author a new ADR-0050 because the square-
  root model is the **completion of D3's own forward-looking contract**,
  not a sibling decoupled decision. The Q1/Q2 operator defaults +
  the Q3 override all live within D3's "1-parameter model" abstraction
  — these are amendments, not a fork. Mirrors the 2026-05-27 Murmur3
  D2 amendment precedent (sub-ADR-abstraction implementation upgrade).
- 2026-06-08 (architect, doc-hygiene): § D2 body now carries a one-line
  canonical note that the shipped RNG is the Murmur3-style bit mixer
  (`crates/exec/src/latency.rs`), NOT the `ChaCha20Rng` sketch in the body —
  reconciling the body to the 2026-05-27 M-FINAL deviation already recorded
  above (audit-2026-06-08 SC-A). Code verified: `latency.rs` runs a Murmur3
  finalizer and explicitly avoids ChaCha20. No code or fence changed.
