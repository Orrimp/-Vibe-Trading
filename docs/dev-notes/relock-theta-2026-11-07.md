# θ-surface re-lock — `#144` (TS gross cap) + `#149` (MN table) — 2026-11-07

**Landed.** 34 surfaces regenerated (0 failed), **18 re-emitted** under ADR-0038 § D6.b,
**16 left alone**. `verify_anchors.sh` 119/119 before and after; `spec_lint` PASS.

## The 16 that did not move are the result worth reading first

basis-reversal ×8, carry-horizon ×4, carry ×2, momentum, mr — all regenerated
**byte-identically**. Affectedness measured rather than assumed, and it bounds the blast radius
of two renderer changes plus a config change to exactly the families they were aimed at. Had any
of those 16 moved, something was wrong with the change rather than with the corpus.

## `#144` — the TS freeze is gone, and the arm got WORSE

The 6 TS surfaces were re-run with the gross cap at **1.00**, the TS grids' own declared target,
instead of 0.50.

| surface | gross-cap refusals, before → after | family verdict |
|---|---|---|
| `ts-momentum` 2023 / 2024 | 388 651 / 416 352 → **0** | UNIFORM-FRAGILE (unchanged) |
| `ts-horizon-daily` 2023 | 174 307 → **0** | **HAS-NON-FRAGILE-CELLS → UNIFORM-FRAGILE** |
| `ts-horizon-daily` 2024 | 179 674 → **0** | UNIFORM-FRAGILE (unchanged) |
| `ts-horizon-4h` 2023 / 2024 | 97 853 / 98 159 → **0** | UNIFORM-FRAGILE (unchanged) |

**Refusals go to zero on all six.** At 0.50 the all-or-nothing cap skipped the WHOLE rebalance
whenever 6 or more of the 10 names signalled long — i.e. selectively at peak breadth, which for a
long-only absolute-momentum arm is when being long matters most. At 1.00 the arm's target can no
longer exceed its cap, so the breach path is unreachable by construction and the arm rebalances
freely.

**One family verdict MOVED, and it moved toward the thesis.**
`v1-ts-horizon-daily-theta-surface-2023` goes **`FAMILY-HAS-NON-FRAGILE-CELLS` →
`FAMILY-UNIFORM-FRAGILE`**. Let the arm trade at the exposure it was specified for and it
performs *worse*: the cells that had been non-fragile are fragile once unfrozen. The
"uniformly fragile" sub-claim that the 1-26 re-lock had withdrawn on this surface is restored
for it.

**No crown eligibility was gained anywhere.** `Fragile` is the only ineligible-to-crown flag, and
after this re-lock all six TS surfaces are uniformly fragile. So the era-qualified ship-passive
conclusion is not weakened by removing the freeze — it is marginally strengthened, which is the
honest outcome for a correction nobody could predict the direction of in advance.

This also closes the question `#144` raised and could not answer: *was the TS edge there, hidden
behind a cap?* Measured: **no.** The arm was handicapped, the handicap is gone, and it still
fails the frozen gate.

## `#149` — the MN tables render, verified on the artifact

12 MN surfaces re-emitted with the delimiter row corrected (19 cells, was 18) and the legend
rewritten so the current meaning leads instead of a superseded present-tense caveat.

Checked on the **real regenerated output**, not a fixture: every delimiter row in all 34 bodies
matches its header's cell count — **0 malformed tables**. The gate
`param_sweep_e2e::bug149_every_rendered_table_is_well_formed_under_gfm` covers five render
branches and was probed RED beforehand.

No MN family verdict moved. The gross-cap refusal counts the MN surfaces now print
(49 617 … 132 021) are `#141`'s disclosure, unchanged by this pass — the MN lane's cap stays at
0.50 because its gross target is ~0.60 and that bind is real, not an artifact.

## No projection re-lock was owed

`theta_surface_reproduction.rs` reads its expected digest from `evidence/anchors.toml` **at run
time** via `anchored_sha(scenario)` rather than holding a copy. So none of the 18 moved digests
had a stale in-test twin — swept and confirmed. That is the design that avoids `#143`'s entire
failure class, and it is the contrast worth drawing: the lane re-lock needed **four** separate
projection re-locks, this one needed none.
