# Application — TA Efficacy & the Selection-Bias Gate (strategy angle)

> Decision-oriented brief for analyst + architect. Derived from
> `research/strategies/knowledge.md` (synthesis) and the 100-entry ledger
> `research/strategies/papers.md` (cited `strategies[N]`). No new papers; this
> distils what is already logged into "what we do with it."
>
> **Our app (ground every claim against this):** a Rust **single-coin crypto
> investment advisor** — paper/sim only, NOT advice, NOT live. Journey: pick ONE
> coin + budget → **bake off EVERY strategy** on a (coin, window) → **rank** under
> a FROZEN robustness gate (1000-path moving-block bootstrap; weakest-link
> verdict; FRAGILE ⇒ cannot be crowned; **buy-and-hold is always the benchmark**
> and is crown-exempt) → forward rule-based plan → **watch it paper-trade**.
> Validated thesis: **no active strategy robustly beats buy-and-hold net of
> costs** — confirmed even on BTC, where the in-sample-best rule goes negative
> out-of-sample. The product sells **measured honesty** — operator goal: "a
> framework for trading with traceable and plausible trading."

This is the file that turns our thesis from an opinion into a **citable, 30-year
empirical result**, and turns "deflate the winner" from folklore into an
**exact, codeable gate**. The TA-efficacy literature mostly *confirms*
ship-passive. That is precisely its value: it tells us which rule families NOT to
chase, and it hands the presenter quotable numbers for "why does the advisor keep
recommending hold?"

---

## 1. Summary of the research

**The TA-efficacy arc — a 30-year sequence from "TA works" to "no, it was
data-snooping + costs," now readable end-to-end with full-text numbers**
(`strategies[85][82][83][84][89][93]`). This is the through-line of our entire
project:

- **Brock–Lakonishok–LeBaron 1992 (the "before")** `strategies[85]`: 26 rules
  (VMA/FMA/TRB) on DJIA 1897–1986; the moving-average **buy-sell spread is
  +0.067%/day**, the double-on-buy/cash-on-sell rule beats holding by **~3.4%/yr
  GROSS**, and returns are inconsistent with four nulls (RW, AR(1), GARCH-M,
  EGARCH). But it is **gross and ex-post-rule-selected** — the two gaps the rest
  of the arc closes.
- **Sullivan–Timmermann–White 1999 ("not after a snooping correction")**
  `strategies[82]`: the same rules expanded to **7,846**; White's Reality Check.
  The best full-universe rule (5-day MA) earns **17.2%/yr in-sample vs 4.3% B&H**
  — but the recursive **ex-ante** best-rule-to-date trader gets only **14.9%**
  (you cannot pick the winner forward), a **one-day execution delay collapses it
  to Sharpe 0.34 / p=0.26** (not significant), the **1987–1996 OOS** best rule is
  insignificant, and **S&P futures show nothing.**
- **Bajgrowicz–Scaillet 2012 ("and costs erase it, and it's not selectable")**
  `strategies[83]`: same 7,846 rules, DJIA→2011, FDR method, **Politis–Romano
  stationary bootstrap, block 10, B=1000 — our exact bootstrap design**. One-way
  costs of just **16–70 bps** zero the edge in the only era it ever existed
  (1897–1962); **post-1962 nothing works even at ZERO cost**; the
  monthly-rebalanced FDR portfolio is **negative OOS even free**; and **<5% of
  selected rules survive one rebalancing** (pure ranking noise).
- **Park–Irwin (the referee)** `strategies[84]`: surveys **92 modern studies (58
  positive / 24 negative / 10 mixed)**, and attributes the positive majority to
  **four biases — data-snooping, ex-post rule selection, risk-estimation
  difficulty, transaction-cost-estimation difficulty** — the exact four our gate
  neutralizes.
- **Marshall–Cahan–Cahan (breadth)** `strategies[89]`: 5,806 rules across **49
  MSCI markets**; nominal-significant in **16 of 23 developed markets**, but
  **ZERO markets** survive the snooping correction (Singapore best rule p=0.05 →
  0.802; Hong Kong significant → insignificant after just **6** added rules).
- **Hudson–Urquhart (the crypto test — closest to our product)**
  `strategies[93]`: 14,919 rules on BTC/LTC/XRP/ETH 2010–2017, OOS = H1-2018. TA
  *survives* the snooping correction **in-sample** on crypto (33.6% of BTC rules
  still significant) and clears realistic costs **in-sample** — but the best
  in-sample **Bitcoin** rule (channel-breakout) goes **NEGATIVE out-of-sample
  (Sharpe −0.050)**, while the less-liquid alts stay positive, because BTC is the
  most liquid / most-arbitraged. *This is our thesis, on our asset class, in one
  paper.*

**The selection-bias correction is a named, mature methodology — and the
full-text reads give us the exact formulas, thresholds, and magnitudes to code**
(`strategies[97][32][98][42]`):

- **Harvey–Liu Sharpe haircut** `strategies[97]`: SR→t-ratio, adjust the p-value
  for N tests via **Bonferroni / Holm / BHY**, back-transform to a haircut SR.
  The haircut is **strongly NON-LINEAR** — for annualized SR < 0.4 it is almost
  always **>50%** (near-total for marginal Sharpes), but for SR > 1.0 it is
  **≤25%**. So the folk "halve the Sharpe" rule is wrong both ways. They
  **recommend BHY (FDR)** for finance and stress feeding in the **average
  cross-strategy correlation** (it cuts the effective N). Multiplicity bites
  fast: **N=10 → ~40% chance of a spurious t≥2.**
- **Deflated Sharpe Ratio** `strategies[32]`:
  `DSR = Z[(ŜR − SR₀)·√(T−1) / √(1 − γ̂₃·ŜR + ((γ̂₄−1)/4)·ŜR²)]`, with the
  benchmark `SR₀ = √V[{ŜRₙ}]·((1−γ)·Z⁻¹[1−1/N] + γ·Z⁻¹[1−1/(N·e)])`, γ≈0.5772.
  It deflates for **five** inputs beyond mean/vol: skew γ̂₃, kurtosis γ̂₄, track
  length T, **cross-config Sharpe dispersion V[{ŜRₙ}]**, and N. Worked example:
  SR=2.5 / 5y daily / N=100 / skew −3 / kurt 10 → **DSR≈0.90 < 0.95 → REJECT**;
  fat tails dropped the survivable N from 88 (Normal) to **46**. Effective-N for
  correlated trials: **N̄ = ρ̂ + (1−ρ̂)·M**.
- **Probability of Backtest Overfitting via CSCV** `strategies[98]`: from the
  same **T×N configs-×-time P&L matrix**, split rows into S blocks, form all
  C(S,S/2) train/test combinations (S=16 → 12,780), find the IS-best config per
  split, take its relative OOS rank ω, logit λ=ln(ω/(1−ω)), and
  **PBO = ∫_{−∞}^0 f(λ) dλ**. The same run yields performance-degradation slope
  (β<0 ⇒ overfit), probability-of-loss, and stochastic-dominance-vs-random for
  free. Its deepest law: **more in-sample optimization is often *negatively*
  related to OOS performance.**
- **The t > 3 hurdle** `strategies[42]`: the multiple-testing literature raised
  the significance bar above the textbook t > 2 because most published factors
  are false positives. This is the t-space partner of the DSR.

**Supporting decay laws.** Published anomalies lose **~26% OOS + ~58%
post-publication** `strategies[92]` (the synthesis notes the corrected full-text
figure is ~10% statistical + ~35% crowding, and that the decay is **largest for
the cheapest-to-arbitrage, low-idiosyncratic-risk names = BTC/ETH/SOL**);
stat-arb roughly halved post-2010 `strategies[16]`; the MA "zoo" collapses to one
data-mineable family `strategies[100]` (every MA indicator is a weighted average
of price changes → sweeping SMA/EMA/MACD is **low-effective-N** multiple
testing).

---

## 2. Possible solutions / what can be done with this research

1. **Add a selection-bias scorecard to the bake-off** that consumes the T×N
   return matrix we already build and reports, per run: effective trial count
   `N_eff`, Deflated Sharpe Ratio `DSR`, Probability of Backtest Overfitting
   `PBO`, and a `MinBTL` pre-flight veto. This is the single largest durable
   addition the literature licenses.
2. **Make the crown threshold honest by construction.** Crown an active arm only
   if it beats B&H *and* clears `DSR ≥ 0.95` (and is not flagged by PBO). Because
   the haircut is non-linear and our single-coin net Sharpes are realistically
   < 0.4, this gate **crowns almost nothing by construction** `strategies[97]` —
   exactly the thesis, now enforced rather than asserted.
3. **Treat the MA menu as ONE family for deflation, not many** `strategies[100]`.
   Use `N̄ = ρ̂ + (1−ρ̂)·M` (or cluster the config return series) so our
   near-identical SMA/EMA/MACD/RSI/Bollinger configs do not over-count N — and so
   Bonferroni does not over-deflate.
4. **Wire the arc into the presenter narrative.** The six-paper sequence is now a
   single quotable paragraph (`strategies[85→82→83][84][89][93]`) answering
   "doesn't everyone say technical analysis works?" — culminating in
   Hudson–Urquhart's negative-OOS Bitcoin result.
5. **Build a standing no-alpha regression test.** Feed the gate synthetic
   no-edge series (GARCH/OU); it must refuse to crown, and DSR/PBO must flag a
   deliberately overfit pick. This proves the gate is doing its job.
6. **Replicate Hudson–Urquhart on our own pipeline** `strategies[93]`: confirm
   the in-sample-best rule family per coin goes sub-B&H out-of-sample, and that
   the best family *differs by coin* (a snooping tell).

---

## 3. Relevance for the project

This file is the **external validation of our core thesis and our gate**, and the
source of the **P0 durable upgrade**.

- **Validates ship-passive on our asset class.** Hudson–Urquhart `strategies[93]`
  is the closest published analogue to our product (single-coin crypto, ~15,000
  rules, snooping-corrected, OOS window) and reaches our exact conclusion on BTC.
  We can stop arguing the thesis and start *citing* it.
- **Names the missing piece of the gate.** Our 1000-path moving-block bootstrap
  tests each *individual* curve's robustness, but does **not** correct for the
  multiple-testing bias of crowning the **best of N** swept strategies. DSR/PBO
  is exactly that correction, and `strategies[83]` confirms our bootstrap design
  (stationary bootstrap, block 10, B=1000) is the right family — we are extending
  a method the literature already endorses.
- **Directly serves "traceable & plausible."** A per-run scorecard (N_eff, DSR,
  PBO, MinBTL pass/fail printed next to the verdict) makes the honesty
  *auditable*: an operator can see *why* a high-Sharpe config was not crowned.
  That is the product.
- **Honest on expected-null.** This research predicts the gate will almost always
  output "all active arms FRAGILE → recommend buy-and-hold." That is not a
  failure — it is the validated, defensible output. The literature even quantifies
  *why*: post-1962 nothing survived on DJIA even free `strategies[83]`; ZERO of 49
  markets survived snooping `strategies[89]`; BTC went negative OOS
  `strategies[93]`.
- **Crypto kicker raises the bar, not lowers it.** Fat tails *shrink* the
  survivable trial budget (ŜR=2.5 clears at N=88 Normal but only N=46 at
  skew−3/kurt10 `strategies[32]`), and the cheapest-to-arbitrage names (BTC/ETH/
  SOL) decay most `strategies[92]`. Our coins are exactly the hard case — more
  suspicion of large sweeps is warranted, not less.

---

## 4. Advantages for the project

- **Turns a claim into a moat.** "No active strategy robustly beats buy-and-hold,
  net of costs" stops being a vibe and becomes a result with six citations and
  exact magnitudes. The frozen gate + benchmark is a **competitive advantage**, not
  a limitation.
- **The inputs already exist.** The per-strategy return matrix, N, and per-config
  Sharpes are already produced by the bake-off. DSR/PBO/MinBTL/N_eff are
  **additive computations on stored data** — no new data pipeline, no change to
  the FROZEN classifier bands.
- **Cheap, high-leverage, low blast radius.** MinBTL is a one-line pre-flight
  veto (`MinBTL ≈ 2·ln(N)/SR²_target` years → refuse to crown when the window is
  shorter); the DSR is a closed-form transform; PBO/CSCV is a deterministic
  combinatorial loop over the matrix. All are pure functions, unit-testable at
  boundaries like the existing `classify_verdict`.
- **Self-documenting honesty.** The scorecard is presenter-ready output that
  *demonstrates* measured honesty rather than asserting it — directly on the
  operator's stated goal.
- **A built-in falsification test.** If the gate ever *does* crown a non-FRAGILE
  active arm with `DSR ≥ 0.95` and `PBO` low, Deprez–Frömmel `strategies[95]`
  (next file) says that is not impossible — so the gate is not rigged to always
  say "hold," which makes the usual "hold" verdict more credible.

---

## 5. Problems and challenges

- **HARD CONSTRAINT — the gate/bands are FROZEN; additions must be ADDITIVE.**
  The 5-signal weakest-link bands live in
  `crates/backtest/src/bakeoff/robustness.rs::verdict_bands` and are operator-
  locked. DSR/PBO/MinBTL/N_eff must be a **new, parallel scorecard + a crown
  gate**, never an edit to `classify_verdict` or the band constants. The eligible-
  to-crown rule (FRAGILE ⇒ ineligible; benchmark exempt) in
  `crates/backtest/src/bakeoff/rank.rs::rank_candidates` can be *tightened* by an
  additive "and DSR≥0.95" check, but the existing comparator order must be
  preserved.
- **HARD CONSTRAINT — anchored report SHAs are byte-immutable (119/119).** The
  bake-off ranking report is anchored. Adding scorecard lines to it mutates the
  body-SHA and breaks the gate. New diagnostics must go to a **new
  (`write_report = false`) artifact** or a new anchor, following the
  `default_macro_field` / ADR-0073 additive precedent (`write_report = false →
  anchor-additive`). Run `scripts/verify_anchors.sh` before and after any touch.
- **HARD CONSTRAINT — `N̄` estimation breaks exactly in our regime (M > T).**
  When there are more configs than window bars (our situation), the config-return
  correlation matrix is ill-conditioned and ρ̂ is itself overfit, so we **MUST
  dimension-reduce / cluster before estimating N_eff** (ONC clustering or PCA),
  per the SYNTHESIS P0 primary-source requirement. A naive ρ̂ would silently
  over-deflate or under-deflate.
- **HARD CONSTRAINT — Decimal, not f64, in library code.** Statistical thresholds
  are necessarily f64 (the existing `robustness.rs` carries
  `#![allow(clippy::float_arithmetic)]` for exactly this). Keep the f64 to an
  isolated statistics module; do **not** let it leak into the Decimal equity /
  P&L path.
- **Calibration is a judgement call, not a constant.** The literature's PBO
  reject bar is stricter than a "≳0.5 = coin-flip" reading; the t=3.0 cutoff was,
  per Harvey, "never intended" as universal `strategies[42]`. We should **report
  the deflated statistic, not a hard binary**, and derive the crown threshold from
  an explicit cost-asymmetry statement (the ORATIO odds-ratio idea in the
  backtesting topic) rather than hard-coding 0.95.
- **Risk of over-claiming "nothing works."** Two rigorous papers cut against a
  blanket null (`strategies[88][95]` — see the counter-thesis file). The honest
  framing is "TA *rarely robustly* beats hold net of costs, and we must TEST it,"
  not "TA never works."
- **The forward paper-trade alone is insufficient.** PBO/CSCV `strategies[98]`
  shows a single hold-out is high-variance and blind to trial count. Keep the
  forward paper-trade, but **pair** it with DSR/PBO on the bake-off matrix.

---

## 6. Concrete next steps / candidate work items

Named, with codebase location and priority. All ADDITIVE to the FROZEN gate.

- **[P0] `selection-bias-scorecard`** — new module
  `crates/backtest/src/bakeoff/overfit.rs` (sibling to `robustness.rs`/`rank.rs`),
  computing `N_eff`, `DSR`, `PBO`, `MinBTL` from the existing T×N return matrix +
  per-config Sharpes. Pure functions, unit-tested at boundaries (mirror
  `classify_verdict`'s test style). Surface a per-run **overfitting scorecard**
  next to the verdict via a `write_report = false` artifact (anchor-safe).
  Citations: `strategies[32][97][98][42]`.
- **[P0] `crown-gate-dsr`** — in
  `crates/backtest/src/bakeoff/rank.rs::rank_candidates`, add an **additive** crown
  guard: an active arm is crown-eligible only if (existing eligibility) **AND**
  `DSR ≥ threshold`. Preserve the existing comparator order and the
  benchmark-exempt rule exactly; emit a new reason code when a high-Sharpe arm is
  demoted by DSR. Threshold derived (not hard-coded) from a cost-asymmetry
  statement.
- **[P0] `n-eff-cluster-first`** — inside the scorecard, when `M > T`, cluster /
  PCA the config return series before estimating `N̄ = ρ̂ + (1−ρ̂)·M`. This is the
  primary-source requirement, not optional. Citation: `strategies[32]` + SYNTHESIS
  P0.
- **[P1] `minbtl-preflight-veto`** — cheapest item; compute
  `MinBTL ≈ 2·ln(N)/SR²_target` and refuse to crown when the (coin, window) is
  shorter than `MinBTL(N)`. One guard in `rank.rs`. Citation: `strategies[83]`,
  SYNTHESIS P0.7.
- **[P1] `no-alpha-gate-regression`** — new test
  `crates/backtest/tests/gate_refuses_no_alpha.rs`: feed GARCH/OU synthetic
  no-edge series; assert the gate does not crown and that a deliberately overfit
  pick is flagged by DSR/PBO. Standing regression. Citation: `strategies[98]`.
- **[P1] `hudson-urquhart-replication`** — analysis spike (not shipped code):
  run our bake-off on BTC/ETH with an explicit IS/OOS split and confirm the
  in-sample-best family goes sub-B&H OOS and differs by coin. Output is a
  presenter exhibit, not a feature. Citation: `strategies[93]`.
- **[P2] `presenter-ta-efficacy-spine`** — fold the six-paper arc paragraph into
  the presenter deck template as the canonical "why hold?" answer. Citation:
  `strategies[85→82→83][84][89][93]`.

---

## 7. Open questions for analyst & architect

- **Threshold derivation.** Do we hard-code `DSR ≥ 0.95` (literature default) or
  derive it from an explicit "a false 'beats-hold' is N× costlier than a miss"
  cost-asymmetry statement (ORATIO)? The latter is more defensible and more
  "traceable" but needs an operator decision on N.
- **PBO as gate vs diagnostic.** Should PBO be a hard crown disqualifier, or a
  reported diagnostic only (with DSR as the binding gate)? The literature reject
  bar and our "report don't binarize" instinct pull in different directions.
- **Where does the scorecard live so it stays anchor-safe?** A new
  `write_report = false` artifact, a new anchor in `spec/anchors.toml`, or a
  UI-only surface (the leaderboard already shows the verdict)? Note `ui` must NOT
  depend on strategy/exec/llm/models — the scorecard data must reach the UI via an
  already-permitted seam.
- **Clustering method for N_eff.** ONC vs PCA vs a simple correlation-threshold
  cluster — which is robust enough for our M>T regime without adding a heavy
  dependency? (The bake-off field is small — ~10 base + 8 ensemble arms — but
  parameter sweeps inflate M.)
- **Do we deflate the *composed-strategy DSL* crowns harder?** A tuned k-of-n
  vote combines signals; the backtesting topic gives a critical-t ladder
  (1-of-1 ≈ 2.1, 3-of-10 ≈ 4, 7-of-100 ≈ 7, k-signal combo ≈ √k·τ). Should the
  composed-strategy seam (`crates/strategy/src/composed/`) carry a heavier hurdle
  than a single base arm?

---

## 8. What NOT to do / effort & blast radius

- **Do NOT touch the FROZEN bands or the comparator order.** Everything here is a
  *parallel* computation + an *additive* crown guard. Editing
  `verdict_bands` or reordering `rank_candidates` is out of scope and breaks the
  contract.
- **Do NOT use Bonferroni on the raw config count.** Our SMA/EMA/MACD/RSI/
  Bollinger configs are near-identical; raw-M Bonferroni massively over-deflates.
  Use `N̄` (cluster-first when M>T) or BHY-with-correlation `strategies[97][100]`.
- **Do NOT edit anchored report files** to add scorecard lines. Use a new
  `write_report = false` artifact; verify with `scripts/verify_anchors.sh`.
- **Do NOT market the gate as "it finds you alpha."** Market it as "it tells you,
  honestly, when there is none" — which is almost always.
- **Effort / blast radius:** MinBTL veto = trivial (one function, one guard).
  DSR + N_eff scorecard = medium, isolated to a new module + a report artifact.
  PBO/CSCV = medium (combinatorial loop, deterministic, well-specified). Crown-
  gate-dsr = small but touches `rank.rs` (the highest-stakes file) — needs the
  existing rank tests green plus a new demotion-reason test. **No new external
  dependency** beyond a clustering helper if ONC is chosen.
