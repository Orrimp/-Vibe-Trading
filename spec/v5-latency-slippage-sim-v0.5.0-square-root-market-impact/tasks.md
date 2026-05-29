---
slug: v5-latency-slippage-sim-v0.5.0-square-root-market-impact
status: draft
owner: analyst
updated: 2026-05-29
---

# tasks — v5 latency-slippage-sim v0.5.0 square-root market-impact

## M0 — Analyst (~0.5 day) ✅ in-flight

- [x] Author `feature.md` v0.1.0 (5 R / R-NR / 4 K / 3 H / 3 Q + pre-drawn 2-cell verdict tree + cost framing both routes)
- [x] Append `[[req]] REQ-V5-LATENCY-SLIPPAGE-V0-5-0-001` to `spec/trace.toml` (state = `proposed`)
- [x] Append Active row to `spec/backlog.md`
- [x] Verify gates green: anchors 71/71 PASS; spec_lint baseline-stable
- [ ] HANDOFF → operator-decide (Q1-Q3 all default to DURABLE per AGENT.md 2026-05-29; Autoapprove-eligible at analyst defaults)

## M-OD — Operator-decide (~0 day, all-DURABLE Autoapprove-eligible)

- [ ] **Q1** — Impact coefficient α — analyst-recommended **(a) α = 1.0** (Kissell 2014 midpoint; DURABLE). Cheap fallback (b) α = 0.5 = +v0.5.1 calibration brief STRICTLY WORSE.
- [ ] **Q2** — Per-asset volume V source — analyst-recommended **(a) 90-day trailing Binance parquet** (revision-pinned). Cheap fallback (b) universe-average = +v0.6.0 V-source replacement STRICTLY WORSE.
- [ ] **Q3** — Synthetic-scenario behavior — analyst-recommended **(a) Linear { bps: 8 } fallback for synthetic** (preserves 9 of 19 SHAs byte-identical across namespaces). Cheap fallback (b) universe-V on synthetic = muddies twin contract STRICTLY WORSE.

## M-T1 — Architect (~1 day)

- [ ] Lock numerical-precision contract for `√` over Decimal (K2): f64 boundary placement; round-half-to-even on `slippage_bps_effective` u32 conversion
- [ ] Pick per-asset volume retrieval shape (R3): Option A (extend `crates/data` with `DailyVolume` query) vs Option B (bake `volume_proxy.toml`)
- [ ] Lock `MAX_SLIPPAGE_BPS` cap (K3): default 1_000 (10%); confirm or override
- [ ] ADR decision: amendment to ADR-0043 § D3 (preferred — durable; the deferred promise being closed) vs new ADR-0050. Note: amendment preserves the engine ADR's continuity; new ADR triggers if the model swap is decoupled enough to be a sibling decision.
- [ ] Confirm namespace `v5-sqrt-impact-2026-05` is the correct pin (mirrors ADR-0045 D2 namespace-twin pattern; parallel to `v5-realdata-medium-2026-05`)
- [ ] Decompose M-DEV into Waves A (model body) / B (enum plumbing) / C (volume retrieval) / D (re-emission) / E (anchor migration + Sharpe-delta table) / F (t1937 third-namespace extension)

## M-DEV — Developer (~3-5 days, Waves A-F)

### Wave A — Model body in `crates/cost/src/slippage.rs` (~0.5-1 day)

- [ ] Replace `apply_slippage(price, side, _notional, bps)` with model-dispatching variant; add `SlippageModel` enum
- [ ] Implement `apply_slippage_sqrt(signal_price, side, notional, alpha, v_daily, max_bps)` with f64-boundary contract (architect-locked at M-T1)
- [ ] Unit tests: `α=1.0, Q=$1M, V=$1B → ~32 bps`; cap saturation at MAX_SLIPPAGE_BPS; deterministic across architectures

### Wave B — Enum plumbing through `LatencySlippageSimConfig` (~0.5 day)

- [ ] Replace `slippage_bps: u16` with `slippage_model: SlippageModel` on `LatencySlippageSimConfig` (backtest crate)
- [ ] Serde adapter: old `slippage_bps: u16` deserializes to `Linear { bps }` for backward-compat (R-NR.2 oracle preservation)
- [ ] Update `crates/backtest/src/scenarios/sim.rs::sim_slippage_cost` to dispatch on enum (ADR-0047 D2 SOLE-LOCATION grep gate stays green)

### Wave C — Per-asset volume retrieval (~0.5-1 day, architect-locked shape)

- [ ] Implement R3 retrieval per M-T1 decision (Option A `DailyVolume` query OR Option B `volume_proxy.toml`)
- [ ] Synthetic-data fallback (K1): emit explicit log line `slippage_model=Linear (fallback: synthetic data has no V proxy)`; route Group A/D/E to Linear { bps: 8 }

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

- [ ] Extend `crates/reports/tests/strategy_anchors_unchanged.rs`: add `SqrtImpact` to `Namespace` enum; add `SQRT_IMPACT_FEATURE_DIRS` + `SQRT_IMPACT_STRATEGY_ANCHORS` constants; add `t1937c_sqrt_impact_strategy_anchors_unchanged` test
- [ ] `cargo test -p reports --test strategy_anchors_unchanged` → 4/4 PASS

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
