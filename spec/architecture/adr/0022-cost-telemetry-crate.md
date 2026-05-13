---
adr: 0022
title: Cost telemetry lives in a dedicated `cost` crate
status: accepted
date: 2026-04-17
supersedes: none
superseded-by: none
---

# ADR-0022: Cost telemetry lives in a dedicated `cost` crate

## Context

LLM token costs, infrastructure spend, data-feed fees, and storage
costs all accrue against the operator's monthly ceiling (per
[`../product.md` § Cost economics](../../product.md#cost-economics--monthly-ceiling)).
v0 has no cost lines (no LLM calls; infra/data/storage shipped at the
free-tier). Each later release adds another cost dimension. The
question is where the telemetry surface lives so v0.5+ drop calls in
without moving code.

## Decision

A standalone `cost` crate owns cost measurement and budget
enforcement. In v0 it ships **empty** (no events emitted) but the
full surface is wired so v0.5+ adds calls without moving code or
refactoring crate boundaries.

Surface:

```rust
pub enum CostEvent {
    Llm {
        provider: ProviderKind,   // renamed from LlmProvider — see ADR-0019
        model: String,
        tier: LlmTier,            // deep_think | quick_think
        role: AgentRole,          // trader, sentiment_analyst, ...
        tokens_in: u64,
        tokens_out: u64,
        tokens_cached_in: u64,
        usd: Decimal,
        correlation_id: Uuid,
    },
    Infra  { line: InfraLine, usd: Decimal, period: Month },  // v1+
    Data   { feed: FeedId,    usd: Decimal, period: Month },  // v1+
    Storage{ bytes: u64,      usd: Decimal, period: Month },  // v1+
}

pub trait CostSink: Send + Sync {
    fn record(&self, event: CostEvent) -> Result<(), CostError>;
}

pub struct CostBudget { /* ceiling + spent; rollup queries audit ledger */ }
impl CostBudget {
    pub fn remaining(&self) -> Decimal;
    pub fn mode_override(&self) -> Option<LlmTier>;  // auto-degrade @ 80%
}
```

A default `LedgerCostSink` (lives in `cost`, depends on `audit`)
writes each `CostEvent` as a journal entry against
`expense:llm:<tier>` and an accrued `liabilities:llm_accrued` contra.
v0 posts zero entries; the accounts exist in the chart of accounts
(v0 R3.2).

`cost` depends on `core` + `audit`. `llm` (v2+) depends on `cost`.
The `BudgetedProvider<Inner>` decorator from
[ADR-0019](0019-v2-llm-strategy.md) Q6 uses `cost::CostBudget`.

## Alternatives considered

- **Keep cost telemetry under `llm`.** Cheap now, but forces an
  extraction once non-LLM cost lines appear (`infra`, `data`,
  `storage` already named in the cost ladder). Rejected.
- **Fold cost into `audit`.** Inverts the dependency direction;
  `audit` is a generic double-entry substrate and shouldn't know
  about OpenAI vs Anthropic. Rejected.
- **Skip the crate at v0 and add it when the first LLM call lands.**
  Same as the first alternative — extraction refactor later. Rejected
  in favour of empty-but-wired now.

## Consequences

- The `cost` crate is the canonical location for any future cost
  dimension. New dimensions (e.g. cloud-storage egress, venue
  withdrawal fees) extend `CostEvent`, not invent a parallel crate.
- The `BudgetedProvider<Inner>` decorator pattern from ADR-0019
  generalises beyond LLMs — any future cost-bearing wrapper can
  follow the same shape.
- `audit::query::pnl_by_strategy` etc. can join against `expense:*`
  accounts to attribute cost to strategy decisions in the operator
  success report ([ADR-0015](0015-operator-success-reports.md)).

## Changelog
- 2026-04-17 (architect): initial accept. Crate scaffolded empty at v0.
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  Cost telemetry during Phase 1A Session 11. `LlmProvider` enum
  reference updated to `ProviderKind` per
  [ADR-0019](0019-v2-llm-strategy.md) Q4 rename.
