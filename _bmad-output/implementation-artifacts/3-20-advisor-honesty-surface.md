# Story 3.20: advisor-honesty-surface

Status: backlog

<!-- Created 2026-08-04 by the adversarial product review
     (docs/dev-notes/product-review-2026-08-04.md, findings 1/5/9/10/13).
     Carried as PRD §13 Q8 — operator go/no-go pending. -->

## Story

As a retail user who was just told to hold rather than trade,
I want the verdict screen to show me what was searched, how certain it is, and what to do next,
so that the product's honesty lives in what I can SEE — not only in the machinery behind it.

## Acceptance Criteria

1. **What was searched.** The verdict surface enumerates the arms actually evaluated for
   this run (count + names, from the live registry — never a hardcoded list), the window,
   and the data revision. A user can distinguish "no strategy beat holding" from "none of
   the N we ran beat holding."
2. **Null vs failure.** A crowned benchmark renders a positive statement of search
   completeness ("N arms ran, M produced signals, all failed the gate") — the screen must
   not be reachable in a state where a silent search failure is visually identical to the
   honest null. At least one negative-control render test proves the two states differ on
   screen.
3. **The scorecard reads as what it is.** The DSR / N_eff / MinBTL block is labelled
   report-only (it does not gate the crown — register entry E-1, ADR-0075). A user must not
   be able to read it as a filter that was applied.
4. **Standing qualifiers are visible.** Where a conclusion currently carries a
   qualification in the record but not on screen — today: the active-trading closure is
   *direction-preserved pending re-lock* (bug-log #67, story 1-25) — the surface shows it,
   sourced from a single in-repo constant so it cannot drift from the record.
5. **A next step exists.** After "hold", the user gets an in-app why-this-lost summary
   (the gate signal that failed, in plain language) and a concrete re-check cadence,
   alongside the existing hand-off export.
6. Standing floor: rendered-PIXEL verification with a negative control (AD-10 — this is a
   UI story, so `docs/dev-notes/iced-ui-render-verification.md` governs); anchors 119/119;
   spec-lint PASS; no strategy/gate behaviour change (presentation only — the FROZEN gate
   is byte-untouched and the crown is unchanged by construction).

## Tasks / Subtasks

- [ ] UX pass: what the honest verdict screen says, in what order, without becoming a wall of text.
- [ ] Dev: registry-sourced arm inventory; search-completeness statement; scorecard labelling; qualifier constant; why-this-lost + cadence.
- [ ] Render verification: populated + negative-control screenshots per AD-10 (BLOCKED until the pixel gate is green — see story 6-9's embedded-font prerequisite).

## Dev Notes

- Origin: product review 2026-08-04 findings 1 (success state == silent-failure state), 5
  ("all strategies" unenumerated), 9 (scorecard displayed but disarmed), 10 (thesis
  asterisk absent from UI), 13 (no next step after "hold").
- **Sequencing:** AC6's pixel proof cannot be produced while the 62-test baseline gate is
  red — story 6-9's embedded-font fix is a hard prerequisite for shipping this story, not
  merely a nice-to-have.
- Do-not-build register: NOT implicated — this is presentation of what the product already
  computed, not a new alpha surface, not multi-asset, not a gate change. The register's
  own thesis ("the product exists to refuse alpha-chasing") argues FOR this story: the
  refusal is only credible if the user can see what was refused and why.
- Deliberately NOT in scope: changing the crown, arming the scorecard (E-1 stands), or
  softening the honest-null verdict.

### References

- Trace: `REQ-ADVISOR-HONESTY-SURFACE-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP)
