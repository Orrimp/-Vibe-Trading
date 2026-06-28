---
slug: lab-end-to-end-v2
status: shipped
owner: tester
updated: 2026-05-30
kind: verification-cross-reference
---

# Verification cross-reference — lab-end-to-end-v2 v0.1.0

> **Not a backtest report. Not an anchored artifact.** This is a
> cross-reference note created 2026-05-30 by the spec-hygiene cleanup
> pass to close a `spec-lint` `shipped-no-tests` violation. The feature
> WAS verified; the evidence lives inline in `feature.md` and the trace
> row rather than in a standalone `reports/test-*.md`. This note points
> to where that evidence already exists. No test results are reproduced
> or fabricated here.

## Where the verification evidence lives

`lab-end-to-end-v2 v0.1.0` was developer-verified inline (gate results
recorded per wave) rather than via a separate tester `test-final-*.md`
report. The authoritative records are:

1. **`../feature.md` § Implementation** — per-wave gate results, including
   Wave D-1:
   - `cargo fmt --check`: PASS
   - `cargo clippy --workspace --features candle,live -- -D warnings`: PASS
   - `cargo test --workspace --lib`: 1070 passed, 0 failed
   - `bash scripts/verify_anchors.sh`: ANCHORS PASS (34 / 34)
   - Tests added are enumerated under each wave (state.rs, cockpit_live.rs,
     lab_run_integration.rs forensic-gate tests, etc.).

2. **`spec/trace.toml` row `REQ-LAB-E2E-V2-001`** — `state = "shipped"`
   (flipped 2026-05-25), comment cites: *"Waves D-1/D-2/D-3/D-4 + D-2.5
   per-pair filter all shipped (commits 79fceb5, 992dd82, cb065fa); 1136
   lib tests + 3/3 lab_run_real_engine PASS; 34/34 anchors byte-identical."*

3. **Git history** — ship commits verified present on `main`:
   - `79fceb5` feat(lab): Wave D-3 Stop button + D-4 progress bar
   - `992dd82` feat(lab): F3 cross-sectional fills + bars aggregation
   - `cb065fa` feat(lab): #57 D-2.5 per-pair filter; #58 trace state alignment

## Why no standalone test report

This feature predates the consistent `reports/test-final-*.md` convention
for analyst-owned closure passes. The 34/34 anchor invariant (its primary
non-regression gate per `feature.md` § R10) is enforced by
`scripts/verify_anchors.sh`, which remains green. The verification is real;
only its packaging differs from the standard tester-report shape.

## Open item for operator / tester (NOT auto-resolved)

If the project wants this feature to carry a standard tester report for
audit-trail uniformity, a tester pass can re-run the workspace test suite +
`verify_anchors.sh` against the current `main` and emit a proper
`test-final-<date>.md` here. This was NOT done in the hygiene pass because
(a) the cleanup brief is SPEC-ONLY (no `cargo`), and (b) fabricating test
output would violate the no-fabrication rule. Flagged as a documentation-
shape gap, not a verification gap.

## Changelog

- 2026-05-30 (spec-hygiene cleanup): note created to close the
  `shipped-no-tests` lint violation by cross-referencing the existing
  inline verification evidence. No test results fabricated.
