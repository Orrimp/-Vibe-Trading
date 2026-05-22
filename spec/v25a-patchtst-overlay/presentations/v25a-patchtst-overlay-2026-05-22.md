---
slug: v25a-patchtst-overlay
mode: release
status: draft
audience: human-operator
updated: 2026-05-22
generated: 2026-05-22T00:00:00Z
version: 0.1.0
commit: a0dee41ebc1352891518cd8ac11d6826cd8992de
predecessor: v25-tcn-threshold-tuning v0.1.0 (shipped 2026-05-21)
parent: v25-dl-forecast-overlay v0.0.0 (roadmap, phase 2 of 4)
---

# v2.5a PatchTST overlay — release deck

## Operator headline

PatchTST trained cleanly on the first try — 30 epochs, 7h 45min wall-clock
(architect estimated 3-5 days), final train_loss **2.6e-5** (67× lower than
epoch 1), σ_train derived **post-training** via the ADR-0035 § D1
frozen-weights forward-pass pattern at **0.007053** (inside the expected
0.005-0.025 range; **no inflation artifact** of the kind that contaminated
v2.5 TCN). The infrastructure works. **But the model F-verdicts at F4 and
the Sharpe-delta vs v1 momentum baseline is only +0.006144 — LOWER than
the retired v2.5 TCN's +0.018 / +0.045 at 1h horizon.** The horizon-bump
hypothesis (H1: 24h would unlock signal that 1h missed) is **falsified**.
Joint evidence across 3 checkpoints and 2 model families now stands at
**F4 / F4 / F4**. v2.5-era DL approaches do not extract the +0.10
Sharpe-delta target on this data / overlay shape. The deck's primary ask
is a multi-week routing decision: (a) retire the entire 4-phase DL
overlay project, (b) continue to v2.5b decoder Transformer for a third
architecture-family try, (c) strategy-side pivot (volatility / regime /
longer-horizon trend), or (d) ship advisory + queue PatchTST BS-2 v0.1.1
sanity check. Standing "Autoapprove all" does NOT apply — this is a
strategic budget decision.

## What landed

- **New PatchTST model** at
  [`crates/forecast/src/patchtst.rs`](../../../crates/forecast/src/patchtst.rs)
  (~800 LoC) — patch embedding (`patch_len=16`, `stride=8`), learnable
  positional encoding (`[n_patches=41, d_model=128]`), custom
  `MultiHeadSelfAttention` block (~50 LoC; avoids
  `candle_transformers::*` API drift per ADR-0036 § D5;
  `.contiguous()?` after transpose fixes candle's
  `MatMulUnexpectedStriding`), channel-independent reshape
  `[batch, channels, n_patches, d_model]` → `[batch*channels, n_patches, d_model]`
  per Nie et al § 3.2 (ADR-0036 § D1). Parameter count
  **431,105** — ~10× smaller than v2.5 TCN (4.4M), well below the
  ADR-0028 5-10M ceiling.
- **Training scaffold** at
  [`crates/forecast/src/bin/train_patchtst.rs`](../../../crates/forecast/src/bin/train_patchtst.rs)
  (~550 LoC) — AdamW + OneCycle + Huber loss matching TCN; σ_train
  computed via `compute_sigma_train_post_training()` **after** the
  training loop with frozen weights (ADR-0035 § D1, ADR-0036 § D3).
  The `Vec<f32>` accumulator lives INSIDE the function scope — not
  declared outside the per-epoch loop, in deliberate contrast to the
  deprecated [`train_tcn.rs:606,676-678`](../../../crates/forecast/src/bin/train_tcn.rs)
  pattern. Cost tripwire `assert_epoch_budget(...)` — hard 24h limit
  + 3× rolling-median escalation per ADR-0036 § D4.
- **`forecast_distribution` bin extended additively** at
  [`crates/forecast/src/bin/forecast_distribution.rs`](../../../crates/forecast/src/bin/forecast_distribution.rs)
  — new `PatchtstBs1` enum variant + `CheckpointHandle` dispatch.
  Existing TCN dispatch byte-identical (K6 invariant; tester confirmed
  empty `git diff` on `tcn.rs`).
- **Sibling strategy** at
  [`crates/strategy/src/patchtst_overlay_momentum.rs`](../../../crates/strategy/src/patchtst_overlay_momentum.rs)
  (~320 LoC) + [`crates/strategy/src/patchtst_sync.rs`](../../../crates/strategy/src/patchtst_sync.rs) —
  `with_patchtst_bs1(base)` / `with_patchtst_bs1_ledger(base, ledger)` /
  `..._tuned(...)` / `..._ledger_tuned(...)` mirror the
  `with_tcn_bs{1,2}*` shape. **Additive only**; the 8 existing TCN
  builders are byte-identical (V-R8 verified).
- **Backtest scenario** at
  [`crates/backtest/src/scenarios/patchtst_overlay_weights.rs`](../../../crates/backtest/src/scenarios/patchtst_overlay_weights.rs)
  + `PatchtstOverlayMomentumWeights` variant in
  [`crates/backtest/src/main.rs`](../../../crates/backtest/src/main.rs)
  with `report_dir_for_scenario` mapping to
  `spec/v25a-patchtst-overlay/reports/`.
- **`sharpe_comparison` extended** at
  [`crates/forecast/src/bin/sharpe_comparison.rs`](../../../crates/forecast/src/bin/sharpe_comparison.rs)
  — `SCENARIOS[5]`, `render_report(&[RerunResult;5])`, PatchTST verdict
  row, filename `sharpe-comparison-patchtst-bs1-realdata-*.md`.
- **ADR-0036** at
  [`spec/architecture/adr/0036-patchtst-training-contract.md`](../../architecture/adr/0036-patchtst-training-contract.md)
  — codifies (D1) patch-embed + pre-LN encoder + projection head
  skeleton; (D2) canonical-arch descriptor extension; (D3) σ_train
  post-training pattern referencing ADR-0035 § D1; (D4) cost tripwire;
  (D5) candle-attention determinism gate.
- **4 new test files** under `crates/forecast/tests/` —
  `forward_determinism_patchtst.rs` (2 tests),
  `sigma_train_not_in_safetensors_patchtst.rs` (1 + 1 ignored),
  `tcn_byte_identity.rs` (K6 scope-creep guard, 1 test),
  `patchtst_overlay_neutrality.rs` (K4 anchor-neutrality, 1 test
  `#[ignore]`d; tester fix wired `--bin backtest`).
- **2 new anchor rows** under `v2.5a.0-patchtst` at
  [`spec/anchors.toml:230,235`](../../anchors.toml) — anchor count
  progression **28 → 30**.
- **Trained checkpoint** at
  `crates/forecast/checkpoints/anchors/patchtst-bs1-62520db9*.safetensors`
  (1.6 MB) + `.metadata.json` (911 B). Model revision SHA
  `62520db92f68c1d323f0782bc367c742cf9439631106ddc0fd492188f6d1cd4d`.

## Architect resolutions (Q1-Q8 — autoapproved at M-OD)

Operator approved the analyst-recommended bundle via "Autoapprove all"
at the M-OD gate; documented here for visibility.

| Q | Choice | Resolution |
|---|---|---|
| Q1 | (a) | PatchTST per Nie et al 2022 (NOT iTransformer / hybrid) |
| Q2 | (a) | Full MVP — code + BS-1 checkpoint + F-verdict + Sharpe + sibling strategy + 2 anchors |
| Q3 | (a) | `patch_len=16, stride=8` (Nie ETT default; 50% overlap) |
| Q4 | (b) | 24h horizon; overlapping targets (sub-Q4a) — ~87k samples |
| Q5 | (c) | Carry-forward 5-feature input (`logret/logrange/logvol_z/hour_sin/hour_cos`) |
| Q6 | (a) | BS-1 2023-01-01..2023-12-31 train span |
| Q7 | (a) | Anchor under version `v2.5a.0-patchtst` |
| Q8 | (a) | Sibling strategy `patchtst_overlay_momentum.rs` (NOT model-agnostic refactor) |

**Topology lock** (ADR-0036 § D1):
`patch_len=16, stride=8, d_model=128, n_heads=4, d_ff=256, n_layers=3, dropout=0.2, context_len=336, target_horizon_bars=24`.

## What you can do now

| Action | Command |
|--------|---------|
| Re-verify all 30 anchors (28 originals + 2 PatchTST) | `bash scripts/verify_anchors.sh` |
| Re-run BS-1 forecast distribution (PatchTST F-verdict bin) | `cargo run -p forecast --release --features candle --bin forecast_distribution -- --scenario patchtst-bs1 --out-dir spec/v25a-patchtst-overlay/reports/` |
| Re-run BS-1 PatchTST backtest (real Binance hourly OHLCV) | `cargo run -p backtest --release --features candle,realdata --bin backtest -- --scenario top10-2023-fy-patchtst-overlay-realdata` |
| Re-run Sharpe-comparison (5-scenario summary) | `cargo run -p forecast --release --features candle --bin sharpe_comparison -- --out-dir spec/v25a-patchtst-overlay/reports/` |
| Adopt PatchTST overlay (advisory, additive builder) | construct `MomentumStrategy::with_patchtst_bs1_ledger(base, ledger)` in the trading host |
| Approve and pick routing | tick a box below; orchestrator opens / closes the picked path |

## Live demo

### Anchor gate — 30 PASS / 30 (verbatim tail)

```
$ bash scripts/verify_anchors.sh 2>&1 | tail -5
PASS  threshold-sweep-bs2-realdata-recalibrated  755bc3801359f1995cf4535215467995df00aeb90c93e695c16750b8c54486c3
PASS  forecast-distribution-patchtst-bs1-realdata  c55c6c5178374f230f5273df1e20d121589ff0b879c20062ee6cbdca7f4646dd
PASS  top10-2023-fy-patchtst-overlay-realdata  5f303cc0812d421e6efdc40c0f412dd8cc0625891c677442bf2d7d2d5336ab4c
---
ANCHORS PASS  (30 / 30)
```

28 originals byte-identical; 2 new PatchTST rows under `v2.5a.0-patchtst`
locked. The pre-existing glob-collision baseline (2 known-FAIL) is now
0 — `verify_anchors.sh` was hardened at commit `09fb962` per the
predecessor's spec-auditor punch-list.

### F-verdict (from `forecast-distribution-patchtst-bs1-realdata-20260521.md`, body-SHA `c55c6c51…`)

```
| F-verdict        | F4 |
| sigma_train      | 0.007053 |
| epsilon          | 0.000500 |
| tau              | 0.600000 |
| frac_passes_confidence_gate | 0.557942688 |
| frac_inside_epsilon         | 0.054883 |
| std/sigma_train ratio       | 1.000 |
| abs_p95          | 0.014274921 |
| Inferences       | 76,800 (10 symbols × 7,680 windows) |
| Wall clock       | 404.8s |
```

**Interpretation.** `std/sigma_train = 1.000` confirms the model is
correctly calibrated — **no σ_train inflation artifact** of the kind
that contaminated v2.5 TCN at first ship (608× / 580× inflation
eliminated only by the recalibrate ship). The gate-survival rate of
55.8% at τ=0.6 shows PatchTST emits high-confidence predictions
(it is not collapsing to mid-range outputs). But
`frac_inside_epsilon = 0.054883 << 0.5` is the load-bearing F4
trigger: only 5.5% of predictions fall inside the ε=0.0005 deadband,
meaning the model **does predict non-trivial magnitudes** — they are
just systematically wrong as **directional** signals at 24h horizon.

### Sharpe-delta (from `sharpe-comparison-patchtst-bs1-realdata-20260521.md`, body-SHA `45140833…`)

```
| Sharpe delta (PatchTST) | 0.006144 (passthrough-2023 vs. patchtst-bs1-2023) |
| Conclusion              | TCN and PatchTST at v2.5a produce no alpha lift
                            over the v1 momentum baseline. PatchTST F-verdict: F4 |
```

## The headline numbers — joint F4 / F4 / F4

| Ship | Model | Horizon | Sharpe-delta | F-verdict | T-classifier |
|------|-------|---------|-------------:|-----------|--------------|
| v25-tcn-threshold-tuning BS-1 | TCN (dilated conv) | 1h | +0.018 | F4 | T-MARGINAL |
| v25-tcn-threshold-tuning BS-2 | TCN (dilated conv) | 1h | +0.045 | F4 | T-MARGINAL |
| **v25a-patchtst-overlay BS-1** | **PatchTST (patch-attention)** | **24h** | **+0.006144** | **F4** | **T-MARGINAL** |

**Joint verdict: F4 / F4 / F4 across 2 model families and 2 horizons.**

### The equity-curve footnote (honest reading)

PatchTST BS-1 has the **highest 2023-FY total return** (+31.13%) — vs the
v1 passthrough baseline's +13.48%, a +17.65 pp absolute-return lift. **But
this is a fee-reduction effect, not alpha.** The 28.96% dampen rate cuts
trades 6203 → 3187 and saves ~$5.8k in fees on a $100k notional. Max
drawdown is **worse** (77.97% vs 73.73%, +4.24 pp). On a risk-adjusted
basis (Sharpe-delta +0.006144) PatchTST is **the worst of the three
F4s** — TCN BS-1 at +0.018 and TCN BS-2 at +0.045 both clear it.
**Honest summary: PatchTST does not improve risk-adjusted returns; it
just trades less.**

### Why F4 with strong calibration?

PatchTST's confidence gate selectively suppresses trades (this is what
brings the dampen rate to 28.96% at τ=0.6) but is **not picking the
right ones to suppress** — the gate suppresses some winners and admits
some losers in roughly fair proportion. Per ADR-0033 § D3:

> F4 — `frac_passes_confidence_gate ≥ 1e-6 AND frac_inside_epsilon ≤ 0.5`
> ⇒ model emits non-trivial-magnitude predictions but they are not
> directionally informative.

This is fully consistent with the joint v2.5 TCN evidence: every
checkpoint tested on this overlay shape at hourly cadence produces
non-trivial predictions that do not generalise to directional signal at
the operator's chosen confidence stratum.

## Hypothesis register status

| H | Statement | Status | Evidence |
|---|---|---|---|
| **H1** | PatchTST extracts more directional signal than TCN at the chosen horizon (24h). Specifically: F-verdict ≠ F4 AND a (τ, ε) cell unlocks ≥ +0.10 Sharpe-delta. | **FALSIFIED** | F-verdict = F4. Sharpe-delta = +0.006144 < TCN at 1h (+0.018 / +0.045) < +0.10 unlock floor. |
| **H2** | PatchTST attention captures intraday-session structure TCN's conv missed. | **INCONCLUSIVE** | F4 verdict at the operator-decision level; the per-hour-of-week side-artifact was not load-bearing and is deferred to a v0.1.1 diagnostic if H1 had passed. |
| **H3** | ~4-6 weeks scope is feasible on Apple Silicon Metal. | **CONFIRMED — substantially under-budget** | Actual <1 day end-to-end (Wave A + Wave B 7h 45min + Wave D + tester gate). Architect estimated 3-5 weeks best case; reality was overnight. |
| **H4** | σ_train post-training derivation pattern works for PatchTST. | **CONFIRMED** | σ_train = 0.007053, in expected 0.005-0.025 range; `std/sigma_train = 1.000` ratio confirms no inflation artifact. ADR-0035 § D1 pattern carries cleanly to a new model family on the first try. |

H1 is the load-bearing falsification: the joint H1 + H4-equivalent
horizon hypothesis from `v25-tcn-horizon-bump-or-retire` is now
**falsified twice** — once by the operator's retirement decision (1h
TCN exhausted) and once by PatchTST at 24h failing to clear it.

## Implications for the 4-phase DL roadmap

Three checkpoints × two model families × two horizons = **F4 / F4 / F4**.
The joint evidence is now load-bearing for the v2.6 bake-off
retirement gate that was originally designed as the canonical closure.

- **Convolutional inductive bias (TCN) — F4 at 1h.** Dilated causal
  convolutions across a 1021-bar receptive field exhaust the
  hypothesis space on this data.
- **Patch-attention inductive bias (PatchTST) — F4 at 24h.**
  Channel-independent transformer encoder over patch tokens, ETT
  benchmark hyperparameters, on the same overlay shape, F4s
  **and** under-performs the retired model.
- **Vanilla decoder-only Transformer (v2.5b, phase 3)** — UNTESTED,
  but the **prior probability has dropped significantly** given two
  orthogonal architectural paradigms both F4 on the same data /
  overlay composition / signal-extraction task. Continuing to phase 3
  would commit ~3-5 weeks of compute to test a third architecture
  family where two precedents converge on no-signal.
- **v2.6 bake-off (phase 4)** — was designed as the canonical
  retirement gate. The arguable case is that the gate can **fire
  now** with high confidence rather than spending v2.5b's budget to
  collect a third F4 data point.

## Operator routing options (the deck's primary ask)

**One decision. Multi-week budget implications. Standing "Autoapprove
all" does NOT apply.**

- **(a) Retire entire 4-phase DL forecast overlay project.** F4 across
  both convolutional + patch-attention paradigms; the prior on a
  third paradigm passing is low. Skip v2.5b decoder Transformer
  (~3-5 weeks compute) and v2.6 bake-off (the bake-off's purpose is
  retirement gating; it would gate on three F4s and reach the same
  conclusion). Pivot research budget to strategy-side directions:
  regime classification, volatility forecasting, longer-horizon trend
  signals. **Highest-EV given joint evidence.** Preserves the
  candle + PatchTST + cockpit-training-control infrastructure for
  future use.
- **(b) Continue to v2.5b vanilla decoder Transformer.** Give a third
  architecture-family one more chance before retirement. ~3-5 weeks
  Apple Silicon Metal compute commitment plus tester / presenter
  overhead. **Low EV** — two F4 precedents on the same data /
  overlay shape make this a "complete the experiment" rather than a
  "discover new signal" commitment.
- **(c) Strategy-side pivot.** Accept that **1h / 24h log-return
  point prediction is the wrong task**. Reformulate as one of:
  (i) volatility forecasting (predict σ of next-N-bar window — the
  ML target is now a positive scalar with structural autocorrelation
  in crypto vol); (ii) regime classification (low-vol / trend /
  high-vol / mean-revert — categorical target); (iii) longer-horizon
  trend (7-14 day signal — different SNR regime entirely). This is
  an analyst / architect re-scope, not a model-family swap. Preserves
  the infrastructure; commits to a substantively different task
  formulation. **Highest-upside if the operator believes signal
  exists but is being asked the wrong question.**
- **(d) Ship advisory + queue PatchTST BS-2 v0.1.1 sanity check.**
  Bank the F-verdict triple as a documented dead-end and queue a
  minimal "did we get unlucky with BS-1" check: train one BS-2
  PatchTST checkpoint (~1-2 days wall-clock at the actual Wave-B
  rate of 7h 45min per model) and confirm the F4 verdict
  generalises across train spans. **Lower EV than (a) / (c)** — the
  joint F4 evidence is already strong; BS-2 is sanity, not new
  information.

### Analyst recommendation

**(a) retire** is the conservative high-EV path. **(c) strategy pivot**
is the high-upside path if the operator believes there is signal in this
data that the 1h / 24h log-return point-prediction framing misses.
The two are not mutually exclusive — (a) frees the compute budget that
(c) needs.

## Test results — verbatim from tester report

From [`reports/test-final-2026-05-22.md`](../reports/test-final-2026-05-22.md)
(initial `VERDICT → FAIL` on T-F12 test-harness defect; orchestrator
landed the 1-line fix inline; K4 re-run PASS).

| Gate | Result | Evidence |
|------|--------|----------|
| T-F1 — `cargo fmt --check` | PASS | No diff. |
| T-F2 — `cargo clippy --workspace -- -D warnings` | PASS | `Finished … in 1.00s` |
| T-F3 — `cargo clippy -p forecast --features candle -- -D warnings` | PASS | `Finished … in 0.23s` |
| T-F4 — `cargo clippy -p backtest --features "candle realdata" -- -D warnings` | PASS | `Finished … in 0.24s` |
| T-F4 — `cargo clippy -p strategy --features forecast,forecast-audit-tick -- -D warnings` | PASS | `Finished … in 3.09s` |
| T-F5 — `cargo test --workspace --lib` | PASS | **311 passed, 0 failed** (0.54s) |
| T-F6a — `forward_determinism_patchtst` (K2) | PASS | 2 passed (0.45s) |
| T-F6b — `sigma_train_not_in_safetensors_patchtst` (ADR-0035 § D4) | PASS | 1 passed, 1 ignored |
| T-F6c — `tcn_byte_identity` (K6 scope-creep guard) | PASS | 1 passed (1.19s) |
| T-F6d — `forecast_distribution_verdict` (ADR-0033 § D3) | PASS | 8 passed (`sharpe_comparison` bin unit tests including F-verdict) |
| T-F7 — Benchmark smoke (`reflection::trail_mirror`) | PASS | compiles + lists |
| T-F8 — 2-run byte-identity on 3 new reports | PASS | SHAs `c55c6c51…` / `5f303cc0…` / `45140833…` match developer record |
| T-F9 — Anchor gate (PRE / POST) | PASS | 28 PASS PRE / **30 PASS POST** (28 originals + 2 new) |
| T-F10 — TCN files byte-identical | PASS | `git diff` empty on 8 TCN checkpoint files |
| T-F11 — `spec-lint` | PASS-WITH-PREEXISTING-DEBT | 86/3 (0 new categories from this feature; total down from 87) |
| T-F12 / T-T-1.i — `patchtst_overlay_neutrality` (K4) | **PASS after harness fix** | scenario body-SHA `8fa47f49…` matches anchored TCN value; PatchTST overlay introduces zero regression on the existing TCN scenario |

**Joint advisory verdict (per ADR-0033 § D3.c, single-checkpoint
BS-1-only ship per Q2=(a)):**
> **F4 — no predictive signal at 24h horizon.** Sharpe-delta +0.006144
> below T-ALPHA-UNLOCKED +0.10. PatchTST does not improve risk-adjusted
> returns vs v1 momentum baseline. The standalone routing under R5 is
> F4 → analyst spawn for v2.6 bake-off retirement gate. At the joint
> 3-checkpoint level (TCN BS-1 + TCN BS-2 + PatchTST BS-1 all F4), the
> routing expands to the 4-way operator-decide above.

## Verification matrix (R10 gates)

| V-id | R10 gate | Status | Evidence |
|------|----------|--------|----------|
| V-1 | `cargo fmt --check` + workspace clippy | VERIFIED | T-F1 + T-F2 PASS |
| V-2 | `cargo clippy -p forecast --features candle` | VERIFIED | T-F3 PASS |
| V-3 | `cargo test --workspace --lib` | VERIFIED | 311 / 311 |
| V-4 | `sigma_train_not_in_safetensors_patchtst` (ADR-0035 § D4) | VERIFIED | T-F6b PASS |
| V-5 | `forecast_distribution_verdict` (ADR-0033 § D3 immutable) | VERIFIED | T-F6d PASS (8 unit tests including F-verdict algorithm) |
| V-6 | 2-run byte-identity — `forecast-distribution-patchtst-bs1-realdata-*` | VERIFIED | SHA `c55c6c51…` matches across runs |
| V-7 | 2-run byte-identity — `top10-2023-fy-patchtst-overlay-realdata-*` | VERIFIED | SHA `5f303cc0…` matches |
| V-8 | `verify_anchors.sh` 28 PRE / 30 POST | VERIFIED | `ANCHORS PASS (30 / 30)` |
| V-9 | `spec_lint.py` baseline (no new categories) | VERIFIED | 86/3 (down from 87/2 prior); `shipped-no-tests` is pre-existing debt from `v25-tcn-horizon-bump-or-retire` ship, not this feature |
| V-10 | Joint advisory verdict recorded in `feature.md § Verification` | VERIFIED | See § Test results above |

## Numbers that matter

- **F-verdict** — **F4** on the only checkpoint (BS-1). Per ADR-0033 § D3
  immutable.
- **Sharpe-delta vs v1 momentum** — **+0.006144** (PatchTST BS-1 vs
  passthrough-2023). Below +0.10 T-ALPHA-UNLOCKED by **-0.094**.
- **Total return** — **+31.13%** (PatchTST BS-1) vs +13.48% (v1
  passthrough). +17.65 pp lift — fee-reduction effect (28.96% dampen
  rate cuts trades 6203 → 3187), NOT alpha.
- **Max drawdown** — **77.97%** (PatchTST) vs 73.73% (v1 passthrough).
  +4.24 pp **worse** under PatchTST overlay.
- **σ_train (canonical, post-training, ADR-0035 § D1)** — **0.007053**;
  inside expected 0.005-0.025 range; `std/σ_train = 1.000` (perfectly
  calibrated; **no inflation artifact**).
- **Training stats** — 30 epochs, final train_loss **2.6e-5** (67×
  lower than epoch 1), **7h 45min wall-clock** Apple Silicon Metal
  (vs architect's 3-5 day estimate — substantially under-budget).
- **Model size** — 431,105 parameters, ~10× smaller than TCN (4.4M),
  well below 5-10M ceiling. `safetensors` 1.6 MB; `metadata.json`
  911 B.
- **Checkpoint SHA** —
  `62520db92f68c1d323f0782bc367c742cf9439631106ddc0fd492188f6d1cd4d`.
- **Tests** — 311 (workspace lib) + 2 (`forward_determinism_patchtst`)
  + 1 (`sigma_train_not_in_safetensors_patchtst`; +1 ignored) + 1
  (`tcn_byte_identity`) + 1 (`patchtst_overlay_neutrality` — re-runs
  PASS after harness fix) + 8 (`sharpe_comparison` bin unit tests) =
  **324 total, 0 failures after K4 fix**.
- **Anchors** — 28 → **30**. 2 new under `v2.5a.0-patchtst`:
  - `forecast-distribution-patchtst-bs1-realdata` →
    `c55c6c5178374f230f5273df1e20d121589ff0b879c20062ee6cbdca7f4646dd`
  - `top10-2023-fy-patchtst-overlay-realdata` →
    `5f303cc0812d421e6efdc40c0f412dd8cc0625891c677442bf2d7d2d5336ab4c`
- **Spec-lint** — 86 violations / 3 categories. **0 new categories or
  count growth from this feature.** Total down from 87 (prior tester
  baseline). `shipped-no-tests` (1) is pre-existing debt on
  `v25-tcn-horizon-bump-or-retire`; `dead-link` (81) is dominated by
  stale roadmap / template paths from older features;
  `trace-broken-path` (4) is v2.5b + v2.6 stub anchors not yet locked.

## Open decisions

**One load-bearing decision** — pick (a) / (b) / (c) / (d) above. The
decision routes a multi-week budget:

- (a) Retire 4-phase DL project → frees ~3-5 weeks compute; pivot to
  alternative research direction (operator selects).
- (b) Continue v2.5b decoder Transformer → commits ~3-5 weeks compute
  to a third architecture family with low prior on success.
- (c) Strategy-side task pivot → commits ~1-2 weeks analyst + architect
  for re-scope, preserves infrastructure, opens new task formulation
  (volatility / regime / longer-horizon trend).
- (d) Ship advisory + queue PatchTST BS-2 v0.1.1 sanity → ~1-2 weeks
  compute for a redundant data point.

## Deferred / out of scope (v0.1.1+)

- **BS-2 PatchTST checkpoint** — deferred to v0.1.1 if operator picks
  (d). H1 falsification on BS-1 alone was sufficient to route to the
  joint-verdict gate; BS-2 is sanity-not-information.
- **Hyperparameter sweep** (`patch_len` / `stride` / `d_model` /
  `n_heads` / `dropout`) — deferred. H1 falsified on the
  analyst-recommended defaults; sweeping is unlikely to recover
  +0.10 Sharpe-delta given two paradigms have F4'd. If operator
  picks (b), v2.5b's analyst should consider whether a defaults
  sweep changes the F4 prior.
- **Walk-forward retraining** — deferred to v2.6 bake-off if it
  ships. Out of scope at MVP per Q6 = (a).
- **Extended feature set** (realized-vol bands / funding-rate /
  open-interest proxies) — deferred per Q5 = (c). May be a primary
  lever if operator picks (c) strategy pivot.
- **`sharpe_comparison` `$0.00 final_equity` parse-report-equity
  cosmetic bug** — pre-existing (not introduced by this feature).
  Queued as a separate hygiene patch. Not blocking.
- **iTransformer / hybrid ensemble** — Q1 alternatives parked per
  analyst rejection. Not revisited under current routing.

## Rollback

This feature is **additive only**.

| Wave | Rollback action | Cost |
|------|-----------------|------|
| A (`patchtst.rs` + `train_patchtst.rs` + 4 tests) | `git revert <wave-A-shas>`. K6 invariant holds the whole time — `tcn.rs` untouched. | ~1 minute |
| B (BS-1 training run + checkpoint files) | `rm crates/forecast/checkpoints/anchors/patchtst-bs1-*` (1.6 MB + 911 B). | ~10 seconds |
| D (forecast_distribution dispatch + strategy sibling + backtest scenario + sharpe_comparison extension + reports) | `git revert <wave-D-shas>` + `rm` the 3 report artifacts. Placeholder routes restorable. | ~5 minutes |
| Anchor lock | revert the 2 new rows at `spec/anchors.toml:230,235`. 28 originals stay byte-identical. | ~1 minute |
| ADR-0036 | `git revert` the ADR commit. | ~30 seconds |
| Full feature | `git revert` the wave commits + `rm` the 3 report files + 2 checkpoint files. | ~10 minutes total |

The non-negotiable safety net: **28 original anchors byte-identical**
(V-8 confirmed `30 / 30 PASS` POST, of which the 28 originals match
their pre-feature SHAs). **8 TCN checkpoint files byte-identical**
(V verified by `git diff HEAD` empty). **`crates/forecast/src/tcn.rs`
byte-identical** (V-K6 confirmed). **8 existing TCN strategy builders
byte-identical** (V-R8 confirmed; new PatchTST builders are additive).
Rollback never touches a locked artifact.

## Closing gates

Both mechanical gates run on the file just written:

```
$ bash scripts/check_presentation.sh spec/v25a-patchtst-overlay/presentations/v25a-patchtst-overlay-2026-05-22.md
<PASS line appears in presenter handoff envelope below>
```

```
$ uv run scripts/spec_lint.py 2>&1 | head -1
spec-lint: FAIL (86 violations in 3 categories)
```

Baseline match: 86 / 3 (per tester report § 8 + this presenter's own
re-run above). **No new categories or count growth introduced by this
presentation file.** `shipped-no-tests` (1) is pre-existing debt on
`v25-tcn-horizon-bump-or-retire`; carry-forward composition matches
the tester's recorded baseline.

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

Operator decided **routing (a) — retire entire 4-phase DL forecast
overlay project** 2026-05-22 via the orchestrator's batched-Q
strategic-routing prompt. The joint F4-F4-F4 evidence across 3
checkpoints (TCN BS-1 @ 1h: +0.018; TCN BS-2 @ 1h: +0.045; PatchTST
BS-1 @ 24h: +0.006) is sufficient to conclude v2.5-era DL approaches
do not extract +0.10 Sharpe-delta alpha on the v1 cross-sectional
momentum baseline using real Binance hourly OHLCV.

**Strategic implications** (handled in a follow-on retirement commit):
- v25-dl-forecast-overlay umbrella → flipped roadmap → deprecated/terminal
- v25b-transformer-overlay Queue stub → deprecated (no expected value
  given F4-F4-F4; v2.5b would test a 3rd architecture with low prior)
- v26-forecast-bakeoff Queue stub → deprecated (nothing to bake off)
- v25-tcn-overlay parent (in-progress) → flipped to shipped/terminal
  (the v2.5 TCN journey reached its evidence-based conclusion)
- Research budget pivots from DL-forecast-overlay to strategy-side
  reformulation (e.g. volatility forecasting, regime classification,
  168h trend signal) OR to non-forecast lanes entirely

Ship v0.1.0.

## Sources cited

- [`feature.md`](../feature.md) — feature brief v0.1.0; R1-R10 + H1-H4 + K1-K6 + Q1-Q8.
- [`tasks.md`](../tasks.md) — T-D-N1..T-D-N26 + T-T-1.a..T-T-1.o.
- [`decomp.md`](../decomp.md) — architect M-T1 decomposition (Waves A / B / C / D / E / F).
- [`reports/test-final-2026-05-22.md`](../reports/test-final-2026-05-22.md) — tester M-FINAL report; initial FAIL on T-F12 harness defect; K4 re-runs PASS after inline 1-line fix.
- [`reports/forecast-distribution-patchtst-bs1-realdata-20260521.md`](../reports/forecast-distribution-patchtst-bs1-realdata-20260521.md) — F-verdict F4 report (body-SHA `c55c6c51…`).
- [`reports/backtest-20260521-220035-top10-2023-fy-patchtst-overlay-realdata.md`](../reports/backtest-20260521-220035-top10-2023-fy-patchtst-overlay-realdata.md) — backtest output, +31.13% total return, dampen rate 28.96% (body-SHA `5f303cc0…`).
- [`reports/sharpe-comparison-patchtst-bs1-realdata-20260521.md`](../reports/sharpe-comparison-patchtst-bs1-realdata-20260521.md) — Sharpe-delta +0.006144 (body-SHA `45140833…`).
- [Predecessor presenter deck 2026-05-21](../../v25-tcn-threshold-tuning/presentations/v25-tcn-threshold-tuning-2026-05-21.md) — predecessor T-MARGINAL ship; style template.
- [ADR-0036](../../architecture/adr/0036-patchtst-training-contract.md) — PatchTST training contract (this feature).
- [ADR-0035](../../architecture/adr/0035-tcn-sigma-train-recalibration.md) § D1 — σ_train post-training pattern; PatchTST applies cleanly to a new model family.
- [ADR-0033](../../architecture/adr/0033-tcn-alpha-investigation-report-shape.md) § D3 — F-verdict algorithm immutable across this feature.
- [ADR-0029](../../architecture/adr/0029-tcn-checkpoint-provenance.md) — canonical-arch descriptor extended additively for PatchTST fields.
- [ADR-0028](../../architecture/adr/0028-v25-dl-forecast-overlay-candle.md) — candle ML framework; 5-10M parameter ceiling honored.
- `spec/anchors.toml:230,235` — 2 new PatchTST anchors under `v2.5a.0-patchtst`.
- `spec/trace.toml` — `REQ-V25A-PATCHTST-001` `state` flips on operator approval below.
- Code sites:
  - [`crates/forecast/src/patchtst.rs`](../../../crates/forecast/src/patchtst.rs) — PatchTST model + ForecastProvider (~800 LoC).
  - [`crates/forecast/src/bin/train_patchtst.rs`](../../../crates/forecast/src/bin/train_patchtst.rs) — training scaffold + `compute_sigma_train_post_training()` (~550 LoC).
  - [`crates/forecast/src/bin/forecast_distribution.rs`](../../../crates/forecast/src/bin/forecast_distribution.rs) — additive `PatchtstBs1` dispatch.
  - [`crates/forecast/src/bin/sharpe_comparison.rs`](../../../crates/forecast/src/bin/sharpe_comparison.rs) — 5-scenario `render_report`.
  - [`crates/strategy/src/patchtst_overlay_momentum.rs`](../../../crates/strategy/src/patchtst_overlay_momentum.rs) — sibling strategy (~320 LoC; 7 unit tests).
  - [`crates/backtest/src/scenarios/patchtst_overlay_weights.rs`](../../../crates/backtest/src/scenarios/patchtst_overlay_weights.rs) — backtest scenario.

## Changelog

- 2026-05-22 (presenter): release deck. Joint F-verdict F4 across 3
  checkpoints + 2 model families + 2 horizons (TCN BS-1 / TCN BS-2 @ 1h;
  PatchTST BS-1 @ 24h). PatchTST Sharpe-delta +0.006144 LOWER than
  retired TCN at 1h (+0.018 / +0.045). H1 falsified — 24h horizon does
  not unlock signal that 1h missed under PatchTST patch-attention
  paradigm. H4 (σ_train post-training pattern works for PatchTST)
  confirmed — σ_train = 0.007053 in expected range, std/σ_train = 1.000.
  H3 (4-6 week scope) substantially under-budget — actual <1 day
  end-to-end vs 3-5 week best-case estimate. 4-way operator-decide
  routing surfaced: (a) retire 4-phase DL project, (b) continue to
  v2.5b decoder Transformer, (c) strategy-side pivot, (d) ship advisory
  + queue PatchTST BS-2 v0.1.1 sanity. Standing "Autoapprove all" does
  NOT apply to this routing decision. Mechanical pre-tick + spec-lint
  gates passed at baseline 86 / 3 (no new categories or count growth).
