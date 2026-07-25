---
slug: research-gap-analysis
status: analysis
owner: analyst
updated: 2026-07-11
title: Gap analysis over the 900-paper research knowledge base (post-P2 / post-NaN-fix)
date: 2026-07-11
author: analyst
scope: READ-ONLY map of what the frozen corpus (research/, time-frozen 2026-06-28) does NOT cover, does NOT apply, or now under-frames given work shipped since. NOT a feature backlog. Changes NO code/gate/anchor/product framing.
inputs:
  - research/SYNTHESIS.md, research/APPLICATIONS.md
  - research/{backtesting,data,strategies,crypto-market-structure}/knowledge.md
  - research/backtesting/application-{overfitting-and-multiple-testing,cost-and-impact-modeling}.md
  - CHANGELOG.md, spec/architecture/00-current-state.md
  - docs/dev-notes/{do-not-build-register,post-v2-scoping-2026-07-09,dsr-report-only-decision-2026-07-09,p2-wobble-thesis-analysis-2026-07-10}.md
  - code ground-truth: crates/backtest/src/bakeoff/scorecard.rs (shipped toolkit surface)
---

# Gap analysis over the research knowledge base — where are the gaps NOW?

> **The question.** The 900-paper corpus (9 topics × ~100, reviews + knowledge +
> applications + SYNTHESIS) was assembled ~June 2026 and drove v2 + the
> remediation. Since then the product shipped far past it (era-qualified thesis,
> PIT discipline P3, lot realism P4, hand-off export P5, crown credibility P1,
> corpus expansion to 2017+Coinbase, scorecard NaN-fix). **Where are the gaps
> now?** Three rigorously-separated kinds: **A** corpus-coverage (never asked /
> covered thinly), **B** application (corpus said-it, code doesn't do-it), **C**
> post-P2 new questions the corpus predates.
>
> **Discipline.** The product is DECLARED DONE (P8 = instrument, maintenance
> mode). This is an honest MAP, not a backlog. The honest-null discipline applies
> to gap analyses too — MOST gaps resolve to **stated-limit / leave**, and I do
> not manufacture work. Read this against the binding
> [`do-not-build-register.md`](do-not-build-register.md): any "gap" whose fix is a
> register row is tagged `register-blocked` and listed for completeness, NOT
> recommended.

---

## TL;DR (≤5 bullets)

- **The corpus holds up well.** It is a hardening/credibility corpus, and the
  work shipped since (P0 scorecard, PIT, lot-realism, corpus-expansion) consumed
  its ship-worthy tranche. Genuine open gaps are **few and mostly stated-limit** —
  the honest-null outcome the product's own thesis predicts. **Counts: A = 4
  corpus gaps (+2 premise-corrections), B = 8 application gaps, C = 3 new
  questions.** Only **one** is a genuine honesty-hardening build-candidate (B7).
- **The 3 most consequential:** (1) **B7 cross-run family-wise multiple-testing**
  (online-FDR / alpha-investing, `backtesting[73]`) — the P2 32-run design created
  a cross-run family that DSR-within-run does not catch; the honest completion of
  the P2 (d)-defense, report-only, gate-untouched. (2) **C2 the "crowning-rate ≫
  noise-floor but 0/19 DSR" state is ANSWERABLE from the corpus** (the FDR
  mixture-model / FWER-vs-FDR distinction) and resolves the apparent P2-errata
  tension into a credibility STRENGTH — an insight, not a build. (3) **A1
  era-dependent GATE calibration is a real corpus blind-spot** — but the honest
  answer (one frozen gate across eras, never per-era tuning) is already what we do
  and is SUPPORTED by the corpus's own pre-registration ethos → stated-limit.
- **Premise-corrections (retire these worries):** the crypto-specific *decay*
  literature is **NOT one citation** — it is multiply-covered (Hurst 0.42→0.49
  `data[43]`, carry 6.45→neg `strategies[74]`/`crypto-ms[67]`, stat-arb halving
  `[16]`, AMH windows `[17][18]`). And **SPA/StepM WAS deep-read** (`backtesting`
  round-3/4 `[7][8][93][99]`, StepM full-text) — it is NOT a corpus gap; it is a
  deliberate application deferral (B3), now **confirmed** by P2.
- **Ground SHIFTED under (post-P2 / post-NaN-fix):** the **SPA deferral (B3)** is
  now empirically **vindicated** (the frozen gate detected the real old-era edges
  *without* SPA's power boost — it had enough power); **PBO-on-the-Tune-surface
  (B2)** is marginally more illustrative but its "homogeneous-grid-only" reason
  **stands**; **cross-run FWER (B7)** is a **newly-salient** gap. **Stand
  unchanged:** ONC-N_eff (closed-form sufficient at N=24, NaN-fix strengthened
  it), tail/EVT slice (D-4), MPPM/MCS/BBC-CV/ORATIO/haircut (redundant-additive or
  moot-while-report-only), cost-default (E-2).
- **register-blocked tempting-fixes (ids only, NOT recommended):** **A-3, A-5,
  B-1, D-4, E-1, E-2.** **Gates:** `verify_anchors.sh` → **119/119 PASS**;
  `spec_lint.py` → **pre-existing FAIL(4), all environmental** (gitignored
  `data/binance/REVISION.toml` absent in this checkout + 1 orphan-feature) — this
  note adds **zero** new violations (delta 0). Verbatim in §5.

---

## 0. Method + what "gap" means here

Read the review/knowledge/application layer of all 9 topics' relevant docs (NOT
`papers.md`, per convention), the two backtesting application docs at full depth,
plus the post-corpus shipped record (CHANGELOG, current-state, the do-not-build
register, the P2 wobble analysis, the DSR-report-only decision). Ground-truthed
the shipped overfitting toolkit against `crates/backtest/src/bakeoff/scorecard.rs`
(memory: spec pervasively lags code — verify before claiming "not built").

A "gap" is judged **for THIS product** (single-coin €200 paper advisor, frozen
robustness gate, ship-passive-on-the-current-era thesis). Absence of a topic that
is out-of-scope (multi-asset, HFT, live) is NOT a gap — it is the negative space.
Each genuine gap gets one recommendation: **research-spike** (bounded reading) |
**build-candidate** (only if it hardens honesty, per the P3/PIT precedent) |
**stated-limit** (document, don't build) | **leave** | **register-blocked** (the
fix is a settled dead-end — list, don't recommend).

---

## A. Corpus-coverage gaps — what the 900 papers never asked / covered thinly

### A1. Era/regime-dependent GATE calibration across liquidity eras — REAL, thin → stated-limit
The corpus deeply covers non-stationarity, structural-break *detection* (Bai–Perron
`backtesting[71]`, BOCPD `data[45][46]`), AMH efficiency-rises-with-liquidity
(`crypto-ms[17][18][54]`), and "treat pre/post-2024 ETF as different regimes"
(`strategies[46]`). What it **never asks** is whether the robustness **GATE
itself** — block length, FRAGILE bands, cost model, DSR threshold — should be
calibrated *differently* across a ~10× liquidity change. It assumes ONE frozen
calibration. The P2 re-run applied the same frozen gate 2017→2026. That is the
**methodologically honest** choice (per-era gate tuning = researcher-DOF,
`backtesting[90][98]`), and the corpus's own pre-registration ethos *supports* it —
but the literature offers no treatment of "is one calibration valid when the
autocorrelation length AND the cost regime both shifted an order of magnitude?"
**Rec: stated-limit.** The frozen-gate-across-eras posture is defensible and
already documented; per-era calibration is the dishonest move. The one testable
sub-question (block-length stability across eras) folds into B8.

### A2. Venue-microstructure EVOLUTION 2017→2026 as a quantified friction series — thin → stated-limit
The corpus has rich *snapshot* data-integrity (wash >70% `crypto-ms[19]`,
fabricated OI `[91]`, ~31%-spoofable depth `[90]`) and the *qualitative* AMH claim
that efficiency rises with liquidity. It has **no quantified time-series** of how
effective spread / depth / friction *evolved* 2017→2026 — exactly what the P2
old-era cost-realism stated limit leans on ("2017-18 books were orders of
magnitude thinner"). We asserted that from first principles + AC5's cross-venue
deviation (7.6 bps 2020 vs 3.3–3.5 bps 2023-26), not a literature series.
**Rec: stated-limit** (matches P2-wobble §4 item 2: a dedicated 2017-18
order-book cost study is negative-ROI for a window the advisor never advises on).
The tempting fix — bump the default cost model to "capture" old-era frictions — is
**register-blocked E-2**.

### A3. Retail-scale execution discretization (lot-size / min-notional) — REAL, structural → stated-limit
Genuinely absent. The cost literature covers fee+spread / impact≈0 / turnover /
vol-scaled spread / intra-candle fills, but **never order discretization or
min-notional** — academic backtests assume continuous position sizing. ADR-0087
lot-realism was correctly built from **exchange filter docs**, not literature.
This is a gap the literature *structurally cannot fill* (an engineering reality,
not a research question). **Rec: stated-limit** — built right, from primary
venue docs, opt-in / default-off; no literature exists or is needed.

### A4. (Premise-correction) Crypto-specific ANOMALY-DECAY is well-covered — NOT a gap
The brief asks whether the decay literature the P2 finding leans on is "one
citation" (McLean–Pontiff). **It is not.** Decay *existence* is multiply-covered
and crypto-specific: Bitcoin Hurst 0.42→0.49 = efficiency rising `data[43]`; crypto
carry Sharpe 6.45→4.06→negative `strategies[74]`/`crypto-ms[67]`; stat-arb halving
post-2010 `strategies[16]`; AMH predictability windows open/close `crypto-ms[17][18]`;
AI/ML-crowding half-lives 5-7y→18mo `data[15]`. What is *thin* is decay **DATING**
(a market-wide transition, vs post-publication decay keyed to a paper's date) —
that half is Category **C1**, not a coverage gap. **Rec: leave** (worry retired).

### A5. (Premise-correction) SPA/StepM beyond DSR/PBO/MinBTL IS in the corpus — NOT a corpus gap
The brief asks whether SPA "was never deep-dived." It **was**: `backtesting`
round-3/4 read Romano–Wolf StepM in **full text** (`[8]`, Algorithm 7.1) plus
Hansen SPA / stepwise-SPA `[7][93][99]`, and catalogued the cross-run tool
(online-FDR / alpha-investing `[73]`). The multiple-testing canon *beyond*
DSR/PBO/MinBTL (SPA, StepM, MPPM, MCS, BBC-CV, ORATIO, difference-of-Sharpes) is
fully mapped in `application-overfitting-and-multiple-testing.md` §2 items 6-11.
So this is **NOT a corpus-coverage gap** — it is a set of deliberate
**application** deferrals → Category **B** (B3-B6). Premise corrected.

### A6. Time-frozen staleness (corpus frozen 2026-06-28) — flag, low-urgency → leave
The corpus is a point-in-time snapshot; it cannot see 2025-26 developments after
the freeze. Existence-check (WebSearch, allowed only to check existence, not
ingest): the 2025-26 public material — institutional-flow migration, four-year-cycle
ending, exchange-reserve structural break, efficiency rising into an
institution-dominated structure — is **market commentary that qualitatively
CONFIRMS the corpus's efficiency-migration prediction**, with **no ingestible new
method**. The fastest-aging slice is the **LLM-time-series verdict** (FinCast was
the frontier; TSFMs move fastest) — but the product's bright line (no predictor in
the ranking, do-not-build **A-1**) is robust to it regardless of what lands.
**Rec: leave** — re-scan only if a NAMED new *method* (not a market narrative)
surfaces; nothing to ingest now.

---

## B. Application gaps — corpus recommendations that never landed in code

Ground truth: the P0 scorecard shipped its high-value core — **N_eff (closed-form
ρ̄) + MinBTL + DSR + `crown_clears_dsr` (informational)** — and turnover + CVaR/
median/skew shipped alongside (`scorecard.rs`, verified). The rest of the
`application-overfitting` candidate ladder (items D-L) and cost-doc items are
named-but-unbuilt. **Framing:** these are the *deliberately-unshipped tail* of a
P0 that shipped its core — the app doc itself said "defer E; A/B/C/D deliver most
of the credibility." The honest question per item is whether the ground SHIFTED
post-P2 / post-NaN-fix.

### B1. Nonlinear Sharpe haircut (Holm/BHY, item D) — unbuilt → leave
Named P0, report-only. DSR already carries the deflation headline; a haircut-RANGE
is redundant-additive surface for ~zero credibility gain over the shipped DSR.
Ground did not shift. **Rec: leave.**

### B2. PBO via CSCV (item F, R2) — KNOWN-DELIBERATE, deferred → leave (research-spike optional)
Deferred to the homogeneous Tune/sweep surface where CSCV is statistically honest;
`scorecard.pbo` is `None` by construction (verified). **Post-P2:** the old-era
ActiveWins are a *better illustration case* now (we finally have windows where
configs crown), so PBO-on-the-sweep is marginally MORE valuable — **but** CSCV is
honest only on a *homogeneous* config grid, NOT the heterogeneous bake-off arms
(SMA vs vote-ensemble vs DVOL are not comparable configs of one search), so the
reason it wasn't wired to the field gate **stands**. The real cost is the
per-config return-matrix enabler (item E). **Rec: leave / research-spike** — bundle
into a future Tune-surface credibility pass only; do NOT plumb E for it alone.

### B3. SPA / StepM studentization (item G, SYNTHESIS P0 #8) — deferral now VINDICATED → leave
The brief's specific question. Why unbuilt: SPA is a **power-restoring** upgrade
(detect a real edge if one exists); the product is honesty-first / expect-null, and
DSR already provides the deflation. **Post-P2 the ground SHIFTED toward CONFIRMING
the deferral:** the P2 data proves real old-era edges existed AND the frozen gate —
*without* SPA — detected them (60-86% crowning in the inefficient eras). The gate
had **sufficient power without SPA**; adding SPA would only make it crown MORE,
cutting directly against the honesty-first posture. Does SPA add anything DSR now
covers? No — they are orthogonal (power vs deflation) — but the honest posture does
not WANT more power. **Rec: leave** — deferral empirically vindicated, not
reopened.

### B4. MPPM manipulation-proof score (item I) — unbuilt → leave
Named to block crowning a negative-skew insurance-seller. The shipped tail block
(CVaR / median / **skew**) already surfaces the negative-skew hazard the operator
needs to see. **Rec: leave** (redundant-additive with the shipped skew/CVaR).

### B5. Model Confidence Set "is hold in the set?" (item J) — unbuilt-but-effectively-present → leave
Named as "the thesis as a test." The **`BenchmarkWins` outcome + benchmark
exemption (ADR-0066)** is the operative poor-man's MCS — it already surfaces "hold
is the least-bad / in the tied set" as the modal result. A formal MCS is
redundant-additive. **Rec: leave.**

### B6. BBC-CV / ORATIO threshold / difference-of-Sharpes CI (items K/L/H) — unbuilt → leave
BBC-CV was flagged "strong candidate to BE our deflation engine," but closed-form
DSR shipped instead (cheaper, sufficient at N=24). ORATIO is **moot while
report-only** — it only matters if a crown veto is chosen, which is do-not-build
**E-1**. Difference-of-Sharpes CI is redundant with the bootstrap weakest-link.
**Rec: leave** (redundant-additive or moot-while-report-only). Wiring ORATIO as
part of a veto is **register-blocked E-1**.

### B7. Cross-run family-wise multiple-testing (online-FDR / alpha-investing) — NEWLY SALIENT → build-candidate (report-only)
The one genuine honesty-hardening candidate. DSR deflates for the arms searched
*within one run* (n≈23-25). It does **not** correct the **cross-run** family
implicit in "we ran 32 symbol-runs across 6 corpora and are reading the ones that
crowned" — P2-wobble §1(d) named exactly this, and DOGE-2020 (crowned Sharpe 1.03
< B&H 1.83, a gate artifact) is the tell. **The corpus already holds the tool:**
online-FDR / alpha-investing (`backtesting[73]`) controls the false-"beats-hold"
rate across a *sequence* of re-runs; a static Šidák/Holm on the crown set is the
cheap version. **Post-P2 this is the newly-salient gap** the 32-run design created.
It **hardens honesty** (per the P3/PIT build precedent), is **report-only**, and
does **not** touch the frozen gate. **Rec: build-candidate — a report-annex line
(the expected false-positive count at α over N runs), NOT a gate change** (exactly
P2-wobble §4 item 4's scoping: completeness, not a blocker). If not built, its
**stated-limit** is already honest: "individual crowns are per-run DSR-graded;
cross-run family correction is noted, not gated."

### B8. Block-length / bootstrap-scheme sensitivity band (spec-curve on the block knob) — unbuilt → research-spike/leave
Named repeatedly (`backtesting[74][20]`, `data[1]` #1/#39): the gate uses ONE
data-driven Politis–White length per series; "sensitivity-check the verdict across
nearby lengths + MBB↔stationary" is flagged important but is not a standing check.
The P2 perturbation was on COST (a spec-curve slice), not block-length. **Rec:
research-spike / leave** — a one-off confirmation the weakest-link verdict is
block-length-stable, not a standing feature; low value since the length is already
data-driven per-series. (This also discharges A1's testable sub-question.)

---

## C. Post-P2 new questions the corpus predates

### C1. Dating the efficiency boundary (WHEN the anomalies decayed — our 2021-22→2023 transition) — partially answerable → research-spike (low-priority) / stated-limit
The corpus dates **post-publication** decay (McLean–Pontiff, event = a paper's
publication), NOT a market-wide **liquidity-era** boundary. Adjacent dating tools
DO exist in-corpus: Bai–Perron structural-break dating (`backtesting[71]`), BOCPD
(`data[45]`), the Hurst-efficiency trend (`data[43]`). Applying them to date the
**crowning-rate-vs-era** series (2017-18 67% → 2020 86% → 2021-22 80% → 2023-24 0%
→ 2025-26 20%) is a *new application*. **Tag: partially answerable-from-corpus.**
Honest ROI is low: the boundary is **un-actionable** (every advisor window ends at
"now"; there is no time machine — chasing the pre-boundary edge is
**register-blocked A-3 + survivorship**). **Rec: stated-limit** (the monotone
crowning-rate gradient already *is* the dated boundary, qualitatively; a formal
change-point date adds credibility-narrative polish, not advice) — a bounded
research-spike only if the operator wants the boundary date stated precisely.

### C2. Interpreting "crowning-rate ≫ noise-floor but 0/19 clear DSR" — ANSWERABLE from corpus → stated-limit (valuable insight)
Our exact old-era state post-NaN-fix: **60-86% crowning vs the ~20% pure-noise
floor** (the P2-2 CI number) yet **0/19 crowns clear the individual DSR≥0.95 bar**.
Is there an established interpretation? **Yes — and it resolves the apparent
P2-errata tension.** The FWER-vs-FDR / mixture-model distinction (`backtesting[85]`
Benjamini–Hochberg, `[30]` the FDR mixture that estimates the *proportion* of
configs with genuine edge, theme 12) gives the reading: when the **family-level**
crowning rate massively exceeds the null false-positive rate, that is evidence of a
real signal **population** even when **no single** crown clears the
individual-hypothesis bar. "0/19 DSR" (individual-level uncertified) and "real
efficiency migration" (family-level signal present) are therefore **both true and
not contradictory** — the FWER bar (per-crown, near-zero tolerance) and the FDR
lens (population proportion) simply answer different questions. **Tag:
answerable-from-corpus.** **Rec: stated-limit** — document this framing in the P2
lineage; it converts the "0/19 DSR" errata from an apparent weakness into a
**credibility strength** (the machinery correctly withholds individual
certification while the family signal is visible). The tempting mis-read — "0/19
DSR, so just wire the DSR veto" — is **register-blocked E-1**.

### C3. Survivorship-bias quantification for a small FIXED universe (P2's BTC/ETH/BNB survivor-of-survivors) — partially answerable → stated-limit
The corpus has survivorship generally: ~5pp/yr return inflation in an emerging
small-cap index (`data[7]`), and the standard fix = **include delisted/dead coins**
(`strategies[14]`). But for the P2 old-era slice this is **post-hoc and
un-reconstructable** — the 2017-18 dead top-10 (BCH/BCC forks, defunct ICO tokens)
have thin/gone data; the P2 note (c) correctly calls it "un-quantifiable from this
data / confounded by construction." **Tag: partially answerable** — the fix
(delisting-inclusion) is known and in-corpus, but retroactive *quantification* for
the frozen 3-coin slice is **unanswerable-in-principle** (the counterfactual
universe's data is unrecoverable). **Rec: stated-limit** — cite `data[7]`'s
~5pp/yr index-survivorship figure as the honest **upper-bound proxy**; do not
attempt exact quantification. The tempting fix — "go multi-coin to test on the full
old-era universe" — is **register-blocked B-1**.

---

## 4. Summary table (gap | kind | tag | recommendation)

| # | Gap | Kind | Tag | Recommendation |
|---|-----|------|-----|----------------|
| A1 | Era/regime-dependent GATE calibration across liquidity eras | A corpus | stated-limit | Document frozen-gate-across-eras as the honest choice; per-era tuning = researcher-DOF |
| A2 | Venue-microstructure EVOLUTION 2017→2026 as a quantified friction series | A corpus | stated-limit | Old-era crown margins = upper bounds; dedicated cost study is negative-ROI (never-advised window) |
| A3 | Retail lot-size / min-notional execution discretization | A corpus (structural) | stated-limit | Built right from exchange docs; literature cannot/does-not cover it |
| A4 | Crypto-specific anomaly-DECAY depth | A (premise-correction) | leave | NOT one citation — multiply-covered; only DATING is thin (→ C1) |
| A5 | Multiple-testing beyond DSR/PBO/MinBTL (SPA/StepM/MPPM/MCS…) | A (premise-correction) | leave | Fully in-corpus; it's an application deferral (→ B3-B6), not a coverage gap |
| A6 | Time-frozen staleness (corpus frozen 2026-06-28) | A corpus | leave | 2025-26 material confirms the migration prediction; no ingestible new method |
| B1 | Nonlinear Sharpe haircut (Holm/BHY) | B application | leave | Redundant-additive over shipped DSR |
| B2 | PBO via CSCV (Tune-surface, R2) | B application (known-deferral) | leave / research-spike | Homogeneous-grid-only reason STANDS; marginally more illustrative post-P2 |
| B3 | SPA / StepM studentization (P0 #8) | B application (known-deferral) | leave | Deferral VINDICATED — gate detected old-era edges without SPA's power |
| B4 | MPPM manipulation-proof score | B application | leave | Shipped skew/CVaR block covers the intent |
| B5 | Model Confidence Set ("is hold in it?") | B application | leave | `BenchmarkWins` + ADR-0066 exemption is the operative form |
| B6 | BBC-CV / ORATIO / difference-of-Sharpes | B application | leave | Redundant / moot-while-report-only (ORATIO-in-veto = E-1) |
| **B7** | **Cross-run family-wise multiple-testing (online-FDR `[73]`)** | **B application** | **build-candidate (report-only)** | **The P2 32-run cross-run family DSR misses; report-annex line, gate-untouched** |
| B8 | Block-length / scheme sensitivity band | B application | research-spike / leave | One-off verdict-stability confirmation; length already data-driven |
| C1 | Dating the efficiency boundary (2021-22→2023) | C new-question | stated-limit / research-spike | Tools exist (Bai–Perron/BOCPD); boundary is un-actionable (A-3) |
| **C2** | **Interpreting crowning-rate ≫ noise-floor but 0/19 DSR** | **C new-question** | **answerable → stated-limit** | **FDR mixture / FWER-vs-FDR: family signal + individual uncertainty are BOTH true — a credibility strength** |
| C3 | Survivorship quant for a small fixed universe | C new-question | stated-limit | Fix (delisting-inclusion) known; retroactive quant unanswerable-in-principle; cite `data[7]` upper-bound |

---

## 5. Summary (the required statements)

**A/B/C counts.** **A = 4** genuine corpus gaps (A1 era-gate-calibration, A2
venue-evolution series, A3 lot/min-notional, A6 staleness) **+ 2 premise-
corrections** (A4 decay well-covered, A5 SPA in-corpus). **B = 8** application
gaps (B1-B8), of which **7 = leave/research-spike** and **1 = build-candidate**
(B7). **C = 3** post-P2 new questions (C1 dating, C2 crowning-vs-noise, C3
survivorship-quant).

**The 3 most consequential gaps overall.**
1. **B7 — cross-run family-wise multiple-testing** (online-FDR / alpha-investing,
   `backtesting[73]`): the only genuine honesty-hardening build-candidate; the P2
   32-run design created a cross-run family DSR-within-run cannot catch; report-only,
   frozen-gate-untouched (P2-wobble §4 item 4 already scoped it).
2. **C2 — the "crowning-rate ≫ noise-floor but 0/19 DSR" state is answerable from
   the corpus** (FDR mixture / FWER-vs-FDR): resolves the apparent P2-errata tension
   into a credibility strength — the highest-value *insight* (not a build).
3. **A1 — era-dependent gate calibration is a real corpus blind-spot**, but the
   honest answer (one frozen gate across eras, never per-era tuning) is already our
   posture and is supported by the corpus's pre-registration ethos → stated-limit.

**Which known-deliberate deferrals the ground SHIFTED under vs which stand.**
- **SHIFTED — vindicated:** **SPA/StepM (B3)** — P2 proved the frozen gate detected
  the real old-era edges *without* SPA's power boost; the deferral is now
  empirically confirmed, not merely deferred.
- **SHIFTED — newly salient:** **cross-run FWER (B7)** — the 32-run P2 design
  created this family; it is the honest completion of the (d)-multiple-testing
  defense.
- **SHIFTED — marginally, reason still valid:** **PBO-on-the-Tune-surface (B2)** —
  the old-era crowns are a better illustration case, but CSCV stays honest only on
  the homogeneous grid, so it remains deferred.
- **STAND unchanged:** **ONC-N_eff** (closed-form sufficient at N=24; the NaN-fix
  *strengthened* the closed-form path), **tail-stressed/EVT slice (D-4)** (the
  corpus-expansion added real worse-crashes as *data* for old-era windows, not as a
  generator; the "can't invent a worse-than-seen crash" limit is unchanged for
  recent windows), **MPPM/MCS/BBC-CV/ORATIO/haircut** (redundant-additive or
  moot-while-report-only), **cost-model default (E-2)** (still opt-in; a default
  bump re-emits 119 anchors for ≈0 gain).

**register-blocked list (ids only, listed for completeness, NOT recommended).**
The tempting-fixes that map to settled dead-ends: **A-3** (re-enable old-era
winning arms / automated search), **A-5** (add arms to chase the old-era edge),
**B-1** (multi-coin to "fix" survivorship), **D-4** (tail-stressed/EVT
worse-than-seen-crash generator), **E-1** (wire the DSR crown-veto because 0/19
clear DSR), **E-2** (bump the cost-model default to capture old-era frictions).

**Gate results (verbatim).**
- `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (119 / 119)` — before AND after
  this write (this note is a `docs/dev-notes/` memo, not an anchored report under
  `spec/**/reports/`, so it cannot perturb a body-SHA).
- `python3 scripts/spec_lint.py` → `spec-lint: FAIL (4 violations in 3 categories)`
  — **pre-existing and environmental**, unchanged by this note (delta 0). All four
  are the gitignored `data/binance/REVISION.toml` not being materialized in this
  checkout (`git check-ignore` confirms it is ignored) + one pre-existing
  `orphan-feature` (`spec/operator-success-reports`): `dead-link` ×1
  (ADR-0040 → `data/binance/REVISION.toml`), `trace-broken-path` ×2
  (`REQ-LAB-YAHOO-REALDATA-001`, `REQ-V5-LATENCY-SLIPPAGE-V0-4-0-001` →
  `data/binance/REVISION.toml`), `orphan-feature` ×1. None is touched by this
  dev-note (no trace/ADR/data edits); in a checkout with the corpus materialized
  these resolve to PASS(0). This note introduces **zero** new violations.

---

### Handoff (informational — no agent spawned)

This is analyst decision-support (a gap MAP), not a feature spec — per the
maintenance-mode posture it does NOT spawn the architect and authors NO `[[req]]`
row. IF the operator elects the one build-candidate (**B7**, the cross-run
false-positive-count report-annex line), the analyst would then author the trace
row and hand to the architect per the normal spine. Everything else is
stated-limit / leave — the honest-null outcome the product's own thesis predicts
for a corpus that already drove its ship-worthy tranche to completion.
