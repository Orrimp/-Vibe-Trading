# Application — Factor Replication & the Honest Counter-Thesis

> Decision-oriented brief for analyst + architect. Derived from
> `research/strategies/knowledge.md` and the ledger
> `research/strategies/papers.md` (cited `strategies[N]`). No new papers.
>
> **Our app:** a Rust **single-coin crypto investment advisor** — paper/sim only,
> NOT advice, NOT live. Bake off every strategy → rank under a FROZEN
> bootstrap gate (FRAGILE ⇒ can't crown; buy-and-hold always benchmark + exempt)
> → forward plan → watch it paper-trade. Thesis: **no active strategy robustly
> beats buy-and-hold net of costs.** Sells **measured honesty** — "a framework
> for trading with traceable and plausible trading."

Short, but load-bearing. The other two files lean on "the literature confirms
ship-passive." This file is the **honest steel-man**: a handful of rigorous
papers find edges that *do* survive — and engaging them is what keeps the
product's honesty *credible* rather than dogmatic. The conclusion does not change
(none of the survivors is "a single textbook rule beats buy-and-hold on BTC/ETH
on raw terminal wealth net of costs"), but the *framing* must: **"TA rarely
robustly beats hold, and we TEST it," not "TA never works."**

---

## 1. Summary of the research

**The counter-thesis — edges that survive proper correction:**

- **Jensen–Kelly–Pedersen** `strategies[88]`: ~**75–85% of equity factors
  replicate** under proper Bayesian multiple-testing — a direct steel-man against
  a blanket "everything is data-snooping." **But the survivors are
  cross-sectional themes** (value, momentum, quality, low-risk) that require a
  *universe* to rank — **none is harvestable on one coin.**
- **Deprez–Frömmel** `strategies[95]`: 75,360 simple rules on Bitcoin,
  cost-aware + multiple-testing-corrected — and simple TA **CAN** beat Bitcoin
  buy-and-hold **out-of-sample on a RISK-ADJUSTED (Sharpe/alpha) basis after costs
  + data-mining correction.** This is the credible counter-thesis on our own
  asset. **Caveats that keep it honest:** it is a *portfolio* of selected rules
  (not one config), and **risk-adjusted ≠ more terminal wealth** — a long-term
  holder may still prefer hold.
- **The counterweight, on the most-liquid crypto:** Hudson–Urquhart
  `strategies[93]` — on Bitcoin specifically, the in-sample-best rules go
  **negative out-of-sample** (alts stay positive), because BTC is the most liquid
  / most-arbitraged.

**What the crypto-factor literature actually supports (and what it doesn't):**

- **Crypto factors reduce to ~market / size / momentum, and even those decay
  OOS** `strategies[41][12]`: the size effect disappears out-of-sample, value and
  quality are weak. Among directional families, **time-series momentum is the most
  academically supported** for crypto `strategies[12]`.
- **Crypto has no clean quality/value factor** `strategies[76]` — Quality-Minus-
  Junk has no clean crypto analogue; on-chain "fundamentals" are the only
  candidate and are thin.
- **The one single-coin "beats buy-hold with a clean control" claim is on-chain
  *valuation*, not TA** `strategies[40]`: MVRV-Z / NUPL / CVDD beat buy-hold and a
  random-entry control — **but it rests on only ~3 cycles** (effective n≈3), so it
  must be deflated heavily and checked for realized-value lookahead. The related
  on-chain *flow* signal (USDT exchange inflows) is real but **weak, short-
  horizon, and reverse-causality-prone** `strategies[77]` (corr < 0.3 at 1–2h).
- **Structural edges decay less than predictive ones** `strategies[24][18][25]`:
  betting-against-beta exists because investors are leverage-constrained; carry
  because of frictions — "who-can-do-what" premia, not forecasts. The crypto
  analogue (funding/basis carry) is the highest-Sharpe crypto edge in the
  literature `strategies[9]` but is **market-neutral, non-predictive, needs a
  perp+margin+funding engine, and has itself decayed toward negative in real
  time** (Sharpe 6.45 → 4.06 → negative; see the crypto-market-structure topic).
- **Negative-skew carry is a risk, not free yield** `strategies[23]`: short-vol /
  variance-risk-premium strategies earn high Sharpe with extreme negative skew —
  rank on left-tail / skew, not just mean Sharpe.

**Net synthesis (from `knowledge.md`):** the surviving cases are (i)
cross-sectional / multi-asset, or (ii) less-liquid alts, or (iii)
risk-adjusted-not-raw — **none is "a single textbook rule beats buy-and-hold on
BTC/ETH on raw terminal wealth net of costs,"** which is the exact bar our
advisor sets.

---

## 2. Possible solutions / what can be done with this research

1. **Adopt risk-adjusted vs terminal-wealth as an explicit, surfaced
   distinction.** Deprez–Frömmel's edge is *risk-adjusted* `strategies[95]`; the
   gate already reports Sharpe and drawdown distributions. Make "risk-adjusted win
   ≠ more money" a first-class line in the verdict so the advisor can honestly say
   "this rule had a better Sharpe but not more terminal wealth, and you're a
   holder."
2. **Calibrate the gate's claim language to the steel-man.** Replace any "TA does
   not work" copy with "we test whether a rule robustly beats hold on THIS coin/
   window; usually it doesn't." This is a presenter/UI-copy change grounded in
   `strategies[88][95]`.
3. **Keep cross-sectional factors explicitly out of scope — and say why.**
   `strategies[88]`'s survivors need a universe; flag them as the rationale for a
   *future multi-coin* mode, not a single-coin candidate.
4. **Treat on-chain valuation (MVRV-Z/NUPL/CVDD) as the top data-driven feature
   to evaluate — gated hard.** `strategies[40]`: clean random-entry control, but
   n≈3 cycles → deflate heavily (this is exactly where the selection-bias gate
   from the first file does the work), check for realized-value lookahead, needs an
   on-chain feed.
5. **Rank on skew / left-tail, not just mean Sharpe** `strategies[23]`, so any
   negative-skew carry-like candidate is penalized by the weakest-link gate.
6. **Flag funding/basis carry as the top *future structural* edge**
   `strategies[9]`, explicitly scoped as needing a perp+margin+funding engine and
   stress-tested (it has decayed in real time), not sold as free yield.

---

## 3. Relevance for the project

- **It sets the honesty calibration of the whole product.** The product's value
  is "measured honesty." Over-claiming "TA never works" is *dishonest* given
  `strategies[88][95]`; the credible claim is conditional ("rarely, robustly, net,
  on THIS coin"). This file is where that calibration is sourced.
- **It makes the usual "hold" verdict more believable.** A gate that *can* in
  principle crown a survivor (and occasionally would, per `strategies[95]`) is more
  trustworthy than one rigged to always say "hold." The counter-thesis is what
  makes ship-passive a *finding* rather than a foregone conclusion.
- **It defines the legitimate non-TA experiments.** The honest survivors point at
  exactly two single-coin-relevant arms to gate: **on-chain valuation**
  `strategies[40]` (a "value" signal distinct from price TA) and **funding/basis
  carry** `strategies[9]` (structural, market-neutral). Both are clearly future /
  feed-dependent, and both must clear the same FROZEN gate.
- **It bounds expectations honestly.** Even the survivors are risk-adjusted
  (Deprez–Frömmel) or thin (MVRV n≈3) or decaying (carry) — so the realistic
  outcome remains "recommend hold," but for *stated, cited* reasons rather than
  blanket skepticism.

---

## 4. Advantages for the project

- **Credibility is the moat.** Engaging the best counter-evidence and *still*
  arriving at ship-passive is far more persuasive to a skeptical operator than
  ignoring it. This file is the "we did our homework" exhibit.
- **It pre-empts the obvious objection.** "But paper X says crypto TA works" is
  answered in advance: yes (`strategies[95]`) on a risk-adjusted basis with a
  portfolio of rules — and our gate reports risk-adjusted vs terminal-wealth so a
  holder can judge.
- **It gives a principled feature roadmap beyond price-TA.** On-chain valuation
  and carry are the two literature-blessed, gate-worthy directions — a far better
  use of future effort than another price-TA rule (which `strategies[100]` shows
  is the same data-mined family).
- **Zero code cost for the highest-value part.** The honesty-calibration and the
  risk-adjusted-vs-terminal-wealth distinction are mostly *copy + an existing
  reported metric*, not new subsystems.

---

## 5. Problems and challenges

- **HARD CONSTRAINT — gate/bands FROZEN.** Surfacing "risk-adjusted vs
  terminal-wealth" must read *existing* reported distributions (Sharpe, total
  return, drawdown — already in `DistributionSummary`), not add a new band or
  alter the weakest-link classifier in
  `crates/backtest/src/bakeoff/robustness.rs`.
- **HARD CONSTRAINT — `ui` must NOT depend on strategy/exec/llm/models.** Any
  copy change or new verdict line in the cockpit/leaderboard must consume data via
  the existing permitted seam, not by adding a UI→strategy/backtest dependency.
- **HARD CONSTRAINT — anchored report SHAs byte-immutable (119/119).** Adding a
  "risk-adjusted ≠ terminal-wealth" line to an anchored ranking report breaks its
  body-SHA. Surface it in a non-anchored artifact / UI, or under the additive
  `write_report = false` precedent.
- **The counter-thesis is genuinely contestable — don't over-correct.**
  Deprez–Frömmel `strategies[95]` is risk-adjusted, portfolio-of-rules, and is
  *contradicted on BTC specifically* by Hudson–Urquhart `strategies[93]`. The
  framing must hold both: "rarely robustly beats hold, must be tested" — not "TA
  works after all."
- **On-chain valuation has a thin-sample trap and a feed dependency.** n≈3 cycles
  `strategies[40]` is below most honest gate thresholds (and the selection-bias
  file's MinBTL veto would likely refuse to crown it); realized-value metrics risk
  lookahead. It needs an on-chain data feed we don't yet have.
- **Carry is out of current scope.** Funding/basis carry `strategies[9]` needs a
  perp + margin + funding-rate + short engine — a large, future build — and has
  already decayed toward negative; it is a "future structural edge," not a
  near-term candidate.

---

## 6. Concrete next steps / candidate work items

- **[P0] `honesty-claim-calibration`** — analyst/UI-copy work item: ensure all
  advisor-facing language is "we test whether a rule robustly beats hold on THIS
  coin/window; usually it doesn't," never "TA never works." Cheap, high-value,
  directly on the "measured honesty" goal. Citation: `strategies[88][95]`.
- **[P1] `risk-adjusted-vs-terminal-wealth-surface`** — surface, in the verdict,
  when an arm wins on Sharpe but NOT on terminal wealth (reads existing
  `DistributionSummary` fields; non-anchored / additive surface only). Citation:
  `strategies[95][35]`.
- **[P1] `rank-on-skew-leftquantile`** — ensure the gate penalizes high-Sharpe /
  strongly-negative-skew candidates (the carry/short-vol profile) via reported
  left-tail quantiles + skewness alongside Sharpe; additive to the scorecard, not
  the FROZEN bands. Citation: `strategies[23]`.
- **[P2] `onchain-valuation-feature-spike`** — research spike (not shipped):
  evaluate MVRV-Z/NUPL/CVDD as a gated long/flat signal with a random-entry
  control, behind the selection-bias gate (expect MinBTL to refuse to crown on
  n≈3). Needs an on-chain feed; deflate heavily; check realized-value lookahead.
  Citation: `strategies[40][77]`.
- **[P2] `funding-carry-scoping-note`** — architect scoping note for a future
  perp+margin+funding engine (ADR-0051 territory): the highest-Sharpe crypto edge,
  market-neutral, but decayed and out of current long/flat-spot scope; stress-test,
  never sell as free yield. Citation: `strategies[9]`.

---

## 7. Open questions for analyst & architect

- **How loud should the counter-thesis be in the UI?** Enough to be honest
  ("a rule can occasionally win risk-adjusted"), without inviting a retail user to
  over-trade. Where is the line?
- **Does the advisor recommend the risk-adjusted winner to a risk-averse user?**
  Deprez–Frömmel `strategies[95]` says a risk-adjusted edge can exist. If the gate
  crowns one on risk-adjusted terms but it has lower terminal wealth, what does the
  advisor *recommend* — and does that depend on a user risk-preference input we
  don't currently collect?
- **Is the multi-coin mode worth scoping now?** `strategies[88]`'s survivors are
  cross-sectional; the diversification-return argument `strategies[35]` says
  multi-coin rebalancing is the one place a structural edge over single-coin hold
  plausibly exists. Is that the natural next product, or out of scope?
- **On-chain feed: build vs defer?** MVRV `strategies[40]` is the top data-driven
  feature but is thin (n≈3) and feed-dependent. Is the feed integration worth it
  given the gate will likely refuse to crown on such a short sample?
- **Carry engine timing.** Funding carry `strategies[9]` is the biggest crypto
  edge but the largest build and is decaying — is there *any* near-term version
  (e.g. a read-only carry diagnostic) worth shipping before the full perp engine?

---

## 8. What NOT to do

- **Do NOT claim "TA never works."** It is contradicted by `strategies[88][95]`
  and undermines the product's credibility. Claim conditionally.
- **Do NOT add cross-sectional factor arms to the single-coin bake-off.** They
  need a universe `strategies[88]`; they belong to a future multi-coin mode.
- **Do NOT crown on-chain valuation off n≈3 cycles** `strategies[40]`. Let the
  selection-bias gate (MinBTL/DSR from the first file) refuse it honestly.
- **Do NOT promise funding carry as free yield** `strategies[9][23]`. It is
  negative-skew, decaying, and out of current scope.
- **Effort / blast radius:** the honesty-calibration + risk-adjusted-vs-terminal-
  wealth surface are near-zero-code (copy + existing metric). On-chain and carry
  are large, feed-/engine-dependent, and explicitly future. The highest value here
  is *framing*, which is cheap.
