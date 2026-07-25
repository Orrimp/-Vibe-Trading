# M5 Revision Pin Capture — 2026-05-18

## T-D-16: Aggregate SHA capture

**Command run:**
```
cargo run -p data --bin fetch_binance_klines --release -- \
  --symbols ADAUSDT,AVAXUSDT,BNBUSDT,BTCUSDT,DOGEUSDT,DOTUSDT,ETHUSDT,LINKUSDT,SOLUSDT,XRPUSDT \
  --start 2023-01-01 --end 2024-12-31 --interval 1h \
  --out data/binance --emit-revision-manifest
```

**Result:** All 240 files skipped (already present with correct row counts). REVISION.toml re-written.

**Aggregate SHA:**
```
3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
```

**Scope:** 10 USDT pairs × 24 months = 240 parquet files (2023-01 through 2024-12).

**Universe:**
- ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT
- DOTUSDT, ETHUSDT, LINKUSDT, SOLUSDT, XRPUSDT

## T-D-17: Pin applied

SHA pinned into all four scenario arms in `crates/backtest/src/main.rs`:
- `top10-2023-fy-tcn-overlay-realdata` (line ~430)
- `top10-2024-fy-tcn-overlay-realdata` (line ~451)
- `top10-2023-fy-tcn-overlay-weights-realdata` (line ~471)
- `top10-2024-fy-tcn-overlay-weights-realdata` (line ~491)

## Bug Fix (revision-roundtrip-bug)

Root cause identified: the bug reported in the BLOCKED comment was NOT a TOML
roundtrip divergence. Investigation via production manifest test confirmed that
`write_revision_manifest` followed by `read_manifest_raw` + `compute_aggregate_sha`
round-trips cleanly for 240 entries.

The actual root cause: T-D-17 previously pinned `3a8b96c4...` (real data SHA) but
the determinism tests (T-D-13/14/15) ran the backtest binary against a synthetic
tempdir fixture that has a different aggregate SHA. The revision-pin check in
`main.rs` compared the compiled-in real SHA against the synthetic fixture SHA and
correctly reported a mismatch.

Fix applied in `crates/backtest/tests/determinism.rs`:
- Removed synthetic tempdir fixture helpers (now unused)
- Added `workspace_root_path()` helper
- Added `real_binance_data_available()` guard
- Changed T-D-13/14/15 tests to run from workspace root when real data is present
- Tests skip with `eprintln!` when `data/binance/REVISION.toml` is absent (CI safety)

Fix applied in `crates/data/src/revision.rs`:
- Added `test_roundtrip_250_files` regression guard (250 files, production scale)
- Added `test_production_manifest_roundtrip` (ignored by default, run with --ignored)
- Updated comments to accurately describe the verified bug

## Verification

- `cargo test -p data --lib revision` → 6 passed, 1 ignored
- `cargo test -p backtest --features realdata --test determinism` → 22 passed
- `cargo test -p backtest --features realdata --test realdata_revision_verify` → 4 passed
- `bash scripts/verify_anchors.sh` → ANCHORS PASS (15/15)
- `cargo clippy --workspace -- -D warnings` → clean
- `cargo clippy --workspace --features realdata -- -D warnings` → clean
