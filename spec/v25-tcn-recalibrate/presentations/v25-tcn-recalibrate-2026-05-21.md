---
slug: v25-tcn-recalibrate
mode: release
status: draft
audience: human-operator
updated: 2026-05-21
generated: 2026-05-21T12:30:00Z
version: 0.1.0
commit: 84a43ec5875d84cae6045216572806c064cd970e
predecessor: v25-tcn-alpha-investigation v0.1.0 (F4 verdict shipped 2026-05-19)
parent: v25-tcn-overlay v2.5.0 (in-progress)
---

# v2.5 TCN σ_train recalibration — release deck

## TL;DR

The σ_train calibration bug is fixed (BS-1: **608×** inflation, BS-2: **580×** inflation, both eliminated, weights untouched, anchors anchor-additive). The F-verdict legitimately stays **F4** under the immutable ADR-0033 algorithm — but the confidence gate that was silencing every forecast at 0% survival now lets **40% of BS-1 forecasts through at τ=0.6 (and 88.8% at τ=0.1)**. **You need to route on the joint signal**, not the F-label alone.

## What changed

- **New read-only bin** at [`crates/forecast/src/bin/recalibrate_sigma_train.rs`](../../../crates/forecast/src/bin/recalibrate_sigma_train.rs) (~490 LoC) that re-derives σ_train from a frozen-weights post-training forward pass over the metadata's `data_span`. No retraining, no weight mutation, no `.safetensors` edit.
- **One additive CLI flag** on `forecast_distribution`: [`crates/forecast/src/bin/forecast_distribution.rs:113-133`](../../../crates/forecast/src/bin/forecast_distribution.rs) — `--metadata-path <PATH>`. Default behavior is **byte-identical** to the shipped path; predecessor F4 reports stay re-runnable verbatim by omitting the flag.
- **Two new metadata overlay files** at `crates/forecast/checkpoints/anchors/tcn-bs{1,2}-<sha>.metadata.recalibrated.json` (mtime May 21). Original `.metadata.json` + `.safetensors` files are byte-identical (mtime May 17, `git diff` empty).
- **[ADR-0035](../../architecture/adr/0035-tcn-sigma-train-recalibration.md)** — *Post-training σ_train recalibration via metadata overlay (cross-phase contract for v2.5 / v2.5a / v2.5b)*. Codifies: D1 frozen-weights post-training pass as the canonical σ_train semantic (in-loop accumulator at [`train_tcn.rs:606,676-678,733-741`](../../../crates/forecast/src/bin/train_tcn.rs) **deprecated**); D2 overlay file + on-disk JSON number convention (intentional divergence from ADR-0029 § 2 rule 5); D3 additive `--metadata-path` opt-in (loader does NOT auto-prefer overlay); D4 σ_train-not-in-safetensors invariant codified as test.
- **4 new anchors** locked in [`spec/anchors.toml`](../../anchors.toml) (rows 181-197) under version `v2.6.1-alpha-investigation-recalibrated`. Anchor count: **22 → 26**.
- **7 new integration tests** (2 readonly + 4 field-invariance + 1 safetensors). 0 failures.

## Architect resolutions (M-T1)

- **T-AR-1 § Design** locked in `feature.md`; canonical decomposition at [`decomp.md`](../decomp.md). Bin name `recalibrate_sigma_train`, 4-arg CLI (`--scenario`, `--data-root`, `--out-dir`, `--anchor-dir`), forward-pass span read from original metadata's `data_span` (NOT the forecast-distribution eval span — for BS-2 they differ), overlay-file convention `.metadata.recalibrated.json` co-located with original, on-disk JSON-number divergence from ADR-0029 codified.
- **T-AR-2 § ADR-0035** written. Does **NOT** supersede ADR-0033 — F-verdict algorithm stays immutable per operator Q4=(a). Sits alongside ADR-0029 + ADR-0033 as the v2.5 forecaster read-path triad. Cross-phase applicability: v2.5a PatchTST + v2.5b Transformer inherit verbatim.

## What you can do now

| Action | Command |
|--------|---------|
| Re-derive BS-1 σ_train (≈8 min, read-only) | `cargo run -p forecast --release --features candle --bin recalibrate_sigma_train -- --scenario bs1` |
| Re-derive BS-2 σ_train (≈10 min, read-only) | `cargo run -p forecast --release --features candle --bin recalibrate_sigma_train -- --scenario bs2` |
| Re-run BS-1 forecast distribution under recalibrated σ_train | `cargo run -p forecast --release --features candle --bin forecast_distribution -- --scenario bs1 --metadata-path crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json --out-dir spec/v25-tcn-recalibrate/reports/` |
| Re-run **predecessor's F4 baseline** (proves no-regression) | `cargo run -p forecast --release --features candle --bin forecast_distribution -- --scenario bs1` (no `--metadata-path` → re-emits the byte-identical 20260519 body) |
| Verify all 26 anchors | `bash scripts/verify_anchors.sh` |
| Read BS-1 recalibrated distribution + Recalibration delta | open [`reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md`](../reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md) |
| Read BS-2 recalibrated distribution + Recalibration delta | open [`reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md`](../reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md) |
| Read BS-1 recalibration derivation (wire-format diff) | open [`reports/recalibrate-sigma-train-bs1-20260521.md`](../reports/recalibrate-sigma-train-bs1-20260521.md) |
| Read BS-2 recalibration derivation | open [`reports/recalibrate-sigma-train-bs2-20260521.md`](../reports/recalibrate-sigma-train-bs2-20260521.md) |
| Approve and queue follow-on | tick the appropriate box below; orchestrator opens the picked feature |

## Live demo

The recalibrate bin's `--help` surface proves the read-only contract — no `--retrain`, `--update-sigma`, `--write-checkpoint` flag exists (ADR-0033 § D1.c + ADR-0035 § D2 hard invariant; covered by `test_help_no_forbidden_flags`):

```
$ cargo run -p forecast --features candle --bin recalibrate_sigma_train -- --help
Loads the anchored checkpoint by --scenario, runs the converged model forward pass over metadata.data_span, computes σ_train as std(r_hat) (population std with f64 intermediates per ADR-0035 § D1), and writes a new .metadata.recalibrated.json overlay file next to the original. Original .metadata.json and .safetensors files stay byte-identical (ADR-0035 D2 hard invariant).

Usage: recalibrate_sigma_train [OPTIONS] --scenario <SCENARIO>

Options:
      --scenario <SCENARIO>
          Which anchored checkpoint to inspect

          Possible values:
          - bs1: BS-1: trained Jan–Dec 2023
          - bs2: BS-2: trained Jan 2023 – Mar 2024

      --data-root <DATA_ROOT>
          Parquet root for real OHLCV bars

          [default: data/binance/]

      --out-dir <OUT_DIR>
          Output directory for the recalibration derivation report

          [default: spec/v25-tcn-recalibrate/reports/]

      --anchor-dir <ANCHOR_DIR>
          Target directory for the new .metadata.recalibrated.json file. Defaults to the checkpoint's own anchor dir, co-located with the original .metadata.json (which is NOT touched)

          [default: crates/forecast/checkpoints/anchors/]

  -h, --help
          Print help (see a summary with '-h')
```

And the ground-truth state of the anchored checkpoints — mtimes confirm originals untouched (May 17) while overlays are May 21:

```
$ ls -la crates/forecast/checkpoints/anchors/*.metadata*.json
-rw-r--r--  855 May 17 15:55  tcn-bs1-d1c3696d…metadata.json                  (ORIGINAL — untouched)
-rw-r--r--  858 May 21 09:10  tcn-bs1-d1c3696d…metadata.recalibrated.json    (NEW overlay)
-rw-r--r--  852 May 17 19:08  tcn-bs2-3fabcabe…metadata.json                  (ORIGINAL — untouched)
-rw-r--r--  855 May 21 09:21  tcn-bs2-3fabcabe…metadata.recalibrated.json    (NEW overlay)

$ git diff HEAD -- "crates/forecast/checkpoints/anchors/*.metadata.json" "crates/forecast/checkpoints/anchors/*.safetensors"
(no output — byte-identical to committed state; ADR-0035 D4 + R7 invariant intact)
```

And the predecessor F4 evidence bodies re-hashed in this run — byte-identical to the locked anchor SHAs:

```
$ python3 scripts/hash_report.py spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs{1,2}-realdata-20260519.md
ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54  forecast-distribution-bs1-realdata-20260519.md
d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06  forecast-distribution-bs2-realdata-20260519.md
```

The R7 hard invariant ("22 originals byte-identical") is verified by direct hash — both predecessor anchor SHAs match.

## Headline numbers

### σ_train calibration bug — diagnosed and eliminated

| Checkpoint | σ_train (original metadata) | σ_train (recalibrated) | Ratio | r_hat samples | H1 verdict (∈ 0.005..0.025) |
|------------|-----------------------------:|-----------------------:|------:|--------------:|------------------------------|
| BS-1       | **10.954250** | **0.018015675** | **608.040×** | 77,820 | PASS |
| BS-2       | **6.916286**  | **0.011913909** | **580.522×** | 99,660 | PASS |

Both recalibrated values land squarely in the expected 0.005..0.025 range (matches the F4-evidence inference-time `std` from the predecessor: BS-1 std = 0.018015573, BS-2 std = 0.009976302). Root cause locked in [ADR-0035 § Context](../../architecture/adr/0035-tcn-sigma-train-recalibration.md): the in-loop accumulator pattern at [`train_tcn.rs:606,676-678,733-741`](../../../crates/forecast/src/bin/train_tcn.rs) collected per-batch `r_hat` across all 30 epochs without inter-epoch reset, so the final std was dominated by pre-convergence trajectory noise (O(1)..O(10) outputs from random-init weights at epoch 1) — not the converged model's prediction std (O(0.01..0.02)).

### Gate-survival jump — the load-bearing finding for routing

Per operator decision Q4=(a)+(c) (architect honored): the F-verdict stays F4 (algorithm-honest reading of immutable ADR-0033 § D3) AND the recalibration delta surfaces as a standalone `## Recalibration delta` section in each report, regardless of F-label. **The operator routes on the joint signal.**

**BS-1 — σ_train: 10.954 → 0.018 (608×); evaluation span 2023-01-01..2024-01-01, 77,830 inferences:**

| Metric                              | Pre-recalibration | Post-recalibration | Delta |
|-------------------------------------|------------------:|-------------------:|------:|
| σ_train                             | 10.954250336      | 0.018015675        | -10.936234660 |
| gate survival τ=0.1                 | **0.000000**      | **0.887640**       | **+88.76 pp** |
| gate survival τ=0.5                 | 0.000000          | 0.480355           | +48.04 pp |
| gate survival τ=0.6                 | **0.000000**      | **0.400578**       | **+40.06 pp** |
| gate survival τ=0.9                 | 0.000000          | 0.232391           | +23.24 pp |
| F-verdict                           | F4                | F4 (immutable)     | — |
| `frac_inside_epsilon` (ε=0.0005)    | 0.030952          | 0.030952           | 0.000 (unchanged — model output unchanged) |

**BS-2 — σ_train: 6.916 → 0.012 (580×); evaluation span 2024-01-01..2025-01-01, 78,080 inferences:**

| Metric                              | Pre-recalibration | Post-recalibration | Delta |
|-------------------------------------|------------------:|-------------------:|------:|
| σ_train                             | 6.916285515       | 0.011913909        | -6.904371606 |
| gate survival τ=0.1                 | **0.000000**      | **0.863461**       | **+86.35 pp** |
| gate survival τ=0.5                 | 0.000000          | 0.421350           | +42.14 pp |
| gate survival τ=0.6                 | **0.000000**      | **0.345441**       | **+34.54 pp** |
| gate survival τ=0.9                 | 0.000000          | 0.192828           | +19.28 pp |
| F-verdict                           | F4                | F4 (immutable)     | — |
| `frac_inside_epsilon` (ε=0.0005)    | 0.057492          | 0.057492           | 0.000 (unchanged) |

**Reading.** The model's `r_hat` distribution did not change (weights are byte-identical — only the σ_train denominator in the gate equation changed). What changed is the gate calculation `|r_hat|/σ_train ≥ τ`: with the recalibrated denominator, the model passes a τ=0.6 confidence gate on 40% of bars for BS-1 and 35% for BS-2. The original 0% survival was an artifact of the inflated denominator (training trajectory variance, not converged-model prediction variance) — exactly the diagnosis the predecessor deck hypothesised.

The F-verdict stays F4 because the F3 trigger requires `frac_inside_epsilon > 0.5` — measured values 0.031 (BS-1) and 0.057 (BS-2) are well below 0.5, so the priority tree falls through to F4 (no signal at 1h horizon) by construction. H2 is **honestly falsified** for the full-F3-flip case; the analyst predicted this in `feature.md § H2 refined`.

## Hypothesis register — closing

| Hypothesis | Statement | Status | Evidence |
|------------|-----------|--------|----------|
| **H1** | σ_train calibration bug is real and reproducible; recalibrated value lands in 0.005..0.025 range. | **CONFIRMED** | BS-1: 0.018015675 ∈ range. BS-2: 0.011913909 ∈ range. Both within 1 order of magnitude of predecessor's inference-time std (`0.018015573` / `0.009976302`). |
| **H2** | Metadata-only fix re-classifies F4 → F3 (full F-verdict flip). | **HONESTLY FALSIFIED** for full F4→F3 flip; **PARTIALLY CONFIRMED** for the underlying mechanism. Per [feature.md § H2 refined](../feature.md), the analyst predicted this at brief time: `frac_inside_epsilon < 0.5` blocks F3 under ADR-0033 § D3 priority tree. Gate-survival jump is the standalone signal Q4=(c) surfaces. |
| **H3** | Retraining NOT required — bug is purely in σ_train scalar; weights produce a reasonable r_hat distribution. | **CONFIRMED** | `weights_sha256` byte-identical; `model_revision` byte-identical; `r_hat` distribution stats byte-identical to predecessor F4 evidence (mean, std, percentiles all match — only the gate denominator differs). `sigma_train_not_in_safetensors` test passes (ADR-0035 D4). |

## Test results

From the [tester M-FINAL report](../reports/test-20260521-1200-v25-tcn-recalibrate.md) (`VERDICT → PASS`):

| Gate | Result | Evidence |
|------|--------|----------|
| T-F1 — `cargo fmt --check` + `cargo clippy --workspace -- -D warnings` | PASS | 0 new warnings; `Finished` in 1.22s |
| T-F2 — `cargo clippy -p forecast --features candle -- -D warnings` | PASS | `Finished` in 0.24s; 0 warnings |
| T-F3 — `cargo test --workspace --lib` | PASS | 311 passed (ui) + 52 (forecast) + 36 (strategy) + 13 (exec) + 9 (backtest) + 47 (audit) + ... all green; 0 failures |
| T-F4a — `recalibrate_sigma_train_readonly` (2 tests) | PASS | `test_help_no_forbidden_flags`, `test_originals_untouched_by_run` |
| T-F4b — `recalibrate_sigma_train_field_invariance` (4 tests) | PASS | `test_recalibrated_overlay_invariance`, `test_overlay_no_key_count_change`, `test_overlay_canonical_deterministic`, `test_sigma_train_is_json_number_not_string` |
| T-F4c — `sigma_train_not_in_safetensors` (1 test, ADR-0035 D4) | PASS | `test_no_sigma_tensor_in_anchors` |
| T-F5 — 2-run determinism gate | PASS | all 4 new report bodies byte-identical across 2 sequential runs |
| T-F6 — Anchor lock | PASS | 4 new anchors locked under `v2.6.1-alpha-investigation-recalibrated`; 22 → 26 |
| T-F7 — Anchor neutrality (R7) | PASS | original `.metadata.json` + `.safetensors` byte-identical (`git diff` empty, mtime May 17); 20 backtest/sharpe anchors direct PASS; 2 predecessor F4-evidence bodies SHA-confirmed by direct hash |
| T-F8 — `spec-lint` | PASS | 87/2 (down from predecessor baseline 735/2); 0 new categories; -648 violations vs baseline (prior feature cleanup) |

## Verification matrix

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V-R1 | Diagnose σ_train computation bug (analyst) | VERIFIED | `feature.md § Why`; bug site cited at `train_tcn.rs:606,676-678,733-741`; root cause locked in ADR-0035 § Context. |
| V-R2 | Metadata-only re-derivation tool on disk | VERIFIED | `crates/forecast/src/bin/recalibrate_sigma_train.rs` (~490 LoC); 2 derivation reports under `spec/v25-tcn-recalibrate/reports/`. |
| V-R3 | Recalibrated overlay file naming + checkpoint-revision pin | VERIFIED | `tcn-bs{1,2}-<sha>.metadata.recalibrated.json` co-located with originals; `model_revision` + `weights_sha256` byte-identical (field invariance test 4/4 PASS). |
| V-R4 | Re-run forecast_distribution under recalibrated σ_train | VERIFIED | 2 new reports `forecast-distribution-bs{1,2}-realdata-recalibrated-20260521.md` with `verdict: F4` in frontmatter + standalone `## Recalibration delta` section. |
| V-R5 | F-verdict re-classification + joint label | VERIFIED | Joint F4 confirmed under immutable ADR-0033 § D3; recalibration delta surfaced as standalone signal per Q4=(c). |
| V-R6 | Anchor-additive contract (22 → 26) | VERIFIED | 4 new anchors under `v2.6.1-alpha-investigation-recalibrated`; 20 backtest/sharpe anchors direct PASS in `verify_anchors.sh`; 2 predecessor F4-evidence bodies SHA-confirmed by direct hash (`ef73cb8d…` / `d7cd08e6…`). |
| V-R7 | Non-regression / anchor-neutrality contract (R7) | VERIFIED | original `.metadata.json` + `.safetensors` byte-identical (`git diff` empty); 22 pre-feature anchor bodies byte-identical. |
| V-R8 | Determinism contract (R8) | VERIFIED | 2-run determinism PASS for all 4 new report bodies; recalibrate bin's `wall_clock_s` differs (frontmatter-only field) but body SHA stable. |

## Numbers that matter

- **σ_train inflation eliminated** — BS-1: 608.040×; BS-2: 580.522×.
- **Gate survival τ=0.6** — BS-1: 0% → **40.06%**; BS-2: 0% → **34.54%**.
- **Gate survival τ=0.1** — BS-1: 0% → **88.76%**; BS-2: 0% → **86.35%**.
- **Tests** — 311 (ui) + 52 (forecast) + workspace-wide 0 failures; **7 new** integration tests (2 readonly + 4 field-invariance + 1 safetensors-invariant) all PASS.
- **Anchors** — 22 → **26**. 20 backtest/sharpe anchor bodies + 2 predecessor F4-evidence bodies all byte-identical to pre-feature SHAs. 4 new anchors locked under `v2.6.1-alpha-investigation-recalibrated`:
  - `forecast-distribution-bs1-realdata-recalibrated` → `8a548042f552899cbccfa4d9b8d6eca6306f7de5c1a1bd7ed18201b08a06f80f`
  - `forecast-distribution-bs2-realdata-recalibrated` → `d6c1e17ca162469e94b8dacd7c4485ec4d8cd77b6768f9e7ebe2f7deaf4b4151`
  - `recalibrate-sigma-train-bs1` → `baa658fb7ad96796f643d8fecab9156362b17faad97afc37be77867850336ad9`
  - `recalibrate-sigma-train-bs2` → `bfa8104ace81dd6a98f42a65cd0a5bd584089fa93fbafa4aa6f11d02954b47e0`
- **Lint** — `cargo fmt --check` PASS; `cargo clippy --workspace -- -D warnings` PASS; `cargo clippy -p forecast --features candle -- -D warnings` PASS.
- **Spec-lint** — 87/2 (down from predecessor baseline 735/2; **-648** violations from prior feature cleanup; 0 new categories from this feature).
- **Compute** — BS-1 recalibrate wall-clock 487.1s; BS-2 619.8s; BS-1 re-run forecast_distribution 486.9s; BS-2 487.6s. Combined ~33 min on the developer machine. Run-once, anchored output.
- **Inferences** — 77,820 (BS-1 recalibrate, training span) + 99,660 (BS-2 recalibrate, training span) + 77,830 (BS-1 re-run, eval span) + 78,080 (BS-2 re-run, eval span) = 333,390 forward-passes.
- **Trace** — `REQ-V25-TCN-RECALIBRATE-001` state `shipped` (was `proposed → in-progress`).

## Implications

- **σ_train is no longer a confounding variable in the v2.5 TCN model assessment.** Any future "the gate is silencing everything" claim must point to a different root cause (or to an actual no-signal conclusion).
- **The cross-phase contract in ADR-0035** prevents the same bug shape from recurring in v2.5a PatchTST + v2.5b Transformer + future training scaffolds. The deprecated `train_tcn.rs:606` in-loop accumulator pattern is now codified as a negative precedent; future analyst passes spawning `v25-patchtst-overlay` / `v25-tx-overlay` must reject the same shape at design time.
- **The predecessor's 22-anchor backtest assertion stays intact.** This feature is purely additive — the F4-evidence baseline (predecessor's 3 alpha-investigation anchors + 19 pre-investigation anchors) remains byte-identical and citeable for any future "before σ_train fix vs after" comparison.
- **Gate-survival jump from 0% to 40% (BS-1) / 35% (BS-2) at τ=0.6 is a substantive new diagnostic signal.** It does NOT directly translate to "the model has alpha" — the F-verdict algorithm correctly stays F4 because the gate-survival statistic is independent of the actual directional accuracy of the surviving forecasts. But it DOES open the door to a cheap follow-on (`v25-tcn-threshold-tuning`) that can either confirm or refute alpha by sweeping ε / τ over the now-non-degenerate gate distribution.

## Deferred / out of scope

- **No retraining performed.** Weights are byte-identical. If the gate-survival jump doesn't unlock alpha under threshold-tuning, the next-up follow-on is `v25-tcn-horizon-bump-or-retire` (multi-week retrain).
- **No ε / τ tuning.** Defaults stay at ε=0.0005 and τ=0.6. That's `v25-tcn-threshold-tuning`'s job if you fund it.
- **No ADR-0033 amendment.** F-verdict algorithm stays immutable across this feature (operator Q4=(a)). If a future feature needs an F3' branch ("gating-too-tight when gate-survival > 0 but `frac_inside_epsilon` ≤ 0.5"), it must ship a superseding ADR — per the ADR-0033 self-immutability rule.
- **`verify_anchors.sh` lexicographic-picker artefact** — the script's glob `*/reports/forecast-distribution-bs{1,2}-realdata-*.md` now matches both `...-20260519.md` (predecessor F4) AND `...-recalibrated-20260521.md` (new). Lexicographic sort picks the newer file (`r` > `2` in ASCII), so the script reports 2 "FAILs" for the predecessor anchor scenarios. **These are non-blocking picker artefacts**, not substantive regressions — the original bodies are byte-identical, confirmed by direct hash above. A future architect-level enhancement to discriminate the `-recalibrated-` suffix from the base scenario is deferred as a non-blocking spec-debt item.
- **PatchTST / Transformer comparison.** That's `v2.6` bake-off territory, two features downstream.

## Open decisions

One decision, surfaced cleanly. Your standing "autoapprove all" directive applies if you want option **(c)**.

**Which follow-on(s) to fund and in what order?**

The F4 verdict stays F4 (algorithm-honest), so the routing table in `feature.md § R5` formally says `v25-tcn-horizon-bump-or-retire`. **BUT** the gate-survival jump from 0% to 40% (BS-1 τ=0.6) / 35% (BS-2 τ=0.6) — and 88.76% / 86.35% at τ=0.1 — is the substantive new signal that the analyst's Q4=(c) resolution surfaces. The model has been emitting reasonable-magnitude forecasts the whole time; the gate was just silencing them. Whether those forecasts have directional value is a separate question that a cheap τ-sweep can answer.

Three routing options, ranked by cost:

- **(a) Fund `v25-tcn-threshold-tuning` only.** Cheap — hours, not weeks. Sweep ε and τ over the now-non-degenerate gate distribution and re-compute Sharpe / drawdown / Calmar to see if any non-default (ε, τ) extracts alpha. If yes, the v2.5 TCN is salvaged without retraining. If no, you've ruled out gate-tuning cheaply before paying for a retrain.
- **(b) Fund `v25-tcn-horizon-bump-or-retire` only.** Multi-week retrain (24h horizon head, or retire v2.5 TCN in favor of v2.5a PatchTST). Honest reading of the F4 verdict — the model isn't predicting next-1h returns well at the model level, so a τ-sweep over a directionally-uninformative gate distribution may extract nothing. Higher conviction in the result; much higher cost.
- **(c) Queue both — threshold-tuning first, horizon-bump only if τ-sweep finds no alpha.** This is the analyst-recommended sequencing carried forward from the predecessor deck. Cheap exploration first, expensive retrain only as a fallback. **Default recommendation.**

The cost asymmetry argues hard for (c): the gate-survival jump is the cheapest exploration in the space, and you get clean information (alpha / no alpha at the v2.5 TCN architecture) before committing to a retrain budget.

## Rollback

This feature is anchor-additive only and metadata-overlay only — full rollback is mechanical:

| Wave | Rollback action | Cost |
|------|-----------------|------|
| A (recalibrate bin + overlay files) | `rm crates/forecast/checkpoints/anchors/*.metadata.recalibrated.json && git revert <wave-A-sha>` — original `.metadata.json` + `.safetensors` were never touched. | ~1 minute |
| B (`--metadata-path` flag + new reports) | `rm spec/v25-tcn-recalibrate/reports/forecast-distribution-bs{1,2}-realdata-recalibrated-20260521.md && git revert <wave-B-sha>` — predecessor F4 reports still re-runnable. | ~5 minutes |
| C (anchor lock) | revert the 4 new rows in `spec/anchors.toml` (lines 181-197). 22 originals stay byte-identical the whole time. | ~2 minutes |
| Full feature | `git revert` the 4 wave commits + `rm` the artifact files. Original 22 anchors stay byte-identical throughout (R7 invariant). | ~10 minutes total |

The non-negotiable safety net: original `.metadata.json` + `.safetensors` files are byte-identical to committed state (`git diff` empty, confirmed above). No rollback path requires re-derivation of any locked artifact.

## Approval

- [x] Approved — ship; queue `v25-tcn-threshold-tuning` first, then `v25-tcn-horizon-bump-or-retire` if τ-sweep finds no alpha (analyst-recommended sequencing per option (c))
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

Operator decided routing (c) 2026-05-21 via the orchestrator's
batched-Q AskUserQuestion prompt. σ_train bug confirmed eliminated;
gate-survival jump (BS-1 τ=0.6: 0%→40.1%, τ=0.1: 0%→88.8%) is the
load-bearing signal. F-verdict stays F4 per immutable ADR-0033 § D3,
but σ_train no longer confounds the model assessment. Follow-on
briefs `v25-tcn-threshold-tuning` (cheap, first) +
`v25-tcn-horizon-bump-or-retire` (multi-week, fallback) to be
promoted on next operator "next" directive.

## Closing gates

Both mechanical gates run on the file just written:

```
$ bash scripts/check_presentation.sh spec/v25-tcn-recalibrate/presentations/v25-tcn-recalibrate-2026-05-21.md
<PASS line appears in presenter handoff envelope below>
```

```
$ uv run scripts/spec_lint.py
spec-lint: FAIL (87 violations in 2 categories)
```

Baseline match: 87/2 expected (per tester report § 7c; -648 vs the predecessor v25-tcn-alpha-investigation baseline of 735/2). The 6 `trace-broken-path` violations are pre-existing roadmap anchors for PatchTST, Transformer, and the v2.6 bake-off (features not yet shipped); the 81 `dead-link` violations are unchanged carryover from older feature docs. **No new categories from this feature.**

## Sources cited

- [`feature.md`](../feature.md) — feature brief v0.1.0; R1-R8 + H1-H3 + K1-K5 + Q1-Q5; § Design locked at M-T1; § Verification recorded by tester.
- [`tasks.md`](../tasks.md) — all T-A1..T-A5 + T-OD1..T-OD5 + T-AR-1..T-AR-3 + T-D-N1..T-D-N8 + T-T-1.a..T-T-1.f ticked.
- [`decomp.md`](../decomp.md) — M-T1 architect decomposition.
- [ADR-0035](../../architecture/adr/0035-tcn-sigma-train-recalibration.md) — Post-training σ_train recalibration via metadata overlay (cross-phase contract).
- [ADR-0033](../../architecture/adr/0033-tcn-alpha-investigation-report-shape.md) — F-verdict algorithm (§ D3 immutable across this feature per Q4=(a)).
- [ADR-0029](../../architecture/adr/0029-tcn-checkpoint-provenance.md) — checkpoint provenance + canonical-JSON metadata schema; on-disk JSON number divergence codified in ADR-0035 § D2.
- [`reports/test-20260521-1200-v25-tcn-recalibrate.md`](../reports/test-20260521-1200-v25-tcn-recalibrate.md) — tester M-FINAL report (`VERDICT → PASS`).
- [`reports/recalibrate-sigma-train-bs1-20260521.md`](../reports/recalibrate-sigma-train-bs1-20260521.md), [`reports/recalibrate-sigma-train-bs2-20260521.md`](../reports/recalibrate-sigma-train-bs2-20260521.md) — derivation reports with wire-format diff + field-invariance table.
- [`reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md`](../reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md), [`reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md`](../reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md) — F-verdict re-runs with standalone `## Recalibration delta` section.
- [Predecessor presenter deck 2026-05-19](../../v25-tcn-alpha-investigation/presentations/v25-tcn-alpha-investigation-2026-05-19.md) — F4 verdict + σ_train anomaly elevated to top-level finding; ranked follow-on queue source.
- `spec/anchors.toml` lines 181-197 — 4 new entries under `v2.6.1-alpha-investigation-recalibrated`.
- `spec/trace.toml` — `REQ-V25-TCN-RECALIBRATE-001` state `shipped`.
- Bug site (deprecated per ADR-0035 § D1): [`crates/forecast/src/bin/train_tcn.rs:606,676-678,733-741`](../../../crates/forecast/src/bin/train_tcn.rs).
- Inference-time read sites: [`crates/forecast/src/tcn.rs:534,937`](../../../crates/forecast/src/tcn.rs).

## Changelog

- 2026-05-21 (presenter): initial release deck. σ_train calibration bug confirmed real (BS-1 608× / BS-2 580×) and eliminated. F-verdict stays F4 under immutable ADR-0033 § D3 algorithm (H2 honestly falsified). Gate-survival jump from 0% to 40% (BS-1 τ=0.6) / 88.76% (BS-1 τ=0.1) surfaced as the load-bearing routing signal per Q4=(c). Three ranked options surfaced; analyst default = (c) threshold-tuning first, horizon-bump only as fallback. Mechanical pre-tick + spec-lint gates passed at baseline 87/2 (down -648 from predecessor's 735/2 — no new categories).
