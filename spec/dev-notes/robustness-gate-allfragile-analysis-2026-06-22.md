---
slug: robustness-gate-allfragile-analysis-2026-06-22
status: draft
owner: analyst
updated: 2026-06-22
tags: [robustness, allfragile, calibration, statistical-honesty, benchmark-exemption, classify-verdict, advisor, product-thesis, decision-support, durable-over-quick]
related:
  - spec/dev-notes/robustness-decision-rule-2026-05-30.md
  - spec/dev-notes/onchain-vs-conclude-fork-2026-06-08.md
  - spec/architecture/adr/0063-ensemble-vote-seam-and-robustness-gate-activation.md
  - spec/advisor-bakeoff-ranking/feature.md
  - spec/advisor-ensemble/feature.md
  - spec/runbooks/passive-baseline.md
  - spec/product.md
  - crates/backtest/src/bakeoff/robustness.rs
  - crates/backtest/src/bakeoff/bootstrap.rs
  - crates/backtest/src/bakeoff/rank.rs
---

# Robustness gate "AllFragile on real crypto" — product / research / honesty analysis

> **Mandate (analyst decision-support, FILES ONLY — no code, no `classify_verdict`
> change, no `spec/*/reports/` touch, no git).** The complete wired advisor
> bake-off on real BTCUSDT H1-2024 (`RobustnessMode::Bootstrap{1000}`, 7-arm
> field) flags ALL 7 arms — including buy-and-hold — `Fragile`, so the outcome is
> always `AllFragile` and the Robust/Marginal/Fragile discrimination (F8 badges +
> the "robust crown") never visibly manifests. This note answers: is that the
> honest truth or a calibration artifact; is it a feature or a product failure;
> and what the advisor should communicate long-term. The architect owns the
> technical/classifier angle in parallel. Every claim traces to source inspected
> this session (the rule's pre-registration note, ADR-0063, the runbook, the three
> bake-off `bakeoff/` modules).

---

## 0. TL;DR — verdict, framing, recommendation

**Q1 verdict: BOTH, cleanly separable — and the separation is the whole fix.**
"All ACTIVE arms Fragile on real crypto" is the **honest truth**, faithfully
reproducing a result this program has banked ~10 times under the same frozen
rule. "**Buy-and-hold also Fragile**" is a **category error** — a calibration
artifact of running the benchmark through a gate that was pre-registered,
defended, and historically applied to judge *candidate active strategies*, never
the baseline they are measured against. The two failures look identical on the
leaderboard but have opposite epistemic status: one is the machine working, the
other is the machine pointed at the wrong target.

**Q2 framing: the always-`AllFragile` outcome is ~85% honest-thesis-manifesting
and ~15% over-stated-into-nihilism.** The honest core ("no active strategy
cleared the bar; hold is the least-bad") is the negative-result thesis correctly
firing and is a *feature*. The nihilist tail ("...and even holding is fragile")
is the artifact — it converts a precise, defensible statement ("active edges
don't survive resampling; the baseline is the baseline") into a useless
undifferentiated "everything is bad," which is both *less true* and *less useful*.
The gate is not useless as a discriminator **in principle** (it discriminates
fine on the active field — see §2.3); it is rendered mute **in this presentation**
because the benchmark, which structurally cannot be "robust" on a single volatile
asset, is dragged into the same pass/fail and collapses every run to the
all-fragile branch.

**Recommendation: (C-then-A) — a layered, durable fix, NOT a threshold relaxation.**
1. **Exempt the benchmark from the candidate fragility verdict** (the §3-B "B1"
   carve-out only — benchmark judged on its own axis, never robustness-gated as a
   candidate). This is not "loosening the bar"; it is correcting a
   *type confusion* the codebase already half-encodes (`is_benchmark` exists;
   `BenchmarkWins` exists). **Architect-owned, needs an ADR amendment to the
   ADR-0059 § D4 / ADR-0063 freeze.**
2. **Add a relative robustness framing** (C): rank the active field by a
   robustness *percentile / ladder* so "least-fragile active arm" is legible even
   when none clears the absolute ROBUST band. The absolute bands stay frozen and
   authoritative; the relative view is additive narration.
3. **Fix the UX copy** (A): the headline becomes "**Nothing cleared the
   robustness bar — holding is the least-bad on this window**," the benchmark is
   shown as the *baseline* (not a co-fragile loser), and the unanimous-vote arm's
   0 trades is surfaced as "**sat in cash — consensus never reached**," not a
   silent zero.

The thing to **NOT** do (option B as a *threshold* move): relax the Fragile
bands, or make them asset-class-aware to manufacture a Robust crown on crypto.
That trades the program's one hard-won asset — a pre-registered rule no result
ever gamed — for a cosmetic distinction, and is the slippery slope the entire
2026-05-30 pre-registration discipline exists to prevent (§3-B).

---

## 1. Q1 — honest truth vs calibration artifact: the rule's documented origin as evidence

The question is decided by **what the rule was designed for**, and the spec
record is unambiguous on every sub-point.

### 1.1 What the rule was DESIGNED to discriminate: overfit ACTIVE strategies

The Fragile thresholds are not generic "is this equity curve nice" bands — they
are a **curve-fit detector for active strategies**, pre-registered before any
number was seen (`robustness-decision-rule-2026-05-30.md`, the
PRE-REGISTRATION NOTICE). Three load-bearing quotes establish the design intent:

- The headline signal, **p5 Sharpe < 0 → Fragile**, is justified explicitly as
  *the curve-fit signature*: "a curve-fit strategy has a high median and a
  **negative** p5 (it works only when the path cooperates)" (§3.1). The whole
  point of the p5 floor is to catch a strategy whose backtested Sharpe is "1.4
  *only-on-that-exact-ordering-of-2023*" (§1). That is a statement about a
  *strategy with tunable parameters*, not about a parameter-free hold.
- The composite is a **weakest-link** rule deliberately so that "a strategy that
  is excellent on 4 signals and loses money in the p5 tail is not robust" (§4
  step 3). This is built to deny a crown to an *active arm* that looks good on the
  median but is lucky in the tail. Buy-and-hold has no median-vs-tail story to
  catch — it is long the asset, full stop.
- ADR-0063 (the gate-activation ADR) states the activation rationale in one
  sentence: "Shipping **ensembles — the easiest-to-overfit candidates** — into a
  gate that cannot reject them would convert the product from 'measured
  robustness' into 'data-mining with extra steps'. So F8 bundles gate activation."
  The gate exists **to reject overfit candidates**. Its design target is named.

### 1.2 What it was calibrated ON: daily active θ-surfaces, NOT hourly buy-and-hold

The bands' magnitudes were validated against **active-strategy parameter surfaces**:

- `passive-baseline.md` records the canonical validation: a 16-cell SMA/RSI/BBands
  θ-surface, "**All 16 scored FRAGILE** — every candidate's p5 Sharpe is negative
  ... including the +97.0 pp [return strategy]" (lines 156, 179). This is the
  proof the rule *was meant to fail high-return-but-path-dependent active arms* —
  a +97 pp strategy that is still Fragile is the rule working as designed.
- The `horizon-retest-robustness` reports that exercised the bands are
  **daily** and **4h** time-series-momentum and carry θ-surfaces (e.g.
  `...-ts-horizon-daily-theta-surface-2024-block-bootstrap-real-fy`). The
  pre-registration §0 itself scoped the rule to "**v1 cross-sectional momentum at
  a single fixed θ\*** over N=500 ... block-bootstrap paths of **2023-FY**
  returns." The calibration substrate was *active, parameterised, often daily,
  multi-symbol* — never a single-asset hourly buy-and-hold.
- Critically: **buy-and-hold was the benchmark every one of those surfaces was
  scored AGAINST, not a row in the surface.** `passive-baseline.md` lines 7/25/27:
  "the benchmark every robustness surface is scored against"; "The BH control has
  existed throughout the robustness program **as the benchmark**"; "'Ship passive'
  promotes it from *benchmark* to *the strategy*." Until the F8 bootstrap
  activation, buy-and-hold was **never robustness-judged as a candidate at all.**
  The first time the benchmark is fed through `classify_verdict` on equal footing
  with candidates is the advisor bake-off — and that is exactly where the artifact
  appears.

### 1.3 Is flagging the BENCHMARK Fragile a meaningful statement or a category error?

It is a **category error**, and the distinction is precise:

- **The literal statement is true.** Buy-and-hold on one volatile asset *is*
  path-dependent: resample H1-2024 BTC log-returns in moving blocks and a healthy
  fraction of synthetic orderings finish below start / show p5 Sharpe < 0. Holding
  a single 60-70%-annual-vol asset genuinely loses risk-adjusted money in its bad
  case. So `classify_verdict` is not *miscomputing* — the benchmark really does
  trip p5 Sharpe < 0. (This is why it is also *partly* honest, not a pure bug.)
- **But it is the wrong question.** The benchmark is the **null hypothesis the
  candidates must beat**, not a candidate competing to clear an absolute bar.
  "Is buy-and-hold robust?" is category-confused in the same way "is the control
  group cured?" is confused in a drug trial — the control defines the baseline
  against which *effect* is measured; it is not itself a treatment to be passed or
  failed. The gate's own pre-registration encodes this: the bands are a
  *candidate* ruler ("a credible `paper→live` **candidate** on the robustness
  axis," §4 step 4). Applying a candidate ruler to the baseline is a type error.
- **The product already half-knows this.** `CandidateResult.is_benchmark` exists
  precisely "to drive the `BenchmarkWins` honesty branch" (its doc comment), and
  `rank_candidates` *does* special-case the benchmark for the `BenchmarkWins`
  outcome (`rank.rs` line 66: `else if crowned.is_benchmark`). The ONE place the
  benchmark is NOT special-cased is the `all_fragile` test (`rank.rs` lines
  60-65), which counts the benchmark's own Fragile flag toward `AllFragile`. So
  the artifact is a **single missed exemption** in code that otherwise already
  models the benchmark as structurally different.

**Q1 conclusion.** "All active arms Fragile" = the honest, designed-for truth
(the rule is doing on real BTC exactly what it did on 10 prior families). "Hold
also Fragile, therefore `AllFragile`" = a calibration/category artifact: a
candidate-overfit detector applied to the baseline it was built to measure
*against*. The fix is not to weaken the detector; it is to stop pointing it at the
benchmark.

### 1.4 Sub-finding — the unanimous-vote arm (0 trades) is ALSO honest-but-mis-presented

`v0.8.vote.unanimous` (4-of-4 over {sma, macd, rsi, bbands}) fires 0 trades →
+0.00% → Sharpe 0.000 → Fragile. Per ADR-0063 § D1 this is the *honest* abstention
rule working: 4-of-4 agreement on one volatile asset is vanishingly rare, and the
abstention-quorum rule deliberately refuses to manufacture a Long from
early-warming members. So the arm correctly **sat in cash the entire window.**
But on the leaderboard it renders as an indistinguishable Sharpe-0 Fragile loser,
identical in appearance to a strategy that traded and lost. That is a
**presentation** failure, not a logic failure — "sat in cash; consensus never
reached" is a *different and more informative* state than "traded and made
nothing," and the UX currently collapses them. (Folds into recommendation A.)

---

## 2. Q2 — the honest-vs-useful tension

### 2.1 Does always-`AllFragile` EMBODY the negative-result thesis or OVER-state it?

The product's locked thesis (`product.md` § scope banner, lines 325-335) is
precise and *bounded*: "**no [active] strategy beat passive buy-and-hold** net of
cost under the frozen ... rule ... This is a **bounded** result ... not a claim
active trading is impossible." The honest thesis is a statement about
**active-vs-passive**: active edges don't survive; passive is the safe default.

Map that onto the two halves of the `AllFragile` outcome:

- **"All active arms Fragile, hold wins by Sharpe" — EMBODIES the thesis exactly.**
  This is `BenchmarkWins` in spirit: nothing beat holding, holding is the
  least-bad, here is the equity curve. It is the negative result the whole
  program earned, manifesting on the live product. Feature.
- **"...and hold is also Fragile, so the verdict is `AllFragile`" — OVER-states
  into nihilism.** It silently upgrades the bounded claim ("active doesn't beat
  passive") into an unbounded one ("**nothing**, not even passive, is robust —
  everything here is bad"). That is *stronger than the thesis*, *less true* (the
  thesis never said passive was robust on a single asset — it said passive was the
  *baseline*), and *actively unhelpful* to the retail user, whose correct takeaway
  is "hold this one" not "this is hopeless, do nothing." The nihilist framing also
  quietly contradicts the product's own success-metric language (`product.md` line
  268): "when buy-and-hold wins the bake-off, the recommendation says so" — but
  under always-`AllFragile`, buy-and-hold *winning* is masked by buy-and-hold
  *being flagged*, so the `BenchmarkWins` copy never fires even though the
  benchmark is, in fact, the top arm by Sharpe.

So: ~85% feature (the honest active-is-fragile core), ~15% bug (the nihilist
benchmark-is-also-fragile tail that suppresses the more accurate `BenchmarkWins`
story).

### 2.2 Is the gate USELESS as a discriminator, or just mute in this presentation?

Mute, not useless — but the distinction needs care.

- **On the active field, the gate is NOT useless.** ADR-0063 § D7b ships a live
  regression (`robustness_bootstrap_bites.rs`) proving the gate *does*
  discriminate: an overfit candidate (resampled p5 Sharpe < 0) is flagged Fragile
  and loses the crown to a robust single, while a robust candidate is NOT flagged
  (no false positive). The machinery discriminates fine **when there is a robust
  candidate to find.**
- **On real single-asset crypto, there is reliably no robust active candidate
  to find** — and that is the honest truth, not a gate defect. The discrimination
  is "mute" in the sense that the *visible* Robust/Marginal ladder never lights up,
  because the real-world answer on this asset class genuinely sits at the
  Fragile/Marginal floor. Recalibrating to *make* a candidate light up Robust
  would be manufacturing a distinction the data does not support (§3-B).
- **The benchmark contamination is what makes it look totally useless.** Because
  the benchmark is dragged into `AllFragile`, even the *one* honest discrimination
  that should always be available — "active arms are Fragile, but the **baseline**
  is the baseline and it won" — is suppressed. Exempt the benchmark and the
  product immediately regains a meaningful, true discrimination on *every* run:
  `BenchmarkWins` ("hold is least-bad") vs the (rare on crypto, real elsewhere)
  `ActiveWins`. The gate stops *looking* useless the moment it stops judging the
  baseline.

### 2.3 The core tension, stated cleanly

The advisor's honest answer on crypto is *usually* "nothing is robust; hold."
That is **useful** to a retail user IF framed as a clear recommendation ("hold
this coin; no strategy earned its complexity here"). It is **useless / corrosive**
IF framed as an undifferentiated "AllFragile" verdict that flags even the answer
("hold") as fragile and offers no ranking, no least-bad, no baseline. **The honest
truth and the useful product are not in conflict — they are separated by exactly
the benchmark exemption + the copy.** The tension is an artifact of presentation,
not a fundamental honest-vs-useful trade-off. This is the key reframe: we do not
have to choose between honest and useful; we have to stop mis-rendering the honest
result as nihilism.

---

## 3. The long-term directions, each with CONSEQUENCES

### (A) Accept-as-honest + improve the UX copy — DO THIS (necessary, not sufficient)

**What:** Touch no gate logic. Make the leaderboard + recommendation say plainly
"**nothing cleared the robustness bar — holding is the least-bad on this
window**"; render the benchmark as the *baseline* row (visually distinct, "vs just
holding" framing per `Recommendation.benchmark_kpis` which already exists for this);
surface the unanimous-vote arm as "**sat in cash — consensus never reached**"
instead of a silent Sharpe-0 Fragile loser.

**Consequences:**
- *Product value:* **+ substantial.** Converts an opaque "AllFragile" into an
  actionable retail takeaway ("hold this coin"). Directly satisfies the
  product.md success-metric honesty gate ("when everything is FRAGILE, the surface
  says 'nothing here is robust'") *and* recovers the "least-bad" guidance.
- *Honesty / thesis:* **+ strengthens.** This is the most honest option — it says
  exactly what is true (nothing robust; hold is least-bad) without manufacturing
  any distinction. It is the negative-result thesis rendered as a usable sentence.
- *UX:* **+ large.** Biggest legibility win per unit effort. UI-owned copy +
  state, composes existing tokens (the F8 `fragile_badge` and
  `LEADERBOARD_FRAGILE_TAG` already exist).
- *Operator trust:* **+ high.** No gate change → no "you moved the goalposts"
  exposure. Pure clarity improvement.
- **Limit (why not sufficient alone):** it leaves the benchmark *flagged Fragile*
  and the outcome literally `AllFragile`, so the `BenchmarkWins` branch still never
  fires and the leaderboard still shows the baseline wearing a "fragile" badge —
  which is the category error, just better-narrated. A. must be paired with B1 to
  remove the false flag on the baseline, otherwise the copy is papering over a
  type error rather than fixing it.

### (B) Recalibrate so the gate discriminates — SPLIT: B1 do, B2/B3 do NOT

This option bifurcates sharply, and conflating the two halves is the main hazard.

**B1 — Exempt the benchmark from the candidate fragility verdict (DO THIS).**
The benchmark is judged on its own axis (it can win `BenchmarkWins`, it always
shows its KPIs as the "vs holding" baseline) but is **never robustness-gated as a
candidate** and **never counts toward `AllFragile`**. Mechanically this is the
missing exemption in `rank.rs` (the `all_fragile` test should range over
*non-benchmark* candidates; the benchmark's flag becomes informational, not
crown-disqualifying).

- *Consequences — product value:* **+ high.** Restores `BenchmarkWins` as the
  modal honest outcome on crypto ("hold won; no active arm was robust"), which is
  *more* informative than `AllFragile` and is what the product was specced to say.
- *Honesty / thesis:* **+ positive, NOT a relaxation.** This does **not** lower
  the bar for any candidate — every active/ensemble arm still faces the identical
  frozen `classify_verdict`. It corrects a *type confusion* (judging the baseline
  by the candidate ruler). It actually *sharpens* the thesis: "active edges are
  fragile; the baseline is the baseline" is the precise claim, vs the over-stated
  "everything including the baseline is fragile."
- *Operator trust:* **+ if done as an explicit, ADR-logged amendment** (not a
  silent threshold nudge). The framing must be "benchmark-is-not-a-candidate,"
  never "we relaxed the gate" — those are categorically different and the ADR must
  say so.
- *Cost / ownership:* **architect-owned.** Requires an ADR amendment to the
  ADR-0059 § D4 / ADR-0063 `classify_verdict` *freeze* (the classifier itself is
  untouched — the change is in `rank_candidates`' `all_fragile`/eligibility logic
  and the benchmark's flag semantics). Needs a day-1 e2e: a field where all active
  arms are Fragile but the benchmark is top-Sharpe yields `BenchmarkWins`, not
  `AllFragile`. Anchor-safe (the advisor path writes no report).
- *Risk:* low and bounded. The only edge case: a field where the benchmark is
  Fragile AND some active arm is genuinely robust — B1 correctly lets the robust
  active arm win (`ActiveWins`), which is already the desired behavior. The
  `is_benchmark` plumbing and `BenchmarkWins` branch already exist, so blast radius
  is small.

**B2 — Loosen the Fragile thresholds for crypto / make them asset-class-aware (DO
NOT).**
- *Consequences — honesty / thesis:* **− severe, possibly fatal to the moat.**
  The program's single durable asset is a **pre-registered rule no result ever
  gamed** (`onchain-vs-conclude-fork` §1.1: "anti-cherry-pick, p5-Sharpe-<0 ...
  rule that no result has ever gamed"; the 2026-05-30 PRE-REGISTRATION NOTICE
  exists *specifically* to prevent post-hoc band-moving). Relaxing the bands so a
  crypto strategy clears ROBUST is *precisely* the post-hoc goalpost-move the
  pre-registration forbids. It would manufacture a "robust" verdict the resampled
  evidence does not support.
- *Slippery slope (explicit):* once "the bar is too strict for crypto" is an
  accepted reason to loosen, every future asset/regime that returns AllFragile
  becomes a candidate for *its own* relaxation, and the rule degrades from "a
  fair adversary" into "a knob tuned until something passes." That is the
  data-mining-with-extra-steps failure ADR-0063 names as the thing the gate
  exists to prevent — achieved by relaxing the gate instead of bypassing it.
  *Strictly worse than having no gate*, because it wears the credibility of one.
- *Operator trust:* **− high exposure.** "You changed the thresholds after seeing
  they failed on crypto" is the exact accusation the pre-registration discipline
  was built to make impossible. Even if defensible in isolation, it forfeits the
  "no result ever gamed it" property that is the program's headline credential.
- *Verdict:* **reject.** If the bands are ever re-centred, the pre-registration
  §6 assumption-1 protocol is mandatory (logged, before/after value, explicit
  operator signoff, "once") — and "AllFragile on crypto" is **not** a calibration
  miss that justifies it; it is the rule correctly reporting that single-asset
  crypto has no robust active edge.

**B3 — Asset-class-aware bands as a "principled" variant of B2 (DO NOT, same
reason).** Dressing the relaxation as "crypto gets a higher vol budget" does not
change that it lowers the bar to produce a pass; it just makes the goalpost-move
look methodological. Same slippery slope, same trust exposure. Reject.

### (C) Relative / different robustness framing — DO THIS (additive, alongside A+B1)

**What:** Keep the absolute frozen bands as the authoritative pass/fail, but ADD
a *relative* read of the active field so "least-fragile active arm" is legible
even when none clears ROBUST. Two concrete forms:
1. **Robustness ladder / percentile within the field:** rank the active
   candidates by *how close to the bands* they sit (e.g. by p5 Sharpe, or by how
   many of the 5 primary signals they pass) and show that ordering, so the user
   sees "macd is the least-fragile of the active arms" vs "bbands is the most
   fragile" — a real gradation the current binary flag hides.
2. **Judge candidates vs the benchmark's OWN fragility:** report each active arm's
   p5 Sharpe (or prob-of-loss) *relative to the benchmark's*, i.e. "does any active
   arm survive the resampled adversary **better** than just holding?" This is the
   relative question the product actually cares about (active must beat passive),
   and it can yield a meaningful "no active arm is more robust than holding"
   even on an all-Fragile field.

**Consequences:**
- *Product value:* **+ meaningful.** Gives the user a within-field gradation and a
  "vs holding" robustness comparison on *every* run — useful even when the
  absolute answer is "nothing robust." Form (2) directly operationalises the
  active-vs-passive thesis on the robustness axis (not just the Sharpe axis).
- *Honesty / thesis:* **+ neutral-to-positive, IF framed carefully.** Percentile
  ranking must be labelled as *relative ordering within a field that is itself all
  below the bar* — "least-fragile of a fragile field" is NOT "robust," and the
  copy must never imply otherwise. Done right, it adds information without
  asserting any alpha. The hazard is a user reading "ranked #1 on robustness" as
  "robust"; the badge/copy must keep the absolute Fragile flag visible alongside
  the relative rank.
- *UX:* moderate effort. A second sort key / a "robustness ladder" column or a
  "vs holding adversary" line. Larger than A, smaller than a new statistical test.
- *Operator trust:* **+ if additive and clearly subordinate to the absolute
  bands.** The frozen rule stays the headline verdict; the relative view is
  explicitly a navigation aid, not a new gate. No goalpost-move because the
  absolute bands are untouched.
- *Risk:* the relative framing can *imply* a distinction where the honest answer
  is "they're all bad." Mitigate by always co-rendering the absolute Fragile state
  and never crowning on relative rank alone (the crown stays gated on the absolute
  rule + the benchmark exemption from B1).

### (D) Other direction — power / data-shape caveat (NOTE, do not act yet)

The bootstrap runs on **one asset's hourly H1-2024 curve**. The pre-registration
(§6 assumption-4) and the on-chain note both flag that tail percentiles get
noisier on thinner series, and a single-asset path has *no cross-sectional
diversification* to soften the p5 tail — the bands were partly calibrated on
*multi-symbol* surfaces where the shared-index null spliced crash blocks *across a
universe* (`robustness-decision-rule` §2). On one asset, the p5 tail is
mechanically harsher (no diversification to rescue it), which is part of *why*
even buy-and-hold trips it. This is **not** a reason to relax the bands (D is not
B2) — it is a reason to (i) be honest in copy that the verdict is single-asset
path-robustness on one window, and (ii) note for the architect that a
*multi-window* or *multi-asset* robustness read (judge the strategy across several
coins/windows, not one) is the durable long-term way to get a non-degenerate
robustness signal — a larger future feature, explicitly out of scope here, flagged
so it is not lost. **Action now: none beyond the copy caveat; record as a
backlog-worthy future direction.**

---

## 4. Long-term recommendation + rationale

**Ship (C-scaffolded) A + B1 + C together; explicitly reject B2/B3; record D as a
future direction.** Concretely, in durable order:

1. **B1 — benchmark exemption (architect-owned, ADR amendment).** The load-bearing
   fix. Stop counting the benchmark's fragility toward `AllFragile`; let
   `BenchmarkWins` fire when hold is top-Sharpe and no active arm is robust. This
   corrects the category error at the root and is *not* a threshold relaxation —
   the ADR must frame it as "benchmark-is-not-a-candidate," with a day-1 e2e
   (all-active-Fragile + benchmark-top-Sharpe ⇒ `BenchmarkWins`). This single
   change moves the modal crypto outcome from the nihilist `AllFragile` to the
   honest, specced `BenchmarkWins`.
2. **A — UX copy (ui-owned).** "Nothing cleared the robustness bar — holding is
   the least-bad on this window"; benchmark rendered as the baseline; unanimous
   arm shown as "sat in cash — consensus never reached." Highest clarity-per-effort.
3. **C — relative robustness ladder + "vs holding adversary" line (additive).**
   Restores a *true* within-field gradation and operationalises active-vs-passive
   on the robustness axis, without asserting any alpha and without touching the
   frozen bands.

**Why this is the durable choice (and the cheap path is a trap):**

- The cheap path is "just relax the bar so something shows Robust" (B2). It is
  *strictly less durable*: it forfeits the program's one irreplaceable asset (a
  pre-registered rule no result ever gamed), opens the per-asset goalpost-move
  slippery slope, and is the exact failure ADR-0063 says the gate exists to
  prevent — now wearing the gate's credibility. Per the operator's
  durable-over-quick lens, the `(Recommended)` tag goes on the *correct* fix
  (exempt the benchmark + frame honestly), NOT the typing-faster one (loosen the
  bar). B2 buys a cosmetic distinction at the cost of the moat.
- A + B1 + C is more work (an ADR amendment + UX + a relative view) but it is the
  fix that "carries forward without amendment": it is correct on crypto, correct
  on a future asset where an active arm *does* survive (the gate still bites,
  `ActiveWins` still fires), and correct in the always-honest reporting of
  negative results. It never needs un-doing when the next asset class is added.
- It also *unifies* the spec: `is_benchmark` + `BenchmarkWins` already exist as
  the architecture's statement that the benchmark is structurally not a candidate;
  B1 finishes wiring that statement through the `all_fragile` branch. We are
  completing an existing design intent, not bolting on a new one.

**If-budget-tightens annotation (the named cheaper lane that is NOT B2):** if only
one thing can ship this cycle, ship **A alone** (UX copy + "least-bad" framing +
"sat in cash"). It delivers the bulk of the *user-facing* honesty improvement with
zero gate-logic risk and no ADR, and it is forward-compatible with B1/C landing
later. A-alone leaves the benchmark technically flagged (the category error
persists under the hood) but renders the honest "hold is least-bad" story
regardless — so the user-visible nihilism is removed even before the root fix
lands. This is the cheaper path I would default to if the architect's ADR
amendment cannot be scheduled this cycle. **Do NOT** substitute B2 as the "quick
win" — A-alone is the correct cheap lane; B2 is never the cheap lane, it is the
moat-eroding lane.

---

## 5. Assumptions & limits (challengeable by operator / architect)

1. **The benchmark-exemption is a `rank_candidates` change, not a `classify_verdict`
   change.** I am asserting the classifier and its bands stay byte-frozen
   (ADR-0059 § D4) and only the eligibility / `all_fragile` logic + the benchmark's
   flag *semantics* move. The architect owns confirming the exact seam; if it turns
   out to require touching the classifier, the freeze protocol (ADR amendment) is
   even more clearly mandatory. Either way it is architect-owned, not a code change
   I make here.
2. **"Buy-and-hold trips p5 Sharpe < 0 on H1-2024 BTC because single-asset crypto
   is genuinely path-dependent" is inferred, not re-simulated.** It is consistent
   with the reported result (BH flagged Fragile) and with the mechanics (60-70%-vol
   single asset, moving-block resampling of one curve, no cross-sectional
   diversification). I did not re-run the bootstrap; the architect's parallel
   classifier analysis should confirm *which* of the 5 primary signals BH trips
   (likely p5 Sharpe and/or prob-of-loss). If BH trips on, say, the p95 drawdown
   tail instead, the "category error" conclusion is unchanged but the copy detail
   shifts.
3. **The recommendation assumes the product wants a usable retail takeaway on
   crypto, not a pure research artifact.** Given the 2026-06-19 pivot (paper retail
   advisor: "pick coin + budget → rank → forward-plan → watch your €200"), this is
   strongly supported by product.md. An operator who reframes the advisor as a
   *robustness research console* (where "AllFragile, full stop" IS the intended
   terminal message) could legitimately choose A-only and skip B1/C — but that
   contradicts the shipped success-metric language ("when buy-and-hold wins ... the
   recommendation says so"), which presumes `BenchmarkWins` is reachable.
4. **Rejecting B2/B3 assumes the pre-registration discipline remains a load-bearing
   product value.** It is, per product.md § Differentiator (5) "measured robustness,
   not asserted alpha" and the on-chain note's "rule no result ever gamed." If the
   operator ever deprioritises that moat, the calculus on B2 changes — but that
   would be a deliberate thesis change, logged, not a quiet threshold nudge.
5. **C's relative framing must never be mistaken for an absolute pass.** The single
   biggest UX hazard in this whole analysis is a user reading "robustness rank #1"
   as "robust." The absolute Fragile flag must stay co-rendered. If that can't be
   guaranteed in the UI, C should be deferred and only A+B1 shipped.

---

## Changelog

- 2026-06-22 (analyst, robustness-gate-allfragile analysis): adjudicated the
  always-`AllFragile`-on-real-crypto finding from the product/research/honesty
  angle (architect owns the classifier angle in parallel). Q1 VERDICT: BOTH,
  separable — "all ACTIVE arms Fragile" is the honest designed-for truth (the rule
  is a curve-fit detector for *active* strategies, p5-Sharpe<0 = the curve-fit
  signature per `robustness-decision-rule-2026-05-30` §3.1; calibrated on
  daily/4h multi-symbol *active* θ-surfaces per `passive-baseline.md`/horizon-retest
  reports; ADR-0063 names the target as "ensembles — the easiest-to-overfit
  candidates"); "hold also Fragile → `AllFragile`" is a CATEGORY ERROR / calibration
  artifact — the benchmark is the null the candidates beat, never historically
  robustness-judged as a candidate (`passive-baseline.md` 7/25/27: BH is "the
  benchmark every surface is scored against"); the product already half-encodes this
  (`is_benchmark` + `BenchmarkWins` exist; the ONLY missed exemption is `rank.rs`'s
  `all_fragile` counting the benchmark). Q2: ~85% honest-thesis-manifesting / ~15%
  over-stated-into-nihilism; the gate is MUTE-in-this-presentation, not useless
  (it discriminates fine on the active field — ADR-0063 D7b proves it); honest and
  useful are NOT in conflict, they're separated by the benchmark exemption + copy.
  Also flagged the unanimous-vote 0-trades arm as honest-but-mis-presented ("sat in
  cash" ≠ "traded and lost"). RECOMMENDATION: ship A (UX copy) + B1 (benchmark
  exemption — architect-owned ADR amendment to ADR-0059 §D4/ADR-0063 freeze; NOT a
  threshold relaxation) + C (relative robustness ladder / vs-holding-adversary,
  additive); explicitly REJECT B2/B3 (loosen / asset-class-aware bands — the
  pre-registration goalpost-move the discipline forbids; slippery slope;
  strictly-worse-than-no-gate); record D (multi-window/multi-asset robustness read)
  as a future direction. Durable-over-quick: `(Recommended)` on the correct fix
  (exempt + frame honestly), NOT the cheap one (relax the bar). If-budget-tightens:
  A-alone is the correct cheaper lane (removes user-visible nihilism, zero gate
  risk, no ADR), NEVER B2. ANALYSIS ONLY — no code, no `classify_verdict` change, no
  `spec/*/reports/` touch, no git.