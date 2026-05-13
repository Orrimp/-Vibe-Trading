---
slug: architecture-04-risk-and-money
status: shipped
owner: architect
updated: 2026-05-13
---

# Risk engine and money math

The pre-trade risk surface, kill-switch contract, and the cross-cutting
money-math invariant that every consumer of the audit ledger relies on.

## Risk engine

- Hard limits encoded in Rust types — an order that violates them cannot
  be constructed, so the violation is a compile error rather than a
  runtime check.
- Kill switch file (`.halt`) + heartbeat. Trip routes:
  - File watch (`fsnotify` on `.halt`).
  - Operator-pressed cockpit button (via `Message::KillConfirmed` →
    `KillSwitch::trip(HaltReason::ManualOperator)`).
  - Heartbeat timeout (no bus events in N seconds).
- Daily P&L stop, per-symbol exposure cap, max drawdown stop. All
  computed against the audit ledger (single source of truth — no
  parallel in-memory accounting).

The sticky-trip semantics in `KillSwitch::trip` and T809 dual-write
discipline are documented in
[01-data-flow.md § Public API surface — bin-shared agent runtime](01-data-flow.md#public-api-surface--bin-shared-agent-runtime-live-cockpit-unified).

## Money math

This rule is project-wide, not specific to the risk engine, but the risk
engine is where the consequences bite hardest: an `f64` rounding error in
a position-size calculation breaks the P&L stop's "did we hit the limit?"
test in non-obvious ways.

**Rule.** Money math uses `rust_decimal::Decimal` wrapped in a
`Money<C: Currency>` newtype. No `f64`, no `f32`, no `u64` cents. Currency
conversion is an explicit operation; cross-currency arithmetic that
doesn't go through the conversion API is a compile error.

See [ADR-0003](adr/0003-decimal-money-math.md) for the full context, the
two reconciliation incidents that forced the rule, the alternatives
considered, and the mechanical enforcement points.

## Changelog
- 2026-05-13 (architect): content migrated from
  `spec/architecture.md` § Risk engine during Phase 1A Session 3. Money
  math section added as a pointer to ADR-0003 because risk and money are
  conceptually linked even though the ADR is cross-cutting.
