---
title: "The Honest Advisor — Single-Coin Investment Advisor (paper)"
status: final
created: 2026-07-24
updated: 2026-07-24
---

# PRD: The Honest Advisor — Single-Coin Investment Advisor (paper)

> **Migration provenance — read this first.** Migrated from `spec/product.md` 2026-07-24 (BMAD Phase 1); **`spec/` remains authoritative until Phase 5b cutover.** This is a **brownfield PRD**: it describes a **shipped, feature-complete product in maintenance mode** (feature-complete 2026-07-09; remediation hardening P0–P8 completed 2026-07-10), not an aspirational build plan. Sources of truth transformed here: `spec/product.md` (the ratified product spec), `spec/dev-notes/do-not-build-register.md` (settled dead-ends + the era-qualified thesis wording), and `CHANGELOG.md` (the shipped-state index).

## 0. Document Purpose

This PRD is for the **operator** (the single stakeholder) and the **downstream BMAD workflow owners** — architecture, epics/stories, sprint status — as the planning root of the BMAD migration. It is Glossary-anchored (§4); features are grouped with globally numbered FRs nested under them (§5); inferences made during migration are tagged inline as `[ASSUMPTION]` and indexed in §14. A companion **`architecture.md`** (authored in parallel from `spec/architecture/`) carries system design; **`PRD-addendum.md`** carries the engine-reuse inventory, engine mechanics, and the rejected-alternative rationale for the key product decisions — depth that belongs downstream of this document.

**Stakeholders.** *Vitaliy* — product owner, operator, and the single retail-investor user the product is designed for. *Claude agents* — the dev-time multi-agent workflow (no runtime role).

**Product status.** The Honest Advisor is **shipped and feature-complete** (2026-07-09). The current posture is **maintenance mode**: "prove it's done," not "do more." New feature proposals are checked against the do-not-build register (§7.2) before any planning work.

## 1. Vision

The whole product answers a single, concrete question:

> "I have **€200**. I want to put it into **one** crypto (say **XRPUSD**). Which strategy should I use, and what should I do over the next few days?"

The Honest Advisor is a **decision-support + paper-simulation tool for one retail investor with a small budget**. In one guided flow the user picks a coin and a budget, the tool bakes off *every* available strategy on that coin over a configurable lookback window, ranks them under a robustness gate with buy-and-hold always in the field as the benchmark arm, explains the winner in plain language, lays out an honest conditional forward plan, and then paper-trades the selection forward on real incoming data so the user watches running P/L on their simulated €200.

The value proposition is: **"a transparent, reproducible, risk-aware bake-off + paper-sim for putting a small fixed budget into one coin" — measured honesty, not asserted alpha.** The product does not promise the selected strategy will win; it promises an honest, measured, reproducible bake-off and a forward paper-sim so the user can see for themselves. This is a **re-framing of an existing, working engine, not a new build**: the backtest engine, strategy library, LLM integration, paper simulator + agent runtime, real market data, double-entry audit ledger, reflection memory, and the iced cockpit all pre-existed the 2026-06-19 pivot and became the engine of the guided journey (inventory in `PRD-addendum.md` §A).

**Product goals** (all delivered):

- Let a non-expert user go from "I have €X for coin Y" to "here is the best-ranked strategy, here's why, and here's what it would do with my budget" in one guided flow.
- Reuse the shipped engine for every heavy-lifting step; net-new code is the orchestration (bake-off loop), the ranking/recommendation surface, the forward-plan generator, budget-aware sizing, and the guided input UX.
- Keep every recommendation auditable and reproducible: seed, lookback window, per-strategy KPIs, and the chosen strategy are recorded so the same inputs reproduce the same ranking.
- Stay paper-only and safe: no real orders, a hard simulated-budget cap, not-advice disclaimers on every recommendation surface.

## 2. Honesty Doctrine (load-bearing)

*This section is the product's epistemic core. Every downstream artifact — architecture, stories, UI copy, marketing of any kind — must preserve it verbatim in meaning. It has no natural home in a generic PRD template; it gets its own section because the template serves the content.*

### 2.1 The era-qualified thesis

The product's validated thesis (900 papers, 9 independent reviews, deep-read at primary-source depth):

> **On the current deep-liquidity market era (2023+), no active strategy robustly beats buy-and-hold net of costs on a single liquid coin** — the modal outcome on every window the advisor can actually run.

The thesis is **era-qualified, never universal**. Do not state it as "no active strategy beats passive" without the era clause: the universal form was falsified by the P2 corpus-expansion re-run (2026-07-10) and the qualified form is the ratified wording. The prior identity of this repo was a research stack hunting for an active edge; that hunt concluded **2026-06-08: ship passive** — across the three reachable channels (price/OHLCV, derivatives-positioning, on-chain), on the then-current sample (2023-24 large-caps), no active strategy beat passive buy-and-hold net of cost under a pre-registered block-bootstrap Monte-Carlo rule (passive: +1.74 Sharpe 2023 / +1.10 2024). That result is **not discarded by the advisor pivot — it is a feature of it.**

### 2.2 Efficiency migration — why the era clause is a strength

The P2 corpus-expansion verdict re-run ran the same frozen gate back across older, thinner-liquidity eras and found **real, gate-crowned, cost-annex-robust active edges in the early market (2017-18, 2020, 2021-22) that decay to ~zero by 2023+** (2023-24: 0/1 ActiveWins; 2025-26: 2/10, both marginal) — the textbook efficiency-migration / anomaly-decay pattern the research corpus predicted. The machinery did not merely fail to find alpha — it **positively detected real historical edges and then detected the boundary where they died**, and reported it. That is a stronger honesty claim than "we looked and found nothing," and it is the credibility story, not a wobble.

Always qualified by: **survivor-of-survivors bias** (the 2017-18 winners are the extreme eventual survivors, not knowably harvestable ex-ante), **old-era cost realism as a stated limit** (old-era crown Sharpe margins are upper bounds; the direction is conservative for today's verdict), and **the scorecard errata** (the initial "16 of 19 old-era crowns clear DSR ≥ 0.95" figure was retracted same-day by the scorecard NaN-variance fix — post-fix, 0/19 clear DSR; the old-era edges are gate-crowned and cost-annex-robust, **not** DSR-certified). **Forward advice is unchanged:** every window the advisor runs ends at "now," where the modal outcome is still just-hold; old-era edges are unreachable in any runnable window, and chasing them is the settled dead-end A-3 (§7.2), not a gap.

### 2.3 Two consequences the product must honour

1. **Passive buy-and-hold is always in the bake-off as the benchmark arm.** "Best strategy" is meaningful only relative to just-holding-the-coin. When buy-and-hold wins for a given `(coin, window)`, the recommendation surface says so plainly ("for this coin over this window, nothing beat simply holding").
2. **The robustness machine is the credibility layer, not decoration.** A strategy that wins on a single backtest path but is FRAGILE under resampling is shown, never crowned. The Monte-Carlo robustness harness is the differentiator that makes "we ranked them" trustworthy rather than a lucky-draw leaderboard.

### 2.4 The stance

- **Paper / sim only.** No live trading, no real orders, no exchange execution, no KYC, no withdrawals. The €200 is a **simulated** budget. (Standing operator constraint; live execution was removed from scope 2026-06-12.)
- **Not financial advice.** A personal backtest-and-simulate sandbox. Every surface that shows a "recommendation" carries a not-advice, past-performance-is-not-indicative disclaimer. The tool helps the user *reason*; it does not tell them to risk money.
- **Not an alpha product.** The differentiator is measured honesty, not asserted alpha. **`BenchmarkWins` is the modal honest outcome** on real crypto: when every active arm is FRAGILE, buy-and-hold is crowned and the €200 paper-trades as a hold. A null result ("nothing beat holding") is a valid, expected, shippable outcome — the product working, not failing.

## 3. Target User

### 3.1 Jobs To Be Done

- **Decide without gambling:** "I have a small fixed amount (€200) I'm willing to put into one coin — tell me, with evidence, which strategy would have served me and whether anything beats simply holding."
- **See it before risking it:** watch the chosen strategy paper-trade my budget forward on real data before any real-world decision.
- **Understand, not obey:** get a plain-language "why this one" grounded in the actual numbers, plus the honest caveat when the answer is "just hold."
- **Trust the process:** know the ranking is reproducible (recorded seed + window), robustness-gated, and not a lucky-path leaderboard.
- **Tinker safely:** tune strategy hyperparameters and immediately see whether a tuned config survives resampling — with overfit configs locked out of promotion.

### 3.2 Non-Users (v1)

- Anyone wanting **live execution** or real-money automation — the product will never place an order.
- **Multi-asset portfolio** builders — the journey is deliberately one coin, one budget at a time.
- **Alpha-seekers / professionals** — no HFT, no market-making, no derivatives desk, no prediction engine; the modal honest answer is "hold."
- **Multiple users / teams** — single-operator by design; no auth, no RBAC.

### 3.3 Key User Journeys *(all shipped — these describe the working product)*

- **UJ-1. Vitaliy puts a simulated €200 on one coin — the golden journey.**
  - **Persona + context:** Vitaliy, the operator and the product's single retail-investor user, wants to know what to do with €200 on one coin — without risking a cent.
  - **Entry state:** cockpit open on his machine (single-operator, no auth); pinned real-market corpus available; the DATA → CALIBRATE → ANALYZE → SUGGEST stepper band orients him.
  - **Path:** (1) In the guided input he picks the coin (e.g. `BTCUSDT`), enters budget `200`, picks a lookback (e.g. `2024 H1`; presets span 2 weeks → ~4 years) — the display shows the honest conversion "€200 ≈ $216.00 (at 1.08 EUR/USD, config)". (2) The bake-off runs every arm plus the buy-and-hold benchmark arm on the same `(coin, window)`, with a determinate progress bar ("Running ⟨id⟩ — X of N"). (3) The Leaderboard renders the ranked field — Sharpe, total return, max-drawdown side by side, churn and tail-risk columns, the data-quality panel — with one highlighted recommendation, the overfitting scorecard beside it, and a plain-language rationale. (4) The Plan screen shows the conditional forward plan: current stance, the entry/exit rules in plain language, projected €200 sizing, configurable 1–30-day horizon. (5) He launches the forward paper-trade and watches running P/L on the Live view.
  - **Climax:** the honest verdict lands. On the golden demo input `(BTCUSDT, €200, 2024 H1)` every active arm comes back FRAGILE under the frozen gate, so the surface says plainly: buy-and-hold is crowned — "simply holding is the least-bad choice on this window" (`BenchmarkWins`; real-data ground truth: buy-and-hold +67.82% on BTCUSDT 2024-Q1) — and the €200 paper-trades as a hold. **This is the product working, not failing.**
  - **Resolution:** the bake-off result is a dated, reproducible artifact (seed + window recorded) he can re-open; the Live view keeps tracking his simulated €200; not-advice disclaimers frame every surface.
  - **Edge cases:** a coin/window outside the pinned corpus triggers an on-demand fetch into a git-ignored cache; if the opt-in LLM narration fails any faithfulness check it silently falls back to the templated copy.

- **UJ-2. Vitaliy tunes a strategy — and the gate locks out the overfit config.** On the Calibrate stage he sweeps a bounded per-family parameter grid (e.g. MACD fast/slow/signal). Each config is scored through the **identical frozen robustness gate**; the surfaced p5/p50/p95 distribution spread — not the in-sample point — is the anti-overfit affordance. A config that reads FRAGILE has its "Use this config" promotion **locked**; a non-FRAGILE tuned config promotes into the forward plan and paper-run, whose header says honestly: "you tuned this; it survived resampling on THIS window — not a guarantee, not advice."

- **UJ-3. Vitaliy exports the plan to take away.** From a Ready forward plan he clicks "Export this plan" and gets a deterministic markdown file (`plan-{coin}-{window}-{seed8}.md`) with operator-ratified wording — including the unbounded-loss warning when a short is crowned. No orders are placed, no venue API is touched, no LLM runs at export time.

## 4. Glossary

*Downstream workflows must use these terms exactly.*

- **Arm** — one competitor in a bake-off: a strategy (or strategy variant) run on the same `(coin, window)` as every other arm.
- **Bake-off** — the orchestrated run of every available arm plus the benchmark arm on one `(coin, window)`, producing a ranked leaderboard and a recommendation.
- **Benchmark arm** — passive buy-and-hold of the chosen coin; always present in every bake-off; the null hypothesis candidates are scored against (exempt from the fragility gate, ADR-0066).
- **Budget** — the user's simulated stake (e.g. €200), converted once at input (EUR→USDT at a configurable static rate) and enforced as a hard sizing cap. Never real money.
- **Lookback window** — the historical slice a bake-off runs over, configurable from 2 weeks to ~4 years; every window ends at "now" (or at the chosen range's end for pinned demos).
- **Crown / crowned pick** — the #1-ranked arm a bake-off recommends. A FRAGILE arm is never crowned.
- **FRAGILE / Robust** — the frozen gate's verdict on an arm: FRAGILE when the resampled p5 Sharpe < 0; Robust otherwise.
- **Frozen robustness gate** — the pre-registered Monte-Carlo rule (stationary/moving-block bootstrap over real returns, Politis–White block length; p5 Sharpe < 0 → FRAGILE) whose bands and verdict classifier are byte-frozen; changes require an explicit operator decision + ADR (§7.2 E-1/E-2).
- **Outcome code** — the bake-off's headline result: `ActiveWins` (a robust active arm is crowned), `BenchmarkWins` (all active arms FRAGILE, or buy-and-hold tops the ranking — the modal honest outcome), `AllFragile` (residual only in benchmark-less fields).
- **Overfitting scorecard** — the report-only credibility annex beside every recommendation: effective number of trials (N_eff) → Deflated Sharpe Ratio (DSR) → minimum backtest length (MinBTL). Informational; never a crown veto.
- **Pre-registered slate** — a FIXED, code-declared set of arms chosen *before* results; the product's standing overfit defense (no search, no parameter hunt).
- **Forward plan** — the honest, conditional description of what the crowned pick would do next: current stance + standing entry/exit rules in plain language + projected budget sizing over a 1–30-day horizon. Not a price forecast, not a dated trade calendar.
- **Forward paper-trade** — the crowned (or promoted tuned) strategy executing on real incoming bars with simulated fills and the budget cap; its running P/L renders in the **Live view**.
- **Narration** — the opt-in LLM-rendered plain-language "why this one," faithfulness-checked against the structured result; falls back to templated copy; never enters the ranking.
- **Anchored report** — a backtest report whose body SHA-256 is locked in the anchors registry (119 anchors); anchored bodies are byte-immutable and gate every change (`verify_anchors` 119/119).
- **PIT / as-of discipline** — the point-in-time data rule: any exogenous series joins the bar clock through an as-of API that makes look-ahead unrepresentable.
- **Era-qualified thesis** — the §2.1 statement, always scoped to the current deep-liquidity era (2023+), never universal.
- **Research mode / Paper mode** — the two operating modes: backtest-only with deterministic seeds and cached LLM replay (bake-off runs here) vs live data feed with simulated fills (the forward paper-trade runs here). A former Live mode was removed from scope 2026-06-12 and is not wired.

## 5. Features

*All features below are shipped. FRs are numbered globally; each carries testable consequences that the delivered tests/reports already verify — they are the acceptance record for the epics/stories phase, not a to-build list.*

### 5.1 Guided investment input

**Description:** The journey opens with one guided form — coin, budget, lookback — instead of config files. Realizes UJ-1 (step 1). The budget is entered in EUR and honestly converted for the USDT-denominated engine.

#### FR-1: Guided coin + budget + lookback input

The user can start a bake-off from a single guided input (coin, budget, lookback window) in the cockpit.

**Consequences (testable):**
- Lookback selection spans 2 weeks → ~4 years via presets; the chosen `(coin, window, budget)` is recorded with the run.
- The same guided input preseeds drill-down navigation (e.g. leaderboard row → Lab inspection uses the leaderboard's coin + window authoritatively).

#### FR-2: Honest EUR→USDT budget conversion

The user can enter the budget in EUR; the system converts once at the input boundary using a configurable static EUR/USD rate and displays the conversion honestly.

**Consequences (testable):**
- Display reads "€200 ≈ $X (at R EUR/USD, ⟨source⟩)" — no "FX not modelled" pretence; the converted amount (e.g. cap 216, not 200) demonstrably reaches the sizing cap (day-1 conversion-applied gate).
- The ranking is FX-invariant: a scalar on the budget cannot change which arm wins.

**Out of Scope:** a first-class `Eur` currency type, ledger FX-PnL, FX trading/prediction (D4 option (c), rejected). A live-fetched rate is a noted v0.3 refinement (§13).

### 5.2 Bake-off orchestration

**Description:** The bake-off loops the existing backtest engine over every available arm plus the benchmark arm on the same `(coin, window)`. Realizes UJ-1 (step 2).

#### FR-3: All-arms bake-off with the always-present benchmark arm

The system can run every available arm and the buy-and-hold benchmark arm on one `(coin, window)` and produce a structured, ranked result.

**Consequences (testable):**
- Buy-and-hold is present in every bake-off field; no configuration removes it.
- A determinate progress indicator reports per-arm progress ("Running ⟨id⟩ — X of N").
- The bake-off *reads* the engine and must not perturb anchored scenarios: `verify_anchors` stays 119/119 across any bake-off run (new arm classes emit no anchored reports).

#### FR-4: Timeframe and starting-capital controls

The user can choose the decision bar size (H1 / H4 / D1) and the simulation's starting capital.

**Consequences (testable):**
- The corpus is resampled once to the chosen timeframe and the *same* coarser bars feed every arm (H1 = identity); the timeframe may legitimately change the ranking.
- Starting capital scales absolute equity (~2× capital → ~2× absolute equity) but does **not** change %-based returns or the ranking — and the UI says so honestly.

#### FR-5: On-demand market data for any coin + window

The user can bake off a `(coin, window)` the pinned corpus doesn't cover; the system fetches it on demand into a git-ignored cache.

**Consequences (testable):**
- Missing exogenous corpora degrade gracefully (arm skipped, never a crash); anchored evidence is unaffected by any fetch (anchor-safe by construction).

### 5.3 Ranking, recommendation & credibility surface

**Description:** The ranked leaderboard plus a single highlighted recommendation, with the robustness gate and the report-only overfitting scorecard as the credibility layer. Realizes UJ-1 (step 3); implements decisions D1 and the §2.3 consequences.

#### FR-6: Robustness-gated ranking (D1) with the honest `BenchmarkWins` outcome

The system ranks arms by: fragility-ineligibility first, then Sharpe, then total return, then lower max-drawdown, then id — and crowns the benchmark when nothing active clears the bar.

**Consequences (testable):**
- A FRAGILE arm is shown on the leaderboard but can never be crowned (including tuned configs and ensembles: a high-Sharpe-but-fragile ensemble is demoted to the robust single).
- The benchmark arm is exempt from the fragility gate (it is the null hypothesis, not a candidate — ADR-0066); when every *active* arm is FRAGILE the outcome is `BenchmarkWins`, never `AllFragile`, and the recommendation says plainly "simply holding is the least-bad choice on this window."
- On `BenchmarkWins`, the budget paper-trades as a hold.

#### FR-7: Leaderboard transparency columns

The user can read Sharpe, total return, and max-drawdown side by side for every arm, plus a Churn (turnover) column, a coherent tail-risk block (CVaR / median / skew), and a display-only data-quality/trust panel.

**Consequences (testable):**
- The data-quality panel (venue + provenance, venue-trust classification, an always-present survival-bias caveat, plain-language warnings) never feeds any gate, rank, or verdict — display-only.
- The benchmark row renders a muted "baseline is path-dependent" note, not a candidate-style FRAGILE pill.

#### FR-8: Report-only overfitting scorecard + crown-credibility band

The system computes N_eff → DSR → MinBTL beside every recommendation, and a crowned pick that fails deflated-Sharpe carries an unmissable in-body weak-evidence band.

**Consequences (testable):**
- The scorecard is informational: the ranking never reads it (`crown_clears_dsr` is never a veto — §7.2 E-1; the veto remains a ready-but-unbuilt operator decision).
- The weak-evidence band appears on a DSR-failing crowned pick and never as a badge on a hold pick.
- Empirical basis (no-alpha CI): on deterministic pure-noise processes the primary frozen gate alone crowns noise ~1 in 5 seeds, and the scorecard flagged every chance-crown — this is why the report-only scorecard is load-bearing.

#### FR-9: Confidence-not-verdict framing + mandatory disclaimers

Every recommendation surface frames the result as a confidence check, not a verdict, and carries the not-advice disclaimers.

**Consequences (testable):**
- The recommendation copy carries "survived resampling on this window; not a guarantee of future edge" framing.
- Every surface showing a recommendation (leaderboard, plan, Live, export) carries not-advice + past-performance-is-not-indicative + simulated-budget disclaimers.

### 5.4 Explanation — "why this one"

**Description:** A plain-language rationale for the crowned pick: always available as structured templated copy; optionally rendered by an LLM under a strict faithfulness contract. Realizes UJ-1 (step 3); the LLM never enters the ranking.

#### FR-10: Templated rationale (always available)

The system renders a plain-language "why this one" from the structured result (reason codes + real KPIs + robustness flags) with no LLM involved.

**Consequences (testable):**
- The templated copy renders for every outcome code, including `BenchmarkWins` ("why buy-and-hold won, why the runners-up lost").

#### FR-11: Faithful LLM narration (opt-in)

The user can opt in to an LLM-rendered narration of the *actual* structured bake-off result — and nothing more.

**Consequences (testable):**
- A deterministic, LLM-free faithfulness post-check discards a narration that crowns a different winner, contradicts the outcome code, emits a number not in the inputs, or trips the frozen predict/advise banned-phrase list (verbatim-number matching included) — falling back to the templated copy.
- Narration is a second async step: the structured result renders immediately; narration lands later; any failure (disabled / error / timeout / budget / post-check) yields the templated copy, never a half-answer.
- The LLM never enters the ranking; a fake-LLM seam serves all tests/renders (no network).

### 5.5 Forward plan & real-world hand-off

**Description:** The honest "what would it do next" surface between the recommendation and the Live view, plus a deterministic export. Realizes UJ-1 (step 4) and UJ-3; implements decision D2.

#### FR-12: Conditional, rule-driven forward plan (D2 — not a forecast)

The system presents the crowned pick's forward plan as: current signal stance (BUY / HOLD / SELL on the latest bar) + the entry/exit rules in plain language + projected budget-aware sizing, over a configurable 1–30-day horizon (default 7).

**Consequences (testable):**
- The plan's IF/THEN rules are faithful to the strategy's real configuration (no fabricated thresholds); ensemble plans name their members (e.g. "at least 2 of {MACD trend, RSI reversion, Bollinger reversion}").
- The plan carries not-a-prediction / not-advice framing; pre-computed future orders are rejected for price-dependent strategies by design — the Live view is the plan unfolding.
- The plan and the forward paper-trade describe/execute the same rules (consistent by construction).

#### FR-13: Deterministic plan export

The user can export a Ready plan as a markdown file with operator-ratified wording.

**Consequences (testable):**
- Export writes `plan-{coin}-{window}-{seed8}.md` via a deterministic serialiser (golden-locked verbatim wording, including the short-crowned unbounded-loss case).
- No orders, no venue API, no LLM at export time; the affordance exists only on a Ready plan.

### 5.6 Forward paper-trade (Live)

**Description:** The selected strategy paper-trades forward on real incoming data; the Live view shows running P/L on the simulated budget. Realizes UJ-1 (step 5).

#### FR-14: Fidelity — the real crowned strategy runs forward

The forward run executes the *actual* crowned strategy (or promoted tuned config), not a proxy.

**Consequences (testable):**
- Registry identity: the forward-run strategy is the same artifact the bake-off ranked (anti-fake gate: registry-identity + divergence proof vs a proxy); an unknown id yields a typed error, never a silent fallback.
- Every arm class in the advisor field is forward-buildable — crowning any arm cannot fail the forward run.
- The supervisor builds the new registry before tearing down the old loop, so a build error cannot strand the Live view.

#### FR-15: Hard budget cap + honest P/L display

Paper sizing never deploys more than the user's simulated budget, and the Live surface shows honest P/L.

**Consequences (testable):**
- The budget cap is a hard limit enforced in sizing (day-1 baseline-equity-divergence e2e covers the sizing modifier, per the standing non-negotiable).
- Where short arms run, the display shows honest negative P&L and the "a short can lose more than your budget" disclaimer.
- Opt-in min-notional / lot-size realism applies at the universal fill chokepoint (auditable skips); the default path stays byte-identical (anchors by construction).

### 5.7 Calibrate — gate-tied parameter tuning

**Description:** A first-class Calibrate stage (the DATA → CALIBRATE → ANALYZE → SUGGEST stepper band orients the whole journey) where hyperparameter tuning is scored by the same credibility layer as everything else. Realizes UJ-2.

#### FR-16: Gate-tied parameter sweep with the FRAGILE promote-lock

The user can sweep a bounded per-family parameter grid; each config is scored through the identical frozen robustness gate.

**Consequences (testable):**
- The surfaced anti-overfit affordance is the p5/p50/p95 distribution spread, not the in-sample point; an overfit config reads FRAGILE and its "Use this config" affordance is locked.
- The sweep is bounded (24 configs max) with honest truncation; the swept config is identity-guarded byte-identical to the shipped strategy definition it claims to be (a swept config is the same strategy that gets ranked).
- The frozen gate stays byte-frozen (the sweep uses an additive distribution sibling; bit-identity of the gate is proven).

#### FR-17: Promotion of a surviving tuned config

The user can promote a non-FRAGILE tuned config into the forward plan and paper-run.

**Consequences (testable):**
- A FRAGILE config can NEVER promote (the anti-overfit gate is final); only non-FRAGILE rows expose a real promote button.
- The promoted forward plan shows the tuned rules (not the defaults) plus a distinct honesty header ("you tuned this; survived resampling on THIS window — not a guarantee, not advice").
- The promotion carries the config through the launch path (wiring proven by a pure-state test, not just paint).

### 5.8 Strategy library & pre-registered arm classes

**Description:** The bake-off field grows only by FIXED, pre-registered slates — never by search. Shipped arm classes: the 4 base rule engines (SMA / MACD / RSI / Bollinger), k-of-n vote ensembles, a separate short-capable slate, a signal-library expansion (Donchian breakout/floor, volume-confirmed breakout, ROC momentum, OBV), and two exogenous-channel probes (Deribit DVOL implied-vol regime; macro risk-on/off). Each expansion's **null result ("also FRAGILE, hold still stands") was the expected, valid, shippable outcome** — honest coverage, not an alpha claim.

#### FR-18: Pre-registered slates only (no search)

The system's arm field grows only by fixed, code-declared, pre-registered slates scored through the identical frozen gate + benchmark arm.

**Consequences (testable):**
- No weight/threshold/membership search exists anywhere in the field-definition path; pre-registration is the standing overfit defense.
- Each new arm class ships a day-1 divergence e2e (its equity diverges from its members'/the benchmark's where non-trivial) and leaves anchors at 119/119 (no anchored reports from new arms).
- Retired ML/forecaster chains (TCN / PatchTST / GARCH-σ / LLM-forecaster) are opt-in only, never default arms (concluded negative verdicts).

#### FR-19: Simulated directional shorts (paper-only)

The user can run the separate short-capable slate, with honest short mechanics and honest warnings.

**Consequences (testable):**
- Shorts are simulated positions with correct signed P&L, maintenance-margin liquidation (cash can go negative — losses NOT capped), and per-bar funding; the long-only default field stays byte-identical.
- The UI carries the "a short can lose more than your budget" disclaimer; the standard advisor field remains long-only unless the short field is chosen.

### 5.9 Robustness gate, reproducibility & audit

**Description:** The credibility layer itself: the frozen gate, deterministic reproducibility, the double-entry audit ledger, the anchored evidence corpus, and the point-in-time data discipline. Implements the §2.3 consequences; underpins every other feature.

#### FR-20: The frozen robustness gate

The system judges every arm by the pre-registered Monte-Carlo rule: resample real returns (stationary/moving-block bootstrap, Politis–White auto block length) into an ensemble of paths and read the outcome distribution (Sharpe p5/p50/p95, drawdown tail, probability of loss) against p5 Sharpe < 0 → FRAGILE.

**Consequences (testable):**
- This is uncertainty quantification, not prediction; the verdict classifier and bands are byte-frozen — any change to the gate's effective crowning behaviour requires an explicit operator decision + its own ADR + an anchor-impact assessment + a day-1 test proving the change bites (§7.2 E-1/E-2).
- The gate is active in the live cockpit bake-off (not just offline reports): a fragile candidate is shown but never crowned.

#### FR-21: Reproducibility, audit ledger & anchored evidence

Every recommendation is auditable and reproducible end to end.

**Consequences (testable):**
- The bake-off seed, lookback window, per-strategy KPIs, and the chosen strategy are recorded; the same inputs reproduce the same ranking; the bake-off result is itself a dated, re-openable report.
- Every paper fill flows through the double-entry audit ledger (Σ debits == Σ credits provable in isolation).
- The 119 anchored backtest report bodies are byte-immutable (`verify_anchors` 119/119 gates every change); look-ahead is unrepresentable by construction for exogenous joins (the as-of/PIT primitive + a look-ahead lint, with explicit publication lag).

## 6. Key Product Decisions (D1–D5, resolved as built)

*The five operator decisions from the 2026-06-19 pivot, with their shipped resolutions. Options considered + rejection rationale live in `PRD-addendum.md` §C.* [ASSUMPTION: D1–D5 are treated as ratified-as-built — `spec/product.md` § Open decisions still shows the literal "_Operator to confirm._" checkboxes unticked, while its `status: shipped` flip (2026-06-21) records that the operator personally defined the product and the MVP shipped implementing exactly the recommended defaults.]

| # | Decision | Resolution (as built) | Where |
|---|---|---|---|
| **D1** | "Best strategy" ranking metric | **Risk-adjusted with a robustness gate**: sort by Sharpe; FRAGILE arms shown, never crowned; Sharpe + return + max-drawdown side by side; tie-break return → lower drawdown; benchmark exempt from the gate; all-active-fragile ⇒ `BenchmarkWins`. | FR-6, FR-7 |
| **D2** | What the forward "plan" concretely is | **Current stance + plain-language rules + projected sizing + a forward paper-run** — the Live view IS the plan unfolding. A deterministic future-order schedule was rejected as misleading for price-dependent strategies. | FR-12, FR-14 |
| **D3** | Ensemble / LLM-mix scope | **Post-MVP, bounded, pre-registered**: MVP ranked one strategy; v0.2 shipped fixed vote-ensembles that earn the crown through the same gate + benchmark — never assumed-better, never a weight search. **LLM/ML stay narration-only** (the LLM never enters the ranking). | FR-11, FR-18 |
| **D4** | EUR budget on a USDT-quoted pair | MVP shipped the labelled 1:1 simplification, then the planned refinement replaced it: **one-time €→USDT conversion at a configurable static rate** with the honest "€200 ≈ $X (at R EUR/USD)" display. First-class `Eur` currency rejected. Residual fork (rate source: live-fetched v0.3) is open — §13. | FR-2 |
| **D5** | Confirm paper-only | **Re-affirmed** — the €200 is simulated; the product never places a real order; not-advice + simulated-budget disclaimers on every recommendation/Live surface. | §2.4, §10.1 |

## 7. Non-Goals (Explicit)

### 7.1 The IS-NOT boundary

- **Not a live broker.** PAPER / SIM ONLY — no live trading, no real orders, no exchange execution, no KYC, no withdrawals. The €200 is a simulated budget. (Live execution was removed from scope 2026-06-12.)
- **Not financial advice.** A personal backtest-and-simulate sandbox; every recommendation surface carries the not-advice + past-performance disclaimer.
- **Not a multi-asset portfolio manager.** One coin, one budget at a time; multi-coin portfolios are out of scope.
- **Not an alpha claim.** The product does not promise the selected strategy will win — it promises an honest, measured, reproducible bake-off and a forward paper-sim (the era-qualified thesis, §2.1–§2.2, governs all claims).
- **Not HFT, not market-making, not regulated derivatives, not tax/lot accounting.**

### 7.2 Settled dead-ends (the do-not-build register)

The authoritative register of settled dead-ends — with the guardrail/evidence that kills each and a cited rebuttal for re-proposals — is `spec/dev-notes/do-not-build-register.md` (13 entries, 5 groups). Summary; **do not re-litigate these**:

| Group | Entries |
|---|---|
| **A — Alpha-chasing** | A-1 a return/direction predictor in the ranking (NO prediction in the ranking is the product's one bright line; a forecaster may only feed a de-risk-only sizing overlay, never the crown) · A-2 deep nets / DRL as the alpha engine (tried-and-retired here) · A-3 automated alpha/parameter search (the product's own threat model; FIXED pre-registered slates are the standing defense) · A-4 LLM-as-trader / multi-agent debate (narration-only bright line) · A-5 new signal primitives added *to find edge* (pre-registered coverage only, never alpha-chasing) |
| **B — Scope-expansion** | B-1 multi-coin / basket portfolio (a separate product track, not an additive arm) · B-2 live trading / real orders / margin / KYC (built and removed 2026-06-12; do NOT re-propose) |
| **C — Infeasible / dishonest data** | C-1 on-chain arms (PIT-infeasible / endogenous; hard-stop fired 2026-06-08) · C-2 sentiment / macro-as-return-signal (fails Granger; at most a vol/regime overlay candidate) |
| **D — Execution-realism overreach** | D-1 market-impact / VWAP-TWAP scheduling (impact ≈ 0 at €200 scale) · D-2 order-book / HFT microstructure overlays (out of horizon; non-goal) · D-3 Kelly / μ-driven smart sizer as a return tool (keep fixed-fraction + vol-only, "size down," never "size up for alpha") · D-4 generative synthetic test data for the gate (keep the model-free block bootstrap) |
| **E — Gate-tampering & anchor-churn** | E-1 a silently-shipped DSR/PBO crown-eligibility veto (report-only by repeated operator decision; a veto requires an explicit decision + ADR + anchor-impact assessment + a day-1 bites-proof) · E-2 a cost-model default bump (would re-emit all 119 anchors for ≈0 honesty gain; new cost realism stays opt-in) |

### 7.3 Carried-forward resolved non-goals

- No auth / no RBAC (single operator, forever).
- No tax / lot accounting or regulatory reporting; operator reports focus on "is this working?" visibility.
- No cloud spend: local snapshots only for DR/backups.

## 8. Delivered Scope *(brownfield: the MVP shipped; this section records what exists)*

### 8.1 Shipped

The **MVP loop** — (1) guided input, (2) bake-off of the existing strategies + buy-and-hold on `(coin, lookback)`, (3) ranked leaderboard + single recommendation with the robustness flag, (4) forward paper-trade with budget-aware sizing in the Live view — shipped 2026-06-21. Everything then planned as post-MVP has also shipped. By tranche (the one-line-per-feature index is `CHANGELOG.md`; per-feature narrative is git history):

- **Advisor MVP + v0.2 (F1–F9 + dynamic data + EUR-FX):** bake-off + ranking engine, leaderboard screen, guided input, budget-aware sizing (day-1 divergence e2e), forward paper-trade with strategy fidelity, conditional forward plan, vote ensembles + activation of the robustness gate in the live bake-off, faithful LLM narration, EUR→USDT conversion, on-demand data.
- **Arm-class expansions (each an honest, pre-registered falsifier slate; each returned the expected null on the tested windows):** benchmark-exemption fix (restoring `BenchmarkWins` as the modal honest outcome), combination slate (13-arm field), directional shorts (separate slate), the leaderboard interactivity epic (inspect, timeframe/capital knobs, gate-tied tuning + promotion), signal-library expansion (18-arm field), DVOL implied-vol probe, macro risk-on probe.
- **v2 research-driven credibility tranche (11 features):** overfitting scorecard (report-only), churn + tail metrics, confidence-not-verdict framing, forward-coverage completion, shared vol estimator, de-risk-only vol overlay, drawdown-control overlay, opt-in cost-model variant, narration-faithfulness hardening, the no-alpha CI capstone, data-quality surface.
- **v3 "prove it's done" close-out:** Calibrate stage + journey stepper, the do-not-build register, the DSR report-only decision record, the end-to-end demo runbook on the golden `(BTCUSDT, €200, 2024 H1)` input, corpus expansion + the P2 verdict re-run (the era-qualification evidence).
- **Remediation P0–P8 hardening:** crown-credibility band, PIT-discipline lint + publication lag, opt-in lot realism, deterministic plan export, scorecard NaN fix + P2 errata, governance triad (lints + notes index + current-state rollup).
- **Foundation (pre-pivot, reused as the engine):** the v0→v5 strategy/backtest ladder, the concluded robustness program (ship-passive 2026-06-08), the retired research lines (kept as negative results), the Lumen cockpit/UI with its render-verification harnesses, and core infrastructure (reflection memory, operator success reports, PIT primitive, anchors + lint gates).

### 8.2 Not built / deferred by decision

- **DSR/PBO crown-eligibility veto** — a ready-but-unbuilt one-line switch; the scorecard stays report-only by explicit, repeatedly re-confirmed operator decision. [NOTE FOR PM: this is the one standing tension between credibility and simplicity — any future flip is an operator decision + ADR, never a maintenance edit.]
- **`lab-recipe-test-harness` v0.3.0+** — the one genuinely-open forward build item (backlog), not required for feature-completeness.
- **Candidates never built:** cockpit-app-bundle, iced-ecosystem-evaluation, ui-gallery-table-cell.
- **3-OS CI matrix** — long parked by operator decision; activated 2026-07-10 (operator-directed) with first-run shakeout fixes following. [ASSUMPTION: the `CHANGELOG.md` "stays operator-parked" note predates the 2026-07-10 activation commits and the activated state is current.]
- **Out of scope (a hypothetical follow-up project, not this product):** real-money execution, KYC, exchange API keys, withdrawals, multi-venue real-money, tax/lot accounting.

### 8.3 Maintenance-mode posture

There is **no add-more-features roadmap**: the v3 scoping found no coherent feature program left — the research well is drained of ship-worthy work and everything remaining is either explicitly-deferred polish or the alpha-chasing the product exists to refuse. The standing rule: **check the do-not-build register before proposing any feature.** Maintenance = keeping the gates green (anchors 119/119, spec lint, test suite), honoring the frozen-gate/anchor guardrails, and the open items in §13.

## 9. Cross-Cutting NFRs

- **Determinism & reproducibility:** deterministic seeds in research mode; the same `(coin, window, seed)` reproduces the same ranking; run-varying values never contaminate anchored report bodies.
- **Honesty surfaces:** not-advice + past-performance + simulated-budget disclaimers on every recommendation/Live/export surface; null results rendered as first-class outcomes, never hidden.
- **Auditability:** every paper fill and recommendation reconstructible from the ledger + recorded run parameters.
- **No-regression floor:** `verify_anchors` 119/119 and the full lib/integration/UI-snapshot suite green on every change.
- **UI truthfulness at the pixel layer:** cockpit surfaces are verified against rendered output (populated-state screenshots with negative controls), not proxy assertions — a passing proxy is not proof the screen draws.
- **Graceful degradation:** missing corpora skip arms without crashing; LLM failure always falls back to templated copy; a forward-run build error never strands the Live view.

## 10. Constraints and Guardrails

### 10.1 Safety

- **Paper / sim only** (terminal): no real-money execution, no KYC, no exchange keys, no withdrawals, no live orders. Research and Paper are the only operating modes; the former Live mode is removed and not wired.
- Risk limits are enforced as Rust types — an illegal order fails at construction; the **budget cap is itself a hard limit**.
- Kill switch (halt file / missed heartbeat → flatten + stop), per-symbol exposure cap, max-drawdown trigger, full audit log.

### 10.2 Money correctness

- Money math uses `Decimal`, never `f64`; every monetary value is typed (`Money<C>`); the engine is USDT-denominated (EUR handled per FR-2).
- The audit crate imports nothing from sibling crates; the ledger reconciler invariant (Σ debits == Σ credits) stays provable in isolation.

### 10.3 Frozen-evidence guardrails (process)

- **Anchored report bodies are byte-immutable** (119 anchors); even mechanical link-fix edits break the gate — changes go through the documented re-emission protocol or not at all.
- **The robustness gate, bands, and benchmark semantics are frozen**; effective-behaviour changes require an explicit operator decision + a dedicated ADR + an anchor-impact assessment.
- **Every strategy overlay or sizing-modifier ships with a day-1 baseline-equity-divergence e2e** (the no-op-overlay precedent: unit tests + anchored reports are NOT sufficient to catch a computed-but-never-applied modifier).
- Crate layering: the UI crate never depends on strategy/exec/models/LLM crates; the bake-off orchestrator lives engine-side, not in the UI.
- No secrets in git; keys via env / secret store.

### 10.4 Cost

- LLMs via API with prompt caching and a strict monthly cost budget with 80% / 100% auto-degrade; local Ollama for cost-free dev; the LLM appears only in opt-in narration (one cheap, cacheable call per recommendation) — never in the bake-off hot loop.
- Monthly opex ladder $45 / $135 / $360 (research mode is LLM-free and cheap).

### 10.5 Privacy & operational simplicity

- Single operator; no auth, no RBAC, no multi-tenancy.
- Local snapshots only for DR; zero cloud spend.

## 11. Language, Runtime & Dependency Policy

- **Rust** stable, edition 2024, single workspace; library code returns `Result`, no `unwrap` outside tests; `tracing` (never `println!`) in library code; `fmt` + `clippy -D warnings` as standing gates.
- Lean on existing, boring Rust crates rather than reinventing quant primitives (tokio / serde / thiserror / reqwest / clap / criterion / proptest family); ML prototyping defaults exist but retired chains stay opt-in.
- LLM providers behind a trait: Anthropic default (prompt caching), OpenAI-compatible, local Ollama; record/replay for tests.
- UI: iced-based cockpit (details, including the vendored render-fix fork and its maintenance contract, are architecture-side — see the companion `architecture.md`).

## 12. Success Metrics

*All three were defined before the MVP shipped and are the delivered acceptance bar; the counter-metrics are as load-bearing as the primary metric — they keep maintenance from optimizing the wrong thing.*

**Primary**

- **SM-1 — The guided loop works end to end.** The operator can, from the cockpit, enter a coin + `200` + a lookback (e.g. `XRPUSDT`, `Last 90d`), get a ranked leaderboard with a single highlighted recommendation and a one-line rationale within one interaction, then start a forward paper-run of that selection and watch €200-scaled P/L move on real data — all reproducible from the recorded seed + window. Validates FR-1..FR-6, FR-14, FR-15, FR-21. *(Demonstrated on the golden `(BTCUSDT, €200, 2024 H1)` demo input.)*

**Secondary**

- **SM-2 — The honesty gate.** When buy-and-hold wins the bake-off — including the modal case where every active arm is FRAGILE — the recommendation says so plainly ("simply holding is the least-bad choice on this window"; `BenchmarkWins`), and the €200 paper-trades as a hold. Validates FR-6, FR-9, FR-10.
- **SM-3 — The no-regression gate.** The 119 anchored backtest body-SHAs stay byte-identical (the bake-off reads the engine; it must not perturb anchored scenarios), and the full lib/integration/UI-snapshot suite stays green. Validates FR-3, FR-20, FR-21.

**Counter-metrics (do not optimize)**

- **SM-C1 — Crown rate / `ActiveWins` frequency.** A rising rate of active crowns is NOT success and must never be optimized for: the modal honest outcome on real crypto is `BenchmarkWins`, and the no-alpha CI showed the primary gate alone crowns pure noise ~1 in 5 seeds (the report-only scorecard caught every chance-crown). More crowns without new evidence = the product failing, not winning. Counterbalances SM-1.
- **SM-C2 — Recommendation "interestingness" / narration engagement.** A more exciting recommendation is not a better one; the LLM never enters the ranking, and no surface may be tuned to make active picks more likely or more persuasive. Counterbalances SM-1, SM-2.

## 13. Open Questions

1. **EUR/USD rate source, v0.3 fork (from D4/FR-2):** the shipped rate is a configurable static default; a live-fetched rate layered on the static value as fallback is the noted v0.3 upgrade. Build or leave? (Operator fork, low stakes.)
2. **`lab-recipe-test-harness` v0.3.0+:** the one genuinely-open forward build item in the backlog (robustness gate cleared, awaiting an analyst spawn). Schedule or retire?
3. **Cross-run family-wise multiple-testing report-annex (online-FDR):** the single build-candidate in the post-remediation gap analysis (everything else is stated-limit/leave). Build as a report-annex, or record as a stated limit?
4. **Demo-runbook approval bookkeeping:** `CHANGELOG.md` still records the end-to-end demo as "awaiting operator approval" while the v3 close-out records it approved — reconcile the stale note at (or before) the Phase 5b cutover.

## 14. Assumptions Index

- §6 — D1–D5 treated as **ratified-as-built**: the literal "_Operator to confirm._" checkboxes in `spec/product.md` § Open decisions were never ticked, but the spec's `status: shipped` flip and the delivered F1–F9 implement exactly the recommended defaults.
- §8.2 — the 3-OS **CI matrix is taken as activated** (2026-07-10 operator-directed commits), superseding the older `CHANGELOG.md` "stays operator-parked" note.
