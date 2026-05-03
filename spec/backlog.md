---
slug: backlog
status: living
owner: orchestrator
updated: 2026-05-03
---

# Backlog

Queued ideas the operator has surfaced but hasn't promoted to a
feature brief yet. One line each + a note on cost or blockers. This
file is editable churn — nothing here is a commitment.

Promote an item to real work by spawning the **analyst**, who turns it
into a `spec/features/<slug>.md` brief and removes the entry here.

## Active

- **Live cockpit unified binary** — promoted 2026-05-01 to
  [`spec/features/live-cockpit-unified.md`](features/live-cockpit-unified.md).
  Status: `draft`, owner: analyst → architect. 15 R-items, 10 V-items,
  8 Open Questions for the architect.
- **Real mark-to-market unrealized P&L** — promoted 2026-05-02 to
  [`spec/features/real-mtm-unrealized-pnl.md`](features/real-mtm-unrealized-pnl.md).
  Status: `draft → architect-pending`, owner: analyst → architect.
  10 R-items, 8 V-items, 8 Open Questions for the architect.
  Anchor risk: preferred byte-identical outcome (existing fixtures
  are fully symmetric, zero open positions at `period_end`); fallback
  re-lock per v1.5a T717 precedent if Q5 resolution extends a fixture.
- **R10 follow-up: per-symbol-position-accounts** — promoted 2026-05-02
  to
  [`spec/features/per-symbol-position-accounts.md`](features/per-symbol-position-accounts.md).
  Status: `draft`, owner: analyst → architect. 11 R-items, 8 V-items,
  8 Open Questions for the architect. Plumbing-only:
  chart-of-accounts migration `006_*.sql` + `post_fill` writer change
  (BTC hardcode → `format!("assets:position:{}", fill.symbol)`) +
  optional `open_positions_at` reader optimization. Anchor risk:
  preferred byte-identical (9 backtest + 2 v1+); architect must
  confirm migration is purely additive at the chart-of-accounts level
  (Q3, Q7) so report bodies stay byte-identical. Originated as the
  R10 deferral note in
  [`real-mtm-unrealized-pnl.md` Design § Q3 / R10 verdict, lines
  386–401, 541–554](features/real-mtm-unrealized-pnl.md).
- **Tape-row → audit modal** — promoted 2026-05-03 to
  [`spec/features/tape-row-audit-modal.md`](features/tape-row-audit-modal.md).
  Status: `draft`, owner: analyst → architect. 15 R-items, 11
  V-items, 9 Open Questions for the architect. Pure UI + new audit
  reader (`journal_entries_for_transaction`) + first-time consumer
  of the proposed `bg_overlay` / `info` / `border_strong` theme
  tokens (Q3) and `FillView::transaction_id` plumbing (Q5). Anchor
  risk: zero — UI feature, no backtest path touched (R12,
  preferred 11/11 PASS byte-identical). First feature to land
  against [ui-design-principles.md](ui-design-principles.md) — the
  "Show the why" cockpit click-through-to-audit path begins here.
- **Journal-transactions metadata reader** — promoted 2026-05-03 to
  [`spec/features/journal-transactions-metadata.md`](features/journal-transactions-metadata.md).
  Status: `draft`, owner: analyst → architect. 7 R-items, 5 V-items,
  6 Open Questions for the architect. Closes the T1206 deviation
  note from `tape-row-audit-modal`: live cockpit's modal header
  currently renders empty `description` + `None` strategy_id
  because T1202's reader is intentionally narrow (entries-only).
  This feature adds a sibling `journal_transaction_metadata(tx_id)`
  reader (header projection — id / ts / description / strategy_id)
  + a new `core::JournalTransactionMetadata` struct, then chains
  it into `cockpit_live`'s `Task::perform`. Anchor risk: zero —
  additive read-only, no write path, no rendering path, no
  anchored code path consumed.
- **v1.5b multi-venue + 1s aggregated trades** — promoted 2026-05-03
  to
  [`spec/features/v1-5b-multi-venue.md`](features/v1-5b-multi-venue.md).
  Status: `draft`, owner: analyst → architect. 15 R-items, 12
  V-items, 12 Open Questions for the architect. **Largest queued
  backend feature.** Coinbase + Kraken adapters, USDC pairs (10-
  symbol mirror set), T612 multi-symbol live `BinanceFeed`
  (deferred at v1 closeout), 1-second aggregated trades
  (`Timeframe::OneSecond`), per-venue `MarketDataSource` impls
  (`BinanceFeed`, `CoinbaseFeed`, `KrakenFeed`), `Venue` enum on
  `Tick` / `Bar`, per-venue feed-reconnect provenance (T805
  extension). Plumbing-only — no new strategy, no new edge claim,
  data-side expansion only. Anchor risk: zero by construction
  (architect must independently grep `spec/reports/**/*.md` to
  confirm zero `venue` strings — R11.2 / Q12). Cost risk: zero —
  all three venues have free public market-data WS APIs (R9 /
  Q8). Failover risk: medium — multi-venue means N independent
  failure modes; per-venue tokio tasks + isolation is the
  architect's call (Q3 / R14). Closes the v1.5a Q5 USDC blocker
  ([architecture.md → v1.5a Q5, lines 1130–1158](architecture.md#v15a-q5--usdc-pairs-blocked-on-v15b-multi-venue))
  and the v1 T612 deferral
  ([v1-cross-sectional-momentum.md → T612 status, lines
  1516–1521](features/v1-cross-sectional-momentum.md#t612-status)).

## Queue

### Strategy

- **Reflection memory (v1.5a Q1 follow-up).** Replaces the R6 placeholder
  in `crates/reports/src/render/memory_highlights.rs`. Will re-lock the
  two `report-sample-*` anchors — precedent at v1.5a T717. Wants fresh
  analyst spawn for memory shape + summarization budget.
- **v2 LLM strategy.** Analyst for prompt design, model choice, and a
  hard cost budget (currently $0/mo). Architect for the LLM-trait
  shape + caching. Likely the biggest scope of any queued item.

### Process / tooling

- **Presenter smoke test against `operator-success-reports`.** Exercises
  every new presenter skill (`present-results`, `verify-anchors`,
  `capture-screenshot` fallback) end-to-end on the just-shipped
  feature. Cheap validation before committing the new agent definition
  to a real new feature. Recommended as a low-risk first fire.

## Conventions

- One-line description; deeper context lives in the eventual
  `spec/features/<slug>.md` brief.
- The orchestrator owns this file; agents may suggest additions but
  the operator approves promotions.
- Items can stay here indefinitely. Stale items get a `_decayed_` tag
  rather than silent deletion so the orchestrator can revisit.

## Changelog

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
