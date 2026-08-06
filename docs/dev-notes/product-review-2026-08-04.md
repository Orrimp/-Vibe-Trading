---
slug: dev-notes
status: living
owner: orchestrator
updated: 2026-08-04
---

# Adversarial product review — the application and its features (2026-08-04)

Scope: the shipped product ("The Honest Advisor" — pick coin + budget → bake
off the strategy registry → rank → forward plan → paper-trade), not the
delivery process (that pass is `audit-2026-08-03.md` + the 2026-07-27
hardening commit `bcfd13b`). Baseline for future product reviews: start from
these findings, confirm or kill each, add new ones — do not re-derive.

## Findings

- **The product's success state and its failure state are the same screen.**
  The advisor bakes off every arm and crowns buy-and-hold for every input an
  operator can actually supply. That is the honest answer, but nothing in the
  UI distinguishes "we searched hard and the null won" from "the search was
  broken and defaulted to the benchmark." The `BenchmarkWins` modal is the
  designed outcome *and* the shape of every plausible silent failure — and bug
  numbers #65 through #69 are five proofs that silent failure is this
  codebase's characteristic mode.

- **The credibility layer's bands were calibrated on evidence now known to be
  contaminated, and the bands are frozen by policy.** `classify_verdict` /
  `verdict_bands` are byte-frozen (AD-1) with thresholds chosen against the
  same research surfaces bug-log #67 has since disclosed as execution-artifact
  noise. The freeze that protects the gate from feature creep also prevents
  the gate from ever being re-derived. Nobody has asked whether the p5-Sharpe
  boundary would sit where it sits if the surfaces were clean.

- **The €200 headline runs on opt-in realism.** Lot-size rounding and
  minimum-notional rejection (`VenueFilter`, ADR-0087) default to *off*, so
  the default advisor path can still produce a plan whose legs are
  unexecutable at the venue the plan names. The product's most concrete
  promise — this exact small budget, honestly — is the one place realism is
  opt-in.

- **The EUR number the user reads is stale by construction.** The FX rate is a
  static config fallback; the live-fetch path was only scoped on 2026-07-27
  (story 3-19, unbuilt). A euro-denominated advisor that silently quotes an
  old rate is misreporting the only figure a retail user actually checks.

- **"All strategies" is an unnamed, unbounded claim.** The bakeoff runs a fixed
  registry; retired research lines remain in-tree but excluded, and nothing in
  the UI tells the user what "all" enumerated. The user cannot distinguish "no
  strategy beat holding" from "none of the seven we happened to keep beat
  holding."

- **A shipped, "done" feature was silently broken for weeks and only a review
  found it.** Compare's cold-boot scanner skipped every engine-written report
  (unindented frontmatter, bug-log #66 A.4), meaning the compare surface never
  populated from real Lab runs. There is no telemetry, no session log, and one
  user on one machine — so feature "doneness" in this product is asserted by
  tests, never by use. The tests were also the thing that was wrong.

- **The pixel gate for a GUI product has been red for over a week.** 62
  baseline comparisons fail at clean HEAD from font-rasterization drift. The
  cause is understood and change-independent, but the practical position is
  that the application's appearance is currently unverified, and any genuine
  visual regression would land invisibly inside the noise.

- **Nobody but the operator can reproduce a single real-data claim.** Every
  pinned corpus is machine-local and gitignored; a fresh clone gets the code,
  the anchors, and no data. The disaster-recovery runbook (added 2026-07-27)
  documents restore-from-backup for the one machine that has it — it does not
  make the product's evidence reproducible by a second party, which is what
  "honest" would require of an advisor that shows you backtests.

- **The scorecard is displayed but disarmed.** DSR / N_eff / MinBTL render
  beside the crown as a haircut the crown-eligibility logic never consults
  (report-only by ratified decision). A number shown next to a verdict, in a
  product whose entire pitch is statistical honesty, will be read as a filter.
  It is decoration.

- **The thesis the product exists to deliver currently carries an asterisk the
  product does not show.** After the 1-17 review the active-trading closure
  stands "direction-preserved pending re-lock" — defensible, well-argued, and
  invisible to the user, whose modal still states the conclusion flatly. The
  errata discipline that governs frozen evidence has no equivalent in the UI.

- **"Feature-complete" is doing public-relations work.** The label is asserted
  in the README and CHANGELOG while the board carries seven un-reviewed
  stories, two critical re-lock stories, three newly-scoped builds, and five
  open bug-log entries. Feature-complete is true of the *feature list* and
  false of the *product state*, and only one of those is what a reader hears.

- **The one thing the user is emotionally invested in has the least ceremony.**
  The paper portfolio — their simulated €200 — has no documented recovery
  story for a mid-session crash, no schema-migration story across releases,
  and no in-app account of what survives a restart. Every other durable
  artifact in this repo (anchors, evidence, trace rows) has a formal
  immutability contract. The user's own state has none.

- **The product answers the question and then abandons the user.** The honest
  verdict is "hold, don't trade." The only next step the app offers is a
  hand-off export. There is no in-app explanation of *why* holding won for
  their coin, no re-check cadence, and no answer to the obvious follow-up ("so
  when should I look again?") — which is precisely the moment a retail user
  goes and finds worse advice elsewhere.

- **The LLM sits inside a product that promises offline honesty.** Narration is
  advisory-only and faithfulness-gated, but it is a paid external dependency in
  an app whose value proposition is that it will not sell you a story. Its
  unavailable / over-budget / degraded states are engineering concerns today,
  not documented user-facing states.

## What is genuinely solid (so a future review doesn't re-litigate it)

The engine's determinism discipline (seeded, byte-anchored, 119 locked report
bodies), the double-entry Decimal ledger, the frozen-gate policy as a
*mechanism*, the do-not-build register, and — verified three times under
adversarial conditions — the advisor bakeoff gate's independence from the
contaminated fill path. The product's honesty machinery is real. The findings
above are about the gap between that machinery and what the user is shown.
