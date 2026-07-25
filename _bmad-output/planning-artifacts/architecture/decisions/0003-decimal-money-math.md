---
adr: 0003
title: Money math uses Decimal and a Money<C> newtype, never f64
status: accepted
date: 2026-04-17
supersedes: none
superseded-by: none
---

# ADR-0003: Money math uses Decimal and a Money<C> newtype, never f64

## Context

Floating-point money math fails open: `0.1 + 0.2 != 0.3` is the textbook
case, but the project-specific pain came from P&L reconciliation. When
fills are aggregated across many trades and currencies, f64 rounding
errors compound silently and produce non-zero "ghost" cash balances at
the end of a backtest. The audit-DB constraint that debits == credits to
the cent failed twice on f64-based aggregation before the rule landed.

Money also has a currency dimension: BTC, USD, USDT, USDC, EUR all coexist
in the same simulation. Adding two `Decimal` values of different
currencies is always a bug, not a design choice. A naked `Decimal` lets
this bug compile.

## Decision

Money math uses `rust_decimal::Decimal` for the numeric value, wrapped in
a `Money<C: Currency>` newtype that pins the currency at the type level.
No `f64`, no `f32`, no `u64` cents, no `String` representations, anywhere
in `crates/audit`, `crates/exec`, `crates/risk`, `crates/strategy` (for
prices, quantities, fees, P&L, balances). Reconciliation rules use
exact-cent equality, never tolerance.

Currency conversion is an explicit operation (`Money<USD>::convert_via`)
that takes an FX rate and produces a new `Money<C>`. No implicit
arithmetic between different `C`s.

## Alternatives considered

- **`u64` minor units (cents, satoshis).** Works for fixed-precision
  pairs but breaks on cross-currency P&L (which BTC unit?) and on fees
  that have higher precision than the venue minor unit (Binance maker
  rebates at 0.075% of notional). Rejected.
- **`bigdecimal` crate.** Arbitrary precision but slower and lacks the
  `serde` + `sqlx` ergonomics of `rust_decimal`. Rejected.
- **Naked `Decimal` without currency phantom.** Allows accidentally
  adding USD + BTC. Rejected — the type system catches this; rejecting
  it at runtime is strictly worse.

## Consequences

- Mechanical enforcement: developer's determinism checklist forbids `f64`
  in any money path. `cargo clippy` rule? Not yet — currently relies on
  review + the tester's anchor gate (an `f64` rounding bug would
  immediately diff the report body).
- Audit-DB columns for amounts use `TEXT NOT NULL` storing the decimal
  string representation (`sqlx`'s `Decimal` ↔ `TEXT` mapping). Migration
  rule: NEVER store money as `REAL` / `DOUBLE`.
- Currency conversion is logged to the audit ledger as a separate event
  type so the reconciliation pass can verify it.
- Violations to watch for: any `.to_f64()` or `as f64` near a money
  value; any `f64::from_str` on a price string; any cross-currency `+`
  / `-` that fails to compile (good — means the type system is doing
  its job).

## Changelog
- 2026-04-17 (architect): initial accept. Cross-cutting invariant.
  Extracted to ADR during Phase 1A split (2026-05-13).
