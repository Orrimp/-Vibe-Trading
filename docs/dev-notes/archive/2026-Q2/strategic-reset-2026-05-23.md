---
slug: strategic-reset-2026-05-23
date: 2026-05-23
authors: analyst
status: proposed
related:
  - docs/dev-notes/feature-state-table-2026-05-22.md
  - docs/dev-notes/feature-state-analyst-review-2026-05-22.md
  - docs/dev-notes/feature-state-architect-review-2026-05-22.md
  - docs/dev-notes/v25-dl-journey-retrospective-2026-05-22.md
  - spec/product.md
---

# Strategic reset — 2026-05-23

> **R3 half-day reset.** Eight features shipped in one session (3 retirements
> + 1 P0 fix + 1 PARTIAL ship + cleanup sweep). Before committing the next
> 6-8 weeks, answer one question with the depth of evidence accumulated this
> week.

## Section 1 — The reset question

**Has the project's moat thesis been validated, or are we still asking the
same question 8 weeks later?**

The moat per `spec/product.md` § Differentiator (lines 67-83) is **(1) Rust
foundation + (2) persistent reflection memory + (3) type-encoded risk +
(4) auditable double-entry ledger**, with `(2) + (4)` named explicitly as
the long-term moat (the 2026-04-17 confirmed bet, product.md line 78-79).
Components (1) and (3) are tablestakes; the differentiator from commodity
quant platforms is memory + audit.

Three candidate answers, each with different routing implications:

- **(A) "Still asking"** — moat thesis is neither validated nor refuted.
  The forecaster-track exhaustion (4 retirements) consumed bandwidth that
  should have moved moat-aligned work forward. Implication: the 6-8 week
  tactical roadmap is rearranging deck chairs; need a *strategic* pivot
  before tactical work resumes.
- **(B) "Validated, but operator-invisible."** Components (2)+(4) are
  built + load-bearing + tested. But the operator (and any future user)
  can't *see* the moat from the cockpit — there's no UI surface that says
  "the lessons cards retrieved this hour caused this trade." Implication:
  reflection-PnL-correlation surface + 7-day acceptance run +
  operator-success-report integration are the unfilled gap.
- **(C) "Genuinely don't know yet."** Both prior interpretations have
  partial evidence. The forecaster-track exhaustion has clouded the
  moat-thesis assessment because the bandwidth went to alpha hunting, not
  moat validation. Implication: a tighter experimental design *is* the
  next move — specifically, a single 7-day continuous-paper run with
  reflection memory writing **and** being CONSUMED by a strategy.

The honest answer is in Section 3. I will argue it from the evidence in
Section 2.

---

## Section 2 — Evidence audit

Twelve major decisions across ~3 weeks of 2026-Q2 (2026-05-03 through
2026-05-23). One row per decision; the "Pattern" column tags whether the
decision strengthened the moat, hunted alpha, built tablestake
infrastructure, or closed a research line.

### 2.1 The decision table

| # | Date | Decision | Predicted outcome | Actual outcome | Pattern |
|---|------|----------|-------------------|----------------|---------|
| 1 | 2026-05-03 | Ship `journal-transactions-metadata` v1.6.1 + `per-symbol-position-accounts` v1.4.0 | Double-entry ledger + per-symbol accounts shipped; reused by every strategy | Both shipped + load-bearing; migrations 005/006 stable | **moat-strengthening (component 4)** |
| 2 | 2026-05-08 | Ship `reflection-memory` v1.8.0 (LessonCard store + top_k + 3-state regime tagger) | Memory infrastructure for future strategies to consume | Shipped + load-bearing; `crates/reflection` consumed by `v3-llm-forecaster` Wave A+C | **moat-strengthening (component 2)** |
| 3 | 2026-05-13 | Ship `v2-llm-strategy` v2.0.0 (LlmProvider trait + Recording/Replay + BudgetedProvider) | LLM infrastructure; foundation for any future LLM-driven feature | Shipped; later reused by `v3-llm-forecaster`; replay-cache pattern hardened | **infrastructure (moat-enabling, not moat-completing)** |
| 4 | 2026-05-17 → 2026-05-21 | Ship `ui-rethink-phase-{a,b,c,d,d-followup,e,f}` (6 phases) | Multi-screen cockpit with Lab + Live + Trail + Compare + Memory + Models + Assistant slot | All 6 phases shipped; Phase F surfaces Memory + Assistant slot (moat-visibility) | **moat-aligned UI (Phase F) + neutral scaffolding (A-E)** |
| 5 | 2026-05-19 | Ship `v25-tcn-alpha-investigation` v0.3.0 (F-verdict ADR-0033 § D3) | Forensic read-only investigation of v2.5 TCN | F4 verdict on BS-1+BS-2; ADR-0033 § D3 immutable | **alpha-hunting (failed)** |
| 6 | 2026-05-20 | Ship `v25-tcn-recalibrate` v0.1.0 (σ_train ADR-0035) | Fix the 608× calibration bug; gate-survival jumps | Bug fixed; gate-survival 0% → 40-89%; F-verdict still **F4** | **alpha-hunting (confounder removed; signal still absent)** |
| 7 | 2026-05-21 | Ship `v25-tcn-threshold-tuning` v0.1.0 (τ × ε sweep) | Find the marginal τ that unlocks alpha | Joint T-MARGINAL (+0.018 / +0.045); below +0.10 unlock gate | **alpha-hunting (signal genuinely weak)** |
| 8 | 2026-05-21 | Ship `v25-tcn-horizon-bump-or-retire`; operator picked (b) RETIRE; pivot to PatchTST | Retire 1h-TCN; reallocate budget to PatchTST | TCN @ 1h retired; PatchTST @ 24h spun up | **retirement #1 (within forecaster track)** |
| 9 | 2026-05-21 | Ship `v25a-patchtst-overlay` v0.1.0 | PatchTST @ 24h unlocks signal where TCN @ 1h failed | F4 + Sharpe-delta **+0.006** (LOWER than retired TCN); H1 falsified | **alpha-hunting (paradigm-family hypothesis falsified)** |
| 10 | 2026-05-22 | Retire `v25-dl-forecast-overlay` umbrella + 4 children (v25b + v26 + 2 phase-2 deprecations) | Joint F4-F4-F4 retires v2.5 DL paradigm; ~3-5 weeks budget freed | Retirement #2 (programme-scale); operator routing (a) RETIRE | **retirement #2 (programme-scale)** |
| 11 | 2026-05-22 morning | Ship `v3-volatility-forecaster` v0.1.0 (Candidate 1) | GARCH vol-targeting unlocks +0.10 Sharpe-delta via risk-layer sizing | Joint V3 × T-VOL-NO-ALPHA; rebaseline shipped same day with `net_delta = 0.000000` | **alpha-hunting (failed) → triggered noop discovery** |
| 12 | 2026-05-22 afternoon | Discover vol-targeting overlay was a **no-op** via caveman probe; ship `v3-volatility-forecaster-noop-fix` v0.1.0 (P0) | Wire the scale-into-fill-quantities path; re-emit anchors per ADR-0038 § D6.b | Fix shipped; **NEGATIVE-NET-DELTA (-0.021719); equity -44.6%** under real wiring; C1 + rebaseline retired; C5 promoted | **P0 engineering fix + retirement #3 + moat-aligned promotion** |
| 13 | 2026-05-22 evening | Ship `v3-llm-forecaster` v0.1.0 (shipped-partial); Wave D deferred indefinitely | LLM-as-forecaster ships 6 of 7 waves; Wave D needs ANTHROPIC_API_KEY | 6 waves shipped; Wave D paused; **shipped-partial precedent established** | **moat-aligned strategy (PARTIAL); first signal-source that consumes reflection memory** |

### 2.2 Aggregate pattern counts

| Pattern | Count | Notes |
|---|---|---|
| Moat-strengthening (components 2/4) | 2 | Decisions #1, #2 — both shipped pre-2026-05-13 |
| Infrastructure (moat-enabling) | 1 | Decision #3 — v2-llm-strategy; consumed by v3-llm later |
| Moat-aligned UI | 1 | Decisions #4 — Phase F specifically (Memory + Assistant slot); A-E neutral |
| Alpha-hunting (forecaster track) | 5 | Decisions #5, #6, #7, #9, #11 — all retired or marginal |
| Retirement | 3 | Decisions #8, #10, #12 — 2 within forecaster track + 1 programme-scale + 1 chain-retirement |
| P0 engineering | 1 | Decision #12 — noop-fix |
| Moat-aligned strategy (PARTIAL) | 1 | Decision #13 — v3-llm-forecaster |

**Read.** Of 13 major decisions in 3 weeks:
- **3 directly strengthened the moat** (decisions #1, #2, #13). All three
  are foundation work or the first strategy that consumes the moat as a
  signal source. None of them shipped *visibility* into the moat for the
  operator.
- **5 alpha-hunting ships ALL failed or marginal** (decisions #5, #6, #7,
  #9, #11). Joint F4-F4-F4 across DL paradigms + V3 × NO-ALPHA on GARCH.
- **3 retirements** (decisions #8, #10, #12 + chain retirement of #11).
  The retirement protocol is the maturest workflow surface in the project.
- **1 P0 fix surfaced a 5-gate blind spot** (decision #12). This is the
  most important engineering finding in the arc.

### 2.3 The forecaster-track ratio

The forecaster track (decisions #5, #6, #7, #8, #9, #10, #11, #12 +
rebaseline) consumed **~10-15 weeks of analyst + architect + dev + tester
budget plus multi-day training compute** (per prior analyst review section
B). Output:
- 4 retirements (TCN @ 1h, PatchTST @ 24h, v3-vol GARCH, v3-vol rebaseline).
- 1 paradigm-family retirement (v2.5 DL umbrella; 3 deprecated children).
- 1 P0 wiring-bug discovery (the noop-fix).
- 0 strategies that survived the +0.10 Sharpe-delta gate.
- 6 ADRs locked (0028, 0029, 0032, 0033, 0035, 0036, 0038) — high
  evidential value for future research; near-zero shipped-product value.

The moat-strengthening ratio across this 3-week window is **3 of 13 = ~23%**
direct moat work, with the forecaster track absorbing **62%** of decisions
and **~75%** of agent-hours by my read.

### 2.4 The five anchored-but-deferred surfaces

Per `spec/product.md` § v3 success metric (lines 399-405): **"90 days
continuous paper-trading on real Binance market data with simulated fills,
weekly auto-generated operator success reports, lesson-card memory
demonstrably accumulating, uptime > 99%, zero risk-limit breaches, LLM cost
inside the v2 monthly budget."**

What's shipped vs deferred against the v3 success metric:

| v3 sub-metric | Status | Evidence |
|---|---|---|
| 90 days continuous paper on real Binance data | **Not started** | `live-cockpit-unified` v1.5.0 ships the runtime; no continuous-paper feature folder exists |
| Weekly auto-generated operator success reports | **On-demand only** | `operator-success-reports` v1.7.0 shipped; scheduler not wired |
| Lesson-card memory demonstrably accumulating | **Infrastructure shipped; consumer thin** | `crates/reflection` v1.8.0 + Phase F UI shipped; only `v3-llm-forecaster` Wave A+C consumes (and is shipped-partial; Wave D pending) |
| Uptime > 99% | **Not measured** | No 24/7 monitoring surface; no uptime SLI |
| Zero risk-limit breaches | **Cannot measure** | Per architect review § E.1: only the `.halt` kill switch ships; daily-loss-stop / max-DD trigger / per-symbol cap / max-leverage NOT shipped |
| LLM cost inside v2 monthly budget | **Bounded by infra; untested at scale** | `BudgetedProvider` shipped; no continuous run has exercised the 80%/100% degrade rules |

**Read.** *None* of the 6 v3 success sub-metrics are demonstrably true
today. Infrastructure for 4 of 6 exists in some form. Two (uptime + risk
breaches) are not even measurable yet.

### 2.5 The reflection-memory loop is half-closed

The moat is reflection memory **+** auditable double-entry. The audit
ledger is fully closed: every fill produces journal rows; per-symbol
position accounts reconcile; MTM ticks emit; ADR-0014 § Q9 "strategy
proposes, risk disposes" holds at sizing time.

The reflection-memory loop is **half-closed**:
- **Write side**: lesson cards land in `crates/reflection` via the
  `ReflectionWriterTap` in `exec::PaperEnginePublisher`. Per closed
  trade → one lesson card. Shipped + load-bearing.
- **Read side**: `top_k` retrieval works; Phase F Memory screen surfaces
  cards to the operator; `v3-llm-forecaster` Wave A+C consumes top_k as
  LLM-prompt context.
- **Closing-the-loop side (the missing half)**: no strategy in production
  uses lesson-card retrieval as a *load-bearing* signal source that moves
  equity. v3-llm-forecaster Wave A+C consumes cards in the prompt, but
  Wave D (the wave that produces the verdict on whether the LLM signal
  unlocks alpha when reading those cards) is **deferred indefinitely** per
  the shipped-partial precedent.

The moat exists as **infrastructure** but not as **demonstrated value**.
We can show a future user the lesson cards. We cannot show that the
lesson cards make the trader better.

---

## Section 3 — The honest answer

> **Section 3 is the load-bearing section. I argue (D), with rationale.**

### 3.1 None of (A), (B), (C) fits cleanly

- **(A) "Still asking"** is too strong. We HAVE built the moat
  infrastructure: `crates/reflection` v1.8.0 + audit ledger + journal
  + per-symbol accounts + Phase F UI all ship and are load-bearing.
  The infrastructure work is real and is not the gap.
- **(B) "Validated, but operator-invisible"** is too generous. The
  moat-as-infrastructure is built; the moat-as-evidence is not. To say
  it's "validated" requires evidence that the moat *does something* — i.e.
  that lesson-card retrieval correlates with PnL, or that the audit trail
  catches a wiring bug a non-audit-driven project would miss. We have
  **one** piece of evidence on (4) — the noop-fix was *caught* by the
  byte-identity-anchor property, which IS an audit-derived gate. We have
  **zero** pieces of evidence on (2) — no production trade has been
  causally linked to lesson-card retrieval.
- **(C) "Genuinely don't know"** is closer to the truth but understates
  what we *do* know. We know the infrastructure works. We know the
  forecaster track did not validate the moat (because the forecaster
  signal sources are commodity, not memory-derived).

### 3.2 Answer (D): The moat-thesis question is half-answered, and the
unanswered half has shifted in shape

The moat thesis decomposes into **three** sub-claims, only one of which
has had a fair test:

1. **(D.1) "The audit ledger is a load-bearing observability surface, not
   a compliance trinket."** STATUS: **VALIDATED** — by the noop-fix
   incident, where the byte-identity property of anchored audit
   reports was the only signal that the vol-targeting overlay was a no-op.
   5 layers of gating missed it (unit, clippy, anchor as feature, architect
   M-T1, tester M-FINAL); the byte-identity-as-smoking-gun *did* surface
   it within 8 hours of the rebaseline ship. This is the strongest piece
   of moat-validating evidence in the project.

2. **(D.2) "Persistent reflection memory is a load-bearing signal source,
   not a UI feature."** STATUS: **NOT YET TESTED**. The only strategy
   that consumes reflection memory as input to a decision (v3-llm-
   forecaster) is shipped-partial; Wave D — the wave that produces the
   verdict on whether the memory-driven signal unlocks alpha — is deferred
   indefinitely. We have built the lane; nothing has run a lap on it.

3. **(D.3) "The (2)+(4) combination compounds — memory makes the trader
   better AND the audit trail proves it."** STATUS: **CANNOT BE TESTED**
   until D.2 is tested, because the compound claim presupposes D.2.

**Sub-claim (D.1) is validated as a side effect of the noop-fix incident.
Sub-claim (D.2) has not been tested. Sub-claim (D.3) is downstream.**

### 3.3 What changed this week that the prior analyst review didn't see

The prior analyst review (2026-05-22) framed the forecaster track as
"anti-moat in opportunity cost." I agree with that read. But it missed
one thing: **the noop-fix is the strongest piece of moat-validating
evidence the project has ever produced**, and it came out of the
forecaster track.

The noop-fix story:
1. v3-volatility-forecaster shipped with a synthetic-baseline-vs-real-
   overlay net_delta of `0.029868` — non-zero, framed as "small but
   informative."
2. The rebaseline pass shipped with real-vs-real net_delta of
   **`0.000000`** — byte-identical.
3. The operator flagged the byte-identity as suspicious. The "anchored
   report bodies are byte-identical to baseline" property — which is
   built into the audit ledger's anchoring contract — was the only
   signal that the wiring was wrong.
4. 30-minute caveman probe → smoking gun at `vol_targeting_overlay.rs:309-319`.
5. P0 fix → real overlay produced **NEGATIVE-NET-DELTA (-0.021719)**.

The byte-identity property is not a moat per se. But it is *enabled by*
the moat (the audit ledger's anchored body-SHA contract). A project
*without* the audit-driven anchor protocol would have shipped the no-op
vol-targeting overlay, declared NO-ALPHA, and moved on — never knowing
the overlay was structurally broken. The audit-trail moat *caught a real
bug that 5 gates missed*.

This is direct evidence that **component (4) — auditable double-entry +
the anchor contract it enables — is operationally load-bearing**. Not as
a compliance feature; as a debugging substrate. This isn't speculative;
it's a 2026-05-22 production incident with a code commit and a
re-emission protocol amendment.

### 3.4 What the noop-fix does NOT validate

The noop-fix does NOT validate sub-claim (D.2) — memory-as-signal-source.
Reflection memory was not consulted in the noop-fix discovery; the
discovery was anchor-driven (component 4), not memory-driven (component 2).

The forecaster track consumed 10-15 weeks of budget pursuing alpha
sources that, even if they had worked, would not have been
memory-derived. The TCN, PatchTST, and GARCH forecasters are
commodity quant primitives; none of them touch `crates/reflection`.

**The forecaster track therefore did three things, only one of which
strengthened the moat thesis:**
1. Falsified the v2.5-era DL-on-OHLCV paradigm (high evidential value;
   moat-neutral).
2. Surfaced the noop-fix incident, which validated component (4) of the
   moat (high evidential value; moat-validating for D.1).
3. Did not test the memory-as-signal-source hypothesis (D.2) at all.

### 3.5 Why this matters for the next 6-8 weeks

If the load-bearing question is "has the moat been validated," the answer
is:

- **(4) — auditable double-entry — VALIDATED**. Anchored byte-identity
  caught a bug that 5 gates missed. This is operational evidence, not
  speculation.
- **(2) — persistent reflection memory — UNTESTED**. The only feature
  that would have tested it (v3-llm-forecaster Wave D) is deferred
  indefinitely.
- **(2)+(4) compound moat — NOT TESTABLE until (2) is tested**.

We are **half-way to validating the moat**. The unfilled half is the part
the prior analyst review correctly identified as "operator-invisible"
(answer B), but the framing should not be "make the moat visible." The
framing should be: **test whether the memory half does anything, before
investing more in UI surfaces that show off something that hasn't been
demonstrated to work**.

This is the single load-bearing strategic finding. It changes the
sequencing of the next 6-8 weeks.

---

## Section 4 — Implications for the next 6-8 weeks

> The orchestrator's hinted 5-step roadmap (Wave D → exec rename → risk
> envelope → reflection-PnL → composed feature) is roughly right, but the
> *sequencing* is wrong if Section 3's answer is correct. I argue from
> first principles.

### 4.1 First principle: test the untested moat half before building the
visibility layer

If (2) — memory-as-signal-source — turns out to be noise-equivalent (LLM
ratings uncorrelated with future returns), then:
- The reflection-PnL-correlation surface (orchestrator step 4) becomes
  a UI for showing operator-visible nothing.
- The 7-day continuous-paper acceptance run (orchestrator step 5)
  shows lesson cards accumulating but not affecting trades.
- The composed feature (orchestrator step 5 of 5) becomes a v1-momentum-
  with-passive-memory-write, which is just v1 momentum with extra audit
  rows.

Conversely, if (2) is shown to be load-bearing (LLM signal correlates with
forward PnL), then:
- The reflection-PnL-correlation surface lands AFTER it has something to
  surface — a real correlation, not a hypothetical one.
- The 7-day acceptance run actually validates D.3 (the compound moat).
- The composed feature shows the operator a working moat, not a UI
  promise.

**Sequencing implication: Wave D goes FIRST**, not last.

### 4.2 First principle: drawdown is the binding constraint, not Sharpe

v1 momentum has 73% max drawdown on 2023-FY real Binance data (per
caveman-probe forensic output). The product.md `paper → live` gate
specifies `Sharpe > 1.0 on 2y OOS data + no fatal regressions` — silent
on drawdown. A 73% max-DD on paper means a continuous-paper acceptance
run will, with non-trivial probability, observe a multi-day drawdown
episode of 30-50% that has nothing to do with the moat hypothesis and
everything to do with v1's structural drawdown profile.

If the acceptance run is structured naively ("run v1 24/7 and emit weekly
reports"), the most likely failure mode is **drawdown-event-dominates-
narrative**, not **moat-fails-to-fire**. The operator-success-report will
spend 80% of its tokens on the drawdown event and 20% on the lesson-card
retrieval question, which inverts the evidential priority.

**Sequencing implication: the risk envelope is a precondition for the
acceptance run, not a parallel deliverable.** Without it, the acceptance
run cannot produce decision-grade evidence about D.2/D.3.

### 4.3 First principle: do not compose work that hasn't been individually
verified

The orchestrator's step-5 composed feature ("`v3-continuous-paper-success-
cycle` that includes risk envelope + canonical baseline + Wave D + memory
surface + 7-day run") is a 5-component composition. The vol-targeting
noop-fix is a case study in why composition without per-component end-to-
end verification fails: the math was right (compute_scale), the wiring
was wrong (scale never read), and the composed test (anchor + tester)
witnessed the failure but couldn't interpret it.

**Sequencing implication: each precondition ships and gets its own
verdict-grade evidence BEFORE the composed acceptance run.** Wave D
verdict separately. Risk envelope separately. Memory-PnL-correlation
queries against historical (non-live) audit rows separately, to confirm
the query shape works before the acceptance run depends on it.

### 4.4 The re-prioritized list

| Move | EV | Cost | Prerequisite | Tests which moat half? | Recommended sequence |
|---|---|---|---|---|---|
| **M1: v3-llm-forecaster Wave D (real backtest + canonical cache)** | **HIGHEST** — fastest path to a verdict on D.2 | 0.5-1 day work + $25-50 LLM spend | ANTHROPIC_API_KEY | **(2) directly** | **FIRST** |
| **M2: Risk envelope v0.1.0 (daily-loss-stop + max-DD trigger + per-symbol cap)** | HIGH — gates any continuous-paper work | ~2 weeks | None — extends `crates/risk/` | Neither (tablestakes) | **SECOND (in parallel with M1 if developer bandwidth allows)** |
| **M3: Canonical v1 momentum baseline anchor** | MEDIUM-HIGH — fixes the apples-to-apples baseline problem the prior analyst review flagged | ~1 day | None | Neither (hygiene) | **SECOND (cheap; do alongside M2)** |
| **M4: Reflection-PnL-correlation query (NOT UI surface yet)** | MEDIUM — proves the query shape works | ~3-5 days | M1 verdict ≠ "memory-noise-equivalent" | **(2) instrumentation; sets up D.3** | **THIRD (gated on M1 verdict)** |
| **M5: Reflection-PnL-correlation UI surface in operator-success-reports** | MEDIUM-HIGH if M1 is positive; LOW if M1 is negative | ~2 weeks | M1 verdict + M4 query | **(2) operator-visible** | **FOURTH (gated on M4)** |
| **M6: 7-day continuous-paper acceptance run** | HIGH if M2 + M5 land; the v3 success-metric demonstrator | ~1 week wall-clock + monitoring | M2 + M5 | **D.3 (compound moat)** | **FIFTH** |
| **M7: `crates/exec/` rename + LiveMatchingEngine scaffold** | MEDIUM — per architect review § E.1; not needed for paper-trade-live, needed only for testnet/real | ~1 week rename + 3-4 weeks LiveMatchingEngine | None (rename) | None (tablestakes) | **DEFER beyond 6-8 weeks** — product.md explicitly out of scope (no real-money) |
| **M8: v3-regime-classifier (C2 promotion)** | LOW — moat-LOW; prior analyst review's "defer (R-O3 fallback)" stands | 4-6 weeks | None | None (commodity regime tagger) | **DEFER** |
| **M9: cockpit-app-bundle** | LOW (phantom audience for single-operator product) | 1-3 weeks | None | None | **DEFER** |

### 4.5 The 6-8 week sequence

```
Week 1     M1 (Wave D real-API)         M2 (risk envelope dev)   M3 (canonical anchor)
Week 2     M1 verdict GATE              M2 cont'd                M3 done
              │
              ├── M1 verdict = "memory-driven signal correlates with PnL"
              │      ↓
Week 3        M4 (correlation query)    M2 done
Week 4        M5 (correlation UI)
Week 5        M5 cont'd
Week 6        M6 (7-day acceptance run kickoff)
Week 7        M6 cont'd
Week 8        M6 verdict → operator-success-report demonstrates D.3
              │
              ├── M1 verdict = "memory-driven signal noise-equivalent"
              │      ↓
Week 3        STOP. Do NOT build M5/M6. Operator-decide moment:
              - (a) Reformulate the memory consumption (different LLM
                prompt shape; different lesson-card retrieval policy;
                different signal extraction)
              - (b) Accept the moat is auditing-only (D.1 valid, D.2
                invalidated); pivot to D.1-amplifying work (e.g.
                audit-driven anomaly detection, audit-driven
                attribution reporting)
              - (c) Re-survey strategy directions per
                strategy-reformulation-survey-2026-05-22.md
                Candidate 7 (full strategy-side reformulation)
```

**The branch point at M1's verdict is the load-bearing decision in the
next 6-8 weeks.** Everything downstream depends on whether the LLM-as-
forecaster signal source has measurable correlation with forward PnL.

### 4.6 What I am NOT recommending

- **NOT recommending "go live with v1 momentum."** Per prior analyst
  review section C, v1 alone has 73% max-DD and zero memory-loop
  consumption; running it continuously without the risk envelope
  produces a deck about drawdowns, not about the moat. (`live` in the
  product.md sense is continuous-paper-on-real-data, not real-money —
  but the failure mode applies to paper too.)
- **NOT recommending "promote v3-regime-classifier (C2)."** Per the
  strategy-reformulation survey, C2 is moat-LOW (regime classification
  is commodity quant). It belongs in the R-O3 fallback slot if Wave D
  comes back noise-equivalent AND option (c) in section 4.5 is the
  chosen pivot.
- **NOT recommending starting `crates/exec/` rename + LiveMatchingEngine
  now.** Per product.md § Non-goals, real-money execution is explicitly
  out of scope; the live-trading scorecard (architect review § E.1) is
  preparing for a follow-up project, not the current one. Defer.

---

## Section 5 — Strategic questions for the OPERATOR

The analyst's job is to surface the questions the operator needs to
answer; not to answer them. These are the questions where my judgment
runs out and the operator's judgment is the binding constraint.

### Q-OPERATOR-1: What is the 12-month success criterion?

The product.md v3 terminal state is "90 days continuous paper on real
data + weekly operator-success-reports + lesson-card memory
demonstrably accumulating." Reading this literally: the project is done
when you have an operator-success-report deck that demonstrates D.3 (the
compound moat). Reading it strategically: the project's terminal state
is the *evidence package* for a follow-up project.

If your 12-month success criterion is:
- **(a) "Ship a personal trading tool I personally use."** Then the
  recommended sequence is right; M1→M6 ends with a tool that works for
  you specifically.
- **(b) "Produce a research artifact / paper / public writeup."** Then
  M1's verdict is the load-bearing experimental result, and the failure
  cases (Wave D noise-equivalent) are *also* publication-worthy. The
  sequence stays similar but M5/M6 may not need to ship — the verdict +
  retrospective IS the artifact.
- **(c) "Build a research platform for future users."** Then the moat
  visibility surface (M5) is more important than M6, because it's what
  future users will see. The 7-day acceptance run becomes a demo, not a
  product.

These three answers route the next 6-8 weeks differently. The Section 4
sequence assumes (a). If you're optimizing for (b) or (c), Section 4 needs
re-pricing.

### Q-OPERATOR-2: If the moat is (2)+(4), what does "operator-visible
moat" look like?

I have been writing as if "operator-visible moat" means a UI surface in
operator-success-reports. But there are at least three operationalizations:

- **UI surface** — operator-success-reports gets a "lesson cards retrieved
  this week, sorted by PnL of trades that retrieved them" section.
- **Research paper / writeup** — a public document with the noop-fix
  case study, the Wave D verdict, and the architecture as the artifact.
- **Venue migration** — port the entire moat stack (reflection + audit)
  to a non-crypto market (e.g. paper-traded equities on a free venue)
  and show the moat compounds in a different domain. This is far out of
  scope but is the strongest demonstration of "moat that compounds in any
  asset class."

If your answer to Q-OPERATOR-1 is (a), the UI surface is the answer.
If (b), the writeup is the answer. If (c), maybe the venue migration is
the answer.

### Q-OPERATOR-3: Are you willing to accept Wave D coming back
noise-equivalent and pivoting to D.1-only?

The probabilistic argument from Section 3 makes Wave D the load-bearing
experiment. The survey rated C5 (LLM-as-forecaster) as LOW-MEDIUM prior
of clearing +0.10 Sharpe-delta (K-llm-3 in survey lines 519-530). On
that prior, the most likely outcome is Wave D comes back with the LLM
signal showing low/no correlation with forward PnL.

If that happens, you have three options (4.5 branch points (a)/(b)/(c)).
Option (b) — accept the moat is D.1 (audit) only, not D.2 (memory) — is
honest but means the product narrative shrinks. Option (a) — reformulate
memory consumption — risks repeating the forecaster-track failure mode
(reformulate, retest, retest, retire).

**The question for you**: how many retries on memory-as-signal-source are
you willing to fund before accepting D.2 is "infrastructure-only, not
load-bearing"? If the answer is "two more after Wave D" you're committing
to roughly 6-12 more weeks of memory-consumption research. If the answer
is "zero more," then Wave D is decision-grade evidence.

### Q-OPERATOR-4: The session-end "Done" — what does Done mean?

The operator-session ended with "Done" after 8 features in one session.
That's a remarkable per-session throughput, but the cadence is hiding
something: each session is a tactical sprint, and the strategic question
has not had a session of its own this entire quarter. The R3 reset
half-day is the first attempt to give the strategic question dedicated
bandwidth.

If "Done" means "I'm satisfied with the tactical sprint cadence and want
to keep doing this," fine; the Section 4 sequence is the next tactical
sprint.

If "Done" means "I'm questioning whether this cadence is producing the
right thing," then the answer in Section 3 — that the moat thesis is
half-validated and the unvalidated half is the bottleneck — is the
diagnostic. The cadence is fine; the *routing criterion* for what to
ship next has been alpha-hunting-biased for 3 weeks. The next 6-8 weeks
should be **moat-validation-biased** (M1 first), not alpha-hunting-biased
(another C-candidate from the survey).

This is the most important question of the four. The cadence isn't the
problem; the prior on "what's worth shipping" is the problem. Until
M1's verdict comes back, every other ship is downstream of an
unanswered question.

---

## Section 6 — Workflow meta-lessons (bonus)

Beyond the strategic-reset brief, this week's session produced ~8
engineering patterns worth codifying. The orchestrator added two to
CLAUDE.md in parallel:
- "every overlay strategy ships with a baseline-equity-divergence test
  from day 1"
- "anchored reports are byte-immutable; documentation-link-fix sweeps
  must invoke ADR-0038 § D6.b"

Here are six more, ranked by ROI.

### 6.1 [CLAUDE.md] "diagnostic-only" comments in load-bearing code are a
code smell, full stop

**Evidence.** The vol-targeting overlay at lines 309-319 carried the
inline comment "diagnostic only — the backtest engine reads quantities
from fills, not from signal metadata." That comment correctly identified
the architectural constraint AND admitted the design intent was abandoned
without replacement. It was the smoking gun, visible in the source for
weeks, missed by 5 review layers.

**Proposed addition to CLAUDE.md § Non-negotiables**:

> A "diagnostic only" or "TODO: wire this" comment in load-bearing code
> path requires either (a) a corresponding TODO row in the feature's
> tasks.md, (b) an issue in docs/dev-notes/, or (c) removal of the
> code path. Comments admitting abandonment without a tracked follow-up
> are forbidden in shipping code.

**Cost**: small grep across `crates/strategy/` etc. to find existing
instances. ~30 min architect.

### 6.2 [AGENT.md] When two-runs-byte-identical is *suspicious*, not
*confirmatory*

**Evidence.** The byte-identity anchor protocol is designed to catch
non-determinism. It cannot tell the difference between "two runs of the
same computation produce identical output" (the property it tests) and
"two runs of different-but-no-op-different computations produce identical
output" (the bug it cannot detect). The vol-targeting overlay's byte-
identity with the un-targeted baseline was the no-op signature.

**Proposed addition to AGENT.md § Tester checklist**:

> When two anchored reports are byte-identical AND the runs were
> structurally different (different overlays / different strategies /
> different gating), flag for analyst review BEFORE locking the anchor.
> Byte-identity across structurally-different runs is the no-op
> signature; byte-identity across structurally-same runs is the
> determinism signature. They look the same; they are not.

**Cost**: 1 paragraph in AGENT.md + tester-skill checklist row.

### 6.3 [AGENT.md] The caveman probe is a first-class diagnostic tool

**Evidence.** The vol-targeting noop was discovered by a 30-minute manual
perturbation: multiply `sigma_hat` by 2.95 in the source, recompile, run
the backtest, observe byte-identical output. This is "stick a probe in
the wire and wiggle it" — engineering's oldest forensic move. It took
the orchestrator 30 minutes to write + run + interpret; saved possibly
multi-week budget on a moot v3-vol salvage.

**Proposed addition to AGENT.md § Engineering patterns**:

> The caveman probe (manual perturbation of a suspected load-bearing
> input, observed at output) is a sanctioned diagnostic move for
> suspicious results. Specifically: when a verdict is unexpectedly null
> (no alpha, no signal, no change) AND infrastructure exists for fast
> re-run (cargo + cached data), hand-patch the most-load-bearing input
> with a coarse perturbation (e.g. ×2 or ×0.5), re-run, observe. If the
> output doesn't move, the wire is broken upstream of the verdict.
> Cost: ~30 minutes wall-clock; ROI: catastrophic-bug-class.

**Cost**: 1 paragraph in AGENT.md. Reuse rate: probably 2-4× per year
across the project's lifetime.

### 6.4 [CLAUDE.md] The shipped-partial precedent should not become a
habit

**Evidence.** v3-llm-forecaster v0.1.0 shipped 6 of 7 waves, with Wave D
deferred indefinitely for external-dependency reasons (no
ANTHROPIC_API_KEY this session). This is sanctioned by the shipped-partial
convention (codified 2026-05-16; first applied 2026-05-22). The convention
is correct as a sanctioned state, but it carries a hidden cost: every
shipped-partial feature is one whose load-bearing verdict has been
deferred, and the deferred-verdict-rate is now 2 of 54 features (~4%).

**Proposed addition to CLAUDE.md § Non-negotiables**:

> The `shipped-partial` state is sanctioned for external-dependency-
> deferrals only (API keys, upstream library bugs, hardware constraints).
> It is NOT sanctioned for scope-shrinking ("we'll come back to that").
> If the deferred wave carries the feature's load-bearing verdict, the
> feature's status MUST flag this explicitly in the frontmatter
> (`verdict_deferred: <wave>`) so future routing knows the feature
> hasn't actually produced its decision-grade evidence yet.

**Cost**: small frontmatter field + spec-update skill update. ~1 hour
architect.

### 6.5 [AGENT.md] Operator-routing-at-presenter is the strategic decision
layer; orchestrator should NOT pre-commit budget on multi-week ships

**Evidence.** The v25-dl-forecast-overlay umbrella + 4 children burned
multi-week budget across 4 ships before the operator's routing-at-
presenter decision retired the paradigm. Each ship would have been
predictable as "low EV per dollar" from the cheaper sibling test, but
each ran multi-week training compute anyway because the orchestrator
treated "next paradigm in the umbrella" as the auto-routed default.

The pattern: when the orchestrator commits multi-week compute to a
research line, the operator-routing-at-presenter is the *only* effective
budget gate. Without explicit operator-decide before each ship, the
budget compounds.

**Proposed addition to AGENT.md § Orchestrator responsibilities**:

> Multi-week ships (any ship whose total wall-clock estimate exceeds 1
> week of compute or 2 weeks of agent time) MUST surface an explicit
> operator-decide at the analyst-pass-final or architect-pass-final
> stage, BEFORE the developer wave begins. The operator is the budget
> gate; the orchestrator's auto-routing should default to "halt + ask"
> at this scale, not "auto-continue."

**Cost**: 1 paragraph in AGENT.md; matches existing operator-decide
patterns at presenter time; just moves the gate earlier in the workflow.

### 6.6 [Process] Strategic-reset half-days should be quarterly, not
crisis-driven

**Evidence.** This R3 reset is the first strategic-reset half-day in 8
weeks. The reset was triggered by an exceptional session (3 retirements
+ P0 fix + shipped-partial) — a crisis cadence. The fact that the
strategic question (Section 1) had been unaddressed for 8 weeks despite
54 feature ships suggests the project's *tactical* throughput is high
enough that *strategic* questions get crowded out by default.

**Proposed addition to AGENT.md § Orchestrator cadences**:

> Quarterly (or per ~50 feature ships, whichever comes first), the
> orchestrator should propose a half-day strategic-reset analyst pass.
> The pass answers one load-bearing strategic question (not tactical),
> reads the relevant evidence base, and emits a re-prioritized roadmap
> independent of the prior session's momentum. This is a structural
> counterweight to the tactical-sprint cadence.

**Cost**: structural; ~0 marginal cost; high reuse rate (already known
to be useful from this very pass).

### 6.7 What worked + what didn't this week

**Worked**:
- Retirement protocol (3 clean retirements in one session, all with
  code-preserved + anchors-locked + dev-note explaining why).
- Caveman probe (30 min to discovery).
- Shipped-partial as a sanctioned state (v3-llm-forecaster ships
  without abandoning context).
- Sub-agent parallelism (analyst + architect ran in parallel on the
  feature-state-table reviews; produced complementary cross-sections).
- ADR-0038 § D6.b protocol survived first invocation.

**Didn't work**:
- 5 layers of gates missed a complete no-op overlay for the entire ship
  cycle of v3-vol-forecaster + v3-vol-rebaseline. The fix (R2 forensic
  gate) is in tree; the meta-lesson (anchored byte-identity across
  structurally-different runs is suspicious) needs codifying (6.2).
- The forecaster-search bias consumed 10-15 weeks of budget on signal
  sources structurally independent of the moat. The routing-prior was
  alpha-hunting-biased; should have been moat-validation-biased.
- The 73% max-DD on v1 momentum has been the load-bearing baseline
  number across 4 retirements and was never anchored once as a canonical
  comparison reference. The "v1 baseline" was computed differently in
  each ship; only the noop-fix's apples-to-apples real-vs-real
  comparison is high-confidence (per prior analyst review section C).

---

## Verification artifacts

- `bash scripts/verify_anchors.sh` → **ANCHORS PASS (34/34)** at HEAD.
- `uv run scripts/spec_lint.py` → `spec-lint: FAIL (63 violations in 1
  categories)` — baseline at pass start; same at pass end (this dev-note
  introduces no new dead links; all references point to existing
  committed files).

## Handoff envelope

```toml
[handoff]
from        = "analyst"
to          = "orchestrator"
feature     = "strategic-reset-2026-05-23"
trace_refs  = []  # dev-note only; no new [[req]] rows
verdict     = "READY-FOR-OPERATOR-STRATEGIC-DECIDE"
priority    = "high"
notes       = """
R3 strategic-reset half-day analyst pass. Answers: has the moat thesis been
validated? Honest answer (D): half-validated. Component (4) — auditable
double-entry — VALIDATED by the noop-fix incident (anchored byte-identity
caught a bug 5 gates missed). Component (2) — persistent reflection
memory — UNTESTED; the only feature that would have tested it
(v3-llm-forecaster Wave D) is deferred indefinitely. The (2)+(4) compound
moat is not testable until (2) is tested. Recommended sequence for next
6-8 weeks: M1 (Wave D) FIRST — branch on the verdict. M2 (risk envelope)
+ M3 (canonical baseline) in parallel. M4 (correlation query) + M5
(correlation UI) + M6 (7-day acceptance run) gated on M1's positive
verdict. Section 5 surfaces 4 strategic questions for the operator;
Section 6 proposes 6 workflow meta-lessons (4 for CLAUDE.md / AGENT.md
amendments, 2 for process).
"""

[inputs]
spec_files = [
  "docs/dev-notes/feature-state-table-2026-05-22.md",
  "docs/dev-notes/feature-state-analyst-review-2026-05-22.md",
  "docs/dev-notes/feature-state-architect-review-2026-05-22.md",
  "docs/dev-notes/v25-dl-journey-retrospective-2026-05-22.md",
  "docs/dev-notes/v3-vol-retirement-and-c5-promotion-2026-05-22.md",
  "docs/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md",
  "docs/dev-notes/strategy-reformulation-survey-2026-05-22.md",
  "docs/dev-notes/shipped-partial-convention-2026-05-16.md",
  "spec/product.md",
  "spec/v3-llm-forecaster/feature.md",
]

[outputs]
spec_files = [
  "docs/dev-notes/strategic-reset-2026-05-23.md",
]
trace_rows_opened = []   # dev-note only
trace_rows_updated = []  # dev-note only
feature_folders_created = []

[open_questions]
items = [
  "Q-OPERATOR-1: 12-month success criterion — personal tool / research artifact / research platform?",
  "Q-OPERATOR-2: 'operator-visible moat' = UI surface / writeup / venue migration?",
  "Q-OPERATOR-3: how many retries on memory-as-signal-source after Wave D before accepting D.2 is infrastructure-only?",
  "Q-OPERATOR-4: is the routing criterion shifting from alpha-hunting-bias to moat-validation-bias for the next 6-8 weeks?",
]

[assumptions]
items = [
  "product.md § Differentiator lines 67-83 is the canonical moat statement; (2)+(4) is the 2026-04-17 confirmed long-term bet.",
  "The noop-fix incident is direct operational evidence that component (4) is load-bearing (audit-driven anchor byte-identity caught a 5-gate-blind-spot bug).",
  "No production trade has been causally linked to lesson-card retrieval; component (2) is shipped as infrastructure but untested as signal source.",
  "v3-llm-forecaster Wave D — the wave that produces the verdict on whether memory-driven signal unlocks alpha — is the load-bearing experiment for the next 6-8 weeks.",
  "Real-money execution is explicitly out of product scope (product.md § Non-goals); LiveMatchingEngine work belongs to a follow-up project.",
  "v1 momentum's 73% max-DD on 2023-FY real Binance data is the binding constraint for any continuous-paper acceptance run; risk envelope is a precondition, not a parallel deliverable.",
]
```

HANDOFF → orchestrator → operator-review (strategic-reset answer)
R3 strategic reset complete.
Dev-note: docs/dev-notes/strategic-reset-2026-05-23.md
Section 3 answer: D (the moat thesis is half-validated; component 4 is operationally validated by the noop-fix incident; component 2 is untested because Wave D is deferred)
Section 4 top-recommended move: M1 — promote v3-llm-forecaster Wave D as the load-bearing experiment that gates the next 6-8 weeks.
Section 5 questions for operator: 4
Section 6 meta-lessons: 6 (4 proposed CLAUDE.md / AGENT.md additions; 2 process/structural)
Anchor gate: ANCHORS PASS (34/34)
Spec-lint: FAIL (63 violations in 1 categories) — baseline preserved
