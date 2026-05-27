---
slug: lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge
status: draft
owner: analyst
updated: 2026-05-27
---

# Tasks — lab-yahoo-realdata v0.1.2

Scaffold authored by analyst M0; architect refines waves at M-T1.

## M0 — Analyst (this pass)

- [x] T-A1 — Survey current HEAD; confirm per-pair `cache_state_badge` shipped + ETH cache populated.
- [x] T-A2 — Author `feature.md` (5 R / 4 K / 4 H / 3 Q + verdict tree + cost framing).
- [x] T-A3 — Author this `tasks.md` scaffold.
- [x] T-A4 — Append backlog Active row.
- [x] T-A5 — Append `REQ-LAB-YAHOO-REALDATA-V0-1-2-001` trace row at EOF, state=`proposed`.
- [x] T-A6 — Verify `bash scripts/verify_anchors.sh` PASS (69/69 baseline preserved).
- [x] T-A7 — Verify `spec_lint.py` no NEW violation categories vs 73/3 baseline.

## M-OD — Operator decide

- [ ] T-OD1 — Q1 (LOAD-BEARING): ticker handling (extend vs separate binary). Default (a) extend.
- [ ] T-OD2 — Q2: summary badge placement. Default (a) source-toggle row.
- [ ] T-OD3 — Q3: summary badge content. Default (c) middle-ground.

## M-T1 — Architect

- [ ] T-T1.1 — Ratify Q1/Q2/Q3 in feature.md § Operator-decide section.
- [ ] T-T1.2 — Decide K3 mitigation cadence (per-frame probe vs cached `LabState` summary refreshed on Lab-Run-complete / data_source toggle). Document the choice.
- [ ] T-T1.3 — Confirm `--ticker` flag validation table mirrors `binance_to_yahoo_ticker_lookup` 10-row inverse.
- [ ] T-T1.4 — Confirm scenario-name derivation rule (`{lowercased-ticker-without-usd}-yahoo-2024-1d-sma-cross`).
- [ ] T-T1.5 — Decompose into M-DEV + M-DEV-UI tasks below; flip `feature.md` owner to `architect`.

## M-DEV — Developer (parallel with M-DEV-UI)

- [ ] T-D1 — Pre-flight: confirm `data/binance/ETHUSDT/2024/` exists + REVISION.toml current (K1 falsifier check). If missing, ROUTE BACK to analyst.
- [ ] T-D2 — Extend `crates/backtest/src/bin/run_yahoo_sma.rs` with `--ticker <TICKER>` Clap arg, default `BTC-USD` (R4).
- [ ] T-D3 — Add scenario-name + symbol + cache-subdir + report-filename substitution (R4.2 / R4.3).
- [ ] T-D4 — Run `cargo run … --bin run_yahoo_sma --` (no flag); assert body SHA == v0.1.1 BTC `8045623b…` (H3 anchor preservation).
- [ ] T-D5 — Run `cargo run … --bin run_yahoo_sma -- --ticker ETH-USD`; record body SHA; re-run ×3 for determinism (H2).
- [ ] T-D6 — Append row to `spec/anchors.toml` (scenario `eth-yahoo-2024-1d-sma-cross`, version `lab-yahoo-realdata-v0.1.2`, sha256 = recorded SHA from T-D5). R1.
- [ ] T-D7 — `bash scripts/verify_anchors.sh` → expect `ANCHORS PASS (70 / 70)` (R1.4).
- [ ] T-D8 — Author `dev-notes/yahoo-vs-binance-divergence-eth-2026-05-XX.md` mirroring v0.1.1 BTC dev-note shape; record H1 delta + verdict (R2.3).
- [ ] T-D9 — Add integration test `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs` (BTC SHA + ETH SHA assertions, R4.4).
- [ ] T-D10 — `cargo fmt --all --check` + `cargo clippy -p backtest -- -D warnings` (R-NR.3 / R-NR.4).

## M-DEV-UI — UI-designer (parallel with M-DEV)

- [ ] T-DU1 — Author `crates/ui/src/widgets/cache_state_summary_badge.rs` (R3.1, R3.2, R3.3, R3.4).
- [ ] T-DU2 — Add 1 new string to `crates/ui/src/strings.rs`: `LAB_CACHE_STATE_SUMMARY_PREFIX = "Cache: "`. Reuse `LAB_CACHE_STATE_EMPTY` for N=0 (R-NR.2).
- [ ] T-DU3 — Extend `crates/ui/src/lab/cache_state.rs` with `probe_summary(cache_root, tickers, now) -> CacheSummary` (R3.5).
- [ ] T-DU4 — Wire summary badge into `crates/ui/src/screens/lab.rs` source-toggle row AFTER the per-pair pill, gated on `data_source = YahooCache` (R3.6).
- [ ] T-DU5 — Add 4 gallery cells: `cache_state_summary_badge__empty`, `…__one_ticker`, `…__two_tickers`, `…__ten_tickers`.
- [ ] T-DU6 — Author 3 unit tests in the widget module: label format, count formatting, ISO date formatting.
- [ ] T-DU7 — Layout verification at 1280 / 1024 / 960 px breakpoints (K4 mitigation).
- [ ] T-DU8 — Per-frame probe latency benchmark for 10-ticker case; assert < 5 ms (H4). If ≥ 5 ms, escalate K3 cached-summary mitigation (talk to architect).
- [ ] T-DU9 — `cargo fmt --all --check` + `cargo clippy -p ui -- -D warnings` (R-NR.3 / R-NR.4).

## M-FINAL — Tester

- [ ] T-F1 — `bash scripts/verify_anchors.sh` exit 0 with `ANCHORS PASS (70 / 70)`.
- [ ] T-F2 — `cargo fmt --all --check` clean.
- [ ] T-F3 — `cargo clippy --workspace -- -D warnings` clean.
- [ ] T-F4 — `cargo test --workspace` ≥ 1187 lib tests pass; new integration test green.
- [ ] T-F5 — Re-run `run_yahoo_sma --ticker ETH-USD` independently; assert SHA matches `spec/anchors.toml` row 70 (H2 second-witness).
- [ ] T-F6 — Re-run `run_yahoo_sma` (no flag); assert BTC SHA `8045623b…` byte-identical (H3 second-witness).
- [ ] T-F7 — Gallery snapshot diff for new + existing cache-state cells.
- [ ] T-F8 — `spec_lint.py` → no NEW violation categories vs 73/3 baseline (R-NR.4).
- [ ] T-F9 — Author `reports/test-final-<date>-lab-yahoo-realdata-v0.1.2.md`; VERDICT → PASS or REGRESSION.

## M-PRESENTER

- [ ] T-P1 — Author `presentations/lab-yahoo-realdata-v0.1.2-2026-05-XX.md` sprint-review deck.
- [ ] T-P2 — Live demo: re-run `run_yahoo_sma --ticker ETH-USD`; show 70/70 anchors PASS + ETH/BTC H1 divergence numbers.
- [ ] T-P3 — Operator approval block + handoff to operator.
