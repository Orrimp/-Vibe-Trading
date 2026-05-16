#!/usr/bin/env python3
"""
kronos_torch_to_onnx.py — one-off conversion: NeoQuasar/Kronos-base → ONNX.

v2.5 / ADR-0027 Q3 / T-M2-1.

## Purpose

Converts the Hugging Face `NeoQuasar/Kronos-base` checkpoint to ONNX format
for use with the `tract` in-process inference engine in `crates/forecast/`.

This script is NOT committed to the runtime path. It is a developer utility
run once (or when the upstream checkpoint updates) and the resulting `.onnx`
is committed via git LFS.

## Pinned HF revision

HF_REVISION = "main"  (update to a commit SHA once the checkpoint stabilises)

## Prerequisites

    pip install torch>=2.1 onnx>=1.15 transformers>=4.35 huggingface-hub

## Usage

    python scripts/dev/kronos_torch_to_onnx.py \
        --output crates/forecast/assets/kronos-base.onnx

## Output

  - `crates/forecast/assets/kronos-base.onnx` — the ONNX file (via git LFS).
  - `crates/forecast/assets/kronos-base.onnx.sha256` — updated SHA-256 hash.
  - License assertion: MIT (NeoQuasar/Kronos-base).

## License

NeoQuasar/Kronos weights are MIT-licensed:
  https://github.com/shiyu-coder/Kronos
  https://huggingface.co/NeoQuasar/Kronos-base

This script itself is MIT-licensed (same as the trading project).
"""

import argparse
import hashlib
import pathlib
import sys

HF_REPO_ID = "NeoQuasar/Kronos-base"
HF_REVISION = "main"  # Pin to a commit SHA for reproducibility once stable.
EXPECTED_LICENSE = "mit"
MODEL_SIZE_PARAMS = 102_300_000  # 102.3M params for `base` per ADR-0027 Q2.

def main() -> None:
    parser = argparse.ArgumentParser(description="Convert Kronos-base → ONNX")
    parser.add_argument(
        "--output",
        type=pathlib.Path,
        default=pathlib.Path("crates/forecast/assets/kronos-base.onnx"),
        help="Output path for the ONNX file.",
    )
    parser.add_argument(
        "--revision",
        default=HF_REVISION,
        help="HF revision SHA (default: main). Pin to a commit SHA for reproducibility.",
    )
    args = parser.parse_args()

    try:
        import torch
        from huggingface_hub import hf_hub_download, model_info
    except ImportError as exc:
        sys.exit(
            f"[kronos_torch_to_onnx] Missing dependency: {exc}.\n"
            "Install with: pip install torch>=2.1 onnx>=1.15 transformers>=4.35 huggingface-hub"
        )

    output_path: pathlib.Path = args.output
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # ── License assertion ────────────────────────────────────────────────────
    print(f"[kronos_torch_to_onnx] Fetching model info for {HF_REPO_ID}@{args.revision} …")
    info = model_info(HF_REPO_ID, revision=args.revision)
    license_tag = getattr(info, "license", "").lower()
    if EXPECTED_LICENSE not in license_tag:
        sys.exit(
            f"[kronos_torch_to_onnx] License check FAILED: expected MIT, got '{license_tag}'.\n"
            "Do NOT proceed. Verify the license before committing the ONNX file."
        )
    print(f"[kronos_torch_to_onnx] License OK: {license_tag}")

    # ── Model load ───────────────────────────────────────────────────────────
    print(f"[kronos_torch_to_onnx] Loading {HF_REPO_ID}@{args.revision} …")
    try:
        from transformers import AutoModelForCausalLM, AutoConfig
        config = AutoConfig.from_pretrained(HF_REPO_ID, revision=args.revision, trust_remote_code=True)
        model = AutoModelForCausalLM.from_pretrained(
            HF_REPO_ID,
            revision=args.revision,
            trust_remote_code=True,
            torch_dtype=torch.float32,
        )
        model.eval()
    except Exception as exc:
        sys.exit(f"[kronos_torch_to_onnx] Model load FAILED: {exc}")

    # ── Param count assertion ────────────────────────────────────────────────
    param_count = sum(p.numel() for p in model.parameters())
    print(f"[kronos_torch_to_onnx] Param count: {param_count:,}")
    tolerance = 0.05  # 5% tolerance
    if abs(param_count - MODEL_SIZE_PARAMS) / MODEL_SIZE_PARAMS > tolerance:
        print(
            f"[kronos_torch_to_onnx] WARNING: param count {param_count:,} deviates from "
            f"expected {MODEL_SIZE_PARAMS:,} by >{tolerance*100:.0f}%. "
            "Verify this is still the `base` checkpoint."
        )

    # ── ONNX export ──────────────────────────────────────────────────────────
    # Dummy input: (batch=1, seq_len=512) of token IDs.
    # Adjust the input shape to match Kronos's tokenizer output if needed.
    print(f"[kronos_torch_to_onnx] Exporting to {output_path} …")
    dummy_input = torch.zeros(1, 512, dtype=torch.long)
    try:
        torch.onnx.export(
            model,
            (dummy_input,),
            str(output_path),
            input_names=["input_ids"],
            output_names=["logits"],
            dynamic_axes={
                "input_ids": {0: "batch_size", 1: "sequence_length"},
                "logits": {0: "batch_size", 1: "sequence_length"},
            },
            opset_version=17,
            do_constant_folding=True,
        )
    except Exception as exc:
        sys.exit(
            f"[kronos_torch_to_onnx] ONNX export FAILED: {exc}\n"
            "If the error is about unsupported ops, trigger the Q3 fallback:\n"
            "  Route to architect per ADR-0027 § Q3 — do NOT silently switch to subprocess."
        )

    print(f"[kronos_torch_to_onnx] Export OK: {output_path} ({output_path.stat().st_size / 1e6:.1f} MB)")

    # ── SHA-256 digest ───────────────────────────────────────────────────────
    sha256 = hashlib.sha256()
    with open(output_path, "rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            sha256.update(chunk)
    digest = sha256.hexdigest()

    sha256_path = output_path.with_suffix(".onnx.sha256")
    sha256_content = f"""# SHA-256 of kronos-base.onnx (NeoQuasar/Kronos-base, HF revision: {args.revision})
#
# Auto-generated by scripts/dev/kronos_torch_to_onnx.py.
# Do NOT edit manually.
# Update by re-running the conversion script.
#
# License: NeoQuasar/Kronos weights are MIT-licensed.
# See: https://github.com/shiyu-coder/Kronos
{digest}
"""
    sha256_path.write_text(sha256_content)
    print(f"[kronos_torch_to_onnx] SHA-256: {digest}")
    print(f"[kronos_torch_to_onnx] Checksum written to: {sha256_path}")
    print()
    print("[kronos_torch_to_onnx] NEXT STEPS:")
    print("  1. git lfs install  (if not done yet)")
    print("  2. git add crates/forecast/assets/kronos-base.onnx")
    print("  3. git add crates/forecast/assets/kronos-base.onnx.sha256")
    print("  4. git commit -m 'chore(forecast): vendor Kronos-base ONNX checkpoint [LFS]'")
    print()
    print("[kronos_torch_to_onnx] Also add the MIT license tag file:")
    license_path = output_path.parent / "kronos-base.onnx.license"
    license_path.write_text(
        "SPDX-License-Identifier: MIT\n"
        "Source: https://huggingface.co/NeoQuasar/Kronos-base\n"
        f"HF Revision: {args.revision}\n"
        "Upstream: https://github.com/shiyu-coder/Kronos (AAAI 2026)\n"
    )
    print(f"  git add {license_path}")


if __name__ == "__main__":
    main()
