---
slug: advisor-corpus-expansion
status: in-progress
owner: developer
updated: 2026-07-09
---

# Tasks — P2 corpus expansion + verdict re-run

Binding design: [ADR-0084](../../architecture/adr/0084-p2-corpus-set-coinbase-adapter-verdict-rerun.md);
feature [§ Design](feature.md) + [§ Backtest Scenarios](feature.md). **Run
`bash scripts/verify_anchors.sh` (→119/119) + `python3 scripts/spec_lint.py`
(→PASS) before AND after every commit.** `write_report=false` on every re-run
path — anchors 119/119 BY CONSTRUCTION. FROZEN gate byte-untouched. Existing
pinned corpora SHAs byte-immutable. No live trading. `ci.yml.deferred` parked.

> **Long-running-task memory:** the multi-hour multi-corpus fetch + re-run MUST
> ship with a copy-pasteable `watch -n N '<probe>'` block (T5, T9). The fetch is
> the dominant wall-clock cost — do NOT block on it.

## Architect (M-T1) — design pass ✅ DONE

- [x] AT1 — resolved Q-CE-1..7; **Q-CE-2 Coinbase-hourly RATIFIED** (Kraken hourly
  infeasible), **Q-CE-3 back-fill DVOL+macro** — each answered in feature.md §
  Design + ADR-0084 D1-D8.
- [x] AT2 — confirmed the R1 corpus set unchanged (1718/2020/2526) + the
  last-closed-month clamp (D5); final corpus table in § Design.
- [x] AT3 — picked a **dedicated `p2_verdict_rerun` harness** (D4, grounded: the
  runner hardcodes `data/binance`); **ADR-0084 authored + registered atomically**.
- [x] AT4 — developer/tester task list appended below (T1-T10 + tester handoff).

## Phase A — the Coinbase second-venue fetcher (ADR-0084 D2.a)

- [ ] **T1 — `coinbase_klines.rs` library module** — new `crates/data/src/coinbase_klines.rs`
  mirroring `binance_klines.rs`. Reuse the shared `binance_klines::Kline` struct +
  `write_parquet` + `expected_bars_per_month` + `should_skip` idempotency verbatim
  (venue-neutral). Add: `build_coinbase_candles_url(product_id, granularity_secs, start_iso, end_iso)`;
  a `CoinbaseKlineFetcher` trait + `HttpCoinbaseKlineFetcher` + a mock (mirror the
  Binance testability seam); `parse_coinbase_candle(&[serde_json::Value]) -> Kline`
  doing the FOUR shim mappings (positional `[time,low,high,open,close,vol]` →
  canonical `Kline`; `time` seconds ×1000 → millis; `close_time = open_time +
  granularity_ms − 1`; `trade_count = 0`); `paginate_coinbase_candles` (300-candle
  window step, ≥200 ms pace, forward sub-windows within a month). _acceptance: unit
  tests — URL builder, positional+seconds→millis parse round-trip, paginator
  300-window boundary + empty-stop, no socket in tests (all via the mock)._
- [ ] **T2 — `fetch_coinbase_klines` bin** — new `crates/data/src/bin/fetch_coinbase_klines.rs`
  (CLI glue mirroring `fetch_binance_klines.rs`): `--symbols` (canonical `BTCUSDT`,
  mapped to `BTC-USD` for the REST call via `data::coinbase_symbol_map`), `--start`,
  `--end`, `--interval` (hourly → granularity 3600), `--out data/coinbase`,
  `--force`, `--emit-revision-manifest`. On-disk layout `<out>/BTCUSDT/<YEAR>/<MM>.parquet`.
  Reuse `data::revision::write_revision_manifest` + the pinned-manifest `should_skip`
  idempotency path verbatim. _acceptance: `cargo run -p data --bin fetch_coinbase_klines
  -- --help` works; a bounded dry-run (one month BTC-USD hourly) writes a valid
  parquet `ReplayFeed` can read + records the earliest served candle (A2)._
- [ ] **T3 — export + build/validate** — add `pub mod coinbase_klines;` to
  `crates/data/src/lib.rs` (re-export the public surface the harness needs). Run
  `rust-build` + `rust-validate` (clippy `-D warnings`, fmt). _acceptance:
  workspace builds; clippy clean; `cargo test -p data` green (T1's unit tests)._

## Phase B — fetch the corpora (ADR-0084 D1 + D2.b + D3 + D5) — the long job

> Emit the `watch` probe block (T5) BEFORE kicking these off. Multi-hour steps.

- [ ] **T4 — fetch the 3 new Binance corpora + the Coinbase cross-check corpus.**
  Each with `--emit-revision-manifest`; record the exact fetch command +
  per-symbol bar totals + aggregate SHA + the "must stay" existing SHAs
  (`3a8b96c4…`, `4f390622…`) in a NON-anchored `reports/fetch-<date>-<corpus>.md`
  (mirror `data/binance-2122/` convention):
  - `data/binance-1718` — `--symbols BTCUSDT,ETHUSDT,BNBUSDT --start 2017-08-01 --end 2018-12-31 --interval 1h`.
  - `data/binance-2020` — `--symbols BTCUSDT,ETHUSDT,BNBUSDT,XRPUSDT,ADAUSDT,LINKUSDT,DOGEUSDT --start 2020-01-01 --end 2020-12-31 --interval 1h`.
  - `data/binance-2526` — `--symbols <all 10> --start 2025-01-01 --end <LAST FULLY-CLOSED UTC MONTH> --interval 1h` (D5 clamp — compute at fetch time; record the end month).
  - `data/coinbase` — `fetch_coinbase_klines --symbols BTCUSDT --start 2020-01-01 --end <LAST FULLY-CLOSED UTC MONTH> --interval 1h --out data/coinbase` (D5 clamp; BTC only).
  _acceptance: 4 `REVISION.toml` written; each corpus's earliest/last month matches
  the intended window (empty pre-listing months warn+skip, not crash); the Coinbase
  earliest served candle recorded (A2); existing pinned corpora untouched
  (`verify_anchors.sh` 119/119)._
- [ ] **T5 — the `watch` probe block** (long-running-task memory). Emit a
  copy-pasteable block, e.g.
  `watch -n 30 'find data/binance-1718 data/binance-2020 data/binance-2526 data/coinbase -name "*.parquet" | wc -l; ls -la data/*/REVISION.toml 2>/dev/null'`.
  _acceptance: the operator can watch fetch progress without blocking._
- [ ] **T6 — exogenous back-fill (D3), additive, existing pinned SHAs
  byte-identical.** DVOL: `fetch_deribit_dvol --currencies BTC,ETH --start 2021-01-01
  --end 2022-12-31` then `--start 2025-01-01 --end <closed-month>` (per-year parquet
  into the EXISTING `data/deribit-dvol/`). Macro: `fetch_yahoo_klines` for
  DXY/GSPC/TNX `--start 2025-01-01 --end <closed-month>` (per-month into the
  EXISTING `data/yahoo-macro/`). Re-emit each corpus's `REVISION.toml` (additive —
  new years/months are new file rows; existing rows byte-identical). _acceptance:
  the pre-existing DVOL/macro file SHAs are unchanged in the re-emitted manifest;
  new years/months present; `verify_anchors.sh` 119/119._

## Phase C — new-corpus consistency + smoke tests (AC7)

- [ ] **T7 — per-new-corpus REVISION internal-consistency test** — mirror
  `crates/data/tests/binance_2122_revision_consistency.rs` for each new corpus
  (`binance-1718`, `binance-2020`, `binance-2526`, `coinbase`): re-derive the
  aggregate SHA from the `[files]` map, assert it equals the claimed
  `[revision].sha256`, assert the expected file count (symbols × months for the
  window). Runs on the committed manifest alone (no parquet on disk). _acceptance:
  `cargo test -p data --test <corpus>_revision_consistency manifest_internal_consistency`
  green for all 4; CI-safe (TOML-only)._
- [ ] **T8 — SKIP-safe smoke consumer per new corpus** — mirror the 2122 T7
  `#[ignore]` smoke: `ReplayFeed` reads the corpus for one symbol/year, prices
  parse to non-zero `Decimal`, SKIP-guards when the gitignored parquets are absent.
  _acceptance: SKIP message + early return when absent; ≥100 bars when present._

## Phase D — the verdict re-run harness (ADR-0084 D4)

- [ ] **T9 — `p2_verdict_rerun.rs` harness** — new `crates/backtest/tests/p2_verdict_rerun.rs`.
  Compose the two proven pieces: (a) load bars per corpus via
  `ReplayFeed::new(<corpus_root>, true).merge_symbols(&[(sym, root)], Timeframe::OneHour)`
  (per `realdata_simple_strategy_bear_survey.rs:168`); (b) run
  `null_data_no_crown.rs::run_field_and_rank`'s exact per-arm sequence
  (`run_scenario` bars_override → `derive_candidate_kpis` → `derive_master_seed` +
  `compute_robustness_flag` → `rank_candidates` → `compute_scorecard`),
  `write_report=false`. Per the R4 matrix, build each corpus's field from the
  SUPPORTED arms only; thread `dvol_override` / `macro_regime_series` via the SAME
  public `resolve_dvol_override` / `load_macro_regime_series` fns for corpora that
  support those arms; ABSENT arms are NOT added to the field (report them "not
  evaluable (no <data>)", never a silent drop). SKIP-safe per corpus. Fixed seed
  base per corpus (recorded). Include the S7/S8 opt-in `VolScaledSpread` annex runs.
  Emit the `watch` probe if the re-run is >2 min. _acceptance: harness runs each
  present corpus + produces `FieldOutcome` per (corpus, arm); `verify_anchors.sh`
  119/119 during AND after; no anchored report body written; SKIP-safe when corpora
  absent._
- [ ] **T10 — full gate sweep** — `bash scripts/verify_anchors.sh` (→119/119) +
  `python3 scripts/spec_lint.py` (→PASS) + `python3 scripts/adr_registry_check.py`
  (→exit 0) + `cargo test --workspace` (the new tests green, nothing else broken).
  Confirm the FROZEN gate files (`bakeoff/robustness.rs`, `bakeoff/rank.rs`,
  `classify_verdict`) are byte-untouched (`git diff --stat`). _acceptance: all
  gates green; frozen-gate files show no diff._

## Handoff to tester

The tester authors the NON-anchored `reports/backtest-<date>-p2-verdict-rerun.md`
(AC1-AC8) from T9's `FieldOutcome` data + the T7/T8 consistency/smoke results, and
links it in the feature `## Verification` section. The report is the AC1-AC8
deliverable; it is NEW and NOT anchored initially.

## Notes

- The Coinbase candle endpoint caps at **300 candles/call** (>300 → rejected) and
  the `time` field is in **seconds**; both handled in T1's shim. Coinbase historical
  data may have gaps ("no data is published for intervals where there are no
  ticks") — the `should_skip` content-SHA idempotency path handles genuinely short
  months exactly as it does for Binance.
- The R4 arm×corpus availability matrix (feature § R4) is the binding arm-selection
  contract for T9. Perp-basis/funding MN arms are ABSENT on every corpus except the
  2324 base (basis is 2023-24 only) — do NOT back-fill them (out of scope).
- If a Coinbase fetch reveals BTC-USD does NOT reach as deep as 2020 on the overlap
  window, narrow `data/coinbase` to the deepest reachable window that still overlaps
  ≥2 Binance corpora and record it in the fetch report (A2 — confirm the earliest
  served candle at fetch time).
- Anchors 119/119 + spec-lint PASS(0) gated per commit; `write_report=false`
  throughout (anchor-safe by construction); FROZEN gate byte-frozen; existing
  pinned corpora SHAs byte-immutable.
