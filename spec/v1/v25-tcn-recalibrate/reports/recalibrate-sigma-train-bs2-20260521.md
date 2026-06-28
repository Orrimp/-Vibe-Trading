---
slug: v25-tcn-recalibrate
scenario: recalibrate-sigma-train-bs2
generated: 2026-05-21T07:21:10Z
wall_clock_s: 619.8
host: M022517718D
git_commit: b9fc3cdb6da2dba4bd274a61b6fbeb05b84aa9f1
model_revision: 3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d
sigma_train_original: 6.916286
sigma_train_recalibrated: 0.011913909
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Recalibration report — BS2 σ_train

## Inputs

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Anchor scenario   | bs2 |
| model_revision    | 3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d  (UNCHANGED — weights byte-identical) |
| weights_sha256    | 5f22b5bcb4c2fdd0b320827b17f4af39f7a7a3a92605c86042535011415ca474  (UNCHANGED) |
| Training span     | 2023-01-01T00:00:00Z .. 2024-03-31T23:00:00Z |
| Data revision SHA | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7 |
| Inferences        | 99660 |

## Result

| Field                       | Value           |
|-----------------------------|-----------------|
| σ_train (original metadata) | 6.916286 |
| σ_train (recalibrated)      | 0.011913909 |
| Ratio (orig / recal)        | 580.522 |
| r_hat count                 | 99660 |

## Wire-format contrast

```diff
- "sigma_train":6.916285514831543
+ "sigma_train":0.011913909
```

(All other 8 metadata fields byte-identical; see § Field invariance.)

## Field invariance — recalibrated overlay vs. original

| Field            | Original                     | Recalibrated                | Match |
|------------------|------------------------------|-----------------------------|-------|
| architecture     | (full obj)       | (verbatim copy)            | ✓ |
| data_span        | (full obj)       | (verbatim copy)            | ✓ |
| epochs_trained   | 30             | 30                        | ✓ |
| final_train_loss | 8.00147e-6     | 8.00147e-6    | ✓ |
| final_val_loss   | 1.05101e-5     | 1.05101e-5    | ✓ |
| model_revision   | 3fabcabecbee94d6… | 3fabcabecbee94d6…  | ✓ |
| tokenisation     | (full obj)       | (verbatim copy)            | ✓ |
| training         | (full obj)       | (verbatim copy)            | ✓ |
| weights_sha256   | 5f22b5bcb4c2fdd0… | 5f22b5bcb4c2fdd0…  | ✓ |
| **sigma_train**  | 6.916286 | 0.011913909 | **CHANGED** |

## Notes

- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.recalibrated.json` original safetensors.
- Read-only against original `.metadata.json` (no mutation).
- σ_train formula: `std(r_hat)` per ADR-0035 § D1 (population std with f64 intermediates,
`1e-8` floor inherited from `train_tcn.rs:738`).
- Forward-pass call site: `TcnForecaster::forward(&x, false)` per ADR-0033 § D1.b.
- Recalibrated metadata canonicalisation: ADR-0035 § D2 (key ordering via ADR-0029 canonicaliser;
on-disk float format is JSON number, NOT the ADR-0029 string-encoded form).
