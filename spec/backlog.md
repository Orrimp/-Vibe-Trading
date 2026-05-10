---
slug: backlog
status: living
owner: orchestrator
updated: 2026-05-04
---
<!-- updated 2026-05-04 (analyst, post-Phase-1-ship roadmap revision) —
     `lumen-phase-1-foundation` shipped (tester third-pass PASS) → moved
     Active → Recent. Lumen master roadmap revised from 4 phases to 6
     at operator request: new Phase 2 Shell IA + Charts and Phase 3
     Detail screens insert ahead of original phases. Old Phase 2
     (viewer backtest) → Phase 4. Old Phase 3 (HumanControl + AgentFeed)
     → Phase 5. Old Phase 4 (Assistant slot) → Phase 6 (still reserved
     for v2 LLM). Five new feature brief stubs queued. UI / cockpit
     queue subsection rewritten for the 6-phase plan. -->
<!-- updated 2026-05-03 (analyst) — `v1.5b multi-venue` moved Active →
     Recent (shipped 2026-05-03). New `lumen-design-adoption` master
     roadmap + `lumen-phase-1-foundation` first-phase brief promoted to
     Active. `lumen-phase-2-viewer-backtest` and
     `lumen-phase-3-human-control-agent-feed` queued.
     `lumen-phase-4-assistant` reserved for v2 LLM. Three operator-locked
     constraints (no brand adoption, no voice rewrite, sequential
     phasing) documented in the master roadmap. -->


# Backlog

Queued ideas the operator has surfaced but hasn't promoted to a
feature brief yet. One line each + a note on cost or blockers. This
file is editable churn — nothing here is a commitment.

Promote an item to real work by spawning the **analyst**, who turns it
into a `spec/<slug>/feature.md` brief and removes the entry here.

## Active

- **v2 LLM strategy** — **PAUSED 2026-05-10 at architect → developer
  handoff.** Operator paused with *"Write it down for now. I will
  come to this point a while later."* Resumption breadcrumb at
  [`spec/v2-llm-strategy/orchestrator-scope-check-2026-05-10.md`](v2-llm-strategy/orchestrator-scope-check-2026-05-10.md)
  — read that file FIRST when resuming; it holds the three
  resumption-time decisions (Q4 bonus rename keep/defer, Q8 strict-
  vs-best-effort replay, Q11 denominator bundle/defer) and the
  recommended next move. Analyst draft + operator Q-resolutions
  (Q1=A foundation-only, Q2=A Anthropic, Q3=C config-file,
  Q10=strawman) + architect Design + 45 dev tasks T1901–T1945 are
  all committed. **Pending: developer pass.** Brief at
  [`spec/v2-llm-strategy/feature.md`](v2-llm-strategy/feature.md)
  (status: in-progress, owner: architect). **Unblocks Lumen Phase 6**
  (Assistant slot reserved until this ships).

## Queue

### Strategy

### UI / cockpit (Lumen design-system adoption — Phase 6 reserved)

- **Lumen Phase 6 — Assistant slot.** _reserved_ — depends on the
  v2 LLM strategy queued item above. Right-rail collapsible panel
  for the v2 LLM assistant per
  [`spec/design/project/ui_kits/desktop/Assistant.jsx`](design/project/ui_kits/desktop/Assistant.jsx).
  Phase 2 reserved the right-rail column-track in the shell grid
  at `Length::Fixed(0.0)`; the actual Phase 6 brief lands when v2
  LLM is approved. Until then, no analyst spawn. Stub at
  [`features/lumen-phase-6-assistant-slot.md`](features/lumen-phase-6-assistant-slot.md).
  _(Phases 1–5 of the lumen-design-adoption initiative are shipped
  and live in the Recent section; this Queue entry is the only
  remaining initiative work, gated on v2 LLM.)_

### Process / tooling

_(empty — the only queued item, the presenter smoke test against
operator-success-reports, ran 2026-05-08; surfaced 4 findings, two
of which became skill-plumbing fixes that shipped in commit
`8b139c2`. See Recent below.)_

## Recent (shipped)

- **Reflection memory (v1.8.0)** — shipped 2026-05-10 (operator
  verbal approval recorded as `[x] Approve with notes` in the
  presenter deck at
  [`spec/reflection-memory/presentations/reflection-memory-2026-05-08.md`](reflection-memory/presentations/reflection-memory-2026-05-08.md);
  one note: flip `ReflectionConfig::enable_writer` default from
  `false` to `true` — applied in the same commit as approval).
  Replaces the R6 placeholder body in
  [`crates/reports/src/render/memory_highlights.rs`](../crates/reports/src/render/memory_highlights.rs)
  with real reflection-memory output. New leaf crate
  [`crates/reflection/`](../crates/reflection/) (lib only — types,
  regime + outcome classifiers, deterministic 32-dim embedding,
  post-mortem-analyst card generator, `ReflectionStore` trait + a
  `SqliteReflectionStore` linear-scan top-K impl, bounded mpsc
  writer task with Prometheus drop counter, retrieval API). Wired
  through `crates/agent/src/{config,main}.rs` + `crates/exec/src/paper.rs`.
  Re-locked the two `report-sample-*` anchors at
  `spec/anchors.toml:67-75`; the 9 strategy-backtest anchors at
  lines 15–58 are byte-identical (negative-invariant test t1812
  enforces). Q-resolutions: Q1 = Option A (deterministic v1, no
  LLM dependency); Q4 = report-only (Strategy trait unchanged);
  Q5 = distillation deferred to follow-up brief
  `reflection-memory-distillation`. Tester report:
  [`spec/reflection-memory/reports/test-2026-05-08-2114-reflection-memory-final.md`](reflection-memory/reports/test-2026-05-08-2114-reflection-memory-final.md)
  (V1–V10 all PASS; 952 / 0 / 3 tests across 124 binaries; 11/11
  anchors PASS; cargo deny advisories/bans/licenses/sources all
  ok). Brief: [`spec/reflection-memory/feature.md`](reflection-memory/feature.md)
  (status: shipped, version: 1.8.0).
- **Presenter smoke test on `operator-success-reports`** — shipped
  2026-05-08 (operator verbal approval recorded as `[x] Approved —
  ship` in commit `587dad7`). Deck at
  [`spec/operator-success-reports/presentations/operator-success-reports-2026-05-08.md`](operator-success-reports/presentations/operator-success-reports-2026-05-08.md);
  pulled evidence from the archived final tester PASS (extracted
  from `spec/archive/pre-lumen-tester-reports-2026-04-to-05-03.tar.gz`)
  + a fresh `cargo test -p reports --test report_scenarios` re-run
  (4/4 PASS, body SHAs match anchors) + a fresh
  `scripts/verify_anchors.sh` PASS (11/11). Surfaced 4 smoke-test
  findings: (1) `present-results` skill missed the archive
  fallback for pre-Lumen tester reports — fixed in `8b139c2`;
  (2) `capture-screenshot` skill defaulted to a manual-capture
  instruction for non-UI features — fixed in `8b139c2` with a third
  "non-UI feature" branch; (3) backlog Recent section had stale
  relative paths inside link parens (cosmetic, fixed in `1a63156`);
  (4) confirmed the audit-immutability call on archived tester
  reports is correct (their internal `spec/features/...` /
  `spec/tasks/...` references describe the layout at time of
  writing). The presenter pipeline is now battle-tested before the
  next real-feature fire.

- **Lumen Phase 5 — HumanControl + AgentFeed rename** — shipped
  2026-05-07 (tester second-pass PASS, presenter approved
  2026-05-08). Tester reports at
  [`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07-lumen-phase-5-humancontrol-agentfeed.md`](lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07-lumen-phase-5-humancontrol-agentfeed.md)
  (first-pass FAIL on fmt drift) and
  [`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md`](lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md)
  (second-pass PASS); brief at
  [`features/lumen-phase-5-humancontrol-agentfeed.md`](features/lumen-phase-5-humancontrol-agentfeed.md)
  (status: `shipped`); presenter deck at
  [`spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md).
  **First phase to ship net-new operator-write surfaces since v0**:
  HumanControl panel widget (execution-mode segmented control +
  daily loss limit / max position / used-today P&L mirror rows +
  kill button as bottom action) on a new "Control" sidebar entry
  (7th); single-click pause-strategy toggle on Strategies-detail;
  typed-confirm `OVERRIDE` flow for risk-veto override per surfaced
  veto event. Two new audit writers (`strategy_paused`,
  `risk_veto_overridden`) — additive `StrategyEventKind` variants
  with no SQL migration (column already TEXT). Module rename
  `tape` → `agent_feed` via `mv` + git rename detection;
  `Cockpit::tape` field name preserved (Q14) to avoid 100+ test
  ripple. **TD-1 four-phase deferral CLOSED** via Path (b) —
  `crates/ui/src/widgets/focus_ring.rs` Subscription-driven
  custom-widget escape hatch wraps all four destructive surfaces
  with a visible accent-bordered halo on focus. New TD-2 row
  added: risk-engine veto-emit upstream wiring deferred (Phase 5
  ships override surface over an empty live `Vec<VetoEvent>`;
  not a safety primary, an observability gap). Anchor risk: zero
  — verified PASS at ship (11/11 byte-identical post additive
  audit-writer additions). 896 tests passed across 110 binaries
  (46 + 2 net-new vs Phase 4); rust-validate clean (after one-line
  `cargo fmt --all` fixup between tester passes); 86 baselines
  attested by ui-designer (67 panel + 17 widget + 2 audit; 13
  net-new + 9 renamed); R16.3 brand-bleed grep clean. Architect
  ratified 15/15 Q-items with zero principled overrides.
  **Phase 5 is the last shippable phase of the lumen-design-adoption
  initiative absent v2 LLM** — Phase 6 (Assistant slot) is reserved
  until the v2 LLM strategy lands.

- **Lumen Phase 4 — Backtest panel (`viewer` bin)** — shipped
  2026-05-06 (tester second-pass PASS, presenter approved
  2026-05-06). Tester reports at
  [`spec/lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06-lumen-phase-4-backtest-panel.md`](lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06-lumen-phase-4-backtest-panel.md)
  (first-pass FAIL on `clippy::match_same_arms`) and
  [`spec/lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md`](lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md)
  (second-pass PASS); brief at
  [`features/lumen-phase-4-backtest-panel.md`](features/lumen-phase-4-backtest-panel.md)
  (status: `shipped`); presenter deck at
  [`spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md).
  Adds the new `viewer` binary at `crates/ui/src/bin/viewer.rs`
  (workspace now ships 3 bins), KPI strip + equity curve + drawdown
  band widgets sharing a refactored `widgets::canvas_chart` core
  with Phase 2's price chart, the cross-phase `core::EquitySeries`
  primitive (rich struct with precomputed drawdown vector inside
  `EquityPoint`), additive `audit::query::equity_curve_for_strategy(strategy_id, since, until)`
  sibling of Phase 2/3 filtered queries, markdown summary parser
  with graceful "—" fallback for missing fields (the 11 anchored
  sample reports omit CAGR + Win rate by design), and **closes the
  Phase 3 deferral** (Strategies-detail sparkline placeholder
  retires; real `widgets::sparkline` lands fed by the new audit
  query). Anchor risk: zero — verified PASS at ship (11/11 byte-
  identical, viewer reads existing committed reports). 850 tests
  passed across 108 binaries (40 + 4 net-new vs Phase 3); rust-
  validate clean (after orchestrator's one-line `match_same_arms`
  fix between tester passes); 72 baselines attested by ui-designer;
  R16.3 brand-bleed grep clean. Architect ratified 12/12 Q-items
  with zero principled overrides (Q1 shape refinement: drawdown_pct
  nested inside `EquityPoint` rather than parallel Vec — eliminates
  length-coupling). **Phase 5 inherits the TD-1 tightening point**:
  Phase 5 ships net-new operator-write controls with typed-confirm
  flows, making the focus-ring deferral (iced still pins `=0.14.0`)
  load-bearing — Phase 5 either folds the iced 0.15+ upgrade or
  commits to the custom-widget escape hatch.

- **Lumen Phase 3 — Detail screens (Strategies / Risk / Audit)** —
  shipped 2026-05-05 (tester first-pass PASS, presenter approved
  2026-05-06). Tester report at
  [`spec/lumen-design-adoption/phase-3-detail-screens/reports/test-2026-05-05-lumen-phase-3-detail-screens.md`](lumen-design-adoption/phase-3-detail-screens/reports/test-2026-05-05-lumen-phase-3-detail-screens.md);
  brief at
  [`features/lumen-phase-3-detail-screens.md`](features/lumen-phase-3-detail-screens.md)
  (status: `shipped`); presenter deck at
  [`spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md).
  Adds Strategies / Risk / Audit sidebar entries (3 → 6) + per-screen
  detail bodies, additive `008_journal_transactions_venue.sql`
  migration (default `'binance'` backfill), `post_fill` writer's
  new `venue: Venue` parameter wired across ~25 call-sites,
  `RiskTelemetry` channel on `agent::EventBus` mirroring `MarketHealth`,
  sibling audit query `recent_journal_filtered` (additive to
  `recent_fills_filtered`), kill-threshold proximity gauge with
  tri-band ramp (`UP_500` ≤70% → `WARN_500` >70% → `DOWN_500` >90%),
  Audit screen filter chip-row (venue · symbol · kind · time-range)
  + fixed 250-row pagination + reuse of T1208 modal, cross-link
  Home → Strategies-detail. Two developer passes (pass 1 cut at
  clean tick boundary after T1701 + T1703 due to context budget;
  pass 2 ticked T1702 migration + T1704–T1716). Architect ratified
  11/11 Q-items with one deferral (Q6 equity-since-deploy sparkline
  → Phase 4, since the cheap path doesn't exist on the current
  state shape and Phase 4 needs the same equity-history primitive).
  Anchor risk: zero — verified PASS at ship (11/11 byte-identical
  post-migration, verified twice during dev pass + once at tester
  gate). 810 tests passed across 104 binaries (29 + 6 net-new vs
  Phase 2); rust-validate clean (fmt + clippy `-D warnings` +
  cargo-deny + docs); 65 baselines attested by ui-designer
  (zero `unknown` token escapes); R16.3 brand-bleed grep clean.

- **Lumen Phase 2 — Shell IA + Charts** — shipped 2026-05-05.
  Tester first-pass `VERDICT → PASS`; report at
  [`spec/lumen-design-adoption/phase-2-shell-ia-charts/reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md`](lumen-design-adoption/phase-2-shell-ia-charts/reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md).
  Brief at
  [`features/lumen-phase-2-shell-ia-charts.md`](features/lumen-phase-2-shell-ia-charts.md)
  (status: `shipped`). Presenter deck approved by operator at
  [`spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md).
  Adds left-sidebar shell (fixed 180 px, T1507-styled, no icons),
  Screen routing (`Cockpit::current_screen` × six variants — Home /
  Debug / Charts wired in Phase 2; Strategies / Risk / Audit
  declared for Phase 3), Home screen (Phase 1 widgets re-housed),
  Debug screen (kill / latency / market health / server time /
  version / placeholder logs), Charts screen (chip-row symbol
  selector + canvas line-series price plot + buy/sell triangle
  markers from `recent_fills_filtered`), per-`(Venue, Symbol)`
  rolling `ChartBuffer` (cap 60 1-min bars), live mode via existing
  `bars_tx`, fixtures mode via deterministic `synthetic_candles`,
  additive `audit::query::recent_fills_filtered(venue, symbol,
  since, until)`, right-rail Phase 6 Assistant slot reservation
  (`Length::Fixed(0.0)`). Anchor risk: zero — verified PASS at
  ship (11/11 anchors byte-identical). 781 tests passed across 98
  binaries (24 + 2 net-new vs Phase 1); rust-validate clean
  (fmt + clippy `-D warnings` + cargo-deny + docs); 53 baselines
  attested by ui-designer (zero `unknown` token escapes); R16.3
  brand-bleed grep clean. All 11 architect Q-resolutions ratified
  with zero deviations from analyst recommendation. **Phase 3
  prerequisite carried forward**: additive `journal_transactions.venue`
  column migration needed before non-Binance fills can populate
  the chart's marker query.

- **Lumen Phase 1 — Foundation (tokens + tiers + status bar)** —
  shipped 2026-05-04. Tester third-pass `VERDICT → PASS`; report at
  [`spec/lumen-design-adoption/phase-1-foundation/reports/test-2026-05-04c-lumen-phase-1-foundation.md`](lumen-design-adoption/phase-1-foundation/reports/test-2026-05-04c-lumen-phase-1-foundation.md).
  Brief at
  [`features/lumen-phase-1-foundation.md`](features/lumen-phase-1-foundation.md)
  (status: `shipped`). Replaced the 12-token theme with the full
  Lumen palette (warm + cool neutrals, accent ramp, sage / clay /
  warn / info semantics, both light and dark modes); added Tier
  0/1/2/3 elevation surface tokens; added whisper shadows + sunken
  inset; added focus ring (Q11 / TD-1 deviation: hover-state ring
  on buttons + ACCENT border-shift on focused inputs as bounded
  approximation, two named upgrade triggers); extended spacing /
  radii / typography ladders to the full Lumen scale; added motion
  tokens; applied Tier 1 styling to existing 6 widgets; applied
  sunken styling to the kill-confirm input; applied the active-row
  2 px left rule to tabular widgets; added a new
  `widgets::status_bar` widget rendering connection / latency /
  account / server time always-visible at the bottom of the shell;
  refreshed the existing 36 panel snapshot baselines (5 net-new for
  T1506 / T1508 = 41 total); superseded
  [`spec/ui-design-principles.md`](ui-design-principles.md) with a
  Lumen-anchored rewrite. Anchor risk: zero — verified PASS at
  ship (11/11 anchors byte-identical). 757 tests passed across 96
  binaries; rust-validate clean (fmt + clippy + cargo-deny + docs);
  R16.3 brand-bleed grep clean. Unblocked the 2026-05-04 master
  roadmap revision (4 → 6 phases).
- **v1.5b multi-venue + 1s aggregated trades** — shipped 2026-05-03.
  Brief at
  [`features/v1-5b-multi-venue.md`](features/v1-5b-multi-venue.md).
  Coinbase + Kraken adapters, USDC pair mirror set (10 symbols),
  T612 multi-symbol live `BinanceFeed`, 1 s aggregated trades,
  `Venue` enum on `Tick` / `Bar`, per-venue feed-reconnect
  provenance. Plumbing-only — expanded the data side, not the
  execution side. 15 R-items, 12 V-items, 12 open questions
  resolved. Anchor risk: zero by construction (no `venue` strings
  in any committed report body). Closes v1.5a Q5 (USDC pairs
  blocker) and v1 closeout T612. The cockpit / `cockpit_live` is
  now stable on the data side; this is the clean window the
  Lumen design adoption initiative lands into.

## Conventions

- One-line description; deeper context lives in the eventual
  `spec/<slug>/feature.md` brief.
- The orchestrator owns this file; agents may suggest additions but
  the operator approves promotions.
- Items can stay here indefinitely. Stale items get a `_decayed_` tag
  rather than silent deletion so the orchestrator can revisit.

## Changelog

- 2026-05-08 (orchestrator, post-Phase-5 cleanup): drained the
  Active section. The 5 prior Active entries (`live-cockpit-unified`,
  `real-mtm-unrealized-pnl`, `per-symbol-position-accounts`,
  `tape-row-audit-modal`, `journal-transactions-metadata`) all
  shipped in v1+ → operator-success-reports cycle and are referenced
  as shipped invariants in the Lumen master roadmap's cross-feature
  invariants table; never moved out of Active. Plus the
  `lumen-design-adoption` master-roadmap row dated 2026-05-03 still
  referenced the obsolete 4-phase plan. All six Active entries
  removed (organizational hygiene; their feature briefs remain at
  `spec/<slug>/feature.md`). UI / cockpit Queue subsection
  collapsed to just Phase 6 (the only remaining initiative work).
  Initiative status: 5-of-6 phases shipped; reaches a natural pause
  point absent v2 LLM. No new feature promoted.
- 2026-05-08 (presenter, Phase 5 sprint review APPROVED): operator
  signed the Phase 5 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
  (`[x] Approved — ship`). Phase 5 closed cleanly: tester second-
  pass PASS (8/8 gates after a one-line `cargo fmt --all` fixup
  between passes; first-pass FAIL preserved on disk for audit), 896
  tests across 110 binaries, 11/11 anchors byte-identical, 86
  baselines attested clean, **TD-1 four-phase deferral CLOSED** via
  Path (b) custom-widget escape hatch, new TD-2 row added for the
  deferred risk-engine veto-emit upstream wiring. **Phase 5 is the
  last shippable phase of the lumen-design-adoption initiative
  absent v2 LLM** — Phase 6 (Assistant slot) is reserved.
  Initiative status: 5-of-6 phases shipped; Phase 6 unlocks when
  v2 LLM is approved. No new active feature promoted from this
  approval; next-step decision is operator's (promote v2 LLM,
  promote a different Active backlog item, or pause the cockpit-
  side initiative as 5-phase-complete).
- 2026-05-06 (presenter, Phase 4 sprint review APPROVED): operator
  signed the Phase 4 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
  (`[x] Approved — ship`). Phase 4 closed cleanly: tester second-
  pass PASS (8/8 gates after a one-line `match_same_arms` fix
  between passes; first-pass FAIL preserved on disk for audit), 850
  tests across 108 binaries, 11/11 anchors byte-identical, 72
  baselines attested clean, three-bin workspace (`viewer` greenfield
  + `cockpit` + `cockpit_live`). Promoted `lumen-phase-5-humancontrol-agentfeed`
  from Queue → Active per the master-roadmap sequencing constraint
  (Constraint 3). Phase 5 brief stub frontmatter bumped from
  `queued` → `active`; the analyst's next pass expands the stub
  into a full brief. **Phase 5 is the first phase to introduce
  net-new operator-write paths** (pause-strategy, override-risk-
  veto, execution-mode toggle), making it the load-bearing decision
  point for the TD-1 focus-ring deferral. Mechanical pre-tick gate
  at sprint-review time PASSED before approval. HANDOFF → analyst
  (Phase 5 brief expansion).
- 2026-05-06 (presenter, Phase 3 sprint review APPROVED): operator
  signed the Phase 3 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
  (`[x] Approved — ship`). Phase 3 closed cleanly: tester first-pass
  PASS (8/8 gates), 810 tests across 104 binaries, 11/11 anchors
  byte-identical post-migration, 65 baselines attested clean, two
  developer passes (pass 1 cut at clean tick boundary; pass 2
  finished T1702 migration + remaining 14 tasks). Promoted
  `lumen-phase-4-backtest-panel` from Queue → Active per the
  master-roadmap sequencing constraint (Constraint 3). Phase 4
  brief stub frontmatter bumped from `queued` → `active`; the
  analyst's next pass expands the stub into a full brief with
  R-items / V-items / Q-items + task list contract. **Phase 4 will
  absorb the deferred Q6 sparkline from Phase 3** — both surfaces
  need the same equity-history primitive. Mechanical pre-tick gate
  at sprint-review time PASSED before approval (`PRESENTATION
  CHECK PASS`). HANDOFF → analyst (Phase 4 brief expansion).
- 2026-05-05 (presenter, Phase 2 sprint review APPROVED): operator
  signed the Phase 2 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
  (`[x] Approved — ship`). Phase 2 closed cleanly: tester first-pass
  PASS (8/8 gates), 781 tests across 98 binaries, 11/11 anchors,
  53 baselines attested clean. Promoted `lumen-phase-3-detail-screens`
  from Queue → Active per the master-roadmap sequencing constraint
  (Constraint 3). Phase 3 brief stub frontmatter bumped from `queued`
  → `active`; the analyst's next pass expands the stub into a full
  brief with R-items / V-items / Q-items + task list contract.
  Mechanical pre-tick gate at sprint-review time PASSED before
  approval (`PRESENTATION CHECK PASS`). HANDOFF → analyst (Phase 3
  brief expansion).
- 2026-05-04 (presenter, Phase 1 sprint review APPROVED): operator
  signed the Phase 1 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
  (`[x] Approved — ship`). Phase 1 closed cleanly. Promoted
  `lumen-phase-2-shell-ia-charts` from Queue → Active per the
  master-roadmap sequencing constraint (Constraint 3). Phase 2
  brief stub frontmatter bumped from `queued` → `active`; the
  analyst's next pass expands the stub into a full brief.
  Mechanical pre-tick gate at sprint-review time PASSED before
  approval (`PRESENTATION CHECK PASS`). HANDOFF → analyst (Phase 2
  brief expansion).
- 2026-05-04 (analyst, post-Phase-1-ship roadmap revision):
  `lumen-phase-1-foundation` shipped → moved Active → Recent.
  Lumen master roadmap revised from 4 to 6 phases at operator
  request (session of 2026-05-04, after Phase 1 third-pass tester
  PASS). Two new phases inserted ahead of the original phase plan:
  Phase 2 Shell IA + Charts (sidebar nav + Home / Debug / Charts
  screens + price chart with buy/sell markers + read-only audit
  query extension), Phase 3 Detail screens (Strategies / Risk /
  Audit sidebar entries over existing backend data). Original
  Phase 2 (Backtest panel) renumbered to Phase 4; original Phase 3
  (HumanControl + AgentFeed) renumbered to Phase 5; original Phase
  4 (Assistant slot) renumbered to Phase 6 (still reserved for v2
  LLM). Five new feature-brief stubs spawned. Operator decisions
  captured as Q11–Q14 in the master open-questions section
  (sidebar fixed-width; chart in both bins; extend `audit::query`
  for marker filtering; keep Phase 2 / 3 split). Anchor risk per
  phase table extended to 6 rows; cross-feature invariants table
  extended to 6 phase columns. UI / cockpit Queue subsection
  rewritten for the 6-phase plan. HANDOFF → architect at Phase 2
  promote (when the operator signs Phase 1 presentation).
- 2026-05-01 (orchestrator): initial draft. Captures the 5 followups
  surfaced at `operator-success-reports` ship; promotes
  live-cockpit-unified to Active.
- 2026-05-01 (analyst): live-cockpit-unified Active line updated to
  reference the just-written
  [`features/live-cockpit-unified.md`](features/live-cockpit-unified.md)
  brief.
- 2026-05-02 (analyst): promoted "Real mark-to-market unrealized P&L"
  from Queue → Active. Brief at
  [`features/real-mtm-unrealized-pnl.md`](features/real-mtm-unrealized-pnl.md);
  HANDOFF → architect.
- 2026-05-02 (analyst): promoted "R10 follow-up:
  per-symbol-position-accounts" from the implicit Queue (deferral
  note in `real-mtm-unrealized-pnl.md` Design § Q3 / R10 verdict) →
  Active. Brief at
  [`features/per-symbol-position-accounts.md`](features/per-symbol-position-accounts.md);
  HANDOFF → architect.
- 2026-05-03 (orchestrator): new UI / cockpit subsection. Added
  `tape-row-audit-modal` per operator decision on UI principles Q4
  (2026-05-03). Promotes when operator picks it up; analyst → architect
  → developer pipeline standard.
- 2026-05-03 (analyst): promoted `tape-row-audit-modal` from Queue
  (UI / cockpit) → Active. Brief at
  [`features/tape-row-audit-modal.md`](features/tape-row-audit-modal.md).
  First feature to land against
  [`ui-design-principles.md`](ui-design-principles.md) (the "Show
  the why" cockpit click-through-to-audit path begins here). 15
  R-items, 11 V-items, 9 open questions for the architect. Anchor
  risk: zero (pure UI + new additive audit reader). HANDOFF →
  architect. The now-empty `### UI / cockpit` Queue subsection has
  been removed; future UI additions will recreate it.
- 2026-05-03 (analyst): added `journal-transactions-metadata` to
  Active. Brief at
  [`features/journal-transactions-metadata.md`](features/journal-transactions-metadata.md).
  Closes the T1206 deviation note in the just-shipped
  [`features/tape-row-audit-modal.md`](features/tape-row-audit-modal.md)
  (Implementation § Async dispatch, lines 625–635) — live cockpit's
  modal header is empty because T1202's reader stays narrow
  (entries-only). New sibling reader + `core::JournalTransactionMetadata`
  struct + cockpit_live `Task::perform` chain. 7 R-items, 5 V-items,
  6 open questions for the architect. Anchor risk: zero (additive
  read-only). HANDOFF → architect.
- 2026-05-03 (analyst): promoted `v1.5b multi-venue + 1s aggregated
  trades` from Queue (Strategy) → Active. Brief at
  [`features/v1-5b-multi-venue.md`](features/v1-5b-multi-venue.md).
  **Largest queued backend feature.** Coinbase + Kraken adapters,
  USDC pair mirror set (10 symbols), T612 multi-symbol live
  `BinanceFeed` (the v1 closeout deferral lands here), 1s
  aggregated trades, `Venue` enum on `Tick` / `Bar`, per-venue
  feed-reconnect provenance (T805 extension). Plumbing-only —
  expands the data side, not the execution side. 15 R-items,
  12 V-items, 12 open questions for the architect. Anchor risk:
  zero by construction (architect-confirmed grep of
  `spec/reports/**/*.md` for `venue`/`coinbase`/`kraken`).
  Cost risk: zero — all three venues have free public
  market-data WS APIs. Failover risk: medium — N independent
  failure modes; per-venue tokio tasks the recommended
  isolation strategy. Closes v1.5a Q5 (USDC pairs blocker) and
  v1 closeout T612. HANDOFF → architect.
- 2026-05-03 (analyst): `v1.5b multi-venue + 1s aggregated
  trades` shipped (verdict PASS, presenter approved). Moved
  Active → Recent. Promoted the
  [`lumen-design-adoption`](features/lumen-design-adoption.md)
  master roadmap + the
  [`lumen-phase-1-foundation`](features/lumen-phase-1-foundation.md)
  first-phase brief to Active. Master roadmap covers the 4-phase
  Lumen design-system adoption: Phase 1 Foundation (Active —
  tokens + tiers + status bar + principles supersede), Phase 2
  Viewer Backtest (Queue — KPI strip + equity curve + drawdown
  band on the offline review surface), Phase 3 HumanControl +
  AgentFeed rename (Queue — richer override controls + tape →
  AgentFeed module rename), Phase 4 Assistant slot (Reserved —
  depends on v2 LLM strategy ship). Operator-locked constraints
  inherited at every phase: NO brand adoption (no "Lumen" name,
  no eye/lens logo), NO voice rules rewrite (`ui::strings`
  unchanged), sequential phasing (one approval per phase), dark
  as default. Phase 1 brief carries 17 R-items, 9 V-items, 9
  open questions for the architect. Anchor risk per phase:
  zero (Phase 1 / 2 / 3 are UI features, no backtest path
  touched); Phase 4 out of scope. The 11 / 11 backtest body-SHA-256
  anchor regression goal stays byte-identical across the entire
  initiative. Cross-feature invariants documented for the 7 prior
  shipped UI-touching features (operator-success-reports,
  live-cockpit-unified, real-mtm, per-symbol, tape-modal,
  journal-tx-metadata, v1.5b). HANDOFF → architect (Phase 1
  first; master roadmap for orientation).
