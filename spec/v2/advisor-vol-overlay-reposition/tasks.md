---
slug: advisor-vol-overlay-reposition
status: dev-done
owner: developer
version: 2.0.0
updated: 2026-06-30
---

# Tasks — Vol-Overlay Reposition (P1-4)

## Developer — DONE

- [x] **Switch the vol source to EWMA (P1-4 path).** Added `VolSource` enum
  (`Ewma`/`Garch`) and `VolTargetingConfig::vol_source` field. New EWMA path calls
  `vol_estimator::ewma_realized_vol` with `LAMBDA_126D_HOURLY` (≈ 0.999771). Legacy
  GARCH path kept for backward-compat.
  - file:line `crates/strategy/src/vol_targeting_overlay.rs:88–125`
  - test cmd `cargo test -p strategy --lib vol_targeting_overlay::tests::ewma_vol_source_computes_sigma_after_warmup`
  - output `test vol_targeting_overlay::tests::ewma_vol_source_computes_sigma_after_warmup ... ok`

- [x] **No-trade band (`no_trade_band`).** Added `no_trade_band: f64` to
  `VolTargetingConfig` (default 0.0 for backward-compat; 0.05 in `p1_4_defaults()`).
  `apply_policy` suppresses resize when `|candidate−current|/current ≤ band`.
  - file:line `crates/strategy/src/vol_targeting_overlay.rs:196–218`
  - test cmd `cargo test -p strategy --lib vol_targeting_overlay::tests::no_trade_band_suppresses_small_change`
  - output `test vol_targeting_overlay::tests::no_trade_band_suppresses_small_change ... ok`

- [x] **De-risk-only (`derisk_only`).** Added `derisk_only: bool` (default `false`
  for backward-compat; `true` in `p1_4_defaults()`). `apply_policy` caps candidate
  scale at `current_scale.min(1.0)` when `derisk_only=true` and raw > cap.
  - file:line `crates/strategy/src/vol_targeting_overlay.rs:230–244`
  - test cmd `cargo test -p strategy --lib vol_targeting_overlay::tests::derisk_only_blocks_upsize`
  - output `test vol_targeting_overlay::tests::derisk_only_blocks_upsize ... ok`

- [x] **`p1_4_defaults()` constructor.** `VolTargetingConfig::p1_4_defaults()` sets
  `vol_source=Ewma`, `ewma_lambda=LAMBDA_126D_HOURLY`, `no_trade_band=0.05`,
  `derisk_only=true`.
  - file:line `crates/strategy/src/vol_targeting_overlay.rs:219–232`
  - test cmd `cargo test -p strategy --lib vol_targeting_overlay::tests::p1_4_defaults_sets_expected_fields`
  - output `test vol_targeting_overlay::tests::p1_4_defaults_sets_expected_fields ... ok`

- [x] **`ReturnVolCorrelation` struct + `pearson_correlation` helper.** Accumulates
  `(return, sigma_hat)` pairs per symbol; exposes `rho: Option<f64>` and `n_obs()`.
  Pearson ρ undefined for constant series (returns `None`).
  - file:line `crates/strategy/src/vol_targeting_overlay.rs:260–325`
  - test cmd `cargo test -p strategy --lib vol_targeting_overlay::tests::return_vol_correlation_positive_series`
  - output `test vol_targeting_overlay::tests::return_vol_correlation_positive_series ... ok`

- [x] **Existing e2e stays green.** `VolTargetingConfig::default()` still uses
  `VolSource::Garch`, `no_trade_band=0.0`, `derisk_only=false` — backward-compat
  preserved. No divergence assertion needed (defaults did not change).
  - file:line `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs:99–145`
  - test cmd `cargo test -p strategy --test vol_targeting_overlay_end_to_end -- --nocapture`
  - output `test overlay_quantity_scale_reflects_computed_factor ... ok`

- [x] **266 lib tests pass (233 before → 266 after; 33 new).** All vol_targeting_overlay
  tests green; no regressions in any other module.
  - file:line `crates/strategy/src/vol_targeting_overlay.rs:835–1280`
  - test cmd `cargo test -p strategy --lib`
  - output `test result: ok. 266 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`

- [x] **Clippy clean.** `cargo clippy -p strategy --tests -- -D warnings` passes.
  - Also fixed `doc-lazy-continuation` in `drawdown_control_overlay_end_to_end.rs`
    (pre-existing from P1-3 developer) and `..Default::default()` in the vol_targeting
    integration test.

- [x] **`cargo fmt --check` passes.**

- [x] **Anchors 119/119.** Overlay operates on `write_report=false` advisor path —
  anchor-safe by construction. `bash scripts/verify_anchors.sh` → `ANCHORS PASS (119/119)`.

- [x] **Frozen gate tests pass.**
  - `cargo test -p backtest --lib scorecard_does_not_change_ranking` → ok
  - `cargo test -p backtest --lib turnover_does_not_change_ranking` → ok

## Tester — to verify (T_FINAL_*)

- [ ] Full `cargo test -p strategy` (lib + integration) regression pass.
- [ ] VERDICT report under `spec/v2/advisor-vol-overlay-reposition/reports/`.
- [ ] `spec/trace.toml` `REQ-V2-P1-4-VOL-OVERLAY-REPOSITION-001` `state = "tester-done"`.
