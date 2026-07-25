---
slug: advisor-lot-realism
status: dev-done
owner: developer
updated: 2026-07-10
---

# Tasks — advisor-lot-realism (P4, opt-in exec-sim lot/min-notional realism)

Ordered for the **developer**. Opt-in-forever per the **ADR-0081 precedent**:
the new mode is **OFF by default**, so anchors stay **119/119 by construction**;
the seam is `PaperEngine::step` (the both-paths chokepoint). The **divergence e2e
(T6)** is the closing gate — the feature is not done until the DOGE small-budget
run's terminal equity provably diverges ≥ 1 bp from the un-rounded baseline AND
the €200-major negative control shows ≈ 0. Full design in
[`feature.md`](feature.md) § Design + [ADR-0087](../../../_bmad-output/planning-artifacts/architecture/decisions/0087-lot-realism-opt-in-exec-sim.md).

## Build order

- [x] **T1 — static filter table (`crates/cost/src/venue_filter.rs`).** New module.
  Local `pub struct VenueFilter { pub step_size: Decimal, pub min_notional: Decimal }`
  mirroring `data::SymbolInfo`'s fields (do NOT add a `cost → data` dep — carry the
  local record). `pub fn venue_filter_for(symbol: &Symbol) -> Option<VenueFilter>`
  returning the checked-in snapshot for the **10 Binance USDT pairs** (BTC/ETH/BNB/
  SOL/XRP/ADA/DOGE/AVAX/DOT/LINK — the advisor corpus) **+ Coinbase `BTC-USD`**;
  unknown symbol → `None`. Module doc carries a `SNAPSHOT_DATE` const + the staleness
  stated-limit note (ADR-0087 § D3). Re-export from `crates/cost/src/lib.rs`.
  — _acceptance: `Decimal` literals only (no `f64`); unknown symbol → `None`; the
  `cost` crate gains NO new dependency (`cargo tree -p cost` unchanged)._
  — **DONE**: `crates/cost/src/venue_filter.rs:58` (`VenueFilter` struct), `:108`
  (`venue_filter_for`, 11 entries incl. `BTC-USD`), `SNAPSHOT_DATE` const at line 39;
  re-exported `crates/cost/src/lib.rs:12,25-27`. Cmd: `cargo tree -p cost` diffed
  before/after — unchanged (no `data` dep added). Test: `cargo test -p cost --lib
  venue_filter` → `test result: ok. 15 passed; 0 failed`.

- [x] **T2 — the Decimal-exact round-down + reject helper.** In `venue_filter.rs`:
  `pub fn round_down_to_step(qty: Decimal, step: Decimal) -> Decimal` = `(qty /
  step).floor() * step` (round-DOWN only; guard `step == 0` → return `qty`
  unchanged). `impl VenueFilter { pub fn admit(&self, qty: Decimal, price: Decimal)
  -> Option<Decimal> }` → `Some(rounded_qty)` when `rounded_qty > 0 && rounded_qty *
  price >= min_notional`, else `None` (reject/skip). — _acceptance: round-down never
  rounds UP (unit test: `0.999→0` at step 1); `admit` returns `None` below
  min-notional; all arithmetic `Decimal`._
  — **DONE**: `crates/cost/src/venue_filter.rs:93` (`round_down_to_step`, guards
  `step <= 0`), `:76` (`VenueFilter::admit`). Test:
  `crates/cost/src/venue_filter.rs:184` `round_down_never_rounds_up` (`0.999→0` @
  step 1). Cmd: `cargo test -p cost --lib venue_filter` → `test
  venue_filter::tests::round_down_never_rounds_up ... ok` (part of the 15-passed run
  above).

- [x] **T3 — unit tests for the table + helpers.** In `venue_filter.rs` `#[cfg(test)]`:
  round-down exactness (whole-DOGE `12.9→12`, BTC `0.123456→0.12345` at step 0.00001),
  min-notional threshold (just-below rejects, just-at admits), unknown-symbol `None`,
  and the `step==0` defensive path. — _acceptance: ≥ 6 unit tests, all green
  (`cargo test -p cost --lib venue_filter`)._
  — **DONE**: `crates/cost/src/venue_filter.rs:169-297`, 15 tests (exceeds the ≥6
  bar) incl. `round_down_whole_doge_step_one` (12.9→12), `round_down_btc_five_decimals`
  (0.123456→0.12345), `admit_exactly_at_min_notional_admits`,
  `admit_one_tick_under_min_notional_rejects`, `venue_filter_for_unknown_symbol_is_none`,
  `round_down_step_zero_guard_returns_qty_unchanged`. Cmd: `cargo test -p cost --lib
  venue_filter` → `test result: ok. 15 passed; 0 failed; 0 ignored`.

- [x] **T4 — config surface (`crates/backtest/src/cli_types.rs`).** Add
  `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]` `#[serde(tag =
  "kind", rename_all = "snake_case")] pub enum VenueFilterMode { LotSizeAndMinNotional }`
  and `#[serde(default)] pub venue_filter: Option<VenueFilterMode>` on
  `LatencySlippageSimConfig`. Extend the `Default` impl with `venue_filter: None`.
  Leave the custom `Deserialize` back-compat path intact (existing configs deserialize
  with `venue_filter = None`). — _acceptance: `LatencySlippageSimConfig::default()
  .venue_filter.is_none()`; existing serde fixtures still round-trip; the `is_noop()`
  helper (if present) still reports noop for the default._
  — **DONE**: `crates/backtest/src/cli_types.rs:58` (`VenueFilterMode` enum), `:90`
  (`venue_filter` field), `:103-115` (`Default` impl incl. `venue_filter: None`),
  `:117-129` (`is_noop` extended with `venue_filter.is_none()`), `:178-186` (custom
  `Deserialize` visitor — added `venue_filter` binding + match arm + threaded into the
  `Ok(..)` construction, so a present `venue_filter` key round-trips and an absent one
  still defaults to `None`, R-NR.2 back-compat preserved). Also patched all 33 existing
  `LatencySlippageSimConfig { .. }` literal-construction sites across
  `cli_types.rs`/`main.rs`/`scenarios/sim.rs`/
  `crates/strategy/tests/latency_slippage_sim_e2e.rs` with `venue_filter: None,` (none
  used `..Default::default()` spread, so each needed the explicit field or the crate
  would not compile — verified via `cargo build -p backtest --lib --features
  realdata,yahoo,candle` → `Finished`). Test:
  `crates/backtest/src/cli_types.rs:308-342` new tests
  `venue_filter_defaults_to_none`, `venue_filter_serde_round_trip`,
  `missing_venue_filter_field_defaults_to_none`. Cmd: `cargo test -p backtest --lib
  --features realdata,yahoo,candle` → `test result: ok. 249 passed; 0 failed; 11
  ignored`.

- [x] **T5 — wire the seam in `PaperEngine::step` (`crates/backtest/src/paper.rs`).**
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
  — **DONE**: `crates/backtest/src/paper.rs:65-78` (`PaperEngine` fields — chose the
  `Option<VenueFilterMode>` + lazily-resolved-per-order-lookup alternative explicitly
  permitted by this task, no separate `VenueFilterTable` type needed since
  `cost::venue_filter_for` is already a pure O(1) lookup), `:101-104`
  (`with_venue_filter_mode` builder), `:109-113` (`sim_filter_stats`), `:146-183` (the
  `step` seam — round-down-then-admit, unknown-symbol no-op, sub-min-notional skip via
  `continue` + tally, NOT a `MatchError`). Tests:
  `crates/backtest/src/paper.rs:411-475` — `venue_filter_rounds_qty_down_when_enabled`,
  `venue_filter_rejects_sub_min_notional_order_no_fill_no_error` (asserts
  `result.is_ok()` AND `fills.is_empty()`), `venue_filter_unknown_symbol_is_noop_when_enabled`,
  `with_venue_filter_mode_is_a_pure_builder`. Cmd: `cargo test -p backtest --lib paper`
  → `test result: ok. 9 passed; 0 failed`.

- [x] **T6 — the day-1 divergence e2e (CLAUDE.md non-negotiable — the closing gate).**
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
  — **DONE**: `crates/backtest/tests/lot_realism_divergence_end_to_end.rs` (new file,
  313 lines). Built BEFORE T5 per the noop-trap discipline: T1–T4 landed first, then
  this file, then a scaffold-only `PaperEngine` (fields + builder existed, `step`
  still used `order.qty()` unconditionally) — run + FAILED (captured below) — THEN
  the real `step` wiring (T5) landed and the same file was re-run + PASSED (captured
  below, also embedded verbatim in the file's own module doc-comment lines 23-63).
  DOGEUSDT budget is **€100** (not €50): at €50 every clip landed just under the
  min-notional floor and 100% of orders were rejected outright (`eq_filtered ==
  initial_capital` exactly) — technically satisfies the assertions but only exercises
  the reject path, not genuine floor-rounding; €100 clips (~€10) clear the floor and
  isolate the "(a) lot rounding shaves a few sats" mechanism ADR-0087 § D4 describes,
  reserving the reject path for `paper.rs`'s
  `venue_filter_rejects_sub_min_notional_order_no_fill_no_error` (T5).
  **FAIL-before** (`cargo test -p backtest --test lot_realism_divergence_end_to_end
  -- --nocapture`, `step` scaffold-only, pre-T5-wiring):
  ```
  FORENSIC GATE FAIL — lot-size rounding is a no-op!
  eq_baseline = 102.48212807787434490257656611
  eq_filtered = 102.48212807787434490257656611
  divergence  = 0.00000000000000000000000000 (0 relative)
  required (>= 1 bp) = 0.0001
  test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
  ```
  **PASS-after** (same command, post-T5-wiring):
  ```
  dogeusdt_small_budget: eq_baseline=102.48212807787434490257656611 eq_filtered=102.42370740160 relative_divergence=0.0005700572126093251165474053 skipped_min_notional(filtered)=0
  test dogeusdt_small_budget_lot_rounding_diverges_from_baseline ... ok
  btcusdt_major_at_200: eq_baseline=200.83200137031358596842456214 eq_filtered=200.8277912739200 relative_divergence=0.0000209632746019544118024876 doge_relative_divergence=0.0005700572126093251165474053
  test btcusdt_major_at_200_is_negative_control ... ok
  test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```
  DOGE diverges 57 bp (5.7× the 1bp gate; direction holds, `eq_filtered <
  eq_baseline`); BTC-€200 negative control diverges 0.21 bp (27× smaller than DOGE,
  `skipped_min_notional == 0`).

- [x] **T7 — anchor-safety enforcement tests (never delete).** In `paper.rs`
  `#[cfg(test)]`: `venue_filter_default_is_none` (asserts
  `LatencySlippageSimConfig::default().venue_filter.is_none()` — mirrors ADR-0081's
  `default_is_linear_bps_8`) and `paper_step_none_is_byte_identical` (a `step` run with
  `venue_filter = None` yields `Fill.qty == order.qty()` and identical `price`/`fee` —
  the proof obligation: default run ≡ pre-change run). — _acceptance: both green; both
  carry a "NEVER DELETE — D6 contract" doc-comment._
  — **DONE**: `crates/backtest/src/paper.rs:373-386` `venue_filter_default_is_none`,
  `:388-408` `paper_step_none_is_byte_identical` (uses **DOGEUSDT** — a symbol IN the
  filter table — with a fractional qty `12.7` that WOULD round if enabled, proving the
  default path never rounds even for a table symbol; both carry the "NEVER DELETE —
  D6 contract" doc-comment). Cmd: `cargo test -p backtest --lib paper` → `test
  paper::tests::venue_filter_default_is_none ... ok` / `test
  paper::tests::paper_step_none_is_byte_identical ... ok` (part of the 9-passed run
  above).

- [x] **T8 — reserved live-agent audit wiring (spec only, do NOT build a live path).**
  Add a doc-comment stub at the `strategy_events` call-site convention (or in
  `venue_filter.rs`) naming the reserved `kind = "min_notional_skip"`
  `StrategyEventWrite` (`crates/audit/src/journal.rs:1623`), the `rebalance_rejected`
  precedent, and that it is built **only when a live-agent caller exists** (no live
  path ships here). Surface `skipped_min_notional` into the advisor run summary where
  the runner already assembles its summary (the primary, in-memory home). — _acceptance:
  NO `crates/audit` behaviour change; NO new `AuditEvent` variant; the tally appears in
  the run summary; the live wiring is documented-but-unbuilt._
  — **DONE, scope note**: `crates/backtest/src/paper.rs:156-172` doc-comment stub
  citing `crates/audit/src/journal.rs:1623` (`strategy_event`) + the
  `rebalance_rejected` precedent at `:1722` (both line numbers verified current via
  `grep -n` before writing the citation), "built only when a live-agent caller
  exists." Verified NO `crates/audit` change and NO new `AuditEvent` variant via
  `git status --porcelain` (crate absent from the diff). **Deviation from the fuller
  tasks.md wording**: "surface `skipped_min_notional` into the advisor run summary"
  was interpreted narrowly per my developer brief's explicit scope guard
  (`crates/cost`, `crates/backtest` paper.rs+e2e only — NOT `engine.rs`'s
  `RunReport`/`CandidateResult` assembly or `crates/agent/src/runtime.rs`'s forward-
  loop summary, both out of the granted crate list) and per the brief's own T8 text
  ("SPEC-ONLY (do not implement); leave the note in tasks.md"). What ships: the
  `sim_filter_stats()` accessor (T5) IS the surface — any caller that owns a
  `PaperEngine` can already read the tally; threading it further into a specific
  report struct is left as a follow-up for whoever wires `venue_filter` into a real
  CLI/forward-loop call site (no such call site exists yet — see the trace row's
  `crates` note). Flagging for architect/tester: confirm this interpretation is
  acceptable or open a follow-up task.

## Gate checklist (run before HANDOFF → tester)

- [x] `cargo test -p cost --lib venue_filter` — green (T2/T3). Output: `test result:
  ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
- [x] `cargo test -p backtest --lib paper` — green incl. T7 enforcement tests. Output:
  `test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 211 filtered out`.
- [x] `cargo test -p backtest --test lot_realism_divergence_end_to_end` — green (T6).
  Output: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
- [x] `bash scripts/verify_anchors.sh` → **119/119** (BEFORE and AFTER). Output:
  `ANCHORS PASS  (119 / 119)` (run after the full implementation landed; the default
  path is unreachable-by-construction so this was expected to hold, and did).
- [x] `python3 scripts/spec_lint.py` → **PASS(0)**. Output: `spec-lint: PASS (0
  violations)`.
- [x] `python3 scripts/adr_registry_check.py --pre-commit` → **exit 0**. Verified via
  shell `$?`.
- [x] `cargo clippy -p cost -p backtest --tests --features
  backtest/realdata,backtest/yahoo -- -D warnings` — clean (`Finished` profile, zero
  warnings/errors; fixed one `single_match_else` + two `doc_markdown` findings during
  development).
- [x] `cargo fmt --check` — clean (exit 0; one `cargo fmt` pass applied during
  development, reformatting 4 files, no logic change — re-verified green after).
- [x] FROZEN-gate diff-empty: `git status --porcelain` shows no
  `bakeoff/{robustness,rank}.rs` / `spec/*/reports/` / `ci.yml.deferred` changes —
  verified (diff confined to `crates/cost`, `crates/backtest`,
  `crates/strategy/tests/latency_slippage_sim_e2e.rs`, this feature's spec files, and
  the trace row).

## Notes

- **T6 budget correction (€100, not €50–200's low end)**: see T6's DONE note above —
  the ADR/tasks.md €50 example lands 100% of DOGE clips just under min-notional at
  this synthetic price path, testing the reject path exclusively rather than
  floor-rounding. €100 was chosen so the primary e2e demonstrates genuine rounding
  (`skipped_min_notional == 0`); the reject path is covered by a dedicated
  `paper.rs` unit test instead (T5).
- **Gate-command correction**: the developer brief's literal
  `cargo test -p backtest --test vol_targeting_overlay_end_to_end` does not resolve —
  that file lives at `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`
  (crate `strategy`, not `backtest`). Ran the corrected `cargo test -p strategy
  --test vol_targeting_overlay_end_to_end` → `test result: ok. 1 passed; 0 failed`.
- **T8 scope deviation**: see T8's DONE note above — flagged for architect/tester
  review.
