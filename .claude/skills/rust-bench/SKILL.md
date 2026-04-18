---
name: rust-bench
description: Run criterion benchmarks on latency-sensitive paths (order book updates, feature calculation, inference). Use when the developer changes a hot path or the tester needs regression data. Compares to previous baseline and emits a delta table.
---

# rust-bench

## Procedure

1. Ensure a release build exists:

   ```bash
   cargo build --workspace --release
   ```

2. Run benchmarks:

   ```bash
   cargo bench --workspace
   ```

   Criterion writes reports to `target/criterion/`.

3. Compare to the previous baseline (criterion does this automatically when
   the baseline is saved under the name `main`):

   ```bash
   cargo bench --workspace -- --save-baseline current
   cargo bench --workspace -- --baseline main
   ```

4. Emit a table: `{ benchmark, median (µs), change vs main (%), verdict }`.

5. Flag anything regressing more than 5% (configurable in
   `spec/architecture.md` under "Performance budget").

## Reporting

- Embed the delta table into the caller's test report under "Benchmarks".
- If regressions found and caller is an agent, `HANDOFF → architect`.
