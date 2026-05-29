---
slug: v5-latency-slippage-sim-v0.5.0-square-root-market-impact
version: 0.1.0
status: draft
owner: analyst
updated: 2026-05-29
predecessor: v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit v0.1.0
parent: backtest-vs-live-execution-gap
priority: P1
---

# v5 latency-slippage-sim v0.5.0 — square-root market-impact model

> Closes the v0.1.0 ADR-0043 § D3 **promise**: "linear bps slippage at v0.1.0;
> defer square-root market impact to v0.2.0+". Upgrades the linear-bps slippage
> model to the academic-canonical square-root market-impact model
> `cost = α · σ · √(Q/V)` (Almgren & Chriss 2001; Kissell 2014) — the
> post-Kyle-1985 industry-standard quote for real-world impact. Re-emits all
> 19 currently-friction-real anchored scenarios under a parallel namespace
> `v5-sqrt-impact-2026-05` so the linear-bps namespace `v5-realdata-medium-2026-05`
> remains a friction-comparison oracle. Anchor count 71 → 90 (additive); both
> models co-exist as namespace twins (mirrors the ADR-0045 D2 noop-vs-canonical
> co-existence pattern).

## Why now

v5 v0.1.0 (ADR-0043 § D3) explicitly noted: "Square-root market impact
(`impact ∝ sqrt(notional / depth)`) needs an order-book depth estimate
that isn't available in our Parquet bar data at v0.1.0. Adding it requires
an order-book ingest module — out of scope." That premise has eroded:

1. **Daily-volume proxy is already on disk.** The Binance parquet feed
   (`data/binance/<SYMBOL>/<YEAR>/`) carries `volume` per OHLCV bar. A
   90-day trailing average daily volume is a deterministic, no-new-data
   surrogate for the order-book depth `V` term (Kissell 2014, ch. 3
   § "Volume-based impact" — the production-grade approximation when
   L2 depth is unavailable).
2. **Linear bps overstates low-turnover drag and understates
   high-turnover drag.** v0.4.0 § Wave C reported TCN-realdata Δ Equity
   ≈ $36.5k per scenario at 6,203 fills under linear 8 bps — a flat
   per-fill cost that has no liquidity feedback. The square-root model
   penalizes large fills relative to daily volume, surfacing the
   real-world failure mode: a backtest signal that prints high-frequency
   trades at high cost may show false alpha under linear-bps and true
   negative alpha under square-root.
3. **v5 anchor migration arc is nominally closed** but the friction
   model itself is still the academic baseline rejected since 2001.
   v0.5.0 is the model-quality upgrade the operator pre-deferred at
   v0.1.0; the durable-over-quick contract (AGENT.md 2026-05-29) makes
   this the right time.

## Scope (v0.5.0)

### R1 — Square-root market-impact model

Replace the `cost = bps · price` linear quote with the Almgren-Chriss
square-root form. Default formula:

```text
slippage_bps_effective = α · √(Q / V) · 10_000
fill_price             = signal_price · (1 ± slippage_bps_effective / 10_000)
```

where:

- `α` is the **impact coefficient** (operator-decide Q1 below; literature
  range 0.5–1.5; analyst-recommended `α = 1.0` per Kissell 2014 midpoint).
- `Q` is the fill notional in USD (`abs(qty) · fill_price`).
- `V` is the asset's daily-volume proxy in USD, sourced from the Binance
  parquet feed as a 90-day trailing average of `volume · close` (analyst-
  recommended Q2 below).
- A **volatility factor** is folded into `α` at v0.5.0 (constant per
  asset, not time-varying) to keep the formula 1-parameter. Time-varying
  σ is a v0.6.0+ refinement deliberately deferred.

**Numerical-precision contract** (load-bearing for K2 falsifier): the
`√` operation does not exist on `rust_decimal::Decimal`. Architect M-T1
locks the conversion boundary — proposal: compute `Q/V` and `√` in
`f64`, convert to `Decimal` at the slippage-bps boundary, apply the
sign × `(1 + bps/10_000)` multiplier in `Decimal` (preserves the
existing R3 D3 contract that fill prices are `Decimal`). This mirrors
the ADR-0043 § D2 Murmur3 amendment precedent — a sub-ADR-abstraction
implementation detail.

**Slippage cap** (K3 falsifier): cap `slippage_bps_effective` at
`MAX_SLIPPAGE_BPS = 1_000` (10% — fat-tail guard for thin-liquidity
hours). The cap is operator-decide-eligible at M-OD if the v0.5.0 dry
runs surface frequent saturation; analyst-recommended default is to
keep the cap as a safety rail and let dry runs falsify it.

### R2 — Plumb through the per-path contract (ADR-0047 D2)

Replace the `slippage_bps: u16` field on `LatencySlippageSimConfig` with
a `slippage_model: SlippageModel` enum:

```text
enum SlippageModel {
    Linear { bps: u32 },                                  // backward-compat
    SquareRoot { alpha: Decimal, volume_lookback_days: u16 },
}
```

The plumbing path is identical to ADR-0047 D2 — the same 7
`ScenarioInput` structs that already carry `LatencySlippageSimConfig`
auto-inherit the new field shape. The shared helper
`crates/backtest/src/scenarios/sim.rs::sim_slippage_cost` (ADR-0047 D2
SOLE-LOCATION grep-gate) dispatches on the enum and routes to the
appropriate model.

**Backward-compat**: `SlippageModel::Linear { bps: 8 }` reproduces the
v0.4.0 canonical config byte-identically. Existing serialized configs
that use the old `slippage_bps: u16` field deserialize via a serde
adapter into `Linear { bps }` — preserves the 19 v0.4.0 friction-real
anchor SHAs (R-NR.1 below).

### R3 — Per-asset daily-volume retrieval

The 90-day trailing volume proxy is computed at scenario load time and
cached per `(symbol, date_window)` tuple. Architect M-T1 picks the
retrieval shape — proposal directions:

- **Option A**: extend `crates/data` with a `DailyVolume` query that
  walks the existing parquet feed (no new schema, no new disk artifact).
- **Option B**: bake a `volume_proxy.toml` aggregate into the
  `data/binance/REVISION.toml` companion at brief time (one-shot
  computation, frozen across runs).

Analyst lean: Option A — deterministic + no new on-disk artifact + the
parquet feed is already revision-pinned (`REVISION.toml` SHA
`3a8b96…bfc7` per v0.4.0 K2 architect check). Final architect decision
locks at M-T1.

### R4 — Re-emit all 19 v0.4.0 friction-real anchored scenarios

Under new namespace `v5-sqrt-impact-2026-05`. The 19-scenario inventory
inherits from v0.4.0 Wave C (8 candle/realdata-gated + 11 already-
friction-real at v0.3.0):

| Group | Scenarios | Source data | Feature flag |
|---|---|---|---|
| Group A — SMA/Composed (Q1=a force-synthetic at v0.3.0) | 5 | synthetic GBM | — |
| Group B — momentum × 2 (top10-2023/2024) | 2 | real Binance | realdata |
| Group C — Pairs zscore-mr × 2 | 2 | real Binance | realdata |
| Group D — TCN-overlay synthetic × 2 | 2 | synthetic GBM | candle |
| Group E — TCN-weights × 2 (top10-2023/2024 candle) | 2 | synthetic GBM | candle |
| Group F — TCN-realdata × 2 (top10-2023/2024 realdata) | 2 | real Binance | realdata |
| Group G — TCN-weights-realdata × 2 | 2 | real Binance | candle+realdata |
| Group H — PatchTST-realdata × 1 (top10-2023) | 1 | real Binance | candle+realdata |
| Group I — VolTarget-GARCH-realdata × 1 (top10-2023) | 1 | real Binance | realdata |

Emit reports to `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/backtest-<YYYYMMDD>-<HHMMSS>-<scenario>.md`.
Determinism gate: 2 independent runs per scenario MUST produce byte-
identical body-SHAs (mirrors v0.4.0 Wave A T-D-N3 gate; compound risk is
candle × realdata × friction × square-root — novel at v0.5.0).

### R5 — Sharpe-delta comparison addendum (linear-bps vs square-root)

Author `reports/sharpe-delta-table-2026-05-<DD>.md` with three columns
per scenario: noop / linear-bps (v0.4.0 SHA) / square-root (v0.5.0 SHA).
Surfaces:

- **H1 falsifier** — square-root drag on TCN-realdata ≥ 2× linear-bps
  drag (high-turnover, ~6.2k fills, high-volume bursts).
- **H2 falsifier** — square-root drag on Pairs zscore-mr + VolTarget-GARCH
  ≈ linear-bps drag (low-turnover; ≤ 200 fills typical).
- **K1 surprise scan** — any scenario where `sharpe(square-root) < 0 ∧
  sharpe(linear-bps) > 0` is a model-sensitivity surprise; the v0.4.0
  H3 generalization (0 K1 across 19 scenarios) gets tested under the
  more conservative model. Per-scenario flag and operator review
  inherit the ADR-0045 D3 contract.

### R-NR — Non-regression contract

- **R-NR.1** — `bash scripts/verify_anchors.sh` reports `ANCHORS PASS
  (90 / 90)` post-R2 (additive: 71 → 90, 19 new rows under
  `v5-sqrt-impact-2026-05`; the existing 71 stay byte-identical).
- **R-NR.2** — All 19 v0.4.0 canonical SHAs (under namespace
  `v5-realdata-medium-2026-05`) stay byte-identical — backward-compat
  serde adapter discharges this (R2).
- **R-NR.3** — All 51 noop-baseline SHAs stay byte-identical (additive
  contract per ADR-0038 § D6.a).
- **R-NR.4** — `crates/exec/` and `crates/audit/` library code is NOT
  touched (mirrors v0.4.0 R-NR.5). `crates/cost/src/slippage.rs` is
  the SOLE library impact site for the model body. `crates/backtest/`
  config + plumbing only.
- **R-NR.5** — `crates/strategy/tests/latency_slippage_sim_e2e.rs` +
  `vol_targeting_overlay_end_to_end.rs` + `vol_killswitch_overlay_end_to_end.rs`
  continue to PASS at ≥ 1 bp divergence under BOTH linear-bps and
  square-root configs (the e2e gate is config-agnostic; CLAUDE.md
  non-negotiable).
- **R-NR.6** — `t1937` + `t1937b` `STRATEGY_ANCHORS` /
  `CANONICAL_STRATEGY_ANCHORS` tables stay byte-identical; new
  `SQRT_IMPACT_STRATEGY_ANCHORS` table added per ADR-0047 D3
  namespace-aware resolver pattern (third namespace lands here).
- **R-NR-UI** — Zero UI surface change at v0.5.0. Pure backend.

## K — Risk register / falsifiers

| K | Risk | Mitigation |
|---|---|---|
| **K1** | **Per-asset volume proxy not available for synthetic scenarios** (Group A SMA/Composed at force-synthetic, Group D TCN-overlay synthetic, Group E TCN-weights synthetic). Synthetic GBM has no "real-world" daily volume. | Fall back to `SlippageModel::Linear { bps: 8 }` for synthetic-data scenarios with an explicit log line `slippage_model=Linear (fallback: synthetic data has no V proxy)`. Documents that 9 of 19 scenarios will report linear-bps SHAs under both v0.4.0 and v0.5.0 namespaces (byte-identical → R-NR.2 trivially holds for those 9). The remaining 10 real-Binance scenarios carry the real model. Operator may override at M-OD to use a universe-average synthetic V (Q3 below) — analyst-recommended against. |
| **K2** | **Numerical precision** — `√` over `rust_decimal::Decimal` has no closed form; f64 conversion boundary risks non-determinism across architectures (the v2.5 TCN Metal-CPU-drift precedent). | Architect M-T1 locks the f64-boundary contract: compute `√(Q/V)` in f64; round to nearest 1 bps as `u32`; convert back to `Decimal` for the sign × multiplier step. Cross-architecture determinism gate runs on the same Apple Silicon canonical box as v0.4.0. 2-run byte-identity required for tester PASS. |
| **K3** | **Square-root model amplifies fills at low-volume hours**: at 03:00 UTC `Q/V` may approach 0.01-0.1 → `bps = 100·sqrt(0.01) = 1_000`; at extreme thin-liquidity 0.5-1.0 → `bps = 7_000-10_000` (70-100%). Unrealistic. | Cap `slippage_bps_effective` at `MAX_SLIPPAGE_BPS = 1_000` (10%); log saturation events. If dry runs show > 5% of fills hitting the cap, route back to architect for refined V proxy (e.g. hourly volume vs daily, or volume-adjusted bar windows). |
| **K4** | **Anchor cascade error rate** — re-emitting 19 reports × 2-run determinism × candle/realdata feature-flag matrix is the largest re-emission since v0.4.0. K4 from v0.4.0 (compound determinism) now also includes the new f64 conversion boundary. | Architect M-T1 surfaces the per-asset volume retrieval shape (R3) as the load-bearing pre-condition. Tester M-FINAL: 2-run byte-identity on all 19 SHAs (mirrors v0.4.0 T-D-N3 gate verbatim). If K4 trips, route back to architect for f64-boundary audit. |

## H — Hypotheses

| H | Hypothesis | Confidence | Falsifier |
|---|---|---|---|
| **H1** | Square-root impact increases Sharpe drag on high-turnover scenarios (TCN-realdata, PatchTST-realdata) by ≥ 2× vs linear-bps. v0.4.0 TCN-realdata at $36.5k Δ Equity under linear should reach ≥ $73k under square-root because the model penalizes the ~6.2k fills × variable Q/V more harshly than the flat 8 bps. | Medium-high | R5 delta table per scenario. If TCN-realdata Δ Equity (square-root vs linear-bps) < 1.5× the linear-bps drag, H1 falsified — square-root model under-penalizes the real-Binance high-turnover regime; investigate α calibration. |
| **H2** | Square-root impact has minimal incremental effect on low-turnover scenarios (Pairs zscore-mr, VolTarget-GARCH overlay; ≤ 200 fills typical). | Medium-high | R5 delta table for Pairs + VolTarget rows. If Δ between models > 30% vs linear-bps absolute drag, H2 falsified — low-turnover paths are MORE model-sensitive than expected; revisit Q/V averaging window. |
| **H3** | Model produces deterministic byte-identical outputs across 2-run gate (R-NR + ADR-0047 D2 contract; same Apple Silicon canonical box as v0.4.0). | Medium | R4 determinism gate; mirrors v0.4.0 T-D-N3. If K4 trips, the f64 conversion boundary is the suspect (architect M-T1 root-cause). |

## Operator-decide questions (Q1-Q3)

| Q | Topic | Options | Analyst-recommended default | Rationale |
|---|---|---|---|---|
| **Q1** | **Impact coefficient α** | (a) **`α = 1.0`** — academic-canonical Kissell 2014 midpoint, well-calibrated to retail-venue real-world impact; future tuning lives in v0.5.1 calibration brief if needed / (b) `α = 0.5` — toward linear; less drag — doesn't capture market-impact realism; defeats the v0.5.0 purpose | **(a) `α = 1.0` (Recommended — DURABLE)** | (a) cost: ~3-5d dev + 1d tester + 0.5d presenter for the proper one-shot model upgrade. (b) cost: ~1d now + future v0.5.1 calibration brief (~3-5d) when the operator observes under-drag = net STRICTLY WORSE wall-clock + repeat cognitive context-swap. Mirrors the v0.1.3 helper-extraction pattern (durable now beats cheap-and-redo later). Academic citations: Almgren & Chriss 2001 § "Optimal execution of portfolio transactions", Kissell 2014 ch. 3 "The market impact model". |
| **Q2** | **Per-asset volume V source** | (a) **90-day trailing daily volume from existing Binance parquet** — already on disk; no new data source; deterministic; revision-pinned via `data/binance/REVISION.toml` / (b) hardcoded universe-average constant — fragile; future v0.6.0+ cleanup brief required to migrate to per-asset | **(a) Binance parquet 90-day trailing (Recommended — DURABLE)** | (a) cost: ~0.5d architect + reuses revision-pinned source = no new K-line in determinism gate. (b) cost: ~0.25d now + ~2-3d v0.6.0 cleanup brief when operator notices BTC and DOGE have identical impact (false equivalence) = net STRICTLY WORSE. Academic citation: Kissell 2014 ch. 3 § "Volume-based impact" — the canonical proxy when L2 depth is unavailable. |
| **Q3** | **Synthetic-scenario behavior** | (a) **synthetic scenarios fall back to `Linear { bps: 8 }` (no V proxy available)** with explicit log message; their v0.5.0 SHAs are byte-identical to v0.4.0 — additive namespace stays clean / (b) use universe-average V from real data — confuses synthetic-scenario semantics; H3 alpha-comparability degrades | **(a) Linear fallback for synthetic (Recommended — DURABLE)** | (a) preserves the v0.4.0 Group A/D/E SHAs as oracles in BOTH namespaces (9 of 19 SHAs trivially byte-identical → R-NR.2 simplified). Clean semantic separation: square-root is a real-data model. (b) muddies the namespace twin contract; v0.6.0 would need a "real vs synthetic" sub-namespace split. Net STRICTLY WORSE. |

**Cost framing — both routes:**

- **Q1+Q2+Q3 = (a)(a)(a) DURABLE** (analyst-recommended): ~3-5 days dev
  (R1 model + R2 enum plumbing + R3 volume retrieval) + ~1 day tester
  (M-FINAL with 19-scenario re-emission + 2-run determinism) + ~0.5 day
  presenter = **~1 week wall-clock**. Closes the v0.1.0 ADR-0043 § D3
  promise with no follow-on briefs queued.
- **Q1+Q2+Q3 = (b)(b)(b) cheap fallback**: ~1 day now + v0.5.1
  calibration brief (~3-5d) + v0.6.0 V-source replacement (~2-3d)
  + v0.6.1 namespace cleanup (~1-2d) = **~3 weeks wall-clock + 3
  re-spawn cycles**. Per the AGENT.md 2026-05-29 durable-over-quick
  contract: STRICTLY WORSE.

## Pre-drawn 2-cell verdict tree (presenter inherits)

| Cell | Condition | Route |
|---|---|---|
| **R-O1** | 19/19 R4 re-emissions succeed + 2-run determinism gate PASS + R5 delta table confirms H1 directional (≥ 2× drag on high-turnover) and H2 directional (≤ 30% delta on low-turnover) + 0 K1 surprises + R-NR.1-6 all green | **SHIP** v0.5.0 + close the ADR-0043 § D3 promise. v5 anchor-migration arc end-to-end now spans v0.1.0 → v0.2.0 → v0.3.0 → v0.4.0 → v0.5.0 = engine + canonical config + per-path wiring + candle/realdata coverage + model-quality upgrade. No further v5 follow-ons unless operator requests intrabar-fill-sampling (pre-deferred from ADR-0043 § Alternatives Rejected). |
| **R-O2** | K2 / K3 / K4 trips OR > 0 K1 surprises (H1 or H2 falsified) | **REGRESSION** — route back to architect for root-cause. Possible outcomes: f64-boundary refinement (K2/K4), α re-calibration brief v0.5.1 (K1 surprise without K2/K3), MAX_SLIPPAGE_BPS lowering (K3 saturation > 5%), or hourly V refinement (K3 thin-liquidity-hours fix). |

## Predecessor / parent chain

- **Parent**: backtest-vs-live execution gap (long-running theme; cited
  in `spec/product.md § Strategy lifecycle`).
- **Predecessor**: `v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit v0.1.0`
  (shipped 2026-05-28). v0.4.0 closed the candle/realdata coverage gap;
  v0.5.0 closes the model-quality gap.
- **Grandparents**: `v5-latency-slippage-sim-v0.3.0-full-path-wiring`
  (per-path plumbing; ADR-0047) + `v5-latency-slippage-sim-v0.2.0-anchor-migration`
  (canonical config; ADR-0045) + `v5-latency-slippage-sim v0.1.0`
  (engine; ADR-0043).
- **Successor (probable)**: none auto-spawned. Operator may request
  intrabar-fill-sampling (pre-deferred from ADR-0043 § Alternatives
  Rejected) or a v0.5.1 calibration brief if K1 surprises require α
  retune.

## Cross-references

- v0.4.0 brief (predecessor) — [`spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/feature.md`](../v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/feature.md)
- v0.4.0 Sharpe-delta table (the template R5 extends) — [`spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/sharpe-delta-table-2026-05-28.md`](../v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/sharpe-delta-table-2026-05-28.md)
- ADR-0043 § D3 (engine ADR — the deferred-to-v0.2.0+ promise this brief closes) — [`spec/architecture/adr/0043-simulated-latency-and-slippage.md`](../architecture/adr/0043-simulated-latency-and-slippage.md)
- ADR-0045 (canonical config + namespace co-existence pattern v0.5.0 mirrors) — [`spec/architecture/adr/0045-v5-canonical-config-and-noop-baseline-namespace.md`](../architecture/adr/0045-v5-canonical-config-and-noop-baseline-namespace.md)
- ADR-0047 (per-path wiring contract — R2 reuses verbatim; namespace-aware resolver pattern R-NR.6 extends to a third namespace) — [`spec/architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md`](../architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md)
- Current linear-bps impl (R1 replaces) — `crates/cost/src/slippage.rs`
- Per-path SOLE-LOCATION helper (R2 dispatches on enum) — `crates/backtest/src/scenarios/sim.rs`
- Binance parquet feed (R3 volume source) — `data/binance/<SYMBOL>/<YEAR>/`; revision-pinned at `data/binance/REVISION.toml` SHA `3a8b96…bfc7`
- Anchors file (additive 71 → 90 under new namespace) — [`spec/anchors.toml`](../anchors.toml)
- Tasks — [`tasks.md`](tasks.md)
- Trace row — `REQ-V5-LATENCY-SLIPPAGE-V0-5-0-001` in [`spec/trace.toml`](../trace.toml)

## Academic citations

- Almgren, R. & Chriss, N. (2001). "Optimal execution of portfolio
  transactions." *Journal of Risk* 3(2): 5–39. The canonical source
  for the square-root impact form `cost = α · σ · √(Q/V)` in the
  context of optimal liquidation.
- Kissell, R. (2014). *The science of algorithmic trading and portfolio
  management*. Academic Press. Chapter 3 "The market impact model"
  ratifies the production-grade volume-proxy variant `α · √(Q/V)`
  when L2 depth is unavailable — the configuration v0.5.0 ships.
- Kyle, A. (1985). "Continuous auctions and insider trading."
  *Econometrica* 53(6): 1315–1335. The pre-1985 linear-bps model is
  the baseline the post-Kyle literature replaced.

## Design

_Architect M-T1 fills this — locks numerical-precision contract (K2),
per-asset volume retrieval shape (R3 Option A vs B), MAX_SLIPPAGE_BPS
cap (K3), and ADR amendment vs new ADR decision._

## Implementation

_Developer fills this._

## Verification

_Tester M-FINAL links to reports here._

## Changelog

- 2026-05-29 (analyst): feature.md v0.1.0 authored. **5 R / R-NR / 4 K /
  3 H / 3 Q** + pre-drawn 2-cell verdict tree + cost framing both
  routes. Closes the ADR-0043 § D3 deferred promise. Q1+Q2+Q3 all
  default to durable per AGENT.md 2026-05-29 contract; cheap fallbacks
  are STRICTLY WORSE wall-clock + repeat cognitive context-swap cost.
  Namespace cascade: 71 → 90 anchors additive under new
  `v5-sqrt-impact-2026-05` pin (mirrors ADR-0045 D2 noop-vs-canonical
  twin pattern; preserves linear-bps namespace as comparison oracle).
  M-T1 architect picks the per-asset volume retrieval shape + f64
  conversion boundary contract. HANDOFF → architect.
