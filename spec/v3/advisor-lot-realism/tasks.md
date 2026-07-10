---
slug: advisor-lot-realism
status: arch-done
owner: architect
updated: 2026-07-10
---

# Tasks — advisor-lot-realism (P4, opt-in exec-sim lot/min-notional realism)

Ordered for the **developer**. Opt-in-forever per the **ADR-0081 precedent**:
the new mode is **OFF by default**, so anchors stay **119/119 by construction**;
the seam is `PaperEngine::step` (the both-paths chokepoint). The **divergence e2e
(T6)** is the closing gate — the feature is not done until the DOGE small-budget
run's terminal equity provably diverges ≥ 1 bp from the un-rounded baseline AND
the €200-major negative control shows ≈ 0. Full design in
[`feature.md`](feature.md) § Design + [ADR-0087](../../architecture/adr/0087-lot-realism-opt-in-exec-sim.md).

## Build order

- [ ] **T1 — static filter table (`crates/cost/src/venue_filter.rs`).** New module.
  Local `pub struct VenueFilter { pub step_size: Decimal, pub min_notional: Decimal }`
  mirroring `data::SymbolInfo`'s fields (do NOT add a `cost → data` dep — carry the
  local record). `pub fn venue_filter_for(symbol: &Symbol) -> Option<VenueFilter>`
  returning the checked-in snapshot for the **10 Binance USDT pairs** (BTC/ETH/BNB/
  SOL/XRP/ADA/DOGE/AVAX/DOT/LINK — the advisor corpus) **+ Coinbase `BTC-USD`**;
  unknown symbol → `None`. Module doc carries a `SNAPSHOT_DATE` const + the staleness
  stated-limit note (ADR-0087 § D3). Re-export from `crates/cost/src/lib.rs`.
  — _acceptance: `Decimal` literals only (no `f64`); unknown symbol → `None`; the
  `cost` crate gains NO new dependency (`cargo tree -p cost` unchanged)._

- [ ] **T2 — the Decimal-exact round-down + reject helper.** In `venue_filter.rs`:
  `pub fn round_down_to_step(qty: Decimal, step: Decimal) -> Decimal` = `(qty /
  step).floor() * step` (round-DOWN only; guard `step == 0` → return `qty`
  unchanged). `impl VenueFilter { pub fn admit(&self, qty: Decimal, price: Decimal)
  -> Option<Decimal> }` → `Some(rounded_qty)` when `rounded_qty > 0 && rounded_qty *
  price >= min_notional`, else `None` (reject/skip). — _acceptance: round-down never
  rounds UP (unit test: `0.999→0` at step 1); `admit` returns `None` below
  min-notional; all arithmetic `Decimal`._

- [ ] **T3 — unit tests for the table + helpers.** In `venue_filter.rs` `#[cfg(test)]`:
  round-down exactness (whole-DOGE `12.9→12`, BTC `0.123456→0.12345` at step 0.00001),
  min-notional threshold (just-below rejects, just-at admits), unknown-symbol `None`,
  and the `step==0` defensive path. — _acceptance: ≥ 6 unit tests, all green
  (`cargo test -p cost --lib venue_filter`)._

- [ ] **T4 — config surface (`crates/backtest/src/cli_types.rs`).** Add
  `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]` `#[serde(tag =
  "kind", rename_all = "snake_case")] pub enum VenueFilterMode { LotSizeAndMinNotional }`
  and `#[serde(default)] pub venue_filter: Option<VenueFilterMode>` on
  `LatencySlippageSimConfig`. Extend the `Default` impl with `venue_filter: None`.
  Leave the custom `Deserialize` back-compat path intact (existing configs deserialize
  with `venue_filter = None`). — _acceptance: `LatencySlippageSimConfig::default()
  .venue_filter.is_none()`; existing serde fixtures still round-trip; the `is_noop()`
  helper (if present) still reports noop for the default._

- [ ] **T5 — wire the seam in `PaperEngine::step` (`crates/backtest/src/paper.rs`).**
  Add `Option<VenueFilterTable>` (or an `Option<VenueFilterMode>` + a lazily-resolved
  per-symbol lookup) + `skipped_min_notional: u64` fields on `PaperEngine`; a ctor/
  builder that accepts the mode (default constructors pass `None`). In `step`, for each
  order **before** building the `Fill`: if the mode is `Some`, call
  `venue_filter_for(order.symbol())` → `admit(order.qty().get(), fill_price)`; on
  `Some(rounded)` build the `Fill` with `qty = Quantity::new(rounded)?`; on `None`
  (reject) **`continue`** (push no `Fill`) and `self.skipped_min_notional += 1`. When
  the mode is `None`, the code path is UNCHANGED (`fill.qty == order.qty()`). Add
  `pub fn sim_filter_stats(&self) -> SimFilterStats` (carrying `skipped_min_notional`).
  — _acceptance: `venue_filter = None` → `Fill.qty == order.qty()` byte-for-byte
  (T7); a rejected order pushes NO `Fill` and is NOT a `MatchError`; unknown symbol
  under an enabled mode → no-op (order fills un-rounded)._

- [ ] **T6 — the day-1 divergence e2e (CLAUDE.md non-negotiable — the closing gate).**
  New `crates/backtest/tests/lot_realism_divergence_end_to_end.rs`, modelled on
  `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`. Run the SAME strategy +
  bars twice on **`DOGEUSDT`** (`step_size = 1`) at a **€50–200 budget** with
  `fixed_fraction(0.1)`: (a) `venue_filter = None` (baseline), (b)
  `Some(LotSizeAndMinNotional)`. Assert `|eq_filtered − eq_baseline| / eq_baseline ≥
  1e-4` AND `eq_filtered <= eq_baseline` (direction). Add a **negative control**: a
  high-price major (BTC) at €200 → assert divergence ≈ 0. Confirm it **FAILs before**
  T5 wiring (compute-but-never-apply guard) and **PASSes after**. — _acceptance:
  divergence ≥ 1 bp on DOGE; direction holds; negative control ≈ 0; FAIL-before/
  PASS-after documented in the test doc-comment._

- [ ] **T7 — anchor-safety enforcement tests (never delete).** In `paper.rs`
  `#[cfg(test)]`: `venue_filter_default_is_none` (asserts
  `LatencySlippageSimConfig::default().venue_filter.is_none()` — mirrors ADR-0081's
  `default_is_linear_bps_8`) and `paper_step_none_is_byte_identical` (a `step` run with
  `venue_filter = None` yields `Fill.qty == order.qty()` and identical `price`/`fee` —
  the proof obligation: default run ≡ pre-change run). — _acceptance: both green; both
  carry a "NEVER DELETE — D6 contract" doc-comment._

- [ ] **T8 — reserved live-agent audit wiring (spec only, do NOT build a live path).**
  Add a doc-comment stub at the `strategy_events` call-site convention (or in
  `venue_filter.rs`) naming the reserved `kind = "min_notional_skip"`
  `StrategyEventWrite` (`crates/audit/src/journal.rs:1623`), the `rebalance_rejected`
  precedent, and that it is built **only when a live-agent caller exists** (no live
  path ships here). Surface `skipped_min_notional` into the advisor run summary where
  the runner already assembles its summary (the primary, in-memory home). — _acceptance:
  NO `crates/audit` behaviour change; NO new `AuditEvent` variant; the tally appears in
  the run summary; the live wiring is documented-but-unbuilt._

## Gate checklist (run before HANDOFF → tester)

- [ ] `cargo test -p cost --lib venue_filter` — green (T2/T3).
- [ ] `cargo test -p backtest --lib paper` — green incl. T7 enforcement tests.
- [ ] `cargo test -p backtest --test lot_realism_divergence_end_to_end` — green (T6).
- [ ] `bash scripts/verify_anchors.sh` → **119/119** (BEFORE and AFTER).
- [ ] `python3 scripts/spec_lint.py` → **PASS(0)**.
- [ ] `python3 scripts/adr_registry_check.py --pre-commit` → **exit 0**.
- [ ] `cargo clippy -p cost -p backtest --tests -- -D warnings` — clean.
- [ ] `cargo fmt --check` — clean.
- [ ] FROZEN-gate diff-empty: `git status --porcelain` shows no
  `bakeoff/{robustness,rank}.rs` / `spec/*/reports/` / `ci.yml.deferred` changes.
