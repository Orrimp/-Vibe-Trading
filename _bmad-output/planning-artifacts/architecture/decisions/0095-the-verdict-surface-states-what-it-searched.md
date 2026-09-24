---
adr: 0095
title: The verdict surface states what it searched, and what would have to change
status: accepted
date: 2026-09-24
supersedes: none
superseded-by: none
---

# ADR-0095: The verdict surface states what it searched, and what would have to change

## Context

The 2026-08-04 product review found five ways the advisor's honesty lived in the
machinery but not on screen. Two mattered most:

- **Finding 1** — the honest-null screen and a screen whose search had silently failed
  were *pixel-identical*. "No strategy beat holding" and "we meant to run twelve,
  something broke, and here is what came back" rendered the same. The screen's honesty
  depended entirely on nothing having gone wrong.
- **Finding 13** — after "hold", the user was told what not to do and then nothing. A
  complete finding and an incomplete answer.

Story 3-20 was sequenced behind story 6-9's embedded-font fix, because AD-10 pixel
proof was impossible while the baseline gate was red. ADR-0093 cleared that on
2026-09-16.

## Decision

**D1 — The mirror carries what the run ASKED FOR, not only what came back.**
`BakeoffReportMirror::requested_arms` is a value-echo of `BakeoffRequest::field` at the
single `from_report` boundary — the same pattern as `run_seed`: no engine change, no
computation, no new dependency edge. It is deliberately NOT derived from `rows`. `rows`
is the output set; `requested_arms` is the input set, and **the difference between them
is the entire finding**. An arm requested that produced no candidate is a silent search
failure, and without the input set on screen it cannot be told from "we ran it and it
lost".

**D2 — The screen states completeness in the affirmative, and incompleteness loudly.**
Under "Strategies tried": *"Searched N of M strategies — K produced trades on this
data"*, then the arm names (from the live request, never a hardcoded list) and the
window. When arms are missing, that line becomes a `DOWN_500` warning naming the count
and saying the ranking **must not be read as "nothing beat holding"**.

An arm that produced no trades is counted separately from one that lost. It did not
lose — it never played, and conflating the two is how a non-test becomes evidence.

**D3 — The standing qualifier has exactly one definition.**
`strings::LEADERBOARD_STANDING_QUALIFIER`. The AC is not "show a caveat" — it is that
the screen and the record cannot drift, which is only true while one place says it.
`honesty_surface_wording::the_standing_qualifier_is_defined_once` scans `crates/ui/src`
and fails the build on a second copy.

The wording states three things, and a test enforces all three, because any two of them
alone would mislead: the direction of the #67 finding is **preserved**, its magnitudes
are **not final**, and the ranking above **does not rest on it** — #67 verified that
`bakeoff/bootstrap.rs` resamples candidate equity curves and never re-executes fills.
A caveat with only the first two would be alarming and wrong.

**D4 — After "hold", the screen names the gate signal that decided it, or says nothing.**
`next_step_after_hold` renders only on `BenchmarkWins` / `AllFragile`, and picks the
reason from the run's own numbers in the order the gates apply: *never traded* (first,
because reading a non-test as a loss is the costliest misreading available here) →
*fragile* → *lost to holding* → *lost to the search count*. If an arm beat holding, held
up under resampling, and cleared the search count yet holding was crowned, the panel
renders **nothing**: that combination should not arise, and inventing plain language for
it would be a guess presented as a finding.

The cadence's load-bearing sentence is the first: *re-running today gives the same
answer — the bake-off is seeded, so only new data can change it.* It is the part that
stops a user re-running a deterministic computation hoping for a different result.

**D5 — The next step renders BELOW the ranked table.** It pairs with the pick, but
ADR-0092 re-ruled the pane order on measurement and the fold is a requirement
(bug-log #96). Measured after this change: the trust block spans rows **473..903** of a
1080 fold, so the scorecard, the pick and the first ranked rows all still clear it.

**D6 — AC3 was already met; it is now guarded rather than rebuilt.**
`LEADERBOARD_SCORECARD_CAPTION` already read "it never changes the result" and already
rendered inside the block ADR-0092 put above the fold. A pixel gate cannot read text, so
the wording is asserted on the constants
(`crates/ui/tests/honesty_surface_wording.rs`) and the fact that those constants reach
the screen is asserted at the pixels. Neither half is sufficient; claiming the pixel
gate covered the wording would be the proxy AD-10 exists to refuse.

**D7 — AC1's "data revision" ships as a STATED LIMITATION, not a placeholder.**
Investigated at source: there is **no single per-run data revision**.

- What exists is an aggregate SHA-256 **per corpus directory**
  (`data/binance` = `3a8b96c4…`), computed by
  `data::revision::read_and_verify_revision_manifest` and **discarded** at
  `crates/backtest/src/bakeoff/mod.rs:424`. A BTCUSDT run's "revision" would be a hash
  dominated by ADA/DOGE/XRP files it never opened, and one run can touch three
  independently-versioned corpora (`binance`, `deribit-dvol`, `yahoo-macro`).
- Worse for the product's headline case: `LeaderboardLookback` offers **relative**
  windows (2 weeks … 4 years) and the pinned corpus spans 2023-01-01 … 2025-01-01, so
  today no relative window is covered. Every such run falls through to
  `data/binance-dynamic`, which by design has **no `REVISION.toml` at all**
  (ADR-0061 D5 — live data is not reproducible).

Surfacing it honestly needs a provenance **enum**, not a string, and ~40-60 lines in
`crates/backtest`. That is an engine change outside a presentation-only story. Recorded
here and in the story rather than faked on screen.

## Alternatives considered

- **Derive the searched-arm count from `rows`** — rejected, and this is the crux. It
  would render "12 strategies searched" from the twelve that came back, which is
  precisely the sentence that is true in the failure case and meaningless in it.
- **Read the corpus manifest from the `ui` crate** — mechanically possible (`ui` has
  the `data` dep) and rejected: `from_report` is contractually pure and total, and the
  UI cannot know which corpus the engine actually read, so it would confidently print
  `3a8b96c4…` for runs whose bars came from the unpinned dynamic cache.
- **Put the next step between the pick and the table** — rejected on ADR-0092's
  measurement: the fold outranks the pairing.
- **A generic "try a longer window" next step** — rejected. It is advice that is
  correct only in one of the four cases, and the whole finding is that a generic answer
  is what the screen already had.

## Consequences

- No byte-exact baseline moves (none renders the Leaderboard screen).
- **`leaderboard_scorecard_render`'s extent measurement changed technique.** It located
  the block's height by requiring all 48 rows of the following block to match
  byte-for-byte at some offset. After this story's three added lines that following
  block lands on a fractional `y`, and a 1-px full-width hairline rasterises one row
  off: at the true 430-px offset **44 of 48 rows are byte-identical** and 4 differ
  across x = 17..1887. The gate now takes the offset maximising exact matches and
  requires a dominant match (≥ 40 of 48). **The fold assertion itself is untouched** —
  loosening that is the #77 failure ADR-0092 refused, and it was never the thing
  failing: the block ends at 903 of 1080.
- **The scorecard block's height is no longer constant across fixtures**, and two y-band
  gates were re-anchored on measurement because of it. D2's arm-inventory line lists the
  requested arms and WRAPS, so a wide field pushes everything below it further down than
  a narrow one. `leaderboard_narration_render`'s band doc comment had stated the opposite
  as a load-bearing assumption ("everything above the block is fixed-height in every
  fixture, so the band does not move with the field size"); that is false as of this ADR.
  Both bands (`REC_TOP`/`REC_BOTTOM` and `BANNER_TOP`/`BANNER_BOTTOM`) now span 950..1190,
  sized for the measured SPREAD across fixtures and stopping short of the ranked table's
  clay at y >= 1205. The measurement table is in both files.

  One of the two was found by it failing (`narration_not_requested_paints_explain_control`,
  "got 0"). The other, `benchmark_wins_banner_has_no_credibility_band`, **stayed green
  while being weakened** — it asserts the ABSENCE of amber in the band, and the amber it
  used to bracket had moved from y≈1050..1092 to y=1122..1164, outside the old band. A
  negative assertion satisfied by the content moving away is bug-log #96's failure mode
  exactly, and it is why both bands were re-measured rather than only the one that went
  red.

- New open defect found while scoping D7 and recorded as **bug-log #105**:
  `bakeoff/mod.rs:541` handles `Ok(_) | Err(_)` in one arm, so a
  `RevisionError::AggregateMismatch` — a corrupt or tampered pinned corpus — is
  indistinguishable from "window not covered", degrades silently to unpinned live data,
  and logs a line that is untrue in that case.
- **Moral**: a screen that can only describe what it received will describe a broken
  search as a complete one. The input set has to be on screen for the output set to
  mean anything.
