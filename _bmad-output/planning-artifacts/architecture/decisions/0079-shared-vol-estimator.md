---
adr: 0079
title: Shared multi-horizon σ̂ vol estimator — pure stateless module, home in `strategy` not `forecast`
status: accepted
date: 2026-06-30
supersedes: none
superseded-by: none
---

# ADR-0079: Shared σ̂ vol estimator (P1-5)

> **Record reconstructed 2026-08-15.** The decision itself was made and implemented on
> **2026-06-30** — its Registry row, its module, and its consumer all date from that day — but the
> ADR *file* was never committed, so the corpus carried 86 files against 87 rows and
> `scripts/adr_registry_check.py` (one-directional) could not see the hole. Found by the claims
> ledger; logged as bug-log **#86**.
>
> **Nothing here is invented.** Every statement below is transcribed from a primary source that
> predates this file: the Registry row (`README.md` L129), the module's own doc-comment
> (`crates/strategy/src/vol_estimator.rs` L1-40, which records the design decisions explicitly),
> `ADR-0078` (which consumes this decision), and `v2-architecture.md` §1 P1-5 / §6.0 D5. Where the
> present-day state differs from the original intent, it is marked **[2026-08-15 audit]** rather
> than silently reconciled — see Consequences.

## Context

Phase 2C introduced two sizing overlays that each need a volatility estimate: the vol-targeting
overlay (P1-4, ADR-0078) and the drawdown-control overlay (P1-3, ADR-0080). Without a shared module
each would grow its own σ̂, and the two would drift — different windows, different λ, different
treatment of the `Decimal` → `f64` boundary — while appearing to measure the same quantity.

A second question was **where** it lives. A volatility estimator superficially resembles forecasting,
which would put it in `crates/forecast`.

## Decision

Add `crates/strategy/src/vol_estimator.rs` as a **pure, stateless** module — plain functions, no
traits, no I/O, no `SystemTime`, no RNG. Determinism holds by construction.

**Home: `crates/strategy` — explicitly NOT `crates/forecast`.** Per operator-ratified decision **D5**
(`v2-architecture.md` §6.0): σ̂ here is a *sizing input*, not a return prediction. Keeping it in
`strategy` preserves the line **"vol-for-sizing ≠ return-prediction"**, and both consumers already
live in `strategy`. `ui` never imports it. **Zero new dependency edge.**

**Functions provided:**

| function | description |
|---|---|
| `log_returns_from_bars` | `ln(close_t / close_{t-1})` from a `Bar` slice |
| `realized_vol_from_returns` | simple trailing-window standard deviation |
| `ewma_realized_vol` | exponentially-weighted σ̂ (EWMA / RiskMetrics-style recurrence) |
| `har_realized_vol` | HAR-RV (Corsi 2009): daily + weekly[5] + monthly[22] equal-weight blend |

**Three exported λ constants**, derived from the half-life identity `λ = 0.5^(1/H)`:

| constant | value | half-life |
|---|---|---|
| `LAMBDA_126D_DAILY` | 0.994513935616829 | 126 daily bars |
| `LAMBDA_126D_HOURLY` | 0.999770810930342 | 3024 hourly bars (126 d) |
| `LAMBDA_RISKMETRICS` | 0.94 | RiskMetrics convention |

**`f64` for statistics, `Decimal` only at the money boundary.** Log-returns and vol use `f64`;
`log_returns_from_bars` converts `bar.close.get()` at the boundary and nowhere else. This mirrors
`crates/backtest/src/bakeoff/scorecard.rs` and does **not** breach AD-9 — AD-9 governs *money math*
(ledger, cash, P&L), and no money value is computed here.

### Anchor safety

Anchor-safe **by construction**: the module sits on no engine path and is reachable only through the
overlays' own opt-in configuration. Anchors were 119/119 unchanged at landing, and remain 119/119 as
of 2026-08-15.

## Status: accepted

Accepted 2026-06-30. Implemented the same day (`crates/strategy/src/vol_estimator.rs`). Story 4.5
`advisor-vol-estimator`; `trace.toml` feature `advisor-vol-estimator`.

## Consequences

- A single σ̂ definition is shared, so the two overlays cannot silently diverge on the thing they
  both claim to measure. That was the point.
- `ADR-0078` depends on this module and defaults to `LAMBDA_126D_HOURLY`.
- The `f64` boundary is now stated, so future readers do not have to re-litigate it against AD-9.

**[2026-08-15 audit] Actual consumption is narrower than the design anticipated.** The module
doc-comment (correctly, in the future tense) said both overlays *"will call these after their
respective reparameterisations land"*. Measured today:

| declared | actual |
|---|---|
| `vol_targeting_overlay.rs` consumes it | **Yes** — `ewma_realized_vol` at `:296`, plus `LAMBDA_126D_HOURLY` |
| `drawdown_control_overlay.rs` consumes it | **No** — the file contains no reference to `vol_estimator` |
| 4 functions provided | **1** has a production caller (`ewma_realized_vol`) |

So `log_returns_from_bars`, `realized_vol_from_returns` and `har_realized_vol` — including the full
Corsi-2009 HAR-RV implementation — currently have **zero production callers**. This is recorded, not
resolved: the original text was written in the future tense and is therefore not false, but "provided
for a consumer that has not arrived" is exactly the declared-vs-executed shape this project keeps
paying for (bug-log #81, #85, #89). Whether to wire `drawdown_control_overlay` to the shared module,
or to narrow this ADR to the one function actually in use, is an open decision.

## References

- Registry row: `README.md` L129 (the row that outlived its file — bug-log **#86**).
- Source: `crates/strategy/src/vol_estimator.rs` (module doc-comment carries the design decisions).
- Architect: `spec/v2/v2-architecture.md` §1 P1-5 and §6.0 D5 (operator-ratified home decision).
- Story: `epics.md` Story 4.5 `advisor-vol-estimator`.
- Consumed by: **ADR-0078** (vol-targeting overlay reposition) — `Consumes: ADR-0079`.
- Sibling: **ADR-0080** (drawdown-control overlay) — intended consumer, not yet wired.
- Research: Corsi (2009), *A Simple Approximate Long-Memory Model of Realized Volatility* (HAR-RV).
