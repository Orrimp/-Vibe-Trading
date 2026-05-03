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

## Queue

### Strategy

- **v1.5b multi-venue + 1s aggregated trades.** Adds Coinbase + Kraken
  + USDC pairs + finishes T612 (multi-symbol live BinanceFeed).
  Largest queued backend feature; needs analyst spawn for venue
  prioritization + ingest topology.
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
