---
adr: 0021
title: RustQuant adopted as a helper crate, not a foundation
status: accepted
date: 2026-04-17
supersedes: none
superseded-by: none
---

# ADR-0021: RustQuant adopted as a helper crate, not a foundation

## Context

RustQuant is a free-time community crate maintained by `avhz` that
offers a broad menu of quantitative finance primitives: stats, FFT,
distributions, optimisation, stochastic processes, ISO codes, basic
ML. Adopting parts of it saves us from re-implementing well-known
algorithms; adopting it wholesale would lock us into an API that
the maintainer himself flags as in churn.

## Decision

RustQuant is a **helper**, not a foundation. We adopt named modules
behind thin adapter wrappers in our crates and ignore the rest.

Adopted modules:

| RustQuant module                         | Used for                                                       | Lives in our crate            |
|------------------------------------------|----------------------------------------------------------------|-------------------------------|
| `math` (risk-reward)                     | Sharpe, Sortino, Calmar, max drawdown, VaR, CVaR               | `risk`, `backtest`            |
| `math` (distributions / FFT / quadrature)| Stats, characteristic functions, numerical integration         | `features`, `models`          |
| `math` (optimization)                    | Root finding, gradient descent for calibration tasks           | `models`                      |
| `stochastics`                            | Brownian / OU / CIR for synthetic data + Monte Carlo           | `backtest`, `data` (synthetic)|
| `time`                                   | Day counters, schedules, conventions for funding etc.          | `core`                        |
| `data`                                   | CSV / JSON / Parquet I/O helpers                               | `data`                        |
| `iso`                                    | Currency / MIC codes                                           | `core`                        |
| `macros`                                 | `assert_approx_equal!` in tests                                | dev-dep, all crates           |

Pin an exact version in `Cargo.toml`. Run the pinned version through
`cargo audit` / `cargo deny`. Isolate every adopted module behind a
thin adapter in our crate so a swap-out is a one-file change.

## Explicitly NOT adopted

- **`instruments`** (bonds, options) — out of scope for spot crypto.
- **`models`** (rate / curve models) — fixed-income focus, not crypto.
- **`ml`** (linear / logistic regression, KNN) — too thin; use
  `linfa` / `candle`.
- **`trading`** (basic LOB) — we own our microstructure layer in
  `data` / `backtest`, tuned for crypto venue quirks. See
  [ADR-0026](0026-v0-simple-paper-engine.md).
- **`cashflows`, `portfolio`** — replaced by our own typed primitives
  in `core` so risk limits are encoded in the type system, not in
  runtime checks. See [ADR-0003](0003-decimal-money-math.md).

## Alternatives considered

- **Adopt RustQuant wholesale.** Free-time maintenance and known API
  churn make this a tail-risk on every release. Rejected.
- **Re-implement everything.** Stats, FFT, distributions are
  well-known; reinventing them costs weeks for zero edge. Rejected.
- **Hand-pick from multiple crates instead of one helper.** Adds
  cross-crate version-coherence cost; RustQuant covers enough of the
  surface that one helper plus targeted alternatives (`linfa`,
  `candle`, `kand`) wins.

## Consequences

- Each adopted module must have an adapter in our crate. Direct
  `RustQuant::...` imports outside the adapter are a code-review
  reject.
- API churn risk is bounded to the adapter modules — a breaking
  change in a RustQuant `math` shape is a one-file change in
  `crates/risk/src/rustquant_adapter.rs` (or equivalent).
- The pin-and-audit rule is mechanical: any RustQuant version bump
  triggers a `cargo audit` re-run before the PR can land.

## Changelog
- 2026-04-17 (architect): initial accept.
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  Foundation libraries — Quant primitives during Phase 1A Session 11.
