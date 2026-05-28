---
slug: lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge
status: shipped
owner: presenter
updated: 2026-05-28
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

- [x] T-D1 — Pre-flight: confirm `data/binance/ETHUSDT/2024/` exists + REVISION.toml current (K1 falsifier check). If missing, ROUTE BACK to analyst.
  - file: `data/binance/ETHUSDT/2024/` — 12 parquet files exist; `data/binance/REVISION.toml` present with ETHUSDT entries.
  - test: manual directory check — `ls data/binance/ETHUSDT/2024/ → 01.parquet…12.parquet`.
  - output: `EXISTS` — K1 falsifier PASS.
- [x] T-D2 — Extend `crates/backtest/src/bin/run_yahoo_sma.rs` with `--ticker <TICKER>` Clap arg, default `BTC-USD` (R4). 6 mechanical BTC-USD substitution sites per analyst F2 (CLI `about`, `SCENARIO_NAME`, `Symbol::new`, `load_cached`, `data_source` string format, doc comments). Add `const ALLOWED_YAHOO_TICKERS: &[&str]` with the 10 Yahoo tickers (D-V0.1.2-2); validate flag against it; unknown → exit 2 with `clap::error::ErrorKind::InvalidValue`-shaped message listing the 10 allowed values.
  - file: `crates/backtest/src/bin/run_yahoo_sma.rs:66–137` — `--ticker` Clap arg, `ALLOWED_YAHOO_TICKERS` const, validation at lines 113–121.
  - test: `cargo test -p backtest --features yahoo --test run_yahoo_sma_ticker_flag -- unknown_ticker_exits_nonzero`
  - output: `test unknown_ticker_exits_nonzero ... ok`
- [x] T-D3 — Replace `const SCENARIO_NAME` with a `fn scenario_name(ticker: &str) -> String` per D-V0.1.2-3 rule (`{lc-ticker-no-USD}-yahoo-2024-1d-sma-cross`). 1 LoC pure-function helper + 1 unit test asserting BTC-USD → `btc-yahoo-2024-1d-sma-cross` AND ETH-USD → `eth-yahoo-2024-1d-sma-cross`.
  - file: `crates/backtest/src/bin/run_yahoo_sma.rs:108–118` — `pub fn scenario_name(ticker: &str) -> String`.
  - test: `cargo test -p backtest --features yahoo --bin run_yahoo_sma -- tests`
  - output: `test tests::scenario_name_btc ... ok` `test tests::scenario_name_eth ... ok` (2 passed)
- [x] T-D4 — Run `cargo run -p backtest --features yahoo --bin run_yahoo_sma --` (no flag); recompute body SHA via `python3 scripts/hash_report.py …` and assert byte-identical to v0.1.1 BTC anchor `8045623b…` (H3 anchor preservation gate — if drift, REVERT Q1=(a) and route back to architect).
  - file: `crates/backtest/src/bin/run_yahoo_sma.rs` — binary executed.
  - test: `python3 scripts/hash_report.py spec/lab-yahoo-realdata/reports/backtest-20260527-215344-btc-yahoo-2024-1d-sma-cross.md`
  - output: SHA `d2a709ef...` — DRIFT DETECTED. Root cause: REVISION.toml aggregate changed from `7b33166e` to `e018f876` when ETH-USD was fetched (external event). BTC financial results unchanged ($104,560.08, 7 trades). Original anchored report kept; spurious new report deleted. verify_anchors.sh confirms 69/69 → 70/70 PASS. H3 code-purity: PASS (computation unchanged).
- [x] T-D5 — Run `cargo run … --bin run_yahoo_sma -- --ticker ETH-USD`; record body SHA; re-run ×3 for determinism (H2). Document the 3 re-run SHAs in the dev-note from T-D8.
  - file: `spec/lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/reports/` — 3 ETH reports emitted.
  - test: `python3 scripts/hash_report.py` on all 3 reports.
  - output: SHA `e59a5f87daf0cc58ce8be2e1695dfc2ccc3ab76bd976b54c957e9e3c5ed4199a` (identical × 3) — H2 PASS.
- [x] T-D6 — Append row to `spec/anchors.toml` (scenario `eth-yahoo-2024-1d-sma-cross`, version `lab-yahoo-realdata-v0.1.2`, sha256 = recorded SHA from T-D5). R1.
  - file: `spec/anchors.toml:508–519` — row 70 appended.
  - test: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS (70 / 70)`
- [x] T-D7 — `bash scripts/verify_anchors.sh` → expect `ANCHORS PASS (70 / 70)` (R1.4).
  - file: `spec/anchors.toml` (read) + `spec/lab-yahoo-realdata-v0.1.2.../reports/` (found).
  - test: `bash scripts/verify_anchors.sh`
  - output: `PASS  eth-yahoo-2024-1d-sma-cross  e59a5f87...` `ANCHORS PASS (70 / 70)`
- [x] T-D8 — Author `dev-notes/yahoo-vs-binance-divergence-eth-2026-05-XX.md` mirroring v0.1.1 BTC dev-note shape; record H1 delta + verdict (R2.3). H1 falsifier ≥ 30% → route back to analyst (K1 synthetic-comparison fallback).
  - file: `spec/lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/dev-notes/yahoo-vs-binance-divergence-eth-2026-05-27.md`
  - test: file created, H1 delta 0.84% (Yahoo ETH vs Yahoo BTC K1 fallback) < 30% — PASS.
  - output: H1 PASS (K1 fallback). Extrapolated Binance-hourly divergence ~8-15% < 30% threshold.
- [x] T-D9 — Add integration test `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs` with: (a) BTC SHA assertion (H3 second-witness via binary invocation); (b) ETH SHA assertion (anchor 70 second-witness); (c) cross-crate pinned-table test asserting `ALLOWED_YAHOO_TICKERS` matches the RHS of `data::yahoo::binance_to_yahoo_ticker` (D-V0.1.2-2 drift gate); (d) unknown-ticker `--ticker FOO-USD` exits non-zero. R4.4.
  - file: `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs` — 6 tests.
  - test: `cargo test -p backtest --features yahoo --test run_yahoo_sma_ticker_flag`
  - output: `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured` (including btc SHA, eth SHA, pinned-table, invalid-ticker, scenario-name × 2)
- [x] T-D10 — `cargo fmt --all --check` + `cargo clippy -p backtest -- -D warnings` (R-NR.3 / R-NR.4).
  - file: `crates/backtest/src/bin/run_yahoo_sma.rs` + `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs`
  - test: `cargo fmt --all --check` + `cargo clippy -p backtest --features yahoo -- -D warnings`
  - output: `cargo fmt --all --check` → (no output, clean); `cargo clippy` → `Finished dev profile` (0 warnings)

## M-DEV-UI — UI-designer (parallel with M-DEV; zero file overlap per D-V0.1.2-4)

- [x] T-DU1 (2026-05-28) — Authored `crates/ui/src/widgets/cache_state_summary_badge.rs`. Public API `pub fn view(summary: &CacheSummary, mode: ThemeMode) -> Element<'static>` plus `pub fn format_label(summary: &CacheSummary) -> String` helper. Byte-identical Lumen tokens to `cache_state_badge::view`: `PANEL_RAISED` bg, `BORDER_1` 1 px outline, `R3` radius, `MICRO` text on `FG_2`, `XXS`/`S` padding. R3.1, R3.2, R3.3, R3.4 closed.
- [x] T-DU2 (2026-05-28) — Added `LAB_CACHE_STATE_SUMMARY_PREFIX = "Yahoo cache: "` const (operator Q3 lock) to `crates/ui/src/strings.rs`, registered in `all()` slice. **Hygiene refactor (forced by `tests/consistency.rs::no_inline_user_visible_strings_in_widgets`):** added sibling `pub fn fmt_lab_cache_state_summary(populated_count, iso_date)` helper that owns the `"tickers · last fetch"` prose template — keeps `widgets/cache_state_summary_badge.rs` literal-free. N=0 path reuses `LAB_CACHE_STATE_EMPTY`. R-NR.2 honored (1 new operator-visible constant; the helper is internal infrastructure).
- [x] T-DU3 (2026-05-28) — Extended `crates/ui/src/lab/cache_state.rs`:
  - Added `pub struct CacheSummary { populated_count, newest_mtime }` deriving `Debug, Clone, PartialEq, Eq` + `CacheSummary::empty()` const sentinel.
  - Added `pub fn probe_summary(cache_root, tickers, _now) -> CacheSummary` iterating the supplied tickers, calling `newest_parquet_mtime` per dir, counting non-empty, returning global max-mtime. Bounded by ~30 stats on the 10-row mirror; pure-fn (no global state). `_now` parameter reserved for a future stale-band variant; current shape doesn't use it.
  - Promoted `newest_parquet_mtime` from `fn` to `pub fn` (clippy-checked `#[must_use]`).
  - Added `pub const ALL_YAHOO_TICKERS: &[&str]` (10 rows: BTC-USD..LINK-USD) mirroring the RHS of the `binance_to_yahoo_ticker_lookup` table.
  - Added 6 unit tests: 10-row sanity, empty dir, one-ticker, two-tickers max-mtime, empty slice, `CacheSummary::empty()` zero-state.
- [x] T-DU3.5 (2026-05-28) — Wired cached-summary on `LabState`:
  - Added `pub cache_summary: Option<CacheSummary>` field with `Default = None` (also defended in `Clone` and `with_selection`).
  - Added invalidation + immediate re-populate in `Message::LabSelectDataSource` arm of `crates/ui/src/state.rs` (calls `probe_summary` synchronously — bounded ~30 stats, well under R-NR.7 budget for a one-shot per toggle).
  - Same invalidation + re-populate in `Message::LabRunCompleted` arm (per architect's preferred "update-side populate" path so `view()` reads `&LabState` immutably; no `RefCell` needed). D-V0.1.2-1 cadence honored.
  - Cold-start: `cache_summary == None` → `view` constructs a transient `CacheSummary::empty()` for the badge (renders the `LAB_CACHE_STATE_EMPTY` label). First user-driven invalidation populates the cached field.
- [x] T-DU4 (2026-05-28) — Wired summary badge into `crates/ui/src/screens/lab.rs` **Lab tab toolbar** (Q2 lock):
  - New `Row` as the FIRST child of the Lab body `Column` (above the pair-chip row), right-aligned via `iced::widget::Space::new().width(Length::Fill)` leading spacer + `cache_state_summary_badge::view(...)` trailing. Rendered for every Lab activation regardless of `data_source` (R3.6); per-pair pill on source-toggle row unchanged.
  - Bumped `chart_canvas_height_for_body_with_training` to account for the new row: 11 children → 12 children, 10 gaps → 11 gaps, added one `CHIP_ROW_HEIGHT_PX` to `fixed`. Updated comments at lines 142-147. The existing `chart_canvas_height_grows_with_body_height` unit test still passes (delta arithmetic is body-proportional; fixed components cancel).
- [x] T-DU5 (2026-05-28) — Added 4 gallery cells: `cache_state_summary_badge__empty`, `__one_ticker`, `__two_tickers`, `__ten_tickers`. Registered widget name in `EXPECTED_WIDGETS` slice; bumped `GALLERY_LOGICAL_HEIGHT` 18_040 → 19_080 (4 × 260 px = 1 040 px). Comment block extended with v0.1.2 entry.
- [x] T-DU6 (2026-05-28) — Authored unit tests + snapshot tests:
  - Widget-internal: 6 unit tests (empty/one/ten format, count-without-mtime defensive path, UNIX-epoch ISO date helper, view-doesn't-panic across dark+light × 3 summaries, `"Yahoo "` prefix-disambiguator guard).
  - `crates/ui/tests/panel_snapshots.rs`: 4 new `cache_state_summary_badge__*` snapshots locking the textual summary + Lumen token names against drift. Stable 2024-12-31 UTC fixture mtime keeps the rendered ISO date deterministic. All 4 baselines accepted (`INSTA_UPDATE=always`).
- [x] T-DU7 (2026-05-28) — Layout smoke at 1280 / 1024 / 960 px (analytical — toolbar row is a right-aligned ~300 px badge in a `width(Fill)` Row): badge width at N=10 with full ISO date ≈ 282 px text + 16 px padding = ~298 px. At 960 px window with sidebar (~190 px) the Lab body is ~770 px; the badge fits comfortably in the trailing 300 px of the row. K4 mitigation (truncate to YY-MM-DD) is NOT needed at v0.1.2 — recorded for the dev-note as the resolved-at-design-time observation.
- [x] T-DU8 (2026-05-28) — `probe_summary` latency: bounded by 30 directory stats on warm APFS (~0.3-1 ms per the H4 forecast). The cached-summary cadence (D-V0.1.2-1) means probe runs only on `LabSelectDataSource` + `LabRunCompleted` — operator-event-driven, never per-frame. Inline benchmark omitted at v0.1.2 (the H4 forecast and the pre-existing single-ticker `probe` 1-ms ceiling are sufficient evidence); a synthetic Criterion bench is a v0.2.0 follow-up if K3 forecasts shift.
- [x] T-DU9 (2026-05-28) — Gates green: `cargo fmt --all --check` clean; `cargo clippy -p ui -- -D warnings` shows 0 NEW errors (9 pre-existing in `lab/{runner,trainer,training_log,progress}.rs`, `live.rs` × 2, `widgets/position_curve.rs` × 3 — within the brief's "pre-existing 9 OK" budget). Snapshot tests: 90/90 panel snapshots green; consistency tests 2/2 green; cockpit_training_pressed_wiring 5/5 green (regression check); ui --lib 411/411 (≥ 397 v0.1.1 baseline + 14 new tests). Anchors 70/70 (developer's M-DEV lane landed concurrently — eth-yahoo row 70 SHA = `e59a5f87…`).

## M-FINAL — Tester

- [x] T-F1 (2026-05-28) — `bash scripts/verify_anchors.sh` exit 0 with `ANCHORS PASS (70 / 70)`.
  - file: `spec/anchors.toml` (70 rows); `spec/lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/reports/` (ETH reports); `spec/lab-yahoo-realdata/reports/` (BTC anchor report)
  - test: `bash scripts/verify_anchors.sh`
  - output: `PASS eth-yahoo-2024-1d-sma-cross e59a5f87...` `ANCHORS PASS (70 / 70)` — verified independently.
- [x] T-F2 (2026-05-28) — `cargo fmt --all --check` clean.
  - test: `cargo fmt --all --check`
  - output: (no output — clean)
- [x] T-F3 (2026-05-28) — `cargo clippy --workspace -- -D warnings`: 9 pre-existing errors in `crates/ui` (within pre-existing 9 OK budget from brief). Zero NEW errors. `cargo clippy -p backtest --features yahoo -- -D warnings` clean.
  - test: `cargo clippy --workspace -- -D warnings` / `cargo clippy -p backtest --features yahoo -- -D warnings`
  - output: 9 pre-existing ui errors (progress.rs, trainer.rs, training_log.rs, runner.rs, live.rs, position_curve.rs) — no new errors introduced by this feature.
- [x] T-F4 (2026-05-28) — `cargo test --workspace --no-fail-fast` — all crates pass; 411 ui lib tests, 90 panel snapshots, 6 backtest integration tests green. No FAILED lines in output. workspace total > 1187.
  - test: `cargo test --workspace --no-fail-fast`
  - output: all `test result: ok.` — no failures.
- [x] T-F5 (2026-05-28) — Re-ran `run_yahoo_sma --ticker ETH-USD` twice independently (tester runs #4 and #5 counting dev's 3). SHA `e59a5f87...` on both runs — matches anchor row 70 exactly. H2 PASS (tester second+third witness, K2 gate: PASS).
  - test: `cargo run --release -p backtest --features yahoo --bin run_yahoo_sma -- --ticker ETH-USD --reports-dir /tmp/eth-tester-verify` × 2
  - output: `e59a5f87daf0cc58ce8be2e1695dfc2ccc3ab76bd976b54c957e9e3c5ed4199a` (both runs)
- [x] T-F6 (2026-05-28) — Re-ran `run_yahoo_sma` (no flag, BTC default). SHA `d2a709ef...` — matches dev's claim; does NOT match v0.1.1 anchor `8045623b...`. Root cause confirmed: REVISION.toml aggregate SHA changed from `7b33166e→e018f876` when ETH-USD data was fetched. This is a known transitional state (see test-final report § H3 analysis). The v0.1.1 anchored report file remains on disk and resolves via `verify_anchors.sh sort|tail-1` — 70/70 PASS confirms this. H3 code-purity PASS; body-SHA drift is an external REVISION.toml event pre-dating the code change.
  - test: `cargo run --release -p backtest --features yahoo --bin run_yahoo_sma -- --reports-dir /tmp/btc-tester-verify`
  - output: SHA `d2a709efc0e9a3b02999518d747b588cec7fe9641b535eda1546d76aa9d6d8f5` — matches dev's `d2a709ef...`; diverges from anchor 69 `8045623b...` as documented.
- [x] T-F7 (2026-05-28) — Gallery snapshot diff: `cargo test -p ui --test panel_snapshots` 90/90 green (86 pre-existing + 4 new `cache_state_summary_badge__*` cells). No regressions in existing cells.
  - test: `cargo test -p ui --test panel_snapshots`
  - output: `test result: ok. 90 passed; 0 failed`
- [x] T-F8 (2026-05-28) — `spec_lint.py` → `spec-lint: FAIL (73 violations in 3 categories)` — matches 73/3 baseline exactly. The `unreferenced-anchor (1)` category (seen during T-F8 pre-fix run) was resolved by converting trace.toml `anchors` field from prose string to TOML array `["eth-yahoo-2024-1d-sma-cross"]`. No new categories post-fix.
  - test: `/opt/homebrew/bin/python3.14 scripts/spec_lint.py`
  - output: `spec-lint: FAIL (73 violations in 3 categories)` — baseline match.
- [x] T-F9 (2026-05-28) — Report authored at `spec/lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/reports/test-final-2026-05-28-lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge.md`. VERDICT → SOFT-PASS.

## M-PRESENTER

- [ ] T-P1 — Author `presentations/lab-yahoo-realdata-v0.1.2-2026-05-XX.md` sprint-review deck.
- [ ] T-P2 — Live demo: re-run `run_yahoo_sma --ticker ETH-USD`; show 70/70 anchors PASS + ETH/BTC H1 divergence numbers.
- [ ] T-P3 — Operator approval block + handoff to operator.
