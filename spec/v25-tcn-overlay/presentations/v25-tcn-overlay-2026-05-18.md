---
slug: v25-tcn-overlay
mode: release
status: draft
audience: human-operator
updated: 2026-05-18
generated: 2026-05-18T07:00:00Z
gate: ci-baseline (phase 1 of 4)
tester_report: spec/v25-tcn-overlay/reports/test-2026-05-18-0616-v25-tcn-overlay.md
commit: 3fbae7538caedb9495bc726649deebb9d26fc127
---

# v2.5 TCN forecast overlay — phase 1 of 4 — release (CI-baseline gate only)

## TL;DR

Phase 1 of the 4-phase DL roadmap ships **CI-baseline infrastructure**: a
TCN training-loop binary, deterministic feature pipeline,
LFS-anchored checkpoint provenance, and two canonical body-SHA-256
anchors (`top10-2023-fy-tcn-overlay`, `top10-2024-fy-tcn-overlay`) —
all green. **This is NOT a strategy ship.** Real TCN-weights training
(M3, T-D-11/T-D-12) remains the open deliverable; feature stays
`in-progress` until that lands under a separate `v2.5.0-tcn-weights`
anchor pair.

## What changed — CI-baseline scope

- **Feature pipeline (R1-R3, T-D-1..3)** — `crates/forecast/src/features.rs`
  (~980 LOC): pure-function parquet → 256-bar × 5-feature window iterator,
  `aligned_batches()` round-robin-by-timestamp multi-symbol batching.
  3-symbol property test confirms same parquet → same window order.
- **TcnForecaster forward pass (R4, T-D-4..7)** — `crates/forecast/src/tcn.rs`
  (~680 LOC): 8 stacked dilated causal residual blocks
  `[1,2,4,8,16,32,64,128]`, k=3, H=96 channels → ~4.4M params. Metal
  feature-gated; CPU is the determinism oracle per ADR-0029 § D2.
- **Training-loop binary (R5-R7, T-D-8..10)** —
  `crates/forecast/src/bin/train_tcn.rs` (~670 LOC): AdamW + OneCycle +
  Huber(δ=0.001), seed `0x00C0FFEE`. 1-epoch BTCUSDT smoke completes
  without panic; two-run metadata-JSON SHA byte-identical
  (`7e341a3b…be72`); `sigma_train` finite.
- **Checkpoint provenance (R8, ADR-0029)** —
  `crates/forecast/src/provenance.rs` (~340 LOC): canonical-JSON
  serialiser with lex-sorted keys, no whitespace, string-encoded
  Decimal floats. 13 provenance tests including key-shuffle and
  golden-SHA pass. Cross-phase contract for v2.5a / v2.5b.
- **Passthrough-forecaster backtest path (R10-R12, T-D-13..16)** —
  `crates/strategy/src/tcn_overlay_momentum.rs` (~690 LOC) +
  `crates/backtest/src/main.rs` integration. Strategy registered as
  `tcn_overlay_momentum`; `PassthroughForecaster` keeps the CI path
  candle-free; two canonical scenarios wired:
  `top10-2023-fy-tcn-overlay` (2208 bars) and
  `top10-2024-fy-tcn-overlay` (6600 bars).
- **Two anchors locked** in `spec/anchors.toml` under version `v2.5.0`
  (lines 95-103). The anchors.toml comment block (lines 85-93)
  explicitly notes these reflect the PassthroughForecaster path and a
  second lock under `v2.5.0-tcn-weights` is required after M3
  completes.

## Why

Per [ADR-0028](../../architecture/adr/0028-v25-dl-forecast-overlay-candle.md)
and the [4-phase DL roadmap](../../v25-dl-forecast-overlay/feature.md):
TCN is built first because (a) it is the lowest-complexity DL family
and gets us to a working baseline fastest, (b) it establishes the
reusable training-loop + provenance + replay-cache + audit + cost
infrastructure that phases v2.5a (PatchTST) and v2.5b (vanilla
Transformer) inherit, and (c) deterministic inference (no
autoregressive sampling) is easier to anchor and audit. This release
closes the CI-portable side of that work: the model wiring exists,
the anchor regression gate exists, the provenance contract is signed
in ADR-0029, and the bake-off in v2.6 has its first competitor's
scaffolding in place.

## What's still open — M3 (real-TCN-weights gate)

| Task | Status | What it produces |
|------|--------|------------------|
| **T-D-11 (M3)** — full BS-1 training run (10 symbols, Jan–Sep 2023 train / Oct–Dec 2023 val, 30 epochs / patience 5, ≤2 h on M-series) | OPEN | `tcn-bs1-<sha>.safetensors` + `.metadata.json` LFS-tracked at `crates/forecast/checkpoints/anchors/`; M3 training report under `spec/v25-tcn-overlay/reports/m3-bs1-training-<date>.md` |
| **T-D-12 (M3)** — same for BS-2 (train 2023 full year / val Q1 2024) | OPEN | `tcn-bs2-<sha>.safetensors` + `.metadata.json`; M3 report extended with BS-2 curves |
| **Second anchor lock** under `v2.5.0-tcn-weights` | OPEN | Replaces the PassthroughForecaster anchors once `--features candle` + real checkpoints produce non-zero `Dampened to Hold` counts |

The tester's PASS changelog entry calls this out explicitly
(feature.md line 721): *"Status remains `in-progress` —
real-TCN-weights anchor lock deferred to M3 (T-D-11/T-D-12, separate
`v2.5.0-tcn-weights` gate)."* The anchors.toml comment block (lines
85-93) is the second canonical source.

## What you can do now (CI-baseline)

| Action | Command |
|--------|---------|
| Run BS-1 backtest (passthrough path, 2023 full year) | `cargo run -p backtest --release -- --scenario top10-2023-fy-tcn-overlay --seed 0xC0FFEE` |
| Run BS-2 backtest (passthrough path, 2024 full year) | `cargo run -p backtest --release -- --scenario top10-2024-fy-tcn-overlay --seed 0xC0FFEE` |
| Verify both new anchors byte-stable | `bash scripts/verify_anchors.sh` |
| Run the 20/20 determinism suite (includes `tt1_top10_{2023,2024}_fy_tcn_overlay_*`) | `cargo test -p backtest --test determinism` |
| Dry-run the training-loop binary (no real training) | `cargo run -p forecast --features candle --bin train_tcn -- --config crates/forecast/train_tcn.toml --dry-run` |
| Smoke-test the 1-epoch training path (provenance SHA determinism) | `cargo test -p forecast --features candle --test smoke_train` |
| Inspect the provenance canonicaliser | `cargo test -p forecast --features candle provenance::tests` |

## Live demo — BS-1 + BS-2 backtest report headers

**BS-1 — `top10-2023-fy-tcn-overlay`** (verbatim from
`spec/v25-tcn-overlay/reports/backtest-20260518-061302-top10-2023-fy-tcn-overlay.md`):

```
# Backtest Report — top10-2023-fy-tcn-overlay

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | top10-2023-fy-tcn-overlay     |
| Universe             | 10 symbols                    |
| Start year           | 2023                          |
| Bars (total)         | 22080                         |
| Initial capital      | $100000.00 USDT               |
| Final equity         | $30235.58 USDT                |
| Total return         | -69.76%                       |
| Max drawdown         | 87.48%                        |
| Trades               | 1224 (614 buys / 610 sells)   |
| Total fees           | $2681.67 USDT                 |
| Seed                 | 0xC0FFEE                      |

## TCN Overlay Modulation
| Passed through       | 1142                          |
| Dampened to Hold     | 0                             |
| Warming-up           | 105                           |
| Dampen rate          | 0.00%                         |
```

**BS-2 — `top10-2024-fy-tcn-overlay`** (verbatim from
`spec/v25-tcn-overlay/reports/backtest-20260518-061309-top10-2024-fy-tcn-overlay.md`):

```
# Backtest Report — top10-2024-fy-tcn-overlay

| Scenario             | top10-2024-fy-tcn-overlay     |
| Bars (total)         | 66000                         |
| Final equity         | $44300.24 USDT                |
| Total return         | -55.70%                       |
| Max drawdown         | 87.48%                        |
| Trades               | 3672 (1838 buys / 1834 sells) |
| Total fees           | $3400.56 USDT                 |
| Passed through       | 3882                          |
| Dampened to Hold     | 0                             |
| Dampen rate          | 0.00%                         |
```

### How to read these numbers

> The `Dampened to Hold = 0` / `Dampen rate = 0.00%` rows are the
> **load-bearing signal** that the CI path is on the PassthroughForecaster.
> The forecaster always returns `(Flat, confidence=0)`, so the overlay
> never disagrees with v1 momentum and the strategy degrades to plain
> v1 cross-sectional momentum on **synthetic ChaCha20 random-walk
> data** (one independent RNG stream per symbol).
>
> **The –69.76% / –55.70% returns and 87.48% max drawdown are NOT a
> strategy regression.** They are the baseline measurement of v1
> momentum applied to seeded random walks; momentum has no edge on
> pure random walks by construction. The success criteria in
> feature.md § Backtest Scenarios (Sharpe ≥ v1 + 0.10; max drawdown ≤
> v1 + 2pp; trades ≤ 1.5 × v1) are not evaluable here and will be
> evaluated in the M3 re-gate when real TCN checkpoints land.
>
> Treat these numbers as a **byte-stable regression fingerprint** for
> the wiring, not as a strategy result.

## Screenshots

_n/a — backend-only ship. No UI surface in this gate (the TCN overlay
is a strategy-layer module; cockpit consumes it transparently once
M3 lands)._

## Verification matrix

| Gate | Status | Evidence |
|------|--------|----------|
| V1 — Determinism: 100 inference calls → byte-identical `ForecastOverlay` JSON (feature.md § Verification) | VERIFIED | T-D-13 inference unit suite `tcn::tests::td13_*` 5/5 pass (tasks.md line 240-243); strict-replay miss + cache-hit-on-second-call both exercised. |
| V2 — Replay: BS-1 + BS-2 reproduce to byte-identical PnL on second run | VERIFIED | Tester re-run SHA matches anchored SHA exactly for both scenarios. BS-1: `01d02584…8ef5`; BS-2: `e24c85ac…6163`. Test report §5.2 / §5.3. |
| V3 — Anchor lock: both new anchors land in `spec/anchors.toml` at ship | VERIFIED | `spec/anchors.toml:95-103` carries both rows under version `v2.5.0`; comment block at lines 85-93 documents the M3 follow-up. |
| V4 — Existing 11 anchors stay byte-identical | VERIFIED | `verify_anchors.sh` 13/13 PASS — all 11 prior scenarios (v0..v2.0.0) unchanged; both new TCN scenarios match. Test report §7.1 verbatim block. |
| V5 — Determinism integration suite (20/20 incl. 2 renamed `tt1_top10_{2023,2024}_fy_tcn_overlay_*`) | VERIFIED | `cargo test -p backtest --test determinism` 20/20 PASS, 62.36 s. Test report §3 + §7.2. |
| V6 — `cargo fmt --check` clean | VERIFIED | Test report §2 row 1. Prior FAIL (2 files in `crates/agent/`) resolved by developer in fix-pass. |
| V7 — `cargo clippy --workspace -- -D warnings` clean | VERIFIED | Test report §2 row 2. Prior FAIL (4 errors in `crates/forecast/src/tcn.rs:684-685,912-913`) resolved. |
| V8 — `spec-lint`: 0 new regressions vs prior baseline | VERIFIED (with caveat — see Risks) | Test report §2.1: 733 violations in 2 categories at tester time. Pre-existing dead-link debt unchanged. Presenter re-check at write time: 734 in 3 categories — the +1 is `missing-frontmatter` on `spec/ui-rethink-phase-a-lab/tasks.md` (unrelated feature, matches the audit-2026-05-18 baseline of 734/3). Not a v25-tcn-overlay regression. |
| V9 — Metadata-JSON canonicaliser byte-stable | VERIFIED | `provenance::tests` 13/13 PASS incl. key-shuffle, golden-SHA, deterministic. Tasks.md T-D-9. Two-run training smoke SHA byte-identical (`7e341a3b…be72`). |
| V10 — Audit + cost emission per inference call | VERIFIED | T-D-13 wires `tracing::info!` on targets `forecast.audit` and `forecast.cost`; payload carries `model_revision`. Cache key includes `model_revision + close_prices + timestamps + sampling_seed`. |

## Numbers that matter

- **Tests**: 20/20 determinism (incl. 2 new canonical `tt1_*`) + 47 forecast lib tests + 7 strategy tests + ~1300 workspace tests = **0 failed**, 4 pre-existing `#[ignore]`.
- **Anchors**: **13/13** PASS (11 pre-existing byte-identical + 2 new TCN).
- **New anchor SHAs** (first 8 chars):
  - `top10-2023-fy-tcn-overlay` → `01d02584…`
  - `top10-2024-fy-tcn-overlay` → `e24c85ac…`
- **Code added (Wave A+B+D)**: ~3 980 LOC across 11 files (forecast pipeline + tcn + provenance + train_tcn + strategy + backtest wiring).
- **Backtest wall-clock**: BS-1 ~0.9 s, BS-2 ~2.8 s on CPU (synthetic, passthrough path).
- **Provenance double-run SHA**: `7e341a3b29f36e362cbf3d4209ad62065e814f0c94a12e3c7e1a7d043821be72` (byte-identical across two 1-epoch smoke runs).
- **TCN topology fingerprint**: 8 blocks, dilations `[1,2,4,8,16,32,64,128]`, k=3, H=96, ~4.4 M params, receptive field 1 021 bars (~42 days at 1h).
- **Spec-lint at write time**: 734 violations in 3 categories (727 dead-link + 6 trace-broken-path + 1 missing-frontmatter in `ui-rethink-phase-a-lab/tasks.md`). Matches the audit-2026-05-18 baseline. Tester saw 733/2 at PASS time; the +1 is unrelated to this feature.

## Risks / known gaps

1. **PassthroughForecaster baseline is intentional CI scope, NOT strategy-shippable as-is.**
   The CI path runs without `--features candle` and the
   `PassthroughForecaster` always emits `(Flat, 0)`. The –69.76% / –55.70%
   returns are baseline measurements of v1 momentum on seeded random
   walks; treating them as a regression would be a category error.
   Read them as a byte-stability fingerprint of the wiring.

2. **Real-TCN anchor lock awaits M3.** T-D-11 + T-D-12 (full
   training runs producing `tcn-bs1` / `tcn-bs2` checkpoints) are open.
   The second anchor lock under version `v2.5.0-tcn-weights` cannot
   land until those checkpoints exist and the `--features candle`
   backtest path runs with a non-zero `Dampened to Hold` count. The
   anchors.toml comment block (lines 85-93) and the tester PASS
   changelog (feature.md line 721) both pin this commitment.

3. **Metal-vs-CPU bit-identity is not proven.** Per ADR-0029 § D2 and
   `feature.md § D2`, candle's Metal backend uses non-deterministic
   reduction order on some ops. Strategy: CPU is the determinism
   oracle for anchor verification; Metal is allowed for training only.
   The LFS-tracked anchor checkpoint mitigates this — we ship weights
   rather than a re-train recipe. The Metal-vs-CPU drift exit gate
   (`max_abs < 1e-4`) is gated behind `--features metal` and runs only
   on Apple Silicon — operator-deferred per tasks.md T-D-7.

4. **Pre-existing spec-lint dead-link debt unchanged.** 727 dead-links
   and 6 future-phase trace-broken-path rows persist. None are caused
   by this feature; both buckets are tracked in
   `spec/dev-notes/audit-2026-05-18.md` as P1 / P2 cleanup tasks. The
   net delta this gate introduces is **negative one category** (the
   prior 2 `unreferenced-anchor` rows from the scenario-naming
   mismatch were fixed in the developer's fix-pass).

5. **`cargo audit` not installed; `cargo deny` pre-existing FAIL.** Both
   pre-date v2.5 and are tracked separately. Test report §2 carries
   these forward unchanged.

## Open decisions

> One decision, load-bearing: close the CI-baseline gate and let M3
> proceed as a separate deliverable.

1. **Approve closing the CI-baseline gate.** Effects of approval:
   - T-D-15, T-D-16, T-T-1 stay ticked.
   - The two `v2.5.0` anchors (`top10-2023-fy-tcn-overlay`,
     `top10-2024-fy-tcn-overlay`) stay locked. They become part of
     the byte-stability contract and will fail
     `verify_anchors.sh` if any future change reaches the
     PassthroughForecaster code path.
   - Feature status correctly stays `in-progress` (NOT `shipped`)
     pending M3.
   - Orchestrator schedules T-D-11 / T-D-12 as the next work item
     (full BS-1 + BS-2 training on M-series, ≤2 h each per R2).
   - Cost to revert if needed: low — both new anchor rows can be
     deleted from `anchors.toml`; the determinism tests would then
     need to be unrenamed or deleted.

   This is NOT a full strategy-ship approval. v2.5.0 ships when the
   `v2.5.0-tcn-weights` gate also passes (M3 + real-TCN re-gate
   producing a separate tester report).

## Approval

- [ ] Approved — close CI-baseline gate, schedule M3
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

### Notes / feedback

<empty until operator fills>

## Changelog

- 2026-05-18 (presenter): initial draft. CI-baseline gate (passthrough
  forecaster) ready for operator approval. Real-TCN-weights gate
  remains open as M3 deliverable. Tester `VERDICT → PASS` at commit
  `3fbae75`. HANDOFF → operator.
