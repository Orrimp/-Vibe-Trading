# crates/forecast

`ForecastProvider` trait + Kronos ONNX forecaster for v2.5.

## Crate layout

```
crates/forecast/
├── src/
│   ├── lib.rs          ForecastProvider async trait
│   ├── kronos.rs       KronosForecaster (stub at M1; tract wiring at M3)
│   └── overlay.rs      combine() helper for signal-level overlay composition
├── assets/
│   ├── kronos-base.onnx            Kronos-base checkpoint (git LFS)
│   ├── kronos-base.onnx.sha256     Pinned SHA-256 (checksum gate)
│   └── kronos-base.onnx.license   SPDX-License-Identifier: MIT
├── build.rs            Checksum gate (asserts SHA-256 at build time)
└── Cargo.toml
```

## LFS bootstrap

The Kronos-base ONNX checkpoint (~410 MB) is tracked via
[git LFS](https://git-lfs.com). Without LFS, `git clone` will download a
tiny pointer file instead of the real weights, and `cargo build -p forecast`
will print a warning and skip inference (the stub returns
`ForecastError::Inference` at runtime).

### First-time setup

```sh
# 1. Install git LFS (once per machine)
brew install git-lfs   # macOS
# or: apt-get install git-lfs   # Debian/Ubuntu

# 2. Enable LFS hooks in this repo (once per clone)
git lfs install

# 3. Pull LFS objects (fetches the actual ONNX file)
git lfs pull
```

### Generating the ONNX from scratch

If you need to re-export from the upstream HuggingFace checkpoint:

```sh
# Requires: python >=3.10, torch >=2.1, onnx >=1.15, transformers >=4.35
pip install torch onnx transformers huggingface-hub

python scripts/dev/kronos_torch_to_onnx.py \
    --output crates/forecast/assets/kronos-base.onnx
```

The script:
1. Asserts the MIT license on the HF repo.
2. Loads `NeoQuasar/Kronos-base` at the pinned revision.
3. Exports via `torch.onnx.export` (opset 17).
4. Writes the SHA-256 to `assets/kronos-base.onnx.sha256`.
5. Prints the git LFS commit commands.

### After generating

```sh
git add crates/forecast/assets/kronos-base.onnx
git add crates/forecast/assets/kronos-base.onnx.sha256
git add crates/forecast/assets/kronos-base.onnx.license
git commit -m "chore(forecast): vendor Kronos-base ONNX checkpoint [LFS]"
```

## Checksum gate

`build.rs` asserts that `assets/kronos-base.onnx` matches the hash in
`assets/kronos-base.onnx.sha256` at every `cargo build -p forecast`. If
the file is absent or the hash says `PENDING`, the gate is skipped with a
warning.

**If the build fails with "ONNX checksum mismatch"**: the checkpoint file
was modified without updating the `.sha256` file. Re-run the conversion
script or restore from LFS.

## Milestone status

| Milestone | Status | Description |
|---|---|---|
| M1 | DONE | `ForecastProvider` trait, `KronosForecaster` stub, `overlay::combine()`, `replay-cache` extraction |
| M2 | PARTIAL | `build.rs` gate, `.gitattributes`, conversion script — ONNX not committed (LFS bootstrap blocked on this machine) |
| M3 | PENDING | `tract` integration — actual model load + forward pass |

## References

- [spec/architecture/12-forecast-overlay.md](../../spec/architecture/12-forecast-overlay.md)
- [spec/architecture/adr/0027-kronos-onnx-tract-integration.md](../../spec/architecture/adr/0027-kronos-onnx-tract-integration.md)
- [spec/v25-kronos-forecast-overlay/tasks.md](../../spec/v25-kronos-forecast-overlay/tasks.md)
- Upstream model: https://github.com/shiyu-coder/Kronos (MIT, AAAI 2026)
- HF weights: https://huggingface.co/NeoQuasar/Kronos-base
