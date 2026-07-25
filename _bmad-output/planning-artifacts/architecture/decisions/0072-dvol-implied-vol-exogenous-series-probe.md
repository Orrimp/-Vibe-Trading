---
adr: 0072
title: DVOL implied-vol exogenous-series probe — the as-of/leak-free join + the single-coin v0.dvol_regime arm
status: accepted
date: 2026-06-26
supersedes: none
superseded-by: none
extends: 0058
---

# ADR-0072: DVOL implied-vol exogenous-series probe + the `v0.dvol_regime` arm

## Context

The single-coin advisor bakes off price-only and derivatives-positioning signals
today; the active-edge hunt CONCLUDED 2026-06-08 ("ship passive") across
price/OHLCV + derivatives-positioning + on-chain. The one untested orthogonal
channel named in `spec/backlog.md` is **options / implied-vol (Deribit DVOL)** —
forward-looking option-market expectations, structurally orthogonal to the
realized-price tape. The analyst scoped it (`spec/advisor-options-impliedvol-probe/feature.md`,
2026-06-26) and reached a **FEASIBLE** PIT verdict — the OPPOSITE of the on-chain
net-flow channel (which died at the PIT gate because the vendor *disclaims*
mutable history). DVOL is a free, no-auth, past-only daily series computed live
from the Deribit option order book (a variance-swap construction with no
address-relabeling substrate) — structurally the **same PIT class as the perp
basis this repo already certified clean** (`crates/backtest/src/basis_data.rs`).
The operator approved a FULL build (honest coverage; a FRAGILE/null result is the
expected, valid, shippable outcome — it closes the vol channel with finality).

Six facts shape this decision (all verified in code via CodeGraph, 2026-06-26):

1. **The bake-off arm path is the `v0.*` string-dispatch path, NOT the
   cross-sectional / param-sweep machinery.** Bake-off arms are a
   `Vec<StrategyId>` of `v0.*` ids (`crates/backtest/src/bakeoff/mod.rs:363`
   `default_field`) dispatched by string match in `run_scenario`
   (`crates/backtest/src/engine.rs:945`+), each built as a `Box<dyn Strategy>`
   and run through the single-coin bar-loop `crates/backtest/src/scenarios/sma_composed_run.rs`.
   The analyst's proposed `ScoreSource::DvolRegime` + `SweepFamily` registration
   (`crates/strategy/src/cross_sectional/config.rs:56`, `crates/backtest/src/bakeoff/sweep.rs:77`)
   targets the **cross-sectional rank / hyperparameter-sweep** subsystems — the
   wrong machinery for a single-coin arm benchmarked vs buy-and-hold. **This ADR
   corrects that seam read.**

2. **The signal DSL cannot express a per-bar exogenous series.** The composed-DSL
   `Expr` (`crates/strategy/src/composed/ast.rs:48`) reads only `Indicator` /
   `BarField` / static `Param` scalar / `Literal` / arithmetic / boolean rules —
   there is no per-bar exogenous-series term, and `Param` is a static `[params]`
   value, not a time-varying series. A DVOL regime weight (per-bar, from an
   external daily series) is therefore inexpressible as a DSL `ComposedStrategy`.
   The arm MUST be a hand-written `impl Strategy`.

3. **The single-coin bar-loop has no sidecar-injection seam today.**
   `sma_composed_run::run` takes only `bars_override` (OHLCV) + `composed_toml_override`
   (a DSL recipe), and the loop calls `registry.on_bar(&bar)` with only a `Bar`
   (`crates/backtest/src/scenarios/sma_composed_run.rs:506`, `crates/strategy/src/traits.rs:10`).
   The cross-sectional basis arm injects its series via `MomentumStrategy::with_funding`
   (the D-BR.3 sidecar carrier, `crates/strategy/src/cross_sectional/momentum.rs:481`) —
   a channel the single-coin path does not have. A small, deliberate extension is
   needed.

4. **The as-of join + REVISION-pin + no-look-ahead falsifier are ~80% reusable
   verbatim.** `basis_as_of` (`crates/backtest/src/basis_data.rs:403`) is a thin
   wrapper over ADR-0058's `PitSeries::as_of_value`; `BasisDataSource::load`
   SHA-verifies the corpus or refuses (`basis_data.rs:45` `EXPECTED_BASIS_REVISION_SHA`);
   `no_look_ahead_falsifier` (`basis_data.rs:553`) future-shifts the series and
   asserts the join result changes. All clone to DVOL with the cadence lifted from
   1h-basis to 1-day-DVOL.

5. **The fetcher + diag spike have exact templates.**
   `crates/data/src/bin/fetch_binance_premium.rs` (`PremiumFetcher` trait +
   `HttpPremiumFetcher` + `MockFetcher` + `paginate_premium` + `write_revision_manifest`)
   is the deterministic/idempotent fetcher shape; `crates/data/examples/basis_diag.rs`
   is the read-only IC-spike shape (with a `--leak-check` falsifier).

6. **The frozen gate + anchor contract are untouched.** The bake-off path sets
   `write_report=false` for every arm (`crates/backtest/src/bakeoff/mod.rs:712`,
   ADR-0059) → no anchored body is written → `scripts/verify_anchors.sh` stays
   119/119 (verified PASS at design time). `classify_verdict` (5-signal
   weakest-link bootstrap), the bands, and the `v0.buyhold` benchmark are FROZEN.

Why now: this is the last orthogonal channel on the board, the one forward-looking
signal (vs every prior channel being a transform of the realized past), and the
cheapest remaining coverage-per-dollar toward an honest "active ≤ passive across
ALL reachable channels" terminal record.

## Decision

Build the DVOL probe as a **single-coin time-series long/flat bake-off arm
`v0.dvol_regime`**, scored by the FROZEN robustness gate and benchmarked vs
`v0.buyhold`, with a pre-registered parameter-light signal and two day-1 gates.

### D1. The DVOL corpus + fetcher

A new `data/deribit-dvol/` corpus (parquets gitignored, `REVISION.toml` tracked),
laid out per-symbol/per-year like `data/binance-basis/`. Schema:
`day_open_ts_ms` Int64, `day_close_ts_ms` Int64, `dvol_open/high/low/close`
Float64 — the signal consumes `dvol_close` ONLY; OHL are banked for provenance.
A new `crates/data/src/bin/fetch_deribit_dvol.rs` (template clone of
`fetch_binance_premium.rs`) with a `DvolFetcher` trait (I/O behind a trait), an
`HttpDvolFetcher` + `MockDvolFetcher`, a `paginate_dvol`, and
`write_revision_manifest`. Endpoint LOCKED:
`public/get_volatility_index_data`, `currency ∈ {BTC, ETH}`, `resolution=43200`
(12h candles → daily close), `/public/` no-auth, Deribit API primary (history to
2021-04), CryptoDataDownload CSV as corroboration/fallback (manifest metadata
only, not a second loader). Deterministic + idempotent; the only clock is the
`REVISION.toml` `fetched_at` metadata label (not hashed into any anchored body).

### D2. The as-of/leak-free join — clone the certified basis seam

A new `crates/backtest/src/dvol_data.rs`, near-exact clone of `basis_data.rs`:
`DvolDataSource::load` (SHA-verify the corpus vs `EXPECTED_DVOL_REVISION_SHA` or
refuse to run) + `dvol_as_of(series, bar_open_ts_ms) -> Vec<Option<Decimal>>`,
which delegates to **ADR-0058 `PitSeries::as_of_value`** (rightmost-at-or-before,
LOCF, `None` warm-up, `Decimal` no-f64-roundtrip). The as-of KEY is
`day_close_ts_ms` (the instant the daily close is FULLY observed); an hourly bar
opening at `t` sees ONLY the most-recent DVOL close with `close_ts ≤ t` — e.g. a
bar opening 2023-05-02T00:00Z sees the 2023-05-01 close, never the 05-02 close.
The `no_look_ahead_falsifier` (`basis_data.rs:553`) is cloned verbatim into
`dvol_data.rs` tests (the join-layer leak-check).

### D3. The pre-registered signal `v0.dvol_regime` (LOCKED, no search)

Per coin `s ∈ {BTC, ETH}`, daily grid, strictly causal:
`weight_t = 1` (HOLD) iff `dvol_t < median(last W=30 DISTINCT daily closes
available as-of t)`, else `0` (CASH). W = 30 daily closes (horizon-matched to
DVOL's own 30-day gauge). Cut = the trailing MEDIAN (self-normalizing,
parameter-light — nothing to argmax). Ties (`dvol_t == median`) resolve to CASH.
Warm-up (< 30 distinct closes) → weight = 1 (HOLD = the benchmark behavior, so the
arm only ever subtracts exposure in a confirmed stress regime and never diverges
from buy-and-hold before the signal is defined). The "not rising sharply" and
"33rd-percentile" clauses from the analyst's §2.2 are DROPPED (each adds a tunable
knob, voiding "nothing to tune"). The median is `Decimal`-exact (even W=30 = mean
of the 15th/16th order statistics). This is the ONLY signal; any sensitivity sweep
is a separate, explicitly-labeled robustness check, never a crowning search.

### D4. The arm = a hand-written `DvolRegimeStrategy: Strategy`

`crates/strategy/src/dvol_regime.rs` — `DvolRegimeStrategy`, constructed with the
pre-resolved as-of DVOL `Vec<Option<Decimal>>` (the strategy does NO joining — it
is pure + unit-testable against a synthetic vector). `on_bar` maintains a bar
cursor + a ring of the last-W DISTINCT daily closes (push only when the as-of
close changes vs the prior bar — dedups the 24× intraday forward-fill into one
daily sample), computes the `Decimal` median, and emits `SignalKind::Buy` on a
0→1 weight transition while flat / `SignalKind::Sell` on a 1→0 transition while
long. Long-only (`short_enabled=false`) rides the existing
`sma_composed_run.rs:534` clamp; sizing is the bar-loop's `FixedFractionSizer(0.10)`,
identical to every `v0.*` arm.

### D5. The registration seam — bake-off `v0.*` path + a new `ScenarioConfig` exogenous override

`v0.dvol_regime` joins `default_field()` (`bakeoff/mod.rs:363`) as one additive
line (NOT a `ScoreSource`/`SweepFamily` variant — wrong machinery, D2/fact-1). A
new `run_scenario` match-arm `"v0.dvol_regime"` (`engine.rs:945`+), structured
like the `v0.obv` arm (`engine.rs:1767`), builds the `DvolRegimeStrategy` from a
NEW `ScenarioConfig.dvol_override: Option<Vec<Option<Decimal>>>` field (default
`None`), mirroring the existing `funding_override`/`basis_override` fields
(`engine.rs:1057`). All existing arms set `dvol_override: None` → byte-identical;
the field is read ONLY by the new arm. The bake-off loop
(`run_bakeoff`, `bakeoff/mod.rs:688`) resolves the as-of vector for BTC/ETH and
threads it in; for any other coin (or a DVOL load failure) it FILTERS
`v0.dvol_regime` out of `field` for that run — the arm is ABSENT from the
leaderboard, never crashed and never degenerate (D8).

### D6. Day-1 gates — BOTH mandatory (CLAUDE.md non-negotiable)

(a) A baseline-equity-divergence e2e
(`crates/backtest/tests/dvol_regime_divergence_end_to_end.rs`, pattern
`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`): on a fixture whose
DVOL crosses its 30d median (flipping the weight), assert
`|equity(v0.dvol_regime) − equity(v0.buyhold)| ≥ 1 bp` at the final bar — a no-op
arm (weight computed, never applied) yields equal equities and FAILS, catching the
`v3-volatility-forecaster-noop-fix` class on day 1. (b) An arm-level no-look-ahead
leak-check (`crates/backtest/tests/dvol_regime_leak_check.rs`, the `basis_data.rs:553`
falsifier lifted to the arm/equity level): build the arm with causal vs
future-shifted DVOL and assert the decision sequences (and equity) DIFFER — a
coincidence proves a leak. Both layers (join + wired arm) are tested because the
v3 precedent showed a clean join and a broken application both pass unit tests.

### D7. Anchor safety + frozen gate

The arm runs `write_report=false` (`bakeoff/mod.rs:712`) → no `spec/*/reports/`
body → 119/119 anchors green before AND after (additive only). `classify_verdict`
+ bands + the `v0.buyhold` benchmark are FROZEN. Existing arms are byte-identical
(one `default_field` entry, one match-arm, one `Option`-defaulted `ScenarioConfig`
field read only by the new arm); the `default_field_unchanged_additive_contract`
test is extended to assert it. Any future banked DVOL coverage surface (BTC/ETH)
is a separate ADR-0038 anchor-additive amendment, not this design; the 9 anchor
SHAs in `spec/anchors.toml` are untouched.

### D8. The BTC+ETH universe restriction — arm ABSENT, never a crash

DVOL exists only for BTC + ETH (no liquid altcoin options → no index). A
`dvol_supported(symbol)` predicate (membership in `{BTCUSDT, ETHUSDT}`) gates the
loop: supported + load-ok → thread `dvol_override`; otherwise drop the arm from
`field` so a non-BTC/ETH bake-off still completes with the other 9 arms (preferred
over a per-arm `UnsupportedDataSource` error, `engine.rs:951`). Honest leaderboard
copy notes "DVOL-regime arm available for BTC/ETH only".

## Alternatives considered

- **Register as `ScoreSource::DvolRegime` + a `SweepFamily` variant** (the analyst's
  §3.3) — REJECTED. Those are the cross-sectional-rank and hyperparameter-sweep
  subsystems; the operator wants a single-coin arm benchmarked vs buy-and-hold in
  the bake-off, whose home is the `v0.*` string-dispatch `default_field` path. Using
  the cross-sectional seam would force a 2-name cross-section (meaningless rank
  noise) and misroute the arm entirely.
- **Express the arm as a DSL `ComposedStrategy`** (a TOML recipe like `btc_obv.toml`)
  — REJECTED, impossible: the DSL `Expr` (`ast.rs:48`) has no per-bar exogenous-series
  term and `Param` is a static scalar. A per-bar DVOL weight cannot be written in the
  DSL.
- **Inject DVOL via the cross-sectional `with_funding` sidecar carrier** (D-BR.3) —
  REJECTED for the single-coin path: that channel belongs to `MomentumStrategy`
  (cross-sectional); the single-coin bar-loop does not consume it. A dedicated
  `ScenarioConfig.dvol_override` (mirroring the precedented `funding_override`/
  `basis_override` fields) is the minimal, in-pattern extension.
- **A vol-risk-premium arm** (implied DVOL minus trailing realized vol) — REJECTED
  as the probe signal: it re-introduces the GARCH-σ realized-vol machinery the
  program retired (muddying "this is a NEW channel") and is a carry/relative-value
  construction closer to the retired funding-carry family. Named as a possible
  follow-on IFF the regime arm shows life (it likely will not, on the honest prior).
- **A "not rising sharply" / quantile-threshold cut** — REJECTED: each adds a tunable
  knob and a second comparison, voiding the parameter-light "nothing to tune"
  guarantee that makes the honest-coverage claim defensible. The rule is exactly
  `dvol < trailing-median`.
- **A DVOL regime OVERLAY on the crowned arm** (flatten the crowned strategy in
  stress regimes) instead of a standalone arm — DEFERRED. The standalone arm is the
  cleaner coverage test (does the IV channel carry its OWN edge); the overlay is a
  larger, different experiment and a possible follow-on.

## Consequences

- **Look-ahead is structurally prevented at the join layer** (ADR-0058 `PitSeries`)
  and falsifier-checked at BOTH the join and the wired-arm layers (D6). If a future
  edit breaks causality, the leak-check FAILS.
- **The no-op class is caught on day 1** by the divergence e2e (D6a) — the
  CLAUDE.md non-negotiable. A regime weight that is computed but never flattens the
  position cannot pass.
- **Anchors stay 119/119** (`scripts/verify_anchors.sh`); the
  `default_field_unchanged_additive_contract` test guards arm-additivity; the
  corpus is SHA-pinned (`EXPECTED_DVOL_REVISION_SHA`) and the loader refuses
  unverified data.
- **A FRAGILE verdict is the success condition for honest coverage** — it closes the
  options/IV channel with the finality the on-chain probe gave its channel. Do NOT
  tune to escape Fragile (voids the pre-registration; the gate exists to prevent
  exactly that cherry-pick).
- **If a future iced/Deribit audit uncovers a DVOL revision/restatement policy**, the
  FEASIBLE verdict flips and this probe is re-evaluated — nothing found 2026-06-26
  suggests one (three independent free mirrors serve a consistent un-disclaimed
  history).
- **No new anchor SHA in `spec/anchors.toml`** is added or changed by this design.

## Changelog
- 2026-06-26 (architect): initial accept. Corrects the analyst's `ScoreSource`/
  `SweepFamily` seam read → single-coin `v0.dvol_regime` arm on the bake-off `v0.*`
  path; hand-written `DvolRegimeStrategy` (DSL cannot express a per-bar exogenous
  series); DVOL injected via a new `ScenarioConfig.dvol_override` (mirroring
  `funding_override`/`basis_override`). Reuses `basis_data.rs` as-of join + REVISION
  loader + no-look-ahead falsifier (ADR-0058 `PitSeries`), `fetch_binance_premium.rs`
  fetcher template, `basis_diag.rs` spike. Signal LOCKED: W=30 daily, trailing-median
  cut, tie→cash, warm-up→hold. Two day-1 gates (divergence ≥1bp + leak-check)
  mandatory. `write_report=false` → 119/119 anchor-safe; frozen gate; BTC+ETH only
  (arm absent otherwise). Extends ADR-0058.
