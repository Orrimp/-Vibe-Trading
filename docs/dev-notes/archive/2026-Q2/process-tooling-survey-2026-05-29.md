---
title: Process / tooling survey — Route C compounder inventory
date: 2026-05-29
authors: [architect]
status: survey
tags: [survey, process, tooling, route-c, compounder, dev-note, read-only]
related:
  - docs/dev-notes/post-v3-strategy-direction-2026-05-29.md
  - docs/dev-notes/weekly-retro-2026-05-27-to-2026-05-29.md
  - spec/backlog.md
  - spec/lab-recipe-test-harness/feature.md
  - _bmad-output/planning-artifacts/architecture/decisions/0048-lab-recipe-test-harness.md
---

# Process / tooling survey — Route C compounder inventory

> **READ-ONLY ARCHITECT-PASS dev-note, NOT a feature brief.** Audit-only —
> no Queue promotion, no `[[req]]` row, no feature folder, no ADR.
> Tabulates every Process / tooling Queue entry + adjacents, ranks Top-5
> by per-cycle benefit × cost × maintenance, proposes Month-1 A/B/C
> picks under the durable contract. Operator decides what (if anything)
> promotes.

## § The compounder argument

Per [`post-v3-strategy-direction-2026-05-29.md` Route C](post-v3-strategy-direction-2026-05-29.md#route-c--accept-v1-momentum-baseline-is-near-frontier-engineer-elsewhere):
after the v3 three-pick set retired with NEGATIVE Sharpe-deltas
(C1 −0.022, C2 −0.294, C5 inconclusive), the dominant read collapsed
to **"v1 momentum is near-frontier; engineer elsewhere."** Route C's
defining property under the durable contract: **all four verdict cells
collapse to "HIGH cognitive investment preserved"** because alpha is
not the success criterion. Process / tooling is the canonical Route C
— every dev cycle, every future strategy lane, every operator-facing
verification benefits regardless of any alpha outcome.

Empirical vindication: `lab-recipe-test-harness v0.1.0` shipped
2026-05-28 (ADR-0048) and caught the Bug #64 D.2.1 design conflict the
**same week** (per [weekly retro § 5](weekly-retro-2026-05-27-to-2026-05-29.md#5-bug-64-d11--d21-attempt-and-revert--harness--re-attempt--wont-fix)).
The harness paid off ~1.5 dev-days within one session and remains a
permanent regression-class defender.

## § Inventory (Process / tooling Queue + adjacents)

Surveyed `spec/backlog.md § Process / tooling` (L2183–2497) + 3 Strategy
process-shaped items + 5 weekly-retro contracts (2026-05-29).

| # | Slug / item | Promoted | State | File-scope | v0.1.0 cost |
|---|-------------|----------|-------|------------|-------------|
| 1 | `v2x-trading-state-bus` | 2026-05-17 | candidate | `crates/llm`, `crates/agent` | ~5-7d |
| 2 | `v26-bakeoff-llm-arbiter` | 2026-05-17 | **RETIRED-by-context** (v2.6 retired 2026-05-22) | n/a | n/a |
| 3 | `v2-llm-strategy-v21-followups` (LLM-budget tile + tracing redactor + clippy) | 2026-05-13 | candidate | `crates/audit`, `crates/ui`, `crates/llm` | ~3-4d |
| 4 | `ui-test-harness-canvas-state-seeding` | 2026-05-12 | candidate | `crates/ui/src/test_support.rs` | ~1-2d |
| 5 | `ui-test-harness-viewport-matrix` (Week-2) | 2026-05-12 | gated on bootstrap | `crates/ui/tests` | ~3-4d |
| 6 | `ui-test-harness-evaluator` (Week-3) | 2026-05-12 | gated | `.claude/agents/`, hooks | ~2-3d |
| 7 | `ui-test-harness-ci` (Week-4) | 2026-05-12 (cheapened) | gated on #5+#6 | `.github/workflows` | ~4d |
| 8 | `ui-contrast-asserter` (WCAG) | 2026-05-15 | candidate | `crates/ui/tests/contrast.rs` | ~0.5d |
| 9 | `ui-update-proptest` | 2026-05-15 | candidate | `crates/ui/tests` | ~5d |
| 10 | `ui-gallery-bin` | 2026-05-15 | **shipped-partial v0.1** (V5+ blocked by #11) | done | done |
| 11 | `ui-gallery-table-cell` (iced Table panic fix) | 2026-05-15 | candidate | `crates/ui/src/widgets/strategies.rs` | ~1d |
| 12 | `ui-a11y-shadow` (AccessKit + kittest) | 2026-05-15 | candidate | `crates/ui/src/a11y.rs` (NEW) | ~7d |
| 13 | `ui-vlm-judge` | 2026-05-15 | candidate | `crates/ui/tests/fixtures` | ~3d |
| 14 | `ui-inspect-mcp` | 2026-05-15 | deferred (cycle 4+) | `crates/ui/cockpit_live` | ~4d |
| 15 | `ui-mutants-pass` | 2026-05-15 | candidate, pairs with #9 | `crates/ui` | ~1d |
| 16 | **Visual-fail HTML reporter** (agent contract gap) | 2026-05-15 | candidate | `crates/ui/tests/fixtures`, `tester.md` | ~1d |
| 17 | `ui-iced-table-panic-upstream` (file bug) | 2026-05-15 | candidate | upstream issue | ~0.5d |
| 18 | `ui-comet-eval` | 2026-05-15 | **DEFERRED** (iced 0.15 stable gate) | n/a | n/a |

### Adjacent process-shaped items (weekly retro § 3-fix)

| # | Item | Promoted | State | Why process-adjacent |
|---|------|----------|-------|----------------------|
| A | Queue-staleness reconciliation sweep | 2026-05-29 (retro § 3-fix-1) | proposed | 3 audits caught stale stubs; orchestrator 30-s pre-flight |
| B | ADR registry atomic-write contract | 2026-05-29 (codified) | contract codified | Architect M-T1: writing ADR = registering in README atomically |
| C | Pending-operator-verifications ledger | 2026-05-29 (retro § 3-fix-5) | proposed | Cross-session verify ledger; presenter decks link in |
| D | 1Password GPG signing recipe | 2026-05-29 (retro § 3-fix-2) | proposed | Intermittent commit failures; standing diagnostic |
| E | Parallel-spec-lint-cross-talk-budget block | 2026-05-29 (retro § 3-fix-3,4) | proposed | Mid-flight orphan-folder false-positives |

### Stale-flag findings (per new pre-flight reconciliation contract)

- **#2 `v26-bakeoff-llm-arbiter`** — v2.6 retired 2026-05-22; this candidate
  is dead by inheritance. **Flag for purge** next analyst sweep.
- **#10 `ui-gallery-bin`** — Queue text reads "v0.1-partial shipped"
  but stub still under Queue; should move to Recent with explicit
  V5+ blocker pointing at #11.
- **#18 `ui-comet-eval`** — listed twice (L2297 + L2488), near-duplicate.
  **Collapse to one entry.**
- All UI-testability deep-dive items (#8, #9, #11–17) carry the same
  2026-05-15 promotion date (~14d age) with no analyst spawn. Per the
  reconciliation contract, batch-review as a cohort.

## § Top-5 ranking by leverage

Bias: per-cycle benefit weighs highest under Route C.

| Rank | Item | Per-cycle benefit | Investment cost | Maintenance | M-T1 fast-skip |
|------|------|-------------------|-----------------|-------------|----------------|
| **1** | `lab-recipe-test-harness v0.3.0+` extension | **LARGE** — every Recipe / aggregator touch | SMALL (~2-3d per Recipe) | LOW (ADR-0048 carries forward) | HIGH |
| **2** | Visual-fail HTML reporter (#16) + Viewport matrix (#5) | **LARGE** — closes agent-contract gap; cross-cuts every UI feature | SMALL+MID (~1d + ~3-4d) | LOW (helper + tester.md stanza) | HIGH |
| **3** | `v2.1 tracing-Layer redactor` (split from #3) | MEDIUM — cross-cutting safety; every structured log benefits | SMALL (~1.5d standalone) | LOW (Layer wiring is one-time) | LIKELY |
| **4** | `ui-contrast-asserter` (#8) | MEDIUM — closes palette-refactor regression class | SMALL (~0.5d) | LOW (data-driven; new tokens auto-cover) | HIGH |
| **5** | Queue-staleness recon + ADR-registry atomic (A+B) | MEDIUM — preventive; eliminates recurring audit drag | SMALL (~1d combined) | LOW (script + CI check) | N/A contract |

Honorable mentions (just below cut-off):

- **`v2x-trading-state-bus` (#1)** — LARGE benefit IF v2 LLM lane
  re-activates; ZERO if dormant. C5 follow-on currently unscheduled.
  Defer to next v2 LLM activation.
- **`ui-update-proptest` (#9) + `ui-mutants-pass` (#15)** — LARGE benefit
  (~40 `Message` variants uncovered) at ~6d combined. Month-2 once
  Top-5 cohort lands.
- **`v2.1 cockpit LLM-budget tile` (split from #3.a)** — gates Lumen
  Phase 6 Assistant; defer with v2 LLM lane.

## § Top-5 deep-dives (condensed)

- **Rank 1 — harness v0.3.0+.** v0.2.0 Active (arch M-T1 done
  2026-05-29; dev Wave A gated on TrainingLogRecipe). After: remaining
  shapes are `LabRunCompletedRecipe` (one-shot), cross-pair TrailMirror
  S1 boundary, operator-side journal-replay. LARGE benefit first wave;
  shrinks as it saturates. `pub trait` seam (ADR-0048) is the durable
  inheritance pattern.
- **Rank 2 — #16 visual-fail HTML + #5 viewport matrix.** #16 writes
  `spec/<slug>/reports/visual-fail-<ts>.html` with baseline / actual /
  diff PNG triple inline + assertion location + VLM verdict (if
  enabled) on visual FAIL — **closes agent-contract gap directly**.
  #5 expands snapshot coverage across panels / modals / status bar /
  agent feed / debug screen × 3 viewports — multiplies the failure
  surface #16 reports.
- **Rank 3 — v2.1 tracing-Layer redactor (split from #3).** Pure-fn
  `redact()` landed v2.0.0 pass-3; this closes the Layer field-visitor
  side (redacts `Bearer ...` / `sk-...` / `anthropic-...` in structured
  logs without explicit `redact()` calls). Split #3 into redactor (~1.5d,
  no v2-LLM dep) + cockpit-llm-budget-tile (~2d, gated on v2 LLM
  activation). Ship redactor now; defer tile.
- **Rank 4 — #8 contrast asserter.** Half-day data-driven WCAG test
  over `(fg, bg)` token pairs in `crates/ui/src/theme.rs`. WARN 2
  weeks → gate. **Best-cheap-pick** — new tokens auto-assert.
- **Rank 5 — Queue-staleness script + ADR registry atomicity (A+B).**
  `scripts/queue_staleness_check.sh` greps for "moved Queue → Active"
  stubs whose target slug has frontmatter `status: shipped`; ~30 s/run
  at session start. ADR registry atomicity is a CI check (write ADR
  file → required commit-time `architecture/adr/README.md` row).
  Operator already paid reactively 3× in 3 weeks (audits 2026-05-07
  / 05-27 / 05-29). Operationalises retro § 3-fix-1 and § 3-fix-6 at
  sub-week cost.

Briefly noted (not Top-5): #4 canvas-state seeding (~1-2d; closes V8
of chart-canvas-overhaul); #10 + #11 gallery V5+ unblock (~1d + ~0.5d
upstream); #12 `ui-a11y-shadow` (~7d, Month-2+).

## § Architect's recommended Month-1 picks (A / B / C)

Three picks under durable-over-quick framing. Operator decides; analyst
writes briefs at promotion.

### Pick A — Test infra trifecta: harness v0.3.0 + Visual-fail HTML (#16) + Viewport matrix (#5)

**Per-cycle benefit.** LARGE. Test infrastructure compounds linearly
with feature count; every UI / Recipe touch from Month-1 onward
inherits. #16 specifically closes the operator-facing failure-artifact
gap (no more "I see FAIL but can't see what failed").

**Investment cost.** ~5-7 dev days (Visual-fail HTML ~1 + viewport
matrix ~3-4 + harness v0.3.0 extras ~2). Two parallel-safe lanes once
#16 lands.

**Maintenance burden.** MID. Tester contract amendment is one-time;
harness pattern self-extends via `pub trait` seams per ADR-0048.
Viewport matrix needs one `.gitattributes` rule to keep baseline-PNG
diffs reviewable.

**Why dominant.** HIGH M-T1 fast-skip for all three (no new ADR;
ADR-0048 + ADR-0042 + ADR-0044 carry forward). The 81-commit
2026-05-27→05-29 week is the empirical proof — `v0.1.0` paid off
in-session. Doing the same for visual snapshots pre-empts the next
Bug-#64-class revert.

### Pick B — Tracing redactor (split from #3) + Contrast asserter (#8)

**Per-cycle benefit.** MEDIUM. Tracing redactor is cross-cutting
safety (every structured log call benefits); contrast asserter closes
the palette-refactor regression class with WARN→gate ladder.

**Investment cost.** ~2-3 dev days combined. Both SMALL, both durable.

**Maintenance burden.** HIGH PAY-FORWARD. Redactor lives at
`crates/audit` / `crates/llm` boundary — every new LLM provider /
tracing field benefits without re-work. Contrast asserter is
data-driven — every new theme token auto-asserts.

**Why this slot.** Both below the no-brainer bar but neither covered
today. Redactor is on the critical path of any v2-LLM follow-on
(C5 v0.2.0 standing-Q) and any new provider integration; landing
it now means it's already there at next LLM lane activation.

### Pick C — Orchestrator hygiene: Queue-staleness script (A) + ADR-registry atomic-write contract (B) + Pending-verifications ledger (C)

**Per-cycle benefit.** MEDIUM — eliminates a recurring 30-s/session
orchestrator drag (3 audits caught the same pattern). Pending-
verifications ledger consolidates a chronic carry-over class (Bug #64
visual-verify, Yahoo bulk fetch, toast-queue smoke tests).

**Investment cost.** LOW. ~1d for `scripts/queue_staleness_check.sh`
+ ~0.5d for the ADR registry write-or-fail wrapper + ~0.5d for the
pending-verifications ledger + presenter-template stanza.

**Maintenance burden.** LOW — scripts live in `scripts/` and tend to
last; ADR registry atomicity is contract + CI check; ledger is one
markdown file + recurring template stanza.

**Why this slot.** "We will need this soon anyway" — operator
already paid reactively 3× in 3 weeks. Pick C operationalises the
weekly retro's 3 concrete fix proposals at sub-week cost. **High
likelihood operator picks regardless of A/B** because the retro
itself recommended it.

## § What's NOT a compounder (despite looking like one)

Honest accounting — these look like Route C but fail per-cycle-benefit
or are too narrowly scoped:

- **#2 `v26-bakeoff-llm-arbiter`** — RETIRED-by-context (v2.6 dead).
- **#18 `ui-comet-eval`** — gated on iced 0.15 stable (not released).
- **#17 `ui-iced-table-panic-upstream`** — one-shot bug report (~0.5d).
  Useful but single-event. Land alongside #11 in passing.
- **#3.c pedantic clippy cleanup** — 2 cast warnings; hygiene-only.
  Roll into next audit's housekeeping.
- **#14 `ui-inspect-mcp`** — interesting capability but deferred to
  cycle 4+ by dev-note §5.2. Per-cycle benefit unclear today;
  `screencapture` + `osascript` covers most current needs.
- **#13 `ui-vlm-judge`** — second-opinion forensic on visual FAIL.
  Compounder IF failures become frequent enough to warrant
  ~3d + $0.50/run shadow budget. Pair with Pick A's #16 to defer
  until evidence warrants.
- **`v2x-trading-state-bus` (#1)** — ZERO benefit while v2 LLM is
  dormant; LARGE when re-activated. Promote at activation, not pre-
  emptively.

## § ADR readiness flag

Per the 2026-05-29 codified architect contract (writing ADR =
registering atomically in `architecture/adr/README.md`), **no Top-5
candidate requires a new ADR.** ADR-0048 carries forward for harness
extensions; ADR-0042 + ADR-0044 carry forward for any Activity /
subscription work. Pick C's orchestrator pre-flight contract likely
belongs in `AGENT.md` rather than a numbered ADR — small operator-
decide if Pick C promotes.

## § Cross-references

- [`post-v3-strategy-direction-2026-05-29.md`](post-v3-strategy-direction-2026-05-29.md) (Route C)
- [`weekly-retro-2026-05-27-to-2026-05-29.md`](weekly-retro-2026-05-27-to-2026-05-29.md) (§ 3-fix items)
- [`spec/lab-recipe-test-harness/feature.md`](../lab-recipe-test-harness/feature.md) + [`v0.2.0 cross-surface`](../lab-recipe-test-harness-v0.2.0-cross-surface-extension/feature.md)
- ADR-0048 / ADR-0042 / ADR-0044 (carry-forward seams for Pick A)
- [`spec/backlog.md § Process / tooling`](../backlog.md)
- [`ui-testability-deep-dive-2026-05-15.md`](ui-testability-deep-dive-2026-05-15.md)

## Closing

**Pick A** dominant: largest per-cycle benefit, empirical precedent
(harness v0.1.0 paid off same-week), zero new-ADR cost. **Pick B**:
cheap, durable, ready-when-you-need-it. **Pick C**: "already paid
reactively 3×" pre-emptive hygiene at sub-week cost. No Queue
promotions made; read-only pre-positioning under the durable contract.
