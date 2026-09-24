# Story 3.20: advisor-honesty-surface

Status: ready-for-dev

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
- [ ] Render verification: populated + negative-control screenshots per AD-10.

## Entry-gate notes (orchestrator, 2026-09-24 — read before scoping the dev pass)

**The blocker is gone.** AC6's pixel proof was sequenced behind story 6-9's embedded-font
fix; that shipped 2026-09-16 as **ADR-0093** (the `ui` crate embeds Inter, the 56 baselines
were re-captured, the macOS gate is green). Nothing about this story is blocked now. The
`Status:` line was also stale at `backlog` while the board carried the operator's
`ready-for-dev` BUILD ruling — corrected here, board is the ruling, AD-4 makes this line the
source of truth.

**Two findings that should shrink the dev pass — verify before building, do not assume:**

1. **AC1/AC2 need no new data source.** `BakeoffReportMirror.rows`
   (`crates/ui/src/leaderboard/state.rs:563`) already carries every candidate with its
   `strategy` name, `is_benchmark`, and `trade_count`, built at the single
   `from_report` boundary from the live report. So "N arms ran, M produced signals" is
   derivable from state the screen already holds — the work is the *rendering* and the
   negative-control test, not plumbing a registry read.
2. **AC3 may already be shipped.** `LEADERBOARD_SCORECARD_CAPTION`
   (`crates/ui/src/strings.rs:3325`) reads "An honesty check on the search behind the pick
   — it never changes the result", and it is rendered as the scorecard block's caption at
   `crates/ui/src/screens/leaderboard.rs:982`. Since ADR-0092 that block LEADS the ready
   pane, so it is above the 1080-px fold. **Prove it at the pixels before writing anything
   for AC3** — if it holds, this AC closes as already-met and says so, per the same scope
   guard story 3-21 used. If it does not (e.g. the caption reads as decoration rather than
   a label), that is the finding, and it is a smaller change than the AC implies.

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
