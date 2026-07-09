# spec/v3 — "prove it's done," not "do more"

This folder holds the **v3 close-out phase**: a *bounded* ship-readiness pass, not
a feature program. v2 is **feature-complete** — the research-driven roadmap (11
features) consumed every ship-worthy application from the 900-paper program, and the
remaining surface is either explicitly-deferred polish or the alpha-chasing the
product exists to refuse. See the scoping memo
[`spec/dev-notes/post-v2-scoping-2026-07-09.md`](../dev-notes/post-v2-scoping-2026-07-09.md)
for the full evidence.

> **The one thing v3 is:** prove the DATA → CALIBRATE → ANALYZE → SUGGEST spine
> hangs together in one honest pass, name its one still-implicit stage, document the
> decisions the operator has already made, and reconcile the docs to
> "feature-complete." **v3 adds no alpha surface.** Every item hardens or *finishes*
> what already exists.

## Operator decisions binding this phase (2026-07-09)

1. **Full close-out** = R3-2 (end-to-end demo) + R3-3a (Calibrate-stage stepper) +
   R3-3b (surface the D3 DSR decision) + R3-4 (docs).
2. **CI stays PARKED.** R3-1 (activate the cross-platform CI matrix) is **OUT of
   scope this phase.** `.github/workflows/ci.yml.deferred` stays inert — do **not**
   `git mv` it. This phase designs nothing that activates CI.

The operator is in **lock-it-down mode, not change-the-gate mode.** That framing is
load-bearing for R3-3b (below): the D3 DSR decision is *documented*, not re-litigated
into a crown veto.

## The four items

| Item | What | Owner | Kind | Status |
|---|---|---|---|---|
| **R3-3a** | Promote `Screen::Tune` to a first-class named **"Calibrate"** stage + a visible **DATA → CALIBRATE → ANALYZE → SUGGEST** stepper across the existing screens. Build the VISIBLE stepper only; **defer** the `agent::AdvisorStage` context-carrier again (D7). | ui-designer (+ developer if needed) | **the ONE real build** | `spec/v3/advisor-calibrate-stage/` — see ADR-0083 |
| **R3-3b** | **Document** the D3 report-only DSR decision + its empirical basis (the P2-2 no-alpha CI). Record that the crown-eligibility veto is a **ready-but-unbuilt** one-line switch (`crown_clears_dsr`). NOT a build; NOT a gate change. | architect (done) | decision doc | [`spec/dev-notes/dsr-report-only-decision-2026-07-09.md`](../dev-notes/dsr-report-only-decision-2026-07-09.md) |
| **R3-2** | A runnable/scripted **end-to-end demo** + committed narrative exercising the whole spine in one honest pass, showing the modal `BenchmarkWins` null as the product working. | presenter | walkthrough + runbook | scoped below — build AFTER R3-3a lands |
| **R3-4** | Reconcile README/CHANGELOG to "v2 complete / feature-complete"; author the authoritative **do-not-build register** so the research dead-ends stop being re-proposed. | docs (analyst/architect) | pure docs | scoped below |

### Hard constraints (every v3 item honors these)

- **FROZEN gate untouched.** `crates/backtest/src/bakeoff/{robustness,rank}.rs` stay
  byte-frozen. No veto wiring. The `crown_clears_dsr` switch stays informational-only.
- **Anchors 119/119.** No edits under `spec/**/reports/` (byte-immutable, keyed by
  NAME). v3 adds **zero** anchors by design (UI/report-only + docs).
- **`feature.md` is the single source of truth** for lifecycle; `trace.toml`
  mirrors it (ADR-0082). A trace row is `arch-done`/design-complete until the feature
  actually ships.
- **CI parked** — no change to `.github/workflows/`.

## R3-3a — the Calibrate stepper (the one real build)

**Spec:** [`advisor-calibrate-stage/feature.md`](advisor-calibrate-stage/feature.md)
· [`advisor-calibrate-stage/tasks.md`](advisor-calibrate-stage/tasks.md) · ADR-0083.

The IA finding that shapes it: today `Screen::Tune` is a *Lab drill-down*
(`crates/ui/src/state.rs`, reached only via `OpenTuneEditor` — it is NOT in the
sidebar), and the four spine verbs do **not** map 1:1 to four screens (DATA and
ANALYZE both live inside `Screen::Leaderboard`). So the stepper is an **orientation
affordance** ("you are here" across the journey), NOT a strict router. It highlights
the current stage from `current_screen`, and promoting Tune → "Calibrate" gives the
spine its second *named, sidebar-visible* stage. Full grounding + the render-layer
verification contract are in the feature spec.

**Verification is at the rendered-pixel layer** (CLAUDE.md non-negotiable): a new
`#![cfg(target_os = "macos")]` render harness proving the stepper paints with the
correct stage highlighted, plus a negative control (a different `current_screen`
highlights a different stage). A passing model state / text snapshot / no-panic boot
is NOT proof. See [`spec/dev-notes/iced-ui-render-verification.md`](../dev-notes/iced-ui-render-verification.md).

## R3-2 — end-to-end demo (presenter, after R3-3a)

**Owner:** presenter. **Build order:** *after* R3-3a lands, so the demo shows the
final IA (the named Calibrate stage + the stepper).

**Deliverable:** a committed narrative at `spec/runbooks/advisor-end-to-end-demo.md`
(+ a scripted cockpit walkthrough / render-verified screenshot walk if practical)
with a golden `(coin, budget, window)` → DATA-quality panel → CALIBRATE sweep →
ANALYZE leaderboard + scorecard → SUGGEST forward plan → forward paper-run, in one
pass.

**Acceptance criteria:**
- Exercises all four spine stages end-to-end from a single golden input, in order.
- Shows the modal `BenchmarkWins` null **as the product working** — the honest
  "nothing beat holding" outcome, NOT a manufactured "active wins." (Honest framing
  is load-bearing.)
- Surfaces the scorecard haircut (DSR/N_eff/MinBTL) side-by-side with the crown, per
  the R3-3b report-only decision.
- Runs against the existing engine with **no new engine code** and **no anchor
  churn** (uses `write_report=false` advisor paths).
- If it includes screenshots, they are render-verified (a populated state, per the
  UI-render-verification note) — not hand-mocked.
- Reconciles cleanly with `python3 scripts/spec_lint.py` (a runbook, no frontmatter
  drift) and leaves `bash scripts/verify_anchors.sh` at 119/119.

## R3-4 — docs + do-not-build register (analyst/architect)

**Deliverables:**
1. **Status reconciliation** — README.md + CHANGELOG.md updated to state "v2
   complete / feature-complete; v3 = bounded close-out." `spec/backlog.md`'s forward
   queue confirmed accurate (the CI item stays "deferred/parked, operator gates
   activation").
2. **The do-not-build register** — an authoritative doc (proposed home:
   `spec/dev-notes/do-not-build-register.md`) consolidating the off-track register
   (scoping memo §4) + `research/APPLICATIONS.md` "Dead ends" so ML/forecasting,
   multi-coin, automated alpha search, LLM-as-trader, etc. stop being re-proposed as
   "gaps" each session (project memory shows they recur).

**Acceptance criteria:**
- Every off-track idea in scoping-memo §4 appears in the register with the guardrail
  it violates and a one-line "why it stays dead."
- README/CHANGELOG no longer imply open feature work beyond this close-out phase.
- Pure docs: no `crates/` change, no anchor change, `spec_lint.py` PASS.

## What v3 is explicitly NOT

- **NOT** a new-feature program (v2 drained the ship-worthy research; see the memo).
- **NOT** a CI activation (R3-1 is parked by operator decision).
- **NOT** a gate change — no DSR/PBO crown veto is wired (R3-3b documents the
  decision; the switch stays unbuilt).
- **NOT** any of the off-track ideas (multi-coin, return prediction in ranking,
  automated alpha search, LLM-as-trader, new alpha-chasing signal primitives, live
  trading) — those are enumerated-and-rejected in the do-not-build register.

## Changelog

- 2026-07-09 (architect): created `spec/v3/` + this README framing the bounded
  close-out phase; scoped the four items (R3-2/R3-3a/R3-3b/R3-4); designed R3-3a
  (`advisor-calibrate-stage/`, ADR-0083); framed R3-3b
  (`spec/dev-notes/dsr-report-only-decision-2026-07-09.md`). Made `spec_lint.py`
  v3-aware (folder resolution + orphan-feature + trace + feature-shipped-drift) and
  documented `verify_anchors.sh` as already reorg-agnostic. CI stays parked (R3-1
  OUT).
