---
version: 0.1.0
status: dev-done
slug: carry-funding-data-backfill
created: 2026-05-31
updated: 2026-05-31
owner: developer
---

# Carry Funding Data Backfill

## Summary

Historical Binance USDⓈ-M perpetual funding rates, backfilled for the 10-symbol
universe over 2023-01-01 .. 2024-12-31, stored as Parquet files under
`data/binance-funding/`. This unblocks the carry strategy (a later feature) from
being data-gated once the MR lane verdict is in.

## Why

Carry is the most-independent return source vs momentum, but was DATA-GATED:
a live forward-only `FundingPoller` + `funding_rates` SQLite table existed, but
the backtest harness reads Parquet from `data/binance/`, not the SQLite ledger.
A historical funding backfill in Parquet eliminates this gate.

## What Was Built

### Fetcher binary

`crates/data/src/bin/fetch_binance_funding.rs`

- Registered as `[[bin]] name = "fetch_binance_funding"` in `crates/data/Cargo.toml`
- Mirrors `fetch_binance_klines.rs` conventions exactly (same CLI interface,
  same month-by-month pagination loop, same idempotency skip, same
  `--emit-revision-manifest` flag using the existing `data::revision` module)
- Endpoint: `GET https://fapi.binance.com/fapi/v1/fundingRate?symbol=<SYM>&startTime=<ms>&limit=1000`
  (public, no auth required)
- Pagination: forward by `fundingTime`; cursor advances to `last_funding_time + 1`
- Rate-limit: 200ms sleep between pages (same as OHLCV fetcher)

### Output layout

```
data/binance-funding/
  <SYMBOL>/
    <YEAR>/
      <MM>.parquet    ← one file per symbol-month
  REVISION.toml       ← aggregate SHA-256 manifest (ADR-0040 / ADR-0032)
```

Mirrors `data/binance/<SYMBOL>/<YEAR>/<MM>.parquet` so a future loader can
extend `crates/backtest/src/realdata.rs` with a `funding_root` parameter.

### Schema

| column        | dtype  | notes                                          |
|---------------|--------|------------------------------------------------|
| symbol        | Utf8   | e.g. `"BTCUSDT"` (USDⓈ-M perp symbol)         |
| funding_time  | Int64  | Unix milliseconds of the 8-hour settlement     |
| funding_rate  | Utf8   | Rate string, precision-preserved (e.g. `"0.00010000"`) |

`funding_rate` is stored as a string (same as OHLCV price columns) to
avoid floating-point rounding on an already-decimal value. A future loader
should parse it via `rust_decimal::Decimal`.

### Universe

The same 10 USDT-pair perpetuals as the OHLCV data (matching by directory
layout in `data/binance/`):

```
ADAUSDT  AVAXUSDT  BNBUSDT  BTCUSDT  DOGEUSDT
DOTUSDT  ETHUSDT   LINKUSDT  SOLUSDT  XRPUSDT
```

### Period

2023-01-01 .. 2024-12-31 inclusive (matches OHLCV price data range).

### Cadence

Binance settles funding every 8 hours (00:00, 08:00, 16:00 UTC) → 3 rows/day.
Expected per month: 84–93 rows (28–31 days × 3). Total: ~21 900 rows for
10 symbols × 2 years.

### Revision manifest

`data/binance-funding/REVISION.toml` is committed (gitignore exception added
in `.gitignore` per the `data/yahoo/REVISION.toml` ADR-0040 precedent). The
manifest uses the existing `data::revision::write_revision_manifest()` function —
the same aggregate-SHA algorithm as the OHLCV revision — so verification
logic is shared.

## Data verdict

**Acquired by agent** — network was available in the sandbox (confirmed via
`curl` probe). Backfill was run against the live Binance public API during
this session and the `REVISION.toml` was emitted.

## Tests

14 unit tests embedded in `fetch_binance_funding.rs` under `mod tests`:

- URL builder (2 tests)
- Expected settlements/month including leap-year February (4 tests)
- Paginator: stops on empty, two-page accumulation, out-of-window filtering (3 tests)
- Parquet schema round-trip (1 test)
- Date helpers (4 tests)

All 14 pass: `cargo test -p data --bin fetch_binance_funding`.

## How a future carry strategy would consume this

The carry signal computes the _cross-sectional rank_ of the 8-hour funding rate
across the universe, then takes long/short positions accordingly (negative
funding = long bias, positive = short bias, normalised by annualised carry).

The consumption path is:

1. **Extend `crates/backtest/src/realdata.rs`** with a `funding_root: Option<PathBuf>`
   field on `RealDataConfig`. When set, load `data/binance-funding/<SYM>/<Y>/<MM>.parquet`
   alongside OHLCV parquets and hydrate a `FundingObs`-compatible struct per bar.

2. **New crate `crates/strategy/src/carry.rs`** (to be designed at carry-strategy
   feature time) — reads the hydrated funding observations, ranks symbols, and
   emits `OrderIntent` sized by a configurable notional.

3. **Backtest scenario** — a new `scenarios/carry-momentum-blended.toml` (or
   standalone `scenarios/carry-only.toml`) listing the 10-symbol universe with
   `funding_root = "data/binance-funding"`.

4. **Alignment note** — funding `funding_time` timestamps align with the OHLCV
   `open_time` of the immediately following 8h bar (settlement at bar boundary).
   The carry strategy must decide whether to trade on the bar that just settled
   (information available at bar open) or the next bar (conservative).

The schema (`symbol`, `funding_time`, `funding_rate`) is intentionally minimal
so the carry-strategy designer can add derived columns (annualised carry,
rolling mean/std) at the strategy layer rather than baking them into the parquet.
