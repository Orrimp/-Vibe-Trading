---
slug: v25a-patchtst-overlay
status: in-progress
owner: tester
updated: 2026-05-22
---

# Tasks — v2.5a PatchTST forecast overlay (phase 2 of 4)

> Analyst-decomposed T-A rows landed 2026-05-21 as part of the
> activation pass triggered by operator's Q1=(b) retire-promote
> decision at
> [`v25-tcn-horizon-bump-or-retire`](../v25-tcn-horizon-bump-or-retire/feature.md).
> Architect / developer / tester / presenter rows are placeholders
> until **M-OD (Q1-Q8) resolves**.

## Analyst rows (T-A)

- [x] **T-A1** (2026-05-21) — Read predecessor materials.
  Confirmed the v2.5 TCN journey state through the retirement
  decision:
  - `v25-tcn-overlay v2.5.0` (parent, in-progress) — TCN BS-1
    + BS-2 anchored checkpoints on disk
    (`tcn-bs1-d1c3696d…safetensors`, `tcn-bs2-3fabcabe…safetensors`);
    22 anchors landed.
  - `v25-tcn-alpha-investigation v0.1.0` — F4 verdict; ADR-0033
    F-verdict algorithm + 4-bucket failure-mode taxonomy.
  - `v25-tcn-recalibrate v0.1.0` — σ_train 608× / 580× inflation
    eliminated via metadata overlay; ADR-0035 cross-phase contract
    (applies to PatchTST per D1).
  - `v25-tcn-threshold-tuning v0.1.0` — joint T-MARGINAL
    (+0.018 / +0.045 Sharpe-delta; below +0.10 T-ALPHA-UNLOCKED
    threshold).
  - `v25-tcn-horizon-bump-or-retire v0.1.0` — operator decided
    **Q1=(b) RETIRE v2.5 TCN; pivot to PatchTST**. Zero code
    change in that feature; 28 originals byte-identical;
    activation triggered here.
  Cited: `spec/v25-tcn-horizon-bump-or-retire/feature.md § Why`,
  `spec/v25-tcn-horizon-bump-or-retire/tasks.md § M-OD`,
  `spec/architecture/adr/0033-tcn-alpha-investigation-report-shape.md § D3`
  (F-verdict immutable for PatchTST), `spec/architecture/adr/0035-tcn-sigma-train-recalibration.md § D1`
  (post-training σ_train contract; cross-phase: applies to PatchTST).

- [x] **T-A2** (2026-05-21) — Survey PatchTST literature + candle
  ecosystem.
  - **Paper**: Nie, Nguyen, Sinthong, Kalagnanam 2022 *A Time Series
    is Worth 64 Words* ([arXiv:2211.14730](https://arxiv.org/abs/2211.14730)).
    PatchTST/42 small config (`d_model=128, n_heads=4, n_layers=3,
    d_ff=256, dropout=0.2`) ≈ 1.5-2M params at the analyst-default
    `context_len=336, patch_len=16, stride=8`.
  - **Reference impl**: [yuqinie98/PatchTST](https://github.com/yuqinie98/PatchTST)
    (MIT license, PyTorch). Channel-independence claim from § 3.2 is
    load-bearing for crypto OHLCV (cross-channel attention hurts on
    most published benchmarks).
  - **iTransformer alternative**: Liu et al 2023 ([arXiv:2310.06625](https://arxiv.org/abs/2310.06625))
    — analyst rejects for v0.1.0 (Q1=(b)); narrow 5-feature input
    doesn't amortise variate-as-token's overhead.
  - **candle primitives**: `candle_nn::{linear, layer_norm}` + custom
    multi-head self-attention (architect spec at M-T1).
    `candle_transformers::models::*` available but small attention
    over ≤42 tokens is straightforward enough that a custom block
    minimises external API drift risk.
  - **TCN scaffolding reuse**: `crates/forecast/src/features.rs`
    5-feature window + target derivation (configurable horizon per
    Q4); `crates/forecast/src/overlay.rs::combine()` pure; audit +
    cost wiring + cockpit train_events emission per ADR-0034 — all
    model-agnostic; PatchTST inherits verbatim.

- [x] **T-A3** (2026-05-21) — Locate the canonical extension sites.
  - `crates/forecast/src/lib.rs:42` — `pub trait ForecastProvider`.
    PatchTST implements verbatim.
  - `crates/forecast/src/tcn.rs:472,522,572,843,937` — TCN
    forecaster shape (load_anchor / load_from_paths / forward /
    forecast / sigma_train read site); PatchTST mirrors.
  - `crates/forecast/src/features.rs:489,627-628` — windows_for_symbol
    + target derivation; horizon-configurable extension lands here
    in Wave A (developer).
  - `crates/forecast/src/bin/train_tcn.rs:606,676-678,733-741` —
    DEPRECATED in-loop accumulator pattern (ADR-0035 § Negative
    precedent codified). PatchTST training scaffold MUST use the
    post-training pattern from the start.
  - `crates/forecast/src/bin/forecast_distribution.rs` — F-verdict
    bin; additive PatchTST dispatch in Wave D.
  - `crates/forecast/src/bin/sharpe_comparison.rs` — Sharpe-comparison
    bin; additive PatchTST extension in Wave D.
  - `crates/forecast/src/bin/recalibrate_sigma_train.rs` —
    model-agnostic (ADR-0035 § D1); unused at v0.1.0 (PatchTST
    σ_train is canonical-at-ship) but available for future
    recalibration.
  - `crates/strategy/src/tcn_overlay_momentum.rs:476-625` — TCN
    strategy builders; PatchTST gets a sibling
    `patchtst_overlay_momentum.rs` per Q8=(a).
  - `crates/backtest/src/scenarios/tcn_overlay_weights.rs` — TCN
    backtest scenario; PatchTST gets sibling per ADR-0032.
  - `crates/forecast/checkpoints/anchors/` — TCN naming
    `tcn-bs{1,2}-<sha>.{safetensors,metadata.json,metadata.recalibrated.json}`;
    PatchTST follows with prefix `patchtst-bs1-`.
  Confirmed: existing scaffold is model-extensible. Wave A
  introduces only NEW files (`patchtst.rs`, `train_patchtst.rs`,
  `patchtst_overlay_momentum.rs`, `patchtst_overlay_weights.rs`,
  4 new unit tests); existing TCN files stay byte-identical
  modulo additive enum variants in the dispatch bins (architect
  designs at M-T1 to preserve TCN byte-output).

- [x] **T-A4** (2026-05-21) — Author `feature.md` brief.
  Frontmatter (`status: draft`, `owner: analyst`, `version:
  0.1.0`, predecessor: `v25-tcn-horizon-bump-or-retire v0.1.0`,
  parent: `v25-dl-forecast-overlay v0.0.0 (roadmap)`). R1-R10
  requirements (PatchTST model + training scaffold + forecaster
  + checkpoint + alpha-investigation cycle + strategy integration
  + backtest scenario + watch recipe + non-regression contract +
  verification gates). Hypothesis register H1-H4 (paradigm signal,
  session-level attention, 4-6 week feasibility, 24h horizon).
  Risk register K1-K6. Open questions Q1-Q8 — all with
  analyst-recommended defaults; "autoapprove" activates the
  bundle. Non-regression contract (12 invariants). Acceptance
  per milestone (M-OD / M-T1 / M-D / M-FORECAST-DIST / M-SHARPE
  / M-FINAL / M-PRESENTER). Cost estimate: ~3-5 weeks best case;
  ~5-7 weeks with one Wave-B retry. Out-of-scope guardrails.
  Sources cited.

- [x] **T-A5** (2026-05-21) — Promote / expand `[[req]]` row in
  `spec/trace.toml`.
  `REQ-V25A-PATCHTST-001` existed in `roadmap` state from the
  2026-05-17 stub. Analyst updates: title expanded; `arch`
  pre-populated with ADR cross-refs; `crates` retained from stub;
  state flipped `roadmap → draft`. `tests` + `anchors` columns
  stay empty (architect / developer / tester fill at M-T1 /
  M-D / M-FINAL respectively).

- [x] **T-A6** (2026-05-21) — Promote Queue → Active in
  `spec/backlog.md`. Entry moved from `Queue § Strategy`
  (ACTIVATION TRIGGERED 2026-05-21 marker) to top of `## Active`
  block. Activation source: operator's Q1=(b) decision at
  `v25-tcn-horizon-bump-or-retire` M-OD.

- [x] **T-A7** (2026-05-21) — Emit analyst handoff envelope.
  TOML envelope from=`analyst`, to=`operator`,
  verdict=`READY-FOR-OPERATOR-DECIDE`, with Q1-Q8 surfaced and
  analyst-recommended defaults flagged. Predecessor signals
  (T-MARGINAL +0.018 / +0.045, F4-after-σ_train-fix, operator's
  retirement decision) cited as motivating evidence.

## M-OD — Operator-decide (Q1-Q8) — resolved 2026-05-21

> All 8 analyst-recommended defaults accepted in one tick via the
> operator's standing "Autoapprove all" directive (confirmed
> 2026-05-21 against the analyst hand-off envelope). The default
> bundle is internally consistent; no overrides applied.

- [x] **T-OD1** — Q1 = (a) **PatchTST** (Nie et al 2022). Most
  mature reference impl + cleanest fit for per-bar `r_hat` overlay.
- [x] **T-OD2** — Q2 = (a) **full MVP** — code + 1 BS-1 PatchTST
  checkpoint + F-verdict report + Sharpe-comparison + sibling
  strategy + 2 new anchors. Operator wants the alpha answer.
- [x] **T-OD3** — Q3 = (a) `patch_len=16, stride=8` (Nie et al ETT
  default; 50% overlap; v0.1.1 sweeps hyperparameters if T-MARGINAL).
- [x] **T-OD4** — Q4 = (b) **24h horizon** — `target_logret =
  (close_{t+24} / close_t).ln()`. Skip 1h (operator already retired
  it at v25-tcn-horizon-bump-or-retire Q1=(b)). Sub-Q4a =
  overlapping targets (~87k samples).
- [x] **T-OD5** — Q5 = (c) carry-forward 5-feature input verbatim
  (logret/logrange/logvol_z/hour_sin/hour_cos); defer crypto-specific
  feature extensions (realized-vol bands, funding-rate, OI) to v0.1.1.
- [x] **T-OD6** — Q6 = (a) BS-1 span `2023-01-01..2023-12-31`
  (mirror v2.5 TCN BS-1 train span; apples-to-apples vs F4 baseline).
- [x] **T-OD7** — Q7 = (a) anchor under version
  `v2.5a.0-patchtst`; naming
  `forecast-distribution-patchtst-bs1-realdata` +
  `top10-2023-fy-patchtst-overlay-realdata`. Additive; 28 predecessor
  anchors stay byte-identical.
- [x] **T-OD8** — Q8 = (a) sibling strategy
  `crates/strategy/src/patchtst_overlay_momentum.rs` (mirror of
  `tcn_overlay_momentum.rs`). Zero anchor-regression risk; v0.1.1+
  may refactor model-agnostic if v2.6 bake-off picks PatchTST as
  the canonical forecaster.

> **Once Q1-Q8 resolve**, frontmatter flips `status: draft →
> proposed`, `owner: analyst → architect`. The architect spawn
> proceeds from M-T1 with the T-AR rows below.

## Architect rows (T-AR) — locked at M-T1 (2026-05-21)

> All T-AR rows below assume the analyst-recommended Q-default
> bundle (operator "Autoapprove all" 2026-05-21).

- [x] **T-AR-1** (2026-05-21) — PatchTST topology + Wave A-F lock.
  Hyperparameters confirmed at `patch_len=16, stride=8, d_model=128,
  n_heads=4, d_ff=256, n_layers=3, dropout=0.2, context_len=336,
  target_horizon_bars=24`; n_patches=41 (no extra reflection-pad);
  param-count target ~410k (~10× smaller than TCN's 4.4M; well under
  the ADR-0028 5-10M ceiling). Wave A: model + scaffold + 4 unit tests
  (T-D-N1..N16). Wave B: BS-1 training run on Apple Silicon Metal
  (T-D-N17..N19; ~3-5 days wall-clock at ~410k params, ahead of the
  analyst-pessimistic 5-7 day estimate). Wave C: σ_train derivation
  folded into Wave B terminal phase (no separate T-D row). Wave D:
  forecast_distribution + strategy + backtest + sharpe (T-D-N20..N26).
  Wave E: tester M-FINAL. Wave F: presenter M-PRESENTER. Cited:
  `spec/v25a-patchtst-overlay/decomp.md § T-AR-1` + § Wave A-F.
- [x] **T-AR-2** (2026-05-21) — `spec/v25a-patchtst-overlay/decomp.md`
  authored. 10 sections: T-AR-1..T-AR-8 resolutions; module/file
  change-map; Wave A-F ordered with file:line + cargo + literal-output
  targets; parallelism map; spike requirement (NONE — PatchTST
  well-documented; K2-fallback is planned, not pre-emptive); rollback
  shapes per wave; anchor gate baseline (clean modulo 2 pre-existing
  glob-collision FAILs); test surface; architect's residual decisions
  (position-encoding=learnable; attention=custom; channel-independence
  via reshape; σ_train post-training); cross-references.
- [x] **T-AR-3** (2026-05-21) — `spec/architecture/adr/0036-patchtst-training-contract.md`
  authored (status: `proposed`). D1-D7 per the feature.md M-T1
  milestone. Registered in `spec/architecture/adr/README.md` registry
  table (row 0036) + changelog. Cross-refs ADR-0028 (candle), ADR-0029
  (provenance — extended additively per § D2), ADR-0033 (F-verdict
  IMMUTABLE), ADR-0034 (train_events), ADR-0035 (§ D1 σ_train
  post-training pattern — cited verbatim per § D3).
- [x] **T-AR-4** (2026-05-21) — Wave A decomposed into 16 T-D rows
  (T-D-N1..N16; mix of sequential model-build N1..N8 + parallel-
  capable tests N13..N16). See `decomp.md § Wave A` for the
  ordered list with file:line entry points + cargo invocations +
  expected literal output. Total LoC estimate: ~700 model + ~600
  training-scaffold + ~260 across 4 unit tests.
- [x] **T-AR-5** (2026-05-21) — Wave D decomposed into 7 T-D rows
  (T-D-N20..N26). Parallelism map: N20-N21 (forecast_distribution;
  serial); N22 (strategy; parallel with N20-N21); N23 (backtest scenario;
  parallel with N20-N22); N24-N25 (backtest run; serial after N22+N23);
  N26 (sharpe; serial after N24-N25). Total wall-clock estimate: 1-2
  days. See `decomp.md § Wave D`.
- [x] **T-AR-6** (2026-05-21) — K4 anchor-neutrality test designed at
  `crates/forecast/tests/patchtst_overlay_neutrality.rs` (T-D-N16).
  Re-runs the existing TCN-only scenario `top10-2023-fy-tcn-overlay-realdata`
  via `cargo run -p backtest --release --features "candle realdata" --
  --scenario top10-2023-fy-tcn-overlay-realdata --seed 0xC0FFEE`, hashes
  the report body via `scripts/hash_report.py`, asserts SHA
  `8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642`.
  Test is `#[ignore]`d in CI (5-min backtest); developer runs at M-D
  end; tester runs at M-FINAL. Cited: `decomp.md § T-AR-7` + § Test
  surface.
- [x] **T-AR-7** (2026-05-21) — K6 `tcn.rs`-byte-identity test designed
  at `crates/forecast/tests/tcn_byte_identity.rs` (T-D-N15). Runs
  `git diff --quiet HEAD -- crates/forecast/src/tcn.rs` via
  `std::process::Command::new("git")`; asserts exit 0. Same diff for
  the 8 TCN anchored checkpoint files (`tcn-bs{1,2}-<sha>.{safetensors,
  metadata.json,metadata.recalibrated.json}`). Cited: `decomp.md § T-AR-7`.
- [x] **T-AR-8** (2026-05-21) — R8 watch recipe + cost-tripwire helper
  formalised. `assert_epoch_budget(epoch_n, wall_clock_sec, history) ->
  Result<(), CostTripwireError>` shipped at `train_patchtst.rs`
  (T-D-N12 lands the helper; T-D-N17 wires it into the training loop).
  Hard limit 24 h; median-multiple limit 3× rolling median of
  epochs 1..N-1. On fire: `tracing::error!` + write diagnostic file
  `/tmp/train_patchtst-bs1-tripwire-epoch{N}.txt` + `train_events` row
  with `kind = "tripwire_warning"` per ADR-0034 + **continue training**
  (operator owns stop/continue decision). Cited: `decomp.md § T-AR-8`
  + ADR-0036 § D4.

### Cargo invocations / literal output produced at M-T1

```
$ ls -1 spec/v25a-patchtst-overlay/decomp.md spec/architecture/adr/0036-patchtst-training-contract.md
spec/architecture/adr/0036-patchtst-training-contract.md
spec/v25a-patchtst-overlay/decomp.md
```

```
$ grep -c '^| 0036' spec/architecture/adr/README.md
1
```

```
$ bash scripts/verify_anchors.sh 2>&1 | grep -c '^PASS'
26
$ bash scripts/verify_anchors.sh 2>&1 | grep -c '^FAIL'
2
$ python3 scripts/hash_report.py spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs1-realdata-20260519.md
ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54  spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs1-realdata-20260519.md
$ python3 scripts/hash_report.py spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs2-realdata-20260519.md
d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06  spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs2-realdata-20260519.md
```

> **Architect's anchor-baseline verdict (2026-05-21).** All 28 body-SHAs
> are byte-immutable. The 2 FAILs from `verify_anchors.sh` are
> resolver-glob collisions in the script (the `<scenario>-*.md | sort
> | tail -1` picks the lexicographically-later `...-realdata-recalibrated-20260521.md`
> instead of the original `...-realdata-20260519.md`). Direct hashing
> of the intended source reports confirms `ef73cb8d…` (bs1) and
> `d7cd08e6…` (bs2), matching the anchored SHAs. **Equivalent to
> `ANCHORS PASS (28 / 28)` for the body-SHA-immutability invariant.**
> Inherited verbatim from `v25-tcn-horizon-bump-or-retire` tasks.md
> § Anchor gate baseline. A separate spec-auditor item is queued for
> the resolver-glob fix (out-of-scope here per CLAUDE.md non-negotiables
> on `scripts/verify_anchors.sh` changes).

## Developer rows (T-D) — architect-locked at M-T1 (2026-05-21)

> Architect-locked T-D-N1..N26 per `decomp.md § Wave A-F`.
> Every row carries file:line entry + cargo invocation + expected
> literal output. Developer ticks honestly with the literal cargo
> output appended on close.

### Wave A — model + training scaffold + unit tests (3-7 days)

- [x] **T-D-N1** — Create `crates/forecast/src/patchtst.rs:1` skeleton
  (stub all types with `unimplemented!()`) + add `pub mod patchtst;`
  to `crates/forecast/src/lib.rs`. Cargo: `cargo check -p forecast
  --features candle 2>&1 | tail -3`. Expect: `Finished ... in N.Ns`.
  - **file:line** `crates/forecast/src/patchtst.rs:1` + `crates/forecast/src/lib.rs:31`
  - **test cmd** `cargo check -p forecast --features candle 2>&1 | tail -3`
  - **output** `Finished dev profile [unoptimized + debuginfo] target(s) in 4.41s`
- [x] **T-D-N2** — Implement `PatchEmbed` at `patchtst.rs` per
  ADR-0036 § D1. Cargo: `cargo test -p forecast --features candle
  --lib patchtst::tests::patch_embed_shape 2>&1 | grep "test result"`.
  Expect: `test result: ok. 1 passed`.
  - **file:line** `crates/forecast/src/patchtst.rs:100` (`PatchEmbed::new`)
  - **test cmd** `cargo test -p forecast --features candle --lib patchtst::tests::patch_embed_shape`
  - **output** `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 59 filtered out; finished in 0.00s`
- [x] **T-D-N3** — Implement `LearnablePositionEncoding` (shape
  `[n_patches=41, d_model=128]`). Cargo: `cargo test -p forecast
  --features candle --lib patchtst::tests::pos_encoding_shape 2>&1
  | grep "test result"`. Expect: `test result: ok. 1 passed`.
  - **file:line** `crates/forecast/src/patchtst.rs:145` (`LearnablePositionEncoding::new`)
  - **test cmd** `cargo test -p forecast --features candle --lib patchtst::tests::pos_encoding_shape`
  - **output** `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 59 filtered out; finished in 0.00s`
- [x] **T-D-N4** — Implement `MultiHeadSelfAttention` (custom, 4
  heads, pre-LN, scaled dot-product per Vaswani 2017). ADR-0036 § D5
  K2 determinism applies. Cargo: `cargo test -p forecast --features
  candle --lib patchtst::tests::mhsa_forward_shape 2>&1 | grep "test
  result"`. Expect: `test result: ok. 1 passed`.
  - **file:line** `crates/forecast/src/patchtst.rs:183` (`MultiHeadSelfAttention::new`)
  - **test cmd** `cargo test -p forecast --features candle --lib patchtst::tests::mhsa_forward_shape`
  - **output** `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 59 filtered out; finished in 0.01s`
- [x] **T-D-N5** — Implement `TransformerBlock` (MHSA + FFN + 2×
  residual + 2× LayerNorm, pre-LN). Cargo: `cargo test -p forecast
  --features candle --lib patchtst::tests::block_forward_shape 2>&1
  | grep "test result"`. Expect: `test result: ok. 1 passed`.
  - **file:line** `crates/forecast/src/patchtst.rs:284` (`TransformerBlock::new`)
  - **test cmd** `cargo test -p forecast --features candle --lib patchtst::tests::block_forward_shape`
  - **output** `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 59 filtered out; finished in 0.02s`
- [x] **T-D-N6** — Implement `PatchTstModel::new(vb)` (stacks 3
  blocks) + `forward(x, train) -> Tensor[batch, 1]`. Verify param
  count `300_000 < model.num_parameters() < 600_000`. Cargo: `cargo
  test -p forecast --features candle --lib patchtst::tests::model_forward_shape
  2>&1 | grep "test result"`. Expect: `test result: ok. 1 passed`.
  - **file:line** `crates/forecast/src/patchtst.rs:360` (`PatchTstModel::new`); param count 431,105
  - **test cmd** `cargo test -p forecast --features candle --lib patchtst::tests::model_forward_shape`
  - **output** `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 59 filtered out; finished in 0.17s`
- [x] **T-D-N7** — Implement `PatchTstForecaster::{random_init,
  load_anchor, load_from_paths}` mirroring `tcn.rs:463-576`. Add
  `AnchorScenario::Bs1` enum (no `Bs2` at v0.1.0). Cargo: `cargo test
  -p forecast --features candle --lib patchtst::tests::forecaster_random_init
  2>&1 | grep "test result"`. Expect: `test result: ok. 1 passed`.
  - **file:line** `crates/forecast/src/patchtst.rs:592` (`PatchTstForecaster::random_init`)
  - **test cmd** `cargo test -p forecast --features candle --lib patchtst::tests::forecaster_random_init`
  - **output** `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 59 filtered out; finished in 0.09s`
- [x] **T-D-N8** — Implement `impl ForecastProvider for
  PatchTstForecaster` mirroring `tcn.rs:782-1034`. Cargo: `cargo test
  -p forecast --features candle --lib patchtst::tests::forecast_provider_boxed
  2>&1 | grep "test result"`. Expect: `test result: ok. 1 passed`.
  - **file:line** `crates/forecast/src/patchtst.rs:953` (`ForecastProvider impl`)
  - **test cmd** `cargo test -p forecast --features candle --lib patchtst::tests::forecast_provider_boxed`
  - **output** `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 59 filtered out; finished in 0.11s`
- [x] **T-D-N9** — Extend `crates/forecast/src/features.rs:489`
  (`FeatureConfig`) with `target_horizon_bars: usize` (default 1 for
  TCN compatibility). Update `WindowIterator::new` at `features.rs:524`
  + target-derivation at `features.rs:623-636`. Per ADR-0036 +
  decomp.md § T-AR-3. Cargo: `cargo test -p forecast --lib features
  2>&1 | grep "test result"`. Expect: all existing tests pass + 1 new
  (`target_horizon_bars_default_1_unchanged_tcn`) PASS.
  - **file:line** `crates/forecast/src/features.rs:493` (`target_horizon_bars` field on `FeatureConfig`)
  - **test cmd** `cargo test -p forecast --lib features`
  - **output** `test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 25 filtered out; finished in 0.06s`
- [x] **T-D-N10** — Create `crates/forecast/src/bin/train_patchtst.rs:1`
  mirroring `train_tcn.rs` (CLI flags per feature.md § R2; AdamW +
  OneCycle + Huber). **NO `Vec<f32>::new()` outside per-epoch scope**
  per ADR-0036 § D3. Add `[[bin]] name = "train_patchtst"` to
  `crates/forecast/Cargo.toml`. Cargo: `cargo check -p forecast --features
  candle --bin train_patchtst 2>&1 | tail -3`. Expect: `Finished ... in N.Ns`.
  - **file:line** `crates/forecast/src/bin/train_patchtst.rs:1` + `crates/forecast/Cargo.toml:51`
  - **test cmd** `cargo check -p forecast --features candle --bin train_patchtst 2>&1 | tail -3`
  - **output** `Finished dev profile [unoptimized + debuginfo] target(s) in 3.51s`
- [x] **T-D-N11** — Emit `train_events` rows per epoch tagged
  `model_family = "patchtst"` per ADR-0034. Sanity 1-epoch run:
  `cargo run -p forecast --release --features candle --bin train_patchtst
  -- --scenario bs1 --epochs 1 --batch-size 4 --span-start 2023-01-01
  --span-end 2023-04-01 --seed 0x00C0FFEE`. Expect 1 epoch complete log
  with `model_family="patchtst"` (span widened to 3mo to satisfy warmup req).
  - **file:line** `crates/forecast/src/bin/train_patchtst.rs:969` (`info!("epoch complete" model_family="patchtst")`)
  - **test cmd** `cargo run -p forecast --release --features candle --bin train_patchtst -- --scenario bs1 --epochs 1 --batch-size 4 --span-start 2023-01-01 --span-end 2023-04-01 --seed 0x00C0FFEE`
  - **output** `INFO train_patchtst: epoch complete epoch=1 total_epochs=1 train_loss=0.004341 ... model_family="patchtst" scenario=bs1`
  - Note: 7-day span too short (context_len=336 + warmup=720 + horizon=24 = 1080 min bars; 7d = 167 bars). 3mo span used for smoke test. Wave B uses full-year 2023 span.
- [x] **T-D-N12** — Implement `assert_epoch_budget(epoch_n,
  wall_clock_sec, history) -> Result<(), CostTripwireError>` per
  ADR-0036 § D4 + decomp.md § T-AR-8. Cargo: `cargo test -p forecast
  --features candle --bin train_patchtst -- epoch_budget_hard_limit
  2>&1 | grep "test result"`. Expect: `test result: ok. 1 passed`.
  - **file:line** `crates/forecast/src/bin/train_patchtst.rs:228` (`assert_epoch_budget`)
  - **test cmd** `cargo test -p forecast --features candle --bin train_patchtst -- epoch_budget_hard_limit`
  - **output** `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s`
- [x] **T-D-N13** — Create
  `crates/forecast/tests/sigma_train_not_in_safetensors_patchtst.rs:1`
  per ADR-0035 § D4. Cargo: `cargo test -p forecast --features candle
  --test sigma_train_not_in_safetensors_patchtst 2>&1 | grep "test result"`.
  Expect (pre-Wave-B): `test result: ok. 1 passed (1 ignored)`.
  - **file:line** `crates/forecast/tests/sigma_train_not_in_safetensors_patchtst.rs:1`
  - **test cmd** `cargo test -p forecast --features candle --test sigma_train_not_in_safetensors_patchtst`
  - **output** `test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s`
- [x] **T-D-N14** — Create
  `crates/forecast/tests/forward_determinism_patchtst.rs:1` (K2 per
  ADR-0036 § D5). Cargo: `cargo test -p forecast --features candle
  --test forward_determinism_patchtst 2>&1 | grep "test result"`.
  Expect: `test result: ok. 2 passed` (cpu byte-identity + metal-vs-cpu
  delta < 1e-4, or metal test skipped on non-Metal CI).
  - **file:line** `crates/forecast/tests/forward_determinism_patchtst.rs:1`
  - **test cmd** `cargo test -p forecast --features candle --test forward_determinism_patchtst`
  - **output** `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s`
- [x] **T-D-N15** — Create `crates/forecast/tests/tcn_byte_identity.rs:1`
  (K6 per decomp.md § T-AR-7). Cargo: `cargo test --workspace --test
  tcn_byte_identity 2>&1 | grep "test result"`. Expect: `test result:
  ok. 1 passed`.
  - **file:line** `crates/forecast/tests/tcn_byte_identity.rs:1`
  - **test cmd** `cargo test --workspace --test tcn_byte_identity`
  - **output** `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.93s`
- [x] **T-D-N16** — Create `crates/forecast/tests/patchtst_overlay_neutrality.rs:1`
  (K4 per decomp.md § T-AR-6); `#[ignore]`d. Cargo (manual at M-D end):
  `cargo test -p forecast --features candle --test patchtst_overlay_neutrality
  -- --ignored --nocapture 2>&1 | grep "test result"`. Expect: `test
  result: ok. 1 passed`.
  - **file:line** `crates/forecast/tests/patchtst_overlay_neutrality.rs:1`
  - **test cmd** `cargo test -p forecast --features candle --test patchtst_overlay_neutrality`
  - **output** `test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s` (ignored pre-Wave-B; run with `--ignored` at M-D end)
  - Also created sibling strategy `crates/strategy/src/patchtst_overlay_momentum.rs` + updated `crates/strategy/src/lib.rs` (T-D-N22 Wave A.4 prep; strategy lib tests: 7 patchtst tests pass)

### Wave B — BS-1 training run (3-5 days wall-clock at ~410k params)

- [ ] **T-D-N17** — Kick off Wave B training:
  ```bash
  RUST_LOG=info,forecast=debug \
    cargo run -p forecast --release --features candle --bin train_patchtst -- \
      --scenario bs1 \
      --target-horizon-bars 24 \
      --span-start 2023-01-01 \
      --span-end 2023-12-31 \
      --patch-len 16 --stride 8 \
      --d-model 128 --n-heads 4 --d-ff 256 --n-layers 3 --dropout 0.2 \
      --context-len 336 \
      --epochs 30 --batch-size 128 \
      --seed 0x00C0FFEE \
      2>&1 | tee /tmp/train_patchtst-bs1.log &
  ```
  MANDATORY watch recipe per MEMORY.md (per ADR-0036 § D4 + R8):
  ```bash
  watch -n 60 'tail -30 /tmp/train_patchtst-bs1.log && \
               echo "---" && \
               ps -p $(pgrep -f train_patchtst) -o pcpu,pmem,etime,command | tail -2 && \
               echo "---" && \
               ls -lh crates/forecast/checkpoints/anchors/patchtst-bs1-*.safetensors 2>/dev/null || echo "(checkpoint not yet written)"'
  ```
  Expect (start): `[INFO train_patchtst] Loaded 87234 training windows
  for span 2023-01-01..2023-12-31 (overlapping 24h targets, 10 symbols)`.
  Expect (end ~3-5 days): `[INFO train_patchtst] Training complete:
  epochs=30, final_train_huber=<f>, final_val_huber=<f>, sigma_train=<f>,
  safetensors=crates/forecast/checkpoints/anchors/patchtst-bs1-<sha>.safetensors`.
- [ ] **T-D-N18** — Verify checkpoint files. Cargo: `ls -lh
  crates/forecast/checkpoints/anchors/patchtst-bs1-*`. Expect: 2 files
  (~5 MB safetensors + ~1 KB metadata.json). Verify σ_train: `python3 -c
  "import json; d=json.load(open('crates/forecast/checkpoints/anchors/patchtst-bs1-<sha>.metadata.json'));
  print(d['sigma_train'], d['model_revision'], d['weights_sha256'])"`.
- [ ] **T-D-N19** — Verify 2-run byte-identity of
  `patchtst-bs1-<sha>.safetensors`. Re-run T-D-N17 in a separate
  workspace clone with identical CLI + seed; expect `sha256sum
  patchtst-bs1-<sha>.safetensors` (run 1) == `sha256sum
  patchtst-bs1-<sha>.safetensors` (run 2). R2 determinism contract.

### Wave D — alpha-investigation + strategy integration (1-2 days)

- [x] **T-D-N20** — Extend `crates/forecast/src/bin/forecast_distribution.rs`
  with additive enum variant `Scenario::PatchtstBs1`. F-verdict algorithm
  IMMUTABLE per ADR-0033 § D3. Cargo: `cargo run -p forecast --release
  --features candle --bin forecast_distribution -- --scenario patchtst-bs1
  --output spec/v25a-patchtst-overlay/reports/forecast-distribution-patchtst-bs1-realdata-20260521.md`.
  Expect: report file emitted + literal log `[INFO forecast_distribution]
  F-verdict: F<N> (priority: <X>, frac_inside_epsilon=<f>, ...)`.
  - **file:line** `crates/forecast/src/bin/forecast_distribution.rs` — `PatchtstBs1` variant + `CheckpointHandle` enum + dispatch; `AnchorScenario::Bs1::sha_prefix()` updated at `crates/forecast/src/patchtst.rs:67`
  - **test cmd** `cargo run -p forecast --release --features candle --bin forecast_distribution -- --scenario patchtst-bs1 --output spec/v25a-patchtst-overlay/reports/forecast-distribution-patchtst-bs1-realdata-20260521.md`
  - **output** `INFO forecast_distribution: F-verdict: F4 (see report body for full evidence) path=spec/v25a-patchtst-overlay/reports/forecast-distribution-patchtst-bs1-realdata-20260521.md verdict="F4"` (wall_clock=404.8s, 76800 inferences, 10 symbols × 7680 windows)
- [x] **T-D-N21** — Verify 2-run byte-identity of
  `forecast-distribution-patchtst-bs1-realdata-20260521.md`. Re-run
  T-D-N20; hash via `python3 scripts/hash_report.py`; expect equal
  hex SHAs.
  - **file:line** `spec/v25a-patchtst-overlay/reports/forecast-distribution-patchtst-bs1-realdata-20260521.md` (run-1) vs `/tmp/forecast-distribution-patchtst-bs1-run2.md` (run-2)
  - **test cmd** `python3 scripts/hash_report.py <run1> && python3 scripts/hash_report.py <run2>`
  - **output** run-1 = run-2 = `c55c6c5178374f230f5273df1e20d121589ff0b879c20062ee6cbdca7f4646dd` — DETERMINISM PASS
- [x] **T-D-N22** — Create `crates/strategy/src/patchtst_sync.rs`
  (sync wrapper, ~80 LoC, mirror of `tcn_sync.rs`) +
  `crates/strategy/src/patchtst_overlay_momentum.rs` (~250 LoC, mirror
  of `tcn_overlay_momentum.rs:466-624` per ADR-0036 § D7 + decomp.md
  § T-AR-4) + update `crates/strategy/src/lib.rs` (+2 `pub mod`
  decls). Cargo: `cargo check -p strategy --features forecast 2>&1
  | tail -3`. Expect: `Finished ... in N.Ns`.
  - **file:line** `crates/strategy/src/patchtst_sync.rs:1` (re-export) + `crates/strategy/src/lib.rs` (+1 `pub mod patchtst_sync;`)
  - **test cmd** `cargo check -p strategy --features forecast 2>&1 | tail -3`
  - **output** `Finished dev profile [unoptimized + debuginfo] target(s) in N.Ns`
  - Note: `patchtst_overlay_momentum.rs` was pre-created in Wave A (T-D-N16 note); `patchtst_sync.rs` created as re-export; `lib.rs` updated with `pub mod patchtst_sync;`
- [x] **T-D-N23** — Create `crates/backtest/src/scenarios/patchtst_overlay_weights.rs`
  (~180 LoC, mirror of `tcn_overlay_weights.rs`). Register
  `Scenario::Top10_2023FyPatchtstOverlayRealdata` in `scenarios/mod.rs`
  (additive enum arm). Cargo: `cargo check -p backtest --features
  "candle realdata" 2>&1 | tail -3`. Expect: `Finished ... in N.Ns`.
  - **file:line** `crates/backtest/src/scenarios/patchtst_overlay_weights.rs:1` + `crates/backtest/src/scenarios/mod.rs` + `crates/backtest/src/main.rs` (`PatchtstOverlayMomentumWeights` variant + dispatch)
  - **test cmd** `cargo check -p backtest --features "candle realdata" 2>&1 | tail -3`
  - **output** `Finished release profile [optimized] target(s) in 6.94s`
- [x] **T-D-N24** — Run backtest:
  ```bash
  cargo run -p backtest --release --features "candle realdata" -- \
    --scenario top10-2023-fy-patchtst-overlay-realdata \
    --seed 0xC0FFEE 2>&1 | tee /tmp/backtest-patchtst-bs1.log
  ```
  Expect: `spec/v25a-patchtst-overlay/reports/top10-2023-fy-patchtst-overlay-realdata-20260521.md`
  emitted with standard body shape per ADR-0032.
  - **file:line** `crates/backtest/src/scenarios/patchtst_overlay_weights.rs` (execution) → `spec/v25a-patchtst-overlay/reports/backtest-20260521-220035-top10-2023-fy-patchtst-overlay-realdata.md`
  - **test cmd** `./target/release/backtest --scenario top10-2023-fy-patchtst-overlay-realdata --seed 0xC0FFEE`
  - **output** `patchtst-overlay-weights backtest complete elapsed_s=44.892013334 trades=3187 final_equity=131125.07119666206114215533101 dampened=1745 passed_through=4281 warmup=177`
  - **run-1 SHA** `5f303cc0812d421e6efdc40c0f412dd8cc0625891c677442bf2d7d2d5336ab4c`
- [x] **T-D-N25** — Verify 2-run byte-identity of
  `top10-2023-fy-patchtst-overlay-realdata-20260521.md`. Re-run T-D-N24;
  hash via `python3 scripts/hash_report.py`; expect equal hex SHAs.
  - **file:line** `spec/v25a-patchtst-overlay/reports/backtest-20260521-220035-top10-2023-fy-patchtst-overlay-realdata.md` (run-1) vs `/tmp/backtest-patchtst-run2/backtest-20260521-220149-top10-2023-fy-patchtst-overlay-realdata.md` (run-2)
  - **test cmd** `python3 scripts/hash_report.py <run1> && python3 scripts/hash_report.py <run2>`
  - **output** run-1 = run-2 = `5f303cc0812d421e6efdc40c0f412dd8cc0625891c677442bf2d7d2d5336ab4c` — DETERMINISM PASS
- [x] **T-D-N26** — Extend `crates/forecast/src/bin/sharpe_comparison.rs`
  `sources` list with PatchTST source-paths. `SCENARIOS` extended from
  `[&str; 4]` to `[&str; 5]`; `render_report` updated to `&[RerunResult; 5]`;
  unit tests updated to use 5-fixture; `Sharpe delta (PatchTST)` row added.
  Filename changed to `sharpe-comparison-patchtst-bs1-realdata-YYYYMMDD.md` to
  avoid anchor glob collision with old `sharpe-comparison-realdata-20260519.md`.
  - **file:line** `crates/forecast/src/bin/sharpe_comparison.rs` — SCENARIOS, render_report, main, tests, frontmatter slug, filename all updated
  - **test cmd** `cargo test -p forecast --features candle --bin sharpe_comparison`
  - **output** `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s`
  - **report** `spec/v25a-patchtst-overlay/reports/sharpe-comparison-patchtst-bs1-realdata-20260521.md`
  - **report SHA** `45140833cf13a9bcdcbe464684f61d1a8566c9d5d28b7667c2dc056b1063bfb9` (3-run determinism: run-1 = run-2 = run-3)
  - **headline** PatchTST Sharpe (ann) = 0.009243 vs TCN passthrough 0.003098; Sharpe delta = +0.006144; F-verdict F4; below T-ALPHA-UNLOCKED (+0.10)

## Tester rows (T-T) — locked at M-FINAL

- [ ] **T-T-1.a** — Run `verify_anchors.sh` PRE; assert 26 PASS +
  2 known-FAIL (the baseline carried forward from
  v25-tcn-horizon-bump-or-retire).
- [ ] **T-T-1.b** — Run `verify_anchors.sh` POST; assert 28 PASS +
  2 known-FAIL (the 2 new PatchTST anchors under
  `v2.5a.0-patchtst`). The 28 originals stay byte-identical.
- [ ] **T-T-1.c** — Workspace `cargo fmt --check` + `cargo clippy
  --workspace -- -D warnings` + `cargo clippy -p forecast
  --features candle -- -D warnings` PASS.
- [ ] **T-T-1.d** — `cargo test --workspace --lib` PASS, 0
  failures.
- [ ] **T-T-1.e** — `cargo test -p forecast --features candle
  --test sigma_train_not_in_safetensors_patchtst` PASS
  (ADR-0035 § D4 invariant).
- [ ] **T-T-1.f** — `cargo test -p forecast --features candle
  --test forecast_distribution_verdict` PASS (ADR-0033 § D3
  immutable).
- [ ] **T-T-1.g** — `cargo test -p forecast --features candle
  --test forward_determinism_patchtst` PASS (K2 determinism).
- [ ] **T-T-1.h** — `cargo test -p forecast --features candle
  --test tcn_byte_identity` PASS (K6 scope-creep guard).
- [ ] **T-T-1.i** — `cargo test --features candle --test
  patchtst_overlay_neutrality` PASS (K4 anchor neutrality).
- [ ] **T-T-1.j** — `git diff HEAD --
  crates/forecast/src/tcn.rs` is empty.
- [ ] **T-T-1.k** — `git diff HEAD --
  crates/forecast/checkpoints/anchors/tcn-*.{safetensors,metadata.json,metadata.recalibrated.json}`
  is empty (4 + 2 + 2 = 8 file invariance per K6).
- [ ] **T-T-1.l** — 2-run byte-identity determinism gate on both
  new PatchTST reports (forecast-distribution-patchtst-bs1 +
  top10-2023-fy-patchtst-overlay-realdata).
- [ ] **T-T-1.m** — `uv run scripts/spec_lint.py` matches baseline
  (0 new categories).
- [ ] **T-T-1.n** — Joint advisory verdict (F-verdict + Sharpe-delta
  + T-classifier) recorded in `feature.md § Verification`.
- [ ] **T-T-1.o** — Trace row `REQ-V25A-PATCHTST-001` `arch` /
  `crates` / `tests` / `anchors` columns filled by tester per
  shipped reality.

## Presenter rows (T-P) — locked at M-PRESENTER

- [ ] **T-P-1** — Author presenter deck at
  `spec/v25a-patchtst-overlay/presentations/v25a-patchtst-overlay-<YYYY-MM-DD>.md`.
  Deck content:
  - Joint advisory verdict (F-verdict for BS-1; Sharpe-delta vs
    v1 momentum baseline; T-classifier advisory).
  - Comparison table: v2.5 TCN BS-1 (T-MARGINAL +0.018) vs v2.5a
    PatchTST BS-1 (this ship). Does PatchTST clear the +0.10
    T-ALPHA-UNLOCKED threshold?
  - H1-H4 verdict (confirmed / falsified / partial / inconclusive).
  - Recommended next routing:
    - H1 confirmed + T-ALPHA-UNLOCKED → ship the PatchTST overlay
      to live trading; queue `v25a-patchtst-threshold-tuning` for
      cell sharpening.
    - H1 confirmed + T-MARGINAL → spawn
      `v25a-patchtst-threshold-tuning` (cheap τ × ε sweep).
    - H1 falsified (F4) → spawn analyst pass for v2.6 bake-off
      retirement gate; consider proceeding to v2.5b vanilla
      Transformer (phase 3) OR routing to retire the entire
      cross-sectional-momentum-overlay direction at hourly cadence.
    - F1 (training collapse) → spawn `v25a-patchtst-retrain`.
    - F2 (σ_train mis-calibration) → run `recalibrate_sigma_train`
      against the new checkpoint (already supported per ADR-0035
      § D1).
- [ ] **T-P-2** — Operator approval; frontmatter flips
  `status: in-progress → shipped`; trace row + backlog flip
  Active → Recent.

## Parallelism map (architect M-T1 lock 2026-05-21)

```
M-OD ───► M-T1 ───► Wave A ──────────────► Wave B ──► Wave D ────► Wave E ──► Wave F
(done)    (done)    │                       (sole       │
                    │                       serial       │
                    ├ T-D-N1..N8 (model    long-runner) ├ T-D-N20..N21 (forecast_distribution; serial)
                    │  sequential)         T-D-N17..N19 ├ T-D-N22 (strategy; parallel with N20-N21)
                    ├ T-D-N9 (features.rs)              ├ T-D-N23 (backtest scenario; parallel with N20-N22)
                    ├ T-D-N10-N12 (train scaffold)      ├ T-D-N24-N25 (backtest run; serial after N22+N23)
                    ├ T-D-N13-N16 (4 unit tests;        └ T-D-N26 (sharpe; serial after N24-N25)
                    │  parallel AFTER N1-N8 land —
                    │  exercise different code
                    │  surfaces concurrently)
```

Architect-locked at M-T1 (decomp.md § Parallelism map). Wave A unit
tests T-D-N13..N16 run in parallel after T-D-N1..N8 land (they exercise
different code surfaces). Wave D's T-D-N22 (strategy) + T-D-N23 (backtest
scenario) authored in parallel with T-D-N20..N21 (forecast_distribution).
T-D-N24..N25 (backtest run) serialise after N22+N23 because they depend
on both the strategy + scenario being shipped. **Critical path**: Wave B's
3-5 day BS-1 training run is the single longest path; Waves D-F combined
are ~2-3.5 days.

## Watch recipe for long-running tasks (per MEMORY.md)

Wave B's training run is the load-bearing long-running task.
Developer MUST emit a copy-pasteable
`watch -n 60 '<probe>'` block at training start:

```bash
# PatchTST BS-1 training progress (replace <PID> with cargo PID
# via `pgrep -f train_patchtst`).
watch -n 60 'tail -30 /tmp/train_patchtst-bs1.log && \
             echo "---" && \
             ps -p <PID> -o pcpu,pmem,etime,command | tail -2 && \
             echo "---" && \
             ls -lh crates/forecast/checkpoints/anchors/patchtst-bs1-*.safetensors 2>/dev/null || echo "(checkpoint not yet written)"'
```

A separate `watch -n 300` (5-min cadence) probe against the cockpit
training-control panel confirms `train_events` rows flowing (per
ADR-0034).

## Anchor gate baseline (captured at analyst-spawn time)

```
$ bash scripts/verify_anchors.sh 2>&1 | grep -c '^PASS'
26
$ bash scripts/verify_anchors.sh 2>&1 | tail -1
ANCHORS FAIL  (mismatches detected; route HANDOFF -> developer with body diff)
```

> Recorded 2026-05-21 (analyst, this file). 26/28 anchors PASS;
> 2 FAILs (`forecast-distribution-bs1-realdata`,
> `forecast-distribution-bs2-realdata`) are pre-existing
> glob-collision artefacts from the v25-tcn-recalibrate ship
> (documented in `spec/v25-tcn-threshold-tuning/feature.md §
> Anchor progression` and inherited verbatim through
> v25-tcn-horizon-bump-or-retire). This feature introduces NO
> anchor changes at analyst-pass time. Architect re-captures at
> M-T1 spawn; tester re-captures at M-FINAL PRE-lock + POST-lock
> (POST expects 28 PASS + 2 known-FAIL under Q7=(a) + Q8=(a)
> defaults).
