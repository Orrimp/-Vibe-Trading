---
slug: v25-tcn-overlay
mode: release
status: draft
audience: human-operator
updated: 2026-05-18
generated: 2026-05-18T13:00:00Z
gate: m3-real-weights-anchor (phase 1 of 4)
tester_report: spec/v25-tcn-overlay/reports/test-2026-05-18-1230-m3-v25-tcn-overlay.md
commit: a12fc6e
predecessor_presentation: spec/v25-tcn-overlay/presentations/v25-tcn-overlay-2026-05-18.md
predecessor_gate: ci-baseline (approved 2026-05-18 morning)
---

# v2.5 TCN forecast overlay — phase 1 of 4 — M3 real-weights anchor gate

## TL;DR

M3 ships **trained TCN weights** anchored under a new version `v2.5.0-tcn-weights`:
two LFS-tracked 30-epoch checkpoints (BS-1, BS-2) on real Binance hourly OHLCV
across all 10 top-USDT symbols, wired into two new backtest scenarios
(`top10-2023-fy-tcn-overlay-weights`, `top10-2024-fy-tcn-overlay-weights`) under
`--features candle`. **Phase 1 of the 4-phase DL roadmap is now
infrastructure-complete** — the entire training + LFS-anchor + candle-gated
backtest pipeline is reusable by v2.5a (PatchTST) and v2.5b (Transformer).
**Honest finding surfaced**: on the current synthetic ChaCha20 GBM backtest
data the real-weights runs are byte-identical to the passthrough runs
(`dampened=0`) because the TCN's `r_hat` falls inside the ε=0.0005 deadband on
i.i.d. Gaussian returns — the anchors lock determinism, not alpha. Real-data
alpha evaluation is queued as new backlog `backtest-real-binance-data`.

## What changed since the CI-baseline approval (2026-05-18 morning)

The predecessor deck
[`v25-tcn-overlay-2026-05-18.md`](v25-tcn-overlay-2026-05-18.md)
closed the CI-baseline gate (passthrough forecaster, 13 anchors). This M3
follow-on adds — and only adds — the real-TCN-weights path. All 13 prior
anchors stay byte-identical.

- **Two LFS-tracked anchor checkpoints** under
  `crates/forecast/checkpoints/anchors/`:
  - `tcn-bs1-d1c3696d….safetensors` + `.metadata.json` (BS-1, trained on 2023
    full year, sigma_train 10.954, weights_sha256 `4ed9064a…`).
  - `tcn-bs2-3fabcabe….safetensors` + `.metadata.json` (BS-2, trained on 2023
    + Q1 2024 validation, sigma_train 6.916, weights_sha256 `5f22b5bc…`).
  - Both use the identical recipe pinned by ADR-0029: 8 blocks, dilations
    `[1,2,4,8,16,32,64,128]`, k=3, H=96, AdamW + OneCycle + Huber(δ=0.001),
    seed `0x00C0FFEE`, 30 epochs.
- **Two new candle-gated backtest scenarios**:
  - `top10-2023-fy-tcn-overlay-weights` → body SHA `7cb1357c…` (BS-1 weights).
  - `top10-2024-fy-tcn-overlay-weights` → body SHA `23c24dae…` (BS-2 weights).
- **`spec/anchors.toml` extended** with two new rows under version
  `v2.5.0-tcn-weights` (anchors.toml lines 113–121). Comment block makes the
  synthetic-data caveat explicit.
- **`crates/forecast/tests/anchors_load.rs`** — 3 new `--features candle`
  smoke tests: BS-1 load + forward, BS-2 load + forward, BS-1 forward
  deterministic. 3/3 PASS.
- **`crates/backtest/tests/determinism.rs`** — 2 new
  `#[cfg(feature = "candle")]` anchor-regression tests
  (`m3_top10_2023_fy_tcn_overlay_weights_anchor_hash_unchanged`,
  `m3_top10_2024_fy_tcn_overlay_weights_anchor_hash_unchanged`). Both PASS in
  the 22/22 candle suite (611 s).
- **Strategy wiring** — `TcnOverlayMomentumStrategy::with_tcn_bs1()` /
  `with_tcn_bs2()` constructors under `#[cfg(feature = "forecast")]` load the
  LFS checkpoints into the strategy. Backtest grew a
  `ScenarioStrategy::TcnOverlayMomentumWeights` variant +
  `run_tcn_overlay_weights_backtest()` (errors explicitly if `--features candle`
  is absent — keeps the CI path candle-free).
- **Feature gate propagation** — `crates/backtest/Cargo.toml` got
  `candle = ["strategy/forecast"]`; `crates/strategy/src/lib.rs` re-exports
  `TcnSyncForecaster` under `#[cfg(feature = "forecast")]`.
- **Two M3 training reports** authored:
  [`m3-bs1-training-2026-05-18.md`](../reports/m3-bs1-training-2026-05-18.md)
  and
  [`m3-bs2-training-2026-05-18.md`](../reports/m3-bs2-training-2026-05-18.md).
  Both carry the verbatim canonical metadata JSON, comparison tables, and an
  explicit "Finding: TCN model outputs Flat on synthetic data" section.
- **`spec/trace.toml`** — REQ-V25-TCN-001 `anchors` column extended from 2 → 4
  names. `tests` array now lists `crates/forecast/tests/anchors_load.rs` and
  `crates/backtest/tests/determinism.rs`. `unreferenced-anchor` count from
  spec-lint remains 0.
- **`tasks.md`** — T-D-11 and T-D-12 ticked with full developer citations
  (checkpoint paths, test names + outputs, anchor SHAs, determinism-test
  names, verify_anchors PASS).
- **No change to the 13 pre-existing anchors.** All `v0`..`v2.0.0` rows plus
  the two `v2.5.0` passthrough rows remain byte-identical (confirmed live —
  see Live demo).

## Why

Per [ADR-0028](../../architecture/adr/0028-v25-dl-forecast-overlay-candle.md)
and [ADR-0029](../../architecture/adr/0029-tcn-checkpoint-provenance.md): the
CI-baseline gate proved the wiring; M3 proves the weights pipeline. We need a
real trained checkpoint anchored under provenance JSON to (a) make
`verify_anchors.sh` a meaningful regression gate against the actual TCN code
path (not just the passthrough degenerate case); (b) hand v2.5a (PatchTST)
and v2.5b (Transformer) a fully populated `crates/forecast/checkpoints/anchors/`
LFS pattern they can copy; (c) lock the canonical-JSON metadata format under
real, non-toy training runs so the cross-phase contract is battle-tested
before two more models inherit it. The dampened=0 finding on synthetic data
is itself diagnostic — it tells us the model is distribution-sensitive (a
safety property), and it pins exactly which next work-item unlocks alpha
evaluation: real Binance parquet feeding the existing
`windows_for_symbol()` iterator.

## Honest finding — synthetic-data gap (read this before approving)

Both new real-weights scenarios produce **byte-identical results to their
passthrough-forecaster counterparts**. This is by design and is documented in
both training reports under §"Finding: TCN model outputs Flat on synthetic data".

> The TCN models were trained on real Binance hourly OHLCV with characteristic
> distributional properties (volatility clustering, fat tails, autocorrelation,
> overnight gaps). The backtest harness currently runs against ChaCha20Rng GBM
> synthetic data — i.i.d. Gaussian log-returns. The model's `r_hat` output
> falls inside the `ε = 0.0005` deadband for every bar on this distribution,
> producing `Direction::Flat` for every signal, which causes 100 %
> pass-through (`dampened = 0`). The model has no signal on
> out-of-distribution data — which is the correct, safety-preserving
> behaviour.

The implication:

| Property | Locked by these anchors | NOT locked by these anchors |
|----------|-------------------------|------------------------------|
| Candle feature compiles + propagates | YES | — |
| LFS checkpoints load deterministically | YES | — |
| `model_revision` SHA stable across runs | YES | — |
| Inference is bit-identical on CPU | YES | — |
| Backtest body-SHA stable across runs | YES | — |
| **TCN beats v1 momentum on real crypto** | — | NO — needs `backtest-real-binance-data` |
| **Sharpe / max-DD success criteria from feature.md** | — | NO — synthetic data has no alpha by construction |

Reference text in the training reports: BS-1 §"Finding: TCN model outputs
Flat on synthetic data" (`m3-bs1-training-2026-05-18.md` lines 142–163);
BS-2 §"Finding: TCN model outputs Flat on synthetic data"
(`m3-bs2-training-2026-05-18.md` lines 147–165).

**Next work-item that unlocks alpha evaluation**:
`backtest-real-binance-data` — wire the backtest harness's bar source to
`windows_for_symbol()` reading real parquet under `data/binance/`, so v2.5
/ v2.5a / v2.5b can be compared on the actual training distribution. This is
also a v2.6 bake-off prerequisite.

## What you can do now

| Action | Command |
|--------|---------|
| Run real-weights BS-1 backtest (candle, 2023) | `cargo run -p backtest --release --features candle -- --scenario top10-2023-fy-tcn-overlay-weights --seed 0xC0FFEE` |
| Run real-weights BS-2 backtest (candle, 2024) | `cargo run -p backtest --release --features candle -- --scenario top10-2024-fy-tcn-overlay-weights --seed 0xC0FFEE` |
| Verify all 15 anchors byte-stable (incl. 2 new) | `bash scripts/verify_anchors.sh` |
| Run the full candle determinism suite (22/22) | `cargo test -p backtest --test determinism --features candle` |
| Run the anchor-load smoke tests | `cargo test -p forecast --features candle --test anchors_load` |
| Inspect a checkpoint's metadata JSON | `cat crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d*.metadata.json` |
| Optional: Metal-vs-CPU drift gate (Apple Silicon) | `cargo test -p forecast --features metal --test metal_cpu_drift` |

## Live demo — `verify_anchors.sh` (15/15 PASS, all four TCN anchors green)

Run live against commit `a12fc6e` at presenter time:

```
$ bash scripts/verify_anchors.sh
PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
PASS  report-sample-7d                      520b1f2968ad52d5981a1cdb3749235416c77c058364bd8c11ebd7d2468f46a3
PASS  report-sample-90d                     c656414ebf6f526372c27ae2d537301c68a0bc71d896f5a7cbc65a02edd60333
PASS  top10-2023-fy-tcn-overlay             01d02584331c4a26334e7c1fb9bd3f16287a6d2024263f869c9658708893eef5
PASS  top10-2024-fy-tcn-overlay             e24c85ac695d9f8f5d4e7f7a8d47f8d33f5567bb02b0be051b6fc76bf4496163
PASS  top10-2023-fy-tcn-overlay-weights     7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4
PASS  top10-2024-fy-tcn-overlay-weights     23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b
---
ANCHORS PASS  (15 / 15)
```

The two bottom rows (`top10-2023-fy-tcn-overlay-weights`,
`top10-2024-fy-tcn-overlay-weights`) are the new M3 entries. The 13 above
them are the pre-existing anchors — byte-identical to their values from the
CI-baseline approval this morning.

### Per-report body-SHA cross-check (live `hash_report.py` output)

```
$ uv run scripts/hash_report.py spec/v25-tcn-overlay/reports/backtest-20260518-102202-top10-2023-fy-tcn-overlay-weights.md
7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4  spec/v25-tcn-overlay/reports/backtest-20260518-102202-top10-2023-fy-tcn-overlay-weights.md

$ uv run scripts/hash_report.py spec/v25-tcn-overlay/reports/backtest-20260518-102845-top10-2024-fy-tcn-overlay-weights.md
23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b  spec/v25-tcn-overlay/reports/backtest-20260518-102845-top10-2024-fy-tcn-overlay-weights.md
```

Both computed hashes match the values locked in `spec/anchors.toml`
(lines 113–121). The byte-stability fingerprint of the real-weights path is
intact.

### Backtest report — `top10-2023-fy-tcn-overlay-weights` (BS-1) — header verbatim

```
$ head -47 spec/v25-tcn-overlay/reports/backtest-20260518-102202-top10-2023-fy-tcn-overlay-weights.md
---
scenario: top10-2023-fy-tcn-overlay-weights
seed: 0xC0FFEE
generated: 2026-05-18T10:22:02Z
wall_clock_s: 205.2
data_source: synthetic (seeded RNG, v2.5 tcn-overlay-weights)
...
# Backtest Report — top10-2023-fy-tcn-overlay-weights

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | top10-2023-fy-tcn-overlay-weights |
| Universe             | 10 symbols                    |
| Start year           | 2023                          |
| Bars (total)         | 22080                         |
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

> The `Dampened to Hold = 0` row is the load-bearing tell: real TCN weights
> running, but the synthetic data lives entirely inside the ε=0.0005 deadband.
> The −69.76 % equity / 87.48 % max-DD numbers are the v1 momentum baseline
> on seeded random walks; momentum has no edge on a random walk by
> construction. These are byte-stability fingerprints, not strategy results.

## Screenshots

_n/a — backend-only ship. No UI surface in this gate (the TCN overlay is a
strategy-layer module; cockpit consumes it transparently)._

## Verification matrix

| Gate | Status | Evidence |
|------|--------|----------|
| V1 — Determinism (100 inference calls → byte-identical `ForecastOverlay`) | VERIFIED | `crates/forecast/tests/anchors_load.rs::td11_bs1_forward_deterministic` PASS (test-2026-05-18-1230-m3 §3). Two forward passes on same input → byte-identical tensors. |
| V2 — Replay (BS-1 + BS-2 reproduce to byte-identical PnL on second run) | VERIFIED | Tester re-run via `hash_report.py` matches `anchors.toml` SHA exactly for both `-weights` scenarios. BS-1 `7cb1357c…`, BS-2 `23c24dae…`. Test report §5.2 / §5.3 + presenter live re-run above. |
| V3 — Anchor lock (both new `-weights` rows land in `spec/anchors.toml`) | VERIFIED | `spec/anchors.toml` lines 113–121 under version `v2.5.0-tcn-weights`; comment block at lines 105–111 documents the synthetic-data caveat. |
| V4 — Existing 13 anchors stay byte-identical | VERIFIED | `verify_anchors.sh` live above: 15/15 PASS, top 13 rows match all prior locked SHAs. Tester §7.4. |
| V5 — Candle determinism integration suite (22/22) | VERIFIED | `cargo test -p backtest --test determinism --features candle` → 22/22 PASS in 611.27 s, including both new `m3_top10_{2023,2024}_fy_tcn_overlay_weights_anchor_hash_unchanged` tests. Test report §3 + §7.2. |
| V6 — `cargo fmt --check` clean | VERIFIED | Test report §2 row 1 — zero diffs. |
| V7 — `cargo clippy --workspace -- -D warnings` clean | VERIFIED | Test report §2 row 2 — zero warnings; `cargo build --workspace` clean in 10.71 s. |
| V8 — `cargo build --workspace --features candle` | VERIFIED | Test report §2 row 3 — candle feature propagates through `strategy/forecast`; backtest binary builds without error. |
| V9 — `spec-lint` no new regressions vs tester PASS baseline | VERIFIED | Tester baseline 733 / 2; presenter live re-check at write time: **733 / 2** (identical). No new categories, no new counts. |
| V10 — `spec/trace.toml` REQ-V25-TCN-001 `anchors` column complete | VERIFIED | All 4 anchor names present and resolving (2 v2.5.0 + 2 v2.5.0-tcn-weights). `unreferenced-anchor` spec-lint count = 0. Test report §7.3. |
| V11 — T-D-11 / T-D-12 developer citations re-verified | VERIFIED | Test report §8: every cited checkpoint path, test name, output line, and anchor SHA confirmed individually. No overclaims. |
| V12 — M3 training reports satisfy documentation contract | VERIFIED | Test report §7.4 — both reports carry verbatim canonical metadata JSON, comparison tables, synthetic-data disclosure, and reproduction recipes. |

## Numbers that matter

- **Tests**: 22/22 candle determinism + 3/3 forecast `anchors_load` + 20/20
  no-candle determinism. **0 failures**, 4 pre-existing `#[ignore]`.
- **Anchors**: **15/15** PASS (13 pre-existing byte-identical + 2 new
  `v2.5.0-tcn-weights`).
- **New anchor SHAs** (first 8 chars):
  - `top10-2023-fy-tcn-overlay-weights` → `7cb1357c…`
  - `top10-2024-fy-tcn-overlay-weights` → `23c24dae…`
- **Checkpoint provenance SHAs** (`model_revision`):
  - BS-1 → `d1c3696d…`, weights_sha256 `4ed9064a…`, sigma_train `10.954`.
  - BS-2 → `3fabcabe…`, weights_sha256 `5f22b5bc…`, sigma_train `6.916`.
- **Training metrics** (Huber loss):
  - BS-1: final train `1.217e-5`, val `1.539e-5`, 30 epochs.
  - BS-2: final train `8.001e-6`, val `1.051e-5`, 30 epochs (lower than BS-1
    on the larger training corpus — expected).
- **Backtest wall-clock** (real weights, candle):
  - BS-1 ~205 s on 22 080 bars.
  - BS-2 ~608 s on 66 000 bars.
- **Spec-lint baseline (presenter re-check at write time)**: `spec-lint:
  FAIL (733 violations in 2 categories)` — identical to tester at PASS.
  Categories: `dead-link` (727, pre-existing) +
  `trace-broken-path` (6, future-phase rows for v2.5a / v2.5b / v2.6).
- **TCN topology fingerprint**: 8 blocks, dilations
  `[1,2,4,8,16,32,64,128]`, k=3, H=96, ~4.4 M params,
  receptive field 1 021 bars (~42 days at 1 h).
- **Predecessor approval**: CI-baseline gate approved this morning via
  [`v25-tcn-overlay-2026-05-18.md`](v25-tcn-overlay-2026-05-18.md).

## Risks / known gaps

1. **Synthetic data has no alpha by construction.** The
   `dampened = 0 / Dampen rate = 0.00 %` lines in both backtest reports are
   the load-bearing tell that the TCN sees i.i.d. Gaussian returns and
   correctly emits Flat. The −69.76 % / −55.70 % equity numbers are v1
   momentum on random walks — NOT a strategy regression. Treat the new
   anchors as byte-stability fingerprints, not as strategy results.
2. **Real-data alpha evaluation is deferred.** It is queued as
   `backtest-real-binance-data` (next backlog item; also a v2.6 bake-off
   prerequisite). Until it lands, success criteria in
   [`feature.md § Backtest Scenarios`](../feature.md#backtest-scenarios)
   (Sharpe ≥ v1 + 0.10, max-DD ≤ v1 + 2 pp, trades ≤ 1.5 × v1) cannot be
   evaluated.
3. **Metal-vs-CPU bit-identity is not proven.** Per ADR-0029 § D2, candle's
   Metal kernels use non-deterministic reduction order on some ops. Strategy:
   CPU is the determinism oracle; Metal is allowed for training only. The
   LFS-tracked checkpoints mitigate — we ship the weights rather than a
   re-train recipe. The drift gate (`max_abs < 1e-4`) is gated behind
   `--features metal` and runs on Apple Silicon only — operator-deferred
   per tasks.md T-D-7.
4. **Pre-existing spec debt unchanged.** 727 dead-links + 6 future-phase
   `trace-broken-path` rows persist. None are caused by this feature; both
   buckets are tracked in
   [`spec/dev-notes/audit-2026-05-18.md`](../../dev-notes/audit-2026-05-18.md).
   M3 introduced **zero** new spec-lint violations.
5. **`cargo audit` not installed; `cargo deny` pre-existing FAIL** —
   `RUSTSEC-2024-0436` (`paste` unmaintained), `MIT-0` license violations.
   Both pre-date v2.5 and are tracked separately. Test report §2 carries
   these forward unchanged.

## Open decisions

> One decision, load-bearing: close the M3 milestone gate. The feature
> status remains `in-progress` regardless (phase 1 of 4; v2.5.0 ships only
> after the v2.6 bake-off).

1. **Approve closing the M3 real-weights anchor gate.** Effects of approval:
   - T-D-11 and T-D-12 stay ticked.
   - The two new `v2.5.0-tcn-weights` anchors stay locked. They join the
     byte-stability contract and will fail `verify_anchors.sh` if any future
     change reaches the real-weights candle code path.
   - The synthetic-data finding is **acknowledged as a known limitation,
     not a regression**.
   - Orchestrator schedules `backtest-real-binance-data` as the next work
     item (parquet feed → real distribution → `dampened > 0` observable).
   - Feature status correctly stays `in-progress`.
   - Cost to revert: low — both new anchor rows can be deleted from
     `anchors.toml`; the two `m3_*` determinism tests would need to be
     deleted or `#[ignore]`-d.

   This is **NOT a full strategy-ship approval**. v2.5 ships when v2.6
   closes the 4-phase bake-off (TCN vs PatchTST vs Transformer vs v1).

## Roadmap forward

Three concrete next-up items, in priority order:

| # | Item | Why now | What it unblocks |
|---|------|---------|------------------|
| a | **`backtest-real-binance-data`** (backlog) — wire the backtest bar source to `windows_for_symbol()` reading real Binance parquet at `data/binance/`. | The single biggest information gain: lets v2.5 / v2.5a / v2.5b be alpha-compared on the actual training distribution. Without it, dampened=0 is structural. | Real-data alpha evaluation for v2.5; v2.6 bake-off prerequisite. |
| b | **Phase 2 — v25a-patchtst-overlay** (reserved on the 4-phase roadmap). | Reuses the entire M3 infrastructure: same training-loop binary shape, same canonical-JSON provenance (ADR-0029), same LFS-anchor pattern, same trace-row schema. | Second model in the bake-off; quantile-classification paradigm contrast vs TCN regression. |
| c | **Optional: Metal-vs-CPU drift exit gate** (T-D-7, operator-deferred). Run `cargo test -p forecast --features metal --test metal_cpu_drift` on Apple Silicon. | Confirms `max_abs < 1e-4` and no Direction-flip on Metal vs CPU. If it fails, the LFS-anchor strategy still holds; if it passes, future re-trains can run Metal-fast without anchor risk. | Operational confidence in Metal training; not load-bearing for anchor verification. |

## Approval

- [ ] Approved — close M3 milestone gate
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

### Notes / feedback

<empty until operator fills>

## Closing gates

Pre-tick guard (`bash scripts/check_presentation.sh`) and spec-lint
(`uv run scripts/spec_lint.py`) results quoted verbatim:

```
$ bash scripts/check_presentation.sh spec/v25-tcn-overlay/presentations/v25-tcn-overlay-m3-2026-05-18.md
PRESENTATION CHECK PASS  (spec/v25-tcn-overlay/presentations/v25-tcn-overlay-m3-2026-05-18.md — approval block UN-ticked)

$ uv run scripts/spec_lint.py
spec-lint: FAIL (733 violations in 2 categories)
```

Spec-lint result `733 / 2` is **identical** to the tester's baseline at
`VERDICT → PASS` (test report §2.1). No new categories, no new counts. The
`spec/dev-notes/audit-2026-05-18.md` baseline was originally 734 / 3; the
delta (−1 violation, −1 category) is the `ui-rethink-phase-a-lab/tasks.md`
`missing-frontmatter` having been resolved on an unrelated feature in the
meantime. No regression introduced since `VERDICT → PASS`.

## Changelog

- 2026-05-18 (presenter): initial draft of the M3 follow-on deck.
  Real-weights anchor gate closed; two new `v2.5.0-tcn-weights` anchors
  locked; honest synthetic-data finding surfaced; roadmap-forward to
  `backtest-real-binance-data` queued. Predecessor approval
  ([`v25-tcn-overlay-2026-05-18.md`](v25-tcn-overlay-2026-05-18.md))
  cited. Tester `VERDICT → PASS` at commit `a12fc6e`. HANDOFF → operator.
