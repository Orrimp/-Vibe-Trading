---
slug: product
status: shipped
owner: analyst
updated: 2026-06-21
---

# Product Requirements — Single-Coin Investment Advisor (paper)

## What this product IS (2026-06-19 pivot — operator-redefined)

A **decision-support + paper-simulation tool for one retail investor with a
small budget.** The whole product answers a single, concrete question:

> "I have **€200**. I want to put it into **one** crypto (say **XRPUSD**).
> Which strategy should I use, and what should I do over the next few days?"

The user journey — the product, end to end:

1. **Pick** a coin (e.g. `XRPUSDT`) and a budget (e.g. €200).
2. **Bake off** *every* available strategy on that coin over a configurable
   lookback (**2 weeks → ~4 years**) — the rules engines (SMA / MACD / RSI /
   Bollinger), momentum, mean-reversion pairs, the regime dispatcher, the
   LLM-as-analyst overlay, and any ML overlay — each backtested on the same
   `(coin, window)`.
3. **Rank & select** the best strategy for that coin from the bake-off
   results (and, later, a *mix* of strategies or a strategy + LLM/ML ensemble),
   with a plain-language **"why this one."**
4. **Plan**: generate a budget-aware **buy/sell plan for the coming N days**
   (configurable horizon) — the current signal + entry/exit rules + projected
   €200 sizing.
5. **Watch**: in the **Live view**, the selected strategy (or mix/LLM)
   **paper-trades forward** on real incoming data and the user sees running
   profit/loss on their simulated €200.

This is a **re-framing of an existing, working engine, not a new build.** The
backtest engine, the strategy library, the LLM integration, the paper
simulator + agent runtime, real Binance/Yahoo market data, the double-entry
audit ledger, reflection memory, and the iced cockpit already exist and are
shipped (see [`../CHANGELOG.md`](../CHANGELOG.md)). The pivot wraps that
machinery in a guided, single-coin, single-budget journey. The roadmap below
is explicit about **reuse vs new** so the prior work reads as the foundation,
not waste.

## What this product IS NOT

- **Not a live broker.** PAPER / SIM ONLY — no live trading, no real orders,
  no exchange execution, no KYC, no withdrawals. The €200 is a **simulated**
  budget. (Standing operator constraint; live execution was removed from scope
  2026-06-12.)
- **Not financial advice.** It is a personal backtest-and-simulate sandbox.
  Every surface that shows a "recommendation" must carry a not-advice,
  past-performance-is-not-indicative disclaimer. The tool helps the user
  *reason*; it does not tell them to risk money.
- **Not a multi-asset portfolio manager.** The journey is deliberately
  **one coin, one budget** at a time. Multi-coin portfolios are out of scope.
- **Not an alpha claim.** The 2026-06 research program concluded that *no
  active strategy beat passive buy-and-hold net of cost* on the 2023-24
  large-cap sample under the frozen robustness rule (see § The robustness
  truth below). The product does **not** promise the selected strategy will
  win — it promises an honest, measured, reproducible bake-off and a forward
  paper-sim so the user can see for themselves.
- **Not HFT, not market-making, not regulated derivatives, not tax/lot
  accounting.** Unchanged non-goals.

## Why this is honest (the robustness truth, carried forward)

The prior identity of this repo was a *research stack* hunting for an active
edge. That hunt **concluded 2026-06-08: ship passive** — across the three
reachable channels (price/OHLCV, derivatives-positioning, on-chain) no active
strategy beat passive buy-and-hold net of cost under a pre-registered
block-bootstrap Monte-Carlo rule. That result is **not discarded by this
pivot — it is a feature of it.** Two consequences the product must honour:

1. **Passive buy-and-hold is always in the bake-off as the benchmark arm.**
   The "best strategy" is meaningful only relative to just-holding-the-coin.
   If buy-and-hold wins the bake-off for a given `(coin, window)`, the
   recommendation surface must say so plainly ("for XRPUSDT over this window,
   nothing beat simply holding").
2. **The robustness machine is the credibility layer, not decoration.** A
   strategy that wins on a single backtest path but is FRAGILE under
   resampling (p5 Sharpe < 0) should be flagged, not silently crowned. The
   existing Monte-Carlo robustness harness (`monte-carlo-bootstrap-path-generator`
   + `strategy-robustness-harness`) is the differentiator that makes "we ranked
   them" trustworthy rather than a lucky-draw leaderboard.

The product's value proposition is therefore: **"a transparent, reproducible,
risk-aware bake-off + paper-sim for putting a small fixed budget into one
coin"** — measured honesty, not asserted alpha.

## Goals

- Let a non-expert user go from **"I have €X for coin Y"** to **"here is the
  best-ranked strategy, here's why, and here's what it would do with my
  budget"** in one guided flow.
- **Reuse the shipped engine** for every heavy-lifting step (backtest, stats,
  paper-sim, ledger, cockpit) — net-new code is the *orchestration* (bake-off
  loop), the *ranking/recommendation* surface, the *forward-plan* generator,
  the *budget-aware sizing*, and the *guided input UX*.
- Keep every recommendation **auditable and reproducible**: the bake-off seed,
  the lookback window, the per-strategy KPIs, and the chosen strategy are all
  recorded so the same inputs reproduce the same ranking.
- Stay **paper-only and safe**: no real orders, hard simulated-budget cap,
  not-advice disclaimers on every recommendation surface.

## The engine being reused (inventory — prior work, not waste)

Everything below is **shipped** (per [`../CHANGELOG.md`](../CHANGELOG.md)) and
becomes the engine for the new journey:

| Capability | Crate / screen | Role in the new journey |
|---|---|---|
| Backtest engine (matching, fills, friction) | `crates/backtest` (`MatchingEngine`, `run_scenario`) | Runs each strategy on `(coin, window)` |
| Risk-metric stats | `crates/backtest/src/stats` (`compute_sharpe_*`, `compute_sortino_*`, `compute_calmar`) + `BacktestKpis` (`total_return_pct`, `max_drawdown`, `trade_count`) | The ranking inputs — no new math needed |
| Strategy library | `crates/strategy` (sma, composed MACD/RSI/Bollinger, cross-sectional momentum, mean-reversion pairs, regime dispatcher, LLM/ML overlays) | The bake-off field |
| Passive baseline | shipped buy-and-hold control ([`runbooks/passive-baseline.md`](runbooks/passive-baseline.md)) | The benchmark arm + safe default |
| Robustness harness | `monte-carlo-bootstrap-path-generator` + `strategy-robustness-harness` | The credibility / fragility flag on the ranking |
| Single-run backtest UX | cockpit **Lab** screen (`crates/ui/src/lab`, `runner::spawn_lab_run`) | The bake-off is essentially *looping Lab over the registry* |
| Forward paper-sim | `crates/agent` runtime (paper mode) + EventBus | Drives the forward paper-trade |
| Live P/L view | cockpit **Live** screen (`crates/ui/src/live.rs`) | Watch the selection paper-trade the €200 |
| LLM integration | `crates/llm` (Anthropic / OpenAI-compat / Ollama, recording/replay) | The LLM-as-analyst overlay + the "why this one" narration |
| Reflection memory | `crates/reflection` (LessonCards, retrieval) | Optional: surface relevant past lessons on the recommendation |
| Audit ledger | `crates/audit` (double-entry, body-SHA anchors) | Makes every paper fill + recommendation reproducible |
| Reports + viewer | `crates/reports`, cockpit Reports screen | Renders the bake-off / forward-run evidence |

The net-new surfaces are named in § Roadmap (the bake-off orchestrator, the
ranking/recommendation surface, the forward-plan generator, budget-aware
sizing, and the guided "new investment" input).

## Key product decisions (operator must confirm — recommended defaults below)

These are the decisions the operator should ratify before the architect locks
the MVP. Each carries a recommended default and the reasoning.

### D1 — "Best strategy" ranking metric **(Recommended: risk-adjusted, with a robustness gate)**

- **(a) Risk-adjusted with a robustness gate (Recommended).** Primary sort by
  **Sharpe** over the lookback (already computed by `compute_sharpe_hourly`),
  with a hard **fragility gate**: any strategy the Monte-Carlo harness flags
  FRAGILE (p5 Sharpe < 0) is shown but cannot be crowned #1 unless *everything*
  is fragile (in which case the surface says "nothing here is robust — consider
  just holding"). Tie-break by total return, then by lower max-drawdown.
  Durable: it is the metric the whole codebase's robustness thesis is built on,
  it won't need re-deriving when ensembles arrive, and it protects a naive user
  from a lucky-path leaderboard.
- **(b) Total return only.** Cheapest to explain ("which made the most money"),
  but rewards a single lucky path and a 73%-drawdown ride equally with a smooth
  one — actively *misleads* a small-budget user. Fallback only if the operator
  wants the simplest possible v1 headline.
- **(c) A blend score** (e.g. weighted Sharpe + return − drawdown penalty).
  More tunable but introduces a magic-weights decision the operator would have
  to defend; defer until there's evidence (a) is insufficient.

Recommendation: **(a)** — show **Sharpe, total return, and max-drawdown side
by side** on the leaderboard (all already available), sort by Sharpe, gate on
robustness. *If-budget-tightens:* ship **(b)** for the MVP headline and add the
robustness gate in the immediate follow-on — but flag that this ships a
known-misleading default and spawns a v0.2 correctness fix.

### D2 — What the forward "plan" concretely IS **(Recommended: live rules + a forward paper-run, NOT pre-computed orders)**

You **cannot** pre-compute future orders for a price-dependent strategy — an
SMA cross or an RSI exit depends on prices that haven't happened. So the "plan"
must be defined as *what we can honestly produce today*:

- **(a) "Current stance + rules + a forward paper-run" (Recommended).** The
  plan surface shows: (i) the strategy's **signal right now** (BUY / HOLD /
  SELL on the latest bar), (ii) the **entry/exit rules in plain language**
  ("buys when SMA-20 crosses above SMA-50; exits on the reverse cross or a
  −8% stop"), (iii) the **budget-aware sizing** that would result (how much of
  €200 deploys on the next BUY), and (iv) a **forward paper-run** (extend the
  existing paper-sim / Live view) that *executes those rules on real incoming
  bars over the next N days* and shows running P/L. This is honest (no
  fabricated future fills), reuses the agent runtime + Live view, and is what
  the operator described in journey step 6.
- **(b) A deterministic order schedule.** Only correct for the rare
  price-independent strategy (e.g. a fixed DCA). Misleading for everything
  else; reject as the general "plan."

Recommendation: **(a)**. The "plan for the coming days" = today's stance +
legible rules + projected sizing, then **the Live view IS the plan unfolding**.

### D3 — Ensemble / LLM-mix scope **(Recommended: LATER, not MVP)**

- **(a) Later (Recommended).** The MVP ranks and runs **one** strategy. A
  *mix* (capital split across top-K) and an *LLM/ML ensemble* (strategy signal
  + LLM-as-analyst confirmation) are a clear **v0.2** enhancement once the
  single-strategy loop is proven end-to-end. The regime dispatcher
  (`crates/strategy/src/regime_dispatcher.rs`) and the v2 LLM overlay already
  exist as the building blocks, so this is genuinely additive, not blocked.
- **(b) MVP.** Triples the surface area (mix sizing, ensemble arbitration, LLM
  cost in the hot loop) before the basic loop is trustworthy. Reject for MVP.

Recommendation: **(a)** — single strategy for MVP; mix + LLM-ensemble in v0.2.

### D4 — EUR budget on a USD/USDT-quoted pair (e.g. XRPUSDT) **(Recommended: treat €200 as 200 quote-units for the MVP, with a labelled FX note)**

The engine is **USDT-denominated end to end** — `BacktestKpis.final_equity`
and all sizing are `Money<Usdt>`; **no `Eur` currency type exists** in
`crates/core` (only `Usdt`/`Btc`/`Eth`). Three honest options:

- **(a) MVP: treat the budget as quote-currency units (Recommended).** The
  user enters "200"; the engine sizes against **200 USDT** on a `*USDT` pair.
  The UI **labels it honestly** — "Budget: €200 ≈ 200 USDT (1:1 assumed; EUR/USD
  ≈ parity, FX not modelled)" — and a tooltip explains the simplification.
  Zero engine change, correct to within the EUR/USD rate (~1.05-1.10), and the
  *relative* strategy ranking is FX-invariant anyway (a constant scalar on the
  budget doesn't change which strategy wins). Durable enough for a paper tool.
- **(b) Apply a fixed EUR→USD rate at entry.** One config constant
  (`eur_usd_rate`) converts €200 → ~$210 of quote currency before sizing.
  Slightly more accurate headline P/L; still no live FX feed. A reasonable
  v0.2 refinement.
- **(c) First-class `Eur` currency + live FX feed.** Correct but heavy — a new
  `Currency` impl, an FX data source, and FX-PnL plumbing through the ledger.
  Out of scope for a paper decision-support tool; reject unless the operator
  wants displayed P/L in real EUR.

Recommendation: **(a)** for MVP (label it clearly), **(b)** as the easy v0.2
upgrade. *If-budget-tightens:* (a) is already the cheapest — no fallback needed.

### D5 — Confirm paper-only **(Recommended: yes — re-affirm explicitly)**

The €200 is simulated; the product never places a real order. This restates
the standing constraint (live exec removed 2026-06-12) for the new framing.
Recommendation: **confirm paper-only** and put the not-advice + simulated-budget
disclaimer on the recommendation and Live surfaces. No reason to revisit.

## MVP definition (the smallest end-to-end loop)

The **MVP** is the smallest slice that delivers *"pick a coin → see the
best-ranked strategy (with why) → watch it paper-trade your €200."* Concretely:

1. A guided **input** (coin + budget + lookback) — even a minimal form.
2. The **bake-off orchestrator** runs the existing strategies (+ buy-and-hold
   baseline) on `(coin, lookback)` by looping the existing backtest runner.
3. A **ranked leaderboard + single recommendation** ("best for this coin,
   over this window, because…") using existing KPIs + the robustness flag.
4. **Forward paper-trade** of the selected strategy with **budget-aware €200
   sizing**, shown as running P/L in the **existing Live view**.

Everything else (forward-plan detail surface, mixes, LLM-ensemble, guided-UX
polish, EUR-FX refinement) is post-MVP. The roadmap orders them.

## Success metrics

- **MVP success:** the operator can, from the cockpit, enter `XRPUSDT` + `200`
  + `Last 90d`, get a ranked leaderboard with a single highlighted
  recommendation and a one-line rationale within one interaction, then start a
  forward paper-run of that selection and watch €200-scaled P/L move on real
  data — all reproducible from the recorded seed + window.
- **Honesty gate:** when buy-and-hold wins the bake-off, the recommendation
  says so; when everything is FRAGILE, the surface says "nothing here is robust."
- **No-regression gate:** the existing 119/119 anchored backtest body-SHAs stay
  byte-identical (the bake-off *reads* the engine; it must not perturb anchored
  scenarios), and the full lib/integration/UI-snapshot suite stays green.

## Constraints (carried forward, still true)

- Language: **Rust** stable, edition 2024. Single-operator; no auth/RBAC.
- **Paper / sim only.** No real-money execution, KYC, exchange keys,
  withdrawals, or live orders (out of scope; a hypothetical follow-up project).
- **Money math uses `Decimal`, never `f64`** (every monetary value is
  `Money<C>`); the engine is **USDT-denominated** today (see D4).
- **Audit imports nothing from sibling crates**; the ledger reconciler
  invariant (Σ debits == Σ credits) stays provable in isolation.
- **The `ui` crate never depends on `strategy` / `exec` / `models` / `llm`** —
  bootstrap of those types happens in `agent`. The bake-off orchestrator must
  respect this layering (it lives in `agent`/`backtest`, not `ui`).
- **Every strategy overlay or sizing-modifier ships with a
  baseline-equity-divergence e2e test from day 1** (the budget-aware sizing
  modifier in the MVP is exactly this kind of surface — see the CLAUDE.md
  non-negotiable; precedent `v3-volatility-forecaster-noop-fix`).
- **Anchored report files in `spec/*/reports/` are byte-immutable** (ADR-0038
  § D6).
- LLMs via API with prompt caching + a strict monthly cost budget (tracked in
  [`architecture.md`](architecture.md)); local Ollama for cost-free dev.
- Lean on existing Rust crates rather than reinventing quant primitives.

## Stakeholders

- **Vitaliy** — product owner, operator, and the single retail-investor user
  the product is designed for.
- **Claude agents** — analyst, architect, developer, ui-designer, tester,
  presenter (dev-time workflow per [AGENT.md](../AGENT.md)).

---

## Engine reference (mechanics retained for the architect)

The sections below document engine mechanics that the new journey reuses. They
are **descriptive of shipped capability**, not new scope. The pivot does not
change them; it composes them.

### Strategy library (the bake-off field)

The `crates/strategy` registry holds named strategies sharing
data/feature/risk/exec scaffolding. Shipped and runnable in the bake-off:
SMA crossover, composed multi-indicator (MACD + RSI + Bollinger),
cross-sectional momentum, mean-reversion pairs, the regime dispatcher, and the
v2 LLM-as-analyst overlay. The retired forecaster chains (TCN / PatchTST /
GARCH-σ / LLM-forecaster) remain in the tree behind feature flags; the bake-off
includes a strategy only when it runs cleanly on a single `(coin, window)` —
the retired ML overlays are **opt-in**, not default arms, given their concluded
negative verdicts.

> **Active-edge-search status (2026-06-08, retained).** Across the three
> reachable channels (price/OHLCV, derivatives-positioning, on-chain) no active
> strategy beat passive buy-and-hold net of cost under the frozen
> block-bootstrap Monte-Carlo rule (passive: +1.74 Sharpe 2023 / +1.10 2024).
> This is a **bounded** result on the 2023-24 large-cap sample, not a claim
> active trading is impossible — untested channels (options/implied-vol, macro,
> social) remain by lower prior or infeasibility. **For the new product this is
> load-bearing:** buy-and-hold is the bake-off's benchmark arm and the safe
> default, and the bake-off must be honest when nothing beats it. Full statement,
> scope, and the methodological spine (frozen pre-registered rule, block-bootstrap
> MC, 119/119 byte-SHA anchors, day-1 falsifiers, anti-cherry-pick renderer,
> live-bar calibration) are preserved in git history and in
> [`runbooks/passive-baseline.md`](runbooks/passive-baseline.md).

### Robustness machine (the credibility layer)

The Monte-Carlo robustness layer resamples *real* returns (stationary block
bootstrap, Politis–White auto block length) into an ensemble of plausible paths
and measures the **distribution** of a strategy's outcome (Sharpe p5/p50/p95,
max-drawdown tail, probability of loss) against a pre-registered rule (p5
Sharpe < 0 → FRAGILE). This is **uncertainty quantification, not prediction**.
In the new product it powers the **fragility flag** on the bake-off leaderboard
(D1): a strategy that wins on one path but is fragile under resampling is shown
but not crowned. Shipped as `monte-carlo-bootstrap-path-generator` +
`strategy-robustness-harness`.

### LLM role (support — narration + the "why this one")

The LLM is a **support layer**, never the alpha source (empirically: three
retired alpha-by-prediction bets — TCN/PatchTST, GARCH-σ, LLM-forecaster). Its
sanctioned roles map directly onto the new journey:

- **"Why this one" narration** — turn the winning strategy's KPIs + robustness
  distribution into a plain-language rationale on the recommendation surface.
- **LLM-as-analyst overlay** — an optional ensemble arm (D3, v0.2) that
  confirms/qualifies a strategy signal.
- **Lesson summarization / tie-break** — distill reflection LessonCards;
  break a statistical tie between two indistinguishable strategies with a
  narrated, auditable rationale. **Never** the primary ranking gate.

Runtime mechanics (retained): dual-tier (`deep_think` / `quick_think`),
provider abstraction (Anthropic default with prompt caching; OpenAI-compatible;
local Ollama), hard monthly token budget with 80%/100% auto-degrade, tool-use
schemas over free-text parsing.

### Data sources

Real market data, retained: spot OHLCV from **Binance** (pinned hourly 2023-24
corpus `3a8b96c4` + a 2021-22 bear corpus `4f390622`) and multi-asset **Yahoo**
data for the Lab. Funding/open-interest and on-chain feeds exist from the
research program but are not required for the single-coin journey. Historical
bulk via venue dumps; the lookback window (2 weeks → ~4 years) selects the slice.

### Risk management (hard requirements, retained)

- Risk limits enforced as Rust types — illegal orders fail at construction.
- The **budget cap is itself a hard limit** in the new product: paper sizing
  may never deploy more than the user's simulated budget.
- Kill switch (`.halt` file / missed heartbeat → flatten + stop), per-symbol
  exposure cap, max-drawdown trigger, full audit log.

### Operating modes (retained, paper is terminal)

1. **Research** — backtest only, deterministic seeds, cached LLM replay. The
   bake-off runs here.
2. **Paper** — live data feed, full pipeline, simulated fills, real LLM cost.
   The forward paper-trade runs here.
3. ~~Live~~ — **removed from scope 2026-06-12.** Not wired, not planned.

### Cockpit information architecture (retained, lightly re-centred)

The cockpit is the operator's one-screen view; the shipped sidebar shell
(Lumen design system) has Home / Charts / Strategies / Risk / Audit / Debug /
Lab / Live / Compare / Memory / Models / Trail / Reports screens. The pivot
**re-centres** the journey on **Lab → (new) Bake-off/Recommendation → Live**;
it does not redesign the shell. New surfaces (guided input, leaderboard +
recommendation, forward-plan detail) attach to this existing IA rather than
replacing it. Order entry / config editing / multi-account remain out of the
cockpit IA (paper tool, single operator, config-driven universe).

### Strategy lifecycle — promotion gates (retained)

A strategy lives in one stage at a time; promotion is explicit and
criteria-driven (`research` → `paper` requires a single-path OOS Sharpe > 1.0
**and** a robustness distribution read against the pre-registered rule). In the
new product the **bake-off + recommendation is a user-facing instance of the
`research` gate**: it surfaces the same Sharpe + robustness read the lifecycle
gate uses, packaged for a non-expert.

### Cost economics (retained)

Monthly opex ladder ($45 / $135 / $360) with the 80%/100% LLM auto-degrade
rule. The single-coin journey is cheap in research mode (no LLM); the LLM cost
appears only in the LLM-as-analyst overlay (D3, opt-in) and the "why this one"
narration (one cheap call per recommendation, cacheable).

### Operator success reports (retained)

Auto-generated "is this working?" reports (equity, Sharpe/Sortino/Calmar/
drawdown, attribution, system health) under
`spec/operator-success-reports/reports/`. In the new product the **bake-off
result is itself a report** — a ranked, dated, reproducible artifact the user
can re-open.

---

## Open decisions

Tracked here until the operator answers; then they migrate into the body.

- [ ] **D1 — ranking metric (2026-06-19):** recommend risk-adjusted (Sharpe)
  with a robustness fragility gate, showing Sharpe + return + max-drawdown
  side by side. _Operator to confirm._
- [ ] **D2 — forward "plan" shape (2026-06-19):** recommend "current stance +
  plain-language rules + budget-aware projected sizing + a forward paper-run"
  (the Live view IS the plan unfolding); reject pre-computed future orders for
  price-dependent strategies. _Operator to confirm._
- [ ] **D3 — ensemble / LLM-mix scope (2026-06-19):** recommend LATER (v0.2),
  not MVP — single strategy first. _Operator to confirm._
- [ ] **D4 — EUR-on-USD handling (2026-06-19):** recommend MVP treats €200 as
  200 quote-units with an honest "≈ 200 USDT, FX not modelled" label; fixed
  EUR→USD rate as the v0.2 refinement; first-class `Eur` currency rejected for
  a paper tool. _Operator to confirm._
- [ ] **D5 — confirm paper-only (2026-06-19):** recommend re-affirming
  paper/sim-only with not-advice + simulated-budget disclaimers on every
  recommendation/Live surface. _Operator to confirm._

### Resolved (carried from prior identity, still binding)

- [x] **Live trading horizon (2026-04-19, reaffirmed 2026-06-12):** paper on
  real data is terminal; no real-money execution, KYC, or withdrawals.
- [x] **Single operator (2026-04-19):** no auth, no RBAC.
- [x] **Tax / reporting (2026-04-19):** no tax/lot accounting; operator reports
  focus on "is this working?" visibility.
- [x] **DR / backups (2026-04-19):** local snapshots only; zero cloud spend.
- [x] **Differentiator / moat (2026-04-17):** persistent reflection memory +
  auditable double-entry ledger; robustness ("measured, not asserted alpha")
  is the epistemic core. All three are retained as the engine's trust layer.

---

## Changelog

- 2026-06-21 (analyst, spec-honesty close-out): flipped `status: draft →
  shipped` — the operator personally defined this product and the MVP is built,
  tested, and committed (advisor F1–F5 + dynamic-data: `58b55b1`, `e0cc34b`,
  `acc3789`, `d4f4dce`, `c9dd275`, `ee5a904`). The advisor loop (pick coin +
  budget → bake-off → ranked pick → forward paper-trade) is the terminal
  deliverable; buy-and-hold benchmark + Monte-Carlo robustness are the
  credibility layer underneath. Reconciled `spec/trace.toml` advisor rows and
  the root `CHANGELOG.md` advisor section in the same sweep.
- 2026-06-19 (analyst, PRODUCT PIVOT — single-coin investment advisor):
  **Rewrote the product identity** from a research stack ("does an active edge
  survive resampled histories?") to a **decision-support + paper-simulation
  tool for one retail investor with a small fixed budget** (operator
  redefinition). New top sections: § What this product IS (the pick-coin +
  budget → bake-off-all-strategies → rank/select-best → forward buy/sell plan →
  watch-paper-trade-P/L journey), § What this product IS NOT (paper-only,
  not-advice, single-coin, not-an-alpha-claim), § Why this is honest (the
  ship-passive robustness verdict is now a *feature* — buy-and-hold is the
  bake-off benchmark arm and the robustness machine is the credibility layer),
  § The engine being reused (an explicit reuse table proving the prior shipped
  work — backtest engine, strategy library, stats/KPIs, LLM, paper-sim, Live
  view, ledger, reflection, cockpit — is the foundation, not waste), § Key
  product decisions D1-D5 (ranking metric; forward-plan shape; ensemble scope;
  EUR-on-USD handling; confirm paper-only — each with a recommended durable
  default and reasoning), § MVP definition (smallest pick-coin → best-strategy
  → watch-€200-paper-trade loop), and § Success metrics. **Preserved as the
  engine reference** (descriptive, not new scope): strategy library + the
  retained 2026-06-08 active-edge-search status, the Monte-Carlo robustness
  machine, the LLM-as-support roles (now mapped to "why this one" narration +
  optional ensemble), data sources, risk requirements (with the budget cap as
  a new hard limit), operating modes (paper terminal; live removed), the
  cockpit IA (re-centred on Lab → bake-off → Live, not redesigned), lifecycle
  gates, cost economics, and operator success reports. Confirmed key
  engine-shape facts against code: the engine is USDT-denominated
  (`BacktestKpis.final_equity: Money<Usdt>`; no `Eur` currency impl exists —
  pins D4); ranking inputs already exist (`compute_sharpe/sortino/calmar` +
  `BacktestKpis.total_return_pct`/`max_drawdown`); the bake-off is a loop over
  the existing Lab runner (`runner::spawn_lab_run`); the forward paper-trade
  extends the existing `agent` paper runtime + cockpit Live view. No anchored
  content touched; no engine code changed. The prior research-program changelog
  (2026-04-17 → 2026-06-08 entries) is preserved verbatim below.
- 2026-06-08 (analyst, doc-hygiene): § Week 2 — added a one-line
  cross-reference noting the `bps: 2` paper-fill slippage is the original
  2026-04 ship history, and that the live canonical v5 friction profile is
  `slippage_bps: 8` per ADR-0045 § D1 / ADR-0043 § D3 (audit-2026-06-08 SC-C).
  Surgical reconciliation only; the terminal-verdict body below is untouched.
- 2026-06-08 (analyst, terminal verdict — active-vs-passive search CONCLUDED):
  finalized the § Strategy library status note from "passive-may-be-terminal /
  on-chain-as-next-probe" to the **TERMINAL verdict: SHIP PASSIVE**. The
  pre-committed on-chain hard-stop fired
  ([`onchain-netflow-spike-2026-06-08.md`](dev-notes/onchain-netflow-spike-2026-06-08.md))
  — exchange net-flows are PIT-infeasible (CryptoQuant disclaims point-in-time
  accuracy; no free immutable past-only series), and the cleaner-PIT
  stablecoin-supply fallback is FRAGILE (sign flips year-over-year under the same
  live-bar that certified the basis signal). The active-vs-passive search is now
  **closed across THREE reachable channels** (price/OHLCV + derivatives-positioning
  + on-chain); no active strategy beats passive buy-and-hold (+1.74/2023,
  +1.10/2024) net of cost under the frozen § 0 rule. **Stated the verdict's SCOPE
  honestly** (it licenses "active ≤ passive in the reachable universe, net of cost,
  on the 2023-24 large-cap perp sample" — NOT "active trading is impossible";
  named the untested lower-prior/infeasible channels: options/DVOL, macro, social;
  noted the +1.74 BH bar is partly a structural bull-leg artifact). **Stated why
  the verdict is CREDIBLE** (the methodological spine: frozen pre-registered § 0
  rule, block-bootstrap MC, 119/119 byte-SHA anchors, day-1 falsifiers, the
  anti-cherry-pick FAMILY-UNIFORM renderer, and the live-bar calibration that
  certified the one real signal — the basis reversal — and correctly killed the
  rest). **Defined "ship passive" CONCRETELY for this research/backtest codebase**:
  mark the already-built+anchored BH control as the canonical production baseline
  in the spec + a short `spec/runbooks/passive-baseline.md` runbook (baseline =
  BH on the configured universe; documented rebalance cadence, monthly default
  proposed; paper-mode run recipe; BH anchor scenarios) — explicitly NO new
  strategy crate / ScoreSource / sweep arm / anchor. Added a top-of-document
  Terminal-verdict banner after § Vision so a reviewer reading only product.md
  comes away with the correct bounded conclusion. No reversal of the thesis — the
  terminal close of the ratified "measured robustness, not asserted alpha" core
  (§ Differentiator (5)); locked (2)+(4) moat, the LLM-as-support reframe, and all
  anchored content untouched.
- 2026-06-08 (analyst, on-chain-vs-conclude fork): sharpened the § Strategy library
  roadmap with an **active-edge-search status** note recording that TWO data domains
  are now exhausted with uniform FRAGILE verdicts under the frozen Monte-Carlo § 0
  rule (OHLCV/price: 4 families × 3 horizons × universe; derivatives-positioning:
  funding-carry + basis long-only + MN basis/funding/residual — with basis ≡ funding
  byte-identical and the basis⊥funding residual at NEGATIVE median Sharpe), passive
  buy-and-hold undefeated. Scoped the claim honestly: it licenses "no harvestable
  edge in price/positioning data on these large-caps net of cost," NOT "no edge
  anywhere" (~1.5 distinct information channels ruled out, not the reachable
  universe). Recorded **on-chain as the pre-registered next-and-final domain probe**
  and that the **realistic terminal strategy may be passive BH — a success of the
  robustness program, not a failure** (the machine correctly killed ~10 active bets;
  "ship passive" = promote the already-built+anchored BH control to production).
  Cross-linked the fork decision-support note
  ([`onchain-vs-conclude-fork-2026-06-08.md`](dev-notes/onchain-vs-conclude-fork-2026-06-08.md)).
  No reversal of the thesis — a sharpening consistent with the ratified § Pillar
  stack (measured robustness, not asserted alpha) and the existing
  demotion-of-prediction-bets record. Locked (2)+(4) moat, the LLM-as-support
  reframe, and all anchored content untouched.
- 2026-05-30 (analyst, monte-carlo-robustness-lane — narrative coherence pass,
  closes spec-audit-2026-05-30 COSMETIC #6): tightened the document's prose to
  the already-ratified core/support reframe — **no new strategic decision, prose
  coherence only.** Reframed § Vision and § Goals (robustness = uncertainty
  quantification not prediction; LLM = support layer), added Differentiator
  item (5) measured robustness beside the (2)+(4) moat, added a scope banner to
  § Trading-time agent roster, and tightened the v1 success metric + `paper`
  lifecycle gate to be distribution-valued. Cross-linked the new
  [`robustness-decision-rule-2026-05-30.md`](dev-notes/robustness-decision-rule-2026-05-30.md).
- 2026-05-30 (analyst, monte-carlo-robustness-lane M0): ratified operator Q4 —
  added top-level § Pillar stack — core vs support (CORE = quantitative
  strategy + Monte-Carlo robustness + future deterministic learning loop +
  risk/ledger moat; SUPPORT = the LLM pillar, explicitly NOT the alpha source).
  Empirical basis tabled (three retired/deferred alpha-by-prediction bets).
- 2026-05-04 (analyst, post-Phase-1 ship): added § Cockpit information
  architecture capturing the terminal product IA (left-sidebar shell, screens,
  separate viewer Backtest surface, reserved right-rail Assistant for v2 LLM).
- 2026-04-19 (operator + analyst): v0 delivered + verified PASS; final Open
  decisions resolved — live/KYC out of scope, single-operator forever, no
  tax/reporting, local-only DR with zero cloud spend.
- 2026-04-17 (analyst + operator + developer): initial scaffold; trading-time
  agent roster, data/indicator suite, dual-tier LLM strategy, memory loop, risk
  requirements, operating modes, config surface, success metrics (adapted from
  [TradingAgents](https://github.com/TauricResearch/TradingAgents)); RustQuant
  + double-entry ledger constraints; brainstorm pass (Differentiator, strategy
  roadmap, universe ladder, lifecycle gates, cost ladder, v0 Candidate C);
  three `[DECIDE]` markers resolved (moat, v0 scope, cost ladder); stale
  `cala-ledger` references updated to `sqlx-ledger` then to raw `sqlx` per
  ADR-0024.
