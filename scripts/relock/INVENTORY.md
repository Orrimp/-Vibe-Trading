# Story 1-26 — the 34 surfaces, and how to derive their invocations

**Status: inventory CONFIRMED, invocations NOT YET DERIVED.** `surfaces.tsv` is
deliberately absent; `run_surfaces.sh` refuses to run without it rather than executing a
guess. Written 2026-09-25.

## The inventory is confirmed, two independent ways

AC2 names "`#86`, `#87`, `#88`, `#89`, `#90`, `#91`, `#92-#99`, `#100-#107`, `#108-#119`
= 34". That numbering is **positional in `evidence/anchors.toml`** — only 5 of the 119
rows carry a `anchor #NN` comment, and all 5 match their index exactly (#87→87, #88→88,
#89→89, #90→90, #91→91).

Cross-check on content: positions 86-119 are **34 anchors, all 34 of them
`*-theta-surface-*` scenarios, zero others**. A wrong offset would have mixed unrelated
scenarios in. The family grouping also reproduces AC2's exactly:

| anchors | family |
|---|---|
| #86 | `v1-momentum` |
| #87 | `v1-mr` |
| #88, #89 | `v1-carry` × {2023, 2024} |
| #90, #91 | `v1-ts-momentum` × {2023, 2024} |
| #92–#95 | `v1-ts-horizon-{4h,daily}` × {2023, 2024} |
| #96–#99 | `v1-carry-horizon-{4h,daily}` × {2023, 2024} |
| #100–#107 | `v1-basis-reversal-fee{00,02,05,10}bps` × {2023, 2024} |
| #108–#119 | `v2-mn-{basis,funding,basisperp}-fee{00,05}bps` × {2023, 2024} |

## The grid mapping is near-mechanical

`GridKind` (`crates/backtest/src/sweep_harness.rs:224`) has exactly the variants the
families need: `Tier1`, `MrTier1`, `CarryTier1`, `TsTier1`, `TwoCell`, `Ts4h`, `TsDaily`,
`Carry4h`, `CarryDaily`, `BasisTier1`, `MnTier1`.

## Why the rest is derivable WITHOUT running anything

`param_robustness_sweep` is self-describing. Every axis has a `required_*` accessor keyed
on the grid:

`required_direction` · `required_selection_mode` · `required_horizon` ·
`required_slippage_bps` · `required_paths` · `required_ensemble_seed`

**These VALIDATE, they do not default.** `validate_grid_axis_pairing` /
`validate_direction_grid_pairing` reject a mismatched invocation and bail naming the
offending axis, the grid and the required value. The binary's own tests already exercise
forged combinations that would emit an anchor's exact name at the wrong fee.

So the manifest is derivable and, more importantly, **machine-checkable before any
compute**: a test can assert, for each of the 34 scenarios, that the proposed flag tuple
(a) passes both validators and (b) produces that exact scenario name. Nothing needs to
run for hours to find out a row is wrong.

## What remains

1. Derive the flag tuple per surface: grid, year, taker-fee (basis/MN only), MN
   sub-variant, score-source, plus the `required_*` values the validators demand.
2. Write the machine-verification test described above.
3. Only then write `surfaces.tsv`.

## And the run is gated regardless

1-26 AC1: `#69` wired and `#68` dropped must land first. The AC is explicit —
"regenerating before all of these land produces a second contaminated corpus — that is
the whole reason for the split". Both are WORK, not decisions; the operator ruled on both
2026-08-19.
