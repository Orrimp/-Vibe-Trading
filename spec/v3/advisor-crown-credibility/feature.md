---
slug: advisor-crown-credibility
status: shipped
owner: operator
updated: 2026-07-10
version: 3.2.0
---

# advisor-crown-credibility — the crown co-presents its overfitting verdict (P1)

> **One-line:** the recommendation banner must CO-PRESENT the credibility verdict.
> A crowned **active** pick that fails the deflated-Sharpe (DSR) check renders an
> unmissable "did not survive the overfitting check — treat as weak evidence" state
> **on the crown banner itself**, not only in the scorecard panel below. This is an
> **additive presentation-layer** change. **No gate change** — `rank.rs` still does
> NOT read `crown_clears_dsr`; the FROZEN gate + scorecard math are byte-untouched.

This is the single build of Remediation-plan **P1** (`spec/backlog.md` §
Remediation plan, ratified 2026-07-09, **D1=(a) presentation-layer**). It is a
**UI / presentation** feature owned by the **ui-designer**: no engine code, no gate
touch, no new anchors, no new field on any `crates/backtest` type.

## Why (the gap this closes)

The FROZEN robustness gate crowns pure noise **~1 in 5 seeds** (P2-2 empirical,
`crates/backtest/tests/null_data_no_crown.rs`; independently re-observed by the
tester on fresh seeds — GBM 1/5, GARCH 1/5 ActiveWins on true-null series). The DSR
scorecard **catches every one** (`crown_clears_dsr == false`, deflated-Sharpe
≈ 0.40–0.78 < 0.95 on those chance-crowns) — that two-layer property is exactly why
the scorecard is load-bearing (`spec/dev-notes/dsr-report-only-decision-2026-07-09.md`
§ empirical basis).

**But the presentation undercuts the protection.** The crown ("`v0.sma` is the best
risk-adjusted pick.") is the recommendation's visual centrepiece — an `H2` headline
at the top of the Recommendation panel. The credibility verdict sits **in a separate
scorecard panel BELOW the ranked table** (`scorecard_block`, rendered under the
table in `ready_pane`). A user who reads the crown and stops — the natural reading
path — is misled at a **measurable rate**: the banner asserts "best pick" with no
co-located signal that the pick failed the honesty check.

**The fix (D1=(a)):** the crown banner itself carries the credibility state. When a
crowned **active** pick fails DSR, the banner renders an unmissable, plain-language,
non-alarmist weak-evidence treatment — co-located with the "best pick" claim so the
two are read together. When it clears DSR, a small positive affordance. This closes
the "trust the crown alone → misled" gap **without touching the gate**.

## Non-goals (explicit — do NOT build these)

- **No crown-eligibility veto. No gate change.** `rank.rs` / `robustness.rs` /
  `scorecard.rs` stay **byte-untouched**. This feature reads the EXISTING
  `crown_clears_dsr` (informational) field and presents it; it never makes it a
  veto. The veto is a settled dead-end — `spec/dev-notes/do-not-build-register.md`
  row **E-1** + `spec/dev-notes/dsr-report-only-decision-2026-07-09.md` (D3
  report-only, re-confirmed 3×). Wiring the veto is NOT this feature's to do.
- **No new field on any `crates/backtest` type.** `Scorecard` /
  `ScorecardView` are unchanged. The credibility state is a **derived value**
  computed in the `ui` layer from fields already present.
- **No new `DSR_THRESHOLD` / no threshold change.** The 0.95 bar lives in
  `scorecard.rs` and already drives `crown_clears_dsr`; the UI presents the boolean,
  it does not re-derive or re-tune the bar.
- **No change to the scorecard panel** beyond what pairs cleanly with the banner
  state (the panel stays; it is the "show your work" detail the banner summarises).
  The existing `scorecard_block` render + its `leaderboard_scorecard_render.rs` guard
  are undisturbed.
- **No badge on a non-`ActiveWins` recommendation** (see Design § BenchmarkWins).

## Verified ground truth (read the code before designing)

Grounded in the current tree (2026-07-09), not spec prose:

- **`crown_clears_dsr` is informational.** `crates/backtest/src/bakeoff/scorecard.rs`
  — `Scorecard.crown_clears_dsr: bool = (deflated_sharpe >= DSR_THRESHOLD)`,
  `DSR_THRESHOLD = 0.95` (line 63). Module + field doc-comments state **"REPORT-ONLY
  … Informational, never a veto."** `rank.rs` does not read it (proven by the
  `scorecard_does_not_change_ranking` unit test, scorecard.rs:944).
- **The mirror boundary is `ScorecardView::from_scorecard`.** `crates/ui/src/leaderboard/state.rs:405`
  — a pure, total `Option<Self>` mirror that carries `crown_clears_dsr` (and
  `deflated_sharpe`, `n_candidates`, `n_eff`, `min_btl_years`) across the seam as
  plain `bool`/`f64`/`usize`. This is the **single** place `Scorecard` becomes a
  `ui` type (`from_report` in the same file assembles the `BakeoffReportMirror`).
- **The banner is `recommendation_block`.** `crates/ui/src/screens/leaderboard.rs:556`
  — renders `headline_copy(report)` (`H2`, `FG_1`) + an optional
  `winner_robustness_clause` (`BODY`) + the F9 `narration_section`, wrapped in
  `frame::panel(LEADERBOARD_RECOMMENDATION_TITLE, …)`. The headline is driven by
  `OutcomeKind` (`headline_copy`, :718). This is where the credibility state renders.
- **`OutcomeKind` (the crown-semantics discriminator).** `crates/ui/src/leaderboard/state.rs:98`
  — `ActiveWins | BenchmarkWins | AllFragile`, mirrored 1:1 from
  `backtest::RecommendationOutcome` (:600). Crown semantics (grounded in
  `crates/backtest/src/bakeoff/rank.rs`):
  - **`ActiveWins`** — an active (non-benchmark) arm is `order[0]` (crowned). This is
    the **only** outcome where "does the crowned pick survive the overfitting check?"
    is a meaningful question about the pick.
  - **`BenchmarkWins`** — buy-and-hold is `order[0]`. The active arms lost; the
    scorecard's DSR is computed on the **max-Sharpe arm (an active loser)**, not on
    the crowned buy-and-hold. Buy-and-hold is **exempt from the gate** (ADR-0066 §
    D1 — the benchmark is the baseline, not a candidate).
  - **`AllFragile`** — no crownable arm at all (every active arm fragile AND the
    benchmark isn't best). The banner already reads as a null verdict ("No active
    strategy cleared the robustness bar").
- **`ready_pane` composition order.** `crates/ui/src/screens/leaderboard.rs:443` —
  `Column[ data_quality, recommendation, table, (scorecard?), (risk_story?),
  (short_field?), disclaimer ]`. The banner is `recommendation` (2nd); the scorecard
  panel is far below the table. This distance is the gap.
- **Existing render harness precedent.** `crates/ui/tests/leaderboard_scorecard_render.rs`
  (`#![cfg(target_os = "macos")]`, populated + negative-control, hue/foreground pixel
  counts, PNG to `/tmp/`) is the exact pattern the credibility render proof follows
  (ADR-0057 macOS gate). The money-shot fixture already exists:
  `fixtures::fake_bakeoff_report_mirror_five_arm` is `OutcomeKind::ActiveWins`
  (`winner: v0.sma` crowned) with `scorecard.crown_clears_dsr == false` — a crowned
  active pick that fails the check. `fake_bakeoff_report_mirror_benchmark_wins` gives
  the `BenchmarkWins` case.
- **Theme tokens exist; no new token needed.** `crates/ui/src/theme.rs` —
  `WARN_50` (soft warn tint backdrop, dual-mode :320), `WARN_500` (deeper warn,
  dual-mode :336), `ACCENT`/`ACCENT_SOFT`, `FG_1/2/3`, `BORDER_1`, `space::*`,
  `radius::*`, `text::*`. All dual-mode via `ModeColor::current(mode)`.
- **Strings discipline.** `crates/ui/src/strings.rs` — every literal is a `pub const`
  registered in `strings::all()`; the scorecard already models this
  (`LEADERBOARD_SCORECARD_BEATS_HOLD_YES/NO`, glyph-carrying ✓/✗). New copy follows
  the same pattern (CLAUDE.md UI rule: zero inline literals/hex).

## Design

### Decision 1 — the credibility state resolves from a pure function; NO new state field (ADR — see § ADR)

A small pure function on the values already at the banner:

```
fn crown_credibility(outcome: OutcomeKind, scorecard: Option<&ScorecardView>) -> CrownCredibility
```

where

```
enum CrownCredibility {
    /// ActiveWins crown that CLEARS DSR — small "passes the overfitting check" affordance.
    Passes,
    /// ActiveWins crown that FAILS DSR — the unmissable weak-evidence state (the money shot).
    WeakEvidence,
    /// Not meaningful on the banner — BenchmarkWins / AllFragile / no scorecard.
    /// Banner renders as today (no credibility affordance).
    NotApplicable,
}
```

Resolution (the ONLY logic):

| `outcome`       | `scorecard` | `crown_clears_dsr` | → `CrownCredibility` |
|-----------------|-------------|--------------------|----------------------|
| `ActiveWins`    | `Some`      | `true`             | **`Passes`**         |
| `ActiveWins`    | `Some`      | `false`            | **`WeakEvidence`**   |
| `ActiveWins`    | `None`      | —                  | `NotApplicable`      |
| `BenchmarkWins` | any         | —                  | `NotApplicable`      |
| `AllFragile`    | any         | —                  | `NotApplicable`      |

- **Home:** a pure `fn` co-located with `recommendation_block` in
  `crates/ui/src/screens/leaderboard.rs` (it consumes only render-time inputs), OR
  as an associated `fn` on the `ScorecardView` mirror in `leaderboard/state.rs` (the
  same discipline as `ScorecardView::from_scorecard`). **ui-designer's call at build
  time** — the constraint is that it is **pure + total + unit-tested per row of the
  table above**, mirroring the ADR-0083 `stage_for` seam.
- **Why no new state field:** the two inputs — `report.recommendation.outcome`
  (`OutcomeKind`) and `report.scorecard` (`Option<ScorecardView>`) — are **both
  already on the `BakeoffReportMirror`** at the banner. The verdict is a pure
  projection of them; a stored field would duplicate derivable state and risk drift.
  This mirrors ADR-0083 D2 (the DATA/ANALYZE discriminator reads the existing
  `PanelState`, no new field). `CrownCredibility` is a transient `view`-time enum,
  not persisted state.

### Decision 2 — the three banner states: copy + visual treatment (ADR)

The state renders **inside `recommendation_block`, directly under the `H2`
headline** (and above/beside the existing `winner_robustness_clause` — see layout
note), so it is co-located with the "best pick" claim. All copy is new
`crate::strings` constants; all colour is existing `crate::theme` tokens.

**(i) `Passes` — crowned active pick clears DSR** (small positive affordance):

- A single quiet line under the headline: a leading `✓` glyph + accent text.
- Copy (`LEADERBOARD_CROWN_PASSES_DSR`, exact wording — plain, non-triumphant):
  > "✓ Passed the overfitting check (deflated-Sharpe above the bar)."
- Colour: `ACCENT` foreground (a reassurance, not a celebration — matches the
  scorecard's muted treatment). No panel tint. The `✓` glyph carries the signal
  beyond colour (accessibility).
- Rationale: a crowned active pick clearing DSR is genuinely rarer/stronger, so a
  small honest positive is warranted — but muted, so it never reads as a hype badge.

**(ii) `WeakEvidence` — crowned active pick FAILS DSR** (the unmissable state — the money shot):

- A **tinted, bordered inline banner-note** wrapping a `WARN`-tier line, placed
  directly under the headline so it is impossible to read the crown without it. A
  leading `⚠` glyph + `WARN_500` text on a `WARN_50` soft-tint fill with a
  `WARN_500` 1px border + `radius::R3` (the exact treatment vocabulary the
  short-field unbounded-loss note + `WARN_50` backdrop token already establish —
  see `short_field_block` and `WARN_50` doc). It is a **sibling to the headline**,
  not a footnote.
- Copy (`LEADERBOARD_CROWN_WEAK_EVIDENCE`, exact wording — honest, plain-language,
  **non-alarmist**, no jargon undefined):
  > "⚠ This pick did not survive the overfitting check — treat it as weak evidence.
  > With this many strategies tried, an edge this size can appear by chance."
- Optional muted second line (`LEADERBOARD_CROWN_WEAK_EVIDENCE_HINT`, `SMALL`,
  `FG_3`) pointing to the detail without repeating it:
  > "See ‘How much to trust this’ below for the deflated-confidence figure."
- Colour/treatment rationale: this is a **real caution about evidence quality**, so
  it earns the `WARN` tier (paired with the word "weak evidence" + the `⚠` glyph, so
  colour is never the only signal). It is deliberately NOT `NEG_*`/error-red — the
  pick is not "broken" or "wrong", it is *weakly evidenced*; the honest register is
  caution, not alarm. The copy explicitly names the **mechanism** (many strategies
  tried → chance edge) so it educates rather than scolds.
- **Crucially additive, not contradictory:** the headline ("`v0.sma` is the best
  risk-adjusted pick.") stays — it is TRUE (it *is* the best of the field). The
  banner-note qualifies *how much that means*. The two read together as "here's the
  best of what we tried, and here's the honest caveat" — the product's core "measured
  honesty, not asserted alpha" made literal on the centrepiece.

**(iii) `NotApplicable` — BenchmarkWins / AllFragile / no scorecard** (no affordance):

- The banner renders **exactly as today** — no credibility line, no tint.
- **BenchmarkWins decision (grounded, NOT a misleading badge):** buy-and-hold is the
  crown, and it is **exempt from the gate** (ADR-0066 § D1 — the benchmark is the
  baseline these are measured against, not a candidate). The scorecard's
  `deflated_sharpe` is computed on the **max-Sharpe active arm (a loser)**, not on
  buy-and-hold — so a "fails the overfitting check" badge on a *hold* recommendation
  would attach an active-arm statistic to a passive pick: **actively misleading**.
  The existing `BenchmarkWins` headline already reads honestly ("No active strategy
  cleared the robustness bar … holding is the least-bad choice"), and the scorecard
  panel's "Beats holding after the search? → Not clearly — holding is the honest
  call" is the correct, in-context readout. So the banner carries **no** DSR badge in
  the `BenchmarkWins` case. Same reasoning retires `AllFragile` (a null verdict on a
  fragile active field — no crowned pick whose evidence-strength to caveat, and DSR
  is on a fragile active arm).
- `ActiveWins` with `scorecard == None` (gate not run / degenerate field) →
  `NotApplicable` too: no credibility figure exists to present, so the banner stays
  as-is rather than asserting a check that wasn't computed.

### Decision 3 — dual-mode + accessibility (ADR)

- **Dual-mode:** every colour resolves via `ModeColor::current(mode)` — the note
  renders correctly under `--theme dark` and `--theme light` (all tokens
  `WARN_50`/`WARN_500`/`ACCENT`/`FG_3` are dual-mode). No hardcoded hex.
- **Colour is never the only signal** (the ADR-0083 dot-marker precedent, CLAUDE.md
  contrast minimum): the `WeakEvidence` state carries a leading `⚠` glyph + the
  literal words "weak evidence"; the `Passes` state carries a `✓` glyph + "Passed".
  A colour-blind operator reads the state from the glyph + text alone.
- **Zero literals/hex:** the four/five copy strings are new `crate::strings`
  constants registered in `strings::all()`; colours are `crate::theme` tokens; **no
  new theme token, no new dependency** (`cargo tree -p ui` unchanged).

### Decision 4 — layout placement inside `recommendation_block`

Insert the credibility element **immediately after the `H2` headline push**, before
the existing `winner_robustness_clause` push, in `recommendation_block`'s `Column`:

```
Column::new().spacing(space::S)
    .push(headline)                        // existing H2 "…best risk-adjusted pick."
    .push(crown_credibility_element(...))  // NEW — the state (i)/(ii), or a 0-size Space for (iii)
    .push(<winner_robustness_clause>?)     // existing
    .push(narration_section(...))          // existing
```

- `NotApplicable` returns a **zero-size `Space`** (the established "render nothing"
  idiom — see `templated_reasons` empty case) so the byte-for-byte pre-feature layout
  is preserved for BenchmarkWins/AllFragile/no-scorecard.
- The `WeakEvidence` note is `width(Length::Fill)` so it spans the panel — an
  unmissable band, not a thin inline clause. `Passes` is a single quiet line.

> **Alternative rejected — only bolder scorecard styling, banner untouched.**
> Rejected: it leaves the money shot (the credibility verdict) below the fold on the
> natural reading path (crown at top, scorecard panel below the table). The whole P1
> finding is "co-present on the crown itself" (D1=(a)); a prettier panel does not
> satisfy it. (Recorded in the ADR § Alternatives.)

> **Alternative rejected — a red/error treatment on the WeakEvidence state.**
> Rejected: `NEG_*`/error-red overstates it — the pick is not broken, it is weakly
> evidenced (the honest modal crypto reality). `WARN` tier + plain "weak evidence"
> wording is the calibrated, non-alarmist register the product's tone demands.

### Anchor + gate safety

- **Anchors 119/119 by construction.** UI-only. The advisor bake-off path runs
  `write_report=false` (scorecard is carried on `Recommendation`, a
  backtest-internal type not on the anchored CLI report path — scorecard.rs § Anchor
  safety). No anchored CLI path reads any `ui` credibility state. Zero anchors added.
  Run `bash scripts/verify_anchors.sh` BEFORE and AFTER → **119/119** both.
- **FROZEN gate byte-untouched.** No edit to `crates/backtest/src/bakeoff/{robustness,rank}.rs`
  or `scorecard.rs`. This feature reads the existing `crown_clears_dsr` and presents
  it; it changes no verdict, adds no field, moves no threshold. The
  `scorecard_does_not_change_ranking` invariant is undisturbed.
- **Existing scorecard render guard undisturbed.** `leaderboard_scorecard_render.rs`
  keeps passing (the scorecard panel is unchanged).

## Acceptance criteria

- A crowned **active** pick that fails DSR (`ActiveWins` + `crown_clears_dsr==false`)
  renders the unmissable `WeakEvidence` state **on the recommendation banner**, with
  plain-language, non-alarmist copy naming the pick as weak evidence + the mechanism.
- A crowned active pick that clears DSR (`ActiveWins` + `crown_clears_dsr==true`)
  renders the muted `Passes` affordance on the banner.
- `BenchmarkWins` and `AllFragile` render the banner **as today** — NO credibility
  badge (grounded in the ADR-0066 benchmark exemption + the DSR-on-active-arm fact).
- The state is a **pure function** of `OutcomeKind` + `Option<&ScorecardView>` with
  **no new stored state field** and **no `crates/backtest` change**.
- Both themes render correctly; colour is never the only signal (`⚠`/`✓` glyph +
  words); zero inline literals/hex; `cargo tree -p ui` unchanged (no new dep).
- **Render-layer proof** (populated money shot + negative controls, macOS-gated) is
  green and the PNGs visibly show the `WeakEvidence` band on the banner (see § UI).
- `bash scripts/verify_anchors.sh` → **119/119** before AND after;
  `python3 scripts/spec_lint.py` → **PASS(0)**.
- `cargo clippy -p ui --tests -- -D warnings` clean; `cargo fmt --check` clean.
- The ADR is accepted + registered **atomically** in `spec/architecture/adr/README.md`.

## Risks

- **Tone drift toward alarm.** The `WeakEvidence` copy must stay non-alarmist — it
  is a caveat, not a "this is broken" scream. `WARN` tier (not error-red) + the exact
  wording above is the guardrail; if the rendered band reads too loud, dial the fill
  to a thinner border-only treatment at render review (a `view`-time call, no ADR
  amendment).
- **Contradiction risk.** The banner-note must READ as additive to the headline
  ("best pick, but weak evidence"), not as a contradiction ("best pick / not the
  pick"). The copy is written to qualify, not negate; verify at the render walk that
  the two lines cohere.
- **BenchmarkWins over-badging.** The single most likely wrong move is slapping a
  "fails overfitting" badge on a hold recommendation. The `NotApplicable` branch +
  its grounding (ADR-0066 exemption, DSR-on-active-arm) is the guard; the render test
  asserts the `BenchmarkWins` banner has NO credibility band.
- **Font-mutex / cosmic-text** render-test flake (the known `param_sweep_render`
  caution) — follow the existing macOS-gated harness pattern + coarse hue thresholds.
- **Scope creep toward the veto.** If a task starts making `rank.rs` read
  `crown_clears_dsr`, STOP — that is the do-not-build register E-1 dead-end. This is
  presentation-only.

## Trace

`REQ-V3-P1-CROWN-CREDIBILITY-001` in [`spec/trace.toml`](../../trace.toml), state
`arch-done` (design-complete; NOT shipped — honoring ADR-0082, feature.md status is
the single source of truth). Arch refs: `spec/backlog.md` § Remediation plan P1,
this feature, `spec/architecture/adr/0085-crown-credibility-co-presentation.md`,
`spec/dev-notes/dsr-report-only-decision-2026-07-09.md` (the report-only decision
this feature honours).

## UI

### Wireframe — the `WeakEvidence` state on the banner (the money shot)

```
┌ Recommendation ─────────────────────────────────────────────────────────────┐
│  v0.sma is the best risk-adjusted pick.                          ← H2 headline│
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ ⚠ This pick did not survive the overfitting check — treat it as weak   │  │  ← WeakEvidence
│  │   evidence. With this many strategies tried, an edge this size can     │  │    band (WARN_50
│  │   appear by chance.                                                    │  │    fill, WARN_500
│  └───────────────────────────────────────────────────────────────────────┘  │    border+text)
│  See ‘How much to trust this’ below for the deflated-confidence figure.       │  ← muted FG_3 hint
│  · Highest Sharpe among the strategies that held up under resampling.         │  ← existing reasons
│  · Beat buy-and-hold on risk-adjusted return.                    [Explain]    │
└──────────────────────────────────────────────────────────────────────────────┘
```

`Passes` state (crowned active clears DSR) replaces the band with a single quiet
line: `✓ Passed the overfitting check (deflated-Sharpe above the bar).` (`ACCENT`).
`BenchmarkWins` / `AllFragile`: the band region is a zero-size `Space` — the banner
is byte-identical to pre-feature.

### The rendered states (all pixel-verified — read the PNGs)

| fixture / state                                         | banner region          | PNG                                      |
|---------------------------------------------------------|------------------------|------------------------------------------|
| `five_arm` (`ActiveWins`, `crown_clears_dsr=false`)     | **⚠ WeakEvidence band**| `/tmp/crown_credibility_weak.png`        |
| `five_arm` mutated to `crown_clears_dsr=true`           | ✓ Passes line          | `/tmp/crown_credibility_passes.png`      |
| `benchmark_wins` (`BenchmarkWins`)                      | (no credibility band)  | `/tmp/crown_credibility_benchmark.png`   |

**Band-intensity call (render review, 2026-07-10 — the pixel-layer judgment the
ADR § Risks flagged): FILLED, not border-only.** `WARN_50` soft-tint fill +
`WARN_500` 1px border + `WARN_500` text + `radius::R3`, `width(Fill)`. Verified at
`/tmp/crown_credibility_weak.png`: the 0.12-alpha amber tint over the near-black
panel is subtle (does not read as "broken"/error-red) while the border + amber text
make the band the unmissable "sibling to the headline" § D3 (ii) requires. No ADR
amendment (a `view`-time call, as the ADR authorised).

### New widget / function

- **`crown_credibility(outcome, Option<&ScorecardView>) -> CrownCredibility`** (pure)
  + **`crown_credibility_element(CrownCredibility, mode) -> Element`** (the view),
  co-located with `recommendation_block` in `crates/ui/src/screens/leaderboard.rs`
  (or on `ScorecardView` in `leaderboard/state.rs` — ui-designer's call; pure +
  unit-tested either way). `CrownCredibility` is a transient `view`-time enum.
- **`recommendation_block`** gains one `.push(crown_credibility_element(...))` between
  the headline and the robustness clause.

### New strings (`ui::strings`, registered in `strings::all()`)

`LEADERBOARD_CROWN_PASSES_DSR`, `LEADERBOARD_CROWN_WEAK_EVIDENCE`,
`LEADERBOARD_CROWN_WEAK_EVIDENCE_HINT` (+ any glyph-bearing sub-constants the
ui-designer factors out). Exact copy per Design § Decision 2.

### New theme tokens

**Zero.** Composes existing tokens only (`WARN_50`, `WARN_500`, `ACCENT`, `FG_1`,
`FG_3`, `BORDER_1`, `space::{XS,S,M}`, `radius::R3`, `text::{BODY,SMALL,MICRO}`). No
new dependency (`cargo tree -p ui` unchanged).

### Accessibility notes

- **Colour is never the only signal:** `⚠` (weak) / `✓` (passes) glyphs + the
  literal words "weak evidence" / "Passed" carry the state without hue.
- **Both themes:** all tokens dual-mode via `ModeColor::current(mode)`; the
  `WeakEvidence` band is legible in `--theme dark` and `--theme light`.
- **No new focus stop:** the credibility element is display-only (no interaction);
  it introduces no keyboard-nav change.

## Changelog

- 2026-07-09 (architect): design pass complete (`status: arch-done`). Grounded P1 in
  the real `crates/ui/` seams (the `ScorecardView::from_scorecard` mirror boundary,
  `recommendation_block` banner, `OutcomeKind` crown semantics, `ready_pane` order,
  the `leaderboard_scorecard_render.rs` harness precedent, the `five_arm` /
  `benchmark_wins` fixtures, the `WARN_50`/`WARN_500` dual-mode tokens). Decided the
  three banner states (Passes / WeakEvidence / NotApplicable), their exact
  non-alarmist copy + `WARN`-tier treatment, and the pure `crown_credibility(...)`
  seam with NO new state field (mirrors ADR-0083 `stage_for`). Grounded the
  BenchmarkWins decision in the ADR-0066 benchmark exemption + the DSR-on-active-arm
  fact → no badge on a hold pick. Recorded the design in ADR-0085 (took 0085 — the
  sibling P2 corpus-expansion ADR had already claimed 0084 when this landed). FROZEN gate +
  scorecard math byte-untouched; the veto stays unbuilt (do-not-build E-1). Handoff
  to ui-designer.
- 2026-07-10 (ui-designer): BUILT (`status: dev-done`). T1–T8 landed exactly per
  ADR-0085 — no redesign. **Resolver home:** an inline pure `fn crown_credibility(
  outcome, Option<&ScorecardView>) -> CrownCredibility` co-located with
  `recommendation_block` in `crates/ui/src/screens/leaderboard.rs` (chosen over the
  `state.rs` assoc-fn option — it consumes only render-time inputs and keeps the
  transient `view`-time enum where it renders; mirrors the ADR-0083 `stage_for`
  screen-local seam). `crown_credibility_element(state, mode)` renders the three
  states; wired with one `.push` between the H2 headline and the robustness clause.
  Three copy strings added + registered in `strings::all()`; **zero** new theme
  token; **zero** new dependency (`cargo tree -p ui` unchanged). **Band-intensity
  call (the pixel-layer judgment the ADR flagged): FILLED, not border-only** —
  `WARN_50` soft-tint fill + `WARN_500` 1px border + `WARN_500` text + `radius::R3`,
  `width(Fill)`. At the render walk (`/tmp/crown_credibility_weak.png`) the filled
  band reads as a calibrated CAUTION (the 0.12-alpha amber tint over the near-black
  panel is subtle — it does not scream "broken"), while the border + amber text make
  it the unmissable "sibling to the headline" the ADR § D3 (ii) demands; border-only
  under-weighted the "impossible to read the crown without it" requirement. **Render
  proof (T7, the closing gate) GREEN + all three PNGs eyeballed:** the WeakEvidence
  band paints on the banner (money shot, `five_arm`; ~6.1k WARN-amber px in the
  banner region vs ~26 for the `Passes` control — delta 6060); `crown_clears_dsr→true`
  shows the muted teal `✓ Passes` line and NO band (the flag-tracks-the-render
  control); `BenchmarkWins` shows NO credibility band (the no-badge-on-a-hold-pick
  invariant). **T7 caught a test-region bug, not a feature bug** — the initial guard
  counted amber in the top third, but with the full 20-arm field the banner lands in
  the LOWER half (the guided-input form + Data-quality panel push it down; band at
  y≈890); the fix restricts the classifier to `y > h/2` (the "read the PNG, not the
  count" lesson — MEMORY). 5 resolver unit tests green (one per D3/D4 row incl.
  `ActiveWins`+`None`→NA and both non-`ActiveWins`→NA). Gates: `cargo build -p ui`
  ✓; `cargo test -p ui --lib` ✓; `cargo test -p ui --test crown_credibility_render`
  ✓ (3/3, macOS); `cargo clippy -p ui --tests -- -D warnings` ✓; `cargo fmt --check`
  ✓; anchors 119/119 before AND after; `spec_lint.py` PASS(0). FROZEN gate
  `bakeoff/{robustness,rank}.rs` + `scorecard.rs` byte-untouched (no `crates/backtest`
  diff); the veto stays unbuilt (E-1). Handoff to tester.
