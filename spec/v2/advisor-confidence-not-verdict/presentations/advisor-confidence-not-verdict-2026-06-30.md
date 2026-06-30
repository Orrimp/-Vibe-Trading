---
slug: advisor-confidence-not-verdict
mode: release
status: draft
audience: human-operator
updated: 2026-06-30
generated: 2026-06-30T07:40:00Z
---

# Confidence check, not verdict — release · Phase 2A closer

## TL;DR

The forward paper-trade now reads as a **confidence check on the crown the
bake-off already chose**, not a fresh verdict — and **Phase 2A is closed**:
P0-1 scorecard, P1 turnover + tail, P0-3 framing — all three credibility-layer
features now tester-PASS, frozen gate byte-untouched, anchors 119/119.

## What changed

- **The Plan screen's headline reads "Confidence check"** (not "Your plan").
  The caption underneath restates the honest framing: *"a confidence check on
  that pick, not a fresh prediction or a guarantee of future edge."* Same paper
  trade, same €200, same standing rules — the relabel just says out loud what
  the screen has always been.
- **A 4-row "How much to trust this pick" block now appears below the plan**,
  surfacing the four facts from P0-1's scorecard alongside the paper-trade:
  *Strategies tried · Deflated confidence · Beats holding? · Minimum history
  needed*. The same scorecard the operator read on the leaderboard while
  crowning is now visible while watching — so "watching it" never gets
  confused with "a new bet being placed."
- **It's pure read-path projection** — a 4-field thin projection
  (`ScorecardSummary`) of the existing `Scorecard`, carried on
  `ForwardPlan.confidence: Option<ScorecardSummary>`, mirrored to `ui` as
  `ConfidenceSummaryView` across the existing ADR-0062 forward-plan mirror
  seam. Zero new dep edges, zero new widgets, zero new theme tokens, zero
  changes to the FROZEN gate. P0-1's `scorecard_does_not_change_ranking`
  identity test still proves `rank_candidates` is byte-identical.

## Why

The v2 analyst's load-bearing workflow gap (`spec/v2/v2-analysis.md` §1
Stage 4): *"the forward paper-trade today reads as a fresh verdict; v2's
honest framing is that it's a **confidence check on a crown already decided**
by the bake-off."* That single relabel + a 4-fact panel make the gap close
mechanically — the SUGGESTION stage no longer implies "this is a new
prediction," and the same trial-aware honesty numbers from ANALYSIS travel
forward into SUGGESTION so the operator sees one consistent honesty story
across the whole journey. The architect's §1 P0-3 entry calls this out as
`[A]` *additive* — reuse P0-1's output through the existing mirror seam, no
new structure (`spec/v2/v2-architecture.md` §1 P0-3 + §5 Phase 2A roadmap +
§6.0 D3 report-only contract).

## What you can do now

| Action | Command |
|--------|---------|
| Open the cockpit, run a bake-off + watch the forward plan paint the new "Confidence check" headline + 4-row honesty block | `cargo run --release -p ui --bin cockpit_live --features fixtures,live` (Leaderboard → run bake-off → Plan tab) |
| Re-prove the FROZEN gate is still byte-identical (the P0-1 identity test) | `cargo test -p backtest --lib bakeoff::scorecard::tests::scorecard_does_not_change_ranking` |
| Re-prove the new `ScorecardSummary` projection (positive + degenerate-empty cases) | `cargo test -p backtest --lib bakeoff::scorecard::tests::scorecard_summary` |
| Re-prove the confidence block paints at the pixel layer + the negative control (`confidence: None` ⇒ block entirely absent) | `cargo test -p ui --test forward_plan_confidence_render --features fixtures` |
| Re-prove the anchored gate is whole | `bash scripts/verify_anchors.sh` |

## Live demo

The load-bearing demo is the rendered PNG from
`forward_plan_confidence_render` itself — the exact pixels the operator will
see when a bake-off crowns a pick and the Plan tab opens. The render test
exercises `fake_forward_plan_with_confidence()`
(`n_candidates=18, deflated_sharpe=0.87, crown_clears_dsr=false, min_btl_years=6.4`)
— a realistic "search ran wide, edge is uncertain" reading.

Verbatim from the tester's reading of `/tmp/forward_plan_confidence_render.png`
(commit `bcc4c24`, run_id `2026-06-30-1430-UTC`):

```
[ Confidence check ]
Watching the crowned strategy as new bars arrive — a confidence check on
that pick, not a fresh prediction or a guarantee of future edge.
The same rules your simulated €200 paper-trade runs.

[ ... existing plan blocks: framing banner, stance, rules,
      sizing, horizon ... ]

[ How much to trust this pick ]
This is a confidence check on the crowned pick, not a fresh verdict.
The pick was chosen in the bake-off; this is how much to trust that choice.

  STRATEGIES TRIED
  18
  Each extra strategy tried raises the bar for the winner.

  DEFLATED CONFIDENCE
  87%
  Probability the pick's edge is real after correcting for the number
  of tries. Above 95% is the honest bar.

  BEATS HOLDING?
  ⚠ Not yet — edge uncertain after the search
  Informational only — never a rule change. The pick stands regardless.

  MINIMUM HISTORY NEEDED
  6.4 yr
  Years of data required to distinguish real edge from luck at this
  trial count.

The confidence block is informational — it does not change the pick
or the rules.
```

Read the headline + the four rows together. The screen says: *the search
tried 18 strategies, the crown's deflated confidence sits at 87% (under
the 95% honest bar), it has not yet shown it beats holding after the
search, and you'd want 6.4 years of data to call this real.* The plan
above still runs the same rules and the same €200 — the operator just
sees the credibility of that pick out loud while it paper-trades. That is
the entire P0-3 contract.

The full tester report (commit `bcc4c24c3a3c3508587bfd8ed625989b422db5cc`)
is at
`spec/v2/advisor-confidence-not-verdict/reports/test-2026-06-30-advisor-confidence-not-verdict.md`.

## Screenshots

The centerpiece — the Plan tab with the "Confidence check" headline + the
new 4-row "How much to trust this pick" block painted below the horizon
band, plus the negative control:

- `/tmp/forward_plan_confidence_render.png` — the populated case rendered by
  `cargo test -p ui --test forward_plan_confidence_render --features fixtures`
  → test `confidence_block_paints_more_foreground_than_without`. Shows the
  headline relabel + the disclaimer banner + the existing plan blocks + the
  new "How much to trust this pick" 4-row block painted below the horizon,
  with the load-bearing footer *"Informational only — never a rule change.
  The pick stands regardless."* Fixture values match the spec:
  `n_candidates=18 / deflated_sharpe=0.87 / crown_clears_dsr=false /
  min_btl_years=6.4`. Path is transient (re-generated on demand); the test
  itself + the tester's verbatim reading in §7 of the report are the durable
  evidence.

- `/tmp/forward_plan_no_confidence_render.png` — the negative control
  rendered by `confidence_block_below_horizon_band`'s sibling case. The same
  "Confidence check" headline still renders (the relabel is permanent), and
  the 4-row block is **entirely absent** when `confidence: None`. This
  proves the `if let Some(c) = confidence { ... }` guard fires correctly —
  the v3-vol-overlay-noop lesson applied at the render layer: the block
  actually draws content, it isn't a no-op label.

## Verification

Pasted verbatim from the tester's `VERDICT → PASS` report
(`spec/v2/advisor-confidence-not-verdict/reports/test-2026-06-30-advisor-confidence-not-verdict.md`,
commit `bcc4c24`):

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1 | `ScorecardSummary` is a 4-field projection of the existing `Scorecard`; `Scorecard::summary()` returns `None` on the degenerate `n_candidates == 0` case | VERIFIED | `bakeoff::scorecard::tests::scorecard_summary_positive_case` + `scorecard_summary_degenerate_yields_none` PASS — test report §3 |
| V2 | `confidence: Option<ScorecardSummary>` carried on both `ForwardRunConfig` and `ForwardPlan`; `build_forward_plan_from_registry` propagates it without new dep edges | VERIFIED | `cargo test -p agent` — 101 pass; `test_forward_plan_builds_from_cfg` PASS — test report §3 |
| V3 | `ConfidenceSummaryView` mirror crosses the `forward_plan/adapter.rs` (ADR-0062) boundary as plain `usize`/`f64`/`bool` — zero new `ui` dep edge on `strategy`/`exec`/`llm` | VERIFIED | `cargo test -p ui --lib` — 583 pass — test report §3; mirror lives at `crates/ui/src/forward_plan/adapter.rs` |
| V4 | FORWARD_PLAN_HEADLINE relabelled to "Confidence check"; FORWARD_PLAN_CAPTION updated; 14 new P0-3 string constants registered in `all_strings()` | VERIFIED | inventory check + render test paints headline + caption + 4-row block — test report §7 |
| V5 | The confidence summary block paints below the Horizon block with all 4 fact rows (strategies / DSR / beats-holding / min history) + the load-bearing "Informational only — never a rule change. The pick stands regardless." footer | VERIFIED | `forward_plan_confidence_render::confidence_block_paints_more_foreground_than_without` PASS — fixture values 18 / 87% / "Not yet — edge uncertain" / 6.4 yr — test report §7 |
| V6 | Negative control: `confidence: None` ⇒ the entire 4-row block is absent (the `if let Some(c) = confidence { ... }` guard fires); the v3-vol-overlay-noop render-layer lesson | VERIFIED | `forward_plan_confidence_render::confidence_block_below_horizon_band` PASS — PNG shows only the disclaimer at bottom — test report §7 |
| V7 | **FROZEN-gate identity** — P0-1's `scorecard_does_not_change_ranking` test still PASS, proving `rank_candidates` is byte-identical (P0-3 is a pure read-path projection over the existing scorecard; the bake-off path is byte-untouched) | VERIFIED | `bakeoff::scorecard::tests::scorecard_does_not_change_ranking` PASS — test report §3 |
| V8 | Anchors hold 119/119 — the forward-plan path runs `write_report=false` by construction, so confidence is anchor-safe | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)` — test report §8 |
| V9 | Spec-lint PASS (0 violations) — no spec structural regression | VERIFIED | `python3 scripts/spec_lint.py` → `spec-lint: PASS (0 violations)` — test report §9 |
| V10 | Clippy `-D warnings` clean on both crates touched | VERIFIED | `cargo clippy -p backtest --tests -- -D warnings` PASS · `cargo clippy -p ui --tests --features fixtures -- -D warnings` PASS (0 warnings each) — test report §2 |
| V11 | `cargo fmt --check` clean | VERIFIED | no diffs emitted, exit 0 — test report §2 |
| V12 | Full regression: 881 tests pass / 0 fail / 8 pre-existing ignores | VERIFIED | `backtest` 195/0/8 · `agent` 101/0/0 · `ui --lib` 583/0/0 · `ui` render 2/0/0 — test report §3 |
| V13 | Cockpit smoke: 7s window, RUST_BACKTRACE=1, zero panics | VERIFIED | `cargo build -p ui --features fixtures,live` `Finished` (1.15 s); panic count = 0 — test report §10 |

## Numbers that matter

- Tests: **881 passed / 0 failed / 8 pre-existing ignores**
  (`backtest` 195/0/8, `agent` 101/0/0, `ui --lib` 583/0/0, `ui` render
  integration 2/0/0). The 2 NEW unit tests are
  `scorecard_summary_positive_case` + `scorecard_summary_degenerate_yields_none`;
  the 2 NEW render tests are `confidence_block_paints_more_foreground_than_without`
  + `confidence_block_below_horizon_band`.
- Anchors: **119 / 119** PASS — frozen gate byte-immutable (forward-plan path
  uses `write_report=false`; the read-path projection is anchor-safe by
  construction).
- Spec-lint: **PASS (0 violations)** — baseline of
  `spec/dev-notes/audit-2026-06-29.md` held.
- Render-layer cost: **62.64 s** for the 2 macOS render tests
  (`forward_plan_confidence_render`); within the existing macOS render-suite
  budget.
- UI surface added: **14** new P0-3 `FORWARD_PLAN_CONFIDENCE_*` string
  constants + the `FORWARD_PLAN_HEADLINE`/`FORWARD_PLAN_CAPTION` relabel; all
  registered in `strings::all_strings()`. **Zero** new theme tokens, **zero**
  new widgets, **zero** new `ui` dep edges.
- Backend `ScorecardSummary` projection: **4 fields**, **+88 lines** to
  `crates/backtest/src/bakeoff/scorecard.rs` (`ScorecardSummary` struct +
  `Scorecard::summary()` method + 2 unit tests).
- Single feature commit: `bcc4c24c3a3c3508587bfd8ed625989b422db5cc` —
  18 files changed, 840 insertions, 14 deletions. Author: developer
  (backend + UI in one pass — smaller scope warranted it).

## Phase 2A — closed

This deck closes the **credibility layer phase** the v2 architect scoped on
2026-06-28 (`spec/v2/v2-architecture.md` §5 Phase 2A). All three features
are now tester-done with FROZEN gate byte-untouched and anchors 119/119:

| Feature | Tester verdict | Commit | Presentation |
|---|---|---|---|
| P0-1 advisor-overfitting-scorecard (DSR + MinBTL + N_eff on the Leaderboard's "How much to trust this" block) | PASS 2026-06-29 | `d3a9a4a` (tester) | `spec/v2/advisor-overfitting-scorecard/presentations/advisor-overfitting-scorecard-2026-06-29.md` |
| P1 advisor-turnover-and-tail-metrics (Churn column + "Risk story" tail block: CVaR/ES, median, skew, Sortino) | PASS 2026-06-29 | `decbcc4` (tester) | _no deck — rolled into this Phase 2A closer_ |
| P0-3 advisor-confidence-not-verdict (this feature — "Confidence check" relabel + 4-row block on the Plan) | PASS 2026-06-30 | `bcc4c24` (feature) | **this deck** |

The three features compose into one coherent story: the same trial-aware
honesty numbers (N / DSR / MinBTL) now appear on the **leaderboard** (where
the operator crowned the pick) and on the **forward plan** (where the
operator watches it paper-trade) — and the leaderboard now also surfaces
the **turnover** and **tail/median** numbers that explain *why* holding
usually wins. One consistent credibility surface across ANALYSIS and
SUGGESTION, all additive, frozen gate untouched.

## What's next

Phase 2B (per `spec/v2/v2-architecture.md` §5):

- **R1 forward-fidelity coverage** — `build_registry_for`
  (`crates/agent/src/runtime.rs:335`) must learn the 14 post-F5b crownable
  arms (5 DSL primitives + 6 ensembles + `v0.dvol_regime` + `v0.macro_riskon` +
  1 floor). Today if the bake-off crowns one of those arms, the forward run
  `bail!`s. Small, well-fenced, no FROZEN-gate impact — pure dispatch
  widening. **The correctness prerequisite for any expanded SUGGESTION
  stage.**

Phase 2C (after R1 lands): drawdown-control overlay + vol-overlay reposition
+ σ̂ multi-horizon estimator. Phase 2D: cost-model opt-in + DATA-quality
surface + narration-faithfulness hardening + no-alpha-gate CI.

ADR-0076 is reserved for atomic registration of this feature (the
`ScorecardSummary` projection + the `ForwardPlan.confidence` carrier + the
"Confidence check" relabel contract) when the operator approves this deck —
per the 2026-05-29 contract, written = registered when the feature lands.

## What this deliberately doesn't do (and why)

- **No new credibility math.** `ScorecardSummary` is a thin projection of
  the existing `Scorecard` from P0-1. The DSR / MinBTL / N_eff numbers are
  identical; only four are surfaced at the forward-plan surface (the four
  the operator actually needs while watching a paper trade). `pbo` and
  `n_eff` are intentionally excluded — `pbo` is `None` in v2 (D1 deferred)
  and `n_eff` is an implementation detail.
- **No veto, no gate.** The 4-row block is informational. The footer
  *"Informational only — never a rule change. The pick stands regardless."*
  is repeated below the block AND restated in the closing note. The
  FROZEN gate (`classify_verdict` / `rank_candidates` / `verdict_bands`) is
  byte-untouched (P0-1's `scorecard_does_not_change_ranking` identity test
  still PASS). The `crown_clears_dsr` flag is still report-only in v2 — D3
  in `spec/v2/v2-architecture.md` §6.0 reserves the one-line veto switch
  for a future operator call.
- **No anchored-report side-effects.** The forward-plan path runs
  `write_report=false` by construction, so this whole feature is anchor-safe.
  Anchors hold 119/119 — same number as before P0-1, P1, and P0-3 landed.

The framing comes from showing the work, not from a new claim. The same
scorecard the operator read while crowning is now visible while watching.
That is the entire ask.

## Open decisions

_No engineering decisions pending — all D1–D9 calls were operator-ratified
on 2026-06-28 (`spec/v2/v2-architecture.md` §6.0) and shipped as
specified. The confidence carrier reuses P0-1's `Scorecard`; the relabel +
4-row block reuses the existing ADR-0062 mirror seam._

One product-direction note (not a decision): the same `ScorecardSummary`
that now travels into `ForwardPlan` is also a candidate to travel into the
Live view's running paper-trade so the operator sees the honesty number
**while the trade is live**, not just on the snapshot Plan tab. This is
out of scope for Phase 2A and would land naturally in Phase 2B/2C
alongside the R1 forward-fidelity coverage refactor.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

### Notes / feedback

<empty until operator fills>

## Changelog

- 2026-06-30 (presenter): initial Phase 2A closer deck — TL;DR + the "Watching
  the crowned strategy / not a fresh verdict" framing, the worked render-test
  PNG (fixture values 18 / 87% / "Not yet — edge uncertain" / 6.4 yr) pulled
  verbatim from `/tmp/forward_plan_confidence_render.png`, the V1–V13
  verification matrix re-citing the tester's `VERDICT → PASS` report (commit
  `bcc4c24`, run_id `2026-06-30-1430-UTC`), the Phase 2A roll-up table (P0-1
  + P1 + P0-3 all tester-done), and the Phase 2B handoff (R1 forward-fidelity
  coverage as the correctness prerequisite for any expanded SUGGESTION
  stage).
