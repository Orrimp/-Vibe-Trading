# Feature triage — cohorts A & B (2026-05-16)

Analyst-produced read-only triage feeding the remediation plan from
`SPEC_HYGIENE_PLAN.md`. Two cohorts:

- **Cohort A** — 4 features marked `in-progress` whose folders show no recent
  activity. Question: ship or abandon?
- **Cohort B** — 13 features the initial audit called "shipped without a test
  report". Question: retro-PASS or revoke?

Three of cohort B turn out to be audit miscounts (see below); true reportless-
shipped count is **10**.

## TL;DR table

| # | Slug | Cohort | Status (frontmatter) | Tasks open/done | Last commit | Verdict |
|---|------|--------|----------------------|-----------------|-------------|---------|
| A1 | v0-paper-sma | A | in-progress | 0/35 | 2026-05-08 (content 2026-04-19) | **SHIP** (bookkeeping flip + retro-PASS) |
| A2 | v05-composed-strategies | A | in-progress (feature.md) / **shipped** (tasks.md) | 0/30 | 2026-05-08 (content 2026-04-20) | **SHIP** (frontmatter drift — tasks already say shipped) |
| A3 | v1-cross-sectional-momentum | A | in-progress | 1/24 (T612 deferred to v1.5) | 2026-05-08 (content 2026-04-30) | **SHIP** (T_FINAL_A/B both `[x]`; single open task deferred) |
| A4 | ui-gallery-bin | A | in-progress / `v0.1-partial` | 39/0 (orchestrator-recovery layout) | 2026-05-15 (`777f4ea`) | **DECIDE LATER** (V1–V4 green, V5+ blocked on tiny-skia table panic) |
| B1 | chart-canvas-overhaul | B | shipped (1.10.0) | n/a | 2026-05-12 (`96ba58b`) | **RETRO-PASS** (TRIVIAL) |
| B2 | journal-transactions-metadata | B | shipped (1.6.1) | n/a | 2026-05-08 | **RETRO-PASS** (TRIVIAL) |
| B3 | live-cockpit-unified | B | shipped (1.5.0) | n/a | 2026-05-08 | **RETRO-PASS** (MEDIUM — cockpit-smoke needed) |
| B4 | per-symbol-position-accounts | B | shipped (1.4.0) | n/a | 2026-05-08 | **RETRO-PASS** (TRIVIAL — unit + ledger) |
| B5 | real-mtm-unrealized-pnl | B | shipped (1.3.0) | n/a | 2026-05-08 | **RETRO-PASS** (TRIVIAL — unit) |
| B6 | tape-row-audit-modal | B | shipped (1.6.0) | n/a | 2026-05-08 | **RETRO-PASS** (MEDIUM — UI snapshot + audit query) |
| B7 | ui-drop-iced-aw | B | shipped (0.1.0) | n/a | 2026-05-16 (`230bc75`) | **RETRO-PASS** (TRIVIAL — V1–V7 green per stub) |
| B8 | ui-gallery-bin | B (dup of A4) | `v0.1-partial` | 39/0 | 2026-05-15 | **NOT-SHIPPED** (status is `v0.1-partial`; audit overclassified) |
| B9 | ui-headless-emulator | B | shipped (0.1.0) | n/a | 2026-05-16 (`b87a82a`) | **RETRO-PASS** (TRIVIAL — V1–V6 green per stub) |
| B10 | ui-session-journal-iced-tester | B | shipped (0.1.0) | n/a | 2026-05-16 (`218cab3`) | **RETRO-PASS** (TRIVIAL — V1/V4–V8 green) |
| B11 | v1-5b-multi-venue | B | shipped (1.2.0) | n/a | 2026-05-08 | **RETRO-PASS** (HARD — multi-venue scenarios; candidate for REVOKE) |
| B12 | cockpit-app-bundle | B | **candidate** | n/a | 2026-05-11 | **NOT-SHIPPED** (stub feature; never promoted) |
| B13 | v25-kronos-forecast-overlay | B | **candidate** | n/a | 2026-05-10 | **NOT-SHIPPED** (stub feature; never promoted) |

`reports/` is genuinely empty (no `test-*.md`) for every cohort-B row except B1,
which has screenshots + diag log but still no `test-*.md`. The audit's empty-
report claim was correct for the 10 true rows.

## Section A — stalled in-progress

- **v0-paper-sma** — version 0.1.0; tasks 0/35; two backtest reports + smoke
  checklist present; anchor SHA locked. **Near-done.** Verdict: **SHIP** —
  flip `status: shipped` and run tester retro-PASS against the existing anchor.
- **v05-composed-strategies** — frontmatter drift: feature.md says `in-progress`,
  tasks.md says `shipped`; tasks 0/30; four backtest reports
  (`btc-2023-1m-macd-trend`, `rsi-reversion`, `bbands-mean-revert`,
  baseline-refresh). **Near-done.** Verdict: **SHIP** — reconcile feature.md.
- **v1-cross-sectional-momentum** — version 1.0.0; tasks 1/24, the single open
  task (T612 multi-symbol live `BinanceFeed`) is explicitly marked
  `[DEFERRED TO v1.5 — operator confirmed: T612 stays [ ] and is NOT a v1
  blocker]`; T_FINAL_A_v1 and T_FINAL_B_v1 both `[x]`; 2 backtest reports.
  **Near-done.** Verdict: **SHIP** — flip status; T612 stays open under v1.5
  lineage.
- **ui-gallery-bin** — status `v0.1-partial`; tasks 39/0 in the new
  orchestrator-recovery layout; commit `777f4ea` is
  `ship(ui-gallery-bin): v0.1-partial — V1-V4 green, V5+ blocked`. V5+ blocked
  on a tiny-skia `Build quad rectangle` panic in `widget::table::Table` from
  the strategies cell (bisected to `GALLERY_CELLS[7]`). **Blocked-on-table-
  cell-bounds-fix.** Verdict: **DECIDE LATER** — either accept v0.1-partial as
  terminal and open a successor feature (`ui-gallery-table-cell` suggested in
  tasks.md), or commit operator effort to V5+.

## Section B — shipped-without-test-report

(Acceptance recap + testable surface + retro-effort + verdict per row.)

- **chart-canvas-overhaul** (v1.10.0) — V1–V14 inherited PASS via presenter
  Acceptance section; screenshots + diag log present. Surface: cockpit-smoke +
  visual. Effort: **TRIVIAL**. **RETRO-PASS** (tester cites presenter
  Acceptance + on-disk screenshots).
- **journal-transactions-metadata** (v1.6.1) — V-items in feature.md §135;
  surface: pure unit + audit query. Effort: **TRIVIAL**. **RETRO-PASS**.
- **live-cockpit-unified** (v1.5.0) — V-items in feature.md §220; surface:
  cockpit-smoke (single-process binary boot, `--features live`). Effort:
  **MEDIUM**. **RETRO-PASS**.
- **per-symbol-position-accounts** (v1.4.0) — surface: unit + ledger
  reconciler invariant. Effort: **TRIVIAL**. **RETRO-PASS**.
- **real-mtm-unrealized-pnl** (v1.3.0) — surface: unit tests over
  `crates/reports` orchestrator MtM path. Effort: **TRIVIAL**. **RETRO-PASS**.
- **tape-row-audit-modal** (v1.6.0) — surface: UI insta snapshot + audit
  query. Effort: **MEDIUM**. **RETRO-PASS**.
- **ui-drop-iced-aw** (0.1.0) — feature.md: `V1-V7 green. 1216 workspace tests
  pass. Anchors 11/11 PASS.` Surface: workspace-test re-run + anchors. Effort:
  **TRIVIAL**. **RETRO-PASS**.
- **ui-gallery-bin** — duplicate of A4; **NOT-SHIPPED** (`v0.1-partial` is not
  `shipped`).
- **ui-headless-emulator** (0.1.0) — feature.md: `V1-V6 green`; surface:
  harness smoke. Effort: **TRIVIAL**. **RETRO-PASS**.
- **ui-session-journal-iced-tester** (0.1.0) — feature.md: V1/V4–V8 green,
  V2/V3 deferred to operator. Effort: **TRIVIAL**. **RETRO-PASS**.
- **v1-5b-multi-venue** (v1.2.0) — V-items in §612 cover multi-venue + 1s
  trade aggregation; testable surface = multi-venue backtest scenarios +
  reconciler. Effort: **HARD** (no scenario reports on disk; would need a
  re-run pass). **RETRO-PASS** with caveat that operator may want to
  **REVOKE-SHIPPED** if multi-venue runs cannot be reproduced from the current
  code state.
- **cockpit-app-bundle** — status `candidate`, frontmatter calls itself a
  "Stub feature file. Not promoted; no analyst spawn." **NOT-SHIPPED** (audit
  miscount).
- **v25-kronos-forecast-overlay** — status `candidate`, identical stub
  disclaimer. **NOT-SHIPPED** (audit miscount).

## Operator decision points

1. The three strategy features (A1/A2/A3) are bookkeeping debt: code + reports
   landed weeks ago, frontmatter never flipped to `shipped`. A single batch
   reconcile + tester retro-PASS pass clears them.
2. The two `candidate` features (B12/B13) and the partial (B8/A4) account for
   3 of the 13 audit "shipped" entries — the headline drops to 10 once those
   are reclassified.
3. The hardest retro-PASS is **v1-5b-multi-venue**; everything else is TRIVIAL
   or MEDIUM.
