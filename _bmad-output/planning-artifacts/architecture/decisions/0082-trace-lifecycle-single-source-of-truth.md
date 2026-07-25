---
adr: 0082
title: feature.md status is the single source of truth for trace.toml state
status: accepted
date: 2026-07-09
supersedes: none
superseded-by: none
---

# ADR-0082: `feature.md status:` is the single source of truth for `trace.toml state=`

## Context

The weekly auditor has surfaced the same drift class on three consecutive
runs (`docs/dev-notes/audit-2026-06-29.md`, `audit-2026-07-06.md` PRIMARY
FINDING + Class B): features whose `feature.md` frontmatter reads
`status: shipped` while their `spec/trace.toml` `[[req]]` row's `state=`
reads something else. The drift split two ways:

1. **Genuine mid-pipeline lag** — a row left at `design-complete` /
   `in-progress` / `implemented` after the feature actually shipped
   (reconciled by orchestrator commits `2fd968f` and `066fa46`).
2. **Undocumented-but-legitimate vocabulary** — the `trace.toml` header
   legend only enumerated `draft | proposed | candidate | in-progress |
   roadmap | shipped | deprecated`, but the file in practice also uses a
   *tester-terminal* family (`verified` / `passed` / `tested` /
   `tester-done`) plus `arch-done` / `design-complete` / `dev-done` /
   `presenter-done` / `shipped-partial` / `retired`. These are an
   established de-facto pipeline convention, not typos.

The auditor (read-only by charter) explicitly deferred the fix to the
architect with two options: (a) bless the tester-terminal words in the
legend, or (b) codify a lint rule that flips them to `shipped` on feature
close. The operator chose the **durable** fix: not merely blessing the
words, but ratifying a normative invariant and enforcing it mechanically.

## Decision

**D1 — Single source of truth.** A feature's `feature.md` frontmatter
`status:` is the single, authoritative record of that feature's lifecycle
state. A `trace.toml` `[[req]]` row's `state=` MUST NOT contradict it. The
data-flow direction is `feature.md → trace.toml`, never the reverse.

**D2 — The shipped invariant.** Concretely and mechanically: *when a
feature's `feature.md status:` is `shipped`, every `[[req]]` row whose
`feature=` slug resolves to that feature MUST have `state = "shipped"`.*
No other `state` value is permitted once the feature has shipped.

**D3 — Tester-terminal aliases are pre-ship-only.** The tester-terminal
family (`verified` / `passed` / `tested` / `tester-done`) and the other
intermediate pipeline states (`arch-done`, `design-complete`, `dev-done`,
`presenter-done`) remain legitimate — but ONLY while the feature is still
pre-ship. They record a real pipeline position (the tester ran gates
green; the presenter assembled a deck) and are NOT flagged by the lint
while `feature.md` is anything other than `shipped`. The moment
`feature.md` reaches `shipped`, the row is flipped to `state="shipped"`.

**D4 — Provenance-preserving flips.** When a row is flipped to `shipped`,
its prior comment is preserved verbatim after a `PRIOR:` marker, e.g.
`state = "shipped"   # reconciled→feature.md=shipped <date> (<why>). PRIOR:
<the entire old comment kept verbatim>`. The pipeline history in the
comment is never destroyed.

**D5 — Mechanical enforcement.** The invariant is enforced by
`scripts/spec_lint.py`, category `feature-shipped-trace-drift`: for every
`[[req]]` whose `feature=` slug resolves to an existing
`spec/**/feature.md` (searched across `spec/`, `spec/v1/`, `spec/v2/` —
mirroring the existing `check_trace` feature-folder resolution), the check
reads that `feature.md`'s `status:` and, if it is `shipped`, asserts the
row's `state == "shipped"`, emitting a violation otherwise. Rows whose
feature is not shipped are never flagged. The rule ships with a
synthetic pass+fail fixture in `spec_lint.py --self-test`.

**D6 — The legend documents the full vocabulary.** The `trace.toml`
header comment enumerates every `state` value actually in use, each with a
one-line definition, split into lifecycle states (mirroring `feature.md`)
and transient intermediate pipeline states, and states the D2 invariant
inline.

## Alternatives considered

- **Bless the tester-terminal words in the legend and stop there (the
  auditor's option (a))** — rejected: it documents the drift instead of
  preventing it. A `passed` row on a shipped feature would stay
  indefinitely, and the same finding would recur every audit. Blessing the
  vocabulary is necessary (D6) but not sufficient.
- **Auto-flip rows to `shipped` from a script at feature-close** — rejected
  as the *primary* mechanism: a mutation tool that rewrites `trace.toml`
  is riskier than a read-only lint, and the flip is a human-reviewed,
  provenance-preserving edit (D4). The lint (D5) catches drift; the flip
  stays a deliberate edit. (A future close-time helper may assist, but the
  lint is the enforcement floor.)
- **Make `trace.toml state=` the source of truth instead** — rejected:
  `feature.md` is the human-facing lifecycle doc the pipeline agents own
  and the CHANGELOG indexes; `trace.toml` is a derived traceability index.
  Inverting the authority would contradict the established ownership model
  (analyst/architect/developer/tester write `feature.md`; the trace row is
  bookkeeping).
- **A dev-note instead of an ADR** — rejected: this ratifies a normative
  cross-cutting schema invariant AND ships mechanical enforcement, the
  same shape as ADR-registry-atomic-lint. That is ADR-grade; a dev-note is
  for observations and how-tos, not ratified invariants that a lint gates.

## Consequences

- `spec_lint.py` category `feature-shipped-trace-drift` fails the aggregate
  run (non-zero exit) if any shipped feature's trace row is not `shipped`.
  The presenter pre-tick / CI catches the drift at the moment it is
  introduced, replacing weekly-audit archaeology.
- Ratification retired the 11 `state="passed"` rows on shipped features
  live at HEAD on 2026-07-09 (all reconciled to `shipped` with `PRIOR:`
  provenance in the same change). After reconciliation `spec_lint.py`
  reports zero `feature-shipped-trace-drift` violations.
- Rows whose feature is pre-ship (`tested`, `tester-done`, etc. on
  features at `dev-done` / `tester-done` / `presenter-done` / `retired`)
  are untouched and remain legal — the invariant only fires on
  `feature.md status: shipped`.
- The `trace.toml` legend now enumerates the full `state` vocabulary; a
  future new state must be added there and (if terminal) reconciled with
  this invariant.
- `spec_lint.py --self-test` gains a `feature-shipped-trace-drift` fixture
  (a shipped-feature/non-shipped-row pair that must produce exactly one
  violation, plus a compliant pair and a pre-ship pair that must produce
  none). CI runs `--self-test` as the rule's proof.
- **CHANGELOG index (2026-07-10 amendment, remediation-plan P6a).** D1 names
  the CHANGELOG as a derived index authoritative-from `feature.md`
  ("`feature.md` … the CHANGELOG indexes [it]" — the stated reason D1 picks
  `feature.md` over `trace.toml`). A SIBLING enforcement of the SAME invariant
  now guards that second index: `spec_lint.py` category
  `feature-shipped-changelog-missing` asserts that every
  `status: shipped` feature (resolved across `spec/`+`spec/v1/`+`spec/v2/`+
  `spec/v3/`, as `feature-shipped-trace-drift` does) is referenced in
  `CHANGELOG.md` — by slug, by any trace REQ-id, by folder path, or via the
  iteration-suffix base slug — with a short, per-entry-justified rollup
  allowlist (`CHANGELOG_ROLLUP_ALLOWLIST`, the `KNOWN_FROZEN_DEAD_LINKS`
  pattern) for the ~26 features indexed under the CHANGELOG's documented
  thematic-rollup convention (the `v0…v5` ladder, the retired DL programme).
  Proven by a `feature-shipped-changelog-missing` `--self-test` fixture. This
  closes the drift class R3-4b found (the entire v2 tranche + the v3 close-out
  were absent from the canonical index until manually reconciled). This is an
  enforcement extension inside D1's philosophy, not a new normative decision —
  hence an amendment here, not a new ADR.

## Changelog
- 2026-07-09 (architect): initial accept. Ratifies the single-source-of-
  truth invariant (D1/D2), the pre-ship-only tester-terminal aliases (D3),
  the provenance-preserving flip pattern (D4), the `spec_lint.py`
  `feature-shipped-trace-drift` enforcement rule (D5), and the full-
  vocabulary legend (D6). Enforced by `scripts/spec_lint.py`; proven by
  `spec_lint.py --self-test`.
- 2026-07-10 (architect, remediation-plan P6a): amendment — a sibling
  mechanical enforcement of the D1 invariant on the OTHER derived index.
  Adds `spec_lint.py` category `feature-shipped-changelog-missing`
  (shipped feature ⇒ a `CHANGELOG.md` reference; slug / REQ-id / folder-path
  / iteration-suffix base slug match + a per-entry-justified rollup
  allowlist for the CHANGELOG's documented thematic-rollup convention) with
  its own `--self-test` fixture. Closes the R3-4b drift class (v2 tranche +
  v3 close-out were missing from the canonical index). Baseline measured at
  amendment time: 114 shipped features, 30 not verbatim-indexed (all under
  documented rollups — 4 auto-resolved by the iteration-suffix normalizer,
  26 allowlisted), 0 genuinely missing. See § Consequences.
