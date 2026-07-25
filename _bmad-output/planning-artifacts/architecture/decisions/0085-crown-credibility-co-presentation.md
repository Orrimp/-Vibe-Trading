---
id: ADR-0085
title: Crown co-presents its overfitting verdict (P1, presentation-layer)
status: accepted
date: 2026-07-09
feature: advisor-crown-credibility
supersedes: []
superseded_by: []
---

# ADR-0085 — The crown co-presents its overfitting (DSR) verdict

> **Numbering note.** Authored concurrently with ADR-0084 (P2 corpus expansion) by a
> sibling architect. ADR-0084 was already registered in the README when this ADR was
> written, so this one took the next free number (0085) per the concurrent-edit
> protocol. Same date is expected.

## Context

The FROZEN robustness gate crowns pure noise **~1 in 5 seeds** (P2-2 empirical,
`crates/backtest/tests/null_data_no_crown.rs`). The Deflated-Sharpe (DSR) scorecard
**catches every one** (`Scorecard.crown_clears_dsr == false` on those chance-crowns)
— that two-layer property is why the scorecard is load-bearing
(`docs/dev-notes/dsr-report-only-decision-2026-07-09.md`).

But the **presentation** undercuts the protection. The crown ("`v0.sma` is the best
risk-adjusted pick.") is an `H2` headline at the top of the Recommendation panel
(`recommendation_block`, `crates/ui/src/screens/leaderboard.rs`). The credibility
verdict lives in a **separate scorecard panel BELOW the ranked table** (`scorecard_block`,
composed under the table in `ready_pane`). A user who reads the crown and stops — the
natural reading path — is misled at a **measurable rate**: the banner asserts "best
pick" with no co-located honesty signal.

The remediation-plan **P1** decision (`spec/backlog.md` § Remediation plan, ratified
2026-07-09) is **D1=(a) — fix at the PRESENTATION layer**: the crown banner must
co-present the credibility verdict. **No gate change** — D3 report-only stands
(re-confirmed 3×); `rank.rs` continues to NOT read `crown_clears_dsr`. This is
additive UI.

This ADR governs the **crown-presentation contract**: what the recommendation banner
is allowed to assert on its own, and what honesty signal it must co-present. That is a
cross-cutting invariant (the banner is the product's recommendation centrepiece across
every advisor journey), hence an ADR rather than a silent `view` tweak.

## Decision

**D1 — the crown banner co-presents the credibility verdict for an active crown.**
When an active (non-benchmark) arm is crowned (`RecommendationOutcome::ActiveWins`),
the recommendation banner renders the crown's DSR verdict **inline, directly under the
`H2` headline** — co-located with the "best pick" claim so the two are read together.
The banner may no longer assert "best pick" for an active crown without co-presenting
whether that pick survived the overfitting check.

**D2 — the state is a pure projection; no new stored state field, no `crates/backtest`
change.** A pure `fn crown_credibility(outcome: OutcomeKind, scorecard:
Option<&ScorecardView>) -> CrownCredibility` (a transient `view`-time enum:
`Passes | WeakEvidence | NotApplicable`) resolves the state from values already on the
`BakeoffReportMirror` at the banner (`recommendation.outcome` + `scorecard`). Mirrors
the ADR-0083 D2 `stage_for` discipline (read existing state, no new field). The
existing informational `Scorecard.crown_clears_dsr` / `ScorecardView.crown_clears_dsr`
field is **read, never made a veto**; `crates/backtest` is byte-untouched.

**D3 — three states + their register:**
- **`Passes`** (`ActiveWins` + `crown_clears_dsr==true`): a muted `✓` "Passed the
  overfitting check" line in `ACCENT` — a reassurance, not a celebration.
- **`WeakEvidence`** (`ActiveWins` + `crown_clears_dsr==false`, the money shot): an
  unmissable inline band under the headline — `⚠` + `WARN_500` text on `WARN_50`
  fill, `WARN_500` 1px border + `radius::R3`, `width(Fill)` — carrying plain-language,
  **non-alarmist** copy that names the pick as *weak evidence* and states the mechanism
  ("with this many strategies tried, an edge this size can appear by chance"). It
  **qualifies** the still-true headline; it does not negate it. Register is **caution,
  not alarm** — `WARN` tier, explicitly NOT `NEG_*`/error-red (the pick is weakly
  evidenced, not broken).
- **`NotApplicable`** (everything else): the banner renders as today (zero-size
  `Space`, byte-identical pre-feature layout).

**D4 — `BenchmarkWins` and `AllFragile` carry NO credibility badge** (the
no-misleading-badge rule). Buy-and-hold is **exempt from the gate** (ADR-0066 § D1 —
the benchmark is the baseline, not a candidate), and the scorecard's `deflated_sharpe`
is computed on the **max-Sharpe active arm (a loser)**, not on the crowned
buy-and-hold. Attaching a "fails the overfitting check" badge to a *hold*
recommendation would bind an active-arm statistic to a passive pick — actively
misleading. The existing `BenchmarkWins` headline + the scorecard panel's "Beats
holding after the search? → Not clearly — holding is the honest call" already read
correctly in context. `AllFragile` is a null verdict on a fragile active field (no
crowned pick whose evidence-strength to caveat; DSR is on a fragile active arm) — same
`NotApplicable` treatment. `ActiveWins` with `scorecard == None` (gate not run /
degenerate) is also `NotApplicable` — no computed figure to present.

**D5 — dual-mode + accessibility + zero-literal discipline (CLAUDE.md UI rules).**
All colour resolves via `ModeColor::current(mode)` (dual-mode; `WARN_50`/`WARN_500`/
`ACCENT`/`FG_3`); **no new theme token**. Colour is never the only signal — `⚠`
(weak) / `✓` (passes) glyphs + the literal words carry the state. All copy is new
`crate::strings` constants registered in `strings::all()`; **no new dependency**
(`cargo tree -p ui` unchanged).

**D6 — verification at the RENDERED-PIXEL layer (CLAUDE.md non-negotiable).**
`crates/ui/tests/crown_credibility_render.rs` (`#![cfg(target_os = "macos")]`,
ADR-0057 gate) proves, at the pixel layer with negative controls:
(1) the `WeakEvidence` band paints on the banner for the `ActiveWins` +
`crown_clears_dsr==false` fixture (`fake_bakeoff_report_mirror_five_arm`, the money
shot); (2) the same mirror with `crown_clears_dsr` flipped `true` shows `Passes` and
NOT the WARN band (the flag-tracks-the-render control); (3) `BenchmarkWins` shows NO
credibility band. Plus `crown_credibility(...)` unit tests per D3/D4 row. A passing
model state / text snapshot / no-panic boot is NOT sufficient.

**D7 — anchor + FROZEN-gate safety.** UI-only; the advisor bake-off path runs
`write_report=false` and the scorecard rides `Recommendation` (a backtest-internal
type off the anchored CLI report path). **Anchors 119/119 by construction**, verified
before AND after. No edit to `crates/backtest/src/bakeoff/{robustness,rank}.rs` or
`scorecard.rs`; no verdict changes; the `scorecard_does_not_change_ranking` invariant
is undisturbed. The crown-eligibility veto stays **unbuilt** (do-not-build register
E-1).

## Alternatives considered

- **Only restyle the scorecard panel (bolder), leave the banner untouched.**
  Rejected — it leaves the credibility verdict below the fold on the natural reading
  path (crown at top, panel below the table). P1 is explicitly "co-present on the
  crown itself" (D1=(a)); a prettier panel does not satisfy it.
- **A red/error (`NEG_*`) treatment on `WeakEvidence`.** Rejected — overstates it. The
  pick is not broken, it is weakly evidenced (the honest modal crypto reality). `WARN`
  tier + plain "weak evidence" wording is the calibrated, non-alarmist register the
  product tone demands.
- **Store a `credibility` field on `BakeoffReportMirror` / `ScorecardView`.** Rejected
  — the verdict is a pure projection of two fields already present; a stored field
  duplicates derivable state and risks drift (ADR-0083 D2 precedent: read existing
  state, no new field).
- **Wire `crown_clears_dsr` as a `rank.rs` eligibility veto** (the "it's additive"
  framing). Rejected / out of scope — it changes the FROZEN gate's effective crowning
  behaviour and is a settled dead-end without a separate operator decision + its own
  ADR + anchor-impact + a day-1 bite test (do-not-build register E-1;
  `dsr-report-only-decision-2026-07-09.md` § the four-step wiring bar). This ADR is
  presentation-only.

## Consequences

- The recommendation centrepiece is now honest on its own: a chance-crowned active
  pick can never be read as "best pick" without the co-located weak-evidence caveat.
  Closes the measurable "trust the crown alone → misled" gap at the presentation
  layer with zero gate risk.
- The banner-presentation contract is now an invariant: future banner changes must
  preserve the co-presentation of the credibility verdict for an active crown (and the
  no-badge rule for a benchmark/fragile crown).
- No behavioural change to crowning, ranking, eligibility, or any anchored artifact —
  the change is byte-invisible to the engine and the 119 anchors.

## References

- `spec/backlog.md` § Remediation plan **P1** (D1=(a), ratified 2026-07-09).
- `spec/v3/advisor-crown-credibility/feature.md` (this feature; § Design has the exact
  copy + tokens + states) + `tasks.md`.
- `docs/dev-notes/dsr-report-only-decision-2026-07-09.md` (D3 report-only — the
  decision this ADR honours; the veto stays unbuilt).
- `docs/dev-notes/do-not-build-register.md` row **E-1** (the veto dead-end).
- ADR-0075 (the report-only overfitting scorecard — `crown_clears_dsr`, `DSR_THRESHOLD`).
- ADR-0066 (benchmark exemption — grounds the `BenchmarkWins` no-badge decision, D4).
- ADR-0083 (`stage_for` pure-resolver / no-new-field precedent, D2) + ADR-0057
  (macOS render-gate, D6).
- `crates/ui/src/screens/leaderboard.rs` (`recommendation_block`, `ready_pane`,
  `scorecard_block`); `crates/ui/src/leaderboard/state.rs`
  (`ScorecardView::from_scorecard`, `OutcomeKind`); `crates/ui/src/theme.rs`
  (`WARN_50`/`WARN_500`); `crates/ui/tests/leaderboard_scorecard_render.rs` (harness
  precedent).
