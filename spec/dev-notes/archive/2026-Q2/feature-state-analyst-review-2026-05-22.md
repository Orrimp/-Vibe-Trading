---
slug: feature-state-analyst-review-2026-05-22
date: 2026-05-22
authors: analyst
status: proposed
related:
  - spec/dev-notes/feature-state-table-2026-05-22.md
  - spec/dev-notes/strategy-reformulation-survey-2026-05-22.md
  - spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md
  - spec/dev-notes/v3-vol-retirement-and-c5-promotion-2026-05-22.md
  - spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md
  - spec/product.md
  - spec/backlog.md
---

# Feature-state table — analyst review (market / strategy / product lens)

> **Sibling perspective** to `feature-state-table-2026-05-22.md`. The
> orchestrator's table is an inventory. This dev-note is a critique — does
> the work that landed match the moat? Were the alpha hypotheses sound?
> What's the highest-EV next move from a market/product perspective (NOT
> from an engineering throughput perspective)?
>
> Read-only on the source table. No new `[[req]]` rows. No code commitment.

## TL;DR

1. **The forecaster track was a misallocation versus the stated moat.** The
   moat in `spec/product.md § Differentiator` lines 78-83 is "(2) +
   (4) — persistent reflection memory + auditable double-entry ledger." 9 of
   13 features in Group A (the v25-dl + v3-vol arc) pursued
   single-asset-forecast-→-strategy-structure alpha, which strengthens
   neither moat component. We have **four retirements** (v25-dl umbrella +
   v25b-transformer + v26-bakeoff + v3-vol + v3-vol-rebaseline) and one
   shipped-partial (v3-llm) before we reached a moat-aligned strategy.

2. **A systematic blind spot: every retired-or-marginal forecaster
   measured "predict a 1-d univariate target → use as overlay signal."**
   The alpha sources we have NOT tested are the ones a vault of audited,
   reflectively-remembered trades is uniquely positioned to exploit:
   regime-conditional execution, cross-pair correlation breaks,
   memory-retrieval-as-signal, post-fill-attribution-as-feature. The v3-llm
   ship is the first time we routed budget toward a moat-aligned source.

3. **"Live trading is the unfilled gap" is correct but the framing is
   dangerous.** The orchestrator's table identifies live as the largest
   EV-per-effort unfilled slot. Reality check: v1 momentum's "positive
   Sharpe" baseline is ~13% return on 2023-FY at **73% max drawdown** on
   real Binance data — that's a 5.4× drawdown-to-return ratio, and the
   product has no risk engine, no kill-switch wiring beyond the `.halt`
   file, no daily-loss stop. "Go live tomorrow" is **NOT** the highest-EV
   move; "build the risk envelope that makes live safe and surface the
   right operator-success-report KPIs" is. Live without that envelope is a
   tempting-but-premature play.

4. **The 54-feature inventory is healthy at the spec-density and crate-
   reuse axes but unhealthy at the "what is the project actually
   building?" axis.** 36 shipped + 2 shipped-partial + 5
   retired/deprecated in ~5 weeks is high velocity. But strip out
   research-retirements and infrastructure plumbing, and the
   *product-defining* surface — the moat surfaces the operator can SHOW
   another human and say "this is why this project is worth my time" — is
   smaller than it looks. **Reflection-memory (`crates/reflection`) +
   double-entry audit ledger are the moat. Everything else is in service
   of them or accidental.**

5. **My recommendation for the next 4-6 weeks: stop the forecaster-search
   bias.** The retire chain pattern (3 DL + 1 GARCH retired) is evidence
   we keep asking the wrong question. Instead, the load-bearing question
   for the next month is: **"Can a paper-trading session produce an
   operator-success-report that makes the moat visible — i.e. a report
   where lesson-card retrieval directly correlated with PnL and the
   double-entry ledger reconciled every cent?"** Answering yes ships the
   product. Answering no surfaces the actual missing piece (most likely:
   risk engine + lesson-card writer pipeline integration into a live
   strategy that ISN'T just the v1 momentum baseline).

---

## A — The alpha story across 4 retirements

### Per-retirement post-mortem (the table inside the table)

| # | Retirement | Alpha hypothesis | Evidence verdict | What it actually tested |
|---|---|---|---|---|
| 1 | `v25-tcn-overlay` retired @ 1h | "TCN can predict next-1h log-return well enough to overlay v1 momentum and unlock +0.10 Sharpe-delta" | F4 + T-MARGINAL (+0.018 / +0.045) | Can a convolutional architecture extract direction from 5-feature hourly OHLCV? **No.** |
| 2 | `v25a-patchtst-overlay` retired @ 24h | "Patch-attention transformer + longer horizon unlocks signal where TCN failed" | F4 + Sharpe-delta +0.006 (LOWER than retired TCN) | Can a patch-attention architecture extract direction at a longer horizon? **No, and the 24h-horizon-helps hypothesis (H1) was falsified.** |
| 3 | `v25-dl-forecast-overlay` umbrella retired | "v2.5-era DL on OHLCV can predict the next bar's μ" (paradigm-family hypothesis) | Joint F4-F4-F4 across 3 checkpoints / 2 model families / 2 horizons | Can ANY DL paradigm extract +0.10 Sharpe-delta from this task framing? **No.** The retrospective explicitly states "the model genuinely doesn't predict returns; throwing more compute won't fix that." |
| 4 | `v3-volatility-forecaster` retired | "GARCH(1,1) vol forecast + vol-targeting overlay extracts +0.10 Sharpe-delta via position-sizing rather than direction-prediction" | MODEL-BROKEN / NO-ALPHA / **NEGATIVE-NET-DELTA** (-0.021719); equity dropped 44.6% post-noop-fix | Can a vol-targeting risk-layer overlay unlock alpha when direction-prediction couldn't? **No, and the wiring-bug discovery means the prior "rebaseline confirms no-alpha" verdict was the right answer but reached via a no-op overlay.** |

### Patterns I see across all 4

**Pattern P1 — Every retired feature measured "single-target univariate
forecast → strategy-structure overlay on v1 momentum."** TCN, PatchTST, and
GARCH all share this architecture-shape:

```
[5-feature window per symbol] → [forecaster] → [scalar prediction (μ or σ)] → [overlay on v1 momentum]
```

The Sharpe-delta gate (+0.10 vs v1 baseline) is measured on
**v1 momentum + this overlay vs v1 momentum alone**. We have never measured
a forecaster against ANY OTHER baseline (e.g. equal-weighted, BTC-only,
hand-rolled regime-aware sizer). We have never measured a strategy that
DOESN'T sit on top of v1 momentum. v1 is load-bearing for every retirement
verdict.

This means: **if v1 momentum is structurally good, every overlay competing
for the same +0.10 Sharpe-delta is asking "can I add to something already
near its ceiling?"** which is the harder version of the question. The
question we've never asked is "is there a different baseline strategy
where these forecasts ARE load-bearing?"

**Pattern P2 — Three of four retirements failed the "predict the future"
task; one failed the "size the present" task.** v25-dl-* asked
"predict μ"; v3-vol asked "predict σ". Both lanes failed against the same
baseline, on the same data, at the same evaluation gate. The Bayesian
update from joint F4-F4-F4-(N3-vol) failure is **not "models are bad";
it's "the overlay-on-momentum task shape is wrong."** This is exactly
what the strategy-reformulation-survey Candidate 7 contemplated as the
unfunded reformulation hypothesis.

**Pattern P3 — Every retired feature was discoverable as low-EV by a
simpler upstream gate.** v25-tcn-recalibrate showed gate-survival was
40-89%; v25-tcn-threshold-tuning showed signal genuinely absent;
v25a-patchtst was a paradigm-family check that confirmed the upstream;
v3-vol's noop bug masked NEGATIVE alpha for 8 hours. **All four
retirements were predictable from the cheaper sibling test that ran
first**, but each cost multi-week training compute anyway. The cheap-
first principle (retrospective lesson #1) is now codified but applied
*after* the multi-week ship in each case. The lesson is not "be cheaper"
— the lesson is **"stop assuming the next paradigm will be different."**

**Pattern P4 — The noop-discovery is the most important finding in the
whole arc.** `v3-vol-overlay-noop-discovery-2026-05-22.md` documents that
**5 layers of gating (unit, clippy, anchor, architect M-T1, tester M-FINAL)
missed a complete-no-op overlay** because each layer tested an adjacent
property. The byte-identity anchor signature WAS the bug, but byte-identity
was treated as a feature, not a smoke-alarm. This is a more serious
finding than any single retirement: **our retire-chain evidence base is
only as trustworthy as our wiring contract, and the wiring contract just
failed in production for the first time.** The R2 forensic gate is now
in tree, but the pattern (does this overlay actually change the
load-bearing observable?) should be applied retroactively to v25-tcn and
v25a-patchtst overlays.

### The systematic blind spot

We have measured **zero** of the following alpha sources:

| Alpha source | Why we missed it | Why it's moat-aligned |
|---|---|---|
| Regime-conditional execution | C2 v3-regime-classifier stayed Queue-only; never spawned | Pure-fn `regime.rs` seed exists; reflection-memory + regime tags compose naturally |
| Cross-pair correlation breaks | Never in any survey; not in v3 candidates | Audit ledger records every fill across all 10 pairs; correlation-aware sizing is exactly what the ledger can prove |
| Market-impact-aware execution | Out of scope per product.md (no HFT, no real money) | But paper-trading slippage assumptions ARE in scope; nobody has audited whether `bps: 2` slippage at top-10 pairs is realistic at our position sizes |
| Memory-retrieval-as-signal (NOT as forecast input) | C5 v3-llm-forecaster routes lesson cards to LLM but doesn't USE retrieval count / lesson-card-PnL-correlation as a direct signal | This IS the moat by definition: lesson cards firing on similar past = a direct, ledger-defensible alpha source |
| Post-fill-attribution-as-feature | Audit ledger has this data but no strategy consumes it | Same as above |
| Order-flow / trade-aggressor / depth-imbalance | product.md § Data § Microstructure mentions these (spread, depth-imbalance, trade-aggressor ratio) but no shipped strategy uses them | Different signal class entirely from OHLCV; could be the underexploited axis |
| Cross-venue arb | product.md § Non-goals defers (no real money) | Not moat-aligned; correctly excluded |

**The blind spot pattern**: every alpha source we MISSED was either (a)
moat-aligned and tractable but never funded, or (b) microstructure-class
and structurally different from the OHLCV-window paradigm we kept
retrying.

---

## B — Product moat alignment

`spec/product.md § Differentiator` (lines 67-83) names four moat
components. Two are confirmed long-term bets:

> **(2) Persistent reflection memory** — every closed trade produces a
> lesson card; the trader retrieves relevant lessons before composing
> the next order.
>
> **(4) Auditable double-entry** — every decision and every cash/position
> move reconciles to a ledger.

The other two (Rust-native + type-encoded risk) are tablestakes — they
make the product run, but they don't differentiate it from a hypothetical
serious competitor.

### Feature-group mapping

| Feature group | Strengthens moat (2)+(4) | Anti-moat or moat-neutral | Net |
|---|---|---|---|
| **Group A strategy research (shipped: v0/v0.5/v1/v1.5a/v1.5b/v2)** | v2-llm-strategy (foundation for LLM-as-signal); v1.5b multi-venue (forces ledger to handle multi-venue cleanly) | v0/v0.5/v1/v1.5a are baselines — moat-neutral by design (none consume reflection memory; v0 doesn't even emit lesson cards) | **NEUTRAL** — necessary but not moat-defining |
| **Group A strategy research (retired: v25-dl umbrella + v25b + v26 + v3-vol + v3-vol-rebaseline)** | NONE — every retired feature was a pure-numeric forecaster that did not consume lesson cards or expose new audit surface | All 5 — they consumed multi-week budget against forecasters that, even if they had worked, would not have strengthened the moat (DL forecasts are commodities; GARCH is textbook) | **NET ANTI-MOAT** — 5 retirements represent ~10-15 weeks of agent + compute budget that could have gone to moat-aligned work |
| **Group A strategy research (`v3-llm-forecaster` shipped-partial)** | **YES** — first strategy that consumes reflection memory + audit at decision time; this is the first moat-aligned signal source the project has shipped | Wave D deferred indefinitely (real API key) — moat is half-built until the deferred wave lands | **NET PRO-MOAT** with execution risk |
| **Group B backtest + audit + data infra** | `reflection-memory` (THE moat surface); `journal-transactions-metadata`; `per-symbol-position-accounts`; `real-mtm-unrealized-pnl`; `audit-tick-consumer-envelope` — every Group B feature IS a moat foundation | None | **STRONGLY PRO-MOAT** — this group is the project's actual differentiator and represents some of the highest-EV completed work |
| **Group C cockpit / live trading** | `operator-success-reports` (LLM analyst over audit ledger = moat-visible); `live-cockpit-unified` provides the surface where the moat becomes operator-visible | `cockpit-app-bundle` candidate, `cockpit-render-regression` — necessary but moat-neutral | **NET PRO-MOAT (modest)** — operator-success-reports is the moat-visibility surface; this category is the right place to lean into |
| **Group D UI rethink (6 phases shipped)** | Phase F (Memory + Models + Assistant slot) directly exposes reflection memory and gates the v3-llm Assistant slot | Phases A-E are operator-facing polish; moat-neutral | **NET PRO-MOAT (one phase)** — Phase F lands the moat-visibility UI; everything else is necessary scaffolding |
| **Group E chart / canvas (4 features)** | NONE — chart canvas does not surface lesson cards, audit reconciliation, or memory retrieval | All 4 — pure UX polish | **NET MOAT-NEUTRAL or mildly ANTI-MOAT** — the time spent here was not moat-building. **Could collapse the 4 features into 1 retro-active "chart subsystem" feature for spec hygiene.** |
| **Group F UI infrastructure / testing (10 features)** | `ui-quality-gate-overhaul`, `ui-session-journal-iced-tester` — test infra that supports moat features | iced ecosystem evaluation + iced-aw cherry-pick + ui-drop-iced-aw — necessary tablestakes | **NET MOAT-NEUTRAL** — necessary plumbing. Density is high; this group reflects the engineering cost of running on iced 0.14. |
| **Group G design system + tape modal** | `lumen-design-adoption` provides the tokens that make moat-visibility UIs (Memory, Audit) coherent | `tape-row-audit-modal` is audit-visibility (mildly pro-moat) | **NET MILDLY PRO-MOAT** |

### The key question: did the forecaster track HURT the moat?

**Yes, in two ways:**

1. **Direct opportunity cost.** Per the strategy-reformulation-survey
   cost framing, each retired forecaster consumed 2-7 weeks of analyst +
   architect + dev + tester budget plus multi-day training compute. Total
   across the 4 retirements ≈ **10-15 weeks**. That budget could have
   funded any of: a paper-trading-live-readiness initiative, a
   lesson-card-PnL-correlation surface in operator-success-reports, a
   risk-engine first cut, or 1-2 more shipped strategies that DO consume
   reflection memory.

2. **Narrative cost.** The session-by-session orchestrator narrative has
   been "we're chasing alpha." That framing is correct given the
   strategy lifecycle gates in product.md, but it has also meant every
   decision-grade conversation has been about forecaster choice, not
   about moat-visibility choice. **The operator's mental model of "what
   this project IS" has drifted toward "ML research" and away from
   "auditable, memory-aware trading agent."** That drift is reversible
   but ought to be reversed deliberately in the next strategic move.

**Mitigating factors**: the v2.5 DL + v3-vol arc DID produce
non-trivial value:
- ADR-0033 § D3 immutable F-verdict algorithm — a reusable measurement
  bar for any future forecaster (high evidential value).
- ADR-0035 cross-phase σ_train recalibration pattern — caught a real
  608×/580× calibration bug; the lesson lives forward.
- ADR-0038 § D6.b anchor re-emission protocol — first response to the
  noop-discovery; reusable contract for future wiring-bug fixes.
- The R2 forensic-gate test pattern — protects against re-introducing
  the no-op category of bug forever.
- A **lot** of repeated evidence that the OHLCV-window-→-overlay task
  shape is exhausted — knowing this saves the next operator/agent from
  reproducing it.

The forecaster track was thus **anti-moat in opportunity cost but
pro-rigor in evidence quality**. Net: the project is more disciplined
at evaluating forecasters than it is at building the moat — which is
the wrong skill ratio for a project whose stated differentiator is
memory + audit.

---

## C — Pressure-testing "live trading is the unfilled gap"

The orchestrator's table closes with:

> **Live trading is the unfilled gap.** `live-cockpit-unified` v1.5.0
> exists, but no feature folder for "v1 momentum goes live" exists. This
> is the largest EV-per-effort unfilled slot.

I want to challenge this from a market/product perspective.

### Is v1 momentum's Sharpe robust enough to put in front of live order
flow?

**Short answer: probably not, but the binding constraint is drawdown,
not Sharpe.**

What we actually know about v1 momentum:

| Metric | Value | Source |
|---|---|---|
| 2023-FY return | ~13% (matches BTC buy-and-hold mediocre year) | `top10-2023-fy-momentum-realdata` baseline cited in noop-fix deck |
| Max drawdown (2023-FY) | **73.73%** | Same baseline |
| Trades | 6203 (top-10 USDT pairs, hourly) | Same |
| Drawdown-to-return ratio | 73.73 / 13.48 ≈ **5.5×** | Computed |
| Calmar (return / max-DD) | 13.48 / 73.73 ≈ **0.18** | Computed |
| Sharpe | Not visible in current anchors; positive but margin unclear | Inferred from "positive Sharpe across all 4 retire-chain comparisons" in orchestrator table |

**Read**: v1 momentum has a respectable return but a catastrophic
drawdown profile. The product.md strategy lifecycle gate
(`paper → live: Sharpe > 1.0 on 2y OOS data; no fatal regressions`)
appears met on the Sharpe axis but is **silent on drawdown** — which is
the binding risk constraint for live.

A 73% max drawdown on paper at simulated fills means a real
operator running this strategy live would have, somewhere in the past
year, watched their account go from $100k to $26k. That's a
margin-call-equivalent event on a paper book; on a live book it's a
career-ending event. **No live deployment should happen without:**

- A daily-loss-stop that fires at <10% drawdown.
- A max-drawdown circuit-breaker at <30% that flatlines and pages.
- Pre-trade slippage modelling that doesn't assume 2bps on a strategy
  doing 6k trades/year across 10 pairs.

### Confidence on the retire-chain baseline numbers

| Comparison | Baseline used | My confidence |
|---|---|---|
| v25-tcn alpha investigation | v1 momentum on `top10-2023-1h-momentum` SYNTHETIC GBM | LOW — the synthetic-vs-real caveat invalidated this once |
| v25-tcn threshold tuning | Same synthetic baseline | LOW |
| v25a-patchtst | Real Binance OHLCV via `top10-2023-fy-momentum-realdata` (added during the v3-vol-rebaseline pass) | MEDIUM-HIGH (real data; deterministic) |
| v3-vol parent | SYNTHETIC GBM baseline (`mean_calibration_ratio` reading was honest but baseline-comparison was apples-to-oranges) | LOW |
| v3-vol-rebaseline | Real Binance baseline | MEDIUM but had the noop-bug masking real wiring |
| v3-vol-noop-fix | Real Binance baseline + real wiring | **HIGH** — first apples-to-apples real-vs-real comparison in the v3 track |

**Observation**: the project has been computing Sharpe-deltas against
mixed-trustworthiness baselines for 6+ weeks. Only the most recent
v3-vol-noop-fix verdict is on apples-to-apples real wiring. That doesn't
invalidate the joint F4-F4-F4 retirement (the signal genuinely isn't
there at scale that matters), but it does mean: **the "v1 momentum
baseline" number we keep citing is computed differently in different
ships.** If we are going to use v1 as the live-trading reference
strategy, we need ONE canonical baseline run that all future
comparisons cite by anchor SHA.

### Failure modes if v1 went live tomorrow

1. **Slippage divergence**: `bps: 2` slippage assumption in the
   simulator vs real Binance order-book at top-10 USDT depths. For
   pairs like BTCUSDT/ETHUSDT this is probably fine; for ranks 6-10
   (e.g. AVAX, MATIC) at certain hours, real slippage is 5-20 bps.
   This eats ~50-75% of v1's marginal alpha in stress.
2. **Regime change vs the 2023-2024 training window**. v1 was anchored
   on a market regime (post-FTX recovery + ETF anticipation + Q4 2023
   rally) that may not generalize. We have NO out-of-window forward-
   tested data, only the 2y backtest span.
3. **Capital cap from cost-economics ladder**. product.md v0 ceiling is
   $45/month total; live trading on real venue would blow this if the
   v0 strategy ran ANY LLM calls per bar. v3-llm-forecaster is a
   $25-50/day proposition at peak; running it live in any quantity
   needs the cost-degrade-to-quick-think gate (product.md cost
   economics § Hard rule) to actually trip and be tested.
4. **Risk engine gap**. product.md § Risk management hard requirements
   names 5 things (typed limits, kill switch, daily-loss stop,
   per-symbol cap, max-leverage, max-drawdown trigger). Of these, the
   one that's clearly shipped is the `.halt` file kill switch. **Daily-
   loss stop, per-symbol cap, max-leverage, max-drawdown trigger — I
   cannot find clear feature ships for any of these.** The 73% max-DD
   in backtest would have tripped a max-drawdown circuit breaker daily
   if one existed. The risk engine is the binding gap, not live wiring.

### Is "go live" the highest-EV next move, or tempting-but-premature?

**Tempting-but-premature**, with caveats.

What "live" actually means in product.md context is **not** real-money
execution (explicitly out of scope per product.md § Non-goals: "Real-
money execution, KYC, exchange API keys, withdrawals. Out of scope for
this project."). The v3 success metric is *continuous paper-trading on
real market data with simulated fills.* So "go live" reduces to "run
v1 momentum continuously, 24/7, on real Binance market data, with
simulated fills, generating weekly operator-success-reports."

That framing changes the analysis:
- The 73% drawdown is on PAPER; no real money is at risk.
- The slippage divergence still matters (paper Sharpe diverges from
  hypothetical-real Sharpe).
- The risk engine gap STILL matters because the kill switch should
  trip in paper too — it's the demo of the moat.

**Reframed:** the highest-EV next move is **not "go live with v1"**.
It's **"prove the operator-success-report cycle works end-to-end on
continuous paper, with the audit ledger + reflection memory visibly
producing decision-grade output, on a strategy SAFER than v1 alone."**

That likely means: v1 momentum + a vol-targeting overlay (NOT GARCH;
maybe a Parkinson estimator or a hand-rolled rolling-σ) sized to keep
the worst-case drawdown to ~25%, running 24/7, with weekly LLM-generated
success reports that demonstrate the reflection memory loop closing.

The product is the operator-success-report cycle, not the strategy.
"Live" in product.md is the WHEN; "moat-visible" is the WHAT.

---

## D — Candidate routing recommendations

The orchestrator's table surfaces 4 open candidates. My read:

### D.1 — C2 v3-regime-classifier

| Field | Value |
|---|---|
| EV estimate | **MEDIUM** |
| Cost estimate | ~4-6 weeks (per strategy-reformulation-survey) |
| Risk profile | Moderate. K-reg-1/2/3 in the survey: regime taxonomies are subjective; two-stage pipelines compound noise; v1 momentum may already implicitly capture trending regimes. |
| Prerequisites | None (regime.rs seed exists; would extend in-place per the C2 draft's analyst-default) |
| Moat alignment | **LOW.** Regime classification by itself is not moat-defining — it's a textbook quant primitive. Becomes moat-aligned only if (a) the regime tags feed into lesson-card embedding (already partially true) AND (b) the strategy that consumes them is something memory-aware, not just regime-conditional v1. |
| Analyst recommendation | **DEFER** unless C5 v3-llm-forecaster F-equivalents and operator needs a fallback. The C5-active comment block explicitly retains C2 as the R-O3 fallback. That's the right disposition. Don't promote C2 now. |

### D.2 — v3-llm-forecaster v0.1.1 (Wave D)

| Field | Value |
|---|---|
| EV estimate | **HIGH** — but with HIGH variance |
| Cost estimate | ~half-day work + $25-50 LLM spend (per orchestrator table) |
| Risk profile | LOW execution risk (small scope: complete the backtest + canonical cache wave that was deferred for API-key reasons). HIGH outcome variance per K8 novel-territory risk on v0.1.0 itself. |
| Prerequisites | `ANTHROPIC_API_KEY` in operator env. |
| Moat alignment | **HIGHEST of any open candidate.** This IS the moat-aligned strategy: LLM-as-forecaster that consumes reflection-memory lesson cards + audit context → emits a typed signal. The product.md § Differentiator language (memory + audit) maps to it 1:1. |
| Analyst recommendation | **PROMOTE — fastest path to moat-visible alpha.** The half-day + $50 spend is the cheapest decision-grade evidence the project can buy right now: it tells us whether the moat-aligned signal source produces alpha. F-equivalent here is decision-grade for the project (says the moat-as-signal hypothesis is falsified at v0.1.0 scope; surface it as evidence for the moat AS a UI surface, not as a signal source). Even on F-equivalent, Wave D shipping completes the v3-llm story and validates the deferred-milestone activation contract precedent. |

### D.3 — Paper-trade-live (no feature folder)

| Field | Value |
|---|---|
| EV estimate | **HIGH but conditional on risk-engine readiness** |
| Cost estimate | Unbounded as scoped today; ~4-8 weeks if scoped tightly to "continuous v1 paper + weekly success report + kill-switch verified live" |
| Risk profile | HIGH if scoped naively ("just run v1 24/7"). LOW if scoped as "build the operator-success-report cycle that proves the moat is visible." |
| Prerequisites | **Three missing pieces, in priority order:** (1) Daily-loss-stop + max-drawdown circuit breaker (currently not shipped per my Group C analysis); (2) One canonical v1 momentum real-data baseline anchored once and cited forever; (3) Operator-success-report cadence wiring (currently on-demand only; needs scheduler). |
| Moat alignment | **HIGH if reframed as "operator-success-report cycle proves the moat."** LOW if reframed as "just run v1 in front of real money" (which is out of product scope anyway). |
| Analyst recommendation | **REFRAME, then promote.** Don't promote "paper-trade-live for v1 momentum." Promote "**continuous-paper-trading-operator-success-cycle**" — a feature that includes the risk envelope, the canonical baseline anchor, the success-report scheduler, and a strategy that actively consumes reflection memory (v3-llm or a stripped-down memory-overlay on v1). That feature delivers the v3 product success metric directly. |

### D.4 — cockpit-app-bundle

| Field | Value |
|---|---|
| EV estimate | **LOW for the project's stated goals** |
| Cost estimate | Likely 1-3 weeks (macOS .app signing + bundle) |
| Risk profile | Operational only. Signing certs, notarization, distribution. |
| Prerequisites | None |
| Moat alignment | **ZERO.** A distributable .app does not strengthen reflection memory or audit ledger. It's distribution polish for a single-operator product. |
| Analyst recommendation | **DEFER.** product.md is explicit about single-operator-forever ("single-operator forever. No auth, no RBAC."). The .app bundle is moat-neutral and serves a phantom audience. Promote only if operator personally wants their cockpit in the dock for ergonomic reasons. |

### D.5 — NEW candidate I'd add: "Risk envelope v0.1.0"

| Field | Value |
|---|---|
| EV estimate | **HIGH** — load-bearing for any future "go live" disposition |
| Cost estimate | ~2-3 weeks scoped tight: daily-loss-stop + max-drawdown trigger + per-symbol exposure cap as typed Rust pipeline upstream of strategy emit |
| Risk profile | LOW — codifies existing product.md § Risk management hard requirements |
| Prerequisites | None — extends existing audit ledger + per-symbol-position-accounts feature |
| Moat alignment | **MEDIUM** — type-encoded risk is moat component (3) per product.md; not the long-term moat but tablestakes for any live disposition |
| Analyst recommendation | **PROMOTE BEFORE paper-trade-live.** The risk envelope is the missing piece between "we have backtests with 73% max-DD" and "we can run continuous paper without panicking when the curve drops." It's also the cheapest thing the project can ship that converts product.md § Risk management § hard requirements from text into code-with-tests. |

### D.6 — NEW candidate I'd add: "Reflection-memory-PnL-correlation surface"

| Field | Value |
|---|---|
| EV estimate | **MEDIUM-HIGH** for moat-visibility (potentially the highest EV per dollar in the project right now) |
| Cost estimate | ~2-3 weeks scoped tight: extend operator-success-reports to include "lesson cards retrieved this week, sorted by PnL of trades that retrieved them" |
| Risk profile | LOW — extends existing feature; no new infra |
| Prerequisites | reflection-memory shipped (already true); operator-success-reports shipped (already true); requires at least one strategy that retrieves lesson cards in production (v3-llm-forecaster Wave A+C shipped this; Wave D backtest pending) |
| Moat alignment | **HIGHEST** — directly visible proof that the moat (memory + audit) produces operator-decidable evidence. |
| Analyst recommendation | **PROMOTE.** This is the single cheapest feature that turns the moat from "we built reflection memory" into "we have evidence that reflection memory makes the trader better." It's the report-side complement to v3-llm-forecaster's signal-side moat work. |

### Routing summary table

| Candidate | EV | Cost | Moat | Recommend |
|---|---|---|---|---|
| C2 v3-regime-classifier | MED | 4-6w | LOW | DEFER (R-O3 fallback) |
| v3-llm-forecaster Wave D | HIGH | 0.5d + $50 | HIGHEST | **PROMOTE FIRST** (cheapest decision-grade evidence) |
| paper-trade-live (reframed as continuous-success-cycle) | HIGH conditional | 4-8w | HIGH | PROMOTE AFTER risk envelope |
| cockpit-app-bundle | LOW | 1-3w | ZERO | DEFER |
| **Risk envelope v0.1.0 (new)** | HIGH | 2-3w | MED | **PROMOTE — gates everything else** |
| **Reflection-PnL-correlation surface (new)** | MED-HI | 2-3w | HIGHEST | **PROMOTE in parallel with risk envelope** |

---

## E — Pattern recognition across the inventory

### E.1 — Velocity

54 features in ~5 weeks (4 retired, 36 shipped, 2 partial, 2 draft, 3
candidate, 1 roadmap, 6 deprecated). **The velocity is high but the
velocity-of-moat-strengthening is much lower** — roughly 5-7 features
across the 54 directly strengthen reflection memory or audit
reconciliation (Group B reflection-memory + journal-tx-metadata +
per-symbol + real-mtm + audit-tick-envelope, plus Group C
operator-success-reports). That's ~10-13% of the inventory directly
moat-aligned.

Spec-driven workflow IS mature (anchor protocols, frontmatter,
retirement contracts) — that's a separate kind of velocity and it's
genuinely impressive. But velocity-of-feature-ship is not the right KPI
when 80%+ of features are moat-neutral or anti-moat.

### E.2 — Feature density / signal-to-noise

Several feature clusters that I'd argue could collapse retroactively
into single features for spec hygiene without losing evidential value:

| Cluster | Feature count | Retro-collapse target |
|---|---|---|
| Chart subsystem (chart-canvas-overhaul + chart-buy-sell-emphasis + chart-fixture-line-clipping + chart-x-axis-local-time) | 4 | "chart-subsystem v1.0.0" with the 4 as decomposed milestones |
| iced ecosystem (iced-aw-cherry-pick + ui-drop-iced-aw + iced-ecosystem-evaluation + iced-native-widgets) | 4 | "iced-stack v1.0.0" |
| UI test infra (ui-headless-emulator + ui-quality-gate-overhaul + ui-session-journal-iced-tester + ui-test-harness-bootstrap + ui-gallery-bin + ui-gallery-table-cell) | 6 | "ui-test-infra v1.0.0" |
| v25-tcn family (v25-tcn-overlay + alpha-investigation + recalibrate + threshold-tuning + horizon-bump-or-retire) | 5 | Already partially collapsed by retrospective; could close as a single shipped "v25-dl-overlay-retirement" historical surface |
| v3-vol family (v3-volatility-forecaster + rebaseline + noop-fix) | 3 | Could collapse to "v3-vol-retired" historical surface plus the standalone noop-fix as a P0 |

**Inferred signal: when the operator/orchestrator is in research mode,
features fork rapidly (each ship spawns 1-3 follow-ons). When in
infrastructure or UI mode, features cluster sensibly. The fork pattern
is what produced the 9 features in the v25-dl + v3-vol arc, and it's
the part of the inventory most concentrated in moat-neutral activity.**

Recommendation: at the next session, run a **spec-hygiene pass** to
mark the chart + iced + UI-test-infra clusters as "feature-family v1.0
shipped" so the inventory better reflects what actually exists as
mental load-bearing surface. This is cosmetic but reduces the cognitive
load of running future audits.

### E.3 — Anti-moat features in retrospect

| Feature | Why it might be anti-moat in retrospect |
|---|---|
| v25-dl-forecast-overlay umbrella + 4 children | Consumed multi-week budget on a paradigm whose retirement was knowable from joint TCN-F4 alone; the umbrella structure made it easy to commit to the next paradigm before fully digesting the prior one |
| v3-volatility-forecaster + rebaseline | Surfaced a noop bug that's now a useful regression test, but the underlying GARCH+vol-target hypothesis would have retired anyway per K-vol-1 in the survey (turnover eats lift on hourly crypto); the multi-week ship was avoidable with a cheaper analyst-pass |
| cockpit-app-bundle (as a candidate) | Phantom audience for a single-operator product |
| iced-ecosystem-evaluation (as a long-running candidate) | Operator-locked to iced 0.14 per CLAUDE.md vendor lock; the evaluation is closed but the candidate stays open as ambient debt |

None of these are "anti-moat" in the harmful sense — each produced real
evidence or unblocked a real path. But each represents budget that
*could* have gone to moat-strengthening if the operator had been
deliberate about moat-alignment as a routing criterion.

### E.4 — Healthy patterns I want to call out

- **The retirement contract precedent (v25-dl + v3-vol both retired
  cleanly with code + anchors preserved + dev-note explaining why) is
  excellent.** This is rare. Most projects don't retire research lines
  cleanly; they just stop talking about them. Our retirement
  protocol means future engineers/agents can trust that
  `status: retired` actually means "we looked, here's what we found,
  here's why we stopped."
- **The shipped-partial state (codified by v3-llm-forecaster Wave D
  deferral) is a precedent worth keeping.** External-dependency-
  deferral is sanctioned; we don't lose the WIP.
- **The R2 forensic-gate pattern from the noop-fix is a generalization
  worth promoting.** Every strategy-composition feature in the v3+ lane
  should ship with an end-to-end gate that asserts the overlay
  changes the load-bearing observable. The retrospective lesson #1 is
  not just "cheap-first" — it's "test that the wire is connected."

---

## F — The strategic question for the next 4-6 weeks

> Frame: "If I were running this project, the load-bearing question
> for the next month is X, and the EV-positive answer is Y."

### The load-bearing question

**Can a single 7-day continuous-paper-trading session produce an
operator-success-report that makes the moat operator-visible?**

By "moat operator-visible," I mean concretely:
- The report shows ≥10 trades whose lesson-card-retrieval correlated
  with PnL (memory loop visibly closing).
- The audit ledger reconciles to the cent across all 10 USDT pairs
  for the full session (double-entry visibly closing).
- The cockpit's Memory + Audit screens show live state that the
  operator can scan in <60 seconds and answer "is this working?"
- The kill switch trips at least once (synthetic trigger is fine)
  and the system flatlines + recovers cleanly.

This question subsumes:
- The risk engine question (kill switch trip requires a risk envelope).
- The continuous-paper question (7 days of uptime).
- The v3-llm-forecaster question (it's the strategy that produces the
  memory-loop trades).
- The operator-success-report cadence question (weekly cadence;
  generation tested).
- The "is v1 momentum enough?" question (NO; need memory-aware
  strategy).

If the answer is YES at 4-6 weeks, the project ships its v3 success
metric per product.md and moves to follow-up-project planning. If NO,
the failure mode tells us exactly what to build next (probably either
risk-engine first or strategy refactor to actually consume memory).

### The EV-positive answer (what I'd build for the next month)

A single feature: **`v3-continuous-paper-success-cycle`** that
composes:

1. **Risk envelope v0.1.0** (my new D.5 candidate) — daily-loss-stop
   + max-drawdown trigger + per-symbol cap, typed in Rust, with tests.
   ~1-2 weeks.
2. **Canonical v1 baseline anchor** — one definitive
   `top10-2023-2024-fy-momentum-realdata-canonical` anchor cited by
   every future comparison. ~1 day, but requires architectural
   discipline (deprecate the synthetic baseline in
   `sharpe_comparison.rs`).
3. **v3-llm-forecaster Wave D** — ship the deferred backtest + real
   API + canonical cache. 0.5 day + $50 LLM spend.
4. **Reflection-PnL-correlation surface in operator-success-reports**
   (my new D.6 candidate) — extend the existing reports to surface the
   memory-loop-PnL evidence. ~1-2 weeks.
5. **7-day continuous-paper run as the acceptance gate** — the
   feature ships when a single 7-day paper run produces a
   moat-visible operator-success-report end-to-end. ~1 week of
   wall-clock + monitoring.

Total: ~4-6 weeks for a single composed feature that EITHER ships the
v3 product success metric (the project's stated end-state) OR surfaces
the load-bearing gap that prevents shipping (so the operator can
allocate accordingly).

**Critically, this feature does NOT pursue more forecaster research.**
The forecaster-search bias is the load-bearing failure mode I want to
break. v3-llm-forecaster is in scope because it's already shipped-
partial and moat-aligned; everything else research-side is parked.

### Options considered and not chosen

| Option | Why not chosen |
|---|---|
| **Continue forecaster research (C2 or a new C6)** | Pattern P3 in section A: every retired feature was predictable from a cheaper sibling test. The Bayesian update says forecaster-search is exhausted at this evaluation gate. Adding C2 now repeats the pattern. |
| **Pivot to risk engine + market-impact modelling** | Risk engine is in scope (subsumed by D.5 candidate). Market-impact modelling is out of product scope (no real money) and would be cargo-cult quant work for this project. |
| **Pivot to non-crypto markets** | product.md is explicit about crypto-only ("we are crypto-only; tickers are exchange/symbol pairs, not stock symbols"). Pivoting markets is a different project; the moat (memory + audit) compounds the same way in any asset class but the data plumbing would consume the budget. |
| **Stop / consolidate / write sabbatical retrospective** | Tempting given the 4 retirements. But the project has not yet shipped its stated v3 success metric (continuous paper + success reports + memory loop demonstrably accumulating). Stopping now would mean leaving the actual product unshipped after building the entire scaffold for it. |
| **Just go live with v1** | Pressure-tested in Section C and rejected. v1 alone is moat-neutral; live without risk envelope is irresponsible (paper or otherwise); the v3 success metric requires memory loop visibility, which v1 alone doesn't produce. |

### What the next session-end revisit should ask

When the next 4-6 weeks closes, the analyst review should ask:
1. Did the moat become operator-visible in a success report?
2. Did the risk envelope trip at least once cleanly?
3. Did v3-llm-forecaster Wave D verdict surface (PASS / F-equivalent)?
4. Is the lesson-card-PnL-correlation a real signal or noise?
5. What's the next load-bearing question?

If the answer to (1) is YES, the project has shipped its product.
The remaining work is documentation + follow-up-project handoff per
product.md § Project scope boundary.

If the answer to (1) is NO, the failure mode is the question. The
analyst review then re-routes to whichever sub-question (risk, memory,
strategy, reports) was the binding constraint.

---

## Handoff envelope

```toml
[handoff]
from        = "analyst"
to          = "orchestrator"
feature     = "feature-state-analyst-review"
trace_refs  = []  # dev-note only; no new [[req]] rows
verdict     = "READY-FOR-OPERATOR-ROUTING"
priority    = "high"
notes       = """
Sibling-perspective analyst review of the 54-feature inventory. Critique
applies a market/strategy/product lens to the orchestrator's table.
Key conclusion: the forecaster track was anti-moat in opportunity cost
(10-15 weeks of budget that didn't strengthen reflection memory + audit
ledger), and the highest-EV next move is NOT "go live with v1" — it's
"prove the moat is operator-visible via a 7-day continuous-paper
operator-success-cycle that includes a risk envelope, the deferred v3-llm
Wave D, and a memory-PnL-correlation report surface." Two new candidates
proposed: risk-envelope-v0.1.0 and reflection-PnL-correlation surface.
Routing recommendations for all 4 open candidates included.
"""

[inputs]
spec_files = [
  "spec/dev-notes/feature-state-table-2026-05-22.md",
  "spec/dev-notes/strategy-reformulation-survey-2026-05-22.md",
  "spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md",
  "spec/dev-notes/v3-vol-retirement-and-c5-promotion-2026-05-22.md",
  "spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md",
  "spec/v3-volatility-forecaster-noop-fix/presentations/v3-volatility-forecaster-noop-fix-2026-05-22.md",
  "spec/product.md",
  "spec/backlog.md",
]

[outputs]
spec_files = [
  "spec/dev-notes/feature-state-analyst-review-2026-05-22.md",
]
trace_rows_opened = []
trace_rows_updated = []
feature_folders_created = []

[open_questions]
items = [
  "Q-MOAT: does the operator agree that the forecaster track was anti-moat in opportunity cost, and that moat-visibility should be the next routing criterion?",
  "Q-RISK: promote a new risk-envelope-v0.1.0 feature before any further strategy or live work?",
  "Q-REFLECTION: promote a new reflection-PnL-correlation surface feature in operator-success-reports to make moat operator-visible?",
  "Q-COMPOSE: compose all open candidates (Wave D + risk envelope + canonical baseline + reflection surface + 7-day continuous paper) into a single `v3-continuous-paper-success-cycle` feature?",
  "Q-V1-DRAWDOWN: is the 73% max-DD on v1 momentum acceptable for continuous paper, or does it trigger a strategy refactor before any live-cadence work?",
]

[assumptions]
items = [
  "product.md § Differentiator (lines 67-83) is the canonical moat statement; (2) persistent reflection memory + (4) auditable double-entry are the long-term moat per the 2026-04-17 confirmed bet.",
  "product.md § Non-goals excludes real-money execution; 'go live' in this project context means continuous paper-trading on real market data with simulated fills.",
  "product.md v3 success metric (90 days continuous paper + weekly operator-success-reports + lesson-card memory demonstrably accumulating + uptime >99% + zero risk-limit breaches + LLM cost inside v2 budget) is the project's terminal state.",
  "The 73% max drawdown on v1 momentum 2023-FY (from the v3-vol-noop-fix mechanism section) is a real number on real Binance hourly data; the v1 baseline anchored across the v25-dl + v3-vol retirements is the same baseline.",
  "Risk engine status: only the .halt-file kill switch is shipped; daily-loss-stop, max-drawdown trigger, per-symbol cap, max-leverage are NOT shipped per my Group C analysis. (Operator to correct if I'm wrong.)",
  "The reflection-memory + audit-ledger pair is operationally complete (reflection-memory v1.8.0 + journal-transactions-metadata v1.6.1 + per-symbol-position-accounts v1.4.0 + real-mtm-unrealized-pnl v1.3.0 + audit-tick-consumer-envelope v0.1.0); the gap is on the strategy-consumes-memory side and on the report-surfaces-memory side."
]
```

HANDOFF → orchestrator
