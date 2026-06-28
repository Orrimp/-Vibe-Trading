---
slug: archive-architecture-changelog
status: deprecated
owner: archive
updated: 2026-05-13
---

# Pre-split architecture.md changelog (2026-04-17 to 2026-05-10)

The per-feature changelog entries that lived in `spec/architecture.md`
prior to Phase 1A Session 12. Archived 2026-05-13 because every entry's
substance is already in the corresponding ADR (ADRs 0001-0026); this
file preserves the prose for future archaeology.

Original line range: 311..1286 of `spec/architecture.md` at the
post-Session-11 state. To retrieve a specific entry, use the ADR
registry in `spec/architecture/adr/README.md` and cross-reference by
date.

## Changelog

- 2026-05-10 (architect, v2 LLM design): appended decisions-index
  block **"v2 — LLM strategy resolutions (Q4–Q11) — confirmed
  2026-05-10"** sibling of the existing v1+ / Lumen Phase blocks.
  Replaced the v0-stub `## LLM integration` paragraph at lines
  421–432 with a cross-reference to the new section. Seven
  architect-decided Q-items resolved: Q4 trait shape (async +
  non-streaming + tool-use + 8-variant `LlmError` + cost-crate
  `LlmProvider → ProviderKind` rename), Q5 prompt-cache (TTL-
  driven, 2 breakpoints, provider-aware builder, Prometheus
  counter pair + `audit::query::cache_hit_ratio_since`), Q6
  budget-gate placement (factory-level decorator + `AtomicU64`
  cents counter + new V12 verification gate for the documented
  0.2% concurrent-overshoot bound), Q7 cost-rate lookup
  (hard-coded base + TOML override, module in `llm` crate), Q8
  replay storage (SQLite WAL + canonical-JSON SHA-256 +
  `schema_version` + 9-row fixture + strict-replay-only at
  v2.0.0), Q9 rate-limit (full jitter + 3 retries + no circuit
  breaker + `Retry-After` honored), Q11 operator-success-report
  denominator update (Option C — bundled with Q5d's `Cache hit
  ratio` row addition; `report-sample-*` anchors re-lock once at
  T_FINAL_V2_LLM_STRATEGY). Operator's four [OPERATOR-DECIDE]
  resolutions (Q1 = foundation-only, Q2 = Anthropic both tiers,
  Q3 = config-file with explicit acknowledgement, Q10 = strawman
  cockpit tile + memo + report line) are inputs, baked into the
  brief verbatim. Foundation-only scope means **zero LLM
  consumers in v2.0.0**; each consumer (post_mortem enrichment,
  news/sentiment overlay, trader debate, reflection-memory
  trader-wiring) becomes its own follow-up brief on the stable
  trait surface this section locks in. **9 strategy-backtest
  anchors at `spec/anchors.toml:15-58` stay byte-identical**
  (R14.2 enforced via T1937 negative-invariant test); **2
  `report-sample-*` anchors at lines 67–75 re-lock once** at
  T_FINAL_V2_LLM_STRATEGY (tester only, never architect). Tasks
  expanded at `spec/v2-llm-strategy/tasks.md` — 45 developer T
  tasks (T1901–T1945) + `T_FINAL_V2_LLM_STRATEGY`. New
  verification gate V12 added (concurrent-overshoot bound).
  Crate / module surface enumerated: 32 new files + 22 existing
  files modified. HANDOFF → developer.
- 2026-05-06 (architect, Phase 5 design): appended Phase 5
  **"Q1–Q15 ratification (Phase 5, confirmed 2026-05-06)"** sub-
  section under the existing Phase 4 ratification block. **15 / 15
  architect Q-items ratified, zero principled overrides on substance:**
  Q1 HumanControl as 7th sidebar entry `Screen::Control` (Lumen
  "always-visible" framing + Phase 2 / 3 IA consistency; Debug-screen
  kill migrates to HumanControl bottom action via new
  `widgets::kill::view_inner` body-extraction helper); Q2 / Q3 new
  audit writers `audit::journal::strategy_paused` +
  `audit::journal::risk_veto_overridden` (sibling of
  `kill_switch_tripped` at `crates/audit/src/journal.rs:316–407`,
  atomic dual-write — memo row + `strategy_events` row in one txn,
  6-digit fractional-second `ts` per HF-3 gate, `kind` PascalCase,
  `error_summary` carries direction / reason); Q4 execution-mode
  runtime-only persistence (cold-start = `ExecutionMode::Observe`, no
  `config/agent.toml` write, no audit writer); **Q5 / TD-1 = path
  (b) custom-widget escape hatch** — verified at design pass
  `crates/ui/Cargo.toml:69` still pins `iced = "=0.14.0"`, iced 0.15+
  has not landed (path a fold-in unavailable), restate-with-deadline
  rejected (Phase 6 v2-LLM gated, operationally indefinite); Phase 5
  is the operator-write-surface sharpening point so a fifth
  restatement is no longer viable; commits to a new
  `crates/ui/src/widgets/focus_ring.rs` Subscription-driven wrapper
  (owns focus state via `iced::keyboard::on_key_press` filtered to
  `Tab` / arrows, emits `Message::FocusChanged(WidgetId)`, renders
  halo via existing `theme::focus::ring(mode)` token) wrapping all
  four destructive surfaces (kill button + kill confirm input +
  override-risk-veto confirm + per-strategy pause + execution-mode
  segments). **The four-phase TD-1 deferral closes at Phase 5 ship.**
  Q6 snapshot rename via `git mv` (preserves history; body diff =
  title-string only); Q7 full Lumen field set (mode + 3 limits +
  kill); Q8 single-click pause-resume (no typed-confirm — bounded-
  destructive); Q9 per-veto override (forward-only — agent does not
  re-emit blocked signal); Q10 unit + integration + audit-row
  snapshot baseline (all three); Q11 ~9 rename + ~12 net-new + 1
  Q1-driven Debug regen + 1 focus-ring net-new + 2 audit-row
  baselines, single `cargo insta accept` pass per Phase 1 Q2 / Phase
  2 V11 / Phase 3 V12 / Phase 4 V12 precedent; Q12 preserve
  `KILL_BUTTON_LABEL = "Stop trading"` (Master Constraint 2); **Q13
  placeholder feed for risk-engine veto-emit; deferred upstream
  wiring tracked as new TD-2 row** (architect flags for orchestrator
  to append to master roadmap's Cross-phase technical-debt section
  on Phase 5 ship); Q14 preserve `Cockpit::tape` field name (rename
  ripples through ~100+ test sites for cosmetic value) — annotated
  via code-comment pointing at `widgets::agent_feed`; Q15 NO new
  audit-query reader (defer; existing `recent_journal_filtered`
  covers via `kind` filtering). Cockpit state diff specified —
  `ExecutionMode` enum, `OverrideRiskVetoState` enum, `VetoEvent`
  struct, four new fields (`execution_mode`, `paused_strategies`,
  `override_risk_veto`, `risk_veto_events`), six new `Message`
  variants (`ExecutionModeSelected`, `StrategyPauseToggled`, +4
  `OverrideRiskVeto*` family). HumanControl panel widget contract
  (`crates/ui/src/widgets/human_control.rs` — frame + title constants
  + mode segment + 3 mirror rows + kill bottom action), pause-
  strategy control contract (single-click `widgets::strategies::pause_button`
  + `pause_strategy_tx` broadcast channel + audit-writer call),
  override-risk-veto control contract
  (`crates/ui/src/widgets/override_risk_veto.rs` mirror of kill-
  confirm + `OVERRIDE` phrase + per-veto button + clear-from-list
  + audit-writer call), execution-mode toggle contract (segmented
  control + 3 hint constants + `execution_mode_tx` broadcast
  channel), audit writer additions (exact signatures + column
  projection table + 7 unit + 2 integration + 2 row snapshot
  baselines), `tape` → `agent_feed` rename (`git mv` preserves
  history; field name preserved per Q14), TD-1 resolution (path b
  custom-widget escape hatch with concrete `focus_ring.rs` shape),
  risk-engine veto-emit deferral (TD-2 tracking row text). Cross-
  feature invariants table re-stated (7 rows, all preserved). **Zero
  anchor risk re-affirmed** — additive `StrategyEventKind` variants;
  no schema migration; no committed report body re-renders. Snapshot
  ripple: 9 rename pairs + ~12 net-new + 1 Debug regen + 1 focus-
  ring + 2 audit-row baselines; single `cargo insta accept` pass.
  Implementation parallelism map: T1901 foundation gate → fan-out
  across T1902 (audit writers) / T1903 (rename) / T1904 (HumanControl
  skeleton) / T1909 (override modal) / T1912 (focus-ring) → T1905 /
  T1906 / T1911 share HumanControl + focus-ring → T1907 / T1908 /
  T1910 share audit writers + focus-ring → narrow at T1913 snapshot
  accept → T1914–T1916 → T_FINAL. Task list at
  [tasks/lumen-phase-5-humancontrol-agentfeed.md](tasks/lumen-phase-5-humancontrol-agentfeed.md)
  with 16 T19xx tasks + tester `T_FINAL_LUMEN_PHASE_5` gate.
  **Master-roadmap follow-ups flagged for orchestrator on Phase 5
  ship (architect does not edit master roadmap directly):** (a)
  TD-1 row gains a 2026-05-06 closure note ("path b custom-widget
  escape hatch shipped at `crates/ui/src/widgets/focus_ring.rs`");
  (b) new TD-2 row appended for the risk-engine veto-emit upstream
  wiring deferral. HANDOFF → developer ‖ ui-designer.
- 2026-05-06 (architect, Phase 4 design): appended Phase 4
  **"Q1–Q12 ratification (Phase 4, confirmed 2026-05-06)"** sub-
  section under the existing Phase 3 ratification block. **12 / 12
  architect Q-items ratified, zero principled overrides:** Q1 richer
  `EquitySeries` shape with `EquityPoint = { ts, equity, drawdown_pct }`
  + precomputed peak / trough / max-DD / inception / as-of (drawdown
  vector inside each point — not a parallel `Vec<Decimal>`; eliminates
  off-by-one risk between consumers); Q2 shared
  `widgets::canvas_chart` core extracted from Phase 2 internal
  helpers + new `polyline_with_fill` primitive shared across four
  wrappers (`widgets::chart` Phase 2 byte-stable, `widgets::equity_curve`,
  `widgets::drawdown_band`, `widgets::sparkline`); Q3 KPI source
  parses the existing markdown summary table (`crates/reports/src/parse.rs::BacktestMetrics::parse_from_report`),
  graceful fallback to `VIEWER_METRICS_UNAVAILABLE` strip on parse
  failure, no sidecar JSON, no write-path change; Q4 CLI-only viewer
  (`clap`-parsed positional `<report-path>`, missing arg → 2,
  non-existent file → 3); Q5 cap at 2000 points via
  `EquitySeries::downsample`; Q6 solid `DOWN_500 @ 0.18` drawdown fill
  + matching `UP_500 @ 0.18` equity fill; Q7
  `equity_curve_for_strategy(ledger, strategy_id, since,
  until: Option<Timestamp>) -> Result<EquitySeries, LedgerError>`
  read-only sibling of `pnl_by_strategy` over the same
  `income:realized_pnl` rows + running cash-balance baseline + new
  `LedgerError::EmptyWindow` variant; Q8 sparkline placement above
  the chip row at the existing 160 px slot; Q9 cap+downsample at
  fetch (`SPARKLINE_POINT_CAP = 120`), one-shot via `Task::perform`
  on `Message::SelectStrategy`, no live update; Q10 dark default
  cold-start; Q11 5 net-new + 1 deletion snapshot ripple, single
  `cargo insta accept`; Q12 new module `crates/core/src/equity_series.rs`
  co-locating `EquitySeries` + `EquityPoint` + `EquitySeriesError` +
  `BacktestMetrics`. TD-1 deferred — verified `crates/ui/Cargo.toml:52`
  still pins `iced = "=0.14.0"`; viewer is zero-button surface and
  cockpit-side sparkline non-focusable so deferral is operationally
  invisible on Phase 4 deliverable. Next re-evaluation at Phase 5
  (HumanControl) analyst kickoff. Cross-feature invariants preserved
  (7 / 7); zero anchor risk re-affirmed (read-only over committed
  reports + read-only audit query addition + UI-only screens; no
  `crates/strategy/` / `crates/cost/` / `crates/backtest/` /
  `crates/reports/src/render/` write-path touched). Library-compat
  checklist: no new deps (no `pulldown-cmark`, no chart crate, no
  file-picker crate; in-module ~30 LOC heading-pre-pass for the
  markdown body). **App-layout table updated** at
  [architecture.md:2947–2951](#app-layout) — `viewer` row's window
  contract becomes "Backtest report shell · KPI strip + equity curve
  + drawdown band + markdown body (Phase 4 — shipped)"; data source
  becomes "`spec/reports/` markdown + `<stem>__equity.csv` companion".
  **Phase 3 deferral closure** — `STRATEGIES_SPARKLINE_DEFERRED`
  retires from `crates/ui/src/strings.rs:261`; new
  `STRATEGIES_SPARKLINE_LOADING` lands; the `strategies_screen__sparkline_deferred.snap`
  baseline retires in the same commit as the `_present.snap` lands.
  Task list at
  [tasks/lumen-phase-4-backtest-panel.md](tasks/lumen-phase-4-backtest-panel.md)
  with 15 T18xx tasks (T1801 foundation gate → fan-out across T1802 /
  T1803 / T1804 / T1808 → widget modules T1805–T1807 + T1809 share
  T1804 canvas-chart core → narrow at T1810 viewer composition + T1811
  cockpit sparkline → narrow at T1812 snapshot accept → T1813–T1815
  → `T_FINAL_LUMEN_PHASE_4`). HANDOFF → developer ‖ ui-designer
  (developer takes T1801–T1815 implementation; ui-designer takes
  the visual-diff attestation sub-block at T1812 / T_FINAL after
  the developer's snapshot refresh pass).
- 2026-05-05 (architect, Phase 3 design): appended Phase 3
  **"Q1–Q11 ratification (Phase 3, confirmed 2026-05-05)"** sub-
  section under the existing Phase 2 ratification block. **11 / 11
  architect Q-items ratified, zero principled overrides:** Q1
  `008_journal_transactions_venue.sql` migration ships in Phase 3
  (`ADD COLUMN venue TEXT NOT NULL DEFAULT 'Binance'` — additive,
  the default is the backfill; writer at
  `crates/audit/src/journal.rs::post_fill` gains a `venue: Venue`
  parameter, two other `INSERT INTO journal_transactions`
  call-sites take same treatment; Phase 2 venue gate dropped from
  `recent_fills_filtered`), Q2 signal history filters
  `Cockpit::strategies_recent_events` (no new audit writer), Q3
  Risk via new `RiskTelemetry` tokio channel mirroring Phase 1
  `MarketHealth`, Q4 audit pagination fixed at 250, Q5 audit
  filter persistence in-session only, Q6 equity sparkline deferred
  to Phase 4 (Phase 3 ships placeholder copy only), Q7 audit query
  as sibling `recent_journal_filtered(ledger, venues, symbol, kind,
  since, until, page_offset, page_size) -> (Vec<JournalRow>, u64)`,
  Q8 sidebar order Home → Debug → Strategies → Risk → Audit →
  Charts via `SIDEBAR_ENTRIES_PHASE_3` constant swap (widget body
  unchanged, six `SIDEBAR_NAV_*` strings already declared at
  Phase 2 declare-now), Q9 kill-threshold gauge as horizontal bar
  via new `frame::threshold_bar` helper (sibling of `active_row` /
  `active_chip`), Q10 read-only display, Q11 ~13 snapshot ripple +
  compound-dispatch cross-link (Home → Strategies-summary row
  click emits `SelectStrategy` + chained `Task::done(SwitchScreen)`
  in the binary). TD-1 deferred — verified
  `crates/ui/Cargo.toml:52` still pins `iced = "=0.14.0"`; next
  re-evaluation at Phase 4 analyst kickoff. Cross-feature
  invariants preserved (7 / 7); zero anchor risk re-affirmed
  (additive migration with constant-string backfill + read-only
  audit query addition + UI-only screens; `crates/strategy/`,
  `crates/cost/`, `crates/backtest/`, `crates/reports/` untouched).
  Library-compat checklist: no new deps. Task list at
  [tasks/lumen-phase-3-detail-screens.md](tasks/lumen-phase-3-detail-screens.md)
  with 16 T17xx tasks (T1701 foundation gate → fan-out across
  T1702 / T1703 / T1707 / T1709 / T1712 → T1704–T1706 / T1708 /
  T1710–T1711 → narrow at T1713 snapshot accept → T1714–T1716
  → `T_FINAL_LUMEN_PHASE_3`). HANDOFF → developer ‖ ui-designer
  (developer takes T1701–T1716 implementation; ui-designer takes
  the visual-diff attestation sub-block at T1713 / T_FINAL after
  the developer's snapshot refresh pass).
- 2026-05-04 (architect, Phase 2 design): appended Phase 2
  **"Q1–Q11 ratification (Phase 2, confirmed 2026-05-04)"** sub-
  section under the existing "Cockpit screen routing (Phase 2+
  contract)" block. **11 / 11 architect Q-items ratified, zero
  principled overrides:** Q1 line-series default, Q2 pan/zoom
  deferred, Q3 `Cockpit::universe` boot-populated, Q4
  `since/until` two-arg signature for `recent_fills_filtered`
  (with Phase 2 venue-handling note — Binance-only fills on disk
  per v1.5b plumbing-only state; Phase 3 Audit screen promotes
  to a `journal_transactions.venue` migration), Q5 chip-row
  bottom-edge T1507 variant via new `frame::active_chip` helper,
  Q6 per-symbol synthetic-candle seed via `DefaultHasher`
  in-process determinism, Q7 right-rail reserved structurally as
  a single `Length::Fixed(0.0)` column (no `cfg!` gate), Q8
  two-field session-scoped persistence (no on-disk state), Q9
  Debug screen logs as placeholder, Q10 audit-query unit test
  only in Phase 2 (integration deferred to Phase 3), Q11 TD-1
  deferred — verified `crates/ui/Cargo.toml:50` still pins
  `iced = "=0.14.0"`, the `button::Status::Focused` and
  `text_input::Style.shadow` API surface has not landed; next
  re-evaluation at Phase 3 analyst kickoff. Cross-feature
  invariants preserved (7 / 7); zero anchor risk re-affirmed
  (read-only audit query extension + UI shell + new widget; no
  strategy / exec / risk / cost / backtest / reports crate
  touched). Library-compat checklist: no new deps (iced unchanged,
  `rand_chacha::ChaCha20Rng` already in workspace, `DefaultHasher`
  is `std`). Task list at
  [tasks/lumen-phase-2-shell-ia-charts.md](tasks/lumen-phase-2-shell-ia-charts.md)
  with 16 T16xx tasks (T1601 foundation gate → fan-out across
  T1602–T1612 → narrow at T1613 snapshot accept → T1614–T1616
  → `T_FINAL_LUMEN_PHASE_2`). HANDOFF → developer ‖ ui-designer
  (developer takes T1601–T1616 implementation; ui-designer takes
  the visual-diff attestation row at T_FINAL after the
  developer's snapshot refresh pass).
- 2026-05-04 (architect, post-Phase-1 ship): added new sub-section
  **"Cockpit screen routing (Phase 2+ contract)"** under Frontend ↔
  backend interfaces. Documents the `Screen` enum, the
  `Cockpit::current_screen` + `Message::SwitchScreen` contract, the
  per-`(venue, symbol)` chart rolling buffer (live = existing
  `bars_tx` channel; fixtures = deterministic synthetic candles via
  `ui::fixtures::synthetic_candles`), the additive `audit::query::recent_fills_filtered`
  signature for chart buy/sell markers, and the right-rail
  column-track reservation for Phase 6 (Assistant slot, gated on v2
  LLM). Updated the App layout table to reflect the multi-screen
  reality and to add `cockpit_live` as a distinct binary row. The
  contract above is the shared scaffolding every Phase 2+ widget
  plugs into; per-phase R-items live in the per-phase briefs at
  [features/lumen-phase-2-shell-ia-charts.md](features/lumen-phase-2-shell-ia-charts.md)
  through
  [features/lumen-phase-6-assistant-slot.md](features/lumen-phase-6-assistant-slot.md).
  Anchor risk: zero per phase (read-only audit query extensions and
  UI shell additions; see master roadmap anchor-risk table).
- 2026-04-17 (architect): initial scaffold.
- 2026-04-17 (architect): added Foundation libraries section; selected
  RustQuant modules `math` / `stochastics` / `time` / `data` / `iso` /
  `macros` as helpers, explicitly excluded fixed-income modules and the
  basic ML/portfolio/LOB modules; locked default crates for async, numerics,
  data, LLM, and testing.
- 2026-04-17 (architect): surveyed [lib.rs/finance](https://lib.rs/finance);
  added `rust_decimal` as mandatory, defined `Money<C: Currency>` strategy,
  added LOB/matching candidates (`orderbook-rs` / `matchcore` / `rust_ob`),
  TA picks (`kand` + `quantedge-ta`), `openpit` as second-line risk,
  an off-the-shelf double-entry ledger for the new `audit` crate (candidates
  then were `cala-ledger` and `sqlx-ledger`; see later changelog entry for
  the final pick), `trade_aggregation` for tick→bar,
  `yfinance-rs` for macro overlay, `time` chosen over `chrono` workspace-wide;
  flagged crypto exchange clients as a research gap; documented exclusions
  (FIX, equities data, options pricing, payments, personal-finance ledgers).
- 2026-04-17 (architect): selected [iced](https://github.com/iced-rs/iced)
  as the single UI stack; added `ui` crate with two binaries
  (`cockpit` live ops, `viewer` backtest); locked design constraints
  (`ui::strings`, `ui::theme`, confirm dialogs on destructive actions,
  first-class empty/loading/error states); UI depends only on `core` + read-only
  `audit`, never on trading logic.
- 2026-04-17 (architect): added Strategy registry & hot-loading section.
  v0 ships compiled-in `Strategy` trait with plug-in-shaped registry;
  v0.5 adds config-driven `ComposedStrategy` (file-watcher hot-swap);
  v1+ adds WASM plugins via `wasmtime`. Native dynamic libs and embedded
  scripting explicitly rejected. Registry mutations journal to audit ledger.
- 2026-04-17 (architect): resolved the five v0-paper-sma open questions from
  the analyst brief. Q1 matching engine: v0 ships a simple `PaperEngine`
  (bps slippage + taker fee + optional bar-VWAP) behind a `MatchingEngine`
  trait; full LOB deferred to v0.5. Q2 audit backing store: `sqlx-ledger` on
  SQLite for single-binary deploy (cala-ledger requires Postgres in current
  releases); migrate at v1+ if hosting shape changes. Q3 UI → audit: approved
  and made explicit via a new `audit::query` read-only module exposing
  `Decimal` aggregates / slice iterators; ledger is the single source of
  truth for P&L (no cockpit accumulator). Q4 cost location: dedicated `cost`
  crate added to the workspace map with `CostEvent` / `CostSink` /
  `CostBudget` surface; v0 wires the scaffold with zero emitters. Q5 crypto
  exchange client: hand-rolled Binance WS adapter on `tokio-tungstenite`,
  isolated behind a `MarketDataSource` trait with `BinanceFeed` / `ReplayFeed`
  / `FakeFeed` implementations; `barter-data` is the explicit v0.5 fallback
  once multi-venue lands.
- 2026-04-17 (architect): added Naming conventions section. Renamed the
  foundation crate package from `core` to `trading_core` workspace-wide to
  stop shadowing Rust stdlib `::core::` (Rust 2024); crate directory stays
  `crates/core/`. Replaces the per-consumer `trading_core = { package = "core" }`
  alias trap and unblocks `cargo test --workspace --doc`. See Week 1 test
  report [spec/archive/test-2026-04-17-1443-v0-paper-sma-week1.md](reports/test-2026-04-17-1443-v0-paper-sma-week1.md (archived; see spec/archive/README.md))
  section 7 (R-A).
- 2026-04-17 (architect): formally signed off `sqlx-ledger` on SQLite as the
  v0 audit-ledger substrate (supersedes the earlier candidate language).
  `cala-ledger` deferred to v1+ — reconsider only if hosted deployment moves
  off single-box *and* `cala-ledger` has gained a SQLite backend by then.
  Week 1 T05/T06 integration tests (5/5 green) confirm the pick.
- 2026-04-19 (architect): added **Disaster recovery & backups** section
  reflecting the operator's locked DR decision — local-only snapshots (daily
  `sqlite3 .backup`, weekly Parquet rsync), RPO 24h / RTO ~1h manual, zero
  cloud spend. Off-site sync and WAL streaming explicitly deferred to the
  follow-up project that lands real-money execution.
- 2026-04-17 (developer): repair pass — updated chart-of-accounts count from
  10 to 13 (added `expense:infra`, `expense:data` per cost-telemetry scaffold;
  LLM accounts were already present). `cala-ledger` count in the v0 decision
  prose updated from 10 to 13.
- 2026-04-19 (architect): reconciled Audit & ledger section to code reality
  and resolved the five v0.5 composed-strategies open questions
  ([feature brief](features/v05-composed-strategies.md)).
  **Doc-reality reconciliation:** `sqlx-ledger` v0.11.14 is Postgres-only
  in shipped releases; during v0 Week 1 the developer discovered this and
  substituted raw `sqlx` + `SQLite` + in-repo migrations preserving
  identical double-entry semantics. The Audit & ledger section now documents
  what the code actually does. Substitution is additive: the `audit::query`
  public API stays `Decimal`/core-types-only and is unchanged.
  **Q1 strategy-event journal:** new sibling `strategy_events` table in the
  SQLite ledger; keeps `journal_entries` monetary-only; exposed via
  `audit::query::strategy_events_since` /
  `audit::query::strategy_history`.
  **Q2 registry concurrency:** `parking_lot::RwLock<HashMap<..>>` — zero
  new deps, sub-microsecond uncontended read, fits the 1m bar cadence.
  `arc-swap` reconsidered in v1+ if tick-latency strategies arrive.
  **Q3 exit policy:** symmetric signal-flip in v0.5; per-strategy
  drawdown clamp deferred to v1+ and will live in `risk`, not inside
  each `ComposedStrategy`.
  **Q4 strategies panel layout:** right column above Open positions;
  keeps observation-oriented widgets together and protects the left
  column's destructive-action focus.
  **Q5 new broadcast message types:** `StrategyLoaded` /
  `StrategySwapped` / `StrategyLoadError` live in `trading_core`; three
  new `agent::EventBus` channels (capacity 32 each); lagged-drop + log
  backpressure matches the v0 pattern.
- 2026-04-30 (architect): resolved the ten v1.5a-mean-reversion-pairs
  open questions from
  [features/v15a-mean-reversion-pairs.md → Notes](features/v15a-mean-reversion-pairs.md#notes--open-questions-for-architect).
  **Q1 split** confirmed — v1.5a is pairs-strategy-only on the
  Binance USDT universe; multi-venue + USDC + 1s aggregated trades
  + T612 are queued in sibling `v15b-multi-venue-live-ingest`.
  **Q2 hedge ratio** fixed β = 1.0 with per-pair TOML override
  (`beta = "..."`); rolling-OLS β deferred to v1.5c. **Q3 spot-only
  formulation** is **C — observation-only short leg**: long-leg `a`
  executes; would-have-shorted `b` logs as
  `pair_short_observation` event so v2 perp executor can backfill
  short P&L from history. **Q4 `pnl_by_pair`** composes existing
  `pnl_by_symbol` against a `PairMembership` map; no schema
  migration; `(a, b)` lex-sorted; v1.5a invariant
  `pnl_by_pair[(a, b)] == pnl_by_symbol[a]` because the `b` leg is
  never traded. **Q5 USDC pairs** blocked on v1.5b multi-venue;
  v1.5a is USDT-only with three default pairs `(BTC, ETH)`,
  `(ETH, SOL)`, `(BNB, BTC)`. **Q6 L2 / funding** stay deferred —
  pair MR doesn't consume either; the v1 funding poller stays as-is
  observation-only. **Q7 `portfolio_exposure_cap`** reuses the v1
  single field; default bumped from `0.50` → `0.75` in the v1.5a
  TOML (Rust default unchanged); strategy-internal
  `exposure_cap_per_pair = 0.25` is the first-line clamp.
  **Q8 `MeanReversionStop` + `pair_short_observation`** extend the
  v0.5 `strategy_events.kind` column (additive — no SQL migration);
  new `audit::journal::mean_reversion_stop` and
  `audit::journal::pair_short_observation` writers. **Q9 per-symbol
  cap composition under stacked pair exposures** — strategy emits
  desired vector, `risk::size_portfolio_target` clamps per-symbol
  (existing v0 invariant); overlapping `a` legs degrade gracefully
  via `rebalance_rejected`; the analyst's default 3-pair list has
  non-overlapping `a` legs by construction. **Q10 pair-bar sync** —
  wait-for-sync on `venue_ts` equality with a configurable
  `max_staleness_minutes` (default 5) clamp; deterministic via the
  v1 `(venue_ts ASC, symbol ASC)` interleave. **v1.5a architectural
  deltas:** new `crates/core/src/pair.rs` (`Pair`, `PairKey`,
  `PairMembership`); new `features::pairs` module (`spread`,
  `rolling_zscore`) reusing v1 `decimal_ln` / `decimal_sqrt` /
  `RingBuffer<Decimal>`; new `strategy::pairs` module
  (`MeanReversionPairsStrategy` — fourth `Strategy` impl alongside
  v0 `sma_crossover`, v0.5 `ComposedStrategy`, v1 `MomentumStrategy`);
  new `audit::query::pnl_by_pair` compose helper; new backtest
  scenarios `pairs-2023-zscore-mr` + `pairs-2024-h1-zscore-mr`.
  No `Strategy` trait change, no `strategy_events` schema change,
  no `risk::size_portfolio_target` shape change, no chart-of-
  accounts addition (v1.5a's 4-symbol universe is a subset of v1's
  10).
- 2026-04-29 (architect): resolved the six v1-cross-sectional-momentum
  open questions from
  [features/v1-cross-sectional-momentum.md](features/v1-cross-sectional-momentum.md#notes--open-questions-for-architect).
  **Q1 L2 ingest** deferred to v1.5 — keeps v1 shippable; momentum score
  is close-to-close, depth has no consumer. **Q2 funding-rate ingest**
  observation-only at v1: hourly REST poller + new `funding_rates`
  SQLite table + `funding_obs` broadcast channel + new
  `trading_core::FundingObs` type; `MomentumStrategy` does not consume
  it (validates the ingest path for v1.5 without expanding hot-path
  cost). **Q3 long-only** confirmed: `K_long=3`, `K_short=0` for v1;
  loader rejects `k_short > 0` with `unsupported_short_sizing` error
  code; perp-shorting waits for v2. **Q4 multi-venue** deferred to v1.5:
  v1 stays Binance-only; the universe-ladder `+Kraken` entry is re-read
  as a v1-series goal. **Q5 universe filtering** is strategy-side
  (pattern A) — strategies filter `Strategy::on_bar` internally; no
  trait change. Pattern B (registry-side via a new `fn universe()`
  trait method) deferred to v2+ if a tick-latency strategy ever
  stresses the budget. **Q6 `RebalanceRejected` ledger surface** —
  extend the v0.5 `strategy_events` table with a new
  `kind = "rebalance_rejected"` variant; no schema migration; new
  `audit::journal::rebalance_rejected` writer + the existing
  `strategy_history` reader. **v1 architectural deltas:** new
  `crates/strategy/src/cross_sectional/` module (`MomentumStrategy`,
  score, selector); vector-order shape in `risk` (`size_portfolio_target`
  alongside the existing scalar `size_and_validate`);
  `RiskLimits.portfolio_exposure_cap: Option<Decimal>` field added;
  `audit::query::pnl_by_symbol` reader + extended chart of accounts
  (the existing 13-account chart is additive — `assets:position:<asset>`
  is parameterized; v1 universe symbols seed nine new sub-accounts at
  startup, no migration); multi-symbol `ReplayFeed` interleave with
  `(venue_ts ASC, symbol ASC)` deterministic sort; `funding_obs`
  broadcast channel added to `agent::EventBus`. No change to the
  v0 `Strategy` trait shape, no change to the v0.5 audit/broadcast
  surfaces beyond the additive items above.
- 2026-05-01 (architect): added **v1+ — Operator success reports
  resolutions (Q1–Q9)** subsection (under "Strategy registry &
  hot-loading" alongside the v1 / v1.5a resolutions) and added
  `crates/reports/` to the workspace layout map. **Q1 crate
  placement** confirmed: dedicated `crates/reports/` lib + bin
  (`cargo run --bin report -- --period <duration>`); deps
  `trading_core` + `audit` (read-only) + `data` (parquet) +
  `cost` (`CostBudget::remaining`). **Q2 `pnl_by_strategy`
  query** lives in `audit::query`; new additive migration
  `004_journal_transactions_strategy_id.sql` adds nullable
  `strategy_id TEXT` column on `journal_transactions`;
  `audit::journal::post_fill` signature gains
  `Option<&str>`. Pre-migration NULL rows surface as
  `(unattributed)`. Mark-to-market for unrealized P&L lives in
  `crates/reports/` (parquet), NOT in `audit::query`. **Q3
  atomic write**: tempfile + `rename` (same as v0 backtest
  binary). **Q4 sparkline**: Unicode-block `▁▂▃▄▅▆▇█` (8-level,
  60-char default width). **Q5 CSV** companion artifacts; six
  canonical files with documented columns. **Q6 reconciliation
  tolerance**: exact-cent `Decimal == Decimal`; on FAIL writes
  sibling `_reconciliation_failure.json` and exits 1. **Q7
  front-matter**: 12 fields including new
  `binary_version` / `git_commit` / `agent_pid` / `host` /
  `reconciliation`. **Q8 kill-switch trip provenance**: new
  `StrategyEventKind::KillSwitchTripped` variant; v0
  `kill_switch_tripped` writer rewritten to dual-write the
  v0 memo journal row PLUS a `strategy_events` row. v0 memo
  rows preserved (no retro-rewrite). **Q9 R6 placeholder
  re-lock plan** documented at task T811 — when reflection-memory
  ships, the new operator-success-report anchors get re-locked
  the same way v1.5a T717 re-locked the top10 momentum anchors.
  **v1+ architectural deltas**: new `crates/reports/` workspace
  member; new `StrategyEventKind` variants `KillSwitchTripped`
  + `FeedReconnect`; two additive audit migrations (`004_…
  strategy_id.sql`, `005_uptime_intervals.sql`); rewritten
  `kill_switch_tripped` (Q8 dual-write); new audit writers
  (`feed_reconnect`, `open_uptime_interval`, `heartbeat_uptime`,
  `close_uptime_interval`); new audit readers
  (`pnl_by_strategy`, `ledger_snapshot_sha`,
  `ledger_inception_ts`, `uptime_intervals_since`); agent
  boots/heartbeats/shutdowns now write to `agent_uptime`;
  agent's `KillSwitch::trip` writes the new strategy event +
  spawns the reports binary out-of-process. No new bus channels.
  No change to `crates/strategy/`, `crates/risk/`, `crates/exec/`
  (call-site update only), `crates/models/`, `crates/llm/`,
  `crates/features/`, `crates/ui/`. The 9 v0/v0.5/v1/v1.5a
  backtest anchor SHA-256s remain non-negotiable post-v1+ (V6
  regression gate).
- 2026-05-01 (architect): documented the runtime crate-dependency
  edge `crates/data → crates/audit` introduced by Wave 1 / T805
  (Binance reconnect handler calling
  `audit::journal::feed_reconnect`). Added the edge to the **Data
  flow** mermaid diagram, plus a new **Crate dependency edges
  (runtime, non-test)** subsection enumerating every sibling-crate
  dep with its single-purpose justification (`exec → audit`,
  `agent → audit`, `reports → {core, audit, data, cost}`,
  `ui → {core, audit}`). Reaffirmed the architectural rule "audit
  is a sink — zero outgoing runtime deps". No code change; flagged
  in Wave 1 and Wave 2 tester reports as undocumented; now closed.
- 2026-05-01 (architect): reconciled the v1+ operator-success-reports
  CSV column schemas in `spec/operator-success-reports/feature.md`
  to match the Wave 2c shipped renderer
  (`crates/reports/src/csv_artifacts.rs`, 134 tests green). Picked
  Option A (code is canonical): equity files emit
  `equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt`
  (realized + unrealized + cash decomposition) rather than the
  spec's prior `equity_usdt,cash_usdt,positions_value_usdt`
  (cash + positions_value decomposition). Operator question "how
  much of my P&L is real?" beats "how much is in cash vs
  marked-to-market positions". Also dropped the `_utc` suffix from
  `ts` columns across `equity-*.csv`, `fills.csv`, `journal.csv`,
  `strategy_events.csv` to match the writer headers; the UTC
  contract remains in the introductory paragraph and the writer
  doc-comments. No anchor risk (CSV companions are not in the 9
  locked anchor SHAs). Wave 2d (T816 anchor capture) proceeds
  against the renderer's actual byte output.
- 2026-05-01 (architect): resolved the eight live-cockpit-unified
  open questions from
  [features/live-cockpit-unified.md → Open questions for architect](features/live-cockpit-unified.md#open-questions-for-architect).
  **Q1** new bin `cockpit_live` at `crates/ui/src/bin/cockpit_live.rs`;
  extract `pub async fn agent::runtime::run(RunHandles, CancellationToken)`
  shared by both the headless `trading` bin and the unified bin
  (overrode analyst's `trading-cockpit` name in favor of
  `cockpit_live` for prefix-parity with `cockpit`). **Q2** iced on
  main thread, multi-thread tokio runtime hosted on a side
  `std::thread::spawn`; `Arc<EventBus>` + `Arc<KillSwitch>` +
  `tokio_util::sync::CancellationToken` shared via clone (matches
  analyst default; macOS GUI-on-main respected). **Q3** iced-led
  shutdown — single `CancellationToken`, 2 s wall-clock bound on
  the side-thread join, force-abort on timeout. **Q4** single
  `config/agent.toml` + new
  `[observability].prometheus_enabled: bool` (`#[serde(default =
  "default_true")]`); no `[cockpit]` section in v1. **Q5**
  `in_process_cron` opt-in unchanged; the new bin re-exports the
  feature gate via `[features] in_process_cron =
  ["agent/in_process_cron"]`. **Q6** single shared
  `Arc<KillSwitch>` — cockpit's `Message::KillConfirmed` calls
  `kill_switch.trip(HaltReason::ManualOperator)` via a closure
  capturing the side-thread tokio Handle; T809 dual-write
  preserved by sticky-trip semantics
  (`tripped.swap(true, SeqCst)`). **Q7** retire `cockpit
  --features live` (its only behavior was an empty-bus stub);
  keep `trading` headless and `cockpit --features fixtures`; add
  `compile_error!` deprecation shim on the old combo. **Q8** zero
  new UI surface; one tooltip-string edit on the kill button.
  **Bus-wiring scope: in-scope** — analyst's finding #1 (only the
  strategy watcher publishes today; `Arc<EventBus>` constructed at
  `crates/agent/src/main.rs:193` is not threaded through
  data/exec/risk producers) is closed by tasks T903a (paper
  engine publishes `fills` + `positions` via a new
  `exec::publisher::FillPublisher` trait), T903b (data feed `tap`
  tasks publish `bars` + `ticks`), T903c (reconciler publishes
  `pnl`), T905 (mode-broadcast forwarder bridges
  `KillSwitch::subscribe()` → `bus.publish_mode`). Without those
  wires R1 ("single binary that runs both") is structurally
  false. **Analyst finding #2** (cockpit `Message::KillConfirmed`
  only mutates `KillState::Flattening`, never calls
  `KillSwitch::trip`) confirmed by reading
  `crates/ui/src/state.rs:397–402`; closed by Q6 + T906.
  **Architectural deltas:** new public API
  `agent::runtime::run` + `agent::runtime::RunHandles` +
  `agent::runtime::shutdown_writer`; new
  `exec::publisher::FillPublisher` trait (keeps `exec → agent`
  cycle open by abstracting the bus type); new bin
  `cockpit_live` at `crates/ui/src/bin/cockpit_live.rs`
  (`required-features = ["live"]`); new field
  `agent::config::ObservabilityConfig::prometheus_enabled: bool`;
  new field `ui::state::Cockpit::kill_switch:
  Option<Arc<dyn Fn(HaltReason) + Send + Sync>>` under
  `cfg(feature = "live")`. **Deprecation:** `cockpit --features
  live` retired (compile_error! shim with migration message).
  **Edge-graph delta:** `ui → agent` becomes load-bearing under
  `--features live` (was previously cosmetic — only consumed by
  the empty-bus stub). No new workspace member; no new system C
  dep; `tokio_util::sync::CancellationToken` already in
  `Cargo.lock`; `assert_cmd` added as dev-dep for the V3 / V9
  subprocess-launch tests in T910 / T912. No anchor risk —
  `spec/anchors.toml`'s 11 entries cover backtest report
  rendering, none cover `agent` or `ui` (R15 + V5).
- 2026-05-02 (architect): resolved the eight
  real-mtm-unrealized-pnl open questions from
  [features/real-mtm-unrealized-pnl.md → Open questions for architect](features/real-mtm-unrealized-pnl.md#open-questions-for-architect).
  **Q1** snapshot vec
  `audit::query::open_positions_at(ledger, ts) ->
  Result<Vec<OpenPosition>, LedgerError>`. **Q2**
  `OpenPosition` lives in `trading_core` (new
  `crates/core/src/position.rs`) for cross-crate visibility.
  **Q3** NO new SQL index in this feature (full-table scan
  fits the 100 ms V8 budget); conditional follow-up
  migration `006_open_positions_index.sql` only if V8 fails.
  **R10** (`post_fill` BTC hardcode at
  `crates/audit/src/journal.rs:82,135`) explicitly
  **DEFERRED** to a follow-up brief
  `spec/per-symbol-position-accounts/feature.md` —
  description-parse path already gives correct per-symbol
  semantics (verified against `build_ledger_90d.rs` 4-symbol
  fixture). **Q4** anchors stay byte-identical — both v1+
  fixtures (`build_ledger_7d.rs`, `build_ledger_90d.rs`) lay
  6+12 perfectly symmetric (Buy, Sell) pairs; net qty == 0 at
  `period_end`; `unrealized = 0`; bodies byte-identical to
  today. All 11 anchor SHAs in `spec/anchors.toml` unchanged.
  **Q5** add a third **non-anchored** test fixture
  `build_ledger_with_open_positions_7d.rs` for V1/V2/V7/V8.
  **Q6** mark-source miss on `MarkError::OutOfRange` →
  `tracing::warn!` + zero contribution + deterministic body
  footnote on the R11.1 reconciliation row IF any miss
  (architect override of analyst's
  surface-as-front-matter `warnings:` recommendation —
  determinism rationale: front-matter path would make
  `unrealized` arithmetic depend on parquet-root health,
  breaking byte-identical re-runs). **Q7** weighted-average
  cost basis with proportional release on each Sell;
  per-unit `Money<Usdt>` on `OpenPosition.avg_cost_basis`.
  **Q8** long-only; net-negative qty raises
  `LedgerError::Database`; real shorts deferred to v2+.
  **real-mtm-unrealized-pnl architectural deltas:**
  additive `trading_core::OpenPosition` struct; additive
  `audit::query::open_positions_at` reader; orchestrator
  diff at `crates/reports/src/lib.rs:135–150`; new test-only
  fixture; new test files for V1/V2/V4/V6/V7/V8. No new
  external dep; workspace edition 2021 unchanged; library
  compatibility checklist N/A. Anchor budget unchanged
  (11 / 11 byte-identical).
- 2026-05-02 (ui-designer): added "Frontend ↔ backend interfaces"
  subsection under `### Frontend — iced`. Formalizes the seven
  load-bearing surfaces between `crates/ui/` and the rest of the
  workspace: (1) `Arc<EventBus>` broadcast — 10 channels with
  per-channel sender, type, capacity, backpressure policy
  (`Lagged` warns + continues, `Closed` emits typed panel-error
  variant); (2) `audit::query` read-only API — 15 read paths the
  cockpit may call, hard constraint that the cockpit MUST NOT call
  audit writers; (3) `KillTripFn` closure — sole operator → backend
  write surface, calls `KillSwitch::trip(HaltReason::ManualOperator)`
  on the side-thread tokio runtime captured in `cockpit_live::main`;
  (4) `spec/reports/**/*.md` — viewer's offline read path plus
  file-naming convention (`backtest-` / `success-` / `test-` /
  `dev-` / `ui-debt-` / `ui-week*-smoke-` prefixes) and reaffirmed
  body-vs-front-matter discipline; (5) theme tokens are the only
  legal color/spacing/type source; (6) strings module is the only
  legal copy source; (7) fixtures provide the dev-mode data path
  for `cargo run --bin cockpit --features fixtures`. No code change;
  documents the existing contract. Companion living doc
  [ui-design-principles.md](../ui-design-principles.md) lands the
  design-system rules these interfaces dress (color palette
  extensions, type/spacing scale lock, density tables, motion
  timings, trading-specific patterns, eight open questions for
  operator).
- 2026-05-03 (architect): resolved the eight
  per-symbol-position-accounts open questions from
  [features/per-symbol-position-accounts.md → Open questions for architect](features/per-symbol-position-accounts.md#open-questions-for-architect).
  **Q1** purely additive migration
  `006_per_symbol_position_accounts.sql` (10 `INSERT OR IGNORE`
  lines, one per pair-symbol in
  `config/agent.toml:62-65 [funding].universe`). **Q2** account-id
  format `assets:position:<SYMBOL>` (full Binance pair, e.g.
  `assets:position:BTCUSDT`); strategy stays in T802 column.
  **Q3** NO backfill — purely additive; legacy
  `assets:position:BTC` rows untouched. **Q4** description-parse
  stays primary in `open_positions_at` / `pnl_by_symbol` /
  `recent_fills`; account-id is a defensive cross-check (warn-only,
  no return-value branch). **Q5** `extract_symbol_from_description`
  retained indefinitely; doc-comment notes new code SHOULD use the
  typed readers instead. **Q6** EXTEND
  `build_ledger_with_open_positions_7d.rs` (override of analyst's
  recommendation b — the existing fixture is non-anchored, so
  extension is anchor-safe). **Q7** anchor risk zero by independent
  re-grep; 11 / 11 byte-identical. **Q8 (corrected)**
  `bootstrap::seed_universe_accounts` has a SHAPE MISMATCH (takes
  base assets like `"BTC"`, not pair symbols like `"BTCUSDT"`); it
  cannot be reused. Mark `#[deprecated]` in T1103; deletion is a
  separable follow-up. The migration is the canonical seed
  (`Ledger::open` runs migrations on every binary boot, so no
  Rust-side defensive seed is needed).
  **per-symbol-position-accounts architectural deltas:**
  new migration `006_per_symbol_position_accounts.sql`; line-edit
  inside `audit::journal::post_fill` body (signature unchanged —
  T802's `(ledger, fill, strategy_id)` byte-identical); defensive
  cross-check + doc-comment in `audit::query`'s description-parse
  path; `#[deprecated]` attribute on `seed_universe_accounts`
  (zero callers — silent in normal builds). No new public API
  surface, no new types, no new dep, no `Cargo.toml` change, no
  `unsafe`. The migration list table above reclaims the `006`
  slot from the conditional `006_open_positions_index.sql` that
  never landed (real-mtm V8 PASSED at 0.287ms). Anchor budget
  unchanged (11 / 11 byte-identical). Tasks T1101–T1107 +
  `T_FINAL_PER_SYMBOL` filed at
  [tasks/per-symbol-position-accounts.md](tasks/per-symbol-position-accounts.md).
- 2026-05-03 (architect): resolved the nine
  tape-row-audit-modal open questions from
  [features/tape-row-audit-modal.md → Open questions for architect](features/tape-row-audit-modal.md#open-questions-for-architect).
  **Q1** `iced::widget::Stack` overlay (no new dep — verified
  `iced 0.14.0` ships Stack via `Cargo.lock` `iced_widget = "0.14.2"`);
  Stack's bottom child is the existing cockpit `Column`, top
  child is a full-bleed `bg_overlay` `Container` capturing
  backdrop clicks → `Message::TapeAuditModalClosed`. **Q2** new
  `pub struct JournalEntry { account, debit, credit, currency,
  ts, memo }` in `crates/core/src/views.rs` — additive; the
  existing `JournalEntryView` (signed-amount collapse) stays
  unchanged for its consumers (`recent_journal`, etc.).
  **Q3** land all three theme tokens in this feature:
  `bg_overlay = #0B0D12`, `info = #7BC2FF`,
  `border_strong = #3A4456` (dark-mode hex from
  [ui-design-principles.md](../ui-design-principles.md)). Light-mode
  hex documented but landed by the broader light-mode feature.
  **Q4** column order `Account | Debit | Credit | Currency`;
  numbers right-aligned, monospace digits, locale-default
  thousands separator (per principles "Numbers are scannable").
  **Q5** `FillView` gains additive field `transaction_id: SmolStr`;
  `Fill` gains additive field `transaction_id: Option<SmolStr>`;
  `audit::journal::post_fill` return type bumped from
  `Result<(), LedgerError>` to `Result<SmolStr, LedgerError>`
  (returns the generated `journal_transactions.id`); the live
  runtime in `crates/agent/src/runtime.rs` stamps
  `fill.transaction_id` from the audit return value before
  `engine.on_fill` fans out on the bus; backtests construct
  `PaperEnginePublisher` with `NullPublisher` so the
  `transaction_id` stamp never fires on the backtest path
  (anchor-safe). **Q6** modal-open-gated `iced::keyboard::on_key_press`
  subscription absorbs `Esc` / `Tab` / arrows / Page-Up / Page-Down
  while the modal is open; subscription is removed on close
  (no leak across cycles). **Q7** specific
  `widgets::journal_transaction_modal` widget (new file); generic
  modal refactor deferred per principles three-uses rule
  (positions-drilldown + strategy-events-drilldown will trigger
  it). **Q8** three new test files: `audit/tests/journal_entries_for_transaction.rs`
  (V11), `ui/tests/tape_row_click_opens_modal.rs` (V1/V3/V4/V5),
  `ui/tests/snapshots/panel_snapshots__tape_audit_modal_ready_paper_fill.snap`
  (V8 / V2). Existing `panel_snapshots__tape_*` stay byte-identical
  (R11 + V7) — `tape_summary` does not inspect `transaction_id`.
  **Q9** modal closes on `Message::AgentHaltedExternally`;
  one modal at a time (`TapeRowClicked` while open replaces
  identity); clipboard `Cmd-C` deferred. **First feature against
  [ui-design-principles.md](../ui-design-principles.md)** — documents
  the click-through-to-audit modal pattern that future drilldowns
  inherit (`Stack` overlay + `bg_overlay` backdrop +
  `border_strong` frame + Esc-to-close subscription +
  `Message::*Clicked(id)` / `*ModalClosed` / `*EntriesLoaded`
  message triplet + per-feature `widgets::*_modal.rs`).
  **tape-row-audit-modal architectural deltas:** new
  `trading_core::JournalEntry` view struct (separate from existing
  `JournalEntryView`); additive field
  `trading_core::FillView::transaction_id: SmolStr`; additive
  field `trading_core::Fill::transaction_id: Option<SmolStr>`;
  new reader `audit::query::journal_entries_for_transaction`;
  return-type change on `audit::journal::post_fill` (signature
  becomes `Result<SmolStr, LedgerError>`); three additive theme
  tokens in `crates/ui/src/theme.rs` (semantic-colors namespace
  grows from 9 to 12); modal pattern precedent
  `iced::widget::Stack` documented for future drilldowns; new
  widget file `crates/ui/src/widgets/journal_transaction_modal.rs`;
  three new `Message` variants (`TapeRowClicked`,
  `TapeAuditModalClosed`, `TapeAuditEntriesLoaded`); new
  `JournalModalState` + `JournalTransactionView` view types in
  `crates/ui/src/state.rs`. No new external dep, no system C
  dep, no `unsafe`, no migration. Anchor budget unchanged
  (11 / 11 byte-identical) — the `FillView::transaction_id`
  field is not rendered into any anchored report body
  (`crates/reports/src/` consumes aggregate cells; backtests
  construct the publisher with `NullPublisher` so the live-mode
  stamp never fires on backtest paths). Tasks T1201–T1209 +
  `T_FINAL_TAPE_MODAL` filed at
  [tasks/tape-row-audit-modal.md](tasks/tape-row-audit-modal.md).
- 2026-05-03 (architect): resolved the six
  journal-transactions-metadata open questions from
  [features/journal-transactions-metadata.md → Open questions for architect](features/journal-transactions-metadata.md#open-questions-for-architect).
  Follow-up to the T1206 deviation note in tape-row-audit-modal:
  the live-mode modal currently renders `description: ""` and
  `strategy_id: None` because the cockpit_live `Task::perform`
  closure constructs a partial `JournalTransactionView` until a
  metadata reader lands. This feature is that reader. **Q1** new
  `pub struct JournalTransactionMetadata { transaction_id: SmolStr,
  ts: Timestamp, description: SmolStr, strategy_id: Option<StrategyId> }`
  in [`crates/core/src/views.rs`](../../crates/core/src/views.rs)
  alongside `JournalEntry` (T1201); re-exported from
  `crates/core/src/lib.rs:48`. **Principled override** on the
  brief default: `description: SmolStr` (not `String`) — symmetry
  with `JournalTransactionView.description: SmolStr` and
  `JournalEntry.memo: SmolStr`; typical paper-fill descriptions
  fit inline-storage. **Q2** two separate readers per T1202's
  "one reader, one job" pattern — no fused
  `(Metadata, Vec<JournalEntry>)` reader; cockpit_live closure
  sequences both. **Q3** four fields; omit the schema's
  `metadata: TEXT NOT NULL DEFAULT '{}'` JSON blob (no consumer,
  three-uses rule applied). **Q4** sequential `await` (NOT
  `tokio::join!`); metadata-`None` short-circuit skips the
  entries query on stale clicks. **Q5** override of brief default
  — re-verify T1207's existing 4 modal snapshots stay byte-identical
  (`JournalModalState` doesn't carry provenance, so a duplicate
  populated-metadata snapshot would be byte-identical noise) +
  add ONE new wiring smoke test
  `crates/ui/tests/cockpit_live_modal_metadata_chain.rs` (NEW)
  driving the chained-fetch path. **Q6** any-`Err` collapses to
  `PanelState::Error(TAPE_AUDIT_MODAL_ERROR_PREFIX + msg)`;
  metadata-`None` → "unknown transaction" error. Consistent with
  today's modal error UX; no new strings.
  **journal-transactions-metadata architectural deltas:** new
  `trading_core::JournalTransactionMetadata` view struct (separate
  from `JournalEntry` and `JournalEntryView`); new reader
  `audit::query::journal_transaction_metadata` (sibling of
  `journal_entries_for_transaction`, T1202 reader is unchanged
  per R7); cockpit_live `Task::perform` closure at
  `crates/ui/src/bin/cockpit_live.rs:496-535` replaces partial-view
  construction with a sequential metadata→entries chain plus Q6
  error mapping; one new audit unit-test file
  `crates/audit/tests/journal_transaction_metadata.rs` (V1 + V2);
  one new ui smoke-test file
  `crates/ui/tests/cockpit_live_modal_metadata_chain.rs` (V3).
  No new external dep, no system C dep, no `unsafe`, no migration,
  no `Cargo.toml` change, no new theme tokens, no new strings, no
  new widget files, no new `Message` variants. Public API the
  cockpit may call grows by one row in the
  [Cockpit ← `audit::query`](#cockpit--auditquery) table
  (`journal_transaction_metadata`). Anchor budget unchanged
  (11 / 11 byte-identical) — the new reader is not on any anchored
  path; backtests use `PaperEnginePublisher` with `NullPublisher`
  and the rendering pipeline in `crates/reports/src/` consumes
  aggregate cells, never `JournalTransactionMetadata`. Tasks
  T1301–T1305 + `T_FINAL_TX_METADATA` filed at
  [tasks/journal-transactions-metadata.md](tasks/journal-transactions-metadata.md).
- 2026-05-03 (architect): resolved the twelve v1.5b-multi-venue
  open questions from
  [features/v1-5b-multi-venue.md → Open questions for architect](features/v1-5b-multi-venue.md#open-questions-for-architect).
  Largest queued backend feature. **Q1** new closed enum
  `trading_core::Venue { Binance, Coinbase, Kraken }` with
  `#[serde(rename_all = "snake_case")]` in
  `crates/core/src/venue.rs` (re-exported at crate root); no
  `Default` impl — every Bar / Tick must construct it. **Q2**
  Coinbase Advanced Trade WS
  (`wss://advanced-trade-ws.coinbase.com`) over the legacy Pro
  WS. **Q3** per-venue `tokio::task::JoinSet` topology
  (`agent::runtime::run` spawns one task per enabled venue);
  `select_all` rejected for panic-poison risk. **Q4** required
  `venue: Venue` field on `Tick` / `Bar` (mechanical migration
  across ~30+ literal sites — every existing literal defaults
  `Venue::Binance`); `Option<Venue>` rejected for forever-bug-
  surface. **Q5** client-side 1s aggregation in new
  `crates/data/src/bar_aggregator.rs` (deterministic on `i64`
  epoch-µs bucketing); new `Timeframe::OneSecond` variant
  (Display `"1s"`). **Q6** doubled USDC universe with
  operator-gated `[universe]` section
  (`usdt_enabled = true` default, `usdc_enabled = false`
  default; legacy `[funding].universe` stays as back-compat
  reader path). **Q7** per-venue stale-data pause @ 30s default
  + new `MarketHealth { Fresh, Stale, Recovered }` enum +
  `EventBus::market_health: broadcast::Sender<MarketHealth>`
  channel (capacity 64) + per-venue watchdog
  `crates/agent/src/stale_watchdog.rs`. **Q8** free
  unauthenticated WS for all three venues confirmed (Binance
  / Coinbase / Kraken). **Q9** worst-case 60 subscription slots
  fits within all three venues' free tiers with ≥10× margin
  on the tightest limit (Coinbase 750 msg/s/IP). **Q10**
  `MockFeed` test harness (`crates/data/src/mock_feed.rs`,
  gated under `cfg(any(test, feature = "fixtures"))`) over
  `wiremock` — covers V1–V7; WS-frame parsing unit-tested
  directly at the per-venue `parse_*_event` private function
  level. **Q11 — principled override of analyst R8.2** —
  schema migration `007_strategy_events_venue.sql` (NULLABLE
  `venue TEXT` column on `strategy_events`) + writer signature
  change `feed_reconnect(ledger, symbol, venue: Venue, ts)`;
  `error_summary`-encoding rejected because v1.5b is the
  load-bearing introduction of `Venue` and audit is the
  boundary where typed attribution matters most. **Q12**
  zero anchor risk re-confirmed by independent grep on
  `spec/*/reports/backtest-*.md` + `spec/operator-success-reports/reports/success-*.md`
  (zero hits on `venue|coinbase|kraken`); hard architectural
  rule: any future renderer change that introduces venue
  strings into a committed report body requires an architect-
  approved re-lock budget. **No new external dep** —
  Coinbase + Kraken adapters reuse `tokio_tungstenite` +
  `serde_json` + `reqwest` (identical to today's `BinanceFeed`).
  No `Cargo.toml` change. No `unsafe`. **Anchor budget
  unchanged (11 / 11 byte-identical).** Migration `007` added
  to the Audit migration list. T612 (multi-symbol live
  `BinanceFeed` fan-out) finally lands as part of v1.5b via
  new `subscribe_bars_multi` / `subscribe_trades_multi` methods
  on `BinanceFeed` using the combined-stream URL; single-symbol
  API unchanged (R10.3). Tasks T1401–T1415 + `T_FINAL_V15B`
  filed at
  [tasks/v1-5b-multi-venue.md](tasks/v1-5b-multi-venue.md).
  T1401 is the sole sequential foundation gate (~30+ mechanical
  fixture-site migrations); ~7 parallel paths fan out after it
  (T1402 ‖ T1403 ‖ T1404 ‖ T1405 ‖ T1406 ‖ T1407 ‖ T1410),
  converging at T1408 (runtime topology) and T1409 (bus
  channel + watchdog). Test wave T1411–T1414 fans out again;
  T1415 sequential at end.
- 2026-05-04 (architect): lumen-design-adoption Phase 1 foundation
  resolutions landing. Q1–Q9 + master Q10 ratified per the analyst
  brief at
  [features/lumen-phase-1-foundation.md](features/lumen-phase-1-foundation.md).
  New section "Lumen design adoption — Phase 1 foundation resolutions"
  documents: token-system rewrite (12 → ~50 tokens; full Lumen
  palette + tier system + shadow ladder + focus ring + spacing 13-step
  + radii 6-step + typography 7-step + motion ladder; flat
  `theme::color::*` SHOUTY_SNAKE_CASE per Q10); Tier 0/1/2/3 + Sunken
  surface specification; active-row pattern (2 px transparent-default
  rule); iced 0.14 `Shadow` API confirmed first-class via
  `iced_core-0.14.0/src/shadow.rs`; new `widgets::status_bar`
  consumer of existing `bus.market_health()` (additive — no producer
  change); split vocabulary (Q8b — connection field "Connected /
  Reconnecting / Disconnected", latency badge keeps "OK / Slow / High
  / Halted"); single-file principles-doc supersede (~480 lines, T1510);
  dark default at boot (Q6 — light values wired but toggle is
  downstream); kill-switch behaviour preserved (Q9 — visual chrome
  only). No new dep — iced 0.14.0 Shadow already supported; `sysinfo`
  for status-bar CPU% deferred. Anchor risk zero by construction
  (`crates/strategy/audit/exec/backtest/reports/` untouched). Cross-
  feature invariants for the 7 prior shipped features documented
  preserved. Tasks `T1501–T1514 + T_FINAL_LUMEN_PHASE_1` filed at
  [tasks/lumen-phase-1-foundation.md](tasks/lumen-phase-1-foundation.md);
  T1501 is the foundation gate (theme rewrite); after T1502 (call-
  site sweep), six dev tasks fan out (T1503–T1508) + spec-only T1510;
  T1509 (status bar shell wiring) and T1511 (one-time 36-snapshot
  refresh) are the narrow points. Phase 2 (viewer Backtest panel) and
  Phase 3 (HumanControl + AgentFeed rename) remain queued; Phase 4
  (Assistant slot) reserved for v2 LLM strategy.
- 2026-05-04 (architect): Q11 mid-phase deviation ratified — iced
  0.14.2 `button::Status` has no `Focused` variant and
  `text_input::Style` has no `shadow` field, so T1504's true
  keyboard-focus-ring acceptance is unachievable under the shipped
  framework. **Option A** ratified: Phase 1 ships hover-state ring on
  buttons + ACCENT border-shift on focused inputs as a bounded best-
  effort approximation; T1504 tick stands as honest under the
  documented iced 0.14.2 API gap. Reasoning: kill-switch destructive
  intent is carried by the typed-confirm `KILL_SAFETY_PHRASE`, not
  the focus halo (operator-impact bounded); Phase 1 "Foundation"
  scope tolerates documented gaps over multi-day custom-widget
  spikes (Option B rejected); same shape as Q3's `shadow_inset`
  outer-only API workaround (architect-consistent). Option C
  (rewriting the acceptance criterion) rejected — preserves the
  original intent as the Phase-N target rather than erasing it.
  Phase-N follow-up filed in
  [features/lumen-design-adoption.md](features/lumen-design-adoption.md)
  under "Cross-phase technical-debt items"; upgrade triggers are
  (a) iced version bump exposing `button::Status::Focused` +
  `text_input::Style.shadow` (likely 0.15+, unverified at this
  ratification), or (b) project-local
  `iced::widget::Component` custom widget owning focus state via
  keyboard subscription. Anchor risk zero (UI-only); cross-feature
  invariant table unchanged. Documented in `crates/ui/src/widgets/kill.rs`
  module-level doc + T1504/T1506 honest-tick rows at
  [tasks/lumen-phase-1-foundation.md](tasks/lumen-phase-1-foundation.md).
- 2026-05-10 (architect): chart-buy-sell-emphasis v1.9 resolutions
  landed — six [ARCHITECT-DECIDE] questions Q1 / Q2 / Q3 / Q6 / Q7
  / Q9 resolved. **Q1 = (a)** additive `strategy_signals` table
  (migration 009), new `journal::post_strategy_signal` writer
  pair, new `audit::query::recent_signals` reader, new
  `SignalLogConfig { enabled: false }` agent-config section.
  Strategy trait fixed; no new bus channel; atomic-write contract
  reused via existing journal-writer pattern. **Q2 = (b)** linear
  interpolation y-snap. **Q3 = (b)** custom canvas
  `Program::update` + custom-drawn tooltip overlay; iced
  `tooltip` is the documented fallback. **Q6** — 13-px triangle
  + `BORDER_STRONG` outline + `shadow_1`-derived whisper shadow
  for fills; 8-px 60%-opacity `UP_400 / DOWN_400` for ghosts.
  **No new theme tokens.** **Q7 = (b)** new
  `crates/ui/src/widgets/volume_histogram.rs` (sibling of
  `chart.rs`). **Q9** — `SignalView` in
  `crates/core/src/views.rs` (sibling of `FillView`), shape =
  analyst strawman + `signal_id: SmolStr`. Operator-resolved
  Q4 / Q5 / Q8 recorded for trace. Anchor risk zero (R9.4 / V8
  hard gate). Tasks T2001–T2027 + T_FINAL_CHART_BUY_SELL_EMPHASIS
  filed at [chart-buy-sell-emphasis/tasks.md](../v1/chart-buy-sell-emphasis/tasks.md);
  8 new files + 12 modified files; `crates/strategy/`,
  `crates/risk/`, `crates/backtest/`, `crates/reports/`,
  `crates/exec/` untouched. UI-heavy feature; ui-designer + developer
  run in parallel per AGENT.md.
