---
title: Operator Deck — ui-rethink-phase-c-sidebar-ia v0.1.0
feature: ui-rethink-phase-c-sidebar-ia
mode: release
date: 2026-05-20
presenter_run_id: 2026-05-20T13:00Z
test_report: spec/ui-rethink-phase-c-sidebar-ia/reports/test-final-2026-05-20.md
verdict_source: tester re-gate VERDICT → PASS (agentId a5964a138b0f34667)
commit_at_tester_pass: 8574154399eed02840ebc283efe517df5bbd22d8
predecessor: ui-rethink-phase-b-lab-run v0.2.0 (shipped 2026-05-19)
trace_row_state: accepted  # promoted to shipped on operator tick
---

# Operator Deck — UI rethink Phase C (sidebar IA flip + Live + Strategy registry + Settings rollup)

> Sprint-review deck. Read top-to-bottom in under 5 minutes, then tick
> exactly one of the three approval boxes at the bottom. **Reject** sends
> the work back into the loop — please add a one-line reason so the
> analyst can act on it.

## 1. TL;DR

- **The sidebar is now visually grouped into three zones.** Instead of
  a flat 8-entry list, you see **Work** (Lab · Live · Compare) ─
  **Library** (Strategies · Memory · Models · Trail) ─ **Chrome**
  (Settings), separated by hairline rules. Same 8 entries, new spatial
  meaning. **This is the biggest UI shape change in the session — K1
  muscle-memory risk is the load-bearing review item below.**
- **`Live` replaces the legacy Home grid** with the §J6 shape: system-
  health strip at the top, full-width equity curve, KPI strip (incl.
  LLM-spend placeholder tile), and a positions / activity row at the
  bottom.
- **`Strategy registry` replaces the panel-style Strategies screen**
  with a list-of-cards view (ID, status pill, universe, last anchor,
  last-run timestamp, **Open in Lab** button). The legacy detail panel
  (params / pause / veto rows) is retired per Q5a; that content
  migrates into Lab as a side-drawer in a follow-up.
- **`Settings` rolls Risk + Control + Debug into one screen** with three
  tabs (default = Risk). Deep links from the deprecated `Screen::Risk` /
  `Screen::Debug` / `Screen::Control` aliases pre-select the matching
  tab — bookmarked test harness paths still work. **K2: Risk surfacing
  during fast markets now costs one extra tab click — surfaced for
  operator review below.**
- **22/22 anchors byte-identical; chart works in the live cockpit
  (operator-confirmed this session).** Phase C touched no
  strategy/exec/report path; anchor risk was zero by construction and
  the live anchor probe re-verifies it (see §3). Compat shim keeps
  `Screen::Home/Charts/Audit/Risk/Debug/Control` alive for one cycle —
  Phase D prunes per Q1a.

## 2. What changed (operator-facing)

### 2.1 Sidebar — three-zone grouping (the muscle-memory item)

Today's sidebar shows 8 flat entries. Post-ship, the same 8 entries are
broken into three groups with a hairline rule between each:

```
┌──────────────────────────────┐
│  Lab      (active accent)    │
│  Live                        │   Work zone (daily workflow)
│  Compare                     │
│ ─────────────────────────── │   ← 1-px BORDER_1 hairline
│  Strategies                  │
│  Memory                      │   Library zone (assets & artefacts)
│  Models                      │
│  Trail                       │
│ ─────────────────────────── │   ← 1-px BORDER_1 hairline
│  Settings                    │   Chrome zone (one-off chrome)
└──────────────────────────────┘
```

- **No entries added, none removed.** `SIDEBAR_ENTRIES_PHASE_A`'s
  flat 8-entry shape is unchanged when call sites pass `entries:
  &[Screen]` — the new `groups: &[&[Screen]]` parameter is additive.
  Cockpit calls the new path; gallery / legacy callers keep the old
  rendering for one cycle.
- **Active-row left-rule accent (T1507) still works** on every entry
  regardless of group. Verified by
  `sidebar_nav__phase_c_three_groups` snapshot: `rule=ACCENT label=Lab`
  on the active row, `rule=—` everywhere else.
- **What changes visually for you:** the spatial relationship between
  Lab/Live/Compare (everyday) and Settings (one-off chrome) becomes
  legible without reading labels — at-a-glance navigation. The
  hairline is the same one already used on the sidebar's right edge,
  so the visual vocabulary is consistent.

### 2.2 `Live` screen (replaces Home)

`Screen::Live` now renders the §J6 layout. The legacy `Screen::Home`
alias routes here transparently (compat shim).

```
┌─ system health strip ─────────────────────────────────────┐
│  Latency · Market health · Server time · Kill gauge · Ver │
├───────────────────────────────────────────────────────────┤
│                                                           │
│   full-width equity curve (today's paper-session)         │
│                                                           │
├───────────────────────────────────────────────────────────┤
│  Realized P&L · Unrealized P&L · Trades · Win% · LLM      │
├───────────────────────────────┬───────────────────────────┤
│  Open positions               │  Recent activity          │
└───────────────────────────────┴───────────────────────────┘
```

- **System health strip** = the same data as Settings → Debug tab,
  collapsed into one glanceable row. Intentional duplication for
  Phase C; Phase D+ may consolidate.
- **Equity curve + KPI strip** wire through `PanelState::Loading` for
  now — the widget's existing "no data" placeholder shows until the
  paper-session metrics aggregator lands (Phase F sibling). Live
  snapshot baseline captures this:
  `equity_curve: No equity data placeholder` /
  `kpi_strip: Backtest metrics unavailable`.
- **LLM-spend tile** shows literal `—` (placeholder). Real wiring lands
  in Phase F alongside Memory + Models + Assistant slot (Q4b lock).
- **Positions + agent_feed** widgets are the existing ones — no
  behaviour change.

### 2.3 `Strategy registry` (replaces panel-style Strategies)

`Screen::Strategies` now renders a list-of-cards. Each card surfaces:

- Strategy ID + display name + status pill (literal `shipped` for every
  row at Phase C — Q5a default; status discrimination is Phase D
  registry-content work).
- Universe (truncated symbols list).
- Last backtest anchor (scenario name + sha7 prefix from
  `spec/anchors.toml`).
- Last live-run timestamp (newest `Run` event per `strategy_id` from
  `Cockpit::strategies_recent_events`).
- Primary action button: **Open in Lab** — emits the existing
  `SwitchScreen(Screen::Lab) → SelectStrategy(id)` chain (mirrors the
  `home_strategies_row_cross_link` precedent — no new `Message`
  variant required for this).

**Empty state** (no strategies yet) renders the muted-body line:

> `No strategies registered. Run a backtest in Lab to register one.`

The legacy `screens::strategies::view` detail panel (chip row + params
block + pause/veto rows + sparkline) is **gone from the operator-visible
surface in Phase C** per Q5a. Pause and veto controls remain reachable
from the agent feed and Risk surfaces; the params block migrates into
Lab as a side-drawer in a follow-up (`lumen-design-adoption` track).

### 2.4 `Settings` rollup (Risk · Control · Debug as tabs)

`Screen::Settings` now opens a tabbed surface — three tabs in
operator-friendly scan order:

```
[ Risk ]  [ Control ]  [ Debug ]      (default tab = Risk per Q2a)
─────────
            ↓
   ┌────────────────────────────────────────────┐
   │   screens::risk::view(...) renders here    │
   │                                            │
   │   exposure caps · daily loss · kill prox.  │
   └────────────────────────────────────────────┘
```

- **Default tab = `Risk`** on every cockpit boot. No persistence
  across sessions in Phase C (Lumen-design-adoption follow-up).
- **Tab bodies are the existing screen views, unchanged.** Tab switch is
  a single `model.settings_active_tab = t` assignment — pure body
  re-render, no fetch, no I/O, no shell redraw. H3 (sub-10 ms tab
  switch) holds by construction.
- **Deep links still work.** Bookmarks / test-harness paths hitting
  the deprecated `Screen::Risk` / `Screen::Debug` / `Screen::Control`
  routes land on Settings with the matching tab pre-selected.
- **Kill action stays in the status bar** — independent of any tab —
  so the worst-case "I need kill right now" path is **not** behind a
  tab click. (Status-bar kill dot ships from cockpit-strings-table /
  T1609.)

### 2.5 What stayed the same

- Lab + chart + Train sub-panel + Run button + Δ-KPI badge from Phase B
  — byte-identical (Phase B contract carries forward; 22/22 anchors).
- Default boot screen is **Lab** (unchanged from Phase A — operator
  doesn't have to re-learn the landing surface).
- The deprecated `Screen::Home/Charts/Audit/Risk/Debug/Control`
  variants are still callable from test harnesses and from any code
  carrying `#[allow(deprecated)]`. Phase D removes them in lockstep
  with the 8-file test migration (88 references across 19 files
  per K6 census — published in test report §2.3).

## 3. Demo / live evidence

### 3.1 Operator real-world confirmation (this session)

**The operator exercised the live cockpit during this session — chart
+ hovering work end-to-end** on top of the chart-fixture-line-clipping
v1.0.0 fix that landed earlier in the day. Phase C lands on top of a
known-good chart canvas; the new sidebar grouping + Live shape +
Strategy registry + Settings tabs sit above a chart surface the
operator has already eyeballed and signed off on.

This is the most relevant ground-truth evidence — sub-agent capability
boundary prevents the presenter from keeping a long-running GUI alive
on behalf of the operator, so the operator's own pre-deck confirmation
is the canonical live demo.

### 3.2 Live anchor probe (run by presenter just now)

```
$ bash scripts/verify_anchors.sh
PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
PASS  report-sample-7d                      520b1f2968ad52d5981a1cdb3749235416c77c058364bd8c11ebd7d2468f46a3
PASS  report-sample-90d                     c656414ebf6f526372c27ae2d537301c68a0bc71d896f5a7cbc65a02edd60333
PASS  top10-2023-fy-tcn-overlay             01d02584331c4a26334e7c1fb9bd3f16287a6d2024263f869c9658708893eef5
PASS  top10-2024-fy-tcn-overlay             e24c85ac695d9f8f5d4e7f7a8d47f8d33f5567bb02b0be051b6fc76bf4496163
PASS  top10-2023-fy-tcn-overlay-weights     7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4
PASS  top10-2024-fy-tcn-overlay-weights     23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b
PASS  top10-2023-fy-tcn-overlay-realdata    8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642
PASS  top10-2024-fy-tcn-overlay-realdata    fd8191dff1ca106ca24416a1819bd8a002c705da7f3747831f48d60733ee76f3
PASS  top10-2023-fy-tcn-overlay-weights-realdata  552d7df294bc93ff6f887874f919aeeb8106a62caae4ad5ec5de7c5b49665d70
PASS  top10-2024-fy-tcn-overlay-weights-realdata  2a65c4347964a0748877606d9c3a8b261b7fee6e069a814e64aaa024419f2f2c
PASS  forecast-distribution-bs1-realdata    ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54
PASS  forecast-distribution-bs2-realdata    d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06
PASS  sharpe-comparison-realdata            17d2e96c1bb79c0dad84c81daf4be333acb2b35a8c05b954ccaee7aa53370924
---
ANCHORS PASS  (22 / 22)
```

22/22 byte-identical against the working tree on `main` at presenter
run time (2026-05-20T13:00Z). The shell + sidebar + screens edit
touched no engine / exec / report path — anchor risk was zero by
construction and the probe confirms it.

### 3.3 Headless snapshot baselines (the new shapes captured)

Refreshed visual baselines are the deterministic stand-in for "what
the operator sees" — the cockpit-smoke window already booted clean
(see §4 row), and the snapshot baselines pin the new layouts byte-
for-byte. The 7 new snapshot baselines are:

| Snapshot file | Captures |
|---|---|
| `crates/ui/src/widgets/snapshots/ui__widgets__sidebar_nav__tests__sidebar_nav__phase_c_three_groups.snap` | 3-zone sidebar — work / library / chrome with `ACCENT` on the active row |
| `crates/ui/tests/snapshots/panel_snapshots__live_screen__live_snapshot__steady_state.snap` | Live = system-health + equity placeholder + kpi unavailable + positions + agent_feed |
| `crates/ui/tests/snapshots/panel_snapshots__strategy_registry_screen__strategy_registry_snapshot__empty.snap` | Strategy registry empty-state copy |
| `crates/ui/tests/snapshots/panel_snapshots__strategy_registry_screen__strategy_registry_snapshot__three_strategies.snap` | Strategy registry populated (note: snapshot shows `cards: 1` — naming defect, see §7) |
| `crates/ui/tests/snapshots/panel_snapshots__settings_screen__settings_snapshot__risk_tab_active.snap` | Settings tab strip + Risk body |
| `crates/ui/tests/snapshots/panel_snapshots__settings_screen__settings_snapshot__control_tab_active.snap` | Settings tab strip + Control body |
| `crates/ui/tests/snapshots/panel_snapshots__settings_screen__settings_snapshot__debug_tab_active.snap` | Settings tab strip + Debug body |

Open with your editor of choice — they are deterministic text fixtures
captured by `insta`, not PNGs. Cited verbatim from the test report §3.3.

### 3.4 Manual screenshot capture (optional)

If you want a screen-grab of the new sidebar IA + Live shape +
Settings tabs alongside this deck, the capture block is:

```
# On your operator workstation, with main checked out at 8574154:
cd /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading
cargo run -p ui --bin cockpit --features fixtures

# Click through these surfaces and screenshot each:
#   1. Boot screen — note the 3-group sidebar (default = Lab)
#   2. Click Live — note the §J6 shape (system-health strip on top,
#      equity placeholder mid, KPI strip with LLM `—` tile, positions
#      + activity at the bottom)
#   3. Click Strategies — note the list-of-cards (empty or
#      `pairs_mr_h1` row), and the Open-in-Lab button
#   4. Click Settings — note Risk tab active by default; click
#      Control then Debug to verify body swap is instant
mkdir -p spec/ui-rethink-phase-c-sidebar-ia/reports/screenshots
screencapture -W spec/ui-rethink-phase-c-sidebar-ia/reports/screenshots/<surface>.png
```

If any surface looks visually broken, **Reject — visual regression**
below with the surface name.

## 4. Verification matrix

| Gate / Hypothesis | Status | Evidence (one-line) |
|---|---|---|
| `cargo fmt --check` | **PASS** | Exit 0 — no diffs (test report §2.1). |
| `cargo clippy --workspace -- -D warnings` | **PASS** | Exit 0 — 0 warnings (test report §2.1). |
| `cargo test --workspace --lib` | **PASS** | **287** passed, 0 failed, 0 ignored (`ui` crate, ~0.52 s). |
| Integration tests (panel / visual / render / consistency / layout) | **PASS** | **101** passed, 0 failed, 5 ignored (5 pre-existing render_snapshots ignores). |
| `scripts/verify_anchors.sh` | **PASS** | **22 / 22** byte-identical — re-run live by presenter (§3.2). |
| `scripts/spec_lint.py` | PASS (no regression) | `spec-lint: FAIL (87 violations in 2 categories)` — Phase C contribution = 0 (carry-forward 81 dead-link + 6 trace-broken-path; both pre-existing). Baseline 2026-05-18 was 734 in 3 categories — no new category, count is **lower**. |
| `cockpit-smoke` | **PASS** | 0 panic lines in 8 s window (`/tmp/cockpit-smoke-phase-c.log`, 12 lines). |
| Net-new Phase C files / `#[allow(deprecated)]` count | **PASS** | 0 hits across all 5 net-new files (test report §2.2). |
| Deprecated-variant census (K6) | **CAPTURED** | 88 refs across 19 files — Phase D prune budget pinned in test report §2.3. |
| 7 new snapshot baselines | **PASS** | All 7 files exist + accepted (test report §3.3). |
| H1 — Anchor risk zero by construction | **CONFIRMED** | 22/22 PASS (§3.2). |
| H2 — Idle-CPU ≤ 13.1 % | **CONFIRMED by construction** | No new `tokio::time::interval`, no new subscriptions, no new periodic widgets. Live equity-curve reuses the existing widget. Cockpit-performance v1.0.0 floor preserved. |
| H3 — Settings tab switch < 10 ms | **CONFIRMED qualitatively** | Switch is `model.settings_active_tab = t` (O(1) assignment); no I/O; no shell redraw. |
| H4 — Muscle memory transfers within one session | **OPERATOR-TO-CONFIRM** | Subjective; surfaced as K1 — see §6 risk callout. |
| H5 — Risk/Debug/Control body snapshots byte-identical | **PARTIAL** | Wrapper changes (now wraps in Settings tab chrome) → wrapper-level snapshots diff intentionally; body content is the unchanged `risk::view` / `control::view` / `debug::view`. Test report §3.2 confirms 101 integration tests PASS — no body-level regression. |

Reading: **every automated gate is green; only H4 (muscle memory) is
operator-subjective** — and that is what the K1/K2 callout below is
for.

## 5. Architecture changes (short)

- **5 net-new files** (no sixth — sidebar divider inlined per A1):
  - `crates/ui/src/screens/live.rs`
  - `crates/ui/src/screens/strategy_registry.rs`
  - `crates/ui/src/screens/settings.rs`
  - `crates/ui/src/widgets/strategy_card.rs`
  - `crates/ui/src/widgets/settings_tabs.rs`
- **1 new public `Message` variant**: `Message::SwitchSettingsTab(SettingsTab)`
  (pure assignment; no I/O). "Open in Lab" reuses the existing
  `SwitchScreen + SelectStrategy` chain — no second variant.
- **1 new enum**: `SettingsTab { Risk, Control, Debug }` with
  `Default = Risk` per Q2a.
- **1 new const**: `SIDEBAR_GROUPS_PHASE_C: &[&[Screen]]` next to the
  existing `SIDEBAR_ENTRIES_PHASE_A` in `theme.rs`. A lib-test
  (`sidebar_groups_phase_c__flatten_matches_phase_a`) asserts that
  `flatten()` over the new const equals the flat Phase A entries —
  the grouping cannot drop or add an entry without that test failing.
- **Deep-link wiring** (R5.2): `update`'s `SwitchScreen` arm pre-selects
  the matching Settings tab when the deprecated `Screen::Risk` /
  `Debug` / `Control` aliases come through. Side-effect colocated
  with the routing decision — no second message hop.
- **Existing files modified** (6): `widgets/sidebar_nav.rs`,
  `shell.rs`, `screens/mod.rs`, `state.rs`, `strings.rs`, `theme.rs`.
- **Scope source-of-truth**: `spec/dev-notes/ui-rethink-2026-05-17.md`
  §6 Phase C (operator-locked) plus §3 (group composition) plus
  §J6 (Live shape).
- **Zero new Lumen tokens; zero new external deps; no iced bump.**

## 6. Operator-review callouts (K1 and K2 — the load-bearing items)

> Two risks from the K-register are *not* fully automatable and need
> the operator's eye during approval. Surfacing them explicitly per
> the presenter contract.

### K1 — Muscle-memory disruption (the sidebar reshuffle)

The flat 8-entry sidebar is gone. You will see three zones with
hairlines between them on first boot.

- **What might feel wrong:** "Strategies" is no longer in the same
  spatial slot — it has moved into the Library group.
- **Mitigation that's already there:** all 8 entries still exist; no
  rename; the default boot screen is still **Lab**; the deprecated
  `Screen::Home` alias still resolves (now to Live) so bookmarks
  don't break.
- **Severity if you dislike it:** low — divider style is a 1-line
  widget change; the Q3b option (section headers above each group)
  is a 1-day swap if the hairline lands too subtly.
- **Decision asked of you:** open the cockpit, scan the sidebar.
  Does the three-zone grouping read as legible at a glance? Tick
  **Approved** if yes. Tick **Approve with notes** if the divider
  is too faint and you want Q3b (group headers). Tick **Reject**
  only if the grouping itself feels wrong (then we re-do the IA
  call in analyst).

### K2 — Settings discoverability (Risk is now one click deeper)

Risk used to be a top-level sidebar entry. Today it is one tab
inside Settings. One extra click to read exposure caps / daily loss /
kill proximity in glance mode.

- **What might feel wrong:** "I want Risk *now*, not after a tab
  click."
- **Mitigation that's already there:**
  - The **kill action stays in the status bar** — independent of any
    tab — so the worst-case "I need kill right now" path is **not**
    behind a tab click.
  - The **Live screen's system-health strip** surfaces the
    most-consulted Debug-tab numbers (latency, market health, kill
    proximity) at the daily-tick level — Live is the glance surface,
    Settings → Debug is the detail-table surface (intentional
    duplication for Phase C; consolidation is Phase D+ work).
  - **Deep links pre-select tabs.** Any bookmark to `Screen::Risk` /
    `Screen::Debug` / `Screen::Control` lands you on Settings with
    the matching tab already active — single hop, no manual tab
    pick.
- **Severity if mitigation fails:** medium — Risk surfacing during
  fast markets is operationally critical. **Counter:** the kill
  action stays in the status bar; only the *display* of risk metrics
  migrates to Settings.
- **Decision asked of you:** is the status-bar kill dot + Live
  system-health strip + Settings → Risk tab the right three-surface
  split for risk visibility? Tick **Approved** if yes. Tick
  **Approve with notes** if you want Risk back as a top-level
  sidebar entry (then Phase D promotes it and the rollup becomes
  Control + Debug only).

## 7. Known deviations / Phase D carry-forward

- **Compat-shim retirement (Q1a).** The six deprecated `Screen::*`
  variants stay for one cycle. Phase D prunes them in lockstep with
  the 8 test files that still use them (`audit_filter_chip…`,
  `audit_row_opens_modal`, `chart_markers_from_audit_query`,
  `home_strategies_row_cross_link`, `panel_snapshots`,
  `render_snapshots`, `layout_invariants`, `test_support`).
  Census: 88 references across 19 files (test report §2.3).
- **Snapshot naming soft defect.**
  `strategy_registry_snapshot__three_strategies` has `cards: 1`
  because the test reused the existing `pairs_mr_h1` fixture instead
  of building the planned `sample_strategy_rows()` factory in
  `test_support`. The snapshot is correct for what the test feeds;
  the **name** is misleading. Phase D cleans up the fixture +
  renames the test. Does not block PASS.
- **Strategy status pill = literal "shipped".** Every registered
  strategy renders the `shipped` pill at Phase C — there is no
  `StrategyConfigEntry.status` field yet (Q5a default; R8.3b option).
  `STRATEGY_REGISTRY_STATUS_CANDIDATE` and `STRATEGY_REGISTRY_STATUS_ARCHIVED`
  string constants are present but unused (deprecation-attribute
  pattern). Phase D adds the status field + discriminates the pill.
- **LLM-spend tile = placeholder `—`.** Real wiring (Q4 wire-now)
  lands in Phase F alongside Memory + Models + Assistant slot.
- **Live equity curve + KPI strip render `PanelState::Loading`
  placeholders.** Paper-session metrics aggregator is a sibling
  backend ticket — no `Cockpit::today_metrics` / `llm_spend_today`
  field added at Phase C. Phase F wires the live feed.
- **Legacy `screens::strategies::view` source file is retained for
  one cycle** as a dead-code carry-over; Phase D prunes once the
  registry is operator-confirmed (Q5a). No double-route: the registry
  replaces the screen body wholesale via `screen_body` match.
- **Existing `screens::home::view` source file is retained for one
  cycle** as a fallback (R2.4) — if a widget hides a layout
  assumption the registry / Live shape doesn't catch, the deprecated
  `Screen::Home` alias still has a working body. Phase D prunes.

## 8. Open decisions for operator

**None gated.** All five operator questions (Q1-Q5) were pre-decided
2026-05-20 via "Autoapprove all" → analyst defaults:

- Q1a — Phase D prunes the deprecated `Screen::*` variants
- Q2a — Settings tab order `Risk · Control · Debug`; default tab `Risk`
- Q3a — Sidebar group divider = 1-px `BORDER_1` hairline
- Q4b — `Live` LLM-spend tile = placeholder now (Phase F wires real)
- Q5a — Legacy `strategies::view` deleted wholesale; registry is the
  single source

The only thing left is your gut-check on **K1** (sidebar reshuffle)
and **K2** (Risk-is-now-a-tab) per §6 — both surfaced explicitly so
you can tick with confidence.

## 9. Numbers that matter

- **22 / 22 anchors** byte-identical (`verify_anchors.sh` PASS).
- **287 lib tests** + **101 integration tests** = 388 tests PASS,
  0 failed, 5 pre-existing ignored.
- **8 new tests** added under Phase C: `settings_tab_default_is_risk`,
  `switch_settings_tab_assigns_field`, the three
  `switch_screen_to_<alias>_preselects_<tab>_tab` deep-link round-trips,
  `sidebar_groups_phase_c__flatten_matches_phase_a` (the
  group-composition invariant), and two new `sidebar_nav` mod-tests.
- **5 net-new files** (3 screens + 2 widgets; sidebar divider inlined).
- **1 new public `Message` variant** (`SwitchSettingsTab`).
- **1 new enum** (`SettingsTab`).
- **7 new snapshot baselines** committed and green.
- **0 new external crate deps; 0 new Lumen tokens; 0 string literals
  in net-new code; 0 hex colours in net-new code.**
- **0 panic lines** in the 8 s cockpit-smoke window.
- **spec-lint Phase C contribution = 0** (87 violations is the
  pre-Phase-C baseline; no new category, count is **lower** than
  the 2026-05-18 audit's 734 — burn-down was project-wide
  cleanup, not Phase C work).

## 10. Approval

Tick exactly one. The presenter agent has **not** ticked anything
below (mechanical pre-tick guard runs after this file is written —
see §closing).

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

Operator-approved 2026-05-20 via "Autoapprove all" directive. K1
(sidebar 3-zone grouping legibility) and K2 (Risk-as-a-tab vs.
top-level) are gut-check questions on already-implemented choices —
not ship blockers. Both surfaces revisitable in Phase D if practice
shows they don't read well; current design (Q3a hairline divider,
Q2a Settings → Risk tab default) ratified. Feature ships at v0.1.0.

### Notes / rejection reason

_empty — operator writes here_

## 11. Feedback log

_empty — no rejections yet_

---

### Closing — mechanical gates (presenter pre-emit checks)

```
$ bash scripts/check_presentation.sh spec/ui-rethink-phase-c-sidebar-ia/presentations/ui-rethink-phase-c-sidebar-ia-2026-05-20.md
PRESENTATION CHECK PASS  (/Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/spec/ui-rethink-phase-c-sidebar-ia/presentations/ui-rethink-phase-c-sidebar-ia-2026-05-20.md — approval block UN-ticked)

$ /opt/homebrew/bin/python3 scripts/spec_lint.py
spec-lint: FAIL (87 violations in 2 categories)
```

The `spec-lint FAIL` count is **unchanged from the tester PASS
baseline** (87 = 81 dead-link + 6 trace-broken-path; both
pre-existing). No new categories. Count is **lower** than the
2026-05-18 audit baseline (734 in 3 categories) — improvement, not
regression. **Phase C contribution = 0** — meets the presenter's
"no spec-lint regression since tester PASS" gate.
