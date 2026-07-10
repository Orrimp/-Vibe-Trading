---
title: P2 corpus-expansion wobble — thesis-level decomposition + framing decision-support
date: 2026-07-10
author: analyst
status: decision-support (READ-ONLY analysis; operator ratifies any product.md/README/CHANGELOG/register framing change)
feature: advisor-corpus-expansion
trace: REQ-V3-P2-CORPUS-EXPANSION-001
inputs:
  - spec/v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun.md  (AC1–AC8, tester PASS)
  - raw 1208-line matrix: /private/tmp/.../scratchpad/p2-verdict-rerun-full.txt (orchestrator-verified)
  - research/SYNTHESIS.md, research/strategies/knowledge.md
scope: analyzes an empirical finding; recommends framing options + a follow-on register; changes NO product framing, NO code, NO reports.
---

# P2 corpus-expansion wobble — is the ship-passive thesis falsified, and what (if anything) changes?

> **CORRECTION (same day, post scorecard-fix `9e8cd05`):** the DSR figures in this analysis
> ("16/19 clear DSR≥0.95", the §1(d) weighting) were computed under the scorecard NaN bug that
> zeroed DSR's variance input. Post-fix **0/19 old-era crowns clear DSR** — the era-boundary
> pattern (60-86% crowning vs the ~20% noise floor, cost-annex-robust) STANDS; individual-edge
> DSR certification does NOT. Component (d) is larger than assessed below. Authoritative
> correction: [`../v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun-errata.md`](../v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun-errata.md)


## 0. The one-paragraph statement of the problem

The P2 verdict re-run (tester PASS, orchestrator-verified against the raw
1208-line matrix) crowned an ACTIVE arm on **19 of 32 primary symbol-runs**,
concentrated in the OLD eras (2017-18 2/3, 2020 6/7, 2021-22 8/10) and rare in
the recent market (2023-24 0/1, 2025-26 2/10). **16 of 19** active crowns clear
DSR ≥ 0.95, and most survive the vol-scaled era-cost annex (S7 2/3, S8 5/7; the
only true outcome flip is DOGE-2020 → BenchmarkWins under wider costs). The
product's validated thesis was authored as "no active strategy robustly beats
buy-and-hold net of costs" — which is TRUE on 2023-24 (where it was measured)
but reads as a **universal-era** claim in one load-bearing place (the do-not-build
register A-group preamble). As a universal-era claim it is now **falsified**; as a
**current-market-era** claim it is intact and, via the MinBTL result (3.99 → 7.90
years, now above the 6.36y honest bar), on materially firmer ground than before
P2. This note decomposes WHY the old eras crown, establishes that **no
user-facing behaviour changes** (the edges are not reachable today), and gives
the operator 3 framing options with a recommendation.

---

## 1. Decomposing the wobble honestly — (a) real inefficiency / (b) cost artifact / (c) survivorship / (d) multiple-testing residue

The old-era `ActiveWins` crowns are **not one thing**. The four candidate causes
are non-mutually-exclusive and each explains a different slice. Ranked by the
strength of the evidence the P2 data + research corpus can bring:

### The raw crown table (old-era ActiveWins, from AC1 + raw log)

| Run | Crowned arm | Crown Sharpe | B&H Sharpe | DSR | Survives era-cost annex? |
|-----|-------------|-------------:|-----------:|----:|:--|
| S1-1718 BTC (2018 bear) | `v0.5.rsi` | **1.86** | −0.10 | 0.9911 | **yes** (S7, crown stays rsi) |
| S1-1718 BNB | `v0.8.vote.k2of4` | **1.88** | 0.64 | 0.9819 | yes (S7, crown-swaps → bbands 0.96) |
| S2-2020 BTC | `v0.donchian_floor` | **2.63** | 1.83 | 0.9953 | **yes** (S8, unchanged) |
| S2-2020 ETH | `v0.sma` | **2.62** | 1.87 | 0.9957 | **yes** (S8, unchanged) |
| S2-2020 BNB | `v0.8.vote.k3of4` | 1.75 | 1.01 | 0.9969 | yes (S8, DSR 0.979) |
| S2-2020 ADA | `v0.8.vote.k2of4` | 2.29 | 1.52 | 0.9898 | yes (S8, crown-swaps → macd 0.98) |
| S2-2020 LINK | `v0.8.vote.tr_mr_sma_bb` | 2.18* | 1.5x | 0.9999 | yes (S8, crown-swaps → sma 0.99) |
| S2-2020 DOGE | `v0.8.vote.majority` | **1.03** | **1.83** | **0.9138 (FAILS)** | **NO → BenchmarkWins** |
| S6-Coinbase BTC (2020-26 mixed) | `v0.donchian_floor` | 0.85 (Marginal) | 0.51 | 0.9854 | n/a (price-only) |

*LINK crown Sharpe read from the S8 crown-swap winner; the exact primary-run
crown-arm Sharpe is in the raw log around L640. The load-bearing numbers are the
BTC/ETH/BNB rows and the DOGE row.*

### (a) REAL early-market inefficiency — STRONG evidence, and it is the dominant honest reading

This is the strongest of the four, and — importantly — it is the reading the
research corpus *predicts*, not one the corpus contradicts:

- **The literature explicitly supports a historical crypto trend/momentum edge.**
  `research/strategies/knowledge.md:97` — "time-series momentum is the most
  academically supported [12][41]" single-asset-computable crypto signal; §6
  (line 27) "crypto-specific evidence cuts both ways" — a real historical TSM
  signal that the same corpus shows *decaying*.
- **The efficiency-migration / decay literature says exactly this edge should
  have died over our window.** `research/SYNTHESIS.md:62` (the round-3 correction,
  full-text McLean–Pontiff): ~10% statistical + **~35% post-publication
  crowding** decay, and — decisively — "the decay is **largest for the
  cheapest-to-arbitrage, low-idiosyncratic-risk names = BTC/ETH/SOL** (our exact
  coins)." `knowledge.md:20-21,56-60` quantifies anomaly decay ("published
  cross-sectional … we test is maximally public → expect the largest decay
  class"); `knowledge.md:116` — patterns that "decayed to nothing post-2015."
- **The P2 data traces exactly that decay curve.** ActiveWins rate is monotone
  with era age: 2017-18 67% → 2020 86% → 2021-22 80% → 2023-24 0% → 2025-26 20%.
  A market that was inefficient early and matured into deep-liquidity efficiency
  is the single hypothesis that fits the whole gradient AND the corpus's own
  prior. **This is the honest headline: the machinery detected the efficiency
  boundary the research said existed.** The two survivors of the S2/S8 stress
  (donchian_floor BTC Sharpe 2.63 > B&H 1.83; sma ETH 2.62 > B&H 1.87) are the
  cleanest examples — large Sharpe margins over hold, DSR ~0.995, unchanged under
  widened costs.

**Weight: this plausibly accounts for the MAJORITY of the durable old-era crowns
(the 2020 BTC/ETH/BNB/ADA/LINK cluster and the 2018-bear RSI crown), i.e. the
5–7 runs that survive both DSR and the cost annex with a crown Sharpe materially
above B&H.**

### (b) Cost-model artifact BEYOND the annex — REAL, quantified as SMALL, but the annex genuinely under-captures 2017-18

The annex (`cost::DEFAULT_VOL_SCALED_SPREAD`, ADR-0081) widens the effective
spread as realized vol rises. What it **does** capture: the fact that thin, jumpy
old-era bars had wider effective spreads (AC5 corroborates — 2020 cross-venue
median deviation 7.6 bps vs 3.3-3.5 bps in 2023-26, so old-era frictions were
demonstrably larger even between two blessed HIGH-reconcilable venues). It flipped
exactly **1 of 10** tested S1/S2 symbol-runs (DOGE-2020) — so on the annex's own
terms the cost effect is real but explains only a small part of the gradient.

What the annex does **NOT** capture, and where 2017-18 true frictions are almost
certainly worse than *either* flat-8bps or the vol-scaled model:

1. **Slippage / market impact on thin books.** Both models charge a spread but
   assume fills at the modeled price for the (small) advisor position; neither
   models order-book depth. 2017-18 Binance books were orders of magnitude
   thinner — a real taker would have walked the book. The vote/RSI arms that
   crown trade more often than B&H, so they pay this uncaptured cost per turn.
2. **Exchange outages / halted trading.** 2017-18 saw repeated Binance overload
   halts during exactly the high-vol windows an active arm most wants to trade.
   The backtest assumes continuous fillability; a real active strategy could not
   act at the signal bar.
3. **Withdrawal / counterparty risk** (not a per-trade cost but a real 2017-18
   frictional drag on any active re-allocation) — irrelevant to B&H which never
   moves.
4. **Maker/taker fee-tier immaturity.** The flat 8 bps is calibrated to modern
   tiered fees; 2017-18 effective all-in taker costs were plausibly 10-25 bps.

**Weight: this does not overturn (a) — the direction is conservative-correct (AC6:
higher true old-era costs make an ActiveWins verdict there STRONGER, not weaker,
since the active arm's net edge is if-anything overstated). But it means the
old-era Sharpe MARGINS are inflated, and a crown like S2-2020-BNB (1.75 vs B&H
1.01, a 0.74 gap) is more fragile to true costs than the raw number suggests.
The honest statement is "the gradient is real; the size of each old-era edge is
an upper bound."**

### (c) Survivor-of-survivors bias — STRUCTURAL, un-quantifiable from this data, and it caps the generalizability hard

AC6/R5 states this in words; here is the quantitative sharpening. The 1718 corpus
is **BTC/ETH/BNB only** — the three largest EVENTUAL survivors of a 2017-18 top-10
that was full of coins now near-zero (BCH/BCC forks, dozens of ICO tokens, several
2018 top-10 names now dead). A "these 3 coins over 2017-18" result is the **most
favourable possible slice** of that era: it is conditioned on survival, which is
itself correlated with having had exploitable trend structure (coins that trended
up and survived are exactly where a trend-following arm looks good). The 2020
corpus's 7 symbols are all still-liquid today — same bias, less acute.

**Weight: this cannot be decomposed OUT of the (a) signal from the P2 data alone
— they are confounded by construction. It means even the durable (a) crowns
"generalize" only to 'the eventual mega-survivors of that era', NOT to 'a random
2017 coin pick'. For the PRODUCT this is the strongest single reason the old-era
edges are un-actionable as forward advice (§2): a user in 2017 did not know which
3 coins would survive.** Any framing that leans on (a) MUST carry (c) as its
immediate qualifier or it over-claims.

### (d) Multiple-testing residue DSR cannot catch at these run counts — REAL and named; the DOGE run is the smoking gun

DSR deflates the crown Sharpe for the number of arms searched **within one run**
(n_candidates ≈ 23-25). It does NOT correct for the **cross-run** search implicit
in "we ran 32 symbol-runs across 6 corpora and are now reading the ones that
crowned." At 32 runs, ~1-2 DSR-clearing false positives at the 0.95 threshold are
expected by construction. Two concrete caveats:

- **The DOGE-2020 crown is a robustness-gate artifact, not a return-beating
  result — and it is the tell.** Its crown arm `v0.8.vote.majority` has raw Sharpe
  **1.03, LOWER than B&H's 1.83** (raw log L24/L37). It crowned only because the
  weakest-link robustness gate reordered near-tied curves; DSR correctly withheld
  credibility (0.9138 < 0.95); and it is the ONE run that flips to BenchmarkWins
  under the cost annex. This is the machinery working — but it demonstrates that a
  "crown" at the margin is not evidence of a real edge, and at 32 runs there will
  be a few such marginal crowns.
- **The scorecard's `n_eff=NaN` / `min_btl_years=0.00` fields are non-trustworthy
  as printed** (AC3, tester-flagged, confirmed PRE-EXISTING on the byte-untouched
  S4 baseline). Root cause: `NaN`-Sharpe arms (`v0.sma_cross_ls`, `v0.always_short`
  when equity goes deeply negative) propagate through the `n_eff` Sharpe-vector
  moment computation, and `f64::max(NaN, x) == x` (IEEE) silently clamps
  `min_btl`'s `n_eff.max(1.0+ε)` to ~1.0 → `min_btl ≈ 0`. **So the per-run MinBTL
  veto is not actually firing** — the AC3 before/after MinBTL numbers (3.99 →
  7.90y) are computed *independently from the corpus windows*, which is correct,
  but the in-gate per-run field is blind. This is a real hardening gap (§4), and
  it means the multiple-testing defense is currently one leg short (DSR fires;
  the MinBTL per-run veto does not).

**Weight: (d) accounts for the 2-3 marginal crowns that fail DSR (DOGE-2020,
XRP-2122, LINK-2526) PLUS an unknown-but-small number of the DSR-clearing crowns
that are cross-run false positives. It does NOT explain the large-margin,
cost-robust 2020 BTC/ETH cluster — those have too much Sharpe headroom over B&H
(0.8-1.0) and survive the annex. Naming (d) honestly is what keeps the (a) reading
from over-claiming.**

### Decomposition summary (the honest apportionment)

| Cause | What it explains | Strength of evidence | Net effect on the thesis |
|-------|------------------|---------------------|--------------------------|
| (a) real early inefficiency | the durable 2020 BTC/ETH/BNB/ADA/LINK cluster + 2018-bear RSI (5-7 runs, big Sharpe margins, DSR ~0.99, cost-robust) | **STRONG** — corpus predicts it (SYNTHESIS §62, knowledge §97) + P2 gradient fits | falsifies the *universal-era* claim; is itself the credibility story (§2 option B) |
| (b) cost beyond annex | inflates every old-era Sharpe MARGIN; 2017-18 worst | REAL, direction conservative-correct, annex flips only 1/10 | caps the *size* of (a); makes crowns upper-bounds |
| (c) survivor-of-survivors | caps generalizability of ALL old-era crowns to 'eventual mega-survivors' | STRUCTURAL, un-decomposable, load-bearing | the core reason (a) is UN-ACTIONABLE forward (§2) |
| (d) multiple-testing residue | 2-3 marginal fails (DOGE/XRP/LINK) + a few cross-run FPs | REAL; DOGE is the smoking gun; MinBTL per-run veto currently blind (NaN) | trims the count; does NOT erase the large-margin cluster |

**The one-line honest read:** the wobble is dominantly a **real, corpus-predicted
early-market inefficiency that has since decayed** (a), with its Sharpe margins
**upper-bounded by un-modeled old-era frictions** (b), its generalizability
**capped at eventual survivors** (c), and a **small marginal-crown residue** DSR
partly-catches (d). None of the four makes the old-era edge reachable by a user
today.

---

## 2. What follows for the PRODUCT's forward-looking advice? — NOTHING changes in behaviour

The advisor advises a user **today**, on **recent data windows** (the cockpit's
Calibrate/bake-off lookbacks are 2-week-to-4-year, all ending at "now"; the
forward paper-trade is genuine unseen recent data). Walk the causal chain:

1. **The old-era edges are not reachable in any window the product runs.** A 2025
   user cannot bake off on 2017-18 bars to "capture" the RSI edge — that data is
   in the past and the edge decayed (the P2 data itself shows it: 2023-24 0/1,
   2025-26 2/10, and the 2 recent crowns fail-or-barely-clear: ADA-2526
   roc_momentum DSR 0.985 but B&H-relative return is thin, LINK-2526 DSR 0.9202
   FAILS). On the CURRENT regime the modal outcome is `BenchmarkWins`, exactly as
   before P2.
2. **Chasing the old-era edge as a live strategy is the exact alpha-chasing the
   do-not-build register forbids** (Group A). "Re-enable donchian_floor because it
   won in 2020" is A-5 (adding arms to chase an edge) crossed with survivorship
   mining. There is no time machine; there is no forward-actionable signal here.
3. **The engine, the frozen gate, the classifier bands, and the crown-credibility
   band are all per-run honest and need no change.** They correctly crowned active
   arms where the data supported it (old eras) and correctly crown B&H where it
   does not (recent eras). That is the machinery working, not a bug.

**Therefore: no user-facing behaviour changes. The change is exclusively in the
HONESTY PROSE — how the product *describes* the scope of its thesis — not in the
engine, the gate, or any advice the user receives.** This is the analyst lean and
the data confirms it.

---

## 3. Framing options for the operator (the decision)

The thesis prose lives in 5 places (exact file/line map in §3.4). Today it is
**inconsistently scoped**: the runbook and two product.md spots ALREADY say
"2023-24 large-cap sample"; but the **do-not-build register A-group preamble
(line 41-42) carries a fully UN-era-qualified universal claim** ("no active
strategy robustly beats buy-and-hold net of costs on a single liquid coin"), as
do product.md line 74 and the § "Why this is honest" opener (lines 93-96) and the
README "held every time" line (34-35). The decision is which framing to make
consistent to.

### Option A — Minimal era qualifier (fallback if budget tightens)

Add a bounding clause everywhere the claim appears: *"…net of costs on the current
market era (2023+); earlier, less-efficient crypto eras (2017-20) showed real,
DSR-clearing active edges that have since decayed — see the P2 corpus-expansion
report."* Touches ~5 lines across 4 files + a one-line register preamble amendment.

- **Pros:** honest, cheap, closes the falsification-as-universal-claim gap, no new
  narrative to maintain.
- **Cons:** treats the finding as a caveat to be footnoted, under-selling what is
  arguably the product's *best* credibility evidence.

### Option B — Fuller "efficiency migration" narrative (Recommended — the durable choice)

Reframe the finding as a **strength**: the market matured, the early edges decayed,
and the product's own machinery detected exactly that boundary — crowning active
arms in the inefficient eras (2017-20), B&H in the efficient ones (2023+), and the
DSR/cost stack correctly grading the marginal ones. State it in product.md §
"Why this is honest" and the register preamble; qualify with (b)/(c)/(d) so it
never over-claims the old edges are actionable. Same ~5-line file touch as A, plus
~2-3 sentences of narrative in product.md § "Why this is honest" and the README
arc.

- **Pros:** This is the **long-term-correct** framing. It converts a "wobble" into
  a demonstration that the credibility machinery *works both ways* — it does not
  merely fail to find alpha (which a skeptic can read as "your gate is too
  strict"), it POSITIVELY detects real historical edges and their decay. That is a
  far stronger honesty claim than "we looked and found nothing," and it is exactly
  what the research corpus predicted (SYNTHESIS §62). It pre-empts the obvious
  external critique ("of course B&H wins in a 2-year bull sample — your test is
  rigged for it") by showing the machine crowns active arms when they genuinely
  win. It will not need re-litigation when the next corpus lands.
- **Cons:** ~2-3 sentences more to write and keep consistent; requires the
  (b)/(c)/(d) qualifier discipline so a future reader does not mistake it for "old
  strategies work."
- **If-budget-tightens fallback:** Option A (the era qualifier alone), which is a
  strict subset of B's edits — B can be reduced to A without rework.

### Option C — Status quo + report-link only (weakest — not recommended)

Leave the universal-claim prose as-is, add a one-line pointer to the P2 report.

- **Cons:** leaves the register A-group preamble asserting a **now-falsified
  universal claim** as "the product's validated thesis." Anyone who reads the P2
  report and then the register sees a contradiction. This is the over-claim the
  product exists to avoid — it is the one option that is affirmatively *dishonest*
  post-P2. Reject.

### Recommendation: **Option B**, with A as the strict-subset fallback.

Grounds: (1) the finding is corpus-predicted and DSR-backed, not noise — it earns
a positive framing; (2) B strictly dominates A on durability (carries forward
across future corpora without amendment) at ~2-3 sentences' marginal cost; (3) B
is the only option that turns the falsification into *added* credibility rather
than a defensive footnote; (4) the honesty discipline B requires (name b/c/d) is
already fully worked out in §1 of this note, so the drafting cost is low. Per the
durable-over-quick operator preference, the Recommended tag goes on B (the choice
whose framing carries forward), not on the cheaper A.

**Both A and B require the same one-line touch on the register A-group preamble**
(the universal claim → era-scoped claim). Option C is the only one that leaves it,
and that is precisely why C is rejected.

### §3.4 — Exact file/line map: where the universal claim lives TODAY (enumerated, NOT edited)

The operator (or a follow-on ratification pass) edits these; this note changes
none of them.

| File | Line(s) | Current wording | Scope today |
|------|--------:|-----------------|-------------|
| `spec/dev-notes/do-not-build-register.md` | **41-42** | "**no active strategy robustly beats buy-and-hold net of costs on a single liquid coin.**" (A-group preamble, "The product's validated thesis") | **UNIVERSAL — the primary carrier of the now-falsified universal claim. The one line B/A most needs to era-scope.** |
| `spec/product.md` | 74-77 | "*no active strategy beat passive buy-and-hold net of cost* on the 2023-24 large-cap sample" | already era-scoped ✓ (light touch only, for consistency of voice) |
| `spec/product.md` | 90-96 | § "Why this is honest" opener: "no active strategy beat passive buy-and-hold net of cost under a pre-registered block-bootstrap" (no era qualifier in the opening sentence) | **semi-universal — the § where Option B's efficiency-migration narrative lands** |
| `spec/product.md` | 336-342 | "This is a **bounded** result on the 2023-24 large-cap sample, not a claim active trading is impossible" | already era-scoped ✓ (strongest existing honest phrasing — a model for the others) |
| `README.md` | 16-17 | "no active strategy beat passive buy-and-hold net of cost … (firmed on real 2021-22 bear-market data)" | partially scoped (names 2021-22/2023-24 evidence, not an era boundary) |
| `README.md` | 34-35 | "stress-tested from every reachable angle … and **held every time**" | **semi-universal — "held every time" is the phrase most in tension with the P2 old-era crowns; needs the era clause** |
| `README.md` | 72 | "when no active strategy robustly beats it — the modal real-crypto outcome" | already honest ✓ ("modal", not "always") |
| `CHANGELOG.md` | 26-29 | "(**ship passive:** no active strategy beat passive buy-and-hold net of cost …) … stress-tested from every reachable angle … and held every time" | **semi-universal — same "held every time" tension as README 34-35** |
| `spec/runbooks/passive-baseline.md` | 17-36 | "on the 2023-24 large-cap perp sample … does **NOT** mean 'active [wins across regimes]'" | **already the MOST honest carrier** — fully era-scoped with an explicit scope-honesty callout. **Model the others on this.** |

**`research/SYNTHESIS.md`** (line 14 thesis box, "no active strategy robustly
beats holding, net of costs") is a **historical research artifact dated
2026-06-28** — per the brief, **leave it**; it records what the 900-paper program
concluded at its time and should not be retro-edited.

**Net:** the single load-bearing edit for A or B is the **register line 41-42**.
Everything else is consistency-of-voice (product.md § "Why this is honest" opener,
README/CHANGELOG "held every time"). The runbook already shows the target voice.

---

## 4. Follow-on work register (recommend; do NOT start)

1. **Scorecard `n_eff` / `min_btl` NaN-hardening (pre-existing, NOT P2).** The
   `f64::max(NaN, x) == x` clamp silently zeroes `min_btl_years` and NaNs `n_eff`
   whenever any arm returns a NaN Sharpe (`sma_cross_ls`/`always_short` on deeply
   negative equity). Effect: the **per-run MinBTL veto never fires** — the
   multiple-testing defense is one leg short (DSR fires; MinBTL does not). Fix:
   filter NaN-Sharpe arms out of the `n_eff` moment computation (or treat a
   NaN-Sharpe arm as `sharpe = very-negative`, not as absent), then re-derive
   `min_btl`. **Recommend: worth doing** — it is cheap (one `bakeoff/scorecard.rs`
   filter), it is a real gap the P2 run surfaced on the byte-untouched S4 baseline,
   and it makes the (d)-decomposition defense actually operative. Report-only /
   additive; touches no classifier band. Confirmed pre-existing → not a P2
   regression, so no urgency, but it is the highest-value follow-on.

2. **A dedicated 2017-18 order-book-era cost-model study — NOT worth it; prefer a
   stated limit.** Building a depth/impact/outage-aware cost model for 2017-18
   (per §1b) would be a large data-acquisition + modeling effort (historical L2
   books, outage logs) for a window the product **never advises on**. The honest
   ROI is negative: it would refine the Sharpe MARGIN of edges that are already
   established as (c) un-actionable and (a) decayed. **Recommend: a one-sentence
   STATED LIMIT** in the P2 report's AC6 lineage / the register — "old-era (pre-2021)
   crown Sharpe margins are upper bounds; true 2017-20 frictions (depth, impact,
   outages, withdrawal risk) exceed both the flat-8bps and vol-scaled models" —
   rather than a modeling feature. (This is analysis, not a build.)

3. **Crown-credibility band (P1, fires on fails-DSR only) — NO band change.** The
   band is **per-run honest**: it fires when a crown fails DSR, which it correctly
   did on DOGE-2020/XRP-2122/LINK-2526. Adding "era context" to the BAND would be
   wrong — the band is a statistical property of one run, not a place for
   cross-era narrative. **Recommend: era context belongs in PROSE (§3 option B)
   and in the P2 REPORT, never in the band logic.** Analyst lean confirmed: no
   band change.

4. **(Optional, low priority) A cross-run multiple-testing note.** If the operator
   wants the (d) defense fully closed, a future report could apply a
   family-wise correction across the 32 runs (e.g. a Šidák/Holm adjustment on the
   set of crowns, or note the expected false-positive count at α=0.05, 32 trials).
   **Recommend: a report-annex line, not a gate change** — the per-run DSR + the
   MinBTL fix (item 1) are the operative defenses; a cross-run note is
   completeness, not a blocker.

---

## 5. TL;DR (≤5 bullets, for the operator)

- **The finding is real and honest both ways: the thesis is FALSIFIED as a
  *universal-era* claim (active arms genuinely crowned + cleared DSR + survived
  cost stress on 2017-20) but INTACT and *stronger* as a *current-market-era*
  claim (2023+ modal `BenchmarkWins`, MinBTL now 7.90y > 6.36y bar).** The old-era
  wobble is dominantly **real early-market inefficiency that has since decayed** —
  which is exactly what the 900-paper corpus predicted (SYNTHESIS §62:
  large-cap-crypto anomaly decay is largest for BTC/ETH).
- **Decomposition:** (a) real decayed inefficiency = the dominant cause, big
  Sharpe margins (2020 donchian 2.63 vs B&H 1.83; sma 2.62 vs 1.87), DSR ~0.99,
  cost-robust; (b) old-era costs beyond the annex inflate the margins (upper
  bounds, not overturns); (c) survivor-of-survivors caps generalizability to
  'eventual mega-survivors' (the reason it's un-actionable); (d) marginal-crown
  residue (DOGE-2020 crowned with Sharpe 1.03 < B&H 1.83 — a gate artifact — and
  the `min_btl=0.00`/`n_eff=NaN` per-run veto is currently blind).
- **NO user-facing behaviour changes.** The old-era edges are not reachable in any
  window the advisor runs (all lookbacks end at "now"), and chasing them would be
  the exact alpha-chasing the do-not-build register forbids. The change is
  **prose-only** — engine, gate, classifier bands, and crown-credibility band all
  stay byte-identical.
- **Recommended framing: Option B (efficiency-migration narrative)** — reframe the
  wobble as *strength* (the machinery positively detected real historical edges
  and their decay, not merely "found nothing"), qualified by (b)/(c)/(d) so it
  never implies old strategies work; **fallback Option A** (minimal era qualifier)
  is a strict subset if budget tightens; **reject Option C** (status quo leaves the
  register asserting a falsified universal claim). **Both A and B require the same
  one load-bearing edit: the do-not-build register A-group preamble, lines 41-42**
  — the only fully-universal, un-era-qualified carrier today (product.md 74/90-96
  and README/CHANGELOG "held every time" are secondary; the runbook L17-36 already
  models the target voice). `research/SYNTHESIS.md` line 14 is historical — leave.
- **Follow-ons (recommend, not started):** (1) scorecard NaN/`min_btl=0.00`
  hardening — cheap, real, makes the MinBTL veto operative — highest value; (2) a
  *stated limit* on 2017-18 cost realism, NOT a modeling feature (negative ROI on a
  window we never advise on); (3) NO crown-credibility-band change (band is per-run
  honest; era context goes in prose/report).

---

### Gate results (verbatim, run before this note landed; re-run after write below)

- `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (119 / 119)`
- `python3 scripts/spec_lint.py` → `spec-lint: PASS (0 violations)`

(This note is a `spec/dev-notes/` memo — not an anchored report, not under
`spec/**/reports/`, adds no `[[req]]`/trace obligation. Post-write gate results
are reported in the handoff summary.)
