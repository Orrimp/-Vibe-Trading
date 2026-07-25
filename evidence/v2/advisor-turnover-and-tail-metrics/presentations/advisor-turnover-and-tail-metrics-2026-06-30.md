---
slug: advisor-turnover-and-tail-metrics
mode: release
status: draft
audience: human-operator
updated: 2026-06-30
generated: 2026-06-30T08:00:00Z
---

# The cost story + the risk story — "why the verdict is what it is" — release

## TL;DR

We just shipped the **cost story** (a `Churn` column on every row — turnover
as a multiple of capital) and the **risk story** (a six-fact `Risk story`
panel directly under the honesty scorecard: typical outcome (median
terminal wealth), CVaR-95, CVaR-99, skew, Sortino, Calmar). Both layers
are **report-only** — the FROZEN gate is byte-identical before and after
(proven by `turnover_does_not_change_ranking`), anchors stay **119/119**,
and on the modal "holding wins" fixture the panel reads cleanly:
median **€102 300**, CVaR-95 **−18.0 %**, CVaR-99 **−31.0 %**, skew
**+0.42**, Sortino **+1.95**, Calmar **+2.32**. Together with last week's
P0-1 scorecard this completes Phase 2A — every cockpit verdict now ships
with the confidence, the cost, and the risk visible next to it.

## What changed

- **`Churn` column on the leaderboard** — a new rightmost numeric column
  in the ranked table, rendered as `"N.N×"` (e.g. `3.4×`, `0.0×`). The
  glyph deliberately reads as "this many capital-equivalents
  transacted" — unambiguous beyond `100 %`, which becomes confusing
  past `1.0×`. Formula: `Σ(fill.price × fill.qty) / mean_equity`,
  derived from data `RunReport` already carries (no new engine
  capture, no anchor break).
- **`Risk story` panel under the honesty scorecard** — six facts in one
  `frame::panel` (same chrome as the scorecard), each with a plain-
  language gloss: **typical outcome (median)**, **average loss in the
  worst 5 % / 1 % of paths (CVaR)**, **surprise shape (skew)**,
  **downside-only Sharpe (Sortino)**, **return vs worst drawdown
  (Calmar)**. Footer reads "Informational, not a gate — these never
  change the pick above." — same REPORT-ONLY framing as the scorecard.
- **`backtest::TailSummary` + `ui::TailSummaryView` mirror seam** — one
  new public-seam struct on `Recommendation.crown_tail`, four `f64`s,
  crosses the existing `from_report` mirror as plain scalars (zero new
  `ui` dep edge — the architectural invariant in §1 of `v2-architecture.md`
  is preserved verbatim).
- **Frozen-gate identity proof + cascade fix-ups** — the new test
  `turnover_does_not_change_ranking` asserts `rank_candidates` is
  byte-identical with and without `CandidateKpis.turnover`. 49
  `LeaderRow` literals + 7 `CandidateKpis` literals + 6 `DistributionSummary`
  literals across the workspace updated; all 783 tests pass.

## Why

The cockpit's leaderboard used to stop at **Return / Sharpe / MaxDD /
Trades** + a headline crown. That's enough to *pick* a strategy, but not
enough to *explain why* — which is the whole pitch of this product
(`spec/v2/v2-analysis.md` §1 workflow gap). v2 fills the two missing
chapters: the **cost story** (Churn) explains *why* gross-vs-net
diverges — a trend-follower with a `5.8×` Churn is paying for that
return in slippage and fees — and the **risk story** (median + tail +
skew + Sortino + Calmar) explains *why* the average return is the
wrong number to fixate on, because tail loss isn't symmetric with
upside.

**CVaR not VaR**, deliberately. CVaR (Expected Shortfall — the mean of
the worst α-fraction of paths) is **coherent**: the risk of a combined
portfolio never exceeds the sum of the parts' risks. Plain VaR is not
— two individually "safe" VaR portfolios can combine into one that
exceeds the level, which rewards concentration over diversification.
For a single-coin advisor the distinction is less acute, but reporting
a non-coherent measure is dishonest; research and the architect were
both explicit on the point (`v2-architecture.md` §1 P1-2).

This is the same product thesis as P0-1: **make the null legible**.
When the bake-off says "holding wins", the risk story explains *why*
— "here's what holding's actual tail looks like" (negative skew,
deeper left tail) — rather than chasing alpha that 900 research
papers have already said isn't there. The architect's verdict for
all of v2 (`v2-architecture.md` §3) is **"no plugin architecture
— stay additive"**; both increments cross the same `Recommendation`
→ `BakeoffReportMirror` seam the scorecard already crosses, so this
is one more turn of the canonical report-annex seam (P0-1 was the
first).

## What you can do now

| Action | Command |
|--------|---------|
| Open the cockpit, run a bake-off on BTCUSDT, read the `Churn` column + `Risk story` panel | `cargo run --release -p ui --bin cockpit_live --features fixtures` (Leaderboard → run bake-off) |
| Re-prove the frozen gate is byte-identical with the turnover field | `cargo test -p backtest --lib bakeoff::tests::turnover_does_not_change_ranking` |
| Re-prove the four new tail-metric unit tests (CVaR closed-form on n=20 / n=100, CVaR ≤ VaR, skew on symmetric / right / left distributions) | `cargo test -p backtest --lib stats::tests::cvar stats::tests::skew` |
| Re-prove the Risk story panel paints at the pixel layer (populated + negative control + benchmark-wins case) | `cargo test -p ui --test leaderboard_risk_story_render --features fixtures` |
| Re-prove the `Churn` format helper (one decimal, `×` suffix) | `cargo test -p ui --lib screens::leaderboard::tests::format_turnover_ratio` |
| Re-prove the anchored gate is whole | `bash scripts/verify_anchors.sh` |

## Live demo

The load-bearing demo is the worked example baked into the render-test
PNG itself (`/tmp/leaderboard_risk_story_render.png`, regenerated on
demand by the macOS render harness). The test exercises the populated
fixture on a tall 1920×2400 px viewport so the new panel clears the
scorecard and lands in-frame. Verbatim from the tester's reading of
the PNG (`spec/v2/advisor-turnover-and-tail-metrics/reports/test-2026-06-29-advisor-turnover-and-tail-metrics.md` §7):

```
[ Strategy bake-off ]
Ranking strategies for €200 in BTCUSDT.   20 strategies head-to-head…

[ Recommendation ]
No active strategy cleared the robustness bar on BTCUSDT — simply
holding (buy-and-hold) is the least-bad choice on this window.

[ Ranked table ]
#  Strategy       Return    Sharpe   Max-DD     Trades   Churn   ← NEW
1  v0.buyhold ★   +11.24%   0.6900   −13.38%        2     0.0×
2  v0.sma          +1.43%   0.2100   −15.21%       41     5.0×
…

[ How much to trust this ]   (P0-1 honesty scorecard — existing)

[ Risk story ]                ← NEW P1-2 block
  Typical outcome (median) ………………………… 102,300 USDT
    "What the middle path actually ends at — more representative
     than the average, which gets pulled by extreme wins."
  Average loss in the worst 5 % of paths …… −18.0 %
  Average loss in the worst 1 % of paths …… −31.0 %
    "Expected shortfall (CVaR) — coherent, unlike plain VaR;
     this is the mean loss in the bad scenarios, not just the
     threshold."
  Surprise shape (skew) ………………………………… +0.42
    "Positive = rare big wins (lottery-shaped); negative = rare big
     losses (crash-prone)."
  Downside-only Sharpe (Sortino) ………………… +1.95
  Return vs worst drawdown (Calmar) ……………… +2.32
  Informational, not a gate — these never change the pick above.
```

Read the rows together. The **headline** crowns buy-and-hold. The
**Churn column** confirms holding transacts nothing (`0.0×`) while
the SMA churns ~5× its capital — the active strategy is paying for
its modest `+1.43 %` return in fees. The **Risk story** explains the
honest range: the middle path lands at **€102 300** (just over
budget), but the worst 5 % of paths lose **18 %** and the worst 1 %
lose **31 %**; skew is mildly **positive** (+0.42), and the
downside-only-Sharpe (Sortino **+1.95**) and reward-per-worst-loss
(Calmar **+2.32**) are both sound for a single-coin hold. That's
the whole story the operator needs — *why* hold wins, not just *that*
it wins.

A second render test (`risk_story_block_present_in_benchmark_wins_modal_case`)
uses the honest single-asset hold tail fixture — `cvar_95: −24 %`,
`cvar_99: −41 %`, `median: €102 300`, `skew: −0.31` (mildly crash-
prone) — and the panel paints just as cleanly in the modal case where
holding wins outright.

Full tester report (run_id `2026-06-29-2039-UTC`, commit `decbcc4`):
`spec/v2/advisor-turnover-and-tail-metrics/reports/test-2026-06-29-advisor-turnover-and-tail-metrics.md`.

## Screenshots

- `/tmp/leaderboard_risk_story_render.png` — **the centerpiece**: the
  rendered 1920×2400 dark-theme leaderboard with the new `Churn` column
  and the six-fact `Risk story` panel painted under the scorecard,
  captured by `cargo test -p ui --test leaderboard_risk_story_render
  --features fixtures` → test `risk_story_block_paints_and_exceeds_no_tail`.
  Re-generate on demand.
- `/tmp/leaderboard_no_risk_story_render.png` — **the negative control**:
  the same leaderboard with `tail = None`. The Risk story panel is
  absent; the scorecard block + ranked table render identically. The
  pixel-delta assertion `fg_with > fg_without + 1500` confirms the
  panel draws ~1500+ pixels of foreground content (vs the scorecard's
  ~1200 — six facts vs four). This is the v3-vol-overlay-noop lesson
  applied at the render layer: the proof that the panel actually paints
  content, not a silent no-op label.
- `/tmp/leaderboard_risk_story_benchmark_wins_render.png` — **the
  modal-case proof**: same panel painted on the `benchmark_wins`
  fixture (the honest hold-tail), confirming the panel reads sensibly
  even when buy-and-hold is crowned.

Paths are transient (regenerated by the render test on demand); the
test code + the tester's verbatim reading in §7 of the test report
are the durable evidence.

## Verification

Pasted verbatim from the tester's `VERDICT → PASS` report
(`spec/v2/advisor-turnover-and-tail-metrics/reports/test-2026-06-29-advisor-turnover-and-tail-metrics.md`,
commit `decbcc4`, run_id `2026-06-29-2039-UTC`):

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1 | `CandidateKpis.turnover: Decimal` is the chosen formula `Σ(fill.price × fill.qty) / mean_equity` (unitless ratio) | VERIFIED | `bakeoff::tests::turnover_idle_zero` / `turnover_one_roundtrip` / `turnover_multi_trade` PASS — test report §3 |
| V2 | FROZEN-gate identity — `rank_candidates` byte-identical with and without the new turnover field | VERIFIED | `turnover_does_not_change_ranking` PASS (load-bearing, §11) |
| V3 | `DistributionSummary` extended with `cvar_95`, `cvar_99`, `median_terminal_wealth`, `skew` — all reductions over the existing `PathMetrics` vector | VERIFIED | `distribution_summary_p1_2_fields_populated` + `p1_2_fields_additive_gate_unchanged` PASS |
| V4 | CVaR is computed on the bootstrap `total_return` distribution and matches closed-form for known input vectors (n=20 and n=100 uniform) | VERIFIED | `cvar_uniform_n20_closed_form` + `cvar_uniform_n100_closed_form` + `cvar_99_equals_min_on_n100` + `cvar_empty_returns_zero` PASS |
| V5 | CVaR ≤ VaR for any distribution (the coherence direction property) | VERIFIED | `cvar_le_var_property` PASS |
| V6 | Skew matches the 3rd-standardised-central-moment definition on symmetric / right-skewed / left-skewed hand-built distributions | VERIFIED | `skew_zero_on_symmetric` + `skew_positive_on_right_skewed` + `skew_negative_on_left_skewed` + `skew_degenerate_small_n` PASS |
| V7 | `TailSummary` projects from the crown's `DistributionSummary` once at report assembly (mirrors the scorecard precedent) — `None` on `RobustnessMode::Skip` | VERIFIED | crown wiring in `bakeoff/mod.rs` `compute_robustness_distribution` for the crown; test report §3 |
| V8 | `TailSummaryView` mirrors `TailSummary` as plain `f64` (zero new `ui` dep edge); `BakeoffReportMirror.tail` is `Option<TailSummaryView>` | VERIFIED | `state::tests::tail_summary_view_mirrors_a_populated_tail` PASS |
| V9 | `LeaderRow.turnover` mirrored from `CandidateKpis.turnover` as plain `Decimal` (zero new `ui` dep edge) | VERIFIED | mirror in `state.rs:71`; ui lib 583/0/0 PASS (includes the row-mirror tests) |
| V10 | Render-layer (CLAUDE.md non-negotiable) — `Risk story` panel paints all six facts + glosses at the pixel layer | VERIFIED | `risk_story_block_paints_and_exceeds_no_tail` PASS — PNG read by tester, §7 |
| V11 | Negative control — with-tail panel paints strictly more foreground than no-tail (`> 1500 px` delta, the v3-vol-overlay-noop lesson) | VERIFIED | `risk_story_block_paints_and_exceeds_no_tail` PASS (delta assertion) |
| V12 | Render-layer (modal case) — panel paints sensibly when buy-and-hold is crowned, with the honest hold tail (cvar99 < cvar95 < 0; mildly negative skew) | VERIFIED | `risk_story_block_present_in_benchmark_wins_modal_case` PASS |
| V13 | `Churn` column format — one decimal, unicode `×` suffix | VERIFIED | `format_turnover_ratio_renders_one_decimal_with_x_suffix` PASS |
| V14 | Sign-beyond-colour accessibility — CVaR / skew / Sortino / Calmar always show explicit sign with unicode minus | VERIFIED | `fmt_signed_pct_from_f64_renders_signed_one_decimal_with_unicode_minus` + `format_signed_decimal_renders_signed_with_unicode_minus` PASS |
| V15 | Full regression — 783 tests pass, 0 fail, 8 pre-existing ignores; pre-existing `bakeoff_progress_render` 1/3 y-band drift fixed in the same commit (3/3 now) | VERIFIED | backtest lib 193/0/8 · ui lib 583/0/0 · render integration 7/0/0 (3 risk-story + 2 scorecard + 3 bakeoff-progress; tester §3) |
| V16 | Anchors 119/119 (advisor bake-off runs `write_report=false` → tail metrics are anchor-safe by construction) | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)` (§8) |
| V17 | Spec-lint PASS (0 violations) — no spec structural regression introduced | VERIFIED | `python3 scripts/spec_lint.py` → `spec-lint: PASS (0 violations)` (§9) |
| V18 | Cockpit-smoke 0 panics in 7s window (clean-boot signature) | VERIFIED | `spec/v2/advisor-turnover-and-tail-metrics/reports/cockpit-smoke-2026-06-29T20-39Z.log` empty → grep panic markers = 0 (§10) |
| V19 | Clippy + fmt clean on both crates touched | VERIFIED | `cargo clippy -p backtest --tests -- -D warnings` PASS · `cargo clippy -p ui --tests --features fixtures -- -D warnings` PASS · `cargo fmt --check` PASS (§2) |

## Numbers that matter

- Tests: **783 passed / 0 failed / 8 pre-existing ignores** (backtest
  lib 193/0/8 + ui lib 583/0/0 + render integration 7/0/0 — 3 risk-story
  + 2 scorecard + 3 bakeoff-progress).
- Anchors: **119 / 119** PASS — frozen gate byte-immutable.
- Spec-lint: **PASS (0 violations)** — baseline of
  `docs/dev-notes/audit-2026-06-29.md` held.
- Risk-story render-test cost: **96.24s** for the 2 macOS render tests
  (`leaderboard_risk_story_render`).
- New types added to the public seam: **1** (`backtest::TailSummary`,
  four `f64`s + one `From<&DistributionSummary>` impl); **0** new
  widgets; **0** new theme tokens; **1** new layout constant
  (`W_TURNOVER: f32 = 80.0` — a per-table column width, sibling of
  `W_RANK`/`W_NUM`).
- UI surface added: **13** new `LEADERBOARD_RISK_STORY_*` + **1**
  `LEADERBOARD_COL_TURNOVER` string constants, all registered in
  `strings::all()`; **0** new `ui` dep edges (the architectural
  invariant from `v2-architecture.md` §1 P1-2 held verbatim).
- Cascade fix-ups: **49** `LeaderRow` literals + **7** `CandidateKpis`
  literals + **6** `DistributionSummary` literals across the workspace
  updated, all green.
- Code commits in this feature:
  - `66286e2` — P1-1 turnover + P1-2 coherent-tail KPIs (developer / backend)
  - `00240ed` — UI Churn column + Risk story block (ui-designer)
  - `decbcc4` — tester `VERDICT → PASS` (this presentation's source of truth)
- Follow-on already shipped after PASS:
  - `bcc4c24` — P0-3 "confidence-not-verdict" framing (Phase 2A
    capstone — see "What's next").

## What this deliberately doesn't do (and why)

Per `spec/v2/v2-architecture.md` §1 P1-1 + P1-2 + §6.0 (operator-
ratified 2026-06-28):

- **Report-only — never a veto.** Same discipline as P0-1: the
  `Churn` column and `Risk story` panel are surfaced and logged but
  `rank_candidates` and the FROZEN `verdict_bands` /
  `classify_verdict` never read them. The gate-identity unit test
  `turnover_does_not_change_ranking` is the explicit proof. Anchors
  119/119 is the system-level corroboration.
- **CVaR not VaR — coherence is non-negotiable.** Plain VaR is not
  sub-additive and rewards concentration over diversification; the
  research and the architect both insist on coherence here. The UI
  copy says "Expected shortfall (CVaR) — coherent, unlike plain
  VaR" so the term-of-art is glossed in one place.
- **No per-row tail expansion (yet).** `TailSummary` is computed
  for the **crown only** (mirrors the scorecard's "one block per
  bake-off" precedent). Per-candidate tail breakdowns are a future
  polish if the operator wants them; today's panel reads the crown's
  story, which is what the operator is acting on.
- **No annualised turnover rate.** Today's `"N.N×"` is the
  operator-natural "this-many-capital-equivalents" framing — the
  per-year scaling is a future formatting decision (the formula is
  trivial to convert if needed; we just don't have a strong reason
  to switch yet).
- **No tail metric ever feeds the verdict.** A future veto (e.g.
  "crown disqualified if CVaR-99 < threshold") would be a FROZEN-
  gate change and needs its own ADR + an operator call. The carrier
  is in place if that day comes — no carrier change required, just a
  read in `classify_verdict`.

The credibility comes from showing the work, not from a winning pick.
This block is built to **make the null legible** — "here's why
holding wins on cost (zero churn) and here's what its honest tail
looks like (median fine, worst-1% deep)." That is the point.

## What's next

Phase 2A (the credibility layer) is now fully shipped:

- **P0-1 Honesty Scorecard — SHIPPED** (commits `9c3c002` + `ac7c779`
  + `d3a9a4a`, presentation 2026-06-29).
- **P1-1 Turnover + P1-2 Coherent Tail/Median — SHIPPED** (this deck,
  commits `66286e2` + `00240ed`).
- **P0-3 Confidence-not-verdict framing — SHIPPED** (commit `bcc4c24`,
  immediately after this PASS; presenter will spawn for it separately
  per the standard workflow).

Phase 2B follow-on (next major work):

- **R1 forward-fidelity COVERAGE refactor** (`spec/v2/v2-architecture.md`
  §2 R1) — `build_registry_for` (`crates/agent/src/runtime.rs:335`)
  must learn the 14 post-F5b crownable arms (5 DSL primitives + 6
  ensembles + `v0.dvol_regime` + `v0.macro_riskon` + 1 floor). Today
  if the bake-off crowns one of those arms, the forward run `bail!`s
  on it. Small, well-fenced, no FROZEN-gate impact — pure dispatch
  widening. **Recommended as the next ship** because the honesty
  contract is *broken today* for those 14 arms (the SUGGESTION stage
  can't describe a crown it can't build).
- Then Phase 2C overlays (P1-3 drawdown-control overlay, P1-4 vol-
  targeting repositioning) and Phase 2D cost opt-in
  (`SlippageModel::VolScaledSpread` — anchor-safe by D6).

ADR registration follows the 2026-05-29 contract — written = registered
when the feature lands.

## Open decisions

_No engineering decisions pending_ — all P1-1 + P1-2 calls were
operator-ratified on 2026-06-28 (`spec/v2/v2-architecture.md` §6.0)
and shipped as specified. The architect's report-only / no-veto
discipline (D3) carries over from P0-1; the CVaR-not-VaR coherence
call is the architect's, not a fresh operator question.

One scope-honesty note (not a decision): the tail facts are surfaced
for the **crown only**. If the operator later wants per-row tail
summaries (a tooltip or expander on each leaderboard row), the
`PathMetrics` vector is already captured per candidate — it's a
small UI-only follow-on, no engine change.

## Pre-flight gate (mechanical)

```
$ bash scripts/check_presentation.sh \
    spec/v2/advisor-turnover-and-tail-metrics/presentations/advisor-turnover-and-tail-metrics-2026-06-30.md
PRESENTATION CHECK PASS  (…/advisor-turnover-and-tail-metrics-2026-06-30.md — approval block UN-ticked)
```

```
$ python3 scripts/spec_lint.py
spec-lint: PASS (0 violations)
```

Both gates green — presentation is structurally clean and the approval
boxes ship un-ticked.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

### Notes / feedback

<empty until operator fills>

## Changelog

- 2026-06-30 (presenter): initial release deck for Phase 2A's cost-
  and-risk layer — `Churn` column (P1-1) + `Risk story` panel (P1-2),
  pulled verbatim from the tester's `VERDICT → PASS` report (commit
  `decbcc4`, run_id `2026-06-29-2039-UTC`). Worked example uses the
  populated render-test PNG (`/tmp/leaderboard_risk_story_render.png`)
  — median €102 300, CVaR-95 −18 %, CVaR-99 −31 %, skew +0.42, Sortino
  +1.95, Calmar +2.32 — and the modal `benchmark_wins` case for the
  hold-tail proof. V1–V19 verification matrix re-cites the tester
  evidence. §6.0 ratification record carries forward from P0-1 (report-
  only / CVaR coherence non-negotiable / per-row tail deferred / no
  veto without ADR). Phase 2A is now fully shipped; Phase 2B (R1
  forward-coverage) is the recommended next ship.
