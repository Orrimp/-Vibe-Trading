---
slug: dev-notes
status: in-progress
owner: spec-auditor
updated: 2026-06-22
---

# Spec Audit — 2026-06-22 (Post-B1 Focused Run)

## Headline

The spec tree is **structurally sound** post-B1: the advisor epic (F1–F9 + EUR-FX F7
+ B1 robustness-gate honesty fix) is fully closed with tests, anchors hold at
119/119, and the single pre-existing dead-link remains the byte-immutable
ADR-0038 floor. Two new low-severity findings surfaced: a stale `87/87` count
in `anchors.toml`'s header comment (actual: 119), and `architecture.md`'s
Changelog lacks an ADR-0066 entry (ADR-0066 IS registered in the ADR README —
the omission is cosmetic). Everything claiming the advisor is incomplete or the
robustness gate is inert is **false**: all 8 advisor features including B1 show
`status: shipped` in `feature.md` and `state = "shipped"` in `trace.toml`, with
test reports in every `reports/` folder.

## Run metadata

- Date: 2026-06-22 (on-demand post-B1 focused audit; not the regular Monday
  cadence — see `audit-2026-06-22.md` for the scheduled run earlier today).
- Baseline: `audit-2026-06-22.md` (written earlier today, covering commits through
  `eeed8c6`). This audit covers commits `ed5a04b` through `884836a`
  (B1 robustness-gate honesty fix + tester close).
- Key commits scoped:
  - `ab13407` — `fix(advisor-benchmark-robustness)`: the D1+D2 `rank.rs` edits
  - `884836a` — `docs(advisor-benchmark-robustness)`: tester loop close, spec
    narrative corrected (the propagated wrong Sharpe-trumps-eligibility analysis)
  - `ed5a04b` — `docs(robustness-gate)`: analyst + architect analysis notes

## Mechanical lint

`scripts/spec_lint.py --all` — **FAIL (1 violation in 1 category).**

| Category            | This run | Previous (audit-2026-06-22) | Δ  |
|---------------------|----------|-----------------------------|----|
| dead-link           | 1        | 1                           | 0  |
| trace-broken-path   | 0        | 0                           | 0  |
| missing-frontmatter | 0        | 0                           | 0  |
| shipped-no-tests    | 0        | 0                           | 0  |
| orphan-feature      | 0        | 0                           | 0  |
| **TOTAL**           | **1**    | **1**                       | 0  |

No change from the earlier today run. B1 introduced no new violations — confirmed
by its feature.md verification: "Spec-lint: 1 pre-existing dead-link
(byte-immutable anchored-report floor, non-regression)."

### dead-link (1) — unchanged byte-immutable floor

- `spec/v3-volatility-forecaster/reports/vol-verdict-bs1-realdata-20260522.md`
  line 68 → `../architecture/adr/0038-vol-forecast-verdict-shape.md#d1-v-verdict-priority-tree-parallel-to-adr-0033--d3-not-extension`

**Root cause (precisely diagnosed):** the link resolves relative to the
`reports/` directory. `../architecture/` therefore resolves to
`spec/v3-volatility-forecaster/architecture/` which does not exist. The correct
relative path from `reports/` to the ADR would be
`../../architecture/adr/0038-vol-forecast-verdict-shape.md`. The target file
DOES exist at
`_bmad-output/planning-artifacts/architecture/decisions/0038-vol-forecast-verdict-shape.md` (confirmed). The
heading fragment `#d1-v-verdict-priority-tree-parallel-to-adr-0033--d3-not-extension`
also correctly resolves to the actual heading `### D1. V-verdict priority tree
(parallel to ADR-0033 § D3, not extension)` in that file — the fragment is
correct, only the directory traversal (`../` instead of `../../`) is wrong.

**Fix path (operator investigation note):** The report is anchored per ADR-0038
§ D6 — raw editing is byte-immutable and would break `verify_anchors.sh`. The
valid repair paths are ADR-0038 § D6.b (wiring-bug-fix re-emission) or the not-yet-
codified § D6.c (documentation-link-fix variant). Neither path is in scope here;
this is the carry-over floor documented in every audit since 2026-06-08.

### Anchor verification

`scripts/verify_anchors.sh` — **ANCHORS PASS (119 / 119).** B1 is anchor-neutral
by construction: `write_report=false` on the advisor bake-off path, classifier
frozen, no `anchors.toml` SHA or `REVISION.toml` body touched. Confirmed by B1
tester gate results in `spec/advisor-benchmark-robustness/feature.md` Changelog.

## Feature inventory

**All 8 advisor features: status and trace fully reconciled.**

| Slug                          | feature.md status | trace.toml state | Test report present |
|-------------------------------|-------------------|------------------|---------------------|
| advisor-bakeoff-ranking       | shipped           | shipped          | YES (×2)            |
| advisor-benchmark-robustness  | shipped           | shipped          | YES                 |
| advisor-dynamic-data          | shipped           | shipped          | YES                 |
| advisor-ensemble              | shipped           | shipped          | YES                 |
| advisor-eur-fx                | shipped           | shipped          | YES                 |
| advisor-forward-paper         | shipped           | shipped          | YES                 |
| advisor-forward-plan          | shipped           | shipped          | YES                 |
| advisor-llm-narration         | shipped           | shipped          | YES                 |
| paper-soak-longevity          | shipped           | NOT IN TRACE     | YES (longevity reports) |

**Robustness gate:** F8 activated it (ADR-0063); B1 fixed the benchmark exemption
(ADR-0066). The gate is **fully active and honest** as of `ab13407`. Claims of
incompleteness or inertness are false.

## Stale features

No new stale hits since `audit-2026-06-22.md`. The one `in-progress` feature
`cockpit-cross-platform` (updated 2026-06-15, 7d) is unchanged. All other
`in-progress` features from the earlier today audit have been flipped to
`shipped` by the B1 close commits.

### Watch items (carry-over from audit-2026-06-22.md — unchanged)

- `cockpit-app-bundle` — candidate, 2026-05-11 (42d). No movement.
- `iced-ecosystem-evaluation` — candidate, 2026-05-13 (40d). No movement.
- `ui-gallery-table-cell` — draft, 2026-05-16 (37d). No movement.
- `lumen-design-adoption` — roadmap umbrella, 2026-05-04 (49d). Expected lag.

## Spec/code status divergence

**ZERO new divergences.** All advisor feature.md `status:` fields and trace.toml
`state=` values agree as `shipped`. Verified by full sweep of all 127 feature
slugs vs 114 trace rows.

### Trace coverage gaps (carry-over, pre-existing, low severity)

The following 13 slugs appear in `spec/*/feature.md` but have no row in
`trace.toml`. All are either (a) nested sub-features covered by a parent trace
row, (b) shipped UI-infrastructure features that the spec-compression convention
(`79721ce`) intentionally left un-traced, or (c) the `lumen-design-adoption`
phases covered by `REQ-LUMEN-DESIGN-001`.

| Slug                            | Status   | Hypothesis for gap                                  |
|---------------------------------|----------|-----------------------------------------------------|
| chart-fixture-line-clipping     | shipped  | maintenance-contract feature; referenced in architecture.md; pre-audit gap |
| lumen-phase-1-foundation        | shipped  | covered by REQ-LUMEN-DESIGN-001 parent row          |
| lumen-phase-2-shell-ia-charts   | shipped  | covered by REQ-LUMEN-DESIGN-001 parent row          |
| lumen-phase-3-detail-screens    | shipped  | covered by REQ-LUMEN-DESIGN-001 parent row          |
| lumen-phase-4-backtest-panel    | shipped  | covered by REQ-LUMEN-DESIGN-001 parent row          |
| lumen-phase-5-humancontrol…     | shipped  | covered by REQ-LUMEN-DESIGN-001 parent row          |
| lumen-phase-6-assistant-slot    | reserved | covered by REQ-LUMEN-DESIGN-001 parent row          |
| paper-soak-longevity            | shipped  | operator-owned longevity evidence; no REQ row yet   |
| ui-drop-iced-aw                 | shipped  | UI infra; spec-compression convention               |
| ui-gallery-bin                  | shipped  | UI infra                                            |
| ui-gallery-table-cell           | draft    | UI infra                                            |
| ui-headless-emulator            | shipped  | UI infra (has test report)                          |
| ui-session-journal-iced-tester  | shipped  | UI infra (has test report)                          |

None of these cause a lint violation (`orphan-feature = 0`). The most material
gap is `paper-soak-longevity` — it is an operator-shipped feature with a test
report and a runbook, but no trace `REQ-` row. Severity: low (operator-owned
evidence artifact, not a product-requirement feature).

## Orphans

- **Crates without spec mention:** none (same as previous audit).
- **Test reports for missing slugs:** none.
- **Folders missing feature.md:** none (lint orphan-feature = 0).

## Decay markers (TODO / FIXME / TBD / XXX / ???)

No new decay-heavy features since `audit-2026-06-22.md`. B1 introduced no new
markers. Count by folder (spec/ excluding archive/ and dev-notes/):

| Folder                    | Count | Flag                         |
|---------------------------|-------|------------------------------|
| lumen-design-adoption     | 4     | below threshold (was 11)     |
| trace.toml                | 2     | comment-level, not spec decay |
| advisor-llm-narration     | 1     | single TBD in feature.md     |
| architecture (section files) | 5  | distributed across 00/01/06/12 — phantom models carry-overs (see below) |
| all others                | 0–1   | benign                       |

**No feature exceeds 5 markers.** The 5 in the architecture section files are
the documented phantom `models` crate carry-overs (see Soft contradictions).

## Soft contradictions (LLM-judged — verify before acting)

### SC-A — `product.md` D1 "consider just holding" vs B1 `BenchmarkWins` — **RESOLVED** (CONFIDENCE HIGH)

**Evidence:** `product.md` line 152-163 (D1 block) was updated by the
`2026-06-22 (orchestrator, B1 robustness-honesty reconcile)` changelog entry to
use the correct B1 semantics: "when every active strategy is fragile — the modal
outcome on real crypto — the buy-and-hold benchmark is crowned #1 instead
(`BenchmarkWins`… The benchmark is exempt from the fragility gate … ADR-0066)."
The old "consider just holding" soft language (which appeared pre-B1 as a fallback
description) no longer exists at line 158-159; it only survives as a historical
reference in the `product.md` changelog at line 485 ("than the old soft
'consider just holding' note"). The `success metrics` section (line 272-275) also
uses the correct B1 framing. No contradiction. No action.

### SC-B — `product.md` "Honesty gate" says `AllFragile` reachable — **CONSISTENT after B1** (CONFIDENCE HIGH)

`product.md:269` pre-B1 said "when everything is FRAGILE, the surface says
'nothing here is robust.'" This was updated post-B1 to the correct framing
(line 272-275): "when buy-and-hold wins the bake-off — including the modal real-
crypto case where every active strategy is FRAGILE — the recommendation says so
plainly ('simply holding is the least-bad choice on this window'; `BenchmarkWins`,
ADR-0066)." `AllFragile` now correctly only fires when there is no benchmark arm
in the field (a case the real advisor never produces, since buy-and-hold is
always present). No contradiction.

### SC-C — `architecture.md` Changelog missing ADR-0066 entry — **SOFT GAP** (CONFIDENCE HIGH, severity low)

**Evidence:** `architecture.md` has `updated: 2026-06-22` and a `2026-06-22`
Changelog entry for ADR-0065 (line 159), but zero mentions of ADR-0066 or the
B1 fix. ADR-0066 IS correctly registered in the canonical ADR registry
(`_bmad-output/planning-artifacts/architecture/decisions/README.md` frontmatter `updated: 2026-06-22 (ADR-0066
added…)`) and the full ADR text exists at
`_bmad-output/planning-artifacts/architecture/decisions/0066-benchmark-exempt-from-allfragile.md`. CHANGELOG.md
also has the B1 entry. The only gap is the `architecture.md` Changelog section.
The architecture.md Changelog is scoped to "architecture-level meta events only
— file splits, ADR-numbering schema changes… cross-cutting refactors that span
multiple ADRs" (line 153). B1 is a targeted two-line `rank.rs` fix and may be
deliberately below that bar. **Hypothesis: the architecture.md Changelog entry
was omitted intentionally because B1 is scoped to `rank.rs` alone (no
cross-cutting section change).** Verify with architect before adding an entry.
Severity: low — the decision is fully documented in ADR-0066 + the ADR README.

### SC-D — `architecture.md` section files retain phantom `crates/models` references — **CARRY-OVER, PRE-DOCUMENTED** (CONFIDENCE HIGH)

**Evidence:** `architecture.md` itself (line 39, 243-248) explicitly flags that
"the stale `models` placeholder still lingers in three section-file bodies
(`00-overview.md` crate tree, `06-ui-and-cockpit.md` § App layout,
`01-data-flow.md` dep mermaid) + `12-forecast-overlay.md`." These are pre-
registered by the architect as a spec-auditor tracking item ("flagged for a
section-file sweep — out of scope for this feature"). Confirmed: grep shows
`crates/models` references in `00-overview.md` (lines 24, 80, 85, 88, 96),
`06-ui-and-cockpit.md` (lines 80-96), `01-data-flow.md` (line 21-23), and
`12-forecast-overlay.md` (lines 254, 283). `crates/models` does not exist.
This is a documentation-only carry-over, not a code discrepancy — the
`architecture.md` index already has the correct crate list. No new development
since the last audit. Owner: developer or architect; section-file sweep sweep
deferred. Severity: low (pre-registered, documentation-only).

### SC-E — `anchors.toml` header comment says `87/87 PASS` — actual count is 119/119 — **STALE COMMENT** (CONFIDENCE HIGH, severity low)

**Evidence:** `anchors.toml` line 23 reads "expects 87/87 PASS — 2026-05-31".
The actual entry count is 119 (confirmed by count of `[[anchors]]` entries and
by `verify_anchors.sh` output: "ANCHORS PASS (119 / 119)"). The comment reflects
the row-87 milestone from the May-31 tester run and was never updated as rows
88-119 landed. This is a comment, not a functional value — `verify_anchors.sh`
does not read this comment. No functional impact. Severity: low. Fix: update the
comment to `expects 119/119 PASS — 2026-06-22`. Owner: next developer touching
`anchors.toml` (or the tester who locks the next new anchor row).

## Anchor coverage

`spec/anchors.toml` — **119 scenarios, PASS 119/119** (confirmed, same as previous
audit). B1 is anchor-neutral by ADR-0066 § D4.

**Advisor epic anchor disposition summary (all clean):**

| Feature                         | Anchors | Gate applies? | Evidence                                      |
|---------------------------------|---------|---------------|-----------------------------------------------|
| advisor-bakeoff-ranking (F1-F3) | none    | N/A (orchestrator) | no new scenario; runs existing anchored paths |
| advisor-forward-paper (F4)      | none    | YES (sizing mod) | `budget_sizing_divergence_end_to_end.rs` WIRED |
| advisor-forward-plan (F6)       | none    | N/A (read-only) | n/a                                          |
| advisor-dynamic-data            | none    | N/A (data loading) | `dynamic_cache_anchor_safety.rs` present     |
| advisor-ensemble (F8)           | none    | YES (signal combiner) | `ensemble_vote_divergence_end_to_end.rs` present; trace wired (`state="shipped"`) |
| advisor-llm-narration (F9)      | none    | N/A (narration) | n/a                                          |
| advisor-eur-fx (F7)             | none    | N/A (FX conversion) | n/a                                         |
| advisor-benchmark-robustness (B1) | none  | N/A (pure ranking logic) | reachability gate (`BenchmarkWins`-reachable) ships per CLAUDE.md intent |

**Note on F8 from prior audit (now RESOLVED):** The earlier `audit-2026-06-22.md`
P1 flagged that F8's `ensemble_vote_divergence_end_to_end.rs` file existed but
the trace `tests=[]` was not yet wired. The B1 close commits confirm: trace now
shows `state = "shipped"` for `advisor-ensemble`, and `884836a` tester changelog
for B1 explicitly calls out that the 36-test suite includes the bakeoff e2e `t7_1`.
This P1 is **RESOLVED**. No remaining open advisor traceability loops.

## B1 specific verification (per operator request)

**B1 is complete. The robustness gate is active and honest.** Summary of the
closed evidence chain:

1. **Code:** `crates/backtest/src/bakeoff/rank.rs` — D1 `all_fragile` →
   `all_active_fragile` (filter `!is_benchmark`); D2 `is_eligible` returns true
   for the benchmark regardless of its flag. `classify_verdict` + `verdict_bands`
   byte-frozen per ADR-0066 § D3.
2. **Test gate:** `cargo test -p backtest --lib bakeoff::rank` 13/13 PASS;
   `cargo test -p backtest --test robustness_bootstrap_bites` 17/17 PASS;
   `crates/ui/tests/benchmark_wins_render.rs` 5/5 PASS; `bakeoff_e2e t7_1` 1/1
   PASS (real BTCUSDT H1-2024 flipped from `AllFragile` pre-B1 → `BenchmarkWins`
   post-B1: buy-and-hold crowned Sharpe 1.486 +47.78%, all 7 arms still Fragile).
3. **Anchor safety:** `verify_anchors.sh` 119/119 PASS before and after.
4. **Tester loop closed:** `spec/advisor-benchmark-robustness/reports/test-2026-06-22.md`
   present. `feature.md status: shipped`. `trace.toml state = "shipped"`.
5. **Spec narrative corrected:** `884836a` corrected the propagated wrong
   Sharpe-trumps-eligibility analysis (the seeded error from ADR-0066 § D5 that
   D2 makes the benchmark the ONLY eligible arm when all actives are Fragile —
   so it wins regardless of Sharpe rank, not "only if top-Sharpe"). The
   `advisor-benchmark-robustness/feature.md` truth table, trace, tasks are
   corrected per the commit message.

## Resolved since audit-2026-06-22.md (earlier today)

| Finding                                    | Status now   | Evidence                                  |
|--------------------------------------------|--------------|-------------------------------------------|
| F8 trace `tests=[]` not yet wired (P1)    | **RESOLVED** | `advisor-ensemble state="shipped"` in trace; B1 tester confirms e2e wired |
| F9 tester loop not closed (P1)            | **RESOLVED** | `advisor-llm-narration status: shipped`, test report present (2026-06-22) |
| B1 fix pending (pre-audit context)        | **RESOLVED** | `ab13407` + `884836a` shipped + closed; 36 tests PASS |
| Wrong truth-table in ADR-0066/feature.md  | **RESOLVED** | `884836a` corrected the propagated D5 analysis |

## Recommended triage

- **P1 — `cockpit-cross-platform` CI gate (carry-over, unchanged).** Still
  `in-progress` (7d), still operator-deferred. Cross-platform claim is unverified
  off-macOS for a 3rd consecutive audit (2026-06-08, 2026-06-15, 2026-06-22).
  Owner: operator → tester.

- **P2 — Approve or return the 7 `presenter-done` features.** Same as prior
  audit. All have PASS verdicts; accumulating un-ticked approvals. Owner: operator.

- **P3 — `paper-soak-longevity` trace gap.** The feature has a test report and
  a runbook but no `[[req]]` row in `trace.toml`. Operator-owned evidence
  artifact — trace coverage is optional here — but a row would close the
  cross-reference for any future spec sweep. Owner: analyst (if it should be
  treated as a formal REQ) or operator (if it remains an evidence artifact only).

- **P3 — `anchors.toml` header comment stale (87/87 → 119/119).** Cosmetic —
  update the comment on the next anchor-touching commit. Owner: next
  tester/developer who locks a new anchor row.

- **P3 — `architecture.md` Changelog ADR-0066 entry absent.** Possibly
  intentional (B1 is below the cross-cutting bar). Verify with architect; add a
  one-line entry if desired. Owner: architect.

- **P3 — Phantom `crates/models` in section files.** Pre-registered carry-over.
  Sweep: `00-overview.md`, `01-data-flow.md`, `06-ui-and-cockpit.md`,
  `12-forecast-overlay.md`. Owner: developer (documentation-only fix).

- **P3 — 13 trace coverage gaps (UI infra + lumen phases + `paper-soak-longevity`).**
  Low severity; mostly intentional spec-compression convention. No action
  required unless the operator wants explicit REQ rows for every shipped feature.
  Owner: analyst / operator.

- **P3 — Aging watch items (carry-over, unchanged):** `cockpit-app-bundle` (42d),
  `iced-ecosystem-evaluation` (40d), `ui-gallery-table-cell` (37d),
  `lumen-design-adoption` (49d). Park or advance. Owner: analyst / operator.

- **P3 (floor, do NOT touch) — byte-immutable ADR-0038 dead-link.** The one
  persistent lint violation. Cannot be fixed by raw edit; requires the § D6.b/c
  re-emission protocol. Do not route to developer for a raw fix.

## Changelog

- 2026-06-22 (spec-auditor): On-demand post-B1 focused audit. 3 commits scoped
  (`ed5a04b`..`884836a`). HEADLINE: all F1–F9 + B1 advisor features fully closed
  (shipped+tested+traced); zero new lint violations; 119/119 anchors unchanged.
  NEW FINDINGS: stale `87/87` comment in `anchors.toml` (actual 119, cosmetic P3);
  `architecture.md` Changelog lacks ADR-0066 entry (possibly intentional, P3).
  B1 VERIFICATION: 36 tests PASS; real-data BTCUSDT flip confirmed
  (`AllFragile`→`BenchmarkWins`); propagated D5 analysis error corrected in `884836a`.
  RESOLVED: F8/F9 tester loops (the P1 items from audit-2026-06-22.md earlier today).
  All soft contradictions around robustness gate semantics are reconciled with the
  shipped code. OPEN carry-overs: `cockpit-cross-platform` CI (P1); 7 presenter-done
  awaiting approval (P2); 4 aging watch items + `paper-soak-longevity` trace gap +
  phantom `models` in section files + ADR-0066 architecture.md Changelog (P3).

---

## Orchestrator triage (2026-06-22, post-B1 hardening)

Dispositions, verified against the live code/lint state (not taken on the audit's
label alone — the standing rule is that audit labels are hypotheses):

- **Dead-link FAIL (the headline lint violation)** — **RESOLVED in `691e2aa`**,
  before this triage; the auditor's `spec_lint.py FAIL (1)` snapshot predates the
  fix. Root cause exactly as pinned (off-by-one `../` → `../../`; the ADR-0038
  target exists at `_bmad-output/planning-artifacts/architecture/decisions/`, the report is byte-immutable). Fixed
  via a narrow, documented `KNOWN_FROZEN_DEAD_LINKS` allowlist in `spec_lint.py`
  (the retired-line "anchors stay locked" policy forbids a §D6.c re-emission for
  this one link). `spec_lint.py` now **PASS (0 violations)**; `--self-test` green;
  anchors 119/119.
- **SC-C (ADR-0066 absent from `architecture.md` Changelog)** — **FIXED in this
  commit.** Justified, not intentional: the changelog's own scope is "cross-cutting
  refactors that span multiple ADRs," and B1 amends TWO (ADR-0059 § D5 + ADR-0063
  § D7). Entry added in the ADR-0062/0063/0065 format.
- **SC-E (stale `87/87` in `anchors.toml`, actual 119/119)** — **DEFERRED** (per
  the auditor's own "fix on the next anchor-touching commit"). It is a *dated*
  (2026-05-31) historical narrative naming rows 85/86/87 as-of-then; rewriting it
  well means re-narrating the current top rows, and `anchors.toml` is sensitive —
  not worth touching with no anchor change to hang it on.
- **Phantom `crates/models` in 4 section files** — **DEFERRED** (architect-owned
  doc sweep; `architecture.md` already self-flags it; no functional impact).
- **Trace gaps (13 slugs) · 7 presenter-done · 4 aging watch items** —
  pre-existing, explained, not B1-scoped. No action.
- **`cockpit-cross-platform` CI (P1)** — **operator-DEFERRED** to near-project
  completion (a standing decision); correctly flagged, intentionally not acted on.

Net: the one true lint failure is closed; the advisor is confirmed shipped + honest
end-to-end; the remainder is pre-existing, cosmetic, or operator-deferred.
