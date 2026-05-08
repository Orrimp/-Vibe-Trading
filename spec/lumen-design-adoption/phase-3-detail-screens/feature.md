---
slug: lumen-phase-3-detail-screens
status: shipped
owner: analyst
updated: 2026-05-05
version: 2.2.0
---

# Lumen Phase 3 — Detail screens (Strategies / Risk / Audit)

> **Phase 3 of 6** in the
> [`lumen-design-adoption`](lumen-design-adoption.md) initiative.
> Master roadmap is the orientation; this brief is the **shippable
> feature**. Operator-locked constraints (no brand, no voice rewrite,
> sequential phases, Phase 6 reserved, no icons until needed) are
> documented in the master file and apply here without re-litigation.
>
> **Operator-locked decisions inherited from the 2026-05-04 master
> revision (Q11–Q14) — not re-opened in this brief:**
>
> - **Q11** — sidebar nav primacy = **fixed-width** (~180 px,
>   text-only labels, no icons). The Phase 3 sidebar grows from 3
>   entries to 6; the widget API parameterisation Phase 2 R1.6
>   anticipated handles the extension without re-litigating Q11.
> - **Q12** — chart data source = **both modes**. Phase 3 inherits
>   the `ChartBuffer` for the optional Strategies-detail equity-
>   since-deploy sparkline (Q6 below); fixtures parity is preserved.
> - **Q13** — buy/sell marker query method placement = **extend
>   [`crates/audit/src/query.rs`](../../crates/audit/src/query.rs)**.
>   Phase 3 follows the same pattern: any new audit-read additions
>   land in `query.rs` next to `recent_fills_filtered` (Phase 2
>   R12), not in a new module.
> - **Q14** — Phase 2 / 3 split = **kept**. Phase 3 ships exactly
>   Strategies + Risk + Audit detail screens; no other surfaces.
>
> The brief expands on the Phase 2 ship dated 2026-05-05
> (`T_FINAL_LUMEN_PHASE_2` first-pass PASS). Phase 3 inherits Phase
> 1's tokens / tiers / status bar, Phase 2's screen-routed shell +
> sidebar nav widget + `Screen` enum (already declares `Strategies
> / Risk / Audit` as variants — the dispatch returns "Not yet"
> placeholders pre-Phase 3) + per-`(Venue, Symbol)` `ChartBuffer`
> + `recent_fills_filtered` audit query + right-rail Phase 6
> reservation.

## Why

Phase 2 closed two cockpit gaps — information hierarchy
(sidebar + Home / Debug split) and visual cross-check (Charts).
Phase 3 closes the **last operator-visible data gap** at v1.5b:
three pieces of backend data that **already exist** but have no
UI surface today.

1. **Per-strategy detail.** `crates/strategy` produces signals;
   `crates/audit` writes `strategy_events` rows; `config/agent.toml`
   carries each strategy's `[[strategy]]` block. Home's Strategies
   summary panel shows one row per strategy; nowhere does the
   operator currently see "what params is `sma-fast-slow` running
   with, and what signals has it emitted in the last hour".
2. **Risk / limits.** `crates/risk::portfolio` enforces per-symbol
   and portfolio caps; `crates/agent::config::RiskConfig` carries
   `per_symbol_exposure_cap`, `daily_loss_stop_pct`,
   `max_drawdown_stop_pct`; the kill switch reads from
   `KillSwitchConfig`. Today the only "approaching breach" signal
   is the kill switch tripping — no early warning.
3. **Audit / journal.** The ledger holds every fill / strategy
   event / reconciliation row since inception. The
   [`tape-row-audit-modal`](tape-row-audit-modal.md) flow surfaces
   per-row detail for rows in the current tape; there is no full-
   ledger browser. "Every fill on Coinbase yesterday" requires
   hand-querying SQLite today.

Phase 3 closes all three as **read-only surfaces over data already
landing in `crates/strategy`, `crates/agent`, `crates/risk`, and
`crates/audit`**. Three sidebar entries, three screens, **no new
audit writers** (Phase 5 HumanControl introduces the first new
operator-write paths). The brief is deliberately tight — Phase 3
inherits more Phase-2 primitives than Phase 2 inherited Phase-1
primitives, so each new screen is mostly a composition of existing
widgets / patterns / query methods.

## Scope (high-level)

Phase 3 ships, in one merge:

- **Sidebar extension** (R1–R3) — three new nav entries inserted
  between Debug and Charts; Phase 2's widget API parameterisation
  carries this without a widget rewrite.
- **Strategies-detail screen** (R4–R6) — per-strategy view with
  read-only params, recent signal events, optional equity-since-
  deploy sparkline (deferred if expensive).
- **Risk / Limits screen** (R7–R8) — per-venue exposure vs caps
  + daily loss limit consumed + kill-threshold proximity gauge
  (horizontal bar, Phase 1 latency-band colour precedent).
- **Audit / Journal screen** (R9–R12) — full ledger browser with
  filter row + pagination + reused `journal_transaction_modal`;
  audit query extension or sibling method for non-fill rows.
- **`journal_transactions.venue` migration** (R13) — additive
  prerequisite the Audit screen's multi-venue surface depends on.
- **Cross-feature invariants preservation** (R14) and **11 / 11
  anchor regression PASS** (R15).

R-clusters: R1–R3 sidebar + screen routing extension; R4–R6
Strategies-detail; R7–R8 Risk / Limits; R9–R12 Audit / Journal
+ audit-query additions; R13 venue migration; R14–R15 invariants
+ anchors.

## Anchor risk

**Zero. State this loudly.** Phase 3 is purely additive over
existing backend data:

- Three new screen modules in `crates/ui/src/screens/` plus three
  new sidebar entries; no widget renames, no widget removals.
- The audit-query additions (R10–R12) are read-only over existing
  rows — the Audit screen's filter, pagination, and the optional
  `recent_journal_filtered` sibling method are new readers, no
  writers.
- The `journal_transactions.venue` migration (R13) is **additive**:
  new column with `DEFAULT NULL`, backfill `'Binance'` for
  existing rows (the v0–v1.5a single-venue assumption per Phase 2
  Design — every shipped fill on disk today is Binance), no row
  rewrites that change description or amount fields.
- No strategy / exec / risk / cost / backtest / reports crate is
  touched. The Risk screen reads risk **state** (mirror fields on
  `Cockpit` fed by the agent runtime); it does not modify limits,
  emit kill events, or change risk-engine behaviour.
- 11 / 11 backtest body-SHA-256 anchors stay **byte-identical** at
  the Phase 3 tester gate. No re-lock budget. No exceptions.

## Snapshot ripple

Expected: ~9–12 net-new baselines (3 per detail screen × 3 screens
covering loading / ready / empty states; sidebar refresh showing 6
entries instead of 3; chip-row variants on Audit-screen filter).
Phase 2's existing baselines remain valid where they don't depend
on the sidebar entry count — the sidebar-nav baselines refresh
once to show the 6-entry default, the per-screen Home / Debug /
Charts baselines are byte-identical (the screens themselves don't
change). **Single `cargo insta accept` pass at end of phase** per
Phase 1 Q2 / Phase 2 V11 precedent.

## Requirements

Numbered, testable, derived from the master roadmap's Phase 3
scope, the architecture-level **Cockpit screen routing (Phase 2+
contract)** in [architecture.md § 3272](../architecture.md), the
[`spec/product.md` § Cockpit information architecture](../product.md)
contract, and the **Information architecture** + **Charts**
sections of [`spec/ui-design-principles.md`](../ui-design-principles.md).
Each R-item ends with a one-line acceptance the tester verifies.
Every R-item preserves the operator-locked constraints and the
cross-feature invariants in the
[master roadmap](lumen-design-adoption.md#cross-feature-invariants).

### R1 — Sidebar nav extends from 3 to 6 entries

- **R1.1** Insert three new entries **between Debug and Charts**
  per the master-roadmap order. Final scan order: **Home → Debug →
  Strategies → Risk → Audit → Charts**. (Q8 below ratifies the
  insertion order.)
- **R1.2** The sidebar widget body
  ([`crates/ui/src/widgets/sidebar_nav.rs:48`](../../crates/ui/src/widgets/sidebar_nav.rs))
  is **unchanged**. Phase 2 R1.6 parameterised the entry list;
  Phase 3 adds a `SIDEBAR_ENTRIES_PHASE_3` constant and the bin
  call-sites swap from `_PHASE_2` to `_PHASE_3`.
- **R1.3** Label strings already exist
  ([`crates/ui/src/strings.rs:230–235`](../../crates/ui/src/strings.rs)
  ships all six `SIDEBAR_NAV_*` constants per Phase 2 forward-compat
  declare-now); the `label_for(Screen)` match arm
  ([`sidebar_nav.rs:28`](../../crates/ui/src/widgets/sidebar_nav.rs))
  already handles all six variants. Phase 3 wakes the dormant
  strings — **no `ui::strings` rewrite** (operator-locked Constraint 2).
- **R1.4** T1507 active-row pattern continues — 2 px ACCENT left
  rule on the active row, no fill change.
- **R1.5** Fixed-width ~180 px sidebar preserved (Q11). Six entries
  × ~24 px + padding ≈ 200 px column — comfortably within typical
  desktop viewport.
- **Acceptance:** `sidebar_nav__six_entries.snap` +
  `sidebar_nav__active_{strategies,risk,audit}.snap` PASS. `cargo
  test -p ui sidebar_nav` PASSES.

### R2 — Screen routing dispatches the three new screens

- **R2.1** The `Screen` enum already declares `Strategies / Risk /
  Audit` ([state.rs:47–51](../../crates/ui/src/state.rs)) per
  Phase 2's declare-now decision. **Phase 3 adds zero enum
  variants** — only the dispatch changes.
- **R2.2** The shell's `screen_body(current_screen, &cockpit)`
  match arm switches the three variants from Phase 2's "Not yet"
  placeholder to the new screen modules' `view()` calls.
- **R2.3** Three new screen modules under
  `crates/ui/src/screens/`: `strategies.rs`, `risk.rs`, `audit.rs`,
  each exposing `pub fn view(state: &Cockpit, mode: ThemeMode) ->
  Element<Message>` matching the Phase 2 shape
  ([`crates/ui/src/screens/mod.rs`](../../crates/ui/src/screens/mod.rs)).
- **R2.4** `Message::SwitchScreen(Screen)` stays a pure assignment;
  Phase 2's `state::tests::switch_screen_is_pure` inherits unchanged.
- **R2.5** Screens read data **straight from `Cockpit`** — no
  on-entry async load. Bus owns freshness;
  [`spec/ui-design-principles.md` § Screens are pure render
  dispatches](../ui-design-principles.md) carries forward.
- **Acceptance:** both bins launch; clicking each new sidebar
  entry renders a non-placeholder screen (verified by absence of
  the Phase 2 `SCREEN_NOT_YET_PLACEHOLDER` string).

### R3 — Sidebar entry-list constant

- **R3.1** Add `pub const SIDEBAR_ENTRIES_PHASE_3: &[Screen] =
  &[Home, Debug, Strategies, Risk, Audit, Charts];` to
  `crates/ui/src/theme/layout.rs` next to the existing Phase 2
  constant.
- **R3.2** Both bins
  ([`bin/cockpit.rs`](../../crates/ui/src/bin/cockpit.rs),
  [`bin/cockpit_live.rs`](../../crates/ui/src/bin/cockpit_live.rs))
  swap their sidebar call-site to pass `SIDEBAR_ENTRIES_PHASE_3`.
- **R3.3** Remove `SIDEBAR_ENTRIES_PHASE_2` on Phase 3 ship —
  no forward-compat need.
- **Acceptance:** both bins render the 6-entry sidebar; Phase 2
  3-entry sidebar baseline updates to the Phase 3 6-entry shape
  in one snapshot pass.

### R4 — Strategies-detail screen layout

- **R4.1** New module `crates/ui/src/screens/strategies.rs`. `pub
  fn view(state: &Cockpit, mode: ThemeMode) -> Element<Message>`,
  matching Phase 2's one-file-per-screen shape.
- **R4.2** Layout, top-to-bottom:
  - **Strategy chip row** — one chip per loaded strategy from
    `Cockpit::strategies` (Phase 1 R5). Selected chip uses the
    **T1609 horizontal active-chip pattern** (Phase 2 R6.3 / Q5 —
    bottom-edge 2 px ACCENT rule, no fill change).
  - **Params block** — read-only key-value rows from the active
    strategy's `[[strategy]]` block in `config/agent.toml`,
    rendered via `frame::col_header` + `widgets::num`.
  - **Recent signal events table** — newest-first, capped at 50
    rows; columns: timestamp, kind, symbol, rationale. Reuses
    `widgets::num` and the vertical T1507 active-row pattern.
  - **Optional equity-since-deploy sparkline** top-right (R6, Q6).
    Empty-state placeholder when deferred.
- **R4.3** Tier 1 chrome via `frame::panel(title)`. Outer padding
  `space::L (16 px)`; inter-section gap `space::M (12 px)`.
- **R4.4** **Read-only** — no edit / deploy / pause buttons. Pause +
  override are Phase 5 HumanControl's ask; Phase 3 ships zero new
  operator-write paths. (Q10.)
- **R4.5** Empty state — no strategy selected → chip row + centred
  `frame::muted_body` "Select a strategy".
- **Acceptance:** `strategies_screen__sma_crossover_default.snap`
  + `strategies_screen__empty_state.snap` PASS.

### R5 — Strategies-detail data path

- **R5.1** Add `pub selected_strategy: Option<StrategyId>` to
  `Cockpit` ([`crates/ui/src/state.rs`](../../crates/ui/src/state.rs))
  with default `None`. Session-scoped persistence per Phase 2 Q8
  (no on-disk state).
- **R5.2** Two selection paths, both ending in `Message::SelectStrategy(StrategyId)`:
  1. Chip click on Strategies screen → pure assignment in `update`
     (mirrors `Message::SelectSymbol`).
  2. Strategy row click on Home → Strategies-summary panel emits
     the same message **and** `Message::SwitchScreen(Screen::Strategies)`
     via the binary's `Task::perform` shim. (Q11 ratifies whether
     this is compound dispatch or a new `OpenStrategy` variant.)
- **R5.3** Params source — `pub strategies_config: Option<StrategiesConfig>`
  on `Cockpit`, populated once at boot from
  [`agent::config::Config.strategies`](../../crates/agent/src/config.rs)
  (live) or `StrategiesConfig::default()` (fixtures). Static for
  the session per Phase 2 `universe` precedent.
- **R5.4** Signal-events source — Q2 ratification: filter the
  existing `Cockpit::strategies_recent_events` (Phase 1 R5,
  populated from the `crates/strategy::Decision` channel) by
  `selected_strategy` at view time. **No new audit writer.**
- **R5.5** Selection persists across screen switches per Phase 2
  `selected_symbol` precedent.
- **Acceptance:** `select_strategy_persists_across_screen_switch`
  unit test in `state::tests` PASSES.

### R6 — Strategies-detail equity-since-deploy sparkline (optional)

- **R6.1** Small sparkline in the top-right of Strategies screen
  showing equity-since-deploy for the active strategy. Width
  ~120 px, height ~36 px, `ACCENT` line, no axes, no tooltip.
- **R6.2** Cheap path — derived from `Cockpit::pnl` historical
  buffer cached at strategy-load time → ships in Phase 3.
- **R6.3** Expensive path — requires a new
  `pnl_by_strategy_history(strategy_id, since)` audit query →
  **defer to Phase 4**. Q6 ratifies which path lands.
- **R6.4** When deferred, render `frame::muted_body` "Equity
  sparkline lands with Phase 4" placeholder.
- **Acceptance:** either `strategies_screen__sparkline_present.snap`
  (cheap) or `strategies_screen__sparkline_deferred.snap` (deferred).

### R7 — Risk / Limits screen layout

- **R7.1** New module `crates/ui/src/screens/risk.rs`. `pub fn
  view(state: &Cockpit, mode: ThemeMode) -> Element<Message>`.
- **R7.2** Layout (single column, Tier 1 chrome), top-to-bottom:
  - **Per-venue exposure section** — one row per `(Venue, Symbol)`
    with current notional / per-symbol cap as a horizontal bar.
    Colour ramp: `ACCENT` <70 %, `WARN_500` >70 %, `DOWN_500`
    >90 % (matches Phase 1 latency-band precedent /
    `theme::color_for_latency_ms`).
  - **Daily loss section** — single bar: used / `daily_loss_stop_pct`
    from
    [`agent::config::RiskConfig`](../../crates/agent/src/config.rs:181).
    Same ramp.
  - **Kill-threshold proximity gauge** — single bar:
    time-since-last-heartbeat / `heartbeat_timeout_ms` from
    `KillSwitchConfig`. Same ramp.
- **R7.3** Horizontal bars per Q9 (analyst recommendation;
  consistency with the rest of the screen). Bar = `Container` with
  left-aligned filled portion + right-aligned "X / Y (Z %)" label.
- **R7.4** Tier 1 chrome via `frame::panel(title)`; outer padding
  `space::L (16 px)`; section gap `space::M (12 px)`.
- **R7.5** **Read-only.** No edit / "raise the limit" buttons —
  Phase 5 HumanControl's ask (Q10).
- **R7.6** Empty state — `Cockpit::risk_state == Loading` →
  `frame::muted_body` "Risk state loading".
- **Acceptance:** three colour-band snapshots
  (`risk_screen__{under_warn,warn,danger}_threshold.snap`) PASS.

### R8 — Risk / Limits data path

- **R8.1** Add `pub risk_state: PanelState<RiskState>` to `Cockpit`.
  `RiskState` is a new struct in `crates/ui/src/state.rs`:

  ```rust
  pub struct RiskState {
      pub per_symbol_exposure: HashMap<(Venue, Symbol), Decimal>,
      pub per_symbol_caps: HashMap<(Venue, Symbol), Decimal>,
      pub daily_loss_used_pct: Decimal,
      pub daily_loss_cap_pct: Decimal,
      pub heartbeat_age_ms: u64,
      pub heartbeat_timeout_ms: u64,
  }
  ```

- **R8.2** Live wiring — Q3 ratification: **new tokio channel** from
  the agent runtime to the cockpit, mirroring Phase 1 `MarketHealth`.
  The risk engine
  ([`crates/risk/src/portfolio.rs`](../../crates/risk/src/portfolio.rs))
  publishes a `RiskTelemetry` snapshot on the bus at 1 Hz; the
  cockpit subscription emits `Message::RiskStateRefreshed(RiskState)`.
- **R8.3** Fixtures wiring — pre-seed `cockpit.risk_state =
  PanelState::Ready(fake_risk_state())` at fixtures-bin boot in
  `crates/ui/src/fixtures.rs`. `fake_risk_state` returns
  deterministic numbers: one venue/symbol <70 %, one 80 %
  (`WARN_500`), one 95 % (`DOWN_500`) — V5 covers all three bands.
- **R8.4** `Message::RiskStateRefreshed(RiskState)` is a pure
  assignment in `update`; bus subscription lives in the binary's
  `Subscription::batch`.
- **R8.5** Memory bound: trivially small (~20 symbols × 64 bytes ≤
  2 KB); no compaction.
- **Acceptance:** `risk_state_refresh` unit test in `state::tests`
  PASSES.

### R9 — Audit / Journal screen layout

- **R9.1** New module `crates/ui/src/screens/audit.rs`. `pub fn
  view(state: &Cockpit, mode: ThemeMode) -> Element<Message>`.
- **R9.2** Layout, top-to-bottom:
  - **Filter row** — venue chips (Binance / Coinbase / Kraken,
    multi-select toggle), symbol text input (exact match), kind
    chips (All / Fill / StrategyEvent / Reconciliation),
    time-range chips (Last 1 h / Last 24 h / Last 7 d). Active
    chips use the Phase 2 T1609 horizontal active-chip pattern.
  - **Pagination header** — "Showing 1–250 of N" + Prev / Next
    (disabled at boundaries).
  - **Journal table** — newest-first; columns: timestamp, venue,
    symbol, kind, description, strategy_id. Row click opens the
    existing `widgets::journal_transaction_modal` (T1208 reused).
- **R9.3** Pagination = **fixed 250 rows / page** (Q4); page state
  on `Cockpit::audit_screen_state`.
- **R9.4** Filter persistence = **in-session only** (Q5); cleared
  on cockpit restart.
- **R9.5** Tier 1 chrome via `frame::panel(title)`; outer padding
  `space::L (16 px)`; filter / table gap `space::M (12 px)`.
- **R9.6** Empty state — `frame::muted_body` "No journal rows match
  these filters". Loading state — Phase 1 `PanelState::Loading`
  skeleton.
- **Acceptance:**
  `audit_screen__default_recent_24h.snap` (≥ 5 fixtures rows),
  `audit_screen__filter_no_match.snap`, and
  `audit_screen__pagination_page2.snap` (fixtures seed 250 + 5
  rows) PASS.

### R10 — Audit screen data path + state

- **R10.1** Add to
  [`crates/ui/src/state.rs`](../../crates/ui/src/state.rs):

  ```rust
  pub struct AuditScreenState {
      pub filter: AuditFilter,
      pub page: u32,                     // 0-indexed
      pub rows: PanelState<Vec<JournalRow>>,
      pub total_count: Option<u64>,
  }

  pub struct AuditFilter {
      pub venues: Vec<Venue>,            // empty = all
      pub symbol: Option<Symbol>,        // None = all
      pub kind: AuditKindFilter,         // All / Fill / StrategyEvent / Reconciliation
      pub time_range: AuditTimeRange,    // Last1H / Last24H / Last7D
  }
  ```

  Plus `pub audit_screen_state: AuditScreenState` on `Cockpit`.
- **R10.2** Three new `Message` variants:
  `AuditFilterChanged(AuditFilter)` (pure; resets `page` to 0,
  triggers refetch via `Task::perform`),
  `AuditPageChanged(u32)` (pure; triggers refetch),
  `AuditRowsLoaded(Result<(Vec<JournalRow>, u64), SmolStr>)`
  (async result → `Ready` / `Error` + `total_count`).
- **R10.3** Async fetch lives in the binary's `Task::perform`
  shim; calls `recent_journal_filtered` (R12.1).
- **R10.4** Pure-function `update` preserved (Phase 2 `SelectSymbol`
  precedent — async work in binary, not in `update`).
- **Acceptance:** `audit_filter_changed_resets_page` unit test in
  `state::tests` PASSES.

### R11 — Audit row → modal trigger reuses T1208

- **R11.1** Per-row click emits `Message::TapeRowClicked(tx_id)`
  (Phase 1 variant — row-click semantics are venue-agnostic). The
  existing
  [`widgets/journal_transaction_modal.rs`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  opens unchanged.
- **R11.2** Modal stays wrapped at the shell level (Phase 2 R3.3) —
  Audit rows open it identically to Home tape rows.
- **R11.3** **No widget rename, no widget code change.** Modal
  continues to render `description` + `strategy_id` (R14.2).
- **R11.4** Q11b ratifies whether to literally reuse
  `Message::TapeRowClicked` or add a new `JournalRowClicked(tx_id)`.
  Analyst recommends literal reuse — no new variant for the same
  action.
- **Acceptance:** `cargo test -p ui audit_row_opens_modal`
  integration test in `crates/ui/tests/` PASSES.

### R12 — Audit-query method extension for non-fill rows

- **R12.1** Add to
  [`crates/audit/src/query.rs`](../../crates/audit/src/query.rs)
  alongside Phase 2's `recent_fills_filtered`:

  ```rust
  /// Phase 3 addition. Return all journal rows matching the
  /// filter, newest-first, paginated. Read-only; additive.
  pub async fn recent_journal_filtered(
      ledger: &Ledger,
      venues: &[Venue],
      symbol: Option<&Symbol>,
      kind: AuditKindFilter,
      since: Timestamp,
      until: Timestamp,
      page_offset: u32,
      page_size: u32,
  ) -> Result<(Vec<JournalRow>, u64), LedgerError>;
  ```

- **R12.2** Q7 — analyst recommends **add a sibling** rather than
  extend `recent_fills_filtered` with `Option<&str> kind`. The
  predicate diverges (fills scan description-prefixed rows;
  non-fill rows scan `strategy_events` + reconciliation tables);
  splitting reads cleaner.
- **R12.3** Implementation: SQL projection over
  `journal_transactions` joined with `strategy_events` (and
  optionally reconciliation tables) filtered by the predicate.
  `ORDER BY ts DESC, rowid DESC` per Phase 2 R12.5. No `f64`.
- **R12.4** Empty result returns `Ok((vec![], 0))`; never `Err`.
- **R12.5** Mandatory unit test in `crates/audit/src/query.rs::tests`
  exercising each filter variant + an empty-window result.
- **R12.6** Mandatory integration test at
  `crates/audit/tests/recent_journal_filtered.rs` — multi-venue,
  multi-symbol, multi-kind. (Phase 2 Q10 deferred this for
  `recent_fills_filtered`; the Audit screen is the natural
  consumer that promotes it now.)
- **Acceptance:** unit + integration test PASS.

### R13 — `journal_transactions.venue` migration (Phase 3 prerequisite)

- **R13.1** Phase 2's `recent_fills_filtered` returns `Ok(vec![])`
  for `venue != Binance` because `journal_transactions` carries no
  venue column today
  ([`crates/audit/src/query.rs:191`](../../crates/audit/src/query.rs)).
  Phase 3's Audit screen needs multi-venue fills — the migration is
  a Phase 3 prerequisite. Q1 ratifies whether to ship in Phase 3
  or split as Phase 3.5.
- **R13.2** Migration shape (next-numbered):

  ```sql
  ALTER TABLE journal_transactions
    ADD COLUMN venue TEXT NOT NULL DEFAULT 'Binance';
  ```

  Default `'Binance'` backfill is honest — every shipped fill on
  disk is Binance per Phase 2 Design (v1.5b is plumbing-only).
- **R13.3** Writer update —
  `crates/audit/src/journal.rs::record_fill` (or the relevant
  helper) gains a `venue: Venue` parameter and stamps on insert.
  Existing call-sites pass `Venue::Binance`; v1.5b's eventual
  multi-venue exec path stamps the actual venue.
- **R13.4** Phase 2 query update — `recent_fills_filtered` drops
  its venue gate and gains a `WHERE venue = ?` predicate.
- **R13.5** Anchor risk: **zero**. Additive migration with
  constant-string backfill; no description / amount rewrites.
- **R13.6** Mandatory migration test in `crates/audit/tests/` —
  open a pre-migration fixture DB, run the migration, assert
  every existing row has `venue = 'Binance'`.
- **Acceptance:** `cargo test -p audit migration_009_*` PASS;
  `recent_fills_filtered(&ledger, Coinbase, ...)` returns matching
  rows from a multi-venue fixture ledger.

### R14 — Cross-feature invariants

- **R14.1** [`tape-row-audit-modal`](tape-row-audit-modal.md): Audit
  screen rows open the same modal as tape rows; trigger preserved
  per Phase 2 R14.5.
- **R14.2** [`journal-tx-metadata`](journal-transactions-metadata.md):
  modal continues to render `description` + `strategy_id`. Audit
  screen's filter row surfaces `strategy_id` as a column (R9.2);
  same metadata reader.
- **R14.3** [`v1.5b-multi-venue`](v1-5b-multi-venue.md): venue
  dimension surfaces on Audit filter chips + Risk exposure section.
  R13 migration completes v1.5b's "fills carry venue" story.
- **R14.4** [`live-cockpit-unified`](live-cockpit-unified.md):
  halted-banner trigger preserved (shell-level per Phase 2 R3.3).
- **R14.5** [`real-mtm-unrealized-pnl`](real-mtm-unrealized-pnl.md):
  PnL card unchanged. Strategies-detail sparkline (R6, if cheap
  path lands) reads from `Cockpit::pnl` buffer; no `color_for_delta`
  signature change.
- **R14.6** [`per-symbol-position-accounts`](per-symbol-position-accounts.md):
  Positions widget unchanged. Risk-screen exposure reads
  `Cockpit::positions` + the new `risk_state` mirror (R8.1).
- **R14.7** [`operator-success-reports`](operator-success-reports.md):
  Debug-screen latency badge colour mapping unchanged.
- **Acceptance:** tester's per-feature invariant table = 7 / 7 PASS.

### R15 — Anchor regression

- **R15.1** All 11 backtest body-SHA-256 anchors in
  [`spec/anchors.toml`](../anchors.toml) verify byte-identical
  post-Phase 3.
- **R15.2** No new anchor scenarios; no re-lock budget; zero
  exceptions. The `journal_transactions.venue` migration (R13)
  is additive with a constant-string backfill — no description /
  amount rewrites — and the `recent_journal_filtered` query is
  read-only over the same rows already exposed by `recent_journal`
  + `recent_fills`.
- **R15.3** `verify-anchors` skill PASS at the Phase 3 tester
  gate.
- **Acceptance:** the tester's anchor table is 11 / 11 PASS.

## Verification (V-items)

The tester gates Phase 3 ship against these V-items. Each maps to
its R-cluster.

- **V1 — Both bins launch with 6-entry sidebar.** `cargo run --bin
  cockpit --features fixtures` and `cargo run --bin cockpit_live
  --features live` launch; sidebar shows six entries in scan order
  Home → Debug → Strategies → Risk → Audit → Charts; Home active by
  default. `sidebar_nav__six_entries` snapshot PASS. (R1, R2, R3.)

- **V2 — Strategies-detail renders chips + params + events.** From
  fixtures bin, click Strategies; chip row renders one chip per
  loaded strategy; first chip active; params block ≥ 3 rows;
  events table ≥ 3 rows. `strategies_screen__sma_crossover_default`
  snapshot PASS. (R4, R5.)

- **V3 — Strategies-detail selection persists.** `cargo test -p ui
  state::tests::select_strategy_persists_across_screen_switch`
  PASSES. (R5.5.)

- **V4 — Strategies-detail equity sparkline (or placeholder).** Q6
  ratification determines which fires —
  `strategies_screen__sparkline_present` (cheap path) or
  `strategies_screen__sparkline_deferred` (placeholder). (R6.)

- **V5 — Risk screen renders three coloured bands.** From fixtures
  bin, click Risk; the per-venue section shows three bars in
  `ACCENT` / `WARN_500` / `DOWN_500` per the deterministic
  `fake_risk_state`. Three colour-band snapshots PASS. (R7, R8.)

- **V6 — Audit screen renders + opens modal.** From fixtures bin,
  click Audit; filter row + table render; click first row → existing
  modal opens with matching `tx_id`.
  `audit_screen__default_recent_24h` snapshot +
  `audit_row_opens_modal` integration test PASS. (R9, R10, R11.)

- **V7 — Audit filter mutates row set.** Change venue filter →
  table re-renders with new count; change kind chip → table
  re-renders again. `audit_screen__filter_no_match` snapshot PASS.
  (R10.)

- **V8 — `recent_journal_filtered` unit + integration tests.**
  `cargo test -p audit query::tests::recent_journal_filtered_*` and
  `cargo test -p audit --test recent_journal_filtered` PASS. (R12.)

- **V9 — Migration 009 + venue-aware fills.** `cargo test -p audit
  migration_009_*` PASSES; pre-migration fixture DB leaves every
  existing row at `venue = 'Binance'`; post-migration,
  `recent_fills_filtered(&ledger, Coinbase, BTCUSDT, since, until)`
  returns matching rows from a multi-venue fixture ledger. (R13.)

- **V10 — Cross-feature invariants.** 7 / 7 PASS in tester's
  per-feature invariant table. (R14.)

- **V11 — Anchors 11 / 11 PASS.** `verify-anchors` PASS; no body
  diff. (R15.)

- **V12 — Snapshot baselines.** Single `cargo insta accept` pass
  at end of phase; ~9–12 net-new + 1 refreshed Phase 2 sidebar
  baseline. Phase 2 per-screen baselines byte-identical. (R1, R4,
  R7, R9.)

- **V13 — `rust-validate` PASS.** `cargo fmt`, clippy
  `-D warnings`, cargo-deny, audit (or N-A) all PASS.

## Acceptance criteria

Phase 3 ships when all of the following hold:

- **Both bins launch with the 6-entry sidebar** (Home → Debug →
  Strategies → Risk → Audit → Charts); Home selected by default;
  status bar still spans bottom. (R1, R2, R3.)
- **Strategies-detail screen** reachable via sidebar; renders chip
  row + params + signal events; the Home → Strategies-summary row
  click cross-links to the same screen with the matching chip
  active. (R4, R5.)
- **Risk-detail screen** reachable; renders horizontal bars at the
  named colour thresholds (`ACCENT` <70 %, `WARN_500` >70 %,
  `DOWN_500` >90 %). (R7, R8.)
- **Audit-detail screen** reachable; filter row + paginated table;
  filter mutations change the row set; row click opens the existing
  journal-transaction modal with matching `tx_id`. (R9, R10, R11.)
- **Audit-query** `recent_journal_filtered` exercised by unit +
  integration tests. (R12.)
- **Migration 009** lands additively; backfills `'Binance'`; Phase 2
  `recent_fills_filtered` venue gate removed. (R13.)
- **Cross-feature invariants PASS** (7 / 7) and **11 / 11 anchor
  regression PASS** (byte-identical bodies). (R14, R15.)
- **`rust-validate` PASS** + **single `cargo insta accept` pass**
  for ~9–12 refreshed baselines. (V12, V13.)

## Open questions for architect

Q11–Q14 from the master roadmap are **operator-locked** and not
opened here. The questions below are the genuinely-open design
choices that ratify at architect kickoff. Each ends with a
one-line **analyst recommendation**.

### Q1 — `journal_transactions.venue` migration scope

**The question:** ship the R13 migration **inside Phase 3** (one
merge, one tester gate) or **split as Phase 3.5** (migration-only
ship before Phase 3's UI gates)?

**Recommended (analyst):** **ship in Phase 3**. The migration is
additive with a constant-string backfill — zero anchor risk,
~30 LOC writer sweep, one migration test. The Phase 2 Design
venue-handling note explicitly flagged this as "Phase 3's problem";
carrying the work to a 3.5 split adds gate friction without
isolating actual risk.

**Alternatives considered:** ship as a standalone-operations brief
independent of the design-system roadmap — rejected, the consumer
(Audit screen) lives here.

### Q2 — Strategies-detail signal-history source

**The question:** R5.4 — read events from a **new `signal_emitted`
audit writer** (rationale JSON), or from the **existing
`crates/strategy::Decision` channel** already in `strategies_recent_events`?

**Recommended (analyst):** **existing channel**. The Phase 3
master-roadmap promise is "no new audit writers" (Phase 5 introduces
the first via HumanControl). `Cockpit::strategies_recent_events` is
already populated by the existing strategy-event subscription;
filtering by the active `selected_strategy` is a view-layer change
with zero new writer code.

**Alternatives considered:** new audit writer in Phase 3 — rejected,
violates no-new-writers stance + adds anchor-risk surface.

### Q3 — Risk screen exposure source

**The question:** R8.2 — read `RiskState` via **direct cockpit-thread
reads** from agent state (mutex / async), or via a **new tokio
channel** mirroring the Phase 1 `MarketHealth` pattern?

**Recommended (analyst):** **channel pattern**. The cockpit-thread
isolation rule (no UI-thread reads from agent-runtime mutexes) is a
load-bearing architectural invariant. The Phase 1 `MarketHealth`
bus channel is the canonical example; Phase 3's `RiskTelemetry`
channel is a structural sibling. ~40 LOC publisher in
`crates/agent/src/runtime.rs`, ~20 LOC subscriber recipe in
`crates/ui/src/live.rs`.

**Alternatives considered:** polled query on each `view()` call —
rejected, couples render-rate to agent state-cache locking.

### Q4 — Audit screen pagination size

**The question:** R9.3 — **fixed 250 rows per page** or
**operator-configurable** (chip selector of 100 / 250 / 1000)?

**Recommended (analyst):** **fixed 250**. No operator-stated need
for configurable; cockpit IA forbids surfaces without need. Fixed
250 keeps the SQL `LIMIT` constant, the snapshot baselines
deterministic, and the call-site one line shorter.

**Alternatives considered:** infinite-scroll — rejected, adds
render-virtualization complexity for an occasionally-visited screen.

### Q5 — Audit screen filter persistence

**The question:** R9.4 — filter state persists **on-disk** across
cockpit restarts, or **in-session only** (in-memory)?

**Recommended (analyst):** **in-session only**. Matches Phase 2 Q8
("the cockpit is an instrument, not a browser") + the
[`spec/ui-design-principles.md` § Persistence](../ui-design-principles.md)
session-scoped rule. No `~/.cockpit-state.json`; no serialization on
`Drop`.

**Alternatives considered:** persist filters under
`~/.config/cockpit/` — rejected, adds an FS-write path the cockpit
deliberately doesn't have.

### Q6 — Strategies-detail equity-since-deploy sparkline cost

**The question:** R6 — sparkline lands in Phase 3 if **cheap**
(cached from `Cockpit::pnl` historical buffer) or **defers to
Phase 4** if **expensive** (new
`pnl_by_strategy_history(strategy_id, since)` audit query).

**Recommended (analyst):** **cheap path if the existing
`Cockpit::pnl` buffer carries per-strategy data; defer to Phase 4
otherwise**. `pnl_by_strategy(ledger, strategy_id, since, until)`
at [`crates/audit/src/query.rs:769`](../../crates/audit/src/query.rs)
already computes per-strategy P&L; architect's design pass measures
whether wiring a 60-bar sparkline buffer costs <50 LOC (cheap) or
requires a new bus subscription (defer).

**Alternatives considered:** ship behind a `phase4-preview` cfg —
rejected, dead-code gating for a one-screen feature.

### Q7 — Audit-query method extension shape

**The question:** R12 — **extend** Phase 2's `recent_fills_filtered`
with `Option<&str> kind` (one method, broader predicate), or **add
a sibling** `recent_journal_filtered` (two methods, narrower
per-method predicate)?

**Recommended (analyst):** **add a sibling**. The fills predicate
scans `description LIKE 'buy %'` / `'sell %'` rows; non-fill rows
scan `strategy_events` + reconciliation tables — different table
set, different join shape. Cramming both into one method either
ships two SQL paths inside one function (code-smell) or unifies the
queries onto a single rows view (premature). Two siblings is honest
about the data shape diverging.

**Alternatives considered:** deprecate `recent_fills_filtered` —
rejected, breaks Phase 2's shipped chart-marker call-site.

### Q8 — Sidebar entry insertion order

**The question:** R1.1 — insert new entries **between Debug and
Charts** (master-roadmap order: Home → Debug → Strategies → Risk →
Audit → Charts), or **grouped at the top with Home**?

**Recommended (analyst):** **master-roadmap order**. Operator scan
priority: trading data first (Home), ops chrome second (Debug),
detail screens third, cross-check chart last. Reordering is
gold-plating; the operator already adapted in Phase 2.

**Alternatives considered:** group Home + detail at top, ops at
bottom — slightly more "logical" but pointless churn.

### Q9 — Risk screen kill-threshold proximity gauge visual style

**The question:** R7.2 — **horizontal bar** (Phase 1 latency-band
precedent), **radial / dial** ("fuel gauge"), or **numeric only**?

**Recommended (analyst):** **horizontal bar**. Visual consistency
with the rest of the Risk screen (per-venue exposure + daily loss
are both horizontal bars) and Phase 1's `theme::color_for_latency_ms`
colour ramp. Radial dials add a new chart primitive Phase 3 doesn't
otherwise need; numeric-only loses the at-a-glance signal.

**Alternatives considered:** colour-only badge — rejected, loses
magnitude.

### Q10 — Per-strategy params + risk caps view scope

**The question:** R4.4 + R7.5 — **read-only** display, or
**editable** with config-write?

**Recommended (analyst):** **read-only**. Matches
[`spec/product.md` § Cockpit information architecture → What stays
out of the cockpit IA → Configuration editor](../product.md):
"`config/agent.toml` is hand-edited; the cockpit never writes
config. (Risk and execution-mode toggles in Phase 5 are exceptions
ratified there.)" Phase 3 holds the line; Phase 5 HumanControl
ratifies the operator-write exceptions.

**Alternatives considered:** edit-but-not-persist — rejected,
operator-locked "no order entry, no config editor" non-goal.

### Q11 — Snapshot ripple + cross-link Message variant

**Two interlocking questions; resolve together.**

**Q11a — snapshot ripple budget.** ~3 per detail screen × 3
screens = 9 + ~3–4 sidebar variants ≈ 12 net-new + 1 refreshed
Phase 2 sidebar baseline = ~13. Architect ratifies the count.

**Q11b — Home → Strategies cross-link.** Compound dispatch
(`SelectStrategy(id)` + `SwitchScreen(Strategies)`) or a new
`Message::OpenStrategy(StrategyId)` variant with combined handler?

**Recommended (analyst, both):** **~9–12 net-new baselines, single
`cargo insta accept` pass** (Phase 1 Q2 / Phase 2 V11 precedent).
For Q11b, **prefer compound dispatch** — Phase 2 R8.2 established
the pattern; reusing it keeps the Message enum smaller.

**Alternatives considered:** dedicated `OpenStrategy` variant —
rejected, costs an enum variant for marginal expressivity.

## Backlog updates

Effective on this brief's promotion (2026-05-05):

### Active

- **`lumen-phase-3-detail-screens`** — this brief, expanded
  from stub status to active. Status: `active`. Owner: analyst.
  Pipeline next stage: **architect**.

### Queue (unchanged from master roadmap)

- **`lumen-phase-4-backtest-panel`** — promotes on Phase 3
  ship. Status: queued.
- **`lumen-phase-5-humancontrol-agentfeed`** — promotes on
  Phase 4 ship. Status: queued.
- **`lumen-phase-6-assistant-slot`** — reserved, linked to v2
  LLM. No analyst spawn until v2 LLM is approved.

### Recent (shipped)

- **`lumen-phase-2-shell-ia-charts`** — shipped 2026-05-05
  (`T_FINAL_LUMEN_PHASE_2` first-pass PASS).
- **`lumen-phase-1-foundation`** — shipped 2026-05-04
  (tester third-pass PASS).

### Stub supersede note

The 2026-05-04 stub of this brief (109 lines, queued status,
high-level scope only) is **superseded by this expansion**. The
Why section is preserved verbatim and extended; Scope is
replaced by the R-item-pointing summary; Open questions are
replaced by the architect Q-items below; Acceptance criteria
are extended to trace each bullet to its R-cluster. Master
roadmap reference unchanged: see
[`lumen-design-adoption.md` Phase 3 section](lumen-design-adoption.md).

## Design

_Architect-owned. Resolves Q1–Q11 — every recommendation lands as
**ratified** unless flagged "Architect override". The analyst sections
above are immutable; this section is the design contract the developer
reads alongside the task list at
[`spec/lumen-design-adoption/phase-3-detail-screens/tasks.md`](../tasks/lumen-phase-3-detail-screens.md)._

### Q-item resolutions

All 11 architect Q-items resolved. **11/11 ratified, zero deviations
from analyst recommendation.** Each row cites the R-item(s) the
resolution lands. Phase 3 inherits more Phase-2 primitives than Phase
2 inherited from Phase 1 (sidebar widget, screen routing, T1507
active-row, T1609 active-chip, `PanelState`, `frame::panel`,
`widgets::num`, `journal_transaction_modal`, `recent_fills_filtered`),
so each Q resolution is short and the design body below is
correspondingly shorter than Phase 2's.

| Q   | Question                                                | Resolution                                                                                                                                                                                                                                                                                                                                                                                            | Ratifies        |
|-----|---------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------|
| Q1  | `journal_transactions.venue` migration scope            | **Ship in Phase 3.** Migration filename: `008_journal_transactions_venue.sql` (next-numbered after the existing `007_strategy_events_venue.sql`). `ALTER TABLE … ADD COLUMN venue TEXT NOT NULL DEFAULT 'Binance'`; the `DEFAULT 'Binance'` clause backfills every existing row in one transaction (every shipped fill on disk is Binance per Phase 2 venue-handling note). The writer at `crates/audit/src/journal.rs::post_fill` gains a `venue: Venue` parameter (the existing `Fill` struct does not carry venue, so the writer takes it explicitly from the call-site as the runtime stamps it post-execution); the new `INSERT INTO journal_transactions` binds `venue.to_string()` post-migration. Phase 2's `recent_fills_filtered` venue gate (`if venue != Venue::Binance { return Ok(Vec::new()) }`) is removed and replaced with a `WHERE venue = ?` SQL predicate. Splitting as Phase 3.5 was rejected — the migration is additive, ~30 LOC, one consumer; gate friction without isolating actual risk. | R13.1–R13.6     |
| Q2  | Strategies-detail signal-history source                 | **Filter `Cockpit::strategies_recent_events`.** Phase 1 R5 already populates the buffer from the strategy-event subscription; Phase 3 filters by `selected_strategy` at view time. Zero new audit writers; Phase 5 HumanControl introduces the first new operator-write paths. A new `signal_emitted` audit writer would violate the master-roadmap "no new audit writers in Phase 3" stance and add anchor-risk surface for a screen that only needs to render existing telemetry.                                                                                                                                                                                                                                                                                                                                                       | R5.4            |
| Q3  | Risk screen exposure source                             | **New tokio channel mirroring Phase 1 `MarketHealth`.** The cockpit-thread isolation rule (no UI-thread reads from agent-runtime mutexes) is load-bearing; the Phase 1 `MarketHealth` bus channel is the canonical example. Phase 3 adds a `RiskTelemetry` bus channel published by `crates/risk/src/portfolio.rs` at 1 Hz; the cockpit's `Subscription::batch` adds a sibling that maps incoming `RiskTelemetry` to `Message::RiskStateRefreshed(RiskState)`. ~40 LOC publisher + ~20 LOC subscriber. Polled view-time reads were rejected — couples render-rate to agent-state-cache locking and breaks the "screens are pure render dispatches" invariant.                                                                                                                                                                              | R8.1–R8.5       |
| Q4  | Audit screen pagination size                            | **Fixed 250 rows / page.** Cockpit IA forbids surfaces without operator-stated need. Fixed 250 keeps the SQL `LIMIT` constant, snapshot baselines deterministic, and the call-site one line shorter. Operator-configurable chip selectors and infinite-scroll were both rejected.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | R9.3, R10.1     |
| Q5  | Audit screen filter persistence                         | **In-session only.** Matches Phase 2 Q8 ("the cockpit is an instrument, not a browser") + the principles-doc session-scoped persistence rule. No `~/.cockpit-state.json`; no `serde::Serialize` on `AuditFilter`; no `Drop` impl writing state. Filter clears on cockpit restart.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | R9.4, R10.1     |
| Q6  | Strategies-detail equity sparkline cost                 | **Defer to Phase 4.** Design-pass measurement: `Cockpit::pnl: PanelState<PnlSnapshot>` is a single snapshot, not a historical buffer — there is no per-strategy history field on `Cockpit` today. Wiring a 60-bar per-strategy buffer requires either (a) a new bus subscription on top of `pnl_by_strategy(ledger, strategy_id, since, until)` ticked at bar-close, or (b) a one-shot fetch on chip-select. Both cost more than the 50-LOC "cheap path" budget the analyst named. Phase 3 ships the **deferred placeholder** — `frame::muted_body("Equity sparkline lands with Phase 4")` top-right of the Strategies screen. Phase 4 owns the wiring (the Backtest panel already needs the same equity-history primitive). | R6.1–R6.4       |
| Q7  | Audit-query method shape                                | **Add a sibling `recent_journal_filtered`.** Fills predicate scans `journal_transactions` filtered by the description-prefix regex (`description LIKE 'buy %' OR description LIKE 'sell %'`); non-fill rows scan `strategy_events` (joined on `transaction_id`) and reconciliation rows. Different table set, different join shape; cramming both into one method either ships two SQL paths inside one function or unifies the queries onto a single rows view. Two siblings is honest about the data shape diverging. The migration added by Q1 means both methods can carry a `WHERE journal_transactions.venue = ?` predicate uniformly.                                                                                                                                                                                              | R12.1–R12.6     |
| Q8  | Sidebar entry insertion order                           | **Master-roadmap order: Home → Debug → Strategies → Risk → Audit → Charts.** Trading data first, ops chrome second, detail screens third, cross-check chart last. Reordering for "logic" is gold-plating; the operator already adapted to the Phase 2 order. The widget body at `crates/ui/src/widgets/sidebar_nav.rs:48` is unchanged — Phase 2 R1.6 parameterised `entries: &[Screen]`; Phase 3 adds the 6-entry constant.                                                                                                                                                                                                                                                                                                                                                                                                              | R1.1            |
| Q9  | Risk kill-threshold gauge visual style                  | **Horizontal bar.** Visual consistency with the per-venue exposure section + daily loss section above it (both horizontal bars) and Phase 1's `theme::color_for_latency_ms` colour ramp. Radial dials add a new chart primitive Phase 3 doesn't otherwise need; numeric-only loses the at-a-glance signal. Bar = `Container` with left-aligned filled portion (`Length::Fill` × ratio) + right-aligned `widgets::num` rendering "X / Y (Z %)".                                                                                                                                                                                                                                                                                                                                                                                            | R7.2–R7.3       |
| Q10 | Per-strategy params + risk caps view scope              | **Read-only.** Matches `spec/product.md` § Cockpit IA → "`config/agent.toml` is hand-edited; the cockpit never writes config. (Risk and execution-mode toggles in Phase 5 are exceptions ratified there.)" Phase 3 holds the line; Phase 5 HumanControl ratifies operator-write exceptions. No edit / pause / deploy / "raise the limit" buttons on Strategies or Risk screens.                                                                                                                                                                                                                                                                                                                                                                                                                                                          | R4.4, R7.5      |
| Q11 | Snapshot ripple budget + cross-link Message variant     | **Q11a — ripple ≈ 13** (~3 per detail screen × 3 = 9 net-new + 3 sidebar variants for `active_strategies / active_risk / active_audit` + 1 refreshed Phase 2 `sidebar_nav__three_entries` → `sidebar_nav__six_entries`). One `cargo insta accept` pass at end of phase per Phase 1 Q2 / Phase 2 V11 precedent. **Q11b — compound dispatch.** Phase 2 R8.2 established the pattern (chip-select uses `SelectSymbol` plus binary-side `Task::perform`); reusing it keeps the `Message` enum smaller. The Home → Strategies-summary row click emits `Message::SelectStrategy(id)` followed by `Message::SwitchScreen(Screen::Strategies)` via `Task::done(...)` chained from the binary's wiring. No new `OpenStrategy` variant. | R5.2, R5.5      |

**No principled overrides.** Analyst recommendations are
operator-aligned and consistent with the master roadmap's
operator-locked Q11–Q14, the cross-feature invariant table, and the
zero-anchor-risk discipline; the architect ratifies all eleven.

### Cockpit state diff

The state diff `crates/ui/src/state.rs` receives in Phase 3:

```rust
// ── crates/ui/src/state.rs — Phase 3 additions ─────────────────────────────

/// Phase 3 — Strategies-detail screen state. Selected strategy persists
/// across screen switches (mirrors Phase 2 `selected_symbol` precedent —
/// Q8). `None` until the operator first enters Strategies or clicks a
/// row on the Home → Strategies-summary panel; cleared only on cockpit
/// restart (Q5 — session-scoped per ui-design-principles).
pub struct Cockpit {
    // … all existing Phase 1 + Phase 2 fields …

    // ── Phase 3 — Detail screens ────────────────────────────────────────
    /// Currently-selected strategy on the Strategies-detail screen.
    /// Set by `Message::SelectStrategy` (chip click on the Strategies
    /// screen, or row click on the Home → Strategies-summary panel
    /// followed by `SwitchScreen(Screen::Strategies)` — Q11b compound
    /// dispatch). Reset to `None` only on cockpit restart.
    pub selected_strategy: Option<StrategyId>,

    /// Read-only mirror of the agent runtime's strategies config,
    /// populated once at boot in both bins (live: from
    /// `agent::config::Config.strategies`; fixtures: from
    /// `StrategiesConfig::default()` or a `fake_strategies_config()`
    /// helper). Static for the session — same precedent as
    /// `Cockpit::universe` (Phase 2 Q3) and `Cockpit::account_label`
    /// (Phase 1 R13.4). `None` if the binary boots before config loads;
    /// the screen renders the empty-state until populated.
    pub strategies_config: Option<StrategiesConfig>,

    /// Live-mirrored risk state. Populated by the new bus subscription
    /// on `RiskTelemetry` events (Q3 — channel pattern; mirrors Phase 1
    /// `MarketHealth`). `Loading` on cold-start until the first
    /// `RiskStateRefreshed` arm fires.
    pub risk_state: PanelState<RiskState>,

    /// Audit-screen sub-state. Filter, page cursor, loaded row set,
    /// and total count all live here so the screen body is a pure
    /// dispatch over `&Cockpit`.
    pub audit_screen_state: AuditScreenState,
}

/// Phase 3 R8.1 — risk-screen mirror. Shipped by the agent runtime's
/// `RiskTelemetry` snapshot (Q3 ratification — channel pattern, sibling
/// of Phase 1 `MarketHealth`). All numeric fields are `Decimal`; no `f64`.
pub struct RiskState {
    pub per_symbol_exposure: HashMap<(Venue, Symbol), Decimal>,
    pub per_symbol_caps: HashMap<(Venue, Symbol), Decimal>,
    pub daily_loss_used_pct: Decimal,
    pub daily_loss_cap_pct: Decimal,
    pub heartbeat_age_ms: u64,
    pub heartbeat_timeout_ms: u64,
}

pub struct AuditScreenState {
    pub filter: AuditFilter,
    pub page: u32,                          // 0-indexed
    pub rows: PanelState<Vec<JournalRow>>,
    pub total_count: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditFilter {
    pub venues: Vec<Venue>,                 // empty = all
    pub symbol: Option<Symbol>,             // None = all
    pub kind: AuditKindFilter,              // All / Fill / StrategyEvent / Reconciliation
    pub time_range: AuditTimeRange,         // Last1H / Last24H / Last7D
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AuditKindFilter { #[default] All, Fill, StrategyEvent, Reconciliation }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditTimeRange { Last1H, Last24H, #[default] Last7D }
// (default = Last7D so first paint shows the widest window operators
// are likely to want; chip row makes narrowing one click.)

/// Newest-first row projection for the Audit screen table. Identifier
/// for the modal trigger lives in `tx_id` (`SmolStr` per Phase 1
/// `TapeRowClicked` precedent); kind discriminates the icon-less label
/// column rendered in the table body.
#[derive(Debug, Clone)]
pub struct JournalRow {
    pub tx_id: SmolStr,
    pub ts: Timestamp,
    pub venue: Venue,
    pub symbol: Option<Symbol>,             // None for non-fill kinds
    pub kind: AuditKindLabel,               // Fill / StrategyEvent / Reconciliation
    pub description: SmolStr,
    pub strategy_id: Option<StrategyId>,
}

pub enum Message {
    // … all existing Phase 1 + Phase 2 variants …

    // ── Phase 3 — Detail screens ────────────────────────────────────────
    /// Strategies-detail chip click OR Home → Strategies-summary row
    /// click. Pure assignment; `selected_strategy = Some(id)`. The
    /// Home-row variant follows up with `Message::SwitchScreen(
    /// Screen::Strategies)` via the binary's `Task::done` chain
    /// (Q11b compound dispatch — no new `OpenStrategy` variant).
    SelectStrategy(StrategyId),

    /// Risk telemetry refresh from the new agent-runtime channel
    /// (Q3 ratification). Pure assignment; `risk_state = Ready(state)`.
    /// `Subscription::batch` recipe in `crates/ui/src/live.rs` maps
    /// incoming `RiskTelemetry` bus events to this variant.
    RiskStateRefreshed(RiskState),

    /// Audit filter chip / input changed. Pure: resets `page` to 0,
    /// flips `rows` to `Loading`. The binary's `Task::perform` shim
    /// dispatches the `recent_journal_filtered` re-fetch.
    AuditFilterChanged(AuditFilter),

    /// Audit pagination Prev / Next. Pure: increments / decrements
    /// `page`, flips `rows` to `Loading`. Binary dispatches re-fetch.
    AuditPageChanged(u32),

    /// Async result of `recent_journal_filtered`. `Ok((rows, total))`
    /// → `rows = Ready(rows); total_count = Some(total)`; `Err(msg)`
    /// → `rows = Error(msg); total_count = None`.
    AuditRowsLoaded(Result<(Vec<JournalRow>, u64), SmolStr>),
}
```

**`Default` impl extension.** `selected_strategy: None`,
`strategies_config: None`, `risk_state: PanelState::Loading`,
`audit_screen_state: AuditScreenState::default()` (`AuditFilter`
default + `page = 0` + `rows = Loading` + `total_count = None`).

**Message-handler diff.** Five new arms, all pure assignments — Phase
2's "every screen is a pure render dispatch" rule carries forward;
async work (Risk channel subscription, Audit `Task::perform`) lives
in the binary, not in `update`.

```rust
Message::SelectStrategy(id) => { model.selected_strategy = Some(id); }
Message::RiskStateRefreshed(s) => { model.risk_state = PanelState::Ready(s); }
Message::AuditFilterChanged(f) => {
    model.audit_screen_state.filter = f;
    model.audit_screen_state.page = 0;
    model.audit_screen_state.rows = PanelState::Loading;
}
Message::AuditPageChanged(p) => {
    model.audit_screen_state.page = p;
    model.audit_screen_state.rows = PanelState::Loading;
}
Message::AuditRowsLoaded(Ok((rows, total))) => {
    model.audit_screen_state.rows = PanelState::Ready(rows);
    model.audit_screen_state.total_count = Some(total);
}
Message::AuditRowsLoaded(Err(msg)) => {
    model.audit_screen_state.rows = PanelState::Error(msg);
    model.audit_screen_state.total_count = None;
}
```

`Message::SwitchScreen` is **unchanged** — Phase 2's
`switch_screen_is_pure` invariant holds (every other field
byte-identical after a screen switch). This is the load-bearing
discipline that lets Q11b ship as compound dispatch instead of a
combined `OpenStrategy` arm.

### Sidebar nav extension

Phase 2 R1.6 parameterised the widget API:

```rust
pub fn view(current_screen: Screen, entries: &[Screen], mode: ThemeMode)
    -> Element<'_>;
```

Phase 3 changes are constant-only. The widget body at
[`crates/ui/src/widgets/sidebar_nav.rs:48`](../../crates/ui/src/widgets/sidebar_nav.rs)
is **untouched**. The `label_for(Screen)` match arm at the same file
already enumerates all six variants (Phase 2 declare-now). The six
`SIDEBAR_NAV_*` constants already ship in `crates/ui/src/strings.rs`.

In `crates/ui/src/theme.rs` (`theme::layout` module):

```rust
/// Phase 3 sidebar entry list — six entries in master-roadmap scan
/// order (Q8). Inserts `Strategies / Risk / Audit` between `Debug`
/// and `Charts` per the analyst's ratified insertion point.
pub const SIDEBAR_ENTRIES_PHASE_3: &[Screen] = &[
    Screen::Home, Screen::Debug,
    Screen::Strategies, Screen::Risk, Screen::Audit,
    Screen::Charts,
];
```

The `SIDEBAR_ENTRIES_PHASE_2` constant is **removed on Phase 3 ship**
(no forward-compat need — both bins swap atomically). Both bins'
`shell::view` call-sites at `crates/ui/src/shell.rs:35` (and
wherever else they reference the constant) swap from `_PHASE_2` to
`_PHASE_3` in one diff.

**No group separator** between the ops chrome (Home, Debug) and the
detail screens (Strategies, Risk, Audit, Charts). The vertical
T1507 rule on the active row + the FG_2/FG_1 emphasis already give
the operator a sufficient visual cue; introducing a separator
widget for one row of horizontal whitespace is gold-plating. The
sidebar at six entries × ~24 px row + `space::M` padding ≈ 200 px
column, comfortably within the fixed `SIDEBAR_WIDTH_PX = 180.0`
column (R1.5 — the widget's intrinsic content can grow vertically;
`Length::Fill` covers it).

### Strategies-detail screen contract

**File:** `crates/ui/src/screens/strategies.rs` (new). Sibling of
the existing `home / debug / charts` modules — the `screens/mod.rs`
index gains a `pub mod strategies;` line.

```rust
pub fn view<'a>(model: &'a Cockpit, mode: ThemeMode) -> Element<'a, Message>;
```

**Layout, top-to-bottom (R4.2):**

1. **Strategy chip row.** One chip per loaded strategy from
   `Cockpit::strategies` (Phase 1 R5). Each chip is a `button`
   carrying `Message::SelectStrategy(row.id.clone())` on press,
   wrapped in `frame::active_chip(content,
   selected_strategy.as_ref() == Some(&row.id), mode)` — the T1609
   horizontal bottom-edge variant from Phase 2 (Q5 ratification).
   Top-right of the same row: the **deferred-sparkline placeholder**
   per Q6 — `frame::muted_body(strings::STRATEGIES_SPARKLINE_DEFERRED)`
   reading "Equity sparkline lands with Phase 4". Net-new constant in
   `ui::strings` (additive — operator-locked Constraint 2 unchanged).
2. **Params block.** Read-only key-value rows from the active
   strategy's `[[strategy]]` block in `cockpit.strategies_config`
   (Q10 — read-only). Rendered via `frame::col_header` +
   `widgets::num` for numeric values (mirrors Phase 1 `pnl::view`
   shape). Each strategy's TOML block surfaces as
   `params: HashMap<SmolStr, ParamValue>` already on `StrategyConfig`
   in `crates/agent/src/config.rs` — no new struct.
3. **Recent signal events table.** Newest-first, capped at 50 rows
   (R4.2). Source: `Cockpit::strategies_recent_events` filtered at
   view time by `selected_strategy` (Q2 ratification — no new audit
   writer). Columns: timestamp, kind, symbol, rationale.

**Cross-link plumbing (R5.2 / Q11b — compound dispatch).** The
existing Home → Strategies-summary panel
(`crates/ui/src/widgets/strategies.rs`) already emits a per-row
click event in scope; Phase 3 wires it as follows in **the binary**
(not in `update`):

```rust
// crates/ui/src/bin/cockpit.rs (and cockpit_live.rs)
// On the Home-screen Strategies-summary row click, the existing
// row handler emits Message::SelectStrategy(id); the binary chains
// the screen switch via Task::done per Phase 2 R8.2 precedent:
Message::SelectStrategy(id) => {
    // 1. Apply pure update (handled by `update`).
    // 2. If the click came from Home (i.e. current_screen != Strategies),
    //    chain the screen switch:
    if cockpit.current_screen != Screen::Strategies {
        return Task::done(Message::SwitchScreen(Screen::Strategies));
    }
    Task::none()
}
```

This is the same shape as Phase 2's `SelectSymbol` chained marker
re-fetch — no new `Message` variant for marginal expressivity.

**Empty state.** No strategy selected (`selected_strategy.is_none()`)
→ chip row + centred `frame::muted_body(strings::STRATEGIES_SELECT_PROMPT)`
("Select a strategy"). Strategies-config still loading (`strategies_config.is_none()`)
→ `frame::muted_body(strings::STRATEGIES_LOADING)`.

**Sparkline scope (Q6 ratification).** Deferred to Phase 4. Phase 3
ships **no sparkline plumbing** — no `pnl_by_strategy_history`
audit query, no per-strategy bar buffer, no canvas widget on the
Strategies screen. The placeholder is a single `muted_body` row;
the snapshot baseline locks the deferral
(`strategies_screen__sparkline_deferred.snap`) so Phase 4 has a
clear "this is the seam" target.

### Risk / Limits screen contract

**File:** `crates/ui/src/screens/risk.rs` (new).

```rust
pub fn view<'a>(model: &'a Cockpit, mode: ThemeMode) -> Element<'a, Message>;
```

**Layout (single column, Tier 1 chrome — R7.2):**

1. **Per-venue exposure section.** One row per `(Venue, Symbol)` in
   `risk_state.per_symbol_exposure`. Each row = horizontal bar
   (`Container` with left-aligned filled portion) + right-aligned
   `widgets::num` rendering "X / Y USDT (Z %)". Colour ramp from
   the Phase 1 latency-band precedent: `ACCENT` < 70 %, `WARN_500`
   ≥ 70 %, `DOWN_500` ≥ 90 % (matches `theme::color_for_latency_ms`
   semantics). Implementation reuses the Phase 1 latency-bar widget
   primitive shape (filled `Container` + number suffix); a new
   `frame::threshold_bar(used: Decimal, cap: Decimal, mode: ThemeMode)`
   helper lives in `crates/ui/src/widgets/frame.rs` (additive,
   sibling of Phase 1 `active_row` and Phase 2 `active_chip`).
2. **Daily loss section.** Single bar: `daily_loss_used_pct /
   daily_loss_cap_pct` with the same ramp and the same
   `frame::threshold_bar` helper.
3. **Kill-threshold proximity gauge.** Single bar:
   `heartbeat_age_ms / heartbeat_timeout_ms` (Q9 — horizontal bar,
   not radial / numeric). Same ramp, same helper. The kill switch
   widget on the Debug screen continues to own the **trip** action;
   the Risk screen's gauge is the **early-warning** signal —
   read-only.

**Tier 1 chrome (R7.4).** `frame::panel(strings::RISK_PANEL_TITLE)`
wrapping the column. Outer padding `space::L`, section gap
`space::M`, row spacing `space::S`.

**Read-only (R7.5 / Q10).** No edit / "raise the limit" buttons.
The cockpit never writes risk config; Phase 5 HumanControl ratifies
the operator-write exceptions.

**Empty / loading (R7.6).** `risk_state == PanelState::Loading` →
`frame::muted_body(strings::RISK_LOADING)`. `Error(msg)` →
`frame::muted_body(format!("Risk feed unavailable: {msg}"))`.

**Channel wiring (Q3 ratification — concrete).** A new
`RiskTelemetry` event type lives in `crates/agent/src/runtime.rs`
(sibling of `MarketHealth`) carrying the same fields as `RiskState`.
The risk engine at `crates/risk/src/portfolio.rs` publishes a
snapshot at 1 Hz via a `bus.publish_risk_telemetry(snapshot)` call
(new method on `EventBus` — additive, sibling of the existing
`publish_market_health`). The cockpit's `Subscription::batch` in
`crates/ui/src/live.rs` adds a recipe that maps incoming
`RiskTelemetry` events to `Message::RiskStateRefreshed(RiskState)`.
~40 LOC publisher + ~20 LOC subscriber per the analyst sketch.
**Fixtures wiring (R8.3):** the fixtures bin pre-seeds
`cockpit.risk_state = PanelState::Ready(fake_risk_state())` at
boot; `fake_risk_state()` lives in `crates/ui/src/fixtures.rs` and
returns deterministic numbers covering all three colour bands per
V5 (one venue/symbol < 70 %, one ≥ 80 %, one ≥ 95 %).

### Audit / Journal screen contract

**File:** `crates/ui/src/screens/audit.rs` (new).

```rust
pub fn view<'a>(model: &'a Cockpit, mode: ThemeMode) -> Element<'a, Message>;
```

**Layout, top-to-bottom (R9.2):**

1. **Filter row** — three sub-rows in a `Column`:
   - **Venue chips** (multi-select) — one chip per `Venue` enum
     variant. Active chips use the T1609 bottom-edge active-chip
     pattern. Empty venue set = "all venues".
   - **Symbol text input** (exact match, sunken styling per Phase 1
     T1506). `None` = all symbols.
   - **Kind chips** (single-select) — All / Fill / Strategy event /
     Reconciliation. Active chip uses T1609.
   - **Time-range chips** (single-select) — Last 1 h / Last 24 h /
     Last 7 d. Default `Last7D`.
2. **Pagination header.** `widgets::num`-rendered "Showing
   `start`–`end` of `total_count`" + Prev / Next buttons. Prev
   disabled at `page == 0`; Next disabled at `(page + 1) * 250 >=
   total_count`.
3. **Journal table.** Newest-first rows from
   `audit_screen_state.rows` (when `Ready`); columns: timestamp,
   venue, symbol, kind, description, strategy_id. Per-row click
   emits `Message::TapeRowClicked(row.tx_id.clone())` — **literally
   reuses the Phase 1 variant** per R11.4 / Q11 (analyst's
   recommendation; the action is row-click → modal-open regardless
   of which screen the row sits on).

**Filter UX choice (filter row chips vs text inputs).** Venue +
kind + time-range = **chips** (small, finite, well-known sets).
Symbol = **text input** (the universe is large + mode-dependent,
chips would crowd the row). Active chips use T1609; the text input
uses Phase 1 sunken styling.

**Pagination (R9.3 / Q4 — fixed 250).** Page state on
`audit_screen_state.page`; `LIMIT 250 OFFSET page * 250` baked into
the `recent_journal_filtered` SQL.

**Filter persistence (R9.4 / Q5 — in-session only).**
`audit_screen_state.filter` lives on `Cockpit`; cleared on cockpit
restart. No serialization, no `Drop` impl, no on-disk path.

**Per-row click reuses existing modal (R11).** No widget code
change. The shell-level modal wrap from Phase 2 R3.3 means the
modal overlays any screen, including Audit; `Message::TapeRowClicked`
flows the same way it does from the Home tape.

**Async fetch lives in the binary** (R10.3) per Phase 2 precedent.
On `Message::AuditFilterChanged` / `Message::AuditPageChanged`, the
binary issues `Task::perform(audit::query::recent_journal_filtered(
&ledger, …), Message::AuditRowsLoaded)`. Debounced via a single
`audit_in_flight: Option<AuditFilter>` field on the bin's app
state, **NOT on `Cockpit`** — keeps `Cockpit` pure-data per Phase 2
discipline.

**Empty / loading states (R9.6).** Loading → Phase 1
`PanelState::Loading` skeleton. Empty after filter →
`frame::muted_body(strings::AUDIT_FILTER_NO_MATCH)` ("No journal
rows match these filters"). Error →
`frame::muted_body(format!("Journal query failed: {msg}"))`.

### Audit query additions

**Exact signature** (Q7 ratification — sibling, not extension):

```rust
/// Phase 3 addition (R12 / Q7). Return the page of journal rows
/// matching the filter, newest-first. Read-only over committed
/// audit data; additive sibling of [`recent_fills_filtered`].
///
/// `venues.is_empty()` ↔ all venues; `symbol.is_none()` ↔ all symbols;
/// `kind == AuditKindFilter::All` ↔ all kinds. The half-open window
/// `[since, until)` matches the `recent_fills_filtered` shape.
/// Returns `(rows, total_count)` so the screen header can render
/// "Showing N–M of T" without a separate `COUNT(*)` round-trip.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn recent_journal_filtered(
    ledger: &Ledger,
    venues: &[Venue],
    symbol: Option<&Symbol>,
    kind: AuditKindFilter,
    since: Timestamp,
    until: Timestamp,
    page_offset: u32,
    page_size: u32,
) -> Result<(Vec<JournalRow>, u64), LedgerError>;
```

**Implementation sketch.** SQL projection over `journal_transactions`
LEFT JOIN `strategy_events` ON `transaction_id` (so non-fill rows
surface). After the `008_journal_transactions_venue.sql` migration
(see below), both tables carry `venue` columns; the WHERE predicate
unifies them as `journal_transactions.venue IN (?, …) AND ts >= ?
AND ts < ? AND <kind-discriminator>`. The kind discriminator:

- `All` → no extra predicate.
- `Fill` → `description LIKE 'buy %' OR description LIKE 'sell %'`
  (same prefix-scan as `recent_fills_filtered`).
- `StrategyEvent` → `EXISTS (SELECT 1 FROM strategy_events se WHERE
  se.transaction_id = journal_transactions.id)`.
- `Reconciliation` → reconciliation rows discriminated by their
  description prefix (Phase 1+ convention: `reconcile %`).

Symbol filtering reuses Phase 2's
`extract_symbol_from_description` helper for fill rows; non-fill
rows fall back to `symbol IS NULL` matches. `ORDER BY ts DESC,
rowid DESC` per Phase 2 R12.5. `LIMIT ? OFFSET ?` for pagination.
The `total_count` returns from a sibling `COUNT(*)` query under
the same WHERE predicate executed in the same async block (one
extra round-trip, well under any user-perceptible threshold for a
250-row page).

**Determinism / money math.** No `f64`. All description-amount
parsing reuses the existing `Price` / `Quantity` newtypes per
Phase 2 R12. Empty result returns `Ok((vec![], 0))`; never `Err`
for "no rows".

**Test scope.** Mandatory:

- **Unit tests** in `crates/audit/src/query.rs::tests` covering each
  filter variant + an empty-window `Ok((vec![], 0))` result + a
  multi-venue / multi-kind seed.
- **Integration test** at `crates/audit/tests/recent_journal_filtered.rs`
  promoted from Phase 2's deferral (Phase 2 Q10 explicitly named
  Phase 3 as the natural promotion point). Seeds 250 + 5 fills
  across two venues × three kinds, asserts the page-2 cursor returns
  the expected tail and the `total_count` is `255`.

### `journal_transactions.venue` migration

**Filename:** `crates/audit/migrations/008_journal_transactions_venue.sql`
(next-numbered after the existing `007_strategy_events_venue.sql`).

**SQL:**

```sql
-- Migration 008 — add venue column to journal_transactions (Phase 3 R13).
-- Additive: NEW column with NOT NULL DEFAULT 'Binance'. The default
-- backfills every existing row (every shipped fill on disk today is
-- Binance per Phase 2 venue-handling note). Post-migration, the
-- writer at crates/audit/src/journal.rs::post_fill stamps the actual
-- venue passed by the runtime caller.
ALTER TABLE journal_transactions
  ADD COLUMN venue TEXT NOT NULL DEFAULT 'Binance';
```

**Backfill semantics.** SQLite's `ADD COLUMN … DEFAULT 'Binance'`
clause writes the literal `'Binance'` for every existing row in one
statement. **No `UPDATE` pass needed**; the `DEFAULT` is the
backfill. The body bytes of every existing journal-transactions row
are unchanged — the migration adds a new column to the schema, not
a row content rewrite.

**Writer-side change (R13.3).** `crates/audit/src/journal.rs::post_fill`
gains a `venue: Venue` parameter:

```rust
pub async fn post_fill(
    ledger: &Ledger,
    fill: &Fill,
    venue: Venue,                            // NEW Phase 3 R13.3
    strategy_id: Option<&str>,
) -> Result<SmolStr, LedgerError>;
```

The `Fill` struct in `crates/core/src/fill.rs` does **not** carry
`venue` — it carries `venue_ts` and `local_ts` but not the venue
identity. Phase 3 takes the venue explicitly from the caller (the
runtime is the source of truth: it knows which venue dispatched
the order that produced the fill). Call-sites:

- `crates/exec/` (paper engine + any future live executor) — pass
  the venue the fill came from (currently always
  `Venue::Binance` per v1.5b plumbing-only state).
- `crates/agent/src/runtime.rs` post-fill hook — same.
- Tests / fixtures that call `post_fill` directly — pass
  `Venue::Binance` explicitly.

The `INSERT INTO journal_transactions` SQL grows the `venue` column:

```rust
sqlx::query(
    "INSERT INTO journal_transactions (id, ts, description, strategy_id, venue) \
     VALUES (?, ?, ?, ?, ?)",
)
.bind(&txn_id)
.bind(&ts)
.bind(&description)
.bind(strategy_id)
.bind(venue.to_string())                    // NEW
```

The two other `INSERT INTO journal_transactions` call-sites in
`crates/audit/src/journal.rs` (the funding-obs and reconciliation
writers at lines 259 / 357 / 437) take the same treatment — each
gets a `venue: Venue` parameter and binds it on insert. For non-
fill rows the source of truth is again the runtime caller; for
v1.5b plumbing-only state every call-site passes `Venue::Binance`.

**Phase 2 query update (R13.4).** `recent_fills_filtered`'s venue
gate (`if venue != Venue::Binance { return Ok(Vec::new()) }`)
**removed** post-migration; replaced with `WHERE venue = ?` in
the SQL. The unit tests at `crates/audit/src/query.rs::tests`
gain a `Venue::Coinbase` case asserting it now returns matching
rows from a multi-venue fixture seed (was previously
`Ok(vec![])`).

**Anchor risk: zero.** Additive column with constant-string
backfill. No description / amount / strategy_id rewrites. The 11
backtest body-SHA-256 anchors are computed over committed report
bodies, not over the audit-DB row layout — the migration cannot
shift any anchored byte.

**Mandatory migration test** in `crates/audit/tests/migration_008.rs`
(or extending an existing migration-test harness if one exists per
the `007` pattern):

1. Open a pre-migration fixture DB (one that has rows with the
   pre-008 schema).
2. Run the migration.
3. Assert every existing row has `venue = 'Binance'`.
4. Insert a new fill via `post_fill(…, Venue::Coinbase, …)`,
   assert the row has `venue = 'Coinbase'`.
5. `recent_fills_filtered(&ledger, Coinbase, BTCUSDT, since, until)`
   returns the new row.

### TD-1 re-evaluation

**Verification on disk:** `crates/ui/Cargo.toml:52` reads
`iced = { version = "=0.14.0", default-features = false, features =
["tiny-skia", "thread-pool", "advanced", "canvas"] }`.

iced 0.15+ has not landed; neither `button::Status::Focused` nor
`text_input::Style.shadow` is available. **Phase 3 ships no
focus-ring upgrade.** Phase 1's bounded approximation (hover-state
ring on the kill / kill-confirm / modal-close buttons; ACCENT
border-shift on the kill confirm input) holds. Operator-impact
bound is unchanged: the kill-switch destructive flow is
typed-confirm gated; the focus halo is a secondary signal.

**Master-roadmap follow-up flagged.** The TD-1 row should be
appended with a 2026-05-05 line under "Promotion timing":

> "Phase 3 design pass (2026-05-05): iced version on disk verified
> still pinned `=0.14.0`; deferral restated. Next re-evaluation at
> Phase 4 (Backtest panel) analyst kickoff. If iced upstream stalls
> through Phase 4, re-evaluate at Phase 5 (HumanControl) — Phase
> 5's per-strategy pause / override controls add new operator-write
> paths whose focus-ring needs may sharpen the cost/benefit on the
> custom-widget path."

The architect does not edit the master roadmap directly; the
orchestrator routes this as a follow-up to the analyst on Phase 3
ship.

### Cross-feature invariants

Phase 3 column from the master roadmap, re-stated with the design
note:

| Feature                          | Phase 3 invariant note                                                     | How preserved                                                                                                                                                                                                                                                                                                                                                                                              |
|----------------------------------|----------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `operator-success-reports`       | Latency badge colour mapping unchanged; Debug screen body unchanged         | Phase 3 adds three sidebar entries; the Debug screen body is untouched. `theme::color_for_latency_ms` continues to drive `widgets::latency::view`. R14.7.                                                                                                                                                                                                                                                  |
| `live-cockpit-unified`           | Halted-banner trigger + shell-level wrap preserved                          | Phase 3 adds three new screens; the halted-banner is wrapped at the shell level (Phase 2 R3.3) and visible regardless of `current_screen`. Trigger logic in `crates/ui/src/state.rs` untouched. R14.4.                                                                                                                                                                                                     |
| `real-mtm-unrealized-pnl`        | PnL card unchanged; `color_for_delta` unchanged                             | Strategies-detail's deferred sparkline placeholder reads no PnL data (Q6 ratification). PnL card stays on Home; helper signature unchanged. R14.5.                                                                                                                                                                                                                                                         |
| `per-symbol-position-accounts`   | Positions widget unchanged                                                  | Risk-screen exposure section reads `Cockpit::positions` + the new `risk_state.per_symbol_exposure` mirror. Positions widget on Home unchanged; row contract preserved. R14.6.                                                                                                                                                                                                                              |
| `tape-row-audit-modal`           | Audit-screen rows open the same modal as Home tape rows                     | `Message::TapeRowClicked(tx_id)` literally reused (Q11/R11.4 — no new variant). Modal wrapped at shell level overlays any screen including Audit. Modal widget body unchanged. R14.1.                                                                                                                                                                                                                      |
| `journal-tx-metadata`            | Modal continues to render `description` + `strategy_id`                     | Audit-screen filter row exposes `strategy_id` as a column (R9.2); the column reads from the same metadata source the modal does. No modal widget change; no metadata-reader change. R14.2.                                                                                                                                                                                                                 |
| `v1.5b-multi-venue`              | Venue dimension surfaces on Audit filter chips + Risk exposure              | The `008_journal_transactions_venue.sql` migration is the additive completion of v1.5b's "fills carry venue" promise. Existing `MarketHealth` rows on Debug + chip row on Charts unchanged; v1.5b plumbing-only state preserved (every shipped fill on disk = Binance via `DEFAULT 'Binance'` backfill). R14.3.                                                                                              |

**Acceptance:** the tester's per-feature invariant table = 7 / 7
PASS.

### Anchor regression

**Zero anchor risk re-affirmed.** The design pass found no path
where Phase 3 touches committed report bodies:

- `recent_journal_filtered` is **read-only over already-committed
  audit rows** — additive sibling of Phase 2's `recent_fills_filtered`.
  No writer, no schema rewrite of existing rows, no
  description-format change.
- `recent_fills_filtered` Phase 3 update (drop the venue gate +
  add `WHERE venue = ?` predicate) is read-only — it changes the
  filter, not any returned row's body bytes.
- The `008_journal_transactions_venue.sql` migration is **additive**:
  `ADD COLUMN … DEFAULT 'Binance'`. SQLite's column-add is a schema
  change, not a row-body rewrite; the existing rows' description /
  amount / strategy_id bytes are untouched.
- The writer-side change (`post_fill` gains `venue: Venue`) stamps
  the new column on **future** rows only; existing rows keep their
  `'Binance'` backfill default forever.
- No strategy / exec / risk / cost / backtest / reports crate is
  touched. The Risk screen reads risk **state** (mirror fields on
  `Cockpit` fed by the new bus channel); it does not modify limits,
  emit kill events, or change risk-engine behaviour.

**Verify-anchors gate at the Phase 3 tester run** must report 11/11
PASS with byte-identical bodies. The R16.3 grep gate from Phase 1
(`grep -rni "lumen\|panel-raised\|panel-sunken\|cool-800"
spec/reports/`) remains zero — Phase 3 adds no new rendered prose
to any committed report.

### Implementation parallelism map

```
T1701 (state additions — foundation gate, sequential)
  ├─ T1702 (008 migration + writer wiring — parallel; audit + crates/exec)
  ├─ T1703 (sidebar 6-entry constant + bin call-site swap — parallel)
  ├─ T1707 (RiskTelemetry channel publisher + RiskState mirror — parallel; risk + agent crates)
  ├─ T1709 (Audit filter row UX + state plumbing — parallel; ui only)
  ├─ T1712 (recent_journal_filtered audit query — parallel; audit crate, no ui dep)
  └─ after T1701 + T1703:
        ├─ T1704 (Strategies-detail screen body — parallel)
        ├─ T1705 (Strategies cross-link compound dispatch — parallel after T1704)
        ├─ T1706 (Strategies sparkline placeholder — parallel after T1704)
        ├─ T1708 (Risk screen body + threshold_bar helper — parallel after T1707)
        ├─ T1710 (Audit pagination header + table body — parallel after T1709 + T1712)
        └─ T1711 (Audit row → modal trigger reuse — parallel after T1710)
                    │
                    ▼
              T1713 (snapshot refresh + ui-designer attestation sub-block — sequential after every visual lands)
                    │
                    ▼
              T1714 (cross-feature invariants verify — sequential)
                    │
                    ▼
              T1715 (anchor regression + R16.3 grep — sequential)
                    │
                    ▼
              T1716 (rust-validate + both bins launch — sequential)
                    │
                    ▼
              T_FINAL_LUMEN_PHASE_3 (tester gate — VERDICT → presenter on PASS)
```

T1701 is the foundation gate (state additions — `selected_strategy`,
`strategies_config`, `risk_state`, `audit_screen_state`, the four
new structs, the five new `Message` variants). T1702 (migration +
writer) and T1712 (query) live in `crates/audit` so they can run
in parallel from day 0. T1707 (RiskTelemetry channel) lives in
`crates/risk` + `crates/agent` so it can run in parallel from day 0.
The narrow point is T1713 (snapshot accept) — every visual must
land before the operator reviews the diff in one pass.

## Implementation

_developer fills this — task list at
[`spec/lumen-design-adoption/phase-3-detail-screens/tasks.md`](../tasks/lumen-phase-3-detail-screens.md)._

## Verification — links

_tester fills this — links to
`spec/lumen-design-adoption/phase-3-detail-screens/reports/test-<timestamp>-lumen-phase-3-detail-screens.md`._

## UI

_ui-designer fills this — links to refreshed snapshots and
the Phase 3 presentation under `spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md` (phase-3-detail-screens section)._

## Changelog

- 2026-05-05 (architect): appended `## Design`. Q1–Q11 ratified
  (008 migration ships in Phase 3, signal history filters existing
  `strategies_recent_events`, Risk via new `RiskTelemetry` tokio
  channel, audit pagination fixed at 250, audit filter persistence
  in-session only, equity sparkline deferred to Phase 4, audit
  query as sibling `recent_journal_filtered`, sidebar order
  Home → Debug → Strategies → Risk → Audit → Charts, kill
  threshold gauge as horizontal bar, read-only display, ~13
  snapshot ripple + compound-dispatch cross-link). 11/11 ratified;
  zero principled overrides. Cockpit state diff specified —
  `selected_strategy`, `strategies_config`, `risk_state`,
  `audit_screen_state` field additions; `RiskState`, `AuditScreenState`,
  `AuditFilter`, `JournalRow` struct additions; five new `Message`
  variants (`SelectStrategy`, `RiskStateRefreshed`, `AuditFilterChanged`,
  `AuditPageChanged`, `AuditRowsLoaded`). Sidebar nav extension
  (constant-only — widget body untouched, `SIDEBAR_NAV_*` already
  declared in Phase 2). Strategies-detail / Risk / Audit screen
  contracts. `recent_journal_filtered` exact signature with
  `(venues: &[Venue], symbol: Option<&Symbol>, kind: AuditKindFilter,
  since, until, page_offset, page_size) -> (Vec<JournalRow>, u64)`.
  `008_journal_transactions_venue.sql` migration shape with `ADD
  COLUMN venue TEXT NOT NULL DEFAULT 'Binance'` (no separate UPDATE
  backfill needed); writer at `crates/audit/src/journal.rs::post_fill`
  gains `venue: Venue` parameter; Phase 2 venue gate dropped from
  `recent_fills_filtered`. TD-1 deferral re-stated — verified
  `crates/ui/Cargo.toml:52` still pins `iced = "=0.14.0"`; next
  re-evaluation at Phase 4 analyst kickoff. Cross-feature
  invariants table re-stated (7 rows). Zero anchor risk re-
  affirmed (additive migration with constant-string backfill +
  read-only audit query addition + UI-only screens). Implementation
  parallelism map: T1701 foundation gate → fan-out across
  T1702–T1712 → narrow at T1713 snapshot accept → T_FINAL. Task
  list at
  [`spec/lumen-design-adoption/phase-3-detail-screens/tasks.md`](../tasks/lumen-phase-3-detail-screens.md)
  with 16 T17xx tasks + tester `T_FINAL_LUMEN_PHASE_3` gate.
  HANDOFF → developer ‖ ui-designer (developer takes T1701–T1716
  implementation; ui-designer takes the visual-diff attestation at
  T_FINAL after the developer's snapshot refresh pass).
- 2026-05-05 (analyst, Phase 3 kickoff expansion): expanded the
  2026-05-04 stub into the full analyst brief — 15 R-items
  grouped into 6 clusters (R1–R3 sidebar + screen routing
  extension; R4–R6 Strategies-detail; R7–R8 Risk / Limits;
  R9–R12 Audit / Journal + audit-query additions; R13
  `journal_transactions.venue` migration; R14–R15 invariants +
  anchors), 13 V-items mapping cleanly onto R-clusters, 10
  acceptance criteria each tracing to its R-cluster, and 11
  architect Q-items (Q1 migration scope inside-Phase-3 vs split,
  Q2 signal-history channel-vs-new-writer, Q3 risk source
  channel-vs-direct-read, Q4 pagination fixed-vs-configurable,
  Q5 filter persistence in-session-vs-on-disk, Q6 equity
  sparkline cheap-vs-expensive, Q7 audit query
  extend-vs-sibling-method, Q8 sidebar entry insertion order,
  Q9 kill-threshold gauge visual style, Q10 read-only-vs-
  editable, Q11 snapshot ripple budget + cross-link Message
  variant naming). Master-roadmap operator-locked decisions
  Q11–Q14 inherited as not-re-opened. TD-1 (keyboard focus
  ring) re-evaluation deferred — verified via Phase 2 design
  pass that iced still pins `=0.14.0` on disk; restated under
  Q9 alternative considerations rather than as a new R-item.
  Anchor risk reaffirmed as **zero** (read-only audit query
  additions + UI screens + additive migration with constant-
  string backfill). Snapshot ripple budget: ~9–12 net-new + 1
  refreshed Phase 2 sidebar baseline ≈ 13, accepted in one
  `cargo insta review` pass per Phase 1 Q2 / Phase 2 V11
  precedent. Brief status `queued` → `active`; owner unchanged
  (analyst → architect at HANDOFF). HANDOFF → architect.
- 2026-05-04 (analyst, master-roadmap revision): stub created
  at the 6-phase roadmap revision. Full brief expansion
  deferred to Phase 3 kickoff per master Q3 (per-phase analyst
  spawn).
