---
slug: simple-strategy-overfit-guard
status: findings
owner: analyst
updated: 2026-06-15
---

# Simple-strategy overfit / robustness guard — the down-market trend-following hedge is PATH-FRAGILE — 2026-06-15

> **Headline.** The 2-case down-market trend-following "hedge" surfaced by the
> 2026-06-14 real-data survey ([`realdata-simple-strategy-survey-2026-06-13.md`](realdata-simple-strategy-survey-2026-06-13.md),
> Finding 1 — `[[realdata-simple-strategy-survey-2026-06-13]]`) does **NOT survive
> path-resampling.** A block-bootstrap Monte-Carlo of N=500 stationary-resampled
> paths, scored against the frozen § 0 decision rule, lands **all 9 cells FRAGILE**
> (every cell's p5 Sharpe < 0). SMA/MACD have a positive *median*-path Sharpe on the
> down-market cells (AVAX·2024 SMA p50 **+0.570**, DOT·2024 SMA p50 **+0.653**), but
> their p5 left tails dip negative (**-0.810**, **-0.910**). So the survey's
> +5.0% / +6.4% down-market protection was sensitive to the **one specific 2024 bar
> ordering** — it is **not path-robust.** This **REVISES** the survey's Finding 1
> "trend-following is a defensible downside hedge" qualifier downward to *path-fragile
> on this evidence*.

This dev-note records the confirmed numbers from the tester's PASS
([`test-2026-06-15-1200-simple-strategy-overfit-guard.md`](../../evidence/v1/simple-strategy-overfit-guard/reports/test-2026-06-15-1200-simple-strategy-overfit-guard.md),
verdict PASS, commit `3d843fa`) and closes the loop on feature
[`simple-strategy-overfit-guard`](../v1/simple-strategy-overfit-guard/feature.md) (AC-OG.5,
task T-OG.13). It is the `findings`-status companion to the un-anchored `#[ignore]`
harness — there is no anchored `spec/*/reports/backtest-*.md` (UN-ANCHORED per feature
§ 3 / D-OG.4).

---

## 1. The question this answers

The survey ran four shipped-default simple strategies (SMA 20/50, textbook MACD,
RSI, BBands) on the real Binance hourly corpus, net of 4 bps taker cost, vs
buy-and-hold. In **18 of 20** (symbol·year) cells buy-and-hold was positive and
crushed every active strategy. In the **2 of 20** cells where buy-and-hold LOST
money, trend-following appeared to protect capital:

- **AVAX·2024:** B&H **-8.2%** → SMA **+5.0%**, MACD **+6.1%**.
- **DOT·2024:** B&H **-19.6%** → SMA **+6.4%** (MACD +0.2%, RSI +1.6%).

The survey flagged this honestly as *"suggestive of the trend-following protection
property, not statistically conclusive"* (2 data points). This feature pre-registered
a single load-bearing test: **is that down-market hedge a real, repeatable property of
the strategy, or an artifact of the one exact 2024 AVAX/DOT price ordering (one lucky
path)?**

Because the survey ran **untuned shipped defaults** (no parameter search), the threat
model is **small-sample path fragility**, NOT parameter-overfitting. That is why the
method is block-bootstrap-on-real-data, not CPCV/PBO (no IS/OOS selection step exists)
and not Deflated Sharpe (no max-over-trials selection). See feature § 0 / D-OG.2 for
the full method-selection justification.

## 2. Method (confirmed)

| Knob | Value | Source |
|---|---|---|
| Generator | `BlockBootstrapPathGen`, single-symbol mode (1-entry universe) | C1, `crates/data/src/synth/bootstrap.rs:71` |
| Paths per ensemble | **N = 500** | C1/Q-RH-1 ratified default — the § 0 bands are calibrated at N=500 |
| Block length | `BlockLengthPolicy::Auto` (Politis–White) | AVAX series 200 bars; DOT 204; AVAX·2023 218 — all > 1, no i.i.d. degeneration |
| Seeds | ADR-0051 D1: `path_seed_j = ensemble_seed.wrapping_add(j·0x9E3779B9)`, constant fill_seed `0xC0FFEE` | determinism, byte-reproducible |
| Data | `data/binance/` hourly Parquet (BinanceCache) | survey's loader verbatim |
| Decision rule | **Frozen § 0:** FRAGILE if `sharpe.p5 < 0` OR `prob_loss > 0.35` OR `dd_p95 > 0.70`; ROBUST iff `sharpe.p5 ≥ 0.5` AND `prob_loss ≤ 0.15` AND `dd_p95 ≤ 0.50`; else MARGINAL. Composite = worst band. | [`robustness-decision-rule-2026-05-30.md`](robustness-decision-rule-2026-05-30.md) § 0 — applied AS-IS, not re-derived |
| Harness | `crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs` (`#[ignore]`, UN-ANCHORED) | feature § 3 / D-OG.4 |

**Determinism confirmed** (AC-OG.3): two consecutive `--release --ignored` runs produced
byte-identical summaries (~78 s each). **Negative control confirmed** (AC-OG.4): the
no-edge mean-reverters RSI/BBands score FRAGILE on both down-market cells — the harness
discriminates and is not blessing no-edge churn.

## 3. The full ensemble table (confirmed — from the tester report)

N=500 per cell. Numbers copied verbatim from
[`test-2026-06-15-1200-simple-strategy-overfit-guard.md`](../../evidence/v1/simple-strategy-overfit-guard/reports/test-2026-06-15-1200-simple-strategy-overfit-guard.md)
§ 5 (Run A, byte-identical to Run B).

| Cell | Strategy | sharpe p5 / p25 / p50 / p75 / p95 | prob_loss | P(sharpe>0) | dd_p50 | dd_p95 | VERDICT |
|---|---|---|---|---|---|---|---|
| AVAX·2024 (down) | SMA 20/50 | -0.810 / 0.020 / **0.570** / 1.119 / 1.909 | 0.248 | 0.752 | 0.055 | 0.100 | **FRAGILE** |
| AVAX·2024 (down) | MACD | -0.475 / 0.252 / **0.895** / 1.369 / 2.146 | 0.160 | 0.840 | 0.027 | 0.048 | **FRAGILE** |
| AVAX·2024 (down) | RSI | -0.788 / -0.252 / 0.189 / 0.674 / 1.612 | 0.396 | 0.604 | 0.026 | 0.047 | **FRAGILE** |
| AVAX·2024 (down) | BBands | -1.217 / -0.603 / -0.175 / 0.246 / 0.909 | 0.594 | 0.406 | 0.025 | 0.046 | **FRAGILE** |
| DOT·2024 (down) | SMA 20/50 | -0.910 / 0.017 / **0.653** / 1.354 / 2.310 | 0.248 | 0.752 | 0.053 | 0.097 | **FRAGILE** |
| DOT·2024 (down) | MACD | -1.915 / -0.896 / -0.230 / 0.429 / 1.271 | 0.598 | 0.402 | 0.047 | 0.080 | **FRAGILE** |
| DOT·2024 (down) | RSI | -0.308 / 0.185 / 0.640 / 1.114 / 1.986 | 0.152 | 0.848 | 0.020 | 0.036 | **FRAGILE** |
| DOT·2024 (down) | BBands | -2.263 / -1.372 / -0.837 / -0.393 / 0.304 | 0.886 | 0.114 | 0.033 | 0.060 | **FRAGILE** |
| AVAX·2023 (up-market control) | SMA 20/50 | -0.137 / 1.005 / 1.651 / 2.305 / 3.175 | 0.062 | 0.938 | 0.043 | 0.073 | **FRAGILE** |

**Probability-of-loss read (the load-bearing tail):** even where the median is
comfortably positive, the down-market trend-followers carry a material loss
probability — SMA `prob_loss = 0.248` on BOTH AVAX·2024 and DOT·2024 (≈1 in 4
resampled orderings ends below the starting equity), and their p5 Sharpe is firmly
negative (-0.810, -0.910). MACD is split: tolerable on AVAX·2024 (p5 -0.475, prob_loss
0.160) but it collapses on DOT·2024 (p5 **-1.915**, prob_loss 0.598). Under the frozen
§ 0 rule, any `sharpe.p5 < 0` ⇒ FRAGILE — so all four strategies on both down-market
cells are FRAGILE.

## 4. What this means

### 4.1 The down-market hedge is PATH-FRAGILE, not robust

The survey's +5.0% / +6.4% protection was a **single-path** observation on the one real
2024 ordering. When we resample plausibly-different orderings of the *same* AVAX-2024 /
DOT-2024 down-market, the protective property does NOT hold across the distribution:
roughly the worst 5% of orderings turn the "hedge" into a loss (p5 Sharpe -0.810 /
-0.910). A positive *median* (SMA p50 +0.570 / +0.653) is consistent with the survey's
one-path number, but the **left tail is the decision variable** under the pre-registered
§ 0 rule, and it is negative. **The hedge was sensitive to the specific 2024 bar
ordering — it is not a path-robust property of the strategy.**

### 4.2 The negative control confirms the harness discriminates

RSI and BBands — strategies the survey found have *no edge anywhere* — land FRAGILE on
both down-market cells (RSI p5 -0.788 / -0.308; BBands p5 -1.217 / -2.263), with BBands
DOT·2024 carrying `prob_loss = 0.886`. The harness is not spuriously blessing no-edge
mean-reverters; it discriminates. RSI DOT·2024 sits at `prob_loss = 0.152` (a hair above
the 0.15 ROBUST threshold) but `sharpe.p5 = -0.308` → FRAGILE regardless. No
miscalibration signal.

### 4.3 The up-market control is the expected calibration, not a defect

AVAX·2023 SMA p5 = **-0.137** — matching the feature's explicit sanity expectation
("consistent with passive-dominates-up-markets, NOT a defect"). It scores FRAGILE by the
p5 < 0 rule, but only by a thin margin, and its distribution is clearly the *healthier*
of the SMA cells (prob_loss 0.062 vs 0.248; P(Sharpe>0) 0.938 vs 0.752). The harness
correctly separates the two regimes via distribution shape even though both formally
score FRAGILE — confirming the test is reading the regime, not a constant.

### 4.4 SCOPE CAP — per-symbol-year only (D-OG.5 / Q-OG.5)

**This is a negative result at the per-symbol-year level. State it that way and no
wider:**

> Within **AVAX-2024 individually** and **DOT-2024 individually**, the down-market
> trend-following hedge does NOT survive path-resampling — it is path-fragile.

It does **NOT** generalize to "trend-following fails to hedge down-markets in general."
The evidence is **2 symbol-years, hourly bars, shipped-default params** — a path-level
robustness statement about resampling *within each symbol's 2024*, not a cross-sectional
claim. A ROBUST verdict here would equally have been capped at the per-symbol level; the
cap is symmetric and applies to this FRAGILE outcome too. A wider down-market universe
(more 2024 bear names, other bear years, daily/minute granularity) is the v0.2.0
follow-on the feature § 7 names, not a conclusion of this ship.

## 5. Decision impact — folds into the passive-baseline thesis

- **"Ship passive" stands, now UNQUALIFIED on this evidence.** The survey added a
  down-market-hedge *nuance* to the ship-passive base recommendation. That nuance does
  not survive path-robustness testing, so on this evidence the ship-passive base
  recommendation no longer carries the "but trend-following is a defensible down-market
  hedge" qualifier. See the 2026-06-15 revision appended to
  [`passive-baseline.md`](../runbooks/passive-baseline.md) § Real-data validation.
- **The active-strategy down-market story is closed on this data.** The pre-registered
  test asked a yes/no question and the answer is FRAGILE. No live/paper-active path is
  added (operator constraint 2026-06-12; analysis tooling only).
- **A trend-following product line is NOT greenlit off this finding.** Had the hedge
  come back ROBUST it would have justified a v0.2.0 follow-on and an anchored canonical
  report (ADR-0051 § D6). It did not, so the un-anchored `#[ignore]` harness +
  this dev-note are the terminal deliverable; no `anchors.toml` churn (feature § 3).

## 6. Cross-references

- **Revises:** [`realdata-simple-strategy-survey-2026-06-13.md`](realdata-simple-strategy-survey-2026-06-13.md)
  Finding 1 (`[[realdata-simple-strategy-survey-2026-06-13]]`) — the +5.0% / +6.4%
  down-market hedge qualifier is downgraded to *path-fragile*; a one-line pointer to this
  note was added there.
- **Decision rule (frozen, applied AS-IS):** [`robustness-decision-rule-2026-05-30.md`](robustness-decision-rule-2026-05-30.md) § 0.
- **Feature + pre-registration:** [`simple-strategy-overfit-guard/feature.md`](../v1/simple-strategy-overfit-guard/feature.md)
  (§ 0 method, § 2 deliverable, D-OG.5 scope cap).
- **Confirmed numbers:** [`test-2026-06-15-1200-simple-strategy-overfit-guard.md`](../../evidence/v1/simple-strategy-overfit-guard/reports/test-2026-06-15-1200-simple-strategy-overfit-guard.md)
  (tester PASS, commit `3d843fa`).
- **Baseline thesis:** [`passive-baseline.md`](../runbooks/passive-baseline.md).

## Changelog

- 2026-06-15 (analyst): authored the `findings` dev-note for task T-OG.13 / AC-OG.5.
  Recorded the confirmed N=500 block-bootstrap p5/p50/p95 Sharpe + prob-of-loss table
  (all 9 cells FRAGILE under the frozen § 0 rule). Headline: the survey's 2-case
  down-market trend-following hedge is **path-fragile** — positive median (SMA p50
  +0.570 / +0.653) but negative p5 tail (-0.810 / -0.910). Capped the claim at the
  per-symbol-year level (D-OG.5) — does NOT generalize to down-markets in general.
  Revised the survey's Finding 1 (one-line pointer) and amended the passive-baseline
  runbook's Real-data-validation section (ship-passive base now UNQUALIFIED on this
  evidence). UN-ANCHORED — no spec/*/reports/ file, no anchors.toml row.
