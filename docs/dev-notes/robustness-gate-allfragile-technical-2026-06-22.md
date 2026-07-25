---
slug: robustness-gate-allfragile-technical-2026-06-22
status: draft
owner: architect
updated: 2026-06-22
tags: [robustness, allfragile, benchmark-exemption, rank-candidates, classify-verdict, adr-draft, anchor-blast-radius, frozen-comparator, advisor, decision-support, durable-over-quick]
related:
  - docs/dev-notes/robustness-gate-allfragile-analysis-2026-06-22.md
  - docs/dev-notes/robustness-decision-rule-2026-05-30.md
  - _bmad-output/planning-artifacts/architecture/decisions/0059-bakeoff-orchestrator-home-and-result-seam.md
  - _bmad-output/planning-artifacts/architecture/decisions/0063-ensemble-vote-seam-and-robustness-gate-activation.md
  - _bmad-output/planning-artifacts/architecture/decisions/0051-monte-carlo-determinism-and-distribution-report-anchoring.md
  - spec/advisor-bakeoff-ranking/feature.md
  - spec/advisor-ensemble/feature.md
  - docs/runbooks/passive-baseline.md
  - crates/backtest/src/bakeoff/rank.rs
  - crates/backtest/src/bakeoff/robustness.rs
  - crates/backtest/src/bakeoff/bootstrap.rs
  - crates/backtest/src/bakeoff/mod.rs
  - crates/backtest/tests/robustness_bootstrap_bites.rs
  - crates/backtest/tests/bakeoff_e2e.rs
---

# Robustness gate "AllFragile on real crypto" — TECHNICAL / classifier / seam / anchor analysis

> **Mandate (architect decision-support, FILES ONLY — NO code, no `classify_verdict`
> change, no `rank.rs` change, no `spec/*/reports/` touch, no git).** Confirms the
> technical specifics of the always-`AllFragile`-on-real-BTCUSDT-H1-2024 finding:
> *which* fragility criterion fires for buy-and-hold, the EXACT (B1) benchmark-exemption
> seam, whether a test pins the current behaviour, and **the anchor blast radius** —
> the load-bearing constraint. Complements the analyst's product/honesty note
> (`robustness-gate-allfragile-analysis-2026-06-22.md`) — does not re-derive its
> product conclusions. Every claim traces to source read this session
> (`bakeoff/{rank,robustness,bootstrap,mod}.rs`, `stats/mod.rs`, the two e2e tests,
> ADR-0059 § D5, ADR-0063 § D4/D5/D7, the 2026-05-30 pre-registration). Anchor
> baseline confirmed **119/119** via `scripts/verify_anchors.sh` (no report touched).

---

## 0. TL;DR — the four technical answers

1. **Which criterion fires for buy-and-hold?** The classifier is a 5-signal OR
   (`classify_verdict`, robustness.rs:134-138): ANY one breach ⇒ Fragile. For a
   single-asset crypto hold on H1-2024 BTC, the **p5-Sharpe < 0** floor is the
   near-certain primary trigger, with **prob_loss > 0.35** the likely co-trigger;
   **p95-MaxDD > 0.70** is a *possible third* but is NOT required and is less
   likely the binding one (see §1.3 — the tester's p95-MaxDD guess and the
   analyst's p5-Sharpe/prob-loss guess are reconciled: **p5-Sharpe is the binding
   signal; prob_loss likely also breaches; p95-MaxDD is plausible-but-secondary**).
   I did NOT re-run the 1000-path bootstrap (no code execution allowed); the
   identification is from the band arithmetic + the mechanics of resampling one
   60-70%-vol asset. **This is a genuine property, not a numerical artifact** —
   single-asset crypto hold IS path-dependent with a money-losing p5 tail, which is
   exactly *why* it's a **category error** (judging the benchmark by the candidate
   ruler), not a *bug* (the classifier is computing correctly). Confirms the
   analyst's category-error read at the signal level.

2. **The (B1) seam — EXACTLY where.** `crates/backtest/src/bakeoff/rank.rs`,
   the `all_fragile` computation at **rank.rs:60-62** (and the eligibility
   partition is left UNTOUCHED). `classify_verdict` and its frozen bands
   (robustness.rs) are **byte-untouched**. The benchmark's flag is still
   **computed** (bootstrap.rs runs for every arm incl. buyhold) and still
   **displayed** (it stays on `CandidateResult.robustness`); it is merely
   **excluded from the `AllCandidatesFragile` determination**. Precise shape in §2.

3. **Does a test pin the current behaviour?** **YES — `rank.rs::t65_all_fragile`
   (rank.rs:298-325)** pins `outcome == AllFragile` on a 2-arm field where BOTH
   arms are Fragile and one **is the benchmark**. Under B1 this fixture's expected
   outcome flips to `BenchmarkWins` — so **B1 must amend this test**, not just add
   one. There is ALSO a *specced-but-missing* gate: ADR-0063 § D7 final paragraph
   promises an "all-Fragile field … still yields `AllFragile`" AND a
   "benchmark-top-eligible-Sharpe → `BenchmarkWins` with ensembles present"
   reachability regression — **neither the all-active-fragile→BenchmarkWins case
   nor that R4.4 reachability pair is actually implemented in any test today**
   (verified: no match in `crates/backtest/tests/` or `crates/ui/tests/`). B1's
   day-1 gate fills that gap (§2.4).

4. **THE ANCHOR BLAST RADIUS (load-bearing).** **(B1) is anchor-safe — 119/119
   holds by construction.** `rank.rs` output reaches NO anchored `spec/*/reports/`
   body: the advisor/bake-off path runs `write_report = false` (mod.rs:590), the
   classifier untouched, and `RobustnessMode::default() == Skip` keeps every
   anchored CLI path on `robustness == None`. **(B2/B3), by contrast, would touch
   the frozen `verdict_bands` constants that the 18 block-bootstrap θ-surface
   anchors are byte-keyed to → a multi-anchor REGRESSION** + forfeits the
   pre-registration moat (§3). Quantified in §3.

**Long-term technical recommendation: ship (B1) as the root fix, amend ADR-0063
§ D7 + ADR-0059 § D5 via a new ADR-0066, REJECT (B2/B3).** (A) is analyst/ui-owned
copy (zero classifier/anchor risk); (C) is additive and frozen-band-safe. Sketch in §4-5.

---

## 1. WHICH criterion fires for buy-and-hold (with the real arithmetic)

### 1.1 The classifier is a 5-signal weakest-link OR (the mechanism)

`classify_verdict` (robustness.rs:120-156) reads five fields off the
`DistributionSummary` and returns `Fragile` if **ANY** breaches its FRAGILE band
(robustness.rs:134-138):

| # | Signal | Field (`stats/mod.rs`) | FRAGILE if | Const |
|---|--------|------------------------|------------|-------|
| 1 | p5 Sharpe (tail floor) | `summary.sharpe.p5` | `< 0.0` | `P5_SHARPE_FRAGILE` |
| 2 | p50 Sharpe (central) | `summary.sharpe.p50` | `< 0.5` | `P50_SHARPE_FRAGILE` |
| 3 | prob-of-loss | `summary.prob_loss` | `> 0.35` | `PROB_LOSS_FRAGILE` |
| 4 | P(Sharpe > 1.0) | `summary.prob_sharpe_gt_1` | `< 0.35` | `PROB_SHARPE_GT1_FRAGILE` |
| 5 | p95 MaxDD tail | `summary.max_dd_tail_p95` | `> 0.70` | `P95_MAXDD_FRAGILE` |

Because it is an OR, **only one breach is needed**, and the verdict does not tell
you *which* — so identifying buy-and-hold's trigger is an inference from the
input distribution shape, not a readout. The distribution is built by
`DistributionSummary::from_path_metrics` (stats/mod.rs:365-418) over the 1000
moving-block resamples of buy-and-hold's H1-2024 BTC equity (`paths:1000`,
Politis–White block length, ADR-0051 sub-seed).

### 1.2 What the H1-2024 BTC buy-and-hold input looks like (the facts on disk)

- Buy-and-hold H1-2024 BTC realized **+47.78%** total return (the finding;
  corroborated by `bakeoff_e2e.rs::t6_2` asserting BTC 2024-Q1 BH `> +20%`, real
  ~+65%). So the **realized** path is strongly up.
- But the robustness gate does NOT score the realized path — it scores the
  **distribution of 1000 resampled re-orderings** of that path's hourly log-returns
  (bootstrap.rs:133-163). BTC H1-2024 hourly returns carry ~60-70% annualized vol
  (the asset class). Moving-block resampling re-orders those hourly blocks: a
  healthy fraction of synthetic orderings front-load the drawdown blocks and finish
  **below start** / with a **negative risk-adjusted** tail.

### 1.3 Which signal binds — reconciling the tester's and analyst's guesses

**p5-Sharpe < 0 is the binding (near-certain) trigger.** The pre-registration note
(`robustness-decision-rule-2026-05-30` § 3.1) is explicit that **p5 Sharpe is
"the single most important number"** and that the threshold is what catches a
high-median/negative-tail curve. A single volatile asset held long has a *wide*
Sharpe dispersion under resampling: with 1000 paths over a 60-70%-vol asset, the
5th-percentile Sharpe almost certainly dips below 0 (the bad-case re-ordering loses
risk-adjusted money). This is the **analyst's guess**, and it is the most
defensible.

**prob_loss > 0.35 is the likely co-trigger.** `prob_loss` is the integer count of
resampled paths finishing below initial equity (stats/mod.rs:380-383, 400). For a
+47.78%-mean single asset with this vol, the mass below break-even can plausibly
exceed 35% — a wide right-skewed terminal-wealth distribution routinely puts
>1/3 of re-orderings underwater even when the mean is strongly positive. So **two**
signals likely breach simultaneously; the OR means either alone suffices.

**p95-MaxDD > 0.70 (the tester's guess) is PLAUSIBLE but NOT required and likely
NOT the binding one.** A 70% tail drawdown on a +47.78% H1-2024 window is a strong
claim — H1-2024 BTC did not draw down 70% on the realized path, and while
resampling can splice adverse blocks, exceeding 70% at the p95 requires the
bad-case re-ordering to stack most of the down-moves. It *can* happen on a
60-70%-vol asset but is the **least certain** of the three. So the tester's
p95-MaxDD hypothesis is not wrong as a *possible* contributor, but the **binding**
signal is p5-Sharpe (with prob_loss as the likely second), not p95-MaxDD.

> **Caveat (honest about method):** I could not execute the bootstrap (no-code
> mandate), so I cannot print the exact `DistributionSummary` numbers. The
> identification is from (a) the band arithmetic, (b) the documented design intent
> that p5-Sharpe is the headline discriminator, and (c) the mechanics of resampling
> one high-vol asset. **A 5-minute operator probe to confirm the exact triggering
> signal(s) is in the recipe at §6** — it adds one `tracing::debug!` of the summary
> and runs the existing bootstrap; it touches no anchored path.

### 1.4 Correct-vs-artifact: this is a CATEGORY ERROR, not a numerical bug

The literal statement "buy-and-hold's resampled p5 Sharpe < 0" is **true** — single
volatile-asset hold genuinely loses risk-adjusted money in its bad case, with **no
cross-sectional diversification to rescue the tail** (the bands were partly
calibrated on *multi-symbol* shared-index surfaces where crash blocks splice across
a universe — `robustness-decision-rule` § 2; on ONE asset the p5 tail is
mechanically harsher). So `classify_verdict` is computing **correctly**. The defect
is **pointing the candidate-overfit ruler at the benchmark** — the null hypothesis
the candidates are scored *against*, never historically a robustness-judged
candidate (this is the analyst's adjudication; the technical confirmation is: the
classifier is correct ⇒ the fix is NOT in the classifier ⇒ it is in *who gets
fed to / counted by* the gate, i.e. `rank.rs`). This is precisely why B1 (an
eligibility-path change) is right and B2/B3 (a classifier-band change) is wrong:
**the numbers are honest; the category is wrong.**

---

## 2. THE (B1) SEAM — exactly where, and the precise change shape

### 2.1 Confirmed: the seam is `rank.rs`, NOT `classify_verdict`

`classify_verdict` + `verdict_bands` (robustness.rs) stay **byte-frozen**
(ADR-0059 § D4 / ADR-0063 § D4 freeze). The benchmark exemption belongs in
`rank_candidates` (rank.rs), specifically the **`all_fragile` computation**:

```
// rank.rs:60-62 — CURRENT (counts the benchmark's own flag):
let all_fragile = candidates
    .iter()
    .all(|c| c.robustness == Some(RobustnessFlag::Fragile));
```

The eligibility partition (`is_eligible`, rank.rs:124-126) and the comparator
(rank.rs:88-121) are **left untouched** — the benchmark is still ranked by the
same Sharpe-primary total order. The ONLY change is that the benchmark's Fragile
flag must NOT count toward the `AllFragile` *outcome determination*.

### 2.2 The precise change shape (descriptive — NOT applied)

`all_fragile` should range over **non-benchmark** candidates only. Conceptually:

```
// PROPOSED shape (architecture description, not a diff):
let all_active_fragile = candidates
    .iter()
    .filter(|c| !c.is_benchmark)
    .all(|c| c.robustness == Some(RobustnessFlag::Fragile));
```

With the outcome branch (rank.rs:64-70) becoming, in effect:

- `AllFragile` iff **all ACTIVE arms are Fragile AND the benchmark does not give a
  crownable result** — i.e. the benchmark is itself Fragile *or* absent. (Edge:
  a field with no benchmark falls back to the current all-arms rule — but the
  bake-off ALWAYS appends `v0.buyhold` (mod.rs:560), so a benchmark is always
  present in the live path.)
- `BenchmarkWins` iff the crown is the benchmark (`crowned.is_benchmark`) — this
  branch (rank.rs:66) **already exists** but is currently **unreachable on
  all-fragile crypto** because `all_fragile` short-circuits to `AllFragile` first
  (rank.rs:64 checks `all_fragile` BEFORE `crowned.is_benchmark`). After B1, when
  all active arms are Fragile and buy-and-hold is the top-Sharpe eligible arm, the
  crown is the benchmark and `BenchmarkWins` fires.

**Two semantic sub-points the ADR must pin (D-clauses):**

1. **The benchmark's flag stays computed + displayed.** B1 does NOT skip the
   bootstrap for the benchmark (bootstrap.rs still runs for `candidate_index ==
   benchmark`), and does NOT null its `robustness` field. The flag remains visible
   on the leaderboard row (informational: "the baseline is itself path-dependent")
   — it simply no longer disqualifies the *field* into `AllFragile`. This preserves
   honesty: we still SHOW that hold is path-dependent; we stop letting that fact
   nuke the recommendation into nihilism.

2. **Eligibility for the CROWN is unchanged for active arms.** B1 does NOT touch
   `is_eligible` (rank.rs:124). An active Fragile arm is still ineligible to be
   crowned (the anti-overfit lock holds). The benchmark was already eligible
   regardless of its flag *for ranking purposes* only via the comparator's
   eligibility partition — and here is the subtle bit: **currently a Fragile
   benchmark IS partitioned as ineligible by the comparator (rank.rs:95-98)**, so
   on all-fragile crypto the benchmark sorts into the ineligible bucket alongside
   the active arms. B1 must therefore ALSO decide whether the benchmark is
   crown-eligible despite a Fragile flag. **Recommended: the benchmark is
   crown-eligible irrespective of its own robustness flag** (it is the baseline,
   not a candidate that must clear the bar) — i.e. `is_eligible` returns `true`
   for `c.is_benchmark` regardless of flag. This is the minimal, consistent
   completion of the "benchmark-is-not-a-candidate" principle and is what makes
   `BenchmarkWins` actually reachable when every arm (incl. the benchmark) is
   flagged Fragile. **This is the one place B1 touches the eligibility predicate**
   — and it is still entirely within `rank.rs`, still classifier-untouched.

> **Architect note — scope correction vs the analyst's read.** The analyst framed
> B1 as "only the `all_fragile` test changes." Reading the comparator, B1 in fact
> needs **two coordinated edits in `rank.rs`**: (i) `all_fragile` → `all_active_fragile`
> (exclude benchmark from the outcome determination), AND (ii) `is_eligible` →
> benchmark is crown-eligible regardless of its flag (so the benchmark can actually
> WIN the crown when all arms are Fragile). Edit (i) alone is insufficient: without
> (ii), an all-Fragile-incl-benchmark field would set `all_active_fragile = true`
> (active arms fragile) but the benchmark would still be partitioned ineligible by
> the comparator, the crown would land on a Fragile *active* arm, `crowned.is_benchmark`
> would be false, and the outcome would fall through to `ActiveWins` on a Fragile
> crown — a WORSE bug than today. Both edits are required and both live in `rank.rs`.
> This is the kind of seam subtlety the ADR must specify precisely.

### 2.3 Determinism: B1 is determinism-neutral

`rank_candidates` is already pure/total/deterministic (no f64 arithmetic — only
`f64::total_cmp` and `Decimal::cmp`; rank.rs:17). B1 adds a `filter` and changes a
predicate — still pure, still total, still no new f64 boundary, still identical
output for identical input. No RNG, no `SystemTime`. **No determinism risk.**

### 2.4 The test that pins the current behaviour (and the day-1 gate B1 needs)

**Pins current behaviour — MUST be amended by B1:**

- `rank.rs::t65_all_fragile` (rank.rs:298-325): two arms, BOTH Fragile, one
  `is_benchmark = true` (the `v0.buyhold` arm), asserts `outcome == AllFragile`.
  **Under B1, the only non-benchmark arm is Fragile but the benchmark is now
  crown-eligible → crown = benchmark → `outcome == BenchmarkWins`.** This fixture's
  expected `outcome` (and its `AllCandidatesFragile` reason assertion) **flips**.
  B1 must update `t65` to reflect the corrected semantics (or split it: keep an
  all-arms-fragile-incl-benchmark case asserting `BenchmarkWins`, and add a
  no-benchmark or benchmark-also-genuinely-worst case for the residual `AllFragile`
  path). This is a **behaviour change to a unit test, fully expected and ADR-logged**
  — not a regression.

- `rank.rs::single_fragile_candidate_is_crowned` (rank.rs:404-416): single arm,
  Fragile, `is_benchmark = false`, asserts `AllFragile`. **Unaffected** — no
  benchmark in the field, so `all_active_fragile` is true and there is no
  crownable benchmark → stays `AllFragile`. (Confirms the no-benchmark fallback.)

- `rank.rs::t64_benchmark_wins` (rank.rs:278-292) and `t63_robustness_gate`
  (rank.rs:238-272): both use `robustness: None` for the benchmark, so they do NOT
  exercise the Fragile-benchmark path and are **unaffected** by B1.

**Specced-but-MISSING gate (the gap B1 closes):** ADR-0063 § D7 final paragraph
mandates "a field where buy-and-hold has the top eligible Sharpe still yields
`BenchmarkWins` **with ensembles present**, and an all-Fragile field (singles +
ensembles) still yields `AllFragile`" (the R4.4 reachability regression). **Grep
confirms no test asserts the all-active-fragile + benchmark-survives →
`BenchmarkWins` case** (nor the R4.4 pair) anywhere in `crates/backtest/tests/` or
`crates/ui/tests/`. The contract comment in `robustness_bootstrap_bites.rs:9`
("Allow `BenchmarkWins` to remain reachable when all active strategies lose")
DECLARES the intent but no test BODY asserts it. So B1's **day-1 gate is not net-new
work — it is finally implementing the R4.4 reachability regression ADR-0063 already
promised**, now with the corrected (benchmark-exempt) semantics:

> **B1 day-1 e2e (the required gate, per CLAUDE.md "every overlay/modifier ships
> with a divergence test from day 1" applied to an outcome-determination change):**
> A field where **all active arms are flagged Fragile** AND the **benchmark is the
> top-Sharpe arm** must yield `outcome == BenchmarkWins` (reason `BenchmarkUndefeated`),
> **NOT** `AllFragile`. Plus the dual: a field where all active arms AND the
> benchmark are Fragile AND the benchmark is NOT crownable-by-Sharpe behaves per the
> chosen residual rule. FAIL-before (today it returns `AllFragile`) / PASS-after.
> This is a pure `rank_candidates` unit/e2e test (no corpus, no bootstrap needed —
> construct `CandidateResult`s with explicit flags, the `t65` pattern).

---

## 3. ANCHOR BLAST RADIUS — the load-bearing answer

### 3.1 (B1) is anchor-safe — 119/119 holds BY CONSTRUCTION

Traced the full path from `rank.rs` output to any anchored byte:

- **The advisor/bake-off path writes NO report.** `run_bakeoff` constructs every
  `ScenarioConfig` with `write_report = false` (mod.rs:590, "anchor-safe: no report
  body written (ADR-0059)"). The `BakeoffReport` / `Recommendation` / `outcome` are
  **in-memory only** — consumed by the cockpit, never serialized to a
  `spec/*/reports/` body. So a change to `rank.rs`'s `outcome` **cannot** alter any
  anchored file's body-SHA.
- **`classify_verdict` + `verdict_bands` are byte-untouched.** B1 does not edit the
  classifier. The 18 block-bootstrap θ-surface anchors (e.g.
  `v1-basis-reversal-fee*-theta-surface-*-block-bootstrap-real-fy`,
  `v2-mn-*-theta-surface-*-block-bootstrap-real-fy`) that hash a body containing
  per-θ ROBUST/MARGINAL/FRAGILE verdicts read `classify_verdict` from the **sweep
  bin path**, which is unchanged. (These anchors come from
  `bin/param_robustness_sweep.rs`, NOT from `rank_candidates` — `rank_candidates`
  is the advisor comparator and has never written an anchored body.)
- **`RobustnessMode::default() == Skip`** (mod.rs:308-310). Every existing caller
  and every anchored CLI path keeps `robustness == None`, so even the *eligibility*
  path is byte-unchanged on anchored runs (and `is_benchmark` only matters when a
  benchmark arm is present in a bake-off, which no anchored report path constructs).
- **Confirmed empirically:** `scripts/verify_anchors.sh` → **ANCHORS PASS (119/119)**
  at HEAD this session (I touched no report). B1, by the above, keeps it 119/119.

**Verdict: (B1) is anchor-additive-equivalent — same class as the `v0.buyhold` arm
(ADR-0059 § D3) and the `RobustnessMode::Bootstrap` activation (ADR-0063 § D5):
"new behaviour is opt-in / advisor-path-only, existing anchored bytes untouched."
119/119 by construction. No `anchors.toml` SHA changes; no `data/*/REVISION.toml`
touched.**

### 3.2 (B2/B3) — by contrast — WOULD break the anchors (why it's costly)

B2/B3 = loosen / asset-class-tune the FRAGILE bands in `verdict_bands`
(robustness.rs:85-109). Those constants are read by `classify_verdict`, which the
sweep bin calls to emit the per-θ verdict strings **into the body of all 18
block-bootstrap θ-surface anchored reports**. Changing any FRAGILE threshold (e.g.
`P5_SHARPE_FRAGILE`, `P95_MAXDD_FRAGILE`) would:

- **Flip per-θ verdict labels in the anchored bodies** (a θ-cell that was FRAGILE at
  the old band could become MARGINAL/ROBUST), mutating the body-SHA of **every one
  of the 18 surfaces whose verdict column changes** → a **multi-anchor REGRESSION**.
  This is not anchor-additive; it is anchor-mutating, and per `spec/anchors.toml`
  discipline + ADR-0038 § D6 it would require explicit re-emission + operator
  signoff for each.
- **Forfeit the pre-registration moat.** The 2026-05-30 PRE-REGISTRATION NOTICE
  freezes the bands "BEFORE C2 emits any number"; § 6 assumption-1 allows a re-centre
  only "once, logged, with operator signoff" — and "AllFragile on crypto" is
  explicitly NOT a calibration miss that justifies it (the rule is correctly
  reporting single-asset crypto has no robust active edge). B2/B3 is the exact
  post-hoc goalpost-move the discipline exists to forbid.

**Quantified cost of B2/B3: up to 18 anchored-report body-SHA breaks (every
block-bootstrap θ-surface whose verdict labels shift) + a mandatory per-anchor
ADR-0038 § D6 re-emission + the loss of the "no result ever gamed the rule"
credential. (B1 breaks ZERO.)** This asymmetry — B1: 0 anchors + 1 ADR amendment;
B2/B3: ≤18 anchors + pre-registration forfeiture — is the decisive technical reason
to take B1 and reject B2/B3, independent of the product argument.

---

## 4. Per-option technical consequences (the operator wants consequences)

| Option | Seam | Anchor impact | Frozen-ADR impact | Determinism | Complexity |
|--------|------|---------------|-------------------|-------------|------------|
| **(A)** UX copy (analyst/ui-owned) | UI strings + leaderboard state; NO backtest change | **0** (no body touched) | none | n/a | low — composes existing `fragile_badge` tokens |
| **(B1)** benchmark exemption (RECOMMENDED) | `rank.rs` `all_fragile`→`all_active_fragile` + `is_eligible` benchmark-always-eligible; classifier UNTOUCHED | **0** — advisor path `write_report=false`, classifier frozen, `default()==Skip` → 119/119 by construction | **AMENDS ADR-0063 § D7 + ADR-0059 § D5** (comparator outcome semantics); classifier-freeze ADR-0059 § D4 / ADR-0063 § D4 **UNCHANGED** | neutral — still pure/total, no f64, no RNG | low-moderate — 2 coordinated `rank.rs` edits + amend `t65` + add the R4.4 day-1 gate |
| **(C)** relative robustness ladder | additive read over the field (sort by p5-Sharpe or signals-passed); co-render absolute flag | **0** — additive display, frozen bands untouched | none (absolute bands stay authoritative) | neutral if Decimal/total_cmp-ordered | moderate — a second sort key / "vs-holding adversary" line |
| **(B2/B3)** loosen / asset-class-aware bands (REJECTED) | `verdict_bands` constants in `robustness.rs` (the classifier itself) | **≤18 anchored θ-surface body-SHA breaks** + per-anchor ADR-0038 § D6 re-emission | **VIOLATES** the ADR-0059 § D4 / ADR-0063 § D4 classifier freeze + the 2026-05-30 pre-registration | band change is deterministic but mutates every dependent anchor | high — recalibration + ≤18 re-emissions + operator signoff + moat loss |

**Key asymmetry restated:** the *correct* fix (B1) is the *cheaper* fix on the
anchor axis (0 vs ≤18 breaks) AND the cheaper fix on the trust axis (an additive
ADR amendment vs forfeiting the pre-registration credential). Durable-over-quick
and anchor-cheap point the **same way** here — B1.

---

## 5. ADR SKETCH — ADR-0066 (benchmark-exempt-from-AllFragile)

> **Title (draft):** ADR-0066 — Benchmark exemption from the `AllFragile`
> outcome determination (`rank_candidates` amendment; `classify_verdict` UNCHANGED).
> **Status:** proposed. **Amends:** ADR-0059 § D5 (the F2 comparator outcome rule),
> ADR-0063 § D7 (the R4.4 reachability gate). **Does NOT touch:** ADR-0059 § D4 /
> ADR-0063 § D4 (the `classify_verdict` + `verdict_bands` freeze) — those stay
> byte-frozen. **Anchor impact:** none (119/119 by construction). **Registry:**
> append the `## Registry` row + bump README `updated:` in the SAME commit
> (2026-05-29 atomic-registration contract).

Proposed D-clauses:

- **D1 — The benchmark is not a candidate for the `AllFragile` determination.**
  `rank_candidates` computes `all_active_fragile` over **non-benchmark** arms only
  (`filter(|c| !c.is_benchmark)`). The `AllFragile` outcome fires iff all ACTIVE
  arms are Fragile AND no benchmark gives a crownable result. Rationale: the
  benchmark is the null hypothesis the candidates are scored against
  (`passive-baseline.md`), never a candidate that must clear the robustness bar.
  This is **NOT a threshold relaxation** — every active/ensemble arm faces the
  identical frozen `classify_verdict`. Frame in the ADR strictly as
  "benchmark-is-not-a-candidate," never "we loosened the gate."

- **D2 — The benchmark is crown-eligible irrespective of its own robustness flag.**
  `is_eligible` returns `true` for `c.is_benchmark` regardless of `robustness`. This
  is the second required edit (without it `BenchmarkWins` stays unreachable on
  all-fragile-incl-benchmark fields — see § 2.2 architect note). The benchmark's
  flag remains **computed + displayed** (informational), but does not gate its
  eligibility or the field's outcome.

- **D3 — `classify_verdict` + `verdict_bands` are byte-UNCHANGED.** Reaffirms the
  ADR-0059 § D4 / ADR-0063 § D4 freeze. The bootstrap still runs for the benchmark
  arm; the flag is still produced; only its *consumption* in `rank_candidates`
  changes. Pre-registration discipline intact (no band moved).

- **D4 — Anchor safety by construction (119/119).** The advisor bake-off path
  writes no report (`write_report=false`, ADR-0059 § D3); `RobustnessMode::default()`
  stays `Skip`; the classifier and the 18 block-bootstrap θ-surface anchors are
  untouched. `verify_anchors.sh` MUST read 119/119 before the first seam and after
  the last (STOP-and-route-back on any non-119). Same additive-equivalent contract
  as ADR-0059 § D3 / ADR-0063 § D5.

- **D5 — Day-1 reachability gate (implements the missing ADR-0063 § D7 / R4.4).**
  Ship the FAIL-before/PASS-after e2e: all active arms Fragile + benchmark
  top-Sharpe ⇒ `outcome == BenchmarkWins` (reason `BenchmarkUndefeated`), NOT
  `AllFragile`; plus the residual all-arms-fragile-no-crownable-benchmark ⇒
  `AllFragile`. Amend `rank.rs::t65_all_fragile` to the corrected semantics. This
  finally lands the reachability regression ADR-0063 § D7 promised but never
  implemented. (CLAUDE.md day-1-divergence-test discipline, applied to an
  outcome-determination change rather than an equity overlay.)

- **D6 — Determinism unchanged.** `rank_candidates` stays pure/total/deterministic,
  no f64 arithmetic, no RNG — D-clause records that B1 introduces no new
  determinism boundary.

**ADRs this leans on:** ADR-0059 (comparator + buyhold arm + write_report=false),
ADR-0063 (gate activation + the R4.4 promise), ADR-0051 (bootstrap determinism,
unaffected), the 2026-05-30 pre-registration (the freeze this ADR is careful NOT
to disturb). **Anchor-mutation ADR NOT triggered** (no anchors.toml SHA change).

---

## 6. Operator verification recipe (confirm WHICH signal binds — 5 min, anchor-safe)

> Optional, for closing the §1.3 inference gap with real numbers. Touches no
> anchored path. Architect/developer can run this when B1 is scheduled.

- **Command:**
  ```
  RUST_LOG=bakeoff.robustness=debug cargo test -p backtest --features realdata \
    --test bakeoff_e2e t6_2 -- --ignored --nocapture
  ```
  (or a throwaway one-off that calls `compute_robustness_flag` on the buyhold
  equity from a H1-2024 BTC bake-off and `eprintln!`s the `DistributionSummary`
  — `summary.sharpe.p5`, `summary.prob_loss`, `summary.max_dd_tail_p95`,
  `summary.prob_sharpe_gt_1`, `summary.sharpe.p50`).
- **Steps:** ensure `data/binance/BTCUSDT/2024/0{1,2,3}.parquet` present (the test
  skips gracefully if absent); run; read the five signal values for the buyhold arm.
- **Timing:** ~1-3 min (1000 paths on one curve is fast; the test itself is the
  slow part if it runs all arms — a one-off on just the buyhold curve is <30s).
- **Expected result:** `summary.sharpe.p5 < 0.0` (the binding trigger);
  `summary.prob_loss` likely `> 0.35`; `summary.max_dd_tail_p95` likely `≤ 0.70`
  (NOT the binding one) — confirming p5-Sharpe is the category-error signal.
- **Failure diagnosis:** if instead `max_dd_tail_p95 > 0.70` is the sole breach
  and `p5 ≥ 0`, the tester's guess was right and the §4 (A) copy detail shifts
  ("tail drawdown" not "tail Sharpe") — the B1 fix is **unchanged** either way
  (still a category error, still the same seam).
- **Cleanup:** none (read-only; no file written; `verify_anchors.sh` still 119/119).

---

## 7. Assumptions & limits (challengeable by operator)

1. **The exact triggering signal is INFERRED, not measured** (no-code mandate).
   p5-Sharpe<0 is the near-certain binding signal on first principles + the
   documented design intent; prob_loss>0.35 likely co-triggers; p95-MaxDD>0.70 is
   possible-but-secondary. §6 confirms with real numbers in 5 min. **The B1 fix and
   the anchor analysis are invariant to which signal binds** — they depend only on
   the classifier being correct (it is) and the seam being `rank.rs` (it is).
2. **B1 requires TWO coordinated `rank.rs` edits, not one** (§2.2 architect note):
   `all_active_fragile` AND benchmark-always-crown-eligible. The analyst's
   "single missed exemption" framing is correct in spirit (it's all in `rank.rs`,
   classifier untouched) but undercounts the edit by one. The ADR must specify both.
3. **`t65_all_fragile` will change expected outcome under B1** — this is a
   deliberate, ADR-logged unit-test behaviour change (`AllFragile` → `BenchmarkWins`
   on its 2-arm benchmark-inclusive fixture), NOT a regression. Anyone reviewing the
   diff must read it as the corrected semantics.
4. **119/119 anchor-safety of B1 assumes the advisor path stays `write_report=false`
   and the classifier stays frozen.** Both hold today (mod.rs:590; ADR-0059/0063
   freeze). If a future change ever serialized `BakeoffReport.outcome` into an
   anchored body, B1's anchor-safety would need re-evaluation — flag for any such
   future feature.
5. **The 18-anchor B2/B3 blast-radius is an upper bound** (every θ-surface whose
   verdict column shifts). The exact count depends on how many θ-cells flip at the
   new band — but the point stands: it is anchor-MUTATING (≥1, plausibly many),
   vs B1's exactly-0. The asymmetry, not the precise B2 count, is the decision input.

---

## Changelog

- 2026-06-22 (architect, robustness-gate-allfragile TECHNICAL analysis): confirmed
  the classifier/seam/anchor specifics complementing the analyst's product note.
  (1) WHICH signal fires for buy-and-hold: `classify_verdict` is a 5-signal OR
  (robustness.rs:134-138); **p5-Sharpe<0 is the binding trigger** (the documented
  headline discriminator, near-certain on a 60-70%-vol single asset under 1000-path
  resampling), **prob_loss>0.35 likely co-triggers**, **p95-MaxDD>0.70
  possible-but-secondary** — reconciling the tester's p95-MaxDD guess (not the
  binding one) and the analyst's p5-Sharpe/prob-loss guess (correct). NOT a numeric
  bug — the classifier computes correctly; it is a CATEGORY ERROR (candidate ruler
  on the benchmark). Inferred not measured (no-code mandate) + a 5-min operator
  probe recipe (§6) to confirm. (2) The (B1) SEAM = `rank.rs` `all_fragile`
  (rank.rs:60-62) → range over non-benchmark arms, classifier byte-UNTOUCHED;
  flagged that B1 needs **TWO** coordinated `rank.rs` edits (`all_active_fragile`
  AND benchmark-always-crown-eligible via `is_eligible`), not the single edit the
  analyst implied — without the second, `BenchmarkWins` stays unreachable and the
  crown lands on a Fragile active arm (worse bug). (3) Test pinning current
  behaviour: **`t65_all_fragile` (rank.rs:298) pins `AllFragile` on a 2-arm
  benchmark-inclusive fragile field — B1 flips it to `BenchmarkWins`, MUST amend**;
  ALSO the ADR-0063 § D7 R4.4 reachability gate (all-active-fragile→BenchmarkWins +
  benchmark-top-Sharpe→BenchmarkWins-with-ensembles) is **specced but NOT
  implemented in any test** — B1's day-1 gate finally lands it. (4) ANCHOR BLAST
  RADIUS (load-bearing): **B1 = 0 anchors (119/119 by construction** — advisor path
  `write_report=false` mod.rs:590, classifier frozen, `default()==Skip`; confirmed
  `verify_anchors.sh` 119/119 this session, no report touched); **B2/B3 = ≤18
  block-bootstrap θ-surface body-SHA breaks** (the bands feed the sweep-bin anchored
  verdict columns) + per-anchor ADR-0038 § D6 re-emission + pre-registration
  forfeiture. RECOMMENDATION: ship **B1** (root fix), amend **ADR-0063 § D7 +
  ADR-0059 § D5 via ADR-0066** (classifier-freeze ADR-0059 § D4 / ADR-0063 § D4
  UNCHANGED), with 6 D-clauses (§5) + the day-1 BenchmarkWins-reachability gate;
  (A) ui-owned copy + (C) additive ladder are frozen-band-safe; **REJECT B2/B3**
  (anchor-mutating + moat-eroding). The correct fix is ALSO the anchor-cheap fix —
  durable-over-quick and anchor-safety agree on B1. ANALYSIS ONLY — no code, no
  `classify_verdict`/`rank.rs` change, no `spec/*/reports/` touch, no git.
