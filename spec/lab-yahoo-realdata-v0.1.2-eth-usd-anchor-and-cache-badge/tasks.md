---
slug: lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge
status: in-progress
owner: developer + ui-designer
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

**RESOLVED 2026-05-27 — all 3 locked at analyst-recommended defaults.**

- [x] T-OD1 (2026-05-27) — Q1 LOAD-BEARING: ticker handling = **(a) extend `run_yahoo_sma.rs` with `--ticker` flag** (default BTC-USD; ~15 LoC delta). Scales DRY across remaining 8 unanchored crypto-mirror tickers. H3 anchor preservation provable by default-arg invocation matching v0.1.1 BTC SHA `8045623b...`.
- [x] T-OD2 (2026-05-27) — Q2: aggregate badge placement = **Lab tab toolbar**. Visible whenever Lab is active; doesn't clutter the global 24 px activity tape.
- [x] T-OD3 (2026-05-27) — Q3: aggregate badge content = **middle-ground "Yahoo cache: 2 tickers · last fetch YYYY-MM-DD"**. Operator-readable at a glance; click-to-expand reveals per-ticker detail.

## M-T1 — Architect

**RESOLVED 2026-05-27 — all 6 closed. See feature.md § Design (D-V0.1.2-1..6).**

- [x] T-T1.1 (2026-05-27) — Ratified Q1=(a) / Q2=(b operator-override) / Q3=(c with `"Yahoo cache: "` prefix). § Operator-decide rows annotated with LOCKED 2026-05-27 stamps; R3.2/R3.6/R-NR.2 amended to reflect operator Q2/Q3.
- [x] T-T1.2 (2026-05-27) — K3 cadence = **cached-summary on `LabState`** (D-V0.1.2-1). `cache_summary: Option<CacheSummary>` field invalidated on `data_source` toggle + `Lab-Run-complete`. Per-frame `view()` reads cached value; no background polling.
- [x] T-T1.3 (2026-05-27) — `--ticker` validation surface confirmed (D-V0.1.2-2). CLI ships a 10-row `const ALLOWED_YAHOO_TICKERS` mirror; cross-crate pinned-table test in `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs` locks drift vs `data::yahoo::binance_to_yahoo_ticker` source-of-truth.
- [x] T-T1.4 (2026-05-27) — Scenario-name rule locked (D-V0.1.2-3): `{lc-ticker-no-USD}-yahoo-2024-1d-sma-cross`. Forecast for 10 rows tabulated; rows 71-78 reserved for v0.1.3+ but unanchored at v0.1.2.
- [x] T-T1.5 (2026-05-27) — M-DEV / M-DEV-UI decomposed below; UI lane gained T-DU3.5 (LabState invalidation hooks). Frontmatter owner flipped `architect → developer + ui-designer`.
- [x] T-T1.6 (2026-05-27) — ADR decision: **extend ADR-0040 § Changelog** (no new ADR-0048). Per D-V0.1.2-6 the v0.1.2 surface is a per-ticker generalisation of existing D7 + D3 — no new architectural decisions. ADR-0040 Changelog edited at this M-T1.

## M-DEV — Developer (parallel with M-DEV-UI; zero file overlap per D-V0.1.2-4)

- [ ] T-D1 — Pre-flight: confirm `data/binance/ETHUSDT/2024/` exists + REVISION.toml current (K1 falsifier check). If missing, ROUTE BACK to analyst.
- [ ] T-D2 — Extend `crates/backtest/src/bin/run_yahoo_sma.rs` with `--ticker <TICKER>` Clap arg, default `BTC-USD` (R4). 6 mechanical BTC-USD substitution sites per analyst F2 (CLI `about`, `SCENARIO_NAME`, `Symbol::new`, `load_cached`, `data_source` string format, doc comments). Add `const ALLOWED_YAHOO_TICKERS: &[&str]` with the 10 Yahoo tickers (D-V0.1.2-2); validate flag against it; unknown → exit 2 with `clap::error::ErrorKind::InvalidValue`-shaped message listing the 10 allowed values.
- [ ] T-D3 — Replace `const SCENARIO_NAME` with a `fn scenario_name(ticker: &str) -> String` per D-V0.1.2-3 rule (`{lc-ticker-no-USD}-yahoo-2024-1d-sma-cross`). 1 LoC pure-function helper + 1 unit test asserting BTC-USD → `btc-yahoo-2024-1d-sma-cross` AND ETH-USD → `eth-yahoo-2024-1d-sma-cross`.
- [ ] T-D4 — Run `cargo run -p backtest --features yahoo --bin run_yahoo_sma --` (no flag); recompute body SHA via `python3 scripts/hash_report.py …` and assert byte-identical to v0.1.1 BTC anchor `8045623b…` (H3 anchor preservation gate — if drift, REVERT Q1=(a) and route back to architect).
- [ ] T-D5 — Run `cargo run … --bin run_yahoo_sma -- --ticker ETH-USD`; record body SHA; re-run ×3 for determinism (H2). Document the 3 re-run SHAs in the dev-note from T-D8.
- [ ] T-D6 — Append row to `spec/anchors.toml` (scenario `eth-yahoo-2024-1d-sma-cross`, version `lab-yahoo-realdata-v0.1.2`, sha256 = recorded SHA from T-D5). R1.
- [ ] T-D7 — `bash scripts/verify_anchors.sh` → expect `ANCHORS PASS (70 / 70)` (R1.4).
- [ ] T-D8 — Author `dev-notes/yahoo-vs-binance-divergence-eth-2026-05-XX.md` mirroring v0.1.1 BTC dev-note shape; record H1 delta + verdict (R2.3). H1 falsifier ≥ 30% → route back to analyst (K1 synthetic-comparison fallback).
- [ ] T-D9 — Add integration test `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs` with: (a) BTC SHA assertion (H3 second-witness via `assert_cmd` invocation of the binary); (b) ETH SHA assertion (anchor 70 second-witness); (c) cross-crate pinned-table test asserting `ALLOWED_YAHOO_TICKERS` matches the RHS of `data::yahoo::binance_to_yahoo_ticker` (D-V0.1.2-2 drift gate); (d) unknown-ticker `--ticker FOO-USD` exits non-zero. R4.4.
- [ ] T-D10 — `cargo fmt --all --check` + `cargo clippy -p backtest -- -D warnings` (R-NR.3 / R-NR.4).

## M-DEV-UI — UI-designer (parallel with M-DEV; zero file overlap per D-V0.1.2-4)

- [ ] T-DU1 — Author `crates/ui/src/widgets/cache_state_summary_badge.rs` (R3.1, R3.2, R3.3, R3.4). Public API: `pub fn view(summary: &CacheSummary, mode: ThemeMode) -> Element<'static>`. Renders `"Yahoo cache: {N} tickers · last fetch {YYYY-MM-DD}"` when `populated_count >= 1`, otherwise the existing `LAB_CACHE_STATE_EMPTY` label. Use the same Lumen tokens as `cache_state_badge::view` (PANEL_RAISED bg, BORDER_1 outline, R3 radius, MICRO text, XXS/S padding).
- [ ] T-DU2 — Add 1 new string to `crates/ui/src/strings.rs`: `LAB_CACHE_STATE_SUMMARY_PREFIX = "Yahoo cache: "` (operator Q3 lock — note the `"Yahoo "` prefix vs analyst's bare `"Cache: "`). Reuse `LAB_CACHE_STATE_EMPTY` for N=0 (R-NR.2).
- [ ] T-DU3 — Extend `crates/ui/src/lab/cache_state.rs` with:
  - `pub struct CacheSummary { pub populated_count: usize, pub newest_mtime: Option<SystemTime> }` (with `Clone, Debug, PartialEq, Eq` for snapshot tests).
  - `pub fn probe_summary(cache_root: &Path, tickers: &[&str], now: SystemTime) -> CacheSummary` — iterates the 10-row mirror, calls `newest_parquet_mtime` per ticker dir, counts non-empty, returns the global max-mtime. Bounded by 10 × ~3 stats = 30 stats per call. Pure-fn (no global state).
  - Helper `pub const ALL_YAHOO_TICKERS: &[&str]` re-exporting the 10 Yahoo tickers from `binance_to_yahoo_ticker_lookup` RHS (or a separate const if cleaner) so callers don't have to know the 10-row table externally.
- [ ] T-DU3.5 — **NEW (architect M-T1, D-V0.1.2-1)**: Wire cached-summary on `LabState`:
  - Add `pub cache_summary: Option<CacheSummary>` field to `crates/ui/src/lab/state.rs::LabState` with `Default = None`.
  - Add invalidation hook in the `LabSetDataSource` handler (or wherever the source toggle dispatches) — set `cache_summary = None`.
  - Add invalidation hook in the Lab-Run-complete branch (search for where the run-button transitions from `Running` → `Done`) — set `cache_summary = None`.
  - In `screens/lab.rs::view`, before rendering the toolbar, populate the field lazily via interior mutability (`RefCell` already used per line 547 comment), or — preferred — populate it in the update handler immediately after invalidation so `view()` reads `&LabState` immutably. Talk to architect if the update-side approach is non-obvious.
- [ ] T-DU4 — Wire summary badge into `crates/ui/src/screens/lab.rs` **Lab tab toolbar** (operator Q2 lock — NOT the source-toggle row): add a NEW `Row` as the FIRST child of the Lab body `Column` (above the existing pair-chip row at line 188). Layout: `Row::new().push(spacer Fill).push(cache_state_summary_badge::view(&summary, mode))` so the badge right-aligns. Render this row for **every** Lab activation regardless of `data_source` (R3.6). The per-pair pill at `lab.rs:226` stays where it is (sibling badge on the source-toggle row, gated on `is_yahoo`).
  - Update the body-height arithmetic in `chart_canvas_height_for_body_with_training` (line 130-165) to deduct one additional `CHIP_ROW_HEIGHT_PX` (~32 px) + one `space::M` gap for the new toolbar row. The 11-children → 12-children comment at line 142-147 needs a +1 bump.
- [ ] T-DU5 — Add 4 gallery cells: `cache_state_summary_badge__empty`, `…__one_ticker`, `…__two_tickers`, `…__ten_tickers`. Same shape as existing `cache_state_badge__*` cells in `crates/ui/src/gallery/routes.rs`.
- [ ] T-DU6 — Author 3 unit tests in the widget module: (a) label format for `populated_count >= 1`; (b) `populated_count == 0` renders the empty label; (c) ISO date formatting of `newest_mtime` via `time::OffsetDateTime` `YYYY-MM-DD` (no `chrono` per workspace convention).
- [ ] T-DU7 — Layout verification at 1280 / 1024 / 960 px breakpoints (K4 mitigation now applies to the Lab toolbar row, not the source-toggle row — see D-V0.1.2-5 risk register update).
- [ ] T-DU8 — One-shot `probe_summary` latency benchmark for the 10-ticker case (NOT per-frame — D-V0.1.2-1 cached cadence means per-frame is never reached); assert < 5 ms even on cold cache (H4). If ≥ 5 ms, that is still acceptable because of the cached-summary design, but log it for the dev-note.
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
