---
date: 2026-06-16
slug: fetcher-idempotency-fix
status: done
author: developer
---

# Fetcher idempotency fix — legitimately-gapped months no longer re-fetched

## Root cause (one line)

`should_skip` compared on-disk row count against the calendar-derived expected
count with strict equality; real Binance exchange gaps produce legitimately-short
months (e.g. Feb 2021 hourly = 671 bars, calendar-expected = 672) that never
satisfy the check, causing ~50/240 months to re-download byte-identical data on
every corpus refresh.

## Chosen signal: REVISION.toml per-file content SHA

The REVISION.toml manifest (emitted by `--emit-revision-manifest`, pinned under
ADR-0032) already records a content SHA-256 for every parquet file in the
`[files]` BTreeMap. This is the most durable signal available: if the on-disk
file's bytes are identical to the previously-pinned fetch, the file is complete
regardless of its row count.

Why not a last-bar-timestamp heuristic? The problem statement explicitly rejected
it: end-of-month gaps produce no last bar in the final hour, so a timestamp check
would incorrectly trigger re-fetches for exactly the months we need to rescue.
The content-SHA approach handles gaps anywhere in the month.

## Implementation

- `should_skip` gains a third parameter `pinned_sha: Option<&str>`.
- Decision tree (see doc comment in the function):
  1. File absent → fetch.
  2. Interval bars unverifiable (e.g. `1d`) → skip conservatively.
  3. `rows == calendar_expected` → skip (fast path, full month).
  4. `rows < expected` AND `file_sha256(path) == pinned_sha` → skip (rescue path: legitimately-gapped month, byte-identical to prior fetch).
  5. Otherwise → re-fetch.
- `main()` loads the existing REVISION.toml once via `data::revision::read_manifest_raw`
  before entering the symbol loop. On first run (no manifest yet), the returned map
  is empty and step 4 is never reached — pre-fix behaviour preserved.
- SHA hashing is only triggered when the row count doesn't match (the ~50 short months),
  so overhead on the 190 full months is zero.
- The manifest `[files]` map is **not modified** by the skip logic; the aggregate
  SHA semantics (ADR-0032 §D2 content-only) are fully preserved.

## 0-re-fetch proof

Idempotent re-run of the full 240-month corpus:

```
cargo run -p data --bin fetch_binance_klines -- \
  --symbols BTCUSDT,ETHUSDT,BNBUSDT,SOLUSDT,XRPUSDT,ADAUSDT,DOGEUSDT,AVAXUSDT,DOTUSDT,LINKUSDT \
  --start 2021-01-01 --end 2022-12-31 --interval 1h \
  --out data/binance-2122 --emit-revision-manifest
```

Result: zero `[OK]` lines (zero live downloads); only:
```
[REVISION] data/binance-2122/REVISION.toml written — aggregate SHA: 4f3906222cbca90c4188443f9a09440c2b7cb72a3a1fa40b7f7598b3fad22a62
```

The rescue path logged (RUST_LOG=fetch_binance_klines=info, single-symbol probe):
```
INFO fetch_binance_klines: loaded existing REVISION.toml for idempotency check out=data/binance-2122 files=240
INFO fetch_binance_klines: row count short but content SHA matches pinned manifest — \
     legitimately gapped month, skipping path=data/binance-2122/ADAUSDT/2021/02.parquet rows=671 expected=672
```

## Pins unchanged

- `data/binance-2122/REVISION.toml` sha256: `4f3906222cbca90c4188443f9a09440c2b7cb72a3a1fa40b7f7598b3fad22a62` (unchanged)
- `data/binance/REVISION.toml` sha256: `3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7` (untouched — fetcher fix never writes to `data/binance/`)
- `cargo test -p data --test binance_2122_revision_consistency manifest_internal_consistency` → PASS

## Unit tests added

Five new tests in `crates/data/src/bin/fetch_binance_klines.rs` `mod tests`:

| Test | Scenario | Expected |
|---|---|---|
| `test_should_skip_short_month_with_matching_pinned_sha_returns_true` | 671 rows, SHA matches manifest | `true` (skip) |
| `test_should_skip_short_month_with_mismatched_pinned_sha_returns_false` | 671 rows, wrong SHA | `false` (re-fetch) |
| `test_should_skip_short_month_no_manifest_returns_false` | 671 rows, no manifest | `false` (re-fetch) |
| `test_should_skip_full_month_skipped_regardless_of_manifest` | 744 rows (full), any/no SHA | `true` (fast path) |
| `test_should_skip_absent_file_returns_false` | file does not exist | `false` |

All 17 tests pass (`cargo test -p data --bin fetch_binance_klines`).
