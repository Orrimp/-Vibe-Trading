---
slug: architecture-02-strategy-registry
status: shipped
owner: architect
updated: 2026-06-21
---

# Strategy registry and hot-loading

Strategies are first-class plug-ins. The runtime owns a typed
registry of active strategies and routes data/signals through each.

## Lifecycle integration

Every strategy registry change (load, swap, unload, demote) emits
a journal entry to the audit ledger. Combined with the strategy
lifecycle gates in
[`../product.md` § Strategy lifecycle & promotion gates](../product.md#strategy-lifecycle--promotion-gates),
the ledger always answers "which strategies were active when this
trade fired?".

The mechanical surface for strategy lifecycle events lives in the
`strategy_events` SQLite table — see
[ADR-0008](../../_bmad-output/planning-artifacts/architecture/decisions/0008-v05-strategy-event-journal-schema.md). The
`kind` column carries an open set of TEXT values including (today)
`Load`, `Swap`, `Unload`, `Reject` (v0.5), `rebalance_rejected`
(v1, [ADR-0013](../../_bmad-output/planning-artifacts/architecture/decisions/0013-v1-cross-sectional-momentum.md) Q6),
`pair_hard_stop_tripped` / `pair_short_observed` / `pair_bar_stale`
(v1.5a, [ADR-0014](../../_bmad-output/planning-artifacts/architecture/decisions/0014-v15a-mean-reversion-pairs.md) Q8 + Q10),
and `KillSwitchTripped` (v1+, [ADR-0015](../../_bmad-output/planning-artifacts/architecture/decisions/0015-operator-success-reports.md) Q8).

## Hot-loading evolution

The hot-loading decision evolved across three releases. Each
release's decision is captured in its own ADR; the decisions
cross-link to form the v0 → v0.5 → v1+ narrative:

- [ADR-0005](../../_bmad-output/planning-artifacts/architecture/decisions/0005-v0-strategy-trait-no-hotload.md) — v0 clean
  trait shape, no hot-load (compiled-in).
- [ADR-0006](../../_bmad-output/planning-artifacts/architecture/decisions/0006-v05-config-driven-composition.md) — v0.5
  config-driven composition (hot-load A) via TOML + file watcher.
- [ADR-0007](../../_bmad-output/planning-artifacts/architecture/decisions/0007-v1-wasm-plugin-deferred.md) — v1+ WASM
  plugins (hot-load B), deferred until a strategy with genuinely
  custom logic justifies it. Native dyn-libs and embedded
  scripting explicitly rejected.

## v0.5 strategy-registry resolution cluster

Five interconnected v0.5 decisions captured as
[ADR-0008](../../_bmad-output/planning-artifacts/architecture/decisions/0008-v05-strategy-event-journal-schema.md),
[ADR-0009](../../_bmad-output/planning-artifacts/architecture/decisions/0009-v05-registry-concurrency.md),
[ADR-0010](../../_bmad-output/planning-artifacts/architecture/decisions/0010-v05-composed-exit-policy.md),
[ADR-0011](../../_bmad-output/planning-artifacts/architecture/decisions/0011-v05-cockpit-strategies-panel.md), and
[ADR-0012](../../_bmad-output/planning-artifacts/architecture/decisions/0012-v05-broadcast-bus-extensions.md). Together they
specify the strategy-event sibling table, the
`parking_lot::RwLock` concurrency choice, the symmetric
signal-flip exit policy, the cockpit Strategies panel placement,
and the broadcast types in `trading_core`.

## Strategy releases (v1 and later)

Each strategy release's architectural resolutions are captured as
a single multi-Q ADR rather than fragmenting into one ADR per Q,
because the Q's within a release tend to be too interdependent to
read separately:

- [ADR-0013](../../_bmad-output/planning-artifacts/architecture/decisions/0013-v1-cross-sectional-momentum.md) — v1
  cross-sectional momentum (Q1–Q6).
- [ADR-0014](../../_bmad-output/planning-artifacts/architecture/decisions/0014-v15a-mean-reversion-pairs.md) — v1.5a
  mean-reversion pairs (Q1–Q10).
- [ADR-0017](../../_bmad-output/planning-artifacts/architecture/decisions/0017-v15b-multi-venue.md) — v1.5b multi-venue
  execution scaffolding (Q1–Q12).
- [ADR-0019](../../_bmad-output/planning-artifacts/architecture/decisions/0019-v2-llm-strategy.md) — v2 LLM strategy
  foundation (Q4–Q11).
- [ADR-0027](../../_bmad-output/planning-artifacts/architecture/decisions/0027-kronos-onnx-tract-integration.md) — v2.5
  Kronos foundation-model forecast overlay (ONNX + tract). The
  cross-cutting overlay pattern lives in
  [12-forecast-overlay.md](12-forecast-overlay.md).

## Cross-cutting rules formalised by the strategy clusters

Several project-wide rules became explicit during the strategy ADR
extractions:

- **Strategy proposes, risk disposes.** Multi-leg / multi-symbol
  strategies emit `Vec<ProposedOrder>` representing their ideal
  action; `risk::size_portfolio_target` clamps to limits. The
  strategy is unaware of the cap. See ADR-0014 Q9.
- **Strategy-side filtering for symbol universes.** The
  `StrategyRegistry::on_bar` fan-out stays minimal; strategies
  filter `bar.symbol` internally. See ADR-0013 Q5.
- **Composites are `Strategy`s, not registry special-cases.** A
  strategy that combines others (regime dispatch, ensemble vote)
  implements the FROZEN `Strategy` trait by holding its members +
  arbitrating their per-bar signals in its own `on_bar` — so it is a
  first-class registry citizen reachable from one id→members mapping,
  never special-cased at the `run_scenario` / `build_registry_for` /
  `StrategyRegistry` seams. See ADR-0049 (`RegimeDispatcher`) and
  ADR-0063 (`EnsembleStrategy` signal-vote — the F8 advisor
  ensemble; un-warmed members ABSTAIN rather than vote FLAT). The
  ensemble's robustness flag is computed on its OWN realized
  equity curve through the now-active `RobustnessMode::Bootstrap`
  gate (ADR-0063 § D4), reusing the Politis–White block-length
  selector + the ADR-0051 sub-seed determinism + the frozen
  `classify_verdict`.
- **Open-set `TEXT` columns for event-type taxonomy.** The
  `strategy_events.kind` column absorbs new event types without
  schema migrations. See ADR-0008 (precedent) and the eight-plus
  `kind` values accumulated across the strategy ADRs above.

## Changelog
- 2026-06-21 (architect): added the "composites are `Strategy`s, not
  registry special-cases" cross-cutting rule (ADR-0063 F8 ensemble
  signal-vote seam — `EnsembleStrategy` implements the frozen
  `Strategy` trait; the abstention-quorum warmup rule; the
  now-active `RobustnessMode::Bootstrap` gate on the ensemble's own
  equity curve). Generalises the ADR-0049 `RegimeDispatcher`
  precedent.
- 2026-05-16 (architect): added ADR-0027 (v2.5 Kronos overlay)
  reference + cross-link to the new
  [12-forecast-overlay.md](12-forecast-overlay.md) cross-cutting
  pattern file.
- 2026-05-13 (architect): body migrated from `spec/architecture.md`
  § Strategy registry & hot-loading during Phase 1A Session 12.
  The lifecycle-integration prose was the only current-state
  content remaining in the monolith; everything else was already
  extracted to ADRs in Sessions 4-10.
