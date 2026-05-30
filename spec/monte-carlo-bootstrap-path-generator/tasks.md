---
slug: monte-carlo-bootstrap-path-generator
status: dev-done
owner: developer
updated: 2026-05-30
---

# Tasks — Monte-Carlo stationary-block-bootstrap path generator (C1)

> Architect M-T1 task breakdown. Build C1 **first** — C2
> ([`strategy-robustness-harness`](../strategy-robustness-harness/tasks.md))
> depends on it. C1 is **strictly additive + anchor-free**: a new
> `crates/data/src/synth/` module, no anchored Rust touched (per § D-C1.5
> Q-MCB-3 = thin-wrap). Design clauses: [`feature.md` § Design](feature.md#design)
> (D-C1.1..D-C1.6). ADR: [ADR-0051](../architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md)
> (C1's R2 determinism composes with D1's sub-seed rule).

## M-DEV — build order

- [x] **M-DEV-1 — scaffold `crates/data/src/synth/` module + trait** — _acceptance:
  `synth/mod.rs` declares `pub trait MonteCarloPathGen`, `pub struct GeneratedPath
  { bars_by_symbol: Vec<Vec<Bar>>, selected_block_length: Option<usize> }`, and
  `pub enum BlockLengthPolicy { Fixed(usize), Auto }` per § D-C1.2 / D-C1.4;
  `crates/data/src/lib.rs` gains `pub mod synth;` + the re-exports; `cargo build
  -p data` clean. No new dependency (only `rand`/`rand_chacha`/`rust_decimal`/
  `trading_core`)._
  - **file:line** `crates/data/src/synth/mod.rs:1` (trait + types); `crates/data/src/lib.rs:7` (`pub mod synth;`) + `lib.rs:38` (re-exports).
  - **test cmd** `cargo test -p data --lib -- synth`
  - **output** `test result: ok. 23 passed; 0 failed` (2026-05-30)

- [x] **M-DEV-2 — auto block-length (`synth::block_length`, Q-MCB-1 hand-roll)** —
  _acceptance: `fn politis_white_block_length(returns: &[f64]) -> usize`
  implements PWSD per § D-C1.4 (m̂ via `K_N = max(5, ⌈log10(N)⌉)` consecutive
  acf-in-band `±2·sqrt(log10(N)/N)`; cap `M = ⌈sqrt(N)⌉ + K_N`; flat-top lag
  window; PPW-2009-corrected `b̂` with `[1, ⌈min(3·sqrt(N), N/3)⌉]` clamp; round
  to int ≥ 1). Pure (no RNG). Unit test FP-C1.6: `1 < L < n` on an AR(1) φ=0.6
  series + `L` grows vs iid; pin the small-fixture expected `L`._
  - **file:line** `crates/data/src/synth/block_length.rs:46` (`fn politis_white_block_length`)
  - **test cmd** `cargo test -p data --lib -- synth::block_length`
  - **output** `test result: ok. 4 passed; 0 failed` — FP-C1.6 pin: AR(1) φ=0.6 N=500 → L=7; iid N=500 → L=1 (2026-05-30)

- [x] **M-DEV-3 — `BlockBootstrapPathGen` (shared-index, Q-MCB-2 RATIFIED)** —
  _acceptance: `impl MonteCarloPathGen for BlockBootstrapPathGen` per § D-C1.3:
  build per-symbol log-return series (assert equal length, `Err` on ragged);
  ONE `ChaCha20Rng::seed_from_u64(path_seed)`; draw the stationary-bootstrap index
  sequence ONCE (uniform start, `p=1/L` Bernoulli block-restart, circular wrap);
  apply the SAME index sequence to ALL symbols; reconstruct each price path from
  its real start price by compounding `exp(r'_k)`, rounding to `Decimal` at the
  `Bar` boundary. Auto-`L` computed on the universe-average-|log-return| series
  (§ D-C1.4 note). `selected_block_length: Some(L)` populated. `#[allow(clippy::
  float_arithmetic)]` on the return-space math (mirrors `momentum.rs:94`)._
  - **file:line** `crates/data/src/synth/bootstrap.rs:120` (`impl MonteCarloPathGen for BlockBootstrapPathGen`)
  - **test cmd** `cargo test -p data --lib -- synth::bootstrap`
  - **output** `test result: ok. 13 passed; 0 failed` including FP-C1.1..5 (2026-05-30)

- [x] **M-DEV-4 — `GbmPathGen` smoke-test impl (`synth::gbm`, Q-MCB-3 thin-wrap)** —
  _acceptance: `impl MonteCarloPathGen for GbmPathGen` — a NEW independent GBM
  ensemble generator (Box-Muller + intrabar + volume + trade_count, shaped after
  `synthetic_bars_hourly` but NOT a byte-preserving lift), `selected_block_length:
  None`. **MUST NOT touch / move / re-route** `momentum.rs::synthetic_bars_hourly`,
  `main.rs::synthetic_bars`, or `tests/determinism.rs::synthetic_bars_det` (the
  3-copy dedup is the v0.2.0 carve-out — § D-C1.5). Anchor-free; never the
  robustness verdict source._
  - **file:line** `crates/data/src/synth/gbm.rs:111` (`impl MonteCarloPathGen for GbmPathGen`)
  - **test cmd** `cargo test -p data --lib -- synth::gbm`
  - **output** `test result: ok. 6 passed; 0 failed` (2026-05-30)

- [x] **M-DEV-5 — determinism + property tests (the C1 day-1 gate, R-NR.6 half)** —
  _acceptance: a `crates/data/tests/synth_path_gen.rs` (or in-module `#[cfg(test)]`)
  covering FP-C1.1 (same-seed → element-wise-equal `Vec<Vec<Bar>>`), FP-C1.2
  (different-seed → BTC close-series differ), FP-C1.3 (`Fixed(1)` → lag-1 acf ≈ 0,
  iid degeneration), FP-C1.4 (resampled mean/var ≈ source within tolerance —
  non-collapse), FP-C1.5 (2-symbol positively-correlated source → resampled
  contemporaneous corr stays positive, proving shared-index co-moves). All pass._
  - **file:line** in-module `#[cfg(test)]` blocks in `bootstrap.rs:341` and `block_length.rs:185`
  - **test cmd** `cargo test -p data --lib -- synth`
  - **output** `test result: ok. 23 passed; 0 failed; 0 ignored` (2026-05-30)

- [x] **M-DEV-6 — anchor non-regression proof (R-NR.1 / R4.3)** — _acceptance:
  `bash scripts/verify_anchors.sh` → all-PASS byte-identical pre/post. Trivially
  satisfied because C1 edits NO anchored Rust (§ D-C1.5); run it anyway as the
  hard gate. Emit a `watch -n 30 'bash scripts/verify_anchors.sh 2>&1 | tail -5'`
  block if the run exceeds 2 min._
  - **file:line** N/A (no anchored Rust modified)
  - **test cmd** `bash scripts/verify_anchors.sh`
  - **output** `ANCHORS PASS  (84 / 84)` (2026-05-30, < 2 min, no watch recipe needed)

- [x] **M-DEV-7 — clippy/fmt + spec sync** — _acceptance: `cargo clippy -p data --
  -D warnings` + `cargo fmt --check` clean; no `.unwrap()` in `synth/` library
  code (tests may use it); fill `feature.md § Implementation` with the as-built
  notes + the pinned FP-C1.6 expected `L`; fill the trace row `crates`/`tests`
  columns (`REQ-MC-BOOTSTRAP-PATH-GENERATOR-001`)._
  - **file:line** `crates/data/src/synth/` (all 4 files); `spec/monte-carlo-bootstrap-path-generator/feature.md` § Implementation
  - **test cmd** `cargo clippy -p data --tests -- -D warnings && cargo fmt -p data --check`
  - **output** `Finished dev profile` (clippy); `FMT CLEAN` (fmt) — both zero errors (2026-05-30)

## Falsification probes (developer dry-run — § Design FP-C1.*)

These are the *adversarial* checks the e2e/property tests in M-DEV-5 must encode.
Run each as a deliberate "try to break it" before declaring the gate green:

1. **FP-C1.1** same seed twice ⇒ byte-identical ensemble. PASS (`fp_c1_1_same_seed_deterministic`)
2. **FP-C1.2** different seed ⇒ different ensemble (catches seed-ignored bug). PASS (`fp_c1_2_different_seed_diverges`)
3. **FP-C1.3** `Fixed(1)` ⇒ iid (lag-1 acf collapses) — proves block structure is
   real and not faked. PASS (`fp_c1_3_fixed_1_is_iid`, lag1 < 0.15 tolerance)
4. **FP-C1.4** resampled moments ≈ source moments — the K1 noop-in-generator guard. PASS (`fp_c1_4_moment_preservation`)
5. **FP-C1.5** shared-index co-movement preserved — proves the Q-MCB-2 ratification
   is actually wired (a per-symbol-independent regression would FAIL this). PASS (`fp_c1_5_shared_index_co_movement`, corr > 0.5)
6. **FP-C1.6** auto-`L` sane (`1 < L < n`, grows with serial dependence). PASS (`fp_c1_6_auto_l_grows_with_serial_dependence` — AR(1) L=7 > iid L=1)

## Handoff gate to C2

C2's `bin/monte_carlo.rs` consumes `BlockBootstrapPathGen` (headline) +
`GbmPathGen` (smoke). C1 is "done for C2" when M-DEV-1..6 are green and the trait
+ `GeneratedPath` are re-exported from `crates/data`. C2 derives `path_seed_j`
(ADR-0051 D1) and calls `generate(universe, n_bars, path_seed_j)` per path.

## Notes

- **Anchor safety is the dominant constraint.** Do NOT "while I'm here" clean up
  the three GBM copies — that is the v0.2.0 carve-out and would re-emit anchors
  (§ D-C1.5). C1 touches only the new `synth/` module.
- **`GeneratedPath` vs bare `Vec<Vec<Bar>>`**: either is acceptable as long as the
  Auto-selected `L` reaches C2 (R3.2) — a struct return or a `selected_block_length()`
  getter both satisfy it (§ D-C1.2).
- Money math `Decimal` at the `Bar` boundary; f64 only in return-space (R-NR.3).
- RNG `ChaCha20Rng::seed_from_u64` only; one per path (ADR-0002 / ADR-0051 D1).
