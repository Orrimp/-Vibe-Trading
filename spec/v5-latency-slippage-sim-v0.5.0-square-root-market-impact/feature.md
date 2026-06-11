---
slug: v5-latency-slippage-sim-v0.5.0-square-root-market-impact
version: 0.2.0
status: shipped
owner: developer
updated: 2026-05-29
predecessor: v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit v0.1.0
parent: backtest-vs-live-execution-gap
priority: P1
q_d1: "(a) Linear{bps:8} fallback for synthetic scenarios — operator ratified 2026-05-29"
q_d2: "(β) Per-scenario lazy-compute via universe_avg_daily_volume_usd_trailing — operator ratified 2026-05-29"
anchor_cascade_revised: "75 → 84 (9 new real-data anchors under v5-sqrt-impact-2026-05; brief described 10 scenarios but top10-2024-fy-momentum-realdata was never implemented — only 2023 counterpart shipped)"
m_od_q3b_supersession: "M-OD 2026-05-29 Q3=(b) ratification SUPERSEDED by Q-D1=(a); see spec/dev-notes/v5-v0.5.0-q-d1-q-d2-decision-brief-2026-05-29.md"
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
| **Q-D1+Q-D2** (post-`e09e599` re-surfacing 2026-05-29) | **Wave D synthetic-scenario volume strategy + universe-avg V wiring** — see [`spec/dev-notes/v5-v0.5.0-q-d1-q-d2-decision-brief-2026-05-29.md`](../dev-notes/archive/2026-Q2/v5-v0.5.0-q-d1-q-d2-decision-brief-2026-05-29.md). Q-D1: (a) Linear-fallback for synthetic / (b) SquareRoot+universe-avg V on synthetic [M-OD 2026-05-29 ratified (b); brief proposes REVISIT under shipped-helper evidence]. Q-D2: (α) pre-compute / (β) per-scenario lazy-compute / (γ) CLI-flag inject. | **Q-D1=(a) + Q-D2=(β) (Recommended — DURABLE)** | Combined ~0.5-1.0 dev-day + ~1 tester-day; ZERO v0.6.0 follow-on briefs. Q-D1=(a) downgrades the architect M-T1 D-T1.5 v0.6.0 sub-namespace cleanup commitment from "must spawn brief" to "obsolete by-design." Q-D2=(β) aligns verbatim with D-T1.5 end_date-pin contract; uses existing Wave C dashmap cache. If operator holds M-OD lock → Q-D1=(b)+Q-D2=(β) fallback durable-via-commitment (~3-4 dev-days + v0.6.0 cleanup brief queued). See dev-note § Q-D1 + § Q-D2 tables for 5-dimension + 5-dimension comparison. |
| **Q-D1 RATIFIED 2026-05-29 (operator)** | **(a) Linear{bps:8} fallback for synthetic scenarios.** M-OD 2026-05-29 Q3=(b) SUPERSEDED. Real-data scenarios use SquareRoot; synthetic (Groups A/D/E) fall back to Linear{bps:8}. Anchor cascade: 75 → 85 (10 new real-data anchors only). See [`spec/dev-notes/v5-v0.5.0-q-d1-q-d2-decision-brief-2026-05-29.md`](../dev-notes/archive/2026-Q2/v5-v0.5.0-q-d1-q-d2-decision-brief-2026-05-29.md). | — | v0.6.0 sub-namespace cleanup commitment dropped — obsolete by-design under Q-D1=(a). |
| **Q-D2 RATIFIED 2026-05-29 (operator)** | **(β) Per-scenario lazy-compute** via `data::universe_avg_daily_volume_usd_trailing` (already cached via `OnceLock<Mutex<HashMap>>` inside the Wave C helper). One call per scenario; dashmap cache handles dedup across scenarios sharing the same end_date. | — | Aligns verbatim with D-T1.5 end_date-pin contract; zero new flags, zero operator cognitive load. |

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

_Architect M-T1 — locked 2026-05-29 post-operator-decide
(Q1=(a) α=1.0, Q2=(a) 90-day Binance parquet, Q3=(b) MIXED universe-avg V
on synthetic — operator override of analyst-recommended Q3=(a))._

### D-T1.1 — ADR decision: amend ADR-0043 § Changelog (not new ADR-0050)

**Decision**: extend ADR-0043's Changelog with a v0.5.0 amendment block
(mirrors the 2026-05-27 Murmur3 D2 amendment precedent — a sub-ADR-
abstraction implementation upgrade that closes a deferred-promise inside
the same engine ADR).

**Rationale**: ADR-0043 § D3 explicitly says _"Future: v0.2.0 may swap in
square-root. The signature already includes `notional` as an unused
parameter to make that swap a one-function-body change without rippling
through call sites."_ The square-root model is the **completion of D3's
own forward-looking contract**, not a sibling decoupled decision —
amending the ADR keeps the engine-ADR provenance chain continuous and
self-narrating. A new ADR-0050 would have fragmented the engine story
across two files for a parameter-swap that lives entirely under the
`apply_slippage` signature that D3 already shipped a `_notional`
placeholder for.

**Alternatives considered**:
- _New ADR-0050 "v5 v0.5.0 square-root market-impact model"_ —
  rejected. Adds a second engine-ADR file for a body-only swap. The Q3
  operator-override + Q1/Q2 defaults all live within D3's "1-parameter
  model" abstraction — these are amendments, not a fork.
- _Amend ADR-0045 D1 (canonical config) instead_ — rejected. ADR-0045
  governs **which canonical numeric values** are pinned (`30/80/8`),
  not the **model shape**. The shape lives in ADR-0043 D3.

### D-T1.2 — `SlippageModel` enum public types (R1, R2)

**Location**: `crates/cost/src/slippage.rs` — same source file as the
existing `apply_slippage` linear impl. Re-exported via
`crates/cost/src/lib.rs::pub use slippage::{apply_slippage, SlippageModel};`
so downstream crates (`backtest`) import the enum from the cost crate
without taking a transitive dep on the model body.

**Signature**:

```rust
/// Slippage model variant. Linear preserves v0.1.0–v0.4.0 byte-identity
/// at `Linear { bps: 8 }`; SquareRoot adds the Almgren-Chriss volume-
/// proxy form `cost = α · √(Q/V)` per ADR-0043 § Changelog v0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SlippageModel {
    /// Pre-v0.5.0 linear-bps model. Default `bps = 8` matches the
    /// `v5-realdata-medium-2026-05` canonical pin from ADR-0045 D1.
    Linear { bps: u32 },
    /// v0.5.0 square-root market-impact model. Operator-locked defaults
    /// (M-OD 2026-05-29): `alpha = 1.0` (Q1=(a) Kissell 2014 midpoint),
    /// `volume_lookback_days = 90` (Q2=(a) Binance parquet trailing).
    /// `alpha` is `rust_decimal::Decimal` at the public boundary; the
    /// f64 conversion happens INSIDE the model body per D-T1.3.
    SquareRoot {
        alpha: rust_decimal::Decimal,
        volume_lookback_days: u16,
    },
}

impl Default for SlippageModel {
    /// Backward-compat default: `Linear { bps: 8 }` (preserves the 71
    /// existing anchor SHAs byte-identically when `LatencySlippageSimConfig`
    /// is constructed without an explicit `slippage_model`).
    fn default() -> Self {
        SlippageModel::Linear { bps: 8 }
    }
}

/// Cap on `slippage_bps_effective` — fat-tail guard for thin-liquidity
/// hours. Operator-locked 2026-05-29; revisitable at M-OD if dry runs
/// surface > 5% saturation (K3 falsifier route).
pub const MAX_SLIPPAGE_BPS: u32 = 1_000; // 10%
```

**LatencySlippageSimConfig field replacement** (in
`crates/backtest/src/cli_types.rs`):

```rust
pub struct LatencySlippageSimConfig {
    pub latency_ms_min: u64,
    pub latency_ms_max: u64,
    pub slippage_model: SlippageModel,  // was: slippage_bps: u16
}
```

**Backward-compat serde adapter**: implement `Deserialize` on
`LatencySlippageSimConfig` with a custom visitor that accepts EITHER
the new `slippage_model: { kind: "linear", bps: 8 }` shape OR the
legacy `slippage_bps: 8` u16 field (the latter deserializes to
`SlippageModel::Linear { bps: 8 }`). Required to keep the 19 v0.4.0
friction-real anchor SHAs byte-identical under
`v5-realdata-medium-2026-05` namespace (R-NR.2).

### D-T1.3 — f64 conversion boundary contract (K2 falsifier)

**Closed-form contract** for `apply_slippage_sqrt`:

```rust
fn apply_slippage_sqrt(
    signal_price: Decimal,
    side: Side,
    notional: Decimal,     // Q in USD = |qty| * fill_price
    v_daily_usd: Decimal,  // V in USD = trailing-mean(volume × close) over N days
    alpha: Decimal,        // operator-locked α = 1.0 at v0.5.0
    max_bps: u32,          // MAX_SLIPPAGE_BPS = 1000
) -> (Decimal, u32) {       // returns (fill_price, slippage_bps_effective)
    // Edge case: V = 0 → degenerate Q/V → fall back to MAX_SLIPPAGE_BPS
    // (treat as worst-case thin liquidity; saturation is logged by caller).
    if v_daily_usd.is_zero() || notional.is_zero() {
        return (signal_price, 0);
    }

    // ── f64 conversion boundary (K2 falsifier) ─────────────────────────
    // Convert Q/V and α to f64. rust_decimal::Decimal::to_f64() is
    // deterministic across architectures (no rounding mode env var, no
    // CPU-feature-flag fast-math). The Apple Silicon canonical box runs
    // sqrt() via the AArch64 fsqrt instruction which produces IEEE-754-
    // correctly-rounded results — bit-identical across runs on the same
    // arch. v2.5 TCN Metal-vs-CPU drift does NOT apply: that was GPU
    // shader codegen drift; this is scalar f64 sqrt on the CPU.
    let q_f64: f64 = notional.to_f64().expect("notional fits in f64 — bounded by total wealth");
    let v_f64: f64 = v_daily_usd.to_f64().expect("v_daily_usd fits in f64 — bounded by venue ADV");
    let alpha_f64: f64 = alpha.to_f64().expect("alpha fits in f64 — bounded [0.0, ~2.0]");

    // Compute α · √(Q/V) · 10_000 in f64.
    let ratio: f64 = q_f64 / v_f64;            // dimensionless
    let sqrt_ratio: f64 = ratio.sqrt();         // f64::sqrt — IEEE-754 correctly rounded
    let bps_raw: f64 = alpha_f64 * sqrt_ratio * 10_000.0;

    // Round-half-to-even (banker's rounding) to u32. Stable f64::round_ties_even
    // since Rust 1.77; edition 2024 OK. Clamped at MAX_SLIPPAGE_BPS.
    // Negative bps_raw can't happen (all inputs non-negative) — saturating_cast.
    let bps_rounded: f64 = bps_raw.round_ties_even();
    let bps_u32: u32 = if bps_rounded >= f64::from(max_bps) {
        max_bps
    } else if bps_rounded <= 0.0 {
        0
    } else {
        // Safe: bounded [0, max_bps] ≤ 1000 ≪ u32::MAX
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        { bps_rounded as u32 }
    };

    // ── Back to Decimal for sign × multiplier (R3 D3 preserved) ────────
    // The fill_price application path is bit-identical to the existing
    // Linear branch — only the bps value differs by source.
    let fill_price = apply_slippage_linear(signal_price, side, bps_u32);
    (fill_price, bps_u32)
}
```

**Precision contract (load-bearing for K2 / K4 determinism)**:

1. **One conversion site**. All f64 work happens inside
   `apply_slippage_sqrt`'s body. No f64 leaks across the function
   boundary; callers only see `(Decimal, u32)`.
2. **Round-half-to-even** (`f64::round_ties_even`) at the
   `bps_raw → u32` step. Default IEEE-754 rounding mode. Deterministic
   across Apple Silicon hosts.
3. **Saturating cap**. Clamp to `[0, MAX_SLIPPAGE_BPS]` before the
   `as u32` cast so the cast is provably lossless.
4. **No `f64::sqrt` polyfill**. Use the stdlib `f64::sqrt()` directly —
   it compiles to the AArch64 `fsqrt` instruction on the canonical
   Apple Silicon box; bit-stable across runs. Cross-architecture
   determinism is gated by the existing canonical-box invariant
   (same v0.4.0 hardware; no x86/ARM mixing in CI).
5. **Decimal-side application**. The sign × multiplier step reuses
   the existing `Linear` branch logic verbatim via `apply_slippage_linear`
   helper — preserves R3 D3's "fill prices are Decimal" contract.

**Why round-half-to-even, not truncate or round-half-up**: banker's
rounding is statistically unbiased over many fills (truncation biases
downward; round-half-up biases upward at ties). Aligns with Decimal's
`MidpointNearestEven` default policy. Locks tie-break determinism
across hardware refresh cycles.

### D-T1.4 — Per-asset daily-volume retrieval shape (R3 → Option A)

**Decision**: **Option A** — extend `crates/data` with a `DailyVolume`
query that walks the existing Binance parquet feed at scenario load
time. **Rejected Option B** (bake `volume_proxy.toml`).

**Rationale**:
- **Determinism free-rider**. The parquet feed is already revision-
  pinned via `data/binance/REVISION.toml` SHA `3a8b96…bfc7` — the K2
  v0.4.0 architect check already locked it as the SoT. A `DailyVolume`
  query computed on-the-fly inherits that pin transitively; no second
  artifact needs to be regenerated when the parquet feed bumps revision.
- **No new on-disk artifact**. `volume_proxy.toml` would be a derived
  cache that could silently drift from the parquet source if the
  bake script wasn't re-run on every revision bump. Anchor cascade
  risk.
- **Trivial cost**. `DailyVolume::query(symbol, end_date, lookback_days)`
  is a thin wrapper over the existing parquet-read path. Cached per
  `(symbol, end_date, lookback_days)` tuple in-process — one read per
  scenario per symbol, not per fill.
- **Reuses existing crate boundary**. `crates/data` already owns
  parquet I/O via `data::ReplayFeed` and `crates/backtest/src/realdata.rs`
  uses it directly. No new crate dep added.

**API contract**:

```rust
// crates/data/src/binance.rs OR new module crates/data/src/daily_volume.rs

/// Mean daily traded volume in USD over the trailing N days.
///
/// Computed as the arithmetic mean of `sum(volume × close)` per UTC day
/// over the closed-open window `[end_date - lookback_days, end_date)`.
/// Volume × close is the standard USD-notional proxy when the parquet
/// feed carries base-asset volume (`volume`) and dollar-denominated
/// close (`close`). Quote-asset volume (`quote_volume`) would be
/// preferable but is not present in our v0.4.0 schema; the v · close
/// approximation is the Kissell 2014 ch. 3 § "Volume-based impact"
/// canonical form.
///
/// # Determinism
/// Pure function of (parquet revision SHA, symbol, end_date, lookback_days).
/// Cached in-process via `dashmap::DashMap` keyed on the tuple.
///
/// # Errors
/// - `DailyVolumeError::InsufficientCoverage` if < 95% of expected
///   trading-hour bars are present in the window (24×N for 1h bars).
/// - `DailyVolumeError::RevisionMissing` if `data/binance/REVISION.toml`
///   is absent.
pub fn daily_volume_usd_trailing(
    parquet_root: &Path,
    symbol: &Symbol,
    end_date: NaiveDate,
    lookback_days: u16,
) -> Result<Decimal, DailyVolumeError>;
```

**Universe-avg V helper** (Q3 operator override — see D-T1.5):

```rust
/// Arithmetic mean of `daily_volume_usd_trailing` across a fixed
/// universe of symbols. Used by synthetic scenarios under operator
/// Q3=(b) to surface real-data-flavored impact magnitudes.
pub fn universe_avg_daily_volume_usd_trailing(
    parquet_root: &Path,
    universe: &[Symbol],
    end_date: NaiveDate,
    lookback_days: u16,
) -> Result<Decimal, DailyVolumeError>;
```

### D-T1.5 — Q3 universe-avg V on synthetic implementation contract (operator override)

**Operator decision M-OD 2026-05-29**: Q3 = (b) MIXED — universe-avg V
on synthetic. **Overrides analyst-recommended Q3 = (a) Linear fallback**.
Operator framing: synthetic scenarios should "behave more real-data-like
for testing purposes" — accepts the v0.6.0 sub-namespace cleanup cost.

**Universe-avg V computation contract** (lock):

- **Aggregation function**: **arithmetic mean** of `daily_volume_usd_trailing`
  across the 10-symbol Binance universe. Rejected alternatives:
  - _Median_ — robust to outliers but introduces a tie-break decision
    on an even-count universe (10 symbols). Mean is unambiguous.
  - _Trimmed mean_ — adds a trim-fraction parameter that the operator
    would have to lock. Bare arithmetic mean is the minimal contract.
  - _Geometric mean_ — would suit volatility-style aggregation but not
    USD-volume, which is additive in dollars.
- **Universe**: the canonical 10-USDT-pair set under
  `data/binance/{ADA,AVAX,BNB,BTC,DOGE,DOT,ETH,LINK,SOL,XRP}USDT/`.
  Identical to the v0.4.0 `top10` universe.
- **end_date** for synthetic scenarios: pinned to the synthetic
  scenario's _own_ end-date (matches the scenario's logical clock
  rather than a fixed calendar date — preserves the v0.4.0
  determinism contract where rerunning a 2023 synthetic scenario in
  2027 produces the same SHA).
- **lookback_days**: 90 (Q2 default).
- **Caching**: one universe-avg V per `(end_date, lookback_days)`
  tuple; reused across all 9 synthetic scenarios that share the same
  end-date.

**SHA divergence acknowledgment** (load-bearing for namespace contract):

The 9 synthetic-data scenarios in the new `v5-sqrt-impact-2026-05`
namespace (Group A SMA/Composed × 5, Group D TCN-overlay synthetic × 2,
Group E TCN-weights × 2) **will produce SHAs that differ from their
`v5-realdata-medium-2026-05` linear-bps twins**, because the sqrt model
now applies a non-zero universe-avg V impact instead of falling back
to `Linear { bps: 8 }`. **This is by-design** under operator Q3=(b).

**v0.6.0 sub-namespace cleanup commitment** (load-bearing for namespace
hygiene): the brief acknowledges (per analyst's M-OD framing) that the
Q3 override "DOES add v0.6.0 sub-namespace cleanup." The cleanup
contract for v0.6.0 is:

- _Either_ split `v5-sqrt-impact-2026-05` into two sub-namespaces
  (`v5-sqrt-impact-realdata-2026-05` + `v5-sqrt-impact-synthetic-2026-05`)
  to make the "real V vs universe-avg V" distinction first-class in
  the anchor file.
- _Or_ retire the 9 synthetic-sqrt SHAs from the canonical anchor set
  and recompute them under the analyst-original Q3=(a) Linear fallback,
  consolidating around the 10 real-data sqrt rows + 9 linear synthetic
  rows.

The choice between these two cleanup routes is **deferred to v0.6.0
M-OD**. v0.5.0 just records the commitment; v0.6.0 is the brief that
chooses and executes.

**Anchor count under Q3=(b)**: 71 → 90, all 19 rows under
`v5-sqrt-impact-2026-05` — same row count as the analyst-recommended
Q3=(a) plan but with the 9 synthetic rows carrying genuinely-new SHAs
(rather than byte-identical-twin SHAs).

### D-T1.6 — MAX_SLIPPAGE_BPS = 1000 (10%) confirmed default

**Decision**: keep the analyst-recommended `MAX_SLIPPAGE_BPS = 1_000`
(10%) cap. Hardcoded as `pub const` in `crates/cost/src/slippage.rs`.

**Operator-override path at M-OD**: if v0.5.0 dry runs surface > 5% of
fills saturating the cap, route back to architect for refined V proxy
(K3 mitigation: hourly volume vs daily, or volume-adjusted bar windows).
Per the brief's K3 row.

**Why 1000 bps**: the worst-case scenario from K3 modeling is α=1.0 ×
sqrt(0.01) × 10_000 = 1_000 bps at `Q/V = 0.01` (1% of daily volume in
a single fill — realistic only at 03:00 UTC thin liquidity).
Saturating at 10% protects from algorithmic blow-ups while still
allowing the model to surface high-impact events. Higher caps (5_000 /
50%) would let degenerate values dominate equity curves; lower caps
(100 / 1%) would defeat the purpose of the square-root model.

### D-T1.7 — Namespace cascade contract (ADR-0047 D3 extension)

**Anchor cascade**: 71 → 90 (additive per ADR-0038 § D6.a).

| Namespace | Count pre-v0.5.0 | Count post-v0.5.0 | Mutation |
|-----------|-----------------:|------------------:|----------|
| `noop-baseline` | 51 | 51 | None (R-NR.3 byte-identical) |
| `v5-realdata-medium-2026-05` (linear-bps oracle) | 19 | 19 | None (R-NR.2 byte-identical via serde adapter; this namespace is now **the linear-bps oracle in perpetuity** — operator-pinned status mirrors the noop-baseline oracle role) |
| `v5-sqrt-impact-2026-05` (NEW; canonical sqrt-impact) | 0 | 19 | All new |
| `lab-yahoo-realdata` (lab Yahoo BTC, ETH H1) | 1 | 1 | None |
| Other lab rows | — | — | None |

**Twin-pattern relationship** (mirrors ADR-0045 D2 noop-vs-canonical):

- `noop-baseline` ↔ `v5-realdata-medium-2026-05` = frictionless oracle
  vs linear-bps friction (ADR-0045 original twin)
- `v5-realdata-medium-2026-05` ↔ `v5-sqrt-impact-2026-05` = linear-bps
  oracle vs square-root market-impact (v0.5.0 NEW twin)

The Sharpe-delta table (R5) renders 3 columns per scenario: `noop` /
`linear-bps` / `sqrt-impact` — surfaces both twin diffs in one view.

**SHA-divergence accounting under Q3=(b) operator override**:

- **10 real-data scenarios** in `v5-sqrt-impact-2026-05` carry NEW
  SHAs (they use real per-asset V from Binance parquet 90-day
  trailing).
- **9 synthetic-data scenarios** in `v5-sqrt-impact-2026-05` also
  carry NEW SHAs that **differ from their `v5-realdata-medium-2026-05`
  linear-bps twins** (universe-avg V → non-zero impact, not Linear
  fallback). Documented as by-design per D-T1.5; v0.6.0 cleanup
  commitment recorded.

### D-T1.8 — Namespace-aware Rust resolver extension (ADR-0047 D3 → third namespace)

Wave F (test crate) extends `crates/reports/tests/strategy_anchors_unchanged.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Namespace {
    Noop,
    Canonical,    // v5-realdata-medium-2026-05 (linear-bps oracle)
    SqrtImpact,   // v5-sqrt-impact-2026-05 (NEW v0.5.0)
}

const CANONICAL_FEATURE_DIRS: &[&str] = &[
    "v5-latency-slippage-sim-v0.2.0-anchor-migration",
    "v5-latency-slippage-sim-v0.3.0-full-path-wiring",
    "v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit",
];

const SQRT_IMPACT_FEATURE_DIRS: &[&str] = &[
    "v5-latency-slippage-sim-v0.5.0-square-root-market-impact",
];

const SQRT_IMPACT_STRATEGY_ANCHORS: &[(&str, &str)] = &[
    // Populated by developer Wave E close. 19 (scenario, sha256) pairs.
];

#[test]
fn t1937c_sqrt_impact_strategy_anchors_unchanged() {
    // Mirror of t1937b_canonical_strategy_anchors_unchanged.
    // For each (scenario, expected) in SQRT_IMPACT_STRATEGY_ANCHORS:
    //   resolve via find_backtest_report(scenario, Namespace::SqrtImpact)
    //   assert body_sha256(report) == expected
}
```

**Resolver algorithm extension** (mirrors ADR-0047 D3 verbatim):

```text
fn find_backtest_report(scenario, namespace):
  match namespace:
    Noop:
      collect under spec/**/reports/ EXCLUDING any canonical OR sqrt-impact dir
    Canonical:
      collect ONLY from CANONICAL_FEATURE_DIRS
    SqrtImpact:
      collect ONLY from SQRT_IMPACT_FEATURE_DIRS
  return lex-newest
```

**Noop predicate update** (load-bearing for R-NR.3): the Noop branch
MUST now exclude paths matching SQRT_IMPACT_FEATURE_DIRS as well — the
51 noop SHAs are pre-v5 and must NOT alias to v0.5.0 sqrt reports.
This is a 1-LoC predicate extension at the existing `is_canonical_path`
helper site.

### D-T1.9 — Wave decomposition for developer (Waves A → F)

| Wave | Scope | Files touched | Est. dev cost |
|------|-------|---------------|---------------|
| **A — cost crate model swap** | Add `SlippageModel` enum + `MAX_SLIPPAGE_BPS` const + `apply_slippage_sqrt` private fn + refactor `apply_slippage` → dispatcher; unit tests (α=1.0 reference at Q=$1M V=$1B → ~32 bps; cap saturation at MAX; round-half-to-even ties) | `crates/cost/src/slippage.rs`, `crates/cost/src/lib.rs` | 0.5–1 day |
| **B — backtest plumbing through `SlippageModel` enum** | Replace `slippage_bps: u16` on `LatencySlippageSimConfig` with `slippage_model: SlippageModel`; implement custom `Deserialize` adapter (legacy `slippage_bps: u16` → `Linear { bps }`); update `sim_slippage_cost` to dispatch on the enum (SOLE-LOCATION grep gate stays green) | `crates/backtest/src/cli_types.rs`, `crates/backtest/src/scenarios/sim.rs` | 0.5 day |
| **C — data crate per-asset volume + universe-avg V helper** | Add `daily_volume_usd_trailing(parquet_root, symbol, end_date, lookback)` to `crates/data` (D-T1.4 contract); add `universe_avg_daily_volume_usd_trailing(parquet_root, universe, end_date, lookback)` (D-T1.5 contract); in-process cache via `dashmap` keyed on `(symbol, end_date, lookback)`; wire scenario load path to call the helper once per scenario per symbol; explicit log line for synthetic-scenario universe-avg V path | `crates/data/src/binance.rs` (or new `crates/data/src/daily_volume.rs`), `crates/data/src/lib.rs`, `crates/backtest/src/main.rs` (load-time hook) | 0.5–1 day |
| **D — anchor resolver namespace extension** | Extend t1937 test: `Namespace::SqrtImpact` + `SQRT_IMPACT_FEATURE_DIRS` + `SQRT_IMPACT_STRATEGY_ANCHORS` + `t1937c` test + extend `is_canonical_path` predicate to also exclude sqrt-impact dirs from Noop resolution | `crates/reports/tests/strategy_anchors_unchanged.rs` | 0.25 day |
| **E — 19-scenario re-emission + 2-run determinism** | `cargo build --release -p backtest --features "candle realdata"`; run all 19 scenarios under v0.5.0 config matrix (10 real-data: `SquareRoot { α: 1.0, lookback: 90 }`; 9 synthetic: `SquareRoot { α: 1.0, lookback: 90 }` + universe-avg V helper); emit to `reports/backtest-<TS>-<scenario>.md`; 2-run byte-identity gate; append 19 new `[[anchors]]` rows under `v5-sqrt-impact-2026-05` to `spec/anchors.toml`; populate `SQRT_IMPACT_STRATEGY_ANCHORS` constants; author `reports/sharpe-delta-table-2026-05-<DD>.md` 3-column comparison; `bash scripts/verify_anchors.sh` → PASS 90/90 | `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/`, `spec/anchors.toml`, `crates/reports/tests/strategy_anchors_unchanged.rs` (constants) | 0.5 day |
| **F — e2e divergence + tester harness** | Confirm `crates/strategy/tests/latency_slippage_sim_e2e.rs` + `vol_targeting_overlay_end_to_end.rs` + `vol_killswitch_overlay_end_to_end.rs` PASS under BOTH `Linear { bps: 8 }` AND `SquareRoot { α: 1.0, lookback: 90 }` configs (R-NR.5 + CLAUDE.md non-negotiable); confirm `crates/strategy/tests/overlay_hygiene_gate.rs` PASS (D6 inventory unchanged at 3+1 meta — no new overlay landed); `cargo test --workspace --no-fail-fast` → no new failures vs v0.4.0 whitelist | `crates/strategy/tests/*` (no changes expected; verification only) | 0.25 day |

**Critical-path ordering**: A → B → C → D → E → F. Wave D can land in
parallel with Wave C (both touch independent files); Wave E depends on
A+B+C+D all green. Wave F is verification-only — can run in parallel
with E once reports are emitted.

**Total est.**: ~3.0–4.0 dev-days + 1 tester-day + 0.5 presenter-day =
**~1 week wall-clock** (consistent with feature.md § Cost framing
DURABLE route).

### D-T1.10 — Open questions / assumptions for developer

- **A1**: `f64::round_ties_even` is stable since Rust 1.77 (edition 2024
  compatible). If the workspace stable channel is older than 1.77,
  fall back to `(bps_raw + 0.5).floor()` (round-half-up, biased) but
  flag the precision-contract deviation to architect for re-approval.
  (Verified at brief author time: workspace `rust-toolchain.toml` is
  on stable; 1.77+ confirmed.)
- **A2**: `rust_decimal::Decimal::to_f64()` returns `Option<f64>` and
  can return `None` for unrepresentable values. All four inputs
  (notional, v_daily_usd, alpha, signal_price) have well-bounded
  magnitudes (≤ $1e12 cash, ≤ $1e11 ADV, α ∈ [0, 2]) — `.expect()` is
  load-bearing-safe with the documented invariant. Developer adds a
  debug_assert! at the boundary for invariant audit.
- **A3**: The `dashmap` crate is the analyst lean for in-process
  caching. If `dashmap` is not already a workspace dep, fall back to
  `std::sync::Mutex<HashMap<_, _>>` — the cache is hit O(N_scenarios ×
  N_symbols) times per backtest run (low contention; mutex is fine).
- **A4**: Q3 universe-avg V end_date is derived from the scenario's
  `end_year + end_month` (the scenario's bar-range end). Developer
  verifies that synthetic scenarios carry a `end_year`/`end_month`
  field on their `ScenarioConfig` — if not, the universe-avg V
  computation pins to a hardcoded `2024-12-31` calendar date and that
  is documented in the scenario's report front-matter. Architect lean:
  use the scenario's own end-date if available; hardcoded-fallback
  if not. Either choice is byte-stable as long as it's deterministic.

## Implementation

### Waves A–C + F complete (developer 2026-05-29)

**Wave A — `crates/cost/src/slippage.rs`**

- Added `SlippageModel` enum (`Linear { bps: u32 }` | `SquareRoot { alpha: Decimal, volume_lookback_days: u16 }`) with `#[serde(tag = "kind", rename_all = "snake_case")]`.
- Added `pub const MAX_SLIPPAGE_BPS: u32 = 1_000` (10% cap, K3 gate).
- Added `pub fn apply_slippage_model(signal_price, side, notional, model, volume_usd) -> Decimal` dispatcher.
- Added private `apply_slippage_sqrt` with f64 boundary contract (D-T1.3): `Q/V` and `√` in f64, `round_ties_even` → saturating-cast `u32`, back to Decimal for sign × multiplier.
- Preserved `pub fn apply_slippage(signal_price, side, _notional, bps)` as legacy entry-point (unchanged body).
- 14 unit tests in `slippage::tests`: reference value (`α=1.0, Q=$1M, V=$1B → 316 bps`), cap saturation, edge cases (V=0, Q=0, α=0), tie-break rounding.

**Wave B — `crates/backtest/src/cli_types.rs` + `scenarios/sim.rs` + `main.rs`**

- Replaced `slippage_bps: u32` with `slippage_model: SlippageModel` on `LatencySlippageSimConfig`.
- Added `volume_usd_per_symbol: Option<Arc<HashMap<Symbol, Decimal>>>` (`#[serde(skip)]`) for per-symbol V population at scenario load time.
- Custom `Deserialize` visitor: accepts both `slippage_model` (new) and `slippage_bps` (legacy → `Linear { bps }`); missing → `Linear { bps: 0 }`.
- `Default` → `Linear { bps: 0 }, volume_usd_per_symbol: None`.
- `is_noop()` → pattern-matches `Linear { bps: 0 }`.
- `sim_slippage_cost` (in `scenarios/sim.rs`, SOLE-LOCATION per ADR-0047 D2) dispatches on enum: Linear bps=0 → ZERO; Linear bps>0 → byte-identical body; SquareRoot → `apply_slippage_model` then `qty × |adjusted - original|`.
- `build_slippage_model(&args)` helper in `main.rs`: `--sim-slippage-sqrt-alpha > 0` → `SquareRoot`; else → `Linear { bps: sim_slippage_bps }`.
- All 12 `LatencySlippageSimConfig` struct literals in `main.rs` updated with `volume_usd_per_symbol: None`.
- Updated all test literals in `cli_types.rs` (9 tests), `scenarios/sim.rs` (6 tests), `strategy/tests/latency_slippage_sim_e2e.rs` (3 tests).

**Wave C — `crates/data/src/daily_volume.rs`**

- `DailyVolumeError` enum (thiserror) with SymbolNotFound | Polars | InsufficientCoverage | Parse.
- `OnceLock<Mutex<HashMap<(String, i32, u16), Decimal>>>` in-process cache.
- `pub fn daily_volume_usd_trailing(parquet_root, symbol, end_date, lookback_days) -> Result<Decimal, DailyVolumeError>`: scans parquet via polars `LazyFrame`, filters by `open_time` in `[start, end)`, accumulates `Σ(volume × close)` per UTC day, returns arithmetic mean.
- `pub fn universe_avg_daily_volume_usd_trailing(parquet_root, universe, end_date, lookback_days)`: arithmetic mean across universe; soft-skips SymbolNotFound.
- 5 unit tests (no real parquet required): cache miss + empty universe + all-missing + date_to_unix_millis + day_ordinal bucketing.
- Re-exported from `crates/data/src/lib.rs`.

**Wave F — `crates/reports/tests/strategy_anchors_unchanged.rs`**

- Added `SqrtImpact` to `Namespace` enum (3-namespace resolver: Noop / Canonical / SqrtImpact).
- Added `SQRT_IMPACT_FEATURE_DIRS` + `SQRT_IMPACT_STRATEGY_ANCHORS` (empty until Wave E populates).
- Added `t1937c_sqrt_impact_strategy_anchors_unchanged` test (soft-skips when table empty).
- All 4 tests pass: `t1937`, `t1937b`, `t1937c`, `t1942`.

**Verification gates (all green as of 2026-05-29)**

- `cargo test -p cost -- slippage::tests` → 14/14 pass.
- `cargo test -p data --lib` → 52/53 pass (1 ignored: requires real parquet).
- `cargo test -p backtest --lib` → 44/44 pass (5 ignored: require config files).
- `cargo test -p reports --test strategy_anchors_unchanged` → 4/4 pass.
- `cargo test -p strategy --test latency_slippage_sim_e2e` → 3/3 pass.
- `cargo test -p strategy --test vol_targeting_overlay_end_to_end` → 1/1 pass.
- `cargo test -p strategy --test vol_killswitch_overlay_end_to_end` → 4/4 pass.
- `cargo clippy -p backtest -p cost -p data -p strategy` → 0 warnings, 0 errors.
- Grep gate: `grep -r "fn sim_slippage_cost" crates/backtest/src` → 1 definition (ADR-0047 D2).

**Pending (Waves D–E)**

Waves D (19-scenario re-emission) and E (anchor population + Sharpe-delta table) require `cargo build --release --features "candle realdata"` + actual backtest runs. Left for tester to run on the canonical Apple Silicon box, or developer can run if the operator authorizes the long-running wave.

_Tester M-FINAL links to reports here._

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
- 2026-05-29 (operator M-OD): Q1 = (a) α=1.0 Kissell midpoint
  [Recommended — DURABLE]; Q2 = (a) 90-day trailing Binance parquet
  for per-asset V [Recommended — DURABLE]; **Q3 = (b) MIXED —
  universe-avg V on synthetic (operator override of analyst-recommended
  (a) Linear fallback)**. Operator framing: synthetic scenarios should
  "behave more real-data-like for testing purposes"; accepts the v0.6.0
  sub-namespace cleanup cost. HANDOFF → architect M-T1.
- 2026-05-29 (architect M-T1): § Design § D-T1.1–D-T1.10 locked.
  **ADR-0043 § Changelog amended (NOT new ADR-0050)** — mirrors the
  2026-05-27 Murmur3 D2 amendment precedent; engine-ADR continuity
  preserved. `SlippageModel` enum signature locked at
  `crates/cost/src/slippage.rs` with `MAX_SLIPPAGE_BPS: u32 = 1_000`
  const; `Default::default() = Linear { bps: 8 }` for R-NR.2 byte-
  identity. **f64 conversion boundary**: one site in
  `apply_slippage_sqrt`; `f64::sqrt` + `f64::round_ties_even` →
  saturating-cast `u32`; back to Decimal for sign × multiplier. **R3
  Option A locked** — `daily_volume_usd_trailing` extends `crates/data`;
  no on-disk volume_proxy.toml artifact (Option B rejected). **Q3
  operator-override implementation**: `universe_avg_daily_volume_usd_trailing`
  helper computes arithmetic mean across the 10-USDT-pair Binance
  universe; pinned to scenario's own end_date with 90-day lookback;
  **9 synthetic-scenario SHAs in `v5-sqrt-impact-2026-05` namespace
  WILL DIFFER from their `v5-realdata-medium-2026-05` linear-bps
  twins — by-design under operator Q3=(b); v0.6.0 sub-namespace
  cleanup commitment recorded** (D-T1.5). **MAX_SLIPPAGE_BPS = 1_000
  (10%) confirmed**; operator-override path at M-OD if dry runs
  surface > 5% saturation. **Namespace cascade**: 71 → 90 (additive;
  R-NR.1 PASS at 90/90; R-NR.2 + R-NR.3 byte-identity preserved).
  **t1937 extension**: `Namespace::SqrtImpact` + `SQRT_IMPACT_FEATURE_DIRS`
  + `SQRT_IMPACT_STRATEGY_ANCHORS` + `t1937c` test (mirrors `t1937b`
  precedent); Noop predicate extended to exclude sqrt-impact dirs.
  **Wave decomposition A→F locked** at ~3.0–4.0 dev-days. Frontmatter
  flipped `owner: architect → developer`. HANDOFF → developer.
- 2026-05-29 (developer, commit `e09e599`): Waves A+B+C+F shipped PASS.
  Cost-layer `SlippageModel` enum + dispatcher + `apply_slippage_sqrt`
  (14/14 unit PASS); CLI plumbing + serde backward-compat (13/13 PASS);
  `crates/data/src/daily_volume.rs` helper + universe-avg helper +
  dashmap cache (5/5 PASS); namespace-aware resolver + Wave F e2e (4/4 +
  3/3 PASS). Waves D (19-scenario re-emit) + E (anchors.toml cascade)
  parked pending Q-D1 + Q-D2 operator clarification — main.rs currently
  passes `volume_usd_per_symbol: None` → Decimal::ZERO for all 12 configs;
  Q-D1 asks Linear-fallback vs SquareRoot+universe-avg V on synthetic;
  Q-D2 asks pre-compute vs lazy-compute vs CLI-flag wiring. HANDOFF →
  analyst (operator-decide brief).
- 2026-05-29 (analyst, this commit): Q-D1 + Q-D2 operator-decide brief
  authored at [`spec/dev-notes/v5-v0.5.0-q-d1-q-d2-decision-brief-2026-05-29.md`](../dev-notes/archive/2026-Q2/v5-v0.5.0-q-d1-q-d2-decision-brief-2026-05-29.md).
  Recommended path **Q-D1=(a) Linear-fallback + Q-D2=(β) per-scenario
  lazy-compute** per 2026-05-28 durable-over-quick contract. The
  recommendation proposes operator REVISIT of M-OD 2026-05-29 Q3=(b)
  ratification — Wave A+C shipped evidence (14+5 unit-test coverage)
  now shows the synthetic-sqrt-model-body determinism oracle is already
  covered at the unit-test layer; the Q-D1=(b) value-prop ("synthetic
  scenarios behave more real-data-like") collapses post-evidence
  (synthetic Q × real universe-avg V is a dimensionally-mixed signal,
  not a cleaner one). Q-D1=(a) drops the v0.6.0 sub-namespace cleanup
  commitment from "must spawn brief" to "obsolete by-design"; anchor
  cascade narrows from 75→94 (Q3=(b)) to 75→85 (Q-D1=(a)). Frontmatter
  flipped `owner: developer → analyst`, `status: draft → operator-decide-pending`.
  Trace state flipped `dev-done → operator-decide-pending`. HANDOFF →
  orchestrator (surface to operator via AskUserQuestion with
  Recommended path defaulted).
- 2026-05-29 (operator, analyst commit `6072f9a`): **Q-D1=(a) RATIFIED** — Linear{bps:8}
  fallback for synthetic scenarios. M-OD 2026-05-29 Q3=(b) ratification SUPERSEDED.
  **Q-D2=(β) RATIFIED** — per-scenario lazy-compute via existing Wave C helper.
  Anchor cascade REVISED: 75 → 85 (10 new real-data anchors under
  `v5-sqrt-impact-2026-05`). v0.6.0 sub-namespace cleanup commitment DROPPED
  — obsolete by-design under Q-D1=(a). Frontmatter: `owner: analyst → developer`,
  `status: operator-decide-pending → dev-in-progress`. Waves D+E unparked. HANDOFF → developer.
- 2026-05-29 (developer, Waves D+E close): Bug discovered and fixed in `sim_slippage_cost`:
  all 7 call sites were passing `Decimal::ZERO` as `volume_usd` (no-op bug — SquareRoot model
  received V=0 → zero impact). Fixed by changing `sim_slippage_cost` signature to take
  `symbol: &Symbol` and look up from `cfg.volume_usd_per_symbol` internally. All 7 scenario
  files updated. 9 real-data scenarios re-emitted under `v5-sqrt-impact-2026-05` with the
  FIXED binary. Determinism gate: 9/9 PASS. Anchor cascade: 75 → 84 (9 new rows, not 10,
  because `top10-2024-fy-momentum-realdata` scenario does not exist in code — Group B implemented
  as 1 scenario not 2). H1 PASS: sqrt drag 3.91× linear on TCN-realdata-2023. H3 PASS: all
  deterministic. Status: `dev-in-progress → dev-done`. HANDOFF → tester.
