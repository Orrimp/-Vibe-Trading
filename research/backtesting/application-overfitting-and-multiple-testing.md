# Application — Overfitting & Multiple-Testing (the P0 gate-upgrade core)

_Decision doc for analyst + architect. Source: `research/backtesting/knowledge.md`
(synthesis) + `research/backtesting/papers.md` (100-entry ledger; cited
`backtesting[N]`) + `research/SYNTHESIS.md` (cross-topic P0). This file is the
**multiple-testing / selection-bias** half of the backtesting research; the
realistic-cost half is in `application-cost-and-impact-modeling.md`._

> **One-line thesis of this doc:** the durable value here is **gate-hardening and
> credibility, not alpha**. The literature gives us a calibrated way to prove
> "we did not fool ourselves by crowning the best of N strategies" — which is
> exactly the *traceable & plausible* product promise. **But the code-grounded
> reality (`MAX_SWEEP_CONFIGS = 24`) makes the correction MILDER than the
> SYNTHESIS implies** — see §3/§5; that honesty is itself the point.

---

## 1. Summary of the research

The multiple-testing canon, read at primary-source depth, establishes a single
hard fact and a toolkit to act on it.

- **Backtest overfitting is the default, not the exception.** Trying even a
  modest number of strategy configs on one dataset produces a spuriously high
  in-sample Sharpe from pure noise; an uncorrected "best of N" Sharpe is
  upward-biased *by construction*. `backtesting[2][3]`. The blunt closer of
  `backtesting[3]`: a backtest that does not report N "makes it impossible to
  assess the risk of overfitting."
- **The selection bias has a closed-form correction — the Deflated Sharpe Ratio.**
  `DSR = PSR(SR₀)` deflates the benchmark from 0 to the *expected maximum* Sharpe
  over N trials, `SR₀ = E[max SR]`, derived via Extreme Value Theory, and folds
  in the crown's skew/kurtosis (which *bite* for crypto's fat tails).
  `backtesting[1][4]`.
- **Two parameters dominate, and both are easy to get wrong.**
  (a) **`V_SR`** in `E[max SR]` is the **cross-trial dispersion of the swept
  configs' Sharpes**, NOT the sampling SE of one Sharpe — the single most common
  error. `backtesting[1]`. (b) **N must be EFFECTIVE, not raw** — correlated
  configs (SMA-50 ≈ SMA-51) carry far fewer than N independent shots; raw count
  wildly over-deflates. `backtesting[1-App.3][86][39][29]`.
- **A model-free companion exists — PBO via CSCV.** From the per-config T×N P&L
  matrix: split time into S blocks, take all C(S,S/2) IS/OOS halves, find the
  IS-best config, measure its OOS rank, logit it; `PBO = fraction of splits where
  the IS-winner landed OOS-below-median`. Needs only the return series.
  `backtesting[2]`, operationalized as a crypto *selection filter* in
  `backtesting[21]`.
- **The haircut is NONLINEAR — "halve the Sharpe" is provably wrong.** A marginal
  Sharpe is gutted; a spectacular one is barely trimmed. Worked numbers: a single
  t=3.0 → adjusted t≈2.0; an annual Sharpe over T=240 → 0.32; at N=200 a real
  top-3 is cut 37%/100%/49% (the middle one wiped). `backtesting[29][87]`.
- **A minimum-backtest-length gate falls out of the same math.** `MinBTL <
  2·ln(N)/SR²_target` years: on 5y of data ≤45 *independent* configs may be tried;
  just 7 independent configs manufacture an IS Sharpe of 1 (OOS 0) on 2 years.
  Necessary, not sufficient. `backtesting[3][1]`.
- **Strictness must not cost all the power.** A White's-Reality-Check-style gate
  can be so conservative it misses a real edge — proven in `backtesting[93]` ("does
  anything beat GARCH(1,1)?"). The fix is to **studentize** (`z = w/σ̂`) and use a
  sample-dependent null (SPA / stepwise-SPA). `backtesting[7][8][99]`.
- **Resampling > single-path.** The path-distribution family (CPCV, bootstrap, our
  gate) is empirically the LEAST overfit; single-path walk-forward (our forward
  run's shape) is the WORST at preventing false discoveries.
  `backtesting[9][11][19][21]`. Our forward paper-trade therefore *supplements*,
  never *replaces*, an in-sample selection-bias correction. `backtesting[29][97]`.
- **Don't over-deflate into nihilism.** A high-deflated-t region survives even
  brutal correction (`backtesting[84]`: p-hacking cannot manufacture t≫4); report
  the *deflated statistic*, not a binary, leaving room for the rare real winner.
  `backtesting[81][100]`.
- **Adjacent rigor that holds up:** effective-N via ONC clustering of the candidate
  return matrix `backtesting[86]`; composed-strategy critical-t ladder
  `backtesting[80]`; derive the threshold from a loss function (ORATIO) rather than
  hard-coding 0.95 `backtesting[40]`; data-driven block length `backtesting[20]`;
  the difference-of-Sharpes test for crown-vs-hold `backtesting[34][35]`; the
  manipulation-proof MPPM score `backtesting[28]`; the Model Confidence Set as a
  ranking output `backtesting[61]`; selection-bias-corrected BBC-CV on machinery we
  already run `backtesting[64]`.

---

## 2. Possible solutions / what this research makes available

Concrete, buildable techniques (all consume artifacts the bake-off can produce):

1. **Deflated Sharpe Ratio (DSR)** — closed-form crown haircut.
   `DSR = Z[(ŜR − SR₀)·√(T−1) / √(1 − γ̂₃·ŜR + ((γ̂₄−1)/4)·ŜR²)]`,
   `SR₀ = √V_SR·((1−γ)·Z⁻¹[1−1/N_eff] + γ·Z⁻¹[1−1/(N_eff·e)])`, γ≈0.5772.
   `backtesting[1]`.
2. **Effective trial count `N_eff`** — a ladder of estimators: closed-form
   `N̂ = ρ̄+(1−ρ̄)·M` `backtesting[1-App.3]` → PCA `backtesting[3]` → entropy
   `backtesting[1-App.3]` → ONC clustering `backtesting[86]`; composed strategies
   inflate toward `(single)^k` `backtesting[80]`.
3. **PBO via CSCV** — model-free overfit probability + the free "performance
   degradation" slope. `backtesting[2]`.
4. **Nonlinear Sharpe haircut (Bonferroni / Holm / BHY)** — port the three exact
   p-value-adjustment formulas; map back to a haircut Sharpe. `backtesting[29]`.
5. **MinBTL pre-flight veto** — refuse-to-crown / low-confidence flag when the
   window is shorter than `2·ln(N)/SR²_target`. `backtesting[3]`.
6. **SPA / StepM studentization** — divide each config's excess-return-vs-hold by
   its bootstrap SE before the max; sample-dependent null. Recovers power; can also
   report the *set* of gate-passers. `backtesting[7][8][99]`.
7. **BBC-CV** — resampling alternative to closed-form DSR: block-bootstrap the
   per-config bars, re-select the winner each resample, score out-of-bag vs hold;
   the centre is a selection-bias-corrected expected performance. `backtesting[64]`.
8. **MPPM** — a manipulation-proof (concave power-utility) ranking score, directly
   comparable to hold's, that blocks crowning an "insurance-seller" with hidden
   negative skew. `backtesting[28]`.
9. **Difference-of-Sharpes test (crown vs hold)** — closed-form correlation-aware
   PSR/MinTRL `backtesting[34]` and Ledoit–Wolf studentized block-bootstrap CI
   `backtesting[35]`; decision = CI excludes zero. Long/flat strategies are highly
   correlated with hold ⇒ smaller SE ⇒ a real difference is *easier* to detect.
10. **Model Confidence Set** — bootstrap the SET of strategies tied for best; **if
    buy-and-hold is in the set, the crown does not robustly beat holding** — our
    thesis as a test; short windows correctly yield a large set. `backtesting[61]`.
11. **ORATIO loss-function threshold** — derive the crown bar from an explicit "a
    false 'beats hold' is N× costlier than a miss" statement instead of asserting
    0.95. `backtesting[40]`.

---

## 3. Relevance for the project

Our bake-off — sweep SMA/MACD/RSI/Bollinger/composed configs on one
`(coin, window)` and crown the best under the bootstrap gate — **is the exact
multiple-testing setup these papers describe.** The mapping is tight:

- The **FROZEN gate** is already a White-Reality-Check-style block bootstrap of
  "best beats benchmark" with **buy-and-hold always the benchmark + exempt**
  (`crates/backtest/src/bakeoff/rank.rs`: benchmark is always crown-eligible;
  FRAGILE = the only ineligible-to-crown flag). What it does NOT do is correct for
  the bias of having *selected* the best of N — that is the P0 gap.
- This directly serves the operator's stated goal — **"a framework for trading
  with traceable and plausible trading."** An overfitting scorecard (N_eff, DSR,
  PBO, MinBTL) printed next to the verdict is *the* credibility layer: it makes
  every crown auditable ("we tried N_eff≈k effective strategies; the crown's
  deflated confidence is X; here is the haircut") and reproducible (seed + window +
  N + KPIs recorded).
- **Expected-null is the honest baseline.** For the sub-0.4 net Sharpes a single
  coin realistically produces, even a *correct* DSR/haircut lands in the reject
  region — which is the thesis ("no active strategy robustly beats holding net of
  costs"). The scorecard does not chase alpha; it *explains why there usually
  isn't any* in language the operator can defend.

### ⚠ The code-grounded correction the SYNTHESIS missed (read this before scoping)

The SYNTHESIS P0 spec was written from the literature's "thousands of configs,
M > T, ill-conditioned correlation matrix" world. **Our code is not that world:**

- **`MAX_SWEEP_CONFIGS = 24`** (`crates/backtest/src/bakeoff/sweep.rs:62`). Raw N
  is tiny. After redundancy-clustering, **N_eff is plausibly single digits.**
- On any window long enough to bootstrap (months of daily or hourly bars),
  **T ≫ 24**, so the **"M > T ⇒ ill-conditioned matrix ⇒ MUST dimension-reduce
  first" mandate (`backtesting[1-App.3]`) almost certainly does NOT apply to us.**
  That mandate was the scariest line in the SYNTHESIS; at N=24 it is moot. Clustering
  is still *nice* (fairness), but it is not a correctness prerequisite here.
- Consequence: the DSR haircut at N_eff≈5–24 is **modest, not the "gut everything"
  picture.** The honest framing is "a small, calibrated haircut + a MinBTL sanity
  check," not "the gate now rejects almost everything." Over-selling the haircut
  would itself be a credibility error.
- **MinBTL actually bites more than DSR at our scale.** `2·ln(24)/SR²_target ≈
  6.4/SR²` years (≈6.4y at SR_target=1, ≈1.6y at SR_target=2). Short crypto
  windows will frequently fail MinBTL — and *that* is the cheap, defensible
  "can't crown with confidence" verdict.

This correction is not a reason to skip P0 — it is the reason to **scope P0
correctly** (cheap, additive, honestly-modest) rather than build the heavy
cluster-first machinery the SYNTHESIS implied.

---

## 4. Advantages for the project

What we gain (all credibility/honesty/robustness — *not* alpha):

- **Auditability.** Each crown ships with N_eff, DSR, PBO, MinBTL pass/fail — a
  reproducible paper-trail that backs the "traceable & plausible" promise.
- **A second, model-free witness.** PBO corroborates the bootstrap verdict from a
  completely different construction (selection-process overfit vs path-robustness).
  Agreement across two methods is the real robustness signal `backtesting[19]`.
- **The thesis, made into a test.** The Model Confidence Set "is hold in the set?"
  output (`backtesting[61]`) turns "nothing beats holding" from a slogan into a
  per-run, falsifiable statement.
- **Calibrated, not punitive.** N_eff (clustering) makes the haircut *fair* to a
  genuine winner instead of bluntly multiplying by raw N `backtesting[39][86]`;
  reporting the *deflated statistic* leaves room for the rare real edge
  `backtesting[84][100]`.
- **Defensible internals.** Surfacing the data-driven block length and the
  studentized statistic makes the gate's robustness claim reproducible rather than
  intuition-based `backtesting[8][20]`.
- **Cheap.** DSR, haircut, MinBTL, MPPM, difference-of-Sharpes PSR are all
  closed-forms over series we already compute; PBO/BBC-CV reuse the bootstrap
  machinery.

---

## 5. Problems and challenges (incl. which HARD CONSTRAINTS they bump)

- **The data-availability blocker (biggest feasibility issue).** PBO/CSCV, N_eff
  clustering, and BBC-CV all need the **full T×N matrix of per-bar returns across
  *all* swept configs.** Today (`crates/backtest/src/bakeoff/mod.rs:646`)
  `CandidateResult` carries a per-candidate `equity_curve` but **not a synchronized
  return matrix**, and the sweep likely retains only surviving candidates. **Pre-req
  for PBO/N_eff: capture the per-config bar-return series for the whole population.**
  DSR/haircut/MinBTL/MPPM do *not* need this (they run off the crown's series + N +
  the configs' Sharpes), so they are shippable first.
- **FROZEN-gate constraint (additive-only).** The robustness bands in
  `crates/backtest/src/bakeoff/robustness.rs` (`verdict_bands`, FRAGILE = p5
  Sharpe < 0, etc.) are frozen. The DSR/PBO scorecard must be **a new, parallel
  eligibility/down-rank input or a pure report annex — it must NOT mutate
  `classify_verdict` or the band constants.** Cleanest: an *additional*
  crown-eligibility predicate in `rank.rs` (e.g. "ineligible if DSR < threshold AND
  not benchmark"), gated behind a config flag, leaving the existing comparator
  intact. Architect must rule on whether a new eligibility gate counts as
  "additive" or is itself a frozen-rule change.
- **Anchor-safety constraint.** 119/119 anchored report files are byte-immutable.
  Any change to the *format* of an anchored ranking report re-emits its body-SHA and
  breaks `verify_anchors.sh`. The scorecard must go into **new** report
  fields/files, or follow the ADR-0038 §D6 re-emission protocol. Run
  `scripts/verify_anchors.sh` before and after.
- **Decimal-vs-f64 constraint.** Money is `Decimal`. DSR/PBO/N_eff are *statistics
  on log-returns*, and the bootstrap already converts equity→`f64` log-returns
  internally (`bootstrap.rs:230`). So the stats stay in `f64` (consistent with the
  existing gate) and **must never round-trip money through f64** — they consume
  returns, not balances. Low risk if it mirrors the existing bootstrap conversion.
- **ui-layering constraint.** `ui` must not depend on `strategy/exec/llm/models`.
  The scorecard is produced in `backtest` and rendered from the existing
  `Recommendation`/report types the UI already reads — **do not let the UI reach
  into strategy internals to recompute anything.**
- **Overfitting-trap meta-risk.** N_eff, the block length, the DSR threshold, and
  PBO's S are themselves *knobs*. Tuning them to make a favoured crown pass is a
  second-order data-snooping (`backtesting[90]`: hidden multiplicity even without
  fishing). They must be **pre-committed / frozen** like the gate, and ideally
  derived (ORATIO `backtesting[40]`, Politis–White `backtesting[20]`) not hand-set.
- **The over-correction risk.** A naive Bonferroni on raw N=24 would over-penalize
  our correlated configs and could manufacture a too-easy "hold wins" — itself
  dishonest. Use a dependence-aware route (clustering / studentized bootstrap), and
  report the deflated number, not a binary `backtesting[39][84]`.
- **Paper-only constraint** is unaffected — this is evaluation methodology, no
  execution surface.

---

## 6. Concrete next steps / candidate work items

Ordered by value-per-effort. "Where" = file under
`crates/backtest/src/bakeoff/`. All **additive**; none touches the frozen bands.

| # | Candidate | Where | Priority | Notes |
|---|---|---|---|---|
| A | **MinBTL pre-flight veto** + low-confidence flag | `rank.rs` (+ report) | **P0** | Cheapest, highest signal at our scale; `2·ln(N)/SR²`. Needs only N + window length. `backtesting[3]` |
| B | **DSR scorecard** for the crown (report-only, no eligibility change yet) | `rank.rs` / report annex | **P0** | Closed-form over the crown's series + configs' Sharpe dispersion (`V_SR`) + N_eff. Report the *deflated* number. `backtesting[1]` |
| C | **N_eff via correlation/clustering** of the swept configs | new `neff.rs` in `bakeoff/` | **P0** | Start with closed-form `ρ̄+(1−ρ̄)·M` `backtesting[1-App.3]`; upgrade to clustering `backtesting[86]`. At N=24, ill-conditioning is moot (§3). |
| D | **Nonlinear Sharpe haircut (Holm default, BHY secondary)** | `rank.rs` / report | **P0** | Port the three exact formulas `backtesting[29]`; show alongside DSR as a *range*. |
| E | **Capture the per-config bar-return matrix** in sweep output | `sweep.rs` + `mod.rs` (`CandidateResult`/a sibling) | **P0 (enabler)** | Unblocks F, G, BBC-CV. The one non-trivial plumbing change. |
| F | **PBO via CSCV** scorecard + down-rank high PBO | `robustness.rs` (additive) + report | **P1** | Needs E. Report PBO + degradation slope; disqualify operating point is a *calibration* choice, report don't binary. `backtesting[2][21]` |
| G | **SPA/StepM studentization** of the gate statistic | `bootstrap.rs`/`robustness.rs` | **P1** | `z = w/σ̂`; restores power so bad configs don't bury a real one. Pair with principled block sizing. `backtesting[7][8][93]` |
| H | **Difference-of-Sharpes CI (crown vs hold)** | `rank.rs` / report | **P1** | Ledoit–Wolf studentized + closed-form PSR_strategy(SR_hold). Exploits crown↔hold correlation. `backtesting[34][35]` |
| I | **MPPM companion score** | `rank.rs` / KPIs | **P1** | Power-mean over stored returns; compare crown's Θ vs hold's Θ. Blocks insurance-seller crowns. `backtesting[28]` |
| J | **Model Confidence Set output ("is hold in it?")** | `rank.rs` / report | **P2** | Bootstrap the tied-for-best set; flag hold ∈ set = thesis confirmed. `backtesting[61]` |
| K | **BBC-CV as a resampling deflation engine** | new module | **P2** | Needs E. Alternative to closed-form DSR; empirically slightly conservative (safe direction). `backtesting[64]` |
| L | **ORATIO-derived threshold** (replace hard-coded 0.95) | `rank.rs` config | **P2** | Makes the cost asymmetry explicit + auditable. `backtesting[40]` |

**Already done — do NOT re-propose** (verified in code, contra SYNTHESIS P0):

- **Data-driven block length is already computed** per series:
  `bootstrap.rs:139` calls `data::synth::block_length::politis_white_block_length`.
  SYNTHESIS item #10 ("confirm Politis–White is computed per-series and logged") is
  satisfied — the remaining gap is only *logging* it in the report. `backtesting[20]`
- **The bootstrap resample is already circular** (wrap-around `% n`, starts over the
  full `0..n`): `bootstrap.rs:265`. SYNTHESIS item "switch to circular blocks to kill
  edge bias" `backtesting[89]` is effectively satisfied.

**Highest-value single item:** **C (N_eff) feeding B (DSR), front-loaded by A
(MinBTL).** N_eff is the one decision that makes every downstream haircut *correct
rather than punitive*; DSR is the headline scorecard number; MinBTL is the cheapest
honest veto and bites hardest at our short-window / small-N reality. Ship A+B+C as
the first additive, report-only increment before touching eligibility or plumbing.

---

## 7. Open questions for analyst & architect

1. **Does a new DSR/PBO crown-eligibility predicate count as "additive," or is it a
   change to the FROZEN decision rule?** (The gate bands are frozen; eligibility
   lives in `rank.rs`.) If frozen, the scorecard ships **report-only** and the
   operator reads the haircut without it auto-vetoing — which may actually be the
   more honest, less-magic design. **Architect call.**
2. **N_eff estimator choice + freeze.** Closed-form `ρ̄+(1−ρ̄)·M` (ship today) vs
   clustering `backtesting[86]` (more rigorous)? Whichever is chosen must be
   **pre-committed** to avoid second-order snooping `backtesting[90]`. Given N=24,
   is the closed form sufficient forever, or do we want clustering headroom for a
   future larger sweep?
3. **DSR threshold: hard-coded 0.95, or ORATIO-derived?** The latter forces an
   explicit "false-crown : miss" cost ratio — more honest, more work. **Analyst
   call** (it is a product/values decision, not just stats). `backtesting[40][29]`
4. **FWER (Holm) vs FDR (BHY) as the default haircut.** FWER fits the skeptical
   "nothing beats hold" prior; BHY is the "set that would pass at 10% FDR" view.
   Default + secondary? `backtesting[29][5]`
5. **Is the per-config return-matrix capture (item E) worth the plumbing now**, or
   do we ship the closed-form-only scorecard (A/B/C/D) first and defer PBO/BBC-CV?
   (My recommendation: defer E; A/B/C/D deliver most of the credibility at a
   fraction of the blast radius.)
6. **Report/anchor format.** Where do the scorecard fields live so we do **not**
   re-emit any of the 119 anchored report SHAs? New report section vs new file vs
   ADR-0038 §D6 re-emission. **Architect + run `verify_anchors.sh`.**
7. **Should the forward paper-trade be explicitly re-labelled "confidence check,
   not verdict"** in the UI, given `backtesting[97]` (OOS low power) and
   `backtesting[11]` (walk-forward worst)? This is a copy/framing decision that
   reinforces the bootstrap-is-the-verdict design.

---

## 8. What NOT to do / out of scope

- **Do NOT oversell the haircut.** At `MAX_SWEEP_CONFIGS = 24` and N_eff in single
  digits, DSR is a *modest* correction. Presenting it as "the gate now rejects
  almost everything" would be inaccurate and would itself dent the credibility the
  feature is meant to build. The honest story is "small calibrated haircut + MinBTL
  sanity check + the thesis usually holds anyway."
- **Do NOT run naive Bonferroni on raw N.** Our configs are correlated; use N_eff or
  a dependence-aware bootstrap. `backtesting[39][29]`
- **Do NOT build the heavy cluster-first / ill-conditioned-matrix machinery the
  SYNTHESIS mandated** until/unless the sweep grows past T (it won't at N=24). §3.
- **Do NOT touch the frozen bands, the buy-and-hold-exempt rule, or any anchored
  report SHA.** Additive only.
- **Do NOT treat a forward "loss" as proof of no-edge** (low power) — and do NOT
  treat the in-sample crown score as the performance number; the deflated/forward
  numbers are the honest ones. `backtesting[97][62][91]`
- **Do NOT add this to the `ui` crate's dependency surface.** Compute in
  `backtest`, render existing report types.
