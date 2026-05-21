---
slug: v25a-patchtst-overlay
status: proposed
owner: architect
updated: 2026-05-21
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

## Architect rows (T-AR) — locked at M-T1

> All T-AR rows below assume the analyst-recommended Q-default
> bundle. If operator overrides Q1 = (b) iTransformer or Q2 = (b)
> code only, the architect re-decomposes accordingly.

- [ ] **T-AR-1** — Lock § Design block in `feature.md`. Wave A
  (model + training scaffold + 4 unit tests). Wave B (training run;
  orchestrator-monitored). Wave C (σ_train derivation folded into
  Wave B). Wave D (forecast_distribution + sharpe_comparison +
  backtest scenario + strategy integration). Wave E (tester gate).
  Wave F (presenter deck).
- [ ] **T-AR-2** — Author `spec/v25a-patchtst-overlay/decomp.md`
  with T-D / T-T row decomposition into Waves A-D.
- [ ] **T-AR-3** — Author **ADR-0036** at
  `spec/architecture/adr/0036-patchtst-training-contract.md`.
  Codifies:
  - **D1** — PatchTST architecture skeleton (patch embed +
    positional embedding + pre-LN transformer encoder +
    projection head to scalar `r_hat`). Hyperparameter defaults
    per R1 (PatchTST/42 small config). Channel-independence per
    Nie et al § 3.2.
  - **D2** — Canonical-arch descriptor extension (ADR-0029-compat).
    PatchTST-specific fields enter the SHA: `patch_len, stride,
    d_model, n_heads, d_ff, n_layers, dropout, context_len,
    target_horizon_bars`. Existing TCN checkpoints' SHAs unchanged.
  - **D3** — σ_train post-training derivation (reference ADR-0035
    § D1 verbatim). No in-loop accumulator. Architect adds a
    code-review check: `train_patchtst.rs` contains zero
    `Vec<f32>::new()` declarations outside the inner-most epoch
    loop scope.
  - **D4** — Cost tripwire (single epoch > 24h wall-clock OR
    epoch N > 3× rolling median of epochs 1..N-1 → escalate).
  - **D5** — K2 candle-attention determinism gate (CPU + Metal
    byte-identical forward pass on a fixed-seed input).
  - **D6** — Anchor strategy: `v2.5a.0-patchtst` version pin;
    `forecast-distribution-patchtst-bs1-realdata` +
    `top10-2023-fy-patchtst-overlay-realdata` (Q7=(a) + Q8=(a)
    defaults).
  - **D7** — Strategy-integration shape (sibling
    `patchtst_overlay_momentum.rs`; not a refactor of
    `tcn_overlay_momentum.rs`).
- [ ] **T-AR-4** — Decompose Wave A into T-D rows
  (developer-callable). Estimated 8-12 T-D rows: PatchTST
  model + patch embed + position embed + attention block +
  encoder + projection head + 4 unit tests + training scaffold +
  audit emission.
- [ ] **T-AR-5** — Decompose Wave D into T-D rows. Estimated
  6-8 T-D rows: forecast_distribution PatchTST dispatch +
  sharpe_comparison PatchTST extension + backtest scenario file +
  strategy builders + 2 report-determinism tests.
- [ ] **T-AR-6** — K4 anchor-neutrality unit test designed.
  CI gate at M-FINAL runs `verify_anchors.sh` PRE and POST and
  asserts 28 originals byte-identical.
- [ ] **T-AR-7** — K6 tcn.rs-byte-identity unit test designed.
  CI gate at M-FINAL asserts `git diff HEAD --
  crates/forecast/src/tcn.rs` is empty.
- [ ] **T-AR-8** — R8 watch recipe template + R8 cost-tripwire
  invariant formalised. Developer-callable `assert_epoch_budget`
  helper specified.

## Developer rows (T-D) — placeholders for Wave A

> Architect locks the final T-D row set at M-T1; the placeholders
> below sketch the analyst's expected decomposition.

### Wave A — model + training scaffold + unit tests (3-7 days)

- [ ] **T-D-N1** — Create `crates/forecast/src/patchtst.rs` skeleton:
  `PatchTstModel` struct (patches, embedding, attention, encoder,
  projection); `PatchTstForecaster` struct with `load_anchor` /
  `load_from_paths` mirroring TCN.
- [ ] **T-D-N2** — Implement patch embedding + positional encoding.
  Input `[batch, channels, time]` → patches
  `[batch, channels, n_patches, patch_len]` → linear projection
  `[batch, channels, n_patches, d_model]` + learnable position
  encoding.
- [ ] **T-D-N3** — Implement multi-head self-attention block (custom
  or via `candle_transformers::*`). Pre-LN ordering. Heads=4,
  d_model=128.
- [ ] **T-D-N4** — Implement transformer encoder (3 layers of MHSA
  + feed-forward + residual + pre-LN).
- [ ] **T-D-N5** — Implement projection head (flatten encoder
  output, linear → scalar `r_hat`).
- [ ] **T-D-N6** — Implement `impl ForecastProvider for
  PatchTstForecaster` mirroring TCN's body (forward → r_hat →
  direction quantisation via ε deadband → confidence via
  |r_hat|/σ_train clamp).
- [ ] **T-D-N7** — Create `crates/forecast/src/bin/train_patchtst.rs`.
  CLI flags per R2. AdamW + OneCycle + Huber. NO in-loop
  σ_train accumulator (ADR-0035 § D1). Post-training σ_train
  frozen-forward-pass derivation at training-complete writes
  `<file_prefix>-<sha>.metadata.json` per ADR-0029.
- [ ] **T-D-N8** — `train_events` row emission per epoch per
  ADR-0034 (existing audit infrastructure; only `model_family`
  field changes to `"patchtst"`).
- [ ] **T-D-N9** — Unit test:
  `crates/forecast/tests/sigma_train_not_in_safetensors_patchtst.rs`
  per ADR-0035 § D4.
- [ ] **T-D-N10** — Unit test:
  `crates/forecast/tests/forward_determinism_patchtst.rs` — CPU +
  Metal byte-identical forward pass on fixed-seed input (K2
  determinism gate).
- [ ] **T-D-N11** — Unit test:
  `crates/forecast/tests/tcn_byte_identity.rs` — asserts
  `git diff HEAD -- crates/forecast/src/tcn.rs` empty + the
  TCN BS-1 / BS-2 anchored checkpoint files byte-identical (K6
  scope-creep guard).
- [ ] **T-D-N12** — Unit test:
  `crates/forecast/tests/patchtst_overlay_neutrality.rs` — asserts
  the TCN-only `top10-2023-fy-tcn-overlay-realdata` scenario's
  body bytes match the anchored SHA (K4 anchor neutrality).

### Wave B — BS-1 training run (5-7 days wall-clock)

- [ ] **T-D-N13** — Run training (LONG-RUNNING). Developer MUST
  emit watch recipe per R8 / MEMORY.md:
  ```bash
  watch -n 60 'tail -30 /tmp/train_patchtst-bs1.log && \
               echo "---" && \
               ps -p <PID> -o pcpu,pmem,etime,command | tail -2 && \
               echo "---" && \
               ls -lh crates/forecast/checkpoints/anchors/patchtst-bs1-*.safetensors 2>/dev/null || echo "(checkpoint not yet written)"'
  ```
  Developer monitors via the cockpit training-control panel
  (ADR-0034). Cost-tripwire per T-AR-8 fires if epoch > 24h OR
  epoch N > 3× rolling median.
- [ ] **T-D-N14** — On training-complete: emit
  `patchtst-bs1-<sha>.safetensors` + `.metadata.json` under
  `crates/forecast/checkpoints/anchors/`. σ_train scalar
  derived via the post-training frozen forward-pass over the
  training-data span (ADR-0035 § D1).
- [ ] **T-D-N15** — Verify 2-run byte-identity of
  `patchtst-bs1-<sha>.safetensors` given identical CLI args +
  seed (R2 determinism contract).

### Wave D — alpha-investigation + strategy integration (1-2 days)

- [ ] **T-D-N16** — Extend
  `crates/forecast/src/bin/forecast_distribution.rs` with additive
  PatchTST dispatch (enum variant `PatchtstBs1` OR string
  dispatch — architect M-T1 decides). Architect-confirm: default
  invocation `--scenario bs1` produces byte-identical body to
  the anchored `forecast-distribution-bs1-realdata` report.
- [ ] **T-D-N17** — Run
  `forecast_distribution --scenario patchtst-bs1` against the
  new checkpoint; emit
  `spec/v25a-patchtst-overlay/reports/forecast-distribution-patchtst-bs1-realdata-<date>.md`.
  F-verdict per the immutable ADR-0033 § D3 algorithm recorded
  in body.
- [ ] **T-D-N18** — Create
  `crates/strategy/src/patchtst_overlay_momentum.rs` with
  `with_patchtst_bs1(base) → Result<Self, …>`,
  `with_patchtst_bs1_ledger(base, ledger) → …` (behind
  `feature = "forecast-audit-tick"`),
  `with_patchtst_bs1_tuned(base, tau, epsilon)`,
  `with_patchtst_bs1_ledger_tuned(...)`. Sibling pattern to
  `tcn_overlay_momentum.rs`; ZERO touch on the TCN file.
- [ ] **T-D-N19** — Create
  `crates/backtest/src/scenarios/patchtst_overlay_weights.rs`
  mirroring `tcn_overlay_weights.rs`. Per ADR-0032 realdata path.
- [ ] **T-D-N20** — Run `backtest --scenario
  top10-2023-fy-patchtst-overlay-realdata`; emit
  `spec/v25a-patchtst-overlay/reports/top10-2023-fy-patchtst-overlay-realdata-<date>.md`.
- [ ] **T-D-N21** — Extend
  `crates/forecast/src/bin/sharpe_comparison.rs` with additive
  PatchTST source-paths in its frontmatter `sources` list. Run
  + emit
  `spec/v25a-patchtst-overlay/reports/sharpe-comparison-patchtst-bs1-realdata-<date>.md`.
- [ ] **T-D-N22** — 2-run byte-identity gate on both new reports
  (M-FORECAST-DIST + M-SHARPE outputs); developer locks the
  anchor SHAs at this step.

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

## Parallelism map for the orchestrator

```
M-OD (Q1-Q8) ── [operator] ────► M-T1 (architect) ────► M-D
                                                          │
                                                          ├─ Wave A — model + scaffold + tests (3-7 days)
                                                          │   └─ T-D-N1..N12 (sequential within wave;
                                                          │       N9-N12 unit tests parallel after N1-N8)
                                                          │
                                                          ├─ Wave B — BS-1 training run (5-7 days; serial)
                                                          │   └─ T-D-N13..N15
                                                          │
                                                          └─ Wave D — alpha-investigation + strategy
                                                              ├─ T-D-N16..N17 (forecast_distribution; serial)
                                                              ├─ T-D-N18..N19 (strategy + scenario; parallel
                                                              │   with N16-N17 — different files)
                                                              ├─ T-D-N20 (backtest; sequential after N18-N19)
                                                              └─ T-D-N21..N22 (sharpe + determinism; sequential)
                                                          ▼
                                                       M-FINAL (tester)
                                                          ▼
                                                       M-PRESENTER (presenter)
                                                          ▼
                                                       operator approval
```

Wave A unit tests T-D-N9..N12 can run in parallel after T-D-N1..N8
land (they exercise different code surfaces). Wave D's T-D-N18-N19
(strategy + scenario files) can be authored in parallel with
T-D-N16-N17 (forecast_distribution dispatch) since they touch
different files. Architect refines the parallelism map at M-T1.

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
