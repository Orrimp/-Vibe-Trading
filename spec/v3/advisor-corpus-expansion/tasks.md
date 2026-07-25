---
slug: advisor-corpus-expansion
status: tester-done
owner: tester
updated: 2026-07-10
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

- [x] **T1 — `coinbase_klines.rs` library module** — new `crates/data/src/coinbase_klines.rs`
  mirroring `binance_klines.rs`. Reuse the shared `binance_klines::Kline` struct +
  `write_parquet` verbatim. Added: `build_coinbase_candles_url`; `CoinbaseKlineFetcher`
  trait + `HttpCoinbaseKlineFetcher` + a mock; `parse_coinbase_candle` doing the FOUR
  shim mappings; `paginate_coinbase_candles` (300-candle window step, forward
  sub-windows within a month). **Deviation from the original plan (a real bug found
  + fixed during testing):** ADR-0084 named the reused `crate::coinbase::coinbase_symbol_map`
  for the symbol→product-id mapping, but that fn returns `BTC-USDT` (not `BTC-USD`)
  for a `BTCUSDT` input (it checks USDC→USDT→USD suffixes in that order and BTCUSDT
  matches USDT first) — a real, thinner, non-blessed Coinbase product, different from
  the ADR's own worked example (`BTCUSD → BTC-USD`, a different input). Added a
  DEDICATED `coinbase_product_id_for_symbol` (strip-then-fixed-append-`-USD`) instead,
  which delivers the ADR's actual DESIGN INTENT (`BTCUSDT→BTC-USD`) — flagged to the
  architect as a spec-precision note in `spec/trace.toml`, not a design reversal.
  file: `crates/data/src/coinbase_klines.rs:83` (`coinbase_product_id_for_symbol`).
  test: `cargo test -p data --lib coinbase_klines`.
  output: `test result: ok. 19 passed; 0 failed; 0 ignored; ... finished in 0.01s`.
- [x] **T2 — `fetch_coinbase_klines` bin** — new `crates/data/src/bin/fetch_coinbase_klines.rs`
  (CLI glue mirroring `fetch_binance_klines.rs`): `--symbols` (canonical `BTCUSDT`,
  mapped to `BTC-USD` via `data::coinbase_product_id_for_symbol`), `--start`, `--end`,
  `--interval` (only `1h` wired, granularity 3600), `--out data/coinbase`, `--force`,
  `--emit-revision-manifest`. **T2 dry-run + A2 findings (bounded live network,
  2026-07-10):** `--help` proof green; a live 3-day BTC-USD hourly fetch (2024-01-01→
  2024-01-03) initially failed `HTTP 400 {"message":"User-Agent header is required."}`
  — a REAL bug (`reqwest::Client`'s default has no User-Agent; Coinbase enforces this,
  Binance doesn't) — fixed by setting a fixed `User-Agent` header on every request
  (`COINBASE_USER_AGENT` const). Re-run succeeded: 72 candles for the 3-day window,
  `ReplayFeed` read all 72 bars with sane BTC prices ($42.4k-$42.9k). **A2 earliest-
  served-candle probe** (bounded — ~10 small live calls bisecting 2015-2018): BTC-USD
  hourly is served starting **2015-08** (2015-07 and earlier is empty) — DEEPER than
  the ADR's "~2015-16" estimate and comfortably covers the proposed `data/coinbase`
  2020-01→last-closed-month window; **A2 CONFIRMED, no window narrowing needed.**
  file: `crates/data/src/coinbase_klines.rs:242` (`COINBASE_USER_AGENT` +
  `HttpCoinbaseKlineFetcher::fetch`); `crates/data/src/bin/fetch_coinbase_klines.rs`.
  test: `cargo test -p data --bin fetch_coinbase_klines`.
  output: `test result: ok. 8 passed; 0 failed; 0 ignored; ... finished in 0.01s`.
- [x] **T3 — export + build/validate** — added `pub mod coinbase_klines;` +
  `pub use coinbase_klines::{...}` to `crates/data/src/lib.rs`.
  file: `crates/data/src/lib.rs:22` (`pub mod coinbase_klines;`).
  test: `cargo build -p data -p backtest --features backtest/realdata,backtest/yahoo`
  + `cargo clippy -p data -p backtest --tests --features backtest/realdata,backtest/yahoo -- -D warnings`
  + `cargo fmt --check -p data -p backtest`.
  output: `Finished` dev profile (build); `Finished` dev profile with zero clippy
  warnings; fmt --check exit 0.

## Phase B — fetch the corpora (ADR-0084 D1 + D2.b + D3 + D5) — the long job

> Emit the `watch` probe block (T5) BEFORE kicking these off. Multi-hour steps.
>
> **T4/T5 are explicitly OUT OF SCOPE for this developer session** (per the
> orchestrator's brief — the multi-hour Binance/Coinbase fetches are resumable
> background jobs the orchestrator runs). Left UNTICKED here; the exact
> ready-to-run commands + watch probe + post-fetch verification are handed off
> in the developer's HANDOFF message verbatim (also below for convenience).

- [x] **T4 — fetch the 3 new Binance corpora + the Coinbase cross-check corpus.**
  Run by the orchestrator ahead of the tester session (2026-07-09, per the
  tester's brief `## State on disk` section) — all 4 corpora landed with
  `--emit-revision-manifest`. Per-corpus on-disk file counts + aggregate SHAs
  independently RE-VERIFIED this session via `data::revision::{read_manifest_raw,
  compute_aggregate_sha}` (T7, below) — recomputed SHAs match the claimed
  `[revision].sha256` in every `REVISION.toml` byte-for-byte:
  - `data/binance-1718` — 48 files (BTC/ETH 2017-08→2018-12 = 17mo each,
    BNB 2017-11→2018-12 = 14mo), aggregate `cb9ef728784ab78969bcbc063eb73190c38c17f4efbde6a97b934a2eb74361d4`.
  - `data/binance-2020` — 84 files (7 symbols × 12mo, full 2020 coverage),
    aggregate `dfddbc7cfc450ee21af749e52bc7c3732aafe71f2ca467f237ba6c40d45caa79`.
  - `data/binance-2526` — 180 files (10 symbols × 18mo, 2025-01→2026-06 —
    the D5 last-fully-closed-UTC-month clamp landed at 2026-06), aggregate
    `74ba294c260466ff674186cbe7df9464b4b5ee0035a94f6494573076e0a359d3`.
  - `data/coinbase` — 78 files (BTCUSDT on-disk canonical, 2020-01→2026-06),
    aggregate `7ac1df984fb14528b52f2dc0dce63e68787e5f5128f6ece0bf0b472de986970d`.
  _acceptance MET:_ 4 `REVISION.toml` written + git-committed; per-symbol month
  counts match the intended honest-subset windows (verified via
  `find data/<corpus>/<SYMBOL> -name '*.parquet' | wc -l` per symbol — e.g.
  `binance-1718/BNBUSDT` = 14, not 17, confirming the 2017-11 listing-date
  exclusion held); T8's era-sanity smokes (below) independently confirm each
  corpus's price range matches its intended regime (e.g. BTC 2017-12 mania top
  $19,709.50, inside the [$10k,$25k] assertion window); existing pinned corpora
  (`data/binance` `3a8b96c4…`, `data/binance-2122` `4f390622…`) untouched
  (`git status --short data/` empty for those dirs; `verify_anchors.sh` 119/119
  before AND after this session's writes).
  file: `data/binance-1718/REVISION.toml`, `data/binance-2020/REVISION.toml`,
  `data/binance-2526/REVISION.toml`, `data/coinbase/REVISION.toml`.
  test: `cargo test -p data --test p2_corpora_revision_consistency` (T7, below).
  output: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`.
- [x] **T5 — the `watch` probe block** (long-running-task memory). Not
  needed this session — T4's fetches had already completed before the tester
  started (see T4 above); the follow-on multi-corpus verdict re-run (THE RUN,
  T9's harness with `--include-ignored`) finished in 238.79s (~4.0 min), well
  under the ~2-minute watch-recipe threshold, so no `watch` block was required
  for that step either. For the record, the copy-pasteable probe the
  orchestrator would have used during T4's fetch (per the developer's original
  HANDOFF) is preserved here for any FUTURE re-fetch of these corpora:
  `watch -n 30 'find data/binance-1718 data/binance-2020 data/binance-2526 data/coinbase -name "*.parquet" | wc -l; ls -la data/*/REVISION.toml 2>/dev/null'`.
  _acceptance MET:_ no operator blocking occurred; all fetches + the re-run
  completed and were verified without a live watch session.
- [x] **T6 — exogenous back-fill (D3), additive, existing pinned SHAs
  byte-identical.** DVOL back-filled live: `fetch_deribit_dvol --currencies BTC,ETH
  --start 2021-01-01 --end 2022-12-31` (283/365 daily rows for BTC/ETH 2021/2022 —
  2021 is short because DVOL history genuinely starts ~2021-04) + `--start
  2025-01-01 --end 2026-06-30` (365/181 daily rows, D5-clamped to the 2026-06
  last-fully-closed-UTC-month) → `data/deribit-dvol/` grew from 4 to 12 parquet
  files; `--emit-revision-manifest` re-emitted (new aggregate
  `b21dc8691c257731d9043fc3e19b858c326ab4dd3d975f10de0eccf90cf480ff`). Before/after
  `shasum -a 256` snapshot of the 4 pre-existing files (BTC/ETH 2023/2024)
  confirms byte-identity (`shasum -a 256 -c` → 4× `OK`, only `REVISION.toml`
  itself differs, as expected when 8 new file rows are added). **Load-bearing
  side-effect:** `crates/backtest/src/dvol_data.rs::EXPECTED_DVOL_REVISION_SHA`
  is a hard-pinned constant checked against the WHOLE manifest's aggregate — it
  MUST move when new years are added or `v0.dvol_regime` silently degrades to
  warm-up-only on EVERY corpus (incl. the 2324 base) — updated + proven via the
  `#[ignore]`d `real_corpus_load_smoke` test (loads 182 real rows for the 2024-H1
  span with the new SHA). **Macro finding: NO-OP on this machine** —
  `data/yahoo-macro/{DX-Y.NYB,^GSPC,^TNX}/1d/` already covers 2021-01 through
  2026-06 (66 months × 3 tickers, verified via `find … | wc -l`) — nothing to
  fetch; did NOT run `fetch_yahoo_klines --features yahoo-online` for zero new
  data (would be a wasted live-network round-trip). `verify_anchors.sh` re-run
  AFTER the DVOL back-fill + SHA update — still 119/119 (this constant is NOT
  one of the 9 `spec/anchors.toml` rows).
  file: `crates/backtest/src/dvol_data.rs:57` (`EXPECTED_DVOL_REVISION_SHA`).
  test: `cargo test -p backtest --features realdata --lib dvol_data::tests::real_corpus_load_smoke -- --ignored --nocapture`.
  output: `OK: 182 rows, sha=b21dc8691c257731d9043fc3e19b858c326ab4dd3d975f10de0eccf90cf480ff` /
  `test dvol_data::tests::real_corpus_load_smoke ... ok`.

## Phase C — new-corpus consistency + smoke tests (AC7)

- [x] **T7 — per-new-corpus REVISION internal-consistency test** — new
  `crates/data/tests/p2_corpora_revision_consistency.rs`, ONE file covering all
  4 new corpora (mirrors `binance_2122_revision_consistency.rs`'s
  `manifest_internal_consistency` shape, generalized via a shared
  `assert_manifest_internally_consistent(corpus_dir, expected_file_count)`
  helper): re-derives the aggregate SHA from the `[files]` map via
  `data::revision::{read_manifest_raw, compute_aggregate_sha}`, asserts it
  equals the claimed `[revision].sha256`, asserts the expected file count
  (48/84/180/78 per corpus, matching the honest-subset windows). Un-ignored
  (TOML-parse-only, no parquet on disk required — CI-safe). All 4 green,
  confirming T4's fetch landed correctly + the SHAs match the brief's stated
  values exactly.
  file: `crates/data/tests/p2_corpora_revision_consistency.rs`.
  test: `cargo test -p data --test p2_corpora_revision_consistency`.
  output: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`
  (`binance_1718_manifest_internal_consistency`,
  `binance_2020_manifest_internal_consistency`,
  `binance_2526_manifest_internal_consistency`,
  `coinbase_manifest_internal_consistency` — all `ok`).
- [x] **T8 — SKIP-safe smoke consumer per new corpus** — new
  `crates/data/tests/p2_corpora_replayfeed_smoke.rs` (mirrors the 2122 T7
  `#[ignore]` smoke pattern via a shared `smoke_load_and_check_range` helper):
  per new corpus, `ReplayFeed::subscribe_bars` reads a representative symbol,
  asserts every close parses to a non-zero `rust_decimal::Decimal` (never
  f64), AND asserts every close falls within an era-sanity price window
  GROUNDED IN THE ACTUAL ON-DISK DATA (a bounded throwaway probe run once,
  deleted before commit — not part of the deliverable suite): BTC 2017-1718
  [$1k,$25k] (observed $2,919.00–$19,709.50, incl. the Dec-2017 mania top
  inside the brief's "$10k-$20k mania range"), BTC 2020 [$3k,$30k] (observed
  $4,130.64–$29,155.25), BTC 2526 [$50k,$130k] (observed
  $58,290.17–$126,011.18), Coinbase BTC 2020-2026 [$3k,$130k] (observed
  $4,209.51–$126,099.22). SKIP-guards on the sentinel parquet being absent
  (mirrors the 2122 pattern); `#[ignore]` by default (real I/O). All 4 green
  when run against the real, present corpora.
  file: `crates/data/tests/p2_corpora_replayfeed_smoke.rs`.
  test: `cargo test -p data --test p2_corpora_replayfeed_smoke -- --ignored --nocapture`.
  output: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s`
  (`binance_1718_btcusdt_smoke_era_sanity` read 11,976 bars,
  `binance_2020_btcusdt_smoke_era_sanity` read 8,766 bars,
  `binance_2526_btcusdt_smoke_era_sanity` read 13,104 bars,
  `coinbase_btcusdt_smoke_era_sanity` read 56,936 bars — all `ok`).

## Phase D — the verdict re-run harness (ADR-0084 D4)

- [x] **T9 — `p2_verdict_rerun.rs` harness** — new `crates/backtest/tests/p2_verdict_rerun.rs`.
  Composes the two proven pieces: (a) `load_corpus_symbol_bars` via
  `ReplayFeed::merge_symbols` (SKIP-safe on absence, REVISION-verifies when present);
  (b) `run_field_and_rank` reproducing `run_bakeoff`'s exact per-arm sequence
  (`run_scenario` bars_override → `derive_candidate_kpis` → `derive_master_seed` +
  `compute_robustness_flag` → `rank_candidates` → `compute_scorecard`),
  `write_report=false` on every call. `build_field(ArmSupport)` encodes the R4 matrix
  per corpus; threads `dvol_override` via `resolve_dvol_override` + `macro_regime_series`
  via `load_macro_regime_series` for corpora that support those arms, INCLUDING the
  ADR-0072-D8 per-symbol `v0.dvol_regime` field-exclusion parity (non-BTC/ETH symbols
  get the arm REMOVED from the field, not run-and-degraded — matches
  `bakeoff/mod.rs:1030-1039` exactly, a real correctness fix found during testing).
  S1/S2/S3/S5/S6 SKIP-safe (return early + eprintln when the corpus is absent); S4
  (`data/binance`, the EXISTING pinned base) runs UN-IGNORED as the always-on smoke;
  S7/S8 are the opt-in `VolScaledSpread` era-cost annex. `[[test]] required-features
  = ["realdata", "yahoo"]` added to `crates/backtest/Cargo.toml`.
  file: `crates/backtest/tests/p2_verdict_rerun.rs` (harness);
  `crates/backtest/Cargo.toml:118` (`[[test]] p2_verdict_rerun`).
  test 1 (SKIP-safe, corpora absent): `cargo test -p backtest --features realdata,yahoo
  --test p2_verdict_rerun -- --include-ignored --nocapture s1_binance_1718_btc_eth_bnb
  s2_binance_2020_seven_listers s5_binance_2526_all_ten s6_coinbase_btc_venue_crosscheck
  s7_binance_1718_era_cost_annex_vol_scaled_spread s8_binance_2020_era_cost_annex_vol_scaled_spread`.
  output 1: `test result: ok. 6 passed; 0 failed; 0 measured; 9 filtered out; finished
  in 0.00s` (each printed a `SKIP …: BTCUSDT absent under …` / `… corpus entirely
  absent — nothing to run` line, no panic).
  test 2 (default `cargo test`, no `--include-ignored` — the CI-realistic invocation):
  `cargo test -p backtest --features realdata,yahoo --test p2_verdict_rerun`.
  output 2: `test result: ok. 8 passed; 0 failed; 7 ignored; 0 measured; 0 filtered
  out; finished in 29.09s` (S4 smoke + 7 unit tests run; S1/S2/S3/S5/S6/S7/S8 correctly
  `#[ignore]`d by default).
  test 3 (full sweep incl. the locally-present `data/binance-2122`, proves the
  harness handles a real multi-symbol field end-to-end): `cargo test -p backtest
  --features realdata,yahoo --test p2_verdict_rerun -- --include-ignored --nocapture`.
  output 3: `test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered
  out; finished in 233.91s`.
  Re-run after T6's DVOL back-fill (proves the harness is stable across the
  corpus-underneath change): same command as test 2 → `test result: ok. 8 passed;
  0 failed; 7 ignored; ... finished in 29.04s`, byte-identical S4 output.
- [x] **T10 — full gate sweep** — `bash scripts/verify_anchors.sh` (→119/119) +
  `python3 scripts/spec_lint.py` (→PASS) + `python3 scripts/adr_registry_check.py`
  (→exit 0) + `cargo build -p data -p backtest --features backtest/realdata,backtest/yahoo`
  + `cargo clippy -p data -p backtest --tests --features backtest/realdata,backtest/yahoo
  -- -D warnings` + `cargo fmt --check -p data -p backtest` + `git diff --stat` on the
  FROZEN gate files. `cargo test --workspace` (the FULL workspace sweep) NOT run —
  scoped to `-p data -p backtest` per the brief's explicit gate list; the tester owns
  the full-workspace confirmation.
  output: `ANCHORS PASS (119 / 119)`; `spec-lint: PASS (0 violations)`; exit 0
  (adr_registry_check); `Finished dev profile` (build, clean); `Finished dev profile`
  (clippy, ZERO warnings); fmt --check exit 0; `git diff --stat -- crates/backtest/src/bakeoff/{robustness,rank,scorecard,mod}.rs`
  → empty (byte-untouched).

## Handoff to tester — CLOSED (2026-07-10)

**Sequencing note (2026-07-10, historical):** T4 (the 4 multi-hour fetches)
had to land BEFORE T7/T8 (new-corpus consistency + smoke) and the full
S1/S2/S3/S5/S6/S7/S8 harness scenarios could produce real results — see the
developer's original HANDOFF message for the ready-to-run T4 commands. The
orchestrator ran T4 ahead of the tester session; by the time the tester
started, all 4 corpora were already on disk with valid `REVISION.toml`
manifests (independently re-verified this session, see T4/T7 above).

**Tester session (2026-07-10) — completed T7, T8, and THE full-corpus verdict
re-run:**

- **T7** (`crates/data/tests/p2_corpora_revision_consistency.rs`) — 4/4 green,
  un-ignored.
- **T8** (`crates/data/tests/p2_corpora_replayfeed_smoke.rs`) — 4/4 green,
  `--ignored`.
- **THE RUN** — `cargo test -p backtest --features realdata,yahoo --test
  p2_verdict_rerun -- --include-ignored --nocapture`: **15/15 passed, 0
  failed, finished in 238.79s (~4.0 min).** Full raw stdout captured at
  `/private/tmp/claude-502/-Users-Vitaliy-Schreibmann-Projects-Privat-trading-trading/362d2a09-04ba-4ea6-a7c1-07605f6e187a/scratchpad/p2-verdict-rerun-full.txt`
  (1,208 lines) and used as the evidence base for the report below.
- **The AC1-AC8 report** —
  [`reports/backtest-2026-07-10-p2-verdict-rerun.md`](../../../evidence/v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun.md),
  linked from the feature's `## Verification` section. **Headline finding
  (NOT suppressed): ship-passive WOBBLES on older/thinner-liquidity crypto
  eras (2017-18/2020/2021-22 show materially more `ActiveWins` crowns, 16/19
  of which clear the DSR≥0.95 credibility check) but HOLDS on the most recent
  regime (2025-26, matching the existing 2023-24 baseline) and on the
  Coinbase second-venue cross-check relative to ITS OWN Binance-era
  counterpart.** The era-cost `VolScaledSpread` annex (S7/S8) explains exactly
  1 of 10 tested symbol-runs as purely cost-sensitive (DOGEUSDT/2020) — the
  rest of the older-era `ActiveWins` gradient survives that stress-test.
  `MinBTL` before/after: the evidence base grew from 3.99 years (2 regimes,
  SHORT of the honest 6.36-year bar by 2.36 years) to 7.90 years (5 regimes +
  1 venue cross-check, MEETING the bar with +1.54 years margin) — the
  cleanest "stronger claim" result in the report. Full detail, per-corpus
  tables, and the exact AC1-AC8 mapping are in the report; this note is a
  summary only.
- **Gates:** `verify_anchors.sh` 119/119 BEFORE and AFTER (the report is a
  NEW non-anchored file, no anchors.toml edit); `spec_lint.py` PASS(0) after a
  one-line relative-link fix in the new report; `cargo fmt --check -p data` +
  `cargo clippy -p data --tests -- -D warnings` both clean on the two new T7/T8
  test files.
- **VERDICT → PASS.** Routed `HANDOFF → analyst (informational, non-blocking)`
  on the wobble-list finding for the product-copy question (should "no active
  edge" be scoped to "in today's deep-liquidity market" vs "in any crypto
  era, ever"?) — not a gate failure, a follow-on research question.

## Notes

- The Coinbase candle endpoint caps at **300 candles/call** (>300 → rejected) and
  the `time` field is in **seconds**; both handled in T1's shim. Coinbase historical
  data may have gaps ("no data is published for intervals where there are no
  ticks") — the `should_skip` content-SHA idempotency path (implemented in
  `fetch_coinbase_klines.rs`, byte-identical decision tree to the Binance bin's)
  handles genuinely short months exactly as it does for Binance.
- **Coinbase Exchange REQUIRES a `User-Agent` header** (`HTTP 400
  {"message":"User-Agent header is required."}` otherwise) — discovered during
  T2's live-network dry-run; `reqwest::Client`'s default has none. Fixed via
  `COINBASE_USER_AGENT` in `HttpCoinbaseKlineFetcher::fetch`
  (`crates/data/src/coinbase_klines.rs`).
- **`coinbase_symbol_map` is NOT the right symbol-mapping fn for this fetcher**
  (a real bug found + fixed during T1 testing) — it returns `BTC-USDT` (not
  `BTC-USD`) for a `BTCUSDT` input. Use `coinbase_product_id_for_symbol`
  instead (strip the on-disk symbol's own quote suffix, always re-append a
  FIXED `-USD`). See T1's tick note for the full analysis.
- The R4 arm×corpus availability matrix (feature § R4) is the binding arm-selection
  contract for T9. Perp-basis/funding MN arms are ABSENT on every corpus except the
  2324 base (basis is 2023-24 only) — do NOT back-fill them (out of scope). They
  never appear in `default_field()`/`default_ensemble_field()`/`default_macro_field()`
  at all (they live in the separate `bakeoff::sweep` robustness-sweep path), so no
  exclusion logic was needed in `build_field` — confirmed by a structural unit test
  (`build_field_never_includes_basis_or_funding_arms`).
- **A2 RESOLVED (bounded live probe, 2026-07-10):** BTC-USD hourly is served
  starting **2015-08** (2015-07 and earlier is empty) — DEEPER than the "~2015-16"
  estimate, comfortably covering the proposed `data/coinbase` 2020-01→last-closed-
  month window. NO window narrowing needed.
- **T6 macro back-fill is a NO-OP on this machine** — `data/yahoo-macro/` already
  covers 2021-01 through 2026-06 for all 3 tickers (DXY/GSPC/TNX). DVOL genuinely
  needed + received the back-fill (2021-22 + 2025-26). **`EXPECTED_DVOL_REVISION_SHA`
  in `crates/backtest/src/dvol_data.rs` moves whenever the DVOL corpus grows** — this
  is a hard-pinned constant over the WHOLE manifest aggregate (not a per-span
  subset); forgetting to update it after a future DVOL back-fill silently degrades
  `v0.dvol_regime` to warm-up-only on EVERY corpus, including the 2324 base.
- Anchors 119/119 + spec-lint PASS(0) gated per commit; `write_report=false`
  throughout (anchor-safe by construction); FROZEN gate byte-frozen; existing
  pinned corpora SHAs byte-immutable (verified via `shasum -a 256 -c` before/after
  the DVOL back-fill: the 4 pre-existing parquet files are byte-identical).
