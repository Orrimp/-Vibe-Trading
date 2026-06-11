---
slug: lumen-design-adoption
status: roadmap
owner: analyst
updated: 2026-05-04
version: 2.0.0
---

# Lumen design-system adoption — master roadmap

> **This is a roadmap, not a feature brief.** It does not ship through the
> analyst → architect → developer → tester → presenter gate as a single
> unit. It is the **contract** for spawning six sequential per-phase
> features (Phase 1 shipped; Phases 2 → 6 queued, with Phase 6 reserved
> until v2 LLM lands). Each phase is its own
> `spec/lumen-design-adoption/phase-N-<name>/feature.md` and runs the standard pipeline.
> The master file tracks the rollout, names the invariants, and records
> cross-phase decisions the operator already locked.
>
> **Roadmap revision 2026-05-04 (post Phase 1 ship).** The original
> 4-phase plan is **superseded** by a 6-phase plan that bundles the
> operator-requested **sidebar IA + price chart** ahead of the original
> Backtest panel and HumanControl phases. The original Phase 2 (Backtest)
> is now Phase 4; the original Phase 3 (HumanControl + AgentFeed) is now
> Phase 5; the original Phase 4 (Assistant slot) is now Phase 6. Two new
> phases — Shell IA + Charts (Phase 2) and Detail screens (Phase 3) —
> precede them, motivated by the operator's session of 2026-05-04 (see
> changelog).

## Why

The shipped cockpit is **operator-correct but design-system-thin**.
[`crates/ui/src/theme.rs`](../../crates/ui/src/theme.rs) ships **12
semantic tokens** (9 original + `bg_overlay` / `info` / `border_strong`
added at the tape-modal feature), six widgets in
[`crates/ui/src/widgets/`](../../crates/ui/src/widgets/) (`tape`,
`positions`, `pnl`, `kill`, `strategies`, `journal_transaction_modal`),
two binaries (`cockpit` for fixtures, `cockpit_live` for the unified
live bin landed at `live-cockpit-unified`), and a 599-line
[`spec/ui-design-principles.md`](../ui-design-principles.md) that
codifies what is — Bloomberg discipline, Linear taste, Stripe state
clarity. None of that is wrong. But there is no **elevation system**,
no **whisper-shadow** language, no concrete **light-mode hex palette**
beyond a proposed table in the principles doc, and no **status bar**
to anchor connection / latency / account / server-time at the bottom
of the shell.

### The Lumen bundle is purpose-built for this project

The
[design conversation transcript](../design/chats/chat1.md) shows the
operator asking the design assistant for "rust driven applications
with focus on desktop", "calm modern fintech", "clear and clean
structure visible", and "I like cards panel with elevation but I
dont know if it is still modern" (chat1.md, lines 30–34). The
assistant's response landed on **Lumen**: a calm, structured
desktop-first system (chat1.md, lines 42–49) with:

- A **3-tier elevation** model (canvas → panel → raised) using *tint
  shift + 1 px hairline + whisper shadow* — exactly the "modern read"
  the operator asked the assistant to clarify (chat1.md, line 47).
- A **warm-paper light** + **cool-deep dark** twin palette, joined by
  a single muted teal accent
  ([`spec/design/project/colors_and_type.css:13–108`](../design/project/colors_and_type.css)).
- **Sage / clay** for up / down (calmer than the standard neon
  green / red), keyed to a 12-hour session at the desk
  ([`spec/design/project/README.md:71–73`](../design/project/README.md)).
- A **status bar** that is **always visible** and shows connection,
  latency, account, server time
  ([`spec/design/project/README.md:127–131`](../design/project/README.md);
  [`spec/design/project/ui_kits/desktop/Shell.jsx:67–81`](../archive/design-prototypes-2026-Q2.tar.gz)).
- A **Backtest report viewer** (`Backtest.jsx`) that is materially
  richer than our existing `viewer` binary —
  KPI strip + equity curve + drawdown band + deploy-live action
  ([`spec/design/project/ui_kits/desktop/Backtest.jsx:54–110`](../archive/design-prototypes-2026-Q2.tar.gz)).
- A **HumanControl** pattern (kill + execution-mode toggle:
  Observe / Supervised / Auto) that extends our shipped kill-only
  surface
  ([`spec/design/project/ui_kits/desktop/HumanControl.jsx:6–55`](../archive/design-prototypes-2026-Q2.tar.gz)).
- An **AgentFeed** pattern that maps onto our existing tape with
  per-event sparkline visualization
  ([`spec/design/project/ui_kits/desktop/AgentFeed.jsx:68–93`](../archive/design-prototypes-2026-Q2.tar.gz)).

The design system was generated **for this project**, by a design
assistant that knew the operator wanted Rust desktop. That changes
the cost/benefit math: **adopting the design system is closing the
loop the operator opened in the design conversation**, not adopting
a third-party kit.

### What adoption gets us

1. **Concrete light-mode hexes.** Today
   [`spec/ui-design-principles.md`](../ui-design-principles.md) lines
   97–110 propose a light palette but none are wired. The Lumen
   `colors_and_type.css` ships both modes, contrast-checked, with a
   single `[data-theme="dark"]` switch. Phase 1 wires both.
2. **Elevation language.** Today every panel uses the same flat
   `BG_ELEV` and a single `BORDER`. Lumen's three-tier system
   (`canvas` / `panel` / `panel_raised` / `panel_sunken` / `overlay`)
   gives panels (Tier 1) a discernibly different surface from
   inputs (sunken) and modals (Tier 3) — no extra colour density,
   just structured tint.
3. **Status-bar anchor.** Connection / latency / account / server-time
   live as inline badges scattered across panels today. Lumen's
   always-visible status bar consolidates them at the bottom of the
   shell — the same place the operator's eye returns between
   scans of the tape.
4. **Forward compatibility for the v2 LLM strategy.** The Lumen
   `Assistant.jsx` slot reserves a **right-rail collapsible
   panel** for the assistant, and the `HumanControl.jsx`
   execution-mode toggle (Observe / Supervised / Auto) maps
   directly onto the v2 LLM gate — Supervised = trade-by-trade
   approval, Auto = within-envelope autonomy, Observe = paper
   only. **Phase 4 is the slot-reservation; not implemented now.**
5. **Closes the cosmetic-vs-system gap.** The shipped cockpit is
   "right" in the principles sense and "thin" in the system sense.
   Adopting Lumen's tokens + tier + motion fills the system gap
   without re-litigating the principles.

### What adoption does NOT get us — and is explicitly excluded

The following are **operator-locked** as out-of-scope for the
adoption initiative; see the next section for details.

- **The Lumen brand itself** — name, eye/lens logo, wordmark, "Lumen"
  in the title bar. The cockpit binary stays `cockpit`/`cockpit_live`.
- **Voice rules rewrite.** Our existing `ui::strings` discipline is
  voice-aligned with Lumen's voice table — no rewrite is dispatched.
- **Lucide icon adoption.** The principles-doc "no icons until
  needed" stays operator-locked.
- **Order-entry surfaces.** OrderTicket / OrderBook / Watchlist /
  Chart / FleetSummary / StrategyDetail / ApprovalQueue — paper-
  trading observation-only product, no order entry, no chart, no
  watchlist (universe is config-driven). Out of scope entirely.

## Master scope — Phases 1 through 6

The design-system adoption breaks into **six phases**, sequenced.
Each phase is its own `spec/lumen-design-adoption/phase-N-<name>/feature.md` brief
that ships through the standard analyst → architect → developer →
tester → presenter pipeline. The master file does **not** list
R-items; per-phase briefs do. The master file's job is to lock the
phase boundaries, the cross-phase invariants, and the
operator-pre-decided constraints.

### Phase 1 — Foundation (tokens + tiers + status bar)

**Status: Shipped 2026-05-04.** Tester third-pass `VERDICT → PASS` —
all 8 gates green; `T_FINAL_LUMEN_PHASE_1` ratified; brief
frontmatter bumped to `shipped`. Report at
[`spec/lumen-design-adoption/phase-1-foundation/reports/test-2026-05-04c-lumen-phase-1-foundation.md`](../archive/tester-reports-2026-05-to-06.tar.gz).
Scope:

- Replace the 12-token palette in
  [`crates/ui/src/theme.rs`](../../crates/ui/src/theme.rs) with the
  full Lumen palette: warm + cool neutral scales, accent ramp 50→900,
  sage / clay / warn / info semantics, both light and dark modes
  ([`spec/design/project/colors_and_type.css:13–160`](../design/project/colors_and_type.css)).
- Add **Tier 0/1/2/3 elevation tokens** + sunken — `canvas`, `panel`,
  `panel_raised`, `panel_sunken`, `overlay` — and the corresponding
  semantic surface tokens.
- Add **whisper shadows** (`shadow_1` / `shadow_2` / `shadow_3` +
  `shadow_inset`) and the **focus ring** (3 px low-alpha accent).
- Extend the spacing scale to the full Lumen ladder
  (`0/2/4/6/8/12/16/20/24/32/40/48/64`) and the radii ladder
  (`2/4/6/8/12 + pill 999`).
- Add the **typography ladder**
  (`display/h1/h2/h3/body/small/micro = 32/24/18/15/13/12/11 px`)
  layered on top of the existing `caption/body/title/display` —
  see Phase 1 Q3 / R7 for the rename strategy.
- Add **motion tokens** (`dur_1=80 / dur_2=140 / dur_3=220 /
  dur_4=320 ms` + cubic-bezier easings) and consume them in modal
  transitions.
- Refactor the existing 6 widgets to consume **Tier 1 styling** —
  hairline border + whisper shadow + tinted background. Inputs and
  table stripes adopt **sunken styling**.
- Add the **active-row pattern** to tabular widgets — 2 px left rule
  in `accent`, no fill change
  ([`spec/design/project/ui_kits/desktop/desktop.css:357–360`](../archive/design-prototypes-2026-Q2.tar.gz)).
- Add a new **status-bar widget** (`crates/ui/src/widgets/status_bar.rs`)
  rendering connection / latency / account / server-time — always
  visible at the bottom of the cockpit shell.
- Refresh the existing 36 panel snapshots once
  (`cargo insta review`); the visual diff is the visible artefact.
- **Supersede `spec/ui-design-principles.md`** with a Lumen-anchored
  rewrite (~300–400 lines, single-file replace per Phase 1 Q7
  recommended resolution).

**Anchor risk:** zero — UI feature, no backtest path touched.

**Operator success-reports anchors:** zero touched — R7 latency
badges may need band-name reconcile (Phase 1 Q8); colour mapping
stays compatible.

**Open questions:** ~9 (see [`lumen-phase-1-foundation.md`](../lumen-design-adoption/phase-1-foundation/feature.md)).

### Phase 2 — Shell IA + Charts (sidebar nav + Home/Debug/Charts screens)

**Status: Queue — promotes next** (Phase 1 shipped). Adds the
left-sidebar navigation shell, splits the cockpit into two starter
screens (Home + Debug), and lands the per-symbol price chart with
buy/sell markers from the audit ledger. Scope:

- **Left sidebar nav widget** — `crates/ui/src/widgets/sidebar_nav.rs`
  (new). Renders a vertical column of nav entries. Phase 2 entries:
  Home, Debug, Charts. Selected entry uses the T1507 `active_row`
  pattern (2 px ACCENT left rule). Iconless-by-default per the
  principles doc — text-only labels until icon adoption is
  re-litigated. Width fixed (`~180 px`), Tier 1 background.
- **Screen routing** — `crates/ui/src/state.rs` gains a
  `pub enum Screen { Home, Debug, Charts }` (Phase 2 set; Phase 3
  extends it to `Strategies`, `Risk`, `Audit`). The `Cockpit` model
  gains `pub current_screen: Screen` plus a `Message::SwitchScreen(Screen)`
  handler. The cockpit shell's `view()` dispatches on
  `cockpit.current_screen` and renders the appropriate screen body.
- **Home screen** — assembles the existing Phase 1 widgets: PnL,
  Positions, Strategies (summary), Tape (recent fills). The
  four-panel grid that ships today becomes the Home screen body
  verbatim — no widget changes beyond their existing layout under
  the new shell.
- **Debug screen** — collects the operator/operations chrome that
  was scattered across the single-page cockpit: kill switch,
  latency badge, market-health detail (per-venue rows from
  `MarketHealth`), server-time detail, version, plus a logs/metrics
  output stub (text-only; structured metrics surface lands when the
  R13.4 lazy-metric infra ships).
- **Charts screen** — new widget `crates/ui/src/widgets/chart.rs`:
  - **Symbol selector** — chip row at the top of the screen, drawn
    from the configured universe (live: from `config/agent.toml`
    `[universe]`; fixtures: from the fixtures-mode hard-coded
    3-symbol set). Selected symbol highlighted via active-row pattern.
  - **Price chart** — iced canvas-based (no external chart crate).
    Renders OHLC or line series for the selected symbol over a
    fixed visible window (default 1 hour of 1-minute bars; pan/zoom
    out-of-scope for Phase 2 — architect resolves at design time).
    Background = `PANEL`; gridlines = `BORDER_1`; line = `ACCENT`;
    up-candle fill = `UP_500`; down-candle fill = `DOWN_500`.
  - **Buy/sell markers** — overlaid on the price chart. Buys =
    upward triangle in `UP_500`; sells = downward triangle in
    `DOWN_500`. Markers come from the audit ledger filtered by
    `(venue, symbol, time-range)` matching the chart's visible
    window.
- **Live mode data path** — fresh ticks from the `MarketHealth`
  feed roll into a per-(venue, symbol) bucket in the `Cockpit`
  model. Architect resolves at Phase 2 design whether this is a
  new field on `Cockpit` or a separate `ChartState` struct
  composed in.
- **Fixtures mode data path** — synthetic candles seeded
  deterministically in `crates/ui/src/fixtures.rs`. Random walk
  with fixed seed, ~1 hour of 1-min bars per symbol; the demo bin
  shows the chart at a stable shape every run (snapshot-friendly).
- **Audit query extension** — `crates/audit/src/query.rs` gains a
  new public method (architect names it at Phase 2 kickoff;
  working name `recent_fills_filtered`) that takes
  `(venue: Venue, symbol: Symbol, time_range: Range<Timestamp>)`
  and returns `Vec<JournalTransaction>`. Additive to the existing
  `recent_fills(limit)`. The chart consumes this method to render
  markers within the visible window.
- **Anchor-risk note on the audit query extension.** New
  read-only query method over the existing audit ledger. Does not
  alter committed report bodies; does not introduce a new report
  rendering path. **11/11 anchor regression PASS expected at
  Phase 2 tester gate.**
- **Both bins in scope.** `cockpit` (fixtures) and `cockpit_live`
  both adopt the sidebar shell + the three screens. The chart
  renders in both modes (live data live, synthetic data fixtures).

**Anchor risk:** zero — additive UI shell + new widget +
read-only audit query. The new query method is read-only over
committed-report-equivalent data; doesn't alter any locked report
body.

**Open questions:** TBD; ~8–10 expected at analyst kickoff,
including chart-window pan/zoom scope, sparkline-vs-OHLC default,
multi-venue overlay (one chart per symbol or one chart per
venue+symbol), and audit-query method placement (`query.rs` vs
new `chart_query.rs`).

### Phase 3 — Detail screens (Strategies / Risk / Audit)

**Status: Queue — promotes after Phase 2 lands.** Adds three new
sidebar entries that surface existing backend data. No new audit
writers; pure read surfaces over what `crates/strategy`,
`crates/agent`, and `crates/audit` already produce. Scope:

- **Strategies-detail screen** — per-strategy view with:
  - Read-only params from `config/agent.toml` `[[strategy]]` blocks.
  - Recent **signal events** (architect resolves at Phase 3
    kickoff: new `Cockpit` field fed by the existing `crates/strategy`
    decision channel, OR a new audit writer if the channel is
    not currently observable from the UI thread).
  - A small **equity-since-deploy sparkline** if the data path is
    cheap; deferred to Phase 4 if it costs a new audit query.
  - Triggered by clicking a strategy row in the Home → Strategies
    summary panel; sidebar entry "Strategies" jumps to the most
    recently selected strategy.
- **Risk / Limits screen** — current per-venue exposure vs caps,
  daily loss limit consumed, kill threshold proximity gauge.
  Reads from the existing risk state in `crates/agent` (`Cockpit`
  gains read-only mirror fields fed by the agent runtime). The
  **kill threshold proximity gauge** is a horizontal bar showing
  `used / max` with `WARN_500` fill at >70%, `DOWN_500` fill at
  >90%.
- **Audit / Journal screen** — full ledger browser:
  - Table with filter row (venue, symbol, kind, time range).
  - Pagination (250 rows per page; the table reuses the active-row
    pattern T1507).
  - Per-row click opens the existing `journal_transaction_modal`
    (T1208 reused, no widget changes).
  - Reuses Phase 2's audit query extension for filtered fetches;
    extends it with a `kind: Option<&str>` parameter if Phase 2's
    method doesn't already accept one (architect resolves at
    Phase 3 kickoff).

**Anchor risk:** zero — UI surfaces over existing backend data.
No new audit writers; the audit query extensions are read-only
and additive.

**Open questions:** TBD. Notable cross-phase question: does the
Strategies-detail screen's signal-history feed require **a new
strategy_events writer** (e.g. `signal_emitted` with rationale
JSON), or is the existing `crates/strategy::Decision` channel
sufficient when piped through the cockpit? Architect resolves at
Phase 3 design.

### Phase 4 — Backtest panel (was original Phase 2)

**Status: Queue — promotes after Phase 3 lands.** Aligns the
**offline backtest review surface** (`viewer` binary) with the Lumen
`Backtest.jsx` pattern. Scope unchanged from the original Phase 2
sketch:

- The Lumen `Backtest.jsx` shows a **KPI strip** (Total return,
  CAGR, Sharpe, Max DD, Win rate, Trades), an **equity curve** with
  filled area, and a **drawdown band** beneath
  ([`spec/design/project/ui_kits/desktop/Backtest.jsx:79–106`](../archive/design-prototypes-2026-Q2.tar.gz)).
- Today the `viewer` bin renders a markdown report from
  `spec/*/reports/backtest-*.md`. Phase 4 adds a **structured KPI
  band + sparkline** above the markdown body, consuming the same
  numbers the report already locks.
- The Lumen "Backtest result" panel includes a **Deploy live**
  CTA. **We exclude this from Phase 4** — the v0/v0.5/v1/v1.5a
  shipped surface is paper-only and the deployment flow is config-
  driven (not a button); the brief explicitly drops the CTA.
- **Single-binary scope.** Phase 4 only touches `viewer`; no
  changes to `cockpit_live` or backend crates.
- May reuse the chart-rendering primitives that ship in Phase 2
  (canvas-based plot widget). Architect resolves at Phase 4
  kickoff whether the equity curve renders via the Phase 2 chart
  module (`widgets::chart::line_series`) or via a viewer-local
  copy.

**Anchor risk:** zero — viewer bin renders existing committed
reports; no new backtest scenarios, no re-anchor budget. The
viewer's structured KPI strip reads from the report's locked
metrics; the body-SHA hash is unchanged.

**Open questions:** TBD; spawned at Phase 4 analyst kickoff.

### Phase 5 — HumanControl + AgentFeed rename (was original Phase 3)

**Status: Shipped 2026-05-07** (tester second-pass `VERDICT → PASS`,
operator-approved 2026-05-08). Reports at
[`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md`](../archive/tester-reports-2026-05-to-06.tar.gz).
Brief frontmatter bumped to `shipped`. **First phase to ship net-new
operator-write surfaces since v0**: HumanControl panel (execution-
mode toggle + limits + kill bottom action) on a new "Control"
sidebar entry, single-click pause-strategy, typed-confirm `OVERRIDE`
flow for risk-veto override. Two new audit writers
(`strategy_paused`, `risk_veto_overridden`) — additive
`StrategyEventKind` extensions, no migration. Module rename
`tape` → `agent_feed` complete; `Cockpit::tape` field name
preserved (Q14). **TD-1 four-phase deferral CLOSED** via Path (b)
custom-widget escape hatch (`crates/ui/src/widgets/focus_ring.rs`).
**TD-2 NEW** row added for the deferred risk-engine veto-emit
upstream wiring. Adopts richer operator override controls and
aligns vocabulary. Scope unchanged
from the original Phase 3 sketch, except that the kill switch /
latency / market-health surfaces now live on the **Debug** screen
(per Phase 2's IA split) — Phase 5 promotes the kill surface to a
**HumanControl** panel that can also live on the Home screen as a
collapsible header or move to a dedicated **Control** sidebar entry
(architect resolves at Phase 5 kickoff). Detail:

- **Adopt the Lumen `HumanControl.jsx` pattern** — the existing kill
  switch becomes the bottom action of a richer "You're in control"
  panel that also shows execution-mode (Observe / Supervised /
  Auto), daily loss limit, max position, and used-today P&L
  ([`spec/design/project/ui_kits/desktop/HumanControl.jsx:6–55`](../archive/design-prototypes-2026-Q2.tar.gz)).
- **Pause-strategy** and **override-risk-veto** controls land as
  per-strategy actions next to each row in the strategies panel
  (one button per row, typed-confirm flow per the principles
  doc — `OVERRIDE` for risk-veto, no phrase needed for pause).
- **Rename `tape` widget to `AgentFeed`.** The existing
  [`crates/ui/src/widgets/tape.rs`](../../crates/ui/src/widgets/tape.rs)
  rendering stays; the rename is module-level only. The Lumen
  vocabulary in the chat transcript and the
  [`AgentFeed.jsx`](../archive/design-prototypes-2026-Q2.tar.gz)
  component ground the rename. **Module path change only — no
  visual change beyond Phase 1's Tier-1 refresh.**
- Consistency-test fixture and snapshot baseline updated for the
  rename (one-time ripple).

**Anchor risk:** zero — UI rename + new HumanControl widget; no
backtest path touched. Pause-strategy / override-risk-veto **may**
add audit writers (architect Q at Phase 5 kickoff); writers are
additive and don't alter committed report bodies.

**Open questions:** TBD; spawned at Phase 5 analyst kickoff.
Notable cross-phase question: does the per-strategy "pause" /
"override" surface require new audit writers (`strategy_paused`,
`risk_veto_overridden`) or reuse `strategy_events` with new
`event_kind` strings? Architect resolves at Phase 5 design.

### Phase 6 — Assistant slot (was original Phase 4)

**Status: Reserved — not implemented in this initiative.** The
Lumen `Assistant.jsx` and Shell `right-side AI assistant panel` is
**opt-in, collapsible, and remembers state**
([`spec/design/project/README.md:131`](../design/project/README.md)).

Phase 6's scope **at the time it lands** (with the v2 LLM strategy):

- Right-rail collapsible panel slot in the shell, hidden by default,
  revealed when the v2 LLM strategy is enabled.
- A composer + message-list widget pattern aligned to
  [`spec/design/project/ui_kits/desktop/Assistant.jsx`](../archive/design-prototypes-2026-Q2.tar.gz).
- Wires into the v2 LLM trait the architect defines at v2 kickoff.
- Coexists with the Phase 2 sidebar nav (the assistant rail lives
  on the right; the nav lives on the left). Phase 2 must not
  consume the right column-track.

**Phase 6's scope at the lumen-design-adoption initiative time
(NOW):** zero shipped UI. Phase 2's shell grid will reserve the
right-rail track in advance — see the Phase 2 brief for the
column-track contract.
[architecture.md](../architecture.md) Frontend section will get a
two-line forward-compat note documenting the slot at Phase 2
landing.

**Anchor risk:** out of scope — not implemented here.

## Operator-locked constraints

These resolutions came from the operator before the analyst was
spawned. Each is **not negotiable** in any per-phase brief; the
brief states the constraint and moves on.

### Constraint 1 — NO brand adoption

> "NO brand adoption. Do NOT rename anything to 'Lumen'. The cockpit
> binary stays `cockpit`/`cockpit_live`. The eye/lens logo, brand
> wordmark, and 'Lumen' name are explicitly out of scope. The DESIGN
> SYSTEM (tokens, components, layout patterns) is in scope; the BRAND
> IDENTITY is not."

**What this excludes:**

- The string `"Lumen"` in the title bar (the JSX
  [`Shell.jsx:16`](../archive/design-prototypes-2026-Q2.tar.gz)
  renders `<span>Lumen</span>` — the Phase 1 status bar /
  title bar adopts the *layout* but **not** the brand string).
- The eye/lens mark
  ([`spec/design/project/README.md:204–207`](../design/project/README.md)
  asset list — `lumen-mark.svg`, `lumen-wordmark.svg`,
  `lumen-monogram.svg`, `lumen-ai-lens.svg`).
- Any "Lumen Trading" lockup or marketing typography.

**What this allows:**

- Adopting the **token names** (`--accent`, `--canvas`, `--panel`,
  `--shadow-1`) when porting them to Rust constants. The token-
  name semantics are design-system content; the *brand-name*
  semantics are not.
- The single muted-teal accent hex `#3F968D`. The colour is not
  a brand asset — every project that paints with this hex doesn't
  thereby become Lumen.

The brief's tone in any per-phase doc is **"the design system",
not "Lumen"**. The brand name appears in the brief only when
citing the source files (which are named `lumen-*`).

### Constraint 2 — NO voice rules rewrite

> "NO voice rules re-write. Our existing `ui::strings` discipline +
> the principles-doc voice section already cover plain-language /
> no-jargon / scannable-numbers. Lumen's voice table is informative
> but we won't dispatch a strings rewrite. Document the alignment
> in the principles doc but no `ui::strings` changes."

**What this excludes:**

- Mass rename of any string in `ui::strings`.
- Refactoring strings to match Lumen's voice examples table
  ([`spec/design/project/README.md:53–60`](../design/project/README.md)).
- Adding the typographic-minus (`U+2212`) sweep across all delta
  strings — the existing `widgets::num` ASCII minus stays.

**What this allows:**

- A note in the rewritten principles doc (Phase 1) documenting
  that our voice rules already align with Lumen's voice table —
  Bloomberg-discipline + Linear-taste + Stripe-state-clarity is
  voice-equivalent to "calm and competent, the way a senior
  trader talks". The principles-doc *cites* the Lumen voice table
  but does not *adopt* it as a rewrite.

### Constraint 3 — Adoption is sequential, not parallel

The six phases ship **one at a time**. The architect spawns each
phase only after the previous phase's presenter records operator
approval. Justification:

- **Phase 1's snapshot ripple** (36 existing panel snapshots
  refresh once) is a known one-time cost. Stacking later phases
  on top would double the visual-diff review burden for no
  reason.
- **Each phase inherits the previous phase's tokens / shell.**
  A later phase's brief written before its predecessor ships
  would have to fork the token / IA contracts in the brief, then
  re-fork on landing — wasteful.
- **Phase 5's `tape` → `AgentFeed` rename** ripples through
  consistency tests and snapshot baselines. Stacking it onto
  earlier phases' already-large refactor passes risks losing the
  rename in the noise.

The operator approved this sequencing at the analyst-kickoff
constraint capture (2026-05-03), reaffirmed at the 2026-05-04
roadmap-revision session.

### Constraint 4 — Phase 6 is forward-compat only

Phase 6's full implementation **requires the v2 LLM strategy** —
which is its own queued backlog item with its own analyst /
architect / developer pipeline. The lumen-design-adoption
initiative ships **only the slot reservation**: a two-line
forward-compat note in
[architecture.md → Frontend](../architecture.md) and a layout-grid
slot in the Phase 2 shell that hides cleanly when the v2 LLM
strategy is not enabled.

**Phase 6's adoption brief lands when v2 LLM is approved.** Until
then, the lumen-design-adoption initiative considers Phase 6
"reserved", not "deferred".

## Adoption strategy — phase-by-phase, sequential

```
            Phase 1 — Foundation                     [SHIPPED 2026-05-04]
            (tokens + tiers + status bar + principles supersede)
                            │
                            ▼
            Phase 2 — Shell IA + Charts              [NEXT]
            (sidebar nav + Home/Debug/Charts screens +
             per-symbol price chart with buy/sell markers +
             audit query extension)
                            │
                            │ HANDOFF analyst → architect → developer →
                            │         tester → presenter → operator
                            │
                            ▼
                       OPERATOR APPROVAL
                            │
                            ▼
            Phase 3 — Detail screens
            (Strategies / Risk / Audit sidebar entries)
                            │
                            ▼
                       OPERATOR APPROVAL
                            │
                            ▼
            Phase 4 — Backtest panel (viewer bin)
                            │
                            ▼
                       OPERATOR APPROVAL
                            │
                            ▼
            Phase 5 — HumanControl + AgentFeed rename
                            │
                            ▼
                       OPERATOR APPROVAL
                            │
                            ▼
            Phase 6 — RESERVED until v2 LLM strategy lands
```

Each phase is **its own feature brief**, **its own task list**, **its
own tester report**, **its own presentation**. The architect spawns
the per-phase pipeline reading **this master file** and the per-phase
brief; the master file is orientation, the brief is the contract.

### Why not one giant Phase

Three reasons:

1. **Snapshot ripple containment.** Each phase produces its own
   snapshot diff. Stacking the chart canvas, the detail-screen
   tables, and the AgentFeed rename onto a single review doubles
   the diff surface and slows the operator's approval gate.
2. **Operator approval cadence.** The presenter's job is to give
   the operator one shippable thing per approval. Five remaining
   phases mean five discrete approvals, each on a clearly-bounded
   surface.
3. **Failure isolation.** If Phase 2's chart turns out to crowd
   the cockpit (operator rejects), Phase 1's tokens and the later
   phases' surfaces don't roll back with it.

### Why this order (1 → 2 → 3 → 4 → 5 → 6)

- **Phase 1 is the foundation** — tokens, tiers, motion, status
  bar. Every later phase consumes Phase 1's primitives. Order
  was forced and the phase has shipped.
- **Phase 2 (Shell IA + Charts) before everything else** — the
  sidebar nav + Home/Debug/Charts triple is the IA contract that
  every later screen plugs into. Phase 3 (detail screens) cannot
  land without sidebar entries to attach them to; Phase 5
  (HumanControl) needs the Debug surface to migrate from. The
  chart bundles into Phase 2 because it motivated the IA rework
  and because the canvas plot widget is reused by Phase 4
  (Backtest equity curve).
- **Phase 3 (Detail screens) before Phase 4 (Backtest)** — Phase 3
  is read-only over existing backend data; Phase 4 touches the
  viewer bin and the report-rendering boundary. Lower-risk first.
- **Phase 4 (Backtest) before Phase 5 (HumanControl)** — viewer
  changes don't touch live trading semantics; HumanControl adds
  new operator-write paths (pause-strategy, override-risk-veto).
  Lower-risk first.
- **Phase 5 (HumanControl + AgentFeed) before Phase 6 (Assistant)**
  — both phases are forward-looking; Phase 5 ships now, Phase 6
  is gated on the v2 LLM strategy and ships with it.
- **Phase 6 is forward-compat** — depends on v2 LLM, ships with
  that.

## Anchor risk per phase

| Phase | Touches strategy/audit/exec/backtest?                                              | New backtest scenarios? | Anchor risk |
|-------|------------------------------------------------------------------------------------|-------------------------|-------------|
| 1     | No — UI tokens + widget rendering                                                  | No                      | **Zero** (verified PASS at ship) |
| 2     | Read-only — adds an additive `recent_fills_filtered` query in `audit::query`        | No                      | **Zero — read-only over existing audit ledger; doesn't alter committed report bodies**     |
| 3     | Read-only — UI surfaces over existing `crates/strategy`, `crates/agent`, `crates/audit` data | No                      | **Zero — UI-only**     |
| 4     | No — viewer binary only                                                            | No                      | **Zero**    |
| 5     | Phase 5 may add new audit writers (pause / override) — architect resolves           | No                      | **Zero — UI rename + new HumanControl widget by default; if writers land, they are additive and don't alter committed report bodies** |
| 6     | Out of scope                                                                       | Out of scope            | **N/A**     |

The 11 / 11 anchor regression goal stays **byte-identical** across
the entire initiative. Any phase that turns out to need a re-lock
follows the v1.5a T717 / T811 precedent (re-anchor only the
affected scenarios; document each re-lock with a one-line
rationale).

**Phase 1 specifically:** the existing 36 panel snapshots
(`crates/ui/tests/snapshots/`) refreshed once at Phase 1 ship.
These are **not backtest anchors** — they are visual-regression
baselines for the cockpit, accepted via `cargo insta review`. The
11 backtest body-SHA-256 anchors in
[`spec/anchors.toml`](../anchors.toml) remained untouched.

**Phase 2 specifically:** snapshot baselines refresh again — every
existing widget moves from a single-page layout to a screen-routed
shell, so each baseline's surrounding chrome differs (sidebar
present, screen body padding shifted). The chart widget brings
~3–5 net-new baselines. Total expected refresh: ~36 (the same set
as Phase 1, all with sidebar context) + ~5 net-new = ~41 baselines.

## Cross-feature invariants

Every prior shipped feature must continue to pass after each phase
ships. The tester gates each phase against this list. The list is
**closed** — adding a new invariant requires architect sign-off in
the per-phase brief.

The invariant table spans all six phases. **Phase 1 column** records
its **shipped reality** (verified at the Phase 1 tester gate); the
remaining columns are the **forward contract** every later phase
inherits. Each column either says `unchanged` (the invariant is not
touched by that phase) or names the specific delta and how the
invariant is preserved.

| Feature | Invariant | Phase 1 (shipped) | Phase 2 | Phase 3 | Phase 4 | Phase 5 | Phase 6 |
|---------|-----------|--------------------|---------|---------|---------|---------|---------|
| `operator-success-reports` | R7 latency badges color contract (green/yellow/red mapping to bands) | reconciled band names with Lumen warn (Phase 1 Q8); colour mapping unchanged | latency badge moves to Debug screen; colour mapping unchanged | unchanged | unchanged | unchanged | unchanged |
| `live-cockpit-unified` | `cockpit_live` bin launches against the agent runtime; halted-banner trips on file watch / kill / heartbeat | reconciled via Tier 1 panel refresh; banner rendering preserved | sidebar-shell wraps cockpit; banner rendered above screen body; trigger preserved | unchanged | unchanged | rename ripples; banner preserved | unchanged |
| `real-mtm-unrealized-pnl` | P&L card shows realised + unrealised columns; `color_for_delta` returns colours from the new palette | tokens swapped; helper signature unchanged | P&L card lives on Home screen; signature unchanged | unchanged | unchanged | unchanged | unchanged |
| `per-symbol-position-accounts` | Positions widget renders one row per symbol with strategy-id chip | row spacing inherits the new compact density; chip uses `accent_soft` background | Positions widget lives on Home screen; row contract unchanged | unchanged | unchanged | unchanged | unchanged |
| `tape-row-audit-modal` | Modal is reachable from any tape row; modal frame uses `border_strong` | modal adopts Tier 3 styling (`shadow_3` + `overlay`); border token preserved | Tape lives on Home screen; modal trigger preserved | Audit screen also reuses the modal; trigger preserved | unchanged | tape rename ripples to AgentFeed; modal trigger preserved | unchanged |
| `journal-tx-metadata` | Modal header renders `description` + `strategy_id` from the metadata reader | unchanged | unchanged | Audit screen surfaces same metadata in its filter row | unchanged | unchanged | unchanged |
| `v1.5b-multi-venue` | `cockpit_live` shows venue-tagged ticks; T805 reconnect events tagged by venue | unchanged | venue dimension surfaces in Debug screen (per-venue market-health rows) and on the chart's symbol selector | venue filter on Audit screen; venue dimension on Risk screen exposure cap | unchanged | unchanged | unchanged |

The tester's per-phase report includes a **cross-feature invariant
table** with PASS / FAIL per row.

## Cross-phase technical-debt items

These are **deviations** ratified mid-phase that ship as bounded
best-effort approximations, with a named upgrade trigger that
promotes a follow-up brief in a later phase. The architect adds
rows here on ratification; per-phase briefs inherit the list and
the tester verifies the row's "shipped state" each phase until the
follow-up lands.

### TD-1 — True keyboard-focus ring (Phase 1 Q11 deviation, ratified 2026-05-04)

**Origin:** Phase 1, T1504/T1506 implementation. Architect Q11 in
[architecture.md](../architecture.md#lumen-design-adoption--phase-1-foundation-resolutions-q1q9--master-q10--mid-phase-q11--confirmed-2026-05-04)
ratified as **Option A** (best-effort approximation + Phase-N
follow-up).

**API gap.** iced 0.14.2 `button::Status` has no `Focused` variant
(only `Active / Hovered / Pressed / Disabled`); iced 0.14.2
`text_input::Style` has no `shadow` field. The 3 px outer-halo
focus ring per `theme::focus::ring(mode)` cannot be wired to
keyboard-focus on buttons or inputs.

**Phase 1 shipped state.** Hover-state ring on the three buttons
named in T1504 (kill trigger, kill confirm, modal close); ACCENT
border-shift (`BORDER_2 → ACCENT`, 1 px) on the kill confirm input
when focused. Documented in `crates/ui/src/widgets/kill.rs`
module-level doc + the T1504/T1506 honest-tick rows in
[tasks/lumen-phase-1-foundation.md](phase-1-foundation/tasks.md).

**Upgrade triggers** (any one promotes the follow-up brief):

1. **iced version bump** that exposes `button::Status::Focused` AND
   a `shadow` field on `text_input::Style`. Likely iced 0.15+;
   verify against the iced changelog at upgrade time. Follow-up
   scope is a one-file sweep across `crates/ui/src/widgets/kill.rs`
   (two button styles + one input) and
   `crates/ui/src/widgets/journal_transaction_modal.rs` (one button
   style) — replace `Hovered` arms with `Focused` arms; add the
   `shadow` field on the input. ~30 lines net change. No new task
   list needed; folds into the iced-upgrade task.
2. **Custom-widget escape hatch.** Project-local
   `iced::widget::Component` that owns focus state via a
   `Subscription` on `keyboard::Event::KeyPressed { key: Tab }`,
   emits a synthetic `FocusChanged(WidgetId)` `Message`, and
   re-renders with the halo. Promoted only if iced upstream stalls
   past Phase 3.

**Promotion timing.** Earliest target is **Phase 2**'s analyst
kickoff — that brief inherits this row and either folds the
upgrade into Phase 2 (if iced 0.15+ has shipped by then with the
fields) or restates the deferral. If neither trigger has fired by
Phase 3 (HumanControl), the architect re-evaluates at Phase 3
kickoff; HumanControl adds new operator-override controls whose
focus-ring needs may sharpen the cost/benefit on the custom-widget
path.

**2026-05-04 design-pass verification (Phase 2 architect).** Verified
[`crates/ui/Cargo.toml:50`](../../crates/ui/Cargo.toml) still pins
`iced = "=0.14.0"`. Neither upgrade trigger has fired
(iced 0.15+ has not landed; no custom-widget escape hatch has been
authored). Phase 2 design-pass **restates the deferral** —
hover-state ring + ACCENT input border-shift continue as the
shipped approximation. Next re-evaluation: **Phase 3 (Detail
screens) analyst kickoff**, post Phase 2 ship. Resolved as Phase 2
Q11 in
[`features/lumen-phase-2-shell-ia-charts.md`](../lumen-design-adoption/phase-2-shell-ia-charts/feature.md).

**2026-05-05 design-pass verification (Phase 3 architect).** Re-
verified [`crates/ui/Cargo.toml:52`](../../crates/ui/Cargo.toml)
still pins `iced = "=0.14.0"`. Neither upgrade trigger has fired.
Phase 3 design-pass **restates the deferral** — same bounded
approximation continues. Next re-evaluation: **Phase 4 (Backtest
panel) analyst kickoff**, post Phase 3 ship. The custom-widget
escape hatch remains the only path open if iced upstream stalls
through Phase 4; architect re-evaluates the cost / benefit at that
point. Phase 3 introduces no new operator-write paths (HumanControl
lands in Phase 5), so the bounded ergonomic gap remains within the
"typed-confirm gates the destructive flow" operator-impact bound
documented above.

**2026-05-06 design-pass verification (Phase 4 architect).** Re-
verified `crates/ui/Cargo.toml` still pins `iced = "=0.14.0"`.
Neither upgrade trigger has fired. Phase 4 design-pass **restates
the deferral** — same bounded approximation continues. The Phase 4
deliverable is the offline `viewer` binary + a cockpit
Strategies-detail sparkline replacement; the **viewer is a
zero-button surface** (CLI-arg-driven, no operator interaction
beyond closing the window), so the focus-ring deferral is
operationally invisible on Phase 4's primary surface. Next
re-evaluation: **Phase 5 (HumanControl) analyst kickoff**, post
Phase 4 ship. Phase 5 is the first phase to introduce **net-new
operator-write paths** (pause-strategy, override-risk-veto,
execution-mode toggle); the cost/benefit on the focus-ring
upgrade tightens materially at Phase 5, so the architect should
expect to either fold the iced 0.15+ upgrade in then OR commit to
the custom-widget escape hatch.

**2026-05-06 CLOSURE (Phase 5 architect).** Re-verified
[`crates/ui/Cargo.toml:69`](../../crates/ui/Cargo.toml) still pins
`iced = "=0.14.0"` (line shifted from `:52` post-Phase-4 viewer-bin
block; pin unchanged). Trigger #1 (iced version bump) has NOT
fired; trigger #2 (custom-widget escape hatch) **fires now at
Phase 5 design-pass**. Phase 5 architect ratified **path (b) —
custom-widget escape hatch** (analyst Q5 framing). Concrete
shape: new `crates/ui/src/widgets/focus_ring.rs` Subscription-driven
wrapper around all four Phase-5 destructive surfaces (kill button +
kill confirm input + override-risk-veto confirm + per-strategy
pause + execution-mode segments). Path (a) iced-fold-in unavailable
(no upstream version landed); path (c) restate-with-deadline
rejected (Phase 6 is v2-LLM-gated and operationally indefinite;
Phase 5 is the operator-write-surface sharpening point — yet
another deferral fails the bounded-cost framing of this row). The
four-phase TD-1 deferral **closes at Phase 5 ship**; the focus-ring
approximation becomes a true halo on every keyboard-focused
destructive control. Implementation lands as Phase 5 task T1912;
acceptance-tested via the visual-diff baseline pass. **Status: in
progress** — closes when Phase 5's tester gate ratifies
`T_FINAL_LUMEN_PHASE_5` PASS.

### TD-2 — Risk-engine veto-emit upstream wiring (Phase 5 Q13 deferral, ratified 2026-05-06)

**Origin:** Phase 5, Q13 / R7 design-pass. Phase 5 architect ratified
the override-risk-veto control surface but **deferred** wiring the
agent-side veto emit (the upstream signal that surfaces "the risk
engine vetoed this order" events to the cockpit). Today
[`crates/agent/src/runtime.rs:1023–1090`](../../crates/agent/src/runtime.rs)
ships `default_risk_telemetry_stub` — a no-op stub that emits an
empty `Vec<VetoEvent>`. Phase 5 ships the operator-side override
surface (the typed-confirm `OVERRIDE` flow + the audit writer
`risk_veto_overridden`) **over the stub feed** — operators see no
veto events to override until the upstream wiring lands.

**API gap.** The risk engine
([`crates/risk/src/portfolio.rs`](../../crates/risk/src/portfolio.rs))
returns vetoes inline as `Result::Err`; today these flow up the
order pipeline and abort the order. The cockpit doesn't observe
them. The wiring needs:
1. A new `VetoEvent` broadcast channel on `EventBus` (mirror of the
   Phase 3 `RiskTelemetry` publisher pattern).
2. A wrapping point in the order-emit path that catches the veto
   and publishes the event before propagating the error.
3. A `Cockpit::risk_veto_events` populator subscription.

**Phase 5 shipped state.** `Cockpit::risk_veto_events: Vec<VetoEvent>`
field exists; the override-surface flow is fully operational against
mock veto events injected via fixtures-mode; live mode has zero
veto events to display because the upstream stub is unchanged.

**Upgrade triggers** (any one promotes the follow-up brief):
1. **Operator request** to see live veto events surface in the
   cockpit (most likely trigger; today the operator can read the
   veto inline via `tracing` logs but not via UI).
2. **Risk-engine evolution** that adds new veto kinds Phase 5's UI
   would have caught — surfacing them gains operator value
   proportional to the new kinds' frequency.
3. **Compliance / audit** requirement that veto events are
   surfaced to the operator at decision time (not just logged).

**Promotion timing.** Earliest target is **Phase 6 (Assistant)
analyst kickoff** — but Phase 6 is gated on v2 LLM, so the natural
slot is **a sibling Phase-5.5 brief** if any of the three upgrade
triggers fires before v2 LLM ships. Until then, the operator-side
override surface is functional but unused in live mode.

**Operator-impact bound.** The risk engine still vetoes upstream
(it has done so since v0); the cockpit-side surface is an
**observation + override path**, not a safety primary. The veto's
job is to stop the order; if the cockpit can't display it, the
order still doesn't ship — only the operator's awareness of the
veto is degraded. Same bound as TD-1: deferral ships a known-
bounded ergonomic / observability gap, not an unbounded safety
gap.

**No anchor impact.** UI + write-side feature; the 11 backtest
body-SHA-256 anchors stay byte-identical. Cross-feature invariants
unchanged.

**Operator-impact bound.** The kill-switch destructive flow is
**typed-confirm gated** (operator types `KILL_SAFETY_PHRASE`); the
focus halo is a secondary signal, not the safety primary. Same
bound applies to the modal close button (the modal is read-only;
no destructive action gated on focus). The deviation ships a
known-bounded ergonomic gap, not an unbounded safety gap.

**No anchor impact.** UI-only; the 11 backtest body-SHA-256
anchors stay byte-identical. The cross-feature invariant table
above is unchanged.

## Open questions for the architect (master-roadmap level)

These cross-phase questions surface at this master-roadmap level and
were answered by the architect at Phase 1 kickoff (Q1–Q10) or by the
operator at the 2026-05-04 roadmap revision (Q11–Q14). Per-phase
briefs inherit the answers. **Q11–Q14 below are the post-Phase-1
questions; Q1–Q10 are preserved unchanged below for audit.** (Q11 was
also used in the Phase 1 brief / architecture.md to track the
focus-ring iced-API deviation — the master-roadmap and per-phase
question numbering are independent.)

### Q11 — Sidebar nav primacy: collapsible or fixed-width?

**The question:** Phase 2's left sidebar is **fixed-width** (~180 px,
always visible) or **collapsible** (icons-only when collapsed,
text-and-icons when expanded, operator-toggleable)?

**Recommended (analyst, post-Phase-1 revision):** **fixed-width**
for Phase 2. Justification: (a) the operator-locked icons-by-default
constraint is "no icons until needed" — collapsible navs need icon
glyphs to make sense in collapsed state, so collapsibility forces
the icon question early; (b) the cockpit's surface area on a
typical desk display is ample (≥ 1440 px wide) so 180 px sidebar
+ screen body fits comfortably; (c) defers the icon-adoption
re-litigation to Phase 5 or later when HumanControl introduces
per-row action buttons that benefit from icons. Architect ratifies
at Phase 2 kickoff.

### Q12 — Chart data source live + fixtures parity

**The question:** the chart renders **only in live mode**
(fixtures bin shows an empty-state placeholder), or **in both
modes** (fixtures uses synthetic candles, live uses tick rollup)?

**Operator decision 2026-05-04:** **both**. Live ticks roll into a
per-`(venue, symbol)` rolling buffer for the live bin; fixtures
mode seeds synthetic 1-min candles deterministically for stable
demo + snapshot baselines. Architect ratifies the buffer shape and
the fixture-seed convention at Phase 2 design.

### Q13 — Buy/sell marker query method placement

**The question:** the new **filtered audit query** (working name
`recent_fills_filtered(venue, symbol, time_range)`) lives in
**`crates/audit/src/query.rs`** alongside `recent_fills`, or in a
new **`crates/audit/src/chart_query.rs`** module?

**Operator decision 2026-05-04:** **extend the audit query** —
i.e. add the method to the existing `query.rs`. Justification:
(a) the existing `recent_fills` already does in-memory filtering
over the same description-prefixed rows; the new method is a
generalisation, not a different concern; (b) one fewer module
keeps the call-site grep-friendly; (c) the chart-screen wiring
imports the same `audit::query` namespace as the existing modal.
Architect ratifies the exact signature at Phase 2 design.

### Q14 — Phase 2 vs Phase 3 split rationale (operator-confirmable)

**The question:** Phase 2 ships sidebar IA + Charts; Phase 3 ships
the three new detail screens (Strategies / Risk / Audit). Why not
bundle them? Why not split further?

**Recommended (analyst, post-Phase-1 revision):** **keep the 2/3
split**. Justification: (a) Phase 2's chart is a non-trivial new
widget (canvas drawing, marker overlay, symbol selector); bundling
three additional table screens onto the same review doubles the
operator's review load; (b) the three Phase 3 screens are more
similar to each other (table-shaped read surfaces over backend
data) than to Phase 2's chart, so they share design review nicely;
(c) splitting Phase 3 further (one screen per phase) inflates the
roadmap to 8 phases without adding decoupling — the three screens
share table widgets and pagination; (d) the operator at the
2026-05-04 session expressed comfort with this split. Architect
ratifies at Phase 2 kickoff.

### Q1 — Token-rename one big merge or staged hex changes?

**The question:** Phase 1 swaps the entire palette in one merge
(R1–R12 all in one PR), or stages it: hex-only swap first, then
elevation tokens, then status bar?

**Recommended (analyst):** **one big merge**. Pros: the
36-snapshot refresh happens once; the visual diff is reviewable in
one `cargo insta review` session; the developer has one mental
model of "the new system" rather than two intermediate ones.
Cons: bigger PR. The `v1.5b` precedent (15 R-items, 12 V-items,
one merge) shows the team handles single-merge UI-adjacent
refactors fine. Architect ratifies at Phase 1 kickoff.

**Alternative:** stage in three sub-PRs. Pros: smaller diffs.
Cons: three snapshot refreshes (the operator looks at a stale
visual three times before the final shape lands).

### Q2 — Principles-doc supersede inside Phase 1 or as a parallel spec-only update?

**The question:** the existing 599-line
[`spec/ui-design-principles.md`](../ui-design-principles.md)
needs to be rewritten Lumen-anchored (~300–400 lines, Phase 1 Q7
recommended single-file replace). Does that happen **inside Phase
1** (one feature, two artefacts) or as a **separate spec-only
update** (no code, just the doc)?

**Recommended (analyst):** **inside Phase 1**. The new principles
doc cites the new tokens — the citations land in the same
review. Decoupling means the doc is wrong for the days between
Phase 1 ship and the principles update.

**Alternative:** parallel spec-only update spawned by the same
analyst kickoff, ships independently. Pros: smaller per-PR
diff. Cons: the doc references tokens that may or may not exist
during the review window.

### Q3 — Phase 2 / Phase 3 brief authorship — single analyst kickoff or per-phase analyst spawn?

**The question:** are Phase 2 and Phase 3 briefs written **now**
(by this analyst, in the same kickoff) or **later** (by a fresh
analyst spawn at Phase 1 ship + 1)?

**Recommended (analyst):** **per-phase analyst spawn**. Phase 1's
shipped reality may shift Phase 2's surface (e.g. if the new
typography ladder doesn't ship cleanly, Phase 2's KPI strip
inherits the workaround). Writing all three briefs at master-
roadmap time freezes assumptions that should remain mobile.

**Alternative:** write all three now. Pros: total brief volume
known up-front. Cons: re-litigation cost on Phase 1 ship.

### Q4 — Status-bar widget data-flow architecture (Phase 1 detail surfacing here)

**The question:** the status bar's four fields (connection,
latency, account, server time) come from **four different
backend surfaces today**. Connection from the EventBus mode
channel; latency from the existing latency badge logic
(`widgets::latency`); account from `config/agent.toml`'s
identity block; server time from the local clock or the audit
DB's `now_utc()` helper. Does Phase 1 wire all four
**eagerly** (status bar is fully populated on first paint) or
**lazily** (each field shows a placeholder until its source
emits)?

**Recommended (analyst):** **eagerly populate the three
config-static fields** (account, server time, connection mode
default), **lazily for latency** (it's a derived signal that
debounces). Architect ratifies the exact wiring in the
Phase 1 design.

### Q5 — Phase 3 audit-writer scope

**The question:** Phase 3's pause-strategy + override-risk-veto
controls — do they **emit new strategy_events** (`event_kind =
"strategy_paused"` etc.), or stay UI-only (the operator's pause
is a runtime flag in `Cockpit` model, not persisted)?

**Recommended (analyst):** **emit strategy_events**. The audit
ledger is the canonical "why" per the principles doc; an
operator pause is exactly the kind of decision the operator
will want to look back on. New writers + their unit tests
land in Phase 3, not Phase 1.

**Alternative:** UI-only. Cons: an operator pause vanishes on
restart and leaves no audit trail.

### Q6 — Cross-phase consistency-test discipline

**The question:** the
`crates/ui/tests/consistency.rs` rules
(`no_inline_hex_colors_in_widgets_or_state` etc.) are spec-
locked. Phase 1 expands the legal token list; Phase 3 renames
`tape` to `AgentFeed`. Does the consistency test stay green
across all three phases without re-litigation?

**Recommended (analyst):** **yes, by design**. Phase 1 expands
the legal-token allow-list inside the test; Phase 3 renames
the module and the test allow-list together. The test stays
the source of truth — no exceptions land mid-phase.

### Q7 — Phase 1 scope of `cockpit` (fixtures bin) vs `cockpit_live` parity

**The question:** Phase 1 refreshes both `cockpit` (fixtures
dev bin) and `cockpit_live` (unified live bin), or just the
live bin?

**Recommended (analyst):** **both**. The fixtures bin is the
ui-designer's daily-driver dev tool; refreshing only the live
bin would leave the fixtures bin stale and the dev workflow
visually inconsistent. Same widget code, both bins inherit.

### Q8 — Phase ordering ratification

**The question:** is the 1 → 2 → 3 → 4 ordering correct, or
does the operator value Phase 3 (HumanControl) ahead of Phase
2 (viewer)?

**Recommended (analyst):** **keep 1 → 2 → 3** per the
"lower-risk first" justification above. **Architect surfaces
this to the operator at Phase 2 kickoff** if any data has
shifted (e.g. the operator has grown the strategy count to
5+, making HumanControl's pause-strategy higher-priority).

### Q9 — Forward-compat pre-reservation breadth

**The question:** Phase 1's shell-grid reservation for the
Phase 4 assistant slot — does it land **immediately** (the
shell grid leaves a hidden right rail) or **lazily** (Phase 4
adds the rail when it lands)?

**Recommended (analyst):** **lazily**. Reserving a hidden rail
that doesn't render in any current build is dead code; v2 LLM's
ship will add the rail then. The Phase 1 brief's
[architecture.md](../architecture.md) Frontend forward-compat
note documents the *intent* without committing the *grid*.

### Q10 — Naming convention in Rust for Lumen tokens

**The question:** Lumen's CSS uses kebab-case (`--panel-raised`).
Rust constants are `SHOUTY_SNAKE_CASE`. Does Phase 1 adopt:

- (a) `theme::color::PANEL_RAISED` — direct rename from
  `--panel-raised`; concise.
- (b) `theme::color::tier::raised` — namespaced by tier;
  hierarchical.
- (c) `theme::surface::PANEL_RAISED` — split surface from
  colour ramps; cleaner separation.

**Recommended (analyst):** **(a) flat constants under
`theme::color`** — matches the existing
`crates/ui/src/theme.rs` shape (no submodules), keeps the
`use ui::theme::color;` grep-friendly. Architect ratifies.

## Backlog updates

Effective on this master-roadmap revision (2026-05-04, post Phase 1
ship):

### Active

- **`lumen-design-adoption`** — this master file. Roadmap; tracks
  the rollout of phases 1 → 6.

### Queue (promote per phase, top-down)

- **`lumen-phase-2-shell-ia-charts`** — promotes next (Phase 1
  shipped). Status: queued.
  [`spec/lumen-design-adoption/phase-2-shell-ia-charts/feature.md`](../lumen-design-adoption/phase-2-shell-ia-charts/feature.md)
- **`lumen-phase-3-detail-screens`** — promotes on Phase 2 ship.
  Status: queued.
  [`spec/lumen-design-adoption/phase-3-detail-screens/feature.md`](../lumen-design-adoption/phase-3-detail-screens/feature.md)
- **`lumen-phase-4-backtest-panel`** — promotes on Phase 3 ship.
  Status: queued. (Was originally Phase 2.)
  [`spec/lumen-design-adoption/phase-4-backtest-panel/feature.md`](../lumen-design-adoption/phase-4-backtest-panel/feature.md)
- **`lumen-phase-5-humancontrol-agentfeed`** — promotes on Phase 4
  ship. Status: queued. (Was originally Phase 3.)
  [`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/feature.md`](../lumen-design-adoption/phase-5-humancontrol-agentfeed/feature.md)
- **`lumen-phase-6-assistant-slot`** — `_reserved_`, linked to v2
  LLM. No analyst spawn until v2 LLM is approved. (Was originally
  Phase 4.)
  [`spec/lumen-design-adoption/phase-6-assistant-slot/feature.md`](../lumen-design-adoption/phase-6-assistant-slot/feature.md)

### Recent (shipped)

- **`lumen-phase-1-foundation`** — shipped 2026-05-04 (tester
  third-pass PASS).
- **`v1.5b-multi-venue`** — shipped 2026-05-03.

### Renamed feature briefs (post-revision)

- The original `lumen-phase-2-viewer-backtest` brief sketch (in this
  master file pre-revision) is now `lumen-phase-4-backtest-panel`.
- The original `lumen-phase-3-human-control-agent-feed` brief
  sketch is now `lumen-phase-5-humancontrol-agentfeed`.
- The original `lumen-phase-4-assistant` brief sketch is now
  `lumen-phase-6-assistant-slot`.

No prior on-disk feature brief existed for original Phases 2/3/4
(they were sketched in the master roadmap only); the rename is a
master-file cross-reference change.

## Initiative status — 5-of-6 phases shipped

As of **2026-05-08**, the lumen-design-adoption initiative is **5-of-6
phases shipped**. Phase 6 (Assistant slot) is reserved until the v2
LLM strategy is approved — it ships zero UI today (right-rail
column-track at `Length::Fixed(0.0)` is the only Phase-6 surface in
the codebase). **Phase 5 is the terminal shippable phase of this
initiative absent v2 LLM.**

| Phase | Status | Shipped | Approval | Tester report |
|---|---|---|---|---|
| 1 — Foundation | ✅ Shipped | 2026-05-04 | 2026-05-04 | [`test-2026-05-04c-lumen-phase-1-foundation.md`](../archive/tester-reports-2026-05-to-06.tar.gz) |
| 2 — Shell IA + Charts | ✅ Shipped | 2026-05-05 | 2026-05-05 | [`test-2026-05-05-lumen-phase-2-shell-ia-charts.md`](../archive/tester-reports-2026-05-to-06.tar.gz) |
| 3 — Detail screens | ✅ Shipped | 2026-05-05 | 2026-05-06 | [`test-2026-05-05-lumen-phase-3-detail-screens.md`](../archive/tester-reports-2026-05-to-06.tar.gz) |
| 4 — Backtest panel | ✅ Shipped | 2026-05-06 | 2026-05-06 | [`test-2026-05-06b-lumen-phase-4-backtest-panel.md`](../archive/tester-reports-2026-05-to-06.tar.gz) |
| 5 — HumanControl + AgentFeed | ✅ Shipped | 2026-05-07 | 2026-05-08 | [`test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md`](../archive/tester-reports-2026-05-to-06.tar.gz) |
| 6 — Assistant slot | _Reserved_ | _gated on v2 LLM_ | — | — |

**Cumulative numbers as of Phase 5 ship:** 896 tests passed across
110 binaries (Phase 1 baseline: 757 / 96; Phase 5 net delta:
+139 tests / +14 binaries). 86 snapshot baselines (67 panel + 17
widget + 2 audit). 11/11 backtest body-SHA-256 anchors byte-identical
through every phase. Three bins in the workspace
(`viewer` / `cockpit` / `cockpit_live`). Every operator-write surface
gates through a typed-confirm flow with focus-ring halo.

**Cross-phase technical-debt rollup (post Phase 5 ship):**
- TD-1 (focus-ring deferral) — **CLOSED** via Path (b) custom-widget
  escape hatch.
- TD-2 (risk-engine veto-emit upstream wiring) — **OPEN**, Phase 5
  deferral. Override surface ships over an empty live `Vec<VetoEvent>`;
  not a safety primary, an observability gap. Promotes when an
  upgrade trigger fires.

**Next steps after Phase 5 ship** (operator-decided):

1. **Promote v2 LLM** — its own analyst → architect → developer
   pipeline. Largest queued backend feature. When v2 LLM ships,
   Phase 6 unlocks.
2. **Promote a different Active backlog item** — see
   [`spec/backlog.md`](../backlog.md) Active section.
3. **Pause the cockpit-side initiative** — declare 5-of-6
   complete; let the v2 LLM rollout pick Phase 6 up when it
   gets there.

The roadmap-level decision is at the operator level; this master
file does not commit a path until it is taken.

## Changelog

- 2026-05-08 (orchestrator, post-Phase-5 ship): Phase 5 (HumanControl
  + AgentFeed rename) operator-approved. Phase 5 master-section status
  bumped Queue → Shipped (2026-05-07 tester second-pass PASS,
  2026-05-08 operator approval). Added **"Initiative status — 5-of-6
  phases shipped"** section above changelog with cumulative-numbers
  table + cross-phase TD rollup + next-step decision frame. **TD-1
  CLOSED** in the Cross-phase technical-debt items section (Path (b)
  custom-widget escape hatch shipped). **TD-2 NEW** row added for
  risk-engine veto-emit upstream wiring deferral. Phase 5 is the
  terminal shippable phase of this initiative absent v2 LLM; the
  cockpit-side rollout reaches a natural pause point. Initiative
  next-step decision routed to operator (promote v2 LLM, promote a
  different Active backlog item, or declare 5-phase-complete).
- 2026-05-04 (analyst, post-Phase-1 revision): **roadmap revised
  from 4 phases to 6 phases** at operator request (session of
  2026-05-04, after Phase 1 third-pass tester PASS). Two new
  phases inserted ahead of the original Phase 2 / 3 / 4:
  - **Phase 2 — Shell IA + Charts** (left sidebar nav, Home /
    Debug / Charts screens, per-symbol price chart with buy/sell
    markers from a new filtered audit query, both bins).
  - **Phase 3 — Detail screens** (Strategies / Risk / Audit
    sidebar entries, read-only over existing backend data).
  Original Phase 2 (Backtest panel) → Phase 4. Original Phase 3
  (HumanControl + AgentFeed) → Phase 5. Original Phase 4
  (Assistant slot) → Phase 6 (still reserved for v2 LLM).
  Operator decisions captured as Q11–Q14 in the master open-
  questions section: Q11 sidebar primacy = fixed-width;
  Q12 chart data = both modes (live + fixtures with synthetic
  candles); Q13 marker query = extend `audit::query` (additive);
  Q14 Phase 2/3 split = keep. Anchor risk per phase table extended
  to 6 rows; cross-feature invariants table extended to 6 phase
  columns; adoption-strategy diagram redrawn. Phase 1 status
  bumped to **shipped** in this file. Five new feature-brief
  stubs spawned at queue-status (each is a placeholder that the
  per-phase analyst expands at promotion time per Q3 of the
  original master Q-list). HANDOFF → architect at Phase 2 promote
  (when the operator signs Phase 1 presentation).
- 2026-05-04 (architect): added "Cross-phase technical-debt items"
  section + first row **TD-1 — True keyboard-focus ring** capturing
  the Phase 1 Q11 deviation ratification. iced 0.14.2 lacks
  `button::Status::Focused` and `text_input::Style.shadow`; Phase 1
  ships hover-state ring on buttons + ACCENT border-shift on focused
  inputs as a bounded approximation. Two named upgrade triggers
  (iced version bump or custom-widget escape hatch); earliest
  re-evaluation at Phase 2 analyst kickoff. T1504 tick stands;
  acceptance bullet read against the Q11 ratification, not literal.
  Anchor risk zero; cross-feature invariants unchanged. See
  [architecture.md → Q11](../architecture.md#lumen-design-adoption--phase-1-foundation-resolutions-q1q9--master-q10--mid-phase-q11--confirmed-2026-05-04).
- 2026-05-03 (analyst): initial master roadmap. Captures the
  4-phase Lumen design-system adoption per the operator's
  Option-C (full adoption, sequential phases) decision
  (2026-05-03). Operator-locked constraints documented:
  no brand adoption, no voice rewrite, sequential ordering,
  Phase 4 forward-compat only. Phase 1 brief at
  [`lumen-phase-1-foundation.md`](../lumen-design-adoption/phase-1-foundation/feature.md);
  Phases 2 / 3 / 4 are queued / queued / reserved.
  HANDOFF → architect (Phase 1 first; master roadmap for
  orientation).
