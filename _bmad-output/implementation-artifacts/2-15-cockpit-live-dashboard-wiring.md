# Story 2.15: cockpit-live-dashboard-wiring

Status: review

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the Live equity curve + KPI strip fed from the live agent,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the Live equity curve + KPI strip fed from the live agent.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

### Review Findings

<!-- bmad-code-review 2026-08-12 (burn-down 11 of 14; commit 72d0138, 886-line diff; layers: Blind 13, Edge 20, Auditor 9 raw — 42 deduped to 20). First UI story of the burn-down; AD-10 governs.
     VERDICT: PASS WITH FINDINGS. The feature is REAL and WORKS at HEAD. The verification record written for it was false, and two defects escaped through exactly that gap — both since fixed by later commits, neither by this story.
     14 patches applied (1 finding correctly REJECTED as wrong). Gates: anchors 119/119 before AND after; spec-lint PASS; clippy 0 in `-p ui -p trading_core`; render suites 144 green incl. a NEW un-gated pixel harness. -->

**VERDICT: PASS WITH FINDINGS.** The Live view genuinely works at HEAD. What failed review is the *verification record*: it offered a test COUNT where AD-10 requires a PIXEL, and the gap was not theoretical — a crash and a 100×-wrong money figure both shipped through it.

- [x] [Review][Record CORRECTED] **The AD-10 claim rested on a count, not a pixel — and no test in the implementing commit would have gone red if the wiring were reverted.** Of the three artifacts the trace row cited: `state.rs` is model-level (never calls `view`); `panel_snapshots.rs` is a **text mirror** that reads the model fields and re-implements the formatting, so it never constructs a widget; and `headless_emulator_smoke.rs` asserts `!screenshot.rgba.is_empty()` — true of any allocated 1280×720 buffer — on the deliberately **empty** state. Orchestrator-verified at source. Trace row now carries the qualification.
- [x] [Review][Record CORRECTED — supersession, both directions] **This story's real AD-10 proof lives in a LATER story's record.** `crates/ui/tests/live_equity_render.rs` — positive (`live_equity_curve_actually_renders`), **negative control** (`harness_catches_dropped_points_empty_curve`), relational (`healthy_curve_draws_far_more_than_broken`) — was written 2026-06-11 by `10d1709`, *after the operator kept seeing no graph*, and was cited in the trace rows for 2-16, 2-17 and Lab/Compare while appearing **zero** times here. Backfilled into this row. This generalized the supersession probe (playbook §4) to its mirror form: later work can *falsify* a claim **or supply the proof a story lacked** — records propagate forward, never backward.
- [x] [Review][Consequence — fixed later, NOT by this story] **The cockpit crashed on the first live bar after boot.** A flat/1-point curve drove `y_max − y_min == 0` → `frac_y = NaN` → a lyon `p.y.is_finite()` assert. This story is precisely what made it reachable, by deliberately rendering from ≥1 point on the default route. Fixed by `10d1709`, whose commit message names the crash verbatim; now pinned by the render harness.
- [x] [Review][Consequence → bug-log #77] **KPI cards rendered money 100× too small** (a 25% drawdown as "0.25%") because a *fraction* was assigned to a *percent* field. Four gates were green on it: two unit tests asserted the implementation's own values, and two snapshot baselines were **regenerated in the same commit as the code they were guarding**. The fixer's message (`3f9fd63`) records it exactly: *"the wiring test had encoded the bug as fact (0.10)"*. **Disclosed as bug-log #77 — a new class**: a baseline regenerated from the code under test has no independent authority over it and cannot disagree with it. The sharper reading of AD-10 is about **authority**, not pixels-vs-units: an assertion is a gate only if its expected value comes from somewhere the implementation cannot reach. Caught in the end by the operator looking at the screen — the one oracle not derived from the implementation.
- [x] [Review][Patch] **The KPI-strip half had no binding render proof.** Reverting `screens/live.rs`'s `&model.live_kpi` to the pre-story `&PanelState::Loading` left the whole suite green — the text mirror never calls the view, and `live_equity_render.rs` crops to the CURVE band so the strip is never sampled. **New un-gated pixel harness `crates/ui/tests/live_kpi_strip_render.rs` (6 tests)**, RED-proven: under that mutation 4 of 6 fail while `headless_emulator_smoke` (4), `live_equity_render` (15) and `panel_snapshots` (111) **all stay green** — 130 pre-existing tests that could not see the story's central change being undone.
- [x] [Review][Patch — honesty] **A healthy-but-flat feed rendered "Backtest metrics unavailable" (six dashes)** — the default first-run state in `ExecutionMode::Observe` (no orders ⇒ no fills ⇒ flat equity ⇒ every present-flag false). "No data" and "data is fine and flat" must not look identical on an honesty-first product. `Ready` now always renders its real values; the genuinely-absent case moved to `PanelState::Empty` at the parse seam.
- [x] [Review][Patch — honesty] **The caption claimed a scope the ring silently invalidated.** Past `LIVE_EQUITY_BUFFER_CAP = 2_880`, `buffer[0]` is no longer the session open, so "Session to date" silently became a rolling 48h return and a drawdown whose peak was evicted **vanished** from the Max-DD card; `theme.rs` admitted it in a comment (*"a longer session quietly slides a 48 h window"*). Worse: the durable hydrate loads only the **newest** 2880 rows, after which the caption switched to "Since inception" — false the moment history exceeds the cap. Fixed with a `LiveEquityWindow::{Anchored,Rolling}` model state and a distinct rolling-window caption, latched on first eviction **and** on a hydrate saturated at the reader's LIMIT.
- [x] [Review][Patch] Also landed: the drifted text mirror re-bound to the production seam (`kpi_strip::renders_unavailable`, the same function `view` is written in terms of) rather than deleted, keeping its card-text assertions; `PanelState::Empty` documented unreachable-by-design for these panels with the false doc corrected (making it reachable would render "No equity data" for a *closed channel* — the same honesty failure as the flat-strip bug); the O(N²) boot hydrate split into stage + derive-once (a 2880-row tail was ~4.15M `EquityPoint` constructions in ONE `update()` on the iced thread); the false "downsample" doc corrected **and** the per-frame full-series deep clone removed (borrow instead); a "Last equity update … (Ns ago)" staleness marker (the health strip does NOT cover this — its `last_tick` tracks market data, which keeps ticking while the P&L feed is dead); `|peak|`/`|first|` denominators so an all-underwater series no longer reports Max DD 0.00%; range-adaptive money labels (the €200 forward budget rendered five gridlines all reading "200"); the 4 cannot-fail `!rgba.is_empty()` asserts replaced by `assert_frame_painted`; and the stale "Trades = 0" doc.
- [x] [Review][Finding REJECTED] **L10 (tie-ordering) was wrong and was not "fixed".** The claim was that `ORDER BY bar_ts DESC, rowid DESC` followed by `.rev()` leaves duplicate-`bar_ts` rows in descending rowid order, corrupting the KPI denominators. It does not: `.rev()` reverses the whole sequence, so **both** keys flip and the result is ascending in `bar_ts` *and* in `rowid`. Orchestrator-verified. A regression test now pins the secondary key (nothing did), so a future "fix" toward `rowid ASC` goes red.
- [x] [Review][Adjudicated] **The `<` vs `<=` delivery guard: the code is right, the doc was wrong.** `as_of` is a wallclock stamp, not a delivery identity — in fast replay many genuinely distinct bars share a millisecond, so `<=` would drop every one after the first, reproducing the dropped-points "no graph" failure. Code kept; the `PnlHydrated` comment and a pre-existing test's misleading comment corrected so code, doc and test finally agree.
- [x] [Review][Standing gap — NOT this story] `panel_snapshots.rs` is `#![cfg(target_os = "macos")]` (pre-existing, ADR-0057 D2), so on the Linux and Windows CI legs it compiles to **zero** tests and the skip is counted nowhere. The new KPI harness is deliberately **un-gated** and runs on all three.

Probes CLEAR: **chain** — no anchored evidence produced or consumed (`evidence/v1/cockpit-live-dashboard-wiring/` does not exist; zero rows in `anchors.toml`; the diff touches nothing under `evidence/`), so **all 20 findings are anchor-impacting: NO** and nothing routes to the 1-24/1-25 re-lock. **AD-9** — no `f64` in KPI or return arithmetic; `Decimal` throughout, with the only conversions at the pixel boundary (one-way; the y-axis label caveat is documented). **Panic paths** — no reachable `.unwrap()`/`.expect()`/index panic in draw or update; the historical lyon NaN is fixed and pinned. **Degenerate geometry** — 0/1/2/identical points, zero range, zero-size inner rect all guarded. **Identity-forge / seed-collision / loop-scope** — genuinely N/A for a UI story, stated rather than skipped. **Known infra red** — the ~62 font-drift visual baselines (owner 6-9) are orthogonal; nothing was re-baselined to make anything pass.

**Verification honesty**: the render suites (144 tests across 5 binaries incl. the new harness), the `ui` lib (629), `trading_core` (48) and `audit` (113) are green, with clippy/fmt/spec-lint/anchors all clean. The **full** `cargo test -p ui` (108 binaries) did **not** complete — this box stalls ~60-230 s per freshly-linked binary in `dyld`, making it a ~3 h run — so the suites outside the render/lib set are unverified for this change. Recorded rather than glossed.

- [ ] `cockpit-live-dashboard-wiring` 0.1.2 - the base feature (presenter-done)

## Dev Notes

- Source feature folder: `spec/v1/cockpit-live-dashboard-wiring/` - frontmatter status **`presenter-done`** (verbatim), version `0.1.2`, updated `2026-06-17`.
- Status mapping: `presenter-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Cockpit & UI › Live cockpit & dashboards.
- Provenance: `git log -- spec/v1/cockpit-live-dashboard-wiring` (full narrative); reports under `evidence/v1/cockpit-live-dashboard-wiring/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-COCKPIT-LIVE-DASHBOARD-001` (state=`tester-done`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
