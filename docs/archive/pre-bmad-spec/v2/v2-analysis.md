---
slug: v2-analysis
status: draft
owner: analyst
updated: 2026-06-28
---

# v2 Product Analysis — research → a frontend-driven, traceable advisor

> **The goal (operator's words, unchanged):** *"a framework for trading with
> traceable and plausible trading."* The differentiator is **MEASURED HONESTY,
> not asserted alpha.** Validated thesis (900 papers, 9 independent reviews,
> deep-read at primary-source depth): **no active strategy robustly beats
> buy-and-hold net of costs** on a single liquid coin.
>
> **What v2 is:** the research-distilled next phase, scored by the **SAME frozen
> robustness gate + same buy-and-hold benchmark** as v1. The #1 convergent output
> of the whole program (surfaced #1 in 6 of 9 topics) is the **P0 selection-bias /
> overfitting scorecard** — the literal "traceable & plausible" credibility layer.
> Most "active edge" features are **expected-null**; the durable value is
> **gate-hardening + honest coverage + risk-shaping**, NOT alpha.

**Reading map for the architect (who runs next):** §1 is the centerpiece (the
end-to-end frontend workflow). §2 is the prioritized feature shortlist (Q1). §3
is the off-track list (Q2). §4 is the complex / needs-more-research set (Q3). §5
tees up the architecture questions. §6 is the honest through-line. Citations point
to `research/<topic>/application-*.md` (the per-topic decision docs) and to current
code paths. Sources read: all 21 application docs + `SYNTHESIS.md` +
`APPLICATIONS.md` + the current cockpit (`crates/ui`) + `spec/product.md`.

---

## §0 What is true RIGHT NOW (verified against code, not spec prose)

Per the standing memory note "verify code before trusting spec status," I checked
the actual tree before scoping. Load-bearing facts:

| Claim | Status in code | Evidence |
|---|---|---|
| The advisor loop is shipped (pick coin+budget → bake-off → rank → forward plan → watch) | **YES** | `crates/ui/src/{leaderboard,forward_plan,live}.rs`; `crates/backtest/src/bakeoff/`; product.md F1–F9 |
| The frozen robustness gate (1000-path moving-block bootstrap, FRAGILE-can't-crown, B&H exempt) | **YES** | `bakeoff/{robustness.rs,bootstrap.rs,rank.rs}`; ADR-0066 |
| Politis–White block length, circular resample | **DONE** (only *logging* missing) | `bootstrap.rs` calls `politis_white_block_length`; wrap-around `% n` |
| Leaderboard columns shown today | **Return · Sharpe · Max DD · Trades** only | `screens/leaderboard.rs` (the ASCII layout doc) |
| **P0 scorecard (DSR / PBO / MinBTL / N_eff) in production** | **ABSENT** | grep: only `crates/backtest/src/bin/param_robustness_sweep.rs` names them — a bin, not the bake-off/leaderboard path |
| **Turnover / CVaR / Sortino / median as ranking outputs** | **ABSENT** from reports/leaderboard | grep: `sortino`/`calmar` are *computed* in `CandidateKpis` but NOT shown; no CVaR anywhere |
| Drawdown-control overlay | **DOES NOT EXIST** | no `drawdown_control_overlay.rs`; only `vol_targeting_overlay.rs` + `vol_killswitch_overlay.rs` |
| A "training" stage in the frontend | **PARTIAL** — the **Tune** screen (`screens/tune.rs`, ADR-0069) is a gate-tied param sweep, but a power-user drill-down off a Leaderboard row, NOT a first-class workflow stage | `screens/tune.rs` |
| Vol/ML "models" | retired TCN/PatchTST/GARCH/markov behind `candle` feature flags; narration-only/opt-in | `crates/forecast/{tcn,patchtst,garch,markov_switching,vol}.rs` |
| `MAX_SWEEP_CONFIGS` | **24** (raw N is tiny → N_eff single-digit → modest haircut, MinBTL bites hardest) | `sweep.rs:62` |
| `ui` purity seam (no dep on strategy/exec/llm/models) | **ENFORCED** via mirror types (`BakeoffReportMirror`, `ForwardPlanView`, `NarrationOutcome`) | `leaderboard/state.rs`, `forward_plan/state.rs` |

**The one-sentence gap:** the engine already *computes* the honest verdict, but the
**frontend workflow stops at Return/Sharpe/Max-DD/Trades + a headline** — it never
shows the operator *the trial budget it paid, the deflated confidence, the turnover
that explains the cost story, or the tail/drawdown risk* — i.e. the "traceable &
plausible" evidence exists in spirit but is not surfaced, and there is no explicit
**DATA → TRAINING → ANALYSIS → SUGGESTION** spine tying the screens together.

---

## §1 The end-to-end workflow — DATA → TRAINING → ANALYSIS → SUGGESTION (THE CENTERPIECE)

The operator's main ask: a **complete end-to-end workflow, frontend-driven**, that
answers *when to BUY, when to SELL, WHICH coin, HOW MUCH money*. Below: the four
stages, what the cockpit does **today**, the **desired** end state, the **gap**, and
**which research features fill it.**

> **Honesty reframe of the four stages (load-bearing).** The operator's verbs map
> onto the engine honestly only if "training" means **vol/risk estimation for
> sizing**, never price/return prediction (the research is decisive: return
> prediction does NOT beat B&H; volatility is the one defensible numeric target —
> `llms/application-llm-timeseries-foundation-models.md`,
> `crypto-market-structure/application-volatility-regimes-and-overlays.md`). And
> "suggestion" = **the rule-based forward plan + the robustness verdict**, NOT a
> price forecast (`backtesting/application-overfitting-and-multiple-testing.md` §3,
> product.md D2). With that reframe, the four-stage spine is fully honest.

### The workflow at a glance

```text
  ┌──────────┐   ┌────────────────┐   ┌────────────────────┐   ┌─────────────────────┐
  │  DATA    │ → │   TRAINING     │ → │     ANALYSIS       │ → │     SUGGESTION      │
  │ pick the │   │ fit RISK for   │   │ bake off + RANK    │   │ when BUY/SELL,      │
  │ coin +   │   │ sizing (vol),  │   │ under the FROZEN    │   │ which coin, how     │
  │ budget + │   │ tune params    │   │ gate + the NEW      │   │ much money — the    │
  │ window   │   │ (gate-tied)    │   │ overfitting        │   │ rule-based forward  │
  │          │   │ NOT price pred.│   │ scorecard          │   │ plan + watch        │
  └──────────┘   └────────────────┘   └────────────────────┘   └─────────────────────┘
   Leaderboard      Tune screen          Leaderboard +            Forward-plan +
   F3 guided input  (ADR-0069)           scorecard (NEW)          Live view
   + data-quality   + vol estimator      + risk metrics (NEW)     + "confidence-not-
   screen (NEW)     (NEW, P1)            + turnover (NEW)         verdict" framing (NEW)
```

---

### Stage 1 — DATA (pick the coin + budget + window; trust the inputs)

**Today (cockpit):** the F3 guided input on the Leaderboard screen (coin + budget +
lookback 2 weeks → ~4 years), feeding Binance hourly + Yahoo corpora. The budget is
treated as quote-units (€200 ≈ 200 USDT; F7 adds a fixed EUR/USD rate). No
data-quality surfacing.

**Desired:** before the operator commits a coin, the workflow should *vouch for the
inputs* — because an honest verdict on dishonest data is phantom alpha. The user
sees: which venue/feed the price came from, a "results are conditional on this coin
surviving the window" note, and a warning if the coin is thin/wash-traded/P&D-prone.

**Gaps:**
- No venue/metric **trust map** surfaced (which feed, why trusted).
- No **universe / coin-quality screen** (thin-coin, wash-trade, pump-and-dump flag).
- No "conditional on survival" honesty note; no PIT/data-revision provenance shown.

**Research features that fill it:**
- **Metric-specific venue trust map** + **universe screen** + **"conditional on
  survival"** note — `crypto-market-structure/application-data-integrity.md` §6
  items B/C/D; `data/application-pit-labeling-stationarity.md` §6 ("coin-selection +
  data-quality guidance" — *display*, not behavior). Cheap, high-credibility, and
  directly "traceable." **Display-only** ⇒ no overlay e2e mandate; it crosses the
  `ui` seam as a plain DTO field (the data-integrity doc names this constraint).
- **PIT confirmation for price/indicator features** — `data/application-splits-leakage-cv.md`
  §6 (confirm price features honor the same as-of discipline `PitSeries` already
  enforces for sidecar features). A standing test, not UI.

### Stage 2 — TRAINING (fit RISK for sizing + tune params, gate-tied — NOT price prediction)

> **This is the stage most at risk of becoming off-mission.** The operator says
> "training"; the research says **training a return predictor is a documented dead
> end** (deep nets, TSFMs, LLMs all fail net of costs on a single coin —
> `deep-learning/application-forecasting-and-significance.md`,
> `llms/application-llm-timeseries-foundation-models.md`). The honest content of a
> "training" stage is therefore **two things the research blesses**: (a) **fitting
> realized volatility** to drive a de-risk-only sizing overlay (vol is forecastable;
> direction is random-walk), and (b) **the parameter sweep that is already
> gate-tied** (the Tune screen).

**Today (cockpit):** the **Tune** screen (`screens/tune.rs`, ADR-0069) is exactly a
gate-tied "training" surface: pick a strategy family + a parameter grid, sweep it,
and every config is scored through the SAME frozen gate (FRAGILE configs are
promotion-blocked). It is honest by construction (overfit configs render FRAGILE).
But it is a **power-user drill-down off a Leaderboard row**, not a visible workflow
stage, and it has **no overfitting-budget readout** (it sweeps configs but doesn't
show the operator the DSR/MinBTL cost of having swept them).

**Desired:** a first-class, legible TRAINING stage that means **"calibrate the risk
inputs and the parameters, honestly"** — and that *visibly charges the search
budget against significance* so the user understands that more tuning = a higher bar
to clear, not more alpha.

**Gaps:**
- **No realized-volatility estimator** feeding a de-risk-only sizing overlay (the
  one defensible numeric "training" target). The shipped `vol_targeting_overlay.rs`
  exists but is framed as a Sharpe tool and uses a single lookback.
- **No overfitting-budget readout on Tune** — the sweep doesn't show DSR/MinBTL.
- The Tune stage is **not part of the named workflow** (hidden drill-down).

**Research features that fill it:**
- **σ̂ upgrade: multi-horizon realized-vol (HAR / two-half-life EWMA)** feeding both
  overlays — `risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md`
  §6 P1-C; `crypto-market-structure/application-volatility-regimes-and-overlays.md`
  §6 item F. Cheap, in `Decimal`, no deep net. This is the honest "training" the
  operator can watch.
- **(Optional, gated) realized-vol forecast** via a *small* model (TTM/PatchTST/
  GARCH/HAR baseline) feeding the **de-risk-only sizing overlay, never the ranking**
  — `llms/application-llm-timeseries-foundation-models.md` §6 P1. **Must clear the
  GARCH/HAR baseline + a calibration check + the cost-aware gate + a day-1
  divergence e2e.** Honest expectation: marginal, drawdown-only.
- **Wire the P0 scorecard into the Tune readout** so the sweep shows "you tried N
  configs → MinBTL needs X years → DSR Y" (the search-budget-charged-against-
  significance principle) — `evolution/application-anti-overfitting-and-search-discipline.md`
  §6; `backtesting/application-overfitting-and-multiple-testing.md` §6.

> **Naming guidance for the architect/ui-designer:** call this stage **"Calibrate"**
> or **"Risk & tuning,"** NOT "Training a model" — the latter invites the
> return-prediction misread the research forbids. The stage *trains/fits vol for
> sizing and tunes rule params*, both gate-tied.

### Stage 3 — ANALYSIS (bake off + rank under the frozen gate + the NEW scorecard)

**This is where the P0 work lands and where "traceable & plausible" becomes visible.**

**Today (cockpit):** the Leaderboard bakes off the field (4 base signals + 8
ensembles + shorts + benchmark) and ranks under the frozen gate, showing **Return ·
Sharpe · Max DD · Trades** + a recommendation headline (`ActiveWins` /
`BenchmarkWins` / `AllFragile`) + reason codes + the F9 LLM narration. It is honest
about *which verdict fired* but does **not** show *why the verdict is statistically
credible*.

**Desired:** every crown ships an **auditable overfitting scorecard** next to the
verdict — "we tried N_eff effective strategies; here's the deflated confidence
(DSR); here's the MinBTL pre-flight; here's PBO; here's the turnover that explains
the cost story; here's the tail (CVaR/drawdown) you'd carry." This is the literal
embodiment of the operator's goal.

**Gaps (the highest-leverage gaps in the whole product):**
- **No overfitting scorecard** — N_eff, DSR, MinBTL, PBO are not computed or shown.
- **No turnover column** — the single most intuitive way to show *why* costs favour
  holding; cheap (no equity change ⇒ no anchor break).
- **No coherent tail metrics** — CVaR/ES, Sortino, median terminal wealth, skew are
  not surfaced (Sortino/Calmar are computed but hidden).
- **No "risk-adjusted win ≠ more terminal wealth"** distinction (a Sharpe win that a
  long-term holder shouldn't prefer is not flagged).

**Research features that fill it (the P0 core + cheap P1 reporting):**
- **P0 overfitting scorecard: N_eff → DSR → MinBTL → PBO** (additive, report-first,
  FROZEN bands untouched) — the convergent #1 of the program:
  `backtesting/application-overfitting-and-multiple-testing.md` §6 (items A/B/C/D/E),
  `evolution/application-anti-overfitting-and-search-discipline.md` §6,
  `data/application-splits-leakage-cv.md` §6, `ml-trading/application-ldp-pipeline-and-meta-labeling.md`
  §6, `deep-learning/application-forecasting-and-significance.md` §6 (F-1). All
  inputs already stored (N + the per-candidate Sharpes + the crown's return series).
- **Turnover as a first-class KPI** — `backtesting/application-cost-and-impact-modeling.md`
  §6 item A (highest credibility-per-effort; pure reporting);
  `strategies/application-execution-and-sizing-rules.md` §6 P0.
- **Coherent tail + median reporting (CVaR/ES, Sortino, median, skew)** —
  `risk-and-sizing/application-position-sizing-and-bet-sizing.md` §6 P1;
  `risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md` §6 P2-D
  ("do it once" — shared item). Near-free from the bootstrap loss distribution.
- **"Risk-adjusted vs terminal-wealth" surface** + **honesty-claim calibration**
  ("we TEST whether a rule beats hold; usually it doesn't" — never "TA never
  works") — `strategies/application-factor-replication-and-the-counter-thesis.md` §6.

### Stage 4 — SUGGESTION (when BUY/SELL, which coin, how much — the rule plan + the watch)

**Today (cockpit):** the Forward-plan screen (F6) renders the crowned strategy's
**current stance + standing IF/THEN entry-exit rules + budget-aware €200 next-BUY
sizing + horizon + not-advice disclaimers** (a conditional, reactive plan — NOT a
forecast, product D2). The Live view paper-trades it forward on real bars and shows
running P/L. This is already the honest shape of "suggestion."

**Desired:** the suggestion stage is *almost there* — it needs (a) the forward
number to measure the *actual* crowned strategy (the F5 skew), (b) the scorecard's
trial-aware numbers shown *alongside* the forward plan (so the forward run reads as a
"confidence check," not a verdict), and (c) the de-risk overlays (drawdown/vol)
offered as an explicit "how much risk do you want to carry" choice on the sizing.

**Gaps:**
- **F5 forward-fidelity skew** — the forward paper-trade runs an SMA proxy for
  non-SMA crowned picks ⇒ the forward number measures a *different* strategy than
  the one crowned. (`build_registry_for` exists at `crates/agent/src/runtime.rs:335`;
  the fix reuses the bake-off's ComposedStrategy-from-TOML.)
- **The forward run is implicitly framed as the OOS verdict** — the research amends
  this: a single hold-out is insufficient (high variance, trial-blind); pair it with
  the scorecard.
- **No drawdown / vol de-risk choice** on the sizing ("never lose more than X% of
  peak" is the one hard, interpretable promise a retail user wants).

**Research features that fill it:**
- **F5b forward-fidelity fix** — `data/application-pit-labeling-stationarity.md` §6
  (P0/P1, "one strategy definition everywhere"); a **correctness fix**, not a
  feature, but it gates the honesty of the entire SUGGESTION stage.
- **Pair the forward run with the scorecard + relabel "confidence check, not
  verdict"** — `data/application-splits-leakage-cv.md` §6 (P0 "pair forward-trade
  with deflation"); `backtesting/application-overfitting-and-multiple-testing.md`
  §7 Q7.
- **Drawdown-control overlay (HWM restart) + repositioned vol overlay** as an
  explicit de-risk choice on sizing — `risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md`
  §6 P1-A/P1-B. "Never lose more than 20% of peak (probabilistic; costs ~40% of
  upside)" is the honest, operator-facing promise.

### Workflow synthesis — what to build to make the spine real

The spine becomes a real, frontend-driven workflow with **four moves**, in order:
1. **ANALYSIS scorecard (P0)** — the credibility layer; everything else hangs off it.
2. **Turnover + tail/median reporting (P1)** — makes the cost & risk story visible.
3. **F5b forward-fidelity + "confidence-not-verdict" framing (P0/P1 correctness)** —
   makes the SUGGESTION honest.
4. **Drawdown/vol de-risk overlays (P1) + σ̂ estimator (P1)** — the honest "training"
   + the one interpretable risk promise.
The DATA-stage trust/universe surface (P1) and the Tune-stage scorecard readout
(P1) round out the spine.

---

## §2 Recommended v2 features (Q1) — prioritized P0/P1/P2

> Every item below is **additive** to the FROZEN gate + bands + B&H benchmark, in
> `Decimal` where money, behind the `ui` mirror seam where surfaced, and (for any
> overlay/sizing-modifier) ships a **day-1 baseline-equity-divergence e2e** (the
> v3-vol-overlay-noop precedent; pattern set already exists —
> `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` et al.). Each carries
> an **honest expected-null-or-real** tag.

### P0 — the credibility core (do these first; they ARE the product goal)

**P0-1 — Overfitting scorecard: N_eff → DSR → MinBTL → PBO (report-first).**
- *What:* compute and surface, per bake-off run, an overfitting scorecard — effective
  trial count `N_eff` (closed-form `ρ̄+(1−ρ̄)·M`, with cluster-first only if it ever
  grows past T; at N=24 ill-conditioning is moot), the **Deflated Sharpe Ratio**
  (exact formula, crown only if DSR ≥ 0.95 AND beats B&H), a **MinBTL pre-flight
  veto** (`2·ln(N)/SR²` — bites hardest at our short-window/small-N reality), and
  **PBO via CSCV** (the enabler: capture the per-config bar-return matrix).
- *Research:* `backtesting/application-overfitting-and-multiple-testing.md` (§6 the
  canonical work-item table A–L; §3 the **code-grounded correction**: at
  `MAX_SWEEP_CONFIGS=24` the haircut is *modest, not "gut everything,"* and MinBTL
  bites hardest); `evolution/application-anti-overfitting-and-search-discipline.md`
  (the exact DSR/MinBTL closed forms, captured first-hand); `data/application-splits-leakage-cv.md`;
  `ml-trading/application-ldp-pipeline-and-meta-labeling.md`; `deep-learning/application-forecasting-and-significance.md` (F-1).
- *Codebase:* `crates/backtest/src/bakeoff/{robustness.rs,rank.rs}` + the ranking
  report + a passive `ui` mirror struct on `RecommendationMirror`/`BakeoffReportMirror`
  (`leaderboard/state.rs`). New report fields/file (anchor-safe; run
  `verify_anchors.sh` before+after). Ship order: **A (MinBTL) + B (DSR) + C (N_eff)**
  closed-form report-only first (no plumbing); **E (return-matrix capture) → F (PBO)**
  as the second increment.
- *Fit:* this is the *literal* "traceable & plausible" layer. Every crown becomes
  auditable. **Expected-null framing:** for the sub-0.4 net Sharpes a single coin
  produces, the gate should crown almost nothing — "REFUSED to crown over B&H" is
  *the product working*, not a failure (this needs a UX framing — see §5).

**P0-2 — F5b forward-fidelity fix (one strategy definition everywhere).**
- *What:* the forward paper-trade must run the *exact* crowned strategy (reuse the
  bake-off's ComposedStrategy-from-TOML in `build_registry_for`), not an SMA proxy.
- *Research:* `data/application-pit-labeling-stationarity.md` §6 (training–serving
  skew; the highest-value item in that strand). Re-flagged by APPLICATIONS.md
  "Correctness fixes."
- *Codebase:* `crates/agent/src/runtime.rs:335` (`build_registry_for`) — the seam
  exists. A **correctness fix**, no FROZEN-constraint impact.
- *Fit:* without it, the SUGGESTION stage's forward number measures a different
  strategy than the one crowned — a silent honesty hole. **Real, not null** (it fixes
  a measurement bug). Project memory already names this as F5b.

**P0-3 — "Confidence check, not verdict" framing + pair forward-trade with the scorecard.**
- *What:* relabel the forward paper-trade as a confidence check (genuine unseen data,
  but high-variance + trial-blind), and show the P0 scorecard numbers alongside the
  forward plan. The one place the research *amends* our design.
- *Research:* `data/application-splits-leakage-cv.md` §6 (P0 pair forward-trade with
  deflation); `backtesting/application-overfitting-and-multiple-testing.md` §7 Q7.
- *Codebase:* the forward-plan output + ranking report (downstream of `rank.rs`);
  copy in `crate::strings`. Mostly framing + wiring the P0-1 output through.
- *Fit:* converts a subtly over-claimed surface into an honest one. **Honesty fix.**

### P1 — concrete, mostly risk-shaping + cost-realism (the visible honesty)

**P1-1 — Turnover as a first-class KPI/column.**
- *What:* surface each candidate's turnover next to its net edge; flag high-turnover
  crowns. Pure reporting (no equity change ⇒ no anchor break).
- *Research:* `backtesting/application-cost-and-impact-modeling.md` §6 A (highest
  credibility-per-effort in that doc); `strategies/application-execution-and-sizing-rules.md` §6 P0.
- *Codebase:* `bakeoff/` KPIs + leaderboard column + mirror. *Fit:* the single most
  intuitive way to show *why* the advisor keeps recommending hold (costs favour the
  lowest-turnover strategy). **Real** (it's a true cost driver), **explains the null.**

**P1-2 — Coherent tail + median reporting (CVaR/ES, Sortino, median terminal wealth, skew).**
- *What:* add CVaR/ES at 90/95/99%, Sortino/Calmar (already computed!), median
  terminal wealth, and skew to the bake-off + forward report — read from the existing
  bootstrap loss distribution. Report CVaR, not VaR (VaR is non-coherent).
- *Research:* `risk-and-sizing/application-position-sizing-and-bet-sizing.md` §6 P1
  (highest-value item there); `risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md`
  §6 P2-D (shared — "do it once").
- *Codebase:* the ranking + forward report (the bootstrap distribution is already in
  `bakeoff/`). Additive; FROZEN bands untouched. *Fit:* surfaces the risk-shaping
  benefit Sharpe hides, and crypto's asymmetry. **Real** (honest measurement).

**P1-3 — Drawdown-control overlay with high-water-mark restart.**
- *What:* a new de-risk-only overlay — cushion multiplier `M(k)=(d_max−d(k))/(1−d(k))`
  + the **HWM restart** (load-bearing: without it BTC Sharpe collapsed −0.04; with it
  held 1.52 and cut max-DD 72%→20%, giving back ~40% of upside). Static (CPPI-like)
  vs ratcheting (TIPP-like) floor as an operator choice; floor disclosed as
  *probabilistic* (gap risk).
- *Research:* `risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md`
  §6 P1-B (the single most actionable new overlay in the program).
- *Codebase:* new `crates/strategy/src/drawdown_control_overlay.rs` (mirror the
  `VolTargetingOverlay`/`Strategy` shape; compose with `FixedFractionSizer`, never
  bypass the budget cap). **Ships a day-1 baseline-equity-divergence e2e** (mandatory).
- *Fit:* the one hard, interpretable promise a retail user wants ("never lose more
  than X% of peak"). **Risk-shaping, NOT Sharpe** — sell drawdown/tail reduction; the
  return cost is large and must be shown (mutual non-dominance framing).

**P1-4 — Reposition the shipped vol-targeting overlay as a risk tool (loose & slow).**
- *What:* keep the mechanics, change the framing/defaults — slow EWMA σ̂ (~126-day
  half-life), a cost-and-vol-scaled no-trade band, **de-risk-only**, optional
  downside-deviation trigger; compute and report each coin's per-window return-vol
  correlation so the operator sees whether a Sharpe gain is even mechanistically
  possible (crypto's leverage effect is reversed, γ=−0.261).
- *Research:* `risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md`
  §6 P1-A; `crypto-market-structure/application-volatility-regimes-and-overlays.md`
  §6 B.
- *Codebase:* `crates/strategy/src/vol_targeting_overlay.rs` (reparameterise) + report.
  Existing e2e stays green. *Fit:* reframe + cost-hardening of a shipped surface — low
  blast radius. **Risk-shaping, NOT Sharpe** (do NOT chase the target — closed-loop
  blew turnover to 1105%/yr).

**P1-5 — σ̂ upgrade: multi-horizon realized-vol (HAR / two-half-life EWMA).**
- *What:* replace/augment the single-lookback vol estimate feeding both overlays with
  a HAR-RV blend or two-half-life EWMA; on daily bars approximate RV with squared
  returns / Garman–Klass. Do NOT over-engineer (DL/SV don't improve the tail).
- *Research:* `risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md`
  §6 P1-C; `crypto-market-structure/application-volatility-regimes-and-overlays.md` §6 F.
- *Codebase:* a shared vol-estimator module used by both overlays (align with the
  existing `crates/backtest/src/scenarios/garch_vol_target_overlay.rs`). *Fit:* the
  honest content of the "training" stage; cheap, `Decimal`. **Real** (vol is
  forecastable), but feeds risk, never the ranking.

**P1-6 — Cost-model hardening + metric-specific venue-trust map.**
- *What:* state-aware (vol-scaled) spread (no mid-price fills on spoofable depth,
  widen 2–3× in high-vol, 5–10× in cascades); a metric-specific venue trust map (OI
  strictly Kraken/HTX; distrust unregulated volume); a fee-sensitivity sweep.
- *Research:* `crypto-market-structure/application-data-integrity.md` §6 A/B;
  `backtesting/application-cost-and-impact-modeling.md` §6 B/E;
  `crypto-market-structure/application-exogenous-signal-arms.md` §6 G;
  `strategies/application-execution-and-sizing-rules.md` §6 P1.
- *Codebase:* `crates/cost/` + `bakeoff/` cost path; venue map a dev-note + data-source
  config. **Anchor risk:** changing the *default* cost path breaks 119 anchors — keep
  new cost realism **opt-in / new-anchored**, or take the deliberate ADR-0038 re-emission
  route (architect call). **Day-1 divergence e2e** for any vol-scaled-spread change.
- *Fit:* under-costing on fake depth is silent dishonesty; this is the cheapest gate
  hardening that may flip marginal picks to FRAGILE. **Real** (it's a true cost),
  **hardens the null.** *Note: calibration tightrope — punitive costs that manufacture
  a too-easy "hold wins" are ALSO dishonest; calibrate, document sources.*

**P1-7 — DATA-stage trust/universe/quality surface (display-only).**
- *What:* a coin-quality screen + "conditional on survival" note + venue/provenance
  readout in the DATA stage (warn on thin/wash-traded/P&D coins).
- *Research:* `crypto-market-structure/application-data-integrity.md` §6 C;
  `data/application-pit-labeling-stationarity.md` §6 ("display, not behavior").
- *Codebase:* a plain DTO field the `ui` already consumes (respect the layering;
  display-only ⇒ no overlay e2e). *Fit:* vouches for the inputs — "traceable" at the
  DATA stage. **Honesty surface.**

### P2 — coverage, narration hardening, gated experiments (expected-null)

**P2-1 — Harden the F9 narration faithfulness check** (verbatim-number match; extend
the banned-phrase list to prediction/causation verbs) — `llms/application-llm-narration-and-agents.md`
§6 P0 (ship as an ADR-0064 amendment). **Honesty hardening.**

**P2-2 — Standing no-alpha-gate + null-data falsification CI test** (run the full
bake-off+rank on GBM/GARCH/OU/regime-switch/bid-ask-bounce nulls; assert it crowns
nothing over B&H, and DSR/PBO flag overfit picks) — `deep-learning/application-forecasting-and-significance.md`
§6 F-2; `data/application-synthetic-and-monte-carlo.md` §6;
`evolution/application-anti-overfitting-and-search-discipline.md` §6. **A permanent
leak/overfit tripwire** — the pipeline-level analogue of the day-1 e2e discipline.

**P2-3 — Matched-activity random-null sub-test** (require the crown to beat random
trading matched on trade frequency AND time-in-market) — `evolution/application-anti-overfitting-and-search-discipline.md`
§6 P1. Cheap; catches lucky-timing edges. **Honesty sub-test.**

**P2-4 — Cost-aware "trade-less" execution filter** (act only when
`|expected_move| > λ·c·|Δpos|`) — `ml-trading/application-ldp-pipeline-and-meta-labeling.md`
§6 P1; `strategies/application-execution-and-sizing-rules.md` §6 P1. Provably can't
underperform "always act" net of costs; **bundle with F5b.** **Expected-null on
return** (restores viability, did NOT beat B&H after Holm) — plausible win is
cost-drag reduction. Needs an `expected_move` definition per rule-based strategy
(analyst sign-off — see §5).

**P2-5 — Funding-sign froth arm** (`v0.funding_froth`) — the one genuinely
probe-worthy exogenous arm; reuses the existing `basis_data`/`funding_data` seam —
`crypto-market-structure/application-exogenous-signal-arms.md` §6 A. **Expect
FRAGILE** → honest coverage, not a win. (Funding↔price is bidirectionally Granger,
an endogeneity trap.)

**P2-6 — Active-plus-hold blend arm** (blend the active sleeve with the B&H core) —
`strategies/application-execution-and-sizing-rules.md` §6 P1. The one robust win in
the honest studies was a ~50% drawdown cut from blending; new bake-off arm + day-1
e2e. **Risk-shaping** (cuts drawdown, expected ≈ B&H on terminal wealth).

**P2-7 — Document the dead-ends + the sizing posture** (an authoritative
"do-not-build" list + a sizing-posture ADR) — `crypto-market-structure/application-exogenous-signal-arms.md`
§6 B; `risk-and-sizing/application-position-sizing-and-bet-sizing.md` §6 P1;
`llms/application-llm-timeseries-foundation-models.md` §6 P2. Prevents wasted feed
budget + re-litigation. **Pure documentation, high value.**

---

## §3 Off-track features (Q2) — what pulls into a COMPLETELY DIFFERENT TRACK

> These break paper-only / single-coin / measured-honesty. Flag loudly; gate hard;
> mostly **do-not-build.**

**OT-1 — Multi-asset / "which coin among MANY" portfolio (THE operator-named risk).**
The operator literally said *"which stock."* This needs an explicit split:
- **Single-coin SELECTION the user already makes** (pick XRP vs BTC at the DATA
  stage, one at a time) is **IN scope** — that is the existing journey, unchanged.
- **Multi-coin SELECTION / a cross-sectional "rank coins and pick the best" or a
  basket portfolio** is a **TRACK CHANGE.** It breaks the single-coin contract
  (product.md "Not a multi-asset portfolio manager"), and the research is decisive
  that the surviving factor edges (value/momentum/quality/low-risk) are
  **cross-sectional — they need a universe to rank and are NOT harvestable on one
  coin** (`strategies/application-factor-replication-and-the-counter-thesis.md` §1).
  Worse, the diversification that would justify a basket **fails in crypto** —
  BTC–ETH ρ>0.85 in stress; "cash is the only real diversifier"
  (`risk-and-sizing/application-position-sizing-and-bet-sizing.md` §1). **Verdict:**
  do NOT scope multi-coin in v2. *If the operator wants it,* it is a separate
  product track with its own gate calibration (cross-sectional N_eff, contagion-aware
  risk), explicitly named, not an "additive arm."

**OT-2 — Live trading / real orders / KYC.** Out of scope by standing operator
constraint (removed 2026-06-12; project memory "no live trading"). Do NOT
re-propose. Paper/sim only; `Decimal`, USDT. (Any maker/limit cost mode stays a
*simulation assumption*, clearly labelled, never a step toward execution —
`strategies/application-execution-and-sizing-rules.md` §7.)

**OT-3 — Automated alpha search (GA / GP / symbolic regression / LLM-code-evolution).**
The single **highest-overfitting-risk idea in the 900-paper program** —
industrialized data-snooping by construction; in-sample winner is *negatively*
correlated with OOS return; expected null on a single coin
(`evolution/application-automated-strategy-search.md` §1). Our FIXED pre-registered
slates are the standing defense. **Do-not-build as a default;** only ever behind the
full guard stack (walk-forward + once-only OOS + DSR/MinBTL budget-charging +
pre-registration) and *expecting* a null — and even then the deliverable is the
*protocol + null verdict*, not a winner. The honest export from this literature is
the discipline (P0), NOT a search engine.

**OT-4 — Return/direction-prediction models in the ranking (TSFM / deep net / LLM
forecaster).** No gate-credible crypto return-alpha; the "language" ablates out;
lower forecast MSE does NOT mean more profit; the only peer-reviewed crypto economic
test gives BTC ~1.0 Sharpe ≈ B&H (`llms/application-llm-timeseries-foundation-models.md`
§8; `deep-learning/application-forecasting-and-significance.md` §8). **A forecaster
may ONLY feed a downstream de-risk-only SIZING overlay (vol), never the crown.** The
retired TCN/PatchTST/GARCH/LLM-forecaster chains in `crates/forecast/` stay opt-in /
narration-only. Putting any of them in the ranking is the one bright line.

**OT-5 — LLM/agent as a trading decision-maker (single or multi-agent "debate").**
Every "LLM beats B&H" result is leakage / no-cost / single-window / factor-harvesting
(`llms/application-llm-narration-and-agents.md` §8). LLMs are **narration-only**
(F9/ADR-0064) + read-only reflection (ADR-0074). The multi-agent "debate" pattern is
seductive and is exactly the configuration the refutations target — at most a
*narration-structuring* device (bull/bear prose), never a decision mechanism.

**OT-6 — On-chain (MVRV/SOPR/netflows) + sentiment (Fear & Greed / social) exogenous
arms.** PIT-infeasible / endogenous / fail Granger; documented dead ends
(`crypto-market-structure/application-exogenous-signal-arms.md` §8;
`crypto-market-structure/application-data-integrity.md` §8). The on-chain hard-stop
already fired 2026-06-08. Do NOT spend paid-feed budget. (On-chain *valuation* MVRV
is a *future, PIT-gated* spike at best — §4 — not a v2 arm.)

**OT-7 — Generative synthetic test data (GAN / diffusion / VAE) for the gate.**
Generators **structurally smooth tails** (Gaussian latent prior can't produce heavy
tails) and overfit a single short path; Historical Simulation / GARCH tie-or-beat
them on the VaR task we care about (`data/application-synthetic-and-monte-carlo.md`
§8). **Keep the model-free moving-block bootstrap as the default.** Research-only;
the one honest gap (can't invent a worse-than-seen crash) is best filled by a
tail-stressed/EVT slice, not a generic generator.

**OT-8 — Kelly / μ-driven "smart sizer" as a return tool.** Quantitatively hopeless
on a no-edge coin — Kelly on a noisy μ̂ loses 27–48% of oracle return, recoverable by
~1–3% at best; the skew hurdle is essentially never met on crypto; we are never in
Kelly's asymptotic regime (`risk-and-sizing/application-position-sizing-and-bet-sizing.md`
§8). **Keep fixed-fraction + vol-only sizing.** A one-knob fractional-Kelly *shrink*
dial is at most a gated P2 experiment, expected ≈ null; "size down, control risk,"
never "size up for alpha."

**OT-9 — Market-impact / VWAP-TWAP execution scheduling.** Impact ≈ 0 at €200 retail
scale (confirmed on BTC); the citations exist to *justify the simple fee+spread
model*, not to build a heavy execution simulator (`backtesting/application-cost-and-impact-modeling.md`
§8; `strategies/application-execution-and-sizing-rules.md` §8). Do-not-build at our scale.

**OT-10 — Order-book imbalance / depth / HFT microstructure overlays.** Depth is
~31% spoofable even on Coinbase; the edge dies on costs and is out of our daily
horizon (`crypto-market-structure/application-data-integrity.md` §6 G). Do-not-build.

---

## §4 Complex / needs more research before building (Q3)

**CX-1 — PBO via CSCV + the per-config return-matrix capture (the P0 enabler).**
PBO/N_eff-clustering/BBC-CV all need the **full T×N matrix of per-bar returns across
ALL swept configs.** Today `CandidateResult` stores only per-candidate equity, and
the sweep likely retains only survivors. This is the **one non-trivial plumbing
change** in the P0 stack (DSR/MinBTL/N_eff-closed-form do NOT need it — ship those
first). The architect must size the capture (`sweep.rs` + `mod.rs`) and rule on the
operating-point calibration (PBO is report-don't-binary). Refs:
`backtesting/application-overfitting-and-multiple-testing.md` §5/§6 E/F;
`data/application-splits-leakage-cv.md` §6.

**CX-2 — N_eff estimator + the M>T question.** The literature mandates cluster-first
when configs M > window bars T (ill-conditioned correlation matrix). **Code-grounded
correction (read this):** at `MAX_SWEEP_CONFIGS=24`, T ≫ 24 on any bootstrappable
window, so the scary "must cluster first" mandate **likely does NOT apply to us** —
the closed-form `ρ̄+(1−ρ̄)·M` is plausibly sufficient *forever* at this scale
(`backtesting/application-overfitting-and-multiple-testing.md` §3). The research need
is: **confirm this on our data**, and decide whether to ship the closed form (cheap,
sufficient now) or carry clustering headroom for a future larger sweep. Pre-commit
the choice (second-order snooping risk). Architect call.

**CX-3 — DSR/PBO crown-eligibility predicate vs report-only (the recurring architect
question).** Does a new DSR/PBO crown-eligibility disqualifier count as "additive" to
the FROZEN gate, or is it itself a frozen-rule change? Report-only is the safest
first ship (and may be the more honest, less-magic design — the operator reads the
haircut without auto-veto). A PBO/DSR *disqualifier* (additive to the existing
Fragile rule) is the stronger product but needs an ADR. Raised in nearly every
backtesting/data/evolution doc. **Architect M-T1 lock.**

**CX-4 — Threshold derivation: hard-code DSR ≥ 0.95 vs ORATIO-derived.** The famous
t=3.0 was "never intended" as a universal cutoff. The ORATIO odds-ratio derives the
bar from an explicit "a false 'beats-hold' is N× costlier than a miss" statement —
more honest, but needs an **operator product/values input** (it's not just stats).
Refs: `backtesting/application-overfitting-and-multiple-testing.md` §7 Q3;
`evolution/application-anti-overfitting-and-search-discipline.md` §7. **Analyst +
operator decision.**

**CX-5 — Gated realized-volatility forecast for the sizing overlay.** The one open
experiment with positive prior evidence (vol is forecastable). Needs: a small model
(TTM/PatchTST/GARCH/HAR), a **calibration check** (TSFMs are confidently wrong on the
risk rail — a miscalibrated vol forecast can *increase* drawdown), clearing the
GARCH/HAR baseline, the cost-aware gate, and a day-1 e2e. Honest expectation:
marginal, drawdown-only. Open: is it worth the engineering given a simple GARCH/HAR
may already capture most of it? Refs: `llms/application-llm-timeseries-foundation-models.md`
§7; `risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md` §7.

**CX-6 — `expected_move` definition for the cost-aware filter.** The filter
(`|expected_move| > λ·c·|Δpos|`) uses an ML forecast magnitude in the source paper;
our crowned picks are rule-based, so `expected_move` must be defined per strategy
(SMA-gap / MACD histogram / distance-from-band). A modeling choice needing **analyst
sign-off** before the filter is honest. Ref:
`ml-trading/application-ldp-pipeline-and-meta-labeling.md` §7.

**CX-7 — Cost-model default change vs anchor stability (the largest blast radius).**
A vol-scaled spread or delay term changes net returns ⇒ every anchored backtest
report's body-SHA breaks. Decision: keep new cost realism **opt-in/new-anchored
forever**, cut a **versioned default bump** with deliberate ADR-0038 re-emission, or
accept the current flat-bps default as "conservative enough." A **process decision
with real blast radius** — architect + `verify_anchors.sh`. Refs:
`backtesting/application-cost-and-impact-modeling.md` §5/§7;
`strategies/application-execution-and-sizing-rules.md` §7.

**CX-8 — Drawdown overlay: static (CPPI) vs ratcheting (TIPP) floor as the default
operator choice; gap-frequency on real windows.** Which framing a retail user
understands/prefers (TIPP protects profits but caps upside) is a **product/UX
decision**; and the floor's breach-frequency under crypto gaps needs measuring on
real coin windows before the "never lose more than X%" promise is calibrated. Ref:
`risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md` §7.

**CX-9 — Tail-stressed / EVT "worse-than-seen-crash" slice (research spike).** The
one honest limit of our bootstrap (can't invent a worse crash). An EVT-grounded
tail-stress lens is more defensible than an ad-hoc multiplier, but crypto-crash
synthesis is an open problem and a hand-tuned slice can fabricate an unrealistic
crash. Decide: is "we state the limit honestly and don't pretend to cover it" the
better posture? Research-only, never wired into the FROZEN gate. Ref:
`data/application-synthetic-and-monte-carlo.md` §6/§7.

---

## §5 Open questions for the ARCHITECT (consumed next)

**Framework evolution / where to extend vs refactor:**
1. **Is a plugin architecture needed to slot these features, or is the existing
   "additive arm + report annex + overlay" pattern sufficient?** The current seams
   (`bakeoff` candidate field, the `RobustnessFlag`/`Recommendation`/`BakeoffReport`
   types, the `ui` mirror discipline, the overlay+day-1-e2e pattern, the exogenous-arm
   `PitSeries` seam) have absorbed every v1 feature additively. My read: **no plugin
   architecture needed** — the P0 scorecard is a report annex + a passive mirror
   struct; overlays are new files composing with `FixedFractionSizer`; the workflow
   spine is a *naming/IA* change over existing screens. But the architect should rule
   on whether the **DATA→TRAINING→ANALYSIS→SUGGESTION spine** wants a first-class
   state machine in `agent` (the bootstrap layer) vs staying screen-routed.
2. **Where does the overfitting scorecard live so it does NOT re-emit any of the 119
   anchored report SHAs?** New report section vs new file vs ADR-0038 §D6 re-emission.
   Run `verify_anchors.sh` before+after. (CX-1/CX-3/CX-7 are the anchor-risk cluster.)
3. **Does a DSR/PBO crown-eligibility predicate count as "additive" to the FROZEN
   gate, or is it a frozen-rule change?** (CX-3 — the recurring M-T1 lock.) My
   recommendation, durable: **ship the scorecard report-only first** (auditable,
   no-magic, zero frozen-gate risk), and make a *later* DSR/PBO veto a one-line switch
   the design anticipates — so the cheap honest ship doesn't foreclose the stronger
   product.
4. **Return-matrix capture (CX-1):** worth the `sweep.rs`/`mod.rs` plumbing now (to
   unblock PBO/BBC-CV), or ship the closed-form-only scorecard (MinBTL+DSR+N_eff)
   first and defer PBO? My recommendation: **defer the plumbing**; the closed forms
   deliver most of the credibility at a fraction of the blast radius.
5. **The "training" stage's home + name.** The Tune screen (ADR-0069) is the honest
   gate-tied "training" surface; the σ̂ estimator + vol overlay are the other half.
   Should v2 *promote Tune into a named workflow stage* (with the P0 scorecard
   readout), and where does the shared vol-estimator module live (`forecast` vs
   `strategy`) without pulling `ui` into a `strategy` dependency? (CX-5.)
6. **Cost-model default vs anchors (CX-7):** opt-in-forever vs versioned re-anchor vs
   "good enough." The largest blast-radius process decision in v2.

**Sequencing / cross-cutting:**
7. **P0 ship order.** I recommend: P0-1 (MinBTL+DSR+N_eff closed-form, report-only)
   → P1-1 (turnover) + P1-2 (tail/median reporting) → P0-2 (F5b) + P0-3
   (confidence-not-verdict) → P1-3/P1-4/P1-5 (drawdown+vol overlays + σ̂) → P2 items.
   The P0 scorecard is the program's single highest-leverage action and gates the
   credibility story; turnover+tail are near-free and make the honesty *visible*; F5b
   is a correctness prerequisite for the SUGGESTION stage.
8. **N_eff method + freeze (CX-2):** closed-form sufficient at N=24, or carry
   clustering headroom? Pre-commit to avoid second-order snooping.
9. **`[[req]]` rows:** each v2 feature gets its own REQ row + `spec/v2/<slug>/`
   folder. This analysis creates `REQ-V2-ANALYSIS-001` (the analysis itself, state
   `proposed`); the architect/first features create the per-feature rows. (See the
   handoff envelope's `[outputs]`.)

**Product/operator decisions teed up (analyst will carry to operator):**
10. **CX-4 (DSR threshold derivation)** and **CX-8 (static vs ratcheting floor)** are
    operator product/values calls, not pure engineering — frame as durable-over-quick
    operator-decide questions when those features are scoped.

---

## §6 The honest through-line — measured, not asserted alpha

Every recommendation above preserves the operator's core: **measured honesty, not
asserted alpha.** Concretely, the workflow + features keep the through-line because:

- **The credibility layer is the product, not decoration.** The P0 scorecard (§2
  P0-1) turns "this strategy won" into "this strategy won; here is the trial budget
  it was penalized for; here is the deflated confidence; here is why it was/wasn't
  crowned." Six of nine research topics independently flagged this as #1. It is the
  literal embodiment of "traceable & plausible."

- **Expected-null is the honest baseline, surfaced as a feature.** For the sub-0.4
  net Sharpes a single coin realistically produces, the correctly-deflated gate
  **crowns almost nothing** — and "REFUSED to crown over B&H" must read as *the
  product working*, not a failure (the UX framing is an explicit open question, §5).
  The nine reviews converge: no active strategy robustly beats holding net of costs.

- **"Suggestion" stays rule-based + verdict-grounded.** The forward plan is a
  conditional, reactive rule plan (current stance + IF/THEN rules + projected sizing),
  NOT a price forecast (product D2); the verdict is the bootstrap-vs-B&H gate, NOT an
  LLM/ML prediction. F5b (§2 P0-2) makes the forward number measure the *actual*
  crowned strategy; the confidence-not-verdict framing (§2 P0-3) stops over-claiming
  the hold-out.

- **"Training" stays honest.** It means fitting **vol/risk for sizing** + gate-tied
  param tuning — NEVER price/return prediction. The research is decisive that return
  prediction (deep nets, TSFMs, LLMs) does not beat B&H; volatility is the one
  defensible numeric target, and only for the de-risk-only sizing overlay.

- **Risk-shaping is the one place active management plausibly adds value — sold
  honestly.** The drawdown + vol overlays (§2 P1-3/P1-4) promise **drawdown/tail
  reduction, never a Sharpe gain** (crypto's leverage effect is reversed); the return
  cost (~40% of B&H upside for the drawdown cut) is shown via mutual-non-dominance
  framing, and the floor is disclosed as probabilistic.

- **Cost realism + turnover make the null legible.** Turnover (§2 P1-1) and
  state-aware costs (§2 P1-6) show *why* the advisor keeps recommending hold (costs
  favour the lowest-turnover strategy) — the thesis made visible and auditable, not
  asserted.

- **The off-track list is itself honesty.** Refusing multi-asset alpha-harvesting,
  return-prediction-in-the-ranking, automated alpha search, and LLM-as-trader (§3)
  keeps the product from quietly becoming the over-claiming framework the research
  exists to deflate. Each "do-not-build" is a documented, cited decision (§2 P2-7).

The net: v2 doesn't chase alpha — it **hardens the honest verdict, surfaces the
evidence, and shapes risk** — which is exactly "a framework for trading with
traceable and plausible trading."

---

## Handoff envelope

```toml
[handoff]
from        = "analyst"
to          = "architect"
feature     = "v2-analysis"
trace_refs  = ["REQ-V2-ANALYSIS-001"]
verdict     = "READY"
priority    = "P0"

[inputs]
brief       = "inline"
artifacts   = [
  "research/APPLICATIONS.md",
  "research/SYNTHESIS.md",
  "research/backtesting/application-overfitting-and-multiple-testing.md",
  "research/backtesting/application-cost-and-impact-modeling.md",
  "research/data/application-splits-leakage-cv.md",
  "research/data/application-pit-labeling-stationarity.md",
  "research/data/application-synthetic-and-monte-carlo.md",
  "research/strategies/application-ta-efficacy-and-selection-bias.md",
  "research/strategies/application-execution-and-sizing-rules.md",
  "research/strategies/application-factor-replication-and-the-counter-thesis.md",
  "research/ml-trading/application-ldp-pipeline-and-meta-labeling.md",
  "research/ml-trading/application-classical-ml-and-baselines.md",
  "research/deep-learning/application-forecasting-and-significance.md",
  "research/deep-learning/application-deep-rl-and-hedging.md",
  "research/evolution/application-automated-strategy-search.md",
  "research/evolution/application-anti-overfitting-and-search-discipline.md",
  "research/llms/application-llm-narration-and-agents.md",
  "research/llms/application-llm-timeseries-foundation-models.md",
  "research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md",
  "research/risk-and-sizing/application-position-sizing-and-bet-sizing.md",
  "research/crypto-market-structure/application-exogenous-signal-arms.md",
  "research/crypto-market-structure/application-volatility-regimes-and-overlays.md",
  "research/crypto-market-structure/application-data-integrity.md",
  "spec/product.md",
  "spec/v2/README.md",
  "crates/ui/src/screens/{leaderboard,forward_plan,tune}.rs",
  "crates/ui/src/{leaderboard,forward_plan}/state.rs",
  "crates/backtest/src/bakeoff/{mod,rank,robustness,bootstrap,sweep}.rs",
]

[outputs]
spec_files  = ["spec/v2/v2-analysis.md", "spec/trace.toml"]
adrs_added  = []

[open_questions]
items = [
  "Plugin architecture for slotting v2 features, or is the existing additive-arm/report-annex/overlay pattern sufficient? (My read: sufficient.)",
  "Does a DSR/PBO crown-eligibility predicate count as additive to the FROZEN gate, or is it a frozen-rule change? (Recommend: ship scorecard report-only first; anticipate a one-line veto switch.)",
  "Anchor protocol for the scorecard report fields — new section vs new file vs ADR-0038 §D6 re-emission?",
  "Return-matrix capture (PBO enabler) now, or defer and ship closed-form MinBTL+DSR+N_eff first? (Recommend: defer.)",
  "N_eff: closed-form sufficient at MAX_SWEEP_CONFIGS=24 (T≫24, so M>T mandate likely moot), or carry clustering headroom?",
  "Cost-model default change vs 119 anchors — opt-in-forever, versioned re-anchor, or 'good enough'? (CX-7, largest blast radius.)",
  "Promote the Tune screen into a named TRAINING/Calibrate stage with the P0 scorecard readout? Where does the shared vol-estimator live (forecast vs strategy)?",
]

[assumptions]
items = [
  "The frozen robustness gate + bands + B&H benchmark stay FROZEN; all v2 work is additive (the operator's standing non-negotiable).",
  "Paper/sim only and single-coin remain hard constraints; multi-asset 'which coin among many' is a track change, not a v2 arm.",
  "'Training' in the operator's four-stage vision = vol/risk-for-sizing + gate-tied param tuning, NEVER price/return prediction (the research is decisive).",
  "The ui purity seam (no dep on strategy/exec/llm/models) holds; the scorecard crosses as a passive mirror struct.",
  "MAX_SWEEP_CONFIGS=24 is stable, so the DSR haircut is modest and MinBTL bites hardest — the honest framing is 'small calibrated haircut', not 'gate rejects everything'.",
  "F5b forward-fidelity (build_registry_for reuse) is a correctness fix with zero FROZEN-constraint impact.",
]
```

HANDOFF → architect
Input files: spec/v2/v2-analysis.md, spec/product.md, research/APPLICATIONS.md, research/SYNTHESIS.md
Open questions: see §5 + the envelope `[open_questions]` (plugin-vs-additive; DSR-veto-vs-report-only; anchor protocol; return-matrix-capture timing; N_eff method; cost-default-vs-anchors; Tune-as-TRAINING-stage)
