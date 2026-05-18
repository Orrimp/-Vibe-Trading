---
slug: architecture-adr-index
status: in-progress
owner: architect
updated: 2026-05-18 (ADR-0033 added)
---


# Architecture Decision Records (ADR) — Index

Every non-trivial architectural decision lives here as a dated,
numbered, immutable record. ADRs are *cited* by `spec/trace.toml` rows
in their `arch` field, by feature `## Design` sections, and by code
review checklists.

## Format

See [TEMPLATE.md](TEMPLATE.md). The required frontmatter is:

```yaml
---
adr: NNNN
title: <short imperative title>
status: proposed | accepted | superseded | deprecated
date: <YYYY-MM-DD>
supersedes: <NNNN | none>
superseded-by: <NNNN | none>
---
```

Body sections: `Context`, `Decision`, `Alternatives considered`,
`Consequences`. Keep ADRs short — most should fit in 100-200 lines.
Longer ADRs usually mean the decision is actually two decisions in a
trench coat.

## Numbering rules

- Numbers are assigned by the architect when the ADR is filed and never
  reused. A `superseded` ADR keeps its number; the new ADR gets a new
  one and references the old via `supersedes:`.
- Numbers are zero-padded to 4 digits (`0001`, not `1`).
- Filename pattern: `NNNN-kebab-case-title.md`.

## Registry

(Cross-referenced from `spec/architecture.md` § ADR registry. This is
the canonical table; the parent file links here.)

| ID    | Title                                              | Status     | Date       |
|-------|----------------------------------------------------|------------|------------|
| 0001  | Crate names avoid stdlib collisions                | accepted   | 2026-04-17 |
| 0002  | RNG seeded with ChaCha20 from config seed          | accepted   | 2026-04-17 |
| 0003  | Money math uses Decimal, never f64                 | accepted   | 2026-04-17 |
| 0004  | Audit-DB uses 6-digit fractional-second timestamps | accepted   | 2026-04-18 |
| 0005  | v0 — clean strategy trait shape, no hot-load       | accepted   | 2026-04-17 |
| 0006  | v0.5 — config-driven strategy composition (hot-load A) | accepted | 2026-04-19 |
| 0007  | v1+ — WASM plugin hot-load deferred                | accepted   | 2026-04-19 |
| 0008  | v0.5 — strategy-event journal schema (Q1)          | accepted   | 2026-04-19 |
| 0009  | v0.5 — registry concurrency: parking_lot::RwLock (Q2) | accepted | 2026-04-19 |
| 0010  | v0.5 — ComposedStrategy exit policy: signal-flip only (Q3) | accepted | 2026-04-19 |
| 0011  | v0.5 — cockpit Strategies panel: right column (Q4) | accepted   | 2026-04-19 |
| 0012  | v0.5 — strategy broadcast types in trading_core (Q5) | accepted | 2026-04-19 |
| 0013  | v1 — cross-sectional momentum resolutions (Q1–Q6)  | accepted   | 2026-04-29 |
| 0014  | v1.5a — mean-reversion pairs resolutions (Q1–Q10)  | accepted   | 2026-04-30 |
| 0015  | v1+ — Operator success reports (Q1–Q9)              | accepted   | 2026-05-01 |
| 0016  | v1+ — real-mtm unrealized PnL plumbing (Q1–Q8 + R10) | accepted  | 2026-05-02 |
| 0017  | v1.5b — multi-venue execution scaffolding (Q1–Q12)  | accepted   | 2026-05-03 |
| 0018  | Lumen design adoption — Phase 1 foundation (Q1–Q11) | accepted   | 2026-05-04 |
| 0019  | v2 — LLM strategy foundation (Q4–Q11)               | accepted   | 2026-05-10 |
| 0020  | Chart buy/sell emphasis (v1.9 Q1–Q9)                | accepted   | 2026-05-10 |
| 0021  | RustQuant adopted as helper, not foundation         | accepted   | 2026-04-17 |
| 0022  | Cost telemetry lives in dedicated `cost` crate      | accepted   | 2026-04-17 |
| 0023  | iced is the single UI stack                         | accepted   | 2026-04-17 |
| 0024  | Audit ledger: raw `sqlx` + SQLite, not `sqlx-ledger`| accepted   | 2026-04-19 |
| 0025  | v0 hand-rolled Binance WS behind trait              | accepted   | 2026-04-17 |
| 0026  | v0 simple paper engine; LOB deferred                | accepted   | 2026-04-17 |
| 0027  | v2.5 — Kronos foundation-model forecast overlay (ONNX + tract) | accepted | 2026-05-16 |
| 0028  | v2.5 — DL forecast overlay trained in `candle` (supersedes 0027) | accepted | 2026-05-16 |
| 0029  | v2.5 — Forecast-checkpoint provenance schema + LFS-anchor strategy | accepted | 2026-05-17 |
| 0030  | Cockpit calls the backtest engine in-process via tightened API | accepted | 2026-05-17 |
| 0031  | `AuditTick<Event, Context>` consumer envelope for audit ledger read path | proposed | 2026-05-17 |
| 0032  | Backtest real-Binance-data path + REVISION.toml data-revision pin | accepted | 2026-05-18 |
| 0033  | v2.5 TCN alpha-investigation report shape + F-verdict algorithm | accepted | 2026-05-18 |

All architectural decisions are now extracted. Remaining Phase 1A
work: final monolith compression (Changelog) and section-file body
finalisation.

## Changelog
- 2026-05-13 (architect): index initialised. ADRs 0001-0004 land in the
  same session (Session 1).
- 2026-05-13 (architect): ADRs 0005-0007 added (hot-load decisions
  cluster) during Session 4.
- 2026-05-13 (architect): ADRs 0008-0012 added (v0.5 strategy-registry
  resolution cluster) during Sessions 5+6.
- 2026-05-13 (architect): ADR-0013 added (v1 cross-sectional momentum
  resolution cluster) during Session 7.
- 2026-05-13 (architect): ADR-0014 added (v1.5a mean-reversion pairs
  cluster) during Session 8.
- 2026-05-13 (architect): ADRs 0015-0017 added (operator reports,
  real-mtm, v1.5b multi-venue) during Session 9.
- 2026-05-13 (architect): ADRs 0018-0020 added (Lumen Phase 1, v2 LLM,
  chart buy/sell emphasis) during Session 10. All strategy-registry
  Q&A clusters now extracted.
- 2026-05-16 (architect): ADR-0027 added (v2.5 Kronos forecast
  overlay — ONNX + `tract` integration, signal-level overlay on v1
  momentum). Cross-cutting overlay pattern landed in new section
  file `spec/architecture/12-forecast-overlay.md`.
- 2026-05-16 (orchestrator): ADR-0028 supersedes ADR-0027 after Wave A
  bootstrap surfaced (a) Kronos lives outside `transformers`,
  (b) two-model architecture requires Rust-side sampling-loop
  reimplementation, (c) crypto-fit was never validated. Operator
  pivoted v2.5 to "train small custom Transformer/TCN in `candle`".
  Wave A crates (forecast / replay-cache / core forecast types) are
  model-agnostic and preserved; only Kronos-specific files removed.
- 2026-05-13 (architect): ADRs 0021-0026 added (Foundation libraries
  substantive decisions) during Session 11. iced UI body migrated to
  06-ui-and-cockpit.md. All architectural decisions now in ADRs.
- 2026-05-17 (architect): ADR-0029 added (TCN/forecaster checkpoint
  provenance schema + LFS-anchor strategy — cross-phase contract for
  v2.5 TCN, v2.5a PatchTST, v2.5b vanilla Transformer). Also
  backfilled the registry table row for ADR-0028 (previously only in
  changelog).
- 2026-05-17 (architect): ADR-0030 added (cockpit in-process backtest
  engine API). Opens the `ui → backtest` edge for the Lab Run button
  shipped by `ui-rethink-phase-a-lab` per operator-decision Q-A2.
- 2026-05-17 (orchestrator): ADR-0031 added (status: proposed) —
  `AuditTick<Event, Context>` consumer envelope for the audit ledger
  read path. Borrowed from
  [barter-rs](https://github.com/barter-rs/barter-rs) per the survey at
  `spec/dev-notes/external-code-patterns-2026-05-17.md`. Decouples
  consumer-side state replicas from producer's channel choice via
  generic Iterator. Strictly additive (existing taps + broadcast tee
  coexist); zero hot-path impact; 11 anchors stay byte-identical.
  Implementation queued in `spec/backlog.md ## Queue`.
  Establishes the invocation pattern reused by Phase B / Phase E
  Compare matrix / v3 continuous-paper.
- 2026-05-18 (architect): ADR-0032 added — backtest real-Binance-data
  path + `REVISION.toml` data-revision pin for the four new
  `top10-*-fy-tcn-overlay[-weights]-realdata` scenarios. Locks
  (a) module placement (`crates/backtest/src/realdata.rs`,
  feature-gated `realdata`, reuses `data::ReplayFeed::merge_symbols`),
  (b) `REVISION.toml` schema + aggregate-SHA algorithm,
  (c) orthogonal `ScenarioDataSource` axis on `Scenario` (not new
  `ScenarioStrategy` variants), (d) `data_revision_sha` in both
  frontmatter (forensics, excluded from body hash) and a new
  `## Data source` body section (anchor integrity, covered by body
  hash). Existing 15 anchors stay byte-identical (K6 + K10).
  Closes T-AR-1, T-AR-3 of
  `spec/backtest-real-binance-data/tasks.md`.
- 2026-05-18 (architect): ADR-0033 added — v2.5 TCN alpha-investigation
  report shape + F-verdict algorithm. Locks (a) read-path placement
  (new bin `crates/forecast/src/bin/forecast_distribution.rs`, NOT
  extension of `crates/backtest`'s TCN dispatch — preserves anchor
  neutrality against the 4 byte-locked `-realdata` anchors and
  generalises to v2.5a/v2.5b alpha-investigations), (b) report
  frontmatter-vs-body discipline + floating-point canonicalisation
  (`%.6f` / `%.9f` / `(x * 1e6) as i64` for bin edges) following the
  ADR-0032 § D4 precedent, (c) F-verdict decision algorithm
  (deterministic F1/F2/F3/F4 classifier over (abs_p95, std,
  sigma_train, frac_inside_epsilon, confidence_gate_survival) with
  F-MIXED escape hatch for BS-1 vs. BS-2 disagreement), (d) Sharpe /
  Sortino / Calmar formulas (sqrt(24·365) hourly annualisation; new
  helpers in the M-SHARPE bin, NOT reuse of `compute_sharpe` which is
  minute-annualised). Existing 19 anchors stay byte-identical
  (R6 non-regression contract). Closes T-AR-3 of
  `spec/v25-tcn-alpha-investigation/tasks.md`.
