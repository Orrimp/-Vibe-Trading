---
slug: post-v2-scoping
status: draft
owner: analyst
updated: 2026-07-09
---

# Post-v2 scoping memo — is there a coherent, honest v3?

> **The one question this memo answers for the operator:** now that v2 has shipped
> complete (11 features, the research-driven roadmap), what comes next — is there a
> coherent, honest v3, or is "The Honest Advisor" feature-complete and the honest
> move ship-readiness/polish rather than new features?
>
> **This is READ-ONLY decision-support.** No code, no gate, no anchors, no status
> touched. It does not propose an architecture — that is the architect's job IF the
> operator greenlights a v3 theme below.

---

## TL;DR (read this, decide, then read the evidence)

- **VERDICT: there is NO coherent "add-more-features" v3, and that is the honest,
  expected outcome — the product's own thesis predicts it.** v2 consumed every
  ship-worthy research application; what remains is either explicitly-deferred
  scope-narrow work (PBO, the DSR→veto decision, the Calibrate stepper) or the
  off-track alpha-chasing the product exists to refuse. Manufacturing a v3 feature
  wave would contradict "measured honesty, not asserted alpha."

- **The one genuinely coherent forward theme is NOT new capability — it is
  *ship-readiness + the workflow-spine last-mile*: (1) activate the parked
  cross-platform CI (the "near-done milestone" it was waiting for is now), (2) build
  the consolidated end-to-end demo the product has never had, (3) finish the two thin
  spine seams v2 approved-but-deferred (the visible DATA→CALIBRATE→ANALYZE→SUGGEST
  stepper, and surfacing the D3 DSR-veto decision the P2-2 CI made empirically
  urgent).** Call this **v3 = "prove it's done," not "do more."**

- **Three deferred-from-v2 items are legitimate small v3 candidates** (PBO on the
  Tune surface; the DSR report-only→veto operator decision; the Calibrate-stage
  stepper). Everything else the operator might reach for — multi-coin, a return
  predictor in the ranking, automated search, LLM-as-trader, new signal primitives
  chasing alpha — is **off-track by an established guardrail** and enumerated in §4
  so you can see it was weighed and dismissed.

---

## 1. Verdict — is there a coherent v3? (with evidence)

**No coherent new-feature v3. The product is functionally complete; the honest next
phase is ship-readiness, not new surface area.** Three independent lines of evidence
converge on this:

### 1a. v2 consumed the ship-worthy research tranche — the well is dry by design

The 900-paper program (`research/SYNTHESIS.md`, `research/APPLICATIONS.md`) produced
exactly one convergent P0 (the overfitting scorecard), a P1 tranche (risk-shaping +
cost realism + honesty surfacing), and a large "do-NOT-build" negative space.
**v2 shipped the entire P0 + the entire actionable P1**, verified against code:

| Research application | v2 status (code-grounded) | Evidence |
|---|---|---|
| P0 overfitting scorecard (N_eff · DSR · MinBTL) | **SHIPPED, report-only** | `crates/backtest/src/bakeoff/scorecard.rs` — `n_eff()`/`dsr()`/`min_btl()` pure fns; `rank.rs` does NOT read it (gate untouched) |
| P0 forward-fidelity ("one strategy everywhere") | **SHIPPED as R1 coverage** — all 32 crownable arms incl. DVOL/macro | `runtime.rs:463–620` (`v0.dvol_regime`, `v0.macro_riskon`, all ensembles/DSL arms build) |
| P0 confidence-not-verdict framing | **SHIPPED** | `advisor-confidence-not-verdict` (commit `bcc4c24`) |
| P1 turnover + coherent tail (CVaR/median/skew) | **SHIPPED** | `advisor-turnover-and-tail-metrics` (`66286e2`) |
| P1 drawdown-control overlay (HWM restart, CPPI 20%) | **SHIPPED** | `advisor-drawdown-control-overlay`, ADR-0080 |
| P1 vol-overlay reposition + σ̂ estimator | **SHIPPED** | ADR-0078; `advisor-vol-estimator` (`c6ef1b3`) |
| P1 cost-model hardening (opt-in VolScaledSpread) | **SHIPPED** | ADR-0081; `advisor-cost-model-opt-in` |
| P1 DATA-quality/venue-trust surface | **SHIPPED** (the redone punt) | `advisor-data-quality-surface` (`67f2a9d`, `91fc2f4`) |
| P2 narration faithfulness hardening | **SHIPPED** | `advisor-narration-faithfulness` (`46acc9e`) |
| P2 no-alpha-gate null-falsification CI | **SHIPPED** | `crates/backtest/tests/null_data_no_crown.rs` (`a43bf3f`) |

What the research explicitly said to keep OUT — automated alpha search, a TSFM/LLM
in the ranking, on-chain/sentiment arms, deep-nets-as-alpha, Kelly-as-a-return-tool,
VWAP/impact scheduling, generative synthetic test data — is **not a "next tranche."
It is the negative space the thesis is built on** (`SYNTHESIS.md §3`, `APPLICATIONS.md`
"Dead ends / do-NOT-build"). There is no third pile of "held for later, will build."
The research pipeline is drained of ship-worthy work.

### 1b. The workflow spine is *functionally* complete end-to-end (the central lens)

v2-analysis's flagged gap was: "the cockpit stops at Return/Sharpe/MaxDD/Trades + a
headline… no explicit data→train→analyze→suggest spine ties the screens." **That gap
is now substantively closed** — every stage has its honest content:

- **DATA** — guided coin/budget/window input (F3) **+ the new DATA-quality/venue-trust
  panel** (P1-7) vouching for the inputs.
- **CALIBRATE** (the honest "training") — the gate-tied Tune sweep (`Screen::Tune`,
  ADR-0069) **+ the shared σ̂ vol-estimator** (P1-5) feeding the de-risk overlays. No
  return prediction — exactly as the research demands.
- **ANALYZE** — bake-off + frozen gate **+ the overfitting scorecard** (DSR/MinBTL/
  N_eff) **+ turnover + coherent-tail (CVaR/median/skew)**. The "traceable & plausible"
  credibility layer is now *visible*, not just computed.
- **SUGGEST** — forward plan (F6) running the **real** crowned strategy (R1 closed the
  14-arm coverage hole) **+ the confidence-not-verdict framing** (P0-3) **+ the
  drawdown/vol de-risk choice** on sizing.

**The residual is thin and cosmetic, not capability.** Two seams v2 *approved but
deferred* remain (both confirmed against code):
1. The **visible cross-stage stepper**. `Screen::Tune` is still a Lab drill-down
   (`state.rs:2298`, `:3494` — navigated preseeded from Lab), not a first-class named
   "Calibrate" stage with a breadcrumb tying the four screens. D7 (`v2-architecture.md`
   §6.0) approved *promoting* it but explicitly deferred the `agent::AdvisorStage`
   context-carrier "until the need is felt."
2. **PBO** — `Scorecard.pbo` is `Option<f64>` and **always `None`** (`scorecard.rs:50,
   87, 156`); deferred to the homogeneous Tune/sweep surface (R2), where CSCV is
   statistically honest.

Neither is a new *product capability*. Both are spine-completeness polish. That is a
v3-lite theme (§3), not a feature program.

### 1c. The product-completeness signals all read "done"

- **Anchors 119/119 PASS** (`scripts/verify_anchors.sh` re-run 2026-07-09) — the
  frozen evidence base is intact and stable.
- **The backlog forward queue is essentially empty of product work.** `spec/backlog.md`
  lists only: cross-platform CI (deferred to "near-done"), a `lab-recipe-test-harness
  v0.3` infra item, and LLM-desk items gated on the *parked* v2 LLM strategy (support
  layer, not alpha). No open product feature.
- **The operator's own epics are closed.** The 7-item leaderboard epic
  (`advisor-param-tuning` "Closes the operator's 7-item leaderboard epic"), the 3
  fresh-channel probes (options/IV, macro, and the combination/short/signal-library
  arm-class expansions) — all shipped, all returned the honest null (`BenchmarkWins`).
- **The thesis has been stress-tested from every reachable angle** — long, combinations,
  shorts, breakout/volume/OBV signals, implied-vol regime, macro cross-asset — and held
  every time (CHANGELOG advisor section). There is no untested *in-scope* channel left
  that isn't a documented dead end.

**Conclusion:** a new-feature v3 would have to invent scope the research says is a dead
end or the guardrails forbid. The honest, thesis-consistent verdict is: **the product
is feature-complete; the next phase is proving it's done.**

---

## 2. (Skipped — there is no "yes, here's the ranked feature list.")

Per the memo contract, §2 is the "if yes" branch. The verdict is no. The nearest thing
to a ranked candidate list is the ship-readiness roadmap in §3, plus the three small
deferred-item candidates triaged in §5. I am deliberately NOT manufacturing a feature
program here — a null result is the valid, valued, thesis-consistent outcome.

---

## 3. The honest alternative roadmap — v3 = "prove it's done"

If the operator wants a bounded next phase (and there is real, honest value in one),
it is **ship-readiness + the spine last-mile**, ranked by leverage. None of these adds
alpha surface; each hardens or completes what exists.

### R3-1 (Recommended, do first) — Activate the parked cross-platform CI

- **What:** `git mv .github/workflows/ci.yml.deferred → ci.yml` (starts the 3-OS
  GitHub Actions matrix). The source is already cross-platform + macOS-verified; only
  the matrix activation was parked.
- **Why now:** it was explicitly deferred to the **"near project completion" milestone**
  (`spec/backlog.md` "Deferred by decision"; project memory
  `project_cockpit_crossplatform_ci_held`). **That milestone is now** — v2 is the last
  planned feature phase, the product is feature-complete (§1). This is the single
  clearest "we are done building, lock in the quality gate" signal.
- **Guardrail:** none violated — it is CI infra, not product. The one caveat is the
  operator explicitly gated this on the milestone judgment, so it is an
  **operator-confirm**, not an autonomous move.
- **Complexity:** trivial (one `git mv` + a first green run to shake out any
  Linux/Windows-specific test flake). This is durable: it prevents cross-platform
  rot the moment the operator stops touching macOS.

### R3-2 (Recommended) — A consolidated end-to-end demo / walkthrough the product has never had

- **What:** a single runnable artifact (a scripted cockpit walkthrough + a committed
  narrative, e.g. `docs/runbooks/advisor-end-to-end-demo.md` with a golden
  `(coin, budget, window)` → leaderboard → scorecard → plan → forward paper-run) that
  exercises the *whole* DATA→CALIBRATE→ANALYZE→SUGGEST spine in one honest pass and
  shows the modal `BenchmarkWins` outcome as the product working.
- **Why:** the product has 11 v2 features + the whole v1 engine but no single
  end-to-end story that proves the spine hangs together for a first-time viewer. This
  is the honest "ship-readiness" deliverable — it is also the best possible artifact
  for the operator's own confidence that it's done.
- **Guardrail:** honest framing is load-bearing — the demo must show the null
  (`BenchmarkWins`) as the expected result, not stage a manufactured "active wins."
- **Complexity:** small-medium (mostly wiring existing surfaces + a runbook; possibly
  a render-verified screenshot walk). No new engine code.

### R3-3 — Complete the two thin spine seams v2 approved-but-deferred

- **What:** (a) the **visible named-stage stepper** promoting Tune from Lab drill-down
  to a first-class "Calibrate" stage with the four-verb breadcrumb (D7's approved-but-
  deferred glue); (b) **surface the D3 DSR decision** — the P2-2 no-alpha-gate CI
  *empirically proved* the frozen gate crowns noise on ~1/5 pure-noise seeds and the
  DSR scorecard caught every one (`phase-2d/feature.md`; `null_data_no_crown.rs`). That
  makes the "report-only vs a crown-eligibility veto" question (D3) **no longer
  hypothetical** — it is an operator values-call the data now motivates.
- **Why:** these are the only *product-visible* incompletenesses in the spine (§1b).
  (a) is IA polish; (b) is the one place new empirical evidence has arrived since v2
  scoping and it deserves an explicit operator decision.
- **Guardrail:** (b) is the sharp one — a DSR/PBO **crown-eligibility veto is a change
  to the FROZEN gate's effective behaviour** and needs its own ADR + explicit operator
  sign-off (it is NOT additive-by-default; `v2-architecture.md` §6 D3, CX-3). Ship the
  *decision surfacing* first; only wire a veto if the operator chooses it, durably,
  with the ADR.
- **Complexity:** (a) small (UI stepper over existing screens, render-verified);
  (b) is a **memo/decision**, not a build, unless the operator elects the veto.

### R3-4 — Documentation + ship-readiness hardening (lowest, ongoing)

- **What:** a final pass reconciling README/CHANGELOG status to "v2 complete,
  feature-complete"; an authoritative **"do-not-build" register** (the P2-7 the research
  named but v2 didn't formally land) so the dead-ends don't get re-litigated next
  session; confirm the `lab-recipe-test-harness v0.3` infra item is still wanted or
  decay it.
- **Why:** cheap honesty hygiene; the do-not-build register is genuinely load-bearing
  (project memory shows ML/forecasting keeps getting re-proposed as a "gap").
- **Complexity:** trivial-small; pure docs.

**Recommended sequence if greenlit:** R3-1 (CI, operator-confirm) → R3-2 (demo) →
R3-3a (stepper) + R3-3b (surface D3) → R3-4 (docs). This is a **short, bounded
"close-out" phase**, not an open-ended program. **If-budget-tightens:** R3-1 alone
(activate CI) is the single highest-value, lowest-cost move and a complete honest
statement of "the build is done" on its own.

---

## 4. Off-track register — what I considered and reject (with the guardrail each violates)

Enumerated so the operator sees these were weighed and dismissed, not overlooked. Each
is a tempting "v3 could be…" that pulls the product off its thesis. (These re-confirm
`v2-analysis.md §3` OT-1..OT-10 — nothing has changed to reopen any of them.)

| Tempting v3 idea | Guardrail it violates | Why it stays dead |
|---|---|---|
| **Multi-coin / "rank many coins, pick the best" / basket** | Single-coin only; NO cross-sectional baskets | Surviving factor edges are cross-sectional (need a universe); diversification fails in crypto stress (BTC–ETH ρ>0.85). A separate product track, not a v3 arm. (OT-1) |
| **A return/direction predictor (TSFM/deep-net/LLM) in the ranking** | NO prediction in the ranking (vol-only/sizing-only/narration-only) | No gate-credible crypto return-alpha; best peer-reviewed result ≈ B&H; lower MSE ≠ more profit. The one bright line. (OT-4) |
| **Automated alpha / parameter search (GA/GP/symbolic/LLM-code-evolution)** | NO automated search — the product's own threat model | Industrialized data-snooping; in-sample winner *negatively* correlates with OOS. Our FIXED pre-registered slates are the standing defense. (OT-3) |
| **LLM-as-trader / multi-agent "debate" decision-maker** | Narration-only bright line | Every "LLM beats B&H" result is leakage/no-cost/single-window. LLMs stay on the narration + read-only-reflection rail. (OT-5) |
| **New signal primitives ATR/VWAP as arms "to find edge"** | NO alpha search; pre-registration only | Confirmed NOT built. Adding them *chasing alpha* is scope-creep toward search; the honest version (more pre-registered coverage) is expected-null and adds surface for ~zero credibility gain post-v2. Backlog one-liner at most, not a v3 theme. |
| **On-chain (MVRV/SOPR/netflows) + sentiment (F&G/social) arms** | In-scope data = only structural PIT/as-of gap | PIT-infeasible / endogenous / fail Granger; documented dead ends; on-chain hard-stop already fired 2026-06-08. Do not spend feed budget. (OT-6) |
| **Live trading / real orders / KYC** | Paper/sim only (standing constraint) | Removed 2026-06-12; do not re-propose. (OT-2) |
| **Kelly / μ-driven "smart sizer" for return; VWAP/impact scheduling; OB-imbalance/HFT; generative synthetic test data** | Various (return-tool sizing; €200-scale impact≈0; sub-horizon; tail-smoothing) | All quantitatively hopeless or out-of-scale at €200/daily; keep fixed-fraction+vol-only sizing, the model-free block bootstrap, and the simple fee+spread model. (OT-8/OT-9/OT-10/OT-7) |
| **A DSR/PBO crown-eligibility veto shipped silently as "additive"** | FROZEN gate stays byte-frozen; additive-only | A veto changes the gate's effective crowning behaviour — it is NOT additive-by-default. Allowed ONLY as an explicit operator decision + its own ADR (this is exactly R3-3b, surfaced honestly, not smuggled). |

---

## 5. Deferred-from-v2 triage (each: v3-candidate / backlog / drop)

The explicit v2 deferrals, enumerated from `v2-analysis.md`, `v2-architecture.md` §6.0,
and confirmed against code. Each gets a one-line recommendation.

| Deferred item | Source | Code status (2026-07-09) | Recommendation |
|---|---|---|---|
| **PBO via CSCV on the Tune/sweep surface** | v2-arch R2/D1; `scorecard.pbo` | `pbo` always `None` (`scorecard.rs:156`) | **v3-candidate (small)** — the R2 plumbing is trivial (return matrix is a `windows(2).ln()` derivation from already-captured `SweepReport.cells[].equity_curve`); honest ONLY on the homogeneous Tune grid. Bundle into R3-3 if the Calibrate stepper lands. |
| **DSR/PBO crown-eligibility veto (report-only → veto)** | D2/D3, CX-3, CX-4 | `crown_clears_dsr` informational-only (`scorecard.rs:102`); `rank.rs` ignores it | **v3-candidate as a DECISION (= R3-3b)** — the P2-2 CI made it empirically urgent. Surface the choice; wire a veto only on explicit operator sign-off + ADR. Do NOT default-build. |
| **Named "Calibrate" stage + `agent::AdvisorStage` context-carrier** | D7 | Tune is a Lab drill-down (`state.rs:2298`); no stepper/stage-carrier | **v3-candidate (= R3-3a, stepper) / backlog (the AdvisorStage carrier)** — build the visible stepper; defer the context-carrier again "until the need is felt" (still true). |
| **New-primitive signals ATR / OBV / VWAP** (v0.2 follow-on) | signal-library-expansion follow-on; product.md changelog | OBV shipped (v1); ATR/VWAP NOT built | **DROP / backlog one-liner** — expected-null, adds arm surface for ~zero post-v2 credibility gain; only ever as pre-registered coverage, never alpha-chasing. Not a v3 theme. |
| **P2-4 cost-aware "trade-less" execution filter** | v2-analysis P2-4 / CX-6 | NOT built (grep empty) | **Backlog** — plausible cost-drag win but expected-null-on-return, and needs the `expected_move`-per-rule analyst sign-off (CX-6) before it's honest. Low priority; not close-out work. |
| **P2-5 funding-sign froth arm `v0.funding_froth`** | v2-analysis P2-5 | NOT built (grep empty) | **Backlog / drop** — expected FRAGILE (bidirectional Granger = endogeneity); honest coverage only. The exogenous-arm channel is already well-covered (DVOL, macro). Low value. |
| **P2-6 active-plus-hold blend arm** | v2-analysis P2-6 | NOT built (grep empty) | **Backlog (the most defensible P2 leftover)** — the one robust finding in the honest studies was a ~50% drawdown cut from blending; it is risk-shaping (expected ≈ B&H terminal), aligns with the drawdown-overlay theme, ships with a day-1 e2e. Promote to v3-candidate ONLY if the operator wants one more risk-shaping arm; otherwise backlog. |
| **Tail-stressed / EVT "worse-than-seen-crash" slice** | CX-9 | NOT built (research-only by design) | **Drop (keep as honest stated-limit)** — the better posture is "we state the limit honestly and don't pretend to cover it" (a hand-tuned crash fabricates unrealistic tails). Never wire into the frozen gate. |
| **ORATIO-derived DSR threshold** (vs hard 0.95) | CX-4 / D2 | Hard 0.95 informational flag | **Backlog, coupled to the veto decision (R3-3b)** — only relevant IF a veto is chosen; then derive the bar from the operator's cost-asymmetry statement. Moot while report-only. |
| **Cost-model default bump (vs opt-in-forever)** | CX-7 / D6 | Opt-in `VolScaledSpread`; default `LinearBps` (anchors 119/119) | **DROP for the foreseeable** — a default bump = ADR-0038 §D6 re-emission across 119 anchors for ≈0 honesty gain at €200 scale. Revisit only if a coin is found where flat-bps mis-costs a *crownable* arm. |
| **Block-length *logging*** (the "only gap" in the P0 bootstrap primitives) | SYNTHESIS P0 #10 | Politis–White computed per-series; logging was the sole gap | **Drop / fold into R3-4 docs** — trivial observability nicety, not a feature. |

**Net:** of the ~11 deferrals, **2 are genuine small v3-candidates** (PBO on the sweep
surface; the DSR-veto *decision*), **1 is a UI stepper** (R3-3a), **1 risk-shaping arm
is a defensible option** (P2-6 blend), and **the rest are backlog-or-drop** — expected-
null coverage, moot-while-report-only, or honest-stated-limits. None reopens the thesis.

---

## Handoff (informational — no agent spawned)

This memo is decision-support for an operator **phase decision**. Per the contract it
does NOT spawn the architect or propose an architecture. IF the operator greenlights a
v3 theme:

- **v3 = "prove it's done"** (R3-1 CI activation → R3-2 end-to-end demo → R3-3
  stepper + surface-the-DSR-decision → R3-4 docs) is the coherent, honest, bounded
  next phase. The analyst would then author the `[[req]]` rows and hand to the
  architect per the normal spine.
- **If instead the operator declares feature-complete + ships:** that is equally valid
  and thesis-consistent; R3-1 (CI) + R3-4 (docs) alone are the minimal honest close-out.

Files cited (all absolute in the repo): `CHANGELOG.md`; `spec/v2/v2-analysis.md`;
`spec/v2/v2-architecture.md`; `spec/backlog.md`; `research/APPLICATIONS.md`;
`research/SYNTHESIS.md`; `spec/product.md`; `crates/backtest/src/bakeoff/scorecard.rs`;
`crates/agent/src/runtime.rs`; `crates/ui/src/state.rs`; `spec/v2/phase-2d/feature.md`;
`.github/workflows/ci.yml.deferred`.
