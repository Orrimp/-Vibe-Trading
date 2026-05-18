---
slug: backtest-real-binance-data
mode: release
status: draft
audience: human-operator
updated: 2026-05-18
generated: 2026-05-18T19:00:00Z
feature_version: 0.1.0
commit: d98622e
tester_verdict: PASS
---

# Real-Binance-data backtest path — release

## TL;DR

The backtest harness now reads **real Binance hourly OHLCV** (240 parquet
files, 10 USDT pairs, 2 years) behind the new `realdata` cargo feature; four
new `-realdata` anchors are locked under version `v2.6.0-realdata` and the
15 original synthetic anchors stay **byte-identical**. Wire-only ship —
alpha-verdict (Sharpe vs v1 momentum) is the next v2.5 follow-on.

## What changed

- **New `realdata` cargo feature on `crates/backtest`** (off by default).
  Default build never compiles the new code path; the CI-on-empty-disk
  floor is preserved.
- **New parquet read path:** `crates/backtest/src/realdata.rs` reuses
  `data::ReplayFeed::merge_symbols` (no cross-crate dep on `forecast`).
- **`data/binance/REVISION.toml` manifest** (writer + verifier in
  `crates/data/src/revision.rs`) pins every parquet's SHA-256; aggregate
  SHA flows into report frontmatter (forensics, excluded from body hash)
  AND into a new body `## Data source` section (anchor-integrity, included
  in body hash).
- **Orthogonal `ScenarioDataSource::{Synthetic, RealData}`** axis on
  `Scenario` (existing `ScenarioStrategy` enum untouched per ADR-0032 D3).
- **Four new scenarios**: `top10-{2023,2024}-fy-tcn-overlay-realdata` plus
  the `-weights-realdata` variants for real TCN inference under
  `--features candle`.

## Why

The TCN models trained in `v25-tcn-overlay` M3 produce `dampened=0` on the
synthetic-GBM backtest path because the trained `r_hat` falls inside the
`ε=0.0005` deadband on i.i.d. Gaussian returns. That blinded v2.5 alpha
evaluation by construction (out-of-distribution input) AND blocked the v2.6
forecast bake-off prerequisite (PatchTST / Transformer would have landed
with the same blind spot). This feature wires the harness onto real Binance
1h data already on disk while preserving the existing 15 synthetic anchors
as a CI floor — the two contracts (strategy/audit byte-stability vs
real-data distribution lock) are kept orthogonal so they fail independently.
See [`spec/backtest-real-binance-data/feature.md` § Why](../feature.md#why)
and [ADR-0032](../../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md).

## Signal-quality finding — surface before approval

**The TCN real-weights path produces `dampened=0` on real Binance data
too**, not just synthetic. The M3-era hypothesis — that the `Flat`
outcome was caused by synthetic Gaussian returns — is **incomplete**.

On 2023fy and 2024fy real-data scenarios, the trained TCN's `r_hat` still
sits inside the `ε=0.0005` deadband (table from the tester report):

| Scenario                                        | Bars  | Passed through | Dampened | Warming-up | Dampen rate |
|-------------------------------------------------|------:|---------------:|---------:|-----------:|------------:|
| top10-2023-fy-tcn-overlay-realdata              | 87590 | 6070           | 0        | 133        | 0.00%       |
| top10-2024-fy-tcn-overlay-realdata              | 87840 | 5800           | 0        | 117        | 0.00%       |
| top10-2023-fy-tcn-overlay-weights-realdata      | 87590 | 6070           | 0        | 133        | 0.00%       |
| top10-2024-fy-tcn-overlay-weights-realdata      | 87840 | 5800           | 0        | 117        | 0.00%       |

Identical signal counts between passthrough and real-weights rows mean the
weights path degrades to passthrough at the strategy level (with zero
dampenings, both code paths produce identical equity). The trained final
val Huber loss of `~1.5e-5` (M3 BS-1/BS-2 checkpoints) is suspiciously
tiny in hindsight — the model may have learned to predict ≈zero rather
than the next-bar log-return.

**This is NOT a regression** — the feature is wire-only scope (R8); the
finding is honest reporting from the tester report's "Key Observations"
section. It surfaces a real v2.5 follow-on: re-train at longer horizons,
loosen the `ε` deadband or the confidence threshold, and inspect the
predictions directly. See _Roadmap forward_ below.

## What you can do now

| Action                                              | Command |
|-----------------------------------------------------|---------|
| Re-verify all 19 anchors (5 sec)                    | `bash scripts/verify_anchors.sh` |
| Run 2023 real-data backtest (passthrough, ~3s)      | `cargo run -p backtest --release --features realdata -- --scenario top10-2023-fy-tcn-overlay-realdata --seed 0xC0FFEE` |
| Run 2024 real-data backtest (real TCN weights, ~40s)| `cargo run -p backtest --release --features "realdata candle" -- --scenario top10-2024-fy-tcn-overlay-weights-realdata --seed 0xC0FFEE` |
| Re-emit the data manifest (only if data refreshed)  | `cargo run -p data --bin fetch_binance_klines --release -- --symbols ADAUSDT,AVAXUSDT,BNBUSDT,BTCUSDT,DOGEUSDT,DOTUSDT,ETHUSDT,LINKUSDT,SOLUSDT,XRPUSDT --start 2023-01-01 --end 2024-12-31 --interval 1h --out data/binance --emit-revision-manifest` |
| Inspect the 2023fy report                           | `open spec/backtest-real-binance-data/reports/backtest-20260518-175640-top10-2023-fy-tcn-overlay-realdata.md` |

## Live demo

`bash scripts/verify_anchors.sh` against the locked 19-anchor set (`v2.6.0-realdata`
rows are the last 4):

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
PASS  top10-2023-fy-tcn-overlay-realdata    8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642
PASS  top10-2024-fy-tcn-overlay-realdata    fd8191dff1ca106ca24416a1819bd8a002c705da7f3747831f48d60733ee76f3
PASS  top10-2023-fy-tcn-overlay-weights-realdata  552d7df294bc93ff6f887874f919aeeb8106a62caae4ad5ec5de7c5b49665d70
PASS  top10-2024-fy-tcn-overlay-weights-realdata  2a65c4347964a0748877606d9c3a8b261b7fee6e069a814e64aaa024419f2f2c
---
ANCHORS PASS  (19 / 19)
```

The first 15 rows are the pre-existing synthetic + operator-success
anchors — all byte-identical (K10 anchor-neutrality PASS). The last 4
rows are the new `v2.6.0-realdata` anchors locked by the tester at T-T-1A.

## Screenshots

_n/a — non-UI feature_ — the backtest binary emits markdown reports
verifiable by SHA, and the live-demo block above is the operator-facing
artifact.

## Verification matrix

| V-id | Description                                                                                | Status   | Evidence |
|------|--------------------------------------------------------------------------------------------|----------|----------|
| V1   | Two sequential runs of each new scenario produce byte-identical body SHA-256 (R8 #2; K5)   | VERIFIED | Test report § 5 K5 table — all 4 scenarios `match: PASS`; 22/22 (`realdata`) + 26/26 (`realdata,candle`) determinism suites green |
| V2   | Four new anchors locked at `v2.6.0-realdata` (R8 #3)                                       | VERIFIED | `spec/anchors.toml` lines 130-148; live `verify_anchors.sh` above |
| V3   | 15 pre-existing anchors stay byte-identical (R8 #4 + K10 mitigation)                       | VERIFIED | `ANCHORS PASS (19 / 19)` — first 15 rows match pre-feature SHAs |
| V4   | Workspace tests green under `--features realdata`                                          | VERIFIED | Test report § 3 — `cargo test --workspace` ~123 passed / 0 failed; realdata determinism 22/22 |
| V5   | `rust-validate` clean (fmt + clippy -D warnings + cargo-deny + docs)                       | VERIFIED | Test report § 2 — `cargo fmt --check` PASS, clippy clean across default / +realdata / +realdata,candle |
| V6   | CI-portable: default-features build passes 15 existing anchors without `data/binance/`     | VERIFIED | Test report § 10 #3 — default build + workspace tests pass; the 4 new scenarios only compile under `#[cfg(feature = "realdata")]` |
| V7   | `data_revision_sha` propagation through frontmatter + body `## Data source` section        | VERIFIED | `backtest-20260518-175640-...-realdata.md` lines 5-6 + 62-72 — frontmatter line + 7-row body table |
| V8   | Missing-data tolerance algorithm (≥99.5% cross-symbol cover) hard-fails below threshold    | VERIFIED | `realdata_revision_verify.rs` 4/4: happy-path + tamper + missing manifest + 0.6% gap |
| V9   | `REVISION.toml` writer ↔ verifier roundtrip stable at 240-file production scale            | VERIFIED | `crates/data/src/revision.rs` 250-file regression test; `data::revision` 6/6 + 1 ignored (production roundtrip, runs under `--ignored`) |
| V10  | Trace row `REQ-BACKTEST-REALDATA-001` shipped, anchors / tests / crates fields filled      | VERIFIED | Test report § 9 + `feature.md` frontmatter `status: shipped` |

## Numbers that matter

- **Tests:** 123 passed, 0 failed across workspace default-features; 22/22
  realdata determinism; 26/26 realdata,candle determinism; 4/4
  `realdata_revision_verify`; 6/6 `data::revision` (+1 ignored 240-file
  production manifest test, runs under `--ignored`).
- **Anchors:** 19/19 PASS (15 originals byte-identical, 4 new locked).
- **Static analysis:** `cargo fmt --check` PASS; clippy `-D warnings` clean
  across 3 feature combinations (default / `realdata` / `realdata,candle`).
- **Data scope:** 240 parquet files, ≈6.0 MB on disk; 10 USDT pairs ×
  24 months × 1h cadence.
- **Coverage vs expected bars:** 2023 = 87 590 / 87 600 (99.99%, within
  R3 99.5% threshold); 2024 = 87 840 / 87 840 (100.00%).
- **Wall-clock per scenario** (Apple Silicon):
  - passthrough realdata: ~3 sec (full year × 10 sym).
  - real-TCN-weights realdata (candle): ~38–40 sec.
  - Both well under the R5 < 90 sec budget.
- **Determinism (K5):** 4/4 scenarios produced byte-identical body SHAs
  across two independent runs at tester time; orchestrator-level
  cross-machine re-runs (10+) all stable.
- **Aggregate data revision SHA**:
  `3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7`.

### Four new anchor SHAs (locked at `v2.6.0-realdata`)

| Scenario                                            | Body SHA-256 |
|-----------------------------------------------------|--------------|
| `top10-2023-fy-tcn-overlay-realdata`                | `8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642` |
| `top10-2024-fy-tcn-overlay-realdata`                | `fd8191dff1ca106ca24416a1819bd8a002c705da7f3747831f48d60733ee76f3` |
| `top10-2023-fy-tcn-overlay-weights-realdata`        | `552d7df294bc93ff6f887874f919aeeb8106a62caae4ad5ec5de7c5b49665d70` |
| `top10-2024-fy-tcn-overlay-weights-realdata`        | `2a65c4347964a0748877606d9c3a8b261b7fee6e069a814e64aaa024419f2f2c` |

Source: tester report § 5 + `spec/anchors.toml:130-148`.

### Equity snapshot (informational — alpha verdict is out of scope)

These are the strategy's real-data run figures; they are NOT a Sharpe
table. Numbers exist only because the scenarios ran end-to-end.

| Scenario                                      | Year | Final equity | Total return | Max drawdown | Trades |
|-----------------------------------------------|------|-------------:|-------------:|-------------:|-------:|
| top10-2023-fy-tcn-overlay-realdata            | 2023 |  $113 479.97 |      +13.48% |       73.73% |   6203 |
| top10-2024-fy-tcn-overlay-realdata            | 2024 |  $105 214.24 |       +5.21% |       78.82% |   5917 |
| top10-2023-fy-tcn-overlay-weights-realdata    | 2023 |  $113 479.98 |      +13.48% |       73.73% |   6203 |
| top10-2024-fy-tcn-overlay-weights-realdata    | 2024 |  $105 214.25 |       +5.21% |       78.82% |   5917 |

Identical equity between passthrough and weights rows is consistent with
`dampened=0` — the weights path degrades to v1 momentum at the strategy
level. Sharpe / Sortino / drawdown-vs-baseline comparison is the
v25-tcn-overlay tester re-spawn (see Roadmap below).

## What the operator approves

Approving "ship" closes the `backtest-real-binance-data` ship gate and:

1. Flips the feature status to `shipped` (already pre-flipped by the
   tester; this confirms the operator gate).
2. Locks `v2.6.0-realdata` as the new regression anchor version for the
   real-data backtest distribution contract.
3. Queues a v2.5 follow-on: re-spawn the v25-tcn-overlay tester against
   the four new `-realdata` anchors to produce the Sharpe / drawdown /
   trade-count comparison vs the v1 momentum baseline. The
   `dampened=0`-on-real-data finding is the operative datum.
4. Preserves the synthetic-anchor CI floor by design — operators with
   no `data/binance/` on disk continue to get the 15-anchor synthetic
   gate; nothing about today's CI changes for them.

## Open decisions

1. **Approve ship.** No outstanding operator-decide blocks: Q1 (parallel
   `-realdata` family), Q4 (10 USDT pairs on disk), Q8 (wire-only scope)
   were all confirmed at analyst defaults on 2026-05-18 (T-OP-1/2/3 ticked,
   see `feature.md` Changelog). The only open box is the operator's
   ship/no-ship decision below.

## Roadmap forward

Three queued items, ranked by load-bearing:

a. **Alpha-verdict eval (v2.5 follow-on, high priority).** Spawn the
   `v25-tcn-overlay` tester against the four new `-realdata` anchors to
   produce a Sharpe / drawdown / trade-count comparison vs the v1
   momentum baseline (`top10-{2023,2024}-1h-momentum`). Given the
   `dampened=0` finding on real data, expect the verdict to surface a
   re-train / ε-loosen / confidence-threshold investigation rather than
   "TCN adds alpha". This is honest signal and the right next move.

b. **`v25a-patchtst-overlay` (phase 2 of DL roadmap).** PatchTST can now
   anchor against real data from the start — no synthetic detour. This
   unblocks the v2.6 forecast bake-off (TCN / PatchTST / Transformer
   head-to-head on real Binance 1h returns).

c. **Metal-vs-CPU drift exit gate (T-D-7 from v25-tcn-overlay backlog).**
   Operator-local Apple Silicon run to confirm Metal-accelerated `tract`
   inference matches CPU bit-for-bit on the tcn-bs1/bs2 checkpoints.
   Low priority; independent of this feature.

## Approval

- [ ] Approved — ship; queue alpha-verdict v25-tcn re-spawn
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

### Notes / feedback

<empty until operator fills>

## Closing gates

Both mandatory gates run after the deck landed. Verbatim:

```
$ bash scripts/check_presentation.sh spec/backtest-real-binance-data/presentations/backtest-real-binance-data-2026-05-18.md
PRESENTATION CHECK PASS  (spec/backtest-real-binance-data/presentations/backtest-real-binance-data-2026-05-18.md — approval block UN-ticked)
```

```
$ uv run scripts/spec_lint.py
spec-lint: FAIL (733 violations in 2 categories)
dead-link (727):
trace-broken-path (6):
```

The `spec-lint: FAIL` line is the baseline — 733 violations in 2
categories matches the expected baseline from the brief (and is in fact
2 dead-links lower than the tester report's 735/2 snapshot, since the
trace-row anchor fill resolved them). **No new spec-lint regression
since the tester's PASS**; the 6 `trace-broken-path` rows are forward
roadmap anchors (PatchTST / Transformer / bake-off) not yet locked, and
the 727 `dead-link` rows are pre-existing lumen-design phase folder
renames already tracked in `spec/dev-notes/audit-2026-05-18.md`.

## Changelog

- 2026-05-18 (presenter): initial draft. Pulled tester report
  (`test-20260518-1800-backtest-real-binance-data.md`, VERDICT PASS at
  commit `df73780`; brief cites `d98622e`), four canonical real-data
  backtest reports, ADR-0032, M5 revision-pin capture, and live
  `verify_anchors.sh` output (19/19). Surfaces the `dampened=0`-on-real-
  data finding from § 5 of the tester report and routes it to a v2.5
  alpha-verdict re-spawn in the roadmap. One open decision: operator
  ship-approval.
