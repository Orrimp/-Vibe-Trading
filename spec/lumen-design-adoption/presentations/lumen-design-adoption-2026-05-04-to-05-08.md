---
slug: lumen-design-adoption
mode: retrospective
dates: 2026-05-04 to 2026-05-08
agent: orchestrator
verdict: APPROVED (5-of-6 phases shipped + operator-approved; Phase 6 reserved for v2 LLM)
supersedes:
  - spec/presentations/lumen-phase-1-foundation-2026-05-04.md
  - spec/presentations/lumen-phase-2-shell-ia-charts-2026-05-05.md
  - spec/presentations/lumen-phase-3-detail-screens-2026-05-05.md
  - spec/presentations/lumen-phase-4-backtest-panel-2026-05-06.md
  - spec/presentations/lumen-phase-5-humancontrol-agentfeed-2026-05-07.md
---

# Lumen design-system adoption — 5-phase retrospective

> **Combined retrospective.** This file consolidates the five
> per-phase sprint-review decks that ratified the lumen-design-
> adoption initiative across 2026-05-04 → 2026-05-08. Each phase
> ran the canonical analyst → architect → developer → ui-designer
> → tester → presenter pipeline; each was operator-approved with a
> typed `[x] Approved — ship` on its individual deck. The original
> per-phase decks were git-removed at consolidation
> (commit `<consolidation>`); recover any individual deck via
> `git show <sha>:spec/presentations/lumen-phase-N-<slug>-<date>.md`.

## TL;DR

Five phases shipped over 4 calendar days. The cockpit went from a
single-page operator-correct surface to a **left-sidebar shell with
seven screens**, a **per-symbol price chart with audit-anchored
buy/sell markers**, a **risk-/limits screen with tri-band threshold
gauge**, a **full audit-journal browser**, a **new viewer binary** with
KPI strip + equity curve + drawdown band, and the cockpit's first
**operator-write surface since v0** (HumanControl panel + per-strategy
pause + risk-veto override). Two cross-phase debts: **TD-1 (focus-ring
deferral) CLOSED** at Phase 5 via custom-widget escape hatch;
**TD-2 (risk-engine veto-emit upstream wiring) OPEN** for a future
phase. **11/11 backtest body-SHA-256 anchors stayed byte-identical
through every phase.** Phase 6 (Assistant slot) reserved for v2 LLM.

## Cumulative numbers

| Metric | Phase 1 baseline | Phase 5 ship | Net delta |
|---|---|---|---|
| Workspace tests | 757 | **896** | +139 |
| Test binaries | 96 | **110** | +14 |
| Snapshot baselines | 41 (36 refreshed + 5 net-new) | **86** (67 panel + 17 widget + 2 audit) | +45 |
| Bins in workspace | 2 (`cockpit`, `cockpit_live`) | **3** (added `viewer`) | +1 |
| Audit query methods (read-only, additive) | `recent_fills`, others | + `recent_fills_filtered` (P2), `recent_journal_filtered` (P3), `equity_curve_for_strategy` (P4) | +3 |
| Audit writers (additive) | `kill_switch_tripped` + others | + `strategy_paused`, `risk_veto_overridden` (P5) | +2 |
| SQL migrations | 7 (`002_strategy_events.sql` last) | **8** (`008_journal_transactions_venue.sql` P3) | +1 |
| Cockpit screens | 1 (single-page) | **7** (Home / Debug / Charts / Strategies / Risk / Audit / Control) | +6 |
| Net-new widgets | n/a | 11 (`status_bar`, `sidebar_nav`, `chart`, `canvas_chart`, `kpi_strip`, `equity_curve`, `drawdown_band`, `sparkline`, `human_control`, `override_risk_veto`, `focus_ring`) | +11 |
| Cross-phase primitives | n/a | 2 (`ChartBuffer` P2, `EquitySeries` P4) | +2 |
| Backtest anchors clean | 11 / 11 | **11 / 11 byte-identical** | 0 (preserved) |
| R16.3 brand-bleed grep | n/a | 0 matches in report bodies (test-* + backtest-*) | 0 (clean each phase) |
| `rust-validate` steps PASS | 5/5 | 5/5 (every phase) | preserved |

**Architect ratifications across initiative:** 64 Q-items resolved
(11 Phase 2 + 11 Phase 3 + 12 Phase 4 + 15 Phase 5 + Phase 1's 11 + the
master-roadmap Q11–Q14 operator-locked decisions); zero principled
overrides of analyst recommendations; one Q deferred (Phase 3 Q6
sparkline → Phase 4, where it shipped on the shared `EquitySeries`
primitive); one minor shape refinement (Phase 4 Q1 nested `drawdown_pct`
inside `EquityPoint` instead of a parallel `Vec<Decimal>`).

**Tester gate cycles across initiative:** 8 total (Phase 1 = 3 passes
[fmt+clippy → rustdoc → PASS]; Phase 2 = 1 pass [PASS]; Phase 3 = 1
pass [PASS]; Phase 4 = 2 passes [`match_same_arms` clippy → PASS];
Phase 5 = 2 passes [fmt drift → PASS]). Three first-pass FAIL reports
preserved on disk per audit discipline.

---

## Phase 1 — Foundation (2026-05-04)

**Approved 2026-05-04** · Tester third-pass `VERDICT → PASS` (8/8 gates)

### TL;DR

Tokens + tiers + status bar. The 12-token palette → full Lumen system
(warm-paper light + cool-deep dark + muted teal accent), three-tier
elevation language, whisper shadows, focus ring, and an always-visible
status bar at the bottom of the shell.

### What changed

- Full Lumen palette in `crates/ui/src/theme.rs` — warm/cool neutrals
  + sage/clay semantic pair (calmer than neon), both modes
  contrast-checked, dark cold-start default.
- Tier 0/1/2/3 elevation surface tokens (`canvas` / `panel` /
  `panel_raised` / `panel_sunken` / `overlay`); whisper shadows;
  sunken inset; focus ring; full spacing/radii/typography/motion
  ladders.
- New `widgets::status_bar` — 24 px row pinned at the cockpit bottom
  with connection dot + latency + account + server time + CPU
  placeholder + version.
- Tier-1 chrome (hairline border + whisper shadow + tinted background)
  applied to the 6 shipped widgets; modals adopt Tier-3 styling.
- T1507 active-row pattern (2 px ACCENT left rule, no fill change).
- 36 panel snapshots refreshed once via `cargo insta review`; 5
  net-new (status_bar × 4, kill_dialog_focused × 1).
- `spec/ui-design-principles.md` superseded with Lumen-anchored
  rewrite; `spec/design/` purpose-built bundle imported as canonical
  contract.

### Tester report

[`spec/lumen-design-adoption/phase-1-foundation/reports/test-2026-05-04c-lumen-phase-1-foundation.md`](../reports/test-2026-05-04c-lumen-phase-1-foundation.md)
(third-pass PASS). First two FAIL reports preserved on disk:
[`-04b…md`](../reports/test-2026-05-04b-lumen-phase-1-foundation.md)
+ second FAIL preserved in git history.

### Screenshots

[`spec/lumen-design-adoption/phase-1-foundation/reports/screenshots/README.md`](../reports/screenshots/lumen-phase-1-foundation/README.md)
— operator-runnable capture commands for `cockpit-fixtures.png` and
`cockpit-live.png`. (Sandbox lacked screen-recording permission;
captures pending operator.)

### Key numbers

- 757 tests / 96 binaries / 11/11 anchors clean
- 41 snapshot baselines (36 refreshed + 5 net-new)

### Carry-forward

**TD-1** (true keyboard-focus ring) ratified as deferral — iced 0.14
lacks `button::Status::Focused` + `text_input::Style.shadow`;
hover-state ring + ACCENT input border-shift as bounded approximation.
Two named upgrade triggers (iced version bump or custom-widget escape
hatch). Closes at Phase 5 ship.

---

## Phase 2 — Shell IA + Charts (2026-05-05)

**Approved 2026-05-05** · Tester first-pass PASS (8/8 gates)

### TL;DR

Left-sidebar shell with three screens (Home / Debug / Charts). The
**Charts screen** plots one symbol's price with buy/sell triangles
overlaid from the audit ledger — the cross-check the operator
surfaced at the 2026-05-04 session.

### What changed

- Sidebar nav widget — fixed 180 px, text-only labels (no icons,
  operator-locked Q11), T1507 active-row pattern. Phase 2 entries:
  Home / Debug / Charts.
- `Screen` enum with all 6 variants declared up-front (Phase 3 wires
  Strategies / Risk / Audit dispatch); `Cockpit::current_screen` +
  `Message::SwitchScreen` pure assignment.
- Home screen body — Phase 1 widgets re-housed (PnL + Positions +
  Strategies summary + Tape).
- Debug screen body — kill switch + latency + per-venue market
  health + server time + version + placeholder logs.
- Charts screen body — chip-row symbol selector (T1609 chip-row
  active-bottom variant) + iced canvas-based price chart (ACCENT
  line, BORDER_1 gridlines, no external chart crate) + buy/sell
  triangle markers in UP_500 / DOWN_500 from the audit ledger.
- Per-`(Venue, Symbol)` `ChartBuffer` rolling buffer on `Cockpit`
  (cap 60 1-min bars). Live mode via existing `bars_tx`; fixtures
  via deterministic `synthetic_candles` (DefaultHasher per-symbol
  seed).
- `audit::query::recent_fills_filtered(venue, symbol, since, until)`
  — additive sibling of `recent_fills`. Read-only over same scan
  pattern.
- Right-rail track reservation (`Length::Fixed(0.0)`) for Phase 6
  Assistant slot.

### Tester report

[`spec/lumen-design-adoption/phase-2-shell-ia-charts/reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md`](../reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md)

### Screenshots

[`spec/lumen-design-adoption/phase-2-shell-ia-charts/reports/screenshots/README.md`](../reports/screenshots/lumen-phase-2-shell-ia-charts/README.md)
— 4 capture commands (`cockpit-home`, `cockpit-debug`,
`cockpit-charts-with-markers`, `cockpit-live-home`).

### Key numbers

- 781 tests / 98 binaries / 11/11 anchors clean
- 53 snapshot baselines (45 panel + 8 widget; +12 net-new vs Phase 1)
- 11/11 architect Q-resolutions, zero deviations

### Carry-forward

Phase 2's `recent_fills_filtered` venue argument returns
`Ok(vec![])` for non-Binance venues — `journal_transactions` doesn't
yet carry a `venue` column. Phase 3 ships the additive migration.

---

## Phase 3 — Detail screens (2026-05-05 ship · 2026-05-06 approval)

**Approved 2026-05-06** · Tester first-pass PASS (8/8 gates)

### TL;DR

Three new sidebar entries — **Strategies**, **Risk**, **Audit** —
surface backend data the cockpit didn't have a UI for. Per-strategy
params + signal events; per-venue exposure + kill-threshold proximity
gauge; full ledger browser with venue/symbol/kind/time-range filters
and 250-row pagination.

### What changed

- Sidebar nav extended from 3 entries to **6**: Home → Debug →
  Strategies → Risk → Audit → Charts.
- Strategies-detail screen — per-strategy view with read-only
  `[[strategy]]` params from `config/agent.toml` + filtered signal-
  event history from existing `strategies_recent_events` buffer
  (architect Q2: no new audit writer; reuse existing channel). Click
  cross-link from Home → Strategies summary jumps straight to
  detail.
- Risk / Limits screen — per-venue exposure + daily loss limit
  consumed + kill-threshold proximity gauge as horizontal bar with
  tri-band ramp (UP_500 ≤70% → WARN_500 >70% → DOWN_500 >90%). New
  `RiskTelemetry` channel on `agent::EventBus` (mirror of Phase 1
  `MarketHealth` publisher).
- Audit / Journal screen — full ledger browser with filter chip-row
  (venue · symbol · kind · time-range) + fixed 250-row pagination +
  per-row click reuses existing T1208 `journal_transaction_modal`.
- `008_journal_transactions_venue.sql` migration — additive
  (`ALTER TABLE … ADD COLUMN venue TEXT DEFAULT NULL` + backfill
  `'binance'` for existing rows). 11/11 anchors byte-identical
  post-migration. `post_fill` writer's new `venue: Venue` parameter
  wired across ~25 call-sites.
- New sibling audit query `recent_journal_filtered` (architect Q7:
  split rather than extend `recent_fills_filtered` with `kind`).

### Tester report

[`spec/lumen-design-adoption/phase-3-detail-screens/reports/test-2026-05-05-lumen-phase-3-detail-screens.md`](../reports/test-2026-05-05-lumen-phase-3-detail-screens.md)

### Screenshots

[`spec/lumen-design-adoption/phase-3-detail-screens/reports/screenshots/README.md`](../reports/screenshots/lumen-phase-3-detail-screens/README.md)
— 4 capture commands (`cockpit-strategies`, `cockpit-risk`,
`cockpit-audit`, `cockpit-live-six-entries`).

### Key numbers

- 810 tests / 104 binaries / 11/11 anchors byte-identical post-migration
- 65 snapshot baselines (54 panel + 11 widget; +12 net-new vs Phase 2)
- 11/11 architect Q-resolutions, 1 deferral (Q6 sparkline → Phase 4)

### Carry-forward

Q6 sparkline deferral — equity-since-deploy on Strategies-detail
needs the same equity-history primitive Phase 4's viewer needs.
Bundled into Phase 4.

---

## Phase 4 — Backtest panel (`viewer` bin) (2026-05-06)

**Approved 2026-05-06** · Tester second-pass PASS (8/8 gates after a
one-line `clippy::match_same_arms` fixup)

### TL;DR

A new **`viewer`** binary renders backtest reports with a **KPI strip
+ equity curve + drawdown band** above the existing markdown body.
Phase 3's deferred per-strategy equity sparkline lands too, on the
shared `core::EquitySeries` primitive.

### What changed

- New `viewer` binary at `crates/ui/src/bin/viewer.rs` — CLI-arg-
  driven, zero-button surface. Workspace now ships **three bins**.
- KPI strip widget — six metric cards (Total return / CAGR / Sharpe
  / Max DD / Win rate / Trades). Source: parsed from existing
  markdown summary table (architect Q3 — no new artefact format).
  Missing fields render as `—` in `FG_3` (sample reports omit CAGR
  + Win rate by design).
- Equity curve widget — polyline in ACCENT + filled area in UP_500
  at low alpha; 5 horizontal BORDER_1 gridlines.
- Drawdown band widget — beneath equity curve; line + filled area
  in DOWN_500 at low alpha. X-positions align with equity curve via
  shared `EquityPoint` shape.
- `core::EquitySeries` cross-phase primitive (rich struct with
  `points: Vec<EquityPoint>` carrying ts/equity/drawdown_pct +
  peak/trough/max_drawdown_pct/inception_ts/as_of_ts).
- Refactored `widgets::canvas_chart` core — Phase 2's price chart
  primitives extracted; viewer's equity curve + drawdown band +
  sparkline all share the same canvas drawing primitives.
- `audit::query::equity_curve_for_strategy(strategy_id, since,
  until)` — additive sibling of Phase 2/3 filtered queries.
- `crates/reports/src/parse.rs` — markdown summary parser with
  graceful fallback for missing fields.
- Closes Phase 3 Q6 deferral — `STRATEGIES_SPARKLINE_DEFERRED`
  retires; new `widgets::sparkline` (~120-point cap with
  downsampling) lands in cockpit Strategies-detail screen body.
- "Deploy live" CTA explicitly excluded — paper-only product;
  deployment is config-driven, not a button.

### Tester reports

[`spec/lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md`](../reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md)
(second-pass PASS). First-pass FAIL preserved at
[`-2026-05-06-…md`](../reports/test-2026-05-06-lumen-phase-4-backtest-panel.md).

### Screenshots

[`spec/lumen-design-adoption/phase-4-backtest-panel/reports/screenshots/README.md`](../reports/screenshots/lumen-phase-4-backtest-panel/README.md)
— 4 capture commands (`viewer-full-report`, `viewer-drawdown`,
`cockpit-strategies-sparkline`, `cockpit-live-strategies-sparkline`).

### Key numbers

- 850 tests / 108 binaries / 11/11 anchors byte-identical
- 72 snapshot baselines (55 panel + 17 widget; +7 net + 1 deletion vs Phase 3)
- 12/12 architect Q-resolutions, zero principled overrides (one shape
  refinement on Q1 — `drawdown_pct` nested inside `EquityPoint`
  instead of parallel `Vec<Decimal>`)
- 2 tester passes (first-pass FAIL on `clippy::match_same_arms`)

### Carry-forward

**TD-1 still pending** — Phase 5 is the load-bearing decision
point because it ships net-new operator-write controls.

---

## Phase 5 — HumanControl + AgentFeed (2026-05-07 ship · 2026-05-08 approval)

**Approved 2026-05-08** · Tester second-pass PASS (8/8 gates after a
one-line `cargo fmt --all` fixup)

### TL;DR

The cockpit's **first net-new operator-write surface since v0**:
a "Control" sidebar entry with execution-mode toggle + per-strategy
pause + risk-veto override (typed-confirm flow). The four-phase
**TD-1 deferral closes** with a custom-widget focus-ring escape
hatch. The `tape` widget renames to `AgentFeed` (module-level only).

### What changed

- HumanControl panel widget on a new "Control" sidebar entry (7th).
  Execution-mode segmented control (Observe / Supervised / Auto,
  runtime-only persistence), daily loss limit + max position +
  used-today P&L mirror rows, kill button as bottom action.
  Debug-screen kill widget retires.
- Pause-strategy single-click toggle button per strategy on
  Strategies-detail (architect Q8: single-click both directions).
- Override-risk-veto typed-confirm modal flow with `OVERRIDE`
  phrase (mirror of kill-confirm). Per-veto button (architect Q9).
- **TD-1 CLOSED** via Path (b) — `crates/ui/src/widgets/focus_ring.rs`
  Subscription-driven custom-widget escape hatch wraps all four
  destructive surfaces with a visible accent-bordered halo on focus.
  Four-phase deferral retired. (Architect verified iced still pinned
  `=0.14.0`; Path (a) iced fold-in unavailable; Path (c) restate-
  with-deadline rejected — Phase 6 v2-LLM-gated, operationally
  indefinite.)
- Two new audit writers — `strategy_paused` + `risk_veto_overridden`.
  Additive `StrategyEventKind` variants at the application layer;
  `strategy_events.kind` column already TEXT, **no SQL migration**.
- `tape` → `agent_feed` module rename. `Cockpit::tape` field name
  preserved (Q14) to avoid 100+ test ripple — code-comment annotation
  points at the new module. 9 baseline filenames `tape_*.snap` →
  `agent_feed_*.snap`.

### Tester reports

[`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md`](../reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md)
(second-pass PASS). First-pass FAIL preserved at
[`-2026-05-07-…md`](../reports/test-2026-05-07-lumen-phase-5-humancontrol-agentfeed.md).

### Screenshots

[`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/screenshots/README.md`](../reports/screenshots/lumen-phase-5-humancontrol-agentfeed/README.md)
— 4 capture commands (`cockpit-control`, `cockpit-strategies-pause-
override`, `cockpit-override-modal`, `cockpit-focus-ring`).

### Key numbers

- 896 tests / 110 binaries / 11/11 anchors byte-identical
- 86 snapshot baselines (67 panel + 17 widget + 2 audit; +14 net delta vs Phase 4)
- 15/15 architect Q-resolutions, zero principled overrides
- 2 tester passes (first-pass FAIL on `cargo fmt` drift introduced by
  ui-designer's expanded-scope baselines pass)

### Carry-forward

**TD-2 OPEN** — risk-engine veto-emit upstream wiring deferred.
Phase 5 ships override surface over an empty live `Vec<VetoEvent>`;
the risk engine still vetoes upstream (safety primary preserved),
operators don't see veto events to override until upstream wiring
lands. Promotion trigger: operator request OR risk-engine evolution
OR compliance requirement.

---

## Cross-phase technical-debt rollup

| TD row | Status | Closure |
|---|---|---|
| **TD-1 — True keyboard-focus ring** | **CLOSED 2026-05-07** | Phase 5 Path (b) custom-widget escape hatch. Four-phase deferral retired. Visible halo lands on every focused destructive control. |
| **TD-2 — Risk-engine veto-emit upstream wiring** | **OPEN** | Phase 5 Q13 deferral. Override surface ships over empty live feed; not a safety primary, an observability gap. |

## Initiative status — 5-of-6 phases shipped

| Phase | Status | Shipped | Approval | Tester report |
|---|---|---|---|---|
| 1 — Foundation | ✅ Shipped | 2026-05-04 | 2026-05-04 | [`test-2026-05-04c-lumen-phase-1-foundation.md`](../reports/test-2026-05-04c-lumen-phase-1-foundation.md) |
| 2 — Shell IA + Charts | ✅ Shipped | 2026-05-05 | 2026-05-05 | [`test-2026-05-05-lumen-phase-2-shell-ia-charts.md`](../reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md) |
| 3 — Detail screens | ✅ Shipped | 2026-05-05 | 2026-05-06 | [`test-2026-05-05-lumen-phase-3-detail-screens.md`](../reports/test-2026-05-05-lumen-phase-3-detail-screens.md) |
| 4 — Backtest panel | ✅ Shipped | 2026-05-06 | 2026-05-06 | [`test-2026-05-06b-lumen-phase-4-backtest-panel.md`](../reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md) |
| 5 — HumanControl + AgentFeed | ✅ Shipped | 2026-05-07 | 2026-05-08 | [`test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md`](../reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md) |
| 6 — Assistant slot | _Reserved_ | _gated on v2 LLM_ | — | — |

Master roadmap at [`spec/lumen-design-adoption/feature.md`](../features/lumen-design-adoption.md).
Feature briefs at `spec/features/lumen-phase-{1..6}-*.md` (all `status:
shipped` except Phase 6 `status: reserved`).

## Operator approvals (audit log)

Each phase's approval was recorded by ticking `[x] Approved — ship`
on its individual sprint-review deck. The decks were git-removed at
this consolidation; the approval records are preserved both in this
retrospective table and in git history (`git log --diff-filter=D
--name-only -- 'spec/presentations/lumen-phase-*.md'` identifies the
removal commit; `git show <sha>:spec/presentations/<file>` recovers
the original).

| Phase | Deck path (git-historical) | Approval line | Approved on |
|---|---|---|---|
| 1 | `spec/presentations/lumen-phase-1-foundation-2026-05-04.md` | `[x] Approved — ship` (line 123) | 2026-05-04 |
| 2 | `spec/presentations/lumen-phase-2-shell-ia-charts-2026-05-05.md` | `[x] Approved — ship` (line 145) | 2026-05-05 |
| 3 | `spec/presentations/lumen-phase-3-detail-screens-2026-05-05.md` | `[x] Approved — ship` (line 149) | 2026-05-06 |
| 4 | `spec/presentations/lumen-phase-4-backtest-panel-2026-05-06.md` | `[x] Approved — ship` | 2026-05-06 |
| 5 | `spec/presentations/lumen-phase-5-humancontrol-agentfeed-2026-05-07.md` | `[x] Approved — ship` (line 161) | 2026-05-08 |

## Next-step decision (operator)

The lumen-design-adoption initiative reaches a natural pause point at
5-of-6 shipped. Three operator-decided paths:

1. **Promote v2 LLM** — its own analyst → architect → developer
   pipeline. Largest queued backend feature. When v2 LLM ships,
   Phase 6 (Assistant slot) unlocks.
2. **Promote a different Active backlog item.** Current
   [`spec/backlog.md`](../backlog.md) Active section is empty after
   the 2026-05-08 cleanup. Queue currently holds Reflection memory
   (v1.5a Q1 follow-up) and v2 LLM strategy as the genuinely-next
   non-Lumen non-Phase-6 candidates.
3. **Pause the cockpit-side initiative** — declare 5-of-6 complete;
   let the v2 LLM rollout pick Phase 6 up when it gets there.

Decision is operator's; this retrospective does not commit a path.

## Mechanical pre-tick gate

This file is a retrospective, not an open approval gate — it
consolidates five already-approved decks. The mechanical pre-tick
gate (`scripts/check_presentation.sh`) is **not run** against
retrospectives because there is no operator approval block to gate.
Each constituent phase's gate ran at its individual deck's write time
(all five passed; the script's PASS lines are quoted in this
session's git history).
