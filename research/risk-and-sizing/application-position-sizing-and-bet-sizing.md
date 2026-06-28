# Application — Position Sizing & Bet Sizing (Kelly, Fractional Kelly, Bet-Sizing, CVaR)

*Decision doc for analyst + architect. Distilled from `research/risk-and-sizing/`
(100 papers, deep-read pass 2026-06-28). Citations `risk-and-sizing[N]` resolve in
`research/risk-and-sizing/papers.md`; the synthesis is `knowledge.md`. This file does
not add papers — it turns the completed research into candidate work.*

> **Our app:** Rust single-coin crypto **advisor** (paper/sim, not advice, not live).
> Pick coin + €200 → bake off every strategy → rank under a FROZEN 1000-path
> moving-block-bootstrap gate (FRAGILE ⇒ can't crown; buy-and-hold always the benchmark
> + exempt) → forward rule-based plan → watch it paper-trade. Validated thesis: **no
> active strategy robustly beats buy-and-hold net of costs.** We ALREADY ship budget-aware
> **fixed-fraction** sizing (`crates/risk/src/sizing.rs`, `FixedFractionSizer`) with a
> HARD budget cap.

**Scope of this file:** *how much to hold* given a signal — Kelly, fractional Kelly,
bet-sizing-from-confidence, risk-of-ruin, distributionally-robust sizing, and CVaR/tail
metrics. The volatility-targeting and drawdown overlays (the *most* actionable risk-shaping
work) live in the sibling file `application-vol-targeting-and-drawdown-overlays.md`; sizing
and overlays meet at the fact that **vol targeting is just Kelly with μ held constant** [4].

**One-line verdict (be honest):** μ-driven sizing is **quantitatively hopeless on a
no-edge coin** — Kelly on a noisy μ̂ loses 27–48% of the oracle return and no cleverness
recovers more than ~1–3% [45]. The value here is **NOT** a sizing edge. It is (a) a
principled, conservative **bound** on how much to hold (shrink hard toward vol-only sizing),
(b) **risk-of-ruin / budget discipline**, and (c) **coherent tail metrics (CVaR)** that make
the advisor's risk numbers honest. "Size down, control risk" — never "size up for alpha."

---

## 1. Summary of the research

**Kelly is real, proven, and the wrong tool for us.** Breiman [100] rigorously proved
Kelly's two optimality results (asymptotic wealth dominance; minimal expected time to a
goal) — but both are **asymptotic and assume a known, favourable edge**, neither of which a
few-year single-coin window with no robust edge has. The deployable single-asset fraction is
**f ≈ μ/σ²** [4][80] (the exact log-normal form is *more* conservative than μ/σ² as vol
rises [4] — so naive μ/σ² *over-bets* crypto). Full Kelly is far too aggressive — huge bets,
deep drawdowns, finite-horizon ruin [3][6]; **it never pays to bet more than full Kelly**
(2× Kelly drives long-run growth to the risk-free rate) [6].

**Estimation error dominates, and it's brutally asymmetric.** Errors in the **mean** return
are **~20× as costly** as covariance errors and **~10×** as costly as variance errors [6].
Sizing rules that lean on μ̂ (Kelly) are fragile *exactly* where our single-coin μ̂ is
weakest; rules that lean only on σ/variance are far more forgiving. The "optimal" rule that
needs estimated means loses to naive 1/N out-of-sample, requiring ~3000–6000 months of data
to win [18]; the single-coin analogue of 1/N is buy-and-hold / a fixed fraction.

**No sizing cleverness rescues a bad μ̂ — now numeric.** The decisive deep-read result [45]:
standard Kelly on an estimated win-probability **loses 27–48% of the oracle terminal
return**; the best conservative-quantile/Monte-Carlo correction **recovers only ~1–3%** of
that. (It *does* beat blind ½-Kelly by 15–30% — shrink in the *direction* of the
uncertainty, not flatly — but the gap caused by not knowing the distribution is
irreducible.) → With μ̂≈0, the μ-dependent part of sizing is a losing game; **shrink it
toward zero ⇒ vol-only sizing**.

**Skew imposes a "hold-at-all" hurdle that crypto essentially never clears.** With non-Gaussian
returns the optimal single-asset weight gains a pure **skewness term −(κ²−1)/(√2κ)** [58];
for the S&P's *tiny* skew (κ=1.042) that penalty (−0.06) is already ~⅓ the size of the
mean-variance term (+0.17) — and crypto's skew is far larger, so the negative-skew penalty
*dominates*, driving the optimal position much smaller than μ/σ² suggests. The clean
long-only threshold: **hold only if μ > √2(κ−1)σ** [58] — a hurdle a real single-coin edge
essentially never meets. (And most "risk premia" are paid for *negative* skew — smooth-Sharpe
insurance-selling [54]; trend/loss-cutting is the desirable positive-skew exception.)

**The deployable form is a one-knob fractional shrink, and two principled dials agree.**
The ridge-Kelly closed form **f^Ri = μ/(σ²(1+2γ))** [3] is full Kelly shrunk by 1/(1+2γ),
one risk-aversion knob γ; at γ=1 you trade ~30% of growth for ~90% less path-variance — the
quantitative heart of why fractional Kelly is the deployable form. It is *the same dial* the
negative-power-utility map [6] reaches from the other direction (½K ↔ δ=−1, ¼K ↔ δ=−3,
η=−2γ). Thorp — with maximal real-world skin in the game — insists on fractional Kelly and
"bet LESS than the formula" [80].

**We are never in Kelly's asymptotic regime; report MEDIANS.** Triple Kelly's Monte-Carlo
**mean** wealth was 940× but its **median** was 0.017× (near-certain wipeout) [55] —
overbetting's average is a rare lottery path masking typical ruin. Kelly's superiority only
showed up after ~10k–40k trades; a single coin over a few years has orders of magnitude too
few, so the growth guarantee is unavailable and finite-sample ruin dominates. Risk of ruin
falls **~exponentially** as you cut per-bet size [17] — the math behind fractional Kelly + a
hard position floor.

**Robustness ↔ conservatism: size for the worst-case distribution.** Distributionally-robust
Kelly maximises worst-case growth over an ambiguity set and is provably long-run optimal
under uncertainty [42]; adding Knightian uncertainty / fat tails pushes the optimal fraction
**down** further [43]. A wide ambiguity set on a no-edge coin drives the bet toward zero
(≈ "just hold / don't actively size"). Concrete recipe: feed a **lower-quantile (pessimistic)
edge AND an upper-quantile vol** into any sizing rule [45][19]; the quantile level is an
interpretable conservatism dial, and a chance-constraint ("only size up if positive expected
return at high confidence") is a significance gate before sizing — aligned with our FROZEN
gate. The single-asset analogue of covariance shrinkage [77] is shrinking μ̂ → 0.

**Bet-sizing-from-confidence (meta-labeling) decouples side from size — but can't make
edge.** López de Prado's meta-labeling [38] separates *which side* (a primary signal) from
*how much* (a secondary classifier's calibrated probability → bet size), with triple-barrier
(profit-take + stop + time) labels. The portable disciplines: decouple side from size;
path-aware labels. But the secondary model is ML on few, overlapping, non-IID samples
(overfitting), and if our thesis holds (no robust edge), its probability hovers near 0.5 and
the sizing adds little but complexity. It makes a *profitable* signal bet better — it does not
manufacture edge.

**CVaR (Expected Shortfall) is the right tail OBJECTIVE/metric; VaR is not.** VaR is
non-coherent — non-subadditive, can punish diversification [82][34]; CVaR is coherent and
LP-optimisable [34], with coherent cousins CDaR [41], EVaR [64] (the most conservative),
and spectral measures [98] (encode the operator's risk aversion). For us CVaR/ES is a trivial
readout from the bootstrap loss distribution. Crypto's tail is enormous and time-varying
(daily 99% ES ≈ −22% [89]; 76% drawdowns [83]) — measure it directly with a fat-tail-aware
method (GARCH-EVT/POT-GPD [88], or our model-free bootstrap), never Gaussian VaR.

**Multi-asset allocators (risk parity / HRP / NCO) are N/A to one coin** but unanimously
endorse **estimation-light, variance-based, inversion-free** sizing: HRP/NCO beat min-variance
OOS by *avoiding* covariance inversion ("Markowitz's curse") [32][33][52][71][77]; ERC sits
between min-variance and 1/N using covariance only, no μ [7][23]. The transferable lesson for
one coin is the *principle* (size by vol, don't trust estimated μ), plus crypto-HRP if we ever
go to a small basket — but note crypto crashes together (BTC–ETH ρ>0.85 in stress [79]), so a
basket diversifies far less than its calm-period correlations suggest; cash is the only real
diversifier [79].

---

## 2. Possible solutions / what can be done with this research

1. **Keep fixed-fraction as the default, and *justify* it from the research** — it is the
   single-coin analogue of 1/N [18] and the honest endpoint of "μ̂≈0 ⇒ f→0" [45][58]. The
   shipped `FixedFractionSizer` is the *right* design; this research is its citation.
2. **If a sizing dial is ever exposed, make it the one-knob fractional-Kelly shrink**
   f^Ri = μ/(σ²(1+2γ)) [3] (= negative-power-utility δ [6]) — heavily capped, with μ̂
   shrunk hard toward 0, and a hard position-size floor (risk-of-ruin [17]).
   **Prefer vol-only sizing** (μ̂=0), because μ-errors cost ~20× σ-errors [6].
3. **Be conservative on the INPUTS, not just the multiplier:** lower-quantile edge + upper-
   quantile vol [45][19]; a chance-constraint significance gate before any size-up.
4. **Report coherent tail risk:** CVaR/ES at 90/95/99% [34][82], CDaR [41], optionally EVaR
   [64] / spectral [98] — all near-free from the bootstrap loss distribution. Report skew
   [54][58] and **median** terminal wealth [55].
5. **(Optional, gated)** Meta-labeling [38] as a *size/trade-less filter* on a crowned trend
   pick (triple-barrier labels), expecting cost-drag reduction, not return.

Every solution is a **bound / discipline / honest metric** — none is a sizing edge.

---

## 3. Relevance for the project

**Directly relevant — sizing is a shipped surface, and this research tells us to keep it
conservative.** The budget-aware `FixedFractionSizer` is live; the research's verdict is that
**this is correct** and that the tempting "smarter" alternatives (μ-driven Kelly) are exactly
where a single-coin advisor loses.

- **The research *is* the justification for fixed-fraction over Kelly.** μ̂ for one coin is
  the least reliable input yet the most consequential for any growth-optimal sizing (20:10:2
  [6]); no quantile/option trick recovers more than ~1–3% of a bad-μ̂ loss [45]; the skew
  hurdle μ>√2(κ−1)σ is essentially never met on crypto [58]; Kelly's guarantee needs ~10k–40k
  trades we don't have [55][100]. **Net: μ̂≈0 ⇒ f→0 ⇒ don't actively size on edge — hold and
  control risk by vol.** That is precisely our fixed-fraction + vol-overlay posture.
- **Vol targeting is Kelly with μ held constant** [4][80] — so the *sizing* research and the
  *overlay* research are one body: the de-risking half of vol targeting is the well-founded
  half of Kelly, and the lever-up half is both dangerous *and* off the table (no leverage
  [2][15]). This file and the overlay file are two views of f ≈ μ/σ².
- **Risk-of-ruin / budget discipline is on-thesis and already enforced.** Ruin probability
  falls ~exponentially with smaller per-bet size [17]; `FixedFractionSizer::with_budget_cap`
  is the hard floor (qty·price ≤ budget, even after equity grows) — the research backs the
  HARD budget cap as not just conservative but *structurally* correct in a fat-tailed,
  leverage-cascade market [37][56].
- **CVaR-over-VaR is a near-free honesty upgrade** that fits crypto's fat tail [82][34][89]
  and our retail framing — and our 1000-path bootstrap already produces the loss distribution
  these measures read from [7][88].
- **"Traceable and plausible" is served by the honest sizing story.** The advisor can *show*
  why it doesn't bet bigger: the estimation-error and skew hurdles are concrete, citable
  reasons, not hand-waving. Reporting **median** outcomes [55] and CVaR makes the sizing
  decision auditable — measured honesty, which is the operator's stated goal.

**Honest caveat:** there is *no* sizing edge to sell. The single-asset value of this entire
literature is conservatism and honest measurement. Anything that claims a sizing *return*
edge on a no-edge coin is overfitting [45][55][91][99].

---

## 4. Advantages for the project

**The advantage is risk-shaping and honesty — drawdown/ruin control is universal even with
no edge.**

- **Conservative sizing reliably reduces drawdown and ruin risk regardless of edge** — the
  mechanical inverse-variance effect is real [1][2][4], ruin falls ~exponentially as bets
  shrink [17], and fractional Kelly trades a little growth for *disproportionate* path-
  smoothness (~30% growth for ~90% less variance at γ=1 [3]). For a retail user who fears
  ruin, this is a genuine, deliverable benefit.
- **The budget cap is vindicated as structurally advantaged, not merely cautious.** In a
  deleveraging spiral the unlevered holder is the natural buyer who survives while levered
  holders are force-liquidated [37][56]; our HARD budget cap and no-leverage design are the
  *correct* posture in crypto, and we can say so with citations.
- **CVaR/CDaR/EVaR/Sortino reporting differentiates by honesty.** Coherent tail metrics
  [82][34][41][64][84] surface crypto's real downside that Sharpe hides; reporting them
  (and skew [54], and medians [55]) is the measured-honesty product, and a competitive
  advantage over frameworks that headline implausible Sharpes [91][99].
- **The "don't out-clever a bad μ̂" result is a moat against scope creep.** [45]'s 27–48%
  loss / ~1–3% recovery number is a decisive, citable reason to *refuse* to build μ-driven or
  ML sizing — it keeps the advisor simple, auditable, and cheap, and protects against the RL
  sizing trap (~8,000 years of data needed [50]).
- **It reuses the engine.** Any fractional-Kelly dial is a one-line transform on the existing
  `FixedFractionSizer`; CVaR is a read of the existing bootstrap distribution — low blast
  radius.

---

## 5. Problems and challenges

**There is no sizing edge — the central honest constraint.** Every μ-driven sizing rule
collapses toward "hold" on a no-edge coin [45][55][58][100]. The challenge is *resisting* the
temptation to ship a "smart sizer" that will overfit; the research is the discipline that
says don't.

**Turnover / cost from any dynamic re-sizing.** Any sizing rule that re-sizes on changing μ̂/σ̂
incurs turnover; constant vol scaling can *lower* net Sharpe once costs count [48][28].
**Mitigation:** prefer static fixed-fraction; if dynamic, use a no-trade band [61] (covered in
the overlay file). Cost is *the* decision variable [89].

**The day-1 baseline-equity-divergence e2e applies to any sizing-modifier.** Per CLAUDE.md
non-negotiables and the v3-vol-overlay-noop precedent, **any** new sizing-modifier (e.g. a
fractional-Kelly dial that changes deployed notional) must ship a day-1 e2e asserting its
output equity diverges from the baseline by ≥ epsilon when the decision variable is non-trivial.
`FixedFractionSizer` already has `crates/risk/tests/budget_sizing_divergence_end_to_end.rs` —
match that pattern for any new dial.

**HARD CONSTRAINTS (named explicitly):**
- **USDT-denominated, `Decimal` not `f64`.** Kelly fractions, σ̂, quantile adjustments, and
  the position floor are all arithmetic that must stay in `Decimal` — no `f64` sizing math.
  `FixedFractionSizer` already uses `rust_decimal::Decimal`; any dial must match.
- **The budget cap is a HARD limit — sizing may never exceed the simulated budget.**
  `FixedFractionSizer::with_budget_cap` enforces qty·price ≤ budget *even after equity grows*.
  Any fractional-Kelly / bet-sizing dial must compose with this cap and never bypass it — the
  cap is a permanent notional ceiling, and a sizing rule that wanted to "lever up" (f>1) is
  both off-thesis and forbidden by the cap (and by no-leverage [2][15]).
- **`ui` must NOT depend on strategy/exec/llm/models.** Sizing state shown in the cockpit must
  flow through the report/data layer, not by `ui` importing `risk`/`strategy`.
- **Gate/bands FROZEN; paper-only; single-coin long-or-flat.** A sizer is a modifier that goes
  *through* the gate; it does not touch the FROZEN classifier bands. Short-selling is a separate
  pre-registered arm; sizing here is long-or-flat fraction-of-budget only.
- **Anchored report SHAs byte-immutable (119/119).** New sizing reports are additive anchors;
  do not edit existing anchored reports.

**Estimation / regime fragility & finite samples.** Any trained sizing parameter goes stale
across crypto regimes [78][83]; we are never in Kelly's asymptotic regime [55][100]; tail
estimates from few trades are noisy [89]. → favour static, parameter-light sizing; pair tail
estimates with the bootstrap; gate any tuned dial with PBO/DSR [69][70].

**Tail estimation is hard and the danger is the tail, not the point.** Better point-vol ≠
better tail risk [51]; Gaussian VaR understates the crypto left tail; VaR is non-coherent
[82]. **Mitigation:** report CVaR/ES (coherent), measure the tail directly from the bootstrap,
sanity-check against GARCH-EVT [88].

**What to be skeptical of:** any sizing scheme reporting a *return* edge on a no-edge coin
(overfitting [45][91][99]); negative-skew smooth-Sharpe sizing (insurance-selling tail bomb
[54]); meta-labeling "confidence" that is in-sample noise [38]; raw optimal-f / full Kelly
exposed to a retail user [47][55]; the mean (not median) of a Kelly Monte-Carlo [55].

---

## 6. Concrete next steps / candidate work items

These are **lower priority than the overlays** (sibling file) and **far lower than the P0 gate
upgrade** (`SYNTHESIS.md`: DSR/PBO/N_eff/MinBTL). The honest default here is **"keep
fixed-fraction; do not build a μ-driven sizer."** The actionable items are *reporting* and
*documentation*, plus one optional gated dial.

### P1 — CVaR/coherent-tail + Sortino + median reporting *(highest-value item in this file)*
- **What:** Add **CVaR/ES at 90/95/99%** [34][82], **CDaR** [41], **Sortino/Calmar** [84],
  **skew** [54][58], and **median terminal wealth** [55] to the bake-off + forward report,
  read from the existing bootstrap loss distribution. Report **CVaR, not VaR** [82]. Optionally
  EVaR [64] / a spectral risk-aversion-weighted number [98].
- **Where:** the bake-off ranking report + forward-plan report (the bootstrap loss distribution
  is already produced in `crates/backtest/src/bakeoff/`). **Additive — does not touch the
  FROZEN classifier bands.** (Shared with the overlay file's P2-D — do it once.)
- **Priority: P1.** Cheap, near-free from existing data, makes the sizing/risk story honest and
  auditable ("traceable and plausible"). Sanity-check against GARCH-EVT [88] as a one-off.

### P1 — Document the sizing posture (an ADR / dev-note, not code)
- **What:** Record *why* the advisor sizes by fixed-fraction + vol, not Kelly-from-μ̂: the
  20:10:2 estimation-error asymmetry [6], the 27–48%/~1–3% bad-μ̂ result [45], the skew hurdle
  μ>√2(κ−1)σ [58], the finite-sample/asymptotic gap [55][100], and the budget cap as ruin
  control [17][37]. This makes the design decision *traceable* and pre-empts future "let's add
  a smart sizer" scope creep.
- **Where:** a `spec/dev-notes/` memo (analyst/architect-owned; via the `spec-update` skill).
- **Priority: P1.** Pure documentation; locks in the honest rationale.

### P2 — Optional one-knob fractional-Kelly dial (gated experiment, expected ≈ null)
- **What:** *If* the operator wants a sizing dial at all, expose the single ridge-Kelly knob
  **f^Ri = μ/(σ²(1+2γ))** [3] (= negative-power-utility δ [6]) with **μ̂ shrunk hard toward 0**
  [45][77], a **hard position-size floor** [17], a **lower-quantile edge + upper-quantile vol**
  conservative input [45][19], and a **hard cap at the budget**. Default the dial to vol-only
  (μ̂=0). Bake off Half-Kelly vs Quarter-Kelly vs fixed-fraction vs vol-target under the FROZEN
  gate (hypothesis: defensive sizing cuts drawdown, not net Sharpe).
- **Where:** a transform layer on `crates/risk/src/sizing.rs` (`FixedFractionSizer`), composing
  with the budget cap; **ship a day-1 baseline-divergence e2e** (pattern:
  `crates/risk/tests/budget_sizing_divergence_end_to_end.rs`); gate the chosen γ with PBO/DSR.
- **Priority: P2 (only on operator request).** Expected ≈ null on Sharpe; value is the
  drawdown-shaping and the *demonstration* of the "no sizing edge" thesis under the gate.

### P2 — Meta-labeling as a trade-less / cost-reduction filter (gated experiment)
- **What:** On a crowned *trend* pick only, a secondary "whether-to-act/size" classifier on
  **triple-barrier** labels [38] (with avg-uniqueness sample weights + purged/embargoed CV).
  The implementable cousin is a **cost-aware execution filter** — act only when
  |expected_move| > λ·c·|Δpos| — which (per `SYNTHESIS.md` item 17, ml-trading) cut trades ~98%
  and restored viability but **did not beat B&H**. Expect cost-drag reduction, not return.
- **Where:** new candidate filter; its own day-1 divergence e2e; cross-reference the ml-trading
  application docs.
- **Priority: P2.** Plausible win is cost reduction; do not expect alpha.

### What NOT to do
- **Do not build a μ-driven / Kelly-from-μ̂ sizer as a return tool** — loses 27–48% to a bad
  μ̂, recoverable by ~1–3% at best [45]; skew hurdle never cleared on crypto [58]; never in the
  asymptotic regime [55][100]. Default to fixed-fraction / vol-only.
- **No full Kelly or raw optimal-f exposed to a retail user** — punishing drawdowns, near-certain
  median ruin when overbet (Triple Kelly median 0.017× [55]); 2× Kelly → risk-free growth [6];
  optimal-f is the aggressive Kelly cousin [47].
- **No RL / heavy-ML sizing** — ~8,000 years of daily data needed even on clean sim [50].
- **No leverage / f>1** — off-thesis, forbidden by the budget cap, and BAB shows the inability
  to lever is economically real [15]. We can only de-risk.
- **Do not headline VaR or the mean Kelly outcome** — VaR is non-coherent [82]; report CVaR and
  the **median** [55][34].
- **Do not pitch a multi-coin basket as "diversification"** — crypto crashes together (ρ>0.85
  in stress [79]); cash is the only real diversifier. (HRP/NCO [32][33][52][71] only if we ever
  *do* go multi-asset, and even then with the contagion caveat.)

### Effort & blast radius (summary)
| Item | Where | Nature | Blast radius |
|---|---|---|---|
| P1 CVaR/Sortino/median reporting | bake-off + forward report | additive metrics from existing bootstrap | low (no FROZEN-band touch) |
| P1 sizing-posture ADR/dev-note | `spec/dev-notes/` | documentation | none (no code) |
| P2 fractional-Kelly dial (gated) | transform on `risk/src/sizing.rs` + day-1 e2e | optional sizing knob | low–med (3 callers of `FixedFractionSizer`) |
| P2 meta-labeling / cost filter | new candidate + day-1 e2e | gated experiment | med |

---

## 7. Open questions for analyst & architect

1. **Do we expose a sizing dial at all, or hold the line at fixed-fraction + vol-only?** The
   research says a μ-driven dial is ≈ null-to-harmful on a no-edge coin [45][55][58]; is the
   demonstration value (showing the operator *why* under the gate) worth the build + the day-1
   e2e? (Default recommendation: document the posture, don't build the dial unless asked.)
2. **If we size by f ≈ μ̂/σ̂², how badly does a noisy single-coin μ̂ blow up vs μ̂=0 (pure vol
   scaling)?** Run the estimation-error stress test [6][45] — quantify the loss on *our* data to
   make the "shrink μ̂ to 0" decision concrete and citable.
3. **Is the exact log-normal Kelly fraction [4] (more conservative than μ/σ²) materially safer
   on crypto's high-σ regime than the μ/σ² approximation?** (If we ever compute a fraction.)
4. **Half-Kelly vs Quarter-Kelly vs fixed-fraction vs vol-target** baked off under the FROZEN
   gate on the same (coin,window): net-of-cost terminal wealth + max drawdown. (Hypothesis:
   defensive sizing cuts drawdown but not net Sharpe.)
5. **Which coherent tail metric is the right headline** for the retail user — CVaR (familiar),
   CDaR (drawdown-native [41]), EVaR (most conservative [64]), or a spectral / risk-aversion-
   weighted number [98] that personalises to the operator's risk aversion? (Report-design /
   product decision.)
6. **What is the right conservatism quantile** for a lower-quantile-edge / upper-quantile-vol
   input [45][19] if a dial ships — and how do we expose it as an interpretable "how cautious"
   knob without implying a false precision about edge?
7. **Does the cost-aware execution filter (|expected_move| > λ·c·|Δpos|)** reduce turnover-drag
   enough to matter on a high-cost crypto coin, given it did *not* beat B&H elsewhere
   (`SYNTHESIS.md` item 17)? Worth a gated test on a crowned trend pick?
8. **How do we present "there is no sizing edge" honestly** without it reading as a product
   weakness — i.e. frame conservative sizing + ruin control + honest tail metrics as the
   *value* (the measured-honesty thesis), not an absence? (Analyst / UX framing.)
