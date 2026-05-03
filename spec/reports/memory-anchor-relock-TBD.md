# Memory anchor relock — TBD

This file is a forward-compatibility marker for the future
**reflection-memory** feature.  It is **not** a feature brief and is
**not** owned by the analyst — it is a TODO breadcrumb that the
eventual reflection-memory architect can `grep` for so the operator-
success-report anchors get re-locked when the placeholder body is
replaced by real reflection-memory output.

## Context

Operator-success-reports v1+ ships the R6 "memory highlights" body as
a fixed placeholder string (see
[`crates/reports/src/render/memory_highlights.rs`](../../crates/reports/src/render/memory_highlights.rs)).
That placeholder is **locked into the v1+ anchor SHA-256s** captured
at task **T816** of `spec/tasks/operator-success-reports.md`.

When the reflection-memory feature ships and the placeholder body
changes, the determinism gate will FAIL on the first run after the
change.  The eventual reflection-memory feature's brief MUST include a
deliverable to re-lock the two new operator-success-report anchors —
the same precedent v1.5a applied to the top10-momentum anchors at task
**T717** of `spec/tasks/v15a-mean-reversion-pairs.md` (the
"anchor re-lock" pattern).

## What the eventual architect must do

1. Rename / replace `crates/reports/src/render/memory_highlights.rs`
   placeholder body with the real reflection-memory output.
2. Run the two operator-success-report scenarios
   (`report-sample-7d`, `report-sample-90d`).
3. Capture the new body-SHA-256s.
4. Update `spec/anchors.toml` with the two new SHAs.
5. Cross-reference this note in the reflection-memory feature brief
   so the orchestrator's tester gate stops here on the round that
   ships the change.

## Owner

Whichever analyst opens the reflection-memory feature opens R6 of
`spec/features/operator-success-reports.md` for re-scoping at the
same time.  The architect for that feature carries the re-lock
deliverable.
