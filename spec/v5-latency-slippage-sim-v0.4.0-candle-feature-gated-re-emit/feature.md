---
slug: v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit
version: 0.1.0
status: draft
owner: analyst
updated: 2026-05-28
predecessor: v5-latency-slippage-sim-v0.3.0-full-path-wiring v0.1.0
parent: backtest-vs-live-execution-gap
priority: P1
---

# v5 latency-slippage-sim v0.4.0 — candle/realdata feature-gated re-emit

> Closes the v0.3.0 **SOFT-PASS carve-out**: 8 strategy scenarios whose
> plumbing was completed at v0.3.0 but whose canonical SHAs remain
> noop-identical because the default CI binary is built without
> `--features candle` and `--features realdata`. v0.4.0 performs the
> feature-flagged rebuild on the canonical Apple Silicon box and
> re-emits canonical reports under the same
> `LatencySlippageSimConfig { latency_ms_min: 30, latency_ms_max: 80, slippage_bps: 8 }`
> (ADR-0045 D1) so the friction sim actually fires through those paths.
> Anchor count stays at 70 — 8 SHAs change in-place under namespace
> `v5-realdata-medium-2026-05`.

## Why now

The v0.3.0 M-FINAL tester report (`spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/test-final-2026-05-27-v5-latency-slippage-sim-v0.3.0-full-path-wiring.md` § 8 Open Items) explicitly defers these 8 to v0.4.0:

| # | Scenario | Anchor namespace | Current canonical SHA (noop-identical) | Feature flag required |
|---|----------|------------------|----------------------------------------|----------------------|
| 1 | `top10-2023-fy-tcn-overlay-weights` | v5-realdata-medium-2026-05 | `7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4` | `candle` |
| 2 | `top10-2024-fy-tcn-overlay-weights` | v5-realdata-medium-2026-05 | `23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b` | `candle` |
| 3 | `top10-2023-fy-tcn-overlay-realdata` | v5-realdata-medium-2026-05 | `8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642` | `realdata` |
| 4 | `top10-2024-fy-tcn-overlay-realdata` | v5-realdata-medium-2026-05 | `fd8191dff1ca106ca24416a1819bd8a002c705da7f3747831f48d60733ee76f3` | `realdata` |
| 5 | `top10-2023-fy-tcn-overlay-weights-realdata` | v5-realdata-medium-2026-05 | `552d7df294bc93ff6f887874f919aeeb8106a62caae4ad5ec5de7c5b49665d70` | `candle realdata` |
| 6 | `top10-2024-fy-tcn-overlay-weights-realdata` | v5-realdata-medium-2026-05 | `2a65c4347964a0748877606d9c3a8b261b7fee6e069a814e64aaa024419f2f2c` | `candle realdata` |
| 7 | `top10-2023-fy-patchtst-overlay-realdata` | v5-realdata-medium-2026-05 | `5f303cc0812d421e6efdc40c0f412dd8cc0625891c677442bf2d7d2d5336ab4c` | `candle realdata` |
| 8 | `top10-2023-fy-vol-target-overlay-realdata` | v5-realdata-medium-2026-05 | `9fa64d467f35797939750fe70a492974a01aee0af197310bbfc0521ef57d2d5f` | `realdata` |

All 8 SHAs are byte-identical to the corresponding noop-baseline rows at `spec/anchors.toml` lines 121-155 + 242 + 272. This is the smoking gun: the v0.3.0 plumbing landed in code (`crates/backtest/src/scenarios/tcn_overlay.rs`, `tcn_overlay_weights.rs`, `patchtst_overlay_weights.rs`, `garch_vol_target_overlay.rs` — confirmed via ADR-0047 § D2 per-path audit), but the dispatch branches at `crates/backtest/src/main.rs:481-660` are `#[cfg(feature = "realdata")]` / `#[cfg(feature = "candle")]` and the CI binary doesn't carry those features.

**v0.4.0 = re-build on the canonical Apple Silicon box with `--features candle realdata`, re-emit, re-anchor.** No new plumbing, no design changes, no engine changes. This is the cheapest possible follow-on.

## Scope (v0.4.0)

### R1 — Feature-flagged canonical re-emission of the 8 deferred scenarios

Rebuild the `backtest` binary with `--features candle realdata` on the canonical Apple Silicon box (per v2.5 TCN training-determinism precedent at `spec/v25-tcn-overlay/feature.md:590` — Metal CPU drift was the original reason for hardware-pinning the candle path). Run each of the 8 scenarios under canonical `LatencySlippageSimConfig { latency_ms_min: 30, latency_ms_max: 80, slippage_bps: 8 }` (ADR-0045 D1, unchanged from v0.3.0). Emit reports to `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/backtest-<YYYYMMDD>-<HHMMSS>-<scenario>.md`.

**Determinism gate**: two independent runs per scenario MUST produce byte-identical body-SHAs (mirroring the v0.3.0 § 5b determinism spot-check on `pairs-2023-zscore-mr` and `top10-2023-1h-momentum`).

### R2 — In-place anchor SHA migration under `v5-realdata-medium-2026-05`

Overwrite the 8 SHAs at `spec/anchors.toml` lines 392-420 + 472-475 + 482-485 (canonical namespace section only) with the new R1 outputs. **Total anchor count stays at 70**; only 8 SHAs change. The 8 noop-baseline rows at lines 121-155 + 242 + 272 stay **byte-immutable** (ADR-0038 § D6 anchor-additive contract — the noop-baseline namespace remains the friction-free oracle).

Per ADR-0038 § D6.b wiring-bug-fix protocol precedent (v3-volatility-forecaster-noop-fix 2026-05-22) plus the v0.3.0 Q3=(a) extend-same-pin decision: the `v5-realdata-medium-2026-05` namespace is bumped in-place. No new namespace introduced.

### R3 — Sharpe-delta table extension (addendum to v0.3.0 series)

Author `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/sharpe-delta-table-<DATE>.md` extending the v0.3.0 series. Groups E-H (TCN-weights / TCN-realdata / PatchTST / VolTarget-GARCH) flip from "=noop (candle/realdata absent)" to live Δ Equity / Δ Sharpe rows. The fleet count of friction-real scenarios goes from **11 → 19** (4 newly-wired in v0.3.0 + 7 momentum-or-equivalent + 8 newly re-emitted here).

K1 surprise scan re-runs across the 8 newly re-emitted scenarios. H1 falsifier (≤ 3 flipped scenarios under canonical config) inherits from v0.3.0; with 0 K1 surprises observed in v0.3.0 across 11 paths, the prior is strong.

### R4 — Non-regression contract

- **R-NR.1** — `bash scripts/verify_anchors.sh` reports `ANCHORS PASS (70 / 70)` post-R2 (same row count, 8 SHAs updated in-place).
- **R-NR.2** — The 8 noop-baseline rows at `spec/anchors.toml:121-155, 242, 272` stay **byte-identical** (ADR-0038 § D6 byte-immutability for noop-baseline namespace).
- **R-NR.3** — The 11 v0.3.0 canonical SHAs (5 Group A + 2 Group B momentum + 2 Group C Pairs + 2 Group D TCN-synthetic) stay byte-identical (R1 only re-emits the 8 candle/realdata-gated scenarios, not the 11 already-friction-real ones).
- **R-NR.4** — `crates/backtest/`, `crates/exec/`, `crates/cost/`, `crates/audit/` library code is **NOT TOUCHED**. v0.4.0 is rebuild-only; the wiring landed at v0.3.0.
- **R-NR.5** — `cargo test --workspace --no-fail-fast` shows no new failures vs the v0.3.0-ship whitelist. `t1937_nine_strategy_anchors_unchanged` stays GREEN. `t1937b_canonical_strategy_anchors_unchanged` may need its `CANONICAL_STRATEGY_ANCHORS` table extended to cover the 8 newly-friction-real scenarios — developer Wave decides at M-DEV kickoff (analyst-recommended yes; mirrors v0.3.0 Wave E pattern).
- **R-NR.6** — `crates/strategy/tests/latency_slippage_sim_e2e.rs` + `vol_targeting_overlay_end_to_end.rs` + `vol_killswitch_overlay_end_to_end.rs` continue to PASS at ≥ 1 bp divergence (CLAUDE.md non-negotiable; mirrors v0.3.0 Wave F).

## K — Risk register / falsifiers

| K | Risk | Mitigation |
|---|---|---|
| **K1** | **Canonical Apple Silicon box not available to the developer** running the rebuild — Metal CPU drift would shift SHAs by host. | M-DEV blocked until rebuild is performed on the operator-locked Apple Silicon canonical box (same box used for v2.5 TCN, v2.6 realdata, v2.5a PatchTST, v3.0 vol-target anchor locks). If unavailable: **route back to analyst with operator-decide on dropping the 4 realdata scenarios from anchor set entirely** (since their SHAs cannot be deterministically reproduced off-canonical). |
| **K2** | **Realdata reference data missing / drifted.** Scenarios 3-8 require `data/binance/REVISION.toml` to match the v2.6.0-realdata lock SHA `3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7` (anchors.toml line ~131). PatchTST scenario 7 additionally requires the BS-1 checkpoint at `model_revision 62520db9...`. | Tester M-FINAL adds a precondition check: `data/binance/REVISION.toml` SHA + checkpoint SHA verified before run. If drifted, route to operator-decide on whether v0.4.0 also pins a fresh data revision (separate brief) or holds. |
| **K3** | **Alpha inversion under realistic friction on TCN-weights / PatchTST / VolTarget.** v0.3.0 found 0 K1 surprises across 11 paths, but these 8 paths use real candle weights + real-Binance data — friction may have larger effect than on synthetic GBM. | R3 delta table flags K1 surprises per scenario. Falsifier inherits H1 (≤ 3 flipped). If > 3 flip, R-O2 routing applies (per-scenario retirement briefs per v0.3.0 precedent). |
| **K4** | **Determinism failure across 2 runs.** If the candle/realdata path is non-deterministic (RNG seed drift, parallel-reduce nondeterminism), R-NR contract fails on its own gate. | Tester runs 2 independent passes per scenario; expects byte-identical SHA. v2.6.0-realdata Wave A confirmed 2-run byte-identity for the same 4 realdata scenarios at noop config — strong prior that friction-applied path is also deterministic. If K4 trips: route back to architect for ChaCha20Rng sub-stream audit per ADR-0043 D2. |

## H — Hypotheses

| H | Hypothesis | Confidence | Falsifier |
|---|---|---|---|
| **H1** | **TCN-overlay friction drag ≈ momentum's $3.5-5.4k.** TCN-weights and TCN-realdata trade at similar frequency to the synthetic TCN-overlay path (1,224-3,672 fills × 8 bps slippage). | Medium-high | R3 delta table per scenario. If TCN-weights Δ Equity differs by > 2× from synthetic TCN-overlay's $1.9k-$4.3k, H1 falsified — indicates the real-weights signal trades at materially different frequency. |
| **H2** | **PatchTST may show larger drag** than TCN due to higher trade frequency (PatchTST overlay's higher-dimensional patch-based forecast can dispatch finer-grained position deltas). | Low-medium | R3 delta table for `top10-2023-fy-patchtst-overlay-realdata` row. If Δ Equity < $5k, H2 falsified (PatchTST trade frequency is closer to TCN than expected). |
| **H3** | **0 K1 surprises across all 8 scenarios** (inherit v0.3.0 H1 generalization). | Medium-high | R3 K1 surprise scan. If > 0 K1 surprises in the 8 newly-friction-real scenarios, H3 falsified — operator decides per scenario whether to retire. |

## Operator-decide questions (Q1-Q2)

| Q | Topic | Options | Analyst-recommended default | Rationale |
|---|---|---|---|---|
| **Q1** | **Canonical box for the candle/realdata feature-flagged rebuild** | (a) Apple Silicon M-series (operator-locked since v2.5 TCN) / (b) other hardware (would require fresh box-anchor pin) | **(a) Apple Silicon M-series** | All prior candle/realdata anchors (v2.5 TCN, v2.6 realdata, v2.5a PatchTST, v3.0 vol-target) were locked on Apple Silicon Metal per `spec/v25-tcn-overlay/feature.md:590` ("metal_cpu_drift on Apple Silicon"). Switching boxes at v0.4.0 would invalidate the determinism prior. Standing-Autoapprove-eligible. |
| **Q2** | **Whether this brief is standing-Autoapprove-eligible** | (a) yes — no design changes vs v0.3.0; pure rebuild + re-emit / (b) no — operator wants explicit Q1-Q3 review per scenario | **(a) yes — standing-Autoapprove-eligible** | No new plumbing, no engine changes, no ADR amendments. The 8 SHAs that change are isomorphic to the v0.3.0 Q3=(a) extend-same-pin precedent. M-OD is structurally empty — operator may still skim before the developer rebuild, but no judgment is required. |

**Both Qs default-cleanly resolve.** M-OD likely empty per Q2=(a).

## Pre-drawn 2-cell verdict tree (presenter inherits)

| Cell | Condition | Route |
|---|---|---|
| **R-O1** | All 8 R1 re-emissions succeed + R2 anchor migration applies clean + R-NR.1-6 all green + 0 K1 surprises (H3 holds) | **SHIP** v0.4.0 + close the v5 anchor-migration arc end-to-end (v0.1.0 engine → v0.2.0 anchor migration → v0.3.0 full-path wiring → v0.4.0 candle/realdata re-emit). No further v5 follow-ons unless operator requests v0.4-square-root-market-impact or intrabar-fill-sampling (both pre-deferred from v0.1.0 D3). |
| **R-O2** | K1 / K3 / K4 trip OR > 0 K1 surprises (H3 falsified) | **REGRESSION** — route back to analyst for root-cause; possible outcomes: per-scenario retirement brief (v0.3.0 Q3=(b) precedent) or v0.4.1 follow-on (e.g. if the 4 realdata scenarios show ≥ 50% Δ Equity drag, operator may want to reconsider whether real-Binance data is the right oracle for the friction-sensitivity audit). |

## Cost framing

| Phase | Effort |
|---|---|
| Analyst (this brief) | ~0.5 day |
| Operator-decide (Q1-Q2 standing-Autoapprove) | ~0 (assuming Q2=(a)) |
| Architect M-T1 — fast-skip (no design changes vs v0.3.0; ADR-0047 unchanged) | ~0 day |
| Developer Wave A — feature-flagged rebuild + 8-scenario re-emission on canonical box | ~0.5-1 day |
| Developer Wave B — R2 anchor SHA migration in `spec/anchors.toml` (8 in-place updates) | ~0.1 day |
| Developer Wave C — R3 Sharpe-delta table addendum + K1 surprise scan | ~0.25 day |
| Developer Wave D — `t1937b_canonical_strategy_anchors_unchanged` table extension (8 entries) | ~0.1 day |
| Tester M-FINAL (verify 70/70 anchors + 2-run determinism + K1 surprise scan + workspace test gate) | ~0.5 day |
| Presenter | ~0.5 day |
| **Total** | **~1-2 days wall-clock** |

## Predecessor / parent chain

- **Parent**: backtest-vs-live execution gap (long-running theme; cited in `spec/product.md § Strategy lifecycle`)
- **Predecessor**: `v5-latency-slippage-sim-v0.3.0-full-path-wiring v0.1.0` (shipped 2026-05-27, commit `21bda41` per v0.3.0 M-FINAL report). v0.3.0 R-O1 SHIP path explicitly carves out the 8 candle/realdata-feature-gated scenarios for v0.4.0; this brief closes that carve-out.
- **Grandparents**: `v5-latency-slippage-sim-v0.2.0-anchor-migration v0.1.0` (2026-05-27) + `v5-latency-slippage-sim v0.1.0` (2026-05-26)
- **Sibling**: v2.5 TCN investigation (analyst pass running in parallel 2026-05-28 — independent scope; will append a separate REQ row to trace.toml).
- **Successor (probable)**: none auto-spawned. Operator may request `v0.4-square-root-market-impact` and/or `intrabar-fill-sampling` (both pre-deferred from v0.1.0 D3 / ADR-0043 Alternatives Rejected) post-v0.4.0 ship.

## Cross-references

- v0.3.0 brief — [`spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/feature.md`](../v5-latency-slippage-sim-v0.3.0-full-path-wiring/feature.md)
- v0.3.0 M-FINAL test report (§ 8 Open Items — the carve-out being closed) — [`spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/test-final-2026-05-27-v5-latency-slippage-sim-v0.3.0-full-path-wiring.md`](../v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/test-final-2026-05-27-v5-latency-slippage-sim-v0.3.0-full-path-wiring.md)
- v0.3.0 Sharpe-delta table (the template R3 extends) — [`spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/sharpe-delta-table-2026-05-27.md`](../v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/sharpe-delta-table-2026-05-27.md)
- ADR-0043 (engine D1-D5) — [`spec/architecture/adr/0043-simulated-latency-and-slippage.md`](../architecture/adr/0043-simulated-latency-and-slippage.md)
- ADR-0045 (canonical config + namespace strategy) — [`spec/architecture/adr/0045-v5-canonical-config-and-noop-baseline-namespace.md`](../architecture/adr/0045-v5-canonical-config-and-noop-baseline-namespace.md)
- ADR-0047 (v0.3.0 per-path plumbing + namespace-aware resolver; v0.4.0 inherits unchanged) — [`spec/architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md`](../architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md)
- Anchors file (target of migration) — [`spec/anchors.toml`](../anchors.toml) lines 392-420, 472-475, 482-485 (canonical SHAs being updated)
- v0.3.0 t1937b table (developer Wave D extends this) — `crates/reports/tests/strategy_anchors_unchanged.rs` `CANONICAL_STRATEGY_ANCHORS`
- Verify script — [`scripts/verify_anchors.sh`](../../scripts/verify_anchors.sh)
- Tasks — [`tasks.md`](tasks.md)
- Trace row — `REQ-V5-LATENCY-SLIPPAGE-V0-4-0-001` in [`spec/trace.toml`](../trace.toml)

## Design

> Architect M-T1 is a **fast-skip** — no design changes vs v0.3.0. ADR-0047 carries forward unchanged. v0.4.0 is a pure rebuild + re-emit pass; the per-path plumbing contract (ADR-0047 D2) was discharged at v0.3.0 Wave A and the 8 scenarios verified at v0.3.0 M-FINAL § 3a grep gate (`sim_slippage_cost` defined exactly once at `crates/backtest/src/scenarios/sim.rs`).

> If the architect at M-T1 discovers any pre-condition drift (data revision SHA, checkpoint SHA, candle/Metal version bump that would force a re-anchor), this section gets a T-AR-1 note and the brief routes back to analyst for K2 expansion.

## Implementation

_Developer M-DEV populates after rebuild + re-emission._

## Verification

_Tester M-FINAL links to reports here after developer M-DEV close._

## Changelog

- 2026-05-28 (analyst): feature.md v0.1.0 authored. **4 R / 4 K / 3 H / 2 Q** + non-regression contract + pre-drawn 2-cell verdict tree + cost framing. Closes the v0.3.0 SOFT-PASS carve-out (8 candle/realdata-feature-gated scenarios). Q1-Q2 standing-Autoapprove-eligible. M-T1 likely fast-skips; M-OD likely empty. Anchor risk: 0 added rows, 8 SHAs updated in-place under existing `v5-realdata-medium-2026-05` pin. ANCHORS PASS (70/70) pre-spec confirmed.
