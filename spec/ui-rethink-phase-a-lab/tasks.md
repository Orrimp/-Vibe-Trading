---
slug: ui-rethink-phase-a-lab
status: draft
owner: analyst
updated: 2026-05-17
---

# Tasks — UI rethink Phase A (chart-centric Lab)

> Skeleton task list. Analyst seeds the M0–M-FINAL milestone shapes
> with one-line acceptance criteria; **architect refines into ordered
> `T-D-1..T-D-N` rows** with explicit file paths, exact acceptance
> commands, and dependency edges. Each milestone closes with its
> own tester gate (`cargo test -p ui` green at minimum); the
> overall feature closes when M-FINAL's gates green AND the three
> non-regression gates (anchors / cockpit-smoke / spec-lint) pass.

## Milestones

### M0 — Screen rename + default-route flip

**Goal.** `Charts` → `Lab` (in name, in code path, in sidebar). Empty
overlays — the existing v1.10.0 chart body renders unchanged, but it
now lives under the `Lab` screen and is the cockpit's default boot
route. This milestone is a **safe rename** that proves the route shape
before any new rendering work lands.

**Maps to:** R1.1, R1.2, R1.3, R1.4, R9 (partial — Lab + Live rename
only at M0; placeholder routes for Compare / Memory / Models / Trail /
Settings land in M-FINAL).

**Acceptance.**
- `cargo test -p ui` green; the test-harness `Charts` references all
  rename to `Lab` (or auto-route via the deprecated alias path per
  R9.3).
- Cockpit boots into Lab; the body is the existing v1.10.0 Charts
  view, pixel-identical.
- Insta snapshot `shell__default_screen_lab` records the new boot.
- `verify-anchors.sh` exit 0 (the rename touches no non-UI crate, so
  the 11 anchors stay byte-identical).

_Architect-fan-out hint:_ one task per file rename (`charts.rs` →
`lab.rs`, `Screen::Charts` → `Screen::Lab`, sidebar entries,
`strings.rs` constants, `Message` variants + deprecated aliases).

---

### M1 — Pair chip + strategy chip + date-range picker

**Goal.** Three new widgets that drive `Cockpit::lab_state` and
re-render the Lab body when changed. No overlay layers yet — the
chart body responds to selection by swapping its data source (cached
report tuple) and re-rendering existing markers, but the equity-curve
+ comparison passes are still M2 + M3 work.

**Maps to:** R3 (pair chip + XRP-first order), R4 (strategy chip with
"+" compare affordance), R5 (date-range picker with named presets +
custom + "narrowed-from" badge).

**Acceptance.**
- `crates/ui/src/widgets/pair_chip.rs`, `strategy_chip.rs`,
  `date_range.rs` exist; each has a unit test + an insta snapshot.
- `Message::LabSelectPair`, `LabSelectStrategy`, `LabToggleCompare`,
  `LabSelectRange` exist and mutate `Cockpit::lab_state` correctly
  (proptests for the toggle + ≤4 enforcement live alongside).
- Cockpit smoke test: start Lab, click XRPUSDT chip (active), click
  ETHUSDT chip (XRP deselects, ETH activates); pick v1.momentum
  strategy chip (renders fills); click "+" on v0.5.macd (no overlay
  yet but `lab_state.compare_set` length is 1).
- XRP-first ordering enforced by the const slice in
  `crates/ui/src/state.rs`; an `assert_eq!` test pins the order.

_Architect-fan-out hint:_ chips and picker are independent widgets;
spawn parallel sub-tasks. Synchronize on the `lab_state` shape (define
the struct in `state.rs` first).

---

### M2 — Equity-curve overlay (reads from cached backtest reports)

**Goal.** Layer 2 of R2 — a single equity-curve polyline on a second
Y-axis. Reads from `spec/<strategy>/reports/backtest-*.md` via the
new `lab/equity_loader.rs`. Read-only; no engine invocation.

**Maps to:** R2.2, R2.4, R7 (full — equity loader + report-selection
rule + per-bar fallback).

**Acceptance.**
- `crates/ui/src/lab/equity_loader.rs` exists; integration test
  loads the v1-cross-sectional-momentum 2024 H1 report and verifies
  expected series shape (start equity, end equity, length).
- `chart::view` accepts an `Option<EquitySeries>` parameter; when
  present, renders the equity polyline + right Y-axis gutter +
  legend chip; when absent, the chart degrades to the v1.10.0 shape.
- Insta snapshot `chart__price_plus_equity_v1_momentum` records the
  two-line overlay on a fixture pair.
- Selecting a strategy with a cached report → equity line appears.
  Selecting one without → "no cached run" empty state (R5.4 / R7.2
  fallback path).

_Architect-fan-out hint:_ equity-loader and chart-render-pass are
independent — spawn parallel sub-tasks once the `EquitySeries` type
is defined in `core` (or `ui` — architect picks per the dev-note's
Phase B/D backend-prep boundary).

---

### M3 — Multi-strategy comparison overlay (≤4 lines)

**Goal.** Layer 3 of R2 — additional equity-curve polylines for each
strategy in `lab_state.compare_set`, color-coded per R2.3 / R8.2, on
the same right Y-axis. Auto-scales Y-axis to cover all visible lines.

**Maps to:** R2.3, R8 (full — `compare_set` ≤4, color slot
assignment, auto-scaling, pair-swap behavior, faded "no data"
legend).

**Acceptance.**
- Insta snapshot `chart__compare_three_strategies` records three
  distinct equity lines on ETHUSDT 2024 H1.
- Cockpit smoke test: with v1.momentum active and "+" on v0.5.macd +
  v0.sma, three lines render. A 4th "+" press adds a 4th line; the
  5th press is no-op + the "max 4 strategies" toast appears.
- Pair-swap test: from the above state, click BTCUSDT chip — each
  comparison strategy's line re-loads against BTCUSDT's cached
  report; any strategy without a cached run for BTCUSDT shows the
  faded "no data" legend chip (no broken line on the canvas).
- Right Y-axis auto-scaling verified via a chart-fixture test that
  passes a known min/max equity range and asserts the gutter labels.

_Architect-fan-out hint:_ once M2's equity loader is in place, M3 is
mostly a loop + a color-slot assignment. Spawn dev + ui-designer in
parallel — the dev side wires the loader fan-out, the ui-designer
owns the legend chip color-swatch rendering.

---

### M-FINAL — Lab tuple persistence + Lumen-token audit + non-regression sweep

**Goal.** Close the feature. Wire `lab_state` persistence to
`~/.config/trading/cockpit-lab-state.json`, run the Lumen Phase 1
token-audit script, run the full non-regression gate stack.

**Maps to:** R6 (full — persistence file + debounce + cold-start
defaults + restore), R10 (Lumen-token audit), R11 (non-regression
contract).

**Acceptance.**
- `cockpit-lab-state.json` writer + reader exist with a debounce of
  500 ms (proptest verifies no write storms under rapid chip
  selection).
- Integration test: select v1.momentum × ETHUSDT × "Last 90d", quit
  the cockpit process, relaunch, Lab restores the tuple.
- Cold-start default: with no on-disk state, Lab opens to the
  curated demo tuple per Q-A3 default (v1.momentum × ETHUSDT ×
  Last 90d) — or the operator-ratified alternative.
- Lumen Phase 1 audit: `grep '#' crates/ui/src/screens/lab.rs
  crates/ui/src/widgets/pair_chip.rs crates/ui/src/widgets/strategy_chip.rs
  crates/ui/src/widgets/date_range.rs` returns zero hex colors;
  same grep for raw string literals returns zero.
- `cargo test -p ui` green (full UI test suite, including the
  267 panel snapshots from the prior ship + the new M0–M3
  snapshots).
- `cockpit-smoke` skill exit 0 (per AGENT.md rule 6 — the
  orchestrator runs this gate before the presenter spawns).
- `verify-anchors.sh` exit 0 — all 11 body-SHA anchors
  byte-identical (R11.1).
- `spec-lint` exit 0 (per AGENT.md rule 7 — no dead links, no
  orphan-feature, no trace-broken-path against
  `REQ-UI-RETHINK-PHASE-A-001`).
- Visual A/B captured on the operator's 3360×1890 Retina: one
  before/after pair per overlay layer (buy/sell markers, equity
  curve, comparison overlay).

_Architect-fan-out hint:_ persistence + lumen-audit + non-regression
sweep are three independent closing tasks. Spawn three closers in
parallel; the tester role merges their outputs into a single
`test-<date>-ui-rethink-phase-a-lab.md` report.

## Notes

- **Branch policy.** All work commits to `main` per AGENT.md §Branch
  & worktree policy. Sub-agents write files only; the orchestrator
  owns commit + push.
- **Parallelism.** M1 chips + picker are independent; M3 wires off M2.
  M-FINAL gates fan out three-wide. The architect breakdown should
  preserve those independence edges.
- **Out of scope.** No audit-ledger schema changes (Phase D), no
  backtest-engine library-call refactor (Phase B), no model registry
  surface (Phase F). If a milestone surfaces a need for one, surface
  to the operator before committing.
- **Trace.toml row.** Analyst creates `REQ-UI-RETHINK-PHASE-A-001`
  in `spec/trace.toml` at HANDOFF time per analyst-contract §
  "trace.toml: own the `[req]` row creation". Architect fills `arch`
  + ADR refs; developer fills `crates` + `tests`; tester fills
  `anchors` (empty list is correct here — Phase A touches no
  strategy/audit/exec code).
