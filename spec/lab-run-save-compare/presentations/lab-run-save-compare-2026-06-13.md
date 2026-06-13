---
slug: lab-run-save-compare
kind: presentation
mode: release
date: 2026-06-13
presenter: presenter-agent
commit: e13cb6c
tester_verdict: PASS
---

# Lab run → save → compare — release review

_Operator deck. Read in under five minutes, then tick one box at the bottom._

## TL;DR

The cockpit **Lab** is now a real strategy-checking tool — run a strategy on
real on-disk Binance data, the result is **saved to disk** (in a git-ignored
`lab-runs/` cache), the equity curve **repaints from that saved file**, and
**Compare** diffs the KPIs of two saved runs — all proven by 712 passing tests
and **zero risk to the 119 locked regression baselines** (verified live, 119/119).

## What changed

- **Runs are now durable.** Before this feature, pressing Run in the Lab computed
  an equity curve in memory and threw it away the instant the cockpit closed or a
  second run started (the engine returned "no file written" even when asked to
  write). Now every Lab run writes a Markdown report **plus a companion equity CSV**
  to `./lab-runs/`, so the curve survives a restart and shows up in Compare.
- **Saved runs live OUTSIDE the regression-test namespace, by design.** The Lab
  writes to `lab-runs/` at the repo root — a directory the regression checker
  (`verify_anchors.sh`) literally never looks in. Ad-hoc operator runs can never
  shadow or corrupt the 119 locked "this is the known-good output" baselines.
  This was the riskiest design call and it is **provably safe**, not safe-by-luck
  (see "Why" and the anchor row in the verification matrix).
- **Compare diffs two saved runs' KPIs and repaints each run's real curve.** Pick
  two runs, see return / Sharpe / max-drawdown / trade count side by side, each
  with its own real equity curve. (One piece is render-proven but not yet wired
  into the screen — see "Open decisions".)

## Why

With live trading removed (your 2026-06-12 decision), the Lab is the project's
only place to ask "does this strategy work on real data?" — but it couldn't
**remember** an answer. The engine has been one step short of this since it was
built: it already computes the full result (equity curve, fills, KPIs) and has a
slot for the file path, but that slot was hardcoded to "nothing." This feature
fills it in. The single non-obvious decision was *where* the file goes: the
regression checker resolves each locked baseline to "the newest matching report
anywhere under `spec/`," so a saved run dropped under `spec/` could silently
become the file a baseline is checked against and break the gate. Putting saved
runs in a sibling `lab-runs/` directory (git-ignored, like the existing `data/`
cache) makes them invisible to that checker by construction — the safe choice is
also the cheapest one, because the existing loaders only needed a changed root
path, not a rewrite.

## What the operator can do now

1. **Run a strategy on real data and keep the result.** Open the cockpit's **Lab**
   screen, pick a data source / strategy / pair / date range, press **Run**. The
   run executes on the on-disk Binance data and persists to `./lab-runs/`.
   - The exact UI path that fires the engine is the Lab's **Run** button →
     `spawn_lab_run` → `backtest::engine::run_scenario(...)` (which now returns a
     real file path instead of `None`). It is triggered by clicking Run in the
     running cockpit; there is no headless "run + save" CLI subcommand today (see
     the honesty note below).
2. **Reopen a past run.** On the next cockpit boot, selecting the same
   strategy+pair+range repaints the equity curve from the saved file (cold load),
   not just from the in-memory mirror.
3. **Diff two runs in Compare.** Open the **Compare** screen to see two saved runs'
   KPI rows side by side, each with its real equity curve.

**Inspect / clear the saved runs (operator-local, never committed):**
```bash
# List saved Lab runs (one dir per strategy slug, reports + companion CSVs):
find lab-runs -type f -name 'backtest-*' | sort

# Clear them all (safe — git-ignored, regenerated on next Run; anchors unaffected):
rm -rf lab-runs/
```

## Live demo

I exercised the **real-data write path the Lab uses** end-to-end, on real Binance
BTCUSDT 2023 data, with the Lab's default seed (`0xC0FFEE`), writing into a
throwaway directory exactly as the Lab's `run_scenario(reports_dir=...)` does.
This is the same engine call the Lab Run button makes.

**Command** (real binary, real data — captured verbatim):
```bash
cargo run --release -p backtest --bin backtest -- \
  --scenario btc-2023-1m-sma-cross --seed 0xC0FFEE --reports-dir /tmp/lab-demo
```

**Captured stdout** (real, trimmed to the load-bearing lines):
```
INFO backtest: loading Parquet data path="data/binance/BTCUSDT/2023"
INFO backtest: Parquet bars loaded bars=17543
INFO backtest: data ready bars=17543 data=real (Binance Vision)
INFO sma_composed_run: backtest complete elapsed_s=0.0055 trades=441 final_equity=107381.95...
Report written: /tmp/lab-demo/backtest-20260613-062007-btc-2023-1m-sma-cross.md
Scenario     : btc-2023-1m-sma-cross
Bars         : 17543
Trades       : 441
Final equity : $107381.95 USDT
Data source  : real (Binance Vision)
Ledger imbal : 0
```

**The saved report it produced** (`## Summary` table, verbatim):
```
| Bars replayed        | 17544                     |
| Initial capital      | $100000.00 USDT           |
| Final equity         | $107381.95 USDT           |
| Total return         | 7.38%                     |
| Sharpe ratio (ann.)  | 7.7975                    |
| Max drawdown         | 4.20%                     |
| Trades               | 441                       |
| Data source          | real (Binance Vision)     |
```

**Determinism, demonstrated live (this is the heart of the feature).** I ran the
exact same command twice. The frontmatter-stripped report bodies were
**byte-identical** — same SHA-256 both times:
```
run 1 body SHA-256: 9b35c926cacd16c0e3b39129a2bff12cd761eac0fbc6c8574825bef4945bc147
run 2 body SHA-256: 9b35c926cacd16c0e3b39129a2bff12cd761eac0fbc6c8574825bef4945bc147
diff <stripped bodies> => BODIES BYTE-IDENTICAL ACROSS RUNS
```
That is the "same inputs ⇒ same saved bytes" property the headline test (H3)
proves at 21,601 points. (Honest note: this real-data body's hash is — correctly —
*different* from the locked `btc-2023-1m-sma-cross` baseline, because that
baseline is a **synthetic-data** oracle, 525,601 bars; my run is **real** Binance
data, 17,544 bars. Different inputs, different bytes — exactly as designed. The
locked baselines were not touched; see AC7 below.)

Raw capture saved at:
`spec/lab-run-save-compare/presentations/artifacts/lab-run-save-compare-2026-06-13/`
(`demo-run-stdout.txt`, `demo-backtest-body.md`).

### Demo recipe (so you can reproduce it yourself)

- **Command** — the real-data engine write path the Lab uses:
  ```bash
  cargo run --release -p backtest --bin backtest -- \
    --scenario btc-2023-1m-sma-cross --seed 0xC0FFEE --reports-dir /tmp/lab-demo
  ```
  To exercise the **clickable** Lab path instead, launch the cockpit and use the UI:
  ```bash
  cargo run -p ui --release --bin cockpit_live --features live   # real engine + Lab Run button
  # (fixtures-only, no engine: cargo run -p ui --bin cockpit)
  ```
- **Steps** — (CLI) run the command above; it loads real BTCUSDT 2023 Parquet,
  runs the SMA crossover, and writes a report under `/tmp/lab-demo/`. (UI) open
  **Lab**, pick a strategy + pair + range, press **Run**, watch the curve repaint
  from the persisted file; then open **Compare** and select two runs to diff KPIs.
- **Timing** — the CLI run is ~2–3 min the first time (cold compile), then the
  backtest itself is **sub-second** (real BTC 2023 = 17,543 bars, ~5 ms compute).
  The cockpit build is the usual UI compile (minutes cold, seconds warm).
- **Expected result** — `Report written: …/backtest-<stamp>-btc-2023-1m-sma-cross.md`
  with `Final equity : $107381.95 USDT`, `Trades : 441`, `Data source : real
  (Binance Vision)`. In the UI, a non-flat equity curve repaints in the Lab and
  both runs draw in Compare.
- **Failure-diagnosis** — if the CLI prints `loading Parquet data` then errors,
  the on-disk Binance data is missing (`ls data/binance/BTCUSDT/2023` should list
  Parquet); if final equity differs from `$107381.95`, the seed or data changed
  (the value is deterministic for `--seed 0xC0FFEE` on this dataset). If the
  cockpit Lab shows a flat 2-point curve, the companion equity CSV did not load —
  that is the exact bug Wave-2 fixed; check a `*-equity.csv` sits beside the `.md`.
- **Cleanup** — `rm -rf /tmp/lab-demo` (CLI) and `rm -rf lab-runs/` (UI runs).
  Both are throwaway; nothing is committed and the 119 locked baselines are
  untouched either way.

## Verification matrix

Each acceptance criterion (AC1–AC9) from the feature's `## Acceptance criteria`
section, with the proving test and one-line evidence. All from the tester report
`spec/lab-run-save-compare/reports/test-2026-06-12-lab-run-save-compare.md`
(VERDICT → PASS); the anchor and determinism lines I re-verified live today.

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| AC1 | `write_report=true` → file + path; `=false` → nothing | VERIFIED | `maybe_write_report_write_true/false_*` tests (backtest), green |
| AC2 | Saved body byte-identical for fixed inputs | VERIFIED | Re-confirmed live: same SHA `9b35c926…` across two real-data runs |
| AC3 | Real Binance data path runs | VERIFIED | Live demo: 17,543 real BTCUSDT 2023 bars → 441 trades, $107,381.95 |
| AC4 | Lab curve repaints from disk (cold load) | VERIFIED | `lab_curve_hydrated_from_lab_runs_report_renders` (render layer) |
| AC5 | Compare diffs two saved runs (KPIs) | VERIFIED | `compare_two_run_overlay_renders_both_series` + ui lib tests |
| AC6 | **H3: in-memory == saved-disk, every point** | VERIFIED | `h3_in_memory_equals_cached_disk` — **21,601 points, element-by-element** |
| AC7 | **Locked baselines stay 119/119 after a Lab write** | VERIFIED | Re-ran live today: **`ANCHORS PASS (119 / 119)`** |
| AC8 | Saved-run cache is bounded (keep last 20/tuple) | VERIFIED | `purge_old_lab_reports_keeps_last_n` — purges both `.md` + `.csv` |
| AC9 | Fixtures cockpit unchanged; I/O behind a seam | VERIFIED | `--features fixtures` 51/51 green; write via `reports_dir` override |
| — | Baseline-equity-divergence e2e gate | N/A | Eval tooling only — no strategy overlay / sizing math on the trading path; AC6/H3 is itself the "saved == computed" guarantee (justified in feature.md § Why) |

## Numbers that matter

- **Tests: 712 passed / 0 failed** (6 ignored). Breakdown: backtest **194**,
  ui lib **454**, fixtures **51**, render layer **12**, H3 gate **1**.
- **Headline gate (H3): 21,601 equity points** round-trip in-memory ⇄ saved-disk,
  **element-by-element** equal (timestamp + Decimal at every index). This test was
  *skipped* before this feature; it now runs and passes.
- **Locked regression baselines: 119 / 119 PASS** — verified live today, not just
  quoted from the report. No row added to `spec/anchors.toml`; no baseline mutated.
- **Real-data demo:** 17,543 BTCUSDT 2023 bars, 441 trades, final equity
  $107,381.95, deterministic body SHA `9b35c926…` (identical across two runs).
- **Spec-lint: 70 violations** (65 dead-link + 5 trace-broken-path) — pre-existing
  debt, **down 1** from the 71 baseline (`audit-2026-06-12`); **no new category,
  no count regression** attributable to this feature.
- **Perf budget:** _n/a — the hot path is disk I/O (report write + companion CSV),
  not a latency-sensitive compute path; no criterion suite is warranted (tester §6)._

## Open decisions

One decision, clearly scoped. **Everything in the shipped scope is done** — this
is about a deferred follow-on, not a defect.

- **The two-run equity OVERLAY (both curves on one chart) in Compare is
  render-proven but not yet screen-wired.** The widget that draws two curves on
  one chart is proven at the pixel layer (`compare_two_run_overlay_renders_both_series`
  passes — both series paint), but it is not yet hooked into the Compare screen's
  selection flow. Wiring it needs (a) a timestamped-series field on the Compare
  cache cell and (b) a two-run selection UX. **What you CAN do today:** compare two
  runs' KPI rows side by side, and see each run's own real equity curve repaint.
  **What's deferred:** seeing both curves superimposed on a single chart.

  - **Approve the shipped scope now** (run → save → compare-KPIs + per-run real
    curves), and I greenlight the overlay-on-one-chart as a small, well-scoped
    follow-on. **(Recommended — durable: ships the proven, complete chain today;
    the overlay is additive and low-risk.)** Cost of "yes": a future small
    follow-on feature for the overlay wiring.
  - **Hold the whole feature** until the overlay is screen-wired. Cost: delays a
    fully-working run→save→compare(KPIs + real curves) chain that is done and green,
    for one additive chart-overlay piece.

## Approval block

Tick exactly one. (I do not tick these — the decision is yours.)

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

**Notes / reason:**

_(operator writes here)_

## Feedback log

_(empty — no rejection yet)_
