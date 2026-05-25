---
slug: lab-polish-round-2
version: 0.1.0
status: proposed
owner: analyst
updated: 2026-05-25
parent: lab-end-to-end-v2
---

# Lab polish round 2 — position curve + param tuning + UI density

## Why now

`lab-end-to-end-v2` v0.1.0 (shipped 2026-05-25) closed the Lab Run flow:
markers + Stop + progress + cache badge + error visibility + per-pair
filter + Yahoo auto-fetch. The Lab is functional but minimal — the
operator can run a backtest and see the result, but cannot:

1. **See open-position size over time** for the active pair (just trades
   + equity). Cross-sectional runs especially: the operator picks
   BTCUSDT inside a top-10 momentum run and sees BTC's fills + bars,
   but cannot tell if BTC was OVER or UNDER the portfolio average
   exposure at any moment in time.
2. **Tune strategy parameters** without editing TOML files. SMA fast=20
   slow=50 is hardcoded in `sma_composed_run.rs:307-308`; same for MACD/
   RSI/BBands inside their respective TOMLs under `config/strategies/`.
   The operator wants to A/B test `sma_crossover(fast=10, slow=30)`
   vs `(20, 50)` without round-tripping through a text editor.
3. **Scan strategy results at a glance** — the current KPI strip is
   useful but doesn't surface buy/sell counts, hold duration, or
   biggest drawdown in one place. Operator typically compares "ran
   yesterday with params X" vs "ran today with params Y" via memory.

## Scope (v0.1.0)

### R1 — Position-curve overlay

A polyline (or stepped histogram) below the price-line on the chart
canvas showing the operator-selected pair's **base-asset position
quantity over time**, derived from the run's fills.

- For single-symbol strategies: position = running cumulative qty of
  Buy − Sell, anchored to bar `close_ts`.
- For cross-sectional strategies: position FOR THE SELECTED PAIR ONLY
  (already filtered by D-2.5 in lab.rs); cross-sectional runs may have
  10 simultaneous positions — only show the active one.

**Implementation:**
- Add `position_curve: Vec<(i64, Decimal)>` to `SmaComposedRunResult`
  + each cross-sectional scenario's result. Compute in the existing
  bar loop (negligible cost).
- Surface via `RunReport.position_curve`, `RunSummary.position_curve`,
  `RunReportMirror.position_curve` (mirror previous bars/fills pipe).
- Filter by `active_symbol` in `lab.rs` (same pattern as D-2.5).
- Add a `position_curve` widget under `crates/ui/src/widgets/` —
  Lumen-token'd, mirrors `volume_histogram` shape. Renders as a
  stepped polyline below the price line.

**Anchor-additive:** in-memory only, doesn't touch Markdown reports.
Should be 34/34 byte-identical.

### R2 — Strategy parameter editor

A two-input row (fast + slow numeric steppers) that appears when the
operator picks `v0.sma`. Defaults to `(20, 50)`. Persists across
re-runs but NOT across cockpit restarts (Phase A keep-it-simple).

**Implementation:**
- Add `LabState.sma_fast_len: Option<usize>` and
  `sma_slow_len: Option<usize>`. Default `None` → fall through to
  `sma_composed_run::run` hardcoded `(20, 50)` (anchor preservation).
- Add the same fields to `LabRunConfig`. `cockpit_live.rs` propagates
  state → config.
- Add fields to `SmaComposedRunInput`. `sma_composed_run::run` uses
  `input.sma_fast_len.unwrap_or(20)` etc.
- New widget `crates/ui/src/widgets/param_stepper.rs`. Lumen mini-card
  with `−` / value / `+` buttons. Mirrors `pair_chip` token use.
- Lab screen renders the row only when `strategy.0 == "v0.sma"` AND
  the selected strategy is single-symbol. Other strategies' TOML-
  loaded params stay file-bound (v0.1.1 follow-up extends to MACD/
  RSI/BBands via a more abstract "ParamSheet" widget per ADR-0030).

**Anchor-additive:** as long as the CLI scenario path passes
`None`/defaults, anchors stay byte-identical. Lab-side override is
a new code path that doesn't touch existing anchored CLI paths.

### R3 — KPI strip densification

Today the `kpi_strip` shows Final / Initial / Max DD / Trade count.
Add: Buys, Sells, Net Δ vs initial, Sharpe (if computable), Avg hold
duration.

**Implementation:**
- Extend `BacktestKpis` with the new fields (additive — defaults zero
  for backward compat). Compute in scenarios that already have the data
  (all single-symbol arms; cross-sectional via existing fills + position
  tracking).
- Extend `kpi_strip::view_for_lab` to render 2-row layout with the new
  numbers.
- 4 new strings under `crates/ui/src/strings.rs`.

**Anchor-additive risk:** BacktestKpis is rendered into the Markdown
report body at `report/sma.rs:145+`. Adding fields to its `Display`
output would mutate the body. **Mitigation:** render the new fields
ONLY in `kpi_strip::view_for_lab` (UI-side), not in the report writer.
Anchors stay byte-identical because the Markdown body's KPI section
doesn't change.

## R-NR (non-requirements / out of scope)

- Multi-tab Compare view (already shipped in Phase E at
  `crates/ui/src/screens/compare.rs`).
- TOML editor for cross-sectional strategy params (deferred —
  needs a structured ParamSheet that respects strategy hash for
  anchor identity).
- Persistent param state across cockpit restarts (Phase B —
  `lab/persistence.rs` extension; deferred).
- Position-curve aggregation across the top-N universe (D-2.5
  scope-creep; v0.2.0 if requested).

## H (hypotheses)

- **H1** — Position-curve overlay improves cross-sectional run
  interpretability for the operator's currently-selected pair.
- **H2** — SMA param editor reduces the round-trip time for A/B
  testing strategy parameters by 5-10×.
- **H3** — KPI densification surfaces enough decision-relevant
  numbers that the operator doesn't need to open the Markdown report
  to compare two runs (operator-flagged at the 2026-05-25 verification
  walk).

## K (known-risks / decisions)

- **K1** — Position-curve sign convention: positive = long, zero =
  flat. No short positions in v0 (FixedFractionSizer rejects). Confirm
  with analyst at M-T1.
- **K2** — Param editor scope: only `v0.sma` in this pass. MACD/RSI/
  BBands have multi-parameter configs (signal periods, thresholds)
  that need a richer UI; deferred.
- **K3** — KPI extension MUST NOT touch Markdown report body. Use the
  R3 mitigation above; verify with `scripts/verify_anchors.sh` after
  every wave.

## Q (operator-decide)

1. **Position-curve representation** — (a) polyline (smooth) /
   (b) stepped (visually clearer for discrete fills) / (c) histogram
   (volume-style). Analyst default: (b) stepped.
2. **Param editor entry point** — (a) always visible / (b) only when
   `v0.sma` picked / (c) collapsed-by-default toggle. Analyst default: (b).
3. **KPI strip layout** — (a) 2-row 4-column grid /
   (b) horizontal scrollable strip / (c) dropdown to add/hide KPIs.
   Analyst default: (a).

## Trace

- REQ row to be appended to `spec/trace.toml` once architect M-T1
  lands the decomp.

## Wave plan (sketch — architect M-T1 will firm up)

- Wave A — analyst (this brief, R1-R3 + Q1-Q3).
- Wave B — operator-decide Q1-Q3.
- Wave C — architect M-T1: decomp.md with T-AR-1..T-AR-N + ADR if
  needed (likely none — extensions are additive).
- Wave D — developer (parallel-allowed):
  - D-1 — position_curve aggregation + widget (R1).
  - D-2 — param_stepper widget + LabState/Config plumbing (R2).
  - D-3 — KPI strip densification (R3).
- Wave E — tester M-FINAL: workspace gates, anchor 34/34, snapshot
  baselines for the new widgets.
- Wave F — presenter sprint review.

Estimated 3-5 days wall-clock.
