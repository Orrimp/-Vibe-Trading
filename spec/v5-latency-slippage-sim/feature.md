---
slug: v5-latency-slippage-sim
version: 0.1.0
status: in-progress
owner: tester
updated: 2026-05-26
predecessor: cockpit-activity-status-bar v0.1.0 (no direct dep — parallel feature track)
parent: backtest-vs-live-execution-gap
priority: P1
---

# Deterministic Latency & Slippage Simulation — close the backtest-vs-live gap

## Why now

Current backtest model assumes:

- **Immediate fill at the bar's close** — `MatchingEngine` in
  `crates/exec` produces fills whose `(price, ts_ms)` are deterministic
  functions of the bar + the order, with **zero** network latency.
- **Zero order-book slippage** — fills land at the bar-recorded price
  with no walk-the-book cost beyond the existing taker-fee model in
  `crates/cost`.

Both are systematically optimistic. Real venues impose 20-100 ms wire
latency typically; market orders walk the book and fill worse than mid.
A backtest that ignores these two frictions **overestimates strategy
alpha** — the well-known "backtest-vs-live gap" that kills paper-to-live
transitions.

This brief introduces **deterministic, optional, default-zero**
simulation of both frictions. Default config is **noop** (latency=0,
slippage=0 bps), so all 34 currently-anchored backtest reports stay
byte-identical until an explicit operator-approved migration at v0.2.0.

## Scope (v0.1.0)

### R1 — Configuration toggle (default-zero noop) [CRITICAL]

Introduce `LatencySlippageSimConfig`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatencySlippageSimConfig {
    /// Latency jitter range in milliseconds. Default: 0..=0 (noop).
    /// When equal (e.g. 50..=50) → fixed delay; when range → uniform jitter.
    pub latency_ms_min: u64,
    pub latency_ms_max: u64,
    /// Linear slippage in basis points. Default: 0 bps (noop).
    /// Sign-applied per `Side`: Buy → fill_price * (1 + bps/10_000);
    /// Sell → fill_price * (1 - bps/10_000).
    pub slippage_bps: u32,
}

impl Default for LatencySlippageSimConfig {
    fn default() -> Self {
        Self { latency_ms_min: 0, latency_ms_max: 0, slippage_bps: 0 }
    }
}
```

`ScenarioConfig` gains a non-optional field `latency_slippage_sim:
LatencySlippageSimConfig` (default via `..Default::default()` on
`ScenarioConfig`). All existing call sites (the 34 anchored scenarios)
construct `ScenarioConfig` without the field → default zeros applied →
byte-identical output.

**Acceptance**: `bash scripts/verify_anchors.sh` exits 0 (34/34 PASS)
post-Wave-A. This is the non-negotiable gate.

### R2 — Latency simulation in `crates/exec`

`MatchingEngine` gains:

```rust
fn apply_latency(
    order_ts_ms: i64,
    cfg: &LatencySlippageSimConfig,
    rng: &mut ChaCha20Rng,
) -> i64;
```

- At zero range (`min==max==0`): returns `order_ts_ms` unchanged
  (byte-identical).
- At fixed delay (`min==max==N`): returns `order_ts_ms + N`.
- At jitter range (`min<max`): uniform-sample `delta ∈ [min, max]` from
  the seeded RNG; returns `order_ts_ms + delta`.

**Determinism contract**: the RNG is a *sub-stream* of the scenario's
existing `ChaCha20Rng`, keyed on `(scenario_seed, order_id)`. NO
wall-clock, NO `thread_rng()`. Reproducible across runs.

### R3 — Slippage simulation in `crates/cost`

New function:

```rust
pub fn apply_slippage(
    signal_price: Decimal,
    side: Side,
    notional: Decimal,
    bps: u32,
) -> Decimal;
```

Linear bps model (D3 in ADR-0043):
- `Side::Buy`: `fill_price = signal_price * (1 + bps / 10_000)`
- `Side::Sell`: `fill_price = signal_price * (1 - bps / 10_000)`
- At `bps == 0`: returns `signal_price` unchanged (byte-identical).

`notional` is included in the signature for future square-root market-
impact extension (Q2; deferred to v0.2.0) but unused at v0.1.0.

### R4 — Audit ledger metric recording

New `AuditEvent` variant in `crates/audit`:

```rust
SimulatedExecMetrics {
    order_id: OrderId,
    fill_id: FillId,
    latency_ms_applied: u64,
    slippage_bps_applied: u32,
    slippage_dollars_applied: Decimal,
},
```

**Skip-when-zero guard** (load-bearing per R-NR.1): the variant is
emitted ONLY when at least one of (latency, slippage) is non-zero.
Otherwise no audit row is written. This preserves byte-identity of
anchored backtest audit ledgers under the noop default.

### R5 — Baseline-equity-divergence e2e test [CLAUDE.md non-negotiable]

NEW `crates/strategy/tests/latency_slippage_sim_e2e.rs` (or similar).
Per the CLAUDE.md non-negotiable:

> Every strategy overlay or sizing-modifier ships with a baseline-
> equity-divergence end-to-end test from day 1.

The test:
1. Run a momentum scenario with `LatencySlippageSimConfig::default()`
   (zeros) → record `baseline_equity` final value.
2. Run the SAME scenario with non-zero config
   (`{ latency_ms_min: 50, latency_ms_max: 100, slippage_bps: 10 }`)
   → record `simulated_equity` final value.
3. Assert `|baseline_equity - simulated_equity| >= 1 bp * baseline_equity`
   (≥ 1 basis-point divergence).

If divergence is < 1 bp, the simulator is a no-op — fail the test loud
per the v3-volatility-forecaster-noop-fix 2026-05-22 precedent
(`spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`).

### R6 — Non-regression contract (R-NR)

R-NR.1 — 34 anchors in `spec/anchors.toml` stay byte-identical at
v0.1.0 ship. Default config = noop. Verified by `bash scripts/
verify_anchors.sh`.

R-NR.2 — `cargo test --workspace --no-fail-fast` shows no new failures
vs the existing whitelist (R8.1 GREEN post-reflection-trader,
Bug #65 closure pending, pre-existing lab_run_engine H3 + paths
flake).

R-NR.3 — No changes to the `Strategy` trait surface. No changes to
the `Signal` type. Both crates `strategy` and `trader` consume the
config via `ScenarioConfig` only.

R-NR.4 — Criterion bench: backtest throughput regression < 1% on the
existing `momentum.rs` 8760-bar scenario. Hot path is the latency-
shift + slippage-multiply on every fill; with zero values, branch
prediction makes the cost effectively free. Verified by new bench
under `crates/exec/benches/`.

R-NR.5 — Zero new ADR amendments (ADR-0038 § D6 wiring-bug-fix
protocol does NOT apply — this is a new feature, not a wiring-bug
fix).

R-NR.6 — Audit-ledger schema stays backward-compat: the new
`SimulatedExecMetrics` variant is additive on the `AuditEvent` enum;
v0 audit replay still parses pre-v0.1.0 ledger rows unchanged.

### R7 — Criterion bench

NEW `crates/exec/benches/latency_slippage.rs` — 3 micro-benches:
1. `apply_latency_noop` — at zero ms; target ≤ 5 ns.
2. `apply_latency_jitter` — at 50..=100 ms; target ≤ 50 ns.
3. `apply_slippage_10bps` — at 10 bps; target ≤ 10 ns.

Plus extend the existing momentum-throughput bench (if one exists)
or add `crates/exec/benches/throughput_with_sim.rs` measuring full-
scenario throughput at noop vs enabled — assert delta < 1% (R-NR.4).

## Operator-decide questions (Q1-Q5)

| Q | Topic | Options | Analyst-recommended default | Rationale |
|---|---|---|---|---|
| Q1 | Latency model | (a) fixed delay / (b) uniform jitter range | **(b) uniform jitter** | Matches user's "20ms-100ms" framing; better real-world fidelity; fixed delay is a degenerate case (min==max) |
| Q2 | Slippage model | (a) linear bps / (b) square-root market impact / (c) bid-ask additive | **(a) linear bps** | (b) needs an order-book depth estimate not available at v0.1; (c) needs bid-ask spread data on bars not available at v0.1; (a) is the academic baseline. Re-decide at v0.2.0 |
| Q3 | Audit row shape | (a) extend `FillMemo` columns / (b) NEW `AuditEvent::SimulatedExecMetrics` variant | **(b) new variant** | (a) mutates an existing schema → migration overhead + v0 audit replay break risk; (b) is purely additive |
| Q4 | Scope: live mode too? | (a) backtest-only / (b) live mode also runs through the simulator | **(a) backtest-only** | Live venues already impose REAL latency + slippage — simulating them would double-count |
| Q5 | Anchor migration timing | (a) defer to v0.2.0 separate brief / (b) bundle into this brief | **(a) defer** | Migrating 34 anchors to non-zero values is a load-bearing operator decision deserving its own brief. v0.1.0 ships the simulator; v0.2.0 decides what enabled values produce the canonical paper-trading reports |

All 5 standing-Autoapprove-eligible at the analyst-recommended defaults.
**Q3 is the most load-bearing** — it locks the audit schema shape;
post-ship migration is expensive.

## K — Risk register

| K | Risk | Mitigation |
|---|---|---|
| K1 | Anchor drift — default config inadvertently produces non-zero output | R-NR.1 hard gate at Wave A end; tester re-runs `verify_anchors.sh` |
| K2 | RNG determinism — wall-clock or `thread_rng()` accidentally introduced | D2 architectural lock: seeded sub-stream only; lint test scans for `thread_rng()` / `SystemTime::now()` in `crates/exec` |
| K3 | Audit volume — every fill writes a new audit row when enabled, flooding the ledger | Skip-when-zero guard (R4); future v0.2.0 may need aggregation per cockpit-activity-audit-ledger-producer precedent |
| K4 | Backtest perf — adding latency/slippage on the hot path slows throughput | R-NR.4 + R7 criterion bench gate < 1% regression |
| K5 | Cross-feature: vol_killswitch_overlay (Bug #65 in flight) — broadened filter touches the executor handoff | Sequence: vol_killswitch fix lands first; this brief's developer rebases after |
| K6 | Cross-feature: cockpit-activity-status-bar — every fill now potentially emits a SimulatedExecMetrics audit event, which the audit-ledger producer (v0.1.1 brief) would surface in the activity tape | Skip-when-zero guard prevents tape flood; v0.1.1 audit-producer's 100ms aggregation handles enabled-state load |
| K7 | Live-mode confusion — operator enables non-zero config and tries to run live, double-counting real latency/slippage | D5 architectural lock: `LatencySlippageSimConfig` is read by `crates/backtest::ScenarioConfig` only; `crates/agent` (live mode) ignores it. Compile-time enforcement via `#[cfg]`-gated or just structural separation |

## H — Hypotheses

| H | Hypothesis | Confidence | Falsifier |
|---|---|---|---|
| H1 | Default-zero config produces byte-identical anchor SHAs vs pre-feature | 95% | Wave A integration test running all 34 anchored scenarios with `Default::default()` config |
| H2 | `{ 50..=100ms, 10bps }` config moves equity ≥ 1 bp on the v1.momentum scenario | High | The e2e divergence test (R5) — if it fails, the simulator is broken; if it passes, H2 confirmed |
| H3 | Audit-ledger schema migration (additive variant) doesn't slow SQLite writes by > 1% | High | New criterion bench `audit_write_with_sim_metrics` extending existing audit-bench |
| H4 | Backtest hot path absorbs the no-op overhead without measurable regression (branch prediction at zero values) | Medium-high | R-NR.4 / R7 criterion gate; rollback plan = `#[inline(always)]` on the noop branch or feature-flag fallback |

## Cost framing

| Phase | Effort |
|---|---|
| Analyst (this pass) | ~0.5 day (done) |
| Operator-decide (Q1-Q5) | ~15 min standing-Autoapprove |
| Architect M-T1 (ADR-0043 + decomp) | ~0.5 day |
| Developer M-DEV (Waves A-E, 5 waves) | ~5-8 days |
| Tester M-FINAL (incl. criterion baselines) | ~1 day |
| Presenter | ~0.5 day |
| **Total** | **~1.5-2 weeks wall-clock** |

## Pre-drawn verdict routing tree (presenter inherits)

| Cell | Condition | Route |
|---|---|---|
| R-O1 | All 5 R rows green + R-NR.1 34/34 anchors + R5 e2e divergence ≥ 1 bp | **SHIP** v0.1.0 + spawn v0.2.0 anchor-migration brief |
| R-O2 | R-NR.4 perf regression 1-3% | **HOLD** — developer tunes the noop branch (PGO / `#[inline(always)]`); re-bench |
| R-O3 | R-NR.4 perf regression > 3% OR e2e divergence < 1 bp | **REGRESSION** — architect re-spawns; possibly D1 falls back to feature-flag (rejected default) |
| R-O4 | 34 anchors drift even with `Default::default()` (R-NR.1 FAIL) | **CRITICAL** — Wave A must be redesigned; HANDOFF → architect, blocks all subsequent waves |

## Predecessor / parent chain

- **Parent**: backtest-vs-live execution gap (long-running theme; cited
  in `spec/product.md § Strategy lifecycle`)
- **Predecessor**: `backtest-real-binance-data v0.1.0` (shipped
  2026-05-18) — locked the parquet revision-pin protocol per ADR-0032
- **Sibling (parallel track)**: `cockpit-activity-status-bar v0.1.0`
  (shipped 2026-05-26) — no direct dep, but K6 surfaces the audit-
  producer interaction
- **Pending sibling**: `vol-killswitch-overlay-noop-fix v0.1.0`
  (Bug #65, Q4=(p3) in flight) — K5 sequencing constraint

## Cross-references

- ADR-0043 — `spec/architecture/adr/0043-simulated-latency-and-slippage.md`
- Tasks — `spec/v5-latency-slippage-sim/tasks.md`
- Trace — `REQ-V5-LATENCY-SLIPPAGE-001` in `spec/trace.toml`
- CLAUDE.md non-negotiable — "every strategy overlay or sizing-modifier
  ships with a baseline-equity-divergence end-to-end test from day 1"
- Pattern reference — `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`
  (the canonical e2e divergence test from the v3-vol-overlay-noop-fix
  2026-05-22 precedent)

## Changelog

- 2026-05-26 (analyst): feature.md v0.1.0 authored. R1-R7 + R-NR.1-6
  + K1-K7 + H1-H4 + Q1-Q5 + verdict tree. Default-zero noop locked
  as the non-negotiable Wave A contract per the operator's directive.
  ADR slot was suggested at 0040 in the operator brief — corrected to
  **ADR-0043** since 0040 is already taken by yahoo-realdata-path
  (0041 = trader-crate-split, 0042 = cockpit-activity-broadcast,
  0043 is the next free number).
- 2026-05-26 (developer): Waves A-E complete. Implementation summary below.

## Implementation

### Developer notes (2026-05-26)

**Waves A-E implemented**. All T-D-N1 through T-D-N10 rows ticked with
evidence. T-D-N11 (workspace test) delegated to tester.

#### Wave A — Configuration toggle (CRITICAL anchor gate)

- `LatencySlippageSimConfig` added to `crates/backtest/src/cli_types.rs:44-64`.
  Derives `Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize`.
  `is_noop()` helper at line 70.
- `ScenarioConfig` gains `latency_slippage_sim: LatencySlippageSimConfig`
  field in `crates/backtest/src/engine.rs`.
- All 9 construction sites updated with `..Default::default()` (5 test sites
  + `main.rs` + `lab/runner.rs` + 2 UI test files).
- **Anchor gate**: 34/34 PASS confirmed.

#### Wave B — Latency simulation

- `crates/exec/src/latency.rs` (new module, 255 lines).
- `apply_latency()` function: noop at zero, fixed delay at min==max,
  Murmur3-based jitter otherwise.
- `latency_u64_for_order()`: XOR-mixes 4 u64 seed words with order_id,
  applies two rounds of Murmur3 finalizer constants (no external hash call).
  Result: `apply_latency_jitter` bench at 2.28 ns (target ≤50 ns — massively
  ahead of target; previous blake3 approach was 61 ns).
- 4 unit tests: noop, fixed, jitter range, determinism.

#### Wave C — Slippage simulation

- `crates/cost/src/slippage.rs` (new module, 148 lines).
- `apply_slippage()`: linear bps, side-signed, noop at bps=0.
- `sim_slippage_cost()` helper in `crates/backtest/src/scenarios/momentum.rs:551`
  deducts slippage from `cash` on every Buy/Sell fill (lines 386-392, 434-440).
- 5 unit tests: noop, buy increases, sell decreases, symmetry, precision.

#### Wave D — Audit-ledger integration

- `AuditEvent::SimulatedExecMetrics` variant added to `crates/audit/src/tick.rs:126`.
- Skip-when-zero guard in tests; actual guard at caller's emit site.
- 3 unit tests: round-trip, skip-when-zero, variant_label.

#### Wave E — e2e + bench + non-regression

- `crates/strategy/tests/latency_slippage_sim_e2e.rs` (228 lines):
  - `noop_byte_identical_to_baseline` — determinism guard.
  - `enabled_diverges_by_at_least_1bp` — FORENSIC GATE (CLAUDE.md non-negotiable).
  - `enabled_audit_metrics_recorded` — skip-when-zero guard semantics.
- `crates/exec/benches/latency_slippage.rs` — 3 micro-benches.
- `crates/exec/benches/throughput_with_sim.rs` — 2 throughput benches.

#### Deviation: `apply_slippage_10bps` bench target

R7 target for `apply_slippage_10bps` was ≤10 ns. Actual: ~19 ns.
Root cause: `rust_decimal` arithmetic requires ~6-10 ns per operation;
the enabled path performs `Decimal::from(bps) / Decimal::from(10_000_u32)`
+ one multiplication = 2-3 Decimal ops = 18-30 ns minimum.
The noop path (bps=0) returns immediately — branch prediction makes it free.
The ≤10 ns target was aspirational and physically impossible with `rust_decimal`.
Decision: document deviation; tester to confirm acceptability.
The noop path (critical for R-NR.4 anchor preservation) is unaffected.

#### RNG architecture deviation (from ADR-0043 § D2 draft)

The ADR draft cited `ChaCha20Rng::from_seed()` for sub-stream derivation.
Benchmarks showed ChaCha20Rng init costs ~400-600 ns per order — an order of
magnitude over the ≤50 ns R7 target. Implemented Murmur3-style bit mixer
instead: XOR-combines the 32-byte seed (as 4 u64 words) with the order_id,
runs two rounds of Murmur3 finalizer constants. Result: 2.28 ns total, 10x
under the target. Quality is sufficient for backtest jitter (cryptographic-
strength distribution not required here). The `ChaCha20Rng`-per-order API
(`latency_rng_for_order`) is retained in the codebase for future multi-sample
use cases.

#### Performance summary

| Bench | Result | Target | Status |
|---|---|---|---|
| `apply_latency_noop` | 1.46 ns | ≤5 ns | PASS |
| `apply_latency_jitter` | 2.28 ns | ≤50 ns | PASS (22x under) |
| `apply_slippage_10bps` | 19 ns | ≤10 ns | MISS (deviation documented) |
| `noop_8760_fills` | 33 µs | baseline | ~3.8 ns/fill |
| `enabled_8760_fills` | 191 µs | opt-in | ~21.8 ns/fill |
