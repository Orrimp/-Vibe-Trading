# Application — Splits, Leakage & Cross-Validation (CPCV / PBO / multiple-testing)

*Decision doc for analyst + architect. Distilled from `research/data/knowledge.md`
(primary) and the 100-entry `research/data/papers.md` ledger (cited `data[N]`), with
the cross-topic `research/SYNTHESIS.md` P0 roadmap. This is the **"what do we change
in the app"** layer for the train/test-split, data-leakage, and cross-validation
strand of the data research. It does NOT add papers.*

> **Scope of this file:** purged/embargoed CV and combinatorial-purged CV (CPCV),
> the eight-type leakage taxonomy, the selection/tuning-leakage result, multiple-
> testing deflation (DSR / PBO-via-CSCV / Harvey-Liu haircut / MinBTL), the
> "walk-forward is the *worst* single-path evaluation" finding, and the **design
> amendment** that the forward paper-trade alone is insufficient. The synthetic-data
> and PIT/labeling strands live in the two sibling files.

---

## 1. Summary of the research

The leakage / CV / multiple-testing literature is the **single most load-bearing
strand** for our product, because our bake-off *is* a multiple-testing engine and
our robustness gate is the credibility layer that has to survive it honestly.

- **Leakage is bigger than the train/test split.** Two backtests with identical
  chronological splits can still access different information at *decision time*.
  A one-switch counterfactual study finds centered/look-ahead rolling windows
  (`TEMP_CENTER`) and using the full day-t bar while assuming an open fill
  (`EXEC_OPEN`) inflate Sharpe by **15–30+ points**, consistently across models and
  markets; global normalization and same-day-close fills matter far less.
  `data[2]`.
- **Naive cross-validation manufactures illusory skill on financial data.** On
  **Bitcoin returns, naive k-fold R² ≈ +0.85 vs purged R² ≈ −1.2** — the gap is the
  pure illusion. Purging (drop train labels overlapping the test window) + embargo
  (buffer after each test fold) are mandatory, not optional. `data[3]`.
- **Selection/tuning leakage is the *worst* kind, and it's quantified.** Across
  2,047 datasets × 6 algorithms, preprocessing leakage (normalization/PCA) is
  negligible (|ΔAUC| < 0.005) while **selection leakage — peeking at the best-of-K —
  is ~40× larger (ΔAUC ≈ +0.040)**. The mechanism: selection inflation
  = `σ·√(2·ln K) + genuine-diversity`, where K = trial count, and the noise term
  decays only as `O(1/√n)`. At small n (~1,900), **~90% of the inflation is pure
  noise exploitation, not skill.** `data[80]`.
- **Multiple testing manufactures winners; the haircut is nonlinear.** There is a
  hard mathematical relationship between trial count and the minimum backtest length
  needed to avoid a spuriously high Sharpe `data[8]`. Four interchangeable deflation
  tools consume our existing bake-off output: Deflated/Probabilistic Sharpe + MinTRL
  `data[25]`, **PBO via CSCV** `data[54]`, the **Harvey-Liu haircut** (Bonferroni/
  Holm/BHY) `data[96]`, and a covariance/complexity penalty `data[98]`.
- **CPCV is the modern standard of evidence; plain walk-forward is the *worst*
  single-path option.** A controlled-environment comparison ranks **CPCV best, plain
  Walk-Forward worst** by PBO and DSR — walk-forward's diagnosed weakness is that a
  single chronological path has weaker stationarity and higher temporal variability.
  `data[21][79]`.
- **A single hold-out — i.e. our forward paper-trade alone — is NOT a sufficient
  overfitting check.** The PBO paper devotes a section to dismantling the hold-out
  method: on public data it is likely already in-sample; the researcher knows how it
  behaved and designs to it; it is inadequate for small samples (< 1,000 obs); it is
  **high-variance** (which hold-out you pick can refute a valid strategy or bless an
  invalid one); and it **ignores the number of trials.** The last two land squarely
  on our forward-paper-trade-as-OOS. `data[54]`.
- **OOS decay is real and quantified (corrected figures).** Refereed predictors lose
  **~10% OOS from data-mining (statistically *insignificant*) + ~35% post-publication
  from crowding** `data[94]`; **64–85% of equity anomalies don't survive** a
  microcap-robust multiple-testing protocol, with liquidity signals the worst (93%
  fail) `data[95]`. Decay is *largest* for the cheapest-to-arbitrage, low-
  idiosyncratic-risk names — i.e. BTC/ETH/SOL, our exact coins. We crown the *max of
  many configs*, not one refereed hypothesis, so the ~10% is a **floor, not a
  forecast**.
- **Independent corroboration of the thesis using our exact machinery.** 7,846
  technical rules, in-sample winner significant even after White's Reality Check, but
  **fails out-of-sample** `data[23]`; the best in-sample BTC rule goes negative OOS
  `data[9]`. The structure (enumerate → find in-sample best → snooping-correct →
  check forward) is *identical* to our bake-off; the conclusion *is* our thesis.

**Through-line:** the durable value of all this is **protecting the gate from
crowning a lucky single path** — not finding alpha. Every result here either
hardens the gate or tells us the crown should deflate toward "= buy-and-hold."

---

## 2. Possible solutions / what can be done with this research

Concrete techniques, all of which consume data we **already store** (the per-
strategy return/equity matrix in the bake-off + the trial count N):

1. **Effective trial count `N_eff`** via `N̄ = ρ̂ + (1−ρ̂)·M` (M = configs,
   ρ̄ = mean pairwise return correlation), or ONC clustering / PCA. **Decisive
   caveat:** when **M > T** (more configs than window bars — *our exact situation*),
   the correlation matrix is ill-conditioned and ρ̄ is itself overfit, so we **must
   dimension-reduce/cluster before estimating `N_eff`**. `SYNTHESIS §P0.1`, `data[80]`.
2. **Deflated Sharpe Ratio (DSR)** — deflate the crowned config's Sharpe by trial
   count, sample length, skew, and kurtosis. The threshold uses the cross-trial
   dispersion `V[{ŜRₙ}]` of the baked-off configs' Sharpes (NOT the standard error of
   one Sharpe). Crypto's fat tails *shrink* the survivable trial budget. `data[25]`.
3. **PBO via CSCV** — model-free, computed on the same T×N matrix: split into S
   combinatorial train/test folds, rank configs in-sample, ask "how often does the
   in-sample winner land below the OOS median?" Yields a single honest overfit number
   + a realistic performance-degradation haircut. `data[54][21]`.
4. **Harvey-Liu haircut** — feed the trial count into a Holm (FWER) or BHY (FDR)
   adjustment; report the **haircut Sharpe** next to the raw Sharpe and B&H. The
   FDR variant answers "of the configs that beat B&H in-sample, what fraction are
   false positives?" — the most honest framing for a leaderboard. `data[96]`.
5. **MinBTL pre-flight veto** — `MinBTL ≈ 2·ln(N)/SR²_target` years; refuse to crown
   when the window is shorter than MinBTL(N). A cheap pre-flight gate. `data[8]`.
6. **Purged + embargoed CV (and CPCV)** — *only if* we ever add ML/labels. Purge
   train labels overlapping the test window; embargo a reaction-lag buffer; for
   horizon labels add uniqueness sample-weights + sequential bootstrap. `data[3]`.
7. **A leakage-audit checklist as a gate** — assert per indicator: (a) trailing-only
   data through the decision bar (no centered windows); (b) next-bar fills, never the
   signal bar's own OHLC; (c) any normalization fit on train only; (d) R² on
   *returns*, never price levels. `data[2]`.
8. **Pair the forward paper-trade with a trial-aware multi-path number** (CSCV/PBO +
   DSR/MinBTL on the bake-off matrix) — the antidote to single-hold-out variance and
   trial-blindness. `data[54]`.

---

## 3. Relevance for the project

This strand maps **one-to-one** onto the advisor's credibility claim ("a framework
for trading with traceable and plausible trading") and the FROZEN gate.

- **The gate already does the philosophically-right thing.** Our 1000-path moving-
  block bootstrap replaces a point estimate with a distribution — exactly the
  "single path is the weakest evaluation" verdict that CPCV-best/walk-forward-worst
  `data[79]` and "don't trust one path" `data[84][100]` reach from other directions.
  We are *not* starting from zero; we are closing one specific hole.
- **The hole is selection bias.** Our bootstrap tests each curve's robustness but
  does **not** correct for the multiple-testing bias of crowning the best of N swept
  strategies. `data[80]`'s `σ√(2 ln K)` decomposition is the *empirical, data-side
  twin* of the Deflated-Sharpe argument: large K (configs × params) and small n (one
  coin's history) push our crown toward **maximal inflation**. This is the gap the
  P0 work item closes.
- **It is additive, not a rewrite.** Everything here is reported *next to* the
  verdict — `N_eff`, DSR, PBO, MinBTL pass/fail as an "overfitting scorecard." The
  FROZEN classifier bands (`P5_SHARPE_FRAGILE = 0.0`, the 5-signal weakest-link
  composite in `robustness.rs`) are **untouched**. The crown eligibility rule
  (Fragile-and-not-benchmark ⇒ ineligible, in `rank.rs`) gains an *additional*
  disqualifier, it does not change the existing one.
- **Honest on expected-null.** The realistic expectation, hardened by full-text
  reads, is that **for the sub-0.4 net Sharpes a single coin produces, the correct
  haircut is >50% to near-total**, so the gate should crown almost nothing by
  construction `data[29][96][95]`. That is not a disappointment — it is the product
  thesis made measurable, and it is the *competitive advantage*: we are the framework
  that says "no" honestly.

---

## 4. Advantages for the project

- **Credibility / auditability.** A per-run overfitting scorecard (N_eff, DSR, PBO,
  MinBTL) turns "trust us" into a reproducible number the operator can audit. This is
  the literal embodiment of "traceable and plausible."
- **Robustness.** DSR/PBO/MinBTL are the trial-count-aware antidote to the one
  weakness the bootstrap doesn't cover (selection across configs), and they consume
  data we already compute — high value, contained blast radius.
- **Honesty.** The deflation tools make the "no active strategy robustly beats
  holding" thesis *visible per run*, rather than a claim in the docs. When a crowned
  config deflates below B&H, the operator sees exactly why.
- **Defensive, not speculative.** None of this is an alpha bet. It protects the
  verdict from a lucky single path — the most durable kind of value for an advisor
  whose entire pitch is measured honesty.

---

## 5. Problems and challenges

- **HARD CONSTRAINT — gate/bands FROZEN (additive only).** DSR/PBO/MinBTL/N_eff
  must be **additive surfaces** computed alongside the existing verdict. They must
  NOT alter `verdict_bands` or the weakest-link `classify_verdict` in
  `crates/backtest/src/bakeoff/robustness.rs`. The crown comparator in
  `rank.rs` may gain an *additional* eligibility disqualifier but must not weaken the
  existing Fragile rule. Any change here needs an ADR (the bands are
  `robustness-decision-rule-2026-05-30`-frozen) and the existing band tests must
  still pass bit-identically.
- **HARD CONSTRAINT — anchored report SHAs byte-immutable (119/119).** The
  overfitting scorecard adds *new* fields to the ranking report. Per ADR-0038 § D6,
  even a mechanical edit to an anchored `spec/*/reports/` file mutates its body-SHA
  and breaks the gate. The scorecard must land in **new** report fields/files and the
  anchored fixtures must be re-emitted via the § D6.b protocol (or left untouched
  with the new output gated behind an opt-in flag). Run `scripts/verify_anchors.sh`
  before AND after.
- **HARD CONSTRAINT — Decimal not f64.** The bake-off equity/return matrix is
  `Decimal`. DSR/PBO math is inherently f64 statistical work; the existing
  `bootstrap.rs` already crosses this boundary via `to_f64()` with documented
  fallbacks (`#![allow(clippy::float_arithmetic)]` for "statistical metric
  computations"). Keep the f64 island contained to the statistical layer; inputs and
  reported headline figures stay Decimal where they're financial quantities.
- **The M > T ill-conditioning is real and easy to get wrong.** Estimating `N_eff`
  from a raw pairwise-correlation matrix when there are more configs than window bars
  produces an *overfit* ρ̄ — the very bias we're correcting would re-enter through the
  correction. Cluster/dimension-reduce first. `data[80]`, `SYNTHESIS §P0.1`.
- **Threshold calibration is a judgment call, not a constant.** The famous t=3.0 was
  "never intended" as a universal cutoff. PBO's operating point and the DSR threshold
  are calibration choices; the honest posture is **report the deflated statistic, not
  a binary**, and derive any threshold from an explicit cost-asymmetry statement
  ("a false 'beats-hold' is N× costlier than a miss"). `SYNTHESIS §P0.5`.
- **Pure Reality-Check is too conservative to detect a real edge if one exists.**
  Studentize each config's excess-return-vs-hold by its bootstrap std (`z = w/σ̂`)
  before the max, SPA-style, or the gate will reject everything including a genuine
  edge — which would undermine the credibility of the "no" if it can't ever say
  "yes." `data[78]`, `SYNTHESIS §P0.8`.

---

## 6. Concrete next steps / candidate work items

Named, located, prioritized. **This file's P0 is the program's single highest-
leverage next action** (`SYNTHESIS §6`).

**P0 — Overfitting scorecard on the bake-off matrix (additive to the FROZEN gate).**
- **What:** compute and surface, per bake-off run, an overfitting scorecard:
  `N_eff` (cluster-first when M>T), DSR (exact formula, crown only if `DSR ≥ 0.95`
  AND beats B&H), nonlinear Harvey-Liu/BHY haircut Sharpe, PBO-via-CSCV, and a
  MinBTL pre-flight veto. Report deflated statistics, not a binary verdict.
- **Where:** `crates/backtest/src/bakeoff/robustness.rs` (PBO, SPA studentization —
  it already produces the per-path `DistributionSummary` and has the f64 island),
  `crates/backtest/src/bakeoff/rank.rs` (DSR crown rule, haircut, MinBTL veto,
  `N_eff`), plus the ranking report writer. The per-strategy return matrix + N are
  already in `bakeoff/mod.rs`/`sweep.rs`.
- **Sequencing (do in this order):** `N_eff` (cluster-first) → DSR crown rule →
  nonlinear haircut + composed critical-t ladder → MinBTL veto → PBO diagnostic →
  SPA studentization. Each is independently shippable.
- **Constraint:** FROZEN bands untouched; new report fields only (anchor protocol);
  needs an ADR for the additive crown-eligibility disqualifier.

**P0 — Pair the forward paper-trade with the bake-off-matrix deflation.**
- **What:** keep the forward paper-trade (genuine unseen data) but stop treating it
  as a sufficient OOS check on its own. Surface the CSCV/PBO + DSR/MinBTL numbers
  alongside the forward plan so the operator sees the trial-aware picture. This is
  the one place the research *amends* our design rather than endorsing it. `data[54]`.
- **Where:** the forward-plan output + ranking report (downstream of `rank.rs`).

**P1 — Leakage-audit checklist as a standing gate.**
- **What:** a test/lint that asserts, per indicator and per run: trailing-only
  windows (no centered), next-bar fills (never the signal bar's OHLC), train-only
  normalization, and R²-on-returns-never-price-levels. Selection/tuning boundary is
  the #1 gate — the window used to crown a config must be disjoint from the window it
  is judged on. `data[2][80][29]`.
- **Where:** new tests under `crates/backtest/tests/`; the next-bar-fill discipline
  is already implied by the engine but should be asserted, not assumed.
- **Note:** `core::pit::PitSeries` (ADR-0058) already makes look-ahead on *sidecar
  features* (funding/basis/on-chain) **unrepresentable** — the as-of join is the only
  query method, so future-data joins are a compile error. The leakage audit should
  confirm that *price/indicator* features route through the same discipline.

**P1 — No-alpha-gate standing regression test.**
- **What:** run the gate on synthetic no-alpha series (GBM/GARCH/OU) — it must refuse
  to crown, and DSR/PBO must flag any overfit pick. `crates/data/src/synth/gbm.rs`
  already exists for this. A standing test that the gate says "no" on pure noise.
- **Where:** `crates/backtest/tests/` consuming `data::synth::gbm`.
  (See the synthetic-and-monte-carlo sibling file for the generator side.)

**P2 — CPCV as an optional second robustness lens.**
- **What:** a combinatorial-purged-CV path-distribution as an *independent* check
  beside the return-block bootstrap (it natively yields PBO + DSR). Only worth it if
  the P0 scorecard proves insufficient. `data[21][79]`.
- **Where:** `crates/backtest/src/bakeoff/` (new module).

---

## 7. Open questions for analyst & architect

1. **Crown rule:** do we *gate* on DSR ≥ 0.95 / a PBO threshold (refuse to crown
   below it), or only *report* the deflated statistics next to the existing verdict?
   The hard constraint says additive; "report-only" is the safest first ship, but a
   PBO-based overfit-rejection disqualifier (additive to the Fragile rule) is the
   stronger product. Which?
2. **Threshold derivation:** do we adopt the ORATIO odds-ratio approach — derive the
   DSR/t threshold from an explicit "a false 'beats-hold' is N× costlier than a miss"
   statement — or hard-code a defensible constant and document it? `SYNTHESIS §P0.5`.
3. **`N_eff` estimator:** ONC clustering vs PCA vs the `N̄ = ρ̂+(1−ρ̂)·M` formula —
   and exactly how we dimension-reduce when M > T (which is the normal case for our
   sweeps). Architect call on the clustering primitive.
4. **Anchor protocol:** do the new scorecard fields land in a **new** report file
   (zero anchor risk) or extend the existing anchored ranking report (requires § D6.b
   re-emission of 119 fixtures)? This is a process decision with real blast radius.
5. **Decimal/f64 boundary:** is the existing `to_f64()` island in `bootstrap.rs` the
   sanctioned pattern for the DSR/PBO math, or do we want a dedicated statistical
   sub-crate to contain it?
6. **Does the verdict actually change?** Empirically: when we run the scorecard on
   the coins we've already crowned, do any survive DSR ≥ 0.95 / a sane PBO bar? If
   *none* do, that is a shippable headline ("the gate now proves the thesis per
   run"). If some do, we need to scrutinize them hard — they're the rare, fragile
   survivors `data[27]`.

---

## 8. What NOT to do / effort & blast radius

- **Do NOT touch the FROZEN bands or the weakest-link composite.** This is additive
  work. The moment a scorecard number feeds back into `verdict_bands` or
  `classify_verdict`, it stops being additive and triggers the full frozen-gate ADR
  process and band-test re-baselining.
- **Do NOT add purged/embargoed CV or CPCV machinery *now*** — they are only needed
  if/when we add ML/labels (a different program). The return-block bootstrap + DSR/
  PBO on the existing matrix is the right tool for our current rule-based strategies.
- **Do NOT "just halve the Sharpe."** The haircut is provably nonlinear — at N=200 a
  genuine top-3 is cut 37% / 100% / 49% (the middle one wiped). Use the real formula.
  `data[29][96]`.
- **Do NOT sell the forward paper-trade as a sufficient OOS proof.** It is necessary
  (genuine unseen data) but high-variance and trial-blind; always pair it with the
  matrix deflation. `data[54]`.
- **Effort / blast radius:** the P0 scorecard is **medium effort, contained blast
  radius** — it reads existing structures (`DistributionSummary`, the return matrix,
  N), adds f64 statistical functions in the already-f64 statistical layer, and writes
  *new* report fields. The two risk surfaces are (a) the anchor protocol for report
  changes and (b) the ADR for any additive crown-eligibility disqualifier. Both are
  process, not engineering, risk.
