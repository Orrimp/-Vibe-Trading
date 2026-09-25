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

## DONE 2026-09-25 — the manifest is derived and proven

`crates/backtest/tests/relock_manifest.rs` enumerates every axis tuple the binary would
ACCEPT and keeps the ones that name an anchored surface. All **34 derive from exactly one
accepted tuple** — no gaps, no ambiguity, first run. Nothing in `surfaces.tsv` was typed
by hand; re-generate it rather than editing:

```
cargo test -p backtest --test relock_manifest write_manifest -- --ignored
```

The always-on gate (`every_anchored_surface_derives_from_exactly_one_accepted_tuple`)
keeps the committed file honest: if a surface ever stops being reachable, that is drift
to report, not a row to hand-write. A second test asserts the known forgery
(`--grid tier1 --taker-fee-bps 20`, which emits anchor `#86`'s exact name at a fee it
never ran) is still refused — if that ever passes, the derivation is filtering nothing.

## The entry gate is open

1-26 AC1 required `#69` wired and `#68` dropped. Verified 2026-09-25: `#69` was wired
2026-08-23 (`723ca742`) with its binding test green, and `#68` took the other branch of
its own "implement-or-drop" — the two were one defect, so wiring the sizer made the drift
axis live. **Operator ruled 2026-09-25 that implemented-and-binding-tested satisfies the
gate.** See 1-26 AC1's annotation.

## Measured cost — the story's estimate is right for ONE family and wrong for the rest

Every distinct invocation shape was run once at `RAYON_NUM_THREADS=8`, 2026-09-25:

| family | surfaces | measured | note |
|---|---|---|---|
| `tier1` momentum (1h) | 1 | **1677 s ≈ 28 min** | matches the story: 18.1 min × 12.4/8 = 28.1 |
| `mr` / `carry` / `ts-tier1` (1h) | 5 | not individually measured | same shape as momentum |
| `basis-tier1` (1h) | 8 | **~40 s** | the story calls this the HEAVIEST lane |
| `mn-tier1` (1h) | 12 | **23 s** | |
| `ts-4h` / `carry-4h` | 4 | **14 s** | |
| `ts-daily` / `carry-daily` | 4 | **11 s** | 1000 paths, not 200 |

So the cost is concentrated in the six `tier1`-family 1h surfaces; **everything else together
is about 12 minutes**. If the other five behave like momentum the whole run is **≈3 h at 8
threads**, and it is hard to construct a case above ~6 h.

**The story's "10.3 h floor, 15-20 h realistic" does not survive contact.** Its cost model
also has the ordering backwards — it calls basis-reversal the heaviest lane (measured: 40 s)
and momentum the cheap end (measured: 28 min). Treat the note as an estimate from one
measurement generalised too far, and this table as the replacement.

## What remains before the compute window

Nothing mechanical:

```
scripts/relock/run_surfaces.sh --out-dir evidence/v2/harness-relock/reports
```

Resumable, `nice -n 19`, builds its own binary with `--features candle,realdata`. Then AC4
(errata + verdict re-derivation) and AC5 (band re-examination) on the output.
