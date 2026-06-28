---
slug: v25-tcn-recalibrate
scenario: recalibrate-sigma-train-bs1
generated: 2026-05-21T07:10:30Z
wall_clock_s: 487.1
host: M022517718D
git_commit: b9fc3cdb6da2dba4bd274a61b6fbeb05b84aa9f1
model_revision: d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2
sigma_train_original: 10.954250
sigma_train_recalibrated: 0.018015675
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Recalibration report — BS1 σ_train

## Inputs

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Anchor scenario   | bs1 |
| model_revision    | d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2  (UNCHANGED — weights byte-identical) |
| weights_sha256    | 4ed9064a3871d8bc911ad8b288dccfc597caa6a09cca3b2395a9e1717b8c7025  (UNCHANGED) |
| Training span     | 2023-01-01T00:00:00Z .. 2023-12-31T23:00:00Z |
| Data revision SHA | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7 |
| Inferences        | 77820 |

## Result

| Field                       | Value           |
|-----------------------------|-----------------|
| σ_train (original metadata) | 10.954250 |
| σ_train (recalibrated)      | 0.018015675 |
| Ratio (orig / recal)        | 608.040 |
| r_hat count                 | 77820 |

## Wire-format contrast

```diff
- "sigma_train":10.95425033569336
+ "sigma_train":0.018015675
```

(All other 8 metadata fields byte-identical; see § Field invariance.)

## Field invariance — recalibrated overlay vs. original

| Field            | Original                     | Recalibrated                | Match |
|------------------|------------------------------|-----------------------------|-------|
| architecture     | (full obj)       | (verbatim copy)            | ✓ |
| data_span        | (full obj)       | (verbatim copy)            | ✓ |
| epochs_trained   | 30             | 30                        | ✓ |
| final_train_loss | 1.21676e-5     | 1.21676e-5    | ✓ |
| final_val_loss   | 1.53892e-5     | 1.53892e-5    | ✓ |
| model_revision   | d1c3696d79933c8d… | d1c3696d79933c8d…  | ✓ |
| tokenisation     | (full obj)       | (verbatim copy)            | ✓ |
| training         | (full obj)       | (verbatim copy)            | ✓ |
| weights_sha256   | 4ed9064a3871d8bc… | 4ed9064a3871d8bc…  | ✓ |
| **sigma_train**  | 10.954250 | 0.018015675 | **CHANGED** |

## Notes

- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json` original safetensors.
- Read-only against original `.metadata.json` (no mutation).
- σ_train formula: `std(r_hat)` per ADR-0035 § D1 (population std with f64 intermediates,
`1e-8` floor inherited from `train_tcn.rs:738`).
- Forward-pass call site: `TcnForecaster::forward(&x, false)` per ADR-0033 § D1.b.
- Recalibrated metadata canonicalisation: ADR-0035 § D2 (key ordering via ADR-0029 canonicaliser;
on-disk float format is JSON number, NOT the ADR-0029 string-encoded form).
