---
adr: 0026
title: v0 ships a simple paper engine; full LOB deferred to v0.5
status: accepted
date: 2026-04-17
supersedes: none
superseded-by: none
---

# ADR-0026: v0 ships a simple paper engine; full LOB deferred to v0.5

## Context

A `MatchingEngine` trait in the `backtest` crate isolates the
fill-and-matching choice from the rest of the workspace. v0 ships
one strategy (SMA crossover) that emits only bar-close market orders
against 1m klines. A full limit-order-book engine has no inputs that
would exercise it at v0 and would consume the entire 2-week budget.

## Decision

v0 ships a simple `PaperEngine` in `backtest`, parameterised by:

- `slippage_bps` (default `2`) — buy fills at
  `bar.close * (1 + bps / 10_000)`, sell fills at
  `bar.close * (1 − bps / 10_000)`.
- `taker_fee_bps` (default `4`) — applied to notional, booked to
  `expense:fees:taker` per the audit-DB chart of accounts (see
  [ADR-0024](0024-audit-sqlite-raw-sqlx.md)).
- Optional bar-VWAP fill price (toggle) for sensitivity runs.
- Deterministic seeded RNG ([ADR-0002](0002-rng-chacha20.md)) for
  any tie-break / jitter.

The `MatchingEngine` trait is **frozen at v0** so swapping to a full
LOB implementation later is an additive change (`Box<dyn
MatchingEngine>`), not a refactor.

## Alternatives considered (deferred to v0.5)

- **`orderbook-rs`.** Lock-free, async. Good performance profile;
  surface needs evaluation against the v0.5 strategy mix.
- **`matchcore`.** State-machine. Cleaner mental model; less proven
  in crypto contexts.
- **`rust_ob`.** Minimal. Easiest to vendor if we end up needing
  partial-fill + IOC/FOK semantics.

All three deferred to v0.5 when limit orders, IOC/FOK flags, and
partial fills become real. v0.5 decision gate: pick one based on
partial-fill fidelity, post-only / IOC / FOK support, and
slippage/fee hook cleanliness.

## Consequences

- v0's `PaperEngine` is sufficient for SMA-class strategies through
  v0.5 composed-strategies — they all emit market orders.
- v1's cross-sectional momentum
  ([ADR-0013](0013-v1-cross-sectional-momentum.md)) and v1.5a's
  pairs ([ADR-0014](0014-v15a-mean-reversion-pairs.md)) also emit
  market orders, so the simple engine carries us further than the
  v0 brief anticipated. v0.5 LOB-pick decision is therefore on a
  longer timer than originally planned.
- The 9 backtest anchors in `anchors.toml` ([`../../../../spec/architecture/11-regression-gate.md`](../../../../spec/architecture/11-regression-gate.md))
  lock against this engine's deterministic output. Swapping in a
  full LOB engine will require re-locking each anchor after
  byte-identical reverification across two `--release` runs.

## Changelog
- 2026-04-17 (architect): initial accept.
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  Foundation libraries — Order book & matching engine during Phase
  1A Session 11.
