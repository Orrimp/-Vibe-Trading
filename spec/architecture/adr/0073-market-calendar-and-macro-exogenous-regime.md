---
adr: 0073
title: Market-calendar layer + macro-exogenous-regime arm seam
status: accepted
date: 2026-06-26
supersedes: none
superseded-by: none
extends: 0040, 0058, 0059
---

# ADR-0073: Market-calendar layer + macro-exogenous-regime arm seam

## Context

The Single-Coin Investment Advisor probes one named-but-untested orthogonal
channel — **cross-asset / macro regime** (DXY / S&P / rates) — as fresh-channel
probe #4 (`spec/advisor-crossasset-macro-regime/feature.md`). Two code-confirmed
obstacles forced this design, and the operator funded the **durable** resolution
of both (NOT the relaxed bypass):

1. **The Yahoo 95% coverage gate assumes 24/7 data.**
   `expected_bars_for_range` (`crates/data/src/yahoo.rs:984`) is a pure
   wall-clock division; for a `1d` window it computes `expected = calendar_days`.
   The gate (`MISSING_DATA_THRESHOLD_PCT = 95`, `yahoo.rs:56`; enforced at
   `yahoo.rs:338-352`) then demands ≥95% of *calendar* days have a bar. Equities
   / index / FX / rates trade ~5/7 calendar days (~71%) plus holidays, so a
   multi-month `1d` macro load trips `YahooError::MissingData`. The code itself
   names the fix as the deferred "v0.2.0 market-calendar layer"
   (`yahoo.rs:982-984`). This is the funded infra.
2. **There is no exogenous-series seam.** The advisor bake-off reads the coin
   from Binance-hourly (`bakeoff/mod.rs:93`), preloads it once, and threads it as
   `bars_override` to every arm. The macro signal is Yahoo-daily — a cross-corpus,
   cross-cadence join. `merge_symbols` (`replay_feed.rs:281`) is single-corpus,
   and `ComposedStrategy::on_bar` (`node.rs:1395`) has no `bar.symbol` demux, so
   a merged coin‖macro stream would make a composed arm try to *trade* the macro
   ticker. The macro must arrive as an arm-scoped **side input**, not tradeable
   bars.

Because this introduces a new `crates/data` domain seam (`MarketCalendar`) that
sits on the **anchor-feeding** Yahoo coverage path, and a new bake-off arm
plumbing channel, the design is recorded here. Two existing primitives make the
build smaller than feared and are load-bearing to the decision: ADR-0058's
`core::pit::PitSeries<T>` (a look-ahead-impossible as-of join) and the fact that
`run_buyhold_path` (`bakeoff/buyhold.rs:38`) is a **pure equity-path function**,
not a `Strategy`.

## Decision

Introduce a durable, ticker-classified **`MarketCalendar`** seam in `crates/data`
that makes the Yahoo coverage gate calendar-aware **without changing
`load_cached`'s public signature** (so crypto coverage is byte-identical), and
implement the `v0.macro_riskon` probe as a **hand-written exogenous overlay** on
the pure buy-and-hold path — reusing `core::pit::PitSeries<bool>` for the
look-ahead-free daily→hourly LOCF join — threaded into one arm via a new optional
`ScenarioConfig` channel. No DSL change, no new dependency, no new crate.

### D1. `MarketCalendar` lives in `crates/data`; the calendar is derived FROM the ticker — `load_cached`'s signature is unchanged

A new `crates/data/src/calendar.rs` defines `enum MarketCalendar { Crypto24x7,
UsEquity }` with `trading_days_in_range(self, start_ms, end_ms) -> usize`
(Crypto24x7 = wall-clock day count; UsEquity = Mon–Fri in range minus a static
`US_MARKET_HOLIDAYS` weekday set) and `classify_ticker(&str) -> MarketCalendar`
(the 12 crypto-mirror pairs + any unknown ticker → `Crypto24x7`; leading `^`,
`=F`/`=X`, or `DX-Y.NYB` → `UsEquity`). `load_cached(ticker, …)`
(`yahoo.rs:266`) already receives the ticker, so it derives the calendar
internally (`classify_ticker(ticker)`) and calls a new
`expected_bars_for_calendar(cal, interval, start, end)` at its step-6 gate
(`yahoo.rs:339`) — **the public signature of `load_cached` does not change**, so
all 14 existing call sites compile and behave identically. `crates/data` is the
right home (ADR-0041 layering): it already owns the Yahoo corpus + the coverage
gate; the calendar is a domain primitive beside the thing it serves.

### D2. The gate becomes calendar-aware ADDITIVELY — `expected_bars_for_range` stays byte-identical; crypto coverage is provably unperturbed

`expected_bars_for_range(interval, start, end)` is **not modified** — it is
pinned by `expected_bars_for_range_arithmetic` + `coverage_threshold_95_pct` +
two more tests (`yahoo.rs:1187`/`:1155`), and remains the `Crypto24x7`
implementation. A NEW `expected_bars_for_calendar(cal, interval, start, end)`
returns `cal.trading_days_in_range(start, end)` for `Days1` and delegates to
`expected_bars_for_range` for `Hours1`/`Minutes1`. For `cal == Crypto24x7` it is
**provably equal** to `expected_bars_for_range` (Crypto24x7's
`trading_days_in_range` IS the wall-clock day count) — a required unit test
(T-CAL). Since `classify_ticker` returns `Crypto24x7` for every crypto ticker
and every unknown ticker, and the Binance coin loader never calls the Yahoo gate
at all, **no crypto coverage decision, no `LoadedBars`, and no anchored report
changes.** The holiday set is a *conservative lower bound on expected* (the gate
is a ≥95% floor, not an equality), so it need not be exhaustive — it can only
make the gate more lenient, never reject valid data. The calendar layer is
proven inert (`scripts/verify_anchors.sh → 119/119`) in its own commit BEFORE
the macro arm exists.

### D3. The macro arm is a HAND-WRITTEN overlay on the pure buy-and-hold path — NOT a `ComposedStrategy` / DSL change

`run_buyhold_path` (`bakeoff/buyhold.rs:38`) is a pure equity-path function that
marks bars to market by timestamp; it never routes through
`ComposedStrategy::on_bar`. So the `v0.macro_riskon` arm is a sibling pure
function `run_macro_gated_buyhold_path(bars, regime: &PitSeries<bool>, capital)`
in the same module — buy-and-hold gated by a per-timestamp regime mask (flat→ON
buys, ON→flat sells, `Decimal`-only). The `ComposedStrategy`-has-no-demux problem
**never arises** because the overlaid benchmark is itself hand-written. The macro
series is computed once by a new `crates/backtest/src/macro_regime.rs`
(`load_macro_regime_series(yahoo_root, range) -> PitSeries<bool>`, using the
UNCHANGED `load_cached` read path) and reduced to a daily regime bool. **The DSL
auxiliary-series alternative (feature § Q-MACRO-1 (b)) is rejected for v0.1**:
the operator's stated intent is a one-shot honest-coverage probe (single arm,
gold dropped), and (b) would force the arm into a `ComposedStrategy` it has no
reason to be plus a parser/typecheck/`node.rs` grammar change. **Fork recorded:
IF a future program wants ≥2 exogenous arms (options-IV, more macro legs), the
DSL auxiliary-series extension becomes the durable seam** and the future
architect inherits that decision from here — v0.1 (a) spawns NO v0.2 cleanup
commitment.

### D4. The daily→hourly LOCF join reuses ADR-0058 `core::pit::PitSeries<bool>` — look-ahead is unrepresentable, not falsifier-policed

The as-of join (an hourly coin bar at `t` sees only the most-recent macro daily
regime with `close_ts ≤ t`; weekend/holiday gaps carry the prior close forward)
is exactly `regime.as_of_value(TimestampMs(bar.open_ts.unix_millis()))` — the
ADR-0058 primitive whose ONLY query returns a record with `ts ≤ query` and which
has no API that returns `ts > query` (guarded by `crates/core/tests/pit_compile_fail.rs`).
**No hand-rolled `partition_point`, no `scripts/`-level look-ahead lint** — the
type makes the leak a compile impossibility. This is the durable consumer
ADR-0058's Consequences named ("a fresh data channel opens"). Warm-up
(`as_of_value → None`) and the SMA(50) pre-roll resolve to FLAT (no position
until the regime is defined). LOCF is cadence-agnostic, so the resample knob
(H4/D1) needs no special-casing.

### D5. Arm registration + the second optional `ScenarioConfig` channel — anchor-additive, `write_report = false`

The arm id `v0.macro_riskon` is registered via a new
`BakeoffConfig::default_macro_field()` (beside `default_field`/
`default_ensemble_field`, `bakeoff/mod.rs:363`), extended into `advisor_field()`
(`runner.rs:53`); `advisor_field_arm_count()` (`runner.rs:67`) auto-tracks. The
macro series is threaded through a NEW
`ScenarioConfig.macro_regime_series: Option<PitSeries<bool>>` field — `None` for
every existing arm and every CLI/Lab/anchor path (struct-update default,
identical contract to `composed_toml_override`, `engine.rs:294`); set to
`Some(..)` ONLY for the macro arm by `run_bakeoff`, which preloads the series
once beside the coin-bar preload. The arm runs `write_report = false` (ADR-0059
advisor-arm convention) — **no report body is written, so the addition is
anchor-additive by construction.** The FROZEN robustness gate + buy-and-hold
benchmark score the macro arm identically to every other arm (FRAGILE ⇒
ineligible to crown). This is NOT a band proposal and NOT a `classify_verdict`
change.

### D6. Verification floor — the day-1 baseline-equity-divergence gate IS required (overlay), plus a no-op control + a leak-check

Unlike ADR-0058 (where equity must NOT move), the macro arm introduces a decision
variable (the regime gate suppresses the position), so the CLAUDE.md "day-1
baseline-equity-divergence e2e test for every strategy overlay" gate **applies
and is mandatory** (the `v3-volatility-forecaster-noop-fix` 2026-05-22 precedent:
a `scale`/mask computed but never applied is exactly the failure unit tests +
anchored reports miss). `crates/backtest/tests/macro_regime_overlay_end_to_end.rs`
ships, all synthetic (no network): (S1) regime-flip ⇒ gated equity diverges from
un-gated buy-and-hold by ≥1 bp; (S2) always-risk-ON ⇒ gated ≈ buy-and-hold (the
no-op-when-never-gating control — both directions mandatory); (S3) forward-shift
the regime by 1 day ⇒ gated equity changes (`assert_ne!` leak-check, pins the arm
routes through `core::pit`); (S4) warm-up `None` ⇒ FLAT. The corpus add and the
calendar layer each re-prove `scripts/verify_anchors.sh → 119/119`.

## Alternatives considered

- **A macro-only relaxed / bypass coverage path (feature § Q-MACRO-2 (b))** —
  rejected: cheaper but carries a documented v0.2.0 cleanup commitment and is a
  carve-out, not the durable fix the code itself names; the operator funded the
  durable `MarketCalendar`. (Recorded as the explicit "minimum-viable carve-out"
  fallback, NOT taken.)
- **Change `expected_bars_for_range`'s arithmetic in place** — rejected: it is
  pinned by 4 tests and is the correct Crypto24x7 behaviour; mutating it risks an
  anchor delta for zero benefit. The calendar-aware count is added additively.
- **Data-driven "expected = distinct actual-bar dates" coverage** — rejected: it
  makes the gate tautological (coverage can never fail), destroying the
  gap-detection the gate exists for. The weekday-minus-holidays lower bound keeps
  the gate meaningful while absorbing holiday-set imprecision via the 95% floor.
- **Pass a `MarketCalendar` parameter through `load_cached`'s signature** —
  rejected: it would touch all 14 call sites and risk a crypto-path delta; the
  ticker already determines the calendar, so internal derivation is both smaller
  and provably crypto-byte-identical.
- **DSL exogenous/auxiliary-series extension to `ComposedStrategy`
  (Q-MACRO-1 (b))** — rejected for v0.1: large parser/typecheck/`node.rs` change
  for a one-shot probe whose overlaid benchmark (`run_buyhold_path`) is not even a
  `ComposedStrategy`. Becomes the durable seam only if ≥2 exogenous arms are
  funded (fork recorded in D3).
- **Hand-roll the daily→hourly as-of join with `partition_point`** — rejected:
  ADR-0058 shipped `core::pit` precisely to make this unrepresentable; a
  hand-roll would re-open the look-ahead surface the project closed and require a
  `scripts/` lint backstop.
- **Merge coin‖macro into one stream via `merge_symbols`** — rejected: it is
  single-corpus (coin=Binance, macro=Yahoo) and would feed the macro ticker to
  the arm as tradeable bars (`node.rs:1374` stamps the incoming `bar.symbol`).
  The macro must be an arm-scoped side input.

## Consequences

If this design is violated:

- **The calendar perturbs crypto coverage** → an anchored Yahoo-fed surface
  silently regenerates. Guarded by: T-CAL (Crypto24x7 ≡ `expected_bars_for_range`
  unit equivalence), the calendar-inert commit checkpoint (M-CAL-4:
  `verify_anchors → 119/119` before the arm exists), and the final M-CLOSE-1
  re-proof. The Binance coin loader never calls the Yahoo gate, so the bake-off
  coin path is structurally out of scope.
- **The macro overlay no-ops** (regime computed, position never gated) → caught
  by the mandatory S1 divergence + S2 no-op-control e2e
  (`crates/backtest/tests/macro_regime_overlay_end_to_end.rs`), the
  CLAUDE.md non-negotiable for every overlay.
- **A look-ahead leak on the as-of join** → unrepresentable via
  `core::pit::PitSeries` (no `ts > query` accessor; `crates/core/tests/pit_compile_fail.rs`
  trybuild fixture), with the S3 forward-shift falsifier pinning that THIS arm
  routes through the primitive rather than a future hand-rolled bypass.
- **The arm writes a report body** → would break anchor-additivity; guarded by
  `write_report = false` on the bake-off arm (ADR-0059) + M-CLOSE-1
  (`verify_anchors → 119/119`).
- **The gate / bands / benchmark drift** → forbidden; the macro arm is scored by
  the FROZEN `classify_verdict` / 5-signal weakest-link bootstrap exactly as every
  other arm (this ADR changes neither).

Mechanical checks: `crates/data` `calendar` + `expected_bars_for_calendar` unit
suites (T-CAL); `crates/backtest` `macro_regime` loader unit suite +
`macro_regime_overlay_end_to_end.rs` (S1–S4); `crates/core` `pit_compile_fail.rs`
(reused leak guard); `scripts/verify_anchors.sh` (119/119 at M-CAL-4, M-FETCH-1,
M-CLOSE-1). Full design + the sequenced plan:
[`spec/advisor-crossasset-macro-regime/feature.md` § Design](../../v1/advisor-crossasset-macro-regime/feature.md#design)
and [`tasks.md`](../../v1/advisor-crossasset-macro-regime/tasks.md).

## Changelog
- 2026-06-26 (architect): initial accept. D1 `MarketCalendar` in `crates/data`,
  calendar derived from ticker → `load_cached` signature unchanged → crypto
  byte-identical; D2 additive `expected_bars_for_calendar`, `expected_bars_for_range`
  untouched, Crypto24x7 equivalence is anchor-safe; D3 hand-written overlay on the
  pure `run_buyhold_path` (NO DSL change — fork to DSL recorded for ≥2 future
  exogenous arms); D4 LOCF join = ADR-0058 `core::pit::PitSeries<bool>` (look-ahead
  unrepresentable); D5 `v0.macro_riskon` registration via `default_macro_field()`
  + new optional `ScenarioConfig.macro_regime_series`, `write_report=false`
  anchor-additive, gate FROZEN; D6 mandatory day-1 divergence + no-op control +
  leak-check, anchors 119/119 re-proved at 3 checkpoints. extends ADR-0040 (Yahoo
  corpus/coverage), ADR-0058 (PIT as-of primitive), ADR-0059 (bake-off arm home +
  write_report=false convention).
