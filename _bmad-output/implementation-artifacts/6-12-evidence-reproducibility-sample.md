# Story 6.12: evidence-reproducibility-sample

Status: ready-for-dev

<!-- Created 2026-08-04 by the adversarial product review (PRD §13 Q7; operator
     decision 2026-08-04: BUILD). Finding 8: every pinned corpus is machine-local
     and gitignored, so no second party can reproduce a single real-data claim. -->

## Story

As anyone who is not the operator — a future maintainer, a reviewer, a sceptic,
or the operator on a new machine,
I want to reproduce at least one of this product's real-data figures from a fresh clone,
so that "honest" means verifiable-by-someone-else, not merely honestly-intended.

## Acceptance Criteria

1. **A committed sample corpus exists.** A small slice of real pinned data (one symbol,
   one bounded window — sized to stay comfortably inside a git repo, LFS if warranted) is
   committed with its own `REVISION.toml` following the sibling-corpus convention, and the
   `.gitignore` exception is added in the same style as `data/binance/`.
2. **One documented figure reproduces end to end.** A single command from a fresh clone
   produces a named figure (a backtest headline number or an equity point) that matches a
   committed expected value within a stated tolerance — the recipe, the command, and the
   expected value live together in a runbook.
3. **It is honest about what it is NOT.** The runbook states plainly that the sample
   reproduces *one* claim, that the full corpora remain machine-local, and that the 119
   anchored bodies are verifiable from the repo alone (they already are — anchors hash
   committed report bodies) while the *runs behind them* are not re-executable by a third
   party. Do not let a sample corpus imply full reproducibility.
4. **It cannot rot silently.** The reproduce path is exercised by CI (or by the anchors
   gate) so that a drift in the engine, the loader, or the sample makes it fail loudly
   rather than becoming a stale recipe in a document nobody runs.
5. Standing floor: anchors 119/119 before AND after (the sample corpus must not disturb any
   existing anchored resolution); spec-lint PASS; the pinned-revision discipline
   (ADR-0032/0040 family) applies to the sample exactly as to the full corpora.

## Tasks / Subtasks

- [ ] Choose the slice (symbol, window, size budget) and the figure it will reproduce — smallest thing that proves the chain end to end.
- [ ] Commit the sample + `REVISION.toml` + `.gitignore` exception; verify the loader's pin check passes on it.
- [ ] Write the runbook recipe (command, expected value, tolerance, the honest-limits paragraph).
- [ ] Wire the reproduce check into CI so it cannot rot.

## Dev Notes

- Origin: product review 2026-08-04 finding 8 / PRD §13 Q7, operator decision BUILD.
- Companion to `docs/runbooks/corpus-restore.md`, which solves a different problem: that
  runbook restores the operator's own machine; this story lets a *second party* verify
  something. Cross-link both ways.
- Sizing discipline: this repo already carries LFS-tracked forecast checkpoints, so large
  binaries are not unprecedented — but the sample must stay small enough that a clone is
  not punished. Prefer one symbol × one month over anything broader.
- The existing pin machinery (`data::revision`, hardened by the story-1-12 review with
  completeness-at-emit and pinned-SHA skip) applies unchanged; do not fork it for the
  sample.

### References

- Trace: `REQ-EVIDENCE-REPRODUCIBILITY-SAMPLE-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 6 (Remediation, Infra & Governance)
