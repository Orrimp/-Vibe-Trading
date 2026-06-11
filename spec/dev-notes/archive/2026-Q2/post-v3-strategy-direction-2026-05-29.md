---
title: Post-v3 strategy direction — survey for next-week planning
date: 2026-05-29
authors: [analyst]
status: survey
tags: [survey, strategy, post-v3, next-direction, operator-decide, planning, dev-note]
related:
  - spec/dev-notes/strategy-reformulation-survey-2026-05-22.md
  - spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md
  - spec/v3-regime-classifier/feature.md
  - spec/v3-volatility-forecaster/feature.md
  - spec/v3-llm-forecaster/feature.md
  - spec/product.md
  - spec/backlog.md
---

# Post-v3 strategy direction — survey for next-week planning

> **This is a SURVEY / dev-note, NOT a feature brief.** No new `[[req]]`
> rows, no Queue → Active promotion, no feature folder, no code commitment.
> The analyst tabulates honest accounting of the v3 three-pick set,
> surfaces what strategy ideas remain live in the backlog, re-reads the
> 2026-05-22 survey for candidates the operator did NOT pick, and
> presents 4 prioritized routes for next-week planning input. Operator
> picks one (or none) of the routes; that decision spawns the next
> analyst pass.

## v3 three-pick post-mortem (honest accounting)

The 2026-05-22 strategy-reformulation survey rated **Candidate 1
(volatility forecasting)**, **Candidate 2 (regime classification)**, and
**Candidate 5 (LLM-as-forecaster)** as the top three picks under the
operator's Q-PICK + Q-SEQ HYBRID resolution. All three shipped some form
of v0.1.0 within the 16-week cumulative-cap window. Net result on the
+0.10 Sharpe-delta alpha-unlock gate:

| Pick | Slug | Shipped | Disposition | Sharpe-delta vs v1 baseline |
|------|------|---------|-------------|------------------------------|
| C1 | `v3-volatility-forecaster v0.1.0` | 2026-05-22 (then noop-fix) | **RETIRED** 2026-05-22 (NEGATIVE-NET-DELTA) | **-0.022** (close-to-zero; overlay equity dropped 44.6% vs un-targeted baseline once vol-targeting was real-wired) |
| C2 | `v3-regime-classifier v0.1.0` | 2026-05-28 (5 dev waves) | **RETIRED** 2026-05-29 (T-REG-NO-ALPHA + V-REG-5 classifier-fails-to-separate) | **-0.294** (strong negative; dispatcher actively LOST money vs un-conditional v1 momentum) |
| C5 | `v3-llm-forecaster v0.1.0-PARTIAL` | 2026-05-22 (operator-approved partial) | **SHIPPED-PARTIAL** (Sharpe-delta inconclusive; determinism + sample-size questions never closed) | **inconclusive** at PARTIAL ship; standing-Q to re-evaluate at follow-on but moat-aligned budget allocation already spent |

**Per-pick mechanism read.**

- **C1 vol forecaster.** The model "worked" in calibration terms
  (GARCH(1,1) baseline cleared H4 vol-is-predictable; QLIKE improvement
  over constant-σ baseline confirmed). The mechanism that killed it:
  vol-targeting overlay on hourly crypto needs the σ-scale to actually
  *help* on baseline-momentum entries, and instead the post-noop-fix
  empirics showed GARCH under-prediction × upper-clamp saturation →
  over-leveraged into drawdown bands. The vol-clustering signal exists;
  the downstream consumer shape (vol-targeting overlay) did not
  monetize it on this baseline. Close-to-zero NEGATIVE means "real
  evidence that the overlay neither helps nor catastrophically hurts —
  it just doesn't lift Sharpe." Cheapest of the three retirements.

- **C2 regime classifier.** The Markov-switching 4-state classifier
  ran (Wave A 14/14 unit tests PASS; convergence on synthetic
  4-regime GBM-with-switch confirmed). The mechanism that killed it:
  V-REG-5 ("classifier fails to separate regimes meaningfully on
  real-Binance data") — the per-bar posteriors on hourly crypto
  don't show distinguishable Bull/Bear/Volatile/Calm clusters that
  map onto downstream strategy-routing decisions. Empirically the
  dispatcher was switching between MomentumStrategy and
  CashHoldStrategy at unhelpful times, and the resulting overlay
  equity lost 29.4% relative to un-conditional momentum. This is the
  strongest signal from the three-pick set: **regime structure is
  either not present at hourly cadence on this universe, or not
  extractable by a 4-state Markov-switching classifier in the time
  budget we gave it.**

- **C5 LLM-as-forecaster.** PARTIAL shipped 2026-05-22. The Phase F
  Assistant slot lit up (Q4=(c) deliverable shipped); the operator
  *sees* the LLM reasoning + retrieved lesson cards live (genuine
  product-differentiator surface). The mechanism that left it
  inconclusive: determinism gates around the Anthropic API + the
  replay-cache + the sample-size cost-bound (R5.4 fire-every-24-bars
  ≈ 3,650 calls/year/symbol) never closed enough to lock a clean
  Sharpe-delta verdict. PARTIAL ship was operator-approved as
  scientific record + UX surface, not as alpha-unlock evidence.
  Standing-Q for follow-on was filed but is not the load-bearing
  next-step.

**Net empirical read.** The 2026-05-22 survey hypothesis was:
*"predict-something-other-than-μ (vol / regime / structured LLM
signal) is information-theoretically independent of the v2.5 DL-on-OHLCV
F4 evidence and may unlock alpha on the v1 cross-sectional momentum
baseline."* After three picks: **the reformulation hypothesis does not
land alpha on hourly crypto + v1 momentum.** The signals exist (vol
clusters; LLM emits structured opinions); they don't translate to
+0.10 Sharpe-delta via the overlay-on-momentum integration mode the
v3 three-pick set used.

**What this empirically tells us (three readings, ranked by analyst
confidence):**

1. **(HIGH)** The **integration mode** (overlay-on-v1-momentum) may be
   the limiting axis, not the signal source. Both C1 vol-targeting +
   C2 regime-dispatching wrapped v1 momentum in a multiplicative or
   gating shape. If v1 momentum's edge already implicitly conditions on
   vol + regime (per the K-reg-3 / K-vol-K5 risk register entries
   surfaced at brief-time), wrapping it with an explicit conditioning
   layer can only subtract, not add.

2. **(MEDIUM)** The **v1 momentum baseline is near-frontier** for
   crypto-hourly + this universe (10 USDT pairs, 2023-2024). Three
   independent v3 reformulations + the joint v2.5 F4-F4-F4 DL chain
   = four orthogonal signal-axes tested, all failing to clear +0.10
   Sharpe-delta vs the same baseline. The simplest hypothesis is
   the baseline is genuinely strong; *any* overlay pays turnover-drag
   without sufficient signal to overcome it.

3. **(LOW-MEDIUM)** The **asset class / cadence** itself is
   adversarial. Crypto-hourly has fat tails + microstructure noise
   + regime-shift speed that defeats both DL signal extraction and
   classical regime/vol modeling on hourly-OHLCV-only inputs.

Reading 1 implies "try a different integration mode." Reading 2
implies "stop trying to beat v1; spend engineering elsewhere."
Reading 3 implies "try a different asset class or cadence." These
are the three load-bearing implications behind Routes A/C/B below.

## What strategy ideas remain live (backlog scan)

After purging the v3 three-pick exhaustion + the v2.5 retirements,
the remaining strategy-relevant ideas in `spec/backlog.md` are:

- **v2.5b vanilla Transformer overlay** — RETIRED 2026-05-22
  (operator routing (a) at v25a-patchtst ship). The 3rd DL paradigm
  was pre-emptively retired on joint F4-F4 prior; v3 retirement set
  reinforces the call. **Not live.**

- **v2.6 forecast bake-off** — RETIRED 2026-05-22; premise moot once
  2 of 3 paradigms F4'd. **Not live.**

- **v3 LLM-as-forecaster v0.2.0 follow-on** — Standing-Q from C5's
  v0.1.0-PARTIAL ship. Would close the determinism + sample-size
  questions. *Live but de-prioritized*: budget already allocated, the
  PARTIAL ship is honest about not having locked Sharpe-delta.
  Re-funding requires a separate operator-decide.

- **`v2x-trading-state-bus` (v2 LLM evolution)** — Queue / Process
  candidate. Refactor pattern that replaces ad-hoc parameter threading
  in the v2 LLM agent pipeline with an owned `TradingState` struct
  (TradingAgents-style). **Architectural prep, not alpha-extraction.**
  Could unlock cleaner future LLM strategy variants. Analyst-strawman:
  this is process-tooling cost, would prep ground for any LLM-strategy
  follow-on but adds no alpha standalone.

- **v2.1 cockpit LLM-budget tile + tracing redactor + clippy cleanup**
  — Queue / Process; closes 3 deferred items from v2-llm-strategy
  v2.0.0 ship. **Hygiene work, not strategy.**

- **Candidate 7 — Strategy-side reformulation** (from the 2026-05-22
  survey) — full multi-strategy ensemble / risk-parity / alpha-blending
  framework. Was the highest-spend candidate (~6-10 weeks); operator
  did not pick at 2026-05-22 because building blocks (C1/C2) needed
  to land first. **Now genuinely live as Route A candidate** — see
  below — but with the honest caveat that C1+C2 building blocks
  retired NEGATIVE; the ensemble shape may have less to ensemble.

**Net live-strategy-idea inventory:** the backlog has been substantially
emptied by the v2.5 + v3 retirement waves. There is no
"just-promote-the-next-Queue-row" path that obviously extracts alpha;
all remaining strategy ideas need a fresh analyst pass to re-justify
their priors.

## Reformulation candidates the v3 three-pick did NOT pick

Going back to the 2026-05-22 survey's 7-row candidate table:

- **Candidate 3 — 168h horizon retest.** Was rated LOW prior at survey
  time (joint F4-F4-F4 evidence weakly predictive that horizon-axis
  exhaustion already happened at 1h+24h). **Updated 2026-05-29 read:**
  v3 retirements add another layer of evidence the *task framing*,
  not the *horizon*, is the load-bearing axis. Candidate 3's prior
  drops further; not recommended.

- **Candidate 4 — Crypto-specific features (funding rate, OI, perp
  basis).** Was rated MEDIUM prior at survey time; long data-bootstrap
  tail (1-2 weeks dev work to source funding rate + OI from Binance
  perp endpoints). **Updated 2026-05-29 read:** Reading 1 above
  ("integration mode is the limiting axis") gives Candidate 4 a
  *slight* uplift — different *input* with the *same task framing*
  is genuinely independent of C1/C2/C5. But the underlying caveat
  stays: we trade spot in this product, funding rate is a sentiment
  proxy not a direct execution signal. Prior stays MEDIUM. **Could
  fare differently than C1/C2/C5: yes, modestly — funding rate
  signal is documented independent of vol/regime/LLM axes.** Cost
  ~5-8 weeks per the 2026-05-22 survey row.

- **Candidate 6 — Non-DL approaches (XGBoost / Kalman / kernel
  ridge / GAM / ESN).** Was rated LOW-MEDIUM prior at survey time.
  **Updated 2026-05-29 read:** this is the candidate the v3 retirement
  set arguably *strengthens* the case for. The cheap-first hypothesis
  (retrospective lesson #1) says "low-capacity models underfit-by-
  design in a way that suits low-SNR data." After three v3
  reformulations failed, the question shifts from "is the task hard"
  (answered: yes) to "is there a low-capacity model that can extract
  the small-but-real signal without overfitting." XGBoost is the
  industry-default tabular forecaster; tries cheaply (training in
  minutes, not days). **Could fare differently than C1/C2/C5: yes,
  modestly — different model-class axis is the cheapest remaining
  orthogonal test.** Cost ~4-6 weeks per the 2026-05-22 survey row.

- **Candidate 7 — Strategy-side reformulation entirely.** Was the
  highest-EV-on-success / highest-spend candidate at survey time.
  **Updated 2026-05-29 read:** without C1+C2 working as building
  blocks, the ensemble shape has weaker inputs. Risk-parity portfolio
  construction at the symbol level *might* extract something
  independent of the v1 cross-sectional momentum baseline (it changes
  the baseline). But scope is large (6-10+ weeks); reversibility is
  LOWER (sticky ADR amendments to strategy registry). **Could fare
  differently than C1/C2/C5: yes, but at significant cost.**

## Operator-decide framing for next-week planning

Four routes, ranked by analyst's estimate of EV-per-week:

### Route A — Continue exploring within DL/LLM/regime/vol framing

**Best specific candidate:** Candidate 6 (non-DL approaches, XGBoost
preferred). Rationale: cheapest remaining orthogonal test (~4-6 weeks
total, tiny compute); information-theoretically independent of the v3
three-pick set (different model-class axis); cheap-to-falsify on a
clean evidence basis.

Alternates within Route A: Candidate 4 (crypto-specific features,
~5-8 weeks); v3-llm-forecaster v0.2.0 follow-on (~3-4 weeks if budget
re-allocated; closes the C5 determinism + sample-size standing-Q).

**Prior of clearing +0.10 Sharpe-delta:** LOW-MEDIUM (per the
2026-05-22 prior, lightly downgraded after the v3 retirement evidence
chain — though Candidate 6 specifically has the "low-capacity model
on low-SNR data" prior that the v3 retirements don't directly
falsify).

**EV-per-week:** moderate. The "is this asset class genuinely
adversarial" question gets one more cheap data point; if XGBoost also
F4s, the case for Route C strengthens.

### Route B — Pivot to a different asset class

Drop crypto-hourly + v1-momentum-baseline; test the same strategy
patterns on equities (e.g. S&P 500 sector ETF momentum) or futures
(e.g. front-month commodity contracts). Reuses the entire
`crates/forecast` + `crates/strategy` + `crates/audit` + realdata
backtest infrastructure with a new data source.

**Why this could fare differently:** equity factor research literature
(Moreira & Muir 2017 vol-targeting; Asness/Frazzini/Pedersen vol-scaling)
shows +0.1-0.3 Sharpe lift on momentum on equities — substantially
larger than the crypto-hourly empirics we just gathered. The transaction
cost regime is different (equities at daily cadence: ~1 bp; crypto-hourly:
~5-10 bps round-trip), and the vol-clustering structure compounds
differently across daily-equity vs hourly-crypto.

**Prior of clearing +0.10 Sharpe-delta on a new baseline:** MEDIUM-HIGH
for vol-targeting on equity-daily-momentum (textbook precedent);
UNKNOWN for re-running v1 cross-sectional momentum on equity tickers
(needs a fresh baseline-establishment pass first).

**EV-per-week:** moderate-to-high, but with a data-sourcing tail
(equity OHLCV bootstrapping; Yahoo/Polygon/Bloomberg data quality
trade-offs already partly explored by the `lab-yahoo-realdata` chain).
Cost ~6-10 weeks for a meaningful baseline + one overlay test.

**Caveat:** changes the product framing substantially. Per product.md,
this is a *crypto* trading agent; pivoting to equities is a strategic
product-direction decision, not just a strategy-track decision. Operator
must explicitly route this as a product direction shift.

### Route C — Accept v1 momentum baseline is near-frontier; engineer elsewhere

The reading-2 hypothesis ("v1 momentum is genuinely near-frontier for
crypto-hourly") is, after the v3 retirements, the simplest explanation
of the entire evidence chain. If true, the highest-EV use of the next
several weeks is NOT more strategy R&D — it's:

- **Paper-trade-live infrastructure** — the v3 success criterion per
  product.md (30 days paper without risk-limit breach +
  cost-within-budget). The agent we already have (v1 momentum +
  audited execution + Phase F UI) hasn't been continuously paper-traded
  long enough to establish the live-deployment readiness gate.
- **UI polish / Lumen Phase 6 Assistant slot** — gated on v2 LLM but
  the Phase F surface itself can compound product-differentiation
  without strategy-side alpha.
- **Process / tooling investment** — `v2x-trading-state-bus`,
  `v2.1 cockpit LLM-budget tile`, testing harness Week-2/3/4 follow-ons.
  These compound engineering velocity for *any* future strategy work.
- **ML productivity tools** — better backtest reporting, regime-aware
  performance attribution UI, walk-forward retraining infra.

**Prior of clearing any alpha gate:** N/A — this route explicitly says
*stop trying for now*. The success metric here is the v3 success
criterion (paper-trading continuity) + product-differentiator
compounding.

**EV-per-week:** HIGH on the product-differentiator axis (per
product.md § Differentiator the moat is (2) persistent reflection
memory + (4) auditable double-entry ledger; both are infrastructure-
sticky engineering investments that compound regardless of strategy
outcome). LOW on the alpha axis (by construction).

**Caveat:** the operator's research-budget pivot decision on
2026-05-22 explicitly said "free up ~3-5 weeks of compute budget…
pivot research budget to strategy-side reformulation or other work."
Route C is the "or other work" branch surfaced explicitly.

### Route D — Pure-research deep-dive (no code commitment)

A 2-week analyst-only deep-dive on one of:

- **What are the actual edge sources of cross-sectional momentum on
  crypto-hourly?** Decompose v1's Sharpe-by-symbol, Sharpe-by-regime
  (using C2's classifier as a *measurement tool* rather than an
  overlay), Sharpe-by-vol-percentile (using C1's GARCH fitter as
  measurement). If we understand WHY v1 works at this Sharpe level,
  we know what could augment vs what would only subtract.
- **A literature-review pass on crypto-hourly + cross-sectional
  momentum** — the survey's bibliography was thin on crypto-specific
  hourly results; a focused 1-week pass on recent papers (2023-2026)
  might surface signal axes none of the seven survey candidates
  covered.
- **A reflection-memory deep-dive on the v3 retirement evidence** —
  what do the C1/C2 backtest reports + v25a F-verdict reports +
  threshold-tuning + recalibrate evidence chain *jointly* say
  about what next-bar information lives in this data? An evidence-
  consolidation pass before committing to another multi-week R&D
  spend.

**Prior of clearing alpha gate:** N/A — pure research; output is a
better-informed analyst pass for whatever Route A/B/C the operator
picks afterward.

**EV-per-week:** moderate. Buys evidence consolidation cheaply but
delays any code-side commitment by ~2 weeks. Best paired with one
of A/B/C as a sequencing decision (e.g. "Route D then re-evaluate").

## Pre-drawn 4-cell verdict tree (durable-contract framing)

Per the durable-contract (cheap fallback labels for fast experiments;
durable labels for multi-month investments), applied to each route's
worst-case outcome:

| Route | If empirics SHIP-ALPHA | If empirics V-MARGINAL | If empirics V-FAIL | If empirics ABANDON |
|-------|------------------------|------------------------|--------------------|--------------------|
| **A — non-DL/feature/LLM-follow-on** | LOW-cost win: ~4-6 week investment, retire-keepable code, evidence consolidates with v3 set. Promote to v0.2.0 follow-on. | LOW-cost partial: similar to C5's PARTIAL ship pattern; ship for evidence record, do not promote to live. | Cheapest exit: F4-equivalent retirement aligned with v3 retirement pattern; ~1-week retirement dev-note. | Salvageable: model-class infra (XGBoost training scaffold or new feature module) stays as substrate for future bets. **Cognitive investment preserved: HIGH.** |
| **B — different asset class** | HIGH-impact: opens a new strategy lane with stronger textbook prior; substantial product-direction implication (re-document moat). | MEDIUM-impact partial: baseline-establishment on new universe is itself a transferable infra investment; some alpha but not enough to pivot product. | Costly exit: ~6-10 weeks spent + new data pipeline maintained without alpha proof. Equity OHLCV pipeline still has standalone value. | **Cognitive investment preserved: MEDIUM** (data pipeline + cross-asset infra stays; strategy-pattern lessons may not transfer back). |
| **C — engineering elsewhere** | N/A by construction — alpha is not the success criterion. Success = paper-trade-live continuity + product-differentiator compounding. | N/A by construction. | N/A by construction. | **Cognitive investment preserved: HIGHEST.** Every hour of paper-trade-live infra / UI / tooling investment compounds regardless of any future strategy R&D outcome. |
| **D — pure research deep-dive** | Output = sharper next analyst pass; no immediate alpha read. | Same — output is evidence consolidation. | Same. | **Cognitive investment preserved: HIGH** as a feeder to A/B/C. Analyst writeup is durable regardless of follow-on route choice. |

**Cell semantics:** ALPHA = Sharpe-delta ≥ +0.10 vs whichever baseline
the route targets; MARGINAL = [+0.05, +0.10); FAIL = < +0.05 net;
ABANDON = K-risk trip or scope blow-up forces early exit.

## Analyst-recommended route (per durable contract)

**Route C — engineering elsewhere** preserves cognitive investment best
regardless of outcome. The durable-contract logic: every other route
(A/B/D) has at least one cell where the cognitive investment is partly
lost (sunk into a strategy lane that retires NEGATIVE, into an
asset-class data pipeline that isn't used, or into a research deep-dive
whose conclusions are superseded). Route C, by contrast, ships
infrastructure (paper-trade-live continuity, Phase F polish, process
tooling) whose value is invariant to alpha outcomes. **All four cells
collapse to "high cognitive investment preserved" because the success
criterion is not alpha-bearing.**

Sequencing recommendation: **Route C as the dominant branch for the
next 4-6 weeks**, paired with a **lightweight Route D-style evidence
consolidation pass** (~3-5 days analyst-only) in the same window to
prep the ground for any future strategy R&D. If the operator
specifically wants one more alpha attempt, **Route A with Candidate 6
(XGBoost / non-DL)** is the cheapest defensible next-bet because
the analyst's confidence in reading 1 ("integration mode is the
limiting axis") is HIGH and Candidate 6 is the only remaining axis
that tests the model-class assumption cheaply.

Routes B and D-standalone are not recommended at this time: Route B
requires a substantial product-direction commitment (crypto → equity)
the operator hasn't surfaced as needed; Route D-standalone delays
all code investment by ~2 weeks without a clear next-bet anchor.

## Cross-references

- Strategy reformulation survey (2026-05-22) — [`strategy-reformulation-survey-2026-05-22.md`](strategy-reformulation-survey-2026-05-22.md)
- v2.5 DL journey retrospective (2026-05-22) — [`v25-dl-journey-retrospective-2026-05-22.md`](v25-dl-journey-retrospective-2026-05-22.md)
- v3 vol-overlay noop discovery (2026-05-22) — [`v3-vol-overlay-noop-discovery-2026-05-22.md`](v3-vol-overlay-noop-discovery-2026-05-22.md)
- C1 retirement record — [`spec/v3-volatility-forecaster/feature.md`](../v3-volatility-forecaster/feature.md)
- C2 retirement record — [`spec/v3-regime-classifier/feature.md`](../v3-regime-classifier/feature.md)
- C5 PARTIAL ship record — [`spec/v3-llm-forecaster/feature.md`](../v3-llm-forecaster/feature.md)
- Product framing constraints — [`spec/product.md`](../product.md)
- Backlog (Queue / Recent / Strategy section) — [`spec/backlog.md`](../backlog.md)
