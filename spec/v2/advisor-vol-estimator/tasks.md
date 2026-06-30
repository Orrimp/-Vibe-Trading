---
slug: advisor-vol-estimator
status: dev-done
owner: developer
updated: 2026-06-30
---

# Tasks — P1-5 Shared σ̂ Vol Estimator

## Completed by developer (2026-06-30)

- [x] T1 — Create `crates/strategy/src/vol_estimator.rs` with the 4 public functions.
  - file: `crates/strategy/src/vol_estimator.rs:1` (new file, 680 lines)
  - test: `cargo test -p strategy --lib vol_estimator`
  - output: `test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 210 filtered out; finished in 0.00s`

- [x] T2 — Register `pub mod vol_estimator;` in `crates/strategy/src/lib.rs`.
  - file: `crates/strategy/src/lib.rs:28` (added `pub mod vol_estimator;` with rationale comment)
  - test: included in `cargo test -p strategy --lib`
  - output: `test result: ok. 233 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`

- [x] T3 — Write inline unit tests for all 4 functions + λ constants.
  - `log_returns_from_bars`: 5 tests (empty, single, constant, 2×step, known sequence)
  - `realized_vol_from_returns`: 5 tests (constant=0, window=0, window>len, known sample, empty)
  - `ewma_realized_vol`: 5 tests (empty, length, λ=0, λ=1, monotone-weight property)
  - `har_realized_vol`: 4 tests (empty, length, all-positive, spike-damping/persistence)
  - λ constant sanity checks: 2 tests (126d-daily ≈126 bars, 126d-hourly ≈3024 bars)
  Total: 23 tests in the module's `#[cfg(test)]` block.

- [x] T4 — Create `spec/v2/advisor-vol-estimator/feature.md` + `tasks.md`.
  - file: `spec/v2/advisor-vol-estimator/feature.md`
  - file: `spec/v2/advisor-vol-estimator/tasks.md` (this file)
  - test: `python3 scripts/spec_lint.py` → `spec-lint: PASS (0 violations)`

- [x] T5 — Add `REQ-V2-P1-5-VOL-ESTIMATOR-001` row to `spec/trace.toml`.
  - file: `spec/trace.toml:3302` (new `[[req]]` block appended)
  - test: `python3 scripts/spec_lint.py` → `spec-lint: PASS (0 violations)`

## For the tester to verify

- T_FINAL_1 — `cargo test -p strategy` clean (all tests including vol_estimator pass).
- T_FINAL_2 — `cargo clippy -p strategy --tests -- -D warnings` clean.
- T_FINAL_3 — `cargo fmt --check` clean.
- T_FINAL_4 — `bash scripts/verify_anchors.sh` 119/119 (additive module; no engine path).
- T_FINAL_5 — `python3 scripts/spec_lint.py` PASS.
