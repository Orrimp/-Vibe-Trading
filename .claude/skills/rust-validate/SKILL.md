---
name: rust-validate
description: Validation pipeline for the Rust workspace — fmt, clippy, audit, deny, docs. Use on every change before tests, and as a gate before any merge. Fails loudly; never auto-fixes without confirmation.
---

# rust-validate

Non-negotiable quality gates.

## Procedure

Run in order; stop and report on the first failure.

0. **Pre-test grep gates** (defence-in-depth, non-deterministic-path /
   look-ahead / secret-leak backstops — cheap, run first):

   ```bash
   bash scripts/check_no_clocks_in_ui_tests.sh
   bash scripts/check_no_raw_asof_join.sh
   ```

   `check_no_raw_asof_join.sh` forbids a raw, hand-rolled time-keyed
   as-of join (`partition_point`/`binary_search_by*` on a `t <= query`
   predicate) anywhere under `crates/*/src/**` outside the sanctioned
   `crates/core/src/pit.rs` home — the ADR-0086 D1 look-ahead lint.
   `--self-test` (synthetic offending + clean fixtures) verifies the
   matcher itself; run it once after touching either script.

1. **Formatting**

   ```bash
   cargo fmt --all -- --check
   ```

2. **Lints** — warnings are errors:

   ```bash
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   ```

3. **Dependency security**

   ```bash
   cargo audit
   ```

   Install with `cargo install cargo-audit` if missing; ask the user before installing.

4. **Policy** (licenses, bans, sources) — if `deny.toml` is present:

   ```bash
   cargo deny check
   ```

5. **Docs build**

   ```bash
   RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
   ```

## Auto-fix policy

- `cargo fmt` auto-fix is allowed **only** when the caller is the developer agent
  acting on its own code.
- `cargo clippy --fix` is **never** auto-applied — propose the patch, require
  explicit go-ahead.

## Reporting

Return a table of `{ step, status, duration, top 3 findings }`.
If all pass, one line is fine: `VALIDATE → PASS (fmt, clippy, audit, deny, doc)`.
