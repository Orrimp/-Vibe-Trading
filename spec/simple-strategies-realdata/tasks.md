---
slug: simple-strategies-realdata
status: draft
owner: analyst
updated: 2026-06-13
---

# Tasks — simple-strategies-realdata

**Analyst draft (2026-06-13).** This task list is provisional — the architect
owns the final decomposition once Q1 / Q1-policy / Q-anchor / Q-tf / Q-loader /
Q-feature / Q-miss are resolved (see
[feature.md § Open questions](feature.md#open-questions-for-the-architect)).
Estimate **S–M**, ≈ 45% exec (engine + Binance loader) / 55% UI.

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

## Provisional waves (architect to confirm / re-cut)

### Wave 0 — operator decision gate (BLOCKS everything)

- **T0.1** — Resolve **Q1-policy** with the operator: Binance + Yahoo both in the
  Lab toggle (default), vs Binance replaces Yahoo, vs leave Binance CLI-only
  (collapses the feature to the § Size-estimate fallback). This reverses a small
  part of the `lab-yahoo-realdata` 2026-05-24 decision — do not proceed on the full
  build without an explicit nod. (Architect surfaces; operator decides.)
- **T0.2** — Resolve **Q-anchor** (UN-ANCHORED recommended) and **Q-tf** (hourly).
  These pin whether the anchor gate is touched (it should not be) and the bar
  cadence.

### Wave A — exec (backtest engine + Binance loader) ‖ Wave B

- **T-A1** — Add `ScenarioDataSource::BinanceCache` to `engine.rs:170-177`
  (anchor-additive, `#[serde]`-compatible, the `YahooCache` pattern). Add the
  `"binance"` report-label match arm (`engine.rs:1104-1106` and the three sibling
  arms). (R1.)
- **T-A2** — Add the `BinanceCache` reject arm to the four cross-sectional arms
  (`engine.rs:881/922/960/1016`) → `RunError::UnsupportedDataSource`. (R1 / AC2.)
- **T-A3** — Single-symbol hourly Binance loader behind a `LabBinanceBarSource`
  trait seam (Q-loader): load `data/binance/<SYM>USDT/<YEAR>/<MM>.parquet` at
  `Timeframe::OneHour` via `data::ReplayFeed` (single-symbol read), clip to range,
  assert the pinned revision SHA, return `(Vec<Bar>, revision_sha)`. **No silent
  synthetic fallback** — typed `Err` on miss/shortfall. (R2 / R6 / AC3 / Q-miss.)
- **T-A4** — The `binance` cargo feature on the `ui` crate (sibling to `yahoo`);
  the loader + toggle option gated on it; fixtures cockpit byte-unchanged without
  it. (Q-feature / AC8.)

### Wave B — UI (toggle + preload wiring + render proof) ‖ Wave A

- **T-B1** — Extend `LabDataSource` (`lab/state.rs:36`) with `BinanceCache`
  (`"binance_cache"` serde); update the default-is-synthetic + round-trip tests
  (`state.rs:587-603`). (R3 / AC7.)
- **T-B2** — Three-way `source_toggle` (`screens/lab.rs:237`): Synthetic / Yahoo /
  Binance; `LabSelectDataSource` extends to the third variant; Binance shows the
  four single-symbol strategies, hides/disables cross-sectional (mirror Yahoo,
  `screens/lab.rs:97-115`). (R3.)
- **T-B3** — `preload_binance_bars` in `runner.rs` (mirror `preload_yahoo_bars`,
  `runner.rs:374`); `spawn_lab_run` sets `scenario_cfg.bars_override = Some(bars)`
  + `data_source = BinanceCache` (mirror `runner.rs:917-918`). (R2.)
- **T-B4** — Render-layer snapshot of the three-way toggle (the
  `live_equity_render.rs` pattern). (R3 / AC7 — project law.)

### Wave C — gates (after A ‖ B land)

- **T-C1** — AC4 no-op-source divergence e2e: `v0.sma × BTCUSDT × 2023` Binance vs
  synthetic, same seed ⇒ equity diverges ≥ epsilon. (THE purpose-built gate;
  pattern `vol_targeting_overlay_end_to_end.rs`.)
- **T-C2** — AC5 persist + Compare round-trip for a Binance run (write to
  `lab-runs/` tempdir → CSV + `.md` → `EquityCache` element-equal round-trip →
  `scan_spec_tree` `CachedCell` with KPIs + overlay). Asserts the shipped chain;
  no new persist/compare code.
- **T-C3** — AC6 anchor tripwire: `verify_anchors.sh` 119/119 after a Binance Lab
  write; no `anchors.toml` row, no committed report.
- **T-C4** — `spec_lint.py` ≤ 70 zero-new + `--self-test` PASS; `cargo fmt` +
  `clippy -D warnings`; render + e2e green.

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
