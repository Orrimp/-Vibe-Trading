---
slug: carry-funding-data-backfill
status: draft
owner: developer
updated: 2026-05-31
---

# Tasks — carry-funding-data-backfill

Data-acquisition spike — **complete** (shipped at `ab815d5`).

- [x] Build `fetch_binance_funding` historical funding-rate fetcher (Binance public `/fapi/v1/fundingRate`).
- [x] Backfill the 10-symbol USDⓈ-M universe for 2023-01 .. 2024-12 → `data/binance-funding/` parquet (240 files; bulk gitignored).
- [x] Pin `data/binance-funding/REVISION.toml` (aggregate SHA `bf1ede44…`, reproducible).
- [x] Gates: `cargo test -p data` 106 pass; fetcher 14 tests; clippy clean.

No further tasks. The carry **strategy** that consumes this data is tracked
separately under [`spec/carry-strategy/`](../carry-strategy/feature.md).
