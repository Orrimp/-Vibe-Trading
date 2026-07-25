---
slug: advisor-crossasset-macro-regime
status: in-progress
owner: developer
updated: 2026-06-26
---

# Tasks — advisor-crossasset-macro-regime

Sequenced per the dependency chain: **market-calendar layer FIRST (the F-2
unblock) → macro fetch/pin → exogenous-series seam + LOCF join → the
`v0.macro_riskon` arm → day-1 divergence + leak-check → bake-off run →
anchors/close.** Design: [feature.md § Design](feature.md#design).
ADR: [ADR-0073](../../../_bmad-output/planning-artifacts/architecture/decisions/0073-market-calendar-and-macro-exogenous-regime.md).

Anchor invariant (CLAUDE.md non-negotiable): `bash scripts/verify_anchors.sh` →
**119 / 119** at every checkpoint. The arm runs `write_report = false`
(anchor-additive). The gate / bands / benchmark stay **FROZEN**.

---

## Phase 1 — The market-calendar layer (the funded unblock, Q-MACRO-2 (a))

- [ ] **M-CAL-1** — New module `crates/data/src/calendar.rs`: `MarketCalendar`
  enum (`Crypto24x7` | `UsEquity`), `trading_days_in_range(self, start_ms,
  end_ms) -> usize` (Crypto24x7 = wall-clock day count; UsEquity = Mon–Fri minus
  `US_MARKET_HOLIDAYS` weekday set), and `classify_ticker(&str) -> MarketCalendar`
  (12 crypto pairs + unknown → `Crypto24x7`; leading `^` / `=F` / `=X` /
  `DX-Y.NYB` → `UsEquity`). Wire `pub mod calendar;` into `crates/data/src/lib.rs`.
  — _acceptance: module compiles; `classify_ticker("BTC-USD")==Crypto24x7`,
  `classify_ticker("^GSPC")==classify_ticker("DX-Y.NYB")==classify_ticker("^TNX")==UsEquity`._

- [ ] **M-CAL-2** — Add `pub fn expected_bars_for_calendar(cal, interval, start,
  end) -> usize` to `crates/data/src/yahoo.rs`: `Days1` → `cal.trading_days_in_range`;
  `Hours1`/`Minutes1` → delegate to the UNCHANGED `expected_bars_for_range`.
  Leave `expected_bars_for_range` byte-identical (it is pinned by 4 existing
  tests). — _acceptance: `expected_bars_for_calendar(Crypto24x7, Days1, s, e) ==
  expected_bars_for_range(Days1, s, e)` for all ranges (T-CAL); the 4 existing
  `expected_bars_for_range*` tests still pass unchanged._

- [ ] **M-CAL-3** — In `load_cached` step-6 (`yahoo.rs:339`), replace
  `expected_bars_for_range(interval, …)` with
  `expected_bars_for_calendar(classify_ticker(ticker), interval, …)`. **Public
  signature of `load_cached` UNCHANGED.** — _acceptance: all 14 existing
  `load_cached` call sites compile unchanged; the crypto coverage path is
  byte-identical (Crypto24x7 ⇒ same expected)._

- [ ] **M-CAL-4 (CALENDAR INERTNESS RE-PROOF, gating)** — Unit test S5/T-CAL in
  `crates/data` asserting the Crypto24x7 equivalence over a range sweep, AND run
  `bash scripts/verify_anchors.sh` → **119/119** with the calendar layer landed
  but BEFORE any macro arm exists (prove the layer is inert in its own commit).
  — _acceptance: T-CAL green; anchors 119/119; `cargo test -p data` green._

## Phase 2 — Macro corpus: fetch + pin (R1, Q-MACRO-3)

- [ ] **M-FETCH-0 (dry-run + symbol resolution, BLOCKS the rule lock)** — Run
  `cargo run -p data --features yahoo-online --bin fetch_yahoo_klines --
  --tickers '^GSPC,DX-Y.NYB,^DXY,^TNX' --interval 1d --start 2025-12-01 --end
  2026-01-31 --dry-run`, then a real short fetch, to confirm: (a) `DX-Y.NYB`
  returns non-empty quotes (lock it) — else fall back to `^DXY` and record the
  swap in feature.md Changelog BEFORE any result; (b) the `^GSPC`/`^TNX`/dollar
  parquet + REVISION rows write cleanly under literal `^GSPC` etc. directory
  names. — _acceptance: dollar-index symbol LOCKED (primary `DX-Y.NYB`); the 3
  ticker dirs materialize; pre-registration intact (locked before bake-off)._

- [ ] **M-FETCH-1 (human-run, out-of-band — needs network)** — Fetch the 3
  locked tickers at `1d` over `2023-06-01 .. 2026-06-30` (superset-covers the
  bake-off windows + the SMA(50) ~72-day pre-roll). Recipe: feature.md § D2.
  — _acceptance: `data/yahoo/<TICKER>/1d/**` parquet + REVISION.toml rows added;
  `bash scripts/verify_anchors.sh` → 119/119 (corpus add is anchor-additive)._

- [ ] **M-LOAD** — Smoke test: `YahooBarSource::load_cached("^GSPC", Days1,
  2024-01-01, 2024-12-31)` (and the dollar + `^TNX`) returns bars WITHOUT
  tripping `YahooError::MissingData` (this is the F-2 unblock, end-to-end, on
  real data). — _acceptance: each macro `load_cached` succeeds with ~252
  trading-day bars; proves D1 against the live corpus._

## Phase 3 — Exogenous-series seam + the LOCF as-of join (R3, Q-MACRO-1 (a))

- [ ] **M-SEAM-1** — New module `crates/backtest/src/macro_regime.rs`:
  `load_macro_regime_series(yahoo_root, range) -> Result<PitSeries<bool>,
  MacroRegimeError>`. Loads the 3 macro daily series via the UNCHANGED
  `load_cached` read path (over `[start − warmup, end)`), computes per-leg
  trailing SMA (past-only), evaluates the D4 3-AND at the union of macro close
  timestamps, and emits `PitSeries<bool>` via `core::pit::PitSeries::from_sorted`.
  — _acceptance: returns a sorted `PitSeries<bool>`; warm-up timestamps before
  SMA(50) availability are absent/`false`; unit test on a fixed fixture._

- [ ] **M-SEAM-2** — Add `macro_regime_series: Option<core::pit::PitSeries<bool>>`
  to `ScenarioConfig` (`engine.rs:202`). All existing constructors set it `None`
  via struct-update (anchor contract identical to `composed_toml_override`).
  — _acceptance: workspace compiles; every existing `ScenarioConfig` literal has
  `macro_regime_series: None`; CLI/Lab/anchor paths byte-identical._

## Phase 4 — The `v0.macro_riskon` arm (R4, LOCKED)

- [ ] **M-ARM-1** — Add `run_macro_gated_buyhold_path(bars, regime, capital) ->
  (Vec<Decimal>, Decimal)` to `crates/backtest/src/bakeoff/buyhold.rs` beside its
  `run_buyhold_path` sibling: per-timestamp `regime.as_of_value(open_ts)` gate;
  flat→ON buys, ON→flat sells; `Decimal`-only; curve shape `n_ts+1` identical to
  buyhold. — _acceptance: unit tests — always-ON ≈ `run_buyhold_path`; always-OFF
  holds flat at initial capital; deterministic; empty-bars edge matches buyhold._

- [ ] **M-ARM-2** — Add the `"v0.macro_riskon"` match arm to `run_scenario`
  (`engine.rs`, modelled on the `"v0.buyhold"` arm at `engine.rs:1847`): resolve
  coin bars from `cfg.bars_override`, read `cfg.macro_regime_series`, call
  `run_macro_gated_buyhold_path`, build `RunReport`, `write_report = false`.
  `is_short_enabled("v0.macro_riskon") = false`. — _acceptance: dispatch routes;
  no report body written; the arm returns a finite equity series + KPIs._

- [ ] **M-ARM-3 (registration seam)** — Add
  `BakeoffConfig::default_macro_field() -> vec![StrategyId("v0.macro_riskon")]`
  (`bakeoff/mod.rs:363` neighbourhood); extend it into `advisor_field()`
  (`runner.rs:53`): `field.extend(BakeoffConfig::default_macro_field());`. In
  `run_bakeoff`, preload `load_macro_regime_series` ONCE (beside the coin-bar
  preload) and set `macro_regime_series: Some(..)` ONLY on the macro arm's
  `ScenarioConfig` (`bakeoff/mod.rs:707`), `None` for all others.
  — _acceptance: `advisor_field_arm_count()` increments by 1; non-macro arms get
  `None`; the macro arm appears in the ranked field; no other arm perturbed._

## Phase 5 — Day-1 divergence + no-op control + leak-check (R5, NON-NEGOTIABLE)

- [ ] **M-TEST-1 (DIVERGENCE — gating)** — `crates/backtest/tests/macro_regime_overlay_end_to_end.rs`
  S1: up-trend coin bars + risk-OFF mid-stretch ⇒ gated final equity diverges
  from `run_buyhold_path` by **≥ 1 bp**. — _acceptance: S1 asserts ≥1bp gap;
  FAILS against a no-op (regime-ignored) implementation._

- [ ] **M-TEST-2 (NO-OP CONTROL — gating)** — S2: regime pinned risk-ON whole
  window ⇒ gated equity ≈ `run_buyhold_path` (equal up to the single initial-buy
  fee). — _acceptance: S2 asserts ≈ buyhold; both directions (S1 + S2) green._

- [ ] **M-TEST-3 (LEAK-CHECK)** — S3: forward-shifting the regime series by 1 day
  CHANGES gated equity (`assert_ne!`); a coin bar at `t < D.close` reads the
  PRIOR regime, never the future `D`. S4: warm-up `None` ⇒ FLAT. — _acceptance:
  S3 + S4 green; proves the arm routes through `core::pit` (no look-ahead)._

## Phase 6 — Bake-off run + honest-coverage acceptance (R7)

- [ ] **M-BAKE (human-run, real corpus)** — Run the advisor bake-off on a real
  coin window with the macro arm in the field; confirm `v0.macro_riskon`
  produces a finite ranked candidate scored by the FROZEN gate (S6). The null
  result (FRAGILE ⇒ ineligible, hold still wins) is the EXPECTED, shippable
  outcome — it flows through `BenchmarkWins`/`AllFragile` with NO change.
  — _acceptance: macro arm ranked + gate-scored; UI honesty branches unchanged;
  result recorded (FRAGILE expected, positive would need OOS confirmation)._

## Phase 7 — Anchors + close

- [ ] **M-CLOSE-1 (ANCHOR RE-PROOF)** — `bash scripts/verify_anchors.sh` →
  **119/119** AFTER the full feature lands; no anchored `spec/*/reports/` file
  mutated. — _acceptance: 119/119; zero anchored-body diff._

- [ ] **M-CLOSE-2** — `cargo fmt`; `cargo clippy --workspace -- -D warnings`;
  `cargo test --workspace` green (incl. S1–S5 + the data/calendar + macro_regime
  suites). Tester writes the report; feature.md § Verification linked.
  — _acceptance: clean clippy + fmt; full suite green; tester VERDICT recorded._

## Notes

- **Why this order:** Phase 1 (the calendar) is the F-2 unblock — without it the
  Phase 2 macro `load_cached` trips `MissingData` (~71% < 95%). The calendar is
  proven INERT (M-CAL-4) in its own commit before the arm exists, so any anchor
  drift is attributable. Phase 3 reuses the SHIPPED `core::pit` primitive
  (ADR-0058) — the LOCF join is `as_of_value`, not a hand-roll. Phase 4's arm is
  a pure-function sibling of `run_buyhold_path` (no `ComposedStrategy`, no demux).
- **Anchor-safety is checked TWICE on the calendar** (M-CAL-4 before the arm,
  M-CLOSE-1 after) and once on the corpus add (M-FETCH-1) — the calendar layer
  perturbing crypto coverage is the single highest anchor risk.
- **Pre-registration:** the ticker set + rule + arm id are LOCKED (D4) before any
  bake-off result is read. The only permitted pre-result change is the
  `DX-Y.NYB`→`^DXY` fallback (M-FETCH-0), recorded in the Changelog.
