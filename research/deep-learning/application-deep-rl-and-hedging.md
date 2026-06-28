# Application — Deep RL, Deep Hedging & Adversarial Robustness

> **Audience:** analyst + architect, planning the next big steps.
> **Source:** `research/deep-learning/knowledge.md` (100-paper synthesis) + the cited
> `deep-learning[N]` ledger entries in `research/deep-learning/papers.md` + `research/SYNTHESIS.md`.
> **Scope of this file:** deep **reinforcement learning** for trading/execution (its
> overfitting + non-reproducibility), **deep hedging** (risk-objective policy learning,
> turnover regularization, costs → inaction), and **adversarial robustness** (input
> parsimony as a robustness *property*, not just a simplicity preference). The
> forecasting-vs-baselines and significance/DSR material lives in the sibling file
> `application-forecasting-and-significance.md` — that one carries the single
> *actionable* outcome (the significance layer feeding P0); this one is mostly
> **"what NOT to do" plus a handful of reusable sub-ideas and one genuine robustness
> principle.**

> **Our app (ground every claim against this):** a Rust **single-coin crypto
> investment advisor** — paper/sim only, NOT advice, NOT live. Journey: pick ONE coin
> + budget → **bake off EVERY strategy** → **rank** under a FROZEN robustness gate
> (1000-path moving-block bootstrap; FRAGILE ⇒ cannot be crowned; **buy-and-hold is
> always the benchmark + exempt**) → forward rule-based plan → **watch it paper-trade**.
> Validated thesis: **no active strategy robustly beats buy-and-hold net of costs.**
> Operator goal: "a framework for trading with traceable and plausible trading."

---

## 1. Summary of the research

**Deep RL for trading — the headline numbers do not survive scrutiny, and the field's
own people say so.** The canonical crypto-portfolio DRL paper reports 4× in 50 days
`deep-learning[10]` — on the 2017 bull run, with survivorship-selected assets and no
buy-and-hold benchmark. The pattern recurs: DQN +29.93% / Sharpe 2.7 on a **30-day**
out-of-sample window with no B&H compare `deep-learning[23]`; SAC +176% with Markowitz
but **no** buy-and-hold on an uptrending 2023–24 `deep-learning[28]`; a PPO+LSTM "31.67%
more than the best benchmark" with unstated costs and window `deep-learning[73]`. The
critical/meta literature is consistent and largely from insiders: no reproducible
baseline-beating standard `deep-learning[12][25][50]`; DRL profits are "false positives
due to overfitting" `deep-learning[18]`; FinRL-Meta names low SNR, survivorship bias,
and backtest overfitting as the *defining* problems `deep-learning[20]`; **changing only
the random seed produces statistically distinct learning curves (t=−9.09, p=0.0016) and
architecture alone swings results 45×** `deep-learning[47]`; an offline policy "is not
trustworthy" because it memorizes one historical path `deep-learning[27]`; and RL beats
buy-and-hold *only when the test regime was already in training* `deep-learning[63]`.

**The two strongest corroborations of our thesis are RL papers.** A rigorously-built
SAC portfolio with real costs, 16 OOS folds over 2003–2026, three markets, HAC-robust
inference, finds **"no strategy achieves statistically significant excess returns
relative to Buy & Hold"** `deep-learning[42]`. And the FinRL Contests — 200+ teams over
three years — found the best **crypto** RL agent essentially **tied holding Bitcoin
(0.66% vs 0.74%)**, while a Sharpe-9.56 top team went **negative the very next
out-of-sample week** `deep-learning[77]`. Even a competing global crowd cannot beat
buy-and-hold on crypto.

**Deep hedging — the right *shape* of objective, and the structural reason holding wins.**
Deep hedging frames the policy as directly minimizing a convex risk measure of P&L
(decision, not forecast) `deep-learning[2]`; Deep Momentum Networks optimize a
differentiable **Sharpe-ratio loss** and add turnover regularization, with the edge
honestly dying at ~2–3 bps `deep-learning[15]`; a dual-Q construction tracks mean AND
variance of cost for a risk-adjusted objective `deep-learning[36]`; a Dirichlet policy
yields valid normalized weights by construction `deep-learning[45]`. The recurring,
on-thesis finding: **with realistic costs the optimal policy trades LESS** — costs push
the optimal action toward inaction/holding `deep-learning[15][36]`, the same force that
makes buy-and-hold hard to beat (independently confirmed on hourly BTC, where a
cost-aware trade filter is the difference between viable and ruinous, yet *still* does
not significantly beat hold `deep-learning[52]`).

**Adversarial robustness — input parsimony is a robustness property.** A DRL trading
agent's reward was cut **214%** by an adversary who merely *trades against it*
(gray-box, no model access — just a normal trading account) `deep-learning[65]`. A
multivariate forecaster's error inflates **+215%** when an attacker makes sparse,
imperceptible perturbations to half of the *correlated auxiliary* series (never touching
the target), and the best defense recovers only part `deep-learning[93]`. A
stock-sentiment model is flipped by **1 injected tweet / 1 word** (~17% success, ~25%
portfolio loss over two years) `deep-learning[35]`. The consistent finding: **more inputs
= more fragility** (multivariate is ~3× more attackable; the cross-series channel is a
*new* attack surface `deep-learning[93]`), under a realistic "just participate in the
same market/feed" threat model. Crypto is far more manipulable than equities — so opaque
learned policies *and* rich multivariate/alt-data inputs are a liability, and simple
transparent rules on few hard-to-game features are **robust by construction**.

---

## 2. Possible solutions / what can be done with this research

This corpus yields **no deployable alpha component** for a single coin. What it yields is
(a) a documented "avoid" list, (b) a few reusable *sub-ideas* for the one place a learned
policy could ever plausibly sit — **position sizing**, not direction — and (c) one
genuine design principle (input parsimony).

1. **Treat all DRL/EIIE/DQN/SAC/DDPG/PPO trading machinery as multi-asset-only and
   unproven.** Irrelevant to a single coin; not worth the overfitting surface
   `deep-learning[1][10][13][23][28][42][45]`. Do *not* build an RL trader.

2. **If a sizing policy is ever explored (it should not be soon), harvest only the
   transferable sub-ideas** — and expect the cost curve to kill the edge:
   - optimize a **risk / Sharpe objective directly** with **turnover regularization**
     `deep-learning[2][15][36]`;
   - **Dirichlet parametrization** for a valid "fraction of budget in [0,1]" output
     `deep-learning[45]`;
   - **dual-Q mean-variance** cost objective for risk-awareness `deep-learning[36]`;
   - **SAC > DDPG** if forced to choose a continuous-action algorithm `deep-learning[28][42]`;
   - **validation-Sharpe agent-selection + turbulence risk-off** as regime-adaptive
     switching logic `deep-learning[13]`; multi-validation-period selection
     `deep-learning[76]`.

3. **Adopt the cost-proportional trade filter as a *strategy primitive* (not a model).**
   "Act only when expected move > k·round-trip-cost" turned ruinous crypto ML strategies
   viable — and is cheap to add to the bake-off `deep-learning[52]`. Honest expected
   result: it cuts turnover but still does not beat buy-and-hold.

4. **Elevate "input parsimony" from a preference to a stated robustness requirement.**
   The adversarial results justify a hard rule: prefer few, hard-to-game price/volume
   features; reject noisy social-sentiment / rich alt-data inputs into the trading
   decision `deep-learning[35][65][93]`. (This also reinforces keeping LLMs on the
   narration rail, off the alpha rail — see the `llms` topic and SYNTHESIS §4.)

---

## 3. Relevance for the project

**Like the sibling file, the relevance is largely negative — and that is valuable.**

- **The two best external validations of our thesis are in this corpus.** `deep-learning[42]`
  (HAC-robust SAC, no significant excess vs B&H) and `deep-learning[77]` (FinRL Contests
  crypto ties holding BTC; Sharpe-9.56 → negative next week) reproduce our exact thesis —
  one of them on crypto specifically, by DRL *proponents*. That is the strongest possible
  "we are not being lazy skeptics; the rigorous RL crowd agrees" citation for the product
  narrative.

- **It is the empirical root of why the FROZEN gate is shaped the way it is.** The
  seed/architecture variance result `deep-learning[47]` (t=−9.09 from the seed alone, 45×
  from architecture) is *why* a single-window Sharpe is worthless and why we demand
  robustness across 1000 resamples + a weakest-link verdict. The Sharpe-9.56→negative
  result `deep-learning[77]` is the expected, not surprising, consequence.

- **Deep hedging explains, structurally, why buy-and-hold is hard to beat.** "Costs push
  the optimal policy toward inaction" `deep-learning[15][36]` is the mechanism behind our
  validated thesis, stated from the policy-optimization side. This is good narration
  material: *we are not asserting holding wins by fiat; the cost-aware optimal policy
  derives it.*

- **Adversarial robustness gives "simple & transparent" a rigorous backing.** Our
  preference for simple rules over opaque models is usually framed as overfitting
  avoidance. This corpus adds a second, independent reason: **simple transparent rules on
  few hard-to-game features are robust by construction in a manipulable market**
  `deep-learning[35][65][93]`. That strengthens the "traceable & plausible" pitch — a rule
  a user can read is also a rule an adversary cannot cheaply flip.

- **It is the documented justification for the retired `forecast` overlay chain and the
  no-RL-trader stance.** `crates/forecast/` holds an opt-in/narration-only TCN/PatchTST/
  GARCH-σ chain concluded not-beating-passive; this file is the literature backing for
  keeping deep ML out of the decision path.

---

## 4. Advantages for the project

- **Avoided work is the main advantage.** The clearest "advantage" is a confident *no*:
  do not build an RL trader, do not wire alt-data/sentiment into the decision, do not
  chase SAC/PPO crypto numbers. Each "no" is backed by a citation, saving design debate.
- **A handful of cheap, testable primitives.** The cost-proportional trade filter
  `deep-learning[52]` and (if sizing is ever explored) turnover regularization + Dirichlet
  output + dual-Q risk objective `deep-learning[15][36][45]` are concrete, low-risk things
  to *test* in the bake-off — with the honest expectation they won't beat hold, which is
  itself a publishable-to-the-user result.
- **A stated robustness principle (input parsimony)** that is independently motivated and
  cheap to enforce — it is mostly a *constraint on what we refuse to add*, so it costs
  nothing and reduces attack surface `deep-learning[35][65][93]`.
- **Strong, neutral, crypto-specific corroboration** for the product narrative
  `deep-learning[42][77]` — the kind of citation that turns "measured honesty" from a
  posture into an evidenced position.

---

## 5. Problems and challenges (risks + HARD CONSTRAINTS bumped)

- **Paper-only / not-live / not-advice.** Nothing here changes that. An RL *trader* is
  doubly out of scope: it is unproven *and* it would be a live-decision engine. Keep all
  of this on the sim/research side.
- **FROZEN gate / bands additive-only.** None of these sub-ideas may alter the FRAGILE
  band or the 5-signal weakest-link composite in
  `crates/backtest/src/bakeoff/robustness.rs`, nor the F2 crown comparator in `rank.rs`.
  A cost-proportional trade filter is a *strategy primitive* evaluated *through* the
  frozen gate — not a modification of it.
- **Overlays ship a day-1 baseline-equity-divergence e2e.** A cost-proportional trade
  filter, a turnover penalty, or any sizing modifier IS an overlay/sizing-modifier and
  inherits the mandate: a day-1 e2e asserting its output equity diverges from the
  un-modified baseline by ≥ a testable epsilon when the decision variable is non-trivial
  (the v3-vol-overlay-noop precedent; pattern at
  `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`; neutrality-test analogue at
  `crates/forecast/tests/patchtst_overlay_neutrality.rs`). A filter that is silently a
  no-op is exactly the failure that mandate exists to catch.
- **USDT-denominated, Decimal not f64.** Any sizing fraction, cost threshold, or
  turnover quantity that touches money is `Decimal`. RL/hedging math (risk measures,
  Q-values, Dirichlet draws) is floating-point and must stay behind a boundary that never
  leaks f64 into the money path.
- **`ui` must NOT depend on strategy / exec / llm / models / forecast.** If any of this
  ever surfaces in the cockpit (e.g. showing a trade-filter's effect), the data must
  reach `ui` through the existing report/DTO seam — confirmed today: `ui` has no such
  dependency. Do not introduce one.
- **Deep ML overlays are opt-in / narration-only.** The retired TCN/PatchTST/GARCH-σ
  chain concluded not-beating-passive. Anything learned from this corpus stays opt-in and
  off the default decision path.
- **Single coin = no cross-section.** Most RL value `deep-learning[24][32][45]` and the
  pairs/stat-arb route `deep-learning[57]` are cross-sectional or two-leg market-neutral.
  Our long-only single-coin advisor *cannot take the short leg* — a structural, not
  tunable, mismatch. Do not import multi-asset RL expecting it to transfer.
- **Adversarial fragility is a reason to *not* add inputs.** The risk is that a future
  "let's add sentiment/on-chain/alt-data" proposal increases the attack surface
  `deep-learning[35][65][93]`. The challenge is cultural: treating "fewer inputs" as a
  feature, not a limitation.
- **Cost curve always bites.** Every sub-idea here is expected to lose its edge within a
  few bps `deep-learning[15][52]`. The challenge is presenting "we tested it and it didn't
  beat hold" as a *successful, honest* outcome rather than a failed experiment.

---

## 6. Concrete next steps / candidate work items

> Almost everything here is **P2 / avoid**. The one cheap, testable, on-roadmap item is
> the cost-proportional trade filter (a strategy primitive, gated normally).

| # | Item | Codebase location | Priority | Notes |
|---|------|-------------------|----------|-------|
| R-1 | **Cost-proportional trade filter** as a bake-off strategy primitive — act only when `|expected_move| > k·round-trip-cost` | new strategy/overlay in `crates/strategy/` + day-1 baseline-divergence e2e | P1/P2 | Cheapest testable item. Cuts turnover; **expected NOT to beat hold** `deep-learning[52]`. MUST ship the divergence e2e (overlay mandate). Evaluated *through* the frozen gate. |
| R-2 | **Stated input-parsimony robustness rule** — codify "few hard-to-game price/volume features; reject noisy alt-data/sentiment into the decision" in the spec | `spec/` (product/architecture note) | P1 | Free; it is a constraint on what we refuse to add `deep-learning[35][65][93]`. Reinforces LLM-on-narration-rail. |
| R-3 | **(If sizing is ever explored) risk-objective sizing sub-ideas** — Sharpe/convex-risk loss + turnover penalty + Dirichlet [0,1] output + dual-Q mean-variance | future `crates/risk/` or `crates/strategy/` experiment + day-1 divergence e2e | P2 | Only if a *sizing* (how much of budget / when to de-risk) policy is ever pursued. Not a directional signal. Expect cost curve to bite `deep-learning[2][15][36][45]`. |
| R-4 | **Do NOT build an RL trader / DRL decision engine** (DQN/PPO/SAC/DDPG/EIIE) | — | **Avoid** | Unproven, overfits, seed/arch variance dominates, ties hold on crypto. `deep-learning[42][47][77]`. |
| R-5 | **Do NOT add deep-hedging machinery as a product** (we hold spot, not options) | — | **Avoid** | Background only; harvest the *objective shape* and "costs → inaction" insight, not the model. `deep-learning[2][36]`. |
| R-6 | **Do NOT wire sentiment / alt-data / social feeds into the trading decision** | — | **Avoid** | Adversarially fragile; crypto feeds are heavily manipulated. `deep-learning[35][65][93]`. |

**Highest-value item here: R-1 (cost-proportional trade filter)** — but note its value is
mostly *confirmatory*: it is the one concrete, cheap experiment this corpus suggests, and
its honest expected outcome ("reduces turnover, still doesn't beat hold") *adds evidence
to the thesis*. The genuinely actionable cross-file item remains the **significance layer
(P0)** documented in `application-forecasting-and-significance.md`.

---

## 7. Open questions for analyst & architect

1. **Is R-1 worth the e2e cost?** A cost-proportional trade filter is cheap to write but
   must ship a day-1 baseline-divergence e2e. Given the strong prior that it won't beat
   hold, is the value of *demonstrating* that (honest-outcome evidence) worth the e2e and
   maintenance? Or is the literature citation `deep-learning[52]` sufficient and we skip
   the code?
2. **Where would a sizing policy live, if ever?** If the team ever revisits *position
   sizing* (not direction), does it belong in `crates/risk/` (alongside vol-targeting /
   drawdown overlays) or `crates/strategy/`? This determines whether R-3 is a risk-tool
   experiment or a strategy experiment — and both need the divergence e2e.
3. **Codify input-parsimony where?** R-2 is a spec note — `spec/product.md` (what we
   are/aren't) or `spec/architecture.md` (a design constraint), or both? It interacts with
   the LLM narration-rail decision (ADR-0064).
4. **Adversarial framing in the product.** Do we want to *surface* "this advisor uses few,
   hard-to-game inputs by design" as a user-facing robustness claim, or keep it internal?
   It is a genuine differentiator but invites scrutiny.
5. **Cost-model fidelity for R-1.** The trade filter's `k·round-trip-cost` threshold needs
   a credible round-trip cost (spread + fee + slippage) in `crates/cost/`. Is the current
   cost model rich enough, or does R-1 depend on a cost-model upgrade first?

---

## 8. What NOT to do / out of scope

- **No RL trader, no DRL decision engine.** Profits are overfitting; seed/architecture
  variance dominates; on crypto it ties buy-and-hold and reverses out-of-sample.
  `deep-learning[18][42][47][63][77]`.
- **No deep-hedging product** (we hold spot, not an options book). Keep only the *objective
  shape* (risk measure + turnover penalty) and the "costs → inaction" insight.
  `deep-learning[2][15][36]`.
- **No sentiment / social / rich alt-data into the trading decision.** Adversarially
  fragile under a realistic threat model; crypto feeds are heavily manipulated; more inputs
  = more attack surface. `deep-learning[35][65][93]`.
- **No pairs / stat-arb / market-neutral strategies.** Structurally two-leg; our long-only
  single-coin advisor cannot take the short leg. `deep-learning[57]`.
- **No multi-asset RL imported expecting transfer.** The genuine RL/ML value is
  cross-sectional; a single coin has no cross-section. `deep-learning[24][32][45]`.
- **Nothing here touches the FROZEN gate/bands or anchored reports.** Sub-ideas are tested
  *through* the gate; overlays/sizing-modifiers ship a day-1 baseline-divergence e2e; money
  quantities stay `Decimal`; `ui` keeps its clean dependency boundary.
