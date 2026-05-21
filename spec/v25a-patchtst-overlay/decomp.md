---
slug: v25a-patchtst-overlay
status: in-progress
owner: developer
updated: 2026-05-21
phase: M-T1 architect-locked
---

# Architect decomposition — v2.5a PatchTST forecast overlay v0.1.0

> Phase 2 of 4 in the DL forecast-overlay roadmap. M-OD resolved
> 2026-05-21 via operator's "Autoapprove all" directive (Q1=(a)
> PatchTST, Q2=(a) full MVP, Q3=(a) patch_len=16/stride=8,
> Q4=(b) 24h target with overlapping samples, Q5=(c) 5-feature
> carry-forward, Q6=(a) BS-1 span 2023-FY, Q7=(a) anchor under
> `v2.5a.0-patchtst`, Q8=(a) sibling strategy).
> Predecessor: `v25-tcn-horizon-bump-or-retire v0.1.0` (operator
> Q1=(b) retired v2.5 TCN at 1h). All 28 predecessor anchors
> byte-immutable.

---

## §1 — T-AR-1..T-AR-8 resolutions

### T-AR-1 — PatchTST topology hyperparameter lock

**Decision.** PatchTST/42 small config per Nie et al 2022 § 4.2,
adapted for `n_patches = 42` over a 336-bar lookback at
`patch_len=16, stride=8` (Q3=(a) defaults):

| Field | Value | Rationale |
|-------|-------|-----------|
| `patch_len` | 16 | Q3=(a); Nie et al ETT default; ≈16 h history per patch on hourly bars. |
| `stride` | 8 | Q3=(a); 50% overlap; reinforces local patterns per § 3.1 of paper. |
| `context_len` | 336 | Nie et al ETT lookback for h=24; ≈14 days. Yields `n_patches = floor((336 - 16) / 8) + 1 = 41`. **Final n_patches = 42** when we left-pad the lookback by 16 bars per Nie et al § 3.1 ("RevIN + reflection pad"). |
| `d_model` | 128 | PatchTST/42 default. |
| `n_heads` | 4 | PatchTST/42 default. `d_model / n_heads = 32` head dim. |
| `d_ff` | 256 | PatchTST/42 default; 2× `d_model`. |
| `n_layers` | 3 | PatchTST/42 default. Smaller than Transformer/big (12 layers) — keeps attention cost manageable on 42 tokens × 5 channels. |
| `dropout` | 0.2 | Nie et al default. |
| `head_dropout` | 0.0 | Projection head — no extra regularisation. |
| `target_horizon_bars` | 24 | Q4=(b); 24h cumulative log-return. |
| `channels` (input features) | 5 | Q5=(c) carry-forward. |
| `output_dim` | 1 | Scalar `r_hat` per sample (same as TCN). |

**Parameter count estimate** (with channel-independence):
- Patch embed: `16 × 128 = 2,048` weights × 1 layer (shared across 5 channels) = ~2k.
- Position embed: `42 × 128 = 5,376`.
- Encoder block × 3:
  - Q/K/V: `3 × (128 × 128) = 49,152` per block.
  - O-proj: `128 × 128 = 16,384` per block.
  - FFN: `2 × (128 × 256) = 65,536` per block.
  - 2× LayerNorm: `2 × (2 × 128) = 512` per block.
  - Per-block total: ~131,584.
  - 3 blocks: ~394,752.
- Projection head: `(42 × 128) × 1 = 5,376` (flatten + linear-to-scalar).
- **Grand total per channel ≈ 410k; with channel-independence + 5 channels processed through shared weights, total params ≈ 410k**. Well under the 1.5-2M analyst estimate (analyst was conservative); ~10× smaller than TCN's 4.4M.

> **Note** — the analyst brief at `feature.md § R1` cites "~1.5-2M params" as an upper bound; the architect lock at ~410k is well within the ADR-0028 5-10M ceiling and the K1 compute-budget envelope. Smaller params means faster training (~3-4 days wall-clock instead of the analyst's pessimistic 5-7 days estimate).

**Citations.**
- Nie et al 2022 § 4.2 (PatchTST/42 config).
- `spec/v25a-patchtst-overlay/feature.md § R1` (analyst hyperparameter list).
- ADR-0028 § R-CEIL (parameter count ceiling 5-10M for the v2.5 family).

### T-AR-2 — ADR-0036 author

**Decision.** Author `spec/architecture/adr/0036-patchtst-training-contract.md` (status `proposed`, date 2026-05-21) covering D1-D7 per the feature.md § M-T1 milestone. ADR file path resolved this M-T1 tick alongside this `decomp.md`. See `spec/architecture/adr/0036-patchtst-training-contract.md` for full text. Cross-refs ADR-0028 (candle), ADR-0029 (provenance — extended additively in D2), ADR-0033 (F-verdict — immutable, PatchTST reports use same priority tree), ADR-0034 (train_events emission), ADR-0035 (σ_train post-training pattern — D1 cited verbatim).

### T-AR-3 — Target-derivation extension for 24h horizon (Q4=(b))

**Decision.** Extend `crates/forecast/src/features.rs` with an **additive helper + config field** rather than a new file. The existing 1h target at `features.rs:623-636` becomes the `target_horizon_bars=1` special case of a generalised computation. Specifically:

1. Add `target_horizon_bars: usize` to `FeatureConfig` (default 1 to preserve v2.5 TCN's existing semantics — TCN code path unchanged byte-for-byte).
2. Update `WindowIterator::new()` (`features.rs:524`) to require `min_bars = warmup + context + target_horizon_bars` (was `+ 1`).
3. Update `WindowIterator::next()` target computation (`features.rs:623-636`) to use `bars[window_end + target_horizon_bars].close` instead of `bars[window_end + 1].close`. The arithmetic stays:
   ```rust
   let t_next = window_end + cfg.target_horizon_bars;
   let close_t = self.bars[window_end].close;
   let close_t1 = self.bars[t_next].close;
   let target_logret = (close_t1 / close_t).ln();
   ```
4. Update `max_cursor = bars.len() - 1 - target_horizon_bars` (was `- 2`) so the iterator stops with enough bars for the target.

**Q4 sub-resolution.** Q4a (overlapping vs non-overlapping targets) = **overlapping** (~87k samples). Implementation cost is zero — the existing iterator already strides by 1 bar; non-overlapping would require an `if (window_end - cursor_start) % target_horizon_bars == 0` filter, which we **do not add** for v0.1.0.

**TCN backward-compat.** The default value `target_horizon_bars = 1` makes every existing TCN call site byte-identical. The K6 invariant ("`tcn.rs` byte-identical") holds because `tcn.rs` does NOT touch `features.rs` directly — it builds its own 5-feature window via the inference-time helper `build_feature_window_from_ohlcv` at `tcn.rs:668-744`. The training-time read site in `train_tcn.rs` will need to pass `FeatureConfig { target_horizon_bars: 1, ..default }` explicitly, which is a comment-only annotation per § K6 (no behavioural change).

**Citations.**
- `crates/forecast/src/features.rs:489-636` (current target-derivation site).
- `crates/forecast/src/tcn.rs:668-744` (TCN inference-time feature builder — untouched).
- `spec/v25a-patchtst-overlay/feature.md § R1, § Q4b` (overlapping samples).

### T-AR-4 — Sibling strategy shape (Q8=(a))

**Decision.** Create `crates/strategy/src/patchtst_overlay_momentum.rs` as a **sibling file** to `tcn_overlay_momentum.rs`. The new file contains:

- `PatchTstOverlayMomentumStrategy` struct (mirror of `TcnOverlayMomentumStrategy`).
- 4 builder pairs (mirror of `tcn_overlay_momentum.rs:476-624`):
  - `with_patchtst_bs1(base) -> Result<Self, forecast::patchtst::PatchTstForecasterError>` (under `feature = "forecast"`).
  - `with_patchtst_bs1_ledger(base, ledger)` (under `feature = "forecast-audit-tick"`).
  - `with_patchtst_bs1_tuned(base, confidence_threshold, direction_epsilon)` (under `feature = "forecast"`).
  - `with_patchtst_bs1_ledger_tuned(base, ledger, confidence_threshold, direction_epsilon)` (under `feature = "forecast-audit-tick"`).
- `Strategy` impl (mirror of `tcn_overlay_momentum.rs:627+`).
- `with_patchtst_bs2*` builders are **NOT** included in v0.1.0 (Q2=(a) ships BS-1 only; v0.1.1 spawns BS-2).
- Strategy ID: `"patchtst_overlay_momentum"`.

**Sync-forecaster wrapper.** Reuse the pattern at `crates/strategy/src/tcn_sync.rs` — create a sibling `patchtst_sync.rs` containing `PatchTstSyncForecaster` that wraps `PatchTstForecaster` and exposes the synchronous methods that `MomentumStrategy::Strategy` impl uses. Both `tcn_sync.rs` and `patchtst_sync.rs` follow the same shape — additive, zero touch on TCN.

**Module wiring.** Add `pub mod patchtst_overlay_momentum;` and `pub mod patchtst_sync;` to `crates/strategy/src/lib.rs`. No re-export changes (additive only).

**Citations.**
- `crates/strategy/src/tcn_overlay_momentum.rs:466-624` (mirror source).
- `crates/strategy/src/tcn_sync.rs` (sync-wrapper precedent).
- `spec/v25a-patchtst-overlay/feature.md § R6` (sibling-strategy spec).

### T-AR-5 — Wave map A-F

**Wave A — Model + training scaffold + unit tests (3-7 days; ~12 T-D rows).**
- **A.1** `crates/forecast/src/patchtst.rs` — model file (1 of 1 — every layer in one file mirroring `tcn.rs`'s single-file layout). Implements:
  - `PatchTstModel` struct (`patch_embed`, `pos_embed`, `encoder_blocks: Vec<TransformerBlock>`, `proj_head`).
  - `PatchEmbed` (Linear `patch_len → d_model`).
  - `SinusoidalPositionEncoding` OR `LearnablePositionEncoding` (architect picks **learnable** per ADR-0036 § D1 — Nie et al ETT default).
  - `MultiHeadSelfAttention` (custom — 4 heads, scaled dot-product, pre-LN; ~50 LoC). Architect chooses **custom** over `candle_transformers::*` per ADR-0036 § D5 (smaller surface for K2 determinism gate).
  - `TransformerBlock` (MHSA + FFN with residual + pre-LN, mirroring Nie et al § 3.1).
  - `ProjectionHead` (Linear `n_patches × d_model → 1`).
  - `PatchTstForecaster` with `random_init` / `load_anchor` / `load_from_paths`.
  - `AnchorScenario::Bs1` (mirror of TCN's enum; only `Bs1` exists at v0.1.0 — `Bs2` enum variant added if/when v0.1.1 ships).
  - `impl ForecastProvider for PatchTstForecaster` mirroring TCN's body (forward → r_hat → direction quantisation via ε deadband → confidence via `|r_hat|/σ_train` clamp).
- **A.2** Extend `crates/forecast/src/features.rs` with `target_horizon_bars` field (T-AR-3).
- **A.3** `crates/forecast/src/bin/train_patchtst.rs` — training scaffold (sibling to `train_tcn.rs`). CLI flags + AdamW + OneCycle + Huber loss + train_events emission per ADR-0034. **No in-loop σ_train accumulator** per ADR-0035 § D1 (the deprecated pattern at `train_tcn.rs:606,676-678,733-741` is explicitly NOT replicated).
- **A.4** Wave C (σ_train derivation) folds into the terminal phase of `train_patchtst.rs`: after the per-epoch loop converges, the bin runs a frozen-weights forward pass over the training-data span and writes `patchtst-bs1-<sha>.metadata.json` with `sigma_train` populated. No `.metadata.recalibrated.json` overlay file at ship.
- **A.5** Unit tests (developer-callable in parallel after model lands):
  - `crates/forecast/tests/sigma_train_not_in_safetensors_patchtst.rs` (ADR-0035 § D4 invariant — parses safetensors header, asserts no tensor name contains `sigma` / `output_scale` / `sigma_train`).
  - `crates/forecast/tests/forward_determinism_patchtst.rs` (K2 candle-attention determinism: CPU + Metal byte-identical forward pass on a fixed-seed input; tolerance 1e-4 per ADR-0029 § 4 Metal-vs-CPU caveat).
  - `crates/forecast/tests/tcn_byte_identity.rs` (K6 scope-creep guard: `git diff HEAD -- crates/forecast/src/tcn.rs crates/forecast/checkpoints/anchors/tcn-*.{safetensors,metadata.json,metadata.recalibrated.json}` is empty).
  - `crates/forecast/tests/patchtst_overlay_neutrality.rs` (K4 anchor neutrality: re-runs the TCN-only `top10-2023-fy-tcn-overlay-realdata` scenario and asserts its body bytes match the anchored SHA `8fa47f49…`).

**Wave B — BS-1 training run (5-7 days wall-clock; orchestrator-monitored).**
- **B.1** Developer kicks off `cargo run -p forecast --release --features candle --bin train_patchtst -- --scenario bs1 --target-horizon-bars 24 --span-start 2023-01-01 --span-end 2023-12-31 --epochs 30 --batch-size 128 --seed 0x00C0FFEE 2>&1 | tee /tmp/train_patchtst-bs1.log`.
- **B.2** MANDATORY watch recipe per MEMORY.md (R8):
  ```bash
  # Replace <PID> via `pgrep -f train_patchtst`.
  watch -n 60 'tail -30 /tmp/train_patchtst-bs1.log && \
               echo "---" && \
               ps -p <PID> -o pcpu,pmem,etime,command | tail -2 && \
               echo "---" && \
               ls -lh crates/forecast/checkpoints/anchors/patchtst-bs1-*.safetensors 2>/dev/null || echo "(checkpoint not yet written)"'
  ```
- **B.3** Cockpit training-control panel (ADR-0034) surfaces `train_events` rows tagged `model_family = "patchtst"`.
- **B.4** Cost-tripwire per ADR-0036 § D4: if single epoch > 24h OR epoch N > 3× rolling median of epochs 1..N-1, developer pauses + emits diagnostic dump + escalates to operator.

**Wave C — σ_train derivation (folded into B's terminal phase; 0 wall-clock).**
- **C.1** Inside `train_patchtst.rs` post-training block (per ADR-0035 § D1): load the just-trained safetensors via `PatchTstForecaster::load_from_paths(<just-written>, <synthetic-metadata-without-sigma>)`; iterate `windows_for_symbol()` over the training span; collect all `r_hat`; compute population std with f64 intermediates and `1e-8` floor; write the canonical `<file_prefix>-<sha>.metadata.json` with the σ_train scalar (JSON number convention per ADR-0035 § D2).

**Wave D — Alpha-investigation + strategy integration (1-2 days; ~6 T-D rows).**
- **D.1** Extend `crates/forecast/src/bin/forecast_distribution.rs` — additive enum variant `Scenario::PatchtstBs1`. The existing `Scenario::Bs1` / `Scenario::Bs2` variants stay byte-identical (architect-locked dispatch design — see § Architect decision below). When `--scenario patchtst-bs1` is passed, the bin loads `PatchTstForecaster::load_anchor(AnchorScenario::Bs1)` and runs the same F-verdict algorithm from ADR-0033 § D3 (algorithm IMMUTABLE; new dispatch arm only).
- **D.2** Run `cargo run -p forecast --release --features candle --bin forecast_distribution -- --scenario patchtst-bs1 --output spec/v25a-patchtst-overlay/reports/forecast-distribution-patchtst-bs1-realdata-20260521.md`. The report body's F-verdict line records F1/F2/F3/F4 per ADR-0033 § D3 IMMUTABLE priority tree. 2-run byte-identity verified.
- **D.3** Create `crates/strategy/src/patchtst_overlay_momentum.rs` + `crates/strategy/src/patchtst_sync.rs` per T-AR-4 above.
- **D.4** Create `crates/backtest/src/scenarios/patchtst_overlay_weights.rs` (mirror of `tcn_overlay_weights.rs`). Register the new scenario name `top10-2023-fy-patchtst-overlay-realdata` in `crates/backtest/src/scenarios/mod.rs`'s `Scenario` enum (additive variant; existing variants byte-immutable).
- **D.5** Run `cargo run -p backtest --release --features "candle realdata" -- --scenario top10-2023-fy-patchtst-overlay-realdata --seed 0xC0FFEE 2>&1 | tee /tmp/backtest-patchtst-bs1.log`. Emit `spec/v25a-patchtst-overlay/reports/top10-2023-fy-patchtst-overlay-realdata-20260521.md`. 2-run byte-identity verified.
- **D.6** Extend `crates/forecast/src/bin/sharpe_comparison.rs` with the new PatchTST source-path in its frontmatter `sources` list. Run + emit `spec/v25a-patchtst-overlay/reports/sharpe-comparison-patchtst-bs1-realdata-20260521.md`. 2-run byte-identity verified.

**Wave E — Tester gate (0.5-1 day).**
- **E.1** `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo clippy -p forecast --features candle -- -D warnings`.
- **E.2** `cargo test --workspace --lib`.
- **E.3** `cargo test -p forecast --features candle --test sigma_train_not_in_safetensors_patchtst` (ADR-0035 § D4).
- **E.4** `cargo test -p forecast --features candle --test forecast_distribution_verdict` (ADR-0033 § D3 IMMUTABLE).
- **E.5** `cargo test -p forecast --features candle --test forward_determinism_patchtst` (K2).
- **E.6** `cargo test -p forecast --features candle --test tcn_byte_identity` (K6).
- **E.7** `cargo test --features candle --test patchtst_overlay_neutrality` (K4).
- **E.8** `git diff HEAD -- crates/forecast/src/tcn.rs` is empty (K6 manual gate).
- **E.9** `git diff HEAD -- crates/forecast/checkpoints/anchors/tcn-*.{safetensors,metadata.json,metadata.recalibrated.json}` is empty (8 file invariance per K6).
- **E.10** 2-run byte-identity gate on both new PatchTST reports.
- **E.11** `uv run scripts/spec_lint.py` matches baseline (0 new categories).
- **E.12** `bash scripts/verify_anchors.sh` PRE: 26 PASS + 2 known-glob-collision-FAIL (baseline carried forward; see § Anchor gate baseline). POST: 28 PASS + 2 known-FAIL (2 new PatchTST anchors land under `v2.5a.0-patchtst`).
- **E.13** Joint advisory verdict (F-verdict + Sharpe-delta + T-classifier) recorded in `feature.md § Verification`.
- **E.14** Trace row `REQ-V25A-PATCHTST-001` `tests` / `anchors` columns filled by tester per shipped reality.

**Wave F — Presenter deck (0.5 day).**
- **F.1** Author `spec/v25a-patchtst-overlay/presentations/v25a-patchtst-overlay-<YYYY-MM-DD>.md` per ADR-0033 § D3 routing matrix (H1 confirmed/falsified, T-MARGINAL/T-ALPHA-UNLOCKED).
- **F.2** Operator approval; frontmatter flips `status: in-progress → shipped`; trace row + backlog flip Active → Recent.

### T-AR-6 — Anchor strategy (Q7=(a) confirmed)

**Decision.** 2 anchors land under version `v2.5a.0-patchtst`. The 28 predecessor anchors stay byte-identical.

| Anchor | Source report | Locked by |
|--------|---------------|-----------|
| `forecast-distribution-patchtst-bs1-realdata` | `spec/v25a-patchtst-overlay/reports/forecast-distribution-patchtst-bs1-realdata-20260521.md` | tester Wave E |
| `top10-2023-fy-patchtst-overlay-realdata` | `spec/v25a-patchtst-overlay/reports/top10-2023-fy-patchtst-overlay-realdata-20260521.md` | tester Wave E |

> **Note** — `sharpe-comparison-patchtst-bs1-realdata` is **not** anchored at v0.1.0 (mirror of the v25-tcn-overlay precedent, where `sharpe-comparison-realdata` was anchored only after the alpha-investigation cycle stabilised on 2026-05-19; here we ship the report without an anchor to keep v0.1.0 lean and let the bake-off in v2.6 lock the cross-model comparison family). If operator overrides this at M-FINAL, tester adds the 3rd anchor; the 28 originals still stay byte-immutable.

**Per-cell tuned anchors** (analogous to the v25-tcn-threshold-tuning ship) defer to v0.1.1 if T-ALPHA-UNLOCKED fires at M-PRESENTER.

### T-AR-7 — Non-regression contract

**Decision.** All 12 invariants from `feature.md § Non-regression contract` are locked. The architect names 2 specific risk mitigations that drop into the developer's hands:

1. **K4 anchor neutrality** — the `patchtst_overlay_neutrality` test (Wave A.5) re-runs the existing TCN-only scenario and hashes the report body, asserting against the anchored SHA `8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642` (top10-2023-fy-tcn-overlay-realdata). This catches any code path leak from PatchTST into the TCN scenario output bytes.

2. **K6 scope-creep guard** — the `tcn_byte_identity` test (Wave A.5) runs `git diff HEAD -- crates/forecast/src/tcn.rs` and asserts empty. The test exec'd via `git` is feature-gated `#[cfg(not(target_env = "musl"))]` because some CI runners have no git binary; the fallback compares against a baked-in SHA via `include_str!`-of-the-anchored-tcn-rs-baseline-via-Wave-A.5 (architect-decide: prefer `git` because it's simpler — fall back only if CI tells us otherwise).

### T-AR-8 — Watch recipe + cost tripwire helper

**Decision.** Per MEMORY.md, the developer MUST emit the watch recipe at Wave B kickoff. The architect codifies a developer-callable helper `assert_epoch_budget(epoch_n: usize, epoch_wall_clock_sec: u64, history: &[u64]) -> Result<(), CostTripwireError>`:

```rust
// crates/forecast/src/bin/train_patchtst.rs (D4-derived)
fn assert_epoch_budget(
    epoch_n: usize,
    epoch_wall_clock_sec: u64,
    history: &[u64],
) -> Result<(), CostTripwireError> {
    const HARD_LIMIT_SEC: u64 = 24 * 3600; // 24 h per ADR-0036 § D4
    if epoch_wall_clock_sec > HARD_LIMIT_SEC {
        return Err(CostTripwireError::HardLimit { epoch: epoch_n, wall_clock_sec: epoch_wall_clock_sec });
    }
    if epoch_n > 0 && !history.is_empty() {
        let mut sorted = history.to_vec();
        sorted.sort_unstable();
        let median = sorted[sorted.len() / 2];
        if epoch_wall_clock_sec > 3 * median {
            return Err(CostTripwireError::MedianMultiple { epoch: epoch_n, wall_clock_sec: epoch_wall_clock_sec, median });
        }
    }
    Ok(())
}
```

The helper is invoked at the end of each epoch in the training loop. On `Err(_)`, the bin emits a `tracing::error!` line, writes a diagnostic file at `/tmp/train_patchtst-bs1-tripwire-epoch{N}.txt`, and **continues** training (the operator owns the "stop or continue" decision after escalation — automatic stop would lose progress if the operator wants to investigate a transient).

The watch recipe in MEMORY.md format goes at the head of `train_patchtst.rs` as a `//! Watch recipe` doc-comment so it's discoverable via `cargo doc`.

---

## §2 — Module / file change-map

| Path | Op | Wave | Notes |
|------|----|------|-------|
| `crates/forecast/src/patchtst.rs` | NEW | A.1 | PatchTST model + forecaster + ForecastProvider impl. ~700 LoC, mirrors `tcn.rs`'s single-file layout. |
| `crates/forecast/src/lib.rs` | EDIT (+1 line) | A.1 | Add `pub mod patchtst;`. |
| `crates/forecast/src/features.rs` | EDIT (additive) | A.2 | Add `target_horizon_bars: usize` field to `FeatureConfig`; default 1 for TCN compatibility. ~6-line touch. |
| `crates/forecast/src/bin/train_patchtst.rs` | NEW | A.3 | Training scaffold. ~600 LoC, mirrors `train_tcn.rs` but with σ_train post-training (ADR-0035 § D1) instead of in-loop accumulator. |
| `crates/forecast/tests/sigma_train_not_in_safetensors_patchtst.rs` | NEW | A.5 | ADR-0035 § D4 invariant test. ~40 LoC. |
| `crates/forecast/tests/forward_determinism_patchtst.rs` | NEW | A.5 | K2 candle-attention determinism (CPU + Metal). ~80 LoC. |
| `crates/forecast/tests/tcn_byte_identity.rs` | NEW | A.5 | K6 scope-creep guard. ~60 LoC. |
| `crates/forecast/tests/patchtst_overlay_neutrality.rs` | NEW | A.5 | K4 anchor neutrality test. ~80 LoC. |
| `crates/forecast/Cargo.toml` | EDIT (+5 lines) | A.3 | `[[bin]]` entry for `train_patchtst`. |
| `crates/forecast/checkpoints/anchors/patchtst-bs1-<sha>.safetensors` | NEW (LFS) | B.1 | Training output. ~5 MB (smaller than TCN's ~17 MB). |
| `crates/forecast/checkpoints/anchors/patchtst-bs1-<sha>.metadata.json` | NEW | B.4 / C.1 | σ_train + provenance (ADR-0029 schema extended with PatchTST hyperparameters per ADR-0036 § D2). |
| `crates/forecast/src/bin/forecast_distribution.rs` | EDIT (additive) | D.1 | New enum arm `Scenario::PatchtstBs1`; ~30-line patch; existing TCN dispatch byte-immutable. |
| `crates/forecast/src/bin/sharpe_comparison.rs` | EDIT (additive) | D.6 | Additive source-path in `sources` list. ~10-line patch. |
| `crates/strategy/src/patchtst_overlay_momentum.rs` | NEW | D.3 | Sibling strategy. ~250 LoC. |
| `crates/strategy/src/patchtst_sync.rs` | NEW | D.3 | Sync-wrapper sibling to `tcn_sync.rs`. ~80 LoC. |
| `crates/strategy/src/lib.rs` | EDIT (+2 lines) | D.3 | `pub mod patchtst_overlay_momentum;` + `pub mod patchtst_sync;`. |
| `crates/backtest/src/scenarios/patchtst_overlay_weights.rs` | NEW | D.4 | Sibling backtest scenario. ~180 LoC. |
| `crates/backtest/src/scenarios/mod.rs` | EDIT (additive) | D.4 | New enum arm `Scenario::Top10_2023FyPatchtstOverlayRealdata`. |
| `spec/v25a-patchtst-overlay/reports/forecast-distribution-patchtst-bs1-realdata-20260521.md` | NEW | D.2 | Anchored at Wave E. |
| `spec/v25a-patchtst-overlay/reports/top10-2023-fy-patchtst-overlay-realdata-20260521.md` | NEW | D.5 | Anchored at Wave E. |
| `spec/v25a-patchtst-overlay/reports/sharpe-comparison-patchtst-bs1-realdata-20260521.md` | NEW | D.6 | Not anchored at v0.1.0 (defer to v2.6 bake-off). |
| `spec/architecture/adr/0036-patchtst-training-contract.md` | NEW | M-T1 | This decomp's sibling ADR. ~180 LoC. |
| `spec/architecture/adr/README.md` | EDIT (+1 line in registry, +1 changelog entry) | M-T1 | Register ADR-0036. |
| `spec/anchors.toml` | EDIT (+2 anchors at end-of-file) | E.12 | `forecast-distribution-patchtst-bs1-realdata` + `top10-2023-fy-patchtst-overlay-realdata` under version `v2.5a.0-patchtst`. 28 existing anchor rows byte-immutable. |
| `spec/trace.toml` | EDIT (REQ-V25A-PATCHTST-001 row) | M-T1 / M-FINAL | architect: state `proposed → in-progress` + add `decomp.md` + ADR-0036 to `arch`; populate `tests` array. tester at M-FINAL: state `in-progress → shipped` + fill `anchors`. |
| `spec/v25a-patchtst-overlay/tasks.md` | EDIT | M-T1 | Tick T-AR-1..T-AR-8; expand T-D-N rows per Wave A-F. |

**Total NEW files**: 13 (5 source + 4 test + 1 metadata + 1 safetensors + 1 ADR + 1 decomp). **EDIT files**: 8 (all additive). **Touch on existing `tcn.rs` / `tcn_overlay_momentum.rs` / `tcn_overlay_weights.rs` / TCN anchor files**: **ZERO** (K6 invariant).

---

## §3 — Wave A-F ordered with file:line entry points

> Each row carries the file:line entry-point + cargo invocation it will produce + the expected literal output. The developer at M-D ticks each in order; rows within a wave that touch different files can be parallelised per § Parallelism map below.

### Wave A — Model + scaffold + tests (3-7 days)

- **T-D-N1** — Create `crates/forecast/src/patchtst.rs:1` skeleton + add `pub mod patchtst;` to `crates/forecast/src/lib.rs:42`. Stub all types with `unimplemented!()`. Cargo: `cargo check -p forecast --features candle 2>&1 | tail -3` → expects `Finished ... in N.Ns`.

- **T-D-N2** — Implement `PatchEmbed { proj: Linear, patch_len: usize, stride: usize }` + `forward(x: &Tensor) -> Tensor` at `patchtst.rs`. Input `[batch, channels, time]` → patches via `Tensor::unfold(2, patch_len, stride)` → `[batch, channels, n_patches, patch_len]` → linear projection to `[batch, channels, n_patches, d_model]`. Cargo: `cargo test -p forecast --features candle --lib patchtst::tests::patch_embed_shape -- --nocapture 2>&1 | grep "test result"` → expects `test result: ok. 1 passed`.

- **T-D-N3** — Implement `LearnablePositionEncoding` (shape `[n_patches, d_model]` learnable parameter; added to patch embeddings). Cargo: `cargo test -p forecast --features candle --lib patchtst::tests::pos_encoding_shape 2>&1 | grep "test result"` → expects `test result: ok. 1 passed`.

- **T-D-N4** — Implement `MultiHeadSelfAttention` (custom; 4 heads; pre-LN; scaled dot-product per Vaswani 2017). Cargo: `cargo test -p forecast --features candle --lib patchtst::tests::mhsa_forward_shape 2>&1 | grep "test result"` → expects `test result: ok. 1 passed`.

- **T-D-N5** — Implement `TransformerBlock` (MHSA + FFN + 2× residual + 2× LayerNorm, pre-LN order). Cargo: `cargo test -p forecast --features candle --lib patchtst::tests::block_forward_shape 2>&1 | grep "test result"` → expects `test result: ok. 1 passed`.

- **T-D-N6** — Implement `PatchTstModel::new(vb: VarBuilder) -> Result<Self>` (stacks 3 blocks; wires patch_embed + pos_embed + encoder + projection head) and `forward(x: &Tensor, train: bool) -> Result<Tensor>` (input `[batch, 5, 336]` → output `[batch, 1]`). Cargo: `cargo test -p forecast --features candle --lib patchtst::tests::model_forward_shape 2>&1 | grep "test result"` → expects `test result: ok. 1 passed`. Verify parameter count via `model.num_parameters()` ≈ 410k (Wave A.1 estimate).

- **T-D-N7** — Implement `PatchTstForecaster::random_init` + `load_anchor(AnchorScenario::Bs1)` + `load_from_paths` mirroring TCN. Cargo: `cargo test -p forecast --features candle --lib patchtst::tests::forecaster_random_init 2>&1 | grep "test result"` → expects `test result: ok. 1 passed`.

- **T-D-N8** — Implement `impl ForecastProvider for PatchTstForecaster` (mirrors `tcn.rs:782-1034` — forward pass → r_hat scalar → direction via `r_hat_to_direction` → confidence via `(|r_hat| / sigma_train).clamp(0, 1)`). Cargo: `cargo test -p forecast --features candle --lib patchtst::tests::forecast_provider_boxed 2>&1 | grep "test result"` → expects `test result: ok. 1 passed`.

- **T-D-N9** — Extend `crates/forecast/src/features.rs:489` (`FeatureConfig`) with `target_horizon_bars: usize` (default 1). Update `WindowIterator::new` at `features.rs:524` and target-derivation at `features.rs:623-636`. Cargo: `cargo test -p forecast --lib features 2>&1 | grep "test result"` → expects all existing tests + 1 new (`target_horizon_bars_default_1_unchanged_tcn`) passing.

- **T-D-N10** — Create `crates/forecast/src/bin/train_patchtst.rs:1` mirroring `train_tcn.rs`. CLI flags per `feature.md § R2`. AdamW + OneCycle + Huber loss. `target_horizon_bars = 24` per Q4=(b). **NO `Vec<f32>::new()` declaration outside the per-epoch loop scope** per ADR-0035 § D1 — codified as the architect's code-review check. Add `[[bin]] name = "train_patchtst"` entry to `crates/forecast/Cargo.toml`. Cargo: `cargo check -p forecast --features candle --bin train_patchtst 2>&1 | tail -3` → expects `Finished ... in N.Ns`.

- **T-D-N11** — In `train_patchtst.rs`, emit `train_events` rows per epoch tagged `model_family = "patchtst"` per ADR-0034. Verify cockpit panel surfaces the new family via manual smoke at M-D end. Cargo: `cargo run -p forecast --release --features candle --bin train_patchtst -- --scenario bs1 --epochs 1 --batch-size 4 --span-start 2023-01-01 --span-end 2023-01-07 --seed 0x00C0FFEE` (sanity 1-epoch run on small span; <1 min wall-clock). Expects `train_events` rows in audit-DB.

- **T-D-N12** — Implement `assert_epoch_budget` helper in `train_patchtst.rs` per T-AR-8. Cargo: `cargo test -p forecast --features candle --lib train_patchtst::tests::epoch_budget_hard_limit 2>&1 | grep "test result"` → expects `test result: ok. 1 passed`.

- **T-D-N13** — Create `crates/forecast/tests/sigma_train_not_in_safetensors_patchtst.rs:1` (mirror of `sigma_train_not_in_safetensors.rs`). Parses safetensors header via `safetensors::SafeTensors::deserialize`; asserts no tensor name contains `sigma` / `output_scale` / `sigma_train`. Test is **conditionally skipped** if no `patchtst-bs1-*.safetensors` exists on disk (so the test passes in Wave A before training lands). Cargo: `cargo test -p forecast --features candle --test sigma_train_not_in_safetensors_patchtst 2>&1 | grep "test result"` → expects `test result: ok. 1 passed (1 ignored)` (the only test is `#[test] fn sigma_not_in_patchtst_safetensors()` — `#[ignore]`d when checkpoint absent).

- **T-D-N14** — Create `crates/forecast/tests/forward_determinism_patchtst.rs:1` (K2). Builds a fixed-seed `PatchTstForecaster::random_init` on `Device::Cpu`, runs `forward(&x, false)` twice with the same input, asserts byte-identical outputs. If `candle::Device::new_metal(0)` succeeds, also runs on Metal and asserts `(metal - cpu).abs().max() < 1e-4` per ADR-0029 § 4. Cargo: `cargo test -p forecast --features candle --test forward_determinism_patchtst 2>&1 | grep "test result"` → expects `test result: ok. 2 passed` (cpu + metal-or-skip).

- **T-D-N15** — Create `crates/forecast/tests/tcn_byte_identity.rs:1` (K6). Runs `git diff --quiet HEAD -- crates/forecast/src/tcn.rs` via `std::process::Command::new("git")`; asserts exit code 0. Same diff for `tcn-*.safetensors`, `tcn-*.metadata.json`, `tcn-*.metadata.recalibrated.json` (4 + 2 + 2 = 8 files). Cargo: `cargo test --workspace --test tcn_byte_identity 2>&1 | grep "test result"` → expects `test result: ok. 1 passed`. (Skipped via `#[ignore]` if `git` not on PATH.)

- **T-D-N16** — Create `crates/forecast/tests/patchtst_overlay_neutrality.rs:1` (K4). Re-runs the existing TCN-only `top10-2023-fy-tcn-overlay-realdata` scenario via `cargo run -p backtest --release --features "candle realdata" -- --scenario top10-2023-fy-tcn-overlay-realdata --seed 0xC0FFEE`; hashes the resulting report body via `scripts/hash_report.py`; asserts SHA `8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642`. **The test is `#[ignore]`d in CI** because it invokes a 5-minute backtest run; developer runs it manually at M-D end and tester runs it at M-FINAL. Cargo: `cargo test -p forecast --features candle --test patchtst_overlay_neutrality -- --ignored --nocapture 2>&1 | grep "test result"` → expects `test result: ok. 1 passed`.

### Wave B — BS-1 training run (5-7 days wall-clock)

- **T-D-N17** — Kick off Wave B training:
  ```bash
  RUST_LOG=info,forecast=debug \
    cargo run -p forecast --release --features candle --bin train_patchtst -- \
      --scenario bs1 \
      --target-horizon-bars 24 \
      --span-start 2023-01-01 \
      --span-end 2023-12-31 \
      --patch-len 16 \
      --stride 8 \
      --d-model 128 \
      --n-heads 4 \
      --d-ff 256 \
      --n-layers 3 \
      --dropout 0.2 \
      --context-len 336 \
      --epochs 30 \
      --batch-size 128 \
      --seed 0x00C0FFEE \
      2>&1 | tee /tmp/train_patchtst-bs1.log &
  ```
  Concurrently emit MANDATORY watch recipe to operator's screen (per MEMORY.md):
  ```bash
  watch -n 60 'tail -30 /tmp/train_patchtst-bs1.log && \
               echo "---" && \
               ps -p $(pgrep -f train_patchtst) -o pcpu,pmem,etime,command | tail -2 && \
               echo "---" && \
               ls -lh crates/forecast/checkpoints/anchors/patchtst-bs1-*.safetensors 2>/dev/null || echo "(checkpoint not yet written)"'
  ```
  Expected literal output (training start): `[2026-05-22T<HH>:<MM>:<SS>Z INFO  train_patchtst] Loaded 87234 training windows for span 2023-01-01..2023-12-31 (overlapping 24h targets, 10 symbols)`. Expected literal output (training end ~5-7 days later): `[INFO  train_patchtst] Training complete: epochs=30, final_train_huber=<f>, final_val_huber=<f>, sigma_train=<f>, safetensors=crates/forecast/checkpoints/anchors/patchtst-bs1-<sha>.safetensors`.

- **T-D-N18** — On training-complete: verify `patchtst-bs1-<sha>.safetensors` + `patchtst-bs1-<sha>.metadata.json` exist under `crates/forecast/checkpoints/anchors/`. σ_train scalar derived via post-training frozen forward-pass (ADR-0035 § D1). Cargo: `ls -lh crates/forecast/checkpoints/anchors/patchtst-bs1-*` and `python3 -c "import json; d=json.load(open('crates/forecast/checkpoints/anchors/patchtst-bs1-<sha>.metadata.json')); print(d['sigma_train'], d['model_revision'], d['weights_sha256'])"`. Expects 2 files (~5 MB safetensors + ~1 KB metadata).

- **T-D-N19** — Verify 2-run byte-identity of `patchtst-bs1-<sha>.safetensors` by re-running T-D-N17 with the same CLI args + seed in a separate workspace clone. Both safetensors files must hash byte-identically: `sha256sum patchtst-bs1-<sha>.safetensors` (run 1) == `sha256sum patchtst-bs1-<sha>.safetensors` (run 2). (R2 determinism contract. Note: this is the **safetensors body SHA**, not the `model_revision` SHA which is over the metadata.)

### Wave C — σ_train derivation (folded into Wave B; 0 wall-clock)

- (Implicit in T-D-N18; no separate Wave C T-D row.)

### Wave D — Alpha-investigation + strategy integration (1-2 days)

- **T-D-N20** — Extend `crates/forecast/src/bin/forecast_distribution.rs` with additive enum variant `Scenario::PatchtstBs1`. The dispatch arm calls `PatchTstForecaster::load_anchor(forecast::patchtst::AnchorScenario::Bs1)` instead of `TcnForecaster::load_anchor(...)`. Everything downstream of the forecaster handle is shared (the F-verdict algorithm from ADR-0033 § D3 is IMMUTABLE). Cargo: `cargo run -p forecast --release --features candle --bin forecast_distribution -- --scenario patchtst-bs1 --output spec/v25a-patchtst-overlay/reports/forecast-distribution-patchtst-bs1-realdata-20260521.md`. Expected literal output: `[INFO forecast_distribution] F-verdict: F<N> (priority: <X>, frac_inside_epsilon=<f>, ...)`.

- **T-D-N21** — Verify 2-run byte-identity of `forecast-distribution-patchtst-bs1-realdata-20260521.md`. Re-run T-D-N20; hash both bodies via `python3 scripts/hash_report.py`; expect equal hex SHAs.

- **T-D-N22** — Create `crates/strategy/src/patchtst_sync.rs` (sync wrapper, mirror of `tcn_sync.rs`) + `crates/strategy/src/patchtst_overlay_momentum.rs` (mirror of `tcn_overlay_momentum.rs` per T-AR-4) + update `crates/strategy/src/lib.rs` with 2 new `pub mod` declarations. Cargo: `cargo check -p strategy --features forecast 2>&1 | tail -3` → expects `Finished ... in N.Ns`.

- **T-D-N23** — Create `crates/backtest/src/scenarios/patchtst_overlay_weights.rs` (mirror of `tcn_overlay_weights.rs`). Register the new scenario name `top10-2023-fy-patchtst-overlay-realdata` in `crates/backtest/src/scenarios/mod.rs`'s `Scenario` enum. Cargo: `cargo check -p backtest --features "candle realdata" 2>&1 | tail -3` → expects `Finished ... in N.Ns`.

- **T-D-N24** — Run the backtest:
  ```bash
  cargo run -p backtest --release --features "candle realdata" -- \
    --scenario top10-2023-fy-patchtst-overlay-realdata \
    --seed 0xC0FFEE \
    2>&1 | tee /tmp/backtest-patchtst-bs1.log
  ```
  Verify `spec/v25a-patchtst-overlay/reports/top10-2023-fy-patchtst-overlay-realdata-20260521.md` is emitted with the standard body shape (Trades / P&L / Sharpe sections per ADR-0032).

- **T-D-N25** — Verify 2-run byte-identity of `top10-2023-fy-patchtst-overlay-realdata-20260521.md`. Re-run T-D-N24; hash both bodies via `python3 scripts/hash_report.py`; expect equal hex SHAs.

- **T-D-N26** — Extend `crates/forecast/src/bin/sharpe_comparison.rs` `sources` list with the new PatchTST source-paths. Run:
  ```bash
  cargo run -p forecast --release --features candle --bin sharpe_comparison -- \
    --output spec/v25a-patchtst-overlay/reports/sharpe-comparison-patchtst-bs1-realdata-20260521.md
  ```
  Verify the new comparison row includes PatchTST alongside v1 baseline + TCN. Verify 2-run byte-identity of the new report.

### Wave E — Tester M-FINAL (0.5-1 day)

- (T-T-1.a..T-T-1.o per existing `tasks.md`; architect locks the row IDs but does not pre-populate output — tester fills the literal output at M-FINAL.)

### Wave F — Presenter M-PRESENTER (0.5 day)

- (T-P-1, T-P-2 per existing `tasks.md`.)

---

## §4 — Parallelism map

```
M-T1 ───► Wave A ──────────────► Wave B ──► Wave D ────► Wave E ──► Wave F
   (this  │                       (sole       │
   tick)  │                       serial       │
          ├ T-D-N1..N8 (model     long-runner) ├ T-D-N20-N21 (forecast_distribution; serial)
          │  sequential)                       ├ T-D-N22 (strategy; parallel with N20-N21 — different files)
          ├ T-D-N9 (features.rs)              ├ T-D-N23 (backtest scenario; parallel with N20-N22)
          ├ T-D-N10-N12 (train scaffold)      ├ T-D-N24-N25 (backtest; serial after N22+N23)
          ├ T-D-N13-N16 (tests; parallel      └ T-D-N26 (sharpe; serial after N24-N25)
          │  AFTER N1-N8 land — they
          │  exercise different code
          │  surfaces and can run
          │  concurrently)
```

**Critical-path note.** Wave B's 5-7 day training run is the single
longest path. Waves D-F are 1-2 + 0.5-1 + 0.5 = ~2-3.5 days
combined. **Total Q2=(a) end-to-end: ~3-5 weeks best case; ~5-7
weeks with one Wave B retry**, matching the analyst's `feature.md
§ Cost estimate`.

---

## §5 — Spike requirement

**Decision.** **NO spike required.** Rationale:

1. PatchTST is well-documented (Nie et al 2022 + the `yuqinie98/PatchTST` reference impl); no novel research risk.
2. candle's primitives (`Linear`, `LayerNorm`, scaled dot-product via `Tensor::matmul`/`Tensor::softmax`) are battle-tested by the v2.5 TCN ship and the broader candle ecosystem (no PatchTST-specific kernel needed).
3. K2 (candle-attention determinism) is mitigated by the Wave A.5 `forward_determinism_patchtst` test — if that test fails at developer time, the developer falls back to a manually-implemented attention block (~80 LoC of explicit Tensor ops). This is a **planned-fallback**, not a research spike.
4. The K1 (compute budget) gate is owned by the `assert_epoch_budget` helper (T-AR-8) which fires automatically — no spike required to size the budget upfront.

If, at developer's discretion, K2 surfaces as a genuine blocker during T-D-N4 (MultiHeadSelfAttention implementation), the developer escalates to the architect — a 0.5-1 day spike to characterise candle's attention numerics on Metal would land at that point, NOT pre-emptively.

---

## §6 — Rollback shape per wave

Every wave must be revertable without breaking the 28 predecessor anchors. Specific rollback shapes:

| Wave | Rollback procedure | Affected anchors |
|------|--------------------|------------------|
| **A** (model + scaffold + tests) | `git revert <commit-range>` removes 5 new source files + 4 new tests + 1 new bin + 6-line `features.rs` patch + 1-line `lib.rs` patch. Existing TCN files untouched. | 0 — pure-additive. |
| **B** (training run) | Delete `crates/forecast/checkpoints/anchors/patchtst-bs1-*` (LFS-tracked; remove via `git lfs untrack` + `git rm`). Cancel via `pkill -f train_patchtst` if mid-run. | 0 — no anchors locked yet. |
| **C** (σ_train derivation) | Same as Wave B rollback (σ_train metadata is co-located with safetensors). | 0. |
| **D** (forecast-dist + sharpe + strategy + backtest scenario) | `git revert <commit>` removes the additive enum variants + new strategy file + new backtest scenario file. Existing TCN dispatch arms byte-immutable; revert is mechanical. | 0 — pure-additive. |
| **E** (tester gate) | If anchor count regresses or any test fails, route back to developer with `HANDOFF → developer` + body-diff per AGENT.md. No anchor-lock until all gates green. | 0 — tester gate is the lock point. |
| **F** (presenter deck) | Presenter deck is markdown-only; revert is `git revert <commit>`. No code, no anchors. | 0. |

**Per-wave rollback validation.** Each rollback must end with `bash scripts/verify_anchors.sh` showing 26 PASS + 2 pre-existing glob-collision FAIL (the baseline carried forward from `v25-tcn-horizon-bump-or-retire`). If a rollback flips the 28-original anchor count, the rollback is incomplete and a bisect-to-find-the-leak follows.

---

## §7 — Anchor gate baseline (captured at architect M-T1 spawn, 2026-05-21)

```
$ bash scripts/verify_anchors.sh 2>&1 | grep -c '^PASS'
26
$ bash scripts/verify_anchors.sh 2>&1 | grep -c '^FAIL'
2
$ bash scripts/verify_anchors.sh 2>&1 | tail -1
ANCHORS FAIL  (mismatches detected; route HANDOFF -> developer with body diff)
```

> **Architect verification (2026-05-21).** The 2 FAILs are **resolver-glob collisions, not body-SHA mutations**.
>
> - `forecast-distribution-bs1-realdata` expected SHA `ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54` — the intended report at `spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs1-realdata-20260519.md` hashes to **exactly that SHA**. Verified via direct hash:
>   ```
>   $ python3 scripts/hash_report.py spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs1-realdata-20260519.md
>   ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54  ...
>   ```
> - `forecast-distribution-bs2-realdata` expected SHA `d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06` — the intended report at `spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs2-realdata-20260519.md` hashes to **exactly that SHA**. Verified.
>
> The `scripts/verify_anchors.sh` glob `<scenario>-*.md | sort | tail -1` picks the lexicographically-later `...-realdata-recalibrated-20260521.md` (which is itself separately anchored under a different name) instead of the original `...-realdata-20260519.md`. **All 28 body-SHAs are byte-immutable.** The 2 FAILs are a known resolver-glob bug inherited verbatim from `v25-tcn-horizon-bump-or-retire` (see that feature's `tasks.md § Anchor gate baseline`), pre-existing this M-T1 tick, with no code mutation between the analyst's 2026-05-21 capture and this architect's 2026-05-21 capture.
>
> **Hence: this `verify_anchors.sh` output is the equivalent of `ANCHORS PASS (28 / 28)` for the body-SHA-immutability invariant.** The architect proceeds. A separate spec-auditor item is queued for the resolver-glob fix (out-of-scope here — touching `scripts/verify_anchors.sh` would risk changing anchor-validation semantics across all in-flight features and requires its own ADR per CLAUDE.md non-negotiables).

---

## §8 — Test surface (architect spec; developer implements)

> Per `feature.md § R10` + § Verification gates. Architect lists test names + asserts; developer ships the actual files at Wave A.5.

| Test file | Wave | Assertion |
|-----------|------|-----------|
| `crates/forecast/tests/sigma_train_not_in_safetensors_patchtst.rs` | A.5 / E.3 | ADR-0035 § D4: safetensors header contains no tensor named `sigma` / `output_scale` / `sigma_train`. |
| `crates/forecast/tests/forward_determinism_patchtst.rs` | A.5 / E.5 | K2: 2 forward passes on the same fixed-seed input return byte-identical outputs (CPU). Metal-vs-CPU delta < 1e-4 per ADR-0029 § 4. |
| `crates/forecast/tests/tcn_byte_identity.rs` | A.5 / E.6 | K6: `git diff HEAD -- crates/forecast/src/tcn.rs` exit 0; same for the 8 anchored TCN checkpoint files. |
| `crates/forecast/tests/patchtst_overlay_neutrality.rs` | A.5 / E.7 | K4: re-running `top10-2023-fy-tcn-overlay-realdata` produces body bytes hashing to `8fa47f49…`. |
| `crates/forecast/tests/forecast_distribution_verdict.rs` (EXTEND) | E.4 | ADR-0033 § D3 IMMUTABLE: the F-verdict classifier produces F1/F2/F3/F4 on the existing TCN cases + the new PatchTST case. |
| Inline `#[test]` blocks in `crates/forecast/src/patchtst.rs::tests` | A.1-A.8 | Shape tests for `PatchEmbed`, `LearnablePositionEncoding`, `MultiHeadSelfAttention`, `TransformerBlock`, `PatchTstModel`, `PatchTstForecaster`, parameter-count assertion. |

---

## §9 — Architect's residual decisions for developer

1. **Position encoding choice** — architect locks **learnable** (Nie et al ETT default) over sinusoidal. Developer does not re-decide.
2. **Attention block source** — architect locks **custom MultiHeadSelfAttention (~50 LoC)** over `candle_transformers::*` per ADR-0036 § D5 (smaller surface for K2 determinism test). Developer does not re-decide unless K2 fails at T-D-N4.
3. **Channel-independence implementation** — architect locks the **reshape pattern** `[batch, channels, n_patches, d_model] → [batch * channels, n_patches, d_model]` (Nie et al § 3.2) over the per-channel-loop pattern. Developer implements via `Tensor::reshape` not Python-style for-loops.
4. **σ_train timing** — architect locks **post-training frozen-weights forward pass over the training-data span** per ADR-0035 § D1 + ADR-0036 § D3. **No** in-loop accumulator. Developer code-reviews `train_patchtst.rs` against this contract before opening the M-D handoff.
5. **2-run byte-identity verification** — developer verifies for **all 3 new reports** (forecast-distribution, top10-backtest, sharpe-comparison) before opening the M-D handoff. Tester re-verifies at M-FINAL.

---

## §10 — Cross-references

- `spec/v25a-patchtst-overlay/feature.md` (analyst brief; R1-R10 / H1-H4 / K1-K6 / Q1-Q8 + § Acceptance per milestone)
- `spec/v25a-patchtst-overlay/tasks.md` (this M-T1 fills the T-AR rows; T-D-N1..T-D-N26 + T-T-1.a..T-T-1.o are the M-D / M-FINAL targets)
- `spec/architecture/adr/0036-patchtst-training-contract.md` (this M-T1's sibling ADR; D1-D7)
- `spec/architecture/adr/0028-v25-dl-forecast-overlay-candle.md` (candle ML framework; covers all 4 phases)
- `spec/architecture/adr/0029-tcn-checkpoint-provenance.md` (canonical-arch descriptor — extended additively per ADR-0036 § D2)
- `spec/architecture/adr/0032-backtest-realdata-path-and-revision-pin.md` (realdata path inherits)
- `spec/architecture/adr/0033-tcn-alpha-investigation-report-shape.md` (F-verdict algorithm IMMUTABLE — PatchTST reports use same priority tree)
- `spec/architecture/adr/0034-cockpit-training-control.md` (train_events emission)
- `spec/architecture/adr/0035-tcn-sigma-train-recalibration.md` (§ D1 σ_train post-training pattern — cross-phase; applies to PatchTST)
- `crates/forecast/src/tcn.rs` (TCN mirror source — byte-immutable through this ship)
- `crates/forecast/src/bin/train_tcn.rs` (TCN training scaffold — mirror source; the `:606,676-678,733-741` in-loop accumulator is the **deprecated negative precedent** per ADR-0035 § D1)
- `crates/strategy/src/tcn_overlay_momentum.rs:466-624` (sibling strategy mirror source)
- `crates/backtest/src/scenarios/tcn_overlay_weights.rs` (backtest scenario mirror source)
- `crates/forecast/src/bin/forecast_distribution.rs` (alpha-investigation bin — extended additively at D.1)
- `crates/forecast/src/bin/sharpe_comparison.rs` (sharpe-comparison bin — extended additively at D.6)

---

## Changelog

- 2026-05-21 (architect, M-T1): initial decomp landed alongside ADR-0036. Closes T-AR-1..T-AR-8 of `tasks.md`. Wave A-F decomposition + parallelism map + rollback shapes + anchor gate baseline (clean, modulo 2 known pre-existing glob-collision FAILs inherited from v25-tcn-horizon-bump-or-retire). Owner flips `architect → developer`. Status flips `proposed → in-progress`. Estimated cost: ~3-5 weeks best case; ~5-7 weeks with one Wave B retry.
