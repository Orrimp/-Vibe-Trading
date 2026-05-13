---
slug: architecture
status: in-progress
owner: architect
updated: 2026-05-04
---

<!-- updated 2026-05-04 (architect) — Q11 mid-phase deviation ratification:
     iced 0.14.2 `button::Status` has no `Focused` variant and `text_input::Style`
     has no `shadow` field. T1504's true keyboard-focus-ring acceptance is
     unachievable under the shipped framework. Option A ratified — Phase 1
     ships hover-state ring on buttons + ACCENT border-shift on focused
     inputs as a bounded best-effort approximation; T1504 tick stands as
     honest under iced 0.14.2 API gap. Phase-N follow-up filed in
     `features/lumen-design-adoption.md` (upgrade trigger: iced version
     bump exposing `button::Status::Focused` + `text_input::Style.shadow`,
     OR custom-widget approach — Phase-2-or-later). Same shape as Q3's
     `shadow_inset` outer-only API workaround. -->

<!-- updated 2026-05-04 (architect) — lumen-design-adoption Phase 1 foundation
     resolutions landing. Q1–Q9 + master Q10 ratified. Token system rewrites
     12 → ~50 tokens (Lumen palette + 5-tier surface system + whisper-shadow
     ladder + focus ring + 13-step spacing + 6-step radii + 7-step typography
     + motion ladder); flat `theme::color::*` SHOUTY_SNAKE_CASE per Q10. iced
     0.14 `Shadow` API confirmed via `iced_core-0.14.0/src/shadow.rs`. New
     `widgets::status_bar` consumes existing `bus.market_health()` (additive,
     no producer change). Single-file principles-doc supersede (~480 lines).
     Operator-locked: no brand, no `ui::strings` rewrite, no icons, sequential
     phases. Anchor risk zero by construction — UI-only feature, no
     `crates/strategy/audit/exec/backtest/reports/` touched. 11 / 11 anchors
     verify byte-identical. Cross-feature invariants for 7 prior shipped
     features preserved. Tasks `T1501–T1514 + T_FINAL_LUMEN_PHASE_1` filed
     at `tasks/lumen-phase-1-foundation.md`. New section "Lumen design
     adoption — Phase 1 foundation resolutions"; changelog entry at the
     bottom. -->

<!-- updated 2026-05-03 (architect) — v1.5b-multi-venue Design landing:
     largest queued backend feature. New closed enum
     `trading_core::Venue { Binance, Coinbase, Kraken }` lands as a
     load-bearing type with `#[serde(rename_all = "snake_case")]`;
     `Tick` and `Bar` gain required `venue: Venue` field (mechanical
     migration across ~30+ fixture sites — every existing literal
     defaults `Venue::Binance`). New `Timeframe::OneSecond` variant
     plus `crates/data/src/bar_aggregator.rs` for client-side 1s
     aggregation on `i64` epoch microseconds (deterministic). New
     `data::CoinbaseFeed` (Coinbase Advanced Trade WS) +
     `data::KrakenFeed` (Kraken WS v2) + `data::MockFeed` test
     harness. T612 finally lands — `BinanceFeed::subscribe_*_multi`
     using the combined-stream URL. New audit migration
     `007_strategy_events_venue.sql` (NULLABLE `venue TEXT` column
     on `strategy_events`); `audit::journal::feed_reconnect`
     signature gains required `venue: Venue` argument. New
     `EventBus::market_health: broadcast::Sender<MarketHealth>`
     channel (capacity 64) for the per-venue stale-data watchdog;
     `MarketHealth { Fresh, Stale, Recovered }` enum in
     `trading_core::venue`. `agent::runtime::run` spawns one
     `tokio::JoinSet` task per enabled venue (panic isolation —
     a Coinbase parser panic does NOT poison Binance / Kraken).
     New `[universe]` config section with `usdt_enabled` /
     `usdc_enabled` toggles + 10 USDC mirror pairs (operator
     opt-in). NO new external crate dep — all three feeds reuse
     `tokio_tungstenite` + `serde_json` + `reqwest`. NO `Cargo.toml`
     change. NO `unsafe`. **Anchor budget: 11 / 11 byte-identical**
     by construction (Q12: independent grep on
     `spec/*/reports/backtest-*.md` + `spec/operator-success-reports/reports/success-*.md`
     returned zero hits on `venue|coinbase|kraken`). Full delta
     in the new "v1.5b — multi-venue resolutions" subsection +
     changelog entry at the bottom. -->

<!-- updated 2026-05-03 (architect) — journal-transactions-metadata Design landing:
     adds new `trading_core::JournalTransactionMetadata` view struct
     (`transaction_id`, `ts`, `description`, `strategy_id`) in
     `crates/core/src/views.rs`; new `audit::query::journal_transaction_metadata`
     reader (single-row `SELECT id, ts, description, strategy_id FROM
     journal_transactions WHERE id = ?` returning `Option<JournalTransactionMetadata>`,
     `Ok(None)` for unknown tx_id); cockpit_live `Task::perform` closure at
     `crates/ui/src/bin/cockpit_live.rs:496-535` rewires from a partial
     `JournalTransactionView` construction to a sequential metadata→entries
     chain with Q6 error mapping (any-`Err` → `PanelState::Error`,
     metadata-`None` → "unknown transaction" error). Read-only additive feature
     off the anchored path — no migration, no new dep, no `Cargo.toml` change,
     no new theme/string/widget surface, no `unsafe`. 11/11 anchors stay
     byte-identical (R5). Full delta in the changelog entry at the bottom. -->

<!-- updated 2026-05-03 (architect) — tape-row-audit-modal Design landing:
     adds new `trading_core::JournalEntry` un-collapsed view struct
     (debit/credit pair, distinct from existing collapsed-amount
     `JournalEntryView`); new `audit::query::journal_entries_for_transaction`
     reader; new additive field `FillView::transaction_id: SmolStr` plus
     `Fill::transaction_id: Option<SmolStr>`; `audit::journal::post_fill`
     return type bumped from `Result<(), LedgerError>` to
     `Result<SmolStr, LedgerError>` (returns the generated
     `journal_transactions.id` for live-mode runtime to stamp on the
     bus-side `Fill` before fan-out); first cockpit modal — establishes
     `iced::widget::Stack` as the modal-overlay precedent for future
     click-through-to-audit drilldowns (positions, strategies); three
     additive theme tokens land (`bg_overlay`, `info`, `border_strong`)
     per the dark-mode hex in `ui-design-principles.md`. 11/11 anchors
     stay byte-identical (R12). Full delta in the tape-row-audit-modal
     architectural deltas subsection below + changelog entry at bottom. -->

<!-- updated 2026-05-01 (architect) — live-cockpit-unified Design landing:
     workspace-map adds `cockpit_live` bin under crates/ui; new public API
     `agent::runtime::run` (RunHandles, CancellationToken); deprecation of
     `cockpit --features live`; dep edge `ui → agent` now load-bearing for
     the unified bin. Full delta in the live-cockpit-unified architectural
     deltas subsection below + changelog entry at the bottom. -->

<!-- updated 2026-05-02 (architect) — real-mtm-unrealized-pnl Design landing:
     adds new `trading_core::OpenPosition` struct, new `audit::query::open_positions_at`
     reader, no new migration (Q3 no-index for v1+; conditional follow-up
     `006_open_positions_index.sql` only if V8 perf gate fails); 11 anchors
     stay byte-identical (Q4). R10 (post_fill BTC hardcode at
     `crates/audit/src/journal.rs:82,135`) explicitly DEFERRED to a follow-up
     brief `per-symbol-position-accounts.md`. -->

<!-- updated 2026-05-02 (ui-designer) — Frontend ↔ backend interfaces
     subsection added under `### Frontend — iced`. Documents the seven
     load-bearing surfaces between cockpit/viewer and the rest of the
     workspace: `Arc<EventBus>` (10 channels + backpressure policy),
     `audit::query` read-only API (15 read paths), `KillTripFn` closure
     (the sole operator → backend write surface, via `KillSwitch::trip`
     on the side-thread tokio runtime), `spec/reports/**/*.md` (viewer
     read path + file-naming convention), theme/strings/fixtures
     widget-side rules. Companion document
     `spec/ui-design-principles.md` lands the design-system rules. -->



# Architecture — Crypto Trading Agent

> **Phase 1A split is in progress (started 2026-05-13).** New
> architectural content goes into `spec/architecture/*.md` (section
> files) or `spec/architecture/adr/*.md` (numbered ADRs). This file is
> on a trajectory to become a thin index. Until that migration is
> complete, the substantive content below is still authoritative for
> any section that has not yet moved. See the **section file registry**
> and **ADR registry** below.

## Section file registry

| File                                                          | Status      | Source range in this file |
|---------------------------------------------------------------|-------------|---------------------------|
| [architecture/00-overview.md](architecture/00-overview.md)              | **shipped** | _moved 2026-05-13 (Session 2)_ |
| [architecture/01-data-flow.md](architecture/01-data-flow.md)            | **shipped** | _moved 2026-05-13 (Session 2)_ |
| [architecture/02-strategy-registry.md](architecture/02-strategy-registry.md) | **shipped** | _body synthesised 2026-05-13 (Session 12)_ |
| [architecture/03-execution-and-venues.md](architecture/03-execution-and-venues.md) | **shipped** | _body synthesised 2026-05-13 (Session 12)_ |
| [architecture/04-risk-and-money.md](architecture/04-risk-and-money.md)  | **shipped** | _moved 2026-05-13 (Session 3)_ |
| [architecture/05-llm-and-reflection.md](architecture/05-llm-and-reflection.md) | **shipped** | _moved 2026-05-13 (Session 3)_ |
| [architecture/06-ui-and-cockpit.md](architecture/06-ui-and-cockpit.md)  | **shipped** | _moved 2026-05-13 (Session 11)_ |
| [architecture/07-observability.md](architecture/07-observability.md)    | **shipped** | _moved 2026-05-13 (Session 3)_ |
| [architecture/08-recovery-and-backups.md](architecture/08-recovery-and-backups.md) | **shipped** | _moved 2026-05-13 (Session 3)_ |
| [architecture/09-performance-budget.md](architecture/09-performance-budget.md) | **shipped** | _moved 2026-05-13 (Session 3)_ |
| [architecture/10-foundation-libraries.md](architecture/10-foundation-libraries.md) | **shipped** | _moved 2026-05-13 (Session 11)_ |
| [architecture/11-regression-gate.md](architecture/11-regression-gate.md) | **shipped** | _body synthesised 2026-05-13 (Session 12)_ |

## ADR registry

Canonical index lives at [architecture/adr/README.md](architecture/adr/README.md).

| ID    | Title                                                      | Status     | Date       |
|-------|------------------------------------------------------------|------------|------------|
| [0001](architecture/adr/0001-crate-name-stdlib-collision.md) | Crate names avoid stdlib collisions      | accepted | 2026-04-17 |
| [0002](architecture/adr/0002-rng-chacha20.md)                | RNG seeded with ChaCha20 from config seed | accepted | 2026-04-17 |
| [0003](architecture/adr/0003-decimal-money-math.md)          | Money math uses Decimal, never f64        | accepted | 2026-04-17 |
| [0004](architecture/adr/0004-fractional-second-timestamps.md) | Audit-DB uses 6-digit fractional-second timestamps | accepted | 2026-04-18 |
| [0005](architecture/adr/0005-v0-strategy-trait-no-hotload.md) | v0 — clean strategy trait shape, no hot-load | accepted | 2026-04-17 |
| [0006](architecture/adr/0006-v05-config-driven-composition.md) | v0.5 — config-driven strategy composition (hot-load A) | accepted | 2026-04-19 |
| [0007](architecture/adr/0007-v1-wasm-plugin-deferred.md) | v1+ — WASM plugin hot-load deferred | accepted | 2026-04-19 |
| [0008](architecture/adr/0008-v05-strategy-event-journal-schema.md) | v0.5 — strategy-event journal schema (Q1) | accepted | 2026-04-19 |
| [0009](architecture/adr/0009-v05-registry-concurrency.md) | v0.5 — registry concurrency: parking_lot::RwLock (Q2) | accepted | 2026-04-19 |
| [0010](architecture/adr/0010-v05-composed-exit-policy.md) | v0.5 — ComposedStrategy exit policy: signal-flip only (Q3) | accepted | 2026-04-19 |
| [0011](architecture/adr/0011-v05-cockpit-strategies-panel.md) | v0.5 — cockpit Strategies panel: right column (Q4) | accepted | 2026-04-19 |
| [0012](architecture/adr/0012-v05-broadcast-bus-extensions.md) | v0.5 — strategy broadcast types in trading_core (Q5) | accepted | 2026-04-19 |
| [0013](architecture/adr/0013-v1-cross-sectional-momentum.md) | v1 — cross-sectional momentum resolutions (Q1–Q6) | accepted | 2026-04-29 |
| [0014](architecture/adr/0014-v15a-mean-reversion-pairs.md) | v1.5a — mean-reversion pairs resolutions (Q1–Q10) | accepted | 2026-04-30 |
| [0015](architecture/adr/0015-operator-success-reports.md) | v1+ — Operator success reports (Q1–Q9) | accepted | 2026-05-01 |
| [0016](architecture/adr/0016-real-mtm-unrealized-pnl.md) | v1+ — real-mtm unrealized PnL plumbing (Q1–Q8 + R10) | accepted | 2026-05-02 |
| [0017](architecture/adr/0017-v15b-multi-venue.md) | v1.5b — multi-venue execution scaffolding (Q1–Q12) | accepted | 2026-05-03 |
| [0018](architecture/adr/0018-lumen-phase-1-foundation.md) | Lumen design adoption — Phase 1 foundation (Q1–Q11) | accepted | 2026-05-04 |
| [0019](architecture/adr/0019-v2-llm-strategy.md) | v2 — LLM strategy foundation (Q4–Q11) | accepted | 2026-05-10 |
| [0020](architecture/adr/0020-chart-buy-sell-emphasis.md) | Chart buy/sell emphasis (v1.9 Q1–Q9) | accepted | 2026-05-10 |
| [0021](architecture/adr/0021-rustquant-adoption.md) | RustQuant adopted as helper, not foundation | accepted | 2026-04-17 |
| [0022](architecture/adr/0022-cost-telemetry-crate.md) | Cost telemetry lives in dedicated `cost` crate | accepted | 2026-04-17 |
| [0023](architecture/adr/0023-iced-frontend.md) | iced is the single UI stack | accepted | 2026-04-17 |
| [0024](architecture/adr/0024-audit-sqlite-raw-sqlx.md) | Audit ledger: raw `sqlx` + SQLite, not `sqlx-ledger` | accepted | 2026-04-19 |
| [0025](architecture/adr/0025-hand-rolled-binance-ws.md) | v0 hand-rolled Binance WS behind `MarketDataSource` trait | accepted | 2026-04-17 |
| [0026](architecture/adr/0026-v0-simple-paper-engine.md) | v0 simple paper engine; LOB deferred to v0.5 | accepted | 2026-04-17 |

All architectural decisions are now extracted to numbered ADRs. The
remaining Phase 1A work is the final monolith compression: shrink the
Changelog (Session 12), and finalise the section-file bodies for the
remaining stubs (02-strategy-registry, 03-execution-and-venues,
11-regression-gate).

## Workspace layout (proposed)

_Migrated to [architecture/00-overview.md § Workspace layout](architecture/00-overview.md#workspace-layout) during Phase 1A Session 2 (2026-05-13)._

## Naming conventions

_Migrated to [architecture/00-overview.md § Naming conventions](architecture/00-overview.md#naming-conventions); the stdlib-collision rule lives as [ADR-0001](architecture/adr/0001-crate-name-stdlib-collision.md)._

## Runtime

_Migrated to [architecture/00-overview.md § Runtime](architecture/00-overview.md#runtime)._

## Data flow

_Migrated to [architecture/01-data-flow.md](architecture/01-data-flow.md). The crate-dependency rules and bin-shared agent-runtime API live there too._

## ML / DL

_Migrated to [architecture/05-llm-and-reflection.md § ML / DL](architecture/05-llm-and-reflection.md#ml--dl)._

## LLM integration

_Migrated to [architecture/05-llm-and-reflection.md § LLM integration](architecture/05-llm-and-reflection.md#llm-integration). The foundation resolutions referenced below remain in this file pending extraction to ADR-0019 (Phase 1A Session 7)._

## Risk engine

_Migrated to [architecture/04-risk-and-money.md § Risk engine](architecture/04-risk-and-money.md#risk-engine). The money-math invariant lives there as a pointer to [ADR-0003](architecture/adr/0003-decimal-money-math.md)._

## Strategy registry & hot-loading

Strategies are first-class plug-ins. The runtime owns a typed registry of
active strategies and routes data/signals through each.

### Strategy hot-loading decisions (v0 → v1+)

_Migrated to numbered ADRs during Phase 1A Session 4 (2026-05-13):_

- [ADR-0005 — v0 clean trait shape, no hot-load](architecture/adr/0005-v0-strategy-trait-no-hotload.md)
- [ADR-0006 — v0.5 config-driven composition (hot-load A)](architecture/adr/0006-v05-config-driven-composition.md)
- [ADR-0007 — v1+ WASM plugins (hot-load B); native dyn-libs and embedded scripting rejected](architecture/adr/0007-v1-wasm-plugin-deferred.md)

### Lifecycle integration

Every strategy registry change (load, swap, unload, demote) emits a journal
entry to the audit ledger. Combined with the [strategy lifecycle gates in
product.md](product.md#strategy-lifecycle--promotion-gates), this means the
ledger always answers "which strategies were active when this trade fired?".

### v0.5 strategy-registry resolution cluster (Q1–Q5)

_Migrated to numbered ADRs during Phase 1A Sessions 5–6 (2026-05-13):_

- [ADR-0008 — v0.5 strategy-event journal schema (Q1)](architecture/adr/0008-v05-strategy-event-journal-schema.md)
- [ADR-0009 — v0.5 registry concurrency (Q2)](architecture/adr/0009-v05-registry-concurrency.md)
- [ADR-0010 — v0.5 ComposedStrategy exit policy (Q3)](architecture/adr/0010-v05-composed-exit-policy.md)
- [ADR-0011 — v0.5 cockpit strategies panel layout (Q4)](architecture/adr/0011-v05-cockpit-strategies-panel.md)
- [ADR-0012 — v0.5 broadcast bus extensions (Q5)](architecture/adr/0012-v05-broadcast-bus-extensions.md)

### v1 — cross-sectional momentum resolutions

_Migrated to [ADR-0013 — v1 cross-sectional momentum resolutions (Q1–Q6)](architecture/adr/0013-v1-cross-sectional-momentum.md) during Phase 1A Session 7 (2026-05-13). Six interconnected decisions: L2 deferred to v1.5, funding-rate observation-only, long-only confirmed, Binance-only, strategy-side universe filtering, and `RebalanceRejected` via `strategy_events` extension._

### v1.5a — mean-reversion pairs resolutions

_Migrated to [ADR-0014 — v1.5a mean-reversion pairs (Q1–Q10)](architecture/adr/0014-v15a-mean-reversion-pairs.md) during Phase 1A Session 8 (2026-05-13). Ten decisions: USDT-universe-only scope, fixed β=1.0, formulation C (observation-only short leg), composed `pnl_by_pair`, USDC blocked on v1.5b, L2/funding stay deferred, single `portfolio_exposure_cap`, two new `strategy_events` kinds, strategy-proposes/risk-disposes composition rule, and wait-for-sync bar synchronization._

### v1+ — Operator success reports resolutions

_Migrated to [ADR-0015 — v1+ Operator success reports (Q1–Q9)](architecture/adr/0015-operator-success-reports.md) during Phase 1A Session 9 (2026-05-13). Nine decisions: dedicated `crates/reports/`, additive migration 004 for `pnl_by_strategy`, tempfile+rename atomicity, Unicode block sparklines, CSV companion artefacts, exact-cent reconciliation, 12-field front-matter, `KillSwitchTripped` strategy_events variant, and the R6 placeholder lifecycle that locks the two `report-sample-*` anchors._

### real-mtm-unrealized-pnl resolutions

_Migrated to [ADR-0016 — real-mtm unrealized PnL (Q1–Q8 + R10)](architecture/adr/0016-real-mtm-unrealized-pnl.md) during Phase 1A Session 9 (2026-05-13). Eight decisions plus R10 deferral: snapshot-vec reader signature, `OpenPosition` in `trading_core`, no new SQL index (V8 perf gate PASSED), byte-identical anchors, non-anchored test fixture, avg-cost-basis mark source (architect override), weighted-average cost basis, long-only at v1+, plus R10 (BTC account hardcode) deferred to per-symbol-position-accounts._

### v1.5b — multi-venue resolutions

_Migrated to [ADR-0017 — v1.5b multi-venue (Q1–Q12)](architecture/adr/0017-v15b-multi-venue.md) during Phase 1A Session 9 (2026-05-13). Twelve decisions: closed `Venue` enum, Coinbase Advanced Trade WS, per-venue `tokio::JoinSet` panic isolation, required `venue` field on `Tick`/`Bar`, client-side 1s bar aggregation, operator-gated USDC universe doubling, per-venue stale-data pause via `bus.market_health`, free unauthenticated WS, 30-60 subscription slots, `MockFeed` test harness, additive audit migration 007 (typed `venue` column), and zero-by-construction anchor risk (11/11 byte-identical)._

### Lumen design adoption — Phase 1 foundation resolutions

_Migrated to [ADR-0018 — Lumen Phase 1 foundation (Q1–Q11)](architecture/adr/0018-lumen-phase-1-foundation.md) during Phase 1A Session 10 (2026-05-13). Token system rewrites 12 → ~50 (flat SHOUTY_SNAKE_CASE), surface tier system, 13-step whisper-shadow ladder, bounded best-effort focus-ring under iced 0.14.2 API gap (Option A), 13/6/7-step spacing/radii/typography ladders, motion timings, status_bar widget consuming existing `bus.market_health()`, single-file principles-doc consolidation, plus master Q10 operator-locked constraints (no brand, no strings rewrite, no icons, sequential phasing). UI-only feature; 11/11 anchors verified byte-identical._

### v2 — LLM strategy resolutions

_Migrated to [ADR-0019 — v2 LLM strategy foundation (Q4–Q11)](architecture/adr/0019-v2-llm-strategy.md) during Phase 1A Session 10 (2026-05-13). Foundation-only v2.0.0 ship — no LLM consumers yet. Decisions cover async non-streaming trait with day-one tool-use, 8-variant `LlmError`, cost-crate `ProviderKind` rename, TTL prompt-cache with provider-aware builder, `BudgetedProvider<Inner>` decorator with AtomicU64 cents counter, hybrid hard-coded + TOML cost-rate lookup, SQLite-WAL canonical-JSON-SHA-256 strict-replay storage, exponential backoff with full jitter (3 retries, no circuit breaker), and the Option C hot-fix that re-locks the two `report-sample-*` anchors once at `T_FINAL_V2_LLM_STRATEGY`._

### Chart buy/sell emphasis (v1.9) — resolutions

_Migrated to [ADR-0020 — chart buy/sell emphasis (v1.9 Q1–Q9)](architecture/adr/0020-chart-buy-sell-emphasis.md) during Phase 1A Session 10 (2026-05-13). Six architect-resolved decisions: additive `strategy_signals` audit table (polled by cockpit), linear-interpolation marker y-snap, custom canvas tooltip implementation, dual-layer marker visual treatment (13-px filled + 8-px ghost trail using Lumen tokens from ADR-0018), `widgets::volume_histogram` per-bar histogram widget, and `SignalView` placement in `crates/core/src/views.rs`._

## Observability

_Migrated to [architecture/07-observability.md](architecture/07-observability.md)._

## Disaster recovery & backups

_Migrated to [architecture/08-recovery-and-backups.md](architecture/08-recovery-and-backups.md)._

## Performance budget

_Migrated to [architecture/09-performance-budget.md](architecture/09-performance-budget.md)._

## Foundation libraries

_Migrated to [architecture/10-foundation-libraries.md](architecture/10-foundation-libraries.md) during Phase 1A Session 11 (2026-05-13). Six substantive decisions extracted to numbered ADRs:_

- [ADR-0021 — RustQuant adopted as helper, not foundation](architecture/adr/0021-rustquant-adoption.md)
- [ADR-0022 — Cost telemetry lives in a dedicated `cost` crate](architecture/adr/0022-cost-telemetry-crate.md)
- [ADR-0023 — iced is the single UI stack across the project](architecture/adr/0023-iced-frontend.md)
- [ADR-0024 — Audit ledger uses raw `sqlx` against embedded SQLite, not `sqlx-ledger`](architecture/adr/0024-audit-sqlite-raw-sqlx.md)
- [ADR-0025 — v0 hand-rolled Binance WS behind a `MarketDataSource` trait](architecture/adr/0025-hand-rolled-binance-ws.md)
- [ADR-0026 — v0 simple paper engine; full LOB deferred to v0.5](architecture/adr/0026-v0-simple-paper-engine.md)

_The detailed UI architecture body (cockpit screen routing, `audit::query` API surface, KPI strip widget contracts, status bar) moved to [architecture/06-ui-and-cockpit.md](architecture/06-ui-and-cockpit.md)._

## Changelog

The pre-Phase-1A changelog (976 lines of per-feature entries dated
2026-04-17 through 2026-05-10) was archived during Phase 1A
Session 12. The full prior content lives at
`spec/archive/architecture-changelog-pre-split-2026-05-13.md` (text)
and is preserved verbatim in git history at the pre-Session-12
commit.

**Going forward**, individual decision changelog entries live in
each ADR's own `## Changelog` section, not here. This file's
changelog is reserved for **architecture-level meta events** — file
splits, ADR-numbering schema changes, registry restructurings, and
cross-cutting refactors that span multiple ADRs.

- 2026-05-13 (architect): **Phase 1A complete.** The 5,635-line
  monolithic `architecture.md` decomposed into:
  - 12 section files under `architecture/` (workspace overview,
    data flow, strategy registry, execution & venues, risk &
    money, LLM & reflection, UI & cockpit, observability, recovery,
    performance budget, foundation libraries, regression gate);
  - 26 numbered ADRs under `architecture/adr/` capturing every
    historical and ongoing architectural decision;
  - A thin index in this file (section + ADR registries + pointer
    blocks).

  Final architecture.md size after this session: ~350 lines (vs
  5,635 original = -94% reduction). All architectural decisions
  are now in numbered ADRs; all current-state documentation is in
  section files. The 976-line per-feature changelog (entries
  2026-04-17 through 2026-05-10) was archived since every entry's
  substance is already in the corresponding ADR.

  Sessions 1-12 of Phase 1A and their dev-notes are linked from
  `spec/dev-notes/phase-1a-*.md`.
