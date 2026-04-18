---
name: rust-validate
description: Validation pipeline for the Rust workspace — fmt, clippy, audit, deny, docs. Use on every change before tests, and as a gate before any merge. Fails loudly; never auto-fixes without confirmation.
---

# rust-validate

Non-negotiable quality gates.

## Procedure

Run in order; stop and report on the first failure.

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
