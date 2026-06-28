# Application — The López de Prado pipeline, meta-labeling & the cost-aware execution filter

_Decision doc for analyst + architect. Distilled from `research/ml-trading/` (100-paper ledger;
cite `ml-trading[N]` → `research/ml-trading/papers.md`) and cross-checked against
`research/SYNTHESIS.md` (P0/P1 roadmap). This is the **genuinely actionable, gate-compatible**
half of the ml-trading corpus: the discipline toolkit (triple-barrier, sample-uniqueness,
purged/embargoed CV, Clustered-MDA), the selection-bias vetoes (MinBTL + the False-Strategy SR₀
threshold), and the two "do-less" overlays (meta-labeling as a trade-filter; the cost-aware
execution filter). The classical-learner / baseline / ensemble material lives in the companion
`application-classical-ml-and-baselines.md`._

> **The app this serves:** a Rust **single-coin crypto investment ADVISOR** — paper/sim only,
> NOT advice, NOT live. Journey: pick ONE coin + budget → **bake off every strategy** on a
> (coin, window) → **rank** under a FROZEN robustness gate (1000-path moving-block bootstrap;
> FRAGILE ⇒ can't crown; **buy-and-hold is always the benchmark and is exempt**) → forward
> rule-based plan → watch it paper-trade. Validated thesis: **no active strategy robustly beats
> buy-and-hold net of costs** (confirmed on hourly BTC: frictionless XGBoost +73.5%/yr → −64% at
> 10 bps; nothing beats B&H after Holm correction — `ml-trading[89]`). The product sells **measured
> honesty** — operator goal: *"a framework for trading with traceable and plausible trading."*

---

## 1. Summary of the research

The ml-trading corpus contains one **coherent, peer-reviewed methodology canon** for "doing
financial ML without fooling yourself," anchored on López de Prado's *Advances in Financial
Machine Learning* (`ml-trading[2]`). It is not a set of alpha recipes — it is a set of **honesty
constraints and trade-fewer filters**, almost all linear/recursive/bootstrap arithmetic (Rust-
friendly, no deep net required). Four strands matter for us:

**(a) The discipline pipeline (`ml-trading[2][42]`).** Seven techniques, each a guardrail:
fractional differentiation (stationarity *with* memory, choosing the minimum `d` that passes ADF,
**fit on train only**); **triple-barrier labeling** (label each event by which of {profit-take,
stop-loss, time-expiry} is hit first — path-dependent, unlike a fixed-horizon return label);
**sample-uniqueness weighting + sequential bootstrap** (overlapping labels are non-IID; down-weight
by *average uniqueness* and bag with `max_samples ≈ avg uniqueness`); **purged k-fold CV + embargo**
(remove training labels whose time-span overlaps a test label; embargo the bars just after — naive
k-fold "vastly over-inflates"); substitution-robust feature importance; and the **Deflated Sharpe
Ratio** (penalize the best-of-N for the number of trials). `ml-trading[42]` ("10 reasons most ML
funds fail") restates these as failure modes; the eight-type **leakage taxonomy** in
`ml-trading[56]` is the cross-field generalization (in civil-war prediction, *every* "complex ML
beats logistic regression" claim failed once leakage was fixed — our thesis in another field).

**(b) The selection-bias vetoes — now full-text with exact formulas (`ml-trading[95][20]`).**
*Pseudo-Mathematics and Financial Charlatanism* (`ml-trading[95]`) is the manifesto and, read to
the math, hands us a **new, free, closed-form gate primitive**:
- **MinBTL (minimum backtest length):** `MinBTL ≈ 2·ln(N) / SR_target²` years. At `SR_target=1`:
  N=10 → 4.6 yr, N=50 → 7.8 yr, N=100 → **9.2 yr**, N=1000 → 13.8 yr. The paper's calibration
  point: **5 years of data ⇒ at most ~45 independent configs**, else the winner shows in-sample
  Sharpe ≈ 1 but **OOS Sharpe = 0**.
- **False-Strategy / expected-max-Sharpe threshold:**
  `SR₀ = √V[SR_n] · ((1−γ)·Φ⁻¹[1−1/N] + γ·Φ⁻¹[1−1/(N·e)])`, γ = Euler-Mascheroni ≈ 0.5772, where
  `V[SR_n]` is the **cross-trial variance of the N baked-off Sharpes** (not the std-error of one
  Sharpe). Veto any crown whose in-sample Sharpe ≤ SR₀.
- That same SR₀ feeds the **Deflated Sharpe Ratio** (`ml-trading[20]`):
  `DSR = Φ((ŜR − SR₀)·√(T−1) / √(1 − γ̂₃·SR₀ + ((γ̂₄−1)/4)·SR₀²))` (skew γ̂₃, kurtosis γ̂₄); crown
  only if `DSR > 0.95`.
- **Memory kicker:** under serial correlation (crypto vol-clustering), overfit picks have
  **negative** expected OOS, not merely zero (`ml-trading[95]`). Companion model-free tool: **PBO
  via CSCV** (`ml-trading[7]`) — fraction of combinatorial splits where the in-sample-best lands
  below the OOS median; rises sharply with trial count.

**(c) Meta-labeling as a "trade-less" filter (`ml-trading[2][15][42]`).** Two-stage: a **primary
model decides the *side*** (our existing SMA/MACD/RSI/Bollinger long/flat signal) and a **secondary
binary classifier decides *whether to act* (and bet *size*)** — it never flips the side, it only
filters/sizes. Trained on triple-barrier labels (1 if the primary's trade would hit its
vol-scaled profit-target before its stop/time barrier). Reported wins are **precision/F1, Sharpe,
and drawdown** — i.e. *fewer, higher-confidence trades → less cost drag* — explicitly NOT a clean
"beats buy-and-hold net of costs over many regimes" result (`ml-trading[15]`). Calibration helps
*fixed* sizing but not learned sizing (`ml-trading[15][36]`). Explain it with **Clustered-MDA**
(`ml-trading[94]`: cluster co-moving features, shuffle the whole cluster — empirically lifted a
meta-labeling model's AUC 0.537→0.672), bootstrap-checked for stability (`ml-trading[78]`).

**(d) The cost-aware execution filter — the implementable cousin, exact rule (`ml-trading[89]`).**
On 70,872 hourly BTC bars, 27-fold walk-forward, 10 bp/turn, sign-based XGBoost went **+73.5%/yr
frictionless → −64%/yr net**. The filter: allow a position change only when
`|expected_move| > λ · c · |position_change|` (c = round-trip cost = 0.001; λ = 2.0; intuition:
a long-only entry needs a forecast move > 0.20%, a reversal > 0.40%). This cut trades **10,619 →
251 (≈98%)** and restored the best long-only XGBoost to **+65.4%/yr at Sharpe 1.09** — **but after
Holm correction, p = 0.89–1.00: it restores *viability*, it does not significantly beat B&H.**

**Through-line:** this strand gives us **honesty primitives we can ship** (MinBTL, SR₀/DSR, PBO,
purged CV, leakage audit) and **two overlays whose plausible win is cost-drag/drawdown reduction,
not return** (meta-labeling, cost-aware filter). Every on-domain test that applied this discipline
to crypto landed on the same verdict the product already sells.

---

## 2. Possible solutions / what can be done with this research

1. **Wire MinBTL + SR₀ + DSR as additive crown pre-conditions in the gate.** All inputs already
   exist: N (configs the bake-off tried), T (window length), and the per-strategy return matrix
   (→ the N Sharpes → `V[SR_n]`, skew, kurtosis). Pure closed-form arithmetic. `ml-trading[95][20]`.
2. **Add PBO via CSCV as a diagnostic** over the same T×N return matrix the bootstrap already
   holds (`ml-trading[7]`). Report as a number; disqualify high values.
3. **Ship the cost-aware execution filter as a new bake-off configuration** — a one-parameter
   (`λ`) gate that sits on top of *any* strategy's signal. It **cannot underperform "always act"
   once costs are charged** (`ml-trading[89]`). For our rule-based strategies, `expected_move` is
   the strategy's own signal magnitude (e.g. SMA-gap, MACD histogram), not an ML forecast.
4. **Prototype meta-labeling as a "whether-to-act" overlay** on an existing crowned strategy —
   small interpretable classifier (tree/logit) on triple-barrier labels, avg-uniqueness sample
   weights, purged/embargoed CV, gated vs B&H net of costs (`ml-trading[2][15][14]`).
5. **Codify the test-data discipline as engine gates** (`ml-trading[5][16][56]`): fit transforms
   on in-sample only; report R² on *returns, never price levels*; an eight-point leakage audit
   per learned component (shift-by-one look-ahead on every indicator; no preprocessing/selection
   on combined sets; no overlapping-label non-independence).
6. **Validate the gate on synthetic no-alpha series** (GARCH/OU/Heston) — it must refuse to crown,
   and DSR/PBO must flag overfit picks (a standing regression test; converges with
   `research/SYNTHESIS.md` P1 item 12).
7. **Adopt triple-barrier as the label primitive** if/when any learned component is trained — it
   maps cleanly onto our existing take-profit / stop-loss / max-holding paper-trade mechanics
   (`ml-trading[2][3][29][50]`).
8. **Use Clustered-MDA for any operator-facing "why it acted" story** (`ml-trading[94]`), since
   our RSI/MACD/momentum/vol features co-move and naive MDA/MDI/SHAP lie under correlation
   (`ml-trading[49][58][8]`).

---

## 3. Relevance for the project

**This is the highest-fit research strand in the entire program for our specific app**, for three
reasons:

- **It hardens the FROZEN gate without touching the frozen classifier bands.** Our gate already
  does the hard part (1000-path moving-block bootstrap, weakest-link, B&H benchmark). What it does
  *not* yet do is correct for the multiple-testing bias of crowning the **best of N** swept
  configs. MinBTL, SR₀/DSR, and PBO are **purely additive** — they consume artifacts the bake-off
  already produces (N + the return matrix) and emit a per-run overfitting scorecard *next to* the
  verdict. This is exactly the P0 gap that `research/SYNTHESIS.md` flags as the single
  highest-leverage next action, and it is the strand that most directly advances *"traceable and
  plausible"*: the crowned card can show **N, MinBTL-vs-window, SR₀, DSR, PBO** — turning "this
  strategy won" into "this strategy won, here is the search budget it was penalized for, and here
  is the probability it's an artifact."

- **It independently validates the product thesis on our exact asset.** `ml-trading[89]` is BTC,
  hourly, walk-forward, 10 bp costs, multiple-testing-corrected — and after Holm, *nothing beats
  B&H*. This is not a transfer from equities; it is our coin, our discipline, our verdict. The
  "expected-null" honesty the product sells is the literature's own finding.

- **Its two overlays fit our skeptical posture.** Meta-labeling and the cost-aware filter are the
  rare ML ideas that *respect* our thesis: they are defenses against trading too much, not bids
  for new alpha. The cost-aware filter in particular is **provably can't-hurt** relative to
  "always act" once costs are charged — a safe default overlay to add to the bake-off menu.

**Honest on expected-null.** None of this is alpha. The literature's verdict is unambiguous:
- The cost-aware filter *restored viability* (Sharpe 1.09) but did **not** beat B&H after Holm
  (`ml-trading[89]`).
- Meta-labeling's reported wins are precision/F1/Sharpe/drawdown — a **cost-drag/drawdown** story,
  not a return story, demonstrated on few datasets (`ml-trading[15]`).
- The selection-bias math predicts the gate will **crown almost nothing over B&H by construction**
  (`research/SYNTHESIS.md` §P0.3: for the sub-0.4 net Sharpes a single coin realistically
  produces, the correct haircut is >50% to near-total).

So the value of this strand is **a more trustworthy null**, plus two overlays that can make a
*losing-to-B&H* strategy lose by less (lower turnover, shallower drawdown) — which is itself a
legitimate, honest thing to surface to the operator.

---

## 4. Advantages for the project

- **Free inputs, additive blast radius.** MinBTL/SR₀/DSR/PBO need only N and the return matrix the
  bake-off already computes. No new data, no new dependency, FROZEN bands untouched, anchored SHAs
  untouched (new fields are additive report content, not edits to anchored report bodies).
- **Closed-form & Rust-trivial.** Inverse-normal CDF, sample variance, Euler-Mascheroni constant,
  a logit rank for PBO — no `candle`/`tract`, no `f64`-vs-`Decimal` tension in the math layer
  (these are statistics over already-computed returns, not money arithmetic).
- **Directly serves "traceable & plausible."** Every number on the crowned card gains a provenance
  and a penalty: *"N=120 configs tried; window 2.5 yr < MinBTL 9.6 yr ⇒ REFUSED to crown over
  B&H."* This is the most legible possible upgrade to the operator's trust.
- **On-domain external corroboration.** `ml-trading[89]` is a citable, recent, BTC-specific,
  cost-and-multiple-testing-correct demonstration that no active ML strategy robustly beats holding
  — a third-party mirror of our gate's verdict we can point the operator to.
- **The cost-aware filter is a strict-improvement overlay.** Unlike most overlays, it provably
  cannot do worse than "always act" net of costs — a low-risk addition that demonstrably moved a
  BTC strategy from ruin (−64%) to viability (+65.4%) by cutting turnover 98% (`ml-trading[89]`).
- **Meta-labeling reuses what we have.** Our SMA/MACD/RSI/Bollinger strategies already emit a
  *side*; meta-labeling bolts a small classifier on top — no rearchitecting of the strategy layer.

---

## 5. Problems and challenges (risks + HARD CONSTRAINTS bumped)

**Research-intrinsic risks:**

- **MinBTL/SR₀/PBO need an *effective* N, and `M > T` is our exact regime.** Our bake-off tries
  more configs (M) than the window has bars (T), so the return-correlation matrix used to collapse
  M → N_eff is ill-conditioned and ρ̄ is itself overfit. Per `research/SYNTHESIS.md` §P0.1
  (full-text Bailey/LdP requirement), we **must dimension-reduce / cluster (ONC or PCA) before
  estimating N_eff** — using raw M is conservative-but-crude, using a naive ρ̄ is wrong. This is
  the main correctness subtlety, not a formula typo.
- **DSR/MinBTL crown almost nothing — which is correct but operationally stark.** With fat tails,
  the survivable trial budget *shrinks* (ŜR=2.5 clears at N=88 Normal but only N=46 at
  skew−3/kurt10 — `research/SYNTHESIS.md` §P0.2). The gate will refuse most crowns over B&H. That
  is the honest outcome; the operator UX must frame "refused to crown" as *the product working*,
  not a failure.
- **The cost-aware filter's `expected_move` is ambiguous for rule-based strategies.** `ml-trading[89]`
  uses an ML forecast magnitude. Our crowned picks are mostly rule-based (SMA/MACD/RSI/Bollinger),
  so we must define `expected_move` per strategy (signal magnitude / distance-from-band) — this is
  a modeling choice that needs analyst sign-off, not a copy-paste.
- **Meta-labeling is the heaviest item and the least likely to pay off in *return*.** It needs
  triple-barrier labels, avg-uniqueness weights, sequential bootstrap, purged/embargoed CV, and a
  calibrator — and its honest prior is "won't clear the gate over B&H, same as the cost-filter"
  (`ml-trading[15][89]`). Worth a gated experiment, not a v1 feature.
- **Purged/embargoed CV is mandatory and easy to get wrong.** Naive k-fold on overlapping labels
  "vastly over-inflates" (`ml-trading[2][14]`); any meta-labeler that skips purge/embargo will
  report a fake edge. CPCV (`ml-trading[14]`) is the empirically-best variant but adds combinatorial
  cost.
- **Interpretability ≠ edge.** Clustered-MDA gives an *honest* "why it acted" but a stably-
  attributed feature group can still belong to a model that loses to B&H (`ml-trading[94][8][58]`).
  Do not let a clean attribution story masquerade as validation.

**HARD CONSTRAINTS this strand must respect (name them in any work item):**

- **Gate/bands are FROZEN — additive only.** MinBTL/SR₀/DSR/PBO go in `crates/backtest/src/bakeoff/
  {robustness.rs, rank.rs}` as **new** computations + **new** report fields. They must **not**
  alter the frozen FRAGILE/robustness classifier bands or the weakest-link verdict logic.
- **Anchored report SHAs are byte-immutable (119/119).** New scorecard fields must land in *new*
  report sections / *new* report files, or follow the ADR-0038 re-emission protocol — never mutate
  an existing anchored report body. Run `scripts/verify_anchors.sh` before and after any
  `spec/*/reports/` touch.
- **Decimal, not f64, for money.** The selection-bias math is statistics over returns (f64 is fine
  for the stats), but anything that flows into a *position size / budget allocation* (e.g.
  meta-label-derived bet size, cost-filter position deltas) must be `Decimal`. Keep a clean seam
  between "statistics on returns" (f64 ok) and "money decisions" (Decimal).
- **`ui` must NOT depend on strategy/exec/llm/models.** The overfitting scorecard is data produced
  by `backtest`; the `ui` renders it as a passive struct (numbers + verdict text). Do **not** let
  the scorecard pull a strategy/exec/llm/models type into the `ui` dependency graph.
- **Overlays ship a day-1 baseline-equity-divergence e2e.** Both the cost-aware filter and any
  meta-labeling overlay are *overlays/sizing-modifiers* → per the v3-vol-overlay-noop precedent
  they MUST ship a day-1 e2e asserting the overlay's output equity diverges from the un-filtered
  baseline by ≥ epsilon when the decision variable is non-trivial. Unit tests on the math + an
  anchored backtest are **not** sufficient to catch a no-op filter.
- **Paper-only; ML is narration-only in v1.** Meta-labeling is a *learned trading component*, so
  it is explicitly out of the v1 narration-only-ML scope. The retired TCN/PatchTST/GARCH/
  LLM-forecaster overlays are the precedent: opt-in, concluded not-beating-passive. A meta-labeler
  must be gated/opt-in and is a candidate experiment, **not** a default v1 strategy.
- **ML default crates: `candle` (prototyping) / `tract` (ONNX serving)** per architecture — but
  note the entire actionable core here (MinBTL/SR₀/DSR/PBO + tree/logit meta-labeler) needs
  **neither**; it's closed-form stats + a shallow classifier. Reach for `candle`/`tract` only if a
  meta-labeler graduates beyond a tree.

---

## 6. Concrete next steps / candidate work items

Named, with codebase location and priority. Priorities mirror `research/SYNTHESIS.md` (the
selection-bias gate is its P0).

- **[P0] `gate-minbtl-and-sr0-veto`** — add MinBTL (`2·ln(N)/SR_target²`) and the False-Strategy
  SR₀ threshold + DSR (`>0.95`) as **additive crown pre-conditions**. Location:
  `crates/backtest/src/bakeoff/{robustness.rs, rank.rs}` + the ranking report. Inputs already
  present (N, T, return matrix). Refuse to crown over B&H when `T < MinBTL(N_eff)` or `ŜR ≤ SR₀` or
  `DSR ≤ 0.95`. `ml-trading[95][20]`. **This is the single highest-value item — see final report.**

- **[P0] `gate-neff-cluster-first`** — estimate **effective trial count** by clustering the return
  matrix (ONC / hierarchical on correlation distance) *before* computing ρ̄, because `M > T`.
  Feeds MinBTL/SR₀/DSR/PBO. Location: `crates/backtest/src/bakeoff/robustness.rs`. This is the
  correctness prerequisite for the item above. `ml-trading[94]`, `research/SYNTHESIS.md` §P0.1.

- **[P0] `gate-pbo-cscv`** — PBO via Combinatorially Symmetric Cross-Validation over the existing
  T×N matrix; report as a diagnostic, disqualify high values. Location:
  `crates/backtest/src/bakeoff/robustness.rs` + report. `ml-trading[7]`.

- **[P0] `gate-overfitting-scorecard`** — surface `{N, N_eff, MinBTL vs window, SR₀, DSR, PBO}` on
  the crowned card as a passive struct rendered by `ui`. Location: `backtest` produces it; `ui`
  renders it (no new `ui→strategy/exec` edge). Directly serves "traceable & plausible".

- **[P1] `cost-aware-execution-filter`** — new bake-off configuration:
  `|expected_move| > λ·c·|Δpos|` (λ operator-tunable, default 2.0; c = round-trip cost). Define
  `expected_move` per rule-based strategy (signal magnitude). **Ships a day-1 baseline-divergence
  e2e.** Location: a new overlay/config feeding `crates/backtest/src/bakeoff/`. `ml-trading[89]`.

- **[P1] `synthetic-no-alpha-gate-test`** — standing regression test: feed GARCH/OU/Heston
  zero-edge series through the bake-off; assert it refuses to crown over B&H and that DSR/PBO flag
  the overfit pick. Location: `crates/backtest/tests/`. `ml-trading[14][95]`,
  `research/SYNTHESIS.md` P1 item 12.

- **[P1] `leakage-audit-checklist`** — codify the eight-point audit as a gate for any learned
  component (shift-by-one look-ahead; in-sample-only transforms; R² on returns not levels; no
  overlapping-label non-independence). Location: `crates/backtest/tests/` + a dev-note.
  `ml-trading[5][16][56]`.

- **[P2] `meta-labeling-spike`** — research spike: tree/logit "whether-to-act" classifier on
  triple-barrier labels, avg-uniqueness weights, sequential bootstrap, purged/embargoed CV,
  Clustered-MDA explanation, gated vs B&H net of costs. **Opt-in, day-1 divergence e2e.** Prior:
  cost-drag/drawdown win at best, no return win. Location: new gated candidate behind a feature
  flag, prototyped with `candle` only if it outgrows a tree. `ml-trading[2][15][94][14]`.

---

## 7. Open questions for analyst & architect

1. **N_eff method:** ONC clustering vs PCA vs raw-M-conservative — which do we ship first for the
   MinBTL/SR₀ inputs, given `M > T`? (Raw M is safe-but-crude and may refuse *everything*; ONC is
   the LdP-blessed answer but adds a clustering dependency.) `research/SYNTHESIS.md` §P0.1.
2. **Crown threshold derivation:** hard-code `DSR > 0.95` / `t = 3.0`, or derive it from an
   explicit cost-asymmetry statement (the ORATIO odds-ratio: "a false 'beats-hold' is K× costlier
   than a miss")? `research/SYNTHESIS.md` §P0.5 / `backtesting[40]`. Operator-facing: do we want a
   single tunable "how costly is a false crown" knob?
3. **`expected_move` definition for the cost-aware filter:** for each rule-based strategy
   (SMA/MACD/RSI/Bollinger), what is the principled forecast-magnitude proxy? Needs analyst
   sign-off before the filter is honest. `ml-trading[89]`.
4. **Scorecard UX:** how do we present "REFUSED to crown over B&H" as *the product working*
   (measured honesty) rather than an error state? This is a presenter/analyst framing question.
5. **Is meta-labeling worth the engineering** given its honest prior (no return win, maybe a
   drawdown/cost win) — or do we get 90% of the value from the strictly-simpler, can't-hurt
   cost-aware filter? `ml-trading[15][89]`.
6. **Forward-trade + PBO pairing:** `research/SYNTHESIS.md` §P0.9 amends our design — the single
   forward paper-trade hold-out is insufficient (high variance, blind to trial count); pair it
   with CSCV/PBO + DSR/MinBTL on the bake-off matrix. Do we accept this amendment to the forward
   phase? `ml-trading[7]`.

---

## 8. What NOT to do / effort & blast radius

**Do NOT:**
- **Do NOT treat the cost-aware filter or meta-labeling as alpha.** Both are cost-drag/drawdown
  tools; neither beat B&H after Holm/in their own studies (`ml-trading[89][15]`). Sell them (if at
  all) as "trade less, lose less," never "beat holding."
- **Do NOT skip purge/embargo on any learned component** — naive k-fold on overlapping labels
  manufactures a fake edge (`ml-trading[2][14]`).
- **Do NOT estimate N_eff from a naive ρ̄ when `M > T`** — the correlation matrix is ill-conditioned;
  cluster first (`research/SYNTHESIS.md` §P0.1).
- **Do NOT report R² on price levels** — it's ≈1.0 trivially and meaningless (`ml-trading[16]`).
- **Do NOT let the overfitting scorecard create a `ui → strategy/exec/llm/models` dependency** —
  render it as a passive data struct.
- **Do NOT mutate anchored report bodies** to add scorecard fields — additive new sections/files
  or the ADR-0038 re-emission protocol only.

**Effort & blast radius (rough):**
- *MinBTL + SR₀ + DSR + PBO + scorecard (P0):* **low effort, additive blast radius** — closed-form
  stats over existing artifacts in `bakeoff/{robustness,rank}.rs` + report fields + a passive `ui`
  struct. The N_eff cluster-first prerequisite is the only non-trivial piece. **Highest value-to-
  effort ratio in the corpus.**
- *Cost-aware filter (P1):* **low-medium** — one overlay config + the `expected_move` modeling
  decision + a day-1 divergence e2e. Strict-improvement, low risk.
- *Synthetic-gate test + leakage checklist (P1):* **low** — test-only, no production code.
- *Meta-labeling spike (P2):* **medium-high** — a full learned-component pipeline (labels, weights,
  CV, calibration, explanation, divergence e2e), opt-in, with a high prior of "no return win." Do
  it last, as research, only after the P0 gate hardening proves out.
