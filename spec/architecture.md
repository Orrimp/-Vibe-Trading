---
slug: architecture
status: shipped
owner: architect
updated: 2026-06-22
---

# Architecture — Crypto Trading Agent

This file is the **thin index** for the system architecture. All
substantive content lives in `spec/architecture/NN-*.md` section
files (current-state design) or `spec/architecture/adr/NNNN-*.md`
(numbered architectural decision records). Phase 1A of the spec
hygiene plan (2026-05-13 → 2026-05-16) compressed the prior
5,635-line monolith into this index plus 12 sections and 26 ADRs.

To answer a specific architectural question, jump straight to the
relevant section or ADR — there is nothing substantive in this
file beyond the registries and the cross-cutting invariants below.

## Cross-cutting invariants

Three rules apply to every section file and ADR. They are the
non-negotiables that any architectural change must preserve.

1. **Audit imports nothing from sibling crates.** The `audit`
   crate's `[dependencies]` lists only `trading_core` plus
   third-party libs. Sibling crates write into the ledger by
   importing `audit`; `audit` never imports back. This keeps the
   reconciler invariant (Σ debits == Σ credits) provable from
   `audit`'s source alone. Detail in
   [01-data-flow.md](architecture/01-data-flow.md).
2. **Money math uses `Decimal`, never `f64`.** Every monetary
   value is a `Money<C: Currency>` newtype wrapping
   `rust_decimal::Decimal`. Aggregation rules are exact-cent with
   zero tolerance. See [ADR-0003](architecture/adr/0003-decimal-money-math.md).
3. **The `ui` crate (lib + every binary target) never depends on
   `strategy`, `exec`, `forecast`, or `llm`.** Bootstrap of those
   types happens in `agent`. (There is **no `crates/models`** — the
   real workspace crate set is agent / audit / backtest / core / cost /
   data / exec / features / forecast / llm / reflection / replay-cache /
   reports / risk / strategy / trader / ui; the ML / DL work lives in
   `crates/forecast` + `crates/features`, not a `models` crate. The four
   crates `ui` must not import are `strategy` / `exec` / `forecast` / `llm`
   — the seam ADR-0059 / ADR-0060 / ADR-0062 all enforce.) Detail in
   [06-ui-and-cockpit.md § App layout](architecture/06-ui-and-cockpit.md#app-layout).

Anchor regressions, RNG seeding, timestamp precision, and report
determinism rules live alongside their respective sections —
[11-regression-gate.md](architecture/11-regression-gate.md),
[ADR-0002](architecture/adr/0002-rng-chacha20.md),
[ADR-0004](architecture/adr/0004-fractional-second-timestamps.md).

## Section file registry

| File | Topic |
|------|-------|
| [00-overview.md](architecture/00-overview.md) | Workspace layout, crate naming, runtime model |
| [01-data-flow.md](architecture/01-data-flow.md) | End-to-end data flow, crate-dependency edges, bin-shared `agent::runtime::run` API, audit migrations |
| [02-strategy-registry.md](architecture/02-strategy-registry.md) | Strategy registry shape, hot-loading lifecycle, journal integration |
| [03-execution-and-venues.md](architecture/03-execution-and-venues.md) | Paper engine, live-mode wiring, `Venue` enum, multi-venue panic isolation |
| [04-risk-and-money.md](architecture/04-risk-and-money.md) | Risk engine surface, kill switch, money-math invariant |
| [05-llm-and-reflection.md](architecture/05-llm-and-reflection.md) | ML / DL stack choice, LLM provider trait, reflection-memory cross-link |
| [06-ui-and-cockpit.md](architecture/06-ui-and-cockpit.md) | Cockpit + viewer architecture, UI isolation rule, `audit::query` surface, Lumen design integration |
| [07-observability.md](architecture/07-observability.md) | `tracing` layout, metrics, log shape |
| [08-recovery-and-backups.md](architecture/08-recovery-and-backups.md) | Audit DB backup cadence, restore drill, boot-id discipline |
| [09-performance-budget.md](architecture/09-performance-budget.md) | Per-component latency / throughput targets |
| [10-foundation-libraries.md](architecture/10-foundation-libraries.md) | Approved foundation crates and substitutes |
| [11-regression-gate.md](architecture/11-regression-gate.md) | Anchor-SHA enforcement, `verify_anchors.sh`, report-determinism rules |

## ADR registry

Canonical index also at [architecture/adr/README.md](architecture/adr/README.md).

| ID    | Title                                                      | Status   | Date       |
|-------|------------------------------------------------------------|----------|------------|
| [0001](architecture/adr/0001-crate-name-stdlib-collision.md) | Crate names avoid stdlib collisions | accepted | 2026-04-17 |
| [0002](architecture/adr/0002-rng-chacha20.md) | RNG seeded with ChaCha20 from config seed | accepted | 2026-04-17 |
| [0003](architecture/adr/0003-decimal-money-math.md) | Money math uses Decimal, never f64 | accepted | 2026-04-17 |
| [0004](architecture/adr/0004-fractional-second-timestamps.md) | Audit-DB uses 6-digit fractional-second timestamps | accepted | 2026-04-18 |
| [0005](architecture/adr/0005-v0-strategy-trait-no-hotload.md) | v0 — clean strategy trait shape, no hot-load | accepted | 2026-04-17 |
| [0006](architecture/adr/0006-v05-config-driven-composition.md) | v0.5 — config-driven strategy composition | accepted | 2026-04-19 |
| [0007](architecture/adr/0007-v1-wasm-plugin-deferred.md) | v1+ — WASM plugin hot-load deferred | accepted | 2026-04-19 |
| [0008](architecture/adr/0008-v05-strategy-event-journal-schema.md) | v0.5 — strategy-event journal schema | accepted | 2026-04-19 |
| [0009](architecture/adr/0009-v05-registry-concurrency.md) | v0.5 — registry concurrency: parking_lot::RwLock | accepted | 2026-04-19 |
| [0010](architecture/adr/0010-v05-composed-exit-policy.md) | v0.5 — ComposedStrategy exit policy: signal-flip only | accepted | 2026-04-19 |
| [0011](architecture/adr/0011-v05-cockpit-strategies-panel.md) | v0.5 — cockpit Strategies panel layout | accepted | 2026-04-19 |
| [0012](architecture/adr/0012-v05-broadcast-bus-extensions.md) | v0.5 — strategy broadcast types in trading_core | accepted | 2026-04-19 |
| [0013](architecture/adr/0013-v1-cross-sectional-momentum.md) | v1 — cross-sectional momentum resolutions | accepted | 2026-04-29 |
| [0014](architecture/adr/0014-v15a-mean-reversion-pairs.md) | v1.5a — mean-reversion pairs resolutions | accepted | 2026-04-30 |
| [0015](architecture/adr/0015-operator-success-reports.md) | v1+ — Operator success reports | accepted | 2026-05-01 |
| [0016](architecture/adr/0016-real-mtm-unrealized-pnl.md) | v1+ — real-mtm unrealized PnL plumbing | accepted | 2026-05-02 |
| [0017](architecture/adr/0017-v15b-multi-venue.md) | v1.5b — multi-venue execution scaffolding | accepted | 2026-05-03 |
| [0018](architecture/adr/0018-lumen-phase-1-foundation.md) | Lumen design adoption — Phase 1 foundation | accepted | 2026-05-04 |
| [0019](architecture/adr/0019-v2-llm-strategy.md) | v2 — LLM strategy foundation | accepted | 2026-05-10 |
| [0020](architecture/adr/0020-chart-buy-sell-emphasis.md) | Chart buy/sell emphasis | accepted | 2026-05-10 |
| [0021](architecture/adr/0021-rustquant-adoption.md) | RustQuant adopted as helper, not foundation | accepted | 2026-04-17 |
| [0022](architecture/adr/0022-cost-telemetry-crate.md) | Cost telemetry lives in dedicated `cost` crate | accepted | 2026-04-17 |
| [0023](architecture/adr/0023-iced-frontend.md) | iced is the single UI stack | accepted | 2026-04-17 |
| [0024](architecture/adr/0024-audit-sqlite-raw-sqlx.md) | Audit ledger: raw `sqlx` + SQLite, not `sqlx-ledger` | accepted | 2026-04-19 |
| [0025](architecture/adr/0025-hand-rolled-binance-ws.md) | v0 hand-rolled Binance WS behind `MarketDataSource` trait | accepted | 2026-04-17 |
| [0026](architecture/adr/0026-v0-simple-paper-engine.md) | v0 simple paper engine; LOB deferred to v0.5 | accepted | 2026-04-17 |

New ADRs are added under `architecture/adr/NNNN-<slug>.md` and
registered both here and in the canonical README. The numbering is
monotonic; never reuse a number even if an ADR is later
superseded — mark the old one `status: superseded` and link
forward.

## Working with this index

- Need to answer "how does X work today?" → start at the section
  file that owns X (see the section registry).
- Need to recover "why was X chosen?" → search the ADR registry.
- Need to add a new architectural decision → write a new ADR
  (next available number), add a registry row here and in
  `architecture/adr/README.md`, then update the relevant section
  file's body to reference it.
- Need to change a cross-cutting invariant → that's an ADR plus
  a section-file edit plus (likely) an anchor refresh. See
  [11-regression-gate.md](architecture/11-regression-gate.md)
  for the anchor procedure.

The pre-Phase-1A monolithic content (including the 976-line
per-feature changelog of architectural deltas dated 2026-04-17
through 2026-05-10) is archived verbatim at
[`spec/archive/architecture-changelog-pre-split-2026-05-13.md`](archive/architecture-changelog-pre-split-2026-05-13.md)
and is preserved in git history at the pre-Session-12 commit. Each
historical decision in the archive maps to one of the 26 numbered
ADRs above.

## Developer deviations

Architectural deviations discovered during implementation. Each entry
references the original spec and the reason for the divergence.

- 2026-05-21 (developer): **`threshold_sweep` bin location** —
  D-AR-1.a (decomp.md) specified `crates/forecast/src/bin/threshold_sweep.rs`.
  Actual location: `crates/backtest/src/bin/threshold_sweep.rs`.
  Reason: placing the bin in `crates/forecast` would create a circular
  dependency (`forecast → backtest → strategy → forecast`). The bin needs
  `backtest` for `RealDataBarSource`, `run_cell`, and `PaperEngine`; adding
  `forecast → backtest` closes a dep cycle. Moving to `backtest` is
  functionally equivalent — the bin is an in-process orchestrator for
  backtest scenarios and `backtest` already depends on `strategy` (which
  depends on `forecast` via the `forecast` feature). Two new constructors
  (`load_from_paths_with_epsilon`, `from_forecaster_with_epsilon`) were added
  to `strategy::TcnSyncForecaster` to avoid the `backtest` bin needing a
  direct `forecast::tcn::TcnForecaster` import. Flagged to architect.

## Changelog

Architecture-level meta events only — file splits, ADR-numbering
schema changes, registry restructurings, cross-cutting refactors
that span multiple ADRs. Decision changelog entries live in each
ADR's own `## Changelog` section. Current-state design changelog
entries live in each section file.

- 2026-06-24 (architect): **ADR-0069 — gate-tied hyperparameter sweep seam**
  (feature `advisor-param-tuning`, leaderboard-epic item #3 — the operator-chosen
  gate-tied option, NOT a naive single-config editor). Recorded in the canonical ADR
  registry ([architecture/adr/README.md](architecture/adr/README.md) +
  `architecture/adr/0069-gate-tied-parameter-sweep-seam.md`). A parameter-grid
  **sibling of the bake-off**: a new library entry point
  `backtest::bakeoff::sweep::run_param_sweep` loops N parameterised configs of ONE
  family (SMA / MACD / RSI / Bollinger), runs each via the existing `run_scenario`,
  and scores each through the **byte-frozen** robustness gate
  (`compute_robustness_flag` → `classify_verdict`) so overfit configs read FRAGILE by
  construction. The `crates/backtest/src/bin/param_robustness_sweep.rs` bin is **NOT
  reused** — it is bin-only AND in the wrong universe (a 10-symbol cross-sectional
  momentum `ThetaCell` grid, unrelated to the single-coin rule-engine params); the
  reuse is the already-extracted *library* primitives + the `run_bakeoff`
  orchestration / `from_report`-mirror / `BakeoffProgress` template, homed in
  `backtest` per ADR-0059 § D1 → **zero new `ui` dep edge**. The bootstrap
  **distribution** (p5/p50/p95) is surfaced — not just the flag — via an additive,
  behaviour-preserving `compute_robustness_distribution` sibling (the spread IS the
  anti-overfit affordance); the gate bands + seed rule stay byte-frozen. The real
  **engine gap** closed: MACD/RSI/Bollinger params are literals inside the `signal`
  DSL string with no runtime override, so the sweep parameterises them by generating
  the `signal` string in memory + `ComposedStrategyConfig::from_str` (SMA already has
  the typed `sma_fast_len/slow_len` seam) — ship SMA-first, then the composed
  families behind a round-trip test. New library API: see the engine-seam note in
  [advisor-param-tuning/feature.md](advisor-param-tuning/feature.md) § 2. Grid capped
  at `MAX_SWEEP_CONFIGS = 24` with honest truncation. FRAGILE is prominent +
  promotion-blocking; the editor is a **Lab sub-view** reached by a "Tune…"
  drill-down (the `InspectStrategyFromLeaderboard` precedent), NOT a 13th nav screen.
  Anchor-safe by construction (every cell `write_report = false` → 119/119, no anchor
  SHA mutates) + the CLAUDE.md day-1 divergence e2e + render-pixel guards. PAPER/SIM
  ONLY. Leans on ADR-0059, ADR-0063, ADR-0066, ADR-0057, ADR-0003.
- 2026-06-22 (orchestrator): **ADR-0066 — benchmark exemption from the
  `AllFragile` outcome** (feature `advisor-benchmark-robustness`, the B1
  robustness-honesty fix). Logged here because it **spans two ADRs**: it amends
  **ADR-0059 § D5** (the F2 comparator outcome rule) and **ADR-0063 § D7** (the
  R4.4 reachability promise), while leaving the classifier freeze (ADR-0059 § D4 /
  ADR-0063 § D4) byte-unchanged. Recorded in the ADR registry
  ([architecture/adr/README.md](architecture/adr/README.md) +
  `architecture/adr/0066-benchmark-exempt-from-allfragile.md`). The F8 robustness
  gate judged the buy-and-hold **benchmark** by the same candidate-overfit ruler
  as the active arms, so on real BTCUSDT every arm flagged Fragile → outcome
  always `AllFragile`, suppressing the honest `BenchmarkWins`. Two coordinated
  `rank_candidates` edits (benchmark dropped from the `AllFragile` determination +
  always crown-eligible) restore `BenchmarkWins` as the modal real-crypto outcome;
  the seam is `rank_candidates`, NOT `classify_verdict`. Commits `ab13407` (fix) +
  `884836a` (closure). Leans on ADR-0059, ADR-0063, ADR-0051.
- 2026-06-22 (architect): **ADR-0065 — EUR→USDT budget-conversion seam
  (configurable static FX rate)** (feature `advisor-eur-fx`, single-coin-advisor
  pivot F7 — the LAST v0.2 item; resolves product § D4's deferred fixed EUR→USD
  rate). Recorded in the canonical ADR registry
  ([architecture/adr/README.md](architecture/adr/README.md) +
  `architecture/adr/0065-eur-usdt-budget-conversion-seam.md`). The operator
  enters a budget in euros but the engine is USDT-denominated end-to-end and **no
  `Eur` type exists**, so today the parsed euro `Decimal` is stamped **1:1** into
  `Money<Usdt>` at `cockpit_live.rs:1431-1437` ("€200 ≈ 200 USDT — FX not
  modelled"). F7 applies `usdt = eur × rate` at that single budget-conversion
  boundary via a new `FxRate {rate, source, as_of}` value object homed in
  **`crates/core::fx`** (the ADR-0058 § D2 home-the-primitive-in-`core`
  precedent — zero new dependency, and because `ui` already imports
  `trading_core` the conversion reaches the UI seam as a `core` type with **no new
  `ui` edge** — `cargo tree -p ui` unchanged by construction). The rate VALUE
  comes from advisor config (`eur_usd_rate: Option<Decimal>`, `#[serde(default)]`)
  defaulting to `DEFAULT_EUR_USD_RATE = 1.08` — **operator-LOCKED to a
  configurable static rate**; a live-fetched rate is an explicit **v0.3 upgrade**
  (a `RateSource` trait + a `crates/data` fetcher behind a fake seam, NOTED-not-
  built) that reuses this constant as its fallback (a strict superset → zero
  rework). One pure `FxRate::convert_eur_to_usdt(eur: Decimal) -> Money<Usdt>` fn
  + a `BudgetConversion {eur, rate, usdt}` carrier computed **once** so the ENGINE
  (`ForwardRunConfig.budget`) and the DISPLAY read the **same** converted value
  (the ADR-0062 one-boundary/two-readers anti-drift discipline — the "$216" the
  operator reads is definitionally the `Money<Usdt>` F4 caps against). EUR stays a
  **labelled `Decimal` at input** (no `core::Eur` marker — a first-class EUR
  currency + ledger FX-PnL is REJECTED) so F4/F5/bake-off stay **unit-agnostic and
  byte-unchanged**. **Anchor-safe by construction (119/119):** the anchored
  CLI/headless path (bake-off, `run_scenario`, sweeps) is USDT-denominated and
  **never reads the rate** — the conversion lives only at the cockpit UI input
  boundary; no `anchors.toml` SHA or report body is touched. Ships with the
  **day-1 conversion-applied e2e gate** (`crates/core/tests/eur_fx_conversion_
  applied.rs`, FAIL-before/PASS-after against a 1:1 stub: converted ≠ 1:1 AND the
  converted value is the one F4 caps against AND display == engine) per the
  CLAUDE.md non-negotiable + the `v3-volatility-forecaster-noop` precedent. The
  three "FX not modelled" literals (`LEADERBOARD_BUDGET_HINT`,
  `FORWARD_PLAN_BUDGET_LINE`, `LIVE_FORWARD_FX_NOTE`) become the honest "€X ≈ $Y
  (at R EUR/USD, source as-of)", driven by the same carrier; hard-cap +
  not-advice copy preserved verbatim. Leans on ADR-0058 § D2, ADR-0060 § D1/D3,
  ADR-0061 § D1, ADR-0062, ADR-0003, ADR-0023/0041.
- 2026-06-21 (architect): **ADR-0063 — ensemble signal-vote seam +
  robustness-gate activation** (feature `advisor-ensemble`, single-coin-advisor
  pivot F8). Recorded in the canonical ADR registry
  ([architecture/adr/README.md](architecture/adr/README.md) +
  `architecture/adr/0063-ensemble-vote-seam-and-robustness-gate-activation.md`).
  The F8 "mix" capability ships as `strategy::EnsembleStrategy` implementing the
  FROZEN `Strategy` trait (the ADR-0049 `RegimeDispatcher` precedent generalised
  to N homogeneous members + a pure vote arbiter) — the ONE primitive reachable
  identically from `run_scenario`, `build_registry_for` (F5b forward-paper), and
  `StrategyRegistry`. Members are reused via a shared `build_member(id)` over the
  SAME `config/strategies/*.toml` the bake-off scored; an un-warmed member
  ABSTAINS (counted in neither the vote nor the denominator). Bundled WITH the
  activation of the today-inert robustness gate via a new
  `RobustnessMode::Bootstrap` per-candidate compute-and-feed (reuses the existing
  Politis–White block-length selector + the ADR-0051 sub-seed determinism + the
  frozen `classify_verdict` — the classifier and its bands are NOT touched).
  Multiple-comparisons hardening re-evaluated and DEFERRED (not-yet at a fixed
  7-arm pre-registered field; ADR-trigger recorded). **Anchor-safe by
  construction (119/119):** `Skip` stays the default, the two ensemble arms + the
  `Bootstrap` mode are opt-in ONLY on the no-report advisor bake-off path, and the
  existing single-id `run_scenario` arms + `default_field()` stay byte-untouched.
  `PlanRuleShape::Ensemble { method, members }` extends the closed plan enum (no
  `String` crosses the seam); `cargo tree -p ui` unchanged. Ships with two day-1
  e2e gates (the vote combines; the gate bites / anti-`Skip`-regression) per the
  CLAUDE.md non-negotiable + the `v3-volatility-forecaster-noop` precedent. The
  cross-cutting "composites are `Strategy`s, not registry special-cases" rule is
  recorded in [architecture/02-strategy-registry.md](architecture/02-strategy-registry.md).
- 2026-06-21 (architect): **ADR-0062 — forward-plan read seam** (feature
  `advisor-forward-plan`, single-coin-advisor pivot F6). Recorded in the
  canonical ADR registry ([architecture/adr/README.md](architecture/adr/README.md)
  + `architecture/adr/0062-forward-plan-read-seam.md`). The F6 forward
  buy/sell plan (a CONDITIONAL, reactive, rule-driven decision surface —
  current stance + standing rules + €200 projected sizing, explicitly NOT a
  price forecast) reaches `ui` via a NEW read-only sibling trait
  `strategy::PlanDescribe` resolved AGENT-SIDE at the `ForwardCommand::Launch`
  boundary from the SAME `build_registry_for(Some(&cfg))` registry the F5
  hot-swap runs (plan↔engine consistency by construction, R7), mirrored to
  `ui` as a `core`-typed `agent::config::ForwardPlan` over a second mpsc
  `RunHandles.forward_plan_rx` (symmetric with the F5 `forward_rx`) — so
  `cargo tree -p ui` stays unchanged (extends the ADR-0059 `BakeoffReport`
  mirror + the ADR-0060 §D6 launch seam). The horizon is display-only
  metadata; **F5 stays byte-identical** (no self-terminate; the forward run
  remains open-ended). Anchor-neutral — `verify_anchors.sh` stays 119/119 by
  construction. **Folded-in honest-spec fix (same pass):** the § Cross-cutting
  invariants layering bullet (invariant 3) named a **phantom `models` crate**
  that does not exist; corrected to the real forbidden-edge set
  `strategy` / `exec` / `forecast` / `llm` and annotated with the actual
  workspace crate set (the ML/DL work lives in `crates/forecast` +
  `crates/features`, not a `models` crate). The stale `models` placeholder
  still lingers in three section-file bodies (00-overview.md crate tree,
  06-ui-and-cockpit.md § App layout, 01-data-flow.md dep mermaid) +
  12-forecast-overlay.md — flagged for a section-file sweep (out of scope for
  this feature; tracked as a spec-auditor item).
- 2026-05-16 (architect): Phase 1A finalised. `architecture.md`
  compressed from 345 lines (a mixed registry + 12 long historical
  HTML preamble comments + sprawl of "_Migrated to..._" pointer
  blocks) to a clean index of cross-cutting invariants + section
  registry + ADR registry + working-with-this-index instructions.
  The HTML preamble comments (Lumen Phase 1, v1.5b multi-venue,
  journal-transactions-metadata, tape-row-audit-modal,
  live-cockpit-unified, real-mtm-unrealized-pnl, frontend↔backend
  interfaces) are now redundant — each one's substance lives in
  the corresponding ADR or feature-folder changelog. Applied three
  folded-in fixes from the same pass: D1 (audit-sink rule prose in
  01-data-flow.md reworded to "audit imports nothing from sibling
  crates" — the inverse-import direction that is actually true);
  D2 (UI isolation rule in 06-ui-and-cockpit.md restated without
  carveouts and with bootstrap location named — `agent`); D3 (two
  missing reflection edges added to the 01-data-flow.md edge
  table: `exec → reflection` write and `reports → reflection`
  read).
- 2026-05-13 (architect): Phase 1A initial split shipped — 12
  section files under `architecture/`, 26 numbered ADRs under
  `architecture/adr/`, plus a thin index in this file. Final
  architecture.md size after Session 12: ~350 lines (vs 5,635
  original = -94% reduction). The 976-line per-feature changelog
  was archived since every entry's substance is already in the
  corresponding ADR. Sessions 1–12 dev-notes linked from
  `spec/dev-notes/phase-1a-*.md`.
