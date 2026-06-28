---
slug: binance-corpus-expansion
date: 2026-06-15
mode: release
phase: presenter
audience: operator
anchored: false
---

# Operator deck — Binance corpus expansion (2021–22 down-market data)

## TL;DR

We fetched and pinned the 2021–22 hourly price history for the 10-symbol
Binance universe as a new, gitignored dataset — it is verified byte-safe (the
existing corpus and all 119 regression anchors are untouched) and ready to ship.

## What changed

- **Added a new pinned dataset** `data/binance-2122/` — 2021–2022 hourly OHLCV
  (open/high/low/close/volume bars) for the same 10 coins as the existing corpus
  (BTC, ETH, BNB, SOL, XRP, ADA, DOGE, AVAX, DOT, LINK). 240 parquet files,
  ~5.6 MB on disk. All 10 symbols have full coverage from 2021-01 (no thin
  early-2021 months).
- **Only the pin is committed to git.** The 240 data files are gitignored; the
  single tracked file is `data/binance-2122/REVISION.toml` — a content fingerprint
  (aggregate SHA-256 `4f390622…`) that lets anyone re-fetch the exact same bytes
  later. Zero repo-size impact beyond a ~30 KB manifest.
- **No code, no strategy, no engine change.** This is pure data infrastructure —
  it reuses the shipped fetch tool verbatim. The forward layout convention was
  recorded as a one-page architecture decision (ADR-0056, own-root-per-timeframe).

## Why

The 2026-06-13 strategy survey found that trend-following (SMA, MACD) protects
capital in *down* markets while buy-and-hold bleeds — the genuinely ship-relevant
result. But that finding rests on only **2 down-market data points** (AVAX and DOT
in 2024), because 2023 and 2024 were both broadly bull years; the corpus simply
contained no real bear market. **2022 was the deep crypto bear** — BTC ≈ −64% on
the year, with the LUNA and FTX collapses inside it — a market-wide, multi-month
drawdown across the *whole* universe. Adding 2021–22 turns the 2-point down sample
into a whole-universe bear sample, which is exactly what's needed to firm up (or
falsify) the down-market-hedge claim.

## What the operator can do now

- **Re-fetch / reproduce the exact dataset** on any machine (read-only historical
  HTTP, no live trading):

  ```bash
  cargo run -p data --bin fetch_binance_klines -- \
    --symbols BTCUSDT,ETHUSDT,BNBUSDT,SOLUSDT,XRPUSDT,ADAUSDT,DOGEUSDT,AVAXUSDT,DOTUSDT,LINKUSDT \
    --start 2021-01-01 --end 2022-12-31 --interval 1h \
    --out data/binance-2122 --emit-revision-manifest
  ```

  Re-running is idempotent and yields the identical pin `4f390622…` (verified by
  the tester — see V4).

- **Confirm the pin is internally consistent** with no data files present (cheap,
  CI-safe, runs on the committed manifest alone):

  ```bash
  cargo test -p data --test binance_2122_revision_consistency manifest_internal_consistency
  ```

- **Re-run the live ground-truth probe** this deck embeds:

  ```bash
  bash -c 'git ls-files data/binance-2122/; find data/binance-2122 -name "*.parquet" | wc -l; grep -m1 "^sha256" data/binance/REVISION.toml'
  ```

## Live demo

Read-only filesystem + git probes run by the presenter (no fetch, no build).
Full capture: `spec/binance-corpus-expansion/presentations/artifacts/binance-corpus-expansion-2026-06-15/ground-truth.txt`.

```
$ git ls-files data/binance-2122/
data/binance-2122/REVISION.toml

$ git ls-files --others --exclude-standard data/binance-2122/   # untracked, gitignore-respecting
(empty — no parquet leaked into git)

$ find data/binance-2122 -name '*.parquet' | wc -l
240

$ du -sh data/binance-2122
5.6M

$ head -9 data/binance-2122/REVISION.toml   # new pin + metadata
[revision]
sha256 = "4f3906222cbca90c4188443f9a09440c2b7cb72a3a1fa40b7f7598b3fad22a62"

[revision.metadata]
generated_at = "2026-06-15T05:52:22Z"
binance_base = "https://api.binance.com"
fetch_tool = "fetch_binance_klines"
fetch_version = "0.1.0"
interval = "1h"

$ grep -m1 '^sha256' data/binance/REVISION.toml   # legacy pin, must stay byte-identical
sha256 = "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7"
```

The two pins are distinct (`4f390622…` new vs `3a8b96c4…` legacy) — the new
dataset is fully isolated; nothing about the existing corpus moved.

## Verification matrix

Source: `feature.md § Acceptance criteria` (AC1–AC7), graded by the tester PASS
report (`reports/test-2026-06-15-binance-corpus-expansion.md`).

| ID | Criterion | Result | Evidence |
|----|-----------|--------|----------|
| V1 / AC1 | 240 parquet + pin; recomputed SHA == claimed | VERIFIED | `manifest_internal_consistency` green; aggregate `4f390622…`; 240 files on disk (live probe above) |
| V2 / AC2 | Legacy `data/binance` pin byte-identical; 119/119 anchors | VERIFIED | `data/binance` first line still `sha256 = "3a8b96c4…"` (live probe); `verify_anchors.sh → ANCHORS PASS (119 / 119)` |
| V3 / AC3 | Only `REVISION.toml` tracked; no parquet in git | VERIFIED | `git ls-files` → 1 file; `git ls-files --others --exclude-standard` → empty (live probe) |
| V4 / AC4 | Re-fetch determinism (identical aggregate SHA) | VERIFIED | Tester re-ran full fetch; `[revision].sha256` unchanged; only advisory `generated_at` moved (excluded from hash per ADR-0032 § D2) |
| V5 / AC5 | SKIP-safe on machines without the corpus | VERIFIED | `manifest_internal_consistency` needs no parquet; smoke consumer prints `SKIP` when sentinel absent (tester code inspection) |
| V6 / AC6 | Prices `Decimal`, never f64 | VERIFIED | `smoke_consumer_btcusdt_2022` read 17 507 BTCUSDT bars via `Utf8 → rust_decimal::Decimal` path |
| V7 / AC7 | spec-lint zero new violations | VERIFIED | `spec_lint.py` → 70 (65 dead-link + 5 trace-broken-path), all pre-existing; 0 new |

## Numbers that matter

| Metric | Value |
|--------|-------|
| Parquet files added | 240 (10 symbols × 24 months) |
| Disk footprint (gitignored) | ~5.6 MB |
| Committed to git | 1 file (`REVISION.toml`, ~30 KB) |
| New dataset pin (aggregate SHA-256) | `4f3906222cbca90c4188443f9a09440c2b7cb72a3a1fa40b7f7598b3fad22a62` |
| Legacy `data/binance` pin (unchanged) | `3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7` |
| Coverage start | 2021-01 (744 bars), all 10 symbols, no ragged months |
| Smoke-read bar count (BTCUSDT) | 17 507 |
| Tests | 2 passed / 0 failed (1 always-on, 1 `#[ignore]` smoke) |
| Regression anchors | 119 / 119 PASS |
| Spec-lint | 70 violations, 0 new (baseline held) |
| Clippy (`-p data --tests`) | 0 warnings |

## Open decisions

1. **Approve `data/binance-2122/` as ready / shipped, or route back?** This is the
   only decision. The dataset is fetched, pinned, verified byte-safe, and produces
   no alpha verdict by itself.

_What a "yes" commits you to:_ nothing now. No anchors to re-lock, no manual
capture. The downstream survey re-run (below) is a **separate future feature**, not
triggered by this approval.

_Downstream cross-link (informational, NOT done here):_ this corpus unblocks a
future down-market survey re-run. The `simple-strategy-overfit-guard` lane just
showed the 2024 down-market trend-following hedge is **path-fragile**, so that
future re-run should test **path-robustness** (e.g. block-bootstrap), not just
point returns — running the same point-return survey on 2021–22 would inherit the
same fragility blind spot. Flagging so the downstream feature is scoped right when
it's picked up.

## Approval block

- [x] Approved — ship — operator, 2026-06-15
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

_Notes / reason:_

## Feedback log

_n/a — no operator feedback yet._
