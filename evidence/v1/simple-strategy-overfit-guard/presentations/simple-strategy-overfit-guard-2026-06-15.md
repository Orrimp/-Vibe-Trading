---
slug: simple-strategy-overfit-guard
mode: release
date: 2026-06-15
agent: presenter
status: approved-shipped
tester_report: spec/simple-strategy-overfit-guard/reports/test-2026-06-15-1200-simple-strategy-overfit-guard.md
findings_note: docs/dev-notes/analysis-2026-06-15-simple-strategy-overfit-guard.md
---

# Operator deck — simple-strategy-overfit-guard — 2026-06-15

## TL;DR

The "trend-following protects you in a crash" hedge we spotted last week was
**one lucky price ordering, not a real strategy property** — so "ship passive"
is now the clean, unqualified recommendation.

## What changed

- **A claim was tested and overturned.** Last week's real-data survey found that
  in the only 2 losing markets (AVAX 2024, DOT 2024), trend-following strategies
  (SMA / MACD) made money while just-holding lost. This deck reports the
  stress-test of that claim — and it **fails the test**.
- **The test method:** re-shuffle each market's hourly price history 500 ways
  ("block-bootstrap"), re-run each strategy on every shuffle, and look at the
  *whole spread* of outcomes instead of the single way history happened to play
  out. All 9 strategy/market combinations score **FRAGILE** under the project's
  pre-frozen pass/fail rule.
- **Two spec files were revised to match:** the survey's "Finding 1" and the
  passive-baseline runbook now carry a one-line "this didn't hold up" pointer.
  Nothing in production code or trading behaviour changes — this is an analysis
  finding only.

## Why

The survey honestly flagged its own down-market hedge as "suggestive, not
conclusive — two data points." Before that nuance could leak into any decision
to go active in bear markets, we pre-registered a single yes/no test: is the
hedge a repeatable property of the strategy, or an artifact of the one exact
2024 AVAX/DOT price sequence? The answer matters because a "yes" would have
opened a trend-following product line, and a "no" closes the active-strategy
down-market story cleanly. The test says **no**: the strategies' *typical*
(median) shuffle is still profitable, but their *bad-luck* shuffle (worst 5%)
goes negative — which is exactly the definition of path-fragile under the rule
we committed to in advance. A hedge you can only count on in the lucky case is
not a hedge.

## What the operator can do now

This is an analysis finding, not a feature with new buttons. The action enabled
is **a documentation decision** plus an optional **independent re-run** to
confirm the numbers for yourself.

1. **Re-run the stress test yourself (optional, ~80 s).** Confirms the table
   below on your own machine, byte-for-byte.
   ```bash
   cargo test -p backtest --test realdata_simple_strategy_overfit_guard \
       --release -- --ignored --nocapture
   ```
   For a long run, watch progress with:
   ```bash
   cargo test -p backtest --test realdata_simple_strategy_overfit_guard --release \
       -- --ignored --nocapture > /tmp/og-run.log 2>&1 &
   watch -n 10 'tail -20 /tmp/og-run.log'
   ```
   Expected: 9 rows, every one ending `**FRAGILE**`, finishing in ~78 s.

2. **Read the one-page finding** (the full reasoning + scope cap):
   ```bash
   open docs/dev-notes/analysis-2026-06-15-simple-strategy-overfit-guard.md
   ```

3. **Approve / reject the finding + the runbook revision** — see the Approval
   block at the bottom.

## Live demo

Real captured stdout from the tester's release run (commit `3d843fa`). This is
the actual ensemble summary the harness prints — N=500 resampled paths per row,
scored against the frozen § 0 rule. Raw capture saved at
`spec/simple-strategy-overfit-guard/presentations/artifacts/simple-strategy-overfit-guard-2026-06-15/ensemble-run-A.txt`.

```text
## Simple-strategy overfit / robustness guard — block-bootstrap N=500
## Frozen § 0 rule: FRAGILE if sharpe.p5<0 OR prob_loss>0.35 OR dd_p95>0.70
##                  ROBUST  if sharpe.p5≥0.5 AND prob_loss≤0.15 AND dd_p95≤0.50
##                  MARGINAL otherwise. Composite = worst band.

| Cell | Strategy | N | sharpe p5/p25/p50/p75/p95 | prob_loss | P(sharpe>0) | dd_p50 | dd_p95 | VERDICT |
| AVAX·2024 (down) | SMA 20/50 | 500 | -0.810/0.020/0.570/1.119/1.909 | 0.248 | 0.752 | 0.055 | 0.100 | **FRAGILE** |
| AVAX·2024 (down) | MACD      | 500 | -0.475/0.252/0.895/1.369/2.146 | 0.160 | 0.840 | 0.027 | 0.048 | **FRAGILE** |
| AVAX·2024 (down) | RSI       | 500 | -0.788/-0.252/0.189/0.674/1.612 | 0.396 | 0.604 | 0.026 | 0.047 | **FRAGILE** |
| AVAX·2024 (down) | BBands    | 500 | -1.217/-0.603/-0.175/0.246/0.909 | 0.594 | 0.406 | 0.025 | 0.046 | **FRAGILE** |
| DOT·2024  (down) | SMA 20/50 | 500 | -0.910/0.017/0.653/1.354/2.310 | 0.248 | 0.752 | 0.053 | 0.097 | **FRAGILE** |
| DOT·2024  (down) | MACD      | 500 | -1.915/-0.896/-0.230/0.429/1.271 | 0.598 | 0.402 | 0.047 | 0.080 | **FRAGILE** |
| DOT·2024  (down) | RSI       | 500 | -0.308/0.185/0.640/1.114/1.986 | 0.152 | 0.848 | 0.020 | 0.036 | **FRAGILE** |
| DOT·2024  (down) | BBands    | 500 | -2.263/-1.372/-0.837/-0.393/0.304 | 0.886 | 0.114 | 0.033 | 0.060 | **FRAGILE** |
| AVAX·2023 (up-market control) | SMA 20/50 | 500 | -0.137/1.005/1.651/2.305/3.175 | 0.062 | 0.938 | 0.043 | 0.073 | **FRAGILE** |

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 77.82s
```

**How to read one row.** Take AVAX·2024 SMA: across 500 reshuffled versions of
2024's AVAX price history, the *median* shuffle earns a Sharpe of **+0.570**
(genuinely good — matches the survey's one-path +5.0%). But the *worst 5%* of
shuffles (the "p5" number) come in at **-0.810**, and **1 in 4 shuffles
(prob_loss 0.248)** end below where they started. The rule says: if the
worst-5% Sharpe is below zero, the result is FRAGILE — you got lucky with the
real ordering. Every down-market row trips that wire.

**Determinism proof (live).** Two independent release runs were diffed; the only
differences are compile/wall-clock timing — every reported statistic is
byte-identical:

```text
$ diff /tmp/og-A.log /tmp/og-B.log
1c1
<     Finished `release` profile [optimized] target(s) in 0.70s
---
>     Finished `release` profile [optimized] target(s) in 0.49s
51c51
< test result: ok. ... finished in 77.82s
---
> test result: ok. ... finished in 78.31s
```

## Verification matrix

The feature pre-registers acceptance criteria (AC-OG.1..6) rather than a `## Verification`
V1..Vn block; the matrix below is built from those criteria, each tied to the
tester's PASS evidence (commit `3d843fa`).

| # | Criterion | Status | Evidence (one line) |
|---|---|---|---|
| AC-OG.1 | Harness prints full distribution + § 0 verdict per ensemble (with corpus) | VERIFIED | Live-demo stdout above — 9 rows, each with p5..p95 / prob_loss / dd / VERDICT. |
| AC-OG.2 | Skips cleanly without the corpus; never in default suite | VERIFIED | Tester: `cargo test -p backtest` → 82 pass, 5 ignored; new harness correctly `#[ignore]`d. |
| AC-OG.3 | Two runs byte-identical (determinism) | VERIFIED | Live `diff` above — only compile/wall-clock timing differs; all stats identical. |
| AC-OG.4 | Negative control: RSI/BBands score FRAGILE, not ROBUST (test isn't rubber-stamping) | VERIFIED | RSI p5 -0.788 / -0.308; BBands p5 -1.217 / -2.263; BBands DOT prob_loss 0.886. All FRAGILE. |
| AC-OG.5 | Headline answered in writing (findings dev-note, with actual numbers) | VERIFIED | `docs/dev-notes/analysis-2026-06-15-simple-strategy-overfit-guard.md` — path-fragile, scoped per-symbol-year. |
| AC-OG.6 | spec-lint ≤ 70 zero-new; clippy clean; no `.unwrap()` outside tests | VERIFIED | Tester: clippy `--tests -p backtest` 0 warnings; spec-lint 70 (baseline, 0 new). |
| — | UN-ANCHORED by design (no `anchors.toml` row) | VERIFIED | `grep -c overfit-guard spec/anchors.toml` → 0; anchors total unchanged at 119. |
| — | Up-market control behaves as expected (sanity, not a defect) | VERIFIED | AVAX·2023 SMA p5 = **-0.137** exactly as feature §2 anticipated; healthiest cell (prob_loss 0.062). |

## Numbers that matter

| Metric | Value |
|---|---|
| Ensembles scored | 9 (4 strategies × 2 down-market cells + 1 up-market control) |
| Paths per ensemble | 500 (frozen N — § 0 bands are calibrated at N=500) |
| Verdict | **9 / 9 FRAGILE** under the frozen § 0 rule |
| Down-market trend-following medians (the part that IS profitable) | AVAX·2024 SMA p50 **+0.570**, DOT·2024 SMA p50 **+0.653** |
| Down-market trend-following bad-luck tail (the decision variable) | AVAX·2024 SMA p5 **-0.810**, DOT·2024 SMA p5 **-0.910** |
| Loss probability, SMA down-market | **0.248** on both AVAX·2024 and DOT·2024 (≈1 in 4 shuffles lose money) |
| Default test suite | 82 passed / 0 failed / 5 ignored (`cargo test -p backtest`) |
| Harness runtime | ~78 s per release run (×2 for the determinism check) |
| Anchors added | **0** (un-anchored `#[ignore]` harness; anchors.toml stays at 119) |
| spec-lint | 70 findings, **0 new** (baseline was 71 at the 2026-06-12 audit) |

## Open decisions

**One decision, and it is binary:** approve this analysis finding and the
two spec revisions it drove, or route it back.

- **The substantive call is already made by the data, not by you:** the
  pre-registered rule said "p5 Sharpe < 0 → FRAGILE," and every down-market cell
  is below zero. There is no parameter to tune, no threshold to argue — the rule
  was frozen *before* the run (`robustness-decision-rule-2026-05-30.md` § 0).
- **What approving commits you to:** the passive-baseline runbook's "but
  trend-following is a defensible down-market hedge" line is downgraded to
  "didn't survive path-resampling — don't quote it as a reason to go active."
  No code, no anchors, no trading behaviour changes. Cost of "yes" is zero
  engineering.
- **Honest scope cap (do not over-read this):** this is a **negative result for
  AVAX-2024 and DOT-2024 specifically** — 2 symbol-years, hourly bars, default
  parameters. It does **NOT** prove "trend-following can't hedge bear markets in
  general." A real bear-market universe test is a *future* re-run: the
  `binance-corpus-expansion` lane just landed the pinned 2021-22 bear-market data
  (commit `4f99b35`) that would let that happen — but that is a v0.2.0 follow-on,
  not part of this ship.
- **If you reject:** the most likely reason would be wanting the wider 2021-22
  bear-market re-run *before* revising the runbook. Say so in the reject line and
  it routes back to the analyst to re-scope.

## Approval block

- [x] Approved — ship — operator, 2026-06-15
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

_Notes / reason:_

## Feedback log

_None yet._

---

## Closing verdict (presenter)

Mechanical gates run after writing this deck:

```text
PRESENTATION CHECK PASS  (spec/simple-strategy-overfit-guard/presentations/simple-strategy-overfit-guard-2026-06-15.md — approval block UN-ticked)
spec-lint: FAIL (70 violations in 2 categories)
```

The `spec-lint` line reads FAIL, but that is the **established pre-existing
baseline** (70 = 65 dead-link + 5 trace-broken-path), unchanged by this deck and
**1 below** the 71-violation 2026-06-12 audit baseline. Zero new findings — the
gate is "≤ 70, zero new" (R-OG.10) and that holds. No structural regression
introduced since the tester's PASS.

> **Note — one frontmatter edit was required to hold the baseline.** Landing
> this deck made `spec_lint.py`'s `status-drift` hook fire (deck + PASS report
> present, but `feature.md` status still `draft`), pushing the count to 71. That
> hook is the audit-2026-06-12 enforcement that the presenter must advance the
> feature status when the deck lands, so the sanctioned fix was applied:
> `feature.md` frontmatter `status: draft → presenter-done`. Re-run confirms 70,
> byte-identical findings to baseline. (`feature.md` is not anchored, not a
> `spec/*/reports/` file, not `trace.toml` — the constraint-protected files were
> not touched.)

**Intended trace change** (orchestrator to apply atomically with committing this
deck — presenter does NOT edit `spec/trace.toml`): row
`REQ-SIMPLE-STRATEGY-OVERFIT-GUARD-001` `state` field `tester-done → presenter-done`.
