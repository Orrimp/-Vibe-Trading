---
slug: advisor-crown-credibility
status: in-progress
owner: ui-designer
updated: 2026-07-09
---

# Tasks — advisor-crown-credibility (P1, D1=(a) presentation-layer)

Ordered for the **ui-designer**. Presentation-only: **no `crates/backtest` edit**,
no gate change, no new anchors, no new field on any backtest type. The render test
(T7) is the closing gate — the feature is not done until the money-shot PNG visibly
shows the `WeakEvidence` band on the banner. See
[`feature.md`](feature.md) § Design + § UI for the exact copy, tokens, and states.

- [ ] **T1 — the pure resolver.** Add `enum CrownCredibility { Passes, WeakEvidence,
  NotApplicable }` + `fn crown_credibility(outcome: OutcomeKind,
  scorecard: Option<&ScorecardView>) -> CrownCredibility`, co-located with
  `recommendation_block` in `crates/ui/src/screens/leaderboard.rs` (or as an assoc.
  `fn` on `ScorecardView` in `leaderboard/state.rs`). Logic is EXACTLY the § Decision-1
  table: `ActiveWins`+`Some`+clears→`Passes`; `ActiveWins`+`Some`+fails→`WeakEvidence`;
  everything else (`BenchmarkWins`/`AllFragile`/`None`)→`NotApplicable`. Pure + total,
  no I/O, no panic. — _acceptance: reads only `OutcomeKind` + `Option<&ScorecardView>`;
  no new stored field; no `crates/backtest` change._

- [ ] **T2 — unit tests for the resolver.** One test per row of the § Decision-1 table
  (5 rows incl. `ActiveWins`+`None` and both non-`ActiveWins` outcomes) asserting the
  returned variant. — _acceptance: every table row covered; `BenchmarkWins`/`AllFragile`
  → `NotApplicable` explicitly asserted (the no-badge-on-a-hold-pick invariant)._

- [ ] **T3 — the copy strings.** Add `LEADERBOARD_CROWN_PASSES_DSR`,
  `LEADERBOARD_CROWN_WEAK_EVIDENCE`, `LEADERBOARD_CROWN_WEAK_EVIDENCE_HINT` (+ any glyph
  sub-constants) to `crates/ui/src/strings.rs`, registered in `strings::all()`. Copy is
  VERBATIM from feature.md § Decision 2 (plain-language, non-alarmist, `⚠`/`✓` glyphs).
  — _acceptance: zero inline literals in the render path; the consistency check (no
  inline strings/hex) passes; wording matches feature.md exactly._

- [ ] **T4 — the view element.** Add `fn crown_credibility_element(CrownCredibility,
  mode) -> Element`: `Passes` → one quiet `✓` line in `ACCENT`; `WeakEvidence` → a
  `width(Fill)` bordered band, `WARN_500` text + `⚠` on `WARN_50` fill, `WARN_500` 1px
  border + `radius::R3`, plus the muted `FG_3` `SMALL` hint line; `NotApplicable` →
  zero-size `Space` (the `templated_reasons`-empty idiom). All colour via
  `ModeColor::current(mode)` (dual-mode); no new theme token. — _acceptance: `cargo tree
  -p ui` unchanged; no hardcoded hex; `NotApplicable` renders nothing (byte-identical
  pre-feature layout for non-`ActiveWins`)._

- [ ] **T5 — wire into the banner.** In `recommendation_block`
  (`crates/ui/src/screens/leaderboard.rs`), push `crown_credibility_element(
  crown_credibility(report.recommendation.outcome, report.scorecard.as_ref()), mode)`
  **immediately after the `headline` push, before the `winner_robustness_clause` push**
  (§ Decision 4). — _acceptance: the element sits under the H2 headline, co-located with
  the crown; the scorecard panel + its `leaderboard_scorecard_render.rs` guard are
  UNTOUCHED._

- [ ] **T6 — dark/light + gallery.** Confirm both themes render (the band legible in
  `--theme dark` and `--theme light`); add a gallery cell for the `WeakEvidence` state
  if the widget is factored as a reusable widget (follow the ADR-0083 gallery-cell
  precedent; skip if it stays an inline `recommendation_block` helper). — _acceptance:
  both-theme legibility confirmed at the render walk._

- [ ] **T7 — RENDER PROOF (the closing gate, MANDATORY).** Add
  `crates/ui/tests/crown_credibility_render.rs`, `#![cfg(target_os = "macos")]`,
  mirroring `leaderboard_scorecard_render.rs` (hue/foreground pixel counts, PNGs to
  `/tmp/`, coarse thresholds robust to font jitter). Three guards:
  1. **`weak_evidence_band_paints_on_banner`** (the money shot) — render the real
     `screens::leaderboard::view` with `fixtures::fake_bakeoff_report_mirror_five_arm`
     (`ActiveWins`, `crown_clears_dsr=false`); assert the banner region paints the
     `WARN`-tier band (WARN hue present in the recommendation-panel band + strict
     foreground delta vs the SAME mirror with the credibility element suppressed, i.e.
     the negative control). Save `/tmp/crown_credibility_weak.png`.
  2. **`passes_state_is_control_not_weak`** (negative control) — the SAME `five_arm`
     mirror with `scorecard.crown_clears_dsr` flipped to `true`; assert the banner shows
     the `Passes` affordance and does NOT paint the WARN band (proves guard 1 is not a
     tautology — the band tracks the flag). Save `/tmp/crown_credibility_passes.png`.
  3. **`benchmark_wins_banner_has_no_credibility_band`** — `fixtures::fake_bakeoff_report_mirror_benchmark_wins`
     (`BenchmarkWins`); assert the banner has NO credibility band/line (the
     no-badge-on-a-hold-pick invariant at the pixel layer). Save
     `/tmp/crown_credibility_benchmark.png`.
  **READ all three PNGs and eyeball them** (MEMORY: a pixel count is a claim, not
  proof). — _acceptance: money-shot PNG visibly shows the ⚠ WeakEvidence band on the
  banner; the `Passes` + `BenchmarkWins` controls visibly differ; all three tests
  green on macOS._

- [ ] **T8 — gates + spec close.** Run and record verbatim:
  `cargo build -p ui`; `cargo test -p ui --lib`; `cargo test -p ui --test
  crown_credibility_render` (macOS); `cargo clippy -p ui --tests -- -D warnings`;
  `cargo fmt --check`; the consistency check; `cargo tree -p ui` (unchanged);
  `bash scripts/verify_anchors.sh` → **119/119**; `python3 scripts/spec_lint.py` →
  **PASS(0)**. Flip `feature.md` status `arch-done → dev-done`; update the trace row
  `crates`/`tests` cells + state (honoring ADR-0082). Handoff to tester. — _acceptance:
  every gate green + captured; anchors 119/119 before AND after; FROZEN gate byte-
  untouched (no `crates/backtest` diff)._

## Notes

- **The one bright line:** this is presentation-only. `rank.rs` must NEVER read
  `crown_clears_dsr` (do-not-build register E-1 / `dsr-report-only-decision-2026-07-09.md`
  D3). If a task drifts toward a veto, STOP.
- **Money-shot fixture already exists** — `fake_bakeoff_report_mirror_five_arm`
  (`ActiveWins`, `winner: v0.sma`, `crown_clears_dsr: false`). No new fixture needed
  for the primary state; T7 guard 2 mutates a clone's `crown_clears_dsr` to `true`.
- **Tone guardrail:** `WeakEvidence` is `WARN` tier, NOT error-red — the pick is
  weakly evidenced, not broken. If the band reads too loud at the render walk, thin it
  to border-only (a `view`-time call, no ADR amendment) — see feature.md § Risks.
- **Anchors keyed by NAME** — run `verify_anchors.sh` before and after; a stray
  `spec/*/reports/` edit is out of scope for this feature.
