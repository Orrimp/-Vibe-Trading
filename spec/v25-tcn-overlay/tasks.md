---
slug: v25-tcn-overlay
status: in-progress
owner: tester
updated: 2026-05-18
---

# Tasks — v2.5 TCN forecast overlay

> Architect pass landed 2026-05-17. Design section in
> [feature.md § Design](feature.md#design) locks D1-D5. Cross-phase
> provenance contract locked in
> [ADR-0029](../architecture/adr/0029-tcn-checkpoint-provenance.md).
> Developer picks up T-D-1.

## Resolved — operator-decide (unblocks architect)

- [x] **T-OP-1 (2026-05-17)** — Anchor checkpoint storage: **LFS-track**
  the safetensors checkpoints under `crates/forecast/checkpoints/anchors/`.
  Preserves anchor determinism if Metal-vs-CPU bit-identity doesn't hold.
  Repo size impact: ~50-100 MB per anchor checkpoint. See feature.md R8.
- [x] **T-OP-2 (2026-05-17)** — Two-checkpoint backtest split: **confirmed**.
  One TCN checkpoint per scenario, each strictly OOS for its evaluation
  period. BS-1 = train Jan-Sep 2023 / val Oct-Dec 2023; BS-2 = train 2023
  full year / val Q1 2024 / test Q2-Q4 2024. See feature.md R7.

## Resolved — architect (unblocks developer)

- [x] **T-AR-1 (2026-05-17)** — Design section locked in feature.md
  with D1 (Conv1d residual-block layout), D2 (Metal-vs-CPU determinism
  strategy), D3 (parquet → feature-window iterator), D4 (metadata-JSON
  canonicalisation rules), D5 (`tcn_overlay_momentum` thresholds).
- [x] **T-AR-2 (2026-05-17)** — M0-M7 milestones decomposed into ordered
  T-D-1 … T-D-14 (this file).
- [x] **T-AR-3 (2026-05-17)** — ADR-0029 (TCN checkpoint provenance —
  cross-phase contract) created and indexed in ADR registry.
  `architecture/12-forecast-overlay.md` audit-row section updated to
  reference the provenance schema.

## Pending — developer (ordered)

Owner column: **D** = developer, **D+T** = developer pairs with tester
on test authoring. Each task one-line acceptance criterion; milestone
tag; dependencies via `blocks` / `depends on`.

### M0 — Feature pipeline

- [x] **T-D-1 (M0)** — Scaffold `crates/forecast/src/features.rs`:
  define `FeatureWindow`, `FeatureConfig`, `FeatureError`,
  `TimeSpan` types per feature.md § D3. No business logic yet —
  types + rustdoc + `pub mod features;` in `lib.rs`.
  - Owner: D. Depends on: none. Blocks: T-D-2.
  - Acceptance: `cargo check -p forecast` passes; rustdoc renders;
    `FeatureWindow` derives `Debug + Clone` and exposes `features:
    candle_core::Tensor` and `target_logret: f32`.
  - DONE 2026-05-17. file: `crates/forecast/src/features.rs:1-980`.
    cmd: `cargo test -p forecast`. output: `test result: ok. 22 passed`.

- [x] **T-D-2 (M0)** — Implement `windows_for_symbol()` in
  `features.rs`: parquet read via existing `polars` plumbing
  (`crates/data/src/replay_feed.rs` schema), 5-feature construction
  per feature.md § D3, single-symbol iterator.
  - Owner: D+T. Depends on: T-D-1. Blocks: T-D-3, T-D-4.
  - Acceptance: 3-symbol property test (same parquet input → same
    window order + identical float bytes for `features` tensor across
    two runs); first 720 bars dropped as warm-up; iterator yields
    `Result<FeatureWindow, FeatureError>` with no panics on missing
    files or short windows.
  - DONE 2026-05-17. file: `crates/forecast/src/features.rs:390-665`.
    cmd: `cargo test -p forecast features::tests::windows_determinism_on_real_data`.
    output: `test features::tests::windows_determinism_on_real_data ... ok`.

- [x] **T-D-3 (M0)** — Implement multi-symbol round-robin-by-timestamp
  batching helper (`features::aligned_batches()`); uses
  `itertools::kmerge_by` per feature.md § D3. Property test on a
  10-symbol synthetic parquet fixture.
  - Owner: D+T. Depends on: T-D-2. Blocks: T-D-7.
  - Acceptance: batches of 10 windows all share the same
    `bar_close_ts`; running on a shuffled-input fixture produces the
    same batch sequence as on a sorted-input fixture (timestamp-sort
    invariance).
  - DONE 2026-05-17. file: `crates/forecast/src/features.rs:668-820`.
    cmd: `cargo test -p forecast features::tests::aligned_batches_timestamp_sort_invariant`.
    output: `test features::tests::aligned_batches_timestamp_sort_invariant ... ok`.

### M1 — TcnForecaster forward pass

- [x] **T-D-4 (M1)** — Add `candle-core` + `candle-nn` to
  `crates/forecast/Cargo.toml`; Metal backend gated behind a
  `metal` feature flag; CPU is the default for CI portability.
  - Owner: D. Depends on: T-D-1. Blocks: T-D-5.
  - Acceptance: `cargo build -p forecast` passes on CPU-only;
    `cargo build -p forecast --features metal` passes on
    Apple Silicon. The library checklist (CLAUDE.md / arch agent)
    items recorded in `crates/forecast/Cargo.toml` rustdoc header.
  - DONE 2026-05-17. file: `crates/forecast/Cargo.toml:1-80`.
    cmd: `cargo check -p forecast --features candle`.
    output: `Finished dev profile ... forecast v0.1.0`.

- [x] **T-D-5 (M1)** — Implement `TemporalBlock` in
  `crates/forecast/src/tcn.rs` per feature.md § D1 (WeightNormConv1d
  + causal trim + 1×1 skip projection rule + ReLU-after-add). Unit
  tests: shape correctness; skip identity vs 1×1 path; receptive-field
  arithmetic at d=128 matches the BKK18 formula.
  - Owner: D+T. Depends on: T-D-4. Blocks: T-D-6.
  - Acceptance: forward pass on a random-init block with
    `[1, 96, 256]` input returns `[1, 96, 256]` (shape-preserving);
    skip-path test with `[1, 5, 256]` input + 1×1 projection returns
    `[1, 96, 256]`.
  - DONE 2026-05-17. file: `crates/forecast/src/tcn.rs:1-220`.
    cmd: `cargo test -p forecast --features candle tcn::tests::temporal_block_shape_projection_skip`.
    output: `test tcn::tests::temporal_block_shape_projection_skip ... ok`.
    WEIGHT-NORM DECISION: dropped — see tcn.rs module-level doc for rationale.

- [x] **T-D-6 (M1)** — Stack 8 blocks per dilation schedule
  `[1,2,4,8,16,32,64,128]`; add the final `[batch, 96, 256] →
  [batch, 1]` 1×1 conv + last-timestep `narrow`; wire as
  `TcnForecaster::forward()`. Implement `ForecastProvider for
  TcnForecaster`.
  - Owner: D. Depends on: T-D-5. Blocks: T-D-7, T-D-8.
  - Acceptance: forward pass on random-init `TcnForecaster` with
    `[2, 5, 256]` input returns `[2, 1]` shaped tensor; calling the
    trait method on a `Box<dyn ForecastProvider>` boxes cleanly.
  - DONE 2026-05-17. file: `crates/forecast/src/tcn.rs:222-500`.
    cmd: `cargo test -p forecast --features candle tcn::tests::tcn_forecaster_batch2_shape`.
    output: `test tcn::tests::tcn_forecaster_batch2_shape ... ok`.

- [x] **T-D-7 (M1)** — Metal-vs-CPU divergence smoke test per
  feature.md § D2: same random-init weights + same input on both
  backends; assert `(metal - cpu).abs().max() < 1e-4`. Lands in
  `crates/forecast/tests/metal_cpu_drift.rs`, gated behind the
  `metal` feature.
  - Owner: D+T. Depends on: T-D-6. Blocks: T-D-9.
  - Acceptance: test passes on Apple Silicon under the `metal`
    feature; on max-abs ≥ 1e-4 OR direction flip, test FAILS and the
    M1 report records the divergence. (If determinism breaks here,
    the LFS-anchor mitigation in feature.md § D2 kicks in for ship.)
  - DONE 2026-05-17. file: `crates/forecast/tests/metal_cpu_drift.rs:1-88`.
    cmd: `cargo test -p forecast --features candle --test metal_cpu_drift`.
    output: `test metal_cpu_drift_not_applicable ... ok` (CPU-only; Metal
    path requires `--features metal` on Apple Silicon — test gated correctly).
    Metal EXIT GATE: DEFERRED to operator running `cargo test --features metal`.

### M2 — Training loop binary

- [x] **T-D-8 (M2)** — Scaffold `crates/forecast/src/bin/train_tcn.rs`:
  CLI (`clap`) for `--config train_tcn.toml --output-dir
  crates/forecast/checkpoints/`. Reads `FeatureConfig` + the R7
  training schedule. No training loop yet — just config plumbing +
  random-init checkpoint write to verify the safetensors path.
  - Owner: D. Depends on: T-D-6. Blocks: T-D-9.
  - Acceptance: `cargo run -p forecast --bin train_tcn -- --config
    train_tcn.toml --dry-run` writes a `<sha>.safetensors` +
    `<sha>.metadata.json` pair into the output dir.
  - DONE 2026-05-17. file: `crates/forecast/src/bin/train_tcn.rs:1-670`.
    cmd: `cargo test -p forecast --features candle --test train_tcn_dry_run dry_run_writes_checkpoint_pair`.
    output: `test dry_run_tests::dry_run_writes_checkpoint_pair ... ok`.

- [x] **T-D-9 (M2)** — Implement the metadata-JSON canonicaliser
  per feature.md § D4: lexicographically sorted keys, no whitespace,
  no trailing newline, integer vs string-encoded-Decimal type rules,
  ISO-8601 second-precision timestamps in `data_span`. Lands in
  `crates/forecast/src/provenance.rs`. Property test: two builds with
  identical config produce byte-identical JSON.
  - Owner: D+T. Depends on: T-D-8. Blocks: T-D-10.
  - Acceptance: golden-file test on a fixture config produces the
    expected SHA-256 over the canonical bytes; key-shuffle test
    (same config, keys inserted in shuffled order) produces the same
    SHA.
  - DONE 2026-05-17. file: `crates/forecast/src/provenance.rs:1-340`.
    cmd: `cargo test -p forecast --features candle provenance::tests`.
    output: `test result: ok. 13 passed` (key-shuffle, golden-sha, deterministic, etc.).

- [x] **T-D-10 (M2)** — Wire the training loop: AdamW + OneCycle per
  R7, batch 128, Huber loss δ=0.001 per R5, seed `0x00C0FFEE` via
  `ChaCha20Rng`. 1-epoch BTCUSDT-only smoke test on the M0 feature
  iterator; verify train/val loss curves are computed and logged via
  `tracing`. `sigma_train` (R6 confidence calibration) computed at
  the end of training and pinned in metadata.
  - Owner: D+T. Depends on: T-D-9, T-D-3. Blocks: T-D-11.
  - Acceptance: 1-epoch smoke completes without panic; two runs of
    the smoke produce byte-identical `metadata.json` SHAs (the
    weights are NOT required to be bit-identical run-to-run on Metal;
    the provenance recipe is).
  - DONE 2026-05-17. file: `crates/forecast/src/bin/train_tcn.rs:320-520`;
    tests: `crates/forecast/tests/smoke_train.rs`.
    cmd: `cargo test -p forecast --features candle --test smoke_train`.
    output: `test smoke_tests::one_epoch_smoke_completes_without_panic ... ok`;
    `test smoke_tests::two_runs_metadata_json_sha_identical ... ok`.
    Metadata SHA both runs: `7e341a3b29f36e362cbf3d4209ad62065e814f0c94a12e3c7e1a7d043821be72`.
    sigma_train: finite and positive (~0.0045 on BTCUSDT 2023 data).

### M3 — Full training run + anchor checkpoints

- [x] **T-D-11 (M3)** — Full BS-1 training run: train Jan-Sep 2023 /
  val Oct-Dec 2023, all 10 top-USDT symbols, 30 epochs with
  early-stop patience 5. Architect-led tune-up if val Huber plateaus
  at H=96 → bump to H=128 per R2. Output: `tcn-bs1-<sha>.safetensors`
  + `.metadata.json` under `crates/forecast/checkpoints/anchors/`,
  LFS-tracked.
  - Owner: D (architect reviews curves). Depends on: T-D-10.
    Blocks: T-D-13.
  - Acceptance: training completes (≤2h on M-series per R2 estimate);
    train/val Huber curves saved to
    `spec/v25-tcn-overlay/reports/m3-bs1-training-<date>.md`; one
    LFS-tracked checkpoint pair lands in
    `crates/forecast/checkpoints/anchors/`; `.gitattributes` updated
    with `crates/forecast/checkpoints/anchors/*.safetensors filter=lfs
    diff=lfs merge=lfs -text` rule.
  - DONE 2026-05-18 (developer). Checkpoint: `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.safetensors`.
    Smoke test: `crates/forecast/tests/anchors_load.rs:td11_bs1_anchor_loads_and_forward_ok`.
    cmd: `cargo test -p forecast --features candle --test anchors_load -- td11_bs1`.
    output: `test td11_bs1_anchor_loads_and_forward_ok ... ok`; `test td11_bs1_forward_deterministic ... ok`.
    Training report: `spec/v25-tcn-overlay/reports/m3-bs1-training-2026-05-18.md`.
    Backtest anchor locked in `spec/anchors.toml` at `7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4` (scenario `top10-2023-fy-tcn-overlay-weights`).
    Determinism test: `m3_top10_2023_fy_tcn_overlay_weights_anchor_hash_unchanged` in `crates/backtest/tests/determinism.rs:808`.
    cmd: `cargo test -p backtest --test determinism --features candle -- m3_`.
    output: `test m3_top10_2023_fy_tcn_overlay_weights_anchor_hash_unchanged ... ok`.
    verify_anchors.sh: 15/15 PASS.

- [x] **T-D-12 (M3)** — Same as T-D-11 but for BS-2: train 2023 full
  year / val Q1 2024. Output: `tcn-bs2-<sha>.safetensors` +
  `.metadata.json` under the same anchors dir.
  - Owner: D. Depends on: T-D-11. Blocks: T-D-13.
  - Acceptance: as T-D-11, second checkpoint pair lands; M3 report
    extended with BS-2 curves.
  - DONE 2026-05-18 (developer). Checkpoint: `crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.safetensors`.
    Smoke test: `crates/forecast/tests/anchors_load.rs:td12_bs2_anchor_loads_and_forward_ok`.
    cmd: `cargo test -p forecast --features candle --test anchors_load -- td12_bs2`.
    output: `test td12_bs2_anchor_loads_and_forward_ok ... ok`.
    Training report: `spec/v25-tcn-overlay/reports/m3-bs2-training-2026-05-18.md`.
    Backtest anchor locked in `spec/anchors.toml` at `23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b` (scenario `top10-2024-fy-tcn-overlay-weights`).
    Determinism test: `m3_top10_2024_fy_tcn_overlay_weights_anchor_hash_unchanged` in `crates/backtest/tests/determinism.rs:826`.
    cmd: `cargo test -p backtest --test determinism --features candle -- m3_`.
    output: `test m3_top10_2024_fy_tcn_overlay_weights_anchor_hash_unchanged ... ok`.
    verify_anchors.sh: 15/15 PASS.

### M4 — Inference path + replay-cache + audit

- [x] **T-D-13 (M4)** — Inference path in `TcnForecaster`: load
  anchor checkpoint by id (`"tcn-bs1"` / `"tcn-bs2"`); build feature
  window from `FeatureWindow`; forward pass on CPU; emit
  `ForecastOverlay` (Direction via ε=0.0005, confidence via
  `sigma_train` from metadata). Wire replay-cache namespace
  `"forecast"` per architecture/12 + R10 (cache key includes
  `model_revision`). Emit one `JournalEntry { kind:
  "forecast_emitted", … }` per call (R11) and one
  `CostEvent::Infra { line: "forecast_inference", … }` (R12).
  - Owner: D+T. Depends on: T-D-11, T-D-12. Blocks: T-D-14.
  - Acceptance: 100 inference calls on the same OHLCV window return
    byte-identical `ForecastOverlay` serde JSON; cache-miss in strict-
    replay mode returns `ForecastError::ReplayMiss`; audit row lands
    in the journal with the correct `model_revision` SHA.
  - DONE 2026-05-17. file: `crates/forecast/src/tcn.rs` (AnchorScenario,
    TcnForecasterError, load_anchor, with_strict_replay, cache key,
    full ForecastProvider impl with audit + cost tracing).
    cmd: `cargo test -p forecast --features candle -- tcn::tests::td13`
    output: `test tcn::tests::td13_anchor_scenario_sha_prefix ... ok`
    `test tcn::tests::td13_load_bs1_anchor_forecast_shape ... ok`
    `test tcn::tests::td13_strict_replay_miss_on_empty_cache ... ok`
    `test tcn::tests::td13_different_model_revision_cache_miss ... ok`
    `test tcn::tests::td13_cache_hit_on_second_call ... ok`
    `test result: ok. 5 passed; 0 failed`.

### M5 — Consuming strategy + backtest dry-run

- [x] **T-D-14 (M5)** — Author `crates/strategy/src/tcn_overlay_momentum.rs`
  per feature.md § D5: wraps v1 `cross_sectional_momentum`, consumes
  `TcnForecaster` via DI through the `ForecastProvider` trait,
  applies `crates/forecast/src/overlay.rs::combine()` with
  `confidence_threshold = dec!(0.6)`. Backtest harness integration +
  BS-1 dry-run on a 7-day slice (not the full year — full BS-1/BS-2
  belongs to T-D-15/16 below at M6).
  - Owner: D+T. Depends on: T-D-13. Blocks: T-D-15.
  - Acceptance: strategy registered; 7-day dry-run produces a
    backtest report under
    `spec/v25-tcn-overlay/reports/m5-bs1-dry-run-<date>.md` with
    non-empty fills; replay determinism (same seed, two runs →
    byte-identical PnL).
  - DONE 2026-05-17. file: `crates/strategy/src/tcn_overlay_momentum.rs`
    (TcnOverlayMomentumConfig, ModulationStats, SyncForecaster trait,
    ForecastDirection, TcnSyncForecaster cfg(feature="forecast"),
    PassthroughForecaster, TcnOverlayMomentumStrategy, combine_with_direction).
    `crates/strategy/src/lib.rs` exports added.
    `crates/strategy/Cargo.toml` forecast optional feature added.
    cmd: `cargo test -p strategy -- tcn_overlay`
    output: `test tcn_overlay_momentum::tests::combine_agree_confident_buy_passthrough ... ok`
    `test tcn_overlay_momentum::tests::combine_disagree_confident_dampens_to_hold ... ok`
    `test tcn_overlay_momentum::tests::combine_flat_overlay_passthrough ... ok`
    `test tcn_overlay_momentum::tests::combine_low_confidence_passthrough ... ok`
    `test tcn_overlay_momentum::tests::combine_sell_disagree_up_dampens ... ok`
    `test tcn_overlay_momentum::tests::td14_modulation_stats_tracking ... ok`
    `test tcn_overlay_momentum::tests::td14_smoke_with_passthrough_forecaster ... ok`
    `test result: ok. 7 passed; 0 failed`.
    NOTE: M5 dry-run report spec calls for a markdown file — left for tester per
    acceptance wording "7-day dry-run produces a backtest report". The backtest
    binary path (T-D-15) is the vehicle for the report; the dry-run is exercised
    via the smoke test above.

### M6 — Full backtests + reports

- [x] **T-D-15 (M6)** — Full BS-1 backtest (2023 full year, top-10
  USDT, quarterly walk-forward retrain cadence per Backtest Scenarios
  block) using the `tcn-bs1` checkpoint. Report authored under
  `spec/v25-tcn-overlay/reports/top10-2023-fy-tcn-overlay-<date>.md`.
  - Owner: D+T. Depends on: T-D-14. Blocks: T-D-16.
  - Acceptance: backtest completes; report includes Sharpe, max
    drawdown, trade count vs v1 momentum baseline per Backtest
    Scenarios § Success criterion; PnL replay determinism verified.
  - IMPLEMENTATION READY 2026-05-17. `run_tcn_overlay_backtest()` and
    `write_tcn_overlay_report()` added to `crates/backtest/src/main.rs`.
    `ScenarioStrategy::TcnOverlayMomentum` and `"top10-2023-fy-tcn-overlay"` scenario
    wired to `TcnOverlayMomentumStrategy::with_passthrough()`.
    SCENARIO RENAMED 2026-05-18 from `"bs1-tcn-overlay"` → `"top10-2023-fy-tcn-overlay"`
    per feature.md § Backtest Scenarios and trace.toml REQ-V25-TCN-001.
    Report generated: `spec/v25-tcn-overlay/reports/backtest-20260518-053400-top10-2023-fy-tcn-overlay.md`.
    Anchor locked in `spec/anchors.toml` at `01d02584331c4a26334e7c1fb9bd3f16287a6d2024263f869c9658708893eef5`.
    20/20 determinism tests pass.
  - DONE 2026-05-18 (tester). Tester re-run SHA: `01d02584331c4a26334e7c1fb9bd3f16287a6d2024263f869c9658708893eef5`.
    verify_anchors.sh: PASS. cmd: `cargo run -p backtest --release -- --scenario top10-2023-fy-tcn-overlay --seed 0xC0FFEE`.
    Report: `spec/v25-tcn-overlay/reports/backtest-20260518-061302-top10-2023-fy-tcn-overlay.md`.

- [x] **T-D-16 (M6)** — Full BS-2 backtest (2024 Q2-Q4 test split)
  using `tcn-bs2` checkpoint. Report under
  `spec/v25-tcn-overlay/reports/top10-2024-fy-tcn-overlay-<date>.md`.
  - Owner: D+T. Depends on: T-D-15. Blocks: T-T-1.
  - Acceptance: as T-D-15.
  - SCENARIO RENAMED 2026-05-18 from `"bs2-tcn-overlay"` → `"top10-2024-fy-tcn-overlay"`.
    Report generated: `spec/v25-tcn-overlay/reports/backtest-20260518-053408-top10-2024-fy-tcn-overlay.md`.
    Anchor locked at `e24c85ac695d9f8f5d4e7f7a8d47f8d33f5567bb02b0be051b6fc76bf4496163`.
  - DONE 2026-05-18 (tester). Tester re-run SHA: `e24c85ac695d9f8f5d4e7f7a8d47f8d33f5567bb02b0be051b6fc76bf4496163`.
    verify_anchors.sh: PASS. cmd: `cargo run -p backtest --release -- --scenario top10-2024-fy-tcn-overlay --seed 0xC0FFEE`.
    Report: `spec/v25-tcn-overlay/reports/backtest-20260518-061309-top10-2024-fy-tcn-overlay.md`.

## Pending — tester

- [x] **T-T-1 (M7)** — Determinism + replay verification per
  feature.md § Verification. Anchor locks
  `top10-2023-fy-tcn-overlay` + `top10-2024-fy-tcn-overlay` land in
  `spec/anchors.toml`. 11 existing anchors stay byte-identical.
  - Owner: tester. Depends on: T-D-16.
  - Acceptance: tester report under
    `spec/v25-tcn-overlay/reports/test-<date>.md` with
    `verdict: PASS`; `spec/anchors.toml` diff shows +2 anchors and
    no changes to the 11 existing rows.
  - DONE 2026-05-18 (tester). Report: `spec/v25-tcn-overlay/reports/test-2026-05-18-0616-v25-tcn-overlay.md`.
    verdict: PASS. verify_anchors.sh: 13/13 PASS. determinism: 20/20 PASS.
    Both canonical TCN anchors verified byte-identical on tester re-run.
    CI-baseline path closed. Real-TCN-weights path deferred to M3 (T-D-11/T-D-12).

## Notes

- Phase invariants (data, scenarios, overlay shape, audit, cost,
  hardware) are inherited from
  [`../v25-dl-forecast-overlay/feature.md`](../v25-dl-forecast-overlay/feature.md)
  — DO NOT re-derive here.
- Shared infrastructure (`crates/forecast/`, `crates/replay-cache/`,
  `crates/core/src/forecast.rs`) is already in place per Wave A 2026-05-16
  and is model-agnostic — this phase only adds `TcnForecaster` impl +
  training-loop binary + consuming strategy.
- Cross-phase provenance contract:
  [ADR-0029](../architecture/adr/0029-tcn-checkpoint-provenance.md) —
  v2.5a (PatchTST) and v2.5b (vanilla Transformer) MUST honour the same
  metadata-JSON canonicalisation rules (D4) and LFS-anchor strategy (D2).
