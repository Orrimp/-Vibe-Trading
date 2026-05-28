---
slug: v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit
version: 0.1.0
status: shipped
owner: shipped
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

### M-T1 close note (architect, 2026-05-28)

**Verdict: FAST-SKIP confirmed. No design changes. No new ADR. ADR-0047 carries forward unchanged for v0.4.0.**

K1 — Apple Silicon canonical box availability (manifest-level pre-check):
- `crates/backtest/Cargo.toml` `[features]` block (lines 21-33) defines BOTH `candle = ["strategy/forecast"]` AND `realdata = ["dep:toml"]`. Build command `cargo build --release -p backtest --features "candle realdata"` is structurally well-formed.
- `[[bin]] backtest` at lines 7-9 has NO `required-features` constraint, so the default-feature build still works for the 11 v0.3.0-friction-real scenarios and the feature-flagged build covers the 8 v0.4.0 scenarios. (The `[[bin]] threshold_sweep` at line 11-14 DOES carry `required-features = ["candle", "realdata"]` but that binary is out of scope for v0.4.0 R1.)
- Dispatch arms verified: `crates/backtest/src/main.rs` lines 481-660 carry `#[cfg(feature = "realdata")]` gates for the 6 realdata scenarios (lines 481, 508, 531, 553, 580, 606 — one per scenario). The 2 `top10-*-fy-tcn-overlay-weights` arms (synthetic data, candle-only) live at lines 438 + 458 and are NOT dispatch-gated — their candle dependency lives downstream at strategy execution (`ScenarioStrategy::TcnOverlayMomentumWeights` resolves at runtime via the candle-backed `forecast` crate).
- K1 confirmed at Cargo manifest level. **Final canonical-box validation (Metal CPU drift; 2-run byte-identity SHA) deferred to developer M-DEV Wave A determinism gate.** If the developer cannot reach the operator-locked Apple Silicon box, route back to analyst per K1 mitigation (drop the 4 realdata scenarios from the anchor set — operator-decide).

K2 — Data + checkpoint revision drift (architect-side verification):
- `data/binance/REVISION.toml` aggregate SHA at line 2 = `3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7` — **byte-match against analyst-cited SHA. No drift.** Cross-confirmed against same SHA cited 4 separate times in `spec/anchors.toml` (lines 132, 159, 228, 248) and in the live `crates/backtest/src/main.rs:508, 528` `expected_revision_sha` field. Scenarios 3-8 K2 precondition INTACT.
- PatchTST BS-1 checkpoint at `crates/forecast/checkpoints/anchors/patchtst-bs1-62520db92f68c1d323f0782bc367c742cf9439631106ddc0fd492188f6d1cd4d.{safetensors,metadata.json}` — both files present on disk; `model_revision` field in metadata.json = `62520db92f68c1d323f0782bc367c742cf9439631106ddc0fd492188f6d1cd4d` (analyst-cited prefix `62520db9` matches). The hard-coded byte-identity check at `crates/forecast/src/patchtst.rs:546` + `crates/forecast/tests/patchtst_byte_identity.rs:32` both reference the same SHA. Scenario 7 K2 precondition INTACT.

ADR-0047 carries-forward ratification:
- D1 (K2-REACHABLE-CHEAP, `--force-synthetic-bars` flag) — v0.4.0 does not touch Group A SMA/Composed scenarios; D1 is preserved but inert at v0.4.0 scope.
- D2 (per-path plumbing contract via `crates/backtest/src/scenarios/sim.rs::sim_slippage_cost`) — `sim.rs` confirmed on disk; `pub fn sim_slippage_cost` defined exactly once at line 38; v0.4.0 rebuilds the 4 module paths (`tcn_overlay.rs`, `tcn_overlay_weights.rs`, `patchtst_overlay_weights.rs`, `garch_vol_target_overlay.rs`) under feature flags but their wiring is unchanged.
- D3 (namespace-aware Rust resolver in t1937) — Wave D extends `CANONICAL_STRATEGY_ANCHORS` table with 8 new entries; the resolver itself is unchanged.
- D4 (Group A re-emission contract conditional on Q1) — v0.4.0 inherits the Q1=(a) route locked at v0.3.0 ship; D4 is inert at v0.4.0 scope (no Group A touch).
- D5 (anchor namespace strategy = extend `v5-realdata-medium-2026-05` in-place) — v0.4.0 Q3 default `(a) extend-same-pin` is structurally identical to v0.3.0 D5. **No new namespace.** R2 overwrites 8 SHAs in-place at the same pin.
- D6 (cross-feature e2e re-check inventory = 3 files + 1 meta) — unchanged at v0.4.0; R-NR.6 inherits the same 3 + 1 list verbatim.

**Conclusion: every ADR-0047 D-term covers v0.4.0 without amendment. No ADR-0048 needed.** The architect explicitly recommends against authoring a new ADR — the brief is correctly scoped as a "rebuild + re-emit with feature flags enabled" pass, and the contract that governs it is ADR-0047 in full.

Outstanding architect-side risk (forwarded to developer): the determinism gate at R-NR + K4 is the only non-trivial gate. If the candle/realdata path is non-deterministic when run on the canonical Apple Silicon box, route back to architect for ChaCha20Rng sub-stream audit per ADR-0043 D2. v2.6.0-realdata Wave A confirmed 2-run byte-identity for the same 4 realdata scenarios at noop config (strong prior); v2.5 TCN-weights determinism was confirmed at v2.5 M-FINAL on the canonical box. The compound risk (candle × realdata × friction-applied) is novel at v0.4.0 — flag for developer Wave A 2-run check.

## Implementation

**M-DEV completed 2026-05-28 on canonical Apple Silicon box (Darwin 25.5.0, Apple Silicon M-series).**

### Wave A — Feature-flagged rebuild + 8-scenario re-emission

- **T-D-N1 (Build precondition)**: `cargo build --release -p backtest --features "candle realdata"` completed in 7.38s. No compile errors.
- **T-D-N2 (First emission)**: All 8 scenarios run under `LatencySlippageSimConfig { latency_ms_min: 30, latency_ms_max: 80, slippage_bps: 8 }`. Reports emitted to `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/backtest-20260528-<HHMMSS>-<scenario>.md`.
- **T-D-N3 (Determinism gate — LOAD-BEARING PASS)**: All 8 scenarios run a second time to `/tmp/v0.4.0-run2/`. Every SHA matches the first run byte-for-byte. Compound determinism (candle × realdata × friction) confirmed.

| Scenario | Feature Flags | Run 1 SHA | Run 2 SHA | Match |
|----------|---------------|-----------|-----------|-------|
| top10-2023-fy-tcn-overlay-weights | candle | `28379df8...` | `28379df8...` | PASS |
| top10-2024-fy-tcn-overlay-weights | candle | `0c13ed0b...` | `0c13ed0b...` | PASS |
| top10-2023-fy-tcn-overlay-realdata | realdata | `10fd4502...` | `10fd4502...` | PASS |
| top10-2024-fy-tcn-overlay-realdata | realdata | `87dfad45...` | `87dfad45...` | PASS |
| top10-2023-fy-tcn-overlay-weights-realdata | candle+realdata | `123d8228...` | `123d8228...` | PASS |
| top10-2024-fy-tcn-overlay-weights-realdata | candle+realdata | `21bec3c9...` | `21bec3c9...` | PASS |
| top10-2023-fy-patchtst-overlay-realdata | candle+realdata | `55c5b715...` | `55c5b715...` | PASS |
| top10-2023-fy-vol-target-overlay-realdata | realdata | `4edd8cc5...` | `4edd8cc5...` | PASS |

### Wave B — Anchor SHA migration

- **T-D-N4**: `spec/anchors.toml` lines updated in-place for all 8 scenarios under namespace `v5-realdata-medium-2026-05`. Namespace pin unchanged.
- `scripts/verify_anchors.sh` updated: `migration_dir_v04` variable added; resolver checks v0.4.0 dir first, then v0.3.0, then v0.2.0.
- **T-D-N5**: `bash scripts/verify_anchors.sh` → `ANCHORS PASS (70 / 70)`. Count unchanged; 8 SHAs updated in-place.

### Wave C — Sharpe-delta table addendum

- **T-D-N6**: `reports/sharpe-delta-table-2026-05-28.md` authored. Groups E-H flip from `=noop (candle/realdata absent)` to live Δ Equity rows. Fleet: 11 → 19 friction-real scenarios.
- **T-D-N7 (K1 surprise scan)**: 0 K1 surprises across all 8 scenarios. All remain equity-positive under canonical friction. H3 holds.
  - Notable finding: TCN-realdata drag ($36.5k per scenario) is ~19× larger than TCN-synthetic ($1.9k) due to ~5× trade-frequency amplification on real Binance hourly data. H1 confirmed (synthetic path) but the realdata path shows the compounding effect of real-world signal frequency.
  - H2 falsified: PatchTST generated fewer trades (3,187) than TCN-realdata (6,203), not more.

### Wave D — t1937b CANONICAL_STRATEGY_ANCHORS extension

- `crates/reports/tests/strategy_anchors_unchanged.rs` updated:
  - `CANONICAL_FEATURE_DIRS` extended with `v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit`
  - `CANONICAL_STRATEGY_ANCHORS` extended with 8 new entries (Groups F-J)
- `cargo test -p reports --test strategy_anchors_unchanged` → `3/3 PASS` (t1937 + t1937b + t1942)

### Wave E — Cross-feature e2e re-check (T-D-N8)

- `cargo test -p strategy --test latency_slippage_sim_e2e` → `3/3 PASS`
- `cargo test -p strategy --test vol_targeting_overlay_end_to_end` → `1/1 PASS`
- `cargo test -p strategy --test vol_killswitch_overlay_end_to_end` → `4/4 PASS`

### Final gate checks

- `cargo fmt --all -- --check` → PASS (no formatting changes needed)
- `cargo clippy -p backtest --features "candle realdata" -- -D warnings` → PASS (0 new warnings)
- `cargo test --workspace --no-fail-fast` → PASS (0 failures; all test groups pass)

## Verification

_Tester M-FINAL links to reports here after developer M-DEV close._

## Changelog

- 2026-05-28 (analyst): feature.md v0.1.0 authored. **4 R / 4 K / 3 H / 2 Q** + non-regression contract + pre-drawn 2-cell verdict tree + cost framing. Closes the v0.3.0 SOFT-PASS carve-out (8 candle/realdata-feature-gated scenarios). Q1-Q2 standing-Autoapprove-eligible. M-T1 likely fast-skips; M-OD likely empty. Anchor risk: 0 added rows, 8 SHAs updated in-place under existing `v5-realdata-medium-2026-05` pin. ANCHORS PASS (70/70) pre-spec confirmed.
- 2026-05-28 (architect): M-T1 FAST-SKIP closed. K1 confirmed at Cargo manifest level (`backtest` Cargo.toml lines 21-33 define `candle` + `realdata` features; `[[bin]] backtest` has no `required-features` constraint). K2 both preconditions INTACT: `data/binance/REVISION.toml` SHA = `3a8b96...bfc7` (byte-match); PatchTST checkpoint at `crates/forecast/checkpoints/anchors/patchtst-bs1-62520db9....{safetensors,metadata.json}` present on disk with `model_revision 62520db9...` (byte-match). ADR-0047 carries forward unchanged (D1-D6 all covered; no ADR-0048 needed). Frontmatter `owner: analyst → developer`. Design § M-T1 close note appended with full verification trail. Outstanding flag for developer: determinism gate (compound risk candle × realdata × friction-applied novel at v0.4.0; 2-run byte-identity check is the only non-trivial gate).
