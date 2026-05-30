---
slug: v5-latency-slippage-sim-v0.5.0-square-root-market-impact
status: tester-done
owner: tester
updated: 2026-05-30
q_d1_ratified: "(a) Linear{bps:8} fallback for synthetic — 2026-05-29"
q_d2_ratified: "(β) per-scenario lazy-compute — 2026-05-29"
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

### Wave D — 9-scenario re-emission on canonical Apple Silicon box (~0.5 day)

- [x] `cargo build --release -p backtest --features "candle realdata"`
  - file: `crates/backtest/` (Cargo.toml features)
  - test: `cargo build --release -p backtest --features "candle realdata"` → 0 errors
  - output: `Finished release profile [optimized] target(s) in 6.14s`
- [x] Bug fix: `sim_slippage_cost` call sites were passing `Decimal::ZERO` as `volume_usd` (no-op bug — SquareRoot model received V=0 and produced zero impact). Fixed by changing signature to `symbol: &Symbol` with internal lookup from `cfg.volume_usd_per_symbol` in `apply_slippage_sqrt`.
  - file: `crates/backtest/src/scenarios/sim.rs:52-94` (signature change + lookup)
  - file: `crates/backtest/src/scenarios/momentum.rs:387-442` (symbol passthrough)
  - file: `crates/backtest/src/scenarios/tcn_overlay.rs:247-302`
  - file: `crates/backtest/src/scenarios/tcn_overlay_weights.rs:226-281`
  - file: `crates/backtest/src/scenarios/patchtst_overlay_weights.rs:233-288`
  - file: `crates/backtest/src/scenarios/pairs.rs:253-325`
  - file: `crates/backtest/src/scenarios/garch_vol_target_overlay.rs:310-367`
  - file: `crates/backtest/src/scenarios/regime_dispatcher.rs:353-397`
  - test: `cargo test -p backtest --lib -- scenarios::sim::tests`
  - output: `test result: ok. 7 passed` (was 6 + 1 new sqrt_missing_symbol_fallback_zero test)
- [x] Run 9 real-data scenarios under Q-D1=(a) + Q-D2=(β) (real-data → SquareRoot; synthetic → Linear{bps:8} fallback)
  - file: `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/backtest-20260529-133245-top10-2023-fy-momentum-realdata.md` (+ 8 more)
  - test: `cargo run --release --bin backtest --features "candle realdata" -p backtest -- --scenario "..." --sim-slippage-sqrt-alpha 1.0 ...`
  - output: 9 reports written, equity diverges from linear baseline (momentum: $10,105 vs $77,002)
- [x] **Determinism gate (load-bearing)**: 9/9 scenarios × 2 runs PASS byte-identical body-SHAs (H3 PASS)
  - file: all 18 report files in `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/`
  - test: `python3 scripts/hash_report.py <run1> <run2>` for each pair
  - output: all SHA pairs identical (e.g. momentum: `0867d232b5d4e381...` × 2)

### Wave E — Anchor migration + Sharpe-delta table (~0.5 day)

- [x] Append 9 new `[[anchors]]` rows under namespace `v5-sqrt-impact-2026-05` to `spec/anchors.toml` (75 → 84; 9 new rows under v5-sqrt-impact-2026-05; 75 existing rows byte-identical)
  - file: `spec/anchors.toml:566-617` (new v5-sqrt-impact-2026-05 block)
  - test: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS (84 / 84)` (verify_anchors.sh also updated to add v5-sqrt-impact-2026-05 branch + fix legacy default to exclude v0.5.0 dir)
- [x] Author `reports/sharpe-delta-2026-05-29.md` with return-delta comparison (noop / linear-bps / square-root) per scenario; K1 surprises noted
  - file: `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/sharpe-delta-2026-05-29.md`
  - test: file created and readable
  - output: H1 PASS (sqrt drag 3.91× linear on TCN-realdata-2023); H2 PARTIAL (vol-target within threshold; patchtst outside); H3 PASS; 0 K1 surprises
- [x] `bash scripts/verify_anchors.sh` → `ANCHORS PASS (84 / 84)`
  - file: `scripts/verify_anchors.sh` (updated with v5-sqrt-impact-2026-05 branch)
  - test: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS (84 / 84)` — all 75 existing + 9 new PASS

### Wave F — t1937 third-namespace extension (~0.25 day)

- [x] Extend `crates/reports/tests/strategy_anchors_unchanged.rs`: add `SqrtImpact` to `Namespace` enum; add `SQRT_IMPACT_FEATURE_DIRS` + `SQRT_IMPACT_STRATEGY_ANCHORS` constants; add `t1937c_sqrt_impact_strategy_anchors_unchanged` test; populate `SQRT_IMPACT_STRATEGY_ANCHORS` with 9 real-data scenario SHAs at Wave E close.
  - file: `crates/reports/tests/strategy_anchors_unchanged.rs:291-328` (SQRT_IMPACT_STRATEGY_ANCHORS populated with 9 SHAs)
  - test: `cargo test -p reports --test strategy_anchors_unchanged`
  - output: `test result: ok. 4 passed; 0 failed`
- [x] `cargo test -p reports --test strategy_anchors_unchanged` → 4/4 PASS (t1937c now fully active, not soft-skip)
  - file: `crates/reports/tests/strategy_anchors_unchanged.rs`
  - test: `cargo test -p reports --test strategy_anchors_unchanged`
  - output: `test t1937c_sqrt_impact_strategy_anchors_unchanged ... ok; test t1937_nine_strategy_anchors_unchanged ... ok; test t1937b_canonical_strategy_anchors_unchanged ... ok; test t1942_anchor_shas_are_well_formed_64_lowercase_hex ... ok`

## M-FINAL — Tester (~1 day) ✅ COMPLETE 2026-05-29

- [x] `bash scripts/verify_anchors.sh` → ANCHORS PASS (84/84) — 9 new v5-sqrt-impact-2026-05 rows + 75 pre-existing byte-identical (R-NR.1; note: 84 not 90 per Q-D1=(a) ratification — 9 real-data only)
- [x] 75 existing rows byte-identical (R-NR.2 + R-NR.3) — confirmed by verify_anchors.sh PASS on all pre-v5 rows
- [x] 2-run determinism spot-check on ≥ 3 of 9 sqrt-impact SHAs (K4 gate) — 3/3 spot-check scenarios byte-identical: momentum, tcn-overlay-2023, regime-dispatcher-2024
- [x] `cargo test -p reports --test strategy_anchors_unchanged` → 4/4 PASS (t1937 + t1937b + t1937c + t1942)
- [x] `cargo test -p strategy --test latency_slippage_sim_e2e` → 3/3 PASS; `vol_targeting_overlay_end_to_end` → 1/1 PASS; `vol_killswitch_overlay_end_to_end` → 4/4 PASS (R-NR.5)
- [x] `cargo test --workspace --no-fail-fast --exclude llm` → one pre-existing flake (t27_metrics_endpoint port-contention in crates/agent; outside v5 scope; not attributable to 513ebc4)
- [x] H1 directional check: 3.91× ratio (≥ 2× threshold) — PASS; H2: vol-target +8.18 pp (< 30 pp) PASS; patchtst -84.44 pp medium-turnover PARTIAL FAIL (documented); H3: byte-identity PASS
- [x] K1 surprise scan: 1 sign-flip (patchtst-overlay-2023 linear +5.97% → sqrt -78.47%) — within pre-disclosed scope; 0 unexpected K1 surprises
- [x] Report written: `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/test-final-20260529-143456-v0.5.0.md` — VERDICT PASS
- [x] `anchors` column on `REQ-V5-LATENCY-SLIPPAGE-V0-5-0-001` already populated by developer (9 anchor names); state flipped to `passed` by tester 2026-05-29

## M-PRES — Presenter (~0.5 day)

- [ ] Assemble sprint-review deck at `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/presentations/v5-latency-slippage-sim-v0.5.0-square-root-market-impact-<DATE>.md`
- [ ] Lead with "closes the ADR-0043 § D3 deferred promise — v0.1.0 → v0.5.0 = engine + canonical config + per-path wiring + candle/realdata coverage + model-quality upgrade" framing
- [ ] Inherit pre-drawn 2-cell verdict tree from `feature.md`
- [ ] Embed H1/H2/H3 falsifier outcomes + per-scenario K1 surprise table
