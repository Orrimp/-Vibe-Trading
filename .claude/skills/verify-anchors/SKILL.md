---
name: verify-anchors
description: Verify the 9 locked backtest body-SHA-256 anchors in spec/anchors.toml against the latest matching reports under spec/reports/. Use as a hard gate before VERDICT → PASS in any tester run that touched strategy, audit, exec, or backtest code. Also use after any change that modifies report rendering. A single FAIL routes HANDOFF → developer with the body diff.
---

# verify-anchors

Single-command regression gate for the 9-anchor body-only SHA-256 contract.

## Procedure

1. Run the verifier:

   ```bash
   scripts/verify_anchors.sh
   ```

   Exit code 0 = all 9 PASS. Non-zero = at least one FAIL or MISS.

2. On FAIL the script prints `expected`, `actual`, and the report file path.
   Compute the diff of the body bytes to localize the drift:

   ```bash
   # Compare two report bodies (front-matter stripped).
   diff <(awk '/^---$/{n++; next} n>=2' spec/reports/<old>.md) \
        <(awk '/^---$/{n++; next} n>=2' spec/reports/<new>.md)
   ```

3. On MISS (no report for a scenario) re-run the backtest first:

   ```bash
   cargo run --release --bin backtest -- --scenario <name>
   ```

   Then re-verify.

## Routing

- **All PASS** → tester proceeds with VERDICT line. Anchors are good.
- **FAIL** → `HANDOFF → developer` with the body diff. Most likely cause is
  a metadata field that should have been in YAML front-matter but leaked
  into the body (HF-1 / T715 pattern). The developer agent's body-vs-front-
  matter checklist exists for exactly this.
- **MISS** for a brand-new scenario the architect added → `HANDOFF →
  developer` to run it; once stable across two runs, append to
  `spec/anchors.toml`.

## When to invoke

Mandatory:
- Any tester run that touched `crates/strategy/`, `crates/audit/`,
  `crates/exec/`, `crates/backtest/`, or report rendering.
- Final tester gate before any version ship (T_FINAL_*).

Optional but cheap:
- After every developer fan-out merge, before declaring DONE.

## Adding a new anchor

Architect approves the new scenario; tester locks it once it is byte-
identical across two sequential `--release` runs at the same seed:

```bash
cargo run --release --bin backtest -- --scenario <name>  # run 1
cargo run --release --bin backtest -- --scenario <name>  # run 2
scripts/hash_report.py spec/reports/backtest-*-<name>.md  # compare two bodies
# If hashes match -> append to spec/anchors.toml under the new version.
```

## What this skill does NOT do

- Does not run backtests. That is `backtest`.
- Does not modify reports. They are append-only.
- Does not auto-update `spec/anchors.toml` on FAIL — that requires architect approval.
