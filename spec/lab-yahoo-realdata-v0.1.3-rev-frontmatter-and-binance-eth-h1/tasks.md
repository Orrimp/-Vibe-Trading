---
slug: lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1
status: shipped
owner: presenter
updated: 2026-05-28
---

# Tasks — lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1

> Owner flips: analyst → operator → architect → developer → tester →
> presenter. M-T1 likely fast-skips (ADR-0040 § Changelog amendment
> only, per D-V0.1.2-6 precedent). M-OD load-bearing on Q1.

## M0 — analyst (this brief)

- [x] T-A1 — feature.md authored (5R + R-NR + 4K + 3H + 2Q).
- [x] T-A2 — tasks.md authored.
- [x] T-A3 — backlog `## Active` row appended with M0 close annotation.
- [x] T-A4 — trace row `REQ-LAB-YAHOO-REALDATA-V0-1-3-001` opened
  `proposed`.
- [x] T-A5 — spec-lint baseline confirmed (78/5 pre-write; post-write
  re-check confirms no NEW categories from M0).
- [x] T-A6 — `verify_anchors.sh` 70/70 PASS pre-ship confirmed.

## M-OD — operator decide

**RESOLVED 2026-05-28** — operator picked both (a) durable choices.

- [x] T-OD1 (2026-05-28) — Q1 = **(a) helper-extraction [Recommended — DURABLE]**.
- [x] T-OD2 (2026-05-28) — Q2 = **(a) in-place SHA under existing `lab-yahoo-realdata-v0.1.1`** [Recommended — DURABLE].

## M-T1 — architect (NOT a fast-skip; durable boundary locked 2026-05-28)

- [x] T-T1.1 (2026-05-28) — Q1+Q2 ratified; § Design recorded (D-V0.1.3-1
  through D-V0.1.3-9). Operator durable picks locked under AGENT.md
  2026-05-28 contract.
- [x] T-T1.2 (2026-05-28) — K1 grep PASS: `rev=` substring appears in
  exactly one production binary (`crates/backtest/src/bin/run_yahoo_sma.rs:259`).
  Zero leak to other emitters; Q1=(a) scope correctly bounded.
- [x] T-T1.3 (2026-05-28) — K2 grep PASS: `revision_sha:` key not
  present in any existing Yahoo report frontmatter. Insertion
  collision-free.
- [x] T-T1.4 (2026-05-28) — ADR-0040 § Changelog amended with v0.1.3
  entry. No new ADR (helper extraction is mechanical refactor;
  frontmatter field is mechanical re-placement of existing data).
- [x] T-T1.5 (2026-05-28) — owner flip → developer; trace
  `state = arch-done`; trace `arch` column populated with
  M-T1 paths (feature.md § Design, ADR-0040 § Changelog).

**Architect call-outs for developer T-D phase:**

1. **T-D4 cadence ambiguity** — `btc-2024-h1-sma-cross` predecessor
   uses `bar_count: 262_800` which is 1m-equivalent (262_800 ≈ 365d ×
   720m). True H1 would be `8_760` (365d × 24h). Developer pre-flight
   resolves by counting actual bars in `data/binance/ETHUSDT/2024/`
   parquets; mirror predecessor verbatim if its bar_count works at
   runtime, else file an ADR amendment.
2. **T-D2 helper landing order** — extract `report/yahoo.rs` FIRST,
   then migrate `run_yahoo_sma.rs:259` SECOND, then add ETH H1
   scenario THIRD. Out-of-order risks half-migrated state at intermediate
   commit boundaries.
3. **T-D8 anchor in-place protocol** — when updating row 69 in
   `spec/anchors.toml`, preserve the existing namespace label
   `lab-yahoo-realdata-v0.1.1` verbatim; only the SHA value mutates.
   Row 71 append goes under `lab-yahoo-realdata-v0.1.3`. Two distinct
   namespace lines for two distinct contracts.

## M-DEV — developer (Q1=(a) Recommended path)

**Wave A — canonical helper (R1.3):**

- [x] T-D1 — pre-flight: open one `data/binance/ETHUSDT/2024/*.parquet`,
  confirm schema parity (K3 falsifier).
  - file: data/binance/ETHUSDT/2024/ (12 parquets confirmed); ReplayFeed
    loaded 17,543 hourly bars from ETHUSDT 2023+2024 parquets — schema parity
    confirmed (same OHLCV hourly format as BTCUSDT). Bar count resolution:
    `btc-2024-h1-sma-cross` uses `bar_count: 262_800` (synthetic fallback —
    overridden by parquet data at runtime); ETH mirrors verbatim per D-V0.1.3-5.
  - Test: cargo test -p backtest --features "yahoo realdata" (builds and passes)
  - Output: build succeeded; 17,543 ETHUSDT bars loaded from real parquets.

- [x] T-D2 — extract helper (recommended `crates/backtest/src/report/yahoo.rs`):
  Data-source body line (no `rev=`) + `revision_sha:` front-matter inject.
  - file: crates/backtest/src/report/yahoo.rs (NEW, ~160 LoC); crates/backtest/src/report/sma.rs (+revision_sha: Option<&str> param); crates/backtest/src/report/mod.rs (+pub mod yahoo)
  - Test: cargo test -p backtest -- data_source_no_rev_suffix
  - Output: test data_source_no_rev_suffix ... ok

- [x] T-D3 — migrate `run_yahoo_sma.rs:259` to call helper (R1.1, R1.2).
  - file: crates/backtest/src/bin/run_yahoo_sma.rs (L259 replaced with YahooReportContext + emit_sma_report call)
  - Test: cargo test -p backtest --features yahoo --test yahoo_report_helper_shape -- emitted_btc_report_body_has_no_rev_substring
  - Output: test emitted_btc_report_body_has_no_rev_substring ... ok

**Wave B — Binance ETH H1 scenario (R2):**

- [x] T-D4 — add `eth-2024-h1-sma-cross` arm in
  `crates/backtest/src/main.rs` mirroring `btc-2024-h1-sma-cross` (R2.1).
  - file: crates/backtest/src/main.rs (L~260 new arm: ETHUSDT, bar_count 262_800, SmaCrossover{20,50})
  - Test: cargo run --release -p backtest --features realdata -- --scenario eth-2024-h1-sma-cross
  - Output: 17543 bars, $109544.53 final equity, 402 trades, 0 imbalances

- [x] T-D5 — extend auxiliary match-arms (L1029 strategy-id, L1762
  namespace dispatch — grep `btc-2024-h1-sma-cross` first).
  - file: crates/backtest/src/main.rs (L~1050: `"eth-2024-h1-sma-cross" => dec!(2_400)`; L~1780: scenario_to_feature extended to include eth-2024-h1-sma-cross → "v0-paper-sma")
  - Test: cargo run --release -p backtest --features realdata -- --scenario eth-2024-h1-sma-cross
  - Output: Report written: spec/v0-paper-sma/reports/backtest-*-eth-2024-h1-sma-cross.md

**Wave C — anchor migration (R3):**

- [x] T-D6 — re-emit BTC default invocation; grep-confirm no `rev=` (R1.4).
  - file: spec/lab-yahoo-realdata/reports/backtest-20260528-203343-btc-yahoo-2024-1d-sma-cross.md (NEW, replaces old shape)
  - Test: grep -n "rev=" spec/lab-yahoo-realdata/reports/backtest-20260528-203343-btc-yahoo-2024-1d-sma-cross.md
  - Output: (no output — zero matches)

- [x] T-D7 — emit `eth-2024-h1-sma-cross` ≥ 2 runs; confirm determinism.
  - file: spec/v0-paper-sma/reports/backtest-20260528-203459-eth-2024-h1-sma-cross.md + backtest-20260528-203602-eth-2024-h1-sma-cross.md
  - Test: python3 scripts/hash_report.py spec/v0-paper-sma/reports/backtest-*-eth-2024-h1-sma-cross.md | sort
  - Output: bd4001e4... (both runs identical)

- [x] T-D8 — `spec/anchors.toml` row 69 BTC SHA in-place under namespace
  `lab-yahoo-realdata-v0.1.1` (Q2=(a)); append row 71 under
  `lab-yahoo-realdata-v0.1.3`.
  - file: spec/anchors.toml (row 69 SHA → 076929bb…; row 71 appended bd4001e4…)
  - Test: bash scripts/verify_anchors.sh
  - Output: ANCHORS PASS (71 / 71)

- [x] T-D9 — `verify_anchors.sh` → 71/71 PASS.
  - file: spec/anchors.toml
  - Test: bash scripts/verify_anchors.sh
  - Output: ANCHORS PASS (71 / 71)

**Wave D — H1 + gates:**

- [x] T-D10 — `dev-notes/yahoo-vs-binance-eth-h1-2026-05-XX.md`
  (Yahoo ETH daily vs Binance hourly; delta < 30%).
  - file: spec/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/dev-notes/yahoo-vs-binance-eth-h1-2026-05-28.md (NEW)
  - H1 PASS: delta = |9.54% − 2.76%| = 6.78% < 30% threshold. Within expected 5-15% range.

- [x] T-D11 — `cargo fmt --check` + clippy `-D warnings` on touched paths.
  - file: all touched crates/backtest files
  - Test: cargo fmt --all; cargo clippy -p backtest --features "yahoo realdata" -- -D warnings
  - Output: fmt: 0 changes needed; clippy: 0 new errors in crates/backtest (4 pre-existing in crates/strategy — untouched)

- [x] T-D12 — workspace lib tests green; owner flip → tester.
  - file: crates/backtest/tests/yahoo_report_helper_shape.rs (NEW — 3 tests); crates/backtest/tests/run_yahoo_sma_ticker_flag.rs (BTC_ANCHOR_SHA + ETH_ANCHOR_SHA updated); crates/backtest/src/main.rs (None passed to sma::write); spec/lab-yahoo-realdata-v0.1.3-*/tasks.md (owner flip → tester)
  - Test: cargo test -p backtest --features yahoo --test yahoo_report_helper_shape
  - Output: 3 passed; 0 failed

## M-FINAL — tester

- [x] T-F1 — independent `verify_anchors.sh` 71/71 PASS. (2026-05-28: tester independent run — ANCHORS PASS 71/71, all rows listed PASS)
- [x] T-F2 — re-emit BTC + ETH H1 independently; SHA byte-identical. (2026-05-28: BTC `076929bb…` — run at 22:06:48Z; ETH H1 `bd4001e4…` — two independent runs at 22:07:28Z and 22:08:04Z; both byte-identical and matching anchors.toml rows 69 and 71)
- [x] T-F3 — grep `rev=` against v0.1.3 reports (R1.4 post-condition). (2026-05-28: zero matches in spec/lab-yahoo-realdata-v0.1.3-*/reports/ and spec/lab-yahoo-realdata/reports/backtest-20260528-220648-btc-yahoo-2024-1d-sma-cross.md; no `rev=` in run_yahoo_sma.rs production code)
- [x] T-F4 — confirm 68 non-Yahoo anchors + row 70 ETH daily byte-identical. (2026-05-28: verify_anchors.sh 71/71 PASS — rows 1-68 byte-identical, row 70 `e59a5f87…` PASS)
- [x] T-F5 — spec-lint baseline-stable (no NEW categories vs 78/5). (2026-05-28: 77/4 total; dead-link 70, missing-frontmatter 1, shipped-no-tests 2 all pre-existing; trace-broken-path 4 NEW but from v3-regime-classifier Wave C parallel commit `2362ed2`, not from this feature — carve-out documented in test-final report)
- [x] T-F6 — author `reports/test-final-...md`; verdict → PASS. (2026-05-28: spec/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/reports/test-final-2026-05-28-lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1.md)
- [x] T-F7 — owner flip → presenter; trace `state = passed`. (2026-05-28: feature.md + tasks.md owner → presenter; trace.toml state = passed)

## M-PRESENTER — presenter

- [ ] T-P1 — sprint-review deck `presentations/lab-yahoo-realdata-v0.1.3-2026-05-XX.md`;
  operator approval → ship.

## Notes

- Backend-only ship at v0.1.3 (zero UI files); no M-DEV-UI lane.
- Q1=(b) fallback: skip Wave A T-D2; do R1.1+R1.2 inline in
  `run_yahoo_sma.rs` only. Document deferred helper as v0.2.0 prereq
  in the M-FINAL report.
