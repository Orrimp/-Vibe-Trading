# Application — Automated Strategy Search (GA / GP / Symbolic Regression / Neuroevolution / LLM-Code-Evolution)

_Decision doc for analyst & architect. Distilled from `research/evolution/`
(100 papers; cite `evolution[N]`) and `research/SYNTHESIS.md`. This file covers the
**generative / search** half of the topic: genetic algorithms, genetic programming,
symbolic-regression alpha mining, neuroevolution, swarm/MOEA optimizers, and the
LLM-driven code-evolution lineage (FunSearch → AlphaEvolve → MadEvolve/QuantEvolve).
The **defensive** half — the DSR/MinBTL formulas, anti-overfitting fitness, and the
random null that *feed our gate* — is the sibling doc
`application-anti-overfitting-and-search-discipline.md`._

> **Our app:** a Rust single-coin crypto **advisor** (paper/sim, NOT advice, NOT
> live). Pick one coin + budget → bake off a FIXED pre-registered slate of strategies
> → rank under a FROZEN 1000-path moving-block-bootstrap gate (weakest-link verdict;
> FRAGILE ⇒ can't crown; **buy-and-hold is always the benchmark and is exempt**) →
> forward rule-based paper-trade. Validated thesis: **no active strategy robustly
> beats buy-and-hold net of costs.** The product sells **measured honesty** — "a
> framework for trading with traceable and plausible trading."

> **One-line verdict for the impatient reader:** automated strategy search is the
> **single highest-overfitting-risk idea in the entire 900-paper program.** It is
> industrialized data-snooping by construction. Build it only behind heavy guards,
> expect a null result, and treat the search budget as a cost charged against
> statistical significance. The durable, low-risk export from this literature is NOT
> a search engine — it is the discipline in the sibling doc.

---

## 1. Summary of the research

**What "automated strategy search" means here.** A family of methods that *search a
vast space of trading rules / factors / parameters against backtest feedback*:
genetic programming evolving rule trees (`evolution[1][10][82][89][90][99][100]`),
genetic algorithms tuning indicator parameters (`evolution[4][86][95][96]`),
symbolic-regression / RL / MCTS alpha-factor miners
(`evolution[2][6][8][11][12]`), neuroevolution of networks
(`evolution[7][59][63][64]`), swarm/ant/MOEA optimizers
(`evolution[16][17][80]`), and the newest wave — **LLM-as-mutation-operator
code-evolution**: FunSearch (`evolution[18]`, Nature) → AlphaEvolve
(`evolution[71]`) → trading ports MadEvolve (`evolution[14]`) and QuantEvolve
(`evolution[69]`).

**The unifying statistical fact.** Every one of these *is multiple-hypothesis
testing at scale*. Searching N configurations against a noisy backtest and crowning
the best is, by construction, the data-snooping setup. Bailey & López de Prado
**prove** via Extreme Value Theory that the expected *maximum* in-sample Sharpe
across N zero-skill trials grows with N, and that the in-sample winner is
*negatively* correlated with out-of-sample return (`evolution[29][98]`). The fancier
the search, the *faster* it finds spurious patterns — the statistics that doom it are
unchanged. Even the careful miners admit this in their own words: TreEvo states
"increasing the number of evaluations may make the methods more prone to
overfitting" (`evolution[11]`); a general GP-methodology study finds GP overfitting
"cannot be solved or suppressed as easily as in more traditional approaches"
(`evolution[21]`).

**What the honest papers conclude — and it is our thesis.** The foundational
GP-trading paper (Allen & Karjalainen 1999, `evolution[1]`) — proper train/select/
test split + costs — found evolved rules do **not** robustly beat buy-and-hold on the
S&P OOS. Twenty-five years later the careful crypto walk-forward / double-OOS study
(`evolution[5]`) reaches the same verdict and shows optimized params beat *random*
params only 8-13% of the time. GA FX systems are superb in-sample and **unprofitable
OOS once costs are imposed** — the authors conclude "markets could be efficient"
(`evolution[86][95]`). Neuroevolution (NEAT) *loses* to buy-and-hold by ~9 points
over 22 years before costs (`evolution[7]`). The clean pattern: active/evolved
strategies, honestly benchmarked net of costs, deliver **risk reduction at best, not
alpha** (`evolution[5][7][27][28][37]`).

**Every "evolution beats the market" result rides a structural lever we lack.** This
is the load-bearing finding for our scope. On inspection, each affirmative result
depends on something a single-coin, long-only, unleveraged retail advisor cannot do:

- **HFT latency / order-flow front-running** — STGP on millisecond data "constantly
  beats the market" by anticipating order flow (`evolution[93][94]`).
- **Cross-sectional breadth / long-short** — neuroevolution and alpha factors only
  beat the index across hundreds of names (`evolution[3][49][63][88]`).
- **Carry / funding** — AdaptiveTrend beats BTC B&H (Sharpe 2.41 vs 0.17) but via a
  150-asset cross-sectional long-short + funding carry (`evolution[49]`).
- **Leverage** — GEP "beats other forecasters" only with a leverage filter
  (`evolution[83]`).
- **Index-reconstitution flows** — XCS beats B&H+random via MSCI add/remove events
  (`evolution[88]`).

Or the result simply **drops a control** — no costs, no B&H, no OOS, no
multiple-testing correction (`evolution[4][20][44][96]`).

**The LLM-code-evolution wave changes accessibility, not statistics.** FunSearch
(`evolution[18]`) genuinely found *provably* better algorithms — but only because it
had a **sound, cheap, correct evaluator** (a math verifier). Ported to trading, the
evaluator becomes a noisy, finite, gameable backtest, and the same machinery finds
*spurious* winners. MadEvolve (`evolution[14]`) is the most self-aware port (BTC,
chronological train/valid/test, market-impact in fitness, IS→OOS-vs-theory check) —
yet it measures gains vs its *own pre-evolution baseline*, never vs holding BTC. The
wave makes "evolve a whole strategy as code" cheap and accessible, which only
*raises* the importance of a hard cost-aware OOS-vs-B&H gate, because the evaluator —
the one part that decides real edge — does NOT improve with the search.

---

## 2. Possible solutions / what can be done with this research

There are three honest options. They are listed in descending order of recommendation
for a ship-passive product.

1. **Do nothing — and document why (Recommended).** Keep our FIXED, pre-registered
   strategy slates (combination-search, short-selling, signal-library) and the
   grid-sweep. Treat this literature as the *evidence base* for why we do not run an
   open-ended evolutionary miner. This is the durable choice: the research says the
   expected outcome of automated search on a single coin is a null, and the guards
   needed to run it safely are expensive to build and easy to get subtly wrong.

2. **Strengthen the existing FIXED bake-off's discipline (Recommended, and already
   the P0 roadmap).** Our grid-sweep *is itself a search* over `(coin, window)`, so
   it must be significance-charged. This is not "adding a search engine"; it is making
   the search we already run honest. All the durable formulas (DSR, MinBTL, PBO,
   random null) live in the sibling doc and feed `bakeoff/{robustness,rank}.rs`.

3. **IF the operator ever explicitly wants automated search — build it behind the
   full guard stack, expecting null (P2, opt-in, off by default).** The literature
   gives a concrete "honesty kit." If we ever attempt it, the minimum viable guard
   stack is:
   - **Walk-forward / double-out-of-sample** — optimize on global-train via
     walk-forward, evaluate **exactly once** on a never-touched window
     (`evolution[5]`). Beware "OOS" data reuse.
   - **Charge the search budget against significance** — the DSR/MinBTL bar tightens
     *as N grows* (`evolution[29][98]`); see sibling doc.
   - **Cost inside the fitness, not as an afterthought** — market-impact penalty in
     the objective; scale-invariant metrics (Sharpe/Calmar can't come from mere
     sizing) (`evolution[14]`).
   - **Compare IS→OOS degradation against multiple-testing theory** — if degradation
     stays *below* the p-hacking baseline, the gain is more likely real
     (`evolution[14]`).
   - **Difficulty-weighted training-subset rotation** as a pre-regularizer — rotate
     candidates across regime blocks, up-weight the blocks each does worst on
     (`evolution[10][92]`); cousin of our weakest-link bootstrap.
   - **Beat a matched-activity random null**, not just B&H (`evolution[5][30][31]`).
   - **Keep buy-and-hold as the benchmark every elite must clear** — the one control
     MadEvolve omitted (`evolution[14]`).
   - **Express evolved logic in our typed composed-strategy DSL** so it is readable
     and re-runnable, not a black box (`evolution[10][82][100]`).
   - Every evolved elite then runs through the *same* FROZEN gate as a hand-written
     strategy. No special path. No exemption.

A **non-search-engine spinoff** worth noting: quality-diversity archives (MAP-Elites
over behavior descriptors like turnover/exposure/risk-profile) are a *presentation*
tool, not an overfitting cure (`evolution[33][62][69]`; QuantEvolve realizes exactly
a MAP-Elites-of-investor-preferences menu). If we ever want to show a user a *menu*
of behaviorally-diverse strategies, this is the shape — but every archived cell must
individually clear the FROZEN gate; "diverse" is not "robust."

---

## 3. Relevance for the project

**High relevance as a guardrail and a thesis-confirmer; low-to-negative relevance as
a feature to build.**

- **It validates the frozen gate as a competitive advantage.** Nine independent topic
  reviews converge here: active single-asset trading does not robustly beat B&H net of
  costs. The evolution literature is the *adversarial* test of that thesis — the
  sub-field whose entire purpose is to manufacture edges — and it still lands on the
  null on a single coin (`evolution[1][5][7][86][95]`; SYNTHESIS §1). That is strong
  evidence our gate + benchmark are correctly calibrated, not over-strict.

- **It maps the "false positives" our gate must reject.** The anti-patterns
  (`evolution[4][20][44][96]`: +550% scalping / 320%/yr FX / ~2000 alphas at
  Sharpe 2.8, all with no costs/B&H/OOS) are *exactly* the shape of result the
  1000-path bootstrap-vs-B&H-net-of-costs gate exists to deflate. They are useful as
  regression fixtures: a synthetic version of each should be refused a crown.

- **It explains WHY a crowned pick is temporary — supporting honest narration.**
  Coevolution / Red Queen / Adaptive Markets (`evolution[35][36][94]`) frame edges as
  competed away by other strategies — a deeper cause of decay than overfitting alone.
  This directly supports our "traceable & plausible" posture: any active crown is
  provisional, B&H is the durable benchmark, and re-evaluation should be event-driven.

- **Honest expected-null framing.** If the operator ever asks "can't we just evolve a
  winning strategy?", the truthful answer grounded in this folder is: *on a single
  coin, long-only, net of costs, almost certainly no — and the more configs we try,
  the more certain the null becomes (`evolution[29][98]`).* The places it "works" are
  structural levers we are scoped out of. We can build the machinery; we should expect
  it to crown nothing, and we should say so up front.

---

## 4. Advantages for the project

What the project *gains* from having read this literature (mostly defensive):

1. **A pre-registration justification with teeth.** Pre-registering FIXED slates (no
   free parameter hunts) is our standing overfit defense. This literature is the
   proof that the alternative — open-ended search — is industrialized snooping
   (`evolution[1][9][21][29]`). The advantage is a defensible, citable rationale for a
   design choice operators might otherwise question as "too conservative."

2. **Concrete regression fixtures.** The anti-pattern papers give us the exact shapes
   of overfit "winners" (`evolution[4][20][96]`). We can encode synthetic analogues as
   standing tests that the gate *must* refuse — turning a literature finding into a
   permanent guard (complements SYNTHESIS P1 item 12, "validate the gate on synthetic
   no-alpha series").

3. **A safe upgrade path if ever needed.** If the operator one day wants automated
   search, we are not starting from zero — the honesty kit (`evolution[5][14][10][92]`)
   is a ready blueprint, and our typed composed DSL (`crates/strategy/src/composed/`)
   is already a sane representation for evolved logic.

4. **A presentation idea that doesn't compromise the gate.** The quality-diversity
   *menu* (`evolution[33][69]`) is a way to show users a spectrum of strategies
   (binned by turnover/exposure) without weakening the verdict — every cell still
   clears the gate. Optional, P2.

5. **Sharper intuition for crypto specifically.** The DSR worked example proves fat
   tails *shrink* the survivable trial budget (Normal: 88 trials; skew−3/kurt10: only
   46) (`evolution[98]`). On heavy-tailed coins we should be *more* suspicious of large
   sweeps — the opposite of the naive intuition. This directly informs how aggressively
   we let any search (including our own grid) expand.

---

## 5. Problems and challenges

**The footgun framing — name it loudly.** An automated-search arm on a ship-passive
product is the most dangerous single idea in the program. Risks:

1. **It manufactures false confidence.** A search WILL surface an in-sample champion
   on any window. Without the full guard stack, a flashy evolved curve could leak into
   narration or the leaderboard and contradict the product's core honest claim. The
   in-sample winner is *negatively* predictive OOS (`evolution[29]`) — so the prettier
   the backtest, the worse the prior on it being real.

2. **Guards are easy to get subtly wrong.** "OOS" data reuse, a too-short window for
   the trial count (below MinBTL), an ill-conditioned `N_eff` when configs > bars — any
   of these silently re-inflates the result. The sibling doc's MinBTL/DSR/N_eff caveats
   are not optional decorations; they are the difference between a gate and a
   rubber stamp.

3. **Compute efficiency is a trap.** Every faster search (Bayesian/TPE, surrogate
   MOEA, 500×-faster TensorNEAT, warm-start GP) buys *more configs per unit compute* =
   *more multiple testing*, reaching the over-fit optimum faster, not finding real edge
   (`evolution[13][33][34][38][56][59]`). If we ever add a faster search, the
   significance correction MUST scale *with* the budget.

4. **The evaluator has no ground truth.** FunSearch works because a math verifier is
   correct and cheap (`evolution[18]`). Our backtest is noisy, finite, and gameable.
   The robustness gate is our (imperfect) stand-in; markets offer no real verifier.

**HARD CONSTRAINTS this work would bump against (name them now):**

- **USDT-denominated, `Decimal` not `f64`.** Any evolved-strategy equity/PnL must
  stay in `Decimal` (the gate's drawdown/equity math is `Decimal`); fitness scoring in
  `f64` is acceptable only behind the same boundary the existing metrics use.
- **`ui` must NOT depend on `strategy`/`exec`/`llm`/`models`.** A search engine lives
  in `backtest`/`strategy`, never wired so that the UI gains a dependency on it. A
  quality-diversity *menu* must be fed to `ui` as plain data (the leaderboard runner
  pattern), not by importing strategy types into `ui`.
- **Overlays ship a day-1 baseline-equity-divergence e2e.** If any evolved output acts
  as an overlay/sizing-modifier, it inherits the v3-vol-overlay-noop precedent: an e2e
  asserting its equity diverges from the un-targeted baseline by ≥ a testable epsilon.
- **Anchored report SHAs are byte-immutable (119/119).** Nothing here may edit an
  anchored report; new diagnostics emit *new* report sections/files.
- **Gate / bands are FROZEN (additive only).** An evolved elite is judged by the
  existing `RobustnessFlag` classifier and `verdict_bands`
  (`crates/backtest/src/bakeoff/robustness.rs`); we do not relax FRAGILE to admit a
  pretty evolved curve. No new crown path.
- **Paper-only.** Nothing here approaches live execution.
- **Pre-registration is the standing defense.** An open-ended search is the *opposite*
  of pre-registration. Reconciling the two means: pre-declare the search space, the
  budget N, the once-only OOS window, and the DSR/MinBTL threshold *before* running —
  i.e. the search itself becomes a pre-registered protocol, not a free hunt.

5. **Cross-sectional methods are inapplicable.** A large slice of the literature
   (portfolio solvers `evolution[16][17]`, alpha-factor collections `evolution[3]`,
   long-short neuroevolution `evolution[63]`) solves problems that *vanish* for one
   coin. Importing them wastes effort and risks smuggling in a structural lever we
   can't actually use.

---

## 6. Concrete next steps / candidate work items

Named, located, prioritized. **None of these is "build a search engine"** — the P0/P1
items harden the search we already run; the search engine itself is an opt-in P2 that
the research says will likely return null.

- **[P0] Significance-charge the existing grid-sweep** — *this is the sibling doc's
  job*, listed here for the cross-reference. The grid-sweep in
  `crates/backtest/src/bakeoff/sweep.rs` + crown logic in `rank.rs` are a search; add
  DSR / MinBTL / PBO / `N_eff` as additive diagnostics in
  `crates/backtest/src/bakeoff/{robustness.rs,rank.rs}`. See
  `application-anti-overfitting-and-search-discipline.md` §6. **Highest value.**

- **[P1] Anti-pattern regression fixtures.** Encode synthetic analogues of the
  cost-free flashy-curve anti-patterns (`evolution[4][20][96]`) and assert the gate
  refuses a crown (FRAGILE or sub-DSR). Location: a new test under
  `crates/backtest/tests/` (alongside `robustness_bootstrap_bites.rs`,
  `short_bakeoff_bear_bull.rs`). Cheap; turns a literature finding into a permanent
  guard. Complements SYNTHESIS P1 "no-alpha-gate" test.

- **[P1] Matched-activity random null as a gate sub-test.** Pit each crowned candidate
  against a random-trading null matched on **trade frequency AND time-in-market**, and
  the search-vs-search comparison against ~N random configs (`evolution[5][30][31]`).
  Location: additive in `crates/backtest/src/bakeoff/robustness.rs`. (Detailed in the
  sibling doc; flagged here because it is the cheapest detector of "looks good vs B&H
  purely from lucky timing.")

- **[P2, opt-in, expected-null] Pre-registered evolutionary search arm.** Only if the
  operator explicitly requests it. A GP/GA loop over our existing strategy primitives,
  emitting candidates as typed composed-strategy DSL
  (`crates/strategy/src/composed/`), pre-declaring `{search space, budget N, once-only
  OOS window, DSR/MinBTL threshold}`, with cost-in-fitness, difficulty-weighted
  subset rotation, IS→OOS-vs-theory, and the matched random null — then every elite
  through the FROZEN gate (`evolution[5][10][14][92]`). Location: a new module under
  `crates/backtest/src/` (or `crates/strategy/src/`), off by default. **Expect it to
  crown nothing; the deliverable is the disciplined protocol + the truthful verdict.**

- **[P2, optional] Quality-diversity strategy menu.** A MAP-Elites archive over
  behavior descriptors (turnover/exposure/risk-profile) presented as a user *menu*
  (`evolution[33][69]`). Every cell must clear the FROZEN gate. Fed to `ui` as plain
  data — no `ui → strategy` dependency. Location: `crates/backtest/src/` producing a
  serializable archive consumed by the leaderboard runner.

---

## 7. Open questions for analyst & architect

1. **Scope decision:** does the operator ever want an automated-search arm at all, or
   is the durable position "FIXED pre-registered slates forever, and this literature is
   the documented reason"? The research supports the latter as the honest default.
2. **If yes to search:** is the deliverable the *protocol + null verdict* (consistent
   with "measured honesty"), or an expectation of finding a winner (which the research
   says is unrealistic on a single coin)? These imply very different success criteria.
3. **Representation:** is the typed composed-strategy DSL
   (`crates/strategy/src/composed/`) expressive enough to be a GP genotype, or would a
   search need primitives it doesn't yet have — and would adding them quietly expand
   the pre-registered space?
4. **Budget accounting:** how do we count N for the *combined* search (existing grid
   sweep + any evolutionary arm) so the DSR/MinBTL bar reflects the *total* trials, not
   just the last stage? (See SYNTHESIS P0 `N_eff` and the cluster-first-when-M>T caveat.)
5. **Menu vs single crown:** is a quality-diversity menu a feature the advisor wants
   (it complicates the "one crowned pick" story), or does it dilute the honest "B&H
   usually wins" message?

---

## 8. What NOT to do / out of scope

- **Do NOT ship an always-on automated alpha miner.** It is industrialized
  data-snooping on a product whose entire value is honesty (`evolution[1][9][21][29]`).
- **Do NOT import cross-sectional machinery** — portfolio solvers, alpha-factor
  collections, long-short neuroevolution (`evolution[3][16][17][63]`). The problems
  they solve do not exist for one coin, and they smuggle in structural levers we lack.
- **Do NOT chase faster search as if it helps.** Faster = more multiple testing, not
  more edge (`evolution[13][33][34][38]`). Efficiency without a budget-scaled
  correction just buys faster overfitting.
- **Do NOT adopt risk-seeking / best-case-tail objectives** (`evolution[12][24]`). On
  noisy data with no ground truth, optimizing best-case = chasing lucky tails = overfit
  by construction (degrades past α>0.85). The gate rewards robust central tendency vs
  B&H, never best-case.
- **Do NOT treat IC / accuracy / a single-window backtest Sharpe as a verdict**
  (`evolution[2][6][8][11][12][87]`). Only cost-net equity-vs-B&H over a path
  distribution counts.
- **Do NOT relax the FROZEN gate to admit an evolved curve.** No special crown path.
  An evolved elite earns its crown by the same `RobustnessFlag` + `verdict_bands` rules
  as a hand-written strategy, or not at all.
