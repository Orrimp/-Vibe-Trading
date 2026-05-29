---
slug: ui-contrast-asserter
version: 0.1.0
status: draft
owner: analyst
priority: P2
updated: 2026-05-29
---

# UI contrast asserter — v0.1.0

> **Pick B Wave 1 promoted feature (cross-cutting safety duo).** Per
> [`spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md)
> this is the cheaper of the two duo pillars (~0.5 dev days), biased
> toward DURABLE: a `crates/ui/tests/contrast.rs` test that enumerates
> every `(fg, bg)` token pair in `crates/ui/src/theme.rs` and asserts
> WCAG 2.1 contrast ratios per
> [`spec/ui-design-principles.md ## Accessibility minimums`](../ui-design-principles.md#accessibility-minimums)
> (4.5:1 AA body, 7:1 AAA equity). Closes an entire class of palette-
> refactor regression without rendering a pixel.

## Why

Per [`process-tooling-survey-2026-05-29.md § Top-5 deep-dives Rank 4`](../dev-notes/process-tooling-survey-2026-05-29.md#-top-5-deep-dives-condensed):
the cockpit theme at
[`crates/ui/src/theme.rs`](../../crates/ui/src/theme.rs) defines
~30-60 color tokens (FG_1/2/3 ladders, CANVAS, PANEL, ACCENT, UP_500,
DOWN_500, WARN_500, etc) used across the UI in `(fg, bg)` combinations.
The
[`ui-design-principles.md ## Accessibility minimums`](../ui-design-principles.md#accessibility-minimums)
contract carries:

- **≥ 4.5:1 contrast** (WCAG 2.1 AA) for body text
- **≥ 7:1 contrast** (WCAG 2.1 AAA) for the equity display

Today, that contract is **enforced by human eyeball on the design
review**. The lumen-design-adoption master roadmap is Phase 6+ and
new tokens land every cycle. A palette refactor — even an unrelated
hex-value tweak to `FG_3` in dark mode — can silently drop a
`(FG_3, PANEL)` pair below 4.5:1 with no test catching it. The
operator notices the legibility regression at the next live cockpit
launch — days or weeks after the refactor lands.

This brief closes the gap with a **`crates/ui/tests/contrast.rs`
test that enumerates `(fg, bg)` token pairs and asserts WCAG ratios**
per the design principles contract:

1. **Data-driven enumeration** of all `(fg, bg)` pairs derived from
   `crates/ui/src/theme.rs` color tokens.
2. **WCAG 2.1 formula** computed per pair (relative luminance ratio).
3. **AA gate** (4.5:1) for body text class; **AAA gate** (7:1) for
   the equity display class.
4. **Per-token opt-out marker** with mandatory `reason: &str` for
   tokens that physically can't meet WCAG (e.g. low-priority
   annotation grey used purely for non-text decoration).
5. **Run in WARN mode for two weeks** before promoting to gate (per
   the bundle Q-DUO-WARN ratification).

Three layered consequences:

- **New tokens auto-asserted.** Once shipped, any new color token
  added to `theme.rs` inherits the contrast check WITHOUT
  per-token wiring.
- **Palette-refactor regression class CLOSED.** A future hex-value
  change to `FG_3` (or any token) triggers the test if any `(FG_3, bg)`
  pair drops below the WCAG threshold.
- **Data-driven nature** matches the survey's "best-cheap-pick"
  framing: ~0.5 dev days, ~0.25 tester day, no rendering needed.

Per process-tooling-survey: **MEDIUM per-cycle benefit, SMALL
investment (~0.5d), LOW maintenance**. Data-driven means new tokens
auto-cover; opt-out list is bounded by reason-stringed annotations.

## Requirements

### R1 — Pair enumeration from `theme.rs`

- **R1.1** A new test file `crates/ui/tests/contrast.rs` enumerates
  the `(fg, bg)` token pairs that need contrast assertion.
  Enumeration is **at compile time** (a `const`-table the test
  iterates) OR **at test runtime** (the test reads the theme's
  `pub const` values and builds the pair list). Architect M-T1
  picks the durable shape; analyst recommends compile-time table
  for review-ability.
- **R1.2** Pairs covered at v0.1.0 (architect M-T1 audits theme.rs
  and ratifies the final list):
  - All `FG_*` ladder tokens against `CANVAS` (dark + light modes)
  - All `FG_*` ladder tokens against `PANEL` (dark + light modes)
  - Equity-display token pair (per AAA gate; architect locates exact tokens)
  - `UP_500` / `DOWN_500` / `WARN_500` against `CANVAS` + `PANEL`
  - `ACCENT` fill against `text-on-accent` token (per
    [`theme.rs:189`](../../crates/ui/src/theme.rs#L189) comment)
- **R1.3** Each pair carries:
  - `pair_id: &'static str` (human-readable, e.g.
    `"fg_3_on_panel_dark"`)
  - `fg: ColorPair` (token + ThemeMode)
  - `bg: ColorPair` (token + ThemeMode)
  - `class: ContrastClass` (one of `Body` = 4.5:1, `Equity` = 7:1,
    `OptOut(reason: &'static str)`)
- **R1.4** Minimum-count assertion: the test ALSO asserts that the
  enumerated pair count is `≥ MIN_PAIRS` (architect M-T1 ratifies
  floor; analyst recommends `≥ 30`). If a future refactor changes
  the token storage shape and silently breaks enumeration, the
  pair count drops to 0 and this floor assertion FAILs with
  "theme token enumeration detected < 30 pairs; refactor likely
  broke enumeration." Defends against R4 (silent enumeration
  break).

### R2 — WCAG 2.1 contrast computation

- **R2.1** Helper function in `crates/ui/tests/contrast.rs`
  computes WCAG 2.1 relative luminance + contrast ratio per the
  [W3C spec](https://www.w3.org/WAI/GL/wiki/Relative_luminance).
  Pure function: `fn contrast_ratio(fg: Rgb, bg: Rgb) -> f64`.
  Returns ratio in range `[1.0, 21.0]`.
- **R2.2** Architect M-T1 picks the dep shape per Q-CONT-2 below:
  hand-rolled (~20 LoC) OR an existing crate like
  `wcag-contrast-ratio`. Both are pure-Rust + zero runtime cost.
- **R2.3** The test iterates pairs and asserts:
  - `class = Body` → `contrast_ratio ≥ 4.5`
  - `class = Equity` → `contrast_ratio ≥ 7.0`
  - `class = OptOut(reason)` → no assertion (logged for audit)
- **R2.4** On FAILURE in **WARN mode** (per R3): test PASSES but
  emits `eprintln!("WARN: contrast pair {pair_id} = {ratio:.2} < threshold {threshold}; reason: <none>");`
  for each failing pair. Test exits PASS so palette refactors
  during the WARN window don't block CI.
- **R2.5** On FAILURE in **gate mode** (post-v0.2.0): test FAILS
  with a panic listing each violating pair + ratio + threshold.

### R3 — WARN-mode default per Pick B Q-DUO-WARN

- **R3.1** The asserter ships with a `cfg`-feature OR env-var
  default that controls **WARN vs gate** behavior. WARN mode is
  the v0.1.0 default per
  [`pick-b-cross-cutting-safety-duo-2026-05-29.md § Q-DUO-WARN`](../dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md#-q-duo-warn--shared-warn-mode-duration-before-gate-promotion).
- **R3.2** Mode controlled by env var `UI_CONTRAST_MODE=warn|gate`
  (default = `warn` at v0.1.0). Architect M-T1 picks the durable
  shape per Q-CONT-3 below; analyst recommends env var to match
  the redactor sibling.
- **R3.3** WARN mode: failures logged to stderr via `eprintln!`;
  test exits PASS.
- **R3.4** Gate mode: failures panic the test; CI fails.
- **R3.5** v0.2.0 patch flips default to `gate` after operator
  observes WARN signal for 2 weeks.

### R4 — Per-token opt-out marker

- **R4.1** A token can be marked **opt-out** with a mandatory
  `reason: &'static str` via the `ContrastClass::OptOut(reason)`
  variant per R1.3. Examples:
  - Low-priority annotation grey on canvas (purely decorative, not text)
  - Iconography fills under accent rings (visual signal, not text)
- **R4.2** The opt-out list is **enumerated at compile time** in
  `crates/ui/tests/contrast.rs` (a `const OPT_OUTS: &[(&str, &str)]`
  table). New opt-outs require a v0.1.x patch (analyst → architect
  → developer loop) — same durable shape as the redactor's closed
  rule set.
- **R4.3** Opt-out entries are logged at test runtime (audit-of-
  exclusions): `eprintln!("opt-out: {pair_id}; reason: {reason}");`
  Architect M-T1 ratifies the initial opt-out list seeded from a
  one-pass theme.rs audit.
- **R4.4** Reason string is mandatory + reviewable: an opt-out
  entry without a reason fails compilation (the `OptOut(&str)`
  variant requires the argument).

### R-NR — Non-regression contract

- **R-NR.1** No production code changes — the test is in
  `crates/ui/tests/contrast.rs` only. No `crates/ui/src/theme.rs`
  edits at v0.1.0 (the test READS the theme's `pub const` tokens
  via the public API).
- **R-NR.2** No design token additions / removals — the test only
  ASSERTS existing tokens. Token deltas happen in unrelated
  features (lumen Phase X follow-ons).
- **R-NR.3** Existing visual snapshot tests under
  `crates/ui/tests/visual_*.rs` PASS byte-identical (no rendering
  affected).
- **R-NR.4** `bash scripts/verify_anchors.sh` → 75/75 PASS
  byte-identical pre/post. Pure test infrastructure addition.
- **R-NR.5** No new Cargo.toml runtime dep. The asserter MAY add
  one `[dev-dependencies]` crate (`wcag-contrast-ratio` per
  Q-CONT-2 fallback) OR hand-roll the formula (~20 LoC).
- **R-NR.6** WARN-mode `eprintln!` warnings are NOT committed to
  anchored reports — they live in cargo test output only.
- **R-NR.7** Zero strings.rs adds, zero UI widget code changes —
  test infrastructure only.

## Falsifiers (K)

- **K1 — Architect M-T1 theme audit finds ≥ 5 tokens that need
  opt-out at v0.1.0 ship.** The WCAG threshold is too tight for
  the existing palette; the opt-out list balloons. **Mitigation**:
  if observed, either (a) tune per-token thresholds (e.g.
  decorative text 3:1, body text 4.5:1, equity 7:1), or (b) expand
  the opt-out shape with per-class reasons. Route back to analyst
  with audit findings.
- **K2 — Token storage refactor breaks enumeration silently.**
  Future lumen Phase 7 (hypothetical) re-orgs token storage from
  `pub const FG_3: ColorPair` to `pub const FG: [ColorPair; 4]`,
  test enumerates zero pairs, passes vacuously. **Mitigation**:
  R1.4 minimum-count floor assertion (≥ 30 pairs) catches the
  break.
- **K3 — Cross-platform color rendering drift.** Test asserts on
  the theme tokens' RGB values, NOT on rendered pixels. Cross-
  platform color profile differences (macOS sRGB vs Windows
  Display P3) don't affect token RGB values, so K3 is not a
  realistic falsifier at the test layer. Logged for completeness.
- **K4 — WCAG formula bug in hand-rolled impl.** If Q-CONT-2 ratifies
  hand-rolled, a subtle bug in the relative luminance computation
  (gamma correction off-by-one) gives wrong ratios. **Mitigation**:
  unit test the formula against the W3C spec's reference vectors
  (`#FFFFFF` on `#000000` = 21:1, `#777` on `#FFF` = 4.48:1, etc).
  4-5 reference vectors at minimum.

## Hypotheses (H)

- **H1 — Test impl ≤ 100 LoC** (~30 LoC for the pair enumeration
  table + ~20 LoC for the WCAG formula + ~15 LoC for the iteration
  + ~10 LoC for the opt-out list + ~25 LoC for mode parsing +
  reference vector tests). Matches the analyst's ~0.5d estimate.
- **H2 — Architect M-T1 theme audit finds ≤ 3 tokens that need
  opt-out.** The existing palette is broadly WCAG-compliant
  (operator-tested by eyeball); the asserter formalizes the
  existing standard rather than retrofitting compliance. WARN-
  mode observation confirms.
- **H3 — Zero existing tests break.** Pure test infrastructure
  addition; no production code touched.
- **H4 — New tokens added post-v0.1.0 auto-inherit assertion**
  without per-token wiring. Verified by adding a synthetic test
  token + asserting the test enumerates it.

## Operator decisions

### Q-CONT-1 — Initial mode default: WARN per bundle ratification

**Q.** Does the asserter ship at v0.1.0 in WARN mode (per the
bundle Q-DUO-WARN ratification) OR gate-from-day-1?

**(Recommended — DURABLE) Option A — WARN mode default for 2
weeks, then v0.2.0 patch to gate.** Inherits the bundle-level
Q-DUO-WARN ratification at
[`pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md).
WARN duration is 2 weeks shared with the redactor sibling. After
operator observes WARN signal during the 2-week window, default
flips to gate via v0.2.0 patch.

**Cost.** ~0 (the env-var flag is ~5 LoC).

**Rationale (DURABLE).** Per AGENT.md 2026-05-28 + the bundle
direction: gate-from-day-1 risks blocking legitimate palette
evolution during the lumen Phase 6+ cycle. WARN observation gives
operator data to tune the opt-out list before gate flip. Cheap-
to-ship is gate-from-day-1 (no flag) but the false-positive
risk profile is worse.

**Option B (cheap fallback — REJECTED at analyst level).**
Gate-from-day-1 with no WARN mode. Saves ~5 LoC env-var parse +
the v0.2.0 patch. **Rejected** per the bundle's R2 + R5 risks
and per the AGENT.md 2026-05-28 durable framing: the cheap
path's "blocks legitimate palette evolution" risk profile is
strictly worse than the WARN observation path.

**Default**: A (Recommended DURABLE; inherited from Q-DUO-WARN).

### Q-CONT-2 — WCAG formula impl: hand-rolled vs `wcag-contrast-ratio` crate

**Q.** Does the asserter hand-roll the WCAG 2.1 formula
(~20 LoC) OR pull in the existing `wcag-contrast-ratio` crate
as a `[dev-dependencies]` entry?

**(Recommended — DURABLE) Option A — hand-rolled ~20 LoC.** The
WCAG formula is **closed math** (defined by the W3C spec; no
ambiguity or future updates expected). Hand-rolling it inline at
the test file gives ZERO dependency footprint, reviewable code at
the call site, and matches the cockpit's preference for minimal
deps. Reference vectors test (per K4 mitigation) catches any
implementation bug.

**Cost.** ~20 LoC for the relative-luminance + contrast-ratio
helper + ~10 LoC for the reference vector tests.

**Rationale (DURABLE).** Per AGENT.md 2026-05-28: dependency
sprawl is itself a maintenance burden. A 20-LoC formula at the
test file is **reviewable in one sitting** and never needs a
version bump. The `wcag-contrast-ratio` crate (Option B) adds a
`[dev-dependencies]` entry that future audits will need to vet for
maintenance + license + supply-chain hygiene.

**Option B (cheap fallback — REJECTED).** Pull in
`wcag-contrast-ratio` crate as a `[dev-dependencies]` entry.
Saves ~20 LoC of formula impl. **Rejected** — the dep-vs-LoC
tradeoff is unfavorable when the math is closed. The crate
adoption pays back if the formula were to evolve (e.g. a WCAG 3.0
APCA contrast model that does a major rework); since WCAG 2.1
math is locked, no future re-work justifies the dep.

**Default**: A (Recommended DURABLE).

### Q-CONT-3 — Per-token opt-out marker: in-file table vs theme.rs attribute

**Q.** Where do per-token opt-outs live? An `OPT_OUTS: &[(&str,
&str)]` table inside `crates/ui/tests/contrast.rs` OR an
attribute / convention on the `pub const` token declaration in
`crates/ui/src/theme.rs`?

**(Recommended — DURABLE) Option A — in-file `OPT_OUTS` table
inside `contrast.rs`.** The opt-out list lives next to the
asserter test that consumes it. Each entry is reviewable at the
test site. New opt-outs require touching ONE file (the test).
theme.rs stays clean of test-only annotations.

**Cost.** ~5 LoC per opt-out entry; ~10 LoC for the table
declaration.

**Rationale (DURABLE).** Per AGENT.md 2026-05-28: the asserter
test owns the contract; the opt-out list is the test's
configuration. Putting opt-out markers on theme.rs (Option B)
mixes test-only metadata into production code — adds noise + a
test-dependency to theme.rs that breaks if the asserter is later
removed.

**Option B (cheap fallback — REJECTED).** Per-token attribute
on `theme.rs` declarations (e.g.
`#[contrast_opt_out("reason")]` macro or a
`/// CONTRAST-OPT-OUT: reason` doc comment convention). Visible
at the token's declaration site. **Rejected** — couples
production code to a test-only concern; macro path requires a
proc-macro crate which is a significant dep escalation; doc
comment convention is fragile (silent on refactor).

**Default**: A (Recommended DURABLE).

## Verdict tree (pre-drawn)

| Q-CONT-1 \ Q-CONT-2 | Q-CONT-2=(a) hand-rolled | Q-CONT-2=(b) crate dep |
|---|---|---|
| **Q-CONT-1=(a) WARN default** | **DURABLE — Recommended.** WARN observation + hand-rolled formula + in-file opt-out table; minimal deps; reviewable in one sitting; bundle-aligned. | INCONSISTENT — durable WARN observation but unnecessary dep adoption for closed math. Operator-override only. |
| **Q-CONT-1=(b) gate-from-day-1** | INCONSISTENT — production-code-touched-on-day-1 risk; cheap dep posture but high friction risk. Bundle-misaligned. | REJECTED — worst of both: gate risk + unnecessary dep. |

Q-CONT-3 is orthogonal to Q-CONT-1 / Q-CONT-2 and overlays on
either verdict cell.

## Design

_Architect M-T1 fills this. Expected DURABLE-fast-skip path:
Q-CONT-1 (a) WARN inherited from Q-DUO-WARN bundle ratification;
Q-CONT-2 (a) hand-rolled formula ratified (no `[dev-dependencies]`
add); Q-CONT-3 (a) in-file `OPT_OUTS` table ratified. Architect
M-T1 also runs a one-pass theme.rs audit: enumerates `(fg, bg)`
pairs, computes contrast per pair via the hand-rolled formula,
seeds the initial opt-out list (≤ 3 per H2), and ratifies
`MIN_PAIRS` floor (≥ 30 per R1.4). No new ADR; ADR-0048
"boundary test" precedent carries forward for this shape;
ADR-0048 § Changelog gets one ride-along row at M-T1 close._

## Backtest Scenarios

_N/A — UI test infrastructure feature; no backtest scenarios
attach. The R-NR.4 anchor contract carries the equivalent
regression guarantee (75/75 anchors byte-identical pre/post)._

## Implementation

_Developer fills at M-DEV._

## Verification

_Tester M-FINAL links the test-final report + the WARN-mode
observation log (cargo test stderr output showing `eprintln!`
warnings, if any) + the falsification probe outcomes +
`bash scripts/verify_anchors.sh` 75/75 PASS byte-identical
pre/post + confirmation that the existing visual-snapshot tests
under `crates/ui/tests/visual_*.rs` PASS byte-identical._

## Changelog

- 2026-05-29 (analyst): M0 brief authored under Pick B Wave 1
  promotion per [`pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md).
  R1 pair enumeration + R2 WCAG compute + R3 WARN-mode + R4
  opt-out marker + R-NR (7 clauses) + K1-K4 falsifiers + H1-H4
  hypotheses + Q-CONT-1/2/3 all bias DURABLE + pre-drawn 4-cell
  verdict tree. ~0.5 dev day + ~0.25 tester day estimate. Trace
  row `REQ-UI-CONTRAST-ASSERTER-001` opened at `proposed`.
  HANDOFF → architect (M-T1 fast-skip likely if Q-CONT-1/2/3 all
  Recommended durable; one-pass theme.rs audit + opt-out list
  seed + MIN_PAIRS floor ratification expected at M-T1).
