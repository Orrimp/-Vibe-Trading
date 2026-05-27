---
title: Sprint Review — v5 latency-slippage-sim v0.3.0 full-path wiring
feature: v5-latency-slippage-sim-v0.3.0-full-path-wiring
version: v0.1.0
mode: release
verdict: SOFT-PASS
commit: 61db5f9
date: 2026-05-27
audience: operator
---

# v5 v0.3.0 — full-path wiring sprint review (2026-05-27)

## TL;DR

v0.3.0 closes the v0.2.0 scope gap: realistic latency + slippage friction
(30–80 ms delay, 8 bps per fill) is now applied across **6 strategy paths**
(was 1 at v0.2.0). Eleven canonical reports re-emitted, nine anchor SHAs
overwritten in-place, **zero alpha-inversion (K1) surprises**, t1937 test
flipped GREEN via a namespace-aware resolver. Tester verdict
**SOFT-PASS** — eight candle/realdata-feature-gated scenarios are wired in
code but need a feature-flagged rebuild to emit friction-bearing SHAs;
deferred to v0.4.0.

## What changed

- **Friction is now real for 11 of 11 runnable scenarios** (5 SMA/Composed,
  2 Pairs, 2 TCN-overlay synthetic, 2 Momentum unchanged) — at v0.2.0,
  only momentum's 2 scenarios carried real friction; the other 12
  canonical SHAs were byte-identical to their friction-free baseline.
- **One CLI flag, one operator decision** — `--force-synthetic-bars` lets
  Group A (SMA/Composed) opt out of the runtime's auto-detect of real
  Binance Parquet data. Operator picked Q1=(a) revert-to-synthetic so the
  noop-vs-canonical comparison stays apples-to-apples (friction is the
  only variable, not also the data source).
- **t1937 anchor-stability test is no longer a constant-update treadmill**
  — the Rust resolver now mirrors the Bash `verify_anchors.sh`
  namespace-aware walk, so future canonical re-emissions don't re-break it.

## Why

After v0.2.0 shipped on the morning of 2026-05-27, the load-bearing
discovery was that 5 of 7 strategy construction paths (SMA, Pairs, TCN
overlay × 4 variants) had `LatencySlippageSimConfig` *declared* but never
*applied* — the canonical SHAs in `spec/anchors.toml` were silently equal
to their friction-free baselines. The v5-realdata-medium-2026-05 pin
looked like a regression gate but was actually 32-of-34 noop. v0.3.0 turns
the pin into what it claimed to be: a real friction gate.

## What the operator can do now

Three new operator-visible behaviours:

1. **Run any of the 6 newly-wired strategies under realistic friction** —
   the canonical `{ 30, 80, 8 }` config is the default in the `v5-realdata-medium-2026-05` namespace. Example invocation (mirrors what produced the
   shipped canonical reports):
   ```bash
   target/release/backtest \
       --scenario btc-2023-1m-sma-cross \
       --seed 0xC0FFEE \
       --force-synthetic-bars \
       --sim-latency-ms-min 30 --sim-latency-ms-max 80 --sim-slippage-bps 8 \
       --reports-dir /tmp/v5
   ```

2. **Opt-in/out of the real-Binance auto-detect** via
   `--force-synthetic-bars`. Without the flag, single-symbol scenarios
   (SMA/Composed) pick up Parquet data when it exists on disk — that's
   how Group A drifted at v0.2.0. With the flag, the runtime is pinned to
   the seeded synthetic GBM and no future data-source upgrade silently
   shifts the anchors.

3. **Trust the anchor gate again** — `bash scripts/verify_anchors.sh` is
   PASS 69/69, and `t1937_nine_strategy_anchors_unchanged` is back to
   GREEN. Both verifying the same invariant from two angles (Bash + Rust),
   now in symmetric namespace-aware shape.

## Live demo — anchor gate (re-run for this presentation)

Command:
```
bash scripts/verify_anchors.sh
```

Tail of output (full run is 69 rows, last 5 shown):
```
PASS  sharpe-comparison-vol-target-bs1-realdata  d21db467f1d25c36de78b405aa950c9025d61b03cb43952ccb7aadefed701a31
PASS  sharpe-comparison-vol-target-bs1-realbaseline  ff2b934961f8cea87c2e44953a746dba3f3b732c42a997c501bbcc3b989d95e9
PASS  btc-yahoo-2024-1d-sma-cross           8045623b4c9b7d9e25e3b53156bd64363d87e575a2f9c4cb0d8b291ae7bb4867
---
ANCHORS PASS  (69 / 69)
```

Determinism spot-check (from the tester report — two independent re-runs
of newly-wired scenarios both reproduced the shipped SHA byte-for-byte):

| Scenario | Computed SHA | anchors.toml SHA | Match |
|----------|-------------|------------------|-------|
| `pairs-2023-zscore-mr` | `01c9da4d…65e76e` | `01c9da4d…65e76e` | PASS |
| `top10-2023-1h-momentum` | `0f6f6eb8…1a8ba3` | `0f6f6eb8…1a8ba3` | PASS |

## The Sharpe-delta story — v0.2.0 → v0.3.0

At v0.2.0, the canonical pin claimed to cover 34 scenarios; in honest
accounting, **only 2 scenarios had real friction-only deltas**. v0.3.0
brings that to **11 scenarios** in the default build (and 19 once the
candle/realdata features rebuild on v0.4.0).

| Slice | v0.2.0 friction-real scenarios | v0.3.0 friction-real scenarios |
|---|---:|---:|
| Momentum (Group B) | 2 | 2 (unchanged) |
| SMA/Composed (Group A) | 0 *(real-data drift, not sim)* | 5 |
| Pairs (Group C) | 0 *(noop-identical)* | 2 |
| TCN overlay synthetic (Group D) | 0 *(noop-identical)* | 2 |
| Candle/realdata-gated (Groups E–H) | 0 | 0 *(wired but feature-flag dormant; v0.4.0)* |
| **Honest total** | **2** | **11** |

Two representative rows from the full table at
[`reports/sharpe-delta-table-2026-05-27.md`](../reports/sharpe-delta-table-2026-05-27.md):

| Scenario | Noop equity | Canon equity | Δ equity | K1 | Driver |
|----------|-------------|--------------|----------|----|--------|
| btc-2023-1m-sma-cross | $47,290.03 | $17,992.64 | -$29,297.39 | 0 | v5-sim+Q1 (synthetic, 12,077 fills × 8 bps) |
| top10-2023-fy-tcn-overlay | $30,235.58 | $28,347.99 | -$1,887.59 | 0 | v5-sim (1,224 fills × 8 bps) |

SMA-cross took a ~$29 k hit on a $100 k notional because it's a
high-frequency synthetic crossover — 12,077 fills × 8 bps slippage is a
textbook example of "friction matters when you trade a lot". TCN overlay
trades ~10× less and takes a ~$1.9 k hit. Both are economically sensible;
neither flips Sharpe sign.

**K1 surprise scan: 0 surprises across all 69 scenarios.** No strategy
that was profitable under noop has become unprofitable under canonical
friction. Hypothesis H1 (≤ 3 flipped) holds with margin.

Cross-link to the v0.2.0 table for the contrast:
[`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md`](../../v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md).

## Why it matters

- **Regression gate semantics restored.** The `v5-realdata-medium-2026-05`
  pin is now a real friction-only gate for 11 of the 34 anchored
  scenarios (it was effectively 2 of 34 at v0.2.0). Future code changes
  to fill / exec / accounting will trip the gate if they perturb
  friction handling — which is what the pin existed to do.
- **Data-source drift becomes operator-opt-in.** v0.2.0's surprise (real
  Binance data sneaking into Group A) cost ~30 min of triage. v0.3.0's
  `--force-synthetic-bars` flag means future similar drift requires an
  explicit opt-in, not a silent runtime check.
- **One less recurring fire.** The t1937 test failure pattern (hardcoded
  SHA constants + lexicographic newest-report resolution) has resurfaced
  in every anchor migration since 2026-04. The namespace-aware resolver
  pattern shared with `verify_anchors.sh` ends that pattern.

## Architecture call-outs

Authored as ADR-0047
([`spec/architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md`](../../architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md)):

- **D1 — K2-REACHABLE-CHEAP verdict.** The synthetic-vs-Parquet
  auto-switch is a 5-LoC CLI flag; route (a) for Q1 was always
  affordable. ADR-0047 D1 codifies it.
- **D2 — per-path plumbing contract.** `sim_slippage_cost` lifted to a
  shared module `crates/backtest/src/scenarios/sim.rs`; grep gate
  enforces exactly one definition workspace-wide.
- **D3 — namespace-aware Rust resolver.** `Namespace::Noop` /
  `Namespace::Canonical` fan-out; new `CANONICAL_STRATEGY_ANCHORS` table
  with 11 v0.3.0 SHAs. Mirrors `verify_anchors.sh:63-110`.
- **D4 — conditional Q1 re-emission.** Group A re-emitted under
  `--force-synthetic-bars` per Q1=(a). ADR locks the conditional.
- **D5 — same canonical namespace pin.** Q3=(a) extend
  `v5-realdata-medium-2026-05`; in-place SHA overwrite. Anchor row count
  stays at 69.
- **D6 — e2e inventory unchanged at 3 files.** Vol-targeting +
  vol-killswitch + latency-slippage-sim overlay divergence tests all
  re-PASS at ≥ 1 bp.

## SOFT-PASS carve-out — honest

Eight scenarios are wired in code but their canonical SHAs in
`anchors.toml` remain byte-identical to their noop baselines because the
default CI binary is built without the `candle` / `realdata` features.
The strategy executable paths exist; they just don't run without the
feature flags. These are deferred to v0.4.0, which is a feature-flagged
rebuild, not new plumbing:

| Family | Scenarios | Reason |
|--------|-----------|--------|
| TCN overlay weights | 2 | `--features candle` absent |
| TCN overlay realdata | 4 | `--features realdata` absent |
| PatchTST overlay | 1 | `--features realdata candle` absent |
| Vol-target GARCH overlay | 1 | `--features realdata` absent |

K1 risk for these 8 is zero (a $0 delta cannot flip Sharpe sign). The
tester's verdict tree explicitly anticipated this carve-out as an R-O1
SHIP sub-condition.

## Verification matrix

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| R1 | `LatencySlippageSimConfig` plumbed into 6 strategy construction paths | VERIFIED | `crates/backtest/src/scenarios/sim.rs` + 6 fill-loop wires; ADR-0047 D2 grep gate PASS |
| R2 | Q1 Group A data-source decision honored | VERIFIED | `data_source: synthetic (seeded RNG, v0 fallback)` in `backtest-20260527-181323-btc-2023-1m-sma-cross.md` frontmatter |
| R3 | t1937 flipped GREEN via namespace-aware resolver | VERIFIED | `cargo test -p reports --test strategy_anchors_unchanged` → 3 passed (t1937, t1937b, t1942) |
| R4 | 9 SHAs overwritten in-place at same `v5-realdata-medium-2026-05` pin | VERIFIED | `spec/anchors.toml` lines 308-398; `verify_anchors.sh` PASS 69/69 |
| R5 | Sharpe-delta table extended; K1 scan = 0 | VERIFIED | `reports/sharpe-delta-table-2026-05-27.md` § Summary |
| R6 | 3 overlay e2e tests re-PASS at ≥ 1 bp | VERIFIED | latency_slippage_sim_e2e 3/3, vol_targeting_overlay 1/1, vol_killswitch_overlay 4/4 |
| R-NR.1 | Anchor gate PASS post-migration | VERIFIED | live re-run quoted above: `ANCHORS PASS (69 / 69)` |
| R-NR.2 | No new panics / crashes | VERIFIED | All 11 re-emitted reports run cleanly; determinism spot-check PASS |
| R-NR.3 | Cross-feature e2e ≥ 1 bp divergence | VERIFIED | R6 evidence |
| R-NR.4 | No NEW workspace test failures + t1937 GREEN | VERIFIED | tester § 3b: 1 pre-existing flake (`h3_in_memory_equals_cached_disk`) unchanged; t1937 flipped |
| R-NR.5 | `crates/exec`/`crates/cost`/`crates/audit` untouched | VERIFIED | commit `21bda41` stat — only `crates/backtest/` + `crates/reports/tests/` + `crates/ui/tests/lab_markers_anchor.rs` |
| R-NR.6 | `AuditEvent::SimulatedExecMetrics` emitted by 6 paths (was 1) | VERIFIED | by construction — sim_slippage_cost callers expanded to 6 |
| H1 | ≤ 3 flipped scenarios under canonical friction | HOLDS | 0 K1 surprises across 69 scenarios (margin of 3) |

## Numbers that matter

- **Anchor count:** 69 / 69 PASS (34 noop-baseline + 35 canonical, of
  which 9 SHAs overwrote v0.2.0 placeholders).
- **Newly-wired strategy paths:** 6 (SmaComposed, Pairs, TcnOverlay,
  TcnOverlayWeights, PatchTstOverlay, GarchVolOverlay).
- **Scenarios with real friction-only deltas:** 11 (was 2 at v0.2.0;
  +9 net).
- **K1 alpha-inversion surprises:** 0 / 69.
- **Workspace test failures:** 1 pre-existing flake
  (`h3_in_memory_equals_cached_disk`, whitelisted since 2026-05-26);
  zero new.
- **Production LoC touched:** ~77 (~42 production wiring + ~30 plumbing
  tests + ~5 CLI flag).
- **Clippy:** clean in `crates/backtest/`; pre-existing-only warnings
  elsewhere (carried from v0.2.0 baseline).
- **Spec-lint:** 72 violations in 3 categories — identical to v0.2.0
  baseline; no regression introduced.
- **Wall-clock spend (full feature):** analyst + architect + 6 dev waves
  + tester + presenter, all closed same-day 2026-05-27.

## Open decisions

None for this approval. v0.3.0 is a binary ship-or-reject. Two items are
*candidates* for a follow-up v0.4.0 brief (operator-discretionary; not
requested by this deck):

- Re-emit the 8 candle/realdata-feature-gated scenarios under a
  feature-flagged rebuild to produce friction-bearing SHAs.
- Square-root market-impact + intrabar-fill-sampling — both deferred
  from v0.1.0 (ADR-0043 Alternatives Rejected).

## What's next

If approved, v0.3.0 ships and closes the v5 anchor-migration arc.
`spec/backlog.md` moves the row from Active → Recent and the feature
frontmatter flips `status: draft → shipped`. v0.4.0 candidates above
spawn only on explicit operator request.

If rejected, the feedback note routes back to the relevant agent and the
ship is held.

## Approval

- [x] Approved — ship  _(2026-05-27, operator)_
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

<!-- operator notes go here -->

## References

- Feature brief: [`spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/feature.md`](../feature.md)
- Tasks: [`spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/tasks.md`](../tasks.md)
- M-FINAL tester report: [`reports/test-final-2026-05-27-v5-latency-slippage-sim-v0.3.0-full-path-wiring.md`](../reports/test-final-2026-05-27-v5-latency-slippage-sim-v0.3.0-full-path-wiring.md)
- Sharpe-delta table (v0.3.0): [`reports/sharpe-delta-table-2026-05-27.md`](../reports/sharpe-delta-table-2026-05-27.md)
- Sharpe-delta table (v0.2.0, for comparison): [`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md`](../../v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md)
- ADR-0047 (this feature's contract): [`spec/architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md`](../../architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md)
- ADR-0045 (canonical config + namespace strategy): [`spec/architecture/adr/0045-v5-canonical-config-and-noop-baseline-namespace.md`](../../architecture/adr/0045-v5-canonical-config-and-noop-baseline-namespace.md)
- ADR-0043 (sim engine D1-D5): [`spec/architecture/adr/0043-simulated-latency-and-slippage.md`](../../architecture/adr/0043-simulated-latency-and-slippage.md)
- Commit: `61db5f9` (tester M-FINAL SOFT-PASS); developer wave A-F: `21bda41`
