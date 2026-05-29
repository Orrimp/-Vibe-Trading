---
slug: v5-latency-slippage-sim-v0.5.0-square-root-market-impact
status: draft
owner: developer
updated: 2026-05-29
---

# tasks — v5 latency-slippage-sim v0.5.0 square-root market-impact

## M0 — Analyst (~0.5 day) ✅ in-flight

- [x] Author `feature.md` v0.1.0 (5 R / R-NR / 4 K / 3 H / 3 Q + pre-drawn 2-cell verdict tree + cost framing both routes)
- [x] Append `[[req]] REQ-V5-LATENCY-SLIPPAGE-V0-5-0-001` to `spec/trace.toml` (state = `proposed`)
- [x] Append Active row to `spec/backlog.md`
- [x] Verify gates green: anchors 71/71 PASS; spec_lint baseline-stable
- [ ] HANDOFF → operator-decide (Q1-Q3 all default to DURABLE per AGENT.md 2026-05-29; Autoapprove-eligible at analyst defaults)

## M-OD — Operator-decide (~0 day, all-DURABLE Autoapprove-eligible) ✅ RESOLVED 2026-05-29

- [x] **Q1** — Impact coefficient α → **(a) α = 1.0** (Kissell 2014 midpoint; DURABLE) ✅
- [x] **Q2** — Per-asset volume V source → **(a) 90-day trailing Binance parquet** (revision-pinned) ✅
- [x] **Q3** — Synthetic-scenario behavior → **(b) MIXED — universe-avg V on synthetic** (operator override of analyst-recommended (a) Linear fallback; adds v0.6.0 sub-namespace cleanup commitment) ⚠ override

## M-T1 — Architect (~1 day) ✅ COMPLETE 2026-05-29

- [x] Lock numerical-precision contract for `√` over Decimal (K2): f64 boundary in `apply_slippage_sqrt`; `f64::sqrt` + `f64::round_ties_even` → saturating-cast u32 ≤ MAX_SLIPPAGE_BPS; back to Decimal for sign × multiplier. Documented in feature.md § D-T1.3.
- [x] Pick per-asset volume retrieval shape (R3) → **Option A**: extend `crates/data` with `daily_volume_usd_trailing` query (analyst lean — deterministic + revision-pinned + no on-disk artifact). Documented § D-T1.4.
- [x] Lock `MAX_SLIPPAGE_BPS` cap (K3) → default **1_000 (10%)** confirmed; operator-override path at M-OD if dry runs surface > 5% saturation. Documented § D-T1.6.
- [x] ADR decision → **amend ADR-0043 § Changelog** (NOT new ADR-0050); mirrors the 2026-05-27 Murmur3 D2 amendment precedent — closes ADR-0043's own deferred § D3 promise without forking a sibling ADR. Documented § D-T1.1.
- [x] Confirm namespace `v5-sqrt-impact-2026-05` is the correct pin (mirrors ADR-0045 D2 namespace-twin pattern; parallel to `v5-realdata-medium-2026-05`). Documented § D-T1.7.
- [x] **Operator Q3=(b) override implementation**: `universe_avg_daily_volume_usd_trailing` helper computes arithmetic mean across 10-USDT-pair Binance universe; pinned to scenario's own end_date with 90-day lookback. **9 synthetic-scenario SHAs in `v5-sqrt-impact-2026-05` namespace WILL DIFFER from their `v5-realdata-medium-2026-05` linear-bps twins — by-design; v0.6.0 sub-namespace cleanup commitment recorded**. Documented § D-T1.5.
- [x] Decompose M-DEV into Waves A (cost crate model swap) → B (backtest plumbing through SlippageModel enum) → C (data crate per-asset + universe-avg V helper) → D (anchor namespace-aware resolver extension; t1937c test) → E (19-scenario re-emission + 2-run determinism + anchors.toml additive + Sharpe-delta table) → F (e2e divergence + tester harness verification). Critical path A → B → C → D → E → F; D parallelizable with C. Documented § D-T1.9.
- [x] Update `spec/architecture/adr/0043-simulated-latency-and-slippage.md` § Changelog with v0.5.0 amendment block.
- [x] Flip frontmatter `owner: architect → developer` on feature.md + tasks.md.
- [x] Populate `arch` column on `REQ-V5-LATENCY-SLIPPAGE-V0-5-0-001` trace row + flip state `proposed → arch-done`.
- [x] HANDOFF → developer (Waves A–F lockstep critical path).

## M-DEV — Developer (~3-5 days, Waves A-F)

### Wave A — Model body in `crates/cost/src/slippage.rs` (~0.5-1 day)

- [x] Replace `apply_slippage(price, side, _notional, bps)` with model-dispatching variant; add `SlippageModel` enum
  - file: `crates/cost/src/slippage.rs:40-53` (enum) + `87-111` (dispatcher)
  - test: `cargo test -p cost -- slippage::tests`
  - output: `test result: ok. 14 passed` (subset of 24 total cost tests)
- [x] Implement `apply_slippage_sqrt(signal_price, side, notional, alpha, v_daily, max_bps)` with f64-boundary contract (architect-locked at M-T1)
  - file: `crates/cost/src/slippage.rs:151-215`
  - test: `cargo test -p cost -- slippage::tests::sqrt_reference_alpha1_q1m_v1b`
  - output: `test slippage::tests::sqrt_reference_alpha1_q1m_v1b ... ok`
- [x] Unit tests: `α=1.0, Q=$1M, V=$1B → 316 bps`; cap saturation at MAX_SLIPPAGE_BPS; deterministic across architectures
  - file: `crates/cost/src/slippage.rs:217-500` (14 unit tests)
  - test: `cargo test -p cost -- slippage::tests`
  - output: all 14 slippage tests ok

### Wave B — Enum plumbing through `LatencySlippageSimConfig` (~0.5 day)

- [x] Replace `slippage_bps: u16` with `slippage_model: SlippageModel` on `LatencySlippageSimConfig` (backtest crate)
  - file: `crates/backtest/src/cli_types.rs:53-77`
  - test: `cargo test -p backtest --lib -- latency_slippage_config_tests`
  - output: `test result: ok. 13 passed`
- [x] Serde adapter: old `slippage_bps: u16` deserializes to `Linear { bps }` for backward-compat (R-NR.2 oracle preservation)
  - file: `crates/backtest/src/cli_types.rs:109-168` (custom Deserialize visitor)
  - test: `cargo test -p backtest --lib -- legacy_slippage_bps_deserializes_to_linear`
  - output: `test cli_types::latency_slippage_config_tests::legacy_slippage_bps_deserializes_to_linear ... ok`
- [x] Update `crates/backtest/src/scenarios/sim.rs::sim_slippage_cost` to dispatch on enum (ADR-0047 D2 SOLE-LOCATION grep gate stays green)
  - file: `crates/backtest/src/scenarios/sim.rs:51-84`
  - test: `cargo test -p backtest --lib -- scenarios::sim::tests`
  - output: `test result: ok. 6 passed`
- [x] Add `volume_usd_per_symbol: Option<Arc<HashMap<Symbol, Decimal>>>` to `LatencySlippageSimConfig`; wire into Default, Deserialize, all struct literals in main.rs, cli_types.rs tests, sim.rs tests, e2e tests
  - file: `crates/backtest/src/cli_types.rs:75-77` (field); `main.rs` (12 sites); `scenarios/sim.rs` (6 test literals); `crates/strategy/tests/latency_slippage_sim_e2e.rs` (2 literals)
  - test: `cargo test -p backtest --lib && cargo test -p strategy --test latency_slippage_sim_e2e`
  - output: all 44 backtest lib tests ok; all 3 e2e tests ok

### Wave C — Per-asset volume retrieval (~0.5-1 day, architect-locked shape)

- [x] Implement R3 retrieval per M-T1 decision (Option A `DailyVolume` query): `daily_volume_usd_trailing` + `universe_avg_daily_volume_usd_trailing` in `crates/data/src/daily_volume.rs`
  - file: `crates/data/src/daily_volume.rs:104-179`
  - test: `cargo test -p data --lib -- daily_volume`
  - output: `test daily_volume::tests::date_to_unix_millis_known_value ... ok` (and 4 others)
- [x] In-process cache (`OnceLock<Mutex<HashMap>>`) keyed on `(symbol, end_date_ordinal, lookback_days)`
  - file: `crates/data/src/daily_volume.rs:79-84`
  - test: `cargo test -p data --lib -- daily_volume`
  - output: `test result: ok. 52 passed` (53 total, 1 ignored - requires real data)
- [ ] NOTE: Synthetic-data call sites currently pass `Decimal::ZERO` as volume_usd; Q3=(b) universe-avg V requires main.rs to populate `volume_usd_per_symbol` before scenario runs. Volume field is wired structurally; actual population for sqrt scenarios is left for Wave D.

### Wave D — 19-scenario re-emission on canonical Apple Silicon box (~0.5 day)

- [ ] `cargo build --release -p backtest --features "candle realdata"`
- [ ] Run all 19 scenarios under `SlippageModel::SquareRoot { alpha: 1.0, volume_lookback_days: 90 }` (real-data) or `Linear { bps: 8 }` (synthetic-data fallback)
- [ ] Emit to `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/backtest-<TS>-<scenario>.md`
- [ ] **Determinism gate (load-bearing)**: 2 independent runs per scenario MUST produce byte-identical body-SHAs (mirrors v0.4.0 T-D-N3)

### Wave E — Anchor migration + Sharpe-delta table (~0.5 day)

- [ ] Append 19 new `[[anchors]]` rows under namespace `v5-sqrt-impact-2026-05` to `spec/anchors.toml` (71 → 90; the 71 existing rows stay byte-identical)
- [ ] Author `reports/sharpe-delta-table-2026-05-<DD>.md` with 3-column comparison (noop / linear-bps / square-root) per scenario; flag K1 surprises
- [ ] `bash scripts/verify_anchors.sh` → `ANCHORS PASS (90 / 90)`

### Wave F — t1937 third-namespace extension (~0.25 day)

- [x] Extend `crates/reports/tests/strategy_anchors_unchanged.rs`: add `SqrtImpact` to `Namespace` enum; add `SQRT_IMPACT_FEATURE_DIRS` + `SQRT_IMPACT_STRATEGY_ANCHORS` constants; add `t1937c_sqrt_impact_strategy_anchors_unchanged` test
  - file: `crates/reports/tests/strategy_anchors_unchanged.rs` (SqrtImpact namespace + t1937c test)
  - test: `cargo test -p reports --test strategy_anchors_unchanged`
  - output: `test result: ok. 4 passed; 0 failed`
- [x] `cargo test -p reports --test strategy_anchors_unchanged` → 4/4 PASS
  - file: `crates/reports/tests/strategy_anchors_unchanged.rs`
  - test: `cargo test -p reports --test strategy_anchors_unchanged`
  - output: `test t1937c_sqrt_impact_strategy_anchors_unchanged ... ok; test t1937_nine_strategy_anchors_unchanged ... ok; test t1937b_canonical_strategy_anchors_unchanged ... ok; test t1942_anchor_shas_are_well_formed_64_lowercase_hex ... ok`

## M-FINAL — Tester (~1 day)

- [ ] `bash scripts/verify_anchors.sh` → PASS 90/90 (R-NR.1)
- [ ] 71 existing rows byte-identical (R-NR.2 + R-NR.3)
- [ ] 2-run determinism spot-check on ≥ 3 of 19 sqrt-impact SHAs (K4 gate)
- [ ] `cargo test -p reports --test strategy_anchors_unchanged` → 4/4 PASS
- [ ] `cargo test -p strategy --test latency_slippage_sim_e2e` + `vol_targeting_overlay_end_to_end` + `vol_killswitch_overlay_end_to_end` → all PASS under BOTH model configs (R-NR.5)
- [ ] `cargo test --workspace --no-fail-fast` → no new failures vs v0.4.0 whitelist
- [ ] H1 directional check: TCN-realdata sqrt drag ≥ 2× linear drag; H2: low-turnover delta ≤ 30%; H3: byte-identity gate
- [ ] K1 surprise scan across 19 scenarios; flag per-scenario if `sharpe(sqrt) < 0 ∧ sharpe(linear) > 0`
- [ ] Author `reports/test-final-2026-05-<DD>-v5-latency-slippage-sim-v0.5.0-square-root-market-impact.md` with VERDICT (PASS / REGRESSION per R-O1/R-O2)
- [ ] Populate `anchors` column on `REQ-V5-LATENCY-SLIPPAGE-V0-5-0-001` trace row + flip state to `passed`

## M-PRES — Presenter (~0.5 day)

- [ ] Assemble sprint-review deck at `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/presentations/v5-latency-slippage-sim-v0.5.0-square-root-market-impact-<DATE>.md`
- [ ] Lead with "closes the ADR-0043 § D3 deferred promise — v0.1.0 → v0.5.0 = engine + canonical config + per-path wiring + candle/realdata coverage + model-quality upgrade" framing
- [ ] Inherit pre-drawn 2-cell verdict tree from `feature.md`
- [ ] Embed H1/H2/H3 falsifier outcomes + per-scenario K1 surprise table
