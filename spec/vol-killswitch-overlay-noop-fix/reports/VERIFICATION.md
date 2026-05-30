---
slug: vol-killswitch-overlay-noop-fix
status: shipped
owner: tester
updated: 2026-05-30
kind: verification-cross-reference
---

# Verification cross-reference — vol-killswitch-overlay-noop-fix v0.1.0

> **Not a backtest report. Not an anchored artifact.** This is a
> cross-reference note created 2026-05-30 by the spec-hygiene cleanup
> pass to close a `spec-lint` `shipped-no-tests` violation. The feature
> WAS verified; the evidence lives in `spec/bug-log.md` § `#65` rather
> than in a standalone `reports/test-*.md`. This note points to where
> that evidence already exists. No test results are reproduced or
> fabricated here.

## Where the verification evidence lives

This is a **P0 safety Bug-fix brief** that was deliberately closed via the
bug-log rather than a formal `test-final-*.md` — a documented orchestrator-
flow precedent ("simple Bug-fix briefs may close via bug-log when test-final
is overkill"). The authoritative records are:

1. **`../../bug-log.md` § `#65`** (`vol_killswitch_overlay` is a no-op) —
   Status FIXED 2026-05-26. Contains the full test evidence block (all 4
   tests green after fix):
   ```
   test post_trigger_signals_are_hold ... ok
   test broadened_filter_dampens_cross_sectional_basket ... ok
   test passthrough_when_threshold_unreachably_high ... ok
   test trigger_fires_and_equity_diverges ... ok
   test result: ok. 4 passed; 0 failed; 0 ignored
   ```
   Note `trigger_fires_and_equity_diverges` — this is the
   baseline-equity-divergence end-to-end test mandated by `CLAUDE.md`
   § Non-negotiables for any strategy overlay / sizing-modifier. It is
   GREEN, satisfying the day-1 e2e gate.

2. **`spec/trace.toml` row `REQ-VOL-KILLSWITCH-NOOP-FIX-001`** —
   `state = "passed"` (flipped 2026-05-27), comment: *"bug-log #65 records
   fix shipped 2026-05-26 (Q4=(p3) 'Both'); orchestrator re-verified
   2026-05-27 (cargo test -p strategy --test vol_killswitch_overlay_end_to_end
   → 4/4 PASS) during audit-2026-05-27 P0 triage. No formal
   test-final/<DATE>.md; bug-log #65 is authoritative shipping record …
   Spec drift was paperwork-only."*

3. **Overlay hygiene gate** — `vol_killswitch_overlay` was removed from the
   `KNOWN_UNCOVERED` allowlist (2/2 gate tests pass), per bug-log #65.

## Why no standalone test report

Per the precedent recorded in the trace row, simple Bug-fix briefs close
via `bug-log.md` when a full tester `test-final-*.md` would be ceremony.
The fix site is `crates/strategy/src/vol_killswitch_overlay.rs` with the
e2e test at `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs`.
The verification is real and was re-confirmed by the orchestrator on
2026-05-27; only its packaging differs from the standard tester-report shape.

## Assessment: NOT a verification gap

This is a documentation-shape gap, not a missing-test gap. The mandated
baseline-equity-divergence e2e test exists and is green; the bug-log entry
is the authoritative, dated shipping record with reproducible test output.

## Changelog

- 2026-05-30 (spec-hygiene cleanup): note created to close the
  `shipped-no-tests` lint violation by cross-referencing bug-log #65 and the
  trace row. No test results fabricated.
