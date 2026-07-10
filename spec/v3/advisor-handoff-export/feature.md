---
slug: advisor-handoff-export
status: proposed
owner: analyst
updated: 2026-07-10
version: 3.4.0
trace: REQ-V3-P5-HANDOFF-EXPORT-001
---

# advisor-handoff-export — the SUGGEST → manual hand-off export (P5)

> **One-line:** at the end of the advisor journey (the SUGGEST stage), let the
> operator export a **deterministic, offline, plain-text checklist** that says —
> honestly and descriptively — *"following this plan manually would mean X."* It
> restates the crowned plan's rules, sizing, the credibility verdict, the data-trust
> context, and the disclaimers, as a portable artifact you can read away from the
> cockpit. **NO order placement. NO venue API. NO new engine computation. NO LLM in
> the export path.** It serialises state the product ALREADY produced.

This is the single build of Remediation-plan **P5** (`spec/backlog.md` § Remediation
plan, ratified 2026-07-09: *"P5 IN, with the wording operator-ratified before build"*).
It is the closest surface in the entire product to the not-advice line, which is why
it goes through analyst-first: the export must be **useful** (it closes the P8-critique
gap — "no path from SUGGEST to the real world, where a real user falls off a cliff")
without ever becoming **advice** (it describes what the plan says; it never tells the
user what they should do).

> **The register B-2 constraint stays intact.** No real orders, no exchange keys, no
> venue API, no KYC, no withdrawals — see `spec/dev-notes/do-not-build-register.md`
> (live trading / order placement). The export is a *reading artifact*, not an
> execution bridge. It ends exactly where the product ends: a described plan the user
> may choose to act on entirely off-platform, on their own account, at their own risk.

## Why (the gap this closes)

The advisor takes the operator all the way to a crowned plan (the SUGGEST stage:
`Screen::ForwardPlan`, F6) — current stance, plain-language IF/THEN rules, budget-aware
sizing, the confidence check, the data-quality readout. But that plan lives **inside the
running cockpit**. There is no way to take it with you. The 2026-07-09 orchestrator
critique named this the sharpest product gap: *the journey stops at a screen*. A retail
user who has decided to act has to hand-transcribe rules off a live UI — the exact
moment the tool's honesty scaffolding (the weak-evidence band, the survivorship caveat,
the "a short can lose more than your €200" disclaimer) silently drops away, because none
of it travels with the plan. That is where "a real user falls off a cliff."

P5 closes it with the **cheapest change that makes the product usable by a human at
all**: a single "Export plan" action that writes the plan — with its full honesty
context attached — to a portable text file the user can keep, re-read, and follow
manually. The value is *portability of the honest plan*, not a new capability. The
export is downstream of every credibility gate; it can only ever restate them.

## Non-goals (explicit — do NOT build these)

- **No order placement, no venue API, no execution bridge.** The export is text. It
  never touches an exchange, never holds a key, never emits an order. Register B-2
  (live trading) stays a settled dead-end. If any task starts wiring a "send to
  broker" / "one-click execute" affordance, STOP — that is out of scope and out of the
  product.
- **No new engine computation.** The export reads the EXISTING `ForwardPlan` (the F6
  structured plan already on the mirror at the SUGGEST screen) + the EXISTING
  `BakeoffReportMirror` (recommendation outcome, per-candidate KPIs, robustness flags,
  the DSR scorecard, the data-quality view). It runs no backtest, no sizing math, no
  robustness pass, no ranking. It is a **pure projection** of state the product already
  computed and displayed. (Mirrors the ADR-0083 `stage_for` / ADR-0085 `crown_credibility`
  discipline: read existing values, derive a view, add no field.)
- **No LLM in the export path.** The export is DETERMINISTIC and offline. The only LLM
  prose that may appear is the **already-faithfulness-gated F9 narration** (ADR-0064) —
  and ONLY if it was already generated and passed its post-check for this run; it is
  included verbatim, **clearly marked as the AI-generated summary of the numbers above**,
  never re-generated at export time, never the source of a number, never load-bearing.
  If F9 narration is absent (the default / disabled / failed-post-check case), the export
  simply omits that block. The bright line: **no network call, no model call, in the act
  of exporting.**
- **No new numbers.** Every figure in the export is one the engine already produced and
  the cockpit already showed (verbatim-number discipline, mirroring the F9 narration
  faithfulness contract: the export may not invent, round differently, or extrapolate a
  single value). If a number isn't already on the mirror, it isn't in the export.
- **No prescriptive voice anywhere in the content.** The not-advice banner is not a fig
  leaf that licenses advice below it. The body itself stays descriptive: **"the plan
  says buy when…", never "you should buy when…"**; **"following this plan manually would
  mean deploying ~X units", never "deploy X units"**. This is the load-bearing content
  rule (see § The not-advice boundary analysis).
- **Not a new anchored report.** The export is a user-triggered artifact written to a
  user-chosen path (or a scratch dir), NOT a `spec/*/reports/*.md` anchored file. It adds
  no anchor and reads no anchored scenario → anchors 119/119 by construction.

## The not-advice boundary analysis (load-bearing — the whole point of P5)

**Where the line is.** Financial *advice* is a recommendation to a specific person to
take a specific action ("buy X", "deploy €200 now", "this will go up"). Financial
*information* is a truthful, descriptive account of what a tool computed ("the bake-off
crowned strategy X on this window"; "strategy X's rule is: buy when the 20-bar average
crosses above the 50-bar"; "past simulated results do not predict the future"). The
entire product already lives on the information side of that line — every recommendation
surface carries the not-advice disclaimer, and the copy is written in the descriptive
register ("`v0.sma` is the best risk-adjusted pick", not "buy `v0.sma`"). **The export
inherits that stance verbatim and must not weaken it**: it is a description of a computed
plan, addressed to no one, prescribing nothing.

**How the template stays on the right side — three structural guarantees.** (1) The
export is a **faithful serialisation of already-descriptive UI copy** — every line is
sourced from an existing `crate::strings` constant that already passed the product's
not-advice discipline (e.g. `FORWARD_PLAN_NOT_A_PREDICTION`, `FORWARD_PLAN_DISCLAIMER`,
the IF/THEN rule strings), so it cannot drift more prescriptive than the surface it
mirrors. (2) The framing verb is locked to **"following this plan manually would mean
X"** — a conditional, second-conditional description of a hypothetical the *user* chose
to enter, never an imperative directed at them; the "What this is NOT" section restates
this in the artifact itself. (3) The **honesty context travels with the plan, not as
fine print**: the credibility verdict (weak-evidence / passes / benchmark-wins), the
survivorship caveat, the "gate can crown noise ~1/5 on nulls" honesty note, and the
unbounded-loss short disclaimer are all *in the body*, co-located with the rules they
qualify — so the export can never present a rule without its caveat. The banner is a
reinforcement of a descriptive body, not a disclaimer bolted onto a prescriptive one.

## Verified ground truth (read the code before designing)

Grounded in the current tree (2026-07-10), not spec prose. The export is a projection of
these EXISTING facts — the P5 build adds a serialiser + a UI trigger, nothing more:

- **The plan already exists as a structured, `core`-typed value.** `agent::config::ForwardPlan`
  (built by `crates/agent/src/plan.rs::build_forward_plan_from_registry`, ADR-0062) carries
  `strategy`, `symbol`, `stance` (`PlanStance::{Flat,Long}`), `latest_signal`
  (`PlanSignal::{Buy,Sell,Hold}`), `rule` (`PlanRuleKind::{SmaCross,MacdCross,RsiReversion,
  BollingerReversion,BuyAndHold,Ensemble}`), `last_close`, `last_bar_ts`, `budget`,
  `projected_units`, `sizing_capped`, `horizon_days`, and `confidence` (the P0-3 scorecard
  summary). This is exactly the object the SUGGEST screen renders — the export serialises it.
- **The plan-to-plain-language mapping is already done.** `crates/ui/src/strings.rs` +
  `crates/ui/src/screens/forward_plan.rs` already turn every `PlanRuleKind` into IF/THEN
  plain-language copy (`FORWARD_PLAN_RULE_SMA_ENTRY_IF_FMT` / `_THEN`, the MACD/RSI/BBands/
  buy-and-hold/ensemble/short variants), the stance badge (`FORWARD_PLAN_STANCE_*`), the
  sizing line (`FORWARD_PLAN_SIZING_BUY_AND_HOLD_FMT`, `FORWARD_PLAN_SIZING_CAPPED_NOTE`),
  the budget/FX line (`FORWARD_PLAN_BUDGET_LINE_FMT` — the €200→USDT conversion + hard cap),
  the horizon (`FORWARD_PLAN_HORIZON_FMT`), and the disclaimers (`FORWARD_PLAN_NOT_A_PREDICTION`,
  `FORWARD_PLAN_DISCLAIMER`). **The export needs almost no new copy** — it re-uses these.
- **The credibility verdict is a pure derived value.** `crown_credibility(outcome,
  Option<&ScorecardView>) -> CrownCredibility{Passes|WeakEvidence|NotApplicable}` (P1,
  ADR-0085, in `crates/ui/src/screens/leaderboard.rs`) + the strings
  `LEADERBOARD_CROWN_WEAK_EVIDENCE` / `_PASSES_DSR` / `_WEAK_EVIDENCE_HINT`. The export
  MUST carry the SAME verdict for the SAME run (a weak-evidence crown exports the
  weak-evidence line; a `BenchmarkWins` outcome exports NO credibility badge — buy-and-hold
  is exempt per ADR-0066).
- **The recommendation outcome discriminates the modal case.** `OutcomeKind::{ActiveWins,
  BenchmarkWins,AllFragile}` (mirrored 1:1 from `backtest::RecommendationOutcome`), with the
  headlines `LEADERBOARD_HEADLINE_BENCHMARK_WINS` (`{coin}`-filled), `_ACTIVE_WINS`
  (`{winner}`-filled), `_ALL_FRAGILE`. **`BenchmarkWins` is the modal real-crypto outcome**
  (the 2026-06-08 ship-passive verdict; the P2 corpus re-run confirmed it holds on every
  recent-era window) — so the export's honest-verdict block leads with the hold case.
- **The data-trust context is a display-only view.** `DataQualityView` (P1-7, the DATA
  stage) → `LEADERBOARD_DATA_QUALITY_{VENUE,PROVENANCE,TRUST_*,SURVIVAL_NOTE,WARNING_*}`.
  The survivorship caveat (`LEADERBOARD_DATA_QUALITY_SURVIVAL_NOTE`) is ALWAYS present.
- **Shorts CAN be crowned; the unbounded-loss disclaimer already exists.** The 5-arm short
  slate (`sma_cross_ls`/`macd_ls`/`rsi_ls`/`bbands_ls`/`always_short`, ADR-0068) runs the
  SAME `rank_candidates` comparator (R-SS.10: `BenchmarkWins`/`AllFragile` reachability
  UNCHANGED) — nothing exempts a short arm from being `order[0]`. **Empirically it DID
  crown:** on a bear window (2022-Q2, BTC −56.2%) `always_short` crowned Robust (+56.2%);
  the 4 `_ls` arms came back Fragile (the honest null). The load-bearing disclaimer
  `SHORT_UNBOUNDED_LOSS_DISCLAIMER` ("A short can lose MORE than your €200 — an unbounded
  loss. A 2× price move wipes you out and then some.") + the plan-level
  `FORWARD_PLAN_RULE_SHORT_LIQUIDATION` already exist. See § The shorts question.
- **The F9 narration is opt-in, faithfulness-gated, and clearly labelled.** `LEADERBOARD_EXPLAIN_LLM_LABEL`
  ("Plain-language summary of the result above (AI-generated)"), only present in the `Ready`
  state after the deterministic post-check passed (ADR-0064). The export includes it ONLY if
  present, verbatim, under that label.
- **The golden fixture already exists.** `crates/ui/src/fixtures.rs::fake_bakeoff_report_mirror_five_arm`
  is `(BTCUSDT, ActiveWins, crown_clears_dsr=false)` — a crowned active pick that fails the
  overfitting check — with `forward_budget = Some(€200)`. This is the money-shot fixture the
  export's render/golden test drives (the `WeakEvidence` + `€200` case).

## Requirements (bounded — a deterministic offline text artifact)

- **R-HE.1 — Deterministic serialiser.** A pure function that takes the EXISTING
  `ForwardPlan` + `BakeoffReportMirror` (+ the `DataQualityView` + optional F9 narration
  already on the mirror) and produces the export text. Same inputs ⇒ byte-identical output
  (no timestamps-of-now, no RNG, no locale drift; any "generated at" line, if included, is
  the plan's `last_bar_ts` / run seed, not wall-clock — architect to confirm, see Q-HE-3).
- **R-HE.2 — Offline + no LLM in the path.** No network, no model call at export time. F9
  narration is included only if already generated + post-checked for this run, verbatim,
  under its AI-generated label.
- **R-HE.3 — Verbatim numbers + verbatim copy.** Every number is one already on the mirror;
  every descriptive line is sourced from an existing `crate::strings` constant (or a small
  set of new export-only constants for the export's own header/section titles — NOT for any
  claim about the plan). No figure is re-derived or re-rounded.
- **R-HE.4 — Modal-case-first honest verdict.** The verdict block leads with the
  `BenchmarkWins` case (the honest plan is buy-and-hold), then `ActiveWins` (with the P1
  credibility state embedded — weak-evidence variant verbatim), then `AllFragile`.
- **R-HE.5 — Credibility travels with the plan.** The export carries the SAME
  `crown_credibility` verdict as the SUGGEST screen for the same run. A weak-evidence crown
  → the weak-evidence line + mechanism, in-body. `BenchmarkWins`/`AllFragile` → NO
  credibility badge (the ADR-0085 `NotApplicable` rule; a DSR badge on a hold pick is
  actively misleading).
- **R-HE.6 — Data-trust context travels with the plan.** Venue + provenance + trust tier +
  the always-present survivorship caveat + any data-quality warnings, from `DataQualityView`.
- **R-HE.7 — Short-aware risk section.** IF the crowned arm is short-capable (`*_ls` /
  `always_short`, or any plan whose rules include a sell-to-open), the export MUST carry the
  `SHORT_UNBOUNDED_LOSS_DISCLAIMER` + the liquidation line — the "a short can lose more than
  your €200" caution is mandatory on any surface where a short is in play (R-SS.4 / ADR-0068).
- **R-HE.8 — The era-qualified thesis + the ~1/5 honesty note.** A one-liner carrying the
  post-`61887c8` era-qualified thesis (the modal outcome on recent-era windows is still
  just-hold; the machine positively detected — and dated the decay of — real older-era edges)
  and the P2-2 honest note that the gate can crown noise ~1 in 5 on true-null series (with
  DSR as the second-layer catch). Both descriptive, both cited.
- **R-HE.9 — "What this is NOT" section, in the artifact.** Not advice; not a prediction;
  past ≠ future; the gate can crown noise ~1/5 on nulls; paper/simulated budget; no real
  orders placed. Restated inside the export so a reader who only has the file still gets the
  full honesty frame.
- **R-HE.10 — Provenance footer.** The coin, budget, lookback window, run seed (if on the
  mirror), and a pointer to the in-cockpit report/Reports screen so the export is traceable
  back to the reproducible run — NOT a claim, just the breadcrumb.
- **R-HE.11 — Anchor + gate safety.** User-triggered artifact to a user/scratch path; NOT a
  `spec/*/reports/*.md` anchored file; reads no anchored scenario. `verify_anchors.sh`
  119/119 before AND after; `spec_lint.py` PASS(0).
- **R-HE.12 — CLAUDE.md baseline-equity-divergence e2e — N/A, recorded not skipped.** P5
  introduces no strategy overlay / sizing-modifier / decision variable; it computes no
  equity. The divergence gate is inapplicable by construction (mirrors the P1 / P3
  precedent — a UI/serialisation feature that changes no verdict). The architect records
  this explicitly in the ADR/tasks rather than silently omitting it.

## The shorts question (answered — load-bearing for R-HE.7)

**Can a short arm be crowned?** **Yes.** The 5-arm short slate runs the identical
`rank_candidates` comparator as every long arm (ADR-0068 R-SS.10 explicitly requires
`BenchmarkWins`/`AllFragile` reachability UNCHANGED — shorts are scored *by* the existing
gate, not exempted from being `order[0]`). It is not merely theoretical: the shipped
science (`spec/v1/advisor-short-selling/reports/test-2026-06-23.md` + the feature's T-D6)
shows that on a **bear window** (2022-Q2, BTC −56.2%) the `always_short` benchmark control
crowned **Robust** (+56.2%), while the four directional `_ls` arms came back **Fragile**
(the expected null). So a real user can absolutely reach a SUGGEST plan whose crowned arm
is a short.

**What the export must say then.** When the crowned plan is short-capable, the export MUST
carry, in-body and co-located with the rules:

1. The **standing short rules** in plain language — the existing plan copy:
   `FORWARD_PLAN_SHORT_RULES_HEADING` ("This strategy can also bet on a decline:"), the
   `FORWARD_PLAN_RULE_SHORT_OPEN_*` / `_COVER_*` IF/THEN lines, and for `always_short` the
   `FORWARD_PLAN_RULE_ALWAYS_SHORT` line ("Open a short now and hold it the whole horizon —
   the down-side mirror of buy-and-hold … it loses on any up-trend by construction").
2. The **liquidation reality** — `FORWARD_PLAN_RULE_SHORT_LIQUIDATION` ("If the loss reaches
   the maintenance-margin floor the short is force-liquidated — the loss is not capped at
   your €200").
3. The **mandatory unbounded-loss disclaimer** — `SHORT_UNBOUNDED_LOSS_DISCLAIMER` verbatim
   ("A short can lose MORE than your €200 — an unbounded loss. A 2× price move wipes you out
   and then some. Simulated paper budget, not financial advice."). This is R-SS.4's
   load-bearing requirement and it is non-negotiable on any surface where a short is in play
   — the export is such a surface.

The honest asymmetry the export preserves: a *long* plan's worst case is losing the €200; a
*short* plan's worst case is losing MORE than the €200. The export must never let a crowned
short read as symmetric with a long. (For the golden `(BTCUSDT, €200, 2024 H1)` case the
crown is a LONG active pick, so the short section is ABSENT — the export only emits it when a
short is actually crowned. The template below shows both the modal long/hold case and the
short-crowned variant.)

## Open questions for the architect (Q-HE-*)

- **Q-HE-1 — Format: markdown vs plain-text `.txt`?** Markdown renders nicely if the user
  opens it in a viewer and keeps the honesty structure (headings, the caveat callouts) legible;
  plain `.txt` is the most portable / paste-anywhere and can't be mistaken for a rendered
  "report". *Analyst lean: **markdown `.md`** (durable — it preserves the co-located-caveat
  structure the not-advice boundary depends on, and it's still readable as raw text; a `.txt`
  variant is a trivial follow-on if wanted).* Architect decides + records in the ADR.
- **Q-HE-2 — Where is it triggered in the UI?** The natural home is the SUGGEST screen
  (`Screen::ForwardPlan`) — an "Export plan" action co-located with the plan it exports (the
  terminal step of the DATA → CALIBRATE → ANALYZE → SUGGEST spine). Alternative: the
  Recommendation/leaderboard block. *Analyst lean: **the SUGGEST/ForwardPlan screen** (it is
  the journey's terminus and where the full plan state is assembled).* Confirm the button
  placement + label at the render walk (a new `crate::strings` const, plain-language, e.g.
  "Export this plan").
- **Q-HE-3 — One export per plan or per run?** The plan is a projection of one crowned
  bake-off run, so "per plan" and "per run" coincide today. Confirm the artifact is scoped to
  the single current crowned selection (NOT a multi-run digest). Also: does the export carry a
  "generated" stamp, and if so is it the run seed / `last_bar_ts` (deterministic) rather than
  wall-clock (non-deterministic)? *Analyst lean: **one artifact per crowned plan**, stamped
  with the deterministic run identity (seed + `last_bar_ts`), NOT wall-clock — so R-HE.1
  byte-determinism holds and two exports of the same run are identical.*
- **Q-HE-4 — Where does the serialiser live (crate layering)?** The plan mapping already lives
  at the `agent` seam (`plan.rs`) and the plain-language copy lives in `ui` (`strings.rs` +
  `screens/forward_plan.rs`). The export reads both plan structure AND ui copy → it most
  naturally lives in `ui` (it is a presentation artifact, and `ui` already owns every string
  it emits), with the serialiser a pure function over the `BakeoffReportMirror` + `ForwardPlan`
  already on the mirror. *Analyst lean: **`ui`** (respects the `ui`-owns-the-words layering;
  no new `strategy`/`backtest` dependency).* Architect confirms against the layering rule
  (`ui` never depends on `strategy`/`exec`/`models`/`llm`).
- **Q-HE-5 — How is it verified?** The export is text, so a **golden/string test** on the
  serialiser (assert the emitted text contains the crowned strategy, the sizing, the
  credibility verdict, the survivorship caveat, the disclaimers; a negative control: the
  `BenchmarkWins` fixture emits NO credibility badge; the short-crowned fixture emits the
  unbounded-loss disclaimer) is the right floor — NOT a pixel render (there is no new
  rendered surface beyond the trigger button; the button itself gets the standard
  render-walk if it lands on a screen). *Analyst lean: **golden-text tests on the serialiser
  + a light render check on the export button** if it's a new visible control.* Architect
  sets the exact gate.
- **Q-HE-6 — What happens on export when there is NO crowned plan yet?** The SUGGEST screen
  already guards this (`FORWARD_PLAN_EMPTY_PROMPT` — "No plan yet…"). Confirm the export
  action is simply disabled/absent until a plan exists (no empty-export artifact). *Analyst
  lean: **disable the action until a plan is crowned** — an empty export has no honest
  content to carry.*

## The ONE decision the operator must make (the wording ratification)

**DECISION-P5-WORDING — ratify the export's draft wording (below) before build.** This is
the operator gate the remediation plan named ("wording operator-ratified before build"). The
complete draft template — header + not-advice/paper-sim banner, the honest-verdict block
(BenchmarkWins-first), the ActiveWins case with the P1 weak-evidence variant verbatim, the
IF/THEN rule restatement, sizing at the €200 budget incl. the €200→USDT note, the
era-qualified thesis + survivorship/data-trust notes, the short/risk section, "What this is
NOT", and the provenance footer — is in § Draft wording below, instantiated on the golden
`(BTCUSDT, €200, 2024 H1)` case. The operator ratifies (or redlines) the wording; the
architect then locks it into the serialiser. **No build starts until the wording is
ratified.** (This is distinct from Q-HE-1..6, which are architect-decides.)

---

## Draft wording (the artifact the operator ratifies)

> **How to read this section.** Below is the EXACT section skeleton of the exported file,
> instantiated with real placeholder examples for the golden case. Lines shown as
> `«SOURCE: CONST_NAME»` are sourced VERBATIM from an existing `crate::strings` constant (the
> not-advice discipline already applied to it) — the operator is ratifying that these existing
> lines are the right ones to carry into the export, plus the small set of **new export-only
> header/section strings** (marked `«NEW»`). Every `{placeholder}` is filled from a value
> already on the mirror. Numbers shown (e.g. `0.004 BTC`, `$216.00`, `1.08`) are illustrative
> of the golden run; the serialiser fills the run's actual verbatim values.
>
> Two variants are shown: **(A) the modal case** — the golden `(BTCUSDT, €200, 2024 H1)` run,
> where the honest answer is buy-and-hold OR a weak-evidence active crown; and **(B) the
> short-crowned variant** — shown to demonstrate the mandatory short/unbounded-loss section.

### Variant A — the golden `(BTCUSDT, €200, 2024 H1)` case

```text
════════════════════════════════════════════════════════════════════════════
  YOUR PLAN — a manual hand-off checklist                                «NEW»
  Coin: BTCUSDT   ·   Budget: €200 (simulated)   ·   Window: 2024-01-01 → 2024-06-30
════════════════════════════════════════════════════════════════════════════

  ⚠ NOT FINANCIAL ADVICE — PAPER SIMULATION                              «NEW header»
  «SOURCE: FORWARD_PLAN_DISCLAIMER»
  Not financial advice. The €200 is a simulated paper budget on
  historical/live data — no real orders are placed. Past behaviour does not
  guarantee future results.

  This file describes what FOLLOWING THIS PLAN MANUALLY would mean. It places  «NEW»
  no orders and connects to no exchange. If you choose to act on it, you do so
  entirely on your own account and at your own risk.

────────────────────────────────────────────────────────────────────────────
  THE MEASURED ANSWER FOR THIS WINDOW                                    «NEW section»
────────────────────────────────────────────────────────────────────────────

  ── If the outcome was BenchmarkWins (the modal real-crypto result) ──
  «SOURCE: LEADERBOARD_HEADLINE_BENCHMARK_WINS  ({coin}=BTCUSDT)»
  No active strategy cleared the robustness bar on BTCUSDT — simply holding
  (buy-and-hold) is the least-bad choice on this window.

  «NEW, descriptive framing line»
  The honest plan for this window is therefore BUY-AND-HOLD. Following it
  manually would mean: buy once now and hold for the horizon — there is no
  timing rule to follow. Here is what that means for your €200. ↓

  ── If the outcome was ActiveWins (an active pick crowned) ──
  «SOURCE: LEADERBOARD_HEADLINE_ACTIVE_WINS  ({winner}=SMA crossover (long/short)…)»
  SMA crossover is the best risk-adjusted pick.

  «— and, when that crown FAILS the overfitting check (the golden five_arm case),
     the credibility verdict is carried IN-BODY, verbatim: —»
  «SOURCE: LEADERBOARD_CROWN_WEAK_EVIDENCE»
  ⚠ This pick did not survive the overfitting check — treat it as weak
  evidence. With this many strategies tried, an edge this size can appear by
  chance.
  «SOURCE: LEADERBOARD_CROWN_WEAK_EVIDENCE_HINT»
  See ‘How much to trust this’ below for the deflated-confidence figure.

  «— when the crown PASSES the check, this line replaces the two above: —»
  «SOURCE: LEADERBOARD_CROWN_PASSES_DSR»
  ✓ Passed the overfitting check (deflated-Sharpe above the bar).

  «— BenchmarkWins / AllFragile carry NO credibility line here (ADR-0085
     NotApplicable — a DSR badge on a hold pick would mislead). —»

────────────────────────────────────────────────────────────────────────────
  RIGHT NOW (as of the last bar)                                         «NEW section»
────────────────────────────────────────────────────────────────────────────

  «SOURCE: FORWARD_PLAN_STANCE_LONG | _FLAT»
  Long — holding
  «SOURCE: FORWARD_PLAN_AS_OF_FMT  ({close}=$61,240.00, {as_of}=2024-06-30 23:00 UTC)»
  As of the last close $61,240.00 (2024-06-30 23:00 UTC).
  «SOURCE: FORWARD_PLAN_LATEST_SIGNAL_FMT + FORWARD_PLAN_SIGNAL_BUY»
  Latest signal on that bar: buy.

────────────────────────────────────────────────────────────────────────────
  THE STANDING RULES — what the plan says (not what you should do)       «NEW section»
────────────────────────────────────────────────────────────────────────────

  «SOURCE: FORWARD_PLAN_NOT_A_PREDICTION»
  This is a conditional, rule-based plan — not a price prediction, and not an
  implied or expected return. It only describes what the strategy will do when
  its conditions are met.

  «SOURCE: FORWARD_PLAN_RULE_IF / _THEN + FORWARD_PLAN_RULE_SMA_ENTRY_IF_FMT / _THEN»
  IF   the 20-bar average crosses above the 50-bar average
  THEN buy (open a position)

  «SOURCE: FORWARD_PLAN_RULE_SMA_EXIT_IF_FMT / _THEN»
  IF   the 20-bar average crosses back below the 50-bar average
  THEN sell (close the position)

  «— for a buy-and-hold plan the rules block is instead: —»
  «SOURCE: FORWARD_PLAN_RULE_BUY_AND_HOLD»
  Buy once now and hold the whole horizon. There is no sell trigger and no
  timing rule.

  «SOURCE: FORWARD_PLAN_CADENCE_FMT  ({horizon}=7)»
  These rules stay in force and are re-checked on every new bar for the next 7
  days — the plan is the rules, not a fixed schedule of trades.

────────────────────────────────────────────────────────────────────────────
  SIZING AT YOUR €200 BUDGET                                             «NEW section»
────────────────────────────────────────────────────────────────────────────

  «SOURCE: FORWARD_PLAN_BUDGET_LINE_FMT
    ({eur}=200, {usdt}=216.00, {rate}=1.08, {source}=config)»
  €200 ≈ $216.00 (at 1.08 EUR/USD, config). It never deploys more than €200 —
  a hard cap.

  «SOURCE: FORWARD_PLAN_SIZING_BUY_AND_HOLD_FMT  ({units}=0.0035, {close}=$61,240.00)»
  Deploy the full €200 now — about 0.0035 units at the last close $61,240.00 —
  and hold for the horizon.
  «— OR, for a rule-based entry, the projected size on the next BUY:
     "about {projected_units} units at the last close {close}", from ForwardPlan.projected_units —»
  «SOURCE: FORWARD_PLAN_SIZING_CAPPED_NOTE  (only when ForwardPlan.sizing_capped)»
  The €200 cap limited this size.

────────────────────────────────────────────────────────────────────────────
  HOW MUCH TO TRUST THIS                                                 «NEW section»
────────────────────────────────────────────────────────────────────────────

  «SOURCE: the P0-3 confidence block values already on ForwardPlan.confidence —»
  «SOURCE: FORWARD_PLAN_CONFIDENCE_CANDIDATES_LABEL»  Strategies tried: 20
  «SOURCE: FORWARD_PLAN_CONFIDENCE_DSR_LABEL»         Deflated confidence: 0.62
  «SOURCE: FORWARD_PLAN_CONFIDENCE_DSR_GLOSS»
    Probability the pick’s edge is real after correcting for the number of
    tries. Above 95% is the honest bar.
  «SOURCE: FORWARD_PLAN_CONFIDENCE_BEATS_HOLD_LABEL + _NO | _YES»
    Beats holding? ⚠ Not yet — edge uncertain after the search
  «SOURCE: FORWARD_PLAN_CONFIDENCE_MIN_BTL_LABEL + _FMT  ({years}=11)»
    Minimum history needed: 11 yr
  «SOURCE: FORWARD_PLAN_CONFIDENCE_NOTE»
    The confidence block is informational — it does not change the pick or the
    rules.

  «NEW — the honest ~1/5 note (P2-2), descriptive + cited»
  A note on the search: this ranking gate, run on pure-noise price series,
  still crowns an "active winner" by chance in roughly 1 run out of 5 (the
  deflated-confidence figure above is the second-layer check that catches those
  chance winners). A crowned pick is the best of what was tried on this window —
  not proof of a real, repeatable edge.

────────────────────────────────────────────────────────────────────────────
  WHERE THIS DATA CAME FROM                                              «NEW section»
────────────────────────────────────────────────────────────────────────────

  «SOURCE: LEADERBOARD_DATA_QUALITY_VENUE_LABEL»       Venue: Binance
  «SOURCE: LEADERBOARD_DATA_QUALITY_PROVENANCE_BINANCE»
    Provenance: Hourly close from Binance klines, cached in the pinned backtest
    corpus.
  «SOURCE: LEADERBOARD_DATA_QUALITY_TRUST_HIGH | _CONDITIONAL | _LOW»
    Trust level: High — reconcilable major-venue price
  «SOURCE: LEADERBOARD_DATA_QUALITY_SURVIVAL_NOTE  (ALWAYS present)»
    Survival bias: Coins that failed to reach today are absent from this
    universe — results overstate the expected outcome for a random new coin.
  «SOURCE: LEADERBOARD_DATA_QUALITY_WARNING_*  (zero or more, only if present)»
    [Warnings, when any: thin liquidity / wash-trading suspicion / pump-and-dump]

  «NEW — the era-qualified thesis one-liner (post-61887c8), descriptive + scoped»
  On this and every recent-era window the advisor can run, the modal honest
  outcome is still just-hold: no active strategy robustly beat buy-and-hold net
  of cost. The same machine did positively detect — and date the decay of —
  real active edges in older, less-efficient markets (2017–2020); those edges
  had decayed to ~zero by 2023. This window ends at "now", where hold stands.

────────────────────────────────────────────────────────────────────────────
  WHAT THIS IS NOT                                                       «NEW section»
────────────────────────────────────────────────────────────────────────────

  • NOT financial advice. This describes a computed plan; it recommends nothing
    to you personally.
  • NOT a prediction. The rules say what the strategy does when conditions are
    met — they do not forecast the price.
  • Past ≠ future. Every number here is measured on historical/simulated data.
    Past simulated results do not predict future outcomes.
  • The ranking gate can be fooled by chance (~1 in 5 on pure-noise series, see
    above). "Best of what we tried" is not "a real edge".
  • Paper only. The €200 is a simulated budget. No real orders are placed and no
    exchange is connected — following this plan for real is entirely your own
    decision, on your own account.

────────────────────────────────────────────────────────────────────────────
  PROVENANCE (so you can reproduce this)                                 «NEW section»
────────────────────────────────────────────────────────────────────────────

  Coin: BTCUSDT   ·   Budget: €200   ·   Window: 2024-01-01 → 2024-06-30
  Crowned pick: SMA crossover (v0.sma)   ·   Horizon: 7 days
  Run seed: {seed}   ·   Last bar: 2024-06-30 23:00 UTC
  Reproduce this run in the cockpit: Reports screen → this bake-off.
  «— optional, ONLY if already generated + post-checked for this run: —»
  «SOURCE: LEADERBOARD_EXPLAIN_LLM_LABEL»
  Plain-language summary of the result above (AI-generated):
    [the verbatim F9 narration text, if and only if it was already produced and
     passed its faithfulness post-check — never generated at export time]

════════════════════════════════════════════════════════════════════════════
  End of plan. Not advice. Paper simulation. Your decision, your account.  «NEW footer»
════════════════════════════════════════════════════════════════════════════
```

### Variant B — the short-crowned variant (mandatory extra section)

When the crowned arm is short-capable (`*_ls` / `always_short`), the STANDING RULES section
gains the down-half rules AND a mandatory risk section is inserted before "What this is NOT":

```text
────────────────────────────────────────────────────────────────────────────
  THE STANDING RULES — what the plan says (not what you should do)
────────────────────────────────────────────────────────────────────────────
  … (the long rules, as above) …

  «SOURCE: FORWARD_PLAN_SHORT_RULES_HEADING»
  This strategy can also bet on a decline:
  «SOURCE: FORWARD_PLAN_RULE_SHORT_OPEN_IF_GENERIC (or the family's exit copy) + _SHORT_OPEN_THEN»
  IF   the trend turns bearish (the entry condition reverses to the downside)
  THEN sell-to-open a short (bet on a decline)
  «SOURCE: FORWARD_PLAN_RULE_SHORT_COVER_IF + _SHORT_COVER_THEN»
  IF   the trend flips back up (the entry condition reverses)
  THEN buy-to-cover (close the short)
  «— for the always_short control instead: —»
  «SOURCE: FORWARD_PLAN_RULE_ALWAYS_SHORT»
  Open a short now and hold it the whole horizon — the down-side mirror of
  buy-and-hold. There is no cover trigger; it loses on any up-trend by
  construction.

────────────────────────────────────────────────────────────────────────────
  ⚠ RISK — A SHORT CAN LOSE MORE THAN YOUR €200                          «NEW section, short-only»
────────────────────────────────────────────────────────────────────────────

  «SOURCE: SHORT_UNBOUNDED_LOSS_DISCLAIMER  (verbatim, mandatory — R-SS.4)»
  A short can lose MORE than your €200 — an unbounded loss. A 2× price move
  wipes you out and then some. Simulated paper budget, not financial advice.
  «SOURCE: FORWARD_PLAN_RULE_SHORT_LIQUIDATION»
  If the loss reaches the maintenance-margin floor the short is force-liquidated
  — the loss is not capped at your €200.
```

> **Ratification note for the operator.** The wording above is built almost entirely from
> strings the product ALREADY ships and that already passed the not-advice discipline; the
> `«NEW»` lines are the export's own section headers + three framing/honesty lines (the
> "following this plan manually would mean" frame, the descriptive BenchmarkWins→"the honest
> plan is buy-and-hold" bridge, and the era-qualified-thesis one-liner). The operator's
> ratification is specifically: **(a)** is the modal-case-first ordering right (BenchmarkWins
> before ActiveWins)?; **(b)** is the "following this plan manually would mean X" frame the
> correct not-advice register?; **(c)** are the three `«NEW»` honesty lines (the ~1/5 note,
> the era-qualified thesis, the "your decision, your account" footer) worded acceptably?;
> **(d)** any redlines. On ratification the architect locks these into the serialiser as
> new `crate::strings` constants.

## Trace

`REQ-V3-P5-HANDOFF-EXPORT-001` in [`spec/trace.toml`](../../trace.toml), state
`proposed` (analyst-authored). Product anchor: [`product.md`](../../product.md) § journey
step 5 (SUGGEST) + § What this product IS NOT (not-advice, paper-only). Remediation-plan
anchor: [`spec/backlog.md`](../../backlog.md) § Remediation plan P5.

## Changelog

- 2026-07-10 (analyst): feature proposed from `spec/backlog.md` § Remediation plan P5
  (ratified 2026-07-09, "wording operator-ratified before build"). Framed the SUGGEST →
  manual hand-off export as a DETERMINISTIC, offline, LLM-free serialisation of the EXISTING
  `ForwardPlan` + `BakeoffReportMirror` (+ `DataQualityView` + optional already-gated F9
  narration) — no order placement, no venue API (register B-2 intact), no new engine
  computation, no new numbers. Wrote the not-advice boundary analysis (the export describes a
  computed plan; the "following this plan manually would mean X" frame + the co-located-caveat
  structure keep it on the information side of the advice line). Delivered the COMPLETE draft
  wording template instantiated on the golden `(BTCUSDT, €200, 2024 H1)` case — BenchmarkWins
  modal-case-first honest verdict, the ActiveWins case with the P1 weak-evidence variant
  verbatim, the IF/THEN rule restatement, €200 sizing incl. the €200→USDT note, the
  era-qualified thesis + survivorship/data-trust notes, the short/unbounded-loss risk section,
  "What this is NOT" (incl. the honest P2-2 ~1/5-on-nulls note), and the provenance footer.
  Answered the shorts question: shorts CAN be crowned (empirically `always_short` crowned
  Robust on a 2022-Q2 bear window while the 4 `_ls` arms were Fragile), and when crowned the
  export MUST carry `SHORT_UNBOUNDED_LOSS_DISCLAIMER` + the liquidation line. Every number/line
  in the template traced to a value/`crate::strings` constant already on the mirror
  (verbatim-number discipline). Q-HE-1..6 for the architect (format / trigger location / one-per-
  plan / crate layering / verification / empty-plan guard); the ONE operator decision isolated
  (DECISION-P5-WORDING — ratify the draft wording before build). Baseline gates green
  (anchors 119/119, spec-lint PASS 0). No code; no anchored content; no ADR (analyst brief).
