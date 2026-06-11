---
slug: dev-notes
status: in-progress
owner: spec-auditor
updated: 2026-05-31
---

# Spec Audit — 2026-05-31

## Headline

The spec tree is meaningfully healthier than the 2026-05-30 baseline: trace-broken-path violations collapsed from 53 to 7 (–46), and the missing-frontmatter and shipped-no-tests categories are completely cleared. The three SHOULD-FIX items from the prior audit are mostly resolved (anchors.toml header updated, MC-bootstrap trace path resolved). The single carry-over SHOULD-FIX is the MR `feature.md` status stale at `dev-done` while the trace correctly reads `tested`. Newly introduced: two new in-progress feature folders (`carry-strategy`, `carry-funding-data-backfill`) have no trace.toml rows yet and no backlog entries — expected for arch-in-progress, but worth noting. The dual-anchor system (ADR-0045 § D6 + § D7) is internally consistent: `verify_anchors.sh` passes 87/87, `check_determinism_anchors.py` passes 14/14 literals. No new decay-heavy folders.

---

## TL;DR Triage Table

| Sev | Item | File(s) | One-line fix |
|-----|------|---------|--------------|
| SHOULD-FIX | MR feature.md status stale | `spec/cross-sectional-mean-reversion-strategy/feature.md` | Flip `status: dev-done` → `status: tested`, `owner: developer → tester` → `owner: tester` |
| SHOULD-FIX | `11-regression-gate.md` narrative says "11 scenarios" (Phase 1A count) | `spec/architecture/11-regression-gate.md` line 62 | Update prose to current 87-anchor count; the "Current anchor set" table is also Phase-1A-vintage and now misleading |
| COSMETIC | `carry-strategy` and `carry-funding-data-backfill` have no trace.toml rows | `spec/trace.toml` | Add `[[req]]` rows; expected omission for arch-in-progress, but add when carry developer starts M-DEV |
| COSMETIC | `carry-strategy` and `carry-funding-data-backfill` are not in `spec/backlog.md` | `spec/backlog.md` | Append entries under Active or Queue; operator knows about them but the backlog is out of sync |
| COSMETIC | 3 pre-existing trace-broken-path: `REQ-VISUAL-FAIL-HTML-REPORTER-001` tests field contains prose paths, `REQ-UI-CONTRAST-ASSERTER-001` arch field points to archived path | `spec/trace.toml` | Pre-existing carry-over; fix when those features get a maintenance pass |
| COSMETIC | `REQ-LAB-YAHOO-REALDATA-V0-1-4-001` arch field: missing path to v0.1.2 per-ticker-scaling feature | `spec/trace.toml` | Pre-existing; that slug folder may have been renamed/consolidated |
| DEFERRED-KNOWN | ADRs 0045–0049 absent from `spec/architecture/adr/README.md` registry table (5-entry gap) | `spec/architecture/adr/README.md` | Carry-over from prior audits; ADRs 0050+0051 ARE registered and correct |
| DEFERRED-KNOWN | 87 dead-link violations (all pre-existing clusters) | Various | Unchanged from 2026-05-30; no new clusters introduced by today's churn |
| DEFERRED-KNOWN | `architecture.md` inline ADR table frozen at ADR-0026 (25 entries missing) | `spec/architecture.md` | Pre-existing carry-over; not impacted by today's churn |

---

## 1. Mechanical Lint

`scripts/spec_lint.py --all` executed (Python 3.11, project root `/Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading`).

**Result: FAIL (94 violations in 2 categories)**

| Category | This run | Previous (2026-05-30) | Delta |
|---|---|---|---|
| dead-link | 87 | 87 | 0 |
| missing-frontmatter | 0 | 3 | **–3 (RESOLVED)** |
| shipped-no-tests | 0 | 2 | **–2 (RESOLVED)** |
| trace-broken-path | 7 | 53 | **–46 (MAJOR IMPROVEMENT)** |

**Delta explanation:**

- **missing-frontmatter (–3):** All three prior violations cleared. `lab-polish-round-2/tasks.md`, `ui-test-harness-viewport-matrix/tasks.md`, and `v5-latency-slippage-sim-v0.5.0-square-root-market-impact/tasks.md` all had invalid status values in the 2026-05-30 audit. These are now valid frontmatter per spec-lint. This is the SHOULD-FIX-1 category from 2026-05-30 fully resolved.
- **shipped-no-tests (–2):** Both prior violations (`lab-end-to-end-v2`, `vol-killswitch-overlay-noop-fix`) are gone. Likely one of: reports/ directories were added, or the features' shipped status was revised so spec-lint no longer triggers the check. Either way, the category is clear.
- **trace-broken-path (–46):** The large prior count of 53 was dominated by prose-not-path entries in the trace's `tests` column for recently-added features (notably the MC bootstrap path-generator prose path that was flagged in 2026-05-30 SHOULD-FIX-3). Those are now resolved. The remaining 7 are all pre-existing carry-over items (see details below).
- **dead-link (0 delta):** Stable at 87. No new clusters. All are pre-existing: ADR-0027 Kronos slug links, `/tmp/` chart screenshot links, v0-paper-sma README stale paths, ADR-0039/0045 skill path links, cockpit-activity-status-bar screenshot artifact links.

**Remaining 7 trace-broken-path violations (all pre-existing carry-over):**
- `REQ-LAB-YAHOO-REALDATA-V0-1-4-001` arch field: `spec/lab-yahoo-realdata-v0.1.2-per-ticker-scaling-and-aggregate-cache-state/feature.md` — slug name mismatch (that folder exists under a slightly different name on disk).
- `REQ-VISUAL-FAIL-HTML-REPORTER-001` arch field: `spec/dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md` — archived path not resolvable.
- `REQ-VISUAL-FAIL-HTML-REPORTER-001` tests (2 paths): `emit_visual_fail_html_default_path_inlines_pngs`, `emit_visual_fail_html_spec_persist_writes_byte_identical_copy` — bare function names without file prefix.
- `REQ-UI-CONTRAST-ASSERTER-001` arch field: same archived `ui-testability-deep-dive-2026-05-15.md` path.
- `REQ-QUEUE-STALENESS-RECONCILIATION-001` tests: `scripts/queue_staleness_check.py --self-test` — script invocation, not a Rust test path.
- `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001` tests: `scripts/operator_ledger_check.py --self-test` — same.

**Sample dead-link violations (top 5 — all pre-existing):**
- `spec/architecture/adr/0027-kronos-onnx-tract-integration.md` — 5 links to `../../v25-kronos-forecast-overlay/feature.md` (slug no longer exists; archived).
- `spec/chart-canvas-overhaul/feature.md` — 7 links to `/tmp/orch-diag/*.png` (ephemeral screenshot paths).
- `spec/cockpit-activity-status-bar/presentations/cockpit-activity-status-bar-2026-05-26.md` — 4 links to `artifacts/.../*.png` (screenshots not committed).
- `spec/architecture/adr/0045-v5-canonical-config-and-noop-baseline-namespace.md` — link to `../../.claude/skills/spec-update/SKILL.md` (relative path from ADR subdir resolves incorrectly).
- `spec/v0-paper-sma/reports/screenshots/README.md` — 6 links to stale `../../features/` and `../../tasks/` paths.

### Anchor verification

`scripts/verify_anchors.sh` run live: **ANCHORS PASS (87 / 87)**

`scripts/check_determinism_anchors.py` run live: **OK — 14 literals match (8 canonical v5-realdata-medium-2026-05, 6 synthetic; 0 skipped: cfg-gated)**

The dual-anchor system (ADR-0045 § D6 + § D7) is internally consistent. The `anchors.toml` header comment reads `expects 87/87 PASS — 2026-05-31` — correctly updated from the stale 2026-05-27 value that was SHOULD-FIX-2 in the prior audit.

Anchor count trajectory (cumulative confirmed):
- 2026-05-29: 84 (v5 sqrt-impact wave)
- 2026-05-30: 85 → 86 (C3 momentum θ-surface added by the strategy-robustness-harness wave)
- 2026-05-31: 87 (MR θ-surface anchor #87 added by cross-sectional-mean-reversion-strategy)
- Next expected: 88 (carry θ-surface, after carry-strategy ships)

---

## 2. Prior Audit SHOULD-FIX Disposition

| Prior finding | Status |
|---|---|
| SHOULD-FIX-1: C1 `monte-carlo-bootstrap-path-generator` `feature.md` status stale (`dev-done` vs trace `tested`) | **PARTIALLY RESOLVED.** C1 (MC bootstrap path-generator) is now clean per spec-lint. However, the **new MR feature** (`cross-sectional-mean-reversion-strategy`) has the same pattern: `feature.md` reads `status: dev-done` / `owner: developer → tester` while the trace row `REQ-XS-MEANREVERSION-001` records `state: tested` (tester 2026-05-31). See finding SHOULD-FIX below. |
| SHOULD-FIX-2: `anchors.toml` main header comment stale (was `69/69 PASS — 2026-05-27`) | **RESOLVED.** Header now reads `expects 87/87 PASS — 2026-05-31`. |
| SHOULD-FIX-3: `REQ-MC-BOOTSTRAP-PATH-GENERATOR-001` tests column prose path | **RESOLVED.** Spec-lint no longer flags this row. The trace tests column was updated with concrete resolvable paths. |

---

## 3. New Feature Folder Audit (2026-05-31 churn)

### `spec/cross-sectional-mean-reversion-strategy/`

| Field | Value | Flag |
|---|---|---|
| feature.md status | `dev-done` | MISMATCH — trace says `tested` |
| feature.md owner | `developer → tester` | STALE — should be `tester` post-tester PASS |
| feature.md updated | `2026-05-31` | OK |
| trace.toml state | `tested` (tester 2026-05-31) | CORRECT |
| tasks.md present | YES | OK |
| reports/ present | YES (test report + θ-surface report) | OK |
| Backlog entry | NONE | COSMETIC OMISSION — feature is tested+complete; presenter step pending |
| Decay markers | 0 | OK |

**Finding (SHOULD-FIX):** `feature.md` was not advanced past `dev-done` after the tester PASS on 2026-05-31. The trace is authoritative and reads `tested`. Flip frontmatter to `status: tested`, `owner: tester`. This is the identical pattern as the C1 SHOULD-FIX-1 from the prior audit — it re-occurred on the immediately-following MR ship. Operator should consider whether the tester handoff checklist should enforce the frontmatter flip as a non-negotiable step.

### `spec/carry-strategy/`

| Field | Value | Flag |
|---|---|---|
| feature.md status | `arch-done` | OK — carry is in-flight developer; arch-in-progress correctly |
| feature.md owner | `architect → developer` | OK |
| tasks.md present | YES | OK |
| trace.toml row | NONE | COSMETIC — no `[[req]]` row yet; expected at arch-in-progress stage, but should be added before M-DEV starts |
| Backlog entry | NONE | COSMETIC — not in backlog.md despite being Active/Queue-class work |
| reports/ | NONE | OK — developer fills at M-DEV |
| Decay markers | 0 | OK |

**Context:** carry-strategy was flagged in the audit scope as arch-in-progress — the operator explicitly marked it "do NOT flag as drift." This is an inventory note only, not a drift finding.

### `spec/carry-funding-data-backfill/`

| Field | Value | Flag |
|---|---|---|
| feature.md status | `draft` | Plausible — spike feature; data was fetched and committed but no full tester review |
| feature.md owner | `developer` | OK |
| tasks.md present | YES | OK |
| trace.toml row | NONE | COSMETIC — same as carry-strategy |
| Backlog entry | NONE | COSMETIC — same |
| reports/ | NONE | No test report; feature.md § Tests section documents 14 unit tests inline | 
| Decay markers | 0 | OK |

**Finding (COSMETIC):** The `carry-funding-data-backfill` feature.md `status: draft` is plausible given it was a data-acquisition spike. However, if the data is committed and the 14 unit tests pass (`cargo test -p data --bin fetch_binance_funding`), the status should be `dev-done` (developer work complete) or `shipped` (if operator considers this shipped). The feature is the dependency blocker for carry-strategy, so its status matters for the carry-strategy task gating.

---

## 4. ADR Registration Sweep

### ADRs 0045–0051 (full sweep)

| ADR | On disk | In README table | Status |
|---|---|---|---|
| 0045 | YES | YES (line ~95) | Clean — D6.3 + D7.1b amendments registered atomically in README updated: 2026-05-31 |
| 0046 | YES | YES | Clean |
| 0047 | YES | YES | Clean |
| 0048 | YES | YES | Clean |
| 0049 | YES | YES | Clean |
| 0050 | YES | YES | Clean |
| 0051 | YES | YES — D6.5 (MR) + D6.6 (carry) amendments registered in README frontmatter updated: 2026-05-31 | Clean |

**Finding: All ADRs 0045–0051 are registered in the README table.** The 5-entry gap noted in the 2026-05-30 audit (ADRs 0045–0049 missing) is now resolved. The README frontmatter `updated:` field reads 2026-05-31 and records both the D6.5 and D6.6 amendments. This is a RESOLVED finding from prior audits.

### ADR-0051 § D6.5 and § D6.6 amendments

Both amendments are present in the ADR body and in the README registry entry:

- **§ D6.5** (MR family axis, 2026-05-31): present at ADR-0051 line 380. States that the strategy-family axis (Momentum vs Reversion) is varied at the config level, NOT the seed level, so it inherits D6.1 SAME-paths verbatim; 86 anchors hold by construction; +1 MR anchor (#87) under `mc-robustness-2026-06`. Cross-references to `cross-sectional-mean-reversion-strategy/feature.md` (file exists).
- **§ D6.6** (carry funding co-resampled series, 2026-05-31): present at ADR-0051 line 439. States that carry co-resamples a SECOND series (funding) under the SAME `idx_seq` as the returns (the `funding_at_return[s][idx_seq[k]]` gather). Zero new RNG draws; 87 anchors byte-identical by construction (additive `Option` fields + serde-default enum); +1 carry θ-surface anchor (#88 planned). Cross-references to `carry-strategy/feature.md` (file exists).

Both amendments are internally consistent with the corresponding feature.md files and with each other.

### Dual-anchor system (ADR-0045 § D6 + § D7)

- `spec/architecture/11-regression-gate.md` — the "Dual-anchor system" section (lines 15–56) correctly documents the two systems (System 1: file anchors via `verify_anchors.sh`; System 2: in-test re-run constants in `determinism.rs`; D7.1 drift-linter `check_determinism_anchors.py`; D7.2 release re-run gate). This section was added 2026-05-30 per the Changelog and is correct and current.
- **One staleness finding:** Lines 62–77 of `11-regression-gate.md` contain the "Current anchor set" section with a narrative that says `"At Phase 1A close, the set is 11 scenarios (the count grew from 9..."` and a table listing only 11 anchor scenarios. The actual current count is **87**. This section is a Phase-1A-vintage snapshot that was never updated as the anchor set grew. It is not a functional problem (the canonical count lives in `anchors.toml`) but it is misleading to a reader. **Finding (SHOULD-FIX):** update the narrative and table to reflect the current 87-anchor set, or replace the table with a pointer to `anchors.toml`.

---

## 5. trace.toml State vs Feature.md Alignment

Spot-check of 2026-05-31 churn rows and recently-tested features:

| Row ID | trace state | feature.md status | Aligned? |
|---|---|---|---|
| REQ-XS-MEANREVERSION-001 | `tested` | `dev-done` | **NO — MISMATCH** |
| (carry-strategy) | no row | `arch-done` | N/A — row expected at M-DEV |
| (carry-funding-data-backfill) | no row | `draft` | N/A — row expected before M-DEV |

**New drift introduced today:** The `cross-sectional-mean-reversion-strategy` `feature.md` was not advanced to `tested` after the tester PASS. This is the same pattern that was SHOULD-FIX-1 in the 2026-05-30 audit for `monte-carlo-bootstrap-path-generator`. It recurred on the very next tested feature.

---

## 6. Decay Markers

Grepped all `spec/` folders (excluding `spec/archive/` and `spec/dev-notes/`) for `TODO`, `FIXME`, `TBD`, `???`, `XXX`. No new decay-heavy folders introduced by today's churn.

| Folder | Count | Flag | Delta vs 2026-05-30 |
|---|---|---|---|
| lumen-design-adoption | 11 | DECAY-HEAVY | 0 |
| cockpit-performance-and-input-responsiveness | 6 | DECAY-HEAVY | 0 |
| operator-ledger-schema-lint | 4 | — | 0 (pre-existing; 1 in anchored test report) |
| v3-volatility-forecaster-noop-fix | 4 | — | 0 |
| ui-rethink-phase-b-lab-run | 4 | — | 0 |
| (others ≤3 each) | ≤3 ea | — | 0 |

No new decay-heavy folders. The `operator-ledger-schema-lint` count is 4 (3 in `feature.md` + 1 in `reports/test-20260530-070513-v0.1.0.md`). The test report marker is `TODO investigate` inside a ledger-entry table row — that report file is in the reports/ directory but spec-lint does not list it as anchored via `anchors.toml` (no operator-ledger anchor found), so it is not a byte-immutable concern, just a cosmetic quality gap.

---

## 7. Soft Contradictions (LLM-judged — verify before acting)

### 7.1 `carry-funding-data-backfill` status `draft` vs content completeness

**Topic:** Feature lifecycle state vs evidence of completion.
- `spec/carry-funding-data-backfill/feature.md` status = `draft` but the body describes fully completed work: `crates/data/src/bin/fetch_binance_funding.rs` built and registered, 14 unit tests passing, REVISION.toml committed, data fetched against live Binance API.
- The `spec/carry-strategy/feature.md` § preamble says the data is "already BANKED (committed `ab815d5`)."

**Assessment:** `draft` understates the actual state. The feature appears to be at least `dev-done` if not `shipped`. This matters because carry-strategy's task list (`tasks.md`) treats the backfill as a prerequisite that is already satisfied — if the backfill is still `draft`, a reader would incorrectly infer it may not be ready. Confidence: medium-high (the evidence of completion is explicit in the body text). Owner: developer (flip status to `dev-done` or `shipped` + add a reports/ note if any test run was done).

### 7.2 `11-regression-gate.md` anchor table vs actual anchor count

**Topic:** Dual-anchor documentation accuracy.
- `spec/architecture/11-regression-gate.md` § "Current anchor set" (line 62+) says the set is "11 scenarios" and lists a Phase-1A-vintage table.
- `spec/anchors.toml` header says `87/87 PASS — 2026-05-31` and `verify_anchors.sh` confirms 87 anchors.

**Assessment:** Clear documentation staleness — not a functional contradiction (the anchors.toml is the canonical source), but the regression-gate doc's "Current anchor set" section actively misleads any reader who looks there for the count. Confidence: high. Owner: developer (update the doc; or add a `> Note: this table reflects Phase-1A only — see anchors.toml for the live count` disclaimer at minimum).

### 7.3 Carry-strategy `feature.md` open questions Q-CARRY-1 through Q-CARRY-5 vs ADR-0051 § D6.6

**Topic:** Are all architect Q-CARRY open questions resolved in the ADR?
- `spec/carry-strategy/feature.md` § Design states "Q-CARRY-1..5 are all resolved + justified below." The design section marks Q-CARRY-1 (funding seam), Q-CARRY-2 (long-only framing), Q-CARRY-3 (bootstrap crux — TRACTABLE), Q-CARRY-4 (raw funding, no vol-norm), Q-CARRY-5 (θ-grid locked) all RESOLVED.
- `spec/architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md` § D6.6 covers the determinism and anchoring contract for carry, confirming Q-CARRY-3 (the crux) is tractable and zero new RNG draws are needed.

**Assessment:** The two documents are consistent — the feature.md resolutions and the ADR amendment agree on the mechanism (shared-index gather, additive `Option` fields, `ScoreSource` serde-default enum). Confidence: medium (read from the feature.md summaries; full code-level verification requires a developer). No contradiction found.

### 7.4 `carry-strategy` claims anchor count 87→88 but `anchors.toml` still at 87

**Topic:** The carry ADR-0051 § D6.6 says "+1 carry θ-surface anchor under `mc-robustness-2026-06` (87→88)". The anchors.toml is at 87. 
- This is expected and correct: the +1 anchor is planned but the carry build hasn't shipped yet. The tester locks the anchor after the dev's anchored run (same pattern as MR at arch-done stage).
- The feature.md and ADR both state this correctly: "No anchor in `spec/anchors.toml` is added by the architect (the tester locks…)."

**Assessment: no contradiction.** The 87→88 statement is a forward projection, correctly qualified. This is the same clean state C2 was in at arch-done when MR was not yet anchored.

---

## 8. Anchor Coverage Sweep

`spec/anchors.toml` — **87 locked scenarios, 87/87 PASS confirmed live.**

### Shipped strategy features vs anchor coverage

New shipped strategy since last audit:
- `cross-sectional-mean-reversion-strategy` (tested, not yet presenter-done) — anchor #87 `v1-mr-theta-surface-2023-block-bootstrap-real-fy` is present and passes. Coverage: COMPLETE.

In-progress strategy since last audit:
- `carry-strategy` — no anchor yet; expected after dev ships anchored run. ADR-0051 § D6.6.4 specifies anchor #88 namespace `mc-robustness-2026-06`, scenario `v1-carry-theta-surface-2023-block-bootstrap-real-fy`. CORRECTLY absent (arch-in-progress).

Pre-existing anchor gaps (carry-over, unchanged):
- `v2-llm-strategy` — shipped, zero anchors, no disposition note. Carry-over P2.
- `v3-llm-forecaster` — shipped-partial, zero anchors (API-key blocked). By-design per feature.md; carry-over.

### mc-robustness-2026-06 namespace

Contains: anchor #85 (`v1-momentum-2023-block-bootstrap-real-fy-mc`), anchor #86 (`v1-momentum-theta-surface-2023-block-bootstrap-real-fy`), anchor #87 (`v1-mr-theta-surface-2023-block-bootstrap-real-fy`). Anchor #88 (carry θ-surface) is not yet present — correct at arch-done stage.

`scripts/verify_anchors.sh` searches three directories for the `mc-robustness-2026-06` namespace: `spec/strategy-robustness-harness/reports/`, `spec/momentum-parameter-robustness-sweep/reports/`, and `spec/cross-sectional-mean-reversion-strategy/reports/`. The carry feature's `reports/` dir will need to be added when the carry θ-surface is anchored — exactly as noted in ADR-0051 § D6.6.4. This is a forward-looking to-do for the developer.

---

## 9. Orphan Check

### Feature folders missing feature.md or tasks.md (carry-overs)

- `spec/cockpit-app-bundle/` — missing `tasks.md` (pre-existing; candidate status; expected pre-developer omission).
- `spec/lumen-design-adoption/` — missing `tasks.md` (pre-existing; roadmap umbrella; expected).

No new orphan folders introduced by today's churn.

### Test reports for missing slugs

None found.

### Carry-strategy and carry-funding-data-backfill — trace/backlog orphan state

Both folders have `feature.md` + `tasks.md` (complete folder structure). They are orphans from the trace.toml and backlog.md perspective only. This is expected for arch-in-progress features per the standard workflow (the architect's M-T1 owns these until M-DEV handoff, at which point the orchestrator typically adds the trace row and backlog entry). Noted as COSMETIC, not a gap.

---

## 10. Recommended Triage

**SHOULD-FIX (action this week):**

- **[SHOULD-FIX-1] MR `feature.md` status stale — trivial one-line flip.**
  `spec/cross-sectional-mean-reversion-strategy/feature.md` frontmatter reads `status: dev-done, owner: developer → tester` but the trace correctly records `state: tested` (tester PASS 2026-05-31). Flip to `status: tested, owner: tester`. Identical pattern to SHOULD-FIX-1 from 2026-05-30 on the C1 MC bootstrap feature — it recurred on the next tested feature. Consider adding frontmatter flip to the tester's PASS checklist as a non-negotiable step to prevent recurrence. Owner: orchestrator.

- **[SHOULD-FIX-2] `11-regression-gate.md` § "Current anchor set" section is Phase-1A-vintage (11 scenarios) while actual count is 87.**
  Lines 62–77 of `spec/architecture/11-regression-gate.md` state "At Phase 1A close, the set is 11 scenarios" and list a 11-row table. Current count is 87 (confirmed live). The section misleads any reader looking for the current anchor inventory. Update the narrative and replace the table with a pointer to `anchors.toml`, or prepend a `> Note: table reflects Phase-1A snapshot only; see spec/anchors.toml for the live count` caveat. Owner: developer / architect.

**COSMETIC (clean up opportunistically):**

- **[COSMETIC-1] `carry-funding-data-backfill` status `draft` vs completed evidence.**
  The feature body describes completed build + 14 passing unit tests + committed data. `draft` understates completion. Flip to `dev-done` (or `shipped` if the operator considers it done). Owner: developer.

- **[COSMETIC-2] `carry-strategy` and `carry-funding-data-backfill` not in `trace.toml`.**
  Both folders have no `[[req]]` row. Expected for arch-in-progress, but add before M-DEV starts so spec-lint can track them. Owner: orchestrator (add row at M-DEV handoff).

- **[COSMETIC-3] `carry-strategy` and `carry-funding-data-backfill` not in `backlog.md`.**
  Neither appears as an Active or Queue entry in the backlog. The operator knows about them, but backlog readers cannot see them. Owner: orchestrator.

- **[COSMETIC-4] 4 pre-existing trace-broken-path violations in `REQ-VISUAL-FAIL-HTML-REPORTER-001` and `REQ-UI-CONTRAST-ASSERTER-001` (archived doc path + bare function names).**
  Carry-over; address when those features get a maintenance pass. Owner: developer (whichever agent last touched those features).

- **[COSMETIC-5] ADR README table ends at ADR-0051 — all registered, but the `architecture.md` inline ADR summary table still frozen at ADR-0026.**
  25 entries missing from `spec/architecture.md` inline table (ADRs 0027–0051). Pre-existing carry-over. Owner: architect.

**DEFERRED-KNOWN (no action needed — pre-existing or operator-deferred):**

- 87 dead-link violations: all pre-existing clusters. Zero new clusters from today's churn.
- 7 trace-broken-path violations: all pre-existing carry-over (archived doc paths, bare function names, Python script invocations).
- `v2-llm-strategy` and `v3-llm-forecaster` anchor gaps: by-design (API-key blocked); unchanged.
- `spec/_probe_lint_test/` orphan folder: pre-existing, >18 days old.
- `lumen-design-adoption` (11 decay markers) and `cockpit-performance-and-input-responsiveness` (6 markers): DECAY-HEAVY carry-overs, unchanged.
- ADRs 0045–0049 were missing from the README in the 2026-05-30 audit. They are NOW registered — this is a resolved finding, not a carry-over.
- `product.md` Introduction vs Pillar stack LLM demotion tension: noted in 2026-05-30 audit as COSMETIC-6; unchanged.

---

## Changelog

- 2026-05-31 (spec-auditor): On-demand audit triggered by heavy churn day: MR (`cross-sectional-mean-reversion-strategy`) shipped tested (FAMILY-UNIFORM-FRAGILE, anchor #87), carry architect M-T1 in progress (`carry-strategy` arch-done + `carry-funding-data-backfill` data-banked), ADR-0051 § D6.5 + § D6.6 amendments landed, `11-regression-gate.md` dual-anchor section added, anchors.toml header updated to 87/87. Mechanical lint: 94 violations (87 dead-link + 7 trace-broken-path); major improvement vs 2026-05-30 (missing-frontmatter –3, shipped-no-tests –2, trace-broken-path –46). Anchors 87/87 PASS + check_determinism_anchors.py 14/14 PASS (dual-anchor system internally consistent). ADRs 0045–0051 all registered in README (prior 5-entry gap resolved). C1/C2/C3 carry-overs clean. MR feature.md status/owner stale (SHOULD-FIX-1; trace authoritative: state=tested). 11-regression-gate.md "Current anchor set" Phase-1A vintage (SHOULD-FIX-2). Carry folders arch-in-progress (no trace/backlog rows — COSMETIC, expected). No new decay-heavy folders.
