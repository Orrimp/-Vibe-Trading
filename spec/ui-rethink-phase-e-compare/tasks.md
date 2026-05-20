---
slug: ui-rethink-phase-e-compare
status: proposed
owner: architect
updated: 2026-05-20
---

# Tasks — UI rethink Phase E (Compare matrix)

> Analyst M0 ordered checklist. Architect M-T1 decomposition adds
> T-T1-* rows; developer T-D-N* rows append once the architect locks
> the decomp. **Per project convention, this file at analyst hand-off
> carries only the T-A* rows; T-T* / T-D-N* are appended in M-T1 by
> the architect.** Pointers: [feature.md](feature.md) carries R1-R8,
> Q1-Q8, K1-K8, H1-H5. Scope source-of-truth:
> [dev-note](../dev-notes/ui-rethink-2026-05-17.md) §6 Phase E
> (lines 1082-1096), §J3 (lines 340-390), §3 IA (lines 651-744).

## M0 — Analyst synthesis

- [x] T-A1 — Read dev-note §6 Phase E (scope source-of-truth) +
  §J3 (Compare strategies across pairs — operator job-story) +
  §3 (Per-pair-first navigation pattern — informs the matrix axis
  design) + §1141 addendum (Q8 operator-decided **background**).
  _Acceptance: feature.md "Why" + "Requirements" anchored to
  dev-note line numbers; no silent scope drift._

- [x] T-A2 — **Predecessor surface audit.** Confirm Phase C sidebar
  IA reserves `Screen::Compare` in `SIDEBAR_GROUPS_PHASE_C` Work
  zone (`crates/ui/src/theme.rs:742`). Confirm `screens::compare`
  is currently a placeholder route via `placeholder::view` at
  `crates/ui/src/shell.rs:96`. Confirm `strings::COMPARE_PLACEHOLDER`
  + `strings::SIDEBAR_NAV_COMPARE` already exist
  (`strings.rs:251-266`).
  _Acceptance: R5.1 / R5.2 / R5.3 cite the existing sidebar +
  placeholder wiring; no Phase C body change required._

- [x] T-A3 — **Report-cache shape audit.** Sample existing report
  frontmatter under `spec/<strategy>/reports/backtest-*.md` (e.g.
  `spec/v1-cross-sectional-momentum/reports/backtest-20260429-195148-top10-2023-1h-momentum.md`)
  and confirm the YAML frontmatter is flat key:value with a single
  nested `strategy:` block. Confirm the scenario→universe mapping
  is reconstructable from the scenario name (e.g. `top10-2023-1h-*`
  → 10-symbol universe; `btc-2023-1m-*` → BTCUSDT). Document Q6
  finding: existing reports carry **universe-aggregate KPIs, not
  per-pair**.
  _Acceptance: R3.1-R3.6 anchored to a real report path; K7
  surfaced honestly._

- [x] T-A4 — **Lab seeding contract audit.** Confirm how Lab state
  is pre-filled via `Message` dispatch — `SelectStrategy` +
  `LabSelectPair` + `LabRangeSelected` already exist at
  `crates/ui/src/state.rs:1305,1370,1810`. Identify the compound-
  dispatch precedent (Phase C `OpenStrategyInLab`, Phase D
  `OpenTrailFor`) so R4.1 mirrors it.
  _Acceptance: R4.1-R4.4 cite the existing seeding messages and
  the compound-dispatch precedent; H5 falsifiable test path named._

- [x] T-A5 — **Anchor-risk pre-flight.** Confirm Phase E touches no
  strategy / audit / exec / report-renderer code; the matrix
  consumes existing report files it does not generate. 22-anchor
  regression gate carry-forward; H2 from Phase D+ predecessor
  applies verbatim.
  _Acceptance: R7.1-R7.7 enumerate the 8-item non-regression
  contract; "anchor risk zero" claim defended by construction._

- [x] T-A6 — **Surface Q1-Q8 with recommended defaults** for
  operator-decide:
  - Q1 axis orientation (analyst-recommended: a — strategies as rows)
  - Q2 recompute cadence (analyst-recommended: c — report-cache only)
  - Q3 cell KPI (analyst-recommended: a — Sharpe)
  - Q4 empty cell behavior (analyst-recommended: b — Run affordance)
  - Q5 entry point (analyst-recommended: a — sidebar only)
  - Q6 multi-symbol universe-aggregate semantic (analyst-recommended:
    a + tooltip, ship per-pair-decomp in v0.2.0)
  - Q7 strategy enumeration source (analyst-recommended: a — registry)
  - Q8 pair enumeration source (analyst-recommended: b — universe
    gating)
  _Acceptance: feature.md "Q-questions (operator-decide)" section
  carries 8 entries each with recommendation + rationale + alt
  options._

- [x] T-A7 — **Author K1-K8 risk register.** K6 (Compare/Lab range
  divergence) and K7 (universe-aggregate semantic confusion)
  surfaced as the load-bearing UX traps; both surfaced honestly
  for operator review at M-FINAL.
  _Acceptance: feature.md K-section carries 8 entries each with
  severity + mitigation._

- [x] T-A8 — **Author H1-H5 hypothesis register.** Each hypothesis
  must be falsifiable by a named test or measurement:
  - H1 cache-hit rate ≥ 30 % at first matrix open (architect M-T1
    enumerates against live spec/ tree)
  - H2 6×10 matrix legibility (operator-subjective at presenter deck)
  - H3 idle-CPU floor ≤ 13.6 % preserved
  - H4 cache scan budget < 50 ms p99 (architect M-T1 micro-bench)
  - H5 `OpenLabFromCompare` round-trip atomic (unit test)
  _Acceptance: feature.md H-section carries 5 entries; each names
  a falsification path._

- [x] T-A9 — **Author acceptance criteria per milestone** (M0,
  M-OD, M-T1, M-FINAL). M-FINAL includes new snapshot baselines:
  `compare__cold_boot_all_empty`,
  `compare__steady_state_populated`,
  `compare__empty_cell_run_affordance`,
  `compare__column_header_hover`.
  _Acceptance: feature.md "Acceptance criteria" section structured
  per Phase D / Phase D+ precedent._

- [x] T-A10 — **Open trace row `REQ-UI-RETHINK-PHASE-E-001`** in
  `draft` state. `arch` / `crates` / `tests` / `anchors` columns
  left empty for architect / developer / tester to fill.
  _Acceptance: trace.toml carries the new row with title quoting
  the dev-note §6 Phase E scope; state = "draft"._

- [x] T-A11 — **Promote backlog entry.** Add `ui-rethink-phase-e-compare`
  to `spec/backlog.md` "Active" section directly above
  `v25-tcn-alpha-investigation`, mirroring the predecessor entry
  format. Carry the v0.1.0 / predecessor / Q1-Q8 / cost callouts
  from feature.md.
  _Acceptance: backlog.md "Active" section carries the new row;
  format consistent with the Phase D / Phase D+ predecessor entries._

- [x] T-A12 — **Emit analyst HANDOFF envelope** per AGENT.md
  communication contract (`from = "analyst", to = "operator",
  verdict = "READY-FOR-OPERATOR-DECIDE"`). Lists spec files
  written + Q1-Q8 that need operator input + assumptions /
  recommended defaults.
  _Acceptance: handoff envelope appended to assistant response;
  trace_refs include `REQ-UI-RETHINK-PHASE-E-001`._

## M-OD — Operator-decide (Q1-Q8) — resolved 2026-05-20

> All eight analyst-recommended defaults accepted in one tick via the
> operator's standing "Autoapprove all" directive (confirmed
> 2026-05-20 against the analyst hand-off envelope).

- [x] T-OD1 — Q1 = (a) strategies as rows, pairs as columns.
- [x] T-OD2 — Q2 = (c) report-cache only; no new recompute orchestration
  (manual via Lab Run; v0.2.0 candidate for background poll).
- [x] T-OD3 — Q3 = (a) Sharpe (single number per cell; matches Lab Run
  anchor metric).
- [x] T-OD4 — Q4 = (b) `Run` affordance per empty cell (reuses Phase B
  Lab Run dispatch).
- [x] T-OD5 — Q5 = (a) sidebar entry only (Phase C reserved entry already
  in `SIDEBAR_GROUPS_PHASE_C` Work zone).
- [x] T-OD6 — Q6 = (a) render all cells with universe-aggregate KPI +
  tooltip; per-pair decomp deferred to v0.2.0 (K7 mitigation noted).
- [x] T-OD7 — Q7 = (a) `Cockpit::strategies_config.strategies` registry
  enumeration.
- [x] T-OD8 — Q8 = (b) per-strategy universe with blanked-grey cells
  outside (honest about which cells are legal).

## M-T1 — Architect decomposition — PENDING

> Architect inherits this file at M-T1 and appends T-T1-* + waves
> A-G with T-D-N* rows per the predecessor pattern. Until the
> architect runs, no T-T or T-D-N rows are listed below; the
> developer must not pull a T-D-N row before the architect locks it.

> Architect M-T1 suggested deliverables (analyst pre-list, not
> binding):
> - K3 `serde_yaml` presence/absence resolution.
> - H1 cache-hit enumeration against live `spec/` tree.
> - H4 cache scan micro-bench.
> - R8.5 net-new file count (1 screen + 1 widget + 1-2 module files
>   + optional 1 helper).
> - Wave decomposition (suggested A-G):
>   - Wave A — `crates/ui/src/compare/cache.rs` module (R3).
>   - Wave B — `crates/ui/src/compare/state.rs` state plumbing (R6).
>   - Wave C — `crates/ui/src/widgets/matrix.rs` widget (R2).
>   - Wave D — `crates/ui/src/screens/compare.rs` body + shell
>     wiring (R1).
>   - Wave E — `Message::OpenLabFromCompare` + Lab seed round-trip
>     (R4).
>   - Wave F — Snapshot baselines + cockpit-smoke (M-FINAL prep).
>   - Wave G — Spec-lint sweep + trace.toml fill-in.

## M-FINAL — Tester sweep — PENDING

> Tester runs after Wave G closes. Gates per feature.md M-FINAL.

- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D
      warnings` exit 0.
- [ ] `cargo test --workspace --lib` 100 % PASS.
- [ ] 4 new snapshot baselines accepted
      (`compare__cold_boot_all_empty`,
       `compare__steady_state_populated`,
       `compare__empty_cell_run_affordance`,
       `compare__column_header_hover`).
- [ ] `scripts/verify_anchors.sh` → 22 / 22 PASS (R7.1).
- [ ] `cockpit-smoke` → 0 panic lines (R7.3).
- [ ] Cockpit-performance v1.0.0 idle-CPU floor ≤ 13.6 % (R7.4, H3).
- [ ] H1 cache-hit enumeration recorded.
- [ ] H4 cold-boot cache scan p99 recorded.
- [ ] H5 round-trip test
      `open_lab_from_compare_sets_lab_strategy_pair_and_range`
      PASS.
- [ ] Author `reports/test-final-<YYYY-MM-DD>.md` per
      `.claude/skills/rust-test/templates/test-report.md`.

## Notes

- **Analyst hand-off shape**: this tasks.md carries only M0 T-A*
  rows + M-OD / M-T1 / M-FINAL placeholders. The architect's M-T1
  pass appends T-T1-* + waves A-G with T-D-N* rows. Developer
  must not pull T-D-N rows before architect locks.
- **Predecessor reference**: Phase D's tasks.md
  (`spec/ui-rethink-phase-d-trail/tasks.md`) is the structural
  template; Phase E follows it 1:1 except for the K5-spike row
  (no architecture-unknown spike needed — the matrix is purely
  additive UI).
- **No cliffs.** Per dev-note §6 line 1134 ("No cliffs at C, E, F
  — each phase is independently shippable and independently
  reversible"), Phase E is reversible via a single revert of the
  `screens::compare` body + `Cockpit::compare_screen_state` field.
