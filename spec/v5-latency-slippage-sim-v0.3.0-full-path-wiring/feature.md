---
slug: v5-latency-slippage-sim-v0.3.0-full-path-wiring
version: 0.3.0
status: in-progress
owner: presenter
updated: 2026-05-27
predecessor: v5-latency-slippage-sim-v0.2.0-anchor-migration v0.1.0
parent: backtest-vs-live-execution-gap
priority: P1
---

# v5 latency-slippage-sim v0.3.0 — full-path wiring + Group A data-source decision + t1937 fix

> The feature **version** being shipped by this brief is **v5
> v0.3.0** (the canonical simulator config is plumbed into the 6
> strategy-construction paths v0.2.0 missed, AND the load-bearing
> Group A data-source question is decided, AND the
> `t1937_nine_strategy_anchors_unchanged` test is resurrected). The
> brief document itself starts at v0.1.0 (analyst-authored).

## Why now (full context)

v5-latency-slippage-sim v0.2.0 shipped on **2026-05-27** (commits
`d2cc343`, `c223d11`, `4dfa2d8`, `d191227`). The anchor count
doubled 34 → 68 under the canonical namespace pin
**`v5-realdata-medium-2026-05`** with the ADR-0045-locked config

```rust
LatencySlippageSimConfig {
    latency_ms_min: 30,
    latency_ms_max: 80,
    slippage_bps:   8,
}
```

The operator approved a **Ship Route (a) partial migration** at
v0.2.0 with two explicit scope gaps documented in the M-FINAL test
report (`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/test-final-2026-05-27-v5-latency-slippage-sim-v0.2.0-anchor-migration.md`)
and the Sharpe-delta table (`sharpe-delta-table-2026-05-27.md`).
This brief closes both:

1. **Scope gap — 6 strategy paths still =noop.** Per the Sharpe-delta
   table § Summary: only **1 of 7 runnable strategy paths** (momentum)
   has `LatencySlippageSimConfig` wired into its construction site.
   The remaining 6 carry canonical SHAs that are **byte-identical to
   their noop-baseline counterparts** because the simulator isn't
   actually applied:
   - **SmaComposed** (5 scenarios — sma-cross, sma-baseline-refresh,
     macd-trend, rsi-reversion, bbands-mean-revert)
   - **TcnOverlay** (8 scenarios — 4 synthetic + 4 realdata across
     2023/2024 + weights variants)
   - **PatchTstOverlay** (1 scenario — top10-2023-fy-patchtst-overlay-realdata)
   - **Pairs** (2 scenarios — pairs-2023, pairs-2024-h1)
   - **VolTargetOverlay** (1 scenario — top10-2023-fy-vol-target-overlay-realdata)
   - **GarchVolOverlay** (analysis paths; verdict at architect M-T1 whether re-emit warrants)

2. **Group A data-source drift (LOAD-BEARING — Q1).** v0.2.0 Wave A
   re-emission picked up real Binance Parquet data for the 5
   SMA/Composed scenarios because the runtime auto-switches synthetic
   → real-Binance data when the data path exists. This shifted
   equity by **+$48k to +$83k per scenario** (Sharpe -13 to -68 in
   noop, Sharpe -13 to +11.6 in canonical). **None of that delta is
   attributable to the v5 sim.** The canonical Group A SHAs are now
   "real-data oracle" SHAs even though the friction sim itself never
   fired on those paths. The operator must decide whether this
   data-source drift is **accepted** (treat 2026-05-27 as the new
   oracle epoch for SMA/Composed) or **reverted** (re-anchor against
   synthetic baseline, then re-apply v5 sim under the same
   pre-realdata environment).

3. **`t1937_nine_strategy_anchors_unchanged` is failing post-Wave A.**
   The test at `crates/reports/tests/strategy_anchors_unchanged.rs`
   hardcodes the original noop-baseline SHA-256 constants for 7
   scenarios (lines 41-77). Its `find_backtest_report` helper
   resolves the "newest" matching report by lexicographic filename
   sort — but does NOT apply the namespace-aware logic that
   `verify_anchors.sh` does. Wave A's canonical reports
   (`backtest-20260527-065*-<scenario>.md`) now sort lexicographically
   after the original noop reports and the test picks up the canonical
   reports, which have different SHAs. The operator whitelisted this
   failure at v0.2.0 M-FINAL with the explicit understanding that
   v0.3.0 closes it.

This brief closes all three gaps in a single ship.

## Scope (v0.3.0)

### R1 — Full-path wiring: plumb canonical `LatencySlippageSimConfig` into the 6 unwired strategy construction sites

`MomentumScenarioInput` carries a `latency_slippage_sim:
LatencySlippageSimConfig` field today; the other 5 scenario-input
structs in `crates/backtest/src/cli_types.rs` (lines 124-211) do not.
v0.3.0:

1. Adds the `latency_slippage_sim` field to each of:
   - `SmaScenarioInput` (drives `sma_composed_run::run`, lines 35,
     298 in `crates/backtest/src/scenarios/sma_composed_run.rs`)
   - `PairsScenarioInput` (drives `pairs::run`, lines 19, 67 in
     `crates/backtest/src/scenarios/pairs.rs`)
   - `TcnScenarioInput` (drives `tcn_overlay::run`, `tcn_overlay_weights::run`,
     `patchtst_overlay_weights::run`, `garch_vol_target_overlay::run`,
     `threshold_sweep::run` — all 5 use the same input struct)
2. Threads the field through each `run` function's fill / exec /
   accounting boundary the same way `momentum::run` does (lines 390,
   438, 555 in `crates/backtest/src/scenarios/momentum.rs`).
3. Wires the CLI flag plumbing in `crates/backtest/src/main.rs` to
   populate the new fields from the same three CLI args
   v0.2.0 added (`--sim-latency-ms-min`, `--sim-latency-ms-max`,
   `--sim-slippage-bps`).

**Architect M-T1 owns the per-scenario plumbing audit** — confirms
each strategy's fill path is actually friction-affectable (a
strategy that never trades won't show divergence; a strategy whose
fills are batched at synthetic mid-prices may need a tiny refactor).
GarchVolOverlay's analysis paths are confirmed in scope OR explicitly
deferred at M-T1.

### R2 — Group A data-source decision [LOAD-BEARING — Q1]

Two routes; **NO SAFE ANALYST DEFAULT** — see § Operator-decide
table below for the analyst's framing.

- **Route (a) — revert Group A to synthetic baseline.** Restore the
  pre-realdata environment (force `bars_override = None` for the 5
  SMA/Composed scenarios at re-emission time, OR pin the runtime
  to the synthetic data path the noop-baseline SHAs were computed
  under). Re-emit canonical SHAs under v5 sim ONLY (no data-source
  drift). Anchors become apples-to-apples comparable to noop-baseline
  modulo friction.
- **Route (b) — accept real-Binance baseline as new oracle epoch.**
  Codify 2026-05-27 as the new oracle date for SMA/Composed; the
  current canonical SHAs stay; v5 sim is applied ON TOP. The
  Sharpe-delta table for Group A becomes "real-Binance vs
  real-Binance + v5 sim" rather than "synthetic + noop vs synthetic
  + v5 sim".

Either route is followed by R1 plumbing — but R2 determines what
data the re-emission consumes. Route (a) preserves the historical
A/B sanity check; route (b) accepts forward-looking real-data
realism.

### R3 — `t1937_nine_strategy_anchors_unchanged` resolution [Q4]

Two routes:

- **Route (a) — update constants.** Bump the 7 hardcoded SHAs in
  `crates/reports/tests/strategy_anchors_unchanged.rs` (lines 41-77)
  to match the new canonical Wave A SHAs (post-R1 + R2). Fragile —
  every future anchor migration re-breaks this test.
- **Route (b) — namespace-aware resolver.** Mirror the
  `verify_anchors.sh` rewrite from v0.2.0 Wave B: the test's
  `find_backtest_report` helper learns to filter by namespace
  (noop-baseline vs canonical) and the constants stay pinned to
  noop-baseline SHAs forever. Future-proof against the v0.4+
  inevitable namespace expansion.

**Analyst recommends (b) namespace-aware** — symmetry with
`verify_anchors.sh`; future-proof. Final selection is operator-
decide at Q4.

### R4 — Anchor SHA migration in `spec/anchors.toml` [Q3]

Two routes:

- **Route (a) — extend existing pin (analyst-recommended).** The
  `v5-realdata-medium-2026-05` namespace SHAs for the 32
  currently-byte-identical-to-noop scenarios get **bumped** to the
  new R1+R2 outputs. Same pin, new SHAs. Total row count stays at
  68. Backwards-incompatible to anyone who cached the v0.2.0 SHAs,
  but `verify_anchors.sh` re-PASSes at 68/68.
- **Route (b) — new pin `v5-realdata-medium-2026-05-full`.** Adds a
  THIRD namespace alongside `noop-baseline` and
  `v5-realdata-medium-2026-05`. The v0.2.0 pin retires (or stays as
  "partial-wiring historical"); the new pin is the canonical full-
  wiring set. Row count grows from 68 to ~100 (68 + 32 new).
  Cleaner history; messier anchors.toml.

**Analyst recommends (a) extend same pin** — the v0.2.0 pin only
existed for 1 day (2026-05-27 to 2026-05-27). It hasn't accrued
operational meaning yet; bumping the SHAs in-place is acceptable.
Route (b) is appropriate if the operator wants v0.2.0's partial-
wiring SHAs preserved for archeology.

### R5 — Sharpe-delta table extension

Extend `spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/sharpe-delta-table-<DATE>.md`
to cover **all 7 strategy paths under canonical friction** (post-R1).
The v0.2.0 table covered 2 paths cleanly (momentum) + 5 paths
contaminated by data-source drift (Group A) + 6 paths showing $0
delta because sim wasn't wired. The v0.3.0 table:

- Group A (SMA/Composed) — reports v5-sim friction effect cleanly
  per R2 decision (route (a) shows pure friction; route (b) shows
  friction-on-top-of-real-data)
- Group B (Momentum) — unchanged from v0.2.0
- Group C (Pairs) — first-time friction effect
- Group D (TCN) — first-time friction effect (8 scenarios)
- Group E (PatchTST) — first-time friction effect
- Group F (VolTarget) — first-time friction effect
- Group G (Analysis/investigation) — verdict per architect at M-T1
  whether sim is meaningful (these are forecast-distribution /
  sharpe-comparison / threshold-sweep analysis reports; many have
  no equity surface at all)
- Group H (Operator success samples) — unchanged

K1 surprise scan re-runs across all 32 newly-wired scenarios.
Hypothesis H1 (≤ 3 flipped scenarios under canonical config) is the
falsifier. If > 3, R-O3 routing applies (operator-decide refine vs
accept).

### R6 — Cross-feature e2e re-check budget [Q5]

Same as v0.2.0 Q4 — re-run every overlay/sizing-modifier e2e
divergence test under the post-R1 canonical config. Architect M-T1
re-surveys; expected scope is unchanged at 3 files
(`vol_targeting_overlay_end_to_end.rs`,
`vol_killswitch_overlay_end_to_end.rs`,
`latency_slippage_sim_e2e.rs`) since no new overlay shipped between
2026-05-27 v0.2.0 and v0.3.0 brief author time. The "≥ 1 bp
divergence vs noop" assertion stays valid; absolute equity values
shift per scenario as new paths actually fire friction.

### R-NR — Non-regression contract

- **R-NR.1** — 32 currently-=noop canonical SHAs are bumped (route
  (a) extend pin) OR retired into new pin (route (b)). Post-R4,
  `bash scripts/verify_anchors.sh` MUST report `ANCHORS PASS
  (68 / 68)` if route (a) OR `ANCHORS PASS (N / N)` for route (b)
  where N is the post-migration total.
- **R-NR.2** — All 34 backtest scenarios still complete cleanly.
  No NEW panics / crashes / nonsensical equity values introduced
  by R1 plumbing.
- **R-NR.3** — Cross-feature e2e tests (R6 scope) still PASS at
  ≥ 1 bp divergence assertion.
- **R-NR.4** — `cargo test --workspace --no-fail-fast` shows NO
  NEW failures vs the v0.2.0-ship whitelist. The whitelisted
  `t1937_nine_strategy_anchors_unchanged` failure **MUST FLIP TO
  GREEN post-R3** (that's R3's gate).
- **R-NR.5** — `crates/exec`, `crates/cost`, `crates/audit`
  library code is **NOT TOUCHED**. R1 plumbing edits only
  `crates/backtest/src/cli_types.rs`, `crates/backtest/src/scenarios/*.rs`,
  `crates/backtest/src/main.rs`. The simulator engine is unchanged
  from v0.1.0 / v0.2.0.
- **R-NR.6** — `AuditEvent::SimulatedExecMetrics` is now emitted
  by 7 strategy paths instead of 1. No schema change; just more
  emission sites.

## K — Risk register / falsifiers

| K | Risk | Mitigation |
|---|---|---|
| **K1** | **Alpha inversion under realistic friction** — newly-wired strategies (TCN, PatchTST, VolTarget, Pairs, SMA/Composed) may flip from positive to negative Sharpe under 8 bps slippage. | R5 delta table surfaces flipped scenarios. Q3 v0.2.0 (per-scenario flag for operator review) precedent applies. H1 falsifier: > 3 flipped → R-O3 route. |
| **K2** ~~RESOLVED 2026-05-27~~ | ~~**R2 route (a) is technically infeasible**~~ — **K2-REACHABLE-CHEAP** confirmed by architect M-T1 (5-LoC CLI flag `--force-synthetic-bars` at `main.rs:977-1020`; no refactor). Route (a) is alive for operator Q1. See § Design T-AR-1 and ADR-0047 D1. | Risk retired. Operator decides Q1 on value-tradeoff merits, not feasibility. |
| **K3** | **Data-source drift contaminates K1 surprise detection for Group A.** Under R2 route (b), it becomes impossible to attribute Sharpe changes to friction vs data-source vs both. | R5 column "Δ Sharpe driver" makes the attribution explicit. For Group A under route (b), driver = "real-data + v5-sim combined" — the noop-baseline SHA is no longer a valid friction-free oracle for SMA/Composed because the data inputs differ. |
| **K4** | **`t1937` failure cascade.** Even after R3 lands, other tests may have similar hardcoded-SHA + lex-sort patterns. | Developer Wave grep for "STRATEGY_ANCHORS\|find_backtest_report\|body_sha256" across `crates/*/tests/*.rs` and lists every offender. Each offender either updates constants OR adopts the namespace-aware pattern from R3. |
| **K5** | **Cross-feature e2e tests still pass at v0.2.0 ship; passing them again is performative.** | True for the 3 known overlay files. The real gate is the NEW 32 strategy-path friction tests embedded in R5's delta table — those are the ones that haven't been validated before. R6 is defensive ("did we break anything"), R5 is offensive ("did we change what we intended to change"). |
| **K6** | **Operator chooses R2 route (a) but R1 plumbing has already landed against route (b) assumptions.** | M-OD operator decide happens BEFORE M-T1 architect lock per AGENT.md. Q1 routes the architect's M-T1 plumbing audit, NOT the developer's M-DEV. No code lands until operator decides. |

## H — Hypotheses

| H | Hypothesis | Confidence | Falsifier |
|---|---|---|---|
| **H1** | **Alpha-degradation per newly-wired path is < 10 % absolute Sharpe under canonical friction.** Most strategies that survived 0-bps backtests survive 8-bps friction. | Medium | R5 delta table aggregate. If > 3 scenarios flip sign OR > 10 scenarios degrade by > 0.5 Sharpe units, H1 falsified → R-O3. |
| **H2** | **R2 route (a) is reachable** — the synthetic-data fallback can be re-engaged for the 5 Group A scenarios without major refactor. | Low-medium | Architect M-T1 audit. If unreachable (K2), force route (b) into Q1 framing as the only viable option. |
| **H3** | **TCN / PatchTST friction effect is dominated by their lower trade frequency** — these overlays trade less often than momentum, so absolute equity drag is < momentum's $3.5k-$5.4k. | Medium | R5 delta table per-group. If TCN drag > momentum drag at similar capital, H3 falsified — indicates TCN's signal is more friction-sensitive than expected. |
| **H4** | **R3 namespace-aware resolver pattern (route b) is a one-day developer task.** Mirror `verify_anchors.sh` Bash logic in Rust. | High | Developer Wave estimate at M-DEV kickoff. If > 2 days, route (a) constant-update becomes more attractive. |

## Operator-decide questions (Q1-Q5)

| Q | Topic | Options | Analyst-recommended default | Rationale |
|---|---|---|---|---|
| **Q1** | **Group A data-source — load-bearing** | **(a) revert to synthetic baseline** / **(b) accept real-Binance baseline as new oracle epoch** | **(a) revert to synthetic baseline (REVISED post-K2)** | **K2 verdict (architect M-T1 2026-05-27): K2-REACHABLE-CHEAP.** The synthetic-vs-Parquet auto-detect lives at `crates/backtest/src/main.rs:977-1020` in a single `if has_parquet { Parquet } else { synthetic }` block, affecting ONLY the single-symbol (SMA/Composed) dispatch arm. A `--force-synthetic-bars` CLI flag (~5 LoC, no refactor) makes route (a) cheap and reachable. **Cost picture has flipped versus the analyst brief.** Route (a) now preserves the apples-to-apples comparison between noop-baseline and canonical SHAs for SMA/Composed (friction is the ONLY variable) at trivial cost. Route (b) accepts more realistic real-Binance data inputs but forfeits the friction-free oracle for 5/68 anchors — operator MAY still prefer (b) for forward-looking-realism reasons, but the previous "(a) has hidden refactor cost" framing no longer holds. **Architect lean (transparency, not a decision): (a)** — preserves regression-gate semantics for 5/68 anchors at trivial cost. Operator override to (b) remains valid; defensible. See ADR-0047 D1 + D4 for the conditional re-emission contract under each route. |
| **Q2** | **Wave ordering** | (a) wire all 6 paths first, then re-emit all canonicals in one atomic Wave / (b) wire-and-emit per-path | **(a) all-then-emit** | Single atomic SHA migration is cleaner for `spec/anchors.toml` review; per-path emission risks intermediate states where some paths show new SHAs and others don't, confusing the Sharpe-delta diff. Cost is the same; risk of mid-migration interrupt is lower with (a). |
| **Q3** | **Anchor namespace strategy** | **(a) extend `v5-realdata-medium-2026-05`** (same pin, new SHAs) / (b) new pin `v5-realdata-medium-2026-05-full` | **(a) extend same pin** | The v0.2.0 pin is 1 day old; bumping the SHAs in-place is acceptable. Route (b) is appropriate ONLY if operator explicitly wants v0.2.0's partial-wiring SHAs preserved for archeology. Storage cost route (b): +32 anchor rows. Clarity cost route (b): two "canonical" pins requiring operator interpretation forever. |
| **Q4** | **t1937 resolution** | (a) update SHA constants / **(b) namespace-aware resolver** | **(b) namespace-aware** | Mirror `verify_anchors.sh` Bash logic in Rust (the script's namespace-filter walk). Future-proof against the inevitable v0.4+ namespace expansion. Route (a) is a 5-minute edit but re-breaks every future anchor migration. |
| **Q5** | **Cross-feature re-check budget** | **(a) re-run all overlay e2e tests under canonical config** / (b) re-run only the load-bearing 3 / (c) defer all to v0.4 | **(a) re-run all** | Same precedent as v0.2.0 Q4 = (a). Anchor cascade isn't optional — half-migrating leaves silent invariant drift. Cost is ~0.5-1 dev-day given the 3-test inventory from v0.2.0 hasn't grown. |

**Q1 is the load-bearing one.** Q2-Q5 are standing-Autoapprove-
eligible at analyst-recommended defaults. The K2 reachability probe
(architect M-T1 2026-05-27 — see § Design and ADR-0047 D1) found
route (a) is cheaply reachable (~5 LoC CLI flag), shifting the cost
picture such that analyst-style "no safe default" no longer fully
applies — but the value tradeoff (regression-gate semantics vs
real-data realism) is still genuinely operator judgment. The
revised analyst default = (a) is recommended but not mandatory.

## Pre-drawn 4-cell verdict tree (presenter inherits)

| Cell | Condition | Route |
|---|---|---|
| **R-O1** | All 6 R rows green + R-NR.1-6 + H1 holds (≤ 3 flipped scenarios across the 32 newly-wired) + R3 flips t1937 to GREEN | **SHIP** v0.3.0 + close the v5 anchor-migration arc; spawn v0.4 follow-on briefs only on operator request (e.g. square-root market-impact deferred from v0.1.0 D3, intrabar fill sampling) |
| **R-O2** | H1 holds but 1-3 K1 retirement candidates surface | **HOLD** — spawn per-scenario retirement briefs per v0.2.0 Q3=(b) operator-review precedent |
| **R-O3** | H1 violated (≥ 4 strategies inverted) | **Operator-decide**: ship the bad news (accept the more realistic alpha picture across the board), refine canonical config (e.g. drop to tight = 20..=50 / 3 bps), OR retire entire strategy families. v0.3.0 ships HELD pending operator. |
| **R-O4** | R-NR.2 fails — a scenario doesn't compile or run cleanly under R1 plumbing | **REGRESSION** — developer iteration; HANDOFF → architect for root-cause; blocks ship |

## Cost framing

| Phase | Effort |
|---|---|
| Analyst (this brief) | ~0.5 day |
| Operator-decide (Q1 load-bearing + Q2-Q5 standing-Autoapprove) | ~30 min (Q1 needs explicit judgment) |
| Architect M-T1 (per-scenario plumbing audit + R2/R3 lock + K2 reachability check) | ~0.5 day |
| Developer Wave A — R1 plumbing for 6 paths (~5-10 LOC per path + 1-2 plumbing tests) | ~1 day |
| Developer Wave B — re-emit canonical reports for 32 scenarios under canonical config | ~1 day |
| Developer Wave C — R4 anchor SHA migration in `spec/anchors.toml` | ~0.25 day |
| Developer Wave D — R5 Sharpe-delta table extension | ~0.5 day |
| Developer Wave E — R3 t1937 resolver fix (namespace-aware) + grep for similar offenders | ~0.5 day |
| Developer Wave F — R6 cross-feature e2e re-runs | ~0.25 day |
| Tester M-FINAL (verify 68/68 anchors + R5 delta + R3 t1937 GREEN + R6 e2e + workspace test gate) | ~0.5-1 day |
| Presenter | ~0.5 day |
| **Total** | **~3-5 days wall-clock** |

## Predecessor / parent chain

- **Parent**: backtest-vs-live execution gap (long-running theme;
  cited in `spec/product.md § Strategy lifecycle`)
- **Predecessor**: `v5-latency-slippage-sim-v0.2.0-anchor-migration v0.1.0`
  (shipped 2026-05-27, commits `d2cc343`, `c223d11`, `4dfa2d8`,
  `d191227`). v0.2.0 R-O1 SHIP path explicitly spawned this brief
  via the operator-approved Ship Route (a) partial migration.
- **Sibling**: `vol-killswitch-overlay-noop-fix` (Bug #65) — its
  e2e divergence test stays green under R6.
- **Successor (probable)**: `v0.4-square-root-market-impact` and/or
  `intrabar-fill-sampling` (both deferred from v0.1.0 D3 / ADR-0043
  Alternatives Rejected) — spawned only on operator request, not
  auto-spawned by this brief.

## Cross-references

- v0.2.0 brief — [`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md`](../v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md)
- v0.2.0 Sharpe-delta table (the 8-group breakdown identifying unwired paths) — [`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md`](../v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md)
- v0.2.0 M-FINAL test report (documenting the t1937 whitelist) — [`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/test-final-2026-05-27-v5-latency-slippage-sim-v0.2.0-anchor-migration.md`](../v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/test-final-2026-05-27-v5-latency-slippage-sim-v0.2.0-anchor-migration.md)
- v0.1.0 brief — [`spec/v5-latency-slippage-sim/feature.md`](../v5-latency-slippage-sim/feature.md)
- ADR-0043 (engine D1-D5) — [`spec/architecture/adr/0043-simulated-latency-and-slippage.md`](../architecture/adr/0043-simulated-latency-and-slippage.md)
- ADR-0045 (canonical config + namespace strategy) — [`spec/architecture/adr/0045-v5-canonical-config-and-noop-baseline-namespace.md`](../architecture/adr/0045-v5-canonical-config-and-noop-baseline-namespace.md). v0.3.0 amends or extends D2 + D4 depending on Q1 route.
- Anchors file (target of migration) — [`spec/anchors.toml`](../anchors.toml)
- t1937 test (R3 target) — `crates/reports/tests/strategy_anchors_unchanged.rs`
- Sim wiring reference (sole wired path today) — `crates/backtest/src/scenarios/momentum.rs` lines 390, 438, 555
- Scenario-input structs (R1 plumbing targets) — `crates/backtest/src/cli_types.rs` lines 124-211
- Tasks — [`spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/tasks.md`](tasks.md)
- Trace row — `REQ-V5-FULL-PATH-WIRING-001` in [`spec/trace.toml`](../trace.toml)
- Verify script — [`scripts/verify_anchors.sh`](../../scripts/verify_anchors.sh) (the namespace-aware Bash pattern R3 mirrors in Rust)

## Design

> Architect M-T1 2026-05-27 — partial design lock. Q1 still open
> (operator-decide); D4 contract conditionally locked per route.
> Q2-Q5 lean toward analyst-recommended defaults.
> Full ADR: [`spec/architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md`](../architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md).

### T-AR-1 — K2 reachability probe verdict: K2-REACHABLE-CHEAP

The synthetic-vs-Parquet auto-switch for Group A (SMA/Composed) lives
at `crates/backtest/src/main.rs:977-1020` in the `else`-arm of the
multi-symbol dispatch:

```rust
let parquet_dir = data_root
    .join(scenario.symbol.to_string())
    .join(scenario.start_year.to_string());
let has_parquet = parquet_dir.exists()
    && std::fs::read_dir(&parquet_dir)
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);
if has_parquet {
    // load Parquet
} else {
    // synthetic_bars(...)
}
```

This auto-switch is NOT entered by momentum, pairs, tcn-overlay, or
realdata scenarios — they branch earlier on `ScenarioDataSource`
(statically declared per scenario at `Scenario::from_name` lines
105-461). It is reached purely by single-symbol (SMA/Composed)
scenarios when Parquet exists on disk (which it does today —
`data/binance/BTCUSDT/2023/01.parquet` … `12.parquet` present).

**Route (a) is cheap and reachable** with a 5-LoC CLI flag:

```rust
// crates/backtest/src/main.rs Args struct
#[arg(long, default_value_t = false)]
force_synthetic_bars: bool,
```

```rust
// main.rs:977
let has_parquet = !args.force_synthetic_bars
    && parquet_dir.exists()
    && std::fs::read_dir(&parquet_dir)
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);
```

**Verdict: K2-REACHABLE-CHEAP** — route (a) is alive for operator Q1
at trivial cost (~5 LoC, no refactor, no struct changes, no test
contract breakage). The K2 risk row in § K is therefore retired;
ADR-0047 D1 codifies the verdict.

### T-AR-2 — Per-path plumbing audit (R1 of the brief)

| # | Strategy | ScenarioInput struct | `run()` site | Wiring LoC | Cross-path note |
|---|----------|---------------------|-------------|------------|------------------|
| 1 | SmaComposed (5 scenarios) | `SmaScenarioInput` (`crates/backtest/src/cli_types.rs:124`) | `sma_composed_run::run` at `crates/backtest/src/scenarios/sma_composed_run.rs:298` | ~8 (add field; thread through fill loops at ~505-540) | Independent — `LatencySlippageSimConfig` field is NEW for `SmaScenarioInput` (likely no — that's the gap) |
| 2 | Pairs (2 scenarios) | `PairsScenarioInput` (`crates/backtest/src/cli_types.rs:178`) | `pairs::run` at `crates/backtest/src/scenarios/pairs.rs:67` | ~10 (pairs has 4-symbol universe; fill loop more complex than SMA) | Independent — field NEW on `PairsScenarioInput` |
| 3 | TcnOverlay (4 scenarios) | `TcnScenarioInput` (`crates/backtest/src/cli_types.rs:195`) | `tcn_overlay::run` at `crates/backtest/src/scenarios/tcn_overlay.rs:69` | ~8 | Shares struct — field added ONCE in `TcnScenarioInput` auto-propagates to #4-#6 |
| 4 | TcnOverlayWeights (4 scenarios) | `TcnScenarioInput` (shared with #3) | `tcn_overlay_weights::run` at `crates/backtest/src/scenarios/tcn_overlay_weights.rs:31` | ~8 (still needs its OWN fill-loop wiring) | Shares struct with #3 |
| 5 | PatchTstOverlay (1 scenario) | `TcnScenarioInput` (shared) | `patchtst_overlay_weights::run` at `crates/backtest/src/scenarios/patchtst_overlay_weights.rs:46` | ~8 | Shares struct with #3-#4 |
| 6 | VolTarget / GarchVolOverlay (1 scenario) | `TcnScenarioInput` (shared) | `garch_vol_target_overlay::run` at `crates/backtest/src/scenarios/garch_vol_target_overlay.rs:105` | ~8 | Shares struct with #3-#5 — clarifies the Group F vs G ambiguity in the brief: `garch_vol_target_overlay` IS a strategy run with an equity surface, IN SCOPE |
| 7 | ThresholdSweep (analysis) | `TcnScenarioInput` (shared) | `threshold_sweep::run_cell` at `crates/backtest/src/scenarios/threshold_sweep.rs:56` | DEFERRED — 2D analysis sweep emits CSV (no equity surface); sim has no meaningful effect | Confirmed out of scope per ADR-0047 D2 |

**Production LoC ~42 + CLI flag ~5 + plumbing tests ~30 = ~77 LoC.**
Aligns with the v0.2.0 Wave A estimate of ~1 day.

**Helper consolidation lock**: `sim_slippage_cost` currently lives
inside `crates/backtest/src/scenarios/momentum.rs:551` as a private
function. Per ADR-0047 D2, the helper is lifted to a new module
`crates/backtest/src/scenarios/sim.rs` with the same signature.
Momentum and the 6 new wiring sites all `use` it; grep enforcement
at tester M-FINAL (`grep -r "fn sim_slippage_cost" crates/backtest/src`
must return exactly 1 line). Lift is behaviour-preserving and
anchor-additive per ADR-0038 § D6.a.

### T-AR-3 — t1937 fix: namespace-aware Rust resolver (R3 / Q4)

Locked per analyst Q4 default = (b) namespace-aware (architect
concurs). Pattern mirrors `scripts/verify_anchors.sh:63-110`.
The existing `STRATEGY_ANCHORS` constant table at
`crates/reports/tests/strategy_anchors_unchanged.rs:41-77` STAYS
pinned to noop-baseline SHAs forever. A NEW `CANONICAL_STRATEGY_ANCHORS`
table is added with the post-R1+R2 SHAs (populated at Wave C close).
The test fans out to both, calling
`find_backtest_report(scenario, Namespace::Noop)` and
`find_backtest_report(scenario, Namespace::Canonical)`. Full
contract in ADR-0047 D3.

### T-AR-4 — ADR decision: ADR-0047 (new ADR, not amendment)

Decision rationale: the per-path wiring contract + namespace-aware
resolver are tightly coupled (the resolver only makes sense after
the 6 paths emit canonical reports), and the K2 verdict warrants a
durable architectural record. A standalone ADR is clearer than a
multi-section amendment to ADR-0045. ADR-0047 references ADR-0045
D2 + D4 as amended-by-extension, not superseded.

### T-AR-5 — Cross-feature e2e re-check inventory (R6 / Q5)

Confirmed unchanged at 3 + 1 meta-gate (per analyst expectation):

| File | Role |
|------|------|
| `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` | Vol-targeting overlay ≥ 1 bp divergence |
| `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs` | Vol-killswitch overlay ≥ 1 bp divergence |
| `crates/strategy/tests/latency_slippage_sim_e2e.rs` | This feature's e2e |
| `crates/strategy/tests/overlay_hygiene_gate.rs` *(meta)* | Audits that every overlay HAS a divergence test |

No new overlay shipped between 2026-05-27 v0.2.0 ship and v0.3.0
brief author time. R6 scope confirmed at 3 + 1 meta.

### Q2-Q5 default-confirm

- **Q2 (Wave ordering)** = (a) all-then-emit — architect concurs.
- **Q3 (Anchor namespace)** = (a) extend `v5-realdata-medium-2026-05` — architect concurs.
- **Q4 (t1937 fix)** = (b) namespace-aware — architect concurs (see T-AR-3 above).
- **Q5 (Cross-feature re-check)** = (a) re-run all — architect concurs (see T-AR-5 above).

## Implementation

Developer M-DEV closed 2026-05-27.

### Wave A — sim.rs lift + 6-path plumbing

- `crates/backtest/src/scenarios/sim.rs` created. `sim_slippage_cost` lifted from `momentum.rs:551` (byte-identical body). Grep gate: exactly 1 definition of `fn sim_slippage_cost` in `crates/backtest/src/`.
- `latency_slippage_sim: LatencySlippageSimConfig` added to `SmaComposedRunInput`, `PairsScenarioInput`, `TcnScenarioInput` in `cli_types.rs`. All 6 strategy fill loops wired with `sim_slippage_cost` (Buy: `cash -= sim_slip_cost`; Sell: `cash -= sim_slip_cost`).
- `--force-synthetic-bars` CLI flag (~5 LoC) added to `crates/backtest/src/main.rs`; guards `has_parquet` predicate per Q1=(a).
- All construction sites updated in `engine.rs` (7), `main.rs` (10+), `bin/run_yahoo_sma.rs` (1), `bin/threshold_sweep.rs` (3), `crates/ui/tests/` (2).
- 6 plumbing unit tests added to `cli_types.rs` under `latency_slippage_config_tests`.

### Wave B — Canonical re-emission (11 synthetic reports)

- 11 canonical reports emitted under `{ 30, 80, 8 }` to `spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/`.
- Group A (SMA): `--force-synthetic-bars` used (Q1=(a)). SMA cross: equity $47k → $17.9k (8bps × 12,077 fills = -$29.3k).
- Group D (Pairs): newly wired; `pairs-2023-zscore-mr` SHA changed from noop-identical to `01c9da4d`.
- Group E (TCN overlay synthetic): newly wired; `top10-2023-fy-tcn-overlay` SHA changed from noop-identical to `1460fcc7`.
- Realdata/candle scenarios: not re-emitted (feature absent in default build); SHA unchanged.
- Determinism verified: 2 independent runs of each scenario produced identical body-SHAs.

### Wave C — Anchor SHA migration (9 rows updated in anchors.toml)

- `spec/anchors.toml` v5 v0.3.0 canonical section updated per Q3=(a) (same `v5-realdata-medium-2026-05` pin, new SHAs for 9 changed scenarios).
- `scripts/verify_anchors.sh` extended: v0.3.0 migration dir added as preferred source; noop exclusion pattern widened to exclude all `v5-latency-slippage-sim-v0.*.0-*` dirs.
- Final: `ANCHORS PASS (69 / 69)`.

### Wave D — Sharpe-delta table

- `reports/sharpe-delta-table-2026-05-27.md` written with full 11-group breakdown.
- K1 surprise scan: **0 surprises** across all 69 scenarios.

### Wave E — t1937 namespace-aware resolver

- `crates/reports/tests/strategy_anchors_unchanged.rs` rewritten with `Namespace` enum, `CANONICAL_FEATURE_DIRS`, `is_canonical_path`, `find_backtest_report` with namespace filter, and `CANONICAL_STRATEGY_ANCHORS` table (11 entries).
- `t1937_nine_strategy_anchors_unchanged` — GREEN.
- `t1937b_canonical_strategy_anchors_unchanged` — GREEN (new test, populated).
- `t1942_anchor_shas_are_well_formed_64_lowercase_hex` — GREEN.

### Wave F — Cross-feature e2e re-checks

- `latency_slippage_sim_e2e.rs`: 3/3 pass.
- `vol_targeting_overlay_end_to_end.rs`: 1/1 pass.
- `vol_killswitch_overlay_end_to_end.rs`: 4/4 pass.

## Verification

_Tester M-FINAL links to reports here after developer M-DEV close._

## Changelog

- 2026-05-27 (analyst): feature.md v0.1.0 authored. **6 R / 6 K /
  4 H / 5 Q** + non-regression contract + pre-drawn 4-cell verdict
  tree + cost framing. Q1 (Group A data-source) is the load-bearing
  question with no safe analyst default — the only Q in v5 series
  history requiring explicit operator judgment. Q2-Q5 standing-
  Autoapprove-eligible. Closes the v0.2.0 R-O1 Ship Route (a)
  partial migration follow-on commitment. ANCHORS PASS (69/69)
  pre-spec confirmed; `scripts/spec_lint.py` requires Python 3.11+
  (not available on analyst environment Python 3.9.6) — orchestrator
  re-runs at M0 close on a Python 3.11+ host.
- 2026-05-27 (architect M-T1): feature.md v0.2.0. § Design section
  filled with K2-REACHABLE-CHEAP verdict (5-LoC CLI flag at
  `main.rs:977-1020`), per-path plumbing audit (~77 LoC total), and
  Q2-Q5 architect-concur-with-analyst-defaults. Q1 reframed with K2
  cost picture inline — architect lean = (a) revert to synthetic
  baseline (now cheap; preserves friction-free oracle for 5/68
  anchors). ADR-0047 authored covering D1 (K2 verdict) + D2 (per-path
  contract + `sim_slippage_cost` shared-location lift to
  `crates/backtest/src/scenarios/sim.rs`) + D3 (namespace-aware Rust
  resolver) + D4 (conditional Group A re-emission per Q1) + D5
  (anchor namespace) + D6 (e2e inventory confirmed at 3 + 1).
  Frontmatter flipped `owner: analyst → operator-decide`. Q1 still
  genuinely operator judgment (value tradeoff, not cost-blocked).
