# Story 1-24 — PWSD fidelity re-lock, errata of record (2026-10-10)

AC4 requires *"an errata/decision note records old-vs-new L values per scenario and the verdict
deltas"*. This is it. The reasoning, the operator rulings and the superseded digests also sit in
`evidence/anchors.toml` beside the rows — this note is the per-scenario table.

## What moved the numbers

Three defects and one unratified policy, all in `crates/data/src/synth/`:

1. **Window scale** — `λ(k/m̂)` → `λ(k/(2m̂))`, PW-2004's convention.
2. **m̂ search** — `ρ̂(m..m+K−1)` → `ρ̂(m+1..m+K)`, PW-2004's convention.
3. **Zero-variance guards** — absolute ε → scale-relative, at **both** sites. The module doc's
   deviation #3 named one; there were two, and fixing only the first left the estimator
   magnitude-sensitive: a ±1e-9 series returned L=1 while its 1e9-scaled twin returned L=68.
4. **`Auto` aggregation** — cross-symbol mean of `|r|` → per-symbol estimate, `max` reduction.
   PWSD is scale-invariant per series, so the old rule's defect existed *only* because averaging
   preceded estimation. `max` was selected by a criterion registered before the numbers existed;
   `standardized_mean_abs` (89) and `per_symbol_median` (67) both came out **shorter** than the
   shipped 115 on the measurable surfaces, which is the adverse direction.

Two fixes landed **anchor-neutral**, proven rather than argued, and that is what makes the
attribution above possible: the ε guard and the AC3 volume proxy were measured against
`v1-carry-horizon-daily-2024`, which reproduced `0427481b…` byte-identically. So every digest
change below belongs to items 1 and 4.

## AC4's direction guard

Expectation: *longer L ⇒ crowns same or fewer ⇒ ship-passive strengthens.* **33 of 35 held; 2 did
not**, and the guard is why that is a sentence in a document rather than an unnoticed change.

`MARGINAL` is crown-eligible — `bakeoff/robustness.rs:26`, *"the comparator treats `Fragile` as
the **only** ineligible-to-crown flag"* — so FRAGILE→MARGINAL is a crown-eligibility change, not a
cosmetic band. **No cell reached ROBUST anywhere in the 35; nothing is crowned.**

Why it inverted on those two: longer blocks widen the null, which held 33 times — but a longer
block also preserves more trend and persistence structure, and for a trend-following or reversal
cell that can lift p50/p95 rather than widen the spread. On `v1-basis-reversal-fee05bps-2023` it is
visible directly: p50 **fell** (0.061623 → 0.057358) while p95 rose (0.806264 → 0.855675) enough to
cross the band. AC4 called its own claim an *expectation*, not a theorem.

Operator ruled 2026-10-10: **disclose and land.** These are research surfaces; `README.md` §
Status already records that the bakeoff gate resamples returns and never re-executes fills, so
crowns, verdicts and the era-qualified ship-passive conclusion stand independently of them.

## Per-scenario table

| scenario | L old → new | crown-eligible cells | family verdict | body |
|---|---|---|---|---|
| `v1-basis-reversal-fee00bps-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 5 → 5 | `FAMILY-HAS-NON-FRAGILE-CELLS` | `3dde4b93…` → `c8c2d2d1…` |
| `v1-basis-reversal-fee00bps-theta-surface-2024-block-bootstrap-real-fy` | 200 → 282 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `f20d4af1…` → `ee0dae2b…` |
| `v1-basis-reversal-fee02bps-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 5 → 5 | `FAMILY-HAS-NON-FRAGILE-CELLS` | `444df215…` → `5f7f152b…` |
| `v1-basis-reversal-fee02bps-theta-surface-2024-block-bootstrap-real-fy` | 200 → 282 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `343b6203…` → `61dd4b19…` |
| `v1-basis-reversal-fee05bps-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 2 → 3 **⚠ADVERSE** | `FAMILY-HAS-NON-FRAGILE-CELLS` | `1d53187c…` → `3b4d1e57…` |
| `v1-basis-reversal-fee05bps-theta-surface-2024-block-bootstrap-real-fy` | 200 → 282 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `b7f0ed98…` → `2ec2aacd…` |
| `v1-basis-reversal-fee10bps-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 2 → 2 | `FAMILY-HAS-NON-FRAGILE-CELLS` | `df189f47…` → `6a269324…` |
| `v1-basis-reversal-fee10bps-theta-surface-2024-block-bootstrap-real-fy` | 200 → 282 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `167e7a03…` → `d0dd1927…` |
| `v1-carry-horizon-4h-theta-surface-2023-block-bootstrap-real-fy` | 80 → 124 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `8807fdb2…` → `2ca33645…` |
| `v1-carry-horizon-4h-theta-surface-2024-block-bootstrap-real-fy` | 49 → 134 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `59cb1ff8…` → `67987f93…` |
| `v1-carry-horizon-daily-theta-surface-2023-block-bootstrap-real-fy` | 9 → 22 | 4 → 3 | `FAMILY-HAS-NON-FRAGILE-CELLS` | `0c4273e6…` → `c0d09c1d…` |
| `v1-carry-horizon-daily-theta-surface-2024-block-bootstrap-real-fy` | 3 → 22 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `0427481b…` → `26643849…` |
| `v1-carry-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 2 → 1 | `FAMILY-HAS-NON-FRAGILE-CELLS` | `1f3dcc49…` → `4c1a0011…` |
| `v1-carry-theta-surface-2024-block-bootstrap-real-fy` | 200 → 282 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `4cf7f277…` → `83ae9abe…` |
| `v1-momentum-2023-block-bootstrap-real-fy-mc` | 204 → 281 | 0 → 0 | `—` | `937cb52f…` → `74c018d5…` |
| `v1-momentum-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `9d69176b…` → `54aab726…` |
| `v1-mr-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `523c2eda…` → `d4a9ac13…` |
| `v1-ts-horizon-4h-theta-surface-2023-block-bootstrap-real-fy` | 80 → 124 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `0b39ccf2…` → `23efeb69…` |
| `v1-ts-horizon-4h-theta-surface-2024-block-bootstrap-real-fy` | 49 → 134 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `cb86000a…` → `447f1ed0…` |
| `v1-ts-horizon-daily-theta-surface-2023-block-bootstrap-real-fy` | 9 → 22 | 0 → 1 **⚠ADVERSE** | **`FAMILY-UNIFORM-FRAGILE` → `FAMILY-HAS-NON-FRAGILE-CELLS`** | `8198ed36…` → `b34e4c88…` |
| `v1-ts-horizon-daily-theta-surface-2024-block-bootstrap-real-fy` | 3 → 22 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `bafd1f4e…` → `0e4b93f5…` |
| `v1-ts-momentum-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `bc47e196…` → `60279c79…` |
| `v1-ts-momentum-theta-surface-2024-block-bootstrap-real-fy` | 200 → 282 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `112283e5…` → `61373d65…` |
| `v2-mn-basis-fee00bps-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `82528944…` → `9ab47036…` |
| `v2-mn-basis-fee00bps-theta-surface-2024-block-bootstrap-real-fy` | 200 → 282 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `9ffbde26…` → `b119b97d…` |
| `v2-mn-basis-fee05bps-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `7ed6add7…` → `8dc57123…` |
| `v2-mn-basis-fee05bps-theta-surface-2024-block-bootstrap-real-fy` | 200 → 282 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `9c23740e…` → `e053a8c7…` |
| `v2-mn-basisperp-fee00bps-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `f2be5aec…` → `0a570e1d…` |
| `v2-mn-basisperp-fee00bps-theta-surface-2024-block-bootstrap-real-fy` | 200 → 282 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `5a7cccfe…` → `c6352ab0…` |
| `v2-mn-basisperp-fee05bps-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `164fb81e…` → `72d0e1d9…` |
| `v2-mn-basisperp-fee05bps-theta-surface-2024-block-bootstrap-real-fy` | 200 → 282 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `78a0df0a…` → `0acde522…` |
| `v2-mn-funding-fee00bps-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `ac134d55…` → `1c915caa…` |
| `v2-mn-funding-fee00bps-theta-surface-2024-block-bootstrap-real-fy` | 200 → 282 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `bd7861ed…` → `c6eb9b01…` |
| `v2-mn-funding-fee05bps-theta-surface-2023-block-bootstrap-real-fy` | 204 → 281 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `e9024299…` → `93c31b79…` |
| `v2-mn-funding-fee05bps-theta-surface-2024-block-bootstrap-real-fy` | 200 → 282 | 0 → 0 | `FAMILY-UNIFORM-FRAGILE` | `735a6efe…` → `579d9229…` |

## The clamp, disclosed

`281`/`282` is the clamp `ceil(min(3·√N, N/3))`, not an estimate. On the 26 full-year hourly
surfaces the literature convention **saturates** it — raw `b̂` is 329.6 against a cap of 281 — so
`selected_block_length_L` there reports the **cap**, and all four aggregation policies collapse
onto it. The operator ruled to keep the clamp; this paragraph is the disclosure that ruling
requires. `politis_white_b_hat_unclamped` exposes the raw `b̂` through the same single pipeline, so
the cap's influence is measurable rather than hidden.
