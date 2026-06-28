# Application — Anti-Overfitting & Search Discipline (the DSR / MinBTL / random-null kit that feeds the gate)

_Decision doc for analyst & architect. Distilled from `research/evolution/`
(100 papers; cite `evolution[N]`) and `research/SYNTHESIS.md`. This file covers the
**defensive** half of the topic — the closed-form selection-bias corrections, the
anti-overfitting fitness/training-subset techniques, and the matched-activity random
null. These are the **durable, low-risk export** of the whole evolution folder: they
FEED the P0 robustness-gate upgrade and the discipline around our existing FIXED
bake-off. The **generative / search** half (GP/GA/symbolic-regression/LLM-code-
evolution, and why it is a footgun) is the sibling doc
`application-automated-strategy-search.md`._

> **Our app:** a Rust single-coin crypto **advisor** (paper/sim, NOT advice, NOT
> live). Pick one coin + budget → bake off a FIXED pre-registered slate → rank under a
> FROZEN 1000-path moving-block-bootstrap gate (weakest-link verdict; FRAGILE ⇒ can't
> crown; **buy-and-hold always the benchmark and exempt**) → forward paper-trade.
> Thesis: **no active strategy robustly beats buy-and-hold net of costs.** Product
> sells **measured honesty** — "a framework for trading with traceable and plausible
> trading."

> **One-line verdict for the impatient reader:** our grid-sweep over `(coin, window)`
> *is itself a search*, so it must be **significance-charged**. This literature hands
> us the exact closed forms to do it — the **Deflated Sharpe Ratio** and **Minimum
> Backtest Length** — plus a cheap **matched-activity random null**. All inputs are
> already computed every bake-off run. This is the highest-value, lowest-risk work
> item in the entire research program, and it is **additive** to the FROZEN gate.

---

## 1. Summary of the research

**The core problem this kit solves.** Crowning "the best of N" strategies/params on a
window is multiple-hypothesis testing. Bailey & López de Prado **prove** (Extreme
Value Theory) that the expected *maximum* in-sample Sharpe across N zero-skill trials
grows with N, and that the in-sample winner is *negatively* correlated with OOS return
(`evolution[29][98]`). A short window + a big sweep is *guaranteed* to surface a
spurious champion. Our bootstrap tests each curve's robustness but does **not** yet
correct for the selection bias of picking the best of many. This kit closes that gap.

### 1a. The Deflated Sharpe Ratio (DSR) — exact, captured first-hand from `evolution[98]`

Built on the Probabilistic Sharpe Ratio. The DSR is the probability the *true* Sharpe
exceeds a threshold `SR₀`, given sample length `T`, return skew `γ̂₃`, and kurtosis
`γ̂₄`:

```
DSR = Z[ (ŜR − SR₀)·√(T − 1) / √(1 − γ̂₃·ŜR + ((γ̂₄ − 1)/4)·ŜR²) ]
```

where `Z[·]` is the standard-normal CDF. The denominator is the **non-Normal standard
error** of the Sharpe estimator — left-skew and fat tails *inflate* it. The
deflation comes from setting `SR₀` to the **expected maximum Sharpe under the null
SR=0**, derived via EVT:

```
SR₀ = √V[{ŜRₙ}] · ( (1 − γ)·Z⁻¹[1 − 1/N] + γ·Z⁻¹[1 − (1/N)·e⁻¹] )
```

with `γ ≈ 0.5772` (Euler-Mascheroni), `Z⁻¹` the inverse-normal CDF, `N` the number of
(independent) trials, and `V[{ŜRₙ}]` the **variance of the Sharpe estimates across the
trials**. So DSR deflates the observed Sharpe by **five extra inputs**: skew,
kurtosis, sample length `T`, across-trial Sharpe variance `V`, and trial count `N`.
**Crown only if `DSR ≥ 0.95`** ("true SR > 0 at 95% after deflation").

**The worked example is the load-bearing crypto lesson.** A strategist finds an
annualized `ŜR = 2.5` over a 5-year daily sample. With `N = 100`, `V = ½`, skew `−3`,
kurt `10` → `SR₀ ≈ 0.1132` (non-annualized) → **`DSR ≈ 0.90 < 0.95` ⇒ NOT a
discovery.** Had the same Sharpe come from only `N = 46` trials, DSR would pass; with
**Normal returns** it clears at `N = 88` trials. **Fat tails roughly *halve* the
tolerable number of trials** (88 → 46). Implication for us: on heavy-tailed crypto we
should be *more* suspicious of large sweeps, the opposite of the naive "more configs =
more chances." And high across-trial Sharpe variance `V` (an undisciplined,
scattershot search) *raises* `SR₀` — a built-in penalty for sloppy search.

### 1b. The Minimum Backtest Length (MinBTL) — exact, captured first-hand from `evolution[29]`

A one-line pre-flight: given a target Sharpe and `N` trials, how long must the window
be before a Sharpe-1 winner could reflect real skill rather than selection luck?

```
MinBTL ≈ [ (1 − γ)·Z⁻¹(1 − 1/N) + γ·Z⁻¹(1 − 1/(N·e)) ]² / E[max SR]²      (γ ≈ 0.5772)
```

For target annualized `E[max SR] = 1`, the years-vs-trials table is the lookup:

| N (trials) | MinBTL (years, SR target = 1) |
|---|---|
| 10 | ~0.5 |
| 50 | ~1.5–2 |
| 100 | ~2.5–3 |
| 1000 | ~5–6 |

Below MinBTL, a Sharpe-1 winner is achievable with **no real edge**. (SYNTHESIS notes
the looser bound `MinBTL ≈ 2·ln(N)/SR²_target` and the exact form's stricter "5 yr ⇒
≤45 configs" — calibrate against whichever is more conservative for the coin.) **Refuse
to crown when the window is shorter than MinBTL(N).**

### 1c. The matched-activity random null — captured first-hand from `evolution[30][31]`

Beyond "beat B&H," require a tuned/crowned pick to beat **random trading**. Chen &
Navet pin down the *fair* construction: the random null must be **matched-activity —
same trade frequency AND same time-in-market (intensity)** as the candidate; otherwise
the comparison is contaminated by differing transaction-cost exposure (a strategy could
"win" merely by trading less). The *search-vs-search* comparison must be
**equal-intensity** — pit our N-config bake-off against a random search drawing ~N
random configs (the same "charge the search budget" logic as DSR/MinBTL). Clean
diagnostic: **if random search beats a lottery null but our optimized pick does not,
the optimizer is overfitting, not finding edge.** A config that beats < ~50% of
matched random sets is overfit, not skilled. The crypto walk-forward bootstrap-vs-
random study quantifies the same idea: optimized EMA params beat random params only
8-13% of the time (`evolution[5]`).

### 1d. Anti-overfitting fitness & training-subset techniques (portable to any search)

- **Difficulty-weighted training-subset rotation** — confirmed by two first-hand
  reads. `evolution[92]` (GP for implied vol) rotates the training subset every g
  generations and *up-weights the subsets the search currently fits worst* (the
  Adaptive-Random "ARSS" variant wins on OOS MSE); `evolution[10]` (vectorial GP for
  trading) independently uses the same idea (random-buffer sampling + segment the
  training set into 3 parts, evaluate one per generation). The principle — never let
  the optimizer score a candidate on one fixed slice; rotate across sub-windows and
  concentrate scrutiny on the *hardest* ones — is a **direct cousin of our weakest-link
  moving-block bootstrap.** Honest caveat from `evolution[10]`: even *with* this
  regularization, evolved strategies showed **frequently negative OOS fitness despite
  positive training fitness** — it abates, doesn't cure (`evolution[21]`).
- **GT-Score: bake the overfitting penalty INTO the objective** (`evolution[68]`,
  full read). `GT-Score = (μ · ln(z) · r²) / σ_d` (μ = mean return; z = excess-return-
  over-B&H significance Z-score; r² = path consistency; σ_d = downside deviation),
  implemented piecewise to *penalize* configs that don't clear B&H beyond sampling
  noise. **Correction from the full read:** it *does* use a B&H benchmark (μ_m, inside
  z) and *does* include a cost-sensitivity check (0–10 bps/side). Real numbers:
  walk-forward generalization ratio **0.365 (GT-Score) vs 0.185 baseline**, but
  GT-Score's *raw* OOS return is **lower** (43.6% vs 46–50%) — it explicitly trades
  return for retention, and even the best objective retains only **~37% of training
  return OOS** (a sobering anchor for how illusory in-sample performance is). Two
  portable imports: (a) a **generalization ratio (validation ÷ training return) as a
  per-candidate overfitting metric** — any pick far below 1 is overfit; (b) a
  **B&H-relative significance penalty baked into ranking** (close to our weakest-link
  verdict). Decisive caveat: the **parametric Z-score breaks under fat tails** — so for
  crypto the *non-parametric* version (our bootstrap + DSR) is the honest
  implementation.
- **Trinary no-trade signal + coherent CVaR / conditional-Sharpe fitness**
  (`evolution[85][99]`) — an explicit *no-trade* action lets a rule abstain when the
  signal is weak (cousin of a cost-aware execution filter and the breakeven-win-rate
  screen), and a CVaR-based fitness is more honest than mean-variance for fat-tailed
  crypto. Turnover/tail knobs, not alpha claims.
- **The breakeven-win-rate kill-switch** (`evolution[35]`) — `W_BE = (1 + C_ratio)/(1 +
  R)` where `C_ratio` = round-trip cost / target profit, `R` = reward-to-risk. At ~0.1%
  costs and 1% target, `W_BE ≈ 55%`. A candidate whose *implied* win rate can't clear
  `W_BE` under our cost model is structurally doomed — a cheap pre-screen *before* the
  bootstrap runs.

### 1e. The complementary corrections (use the trio, not one)

The three search-size-aware corrections are **complementary**, not redundant
(`evolution[50][53][98]`; SYNTHESIS P0):
- **DSR** = closed-form *parametric* Sharpe deflation (`evolution[98]`).
- **PBO via CSCV** = *non-parametric* Probability of Backtest Overfitting via
  Combinatorially-Symmetric Cross-Validation; AutoQuant applies it to crypto perps and
  still finds **substantial residual overfitting after careful tuning**, framing itself
  as "validation infrastructure, not proof of persistent alpha" — the closest external
  mirror of our mission (`evolution[53]`).
- **White's Reality Check** = *bootstrap-over-the-universe* (`evolution[50]`); Holm
  correction flipped a >65%-return BTC strategy to "not significant vs B&H"
  (`evolution[32]`, cross-referenced).

**And the empirical proof neither correction alone suffices:** STW's best DJIA rule
**survived the data-snooping correction in-sample 1897-1986 yet was insignificant OOS
1987-1996** (Reality-Check p ≈ 0.12) and earned nothing on S&P futures (`evolution[50]`).
So the verdict must rest on held-out / resampled / cost-net performance *and* a
search-size correction — which is exactly why our gate keeps **both** the bootstrap
**and** (with this kit) the DSR/PBO/MinBTL corrections.

### 1f. Event-driven re-baking (the "update when required" principle)

`evolution[97]` ("update when required") + coevolution/AMH (`evolution[35][36][94]`):
make any re-bake of a crowned pick **event-driven, not calendar-driven** — trigger only
when a monitored statistic (realized-vs-expected divergence, regime-change flag,
edge-decay signal) crosses a threshold. Calendar re-baking churns costs *and* draws
fresh overfit winners each cycle (`evolution[29]`).

---

## 2. Possible solutions / what can be done with this research

1. **Add an additive "overfitting scorecard" to the gate (Recommended, P0).** Compute
   and surface `{N_eff, DSR, PBO, MinBTL pass/fail}` next to the existing FRAGILE/
   MARGINAL/ROBUST verdict — **without touching the FROZEN classifier bands.** The
   crown rule becomes: existing weakest-link verdict **AND** `DSR ≥ 0.95` **AND** beats
   B&H (B&H exempt). Report the *deflated* statistic, not a new binary band.

2. **Add a MinBTL pre-flight veto (Recommended, P0, cheapest).** Before crowning,
   assert `window_length_T ≥ MinBTL(N)`. One-line guard using the table in §1b.

3. **Add a matched-activity random-null sub-test (Recommended, P1).** Alongside "beat
   B&H," require the crowned pick to beat a matched-activity random null and the search
   to beat ~N random configs. Cheap; catches lucky-timing edges.

4. **Pre-regularize any optimizer with difficulty-weighted subset rotation (P1/P2).**
   If/when we run any search (including a finer grid), rotate candidates across regime
   blocks and up-weight the blocks each does worst on (`evolution[10][92]`).

5. **Report a generalization ratio per candidate (P1).** validation ÷ training return;
   flag anything far below 1 (`evolution[68]`).

6. **Add a breakeven-win-rate pre-screen (P2, optional).** Reject candidates whose
   implied win rate can't clear `W_BE` before the bootstrap runs (`evolution[35]`).

---

## 3. Relevance for the project

**Directly relevant — this is the implementation-ready core of the P0 roadmap.**

- **It is the formula behind the planned gate upgrade.** SYNTHESIS §2 P0 names exactly
  these corrections, and five independent topic reviews flagged the same hole. The
  evolution deep-read is where the **exact closed forms** were captured first-hand
  (`evolution[98]` DSR Eq.2 + expected-max-SR Eq.1; `evolution[29]` MinBTL + the
  years-vs-trials table). This turns "we should correct for selection bias" into
  codeable arithmetic.

- **Every input is already computed.** The mapping into our pipeline is direct:
  - `N` ← bake-off config / param count (the trials).
  - `V[{ŜRₙ}]` ← variance of the Sharpe ratios across all baked-off configs (we
    already compute per-config Sharpe).
  - `T` ← window length in periods.
  - `γ̂₃, γ̂₄` ← realized skew / kurtosis of the crowned strategy's return series.

- **It strengthens "traceable & plausible."** A per-run overfitting scorecard
  (`N_eff/DSR/PBO/MinBTL`) is exactly the kind of auditable, honest artifact the
  product sells — it shows *why* a pick was or wasn't crowned, in numbers a skeptical
  user can re-derive. AutoQuant's framing — "validation infrastructure, not proof of
  persistent alpha" (`evolution[53]`) — is our mission stated by an external paper.

- **It hardens the expected-null.** On the sub-0.4 net Sharpes a single coin
  realistically produces, the nonlinear haircut is >50% to near-total (SYNTHESIS P0
  item 3) — so a correctly-deflated gate should **crown almost nothing by
  construction**, which is the honest outcome, not a bug.

---

## 4. Advantages for the project

1. **Highest value / lowest risk in the program.** Purely additive, no new free
   parameters, no live exposure, and it *reinforces* (never weakens) the honest thesis.
   Contrast with the sibling doc's search engine, which is high-risk and expected-null.

2. **Closes the one design gap the deep read flagged.** The forward paper-trade alone
   is insufficient (single hold-out = high variance, blind to trial count,
   `evolution`/SYNTHESIS via the PBO paper). Pairing it with CSCV/PBO + DSR/MinBTL on
   the bake-off matrix is the one place the research *amends* our design — and this kit
   is that amendment.

3. **Crypto-calibrated.** The DSR worked example proves fat tails shrink the survivable
   trial budget (`evolution[98]`) — so the correction is *tighter* exactly where our
   assets are heavy-tailed. The kit makes the gate smarter about BTC/ETH/SOL
   specifically, not just generically stricter.

4. **A cheap, powerful new sub-test.** The matched-activity random null
   (`evolution[5][30][31]`) catches edges that look good vs B&H purely from lucky
   timing — a class the current gate can miss — for very little code.

5. **A re-bake policy that doesn't churn.** Event-driven "update when required"
   (`evolution[97]`) gives a principled trigger that avoids both stale crowns and the
   cost/overfit churn of calendar re-baking.

---

## 5. Problems and challenges

1. **`N_eff` is non-trivial when configs > window bars (our exact situation).** With
   `M > T` the correlation matrix is ill-conditioned and the naive `N̄ = ρ̂ + (1−ρ̂)·M`
   is itself overfit — we **MUST cluster / dimension-reduce (ONC or PCA) before
   estimating `N_eff`** (SYNTHESIS P0 item 1, now a primary-source requirement). Getting
   this wrong silently mis-deflates the gate.

2. **The parametric Z-score breaks under fat tails.** GT-Score's parametric
   significance and even DSR's Normal approximation degrade on heavy-tailed crypto
   (`evolution[68][98]`). The honest implementation pairs the parametric DSR with the
   *non-parametric* bootstrap + PBO — never the closed form alone.

3. **Threshold calibration is a judgment call.** `DSR ≥ 0.95` and PBO operating points
   are calibration choices, not laws (`evolution[50][53]`; SYNTHESIS P0 item 6 — derive
   the bar from an explicit cost-asymmetry "ORATIO" statement rather than hard-coding
   t=3.0). Document the rationale; report the deflated statistic, not just a pass/fail.

4. **The random null must be constructed correctly or it's worthless.** Matched on
   trade frequency AND time-in-market; equal-intensity search-vs-search
   (`evolution[31]`). An unmatched null contaminates the comparison with cost-exposure
   differences and gives a false verdict either way.

5. **Abates, never cures.** Every technique here *reduces* overfitting; none eliminates
   it (`evolution[10][21][68]`). The gate should still expect the honest null and the
   product should still say "B&H usually wins."

**HARD CONSTRAINTS this work must respect:**

- **Gate / bands are FROZEN — additive only.** The DSR/MinBTL/PBO/random-null are new
  diagnostics layered *beside* the existing `RobustnessFlag` classifier and
  `verdict_bands` in `crates/backtest/src/bakeoff/robustness.rs`. We do not edit the
  FRAGILE/MARGINAL/ROBUST band constants (`P5_SHARPE_FRAGILE`, `P50_SHARPE_FRAGILE`,
  …). The crown rule *composes* the new checks with the existing verdict; it does not
  rewrite it.
- **`Decimal` not `f64` for money.** Equity/PnL/drawdown stay `Decimal` (the gate's
  `max_drawdown`/equity math is `Decimal`). DSR/skew/kurtosis are statistical scalars
  computed in `f64` behind the same boundary the existing `sharpe`/`calmar` helpers
  already use (`crates/reports/src/render/risk_metrics.rs`) — acceptable, consistent
  with current practice.
- **Anchored report SHAs byte-immutable (119/119).** The overfitting scorecard emits
  *new* report sections/files; it must not mutate any anchored report body (run
  `scripts/verify_anchors.sh` before and after touching anything under
  `spec/*/reports/`).
- **`ui` must NOT depend on `strategy`/`exec`/`llm`/`models`.** The scorecard is plain
  data produced in `backtest` and fed to `ui` via the existing leaderboard runner
  pattern — no new cross-crate dependency.
- **Paper-only; pre-registration is the standing defense.** This kit *operationalizes*
  pre-registration: it charges the search budget N against significance so that even
  our FIXED slates can't accidentally crown a selection-bias artifact.

---

## 6. Concrete next steps / candidate work items

Named, located, prioritized. All additive to the FROZEN gate.

- **[P0] `N_eff` with cluster-first-when-M>T.** Estimate effective trial count from
  the per-config return matrix via ONC clustering / PCA (NOT naive ρ̄ when configs >
  bars). Location: `crates/backtest/src/bakeoff/robustness.rs` (feeds DSR's `N`).
  **Do first** — DSR/MinBTL both consume it.

- **[P0] DSR crown rule.** Implement Eq.2 + expected-max-SR Eq.1 (§1a). Inputs:
  `N ← N_eff`, `V ← var of per-config Sharpes`, `T ← window periods`, `γ̂₃/γ̂₄ ←
  crowned-strategy skew/kurtosis`. Crown only if `DSR ≥ 0.95` AND existing weakest-link
  verdict passes AND beats B&H (B&H exempt). Location:
  `crates/backtest/src/bakeoff/{robustness.rs,rank.rs}` + ranking report. **FROZEN
  bands untouched.** `evolution[98]`.

- **[P0] MinBTL pre-flight veto.** Assert `T ≥ MinBTL(N_eff)` before crowning (§1b
  table / closed form). One-line guard. Location:
  `crates/backtest/src/bakeoff/rank.rs`. `evolution[29]`.

- **[P0] PBO via CSCV diagnostic.** Compute PBO from the same T×N matrix (logit-rank
  over combinatorial splits); report and disqualify high values (operating point is a
  calibration choice). Location: `crates/backtest/src/bakeoff/robustness.rs`.
  `evolution[53]`.

- **[P0] Surface a per-run overfitting scorecard** `{N_eff, DSR, PBO, MinBTL
  pass/fail}` next to the verdict in the ranking report. Location: ranking report
  render path consuming `bakeoff` output (no `ui → strategy` dependency).

- **[P1] Matched-activity random-null sub-test.** Matched on trade frequency AND
  time-in-market; search-vs-search against ~N random configs (§1c). Location:
  `crates/backtest/src/bakeoff/robustness.rs`. `evolution[5][30][31]`.

- **[P1] Generalization ratio per candidate.** validation ÷ training return; flag
  far-below-1. Location: ranking output. `evolution[68]`.

- **[P2, optional] Breakeven-win-rate pre-screen.** Reject candidates whose implied win
  rate can't clear `W_BE = (1 + cost/target)/(1 + R)` before the bootstrap.
  Location: `crates/backtest/src/bakeoff/rank.rs` (cheap pre-filter). `evolution[35]`.

- **[P2, optional] Event-driven re-bake trigger.** "Update when required" — re-bake a
  crowned pick only on a divergence/regime/decay signal, not on a calendar. Location:
  forward-plan / watch path. `evolution[97]`.

- **[P1, standing] Validate the gate on synthetic no-alpha series.** Feed GARCH/OU/
  Heston no-edge paths; assert the gate refuses to crown and DSR/PBO flag overfit
  picks. Location: `crates/backtest/tests/` (standing regression). Complements the
  anti-pattern fixtures in the sibling doc.

---

## 7. Open questions for analyst & architect

1. **`N_eff` method:** ONC clustering vs PCA vs a simpler conservative bound — which is
   robust enough for our `M > T` regime, and how is it tested? (Primary-source
   requirement, not optional.)
2. **Threshold derivation:** hard-code `DSR ≥ 0.95`, or derive the bar from an explicit
   cost-asymmetry statement ("a false 'beats-hold' is X× costlier than a miss",
   SYNTHESIS P0 ORATIO)? The latter is more honest but needs an operator input.
3. **Scorecard vs binary:** report the deflated *statistic* (recommended — preserves
   nuance) or collapse to pass/fail? How does it render for a retail user without
   implying false precision?
4. **Budget accounting across stages:** how is N counted when the grid sweep and any
   future search both contribute trials, so DSR reflects *total* multiple testing?
5. **Forward-trade pairing:** confirm the design amendment — keep the forward
   paper-trade (genuine unseen data) but *pair* it with CSCV/PBO + DSR/MinBTL on the
   bake-off matrix (single hold-out alone is insufficient per the PBO paper).
6. **Block length:** is the moving-block length computed per `(coin, window)`
   correlogram (Politis–White, ADR-0063) and logged? A too-short block under-estimates
   long-run variance → over-narrow CIs, which would *under*-deflate the gate.

---

## 8. What NOT to do / out of scope

- **Do NOT edit the FROZEN FRAGILE/MARGINAL/ROBUST bands.** Every item here is additive
  — a new layer beside `verdict_bands`, never a rewrite.
- **Do NOT use the parametric closed form alone on crypto.** Pair DSR with the
  non-parametric bootstrap + PBO; the parametric Z-score breaks under fat tails
  (`evolution[68][98]`).
- **Do NOT treat passing the corrections as proof of alpha.** Even AutoQuant, after
  careful tuning + PBO, calls itself "validation infrastructure, not proof of
  persistent alpha" (`evolution[53]`). The corrections *reduce false positives*; they
  do not manufacture a real edge that isn't there.
- **Do NOT calendar-re-bake.** It churns costs and draws fresh overfit winners each
  cycle; make re-baking event-driven (`evolution[29][97]`).
- **Do NOT skip the `N_eff` cluster step in the M>T regime.** A naive ρ̄ on an
  ill-conditioned matrix silently mis-deflates the gate (SYNTHESIS P0 item 1).
