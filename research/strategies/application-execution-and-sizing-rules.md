# Application — Execution, Cost Realism & Rule-Family Sizing

> Decision-oriented brief for analyst + architect. Derived from
> `research/strategies/knowledge.md` and the ledger
> `research/strategies/papers.md` (cited `strategies[N]`). No new papers.
>
> **Our app:** a Rust **single-coin crypto investment advisor** — paper/sim only,
> NOT advice, NOT live. Pick coin + budget → bake off every strategy → rank under
> a FROZEN 1000-path moving-block-bootstrap gate (FRAGILE ⇒ can't crown;
> buy-and-hold always the benchmark + exempt) → forward rule-based plan → watch it
> paper-trade. Thesis: **no active strategy robustly beats buy-and-hold net of
> costs.** Sells **measured honesty** — "a framework for trading with traceable
> and plausible trading."

This file covers two linked things the strategy literature settles for us:
**(a) how to model execution cost honestly at retail size** (the answer is small
and specific — fees + spread + a delay term, *not* scheduling or impact), and
**(b) how the rule families — trend / momentum / mean-reversion / breakout —
inform the forward-plan, sizing overlays, and the cost-aware "trade-less"
angle.** Both converge on one product lever: **costs are the headline decision
variable, and trading less is usually the honest improvement.**

---

## 1. Summary of the research

**Execution algorithms are one continuum, and all of it is background at €200.**

- **Perold's implementation shortfall** `strategies[87]` is the cost-accounting
  frame: charge fees + spread + delay + opportunity cost against an instant,
  costless "paper" fill — and *our sim IS that paper portfolio*. This is the
  correct mental model for our cost engine.
- **VWAP/TWAP/IS differ only by a risk-aversion knob** `strategies[90][3]`. VWAP
  is the risk-neutral optimum (trade with the volume curve); a risk-averse trader
  front-loads (implementation-shortfall schedule); Almgren–Chriss frames it as
  `min E[cost] + λ·Var[cost]` with permanent + temporary impact.
- **Crypto volume is much harder to predict than equities**, so forecast-first
  execution is brittle `strategies[91]`; learn the slippage objective directly if
  at all.
- **Impact is negligible at retail size** `strategies[20]` (the square-root law
  `impact ∝ σ·√(Q/V)` is robust but vanishes at €200). The crypto microstructure
  refinements — book *resilience* / recovery speed `strategies[71]`, the
  state-dependent spread that **widens exactly during the volatility spike you are
  trying to fade** (1.6% gross → 0.44% net `strategies[27]`), lower weekend
  liquidity → wider weekend spreads `strategies[55]` — matter only as **cost-model
  inputs**, not as schedules.
- **Maker vs taker fees are a first-class, large cost lever** `strategies[65]`:
  posting limit (maker) orders dodges taker fees, at the cost of fill/queue risk.
- **Bayesian TCA** `strategies[57]`: rank execution by implementation shortfall
  treated as a **noisy latent draw**, not a point estimate — the same
  distributional discipline our return-bootstrap already uses.

**Net:** the only execution cost that matters for us is **spread crossed + fees +
a delay/slippage term, charged against the arrival (decision-bar) price, treated
as a state-dependent random draw.** Everything about scheduling and impact is
out of scope at our size.

**Costs are what flip strategies from winning to losing.**

- **Single-coin BTC technical rules beat buy-hold GROSS and LOSE net of 2–4%
  round-trip fees** `strategies[11]` — the most directly applicable negative
  result we have, and the reason a **fee-sensitivity sweep** must be first-class.
- **Costs destroy 55–90% of even peer-reviewed mean-reversion edges, and the
  fanciest method is the most cost-exposed** `strategies[72]`:
  distance/cointegration/copula pairs go 91/85/43 bps gross → **38/33/5 bps net**
  (copula gutted to 5 bps). Simpler ≈ better net.
- **Even the growth-optimal (Kelly) strategy goes bankrupt if it rebalances too
  often under costs** `strategies[51]` — the formal backbone of "penalize
  turnover" and of why low-turnover/hold is the honest single-coin default.
- **Riskless crypto arbitrage is gone net of costs/latency** `strategies[64]`
  (triangular: 0.1–0.5%/cycle, sub-second, needs 15–50 ms infra). If even a
  *riskless* edge cannot beat fees, a statistical bar-level directional edge
  beating buy-and-hold certainly cannot.

**How the rule families inform the forward-plan and sizing — sizing > signal.**

- **Time-series momentum** `strategies[1]`: signal = sign of trailing-N return;
  position sized **inversely to realized vol**. The vol-scaling, not the entry
  rule, is the load-bearing ingredient — but expect it to *underperform* net of
  costs on one coin.
- **Mean-reversion = OU s-score on a detrended residual** `strategies[62][39]`:
  enter ≈ ±1.25σ, exit ≈ 0; **require OU-stationarity first** `strategies[33]`;
  count a **state-dependent spread on every entry/exit** `strategies[27]`.
- **Overlay sign is regime-dependent — stops help trends, hurt ranges**
  `strategies[38]`: a stop-loss adds value ONLY under positive serial correlation
  and *destroys* return under mean-reversion. Never apply an exit overlay
  family-blind.
- **Vol-targeting is in-sample-fragile and crypto-inverted** `strategies[21][22]
  [58]`: in-sample Sharpe gains largely vanish OOS (unstable fitted params), and
  crypto's **inverse leverage effect** (vol rises in *rallies*) can make an
  equity-calibrated overlay de-risk during up-moves. Ship it (if at all) as a
  drawdown/variance tool with fixed pre-registered params, never as alpha.
- **Match each family to its theoretically-correct horizon band**
  `strategies[49]`: reversion at very-short and very-long scales, trend in the
  middle (days–weeks). Do NOT sweep every family over every lookback — that
  manufactures multiple-testing and fits families in regimes where they shouldn't
  work.
- **The one robust win across the honest studies is a blend.** The crypto
  walk-forward EMA study `strategies[68]` (in-sample beats B&H, OOS ≈ B&H, worse
  after 0.1% costs, beats random params only 8–13.7%) found the durable result was
  a **~50% drawdown cut from blending the active sleeve with a buy-and-hold core**
  — echoed by dual-momentum `strategies[47]`.
- **Report compound/geometric return; value vol-reduction via the ½σ² drag**
  `strategies[35]`. A single buy-and-hold coin earns zero diversification return,
  so it is the honest single-asset ceiling.

---

## 2. Possible solutions / what can be done with this research

1. **Audit and harden the cost model to "implementation shortfall vs arrival
   price."** Confirm every simulated fill is debited spread + fees + a delay/
   slippage term against the decision-bar price (our `sim_slippage_cost` already
   charges fee + slippage; the question is whether the *delay* term and a
   *state-dependent* spread are modeled).
2. **Make a fee-sensitivity sweep + turnover penalty a first-class ranking
   output.** Report, per arm, the round-trip cost at which it crosses from
   beating to losing vs buy-hold (replicate `strategies[11]`'s sweep).
3. **Make spread state-dependent** (wider on volatile/extreme bars and weekends)
   so reversal/dip-buying rules cannot win on an unrealistically flat spread
   `strategies[27][55][71]`.
4. **Offer a maker/taker-aware cost-model option** `strategies[65]` — model the
   fill/queue risk; default to taker-style crossing for conservatism.
5. **Add a cost-aware "trade-less" execution filter** to the forward-plan: act
   only when `|expected_move| > λ·c·|Δpos|` (a meta-labeling cousin). The honest
   expectation is reduced cost-drag, not new return.
6. **Gate every exit/stop overlay by a trend filter** `strategies[38]` and ship
   it with a day-1 baseline-equity-divergence e2e (the v3-vol-overlay-noop
   precedent).
7. **Match the bake-off's horizon sweep to each family's correct band**
   `strategies[49]` instead of a dense every-family-every-lookback grid (which
   also inflates the multiple-testing count the other file's gate must deflate).

---

## 3. Relevance for the project

- **Confirms the cost engine's scope is correct and small.** We are right to
  ignore market-impact and execution-scheduling models — they are background at
  €200 `strategies[20][90]`. The energy belongs on **fees + spread + delay**,
  charged distributionally. This is a *narrowing* relevance: it tells us what NOT
  to build.
- **Costs ARE the product's decision variable.** `strategies[11][72][51]` are the
  crispest explanation of *why* the advisor keeps recommending hold: the same rule
  is a winner gross and a loser net. A fee-sensitivity sweep makes that visible
  and "traceable."
- **"Sizing > signal" reframes overlays as risk tools, not alpha.** The honest
  contribution of vol-targeting / stops / regime-flat overlays is
  drawdown/variance reduction, regime-conditional — and crypto's inverse leverage
  effect means an equity-calibrated overlay can actively mis-time the coin
  `strategies[58]`. This keeps overlay claims plausible.
- **The blend is the one defensible "better default product."** An
  active+buy-and-hold blend cut drawdown ~50% with little return give-up across
  the honest studies `strategies[68][47]`. That is a real, gateable candidate that
  improves the *experience* without claiming to beat hold on terminal wealth.
- **Honest on expected-null.** Every directional/overlay candidate here is
  expected to tie-or-lose buy-and-hold on net terminal wealth. The value is
  *measuring* that honestly (state-dependent costs, turnover penalty, geometric
  return) so the advisor's "hold" verdict is credible.

---

## 4. Advantages for the project

- **A small, well-scoped cost model is a feature.** Modeling exactly the three
  costs that matter (fees, spread, delay) — and proving impact/scheduling don't —
  is more defensible and more maintainable than a heavy execution simulator.
- **The fee-sensitivity sweep is presenter gold.** "This rule wins at 0 bps and
  loses at 2–4% round-trip" `strategies[11]` is the single most intuitive way to
  explain ship-passive to a retail operator.
- **State-dependent spread closes a real overstatement hole.** A flat spread
  flatters every reversal/dip-buy rule precisely because the real spread widens
  when those rules trade `strategies[27]`. Fixing it makes the gate harder to fool
  — complementary to the selection-bias gate in the other file.
- **The blend is low-risk and high-perceived-value.** It reuses existing arms
  (active sleeve + buy-and-hold benchmark), needs no new signal research, and
  delivers a drawdown improvement a retail user actually feels.
- **Reuses existing seams.** Cost lives in `crates/cost/`; the bake-off field in
  `default_field()`; the forward-plan in `crates/ui/src/forward_plan/`. Most work
  here is configuration + additive arms, not new subsystems.

---

## 5. Problems and challenges

- **HARD CONSTRAINT — overlays ship a day-1 baseline-equity-divergence e2e.**
  Per the v3-volatility-forecaster-noop precedent, any sizing/exit/blend overlay
  MUST ship an e2e test asserting its output equity diverges from the un-targeted
  baseline by ≥ a testable epsilon when the decision variable is non-trivial. Unit
  tests on the math layer + anchored backtest reports are NOT sufficient — a `scale`
  computed but never applied passed both. Pattern:
  `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`.
- **HARD CONSTRAINT — Decimal, not f64, in the equity/P&L path.** The cost model
  is already Decimal (`apply_slippage` etc. operate on `Decimal`); the
  square-root impact model isolates its f64 to a documented conversion boundary
  (`apply_slippage_sqrt`, D-T1.3). Any new cost term (delay, state-dependent
  spread) must keep f64 out of the fill-price path.
- **HARD CONSTRAINT — `ui` must NOT depend on strategy/exec/llm/models.** The
  forward-plan UI (`crates/ui/src/forward_plan/`) surfaces the plan; the actual
  rule resolution + cost-aware filter must live behind the existing
  agent/strategy seam (`build_registry_for` in `crates/agent`), reached via a
  permitted boundary — not by adding a UI→strategy dependency.
- **HARD CONSTRAINT — anchored report SHAs are byte-immutable (119/119).**
  Changing the default cost model (e.g. flipping the default slippage `bps`, or
  adding a delay term to the default path) would change every backtest's numbers
  and break anchors. New cost terms must be **opt-in / non-default** (the
  `SlippageModel::default()` byte-identity discipline already in place), with new
  anchors for new configs.
- **HARD CONSTRAINT — gate/bands FROZEN; paper-only.** A state-dependent spread or
  delay term changes net equity, which feeds the FROZEN gate — fine, because the
  gate operates on whatever equity it's given, but the *default* path must stay
  anchor-stable. And maker/limit modeling must remain a *simulation* assumption,
  not a step toward live execution (no live trading — out of scope).
- **Forward-fidelity gap (known).** The shipped forward paper-trade runs an SMA
  proxy for non-SMA crowned picks; the documented fix (F5b) is to reuse the
  bake-off's ComposedStrategy-from-TOML in `build_registry_for`. A cost-aware
  filter or blend on the forward-plan inherits this gap until F5b lands — so this
  work should *follow* or *bundle with* F5b, not assume forward fidelity.
- **Maker fills add fill/queue risk we can't fully simulate.** A maker (limit)
  model genuinely costs less `strategies[65]` but assumes fills that may not
  happen; conservatism argues for taker-style crossing as the default and maker as
  an explicit, clearly-labeled optimistic option.
- **State-dependent costs are hard to calibrate honestly.** "Wider spread on
  volatile bars / weekends" needs a defensible parameterization; an arbitrary one
  just trades one mis-statement for another. Prefer a conservative, documented
  multiplier over a fitted one.

---

## 6. Concrete next steps / candidate work items

- **[P0] `fee-sensitivity-sweep`** — first-class ranking output: per arm, the
  round-trip cost at which net return crosses buy-hold. Lives next to the bake-off
  ranking (`crates/backtest/src/bakeoff/`), reads existing per-arm equity. The
  single most product-aligned, presenter-ready addition. Citation:
  `strategies[11]`.
- **[P0] `turnover-penalty-in-rank`** — ensure short-window crossovers cannot win
  on gross numbers; penalize turnover explicitly in the ranking inputs (additive,
  does not touch FROZEN bands). Citation: `strategies[51][72]`.
- **[P1] `cost-model-audit-arrival-price`** — verify `sim_slippage_cost`
  (`crates/backtest/src/scenarios/sim.rs`) charges spread + fees + a **delay**
  term vs the arrival/decision-bar price, treated as a state-dependent draw. Add a
  delay term as a **non-default, anchor-additive** cost option if missing.
  Citation: `strategies[87][57]`.
- **[P1] `state-dependent-spread`** — opt-in cost-model variant in
  `crates/cost/src/slippage.rs` that widens spread on high-vol bars and weekends
  (conservative documented multiplier). New anchors for the new config; default
  path unchanged. Citation: `strategies[27][55][71]`.
- **[P1] `active-plus-hold-blend`** — new bake-off arm (extend the ADDITIVE field
  list à la `default_ensemble_field`, NOT `default_field()` directly) that blends
  the active sleeve with the buy-and-hold core; ships with a day-1
  baseline-divergence e2e. The "better default product." Citation:
  `strategies[68][47]`.
- **[P1] `cost-aware-trade-filter`** — forward-plan filter: act only when
  `|expected_move| > λ·c·|Δpos|`; behind `build_registry_for` (`crates/agent`),
  surfaced via the existing forward-plan seam. Expect reduced cost-drag, not
  return; **bundle with F5b** so forward fidelity holds. Citation:
  `strategies[65]` + meta-labeling.
- **[P2] `maker-taker-cost-option`** — opt-in maker/limit cost mode with explicit
  fill/queue-risk caveat; default stays taker. Citation: `strategies[65]`.
- **[P2] `horizon-banded-sweep`** — restrict each family's lookback sweep to its
  theoretically-correct band (reversion short/long, trend middle) rather than a
  dense grid; reduces both overfitting and the multiple-testing count.
  Citation: `strategies[49]`.

---

## 7. Open questions for analyst & architect

- **Does our cost model already include a delay term?** `sim_slippage_cost`
  charges fee + slippage; is there an arrival-vs-fill *delay* component, or do we
  fill at the decision bar? `strategies[87]` says delay + opportunity cost are
  first-class TCA terms — if absent, is adding them worth the new-anchor churn?
- **Default cost realism vs anchor stability.** State-dependent spread and a delay
  term are *more honest* but changing the default breaks 119 anchors. Do we (a)
  keep them opt-in forever, (b) cut a versioned default bump with a fresh anchor
  set, or (c) accept the current flat-bps default as "conservative enough"?
- **Is the blend a product default or a candidate arm?** The blend is the one
  robust win — should the advisor *recommend* the active+hold blend when no active
  arm is robust, or only offer it as a baked-off option?
- **Maker modeling vs the no-live-trading boundary.** A maker cost mode is a
  simulation assumption, but does it risk implying a path to live execution the
  operator has ruled out? How do we label it so it stays clearly paper-only?
- **F5b sequencing.** Should the cost-aware trade filter and blend wait for F5b
  (forward fidelity), or can they ship against the SMA-proxy with an explicit
  caveat? The forward-plan numbers are only trustworthy for the crowned family's
  rule once F5b lands.
- **State-dependent spread calibration.** What is a defensible, conservative
  vol-/weekend-spread multiplier that improves honesty without becoming a fitted
  parameter of its own?

---

## 8. What NOT to do / effort & blast radius

- **Do NOT build VWAP/TWAP/IS scheduling or a market-impact simulator.** They are
  background at €200 `strategies[20][90][91]`; building them is effort spent where
  the literature says it doesn't matter.
- **Do NOT change the default slippage/fee path casually.** It breaks anchors.
  New cost realism is opt-in + new-anchored.
- **Do NOT ship any overlay without a day-1 baseline-equity-divergence e2e.** The
  noop-fix precedent is a hard rule, not a guideline.
- **Do NOT apply stops/exits family-blind.** Gate them by a trend filter
  `strategies[38]`; on mean-reversion they actively destroy return.
- **Do NOT sell vol-targeting / blend / trade-filter as alpha.** They are
  drawdown/cost-drag tools; on net terminal wealth they are expected ≈ null vs
  hold.
- **Effort / blast radius:** fee-sweep + turnover penalty = small, additive,
  high-value. Cost-model audit = small (read + possibly one opt-in term).
  State-dependent spread = medium (opt-in variant + new anchors). Blend +
  cost-aware filter = medium and **must** carry e2e divergence tests and respect
  F5b. No new external dependencies; the `cost`, `backtest/bakeoff`, `agent`, and
  `ui/forward_plan` seams already exist.
