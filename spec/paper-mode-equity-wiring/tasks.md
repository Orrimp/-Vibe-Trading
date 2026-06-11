---
slug: paper-mode-equity-wiring
status: draft
owner: analyst
updated: 2026-06-11
version: 0.1.0
trace: REQ-LIVE-EQUITY-PAPER-001
---

# Tasks — paper-mode-equity-wiring

> Analyst draft stub. The architect refines these into M-DEV waves after
> resolving Q1–Q6 (the unify-vs-seam decision in Q1 sets the task shape).
> Exec-led (~90% `crates/agent`); the lone UI task is a render-test
> assertion. No new crate, no new dependency expected (the feed, exec
> engine, sizer, store, trait, and UI are all already in use).

## Provisional task list (architect to refine)

- **T1 — Structural decision + skeleton (Q1).** Pick unify (a, recommended) /
  seam (b) / reject duplicate (c). If (a): rename/generalize
  `spawn_research_trading_loop → spawn_trading_loop(feed, mode, store, …)`
  preserving research outputs exactly. Lands the signature + the research-arm
  call-site change (still passing the replay feed). (R1, R4)
- **T2 — Wire the paper arm to the trading loop against the live feed.** In the
  `Mode::Paper` branch (`runtime.rs:544`), call the (unified) loop with the
  Binance feed + `Some(store)`; remove the `drop(state_tx)` idle-reconciler
  stub (`runtime.rs:655-683`). (R1, R2, AC4)
- **T3 — Reconcile the periodic reconciler's role (Q2).** Retire its paper
  MINT role (loop becomes the single writer) OR keep it fed real `state_tx` +
  bar-paced. Remove the free-60 s-timer marking. (R2, R3, AC2)
- **T4 — Pin the per-bar persist seam (Q4).** Loop-direct `LiveEquityStore`
  write per bar, gated `mode != Research`, fire-and-forget (A6 unchanged); ONE
  writer. (R2, AC1, AC2)
- **T5 — Data-layer divergence + paper-fill + no-real-orders tests.** Paper-loop
  integration test against a faked moving feed: fills produced, per-bar
  `total_equity` non-constant, persisted rows non-constant, no live exec client.
  Extends `equity_store_integration.rs` AC1. (AC1, AC2, AC3, AC5)
- **T6 — Non-flat render gate (the UI task, R5/AC6).** Extend
  `crates/ui/tests/live_equity_render.rs`: `CURVE_Y_VAR_MIN` Y-variation
  assertion `(max_y - min_y) ≥ CURVE_Y_VAR_MIN` for a moving curve + a
  self-proving FLAT contrast that fails it (the flat-line guard passes
  `count`/`x_span`). Render-layer half of the divergence gate. (AC6)
- **T7 — Research byte-safety + anchor proof (R4/AC7).** Research/paced-replay
  tests green; 19 anchored backtest reports byte-unchanged; explicit
  anchor-count assertion. (AC7)
- **T8 — Fixtures smoke + no-feature build unchanged (AC8).**

## Wave hint (if Q1=(a) unify)

- **Wave 0:** T1 (skeleton/signature — unblocks everything).
- **Wave 1 (parallel):** developer T2–T5, T7 (exec) ‖ ui-designer T6 (render
  test). Reconverge at T8.

## `[[req]]` row for `spec/trace.toml` (analyst-authored; orchestrator to apply via `spec-update`)

> The analyst owns `[[req]]` row creation but this task is folder-scoped (no
> direct `trace.toml` edits). The row below is ready to insert; `arch` lists
> the analyst-draft files (architect appends the ADR), `crates`/`tests` are
> analyst seeds the developer/tester refine, `anchors` stays empty (additive —
> backtest anchors byte-unchanged by construction, AC7). State `proposed`.

```toml
[[req]]
id          = "REQ-LIVE-EQUITY-PAPER-001"
title       = "Paper-mode equity wiring — make paper-mode cockpit equity REAL (not a flat line at initial_capital_usdt). Root cause (presenter 2026-06-11): the Mode::Paper arm of runtime.rs connects a live Binance WS feed + an idle reconciler whose state_tx is DROPPED (runtime.rs:674), so equity = cash + 0*0 = initial_capital forever, and the shipped live-equity-history-durable (ADR-0052) faithfully persists/re-hydrates that flat line. NO strategy loop, order placement, or paper fills run in paper mode today (registry.on_bar is called only by spawn_research_trading_loop). Fix: run the existing per-bar equity pipeline (registry.on_bar -> risk::size_and_validate -> PaperEngine::step -> publish fills/positions + rich PnlSnapshot, total_equity = cash + base_qty*mark) against the LIVE feed in paper mode — recommended via UNIFYING spawn_research_trading_loop into one feed-parameterized spawn_trading_loop(feed, mode, store) for research AND paper (Q1=a; the loop body is already mode-agnostic + feed-parameterized, runtime.rs:946; behavior-preserving rename guarded by research tests + anchor gate). Remove drop(state_tx); ONE per-bar writer (loop-direct LiveEquityStore persist, gated mode != Research, fire-and-forget per A6); bar-stream cadence (mark to bar.close), not the free 60s timer. Research replay + 19 backtest anchors byte-unchanged (R4/AC7 — backtest never calls runtime::run). Settled law NOT reopened: two-timestamp contract (as_of wallclock / bar_ts data time), LiveEquityStore trait + A2 research-writes-nothing gate, Decimal/Money<Usdt> never f64, every external I/O behind a trait. CLAUDE.md baseline-equity-divergence gate INTENT APPLIES here (this adds a strategy/sizing/exec decision into paper mode, unlike the durable feature's genuine N/A) and is satisfied by AC6 (Y-variation render proof — a flat line PASSES the existing ACCENT count/x_span because equity_curve.rs centers a degenerate series as a full-width horizontal line; only the ACCENT bbox HEIGHT (max_y-min_y) >= CURVE_Y_VAR_MIN discriminates flat from moving) + AC1's data-layer non-constant-total_equity assertion. Effort ~M, ~90% exec (crates/agent runtime.rs/reconciler.rs); store/trait/migration/hydrate/captions/PnlSnapshot all shipped by ADR-0052; the lone UI change is the Y-variation render assertion."
feature     = "paper-mode-equity-wiring"
product     = "spec/product.md"
arch        = [
  # ANALYST (2026-06-11) — v0.1.0 brief, Q1-Q6 with recommended defaults.
  "spec/paper-mode-equity-wiring/feature.md",
  "spec/paper-mode-equity-wiring/tasks.md",
]
crates      = ["crates/agent"]   # developer refines; exec-led, ~90% runtime.rs/reconciler.rs
tests       = [
  # render-layer divergence gate (AC6) — Y-variation assertion turns "a curve drew" into "the curve moved".
  "crates/ui/tests/live_equity_render.rs",
  # data-layer divergence (AC1/AC2) — paper loop produces non-constant total_equity + one-row-per-bar.
  "crates/agent/tests/equity_store_integration.rs",
]
anchors     = []   # N/A — additive only; backtest anchors byte-unchanged by construction (AC7). Baseline-divergence gate INTENT satisfied by AC6 + AC1 data-layer assertion (NOT rubber-stamped N/A — Q6).
state       = "proposed"
```
