---
adr: 0029
title: v2.5 — Forecast-checkpoint provenance schema + LFS-anchor strategy (cross-phase contract)
status: accepted
date: 2026-05-17
supersedes: none
superseded-by: none
---

# ADR-0029: Forecast-checkpoint provenance schema + LFS-anchor strategy

## Context

[ADR-0028](0028-v25-dl-forecast-overlay-candle.md) commits the v2.5
DL-forecaster slot to small custom models trained in `candle`, with four
phases planned: **v2.5** (TCN, current), **v2.5a** (PatchTST),
**v2.5b** (vanilla Transformer), and the **v2.6** bake-off retirement
decision. ADR-0028 covers the framework choice and Kronos pivot
rationale, but does NOT lock the per-phase artifacts each phase must
produce — checkpoint format, provenance hashing rules, anchor-storage
policy, determinism caveats.

The v2.5 TCN analyst pass (2026-05-17,
[`docs/archive/pre-bmad-spec/v1/v25-tcn-overlay/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/v25-tcn-overlay/feature.md))
authored a checkpoint-provenance schema (R8) and surfaced two operator-
decide questions on anchor storage (LFS-track vs regenerate-from-seed,
T-OP-1) and backtest split (one-checkpoint vs two-checkpoint, T-OP-2).
The operator answered both on 2026-05-17.

These decisions are **load-bearing across all four phases**:

- v2.5a (PatchTST) and v2.5b (vanilla Transformer) MUST emit
  checkpoints with provenance JSON that an inspector can hash to
  reproduce `model_revision` without loading the safetensors body.
  Diverging from the schema would break anchor verification across
  phases and complicate the v2.6 bake-off comparison.
- The LFS-anchor strategy is dictated by the Metal-vs-CPU determinism
  caveat (candle Metal kernels are not bit-identical to CPU). That
  caveat applies to every Metal-trained checkpoint, not just TCN.
- The strict-OOS two-checkpoint backtest split is a cross-phase
  fairness invariant: BS-1 (2023) and BS-2 (2024) each get a
  checkpoint trained strictly OOS for their evaluation period; the
  same rule applies to v2.5a/v2.5b so the v2.6 bake-off compares
  apples to apples.

Recording this at the architecture level (not just in
`docs/archive/pre-bmad-spec/v1/v25-tcn-overlay/feature.md`) gives v2.5a/v2.5b a single
citable contract.

## Decision

Lock the following as a cross-phase contract for ALL forecaster phases
under the ADR-0028 umbrella (v2.5, v2.5a, v2.5b):

### 1. Provenance JSON schema

Every forecaster checkpoint produces a `<sha>.metadata.json` sibling to
the `<sha>.safetensors` weights file. `<sha>` is SHA-256 over the
canonical bytes of the metadata JSON. The shape (TCN-instantiated; other
phases substitute `architecture` and `tokenisation` sub-objects with
their own family-appropriate keys):

```json
{
  "architecture": {"blocks": 8, "channels": 96, "kernel": 3,
                    "dilations": [1,2,4,8,16,32,64,128], "dropout": "0.1"},
  "tokenisation": {"context_bars": 256, "features": ["logret","logrange",
                    "logvol_z","hour_sin","hour_cos"]},
  "training": {"optimiser": "adamw", "lr_max": "0.001", "schedule": "onecycle",
                "batch": 128, "epochs": 30, "loss": "huber", "huber_delta": "0.001",
                "seed": 12648430},
  "data_span": {"start": "2023-01-01T00:00:00Z", "end": "2023-12-31T23:00:00Z",
                  "symbols": ["ADA","AVAX","BNB","BTC","DOGE","DOT","ETH","LINK","SOL","XRP"],
                  "interval": "1h", "source": "binance"},
  "weights_sha256": "<sha256 of the safetensors file body, hex-lowercase>",
  "sigma_train": "0.001234",
  "metrics": {"final_train_huber": "0.000312", "final_val_huber": "0.000358",
               "epochs_run": 22}
}
```

Required top-level keys: `architecture`, `tokenisation`, `training`,
`data_span`, `weights_sha256`, `sigma_train`, `metrics`. Per-phase
analyst passes may extend the `architecture` and `tokenisation`
sub-objects (e.g. PatchTST will add `patch_len`, `stride`,
`embed_dim`; vanilla Transformer will add `n_heads`, `n_layers`,
`d_model`). Other top-level keys are forbidden.

### 2. Canonicalisation rules

The metadata JSON MUST be byte-stable across operators:

- **Key ordering**: object keys sorted lexicographically (UTF-8 byte
  order), recursively.
- **Whitespace**: no whitespace between tokens. No trailing newline.
- **Numbers**: integer-valued fields (`blocks`, `channels`, `kernel`,
  `context_bars`, `batch`, `epochs`, `seed`) emit as integers without a
  decimal point. Float-valued fields (`dropout`, `lr_max`,
  `huber_delta`, `sigma_train`, all `metrics` floats) are
  **string-encoded** via `format!("{:.6}", value)` (six decimal places).
  Raw float literals are forbidden in the schema to eliminate
  IEEE-754 cross-machine rounding drift.
- **Timestamps in `data_span`**: ISO-8601 with `T` separator and `Z`
  zone, second precision (no fractional seconds — bar boundaries are
  whole hours). Example: `"2023-01-01T00:00:00Z"`. Distinct from the
  6-digit fractional-second audit-row format (ADR-0004); that applies
  to journal posts, not provenance JSON.
- **`weights_sha256`**: computed BEFORE the metadata is assembled, over
  the safetensors body bytes, hex-lowercase, no `0x` prefix.

The full `model_revision` SHA is then SHA-256 over the canonical
metadata bytes (which include `weights_sha256`).

Implementation lives in `crates/forecast/src/provenance.rs` (TCN
introduces it at M2 / T-D-9); v2.5a/v2.5b reuse the same canonicaliser
module verbatim.

The precedent is v2 LLM Q8 (canonical-JSON cache-key contract,
[ADR-0019](0019-v2-llm-strategy.md)). Same rule set, restated for
forecasters so future analysts cite a single ADR.

### 3. LFS-anchor strategy

Fixture-anchored checkpoints (the ones referenced by the locked
`evidence/anchors.toml` rows) ship in-repo via Git LFS under
`crates/forecast/checkpoints/anchors/<id>-<sha>.safetensors` +
`<id>-<sha>.metadata.json`. The `.gitattributes` rule:

```
crates/forecast/checkpoints/anchors/*.safetensors filter=lfs diff=lfs merge=lfs -text
```

Repo size impact: ~50-100 MB per anchor checkpoint. Acceptable per
operator decision T-OP-1 (2026-05-17), traded for anchor-verification
speed and determinism robustness.

**Non-anchored** checkpoints (operator-trained, experimental, scratch)
live under `crates/forecast/checkpoints/` (not `anchors/`) and are
gitignored.

### 4. Metal-vs-CPU determinism caveat

`candle` Metal kernels are NOT formally bit-identical to the CPU
backend. Cross-phase strategy:

- **CPU is the determinism oracle.** Anchor-verification `cargo test`
  jobs run inference on the CPU backend only. Metal stays for training
  (where speed matters) and operator-facing live inference (where
  small numerical drift below the ε direction band is acceptable).
- **Per-phase smoke test**: each phase MUST land a Metal-vs-CPU drift
  test asserting `(metal_tensor - cpu_tensor).abs().max() < 1e-4` on a
  representative forward pass. On failure → the LFS-anchor strategy
  becomes load-bearing (we ship the weights, not just a recipe).
- **LFS-anchor mitigation rationale**: because Metal-vs-CPU is not
  bit-identical, retraining from the seed on a different operator's
  machine would NOT reproduce identical weights. Shipping the weights
  via LFS bypasses the determinism gap entirely.
- **Replay cache neutralises drift on the consumer side**. Cache rows
  store post-quantisation `ForecastOverlay` (Direction + Decimal
  confidence), so anchored backtests replay identically across
  operator machines regardless of which backend trained the underlying
  weights.

### 5. Strict-OOS two-checkpoint backtest invariant

Every forecaster phase MUST ship two anchored checkpoints per scenario,
each strictly out-of-sample for its evaluation period:

- **BS-1 checkpoint**: trained on Jan-Sep 2023, validated on Oct-Dec
  2023. Evaluated on the full 2023 year via walk-forward retraining
  cadence.
- **BS-2 checkpoint**: trained on 2023 full year, validated on Q1 2024,
  evaluated (test) on Q2-Q4 2024.

This invariant is the analyst's strong recommendation (v2.5 R7,
confirmed at T-OP-2 on 2026-05-17). v2.5a (PatchTST) and v2.5b
(vanilla Transformer) inherit it. One-checkpoint splits that train on
2023 and evaluate BS-1 in-sample are NOT permitted — they break the
v2.6 bake-off fairness comparison.

## Alternatives considered

1. **Extend ADR-0028 with the schema** instead of opening ADR-0029.
   Rejected: ADR-0028 is already the framework-and-pivot decision;
   bolting the per-phase artifact contract onto it would double the
   ADR's surface area and force v2.5a/v2.5b analysts to disentangle
   "framework" from "artifact format". A separate, narrowly-scoped
   ADR is cleaner per the ADR-style rule in
   [adr/README.md](README.md) ("Longer ADRs usually mean the decision
   is actually two decisions in a trench coat").
2. **One ADR per phase** (ADR-0029 = TCN, ADR-00xx = PatchTST schema,
   etc.). Rejected: the schema rules are intentionally identical
   across phases; replicating them three times invites drift. The
   per-phase `feature.md § Design` documents extend the
   `architecture` + `tokenisation` sub-objects without re-litigating
   canonicalisation.
3. **Regenerate anchors from seed on every verification** (the
   non-LFS path from T-OP-1). Rejected per the operator decision:
   Metal-vs-CPU bit-identity is not proven, so retraining from seed
   on a different machine could diverge. The LFS-ship-the-weights
   path trades repo size for determinism robustness.
4. **One-checkpoint backtest split** (train on 2023, evaluate BS-1
   in-sample, BS-2 OOS). Rejected per T-OP-2: BS-1 in-sample
   evaluation is not a fair backtest and would inflate Sharpe vs the
   v1 momentum baseline.

## Consequences

### New files (this ADR scope)

- This file: `_bmad-output/planning-artifacts/architecture/decisions/0029-tcn-checkpoint-provenance.md`.
- TCN-side implementation lives at
  `crates/forecast/src/provenance.rs` (lands at T-D-9, M2 milestone
  for v2.5 TCN per `docs/archive/pre-bmad-spec/v1/v25-tcn-overlay/tasks.md`).

### Modified files

- `_bmad-output/planning-artifacts/architecture/decisions/README.md` — registry row added for ADR-0029.
- `docs/archive/pre-bmad-spec/architecture/12-forecast-overlay.md` — audit-row shape section
  references the provenance schema for the `model_revision` column.

### Cross-phase implications

- v2.5a (PatchTST) and v2.5b (vanilla Transformer) `feature.md §
  Design` MUST cite this ADR for canonicalisation rules + LFS-anchor
  strategy + two-checkpoint OOS invariant. Their analyst passes are
  free to vary the `architecture` and `tokenisation` sub-object keys.
- The v2.6 bake-off comparison reports rely on the two-checkpoint
  invariant. If any phase ships a one-checkpoint backtest, the
  bake-off cannot include it without explicit operator override.

### Anchor implications

- `crates/forecast/checkpoints/anchors/*.safetensors` join the
  LFS-tracked anchor surface. `.gitattributes` updated at T-D-11.
- No effect on the 11 existing locked anchors in
  `evidence/anchors.toml`.
- v2.5 adds 2 new anchors at ship (`top10-2023-fy-tcn-overlay`,
  `top10-2024-fy-tcn-overlay`).

## References

- [ADR-0028](0028-v25-dl-forecast-overlay-candle.md) — parent
  framework decision (custom Transformer/TCN in `candle`).
- [ADR-0019](0019-v2-llm-strategy.md) — Q8 canonical-JSON cache-key
  precedent (the rules this ADR restates for forecasters).
- [ADR-0004](0004-fractional-second-timestamps.md) — 6-digit
  fractional-second audit-row format (distinct from second-precision
  data_span timestamps locked here).
- [ADR-0002](0002-rng-chacha20.md) — `ChaCha20Rng` seed contract
  cited by `training.seed`.
- [`docs/archive/pre-bmad-spec/architecture/12-forecast-overlay.md`](../../../../docs/archive/pre-bmad-spec/architecture/12-forecast-overlay.md)
  — cross-cutting overlay design pattern.
- [`docs/archive/pre-bmad-spec/v1/v25-tcn-overlay/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/v25-tcn-overlay/feature.md)
  — v2.5 TCN feature spec; this ADR locks the schema referenced
  there as R8 / D4.
- [`docs/archive/pre-bmad-spec/v1/v25-dl-forecast-overlay/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/v25-dl-forecast-overlay/feature.md)
  — 4-phase roadmap umbrella.
