---
slug: ui-rethink-2026-05-17
status: living
owner: ui-designer
updated: 2026-05-17
---

# UI rethink — screen-by-screen redesign proposal (2026-05-17)

> Research dev-note in response to operator critique on 2026-05-17:
> *"I don't like the UI very much. It does not seem like a good fit
> for this project."*
>
> This is a strategy doc, not a feature brief. No code lands from this
> document. The Lumen design system (tokens, tier elevation, status
> bar, motion ladder, type ladder) is **not** under review — it is the
> floor. What's under review is the **screen inventory and navigation
> model** that sits on top of it. Phases 1-5 of
> [`spec/lumen-design-adoption`](../lumen-design-adoption/feature.md)
> shipped a beautiful design system bolted onto a Bloomberg-shaped
> seven-screen sidebar that is **strategy-blind and pair-blind** in a
> project whose stated headline workflow is "test strategies on
> different trading pairs". This note diagnoses that mis-fit and
> proposes a corrective IA grounded in the eight operator
> jobs-to-be-done.
>
> **Predecessors not re-litigated:**
> [`ui-testability-deep-dive-2026-05-15`](ui-testability-deep-dive-2026-05-15.md)
> (test-harness shape — orthogonal, holds);
> [`ui-testing-direction-2026-05-12`](archive/2026-Q2/ui-testing-direction-2026-05-12.md)
> (test-stack picks — holds);
> [`lumen-design-adoption/feature.md`](../lumen-design-adoption/feature.md)
> (Phase 1-5 design-system contracts — holds);
> [`chart-canvas-overhaul/feature.md`](../chart-canvas-overhaul/feature.md)
> (chart canvas regression history — holds; the chart widget itself
> survives this rethink). The critique trail collectively says the
> implementation is sound and the design tokens are right; this note
> says the **screen layer above them** is not.

## 0. TL;DR for the operator's tick

The cockpit ships **seven screens** (Home, Charts, Strategies, Risk,
Audit, Control, Debug) organised by **system facet** (positions vs
risk vs ledger vs operations). The project goal organises by
**operator workflow** (pick a strategy → test it on a pair → inspect
the agent's reasoning → override). Those two organisations don't
overlap.

Three concrete mis-fits:

1. **No first-class "test a strategy against THIS pair AND THIS date
   range" surface.** The headline operator job (J2 below) requires a
   `cargo backtest` CLI today. The `viewer` binary opens *one
   committed report at a time* — it is a presentation surface, not
   an authoring surface.
2. **Strategies screen is strategy-first, the universe is symbol-first.**
   v1 momentum has a 10-symbol universe; v1.5a pairs trades two
   symbols simultaneously; v2.5 TCN scenarios are named `BS-1`
   (BTC) and `BS-2` (ETH+BTC). The operator's mental model is
   "what does v1 do on ETHUSDT vs SOLUSDT" — the Strategies screen
   answers "what does v1 do" with a parameter sheet, full stop.
3. **The audit ledger is the locked moat
   ([product.md → Differentiator](../product.md#differentiator)) but
   the Audit screen is a flat-table journal browser** that doesn't
   lead the operator from "a fill happened" to "the analyst-debate-
   trader-risk-PM chain that produced it" — the whole point of
   building the multi-agent architecture.

The redesign holds Lumen tokens unchanged. It replaces the Home /
Strategies / Risk / Audit / Debug / Control sidebar with a **pair-
context shell** (left rail = active pairs from the universe), a
top-bar **strategy chip row** (the cross-cut over pairs), and a
single body that re-uses the existing Charts widget as the working
canvas. Backtest, Compare, Decision-trail, Memory, and Model-
provenance become **modes of that one canvas**, not separate
screens with separate skeletons. Live monitoring becomes a sticky
overlay over the canvas, not a competing top-level surface.

**One headline phase-1 vertical to test the hypothesis:** ship the
J2 "test strategy × pair × date-range" workflow as the new home,
behind a feature flag, against fixture data. If the operator likes
the shape the rest of the redesign cascades; if not the cost is
two weeks and zero touched anchors. Detail in § 6.

## 1. Audit — what the current cockpit actually surfaces

### Screen inventory (from `crates/ui/src/screens/`)

| Screen | Source | Purpose (declared) | Serves the locked goal? |
|---|---|---|---|
| **Home** | `home.rs` | 2×2 grid: PnL + Positions / Strategies + AgentFeed | Partly — operator's default tick, but is a static dashboard, not an entry point to any workflow |
| **Charts** | `charts.rs` | Per-pair price chart + buy/sell markers + volume tile + position mirror | **Yes** — closest thing to a working surface; mis-named "Charts" when it's actually the *symbol-context view* |
| **Strategies** | `strategies.rs` | Strategy chip row, params block, recent signal events, equity sparkline, pause/veto controls | Partly — surfaces *configuration*, not the *workflow*. No "run on this pair" affordance |
| **Risk** | `risk.rs` | Per-symbol exposure bars, daily loss bar, kill-threshold proximity | Yes — but lives as a separate screen the operator only opens reactively. Should be a status-bar tile + drill-down |
| **Audit** | `audit.rs` | Filter row + paginated journal table | **Mis-fits the goal.** It's a CRUD-style journal browser. The differentiator demands a *decision-trail viewer* — start from a fill and walk backwards through analysts → debate → trader → risk → PM. Current screen can find a row; it can't tell a story |
| **Control** | `control.rs` | `HumanControl` panel: execution-mode toggle + kill | Yes — but coexists awkwardly with the live cockpit; the kill button should be globally reachable, not gated behind a sidebar click |
| **Debug** | `debug.rs` | Latency, per-venue market-health, server time, version, logs placeholder | Yes for ops chrome, but the screen is a junk drawer. "Logs/metrics placeholder" has been there since Phase 2 |
| **(`viewer` bin)** | `viewer.rs` | Open one frozen Markdown backtest report (KPI strip + equity curve + drawdown band + body) | **Mis-fit.** Operator-facing backtest authoring should run *in the cockpit* against arbitrary scenarios; the viewer is a presentation surface for committed reports only |
| **(`ui_gallery` bin)** | `bin/ui_gallery.rs` | Storybook for widgets (dev-only) | Out of scope — internal tool |

### Honest mis-fit specifics

- **"Strategies" is a noun, not a verb.** The screen shows what strategies
  exist; it doesn't help the operator *do* anything with them. The
  pause-button row is the only verb on the screen, and it's a runtime
  safety control, not part of the author/test loop.
- **"Charts" is the actual workhorse, mis-labelled.** It's the only
  screen with a chip row that lets the operator switch contexts
  (symbols). It's where the operator should *spend most of their
  time*, but the sidebar puts it third and the screen has no
  strategy concept — markers are present but unattributed.
- **"Audit" is a flat table over a ledger that is the moat.** A row
  click opens a transaction modal (good); there is no path from
  a fill row to the LLM prompt, the analyst opinions, the
  forecaster overlay, the risk clamp, the PM decision. Build the
  multi-agent pipeline, then bury the chain behind a pagination
  button — the differentiator is invisible.
- **"Home" is a screensaver.** The 2×2 dashboard tells the operator
  "things are happening" but no panel is clickable into a deeper
  flow. Click the strategies row → it doesn't take you anywhere
  (sidebar swap to `Strategies` is required).
- **Live monitoring and backtest are different binaries today.** The
  `cockpit` and `cockpit_live` bins share the shell; the `viewer`
  bin is a third process. The operator workflow J2 — test a
  strategy then see it run live — crosses processes and crosses
  binaries.
- **The Lumen `Backtest.jsx` design (KPI strip + equity + drawdown
  + deploy-live action) is mostly unimplemented in the cockpit.**
  It exists in the `viewer` binary and only loads frozen reports.
  Phase 4 of the Lumen roadmap put the rich backtest panel
  *inside the cockpit*, but Phase 4 hasn't shipped — so the
  rich-backtest-panel pattern that would make J2 trivial sits
  unspawned in the queue.

### What surprised me in the audit

1. **`screens/charts.rs` is the most operator-aligned screen and has
   the worst name.** Rename it `Pair` and it becomes the natural
   home for J1-J4 + J8 — chart + strategy chip + agent feed +
   model overlay + decision drill-down all share the same chart
   spine. The sidebar's "Charts" label hides the most useful screen
   under a generic noun.

2. **`HumanControl` is a Phase-5 panel, but the kill switch
   migrated *out of Debug into Control* during Phase 5
   (`screens/debug.rs` lines 26-29).** That migration is correct
   by the design contract and wrong by the human-friendly rule —
   a kill switch behind a sidebar click violates the "destructive
   actions are confirm-gated but reachable in one motion" rule.
   The kill belongs on the status bar (a tiny red square always
   visible), with the confirm-phrase dialog escalating from there.

3. **No `Memory` screen exists.** The reflection-memory feature
   shipped (audit ledger lesson cards, retrieval at decision
   time) and yet the cockpit cannot show the operator "what
   lessons did the agent learn last session". The whole locked
   moat is silent in the UI.

4. **No `Model` screen exists.** v2.5 TCN, v2.5a PatchTST, v2.5b
   Transformer, v2.6 bake-off — four trained-model features in
   flight, and the cockpit has no surface for training-run
   provenance, sigma calibration, val loss, or forecast quality.
   The operator must read `spec/v25-*/reports/*.md` Markdown files
   to learn anything about a model. Build the model, hide the
   model.

5. **Phase 6 (Assistant slot) is the one *correct* IA call.** A
   right-rail Assistant panel for v2 LLM conversation is the
   right shape; the bug is that it depends on v2 LLM landing.
   That dependency is now satisfied (v2 LLM shipped per
   product.md roadmap status); Phase 6 can spawn whenever the
   operator approves it. The redesign below preserves the
   right-rail slot.

## 2. Goal-driven redesign — the eight operator jobs

Each subsection lays out the job, the proposed screen surface, the
data sources, the operations exposed, confirmation gates for
destructive paths, and which Lumen primitives to use. The naming
convention below introduces three new screen verbs — **Pair**,
**Lab**, **Trail** — that replace the current Home / Charts /
Strategies / Audit nouns. Risk / Debug / Control stay as ops
chrome but move out of the primary scan.

### J1 — Author / pick a strategy

**Job framing.** The operator opens the cockpit, scans the
strategy registry, and picks one to study. They want to see
**all strategies**, their family (rule-based / LLM / DL), their
shipped status (research / paper / live / deprecated), and a
one-glance recent-equity sparkline so the question "is this
strategy worth my attention right now?" is answerable in two
seconds.

**Proposed screen: `Strategy registry` (replaces current
Strategies).** A scrolling list view, one row per strategy, with:

- **Strategy id** (e.g. `v1.cross_sectional_momentum`)
- **Family pill** (Rule / Composed / LLM / DL / Hybrid)
- **Lifecycle pill** (`research` / `paper` / `live` / `deprecated`)
- **Universe** (the symbols this strategy is configured for)
- **30-day Sharpe sparkline** (paper or backtest; the most recent
  signal available)
- **Last-event chip** (Load / Swap / Reject from
  `strategies_recent_events`)

Click a row → **opens the Lab (J2) seeded with the strategy
selected**. Cmd+K palette: filter by family or lifecycle. No
operator-write affordance — strategies are config-driven
(`config/agent.toml`); editing happens out-of-band.

**Layout sketch:**
```
+────────────────────────────────────────────────────────────────+
│ Strategy registry                          [filter ▾] [Cmd+K]  │
+────────────────────────────────────────────────────────────────+
│ v0.sma                Rule    paper    BTCUSDT       ▁▂▃▂▁▂▃   │
│ v0.5.macd_trend       Rule    paper    BTCUSDT       ▁▂▁▃▄▃▂   │
│ v1.momentum           Rule    paper    Top-10 USDT   ▂▃▄▃▄▅▄   │
│ v1.5a.pairs           Rule    paper    BTC/ETH       ▃▄▃▄▃▂▃   │
│ v2.llm_overlay        LLM     paper    BTCUSDT       ▂▃▄▅▄▃▄   │
│ v2.5.tcn_overlay      DL      research BTCUSDT       (n/a)     │
│ ...                                                            │
+────────────────────────────────────────────────────────────────+
```

**Data sources.** `Cockpit::strategies` (already present),
`Cockpit::strategies_config` (already present), reflection-memory
or operator-success-reports for Sharpe sparkline data,
`Cockpit::strategies_recent_events`. **Zero new backend work.**

**Operations.** Open in Lab (default click), Compare in Matrix
(secondary; opens J3 with the strategy preselected). No
destructive actions on this screen.

**Confirmation gates.** None — read-only.

**Lumen primitives.** Tier 1 panel; `frame::active_row` for
hover/select; pill style from
[`docs/design/project/ui_kits/desktop/desktop.css`](../../../../spec/archive/design-prototypes-2026-Q2.tar.gz);
sparkline widget already exists.

### J2 — Test a strategy against a symbol + date range  (HEADLINE)

**Job framing.** This is the verb the operator named on 2026-05-16
("testing strategies to different trading pairs"). The operator
already has a strategy in mind; they want to ask "what happens if
I run **v1 momentum** on **ETHUSDT** from **2024-01-01 to
2024-06-30** with default parameters?" and see the answer in
seconds — equity curve, drawdown, fills, KPI strip — without
leaving the cockpit. Today this requires a `cargo run --bin
backtest` invocation.

**Proposed screen: `Lab` (new — the headline screen).** A
single-canvas workspace with:

- **Top bar (sticky):**
  - **Strategy chip** (selected from J1 or via dropdown)
  - **Pair chip** (selected from universe or via dropdown)
  - **Date-range picker** with named presets ("Last 30d",
    "Last 90d", "v25 BS-1 scenario", "v25 BS-2 scenario") +
    custom range
  - **Parameter editor toggle** (collapsed by default; expands
    a side-drawer mirroring the current `params_block` from
    `screens/strategies.rs`, editable)
  - **Run** button (primary; runs to completion in-process via
    the existing backtest engine; spinner while running)
- **Body (main):** The existing chart widget at full width, with:
  - Price line for the selected pair
  - **Fill markers** from the backtest result (the existing
    `chart::markers` data path)
  - **Equity overlay** on a secondary y-axis (toggle on/off; the
    existing `equity_curve` widget composed at chart scale)
  - **Drawdown band** below the chart (the existing
    `drawdown_band` widget; toggle on/off)
- **KPI strip (bottom row):** total return, Sharpe, Sortino,
  max drawdown, win rate, n trades — the existing
  `kpi_strip::view` over the backtest's `BacktestMetrics`
- **Action row (bottom):** **Save report** (writes a Markdown
  report to `spec/<strategy>/reports/` for the audit ledger);
  **Compare to baseline** (opens J3 in compare mode);
  **Promote → paper** (lifecycle gate — confirm-phrase dialog;
  defers to operator + tester per
  [product.md → Strategy lifecycle](../product.md#strategy-lifecycle--promotion-gates))

**Layout sketch:**
```
+────────────────────────────────────────────────────────────────+
│ Lab                                                            │
│ [v1.momentum ▾]  [ETHUSDT ▾]  [Last 90d ▾]  [params]  [▶ Run] │
+────────────────────────────────────────────────────────────────+
│                                                                │
│                    PRICE + FILL MARKERS                        │
│                                                                │
│             (with optional equity overlay)                     │
│                                                                │
+────────────────────────────────────────────────────────────────+
│ ▁▁▃▅▆▄▂▁▁ DRAWDOWN                                             │
+────────────────────────────────────────────────────────────────+
│ Total +12.4%  Sharpe 1.42  Sortino 1.88  MaxDD −8.1%  Win 54% │
+────────────────────────────────────────────────────────────────+
│           [Save report]  [Compare]  [Promote → paper]          │
+────────────────────────────────────────────────────────────────+
```

**Data sources.**
- Strategy registry — `Cockpit::strategies`
- Universe — `Cockpit::universe`
- Historical bars — existing `data` crate's Parquet loader (live
  cockpit already loads via `chart_buffer`; backtest invocation
  re-uses the same path)
- Backtest engine — existing `crates/backtest` (called as a
  library, not a binary)
- Results — `BacktestMetrics` + `EquitySeries` + `Vec<FillView>`
  flowing back into `Cockpit::lab_result` (new field; pure
  presentation state)
- Markdown report writer — existing `crates/reports` (called as
  library, not the test-time tool)

**Operations.** Run, Save report, Compare to baseline, Promote.
Parameter editor is in-screen — when the operator changes a
parameter and re-runs, the previous run's result becomes the
"baseline" automatically (one-click compare).

**Confirmation gates.**
- **Run** — non-destructive; spinner only.
- **Save report** — non-destructive (writes to disk under
  `spec/<strategy>/reports/`); shows a brief toast confirming
  the report path.
- **Promote → paper** — destructive (lifecycle change;
  modifies `config/agent.toml`); confirm-phrase dialog
  ("Type the strategy id to confirm") + tester-report gate
  per product.md lifecycle rules.

**Lumen primitives.** Tier 1 panel; chart canvas (existing);
`kpi_strip` (existing); `drawdown_band` (existing); modal Tier
3 for promote-confirm (`override_risk_veto` modal pattern
already established).

### J3 — Compare strategies across pairs

**Job framing.** "Of my 6 strategies × my 10 pairs, which
(strategy, pair) cells have the best Sharpe? Best drawdown?
Where does v2 LLM beat v1 momentum?" A matrix view, drillable
both ways.

**Proposed screen: `Compare` (new — secondary to Lab).** A
heatmap-style table:

- Rows: strategies (filter from registry)
- Columns: pairs (filter from universe)
- Cell: a small KPI summary card — Sharpe value, colored by sign
  (UP_500/DOWN_500/WARN_500), with a 30-bar sparkline of equity
  beneath
- Cell click → opens Lab seeded with that (strategy, pair, date-
  range) — preserving the current date-range picker selection

Above the matrix: the same date-range picker as Lab, plus a
KPI-axis toggle (Sharpe / Sortino / Total return / Max drawdown
/ Win rate).

**Layout sketch:**
```
+────────────────────────────────────────────────────────────────+
│ Compare  [Last 90d ▾]  [Sharpe ▾]                              │
+────────────────────────────────────────────────────────────────+
│              BTCUSDT  ETHUSDT  SOLUSDT  …  AVAX                │
│  v0.sma       0.94     n/a      n/a    …  n/a                  │
│  v1.momentum  1.42     1.18     0.88   …  0.61                 │
│  v1.5a.pairs  —        —        —      …  —                    │
│  v2.llm       1.31     1.05     0.71   …  0.48                 │
│  v2.5.tcn     1.38     n/a      n/a    …  n/a                  │
+────────────────────────────────────────────────────────────────+
```

**Data sources.** Same as J2; the matrix is *n × m* backtest
invocations. v1 ships a **cache** — if `(strategy, pair, range)`
already has a report in `spec/<strategy>/reports/`, the matrix
reads the report's frontmatter for the KPIs and shows a "cached"
badge; the operator can click "Recompute" to force a re-run.

**Operations.** Toggle KPI axis, recompute, click cell → drill
into Lab.

**Confirmation gates.** None on the matrix (read-only / runs
backtests).

**Lumen primitives.** Tier 1 panel; mini-`kpi_strip`-style cell;
`active_row` border tint for hover.

### J4 — Inspect a single agent decision

**Job framing.** The headline differentiator. From any fill row,
the operator clicks **Why?** and gets a vertical walk through
the decision chain: analyst opinions → debate transcript →
trader proposal → risk clamps → PM approval → fill. Every node
links to its inputs (bar data, feature snapshot, LLM prompt,
forecaster overlay).

**Proposed screen: `Trail` (replaces current Audit).** Three
modes:

1. **Search mode** (default) — what the current `audit.rs`
   filter+table is, but every row has a chevron icon that
   means "click to walk the chain".
2. **Trail mode** (new) — opened by clicking a row's chevron.
   Renders a vertical timeline of the decision pipeline, with
   each stage as a Tier 1 panel:
   ```
   ┌─ FILL  2024-01-15T14:23:01.418Z  BTCUSDT  BUY 0.05  @ 41,820
   │
   ├─ PM    APPROVE   confidence 0.82   risk-clamped from 0.10 → 0.05
   │
   ├─ RISK  ✓ vol     ✓ liquidity      ✗ correlation (re-sized)
   │       Reason: portfolio_btc_exposure 18% > 15% cap
   │
   ├─ TRADER  BUY 0.10  TIF: IOC  invalidation: 41,500
   │         "Bull thesis dominant; news + macro both BUY"
   │         [show full thesis]
   │
   ├─ DEBATE  bull > bear  3 rounds  consensus 0.74
   │         [show transcript]
   │
   ├─ ANALYSTS (parallel)
   │   ├─ fundamentals  BUY  0.70  [show evidence]
   │   ├─ sentiment     BUY  0.81  [show prompt + response]
   │   ├─ news          HOLD 0.55  [show 3 headlines]
   │   ├─ technical     BUY  0.79  forecast overlay σ=0.014
   │   │                [show TCN forecast]
   │   └─ macro         HOLD 0.50  (gated off in v1)
   │
   └─ INPUTS  bar 14:22  feature snapshot  market health: fresh
   ```
3. **Forecaster diff mode** (when a forecast overlay was
   present) — opens the chart at the bar that triggered the
   trade and overlays the TCN/PatchTST/Transformer prediction
   bands alongside the actual price path, so the operator can
   answer "was the forecast right?".

**Data sources.**
- Audit ledger (already provides the fill row)
- Decision-trail rows — **cross-cutting question:** the audit
  ledger may not currently persist enough of the analyst opinions
  / debate transcript / LLM prompt to render this view. v2 LLM
  shipped under [`v2-llm-strategy`](../v2-llm-strategy/feature.md);
  the analyst should confirm the schema covers the trail
  (open question Q1 in § 5).
- Forecast bands — `crates/forecast` (v2.5 model serving)

**Operations.** Click any link → open a side-drawer with the raw
artifact (prompt, response, transcript, forecast tensor). Copy
prompt/response to clipboard for off-line analysis. **Save trail**
→ generate a Markdown brief for the strategy's reports folder.

**Confirmation gates.** None — read-only.

**Lumen primitives.** Tier 1 panels stacked vertically; each
stage is a panel with `active_row` left-rule when expanded;
side-drawer is a Tier 2 raised panel; transcript view uses
the existing markdown body renderer from `viewer.rs`.

### J5 — Override / intervene

**Job framing.** The operator sees something they don't like and
wants to act: stop the agent, kill a strategy, close a position,
override a risk veto, pause a pair from trading. **Reachable
without leaving the current screen.**

**Proposed surface: `Action bar` (always visible) + per-row
context actions.** Replaces the current Control screen as the
primary surface (the screen survives as a "Settings" detail).

- **Status bar (bottom of shell):** A small red dot is the kill
  affordance. Click → confirm-phrase dialog (`STOP`) → trip
  the kill switch. Same dialog as today, just always reachable.
- **Per-position close button:** On the Positions panel
  (existing widget; surfaced in the Pair screen — see § 2.6
  below — and as a J6 monitor card). Click → confirm-phrase
  dialog (typed symbol) → flatten that position.
- **Per-strategy pause:** Existing `strategies_widget::pause_button`
  pattern; reachable from the Pair screen's strategy chip row
  (right-click or long-press menu).
- **Risk veto override:** Existing `override_risk_veto::modal_view`;
  surfaced as a banner above the Trail view when a recent
  veto is unresolved. The current Strategies-screen veto row
  migrates here.
- **Pause pair:** New affordance — operator-issued
  `Message::PausePair(Venue, Symbol)` that takes the pair out
  of every active strategy's universe until resumed. Useful
  during a venue outage. Cross-cutting question Q2.

**Confirmation gates.**
- **Kill switch:** typed `STOP` (existing).
- **Close position:** typed symbol id (e.g. `BTCUSDT`).
- **Pause strategy:** single click (reversible — Resume is one
  click).
- **Override risk veto:** typed `OVERRIDE` + the veto reason
  echoed back (already implemented in `override_risk_veto`).
- **Pause pair:** single click (reversible).

**Lumen primitives.** Tier 3 modals for destructive gates;
Tier 2 dropdown for the per-position context menu; status
bar (already shipped) for the kill dot.

### J6 — Monitor live (paper) session

**Job framing.** The "is this working?" tick. Equity curve,
open positions, recent agent activity, LLM spend (v2+), system
health, latency, kill-threshold proximity — at a glance.

**Proposed screen: `Live` (replaces current Home, demoted from
default).** A monitor dashboard, but the **default screen** is
now Lab (J2) — Live is what the operator opens when they want
to *watch*, not *work*.

- **Equity curve** (full width, top) — running paper-session
  equity, with annotations for fills + LLM spend ticks
- **KPI strip** — today's realized P&L, today's unrealized P&L,
  today's trade count, today's win rate, today's LLM spend ($)
- **Open positions** (table — existing widget)
- **Recent agent feed** (existing widget) — but enriched: each
  entry has a "Trail" chevron that jumps to J4 for that decision
- **System health row** (compact) — venue latency dots, market-
  health pills, server-time skew, kill-threshold gauge, version
  badge. The current Debug screen contents collapsed onto one
  row.

**Layout sketch:**
```
+────────────────────────────────────────────────────────────────+
│ Live   ●online  lat 32ms  ●halt-ready  v0.13.0  llm $0.42/$20 │
+────────────────────────────────────────────────────────────────+
│           EQUITY CURVE (with fill + spend annotations)         │
+────────────────────────────────────────────────────────────────+
│ +$124  +$28  3 trades  67% win  $0.42 LLM                      │
+────────────────────────────────────────────────────────────────+
│  OPEN POSITIONS               │  RECENT ACTIVITY               │
│  BTCUSDT  +0.05  +$28  +0.7%  │  14:23  BUY 0.05 BTC  Why?     │
│  ETHUSDT  +0.20  +$50  +1.1%  │  14:18  RISK clamp 0.10→0.05   │
│  …                            │  …                             │
+────────────────────────────────────────────────────────────────+
```

**Data sources.** All existing — `Cockpit::pnl`,
`Cockpit::positions`, `Cockpit::strategies_recent_events`,
`MarketHealth`, latency. New: per-day LLM spend cumulative
(reads from `crates/llm` budget tracker — already exists for
the 80%/100% auto-degrade rule per
[product.md → Cost economics](../product.md#cost-economics--monthly-ceiling)).

**Operations.** Click any row → open Trail (J4); click position
row → close-position dialog (J5).

**Confirmation gates.** None on the screen itself; flows lead
into J5 for destructive paths.

**Lumen primitives.** Tier 1 panels in a 2-column grid; existing
chart widget for equity curve; status row uses the existing
status-bar style at full width.

### J7 — Inspect the reflection memory

**Job framing.** "What did the agent learn last session? What
lessons will it retrieve for tomorrow's first decision?" The
locked moat made visible.

**Proposed screen: `Memory` (new).** A list view with two
modes:

1. **Cards mode** (default) — one card per `lesson_card` row
   from the reflection-memory store. Each card shows: the
   trade context (pair + date + outcome), the lesson body (LLM
   summary), the retrieval-relevance score, and a "Was this
   used recently?" stamp (which decisions in the last N days
   retrieved this card).
2. **Cluster mode** — the weekly-distilled cluster view (per
   `reflection-memory/feature.md` R8) — promoted-to-prompt-library
   rules vs candidate clusters.

Each card has a chevron → Trail view for the trade that produced
it.

**Data sources.** `crates/reflection_memory` (already exists);
audit ledger for the source trade. Read-only.

**Operations.** Click a card → Trail view of the source trade.
Promote-cluster-to-rule (writes to a prompt-library file under
`config/`) — destructive (modifies prompt library); confirm-
phrase gate.

**Confirmation gates.** Promote-cluster — typed cluster name.

**Lumen primitives.** Tier 1 cards in a vertical scroll; markdown
body renderer for the lesson body.

### J8 — Inspect a model version

**Job framing.** v2.5 TCN, v2.5a PatchTST, v2.5b Transformer,
v2.6 bake-off. The operator wants to know: which checkpoint
is currently serving? When was it trained? What was the val
loss? What's the sigma_train? Is the forecast quality drifting?

**Proposed screen: `Models` (new).** A list view, one row per
trained model checkpoint:

- Model id (`v2.5.tcn-checkpoint-2026-05-15`)
- Family (TCN / PatchTST / Transformer)
- Training data span (`2023-01-01..2024-12-31 BTCUSDT 1m`)
- Val loss, train loss
- `sigma_train` (the calibration constant the trader uses)
- Checkpoint SHA, file size
- **Status pill** — serving / staged / archived
- **Forecast-quality sparkline** — last-N-bars actual vs
  predicted residuals; flat-near-zero is good

Click a row → detail view with:
- Full training metadata block
- Calibration plot (predicted vs realized; cheap to render if
  the residual series is already on disk)
- Audit ledger query: which decisions in the last week consumed
  this checkpoint?
- **Promote / archive / unload** actions (destructive — gated)

**Data sources.** `crates/forecast` model registry (already
exists per the v2.5 roadmap); audit ledger for consumption
queries.

**Operations.** Click row → detail; promote/archive/unload.

**Confirmation gates.**
- **Promote** — typed checkpoint SHA prefix.
- **Archive** — typed checkpoint SHA prefix.
- **Unload (force)** — typed `UNLOAD` + the model's id.

**Lumen primitives.** Tier 1 list rows; Tier 1 detail panel;
existing sparkline widget for forecast-quality plot.

### Mapping summary — jobs to screens

| Job | Screen | Status |
|---|---|---|
| J1 — Pick a strategy | `Strategy registry` | Refactored from current Strategies |
| **J2 — Test strategy × pair × range** | **`Lab` (new — default)** | **New; absorbs current Charts + viewer** |
| J3 — Compare across pairs | `Compare` | New |
| J4 — Inspect agent decision | `Trail` | Refactored from current Audit |
| J5 — Override / intervene | `Action bar` + context menus | Refactored from current Control |
| J6 — Monitor live | `Live` | Refactored from current Home + Debug |
| J7 — Reflection memory | `Memory` | New |
| J8 — Model provenance | `Models` | New |

## 3. Per-pair-first navigation pattern

> The operator on 2026-05-16: *"Consider me as human in control,
> including testing strategies to different trading pairs."*
>
> This is a navigation question, not a screen-content question.
> Three candidates:
>
> 1. **Pair-first** — sidebar = active pairs; pick BTCUSDT and
>    every screen reframes to that pair.
> 2. **Strategy-first** — sidebar = strategies; pick v1.momentum
>    and every screen reframes to that strategy.
> 3. **Hybrid** — top-level Compare matrix; drill-down both
>    ways.

### Recommendation: **Hybrid with strategy-leading**

**The default entry point is the strategy.** The operator
typically opens the cockpit with a strategy hypothesis already
in mind — they want to know "does v1 momentum work on ETH?",
not "what's interesting about ETHUSDT?". Strategy → pair is
the **typical workflow order**.

**The pair is a first-class chip, not a separate sidebar.** Once
the operator picks a strategy (J1), the Lab (J2) presents a
pair chip at the top of the canvas. That chip is the cross-cut
into the strategy's universe — clicking it cycles through
pairs. The operator can also enter the Lab pre-pair-selected
from the J3 Compare matrix.

**The Compare matrix is the bidirectional bridge.** Operator
who wants the pair-first mental model opens Compare with the
column axis selected (pairs); clicking a pair column header
fixes the pair and shows all strategies for it. Operator who
wants strategy-first uses the row axis. **Same screen, two
mental models.**

**Why not pair-first sidebar:**

- Strategies have *universes*; universes contain pairs. Putting
  pairs in the sidebar inverts the data hierarchy.
- v1.5a pairs strategies are **two pairs at once**; a pair-first
  sidebar can't represent them cleanly (the strategy spans the
  ETH and BTC rows).
- v1 momentum has 10 symbols; pinning one pair in the sidebar
  hides the cross-sectional view that makes the strategy work.
- The Lumen design system's sidebar metaphor is "modes of the
  app"; pairs are not modes, they're parameters.

**Why not strategy-only sidebar:**

- Some operator workflows are pair-driven (e.g. "ETHUSDT is
  acting weird tonight; how are my strategies handling it?").
  Compare matrix answers this; a strategy-only sidebar wouldn't.
- The Live monitor (J6) is portfolio-wide; sidebar selection
  shouldn't override it.

**Why the hybrid Compare matrix:**

- Both mental models read off the same data (strategy × pair ×
  range Sharpe). The matrix is the natural home for "which
  cells matter?".
- Cell click → Lab. Row header click → Lab with strategy locked,
  pair-chip pickable. Column header click → Lab with pair
  locked, strategy-chip pickable. Three drill-downs from one
  surface.

**Implication for the sidebar:**

```
Sidebar (top to bottom):
  Lab          (J2 — default; the workshop)
  Live         (J6 — the monitor)
  Compare      (J3 — the matrix)
  ─────
  Strategies   (J1 — the registry)
  Memory       (J7 — the lessons)
  Models       (J8 — the checkpoints)
  Trail        (J4 — the audit-as-story; reachable inline too)
  ─────
  Settings     (J5 + current Risk/Debug/Control collapsed)
```

The top group is the **everyday workflow** (3 entries; muscle
memory). The middle group is the **library** (4 entries). The
bottom is the **ops drawer** (1 entry that opens a tabbed
detail with Risk / Debug / Control / Settings).

**Reference: v1 momentum's 10-symbol universe + v2.5 BS-1/BS-2
scenarios.** These confirm the hybrid recommendation. v1's
universe demands a strategy view that *aggregates* across
pairs (Compare row). BS-1 (BTC) vs BS-2 (ETH+BTC) demand
side-by-side comparison (Compare row at two columns). Neither
fits a pair-first sidebar.

## 4. What to keep / change / drop

### Keep (refactored or as-is)

| Asset | Disposition |
|---|---|
| **Lumen design system (Phase 1 tokens)** | As-is — the executable contract in `theme.rs` is the floor |
| **Lumen Phase 2-5 widgets** (`frame`, `kpi_strip`, `equity_curve`, `drawdown_band`, `sparkline`, `volume_histogram`, `agent_feed`, `human_control`, `override_risk_veto`, `journal_transaction_modal`, `latency`, `kill`, `pnl`, `positions`) | As-is — every one of these survives into the new screens |
| **Chart canvas** (`widgets/chart.rs` + `canvas_chart.rs` + `chart_legend.rs` + `chart_tooltip.rs`) | As-is, but **becomes the centerpiece** of the Lab + Trail + Models screens |
| **Status bar** (`widgets/status_bar.rs`) | As-is; gains the kill-dot for J5 |
| **`screens/charts.rs`** | Refactored → `screens/lab.rs` (renamed + extended with date-range picker + run button + KPI strip integration) |
| **`screens/strategies.rs`** | Refactored → `screens/strategy_registry.rs` (list-of-cards mode; the params block migrates into Lab as a side-drawer) |
| **`screens/risk.rs`** | Refactored → tab inside `Settings` (J5); the exposure bars stay |
| **`screens/control.rs`** | Demoted — content migrates to `Settings`; kill dot to status bar |
| **`screens/debug.rs`** | Demoted — content migrates to `Settings` ops tab + the system-health row in Live |
| **`screens/audit.rs`** | Refactored → `screens/trail.rs` (filter+table → search mode; new Trail mode is the headline addition) |
| **`screens/home.rs`** | Refactored → `screens/live.rs` (the 2×2 grid pattern survives but with enriched panels) |
| **`viewer.rs` binary** | Kept as-is for opening *committed* report files (PR diffs, presenter handoffs). Lab subsumes its authoring role |

### Change (substantial refactor required)

- **Sidebar IA** — replace `[Home, Charts, Strategies, Risk, Audit,
  Control, Debug]` with the three-group structure proposed in § 3.
- **Default screen at boot** — `Home` → `Lab`. The operator's
  first sight of the cockpit is the workshop, not a dashboard.
- **Audit row click** — currently opens `journal_transaction_modal`;
  changes to opening the Trail view (the modal becomes the
  inputs side-drawer inside Trail).
- **`Message::SwitchScreen`** — extended with new screen variants
  (`Lab`, `Compare`, `Memory`, `Models`, `Trail`, `Live`,
  `Settings`); old variants deprecated with a one-cycle
  compatibility shim so the test harness migrates.
- **Backtest path** — currently CLI-only (`cargo run --bin
  backtest`); needs to be reachable as a library call from the
  Lab screen's `Run` button. **Cross-cutting** — see Q3.

### Drop (or merge)

- **`screens/home.rs` as a separate screen** — content reframed
  into Live (J6).
- **`screens/audit.rs` as a "browse the ledger" surface** — the
  flat-table affordance survives inside Trail's search mode
  but is no longer the primary view.
- **`screens/control.rs` as a top-level screen** — content into
  Settings + status bar.
- **`screens/debug.rs` as a top-level screen** — content into
  Live (status row) + Settings (ops tab).
- **Sidebar nav `[Home, Charts, Strategies, Risk, Audit, Control,
  Debug]`** — replaced by the new three-group sidebar.

### New (no existing surface)

- **`screens/lab.rs`** — J2; the headline new screen.
- **`screens/compare.rs`** — J3; the matrix.
- **`screens/strategy_registry.rs`** — J1 (refactor of strategies.rs
  in practice; new in shape).
- **`screens/trail.rs`** — J4 search+trail mode.
- **`screens/live.rs`** — J6 (refactor of home.rs).
- **`screens/memory.rs`** — J7.
- **`screens/models.rs`** — J8.
- **`screens/settings.rs`** — J5 ops drawer (Risk / Debug / Control
  / kill + bookkeeping settings).

### Design-system primitives — what survives, what extends

**Survives unchanged:** every Lumen Phase 1 token (color, type,
spacing, radius, motion, shadow); every Phase 2-5 widget; the
shell composition (sidebar | body | status-bar | right-rail).

**Extends (additive only; no breaking changes):**
- **Date-range picker widget** — new (`widgets/date_range.rs`).
  Used by Lab and Compare. Token-only styling; no new tokens.
- **Strategy chip + pair chip widget** — small extension to
  `widgets/strategies.rs` chip styling, parameterised on
  "strategy" vs "pair" semantics. No new tokens.
- **Matrix widget** — new (`widgets/matrix.rs`). A grid of mini-
  KPI cells. No new tokens; reuses card chrome from `frame.rs`.
- **Trail node widget** — new (`widgets/trail_node.rs`). Tier 1
  panel with a left-rule connector to the next node. Uses
  existing accent tokens for active-row pattern.
- **Lesson card widget** — new (`widgets/lesson_card.rs`). Tier
  1 card with a markdown body. Reuses the viewer's markdown
  renderer.
- **Model row widget** — new (`widgets/model_row.rs`). Same
  shape as the strategy registry row, different fields.

**No new theme tokens** are required by the redesign. If one
becomes necessary during implementation, that is a smell and
should be flagged to the operator.

### Headline counts

- **Screens kept** (refactored or as-is): **3** (Charts→Lab,
  Strategies→Registry, Audit→Trail are refactors of existing
  files; home/risk/control/debug are demoted/merged but their
  code survives).
- **Screens dropped/merged**: **4** (Home, Risk, Control,
  Debug as top-level screens; their content survives elsewhere).
- **Screens new**: **5** (Lab as a refactor + 5 new modes —
  Compare, Memory, Models, Settings, Live's redesigned
  composition).
- **Widgets new**: **6** (date-range, strategy/pair chip
  extension, matrix, trail-node, lesson-card, model-row).
- **Widgets kept**: **18** (every Lumen Phase 1-5 widget
  survives).
- **Lumen tokens changed**: **0**.

## 5. Open questions for the operator

Five-to-eight crisp questions, each with a recommended default
and the cost of being wrong. The operator answers these before
any implementation work spawns.

### Q1 — Does the audit ledger schema cover the J4 Trail view?

**The question.** The Trail view needs analyst opinions, debate
transcripts, trader proposals, risk clamps, PM decisions, LLM
prompt/response pairs, and forecast bands all reconstructable
from a fill row. Does the v2 LLM audit-ledger schema (per
[`v2-llm-strategy/feature.md`](../v2-llm-strategy/feature.md))
already persist this, or do we need new writers?

**Recommended default.** Assume **yes for analysts + debate +
trader + risk + PM** (the multi-agent pipeline shipped per
product.md roadmap), **no for LLM prompt/response and forecast
bands** (those may be opt-in for cost reasons). Phase 1 ships
Trail with the agent-pipeline portion fully wired and the
LLM/forecast nodes as collapsed "[show prompt — not persisted]"
placeholders.

**Cost if wrong.** If the schema is thinner than assumed, Trail
ships with empty nodes (degrades gracefully — every node already
has an `Empty` state per the human-friendly rule). New audit
writers are a cross-cutting backend feature, not a UI one.

### Q2 — Should "pause pair" be a J5 affordance?

**The question.** The operator can pause a strategy today (J5);
should there be a parallel "pause pair" — "BTC is acting weird,
stop trading it until I resume"? Today this requires editing
`config/agent.toml` or pausing every strategy that touches BTC.

**Recommended default.** **Yes.** Add `Message::PausePair(Venue,
Symbol)` in Phase 2 of the rollout. The cost is small (one
strategy-engine field), the operator value is high (matches
the v1 / v1.5b multi-venue era's failure-isolation need).

**Cost if wrong.** A "pause pair" affordance that no operator
ever uses; one extra Message variant.

### Q3 — Can the backtest engine be called as a library from the cockpit?

**The question.** Today `cargo run --bin backtest` is a separate
binary. The Lab `Run` button needs to invoke the engine
in-process. Is the engine library-callable today, or is it
binary-coupled?

**Recommended default.** Assume **the engine is library-callable**
(it lives in `crates/backtest`; the binary is a thin wrapper).
If the in-process call path needs refactoring, treat it as a
prerequisite spawned ahead of Lab's phase-1 ship — this is the
load-bearing dependency.

**Cost if wrong.** If the engine is binary-coupled, Lab's `Run`
button needs an out-of-process invocation (cockpit shells out
to `cargo run --bin backtest`, parses the report). That works
but it's ugly; the in-process path is the right answer.

### Q4 — Should Lab persist its current (strategy, pair, range,
params) tuple across cockpit restarts?

**The question.** The operator's last Lab session was
"v1.momentum × ETHUSDT × Last 90d". Should re-opening the
cockpit re-seed Lab with that selection, or start fresh?

**Recommended default.** **Persist.** Operator workflows are
iterative — they tweak a parameter, re-run, tweak another,
re-run. Losing the tuple on restart is friction. Store as a
small JSON blob under `~/.config/trading/cockpit-lab-state.json`.

**Cost if wrong.** A useless cache file; one cycle to add a
"start fresh on launch" toggle.

### Q5 — Where does the `Models` screen get its data?

**The question.** v2.5 / v2.5a / v2.5b / v2.6 are queued
(`status: draft` / `in-progress`). They will produce trained
checkpoints + training metadata. Is there a model registry today
(`crates/forecast`?) that the Models screen reads, or does this
screen wait on v2.6's bake-off-ready registry?

**Recommended default.** Models ships **alongside v2.5** (the
first v2.5 checkpoint is the first row in the table). v2.5 will
write metadata to disk anyway; the screen reads what's there.
Bake-off-style comparison features defer to v2.6.

**Cost if wrong.** Models renders an empty list for a few weeks
until v2.5 lands its first checkpoint — the screen's empty
state ("No models trained yet. Train one with `cargo run --bin
train-v25-tcn`") is the affordance.

### Q6 — Does the right-rail Assistant (Phase 6 Lumen) ship in
this rethink or stay deferred?

**The question.** Phase 6 of the Lumen roadmap reserves a
right-rail Assistant slot for v2 LLM conversation. v2 LLM
shipped; Phase 6 is technically unblocked. Does it ship with
this redesign or stay deferred?

**Recommended default.** **Stay deferred for one cycle.** The
J2 vertical is the primary value of this rethink; the Assistant
slot is a separate operator-conversation feature. Ship the IA
rework first, then spawn Phase 6 against the new IA (the slot
shape doesn't change).

**Cost if wrong.** One redundant prompt cycle if the operator
wanted Assistant alongside the redesign.

### Q7 — Pair-picker ordering — alphabetical, by volume, by
position size, or by recent activity?

**The question.** The Lab pair chip cycles through the universe.
What's the default order?

**Recommended default.** **By recent activity** (most-recent
fill timestamp, descending). Falls back to alphabetical for
pairs with no recent fills. Matches the operator's likely
attention: "which pair did I last care about?".

**Cost if wrong.** One extra setting toggle in Settings to
switch to alphabetical or volume-weighted.

### Q8 — Does the Compare matrix's "Recompute" run in the
foreground (blocking the UI) or background (non-blocking with
a progress strip)?

**The question.** A full recompute is N strategies × M pairs
backtests; for v1 momentum at 10 pairs × 6 strategies =
60 backtests. On a developer laptop that's minutes.

**Recommended default.** **Background, with a status-bar
progress strip and per-cell spinner.** The operator can keep
working in Lab while the matrix fills in.

**Cost if wrong.** Synchronization complexity if the operator
edits a strategy parameter mid-recompute. Mitigation: stale
cells get a "stale" badge when their inputs change.

## 6. Migration sequencing — high-level phases

Not a `tasks.md`. The point is the sequencing argument: which
slice ships first to test the hypothesis, what depends on what,
where the rollback cliffs are.

### Phase A — Lab vertical (the thin slice)

**Goal.** Prove the J2 workflow against fixtures + cached
backtest results, behind a feature flag. **One screen, one
job.**

- Add `Screen::Lab` to the enum + sidebar entry behind a
  `lab-screen` feature flag.
- Reuse the existing chart widget unchanged.
- Wire `Cockpit::lab_state` (strategy + pair + date-range +
  params + result).
- Phase-A shortcut: the `Run` button reads a **pre-computed**
  report from `spec/<strategy>/reports/` if one matches the
  tuple; otherwise renders "Run backtest" with a CLI hint
  ("run `cargo run --bin backtest --strategy v1.momentum
  --pair ETHUSDT --range last-90d` then refresh"). This
  defers Q3's library-call decision while shipping value.
- Save-report button writes to the existing report directory
  shape (no schema changes).

**Operator review checkpoint.** Does the Lab shape feel
right? If yes → Phase B. If no → cheap rollback (one screen,
one feature flag, zero touched anchors, zero touched non-UI
crates).

**Cost.** ~1-2 weeks. **Anchor risk: zero** (read-only over
existing reports + existing chart + existing audit data).

### Phase B — Backtest engine as library + Lab Run button

**Goal.** Make the J2 `Run` button a real in-process backtest.

- Backend cross-cut: confirm `crates/backtest` is library-callable
  (Q3); refactor if not.
- Wire the Lab screen's `Run` button to call the engine and
  populate `lab_state.result`.
- Add the "compare to previous run" affordance.

**Operator review checkpoint.** Does running a backtest from
the cockpit feel right? Is the spinner shape acceptable?

**Cost.** ~1-2 weeks. **Anchor risk: low** (new library call
path; no committed report bodies change).

### Phase C — Sidebar IA flip + Live + Strategy registry

**Goal.** Replace the seven-screen sidebar with the new IA;
demote Home/Risk/Control/Debug.

- New sidebar with the three-group structure from § 3.
- `Live` screen lands as the redesigned Home.
- `Strategy registry` lands as the redesigned Strategies.
- `Settings` lands as the Risk + Control + Debug rollup.
- Old sidebar entries removed; `Message::SwitchScreen`
  variants deprecated with a compatibility shim for one
  cycle.

**Operator review checkpoint.** Does the new sidebar feel
better? Does muscle memory transfer?

**Cost.** ~2-3 weeks. **Anchor risk: zero**.

### Phase D — Trail view (J4) — the differentiator made visible

**Goal.** The decision-trail mode of the Audit-now-Trail
screen.

- Confirm Q1 schema coverage; new audit-ledger query method
  if needed (new field — additive — no anchor risk).
- New `screens/trail.rs` + `widgets/trail_node.rs`.
- Side-drawer for raw artifacts (LLM prompt, debate
  transcript, forecast tensor).
- Trail chevron added to Live's recent-activity rows and to
  the Audit-now-Trail search-mode rows.

**Operator review checkpoint.** Does the trail tell the story
the operator wants? Is the agent-pipeline visualisation
right?

**Cost.** ~3-4 weeks (depends on Q1 schema gap). **Anchor
risk: low** (potentially new audit writers; additive only).

### Phase E — Compare matrix (J3)

**Goal.** Matrix view over strategy × pair × range with the
report-cache shortcut.

- New `screens/compare.rs` + `widgets/matrix.rs`.
- Report-cache lookup over `spec/<strategy>/reports/`
  frontmatter.
- Recompute orchestration (Q8 background).
- Cell click → Lab seeded.

**Operator review checkpoint.** Does the matrix shape work?
Are the KPI cells legible at 6×10?

**Cost.** ~2-3 weeks. **Anchor risk: zero**.

### Phase F — Memory + Models + Phase-6 Assistant slot

**Goal.** Round out the IA with J7 + J8 + the deferred
Lumen Phase 6.

- `screens/memory.rs` over `crates/reflection_memory`.
- `screens/models.rs` over `crates/forecast` registry
  (gated on v2.5 landing a checkpoint per Q5).
- Right-rail Assistant slot wakes for v2 LLM (per Phase 6
  Lumen contract — slot reservation already exists in
  `shell.rs`).

**Operator review checkpoint.** Final sweep — anything missing?

**Cost.** ~3-4 weeks. **Anchor risk: zero**.

### Phase ordering summary

```
A (Lab thin slice) → B (Lab Run) → C (Sidebar IA flip)
                                 ↓
                                 D (Trail)
                                 ↓
                                 E (Compare)
                                 ↓
                                 F (Memory + Models + Phase 6 slot)
```

- **Cliff: Phase A operator review.** If the Lab shape is
  wrong the rest of the redesign needs reshaping. Phase A
  exists to test the hypothesis cheaply.
- **Cliff: Phase D Q1 schema confirmation.** If the audit
  ledger doesn't cover the trail, Phase D becomes a backend
  feature (write new audit rows for the multi-agent
  pipeline) and is no longer purely UI.
- **No cliffs at C, E, F** — each phase is independently
  shippable and independently reversible.

**Total cost estimate (rough):** 12-18 weeks across 6 phases,
with operator-review gates after each phase. The orchestration
is week-scale, not month-scale, because each phase ships a
*new screen the operator can use today*.

## Addendum — Operator answers + investigations (2026-05-17)

### Operator answers to the 8 questions (Section 5)

| Q | Resolution |
|---|-----------|
| Q1 — Audit ledger covers Trail? | **Investigate** → resolved below |
| Q2 — "Pause pair" as J5 affordance? | **Yes** |
| Q3 — Backtest engine library-callable? | **Yes, should be — investigate** → resolved below |
| Q4 — Lab persists `(strategy, pair, range, params)` tuple? | **Yes (default accepted)** |
| Q5 — Models screen data source? | **Investigate** → resolved below |
| Q6 — Phase-6 Assistant slot in this rethink? | **Stay deferred** (not in Phase A-E scope) |
| Q7 — Pair-picker default order? | **XRP first, then ETH, then v1 universe alphabetical** (operator personal preference) |
| Q8 — Compare matrix recompute foreground or background? | **Background** |

### Investigation findings

**Q1 — Audit ledger schema coverage for Trail (~70%, no cliff).**
Migrations 001-009 already include strategy_events (002), strategy_signals (009),
journal_transactions_strategy_id (004), strategy_events_venue (007), plus the
financial ledger (001). "Bar → features → signal → fill → P&L" chain mostly
covered. NOT first-class today: LLM prompt SHA, forecast overlay confidence,
risk clamp reason. Phase D prep: either metadata-JSON convention OR small
dedicated migration. Bounded extension, not a cliff.

**Q3 — Backtest engine library-callable (yes, needs tightening).**
`crates/backtest/Cargo.toml` declares the bin AND `lib.rs` exposes
`pub mod engine; pub mod paper;`. Public surface today is sparse — needs
~1 day of API tightening for `backtest::engine::run_scenario(strategy, pair,
range, params) → Result<Report>` for inline Lab use. Phase B unblocked.

**Q5 — Models screen data source (multi-source, all on disk already).**
- Training-run provenance: `crates/forecast/checkpoints/anchors/*.{safetensors,metadata.json}` (TCN BS-1 present 2026-05-17; BS-2 imminent)
- Reflection memory: `crates/reflection/src/store/`, lesson cards, embeddings, retrieval
- Inference cache: `crates/replay-cache/` namespace `"forecast"` — calibration history

No new backend work for the Models screen.

### Operator-added constraint: charts are the door

> *"Put a lot of work into charts. This is the door to me. I want to see on
> the chart how successful the strategy select is. How much money is before,
> after, and also compare it to result of other strategies. Also it would be
> nice to have the overlay working for buy and sell positions."*

The Lab screen becomes **chart-centric** with three overlay layers fused on
a single canvas:

| Overlay | Status | Effort |
|---------|--------|--------|
| Buy/sell markers on price | `chart-buy-sell-emphasis` v1.10.0 shipped 2026-05-12 — wire into Lab | trivial |
| **Equity curve** (capital before/during/after this strategy run) | **NEW** — backtest engine emits per-bar equity; render as second Y-axis | medium |
| **Multi-strategy comparison** (≤4 strategies' equity curves on same pair/range) | **NEW** — color-coded lines on same canvas | medium |

Existing foundation: `chart.rs` (1537 LOC) + `chart_legend.rs` (565) +
`chart_tooltip.rs` (562) + `screens/charts.rs` (597). Phase A adds overlay
layers + data plumbing, not a rewrite.

### Adjusted Phase A scope (chart-centric Lab)

| Item | Effort |
|------|-------:|
| Rename `screens/charts.rs` → `screens/lab.rs`; set as default route | trivial |
| Wire existing buy/sell markers onto Lab chart | trivial |
| New widget: date-range picker pinching chart range | medium |
| New widget: pair chip — **XRP, ETH, BTC, …v1-universe-alpha** ordering | small |
| New widget: strategy chip on chart (port from registry) | small |
| New widget: equity-curve overlay (second Y-axis, cached reports at Phase A) | medium |
| New widget: comparison-line overlay (≤4 strategies, color-coded) | medium |
| Read-only at Phase A — cached backtest reports + fixtures (Phase B wires live engine) | — |
| Lab persists `(strategy, pair, range, params)` per Q4 | small |

**Net Phase A effort:** ~2 weeks (unchanged from original), but deliverable
is chart-centric with three named overlays — not generic Lab.

### Next step

HANDOFF → analyst to author the formal `ui-rethink-phase-a-lab` feature folder
based on the locked direction above.

## Changelog

- 2026-05-17 (orchestrator): addendum — operator answered 8 questions
  (Q2/Q4/Q6/Q7/Q8 directly; Q1/Q3/Q5 delegated to orchestrator
  investigation, all three resolved as "no cliff"). Operator added the
  load-bearing **charts-as-door** constraint with three named overlays
  (buy/sell markers, equity curve, multi-strategy comparison).
  XRP-first pair ordering locked. Phase A scope re-cast as chart-centric
  Lab.
- 2026-05-17 (ui-designer): initial dev-note — screen-by-screen
  redesign proposal in response to operator critique
  "I don't like the UI very much. It does not seem like a good
  fit for this project." Eight jobs-to-be-done, hybrid-with-
  strategy-leading navigation, six-phase migration with operator
  review gates between each. Single dev-note output per the
  brief; no feature folder spawned yet. Predecessors not
  re-litigated: ui-testability-deep-dive-2026-05-15,
  ui-testing-direction-2026-05-12, lumen-design-adoption Phase
  1-5 contracts, chart-canvas-overhaul history.
