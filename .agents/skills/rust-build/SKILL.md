---
name: rust-build
description: Build the Rust trading workspace. Use whenever code was changed, before running tests, or when verifying that a feature compiles. Runs cargo check first (fast), then cargo build, surfaces errors verbatim, and fails loudly.
---

# rust-build

Standard build pipeline for this workspace.

## Procedure

1. **Fast check**

   ```bash
   cargo check --workspace --all-targets
   ```

   Stop and report on first error — do not continue to full build.

2. **Debug build** (only if check passed)

   ```bash
   cargo build --workspace --all-targets
   ```

3. **Release build** (only when asked for benchmarks, backtests, or shipping)

   ```bash
   cargo build --workspace --release
   ```

## On failure

- Copy the raw error block into the response. Do not paraphrase compiler diagnostics.
- If it is an obvious missing dependency, propose the `Cargo.toml` edit; do NOT apply it silently — confirm with the user unless the developer agent is the caller.
- Route `HANDOFF → developer` if called by an agent other than developer.

## On success

Report:

- Time taken
- Number of crates built
- Any warnings (count + first three)
