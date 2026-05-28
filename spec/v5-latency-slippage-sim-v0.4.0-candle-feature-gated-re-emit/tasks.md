---
slug: v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit
status: in-progress
owner: developer
updated: 2026-05-28
---

# tasks — v5 latency-slippage-sim v0.4.0 candle/realdata feature-gated re-emit

## M0 — Analyst (~0.5 day) ✅ in-flight

- [x] Author `feature.md` v0.1.0 (4 R / 4 K / 3 H / 2 Q + non-regression contract + 2-cell verdict tree + cost framing)
- [x] Append `[[req]] REQ-V5-LATENCY-SLIPPAGE-V0-4-0-001` to `spec/trace.toml` (state = `proposed`)
- [x] Append Active row to `spec/backlog.md`
- [x] Verify gates green: `bash scripts/verify_anchors.sh` PASS (70/70); `python3.14 scripts/spec_lint.py` no NEW categories
- [ ] HANDOFF → operator-decide (Q1-Q2 standing-Autoapprove-eligible per analyst recommendation; M-OD likely empty)

## M-OD — Operator-decide (~0 day, standing-Autoapprove)

- [ ] Q1 — Canonical box for candle/realdata feature-flagged rebuild — **analyst-recommended (a) Apple Silicon M-series** (operator-locked since v2.5 TCN; Metal CPU drift prior)
- [ ] Q2 — Standing-Autoapprove-eligible — **analyst-recommended (a) yes** (pure rebuild + re-emit; no design changes)

## M-T1 — Architect (~0 day, fast-skip) ✅ closed 2026-05-28

- [x] Confirm no design changes vs v0.3.0 (ADR-0047 carries forward unchanged) — D1-D6 all cover v0.4.0; no ADR-0048 needed
- [x] Confirm `data/binance/REVISION.toml` SHA still matches `3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7` (K2 precondition) — byte-match verified
- [x] Confirm PatchTST BS-1 checkpoint still at `model_revision 62520db9...` (K2 precondition) — `.safetensors` + `.metadata.json` present at `crates/forecast/checkpoints/anchors/`; metadata `model_revision` field byte-matches
- [x] No precondition drift — fast-skip handoff to developer (see feature.md § Design M-T1 close note)

## M-DEV — Developer (~1 day, sequential Waves A-D)

### Wave A — Feature-flagged rebuild + 8-scenario re-emission (~0.5-1 day)

- [ ] Build canonical binary: `cargo build --release -p backtest --features "candle realdata"` on Apple Silicon
- [ ] Run each of the 8 scenarios under canonical `LatencySlippageSimConfig { 30, 80, 8 }` (ADR-0045 D1; ADR-0047 D4 inherits unchanged):
  - [ ] `top10-2023-fy-tcn-overlay-weights`
  - [ ] `top10-2024-fy-tcn-overlay-weights`
  - [ ] `top10-2023-fy-tcn-overlay-realdata`
  - [ ] `top10-2024-fy-tcn-overlay-realdata`
  - [ ] `top10-2023-fy-tcn-overlay-weights-realdata`
  - [ ] `top10-2024-fy-tcn-overlay-weights-realdata`
  - [ ] `top10-2023-fy-patchtst-overlay-realdata`
  - [ ] `top10-2023-fy-vol-target-overlay-realdata`
- [ ] Emit reports to `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/backtest-<YYYYMMDD>-<HHMMSS>-<scenario>.md`
- [ ] **Determinism gate**: run each scenario twice; assert byte-identical body-SHAs (R-NR + K4 falsifier)

### Wave B — Anchor SHA migration (~0.1 day)

- [ ] Update 8 SHAs in `spec/anchors.toml` canonical section (lines 392-420 + 472-475 + 482-485); namespace pin `v5-realdata-medium-2026-05` unchanged
- [ ] Run `bash scripts/verify_anchors.sh` → expect `ANCHORS PASS (70 / 70)`

### Wave C — Sharpe-delta table addendum (~0.25 day)

- [ ] Author `reports/sharpe-delta-table-<DATE>.md` extending v0.3.0 series; flip Groups E-H from `=noop (candle/realdata absent)` to live Δ Equity / Δ Sharpe
- [ ] Run K1 surprise scan across the 8 newly-friction-real scenarios; verify H3 holds (0 K1 surprises) or flag for R-O2 route

### Wave D — t1937b `CANONICAL_STRATEGY_ANCHORS` table extension (~0.1 day)

- [ ] Extend `crates/reports/tests/strategy_anchors_unchanged.rs` `CANONICAL_STRATEGY_ANCHORS` with 8 new entries
- [ ] `cargo test -p reports --test strategy_anchors_unchanged` → expect 3/3 PASS (t1937 + t1937b + t1942)

## M-FINAL — Tester (~0.5 day)

- [ ] `bash scripts/verify_anchors.sh` → PASS 70/70 (R-NR.1)
- [ ] Confirm 8 noop-baseline rows at `spec/anchors.toml:121-155, 242, 272` byte-identical (R-NR.2)
- [ ] Confirm 11 v0.3.0 canonical SHAs unchanged (R-NR.3)
- [ ] Determinism spot-check (2 scenarios independently re-run; SHA match against anchors.toml) — K4 gate
- [ ] `cargo test -p reports --test strategy_anchors_unchanged` → 3/3 PASS
- [ ] `cargo test -p strategy --test latency_slippage_sim_e2e` → 3/3 PASS (R-NR.6)
- [ ] `cargo test -p strategy --test vol_targeting_overlay_end_to_end` → 1/1 PASS
- [ ] `cargo test -p strategy --test vol_killswitch_overlay_end_to_end` → 4/4 PASS
- [ ] `cargo test --workspace --no-fail-fast` → no new failures vs v0.3.0 whitelist
- [ ] Author `reports/test-final-<DATE>-v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit.md` with verdict (PASS or REGRESSION)
- [ ] Populate `anchors` column on `REQ-V5-LATENCY-SLIPPAGE-V0-4-0-001` trace row + flip state to `passed`

## M-PRES — Presenter (~0.5 day)

- [ ] Assemble sprint-review deck at `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/presentations/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit-<DATE>.md`
- [ ] Lead with "closes the v5 anchor-migration arc end-to-end" framing (v0.1 → v0.2 → v0.3 → v0.4 = 19/19 friction-real scenarios)
- [ ] Inherit pre-drawn 2-cell verdict tree from `feature.md`
