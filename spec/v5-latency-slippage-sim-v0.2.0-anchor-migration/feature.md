---
slug: v5-latency-slippage-sim-v0.2.0-anchor-migration
version: 0.1.0
status: draft
owner: analyst
updated: 2026-05-27
predecessor: v5-latency-slippage-sim v0.1.0 (shipped commit a5f8647 2026-05-26)
parent: backtest-vs-live-execution-gap
priority: P1
---

# v5 latency-slippage-sim v0.2.0 — anchor migration to canonical non-zero friction

> The feature **version** being shipped by this brief is **v5
> v0.2.0** (the simulator config flips from default-zero noop to a
> canonical non-zero friction). The brief document itself starts at
> v0.1.0 (analyst-authored).

## Why now (full context)

v5-latency-slippage-sim v0.1.0 shipped on **2026-05-26 (commit
a5f8647)** under a 5-wave M-DEV. The default config is **zeros** for
all three knobs (`latency_ms_min = 0`, `latency_ms_max = 0`,
`slippage_bps = 0`), so all 34 SHA-256 anchors in
[`spec/anchors.toml`](../anchors.toml) stayed byte-identical — the
non-negotiable Wave A acceptance gate
(`bash scripts/verify_anchors.sh` → `ANCHORS PASS (34 / 34)`).

But **default-zero is operationally meaningless**. We shipped the
engine without using it. From the v0.1.0 brief
[`spec/v5-latency-slippage-sim/feature.md`](../v5-latency-slippage-sim/feature.md)
§ Operator-decide table, Q5 verbatim:

> | Q5 | Anchor migration timing | (a) defer to v0.2.0 separate brief
> / (b) bundle into this brief | **(a) defer** | Migrating 34 anchors
> to non-zero values is a load-bearing operator decision deserving
> its own brief. v0.1.0 ships the simulator; v0.2.0 decides what
> enabled values produce the canonical paper-trading reports |

v0.1.0's verdict tree R-O1 (`spec/v5-latency-slippage-sim/feature.md`
§ Pre-drawn verdict routing tree) explicitly mandates the spawn:

> R-O1 | All 5 R rows green + R-NR.1 34/34 anchors + R5 e2e divergence
> ≥ 1 bp | **SHIP** v0.1.0 + spawn v0.2.0 anchor-migration brief

**This is that brief.** v0.2.0 is the load-bearing decision: pick a
canonical non-zero config representing "realistic production
friction" and re-emit the anchored reports under that config. Every
alpha number in [`spec/anchors.toml`](../anchors.toml) then represents
a strategy's edge UNDER simulated friction — closing the backtest-vs-
live gap by construction, not just by virtue of having the engine
compiled in.

## Scope (v0.2.0)

### R1 — Canonical config selection [load-bearing — Q1]

Pick one of three friction profiles for the new canonical
`LatencySlippageSimConfig`:

| Profile | `latency_ms_min` | `latency_ms_max` | `slippage_bps` | Maps to |
|---|---|---|---|---|
| **tight** | 20 | 50 | 3 | Premium retail / colocated; tight spread venues |
| **medium** *(analyst-recommended)* | 30 | 80 | 8 | Realistic crypto-spot retail ca. 2024-2025 (Binance / Coinbase / Kraken non-API-tier-3) |
| **aggressive** | 50 | 150 | 15 | Stress test: thin alt-coin venues / volatile sessions |

**Analyst recommendation: (b) medium** — matches the typical latency
+ slippage band a sophisticated retail operator faces on the
top-3 crypto-spot venues under normal market conditions.

Final selection is operator-decide at Q1.

### R2 — Re-emit all 34 anchored backtest reports under the canonical config

Every report file referenced by the 34 anchors in
[`spec/anchors.toml`](../anchors.toml) is re-generated under the
chosen canonical config. Reports include:

- 9 v0/v0.5/v1 strategy scenarios (sma-cross, macd-trend,
  rsi-reversion, bbands-mean-revert, momentum 2023/2024, pairs zscore
  2023/2024 + sma-baseline-refresh)
- 2 operator-success reports (`report-sample-7d`,
  `report-sample-90d`)
- 23 v2.5 TCN / v2.5 PatchTST / overlay anchors (top10-2023-fy-tcn-
  overlay, etc.) — exact count to be cross-checked by architect from
  the file (currently 34 total anchored rows)

**Acceptance**: each report's body-SHA-256 (per
[`scripts/hash_report.py`](../../scripts/hash_report.py)) differs from
its v0.1.0-ship SHA — that's the **point** of v0.2.0; byte-identity
would mean the simulator is still noop.

### R3 — Anchor SHA migration in `spec/anchors.toml`

34 OLD SHA values → 34 NEW SHA values with a NEW namespace pin.
Proposed namespace tag (operator-decide via Q1 selection):

- `v5-realdata-medium-2026-05` (if Q1 = medium)
- `v5-realdata-tight-2026-05` (if Q1 = tight)
- `v5-realdata-aggressive-2026-05` (if Q1 = aggressive)
- `v5-realdata-ladder-2026-05` (if Q1 = ladder — see Q1 option (d))

The version field on each `[[anchors]]` row gets a fresh `v0.2.0`
bump (or a date-pin tag — architect picks the exact format in M-T1).

### R4 — Retire-or-keep decision for OLD noop anchors [Q2]

Three options:
- **(a) keep as `noop-baseline` namespace** *(analyst-recommended)* —
  the OLD 34 rows stay in `anchors.toml` under a segregated namespace,
  serving as the historical "pre-friction" reference. New 34 rows
  land alongside under the canonical namespace.
- (b) retire entirely — delete the OLD 34 rows.
- (c) keep + migrate forward (both visible at the same version pin).

Analyst-recommended: **(a) keep as noop-baseline** for
historical-evidence preservation and to enable A/B sanity checks
("did the migration land where we expected?"). Storage cost is one
file; clarity cost is zero given namespace separation.

### R5 — Sharpe / drawdown / final-equity delta table

NEW dev-note `spec/dev-notes/v5-anchor-migration-friction-delta-<DATE>.md`
quantifies the alpha shift across all 34 scenarios:

| Scenario | Sharpe (noop) | Sharpe (canonical) | Δ | Max DD (noop) | Max DD (canonical) | Δ | Final equity Δ % | Flipped? |
|---|---|---|---|---|---|---|---|---|

The "Flipped?" column flags scenarios that went profitable →
unprofitable under friction (load-bearing for K1). Tester / developer
co-author this from the M-DEV Wave A re-run output.

### R6 — CLAUDE.md non-negotiable wording check

The CLAUDE.md non-negotiable currently reads:

> Every strategy overlay or sizing-modifier ships with a baseline-
> equity-divergence end-to-end test from day 1. … The required gate
> is an e2e test that asserts the overlay's output equity diverges
> from the un-targeted baseline equity by ≥ 1 bp.

The "≥ 1 bp" threshold gates **divergence vs noop**. Under canonical
friction, divergence vs noop will be 50-500 bp (much larger). The
1-bp threshold remains valid — it gates "did the overlay actually
fire", not "how big is the canonical friction".

**No CLAUDE.md edit expected.** R6 is a check-and-confirm gate; if
analyst's reading is wrong, architect surfaces the amendment at M-T1.

### R7 — Cross-feature retrospective [load-bearing for cross-product]

Every overlay / sizing-modifier with an existing e2e divergence test
must re-verify under the new canonical config. Known scope:

- `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` (v3 vol overlay)
- `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs` (Bug #65)
- `crates/strategy/tests/latency_slippage_sim_e2e.rs` (v5 itself)
- TCN overlay e2e (v2.5)
- PatchTST overlay e2e (v25a — if shipped)
- position_curve overlay (if e2e present)

Architect surveys the actual list at M-T1; developer Wave D re-runs
each. The "1 bp divergence vs noop" assertion stays valid; the
**absolute equity values** in each test's "enabled" path now reflect
the canonical friction baseline.

### R-NR — Non-regression contract

- **R-NR.1** — OLD anchors retire (or move to `noop-baseline`
  namespace per Q2 = (a)); NEW 34 anchors lock at this brief's
  M-FINAL. **NOT byte-identical to v0.1.0-ship** — that's the point.
- **R-NR.2** — All 34 backtest scenarios still **PASS** (just at NEW
  SHA values). Every report runs to completion without panic /
  unwind; every scenario produces a valid report file.
- **R-NR.3** — Cross-feature e2e tests still PASS post-migration
  (R7 scope). The "≥ 1 bp divergence" assertion holds.
- **R-NR.4** — `cargo test --workspace --no-fail-fast` shows no
  NEW failures vs the v0.1.0-ship whitelist.
- **R-NR.5** — No code changes to `crates/exec`, `crates/cost`,
  `crates/audit`, `crates/strategy/src/` (library code). Migration
  is a config-default flip + report re-emission + anchor SHA bump.
  Test files may be touched only to update absolute expected values
  per R7.
- **R-NR.6** — Audit-ledger schema unchanged. The
  `AuditEvent::SimulatedExecMetrics` variant from v0.1.0 D4 is
  ALREADY in place; v0.2.0 just exercises it (the skip-when-zero
  guard no longer skips, since the canonical config is non-zero).

## K — Risk register

| K | Risk | Mitigation |
|---|---|---|
| **K1** | **Strategy alpha disappears under realistic friction** — some strategies that were profitable at 0 bps may show **negative Sharpe** under 8 bps slippage. Operator must decide whether to retire those strategies or accept the realistic picture. | R5 delta table surfaces flipped scenarios. Q3 codifies the retire-or-keep policy. |
| K2 | Operator can't tell which is the "real" anchor (old vs new namespace coexisting). | Q2 = (a) namespace separation; the canonical namespace is THE current reference; noop-baseline is historical only. README in `spec/anchors.toml` header gets a sentence per architect M-T1. |
| K3 | Migration breaks external tooling (e.g. operator dashboards, CI gates) that expects the OLD SHA values. | The `verify_anchors.sh` script is the only known consumer — it consumes the file structurally, not the SHA values themselves. No external tool surveyed at brief time; architect re-checks at M-T1. |
| K4 | v0.2.0 ships under K1 surprise — a strategy retires because of friction, but operator wanted to keep it. | Q3 = (b) per-scenario flag forces operator review before retirement. |
| K5 | Cross-feature anchor cascade — re-running `vol_killswitch_overlay_end_to_end` with friction might produce different Hold-emission counts → test invariants need re-anchoring. | R7 scope explicitly lists each affected e2e; developer Wave D re-runs and updates absolute expectations. |
| K6 | Re-emission run-time blows up — 34 scenarios × full backtest is 0.5-2 hours of CPU. | Wave A budgets 2-3 days; developer parallelizes with `cargo test --jobs N` or by-scenario shards. Watch-recipe per MEMORY.md `feedback_watch_recipe_for_long_running.md`. |
| K7 | TCN / PatchTST overlay anchors hit the `candle` feature gate — non-CI machines only. | Tester confirms M-DEV Wave A is run on the Apple Silicon canonical box (same precedent as v2.5 TCN second-lock); fall back to passthrough-forecaster anchors only if `candle` feature absent. |

## H — Hypotheses

| H | Hypothesis | Confidence | Falsifier |
|---|---|---|---|
| **H1** | **90 %+ of scenarios stay profitable under analyst-recommended canonical config (medium = 30..=80 / 8 bps)**; fewer under aggressive. | Medium-high | R5 delta table: count of "Flipped?=YES" rows / 34. If > 3, H1 falsified. |
| H2 | Sharpe drops by 0.2-0.5 across most scenarios; max drawdown grows by 1-3 percentage points under canonical config. | Medium | R5 delta table aggregate. |
| H3 | `vol_targeting_overlay`'s alpha advantage shrinks but doesn't invert under friction — this is its raison d'être. | High | R7 vol_targeting e2e re-run; if the overlay's "enabled" equity falls below the un-targeted baseline equity under canonical config, H3 falsified and a deeper review is warranted. |
| H4 | The `latency_slippage_sim_e2e` 1-bp divergence assertion still passes under canonical config (since 8 bps slippage produces ~30-100 bps equity divergence on the test scenario). | High | R7 re-run. |
| H5 | The CLAUDE.md non-negotiable threshold (≥ 1 bp) needs no edit — divergence semantics scale, threshold doesn't. | High | R6 check at M-T1; architect surfaces any amendment. |

## Operator-decide questions (Q1-Q4)

| Q | Topic | Options | Analyst-recommended default | Rationale |
|---|---|---|---|---|
| **Q1** | **Canonical config** | (a) tight (20..=50 / 3 bps) / **(b) medium (30..=80 / 8 bps)** / (c) aggressive (50..=150 / 15 bps) / (d) ladder — ship 3 namespaces at once for A/B comparison | **(b) medium** | Matches realistic crypto-spot retail venue conditions ca. 2024-2025 on top-3 venues. (a) understates real friction for non-colocated retail; (c) is a stress-test config, not a default. (d) is appealing but triples the migration cost and dilutes the "canonical" semantics. Re-decide at v0.3.0+ if ladder operationally needed. |
| **Q2** | **Retire-or-keep OLD noop anchors** | (a) **keep as `noop-baseline` namespace** / (b) retire entirely / (c) keep + migrate forward (both visible) | **(a) keep as noop-baseline** | Historical-evidence preservation; one-file storage cost; zero clarity cost with namespace separation. Enables A/B sanity checks at v0.2.0 ship review. |
| **Q3** | **Strategy retirement on K1 surprise** | (a) auto-retire any strategy that goes net-negative under canonical config / **(b) flag per scenario for operator review** / (c) ship all 34 even if some are net-negative | **(b) flag per scenario** | Auto-retire is irreversible without re-spec work; ship-all sweeps the K1 problem under the rug. Per-scenario review forces explicit acceptance. |
| **Q4** | **Cross-feature re-check budget** | (a) **re-run all overlay e2e tests under canonical config** / (b) re-run only the load-bearing ones (vol_targeting + vol_killswitch + tcn_overlay) / (c) defer all to v0.3 | **(a) re-run all** | Anchor cascade isn't optional — half-migrating leaves silent invariant drift in `crates/strategy/tests/`. Cost is ~2-3 dev-days vs ~6 weeks of accumulated drift if deferred. |

All 4 Qs are standing-Autoapprove-eligible at analyst-recommended
defaults. **Q1 is the most load-bearing** — it locks the canonical
friction profile every future strategy's alpha is measured against.

## Pre-drawn 4-cell verdict tree (presenter inherits)

| Cell | Condition | Route |
|---|---|---|
| **R-O1** | All 7 R rows green + R-NR.1-6 + H1 holds (≤ 3 flipped scenarios) + R-O3 no retirement candidates | **SHIP** v0.2.0 + spawn v0.3 follow-on briefs for square-root market-impact (D3 deferred) + intrabar fill sampling |
| **R-O2** | H1 holds but K1 retirement candidates surface (1-3 flipped scenarios) | **HOLD** — spawn per-scenario retirement briefs per Q3 = (b) operator review |
| **R-O3** | H1 violated (≥ 4 strategies inverted) | **Operator-decide**: ship the bad news (accept the more realistic alpha picture across the board) or refine canonical config (re-pick Q1 — e.g. (a) tight, then re-migrate) |
| **R-O4** | R-NR.2 fails — a scenario doesn't compile or run cleanly under canonical config | **REGRESSION** — developer iteration; HANDOFF → architect for root-cause; blocks ship |

## Cost framing

| Phase | Effort |
|---|---|
| Analyst (this brief) | ~0.5 day |
| Operator-decide (Q1-Q4) | ~15 min standing-Autoapprove |
| Architect M-T1 (R7 cross-feature survey + namespace convention + anchors.toml schema) | ~0.5 day |
| Developer Wave A — re-run 34 backtests under canonical config | ~2-3 days |
| Developer Wave B — anchor SHA migration in `spec/anchors.toml` | ~0.5 day |
| Developer Wave C — R5 Sharpe / DD / equity delta table | ~0.5 day |
| Developer Wave D — cross-feature e2e re-checks (R7) | ~2-3 days |
| Tester M-FINAL (verify all 34 NEW anchors + R7 cross-feature gates) | ~1 day |
| Presenter | ~0.5 day |
| **Total** | **~1-2 weeks wall-clock** |

## Predecessor / parent chain

- **Parent**: backtest-vs-live execution gap (long-running theme;
  cited in `spec/product.md § Strategy lifecycle`)
- **Predecessor**: `v5-latency-slippage-sim v0.1.0` (shipped
  2026-05-26, commit `a5f8647`) — locked the simulator engine + audit
  variant + e2e divergence test under default-zero noop
- **Sibling**: `vol-killswitch-overlay-noop-fix` (Bug #65) — its
  e2e divergence test must re-pass under canonical friction per R7
- **Successor (probable)**: `v0.3-square-root-market-impact` and/or
  `intrabar-fill-sampling` (both deferred from v0.1.0 D3 / ADR-0043
  Alternatives Rejected)

## Cross-references

- v0.1.0 brief — [`spec/v5-latency-slippage-sim/feature.md`](../v5-latency-slippage-sim/feature.md)
- v0.1.0 tasks — [`spec/v5-latency-slippage-sim/tasks.md`](../v5-latency-slippage-sim/tasks.md)
- ADR-0043 (D1-D5) — [`spec/architecture/adr/0043-simulated-latency-and-slippage.md`](../architecture/adr/0043-simulated-latency-and-slippage.md)
- Anchors file (migration target) — [`spec/anchors.toml`](../anchors.toml)
- Tasks — [`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/tasks.md`](tasks.md)
- Trace row — `REQ-V5-ANCHOR-MIGRATION-V0-2-0-001` in [`spec/trace.toml`](../trace.toml)
- CLAUDE.md non-negotiable — every overlay/sizing-modifier ships
  with a baseline-equity-divergence e2e test from day 1
- Pattern reference — `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`
- Verify script — [`scripts/verify_anchors.sh`](../../scripts/verify_anchors.sh)
- Hash recompute — [`scripts/hash_report.py`](../../scripts/hash_report.py)

## Changelog

- 2026-05-27 (analyst): feature.md v0.1.0 authored. **7 R / 7 K /
  5 H / 4 Q** + non-regression contract + pre-drawn 4-cell verdict
  tree + cost framing. Q1 (canonical config = medium) is the
  load-bearing default. v0.1.0's R-O1 SHIP path (commit `a5f8647`,
  2026-05-26) explicitly spawned this brief per Q5 = (a) defer.
