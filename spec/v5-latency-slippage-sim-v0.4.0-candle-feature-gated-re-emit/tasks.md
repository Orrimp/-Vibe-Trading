---
slug: v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit
status: in-progress
owner: presenter
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

- [x] Build canonical binary: `cargo build --release -p backtest --features "candle realdata"` on Apple Silicon
  - file: `crates/backtest/Cargo.toml` (feature definitions lines 21-33)
  - test cmd: `cargo build --release -p backtest --features "candle realdata"`
  - output: `Finished 'release' profile [optimized] target(s) in 7.38s`
- [x] Run each of the 8 scenarios under canonical `LatencySlippageSimConfig { 30, 80, 8 }` (ADR-0045 D1; ADR-0047 D4 inherits unchanged):
  - [x] `top10-2023-fy-tcn-overlay-weights` → `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/backtest-20260528-182120-top10-2023-fy-tcn-overlay-weights.md`
  - [x] `top10-2024-fy-tcn-overlay-weights` → `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/backtest-20260528-182154-top10-2024-fy-tcn-overlay-weights.md`
  - [x] `top10-2023-fy-tcn-overlay-realdata` → `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/backtest-20260528-182204-top10-2023-fy-tcn-overlay-realdata.md`
  - [x] `top10-2024-fy-tcn-overlay-realdata` → `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/backtest-20260528-182214-top10-2024-fy-tcn-overlay-realdata.md`
  - [x] `top10-2023-fy-tcn-overlay-weights-realdata` → `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/backtest-20260528-182304-top10-2023-fy-tcn-overlay-weights-realdata.md`
  - [x] `top10-2024-fy-tcn-overlay-weights-realdata` → `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/backtest-20260528-182352-top10-2024-fy-tcn-overlay-weights-realdata.md`
  - [x] `top10-2023-fy-patchtst-overlay-realdata` → `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/backtest-20260528-182438-top10-2023-fy-patchtst-overlay-realdata.md`
  - [x] `top10-2023-fy-vol-target-overlay-realdata` → `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/backtest-20260528-182448-top10-2023-fy-vol-target-overlay-realdata.md`
- [x] Emit reports to `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/backtest-<YYYYMMDD>-<HHMMSS>-<scenario>.md`
  - All 8 reports emitted at `20260528-18xxxx` timestamps.
- [x] **Determinism gate**: run each scenario twice; assert byte-identical body-SHAs (R-NR + K4 falsifier)
  - All 8 second-run SHAs byte-match first-run SHAs. Compound determinism (candle × realdata × friction) confirmed. K4 NOT triggered.
  - test cmd: `python3 scripts/hash_report.py <path>` on both runs
  - output: 8/8 SHA pairs identical

### Wave B — Anchor SHA migration (~0.1 day)

- [x] Update 8 SHAs in `spec/anchors.toml` canonical section (lines 392-420 + 472-475 + 482-485); namespace pin `v5-realdata-medium-2026-05` unchanged
  - file:line: `spec/anchors.toml` lines 395, 400, 405, 410, 415, 420, 475, 485
  - Also updated: `scripts/verify_anchors.sh` — added `migration_dir_v04` variable; resolver updated to check v0.4.0 first
- [x] Run `bash scripts/verify_anchors.sh` → `ANCHORS PASS (70 / 70)`
  - test cmd: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (70 / 70)`

### Wave C — Sharpe-delta table addendum (~0.25 day)

- [x] Author `reports/sharpe-delta-table-<DATE>.md` extending v0.3.0 series; flip Groups E-H from `=noop (candle/realdata absent)` to live Δ Equity / Δ Sharpe
  - file: `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/sharpe-delta-table-2026-05-28.md`
  - 19 friction-real scenarios now covered (was 11 at v0.3.0)
- [x] Run K1 surprise scan across the 8 newly-friction-real scenarios; verify H3 holds (0 K1 surprises) or flag for R-O2 route
  - Result: **0 K1 surprises.** H3 holds. All 8 scenarios remain equity-positive under canonical friction. No retirement candidates.
  - H1 CONFIRMED (candle path). H2 FALSIFIED (PatchTST fewer trades). H3 CONFIRMED.

### Wave D — t1937b `CANONICAL_STRATEGY_ANCHORS` table extension (~0.1 day)

- [x] Extend `crates/reports/tests/strategy_anchors_unchanged.rs` `CANONICAL_STRATEGY_ANCHORS` with 8 new entries
  - file:line: `crates/reports/tests/strategy_anchors_unchanged.rs` lines 79-82 (CANONICAL_FEATURE_DIRS), lines 214+ (8 new CANONICAL_STRATEGY_ANCHORS entries)
- [x] `cargo test -p reports --test strategy_anchors_unchanged` → `3/3 PASS` (t1937 + t1937b + t1942)
  - test cmd: `cargo test -p reports --test strategy_anchors_unchanged`
  - output: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s`

## M-FINAL — Tester (~0.5 day) ✅ completed 2026-05-28

- [x] `bash scripts/verify_anchors.sh` → PASS 70/70 (R-NR.1)
  - test cmd: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (70 / 70)` — 70/70 rows PASS including 8 new canonical SHAs
- [x] Confirm 8 noop-baseline rows at `spec/anchors.toml:121-155, 242, 272` byte-identical (R-NR.2)
  - verify_anchors.sh PASS on noop-baseline rows confirms byte-immutability
- [x] Confirm 11 v0.3.0 canonical SHAs unchanged (R-NR.3)
  - verify_anchors.sh PASS on all Group A-D v5-realdata-medium-2026-05 rows confirms
- [x] Determinism spot-check (2 scenarios independently re-run; SHA match against anchors.toml) — K4 gate
  - test cmd: `python3 scripts/hash_report.py <report-path>` on 2 scenarios
  - `top10-2023-fy-patchtst-overlay-realdata`: `55c5b715...` = anchors.toml line 475 — MATCH
  - `top10-2023-fy-vol-target-overlay-realdata`: `4edd8cc5...` = anchors.toml line 485 — MATCH
- [x] `cargo test -p reports --test strategy_anchors_unchanged` → 3/3 PASS
  - output: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s`
- [x] `cargo test -p strategy --test latency_slippage_sim_e2e` → 3/3 PASS (R-NR.6)
  - output: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.90s`
- [x] `cargo test -p strategy --test vol_targeting_overlay_end_to_end` → 1/1 PASS
  - output: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`
- [x] `cargo test -p strategy --test vol_killswitch_overlay_end_to_end` → 4/4 PASS
  - output: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`
- [x] `cargo test --workspace --no-fail-fast` → no new failures vs v0.3.0 whitelist
  - Pre-existing whitelisted failures only: lab_run_engine (flake), render_snapshots + visual_snapshots (UI-track parallel features post-v0.3.0, not attributable to v0.4.0)
  - Zero new failures attributable to v0.4.0 (crates touched: reports, scripts, spec only)
- [x] Author `reports/test-final-2026-05-28-v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit.md` with verdict (PASS)
- [x] Populate `anchors` column on `REQ-V5-LATENCY-SLIPPAGE-V0-4-0-001` trace row + flip state to `passed`
  - anchors column converted from SHA hex → scenario names (spec_lint.py compliance)
  - state flipped `dev-complete → passed` with tester M-FINAL citation

## M-PRES — Presenter (~0.5 day)

- [ ] Assemble sprint-review deck at `spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/presentations/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit-<DATE>.md`
- [ ] Lead with "closes the v5 anchor-migration arc end-to-end" framing (v0.1 → v0.2 → v0.3 → v0.4 = 19/19 friction-real scenarios)
- [ ] Inherit pre-drawn 2-cell verdict tree from `feature.md`
