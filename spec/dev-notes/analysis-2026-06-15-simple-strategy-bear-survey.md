---
slug: simple-strategy-bear-survey
status: findings
owner: analyst
updated: 2026-06-15
---

# Simple-strategy bear-market survey — no apparent bear "winner" is PATH-ROBUST; the 2021-22 deep bear FIRMS ship-passive — 2026-06-15

> **Headline.** Over the deepest, widest bear corpus the program has
> (`data/binance-2122/` — 10 large-caps x {2021, 2022} hourly, the entire
> universe down in 2022: BTC ~ -64%, SOL ~ -94%, AVAX ~ -90%), a two-stage
> survey asked whether **ANY** of the four shipped simple strategies (SMA 20/50,
> MACD, RSI, BBands) shows a **PATH-ROBUST** edge. Stage 1 (point survey) found
> **40 apparent winners** -- cells that beat buy-and-hold by >= 10 pp while B&H was
> negative -- **all from 2022**, several with spectacular single-path margins
> (SOL.2022 RSI: B&H **-94.2%** vs strat **+2.8%**, a **+97.0 pp** margin). Stage 2
> (N=500 block-bootstrap, scored against the frozen section 0 rule AS-IS) then
> path-tested the top 16 of those 40, and **every one scored FRAGILE** -- every
> candidate's p5 Sharpe is negative, including the +97 pp headline winner
> (SOL.2022 RSI p5 = **-0.888**). The up-market contrast cell (SOL.2021 SMA, a
> bull-leg control) scored **MARGINAL** (p5 = **+0.439**, positive), so the test
> discriminates regime direction correctly -- the all-FRAGILE bear verdict is a
> real signal, not a constant. **No strategy showed a path-robust edge. The
> 2021-22 deep bear FIRMS ship-passive.** The 2026-06-08 terminal "ship passive"
> verdict stands, now on the strongest available bear evidence.

This dev-note records the confirmed numbers from the tester's PASS
([`test-2026-06-15-1200-simple-strategy-bear-survey.md`](../v1/simple-strategy-bear-survey/reports/test-2026-06-15-1200-simple-strategy-bear-survey.md),
verdict PASS, commit `4585cf9`) and closes the loop on feature
[`simple-strategy-bear-survey`](../v1/simple-strategy-bear-survey/feature.md) (AC-BS.7,
task T-BS.14). It is the `findings`-status companion to the un-anchored `#[ignore]`
harness -- there is no anchored `spec/*/reports/backtest-*.md` (UN-ANCHORED per
feature section Anchoring / D-BS.4). It is the direct successor to the 2026-06-15
overfit-guard ([`analysis-2026-06-15-simple-strategy-overfit-guard.md`](analysis-2026-06-15-simple-strategy-overfit-guard.md)),
generalising its survey-identifies -> bootstrap-guards shape from 2 hand-picked
2024 cells to an automated, corpus-wide pipeline over a real market-wide bear.

---

## 1. The question this answers

The lineage: the 2023-24 real-data survey
([`realdata-simple-strategy-survey-2026-06-13.md`](realdata-simple-strategy-survey-2026-06-13.md))
found passive dominates in up markets, with one apparent down-market nuance -- in
the only 2 cells where B&H lost (AVAX.2024 -8.2%, DOT.2024 -19.6%) the
trend-followers protected capital. The overfit-guard then block-bootstrap-tested
exactly those 2 cells and found them **path-fragile** -- the hedge was an artifact
of the one 2024 ordering. But that finding was **narrow by construction**: it
rested on **2 idiosyncratic alt-coin dips inside an otherwise-bull 2023-24**. The
corpus simply did not contain a real, market-wide bear.

Now it does. `data/binance-2122/` (pin `4f390622`) put 2021-22 hourly bars for the
same 10 symbols on disk -- turning a 2-point down sample into a market-wide bear
(the whole universe down in 2022, plus the H1->H2-2021 drawdown leg). **2022 is the
real test**: a multi-month, cross-universe drawdown (BTC ~ -64%, LUNA/3AC in
May-Jun, FTX in Nov) -- exactly the regime where a trend-follower's "cut losers,
sidestep the drawdown" property *should* shine if it is real anywhere.

The sharp question, pre-registered before any 2021-22 number existed: **does ANY
of the four show a PATH-ROBUST edge in a real, deep bear, or does this further
FIRM the ship-passive verdict on a wider, deeper bear sample?**

## 2. Method (confirmed) -- two-stage, the overfit-guard shape made corpus-wide

The overfit-guard was implicitly two-stage (survey identifies 2 apparent winners ->
bootstrap guards those 2). This feature made that shape **explicit and automated**:

### Stage 1 -- point survey (cheap, 1 path per cell)

10 symbols x {2021, 2022} x 4 strategies = **80 single-path backtests**, each
strategy's total return % vs buy-and-hold, net of cost, over `data/binance-2122/`.
A cell is an **apparent winner** under the **FROZEN section-0-style pre-registered
predicate** (D-BS.2, fixed before the run):

> `buy_and_hold_pct < 0` **AND** `strat_ret_pct - buy_and_hold_pct >= 10.0` pp.

(Down-market gate + 10-pp margin. X = 10 pp justified against the 2024 motivating
margins of ~ 13/26 pp -- "this bar would have caught the 2024 cells", set without
reference to any 2021-22 figure.) Apparent winners are capped at **top-16 by margin
DESC**, deterministic tie-break `(margin DESC, symbol ASC, year ASC, strat_idx ASC)`.
Stage 1 **concludes nothing** -- a single-path margin is exactly the observation the
overfit-guard showed can be path-fragile. It only **selects candidates**.

### Stage 2 -- block-bootstrap path-robustness guard (expensive, N=500 per candidate)

For each of the 16 candidates (NOT all 80 cells -- bootstrap the *candidates*,
exactly as overfit-guard bootstrapped its 2), N=500 stationary block-bootstrap
paths (`BlockBootstrapPathGen`, single-symbol mode, `BlockLengthPolicy::Auto`),
reduced to Sharpe p5/p25/p50/p75/p95 + prob-of-loss + max-DD tail via
`DistributionSummary::from_path_metrics`, and scored against the **frozen section 0
rule AS-IS** (`sharpe.p5 < 0 => FRAGILE`; `prob_loss > 0.35 => FRAGILE`;
`max_dd_tail_p95 > 0.70 => FRAGILE`; ROBUST iff `p5 >= 0.5 AND prob_loss <= 0.15 AND
dd_p95 <= 0.50`; else MARGINAL; composite = worst band). The bands are NOT
re-derived or softened -- the section 0 rule is the ruler; the 2021-22 numbers are
scored against it.

| Knob | Value | Source |
|---|---|---|
| Generator | `BlockBootstrapPathGen`, single-symbol mode (1-entry universe) | C1, `crates/data/src/synth/bootstrap.rs` |
| Paths per ensemble | **N = 500** | section 0 bands calibrated at N=500 -- transferred AS-IS |
| Block length | `BlockLengthPolicy::Auto` (Politis-White) | **200-210 bars across all candidates -- no L<=1 i.i.d. degeneration** (Q-BS.5 PASS) |
| Seeds | ADR-0051 D1: `path_seed_j = ensemble_seed.wrapping_add(j*0x9E3779B9)`, constant fill_seed `0xC0FFEE`; distinct `ensemble_seed` per (strategy x cell) | byte-reproducible |
| Data | `data/binance-2122/` hourly Parquet, pin `4f390622` (read-only) | corpus-expansion feature |
| Decision rule | **Frozen section 0** (above), applied AS-IS | [`robustness-decision-rule-2026-05-30.md`](robustness-decision-rule-2026-05-30.md) section 0 |
| Harness | `crates/backtest/tests/realdata_simple_strategy_bear_survey.rs` (`#[ignore]`, UN-ANCHORED) | feature section Design D-BS.1 |

**Determinism confirmed** (AC-BS.5): two consecutive `--release --ignored
--nocapture` runs produced byte-identical Stage-1, candidate, and Stage-2 tables
(empty diff). **Negative control + discrimination confirmed** (AC-BS.6): no
mean-reverter scored ROBUST; the up-market contrast cell scored clearly different
(positive p5) from the all-negative-p5 bear candidates.

## 3. Stage 1 -- the apparent winners (confirmed)

**40 qualifying cells before cap -- ALL from 2022.** No 2021 cell qualified: 2021
was a two-peak bull (positive full-year B&H for most symbols), so the down-market
gate `B&H < 0` excluded it -- the apparent-winner set is entirely the 2022
market-wide bear, exactly the predicted shape. Top 16 by margin kept, 24 dropped.
Abbreviated Stage-1 table (full 80-cell table in the tester report section 5):

| Symbol . Year | B&H% | SMA | MACD | RSI | BBands |
|---|---|---|---|---|---|
| ADAUSDT . 2021 | +624.6% | +14.0% | +5.4% | +6.2% | +8.8% |
| ADAUSDT . 2022 | -81.5% | -8.0% | +0.9% | -0.3% | -3.6% |
| AVAXUSDT . 2021 | +3271.1% | +61.4% | +20.2% | +8.5% | +10.6% |
| AVAXUSDT . 2022 | -90.2% | -7.2% | -2.4% | +1.7% | -1.1% |
| SOLUSDT . 2021 | +10908.2% | +33.3% | +12.2% | +6.9% | +5.8% |
| SOLUSDT . 2022 | -94.2% | -6.2% | -2.9% | +2.8% | -4.9% |
| BTCUSDT . 2022 | -64.5% | -4.0% | -2.8% | -5.0% | -4.7% |

The headline single-path margins are dramatic -- **SOL.2022 RSI: B&H -94.2% vs
strat +2.8%, a +97.0 pp margin**; AVAX.2022 RSI: B&H -90.2% vs +1.7%, +91.9 pp.
Top candidates by margin (all 2022, confirming the market-wide-bear driver):

| Rank | Cell | Strategy | B&H% | Strat% | Margin | Keep? |
|---|---|---|---|---|---|---|
| 1 | SOLUSDT . 2022 | RSI | -94.2% | +2.8% | +97.0 pp | KEEP |
| 2 | AVAXUSDT . 2022 | RSI | -90.2% | +1.7% | +91.9 pp | KEEP |
| 3 | SOLUSDT . 2022 | MACD | -94.2% | -2.9% | +91.2 pp | KEEP |
| 4 | SOLUSDT . 2022 | BBands | -94.2% | -4.9% | +89.2 pp | KEEP |
| ... | (12 more, all 2022 bear cells) | | | | | KEEP |

**This is precisely the trap the two-stage method exists to catch.** A +97 pp
margin over buy-and-hold looks like the most protective hedge the program has ever
found. Stage 1 deliberately concludes nothing from it.

## 4. Stage 2 -- block-bootstrap verdicts (confirmed -- ALL 16 FRAGILE)

N=500 per candidate. Numbers copied verbatim from the tester report section 5 (Run A,
byte-identical to Run B). The last row is the out-of-predicate up-market contrast
control.

| Cell | Strategy | sharpe p5 / p25 / p50 / p75 / p95 | prob_loss | P(sharpe>0) | dd_p50 | dd_p95 | VERDICT |
|---|---|---|---|---|---|---|---|
| SOLUSDT . 2022 | RSI | **-0.888** / -0.122 / 0.430 / 1.041 / 1.948 | 0.310 | 0.690 | 0.040 | 0.075 | **FRAGILE** |
| AVAXUSDT . 2022 | RSI | -0.966 / -0.186 / 0.424 / 1.089 / 1.848 | 0.312 | 0.688 | 0.028 | 0.054 | **FRAGILE** |
| SOLUSDT . 2022 | MACD | -2.182 / -1.410 / -0.871 / -0.370 / 0.452 | 0.868 | 0.132 | 0.056 | 0.095 | **FRAGILE** |
| SOLUSDT . 2022 | BBands | -3.100 / -2.302 / -1.797 / -1.210 / -0.451 | 0.986 | 0.014 | 0.054 | 0.084 | **FRAGILE** |
| AVAXUSDT . 2022 | BBands | -2.800 / -1.937 / -1.313 / -0.711 / 0.112 | 0.930 | 0.070 | 0.037 | 0.068 | **FRAGILE** |
| SOLUSDT . 2022 | SMA 20/50 | -2.514 / -1.590 / -1.042 / -0.483 / 0.305 | 0.890 | 0.110 | 0.112 | 0.178 | **FRAGILE** |
| AVAXUSDT . 2022 | MACD | -2.115 / -1.290 / -0.754 / -0.208 / 0.438 | 0.836 | 0.164 | 0.051 | 0.087 | **FRAGILE** |
| DOTUSDT . 2022 | RSI | -1.474 / -0.577 / -0.055 / 0.379 / 1.041 | 0.534 | 0.466 | 0.029 | 0.053 | **FRAGILE** |
| AVAXUSDT . 2022 | SMA 20/50 | -2.562 / -1.756 / -1.183 / -0.532 / 0.453 | 0.880 | 0.120 | 0.116 | 0.196 | **FRAGILE** |
| DOTUSDT . 2022 | BBands | -1.148 / -0.256 / 0.284 / 0.843 / 1.689 | 0.374 | 0.626 | 0.014 | 0.030 | **FRAGILE** |
| ADAUSDT . 2022 | MACD | -1.781 / -0.668 / 0.027 / 0.682 / 1.744 | 0.482 | 0.518 | 0.039 | 0.072 | **FRAGILE** |
| ADAUSDT . 2022 | RSI | -1.219 / -0.467 / -0.031 / 0.527 / 1.240 | 0.512 | 0.488 | 0.023 | 0.048 | **FRAGILE** |
| DOTUSDT . 2022 | MACD | -2.799 / -1.962 / -1.520 / -1.054 / -0.370 | 0.984 | 0.016 | 0.061 | 0.095 | **FRAGILE** |
| ADAUSDT . 2022 | BBands | -2.821 / -2.201 / -1.759 / -1.312 / -0.613 | 0.994 | 0.006 | 0.035 | 0.060 | **FRAGILE** |
| LINKUSDT . 2022 | RSI | -1.118 / -0.256 / 0.396 / 0.959 / 2.000 | 0.350 | 0.650 | 0.031 | 0.058 | **FRAGILE** |
| ADAUSDT . 2022 | SMA 20/50 | -2.848 / -1.985 / -1.367 / -0.796 / 0.055 | 0.942 | 0.058 | 0.110 | 0.171 | **FRAGILE** |
| SOLUSDT . 2021 *(up-market contrast)* | SMA 20/50 | **+0.439** / 1.428 / 2.059 / 2.660 / 3.485 | 0.012 | 0.988 | 0.073 | 0.132 | **MARGINAL** |

**The single most important number:** the +97 pp headline winner, **SOL.2022 RSI,
has p5 Sharpe = -0.888** -- despite the largest apparent margin in the entire bear
corpus, ~ 31% of resampled orderings end below starting equity (prob_loss 0.310)
and the worst-5% tail is a clear loss. The apparent hedge was path-luck. The two
highest-p5 candidates (the two RSI cells, p5 -0.888 / -0.966) are also the
*least*-bad, and they are still firmly FRAGILE; the MACD/BBands/SMA candidates are
far worse (several p5 below -2.5, prob_loss up to 0.994). **No candidate is even
close to ROBUST. None is MARGINAL. All 16 are FRAGILE.**

## 5. What this means

### 5.1 The intuition worth recording -- why point returns lie in a catastrophic bear

In a catastrophic bear (B&H -90%+), **almost any strategy that happens to be
flat-or-short on the one realized path looks heroic on point returns.** If holding
loses 94% and a strategy ends roughly flat, the single-path margin is ~ +97 pp by
arithmetic -- it tells you the strategy was *not long the crash on that exact
ordering*, which is nearly the whole of the apparent edge. It does **not** tell you
the strategy would have sidestepped a *differently-ordered* version of the same
bear. **Path-resampling is exactly the test that separates "lucky timing on the one
historical ordering" from "a robust edge that holds across plausible re-orderings
of the same bars" -- and on the deepest bear we have, none survived it.** The bigger
the apparent point-return margin in a deep bear, the more it is dominated by
realized-path luck, and the more the bootstrap p5 tail is the number that matters.
The data shows this directly: the most spectacular Stage-1 margins (SOL/AVAX.2022)
produce the *highest* (least-negative) p5 tails, while still being negative -- the
margin and the robustness are nearly orthogonal.

### 5.2 The negative control + discrimination confirm the harness reads the regime

- **9 of the 16 candidates were mean-reverters** (RSI/BBands) -- the strategies the
  2023-24 survey found have no edge anywhere. **All 9 scored FRAGILE.** The
  highest-p5 mean-reverter (SOL.2022 RSI) sits at p5 -0.888. **No mean-reverter
  came back ROBUST** (AC-BS.6 PASS) -- the harness is not spuriously blessing
  no-edge churn dressed up by a deep-bear margin.
- **The up-market contrast cell discriminates.** SOL.2021 SMA (a bull-leg control,
  outside the predicate) scored **MARGINAL with p5 = +0.439** (positive),
  prob_loss 0.012, P(Sharpe>0) 0.988 -- clearly distinct from the all-negative-p5
  bear candidates. The test separates regime direction correctly; the all-FRAGILE
  bear verdict is a real reading of the data, not a constant the harness returns
  regardless of input.

### 5.3 The high-value tail did NOT materialize -- the question stays closed

The feature named an **asymmetric, low-prior, high-value tail explicitly**: a
**ROBUST survivor on a real market-wide bear would be the most credible non-passive
signal the program has produced** -- it would have **REOPENED** the active-vs-passive
question for a scoped v0.2.0 follow-on (a trend-following down-market product line),
and it would have justified an anchored canonical report under ADR-0051 section D6.
The whole point of stress-testing on the deepest bear was that a survivor *there*
would be the one result that should most change the operator's mind.

**No such survivor appeared.** Zero of 16 candidates -- including the +97 pp
headline cell -- scored ROBUST or even MARGINAL. The reopen path is **not**
triggered. The active-vs-passive question **stays closed**; no v0.2.0 trend-following
product line is greenlit off this evidence; the un-anchored `#[ignore]` harness +
this dev-note are the terminal deliverable (no `anchors.toml` churn, per feature
section Anchoring).

### 5.4 SCOPE CAP -- what this does and does NOT prove (honest, per D-OG.5 / feature section Scope cap)

**State the result at the level the evidence supports, and no wider:**

> This FIRMS ship-passive on the **strongest available bear evidence** -- but it is
> still **hourly bars, shipped-default params, 4 simple strategies, 10 large-cap
> symbols, and the 2 specific years 2021-22**. A null result (no path-robust edge)
> firms ship-passive; it does **NOT** prove "no strategy can ever beat passive in a
> bear market." Untuned defaults, one timeframe, a fixed universe, and two specific
> years are not the space of all strategies.

Specifically, per the overfit-guard's symmetric cap:
- A per-symbol-year FRAGILE verdict is a statement about **path-resampling within
  that symbol's 2021 or 2022 individually**, NOT a cross-sectional "trend-following
  can never hedge a bear" claim. The cap is symmetric -- a ROBUST verdict would have
  been capped the same way.
- The frozen section 0 rule scores robustness **to resampled real history**, not to
  out-of-history regimes -- block bootstrap cannot synthesize a regime 2021-22 never
  contained (a future 2025-style event, a regulatory shock is outside what this
  distribution speaks to).
- 2021-22 is **two specific bear/sideways years**; a different deep bear is outside
  this distribution.

What it DOES firm, decisively: the previous fragility finding has gone from "the
one 2024 hedge wasn't real" to "across a whole **deep-bear universe** -- 40 apparent
winners, the single most spectacular +97 pp margin in the corpus -- **no apparent
winner is path-robust.**" That is a far stronger statement than the overfit-guard's
2-cell result, and it points the same direction.

## 6. Decision impact -- folds into the passive-baseline thesis

- **"Ship passive" stands, now on the strongest available bear evidence.** The
  2026-06-08 terminal verdict, already promoted to the production baseline, is not
  reopened. This stress-test was a deliberate test of an already-closed decision
  against the deepest bear we have; the decision survived it. See the 2026-06-15
  bear-survey note appended to [`passive-baseline.md`](../runbooks/passive-baseline.md)
  section Real-data validation.
- **The bear-market active story is closed on this data.** The pre-registered test
  asked a yes/no question -- does any simple strategy show a path-robust edge in a
  real deep bear -- and the answer is no (16/16 FRAGILE). No live/paper-active path
  is added (operator constraint 2026-06-12; analysis tooling only).
- **No trend-following product line is greenlit.** Had any candidate come back
  ROBUST it would have justified a v0.2.0 follow-on and an anchored canonical report
  (ADR-0051 section D6). None did, so this is terminal; no `anchors.toml` row.

## 7. Cross-references

- **Predecessor (generalised here):** [`analysis-2026-06-15-simple-strategy-overfit-guard.md`](analysis-2026-06-15-simple-strategy-overfit-guard.md)
  (`[[analysis-2026-06-15-simple-strategy-overfit-guard]]`) -- the 2024 down-market
  hedge was path-fragile on 2 cells; this generalises that survey-identifies ->
  bootstrap-guards method to a corpus-wide automated pipeline over a real
  market-wide bear, and reaches the same direction with far stronger evidence (40
  apparent winners, 16 bootstrapped, all FRAGILE).
- **Original survey:** [`realdata-simple-strategy-survey-2026-06-13.md`](realdata-simple-strategy-survey-2026-06-13.md)
  (the 2023-24 survey; its Finding 1 down-market nuance was revised path-fragile by
  the overfit-guard and is now further firmed against a deep bear here).
- **Decision rule (frozen, applied AS-IS):** [`robustness-decision-rule-2026-05-30.md`](robustness-decision-rule-2026-05-30.md) section 0
  (`sharpe.p5 < 0 => FRAGILE` -- the ruler; the 2021-22 numbers are scored against
  it, not the reverse).
- **Feature + pre-registration:** [`simple-strategy-bear-survey/feature.md`](../v1/simple-strategy-bear-survey/feature.md)
  (section Design D-BS.2 frozen predicate + cap, section Scope cap).
- **Confirmed numbers:** [`test-2026-06-15-1200-simple-strategy-bear-survey.md`](../v1/simple-strategy-bear-survey/reports/test-2026-06-15-1200-simple-strategy-bear-survey.md)
  (tester PASS, commit `4585cf9`).
- **Corpus consumed:** [`binance-corpus-expansion/feature.md`](../v1/binance-corpus-expansion/feature.md)
  (`data/binance-2122/`, pin `4f390622`).
- **Baseline thesis:** [`passive-baseline.md`](../runbooks/passive-baseline.md).

## Changelog

- 2026-06-15 (analyst): authored the `findings` dev-note for task T-BS.14 / AC-BS.7.
  Recorded the confirmed two-stage result: Stage 1 found 40 apparent winners (all
  2022, top margin SOL.2022 RSI +97.0 pp); Stage 2 (N=500 block-bootstrap, frozen
  section 0 rule) scored all 16 bootstrapped candidates FRAGILE (every p5 Sharpe < 0,
  incl. the +97 pp winner at p5 -0.888); up-market contrast SOL.2021 SMA MARGINAL
  (p5 +0.439) so the test discriminates. Headline: the 2021-22 deep bear FIRMS
  ship-passive -- apparent bear winners are strategies that sat out the crash on the
  one historical ordering; bootstrap shows the edge is path-luck, not robust.
  Recorded the intuition (point returns lie in a -90% bear; path-resampling is the
  separating test) and the high-value tail that did NOT materialise (no ROBUST
  survivor -> reopen path NOT triggered -> question stays closed). Applied the scope
  cap (per-symbol-year; hourly/default-params/10-large-caps/2-bear-years; firms but
  does not prove no strategy can ever work). Cross-referenced the overfit-guard
  predecessor + the section 0 rule. Amended the passive-baseline runbook section
  Real-data validation (a dated 2026-06-15 bear-survey note strengthening the
  same-day overfit-guard revision). UN-ANCHORED -- no spec/*/reports/ file, no
  anchors.toml row.
