---
adr: 0099
title: Make "verbatim" mechanical in the verdict-transcription tests
status: accepted
date: 2026-11-07
supersedes: none
superseded-by: none
---

# ADR-0099: Make "verbatim" mechanical in the verdict-transcription tests

## Context

Three verdict classifiers live inside `[[bin]]` targets, so no integration test can call
them. Each is therefore "tested" by a file that re-declares the types and re-implements the
algorithm, then tests its own copy:

| verdict | shipped | its test | decided in |
|---|---|---|---|
| F1–F4 | `forecast/src/bin/forecast_distribution.rs` `verdict::classify` | `forecast/tests/forecast_distribution_verdict.rs` | ADR-0033 § D3.d |
| V1–V5 | `forecast/src/bin/vol_verdict.rs` `compute_aggregate` + `classify_verdict` | `forecast/tests/vol_verdict_mutual_exclusivity.rs` | ADR-0038 § D1.b |
| V-REG-1..5 | `forecast/src/bin/regime_verdict.rs` `classify_vreg` | `forecast/tests/vreg_mutual_exclusivity.rs` | ADR-0049 § D4 |

The practice is deliberate and its justification holds: inlining keeps the tests free of the
`candle` / `realdata` feature gates, and each file cites the one before it as precedent. All
three state that the copy is **verbatim**.

Nothing enforced that word. Edit a shipped threshold and all fifteen per-label fixtures plus
three 100-sample mutual-exclusivity properties keep passing, green, indefinitely — the suite
would be measuring the transcription, not the shipped behaviour. Measured 2026-11-07
(bug-log `#154`): all four transcribed functions were predicate-faithful, **and two of the
four had already drifted in code** — dropped struct fields, an inlined method, a renamed
local, omitted evidence construction. Harmless, every one. Also unannounced, every one. Four
faithful copies and two silent divergences is what an unenforced convention looks like
*before* it costs anything.

> **CORRECTION 2026-11-07 (same day, after this ADR was accepted).** The paragraph above
> implies the transcription is the *only* coverage. Measured afterwards, it is not: `vol_verdict`
> carries 8 bin-local tests and `regime_verdict` 11, all calling the shipped function and all
> running under default features. Only `forecast_distribution` has none for `classify`, and that
> binary is `required-features = ["candle"]` so its targets are skipped outright. Probed: loosening
> V-REG-2 `0.95 → 0.80` turns 3 of 11 red; loosening V4 `< 7 → < 3` leaves 8 of 8 green, because
> that fixture sets zero symbols improving and so sits nowhere near its boundary. The decision
> below stands unchanged — D1–D5 are what they were — but its justification is narrower and more
> precise: for F1–F4 the gate is the only check of `classify`; for V1–V5 it is the only check that
> sees a loosened boundary; for V-REG it is redundant on thresholds and still the only thing
> pinning the transcription. See bug-log `#154` for the probes.

## Decision

Keep the three ADRs' self-contained-test practice; make its stated invariant mechanical.
`scripts/verdict_transcription_check.py` runs in `.githooks/pre-commit` and in the CI gate
step beside `spec_lint.py`, and it decides as follows.

- **D1 — Branch fidelity, declared.** For each of the three classifiers, the set of `if`
  conditions in statement position is extracted from BOTH files and must equal a set
  **written down in the script**. Both-sides equality alone is not enough: it would pass
  when the two files move together but in a direction nobody reviewed, and it would pass
  when a normaliser bug mangles both identically — which happened while writing this gate
  (`.max(1e-12)` became `.max1e-12` on both sides and compared equal; only the independent
  declaration caught it). A threshold change must therefore touch three places in one
  commit, and that third place is the review moment the practice has been missing.
- **D2 — Normalisation is defined, not best-effort.** String literals are replaced before
  comments are stripped, so a `//` inside a literal cannot eat a line; local `let` bindings
  of simple expressions are substituted, so a named threshold and its literal compare equal
  and a predicate hidden behind `let x = …; if x` is actually compared; parentheses are
  stripped only around a bare literal, never around a call's arguments. Expression-position
  `if`s are excluded — today they differ only because a copy omits the evidence `String`.
- **D3 — A zero-branch function does NOT get the branch check.** `compute_aggregate` has no
  `if` at all, so comparing its condition sets would compare the empty set to the empty set
  and print PASS. That is the exact failure this ADR exists about, so it is refused.
  Instead: every `agg.<field>` the shipped classifier reads must exist on BOTH
  `AggregateStats` declarations — which catches the real risk (the copy's struct losing a
  predicate input, so a branch no fixture can reach) without demanding textual identity of
  arithmetic the copy legitimately inlines.
- **D4 — The gate carries its own probes.** `--self-test` mutates in-memory copies of the
  real sources and asserts each mutation is caught: a loosened shipped threshold, a dropped
  shipped predicate, a loosened COPY threshold, a COPY struct losing a predicate input, plus
  an unmutated control that must be clean. CI runs it on every pass, so the teeth are
  re-measured rather than remembered — the `spec_lint.py --self-test` precedent.
- **D5 — Scope is stated, including what is NOT covered.** The gate does not prove the two
  implementations compute the same numbers. A copy may still diverge in a way it cannot see.
  What a copy can no longer do is diverge in a branch condition or a dropped predicate input
  in silence, and the script's module doc says so in those words rather than implying more.

## Alternatives considered

- **Move the three `verdict` modules into `forecast`'s lib so the tests call the real code** —
  rejected: it would drag the `candle` / `realdata` feature gates into the test targets, which
  is the precise cost ADR-0033 § D3.d, ADR-0038 § D1.b and ADR-0049 § D4 each chose to avoid.
  Reversing three ratified decisions to fix an unenforced claim is the wrong end of the problem.
- **A Rust integration test that parses the two sources** — rejected: same parser, more code,
  and it would sit in `forecast`'s test targets where a feature gate can skip it. The gate
  belongs in the `scripts/` chain that `.githooks/pre-commit` and CI run unconditionally.
- **Assert the full normalised function bodies are identical** — rejected: it fails today on
  three legitimate differences (an inlined method, a renamed local, omitted evidence
  construction), so it would have to normalise renaming and inlining away. A gate that must be
  taught to ignore that much is a gate whose failures nobody will trust.
- **Both-sides equality with no declared set** — rejected under D1 above: it cannot see a
  coordinated change, and it cannot see a normaliser that mangles both inputs the same way.

## Consequences

A verdict threshold can no longer move on one side alone, and cannot move on both sides
without the declared set moving too — a three-file commit, by design. A legitimate change
will be blocked at `git commit` until the script is updated; that is the intended friction
and `--no-verify` remains the documented emergency bypass.

Mechanically checked by:

- `scripts/verdict_transcription_check.py` — the gate (declared sets live in `BRANCH_PAIRS`
  and `INPUT_PAIR`).
- `scripts/verdict_transcription_check.py --self-test` — four probes plus a clean control.
- `.githooks/pre-commit` and `.github/workflows/ci.yml` (gate step) — where it runs.

Violated if: a classifier's conditions change in one file only; a copy's `AggregateStats`
loses a field the shipped predicates read; or the declared set is emptied, which the gate
reports as vacuous rather than passing.

## Changelog
- 2026-11-07 (orchestrator): initial accept. Written with bug-log `#154`, which carries the
  measurement (all four functions predicate-faithful; two already code-drifted) and the
  near-miss that forced D1's independent declaration.
