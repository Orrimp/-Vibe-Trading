---
slug: advisor-handoff-export
status: proposed
owner: analyst
updated: 2026-07-10
---

# Tasks — advisor-handoff-export (P5, the SUGGEST → manual hand-off export)

Analyst-authored skeleton for the **architect** to refine at M-T1. **The build is
GATED behind the operator's `DECISION-P5-WORDING` ratification** (the remediation-plan
"wording operator-ratified before build" gate) — T0 below is that gate, and no code task
starts until it is checked. See [`feature.md`](feature.md) for the brief, the not-advice
boundary analysis, the § Draft wording template (the thing being ratified), the shorts
finding, and Q-HE-1..6.

Hard invariants for every task (from the CLAUDE.md non-negotiables + the P5 non-goals):
**NO order placement / NO venue API (register B-2 intact); NO new engine computation; NO
LLM in the export path (deterministic + offline); NO new numbers (verbatim from the
mirror); NO prescriptive voice in the content ('the plan says X', never 'you should X');
anchors 119/119 before AND after; spec-lint PASS(0).**

## Gate

- [x] **T0 — operator ratifies `DECISION-P5-WORDING`.** ✅ RATIFIED AS DRAFTED (operator, 2026-07-10; orchestrator-recorded — the § Draft wording Variant A+B is the binding serialiser contract). The operator reviews § Draft
  wording in `feature.md` (instantiated on the golden `(BTCUSDT, €200, 2024 H1)` case) and
  ratifies or redlines: (a) the BenchmarkWins-first ordering; (b) the "following this plan
  manually would mean X" not-advice frame; (c) the three `«NEW»` honesty lines (the ~1/5
  note, the era-qualified thesis one-liner, the "your decision, your account" footer);
  (d) any redlines. — _blocking: no build task starts until this is checked._

## Architect (M-T1, after T0)

- [ ] **T1 — decide Q-HE-1..6 + take the ADR.** Format (md vs txt), UI trigger location
  (the SUGGEST/ForwardPlan screen), one-per-plan + the deterministic seed/`last_bar_ts`
  stamp (NOT wall-clock), crate layering (`ui`, respecting `ui`-never-depends-on-
  `strategy`/`exec`/`models`/`llm`), verification floor (golden-text serialiser tests + a
  light render walk if the button is a new visible control), and the empty-plan guard.
  Record the R-HE.12 divergence-e2e-N/A explicitly. Lock the operator-ratified wording into
  the serialiser design. — _acceptance: ADR accepted + registered atomically in
  `adr/README.md`; the ADR does NOT collide with the sibling advisor-lot-realism ADR
  number (read the registry first)._

## Developer ‖ ui-designer (after T1)

- [ ] **T2 — the deterministic serialiser.** A pure function over the EXISTING
  `BakeoffReportMirror` + `agent::config::ForwardPlan` (+ `DataQualityView` + optional
  already-gated F9 narration) → the export text, per the ratified § Draft wording. Every
  line sourced from an existing `crate::strings` const (or a new export-only header/section
  const — NOT a new claim about the plan). Same inputs ⇒ byte-identical output. —
  _acceptance: no network/model call; no re-derived number; the modal `BenchmarkWins` case
  leads the verdict block._
- [ ] **T3 — credibility + data-trust + short branches.** Wire the P1 `crown_credibility`
  verdict (WeakEvidence/Passes/NotApplicable) in-body; the always-present survivorship
  caveat + venue/provenance/trust + any warnings; and — when the crowned arm is
  short-capable — the short IF/THEN rules + the liquidation line + `SHORT_UNBOUNDED_LOSS_DISCLAIMER`
  verbatim (R-HE.7). — _acceptance: BenchmarkWins/AllFragile emit NO credibility badge
  (ADR-0085 NotApplicable); a short crown emits the unbounded-loss disclaimer._
- [ ] **T4 — the export trigger (UI).** The "Export this plan" action on the resolved SUGGEST
  screen (per T1), disabled/absent until a plan is crowned (Q-HE-6). New plain-language
  `crate::strings` const for the label. — _acceptance: no export action offered on the
  empty-plan state; if the control is a new visible surface, it passes the standard
  render walk._
- [ ] **T5 — golden-text + determinism tests.** Golden-text tests on the serialiser over
  `fixtures::fake_bakeoff_report_mirror_five_arm` ((BTCUSDT, ActiveWins, crown_clears_dsr=false,
  €200)): assert the emitted text carries the crowned strategy + the €200 sizing + the
  WeakEvidence line + the always-present survivorship caveat + the disclaimers. Negative
  controls: the `BenchmarkWins` fixture emits NO credibility badge; a short-crowned fixture
  emits `SHORT_UNBOUNDED_LOSS_DISCLAIMER`. A byte-determinism test (same inputs ⇒ identical
  output; no wall-clock). — _acceptance: green; the negative controls prove the branches
  track the state, not a tautology._

## Tester (closes the loop)

- [ ] **T6 — gates + lifecycle.** `cargo build`/`test -p ui` + clippy `-D warnings` + `fmt
  --check` + the golden/determinism tests + `verify_anchors.sh` **119/119** before AND after
  + `spec_lint.py` **PASS(0)**; confirm NO `crates/backtest` diff (no engine computation) and
  register B-2 intact (no order/venue code). Own the `feature.md status` + trace `state` flip
  to `shipped` only on `VERDICT → PASS` (ADR-0082). — _acceptance: the export is a
  deterministic text artifact, adds no anchor, and the honesty context (credibility +
  survivorship + short disclaimer) is present in the golden output._
