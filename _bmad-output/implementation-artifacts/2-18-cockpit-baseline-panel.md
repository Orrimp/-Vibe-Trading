# Story 2.18: cockpit-baseline-panel

Status: review

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the Baseline panel surfacing the shipped passive buy-and-hold result,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the Baseline panel surfacing the shipped passive buy-and-hold result.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

### Review Findings

<!-- bmad-code-review 2026-08-12 (burn-down 12 of 14; commits f1c1bf3 + 8dbe6ae, 1,835-line diff; layers: Blind 15, Edge 15, Auditor 7 raw — 37 deduped to 21). Second UI story; AD-10 governs.
     VERDICT: PASS WITH FINDINGS. The code delivered all 7 requirements and all 10 tasks; the VERIFICATION RECORD did not, in two independent ways, and three displayed numbers were wrong or unqualified on the project's own honesty surface.
     14 patches applied, ALL held up on inspection (none skipped). Gates: anchors 119/119 before AND after; spec-lint PASS; clippy 0; targeted suites 142; ui lib 643; trading_core + audit green; `cargo check --workspace --all-targets` clean. -->

**VERDICT: PASS WITH FINDINGS.** This screen is the operator-facing comparator for the era-qualified "active ≤ passive" thesis, so wrong or unqualified numbers on it are correctness defects. Three were found, all now fixed.

- [x] [Review][Patch — HIGH] **The drawdown band rendered a FRACTION with a `%` suffix: every drawdown read 100× too small.** `drawdown_band.rs` took `EquitySeries::max_drawdown_pct` — computed as `(peak−amt)/|peak|`, i.e. `0.4182` for 41.82% — and formatted it `"{dd:.1}%"`. Orchestrator-verified at source. **The repo's own committed snapshot carried the contradiction on adjacent lines**: `max_dd: −48.95%` beside `max_dd: 0.41817608…`. Also reached Reports, the viewer bin and the gallery. **Root cause fixed structurally, not patched**: `EquitySeries::max_drawdown_pct` and `BacktestMetrics::max_drawdown_pct` shared a name *in the same file* with different units, a collision that had already produced one 100× bug (bug-log #77) — the `EquitySeries`/`EquityPoint` fields are renamed `max_drawdown_frac`/`drawdown_frac` across 3 crates, so instance #3 is a **compile error**. RED-proven: removing the `× 100.0` yields `left: "0.6%" right: "60.7%"`, reproducing the reported symptom exactly.
- [x] [Review][Patch — HIGH] **The Max-DD card and the curve drawn beneath it disagreed by up to 7.13 points.** The card is the hourly-derived §7.1 const (34.57% / 48.95%); the curve is built from the **daily-sampled** CSV (33.3061% / 41.8176%, independently recomputed and matching the snapshot fractions exactly). `loader.rs` states the requirement it violated: *"the panel draws the realized curve, so the strip must match the line."* Resolved by **disclosure rather than re-derivation** — recomputing the card from the daily CSV would put the cockpit at odds with §7.1 and every artifact citing it — with the curve figure computed from the *loaded* series so it cannot rot, plus a test binding the two.
- [x] [Review][Patch — HIGH] **The displayed Sharpe was a different quantity from the project's published passive bar, unlabelled.** Screen showed the realized single-path figure (1.8417 / 0.8925); the PRD headlines the bootstrap p50 (+1.74 / +1.10 — orchestrator ground truth 1.735275 / 1.104731). So 2023 **overstated by ~6%** and 2024 **understated by ~19%**, on the screen whose entire purpose is "this is the bar active must clear". Both quantities are legitimate and the design's choice is right; the defect was that "realized", "single-path", "bootstrap" and "p50" appeared **nowhere** in the UI. Now qualified on screen, inside the existing no-overclaim gate.
- [x] [Review][Patch] **No cost disclosure on the honesty surface** — the BH construction is explicitly gross and the screen showed "+196.22%" without saying so. Caption now states it.
- [x] [Review][Record CORRECTED — supersession, mirror form] **This story shipped zero rendered-pixel proof of its own screen, and the real proof lives in a later story's file cited in ZERO trace rows.** `_audit_group_a_render.rs` (`audit_baseline_populated_renders_curve`, >1000 curve px both years, production `load_into`, with a real negative control at <600 px) was written 10 days later by `151073a`, whose own header says: *"Baseline — NO pixel-render test existed … had NEVER been pixel-verified."* A worse asymmetry than 2-15's, where the vindicating harness at least appeared in three successor rows. Backfilled into the trace row.
- [x] [Review][Record CORRECTED — governance] **The trace row's "all integration suites GREEN" was false when written.** Timeline verified at source: implement `f1c1bf3` 19:23 → tester VERDICT PASS 19:53 → repair `8dbe6ae` next morning 07:11. The tester report's suite table **omits `visual_snapshots` and `render_snapshots` entirely**, folding them into "All other suites"; 56 tests in them were red, caused by this story's own T7 nav entry. The 11.5h-later repair updated **zero** spec/trace/CHANGELOG files, and the claim stood uncorrected for two months.
- [x] [Review][Investigated — NO finding] **The 56-snapshot re-baseline (`8dbe6ae`) was rigorous, not laundering.** Verified independently, all 56 files, no sampling: content area **byte-identical** in every one; rows below the insertion band are a pure translation by exactly one nav-row pitch; the residue is sub-pixel antialiasing phase from the group growing 3→4 entries; the commit contains **zero** source files. Its dev-note classified before regenerating (diff bbox, cross-fixture identity, an opt-level hypothesis refuted by controlled experiment), proved masked-content-diff = 0, and re-proved gate strictness by mutation. **It is the most rigorous re-baseline in the repo — and nothing required any of it**; nine later re-baselines imitated none of it, most buried in ordinary feature commits. Protocol now recorded in the playbook; codifying it as an ADR is an operator call, deliberately not minted during review.
- [x] [Review][Patch → bug-log #77 RIDER] **The visual gate manufactures its own expected value.** `visual_diff.rs`: `if !baseline.exists() { actual.save(baseline)?; return Ok(()) }` — delete a baseline PNG and the test **writes it and passes green**, with `visual_snapshots.rs` documenting exactly that as the sanctioned accept-a-change workflow. The only safeguard is a human remembering to open the PNG. This is the vacuity class reaching the *harness*, and it is the **live exposure for story 6-9's pending ~62-file font re-baseline**, which will pass by construction whether the screens are right or not.
- [x] [Review][Patch] **The "re-sync trigger" could not trigger.** `baseline_metrics_match_characterization` was documented as the mechanism firing when the characterization is re-run — it asserted the const against **literals in its own file** and never opened the doc. It now parses §7.1 **and** §7.3, with `section_7_1_parser_reads_the_document_it_is_given` proving the parser isn't itself vacuous. (A patch-2 mutation produced a free RED-proof of this: the failure message quoted a value *parsed out of the characterization*.)
- [x] [Review][Patch] **The 5 panel snapshots could not go RED on any view-layer defect** — the mirror re-implemented the composition and never called `screens::baseline::view`; deleting `.push(kpi)` or hard-wiring the year left them green. Fixed *both* ways: mirror routed through production seams **and** a real composition gate (`baseline_body_composes_every_row_in_order`) that lays out the production body and pins row count, order and the widget size contracts — because routing alone cannot catch a deleted push.
- [x] [Review][Patch] **The year toggle had no rendered-difference test** — swapping the two CSVs' contents left everything green while the screen showed 2023's curve with 2024's KPIs; the downstream pixel harness renders both years but only asserts `>1000 px` each, never that they *differ*. Now bound at data and render level. RED-proven: hard-wiring `active_curve()` to 2024 makes the renders differ in <1% of pixels and fails — **while still clearing both pre-existing `>1000 px` assertions**, exactly the reviewer's point.
- [x] [Review][Patch] Also landed: Sortino/Calmar doc-parsed instead of pinned by a regenerated snapshot, and the previously *static* both-years line made year-specific; 4 silent skips made loud and counted (this was not theoretical — the artifacts dir already moved once and one reference was **still stale**); `Screen::Baseline` added to `layout_invariants` (its screen list is hand-enumerated and omitted it, making the "layout green" claim vacuous for this screen) plus a `Scrollable` and a down-to-400px viewport test; header/schema validation with `BASELINE_DATA_CORRUPT` split from `…UNAVAILABLE` (the old string told the operator the file "isn't bundled" even when it was bundled and merely unreadable); Loading/Empty split via a shared production seam so "no data" and "not loaded yet" stop rendering identically; a reachable **boot-path `Decimal` overflow panic** replaced with checked arithmetic — choosing **reject over clamp**, because falling back to zero would understate a drawdown on the honesty surface; the band converted to borrow like its 2-15 sibling; thousands separators on the money axis.

Probes CLEAR: **chain — verified at the producing code, do NOT route to 1-25**: `build_buyhold_curve` fixes quantities at bar 0 and computes `Σ qty × close`, never constructing an `Order`, never entering `PaperEngine::step`, applying no fee and no funding — so the BUYHOLD surface is genuinely clean of #67/#69/#71/#72/#73. **One rider DOES reach it**: the √8575 constant (verified arithmetically — it squares to 8574.9999 while its own doc claims `sqrt(24*365)`) feeds the displayed Sharpes, a ~1.07% understatement; routed to 1-25 as a **re-sync obligation**, since correcting it silently obsoletes the cockpit const. **AD-9** — `Decimal` throughout the load path, floats only at the pixel boundary, one-way. **Identity-forge / seed-collision / loop-scope** — genuinely N/A, stated not skipped. **`.unwrap()` in draw/update** — none outside `#[cfg(test)]`. **All 21 findings anchor-impacting: NO** (no `evidence/` file touched, `anchors = []` correct, BH curve non-anchored).

**Snapshot discipline (bug-log #77 applied to our own fix)**: 8 `.snap` files changed. Each was accounted for individually with the independently-derived literal that now guards it; the widget snapshot was **hand-written from the derivation before the test was ever run**. One file (the Live steady-state placeholder) had no guarding literal — that gap was found and closed with a production seam plus a state→copy assertion before the pass reported. A regenerated snapshot records; the literal beside it gates.

- [ ] `cockpit-baseline-panel` 0.1.0 - the base feature (presenter-done)

## Dev Notes

- Source feature folder: `spec/v1/cockpit-baseline-panel/` - frontmatter status **`presenter-done`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `presenter-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Cockpit & UI › Live cockpit & dashboards.
- Provenance: `git log -- spec/v1/cockpit-baseline-panel` (full narrative); reports under `evidence/v1/cockpit-baseline-panel/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-COCKPIT-BASELINE-001` (state=`tester-done`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
