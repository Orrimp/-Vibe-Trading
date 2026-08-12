---
slug: dev-notes
status: living
owner: orchestrator
updated: 2026-08-04
---

# Code-Review Playbook — the burn-down's accumulated knowledge

**Why this file exists:** eight `bmad-code-review` runs (stories 1-10 → 1-17)
established a repeatable pattern, a fixed known-and-owned routing set, and a
family of mandatory probes born from real defects. Before this file, every run
re-derived all of it from scratch in hand-written prompts. **Read this first;
it is the difference between a 3-hour review and a 45-minute one.**

Loaded automatically by `bmad-code-review` (see `_bmad/custom/bmad-code-review.toml`).

---

## 1. The mechanical start (2 minutes, not 20)

```bash
bash scripts/review_prep.sh <story-key>        # e.g. 1-18-horizon-retest-robustness
```

It finds the implementing commits, writes a code-only diff to the session
scratchpad, prints the baseline gates, and prints the story's triad legs
(trace state + CHANGELOG line). Everything below assumes you ran it.

## 2. The three layers (spawn in ONE message, parallel)

| Layer | Skill | Hunts |
|---|---|---|
| Blind Hunter | `bmad-review-adversarial-general` | anything wrong, prioritized by consequence |
| Edge Case Hunter | `bmad-review-edge-case-hunter` | boundaries, degenerate inputs, unhandled paths |
| Acceptance Auditor | (plain prompt) | delivered-vs-specified, per-AC, current-tree regression |

**Non-negotiable prompt ingredients** (omit one and the layer wastes a pass):

1. The diff path — *"its contents ARE the diff under review"*.
2. The **known-and-owned list** (§3) with *"route with a verified chain, do NOT re-report"*.
3. The **mandatory probes** (§4).
4. *"The sweep-bin code was extracted to `sweep_harness.rs`/`mc_harness.rs` — verify findings against the SEAM at HEAD, not the diff's old bin lines."*
5. *"Tag every finding ANCHOR-IMPACTING yes/no"* (drives the triage split).
6. *"Do NOT modify files. Do NOT run cargo."* (the orchestrator owns gates).
7. For the Auditor only: the story spec, the ACs **as of the implementing commit**
   (`git show <sha>:spec/<slug>/feature.md` — the frozen stubs at HEAD are useless),
   the architect's locked design commit, and the anchored evidence.

## 3. Known-and-owned — route, never re-report

| Item | Disclosure | Owner |
|---|---|---|
| Cross-symbol fill mispricing (`PaperEngine::step` prices every batched order at the stepped bar's close; `run_path` + `run_cell` both) | bug-log #67 | story **1-25** |
| `√8575` vs documented `√8760` annualization (1h lanes only; the horizon siblings are correct — verified 1-18) | #67 rider | **1-25** |
| PWSD window `λ(k/m̂)` vs literature `λ(k/(2m̂))` + m̂ range + `|r|`-mean target | 1-13 decision | story **1-24** |
| Inert drift/hold-band swept axis (hashed + printed + dead field) | bug-log #68 | **1-25** |
| `portfolio_exposure_cap` inert engine-wide (`Order::new` checks only the per-symbol cap) | bug-log #69 | **1-25** |
| Contaminated anchors: #86, #87, #90, #91, #92-#99, **#100-#107** (basis; clear of #72/#73 — verified, not assumed) — **and #88/#89** (carry 1h, via #73) | #67/#73 blast radius | **1-25** inventory |
| R3 coverage gate compared COARSE expected vs RAW loaded (a 95.9%-missing corpus passed) | bug-log #70 | FIXED 2026-08-04 |
| `Order::new` exposure cap is SIDE-BLIND — rejects position-CLOSING sells, silently | bug-log #71 | **1-25** |
| Cosmetic 1h timestamp ladder made funding accrual horizon-blind | bug-log #72 | code FIXED; anchors → **1-25** |
| Funding accrued once per SYMBOL-BAR, not per settlement (universe-size dependent) | bug-log #73 | code FIXED; anchors → **1-25** |

**The reviewer's job on these:** verify the consumption chain for THIS story's
artifacts and state it (that grows 1-25's inventory), then move on. A re-report
is wasted budget; a missing chain-verification is a gap in the re-lock plan.

**Standing exemption:** the advisor's bakeoff gate (`bakeoff/bootstrap.rs`)
resamples log-returns from candidate equity curves and never re-executes fills.
Crowns, verdicts, and the era-qualified ship-passive thesis are independent of
every contaminated research surface. Re-verify if the bakeoff path changes;
otherwise cite it.

## 4. Mandatory probes — the lineage, weaponized

Every entry below was a real defect that shipped past a VERDICT→PASS. Each is
now a question every review must ask.

| Probe | Question | Born from |
|---|---|---|
| **Axis-execution** | Every swept/configured parameter: trace the caller graph. Is it *executed*, or only hashed/printed/stored? | #68 (drift axis) |
| **Binding-limit** | Every declared risk limit: construct a scenario where it must bind. Does it? | #69 (exposure cap) |
| **Vacuity** | Every test named as a gate: would it go RED under the exact bug its name/comment claims to catch? Re-implementations of production logic inside the test = vacuous. | #66, and 3-of-5 in 1-17 |
| **Skip-visibility** | Any corpus/fixture-gated test: does it self-skip green? Is the skip *counted* anywhere? | #66 (cwd-relative root) |
| **Chain** | Does this story's anchored evidence consume a known-contaminated path? Which rows? | #67 |
| **Identity-forge** | Can any CLI combination emit an *existing anchor's* scenario name or land in its directory? (`--direction`/`--grid`/`--selection-mode`/`--score-source`/out-dir) | 1-15 M2, 1-16 H1, 1-17 H |
| **Loop-scope** | A rule inside a per-bar loop: what is the loop ITERATING? With multi-symbol merged bars, "once per bar" and "once per instant" differ — and the difference is invisible in every aggregate the report prints. | #73 (funding × universe size) |
| **Seed-collision** | Any additive seed derivation (`base + i·CONST`) collides on anti-diagonals. Must be splitmix-mixed. | 1-13, 1-14, 1-15 (three instances) |
| **Channel** | Does the test inject the thing under test through the SAME channel production uses? If not: does the *test* channel have an effect of its own that could account for the difference the assertion measures? A signal injected through a channel that also moves equity directly gives you a gate that passes with the signal fully destroyed. | #74 (basis via `funding_override`) |
| **Supersession** | Did later work FALSIFY a conclusion this story froze into an anchored body? Read the successor story's trace row and any adjudication dev-note. Propagation runs forward into the next story's rationale automatically — backward into the evidence, never. | 1-20 H4 (fee-bleed) |

**On the supersession probe.** In 1-20 the anchored bodies asserted "fee-bleed
consumes the edge" while the adjudication written *the same day from those same
eight surfaces* concluded "the fee-sweep falsified the fee-bleed hypothesis —
the killer is BETA, not fees." That finding was carried forward into the next
story's trace row and became the entire rationale for building it; it was never
carried back. Anchored bodies are byte-immutable, so a superseded conclusion
inside one can only be corrected by an errata or at a re-lock — which means
nobody does it unless a reviewer asks. The corpus is the artifact a future
reader trusts first; a contradiction there outlives every dev-note that
resolved it. **Ask of every conclusion in an anchored body: does the project
still believe this?**

**On the identity-forge probe — enumerate, do not spot-check.** It has now fired
in *four consecutive* stories, each time on an axis added by that story and never
added to the pairing guard. Do not ask "is there a forge?"; list every CLI flag
that reaches either the fill arithmetic or the hashed body, and check each one
against both the scenario NAME and the guard. 1-20's fee axis was the worst
instance precisely because it left no visible token — the forged body renders
*identically* to the real one.

**On the vacuity + channel probes — the gate itself is in scope.** The AD-16
day-1 divergence e2e is mandated by CLAUDE.md, which makes it the single
artifact most likely to be written to satisfy a checklist. In 1-20 it was
satisfied by a test that stayed green with the signal returning a constant
zero. Run the mutation; a gate you have not seen go RED is a claim, not a gate.

## 5. Triage rules

- **Assign your own severities.** Sub-agent severities are advisory; they lack
  the cross-story context that decides consequence.
- **Split by anchor-impact, not by severity.** Anything whose fix changes a
  hashed body → the re-lock program (1-24/1-25), recorded *visibly* in that
  story's inventory. Everything else → the patch list.
- **New disclosure?** If a finding is a *class* (not an instance), it earns a
  bug-log number and a moral. The moral is the part that prevents recurrence.
- **Defers carry `Owner:` + `Revisit:`** (a concrete event). Two retrospectives
  unowned → escalate or close as accepted-limit.

## 6. Patch-pass contract (what the dev agent must be told)

- Anchor-safety is the prime constraint: `verify_anchors.sh` **before first edit
  and after last** — quote both. Comment-only edits inside `evidence/` are
  legitimate *and* must be double-gated.
- Validation additions may reject **only** combinations no checked-in config or
  anchored invocation uses — prove it by enumerating the accept-set in a test.
- Report **error lines verbatim**, never counts (`grep -c` hides the diagnosis).
- **Apply the vacuity probe to YOUR OWN patch, not just to the diff under
  review.** Story 1-18 found that the 1-15 review's own grid-discriminator fix
  was inert in production — the binary kept an inline copy of the naming logic
  and never called the extracted seam, while the test asserted the library
  function nobody used. A fix that lands in a seam MUST be proven to bind at the
  call site: assert the production output (literal expected strings), not the
  helper's return value. Extracting a seam and testing the seam proves nothing
  about the binary.
- Expect the agent to die on a session limit mid-pass. That is routine: its
  edits are on disk, and the orchestrator finishes inline. Always re-verify
  yourself regardless of how complete the agent's report looks.

## 7. Close-out — the triad flip

All four legs in ONE commit, or the pre-commit hook rejects it (correctly):

1. Story: `Status: done`, every `[Review][Patch]` box ticked, close-out block
   with **literal** gate lines.
2. `trace.toml`: `state = "shipped"` with the PRIOR comment preserved.
3. `CHANGELOG.md`: the story's index line exists (extend it with the review's
   headline if the review changed what the feature *means*).
4. `sprint-status.yaml`: `done` + the remaining-count header note.

Then: `python3 scripts/spec_lint.py` (exit 0) and commit with `--no-gpg-sign`
(the 1Password vault wedge oscillates; commits queue for the operator to push).

## 8. Known infra reds — do NOT chase these

- **62 visual-baseline tests fail at clean HEAD** — glyph-localized system-font
  rasterization drift, proven change-independent by stash A/B.
  `docs/dev-notes/visual-baseline-drift-2026-07-27.md`. Fix order matters:
  embed `fira-sans` *before* re-baselining.
- **Story 6-9 CI shakeout** is open; a red CI leg is not necessarily your diff.
- `#[ignore]`d tests (real-corpus probes) do not gate — call them out, don't
  treat them as failures.

## 9. Application-level reviews (different from story reviews)

A review of *the product* rather than a diff starts from
[`product-review-2026-08-04.md`](product-review-2026-08-04.md) — 14 standing
findings plus an explicit "what is solid" list. Confirm or kill each, add new
ones, and re-date the note. Sources worth re-reading first: `CHANGELOG.md`
(the feature index), `PRD.md` §13 (open questions), the do-not-build register,
and the bug-log's open entries. Do not re-derive the standing set.

## 10. What good looks like

A finished review produces: a story `### Review Findings` block whose every
line cites `file:line`, a triage split that names its owner for each
anchor-impacting item, literal gate output in the close-out, and — when the
story earned one — a bug-log entry whose **moral** generalizes the defect.

Eight reviews in, the recurring theme is one sentence: **declared is not
executed.** Configured, hashed, printed, documented, ratified, and named-in-a-
test all mean nothing until a caller graph or a red test proves the code runs.
