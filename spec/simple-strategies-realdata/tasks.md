---
slug: simple-strategies-realdata
status: presenter-done
owner: architect
updated: 2026-06-13
---

# Tasks — simple-strategies-realdata

**Architect decomposition (2026-06-13, v0.2.0).** All open questions resolved in
[feature.md § Architecture](feature.md#architecture): Q1-policy = three-way toggle
(operator-settled); Q-anchor = UN-ANCHORED (operator-settled); Q-tf = hourly, NO
engine timeframe field (loader pins `Timeframe::OneHour`, engine consumes
`bars_override` cadence-agnostically); Q-loader = `preload_binance_bars` behind a
new `LabBinanceBarSource` trait via the existing `spawn_preload_on_rt`;
Q-feature = a `binance` cargo feature on `ui`; Q-miss = typed `Err` + re-fetch
hint, NEVER silent synthetic. **NO new ADR** — implementation under ADR-0040 +
ADR-0055 § D3. Estimate **S–M**, ≈ 45% exec / 55% UI.

**Parallelization point:** Wave 0 pins the two contract surfaces — the engine
`ScenarioDataSource::BinanceCache` variant (A1) and the UI
`LabDataSource::BinanceCache` toggle value + `"binance_cache"` serde (A4). Once
those two enum values land, **Wave A (exec) and Wave B (UI) run in parallel** —
neither blocks the other because the bars cross the boundary via the already-
existing `bars_override` field. Wave C (gates) runs after A ‖ B converge.

**Settled inputs the design MUST honor** (feature.md § Why):

1. **The Yahoo seam IS the mechanism — mirror it, do not reinvent.** Real-data-
   in-Lab already exists for Yahoo as a four-part seam: `LabDataSource` enum
   (`lab/state.rs:36`), `source_toggle` widget (`screens/lab.rs:237`),
   `preload_yahoo_bars` (`runner.rs:374`), and the engine threading
   `cfg.bars_override` verbatim into the four single-symbol arms
   (`engine.rs:1084/1161/1232/1306`). Binance bars ride the SAME `bars_override`
   field; the engine run logic does NOT change.
2. **Reuse the lab-run-save-compare chain verbatim.** `maybe_write_report` →
   `lab-runs/<slug>/reports/backtest-<ms-stamp>-<scenario>.md` (+ companion equity
   CSV) → two-root loaders → Compare KPIs + overlay. A Binance run produces a
   `RunReport` of the same shape, so persist/compare/overlay auto-apply (ADR-0055
   § A2). No new template, no new compare math.
3. **The Binance corpus is 1h, pinned `3a8b96c4…`, gitignored (ADR-0040).** The
   Lab Binance loader loads at `Timeframe::OneHour` (the CLI single-symbol 1m path
   at `main.rs:1298` does NOT fit). Assert the on-disk revision SHA on load.

**Project-law reminders (binding):**

- `Decimal` / `Money<Usdt>` for money, never `f64` (Sharpe/drawdown stay
  display-only `f64` per ADR-0003).
- **Anchor safety = 119/119 untouched.** Under Q-anchor = UN-ANCHORED (recommended),
  this feature does NOT touch `spec/anchors.toml`, commits no `spec/*/reports/`
  file, mutates no body-SHA. CLI/anchor paths never construct `BinanceCache`, so
  all 119 anchored bodies are byte-identical (the `YahooCache`-addition neutrality
  pattern, `lab-yahoo-realdata/decomp.md § T-AR9`). AC6 is a tripwire.
- **Baseline-equity-divergence gate** is N/A as written (no overlay / sizing),
  BUT its purpose-built analog — the **no-op-source guard** (AC4) — IS required:
  a Binance run's equity must diverge from the synthetic baseline for the same
  (strategy, symbol, range, seed), proving real bars reached the strategy.
- **Render-layer verification** for any Lab toggle/chip change (the
  `live_equity_render.rs` panel-snapshot pattern), not only the model layer
  (`feedback_verify_ui_at_render_layer`).
- **NO live trading** — real-data backtesting only (`project_no_live_trading`).
- **No worktrees / no branches** — work on `main`; orchestrator commits; sub-agents
  write files only (`feedback_no_worktrees`).

## M-DEV waves (architect-confirmed)

Wave 0 pins both contract enum values so A ‖ B parallelize. Wave A is exec
(`backtest` engine + the loader's data-layer core), Wave B is UI (toggle +
preload wiring + render proof) — they run **concurrently** after Wave 0. Wave C
is the gate sweep after both converge. **Every UI-touching task names its
render-layer verification.** ✦ = the no-op-source / anchor / determinism gates.

### Wave 0 — contract pins (BLOCKS A ‖ B; tiny, do first)

- [ ] **T0.1** — **Engine contract:** add `ScenarioDataSource::BinanceCache`
  (`crates/backtest/src/engine.rs:170-177`), `#[serde]` `"binance_cache"`,
  anchor-additive (the `YahooCache` precedent; CLI paths never construct it).
  Compile-only at this step (arms come in Wave A). — _acceptance: `cargo check -p
  backtest` green; the enum has the third variant._
- [ ] **T0.2** — **UI contract:** add `LabDataSource::BinanceCache`
  (`crates/ui/src/lab/state.rs:36-42`), serde `"binance_cache"`. Update the
  default-is-`Synthetic` + serde round-trip tests (`state.rs:~587-603`). —
  _acceptance: `"binance_cache"` serde round-trips; unknown string still falls
  back to `Synthetic`._

### Wave A — exec (backtest engine + Binance loader data-core) ‖ Wave B

- [ ] **T-A1** — Add the `BinanceCache => "binance"` arm to the `data_source_str`
  match in **all four** single-symbol arms (verified `engine.rs:1104-1107`,
  `1175-1178`, + rsi/bbands siblings). Non-exhaustive match → compile-enforced.
  `rev_sha` stays `None` on the engine path. (R1.) — _acceptance: AC1 — a
  `ScenarioConfig { data_source: BinanceCache, bars_override: Some(real bars),
  strategy: v0.sma }` returns a `RunReport` with non-empty equity + `data_source`
  string `"binance"`._
- [ ] **T-A2** — Change the four cross-sectional reject guards
  (`engine.rs:881/922/960/1016`) from `== YahooCache` to
  `matches!(.., YahooCache | BinanceCache)` → `RunError::UnsupportedDataSource`.
  (R1.) — _acceptance: AC2 — the four cross-sectional arms reject `BinanceCache`
  exactly as `YahooCache`._
- [ ] **T-A3** — **Loader data-core (single-symbol, hourly).** Implement the body
  of `preload_binance_bars` (the function lands in `runner.rs` under Wave B, but
  its data-layer mechanics are exec-owned): `data::revision::read_and_verify_revision_manifest("data/binance")`
  → assert pin `3a8b96c4…` (loud `Err` on `RevisionMismatch`/`RevisionMissing`);
  `data::ReplayFeed::new("data/binance", true).subscribe_bars(Symbol, Timeframe::OneHour)`
  → collect → clip to range → `(Vec<Bar>, revision_sha)`. **Timeframe pinned at
  the loader (A2): NO engine timeframe field.** **NEVER synthesize on miss** —
  typed `Err` + re-fetch hint (NO in-Lab auto-fetch; Binance is pinned per
  ADR-0040). (R2 / R6 / Q-tf / Q-miss.) — _acceptance: ✦ AC3 — loads non-empty
  hourly bars for `BTCUSDT × 2023`, asserts the on-disk revision SHA; a missing
  corpus returns the typed cache-miss error (re-fetch hint), NOT synthetic bars.
  Documented operator recipe since the corpus is gitignored._

### Wave B — UI (toggle + preload wiring + render proof) ‖ Wave A

- [ ] **T-B1** — **Three-way `source_toggle`** (`crates/ui/src/widgets/source_toggle.rs`
  — today two chips). Add the Binance chip; it dispatches the existing
  `Message::LabSelectDataSource(LabDataSource::BinanceCache)` (no message-shape
  change; the `state.rs:2488` / `runner.rs:1425` handler absorbs it). (R3.) —
  _acceptance + RENDER-LAYER VERIFICATION: AC7 — a panel-snapshot test
  (`live_equity_render.rs` `iced_test::screenshot` pattern) renders the toggle
  with **three** chips and the correct active-state highlight. Model-layer alone
  is NOT sufficient (project law `feedback_verify_ui_at_render_layer`)._
- [ ] **T-B2** — **Strategy gating** mirrors Yahoo: extend the
  `data_source == YahooCache` show-only-`SINGLE_SYMBOL_STRATEGIES` logic
  (`screens/lab.rs:97-115`, list at `:103`) to
  `matches!(.., YahooCache | BinanceCache)` — Binance hides/disables the
  cross-sectional chips. (R3.) — _acceptance + RENDER-LAYER VERIFICATION: a
  render-layer / view-state assertion that with Binance selected ONLY the four
  single-symbol strategy chips are shown (cross-sectional hidden), mirroring the
  Yahoo screenshot baseline._
- [ ] **T-B3** — **`preload_binance_bars` + `LabBinanceBarSource` trait**
  (`runner.rs`, mirror `preload_yahoo_bars` `:374` + `LabYahooBarSource` `:222`
  + `DefaultLabYahooBarSource` `:250`). Gate the function `#[cfg(feature =
  "binance")]`, the trait `feature = "live"`, the default impl
  `all(feature = "live", feature = "binance")`. **Route through the EXISTING
  `spawn_preload_on_rt`** — generalize it over a shared `LabBarSource`
  (super-trait or generic fn) so BOTH sources hit the one rt.spawn enforcement
  point (ADR-0050 § D4 — do NOT add a second inline `rt.spawn`). (R2 / Q-loader.)
  — _acceptance: AC8 — the trait seam lets a fake source be injected without the
  real corpus; the callthrough regression guard
  (`lab_runner_preload_callthrough_e2e.rs`) still passes for the generalized
  spawn point._
- [ ] **T-B4** — **`spawn_lab_run` wiring** (mirror `runner.rs:917-919`): on
  `LabDataSource::BinanceCache`, preload via `LabBinanceBarSource` (default or a
  new `binance_source_override` test seam), then set `scenario_cfg.bars_override
  = Some(bars)` + `scenario_cfg.data_source = ScenarioDataSource::BinanceCache`.
  Reuse `classify_preload_result`. (R2.) — _acceptance: a Binance run dispatches
  through `run_scenario` with bars injected; `data_source` round-trips to
  `BinanceCache`._
- [ ] **T-B5** — **`binance` cargo feature on `ui`** (`Cargo.toml`, sibling to
  `yahoo` `:238`): `binance = ["dep:data", "data/<binance-read-feature>"]` (NO
  `*-online` — Binance does not auto-fetch). Add the
  `#[cfg(not(feature = "binance"))]` friendly-error guard in `spawn_lab_run`
  (mirror the Yahoo guard `runner.rs:830-838`): selecting Binance without the
  feature returns "rebuild with `--features binance`", never a panic. Confirm
  with operator whether to add `binance` to `ui` `default` (currently
  `["live", "yahoo"]`) so the everyday cockpit ships it. (Q-feature.) —
  _acceptance: ✦ AC8 — fixtures cockpit (no `binance` feature) HIDES the Binance
  chip and is byte-identical to today; no `lab-runs/` Binance dir created without
  the feature; render-snapshot of the no-feature toggle shows two chips._

### Wave C — gates (after A ‖ B converge)

- [x] **T-C1 ✦** — **No-op-source divergence e2e (THE purpose-built gate).**
  Run `v0.sma × BTCUSDT × 2023` on Binance bars and on synthetic bars with the
  SAME `(strategy, symbol, range, seed)`; assert the equity curves **diverge by ≥
  epsilon** — proving real parquet bytes reached the strategy, not a silent
  synthetic fallback. Pattern: `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`.
  Gated `#[cfg(feature = "binance")]`. — _acceptance: AC4 — final-equity (or
  curve) delta ≥ epsilon; the test FAILS if the loader ever silently synthesizes._
  **TESTER VERIFIED 2026-06-13**: `binance_cache_real_bars_diverge_from_synthetic_baseline` PASS (epsilon = 1 USD);
  `binance_run_diverges_from_synthetic_baseline` PASS (delta assertion + series non-identical); `loader_missing_corpus_returns_typed_err_not_synthetic` PASS.
- [x] **T-C2** — **Persist + Compare round-trip for a Binance run (CLOSE-OUT
  AC).** Point the engine write at a `lab-runs/` tempdir (`reports_dir`), run a
  Binance single-symbol scenario with `write_report = true`, assert: (i) `.md` +
  companion equity CSV written; (ii) `EquityCache::get_or_load` parses the equity
  series element-by-element-equal to the in-memory series (H3 round-trip, holds
  by determinism); (iii) `compare::scan_spec_tree` builds a `CachedCell` with
  KPIs + a loadable overlay series. **NO new persist/compare code** — asserts the
  shipped ADR-0055 chain works for a Binance-sourced run. — _acceptance: AC5 — the
  full lab-run-save-compare chain round-trips a Binance run._
  **TESTER VERIFIED 2026-06-13**: `binance_run_persists_and_round_trips_through_compare` PASS.
- [x] **T-C3 ✦** — **Anchor tripwire (mandatory).** After a Binance Lab run
  writes to `lab-runs/`, `scripts/verify_anchors.sh` is still **119/119 PASS**.
  Explicit: no row added to `spec/anchors.toml`, no committed `spec/*/reports/`
  file, no anchored body-SHA mutated. (UI + data-source change only.) —
  _acceptance: AC6 — 119/119; `git status` shows no `spec/anchors.toml` or
  `spec/*/reports/` mutation._
  **TESTER VERIFIED 2026-06-13**: `scripts/verify_anchors.sh` → ANCHORS PASS (119 / 119).
- [x] **T-C4 ✦** — **Render-layer equity-curve proof.** A Binance-sourced run's
  equity curve actually rasterizes via the `live_equity_render.rs` ACCENT-pixel
  signal (curve paints ⟺ ACCENT-pixel-count > threshold) — closes the
  "wired but doesn't paint" gap for the Binance path. — _acceptance: AC7 — the
  rendered Binance curve draws a visible ACCENT polyline._
  **TESTER VERIFIED 2026-06-13**: `binance_sourced_equity_curve_rasterizes` PASS; `three_way_toggle_active_chip_marches_right` PASS; `binance_chip_renders_visible_highlight` PASS.
- [x] **T-C5** — **Validate sweep.** `cargo fmt`; `cargo clippy --workspace
  --all-targets -- -D warnings`; `cargo test -p backtest -p ui` (incl.
  `--features binance,live` for the new tests) green; `scripts/spec_lint.py` ≤ 70
  zero-new + `--self-test` PASS. **No `adr_registry_check.py` run needed — no ADR
  added.** — _acceptance: all gates green; determinism + `Decimal`/`Money<Usdt>`
  (no `f64` money) + NO-live-trading upheld._
  **TESTER VERIFIED 2026-06-13**: `cargo fmt --check` clean; `cargo clippy -p backtest -p ui` (production code) clean; `cargo test` all suites PASS; spec-lint: 70 violations, 0 new (baseline 71 — improved by 1).

## Notes

- **Baseline-equity-divergence gate (CLAUDE.md):** N/A as written — no strategy
  overlay, no sizing modifier, no new decision variable (the strategies are
  byte-unchanged; only the bars differ). Its purpose-built analog — the
  no-op-source divergence guard, **T-C1 / AC4** — IS mandatory and is the
  headline gate.
- **Two genuine close-out ACs** beyond the basics: **AC4** (real bars reach the
  strategy) and **AC5** (the persist/compare chain auto-applies to a Binance run).
- **No new dependencies** — `data::ReplayFeed` + `data::revision` already exist;
  the `binance` ui-feature re-exports the existing parquet read path.

## Appendix — proposed `[[req]]` row (for `spec/trace.toml`)

Per the orchestration contract the analyst owns `[[req]]` creation, but this
feature is **draft** and the task says append the row here rather than edit
`spec/trace.toml` directly. The orchestrator/architect promotes this into
`spec/trace.toml` via `spec-update` when the feature moves draft → proposed.
Architect fills `arch`; developer fills `crates` / `tests`; tester fills `anchors`.

```toml
[[req]]
id          = "REQ-SIMPLE-STRATEGIES-REALDATA-001"
title       = "Simple single-symbol strategies (v0.sma / v0.5.macd / v0.5.rsi / v0.5.bbands) run on real pinned Binance hourly data (revision 3a8b96c4…) in the cockpit Lab via a BinanceCache data-source toggle, mirroring the shipped Yahoo seam (LabDataSource enum + source_toggle + preload_*_bars + bars_override). Single-symbol only; cross-sectional arms reject BinanceCache (UnsupportedDataSource). UN-ANCHORED — lab-runs/-only, no anchors.toml row, no committed report (119/119 tripwire). Persist/compare/overlay auto-apply from lab-run-save-compare. No-op-source divergence guard (Binance vs synthetic equity diverges, same seed). Render-layer gate. NO live trading; Decimal/Money<Usdt> never f64; deterministic given fixed parquet + seed."
feature     = "simple-strategies-realdata"
product     = "spec/product.md"
arch        = []
crates      = []
tests       = []
anchors     = []
state       = "proposed"
```
