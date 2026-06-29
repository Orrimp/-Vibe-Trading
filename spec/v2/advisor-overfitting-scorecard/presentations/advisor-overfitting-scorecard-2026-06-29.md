---
slug: advisor-overfitting-scorecard
mode: release
status: draft
audience: human-operator
updated: 2026-06-29
generated: 2026-06-29T14:05:00Z
---

# The Honesty Scorecard — "how much to trust this" — release

## TL;DR

We just shipped the credibility layer: every bake-off recommendation now
carries an **honesty check next to it** answering the one question the
whole product rests on — *"did we fool ourselves by trying many strategies?"*
The block reads in plain language ("Strategies tried: 2 — about 2 truly
independent. Deflated confidence: 38%. Minimum history needed: about 1.1
years. Beats holding after the search? ✗ Not clearly — holding is the
honest call."), it is **report-only — it never changes the pick above**,
and the frozen robustness gate is proven byte-identical before and after
the scorecard runs.

## What changed

- **Backend Scorecard** — a new pure module `crates/backtest/src/bakeoff/scorecard.rs`
  (880 lines) computes the three closed-form numbers — Strategies tried &
  effective (N / N_eff), Deflated confidence (DSR), Minimum history needed
  (MinBTL) — from inputs that already exist (per-candidate Sharpe vector +
  the crown's bootstrap distribution). The scorecard is carried on
  `Recommendation.scorecard` and **only ever logged or surfaced** — it is
  never fed into ranking, the frozen gate, or the verdict.
- **The "How much to trust this" leaderboard block** — directly under the
  ranked table, the cockpit now paints a four-fact panel (introductory
  line, then the four facts each with a plain-language gloss), reusing the
  existing `frame::panel` (zero new widgets, zero new theme tokens). The
  block is omitted on the degenerate `n_candidates == 0` case so a fresh
  screen doesn't paint a misleading all-zero readout.
- **The gate-identity proof** — `scorecard_does_not_change_ranking`
  (`bakeoff/scorecard.rs` unit test) asserts `rank_candidates` produces
  byte-identical output (crowned, outcome, order) before and after the
  scorecard is computed. The FROZEN robustness gate
  (`verdict_bands` + `classify_verdict`) is provably untouched. Anchors
  hold **119/119** because the advisor bake-off runs `write_report=false`
  — the scorecard is anchor-safe by construction.

## Why

The product thesis is "traceable & plausible," not "we found alpha." Nine
independent research reviews (900 papers) reached the same conclusion: on a
single coin, no active strategy reliably beats simply holding once costs
are paid. That's not a bug to fix — it's the credibility we sell. **The
scorecard makes the search behind the verdict visible**, so when the
modal answer is "just hold," it reads as the expected, fine answer, not a
failure. The architect's verdict for the whole v2 phase
(`spec/v2/v2-architecture.md` §3) is **"no plugin architecture — stay
additive"**: three latent registration seams (arm / overlay /
**report-annex**) are formalized rather than building a runtime plugin
host. The Scorecard *is* the canonical report-annex seam — one struct,
one carrier field, one mirror field, no host, no registry, no dynamic
dispatch. This is the literal "traceable & plausible" product thesis
made visible, P0-1 in the v2 plan (`spec/v2/v2-architecture.md` §1).

## What you can do now

| Action | Command |
|--------|---------|
| Open the cockpit, run a bake-off on BTCUSDT, read the "How much to trust this" block under the leaderboard | `cargo run --release -p ui --bin cockpit_live --features fixtures` (Leaderboard → run bake-off) |
| Re-prove the frozen gate is byte-identical with and without the scorecard | `cargo test -p backtest --lib bakeoff::scorecard::tests::scorecard_does_not_change_ranking` |
| Re-prove the full backend scorecard suite (16 unit tests + DSR worked examples + N_eff edge cases + MinBTL formula + Acklam-Halley `normal_inv_cdf` roundtrip) | `cargo test -p backtest --lib bakeoff::scorecard` |
| Re-prove the UI block paints at the pixel layer (macOS render harness) | `cargo test -p ui --test leaderboard_scorecard_render --features fixtures` |
| Re-prove the anchored gate is whole | `bash scripts/verify_anchors.sh` |

## Live demo

The load-bearing demo is the worked example baked into the render-test
PNG itself. The render test exercises the **modal `BenchmarkWins` case**
— two candidates (buy-and-hold + a single SMA) tried against BTCUSDT,
no active strategy clears the robustness bar — and is the exact output
the operator will see for "the honest, common answer." Verbatim from the
tester's reading of `/tmp/leaderboard_scorecard_render.png`:

```
[ Recommendation ]
No active strategy cleared the robustness bar on BTCUSDT — simply
holding (buy-and-hold) is the least-bad choice on this window.

[ Ranked table ]
#  Strategy       Return    Sharpe   Max-DD     Trades
1  v0.buyhold ★   +11.24%   0.6900   −13.38%        2
2  v0.sma          +1.43%   0.2100   −15.21%       41

[ How much to trust this ]
An honesty check on the search behind the pick — it never changes
the result.

  STRATEGIES TRIED
  2 — about 2 truly independent

  DEFLATED CONFIDENCE
  38%
  Chance the edge is real after accounting for how many we tried.

  MINIMUM HISTORY NEEDED
  about 1.1 years of data
  Trust the result only with at least this much history behind it.

  BEATS HOLDING AFTER THE SEARCH?
  ✗ Not clearly — holding is the honest call
  Informational, not a gate — this never changes the pick above.
```

Read those rows together. The verdict above (buy-and-hold crowned) and
the four facts below describe the *same picture*: only two strategies
in the field, the deflated confidence is modest, and the search hasn't
beaten holding. The scorecard isn't telling you "you missed an edge"
— it's confirming the search was *honest* and the modest verdict is
the right one. Most of the time it will read like this, and **that is
the point.**

The full tester report (run_id `2026-06-29-1320-UTC`, commit `d3a9a4a`)
is at
`spec/v2/advisor-overfitting-scorecard/reports/test-2026-06-29-advisor-overfitting-scorecard.md`.

## Screenshots

The centerpiece — the cockpit's leaderboard with the new "How much to
trust this" block painted under the ranked table:

- `/tmp/leaderboard_scorecard_render.png` — the rendered 1920×1080
  dark-theme leaderboard captured by
  `cargo test -p ui --test leaderboard_scorecard_render --features fixtures`
  → test `scorecard_block_present_in_benchmark_wins_modal_case`. Shows
  the BTCUSDT bake-off with the modal `BenchmarkWins` outcome — the
  recommendation header reads "No active strategy cleared the robustness
  bar on BTCUSDT — simply holding (buy-and-hold) is the least-bad choice
  on this window", the ranked table holds 2 rows (v0.buyhold ★ best,
  v0.sma below), and the new scorecard block paints clearly below the
  table with all four facts plus their plain-language glosses legible.
  Path is transient (re-generated by the render test on demand); the
  test itself + the tester's verbatim reading in the report are the
  durable evidence.

- The negative-control proof
  (`scorecard_block_paints_and_exceeds_no_scorecard`) confirms the
  with-scorecard frame paints strictly more foreground (>1200 px delta)
  than the same frame with `scorecard = None` — i.e. the block actually
  draws content, it isn't a no-op label. This is the v3-vol-overlay-noop
  lesson applied at the render layer.

- Cockpit-smoke log:
  `spec/v2/advisor-overfitting-scorecard/reports/cockpit-smoke-2026-06-29T08-41Z.log`.
  Empty file → the smoke probe found `0` panic markers
  (`grep -c "panicked at\|non-unwinding panic\|fatal runtime error"`).
  An empty smoke log is the clean-quiet-boot signature (iced fixtures
  emits no tracing in the smoke window). Per the cockpit-smoke skill's
  capability boundary, the smoke is Orchestrator-only — the ui-designer
  produced the log and the tester defers to it.

## Verification

Pasted verbatim from the tester's `VERDICT → PASS` report
(`spec/v2/advisor-overfitting-scorecard/reports/test-2026-06-29-advisor-overfitting-scorecard.md`, commit `d3a9a4a`):

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1 | The scorecard is computed from inputs that already exist (Sharpe vector + bootstrap distribution) and lives on `Recommendation.scorecard` | VERIFIED | `compute_scorecard_single_candidate` / `compute_scorecard_degenerate_empty` PASS — see test report §3 |
| V2 | Closed-form `N_eff = ρ̄ + (1−ρ̄)·M` is frozen at the 24-config scale (D4) | VERIFIED | 4 N_eff tests PASS — `n_eff_empty_returns_zero`, `n_eff_single_candidate`, `n_eff_perfectly_correlated_returns_one`, `n_eff_uncorrelated_field_approaches_m` |
| V3 | `min_btl ≈ 2·ln(N)/SR²` matches the literature formula across the edge cases | VERIFIED | 3 MinBTL tests PASS — `min_btl_formula_matches_2lnn_over_sr2`, `min_btl_n_eq_24_sr_eq_1`, `min_btl_zero_for_n_le_1` |
| V4 | DSR matches the research worked examples (fails-at-N=100, passes-at-N=46, clears-at-N=88-Normal) | VERIFIED | 3 DSR tests PASS — `dsr_research_worked_example_fails_at_n100`, `dsr_research_worked_example_passes_at_n46`, `dsr_normal_returns_clears_at_n88` |
| V5 | High-accuracy `normal_inv_cdf` (Acklam rational + one Halley refinement step) roundtrips against `normal_cdf` to a tight tolerance | VERIFIED | `normal_inv_cdf_roundtrip` + `normal_cdf_symmetry_and_boundary` PASS |
| V6 | **FROZEN-gate identity** — `rank_candidates` byte-identical with and without the scorecard | VERIFIED | `scorecard_does_not_change_ranking` PASS (the load-bearing test for D3 — report-only, never a veto) |
| V7 | PBO is intentionally `None` in v2 (deferred to the Tune/sweep surface per D1) | VERIFIED | `compute_scorecard_pbo_always_none` PASS |
| V8 | `ScorecardView` mirror crosses the `BakeoffReportMirror::from_report` boundary as plain `usize`/`f64`/`bool` (zero new `ui` dep edge); `None` on degenerate empty field | VERIFIED | `state::tests::scorecard_view_mirrors_a_populated_scorecard` + `state::tests::scorecard_view_is_none_for_degenerate_empty_field` PASS — see test report §3 |
| V9 | The "How much to trust this" block paints at the pixel layer with all four facts + glosses (CLAUDE.md non-negotiable for UI) | VERIFIED | `scorecard_block_paints_and_exceeds_no_scorecard` + `scorecard_block_present_in_benchmark_wins_modal_case` PASS (2/2, 94.20s); PNG read by tester — see report §7 |
| V10 | The negative control proves the block paints *content* (>1200 px foreground delta), not just a no-op label — the v3-vol-overlay-noop lesson at the render layer | VERIFIED | `scorecard_block_paints_and_exceeds_no_scorecard` PASS (foreground delta >1200 px) |
| V11 | The frozen gate is byte-immutable at the system level — anchors 119/119 | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)` (advisor bake-off runs `write_report=false` → scorecard is anchor-safe by construction) |
| V12 | Clippy `-D warnings` clean across the two crates touched | VERIFIED | `cargo clippy -p backtest --tests -- -D warnings` PASS · `cargo clippy -p ui --tests --features fixtures -- -D warnings` PASS (0 warnings each) — see test report §2 |
| V13 | `cargo fmt --check` clean | VERIFIED | (no output, exit 0) — see test report §2 |
| V14 | Full regression: 759 tests pass / 0 fail / 8 pre-existing ignores (all 8 unrelated to scorecard) | VERIFIED | `backtest` 178/0/8, `ui` lib 579/0/0, `ui` render 2/0/0 — see test report §3 |
| V15 | `spec_lint.py` PASS (0 violations) — no spec structural regression | VERIFIED | `python3 scripts/spec_lint.py` → `spec-lint: PASS (0 violations)` — see test report §9 |
| V16 | Cockpit-smoke shows zero panics in the boot window | VERIFIED (orchestrator-attributed) | `spec/v2/advisor-overfitting-scorecard/reports/cockpit-smoke-2026-06-29T08-41Z.log` empty → `grep -c "panicked at|non-unwinding panic|fatal runtime error"` = 0 (ui-designer-produced; tester defers per skill capability boundary) |

## Numbers that matter

- Tests: **759 passed / 0 failed / 8 pre-existing ignores** (16 backend scorecard
  unit tests + 2 `ScorecardView` mirror tests + 2 render-layer tests, all PASS;
  remaining 739 = full `backtest` + `ui` regression).
- Anchors: **119 / 119** PASS — frozen gate byte-immutable (advisor bake-off
  runs `write_report=false`; scorecard is anchor-safe by construction).
- Spec-lint: **PASS (0 violations)** — baseline of the
  `spec/dev-notes/audit-2026-06-29.md` audit held.
- Render-layer cost: **94.20s** for the 2 macOS render tests
  (`leaderboard_scorecard_render`); within the existing macOS render-suite
  budget.
- Scorecard backend module size: **880 lines** (`scorecard.rs`), of which
  16 unit tests including the gate-identity proof.
- UI surface added: **13** new `LEADERBOARD_SCORECARD_*` string constants
  (registered in `strings::all()`), **zero** new theme tokens, **zero**
  new widgets, **zero** new `ui` dep edges.
- Code commits in this feature:
  - `9c3c002` — backend scorecard module (developer)
  - `ac7c779` — UI "How much to trust this" block (ui-designer)
  - `d3a9a4a` — `advisor_field` arm-count refresh + clippy-1.94 `allow` annotations
  - `1d5b114` — tester `VERDICT → PASS` (this presentation's source of truth)
- Follow-on already shipped after PASS:
  - `66286e2` — P1-1 turnover + P1-2 coherent-tail KPIs (Phase 2A
    continuation, see "What's next" below).

## What this deliberately doesn't do (and why)

Per `spec/v2/v2-architecture.md` §6.0 (operator-ratified 2026-06-28):

- **Report-only — never a veto** (D3). The scorecard is logged and
  surfaced; `rank_candidates` and the frozen `verdict_bands` /
  `classify_verdict` never read it. A DSR/PBO crown-veto would be a
  FROZEN-gate change and needs its own ADR + an operator call. The
  `Scorecard.crown_clears_dsr` flag is *informational*, designed as a
  one-line switch so a *later* veto is a small change — the carrier is
  already there.
- **PBO deferred to the Tune/sweep surface** (D1). PBO/CSCV is
  statistically meaningful on a homogeneous sweep grid (24 cells of one
  family). On an 18-arm heterogeneous bake-off field it's marginal. We
  ship closed-form DSR/MinBTL/N_eff in v2 (most credibility, zero risk);
  PBO lands on the Tune surface in a later increment.
  `compute_scorecard_pbo_always_none` proves the v2 invariant: `pbo`
  is always `None`.
- **N_eff frozen at the 24-config scale, closed-form** (D4). At
  `MAX_SWEEP_CONFIGS = 24`, T ≫ 24 on any bootstrappable window — the
  literature's "must cluster first when M>T" rule doesn't apply to us,
  so the closed form `ρ̄ + (1−ρ̄)·M` is sufficient forever at this scale.
  The freeze closes the door on second-order snooping (CX-2).
- **No DSR threshold / no ORATIO** (D2). Hard-coding "DSR ≥ 0.95"
  or deriving the threshold from an odds-ratio is a values call that
  belongs *with* a veto decision (D3). We surface the haircut and let
  the operator read it; we do not crown or de-crown by DSR.
- **The FROZEN robustness gate is byte-untouched**. The gate-identity
  unit test `scorecard_does_not_change_ranking` proves
  `rank_candidates` produces byte-identical `crowned`, `outcome`, and
  `order` before and after the scorecard is computed. Anchors **119/119**
  is the system-level corroboration: every anchored report body's SHA
  is unchanged.

The credibility comes from showing the work, not from a winning pick.
This block is built to **confirm "just hold"** the majority of the time
— and that's the point.

## What's next

Phase 2A (the credibility layer) is on track:

- **P0-1 Scorecard — SHIPPED** (this deck).
- **P1-1 Turnover + P1-2 Tail/Median KPIs — SHIPPED**
  (commit `66286e2`, 2026-06-29, immediately after P0-1 PASS). Per
  `spec/v2/v2-architecture.md` §1 these are reductions over the
  *existing* 1000-path `PathMetrics` vector — additive, gate-untouched,
  anchor-safe. The ui-designer surfacing those into the leaderboard
  columns/blocks is in flight.

Phase 2B follow-on (next major work):

- **R1 forward-fidelity COVERAGE refactor** (`spec/v2/v2-architecture.md`
  §2 R1) — `build_registry_for` (`crates/agent/src/runtime.rs:335`)
  must learn the 14 post-F5b crownable arms (5 DSL primitives + 6
  ensembles + `v0.dvol_regime` + `v0.macro_riskon` + 1 floor). Today
  if the bake-off crowns one of those arms, the forward run `bail!`s.
  Small, well-fenced, no FROZEN-gate impact — pure dispatch widening.
- **P0-3 confidence-not-verdict framing** alongside the forward plan
  (mirrors this scorecard into `ForwardPlan` so the SUGGESTION stage
  reads the same honesty signal as ANALYZE).

ADR-0075 is **reserved** for atomic registration when the operator
approves this deck (per the 2026-05-29 contract — written = registered
when the feature lands).

## Open decisions

_No engineering decisions pending — all D1–D4 calls were operator-ratified
on 2026-06-28 (`spec/v2/v2-architecture.md` §6.0) and shipped as
specified._

One scope-honesty note (not a decision): the scorecard surfaces three
numbers (N/N_eff, DSR, MinBTL) plus the "Beats holding?" flag. PBO is
intentionally deferred. If the operator later wants the DSR/PBO veto
(the M-T1 lock — D3 in `v2-architecture.md`), the design has a one-line
switch ready (`Scorecard.crown_clears_dsr`), but it is a FROZEN-gate
change and needs its own ADR + operator call.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

### Notes / feedback

<empty until operator fills>

## Changelog

- 2026-06-29 (presenter): initial release deck — TL;DR + honesty framing,
  the BTCUSDT modal-`BenchmarkWins` worked example pulled verbatim from
  the render-test PNG, the V1–V16 verification matrix re-citing the
  tester's `VERDICT → PASS` report (commit `d3a9a4a`, run_id
  `2026-06-29-1320-UTC`), explicit §6.0 ratification record (report-only
  / N_eff frozen at 24-config / PBO deferred / no threshold), and the
  Phase 2A continuation note (P1-1+P1-2 already shipped under `66286e2`,
  ui-designer surfacing in flight).
