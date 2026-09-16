---
adr: 0092
title: The leaderboard's credibility block leads the pane
status: accepted
date: 2026-09-16
supersedes: none
superseded-by: none
---

# ADR-0092: The leaderboard's credibility block leads the pane

## Context

ADR-0075 § Decision placed the P0-1 overfitting scorecard — the *"How much to trust
this"* panel — **below the ranked table**, so it would read as "here's the pick, and
here's how much to trust it". Three months later `67f2a9d`
(advisor-data-quality-surface P1-7, 2026-07-06) added a Data quality panel at the
**top** of the same scrollable pane.

Nobody re-measured the fold. On the operator's 1920×1080 viewport the scorecard then
rendered **entirely off-screen** (bug-log #96, bisected to `67f2a9d`, diagnosed at the
pixels): a user met the recommendation and the ranked table without ever seeing the
panel whose only job is to say how much of it to believe. The honesty surface was the
one thing that needed scrolling.

Its own render gate could not catch that. With the block off-screen the only pixels
that differed between the gate's two frames were the **scrollbar thumb** — a 10-px
strip at x ≈ 1894 — because adding content below the fold lengthens the scroll extent,
which shortens the thumb. The gate was measuring chrome, with the opposite sign
(`with=39794 vs without=41774, delta=-1980`). The test's fixture had reserved the
vertical budget by construction ("a 2-row field so the scorecard block fits"); a later
feature spent it, and nothing was enforcing the reservation.

## Decision

**D1 — The scorecard block renders FIRST in the ready pane, and the pick follows it.**
Order: scorecard → recommendation → ranked table → Data quality → Risk story → short
field note → disclaimer.

The first implementation took the ruling literally — scorecard, then Data quality, then
the pick — and the render showed what that costs. Measured at 1920×1080 with the guided
input present: plan panel + context end at y=445, the scorecard spans 473..831, Data
quality runs to ~1060, and the recommendation (147 px) and the whole ranked table
(175 px) land BELOW the fold. That trades #96's complaint for a worse one: the user sees
two honesty panels and has to scroll to learn which strategy won. Re-ruled by the
operator on measurement (2026-09-16): Data quality moves under the table — of the three
readouts it is the least urgent to someone deciding whether to act — so the trust
readout, the pick, and the first ranked rows all clear the fold.

**D2 — This amends ADR-0075's placement clause only.** The panel's four facts, their
plain-language glosses, its report-only status and the frozen-gate identity test are
untouched. Two earlier placement rationales end with it: P1-7's "Data quality renders
FIRST" (the DATA → ANALYZE reading order) and P1-2's "Risk story sits directly below
the scorecard so the two honesty layers pair". Both were rationales for an ordering,
never requirements on one. The fold is a requirement.

**D3 — Visibility is a tested property now, not a layout accident.**
`leaderboard_scorecard_render` measures the block's extent in a frame tall enough that
nothing scrolls or clips (asserted, not assumed), then asserts that extent ends above
y = 1080 and that the 1080-px frame paints those rows identically to the unclipped one.
No measurement in the leaderboard gates reads the scrollbar gutter, so no gate on this
screen can be satisfied by chrome again.

## Alternatives considered

- **(b) Scroll the harness, or render a taller viewport, and leave the layout alone** —
  rejected. It restores the gate and accepts the thing that was actually wrong: the
  block stays below the fold.
- **Loosen the assertion, or re-baseline the delta** — rejected, and recorded in the
  bug log as the failure mode of #77. The number compared was the scrollbar, so any
  threshold fitted to it is fitted to noise.
- **Shrink the Data quality panel to make room** — rejected: it trades one block's
  visibility for another's, and the next feature takes the space back.

## Consequences

- The recommendation and the ranked rows move down by the scorecard's height (358 px
  measured). "The ranked rows keep their position" was ADR-0075's reason for placing the
  block below the table; the fold outranks it.
- A first screen now reads: how much to trust this → the pick → the ranking. Data
  quality is one scroll down. The crown's own credibility line (ADR-0085 § D1/D4) still
  co-presents with the recommendation, so the pick never appears without its verdict.
- `leaderboard_data_quality_render` loses its top-band crop entirely: with the panel
  below the table it owns no fixed y-range, and a crop at a fixed offset would measure
  whatever happens to be there — the #96 failure mode. Its presence is proven by the
  Warnings-row negative control instead.
- Every leaderboard gate now renders the whole pane (2400 px) rather than the fold, so a
  reorder can no longer silently empty a band scan. Two bands stay anchored to measured
  geometry — the banner (850..1100) in `crown_credibility_render` and the same range in
  `leaderboard_narration_render` — because their colour predicates also match the table's
  clay and must exclude it. Both cite the render they were measured from.
- The moral, recorded in bug-log #96: a pixel gate that compares two renders can be
  satisfied by ANY difference between them, including chrome the feature never touches.
  A harness that reserves a resource by construction has an invariant with no
  enforcement, and the next feature to want that space takes it silently.
