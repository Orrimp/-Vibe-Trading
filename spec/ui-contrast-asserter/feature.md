---
slug: ui-contrast-asserter
version: 0.2.0
status: shipped
owner: analyst
priority: P2
updated: 2026-06-15
---

# UI contrast asserter — v0.2.0 (close-out: ship + gate-flip + 6-pair disposition)

> **v0.2.0 close-out brief — appended 2026-06-15 (analyst).** Scopes the
> tidy-toward-done of the shipped-but-never-presented WCAG asserter. THREE
> deliverables, low-risk and proportionate (this is a close-out, NOT a color
> redesign): (1) run the parked presenter on v0.1.0 to formally ship the
> asserter as-is (already PASSES in WARN mode); (2) flip the default
> `UI_CONTRAST_MODE` WARN→gate so the asserter ENFORCES; (3) dispose of the
> 6 known sub-AA light-mode pairs — RECOMMENDED zero-churn path is ratify all
> 6 as documented `OPT_OUTS` entries. **The v0.1.0 body below is preserved
> verbatim as the shipped baseline; v0.2.0 content is in
> [§ v0.2.0 close-out](#v020-close-out) at the end of this file.**

---

# UI contrast asserter — v0.1.0

> **Pick B Wave 1 promoted feature (cross-cutting safety duo).** Per
> [`spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/archive/2026-Q2/pick-b-cross-cutting-safety-duo-2026-05-29.md)
> this is the cheaper of the two duo pillars (~0.5 dev days), biased
> toward DURABLE: a `crates/ui/tests/contrast.rs` test that enumerates
> every `(fg, bg)` token pair in `crates/ui/src/theme.rs` and asserts
> WCAG 2.1 contrast ratios per
> [`spec/ui-design-principles.md ## Accessibility minimums`](../ui-design-principles.md#accessibility-minimums)
> (4.5:1 AA body, 7:1 AAA equity). Closes an entire class of palette-
> refactor regression without rendering a pixel.

## Why

Per [`process-tooling-survey-2026-05-29.md § Top-5 deep-dives Rank 4`](../dev-notes/archive/2026-Q2/process-tooling-survey-2026-05-29.md#-top-5-deep-dives-condensed):
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
  [`pick-b-cross-cutting-safety-duo-2026-05-29.md § Q-DUO-WARN`](../dev-notes/archive/2026-Q2/pick-b-cross-cutting-safety-duo-2026-05-29.md#-q-duo-warn--shared-warn-mode-duration-before-gate-promotion).
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
[`pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/archive/2026-Q2/pick-b-cross-cutting-safety-duo-2026-05-29.md).
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

**M-T1 architect 2026-05-29.** All three operator-decide questions
RATIFIED on the DURABLE Recommended path. K1 partially TRIGGERED at
audit (8 sub-AA pairs found — but 7 are `FG_4` placeholder/disabled
tier by design + 1 is `BORDER_STRONG` decorative; all opt-out-able with
mandatory reason strings, NOT threshold-tuning). Two GENUINE WCAG-AA
violations surfaced by the dry-run audit: `FG_ON_ACCENT` on light-mode
`ACCENT` fill at **3.52:1** + `WARN_500` on light-mode `CANVAS` at
**2.96:1**. These are NOT opt-outs — they are the exact "palette-
refactor regression class" the asserter exists to surface. WARN-mode
default (Q-CONT-1=(a)) keeps these from blocking CI during the 2-week
observation window; operator decides at v0.2.0 promotion whether to
(a) tune the offending hex values upstream, (b) downgrade the affected
pair classes, or (c) carry the WARN signal as known-debt. **H2
INVALIDATED**: opt-out count is ~9 (not ≤ 3) once `FG_4` tier is
data-driven enumerated. Opt-out list is bounded + reviewable; not a
gate-blocker. H1 / H3 / H4 hold.

### Operator-decide ratifications

#### Q-CONT-1 — WARN-mode default → RATIFIED (a) DURABLE

WARN-mode default for 2 weeks per Q-DUO-WARN bundle inheritance.
Env var `UI_CONTRAST_MODE=warn|gate`, default `warn` at v0.1.0,
flipped to `gate` at v0.2.0 patch after operator observes WARN
signal during the 2-week window. Audit dry-run found 2 GENUINE AA
violations (light-mode `FG_ON_ACCENT_on_ACCENT` 3.52 + `WARN_500
on CANVAS_light` 2.96); gate-from-day-1 would have blocked CI on
sibling features. WARN observation gives operator the data to
decide upstream hex tune vs class downgrade vs known-debt at
v0.2.0 promotion.

#### Q-CONT-2 — WCAG formula impl → RATIFIED (a) DURABLE

Hand-rolled ~20 LoC WCAG 2.1 formula in `crates/ui/tests/contrast.rs`.
Zero `[dev-dependencies]` add. Math is closed (W3C-locked). Reference
vectors validate (architect dry-run: `WHITE on BLACK = 21.0000`,
`#777 on #FFF = 4.4781`, `#888 on #000 = 5.9240` — all match published
WCAG 2.1 reference values to 4 decimal places via the `lin → L → ratio`
chain). Library-compatibility checklist skipped — no new dep.

#### Q-CONT-3 — Opt-out marker placement → RATIFIED (a) DURABLE

In-file `OPT_OUTS: &[OptOutEntry]` table inside
`crates/ui/tests/contrast.rs`. Production `crates/ui/src/theme.rs`
stays test-annotation-free. Each entry carries mandatory `reason: &str`
and `pair_id: &str`. New opt-outs require touching ONE file (the
test); analyst → architect → developer review loop applies as
designed.

### D-clauses

#### D-CONT-1 — Asserter location + struct shape

- **File**: `crates/ui/tests/contrast.rs` (NEW). Sibling tests like
  `crates/ui/tests/consistency.rs` and `crates/ui/tests/layout_invariants.rs`
  set the precedent for `tests/<x>.rs` pure-Rust assertions reading
  `crates/ui/src/theme.rs` `pub const` tokens via the public API.
- **Types**:
  ```rust
  #[derive(Debug, Clone, Copy)]
  pub enum ContrastClass {
      /// WCAG 2.1 AA body text — assert ratio ≥ 4.5.
      Body,
      /// WCAG 2.1 AAA equity-critical text — assert ratio ≥ 7.0.
      Equity,
      /// Skip with mandatory reason — logged at runtime for audit.
      OptOut(&'static str),
  }

  #[derive(Debug, Clone, Copy)]
  pub struct ContrastPair {
      pub pair_id: &'static str,
      pub fg: iced::Color,
      pub bg: iced::Color,
      pub class: ContrastClass,
  }
  ```
- **Function**: `fn contrast_ratio(fg: iced::Color, bg: iced::Color) -> f64`
  returns the WCAG 2.1 ratio in `[1.0, 21.0]`. Pure function; no I/O.
- **Test fn**: `#[test] fn all_theme_pairs_meet_wcag()` iterates the
  `PAIRS` const table, computes per-pair ratio, dispatches per `class`,
  and either PANICs (gate mode) or `eprintln!`s (WARN mode) on failure.
  Also asserts `PAIRS.len() >= MIN_PAIRS` floor per D-CONT-5.
- **Helper test fns** (separate `#[test]` per reference vector):
  - `fn ref_vector_white_on_black_is_21()`
  - `fn ref_vector_black_on_white_is_21()`
  - `fn ref_vector_777_on_fff_is_4_48()`
  - `fn ref_vector_888_on_000_is_5_92()`
  Reference vectors validate the formula impl per K4 mitigation. Each
  uses `assert!((computed - expected).abs() < 0.01)` for f64 tolerance.

#### D-CONT-2 — WCAG 2.1 contrast formula

Hand-rolled per Q-CONT-2 ratification. Three pure functions:

```rust
/// sRGB channel [0,1] → linearized luminance per WCAG 2.1.
fn linearize(c: f32) -> f64 {
    let c = c as f64;
    if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// Relative luminance per W3C "Relative luminance" definition.
fn relative_luminance(color: iced::Color) -> f64 {
    0.2126 * linearize(color.r)
        + 0.7152 * linearize(color.g)
        + 0.0722 * linearize(color.b)
}

/// Contrast ratio per WCAG 2.1: `(L_lighter + 0.05) / (L_darker + 0.05)`.
fn contrast_ratio(fg: iced::Color, bg: iced::Color) -> f64 {
    let l1 = relative_luminance(fg);
    let l2 = relative_luminance(bg);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}
```

**Spec reference**: [W3C Relative luminance][rl]; [WCAG 2.1 contrast
ratio][wcag-cr]. Ratio range `[1.0, 21.0]` (1.0 = identical colors,
21.0 = pure white on pure black).

Alpha handling: `iced::Color` carries `a: f32`. **The asserter ignores
alpha** — the WCAG 2.1 formula is defined on opaque colors. The two
sub-AA tokens with `a < 1.0` in the audit (`UP_50`, `DOWN_50`,
`WARN_50`, `INFO_50`, `ACCENT_SOFT`, `OVERLAY`) are NOT body-text
tokens — they're tint backdrops. None enter the PAIRS table at v0.1.0.
Future tint-as-text usage would require opt-out with reason
`"alpha-tinted-decorative"`. Documented in the file header.

[rl]: https://www.w3.org/WAI/GL/wiki/Relative_luminance
[wcag-cr]: https://www.w3.org/TR/WCAG21/#dfn-contrast-ratio

#### D-CONT-3 — Theme.rs token-pair enumeration (M-T1 dry-run)

**Architect M-T1 enumerated 30 `ModeColor` constants** in
`crates/ui/src/theme.rs::color` (lines 128-404; `SPINNER_TINT` aliases
`FG_3` and is NOT a new token). The PAIRS table at v0.1.0 ships **66
entries** organized by intent:

| Group | Pair count | Class | Notes |
|-------|-----------|-------|-------|
| `FG_1..4 × {CANVAS, PANEL, PANEL_RAISED, PANEL_SUNKEN} × {Dark, Light}` | 32 | `Body` or `OptOut("disabled-text-tier")` for FG_4 | Core text legibility matrix |
| `FG_1 × {CANVAS, PANEL} × {Dark, Light}` (equity duplicates with Equity class) | 4 | `Equity` | AAA gate for H1 equity display per ui-design-principles § Accessibility minimums |
| `FG_ON_ACCENT × {ACCENT, ACCENT_HOVER, ACCENT_PRESS} × {Dark, Light}` | 6 | `Body` | Accent-fill button text legibility |
| `{UP_500, DOWN_500, WARN_500, INFO_500} × {CANVAS, PANEL} × {Dark, Light}` | 16 | `Body` | Semantic ramp text per P&L coloring rule |
| `{UP_400, DOWN_400} × {CANVAS, PANEL} × {Dark, Light}` | 8 | `OptOut("chart-line-stroke-not-text")` | Chart line strokes per ui-design-principles; non-text by design |
| `{ACCENT_2, ACCENT_3, ACCENT_4, ACCENT_5} × {CANVAS, PANEL} × {Dark, Light}` | 16 | `OptOut("chart-comparison-stroke-not-text")` | Comparison-overlay strokes per `accent_palette()` doc; non-text |
| `BORDER_STRONG × CANVAS × Dark` | 1 | `OptOut("border-not-text")` | Hairline border decoration |

**Total: 83 entries.** Architect uses **`MIN_PAIRS = 60`** floor (per
D-CONT-5) — comfortably below the v0.1.0 count of 83 to allow normal
palette evolution without a tasks bump, while still catching a
catastrophic enumeration-shape refactor that drops to ~0.

**Audit ratio table — DARK mode** (selected rows; full ratios in
M-T1 dry-run computed Python check):

| Pair | Ratio | Class | Verdict |
|------|-------|-------|---------|
| `fg_1_on_canvas_dark` | 15.01 | Body / Equity | PASS / PASS |
| `fg_2_on_canvas_dark` | 9.61 | Body | PASS |
| `fg_3_on_canvas_dark` | 5.02 | Body | PASS |
| `fg_3_on_panel_dark` | 4.57 | Body | PASS (marginal) |
| `fg_3_on_panel_raised_dark` | 3.75 | Body | **WARN** — sub-AA |
| `fg_4_on_*` | 2.25–3.25 | OptOut("disabled-text-tier") | SKIP (logged) |
| `fg_on_accent_on_accent_dark` | 8.21 | Body | PASS |
| `up_500_on_canvas_dark` | 5.56 | Body | PASS |
| `down_500_on_canvas_dark` | 5.49 | Body | PASS |
| `warn_500_on_canvas_dark` | 9.20 | Body | PASS |
| `info_500_on_canvas_dark` | 7.08 | Body | PASS |
| `border_strong_on_canvas_dark` | 1.95 | OptOut("border-not-text") | SKIP |

**Audit ratio table — LIGHT mode**:

| Pair | Ratio | Class | Verdict |
|------|-------|-------|---------|
| `fg_1_on_canvas_light` | 16.91 | Body / Equity | PASS / PASS |
| `fg_3_on_canvas_light` | 4.90 | Body | PASS |
| `fg_3_on_panel_light` | 5.16 | Body | PASS |
| `fg_4_on_*` | 2.44–2.90 | OptOut("disabled-text-tier") | SKIP |
| `fg_on_accent_on_accent_light` | **3.52** | Body | **WARN** — sub-AA |
| `up_500_on_canvas_light` | **4.46** | Body | **WARN** — marginally sub-AA |
| `down_500_on_canvas_light` | **4.33** | Body | **WARN** — sub-AA |
| `warn_500_on_canvas_light` | **2.96** | Body | **WARN** — sub-AA |
| `warn_500_on_panel_light` | **3.11** | Body | **WARN** — sub-AA |
| `info_500_on_canvas_light` | 5.07 | Body | PASS |
| `info_500_on_panel_light` | 5.34 | Body | PASS |

**Two genuine WCAG-AA defects surfaced by M-T1 dry-run** (NOT opt-outs):

1. **`FG_ON_ACCENT` on light-mode `ACCENT`** — pure-white text on
   accent-400 (`#3F968D`) renders at 3.52:1. Affects accent button
   text in light mode. Recommendation at v0.2.0 promotion: tune
   `ACCENT.light` to `#2A7B73` (accent-500) which already passes
   contrast as the existing `ACCENT_HOVER.light` — OR tune
   `FG_ON_ACCENT.light` away from pure white.
2. **`WARN_500` family on light-mode `CANVAS` / `PANEL`** — amber
   on warm-50 / warm-25 renders at 2.96–3.11. Affects latency
   warnings + caution text in light mode. Recommendation at v0.2.0:
   tune `WARN_500.light` to a darker amber (e.g. `#8C6324`) OR
   downgrade affected widgets to the existing `WARN_400` darker
   variant for text use.

WARN mode logs these without blocking CI. Operator decides at v0.2.0
promotion.

**Marginal sub-AA pairs** (4.33–4.46): `up_500_on_canvas_light`,
`down_500_on_canvas_light`. Recommend monitoring during WARN window
— may pass on `PANEL` but fail on `CANVAS`; widget surface usage
dictates whether this is real or theoretical. Listed in v0.2.0
review queue.

#### D-CONT-4 — Opt-out list seed (architect M-T1)

**`OPT_OUTS` table seeded with 9 entries** (NOT the H2 ≤ 3 estimate —
H2 INVALIDATED but the count remains bounded and reviewable):

```rust
struct OptOutEntry {
    pair_id: &'static str,
    reason: &'static str,
}

const OPT_OUTS: &[OptOutEntry] = &[
    // FG_4 placeholder/disabled tier — sub-AA by design per
    // ui-design-principles ## Color palette "FG_4 — placeholder /
    // disabled". Disabled text MAY be sub-AA per WCAG 2.1 § 1.4.3
    // ("inactive UI components" exception).
    OptOutEntry { pair_id: "fg_4_on_canvas_dark",         reason: "disabled-text-tier" },
    OptOutEntry { pair_id: "fg_4_on_panel_dark",          reason: "disabled-text-tier" },
    OptOutEntry { pair_id: "fg_4_on_panel_raised_dark",   reason: "disabled-text-tier" },
    OptOutEntry { pair_id: "fg_4_on_panel_sunken_dark",   reason: "disabled-text-tier" },
    OptOutEntry { pair_id: "fg_4_on_canvas_light",        reason: "disabled-text-tier" },
    OptOutEntry { pair_id: "fg_4_on_panel_light",         reason: "disabled-text-tier" },
    OptOutEntry { pair_id: "fg_4_on_panel_raised_light",  reason: "disabled-text-tier" },
    OptOutEntry { pair_id: "fg_4_on_panel_sunken_light",  reason: "disabled-text-tier" },
    // Border decoration — non-text hairline divider per
    // ui-design-principles ## Tier elevation model.
    OptOutEntry { pair_id: "border_strong_on_canvas_dark", reason: "border-not-text" },
];
```

**Excluded from PAIRS table entirely** (not even logged as opt-outs;
these are class-mismatches, not per-pair opt-outs):
- `UP_400 / DOWN_400 × CANVAS / PANEL` (8 pairs) — chart-line strokes;
  class `OptOut("chart-line-stroke-not-text")` if enumerated, but the
  audit lists them under PAIRS so they're logged. Operator note: keep
  these in PAIRS with class `OptOut` to maintain the data-driven
  enumeration shape (per R4.3 audit-of-exclusions).
- `ACCENT_2..5 × CANVAS / PANEL` (16 pairs) — comparison-overlay
  strokes per `accent_palette()` doc comment ("multi-strategy comparison
  overlay … chart line draw pass"). Same treatment: keep in PAIRS with
  class `OptOut("chart-comparison-stroke-not-text")`.

**Net opt-out logged entries: 9 + 8 + 16 = 33 OptOut entries** out
of 83 total PAIRS. **50 entries** assert as `Body` or `Equity`. Of
those 50, **5 will WARN-log in WARN mode** at v0.1.0 ship per the
D-CONT-3 audit (`fg_3_on_panel_raised_dark`,
`fg_on_accent_on_accent_light`, `up_500_on_canvas_light`,
`down_500_on_canvas_light`, `warn_500_on_canvas_light`,
`warn_500_on_panel_light`) — call it 6 to be precise (fg_3 marginal +
2 accent variants + 3 warn/up/down).

#### D-CONT-5 — MIN_PAIRS floor

```rust
/// Floor for the PAIRS table count. Defends against K2 — a future
/// theme.rs refactor that re-shapes token storage and silently
/// breaks PAIRS enumeration. The v0.1.0 PAIRS table contains 83
/// entries; 60 is a comfortable floor that allows normal palette
/// evolution (token removal / class downgrade) without bumping the
/// floor in tasks.md, while still catching a catastrophic
/// enumeration-shape break that drops the count to ~0.
const MIN_PAIRS: usize = 60;

#[test]
fn pairs_table_meets_minimum_count() {
    assert!(
        PAIRS.len() >= MIN_PAIRS,
        "theme token enumeration detected only {} pairs; \
         refactor likely broke enumeration (MIN_PAIRS = {})",
        PAIRS.len(),
        MIN_PAIRS,
    );
}
```

#### D-CONT-6 — WARN-mode mechanism (Q-CONT-1 ratification)

Environment-variable gated. Default `warn` at v0.1.0; flips to `gate`
at v0.2.0 patch (2026-06-12 or after operator confirms the WARN
observation window). Implementation sketch:

```rust
/// Mode selector. Returns `Mode::Warn` unless `UI_CONTRAST_MODE=gate`.
enum Mode { Warn, Gate }
fn current_mode() -> Mode {
    match std::env::var("UI_CONTRAST_MODE").as_deref() {
        Ok("gate") => Mode::Gate,
        _ => Mode::Warn,  // default at v0.1.0
    }
}
```

On per-pair failure:
- **Warn**: `eprintln!("WARN: contrast pair {pair_id} = {ratio:.2} < threshold {threshold:.1}");`
  Continue iteration. After all pairs processed, test exits PASS.
- **Gate**: Collect violations into a `Vec<String>`; if non-empty,
  `panic!("contrast assertion failed:\n{joined}")` at end of test.

On per-pair opt-out (`OptOut(reason)`): always log audit line
`eprintln!("opt-out: {pair_id}; reason: {reason}; ratio: {ratio:.2}");`
regardless of mode (per R4.3 audit-of-exclusions).

**v0.2.0 promotion contract**: a follow-up brief at 2026-06-12+ flips
the `current_mode()` default arm from `_ => Mode::Warn` to
`_ => Mode::Gate`. The env var `UI_CONTRAST_MODE=warn` becomes the
opt-out escape hatch (CI-pinning, local dev) but operator default
is gate.

#### D-CONT-7 — ADR contract

**No new ADR.** Per analyst direction + the test-infra precedent set
by ADR-0048 (`lab-recipe-test-harness`), the asserter is a
boundary-test-shaped test fixture that reads `pub const` tokens via
the production public API and asserts a closed-math invariant. This
is the same pattern ADR-0048 § D1-D6 established for the lab-recipe
boundary tests. **One Changelog row appended to ADR-0048** at this
M-T1 close documenting the carry-forward; ADR-0048 § README registry
table summary line updated atomically per architect.md § ADR registry
atomic-write contract.

### Falsification probe P-CONT-1 (T-T1 self-falsification)

Falsifier: temporarily add a deliberately-low-contrast pair to the
PAIRS table and confirm the test FAILs in gate mode + WARN-logs in
warn mode.

**Recipe** (developer runs at M-DEV T-CONT-D6; tester re-runs at
M-FINAL T-CONT-FINAL.3):

```bash
# 1. Edit crates/ui/tests/contrast.rs PAIRS table — insert at top:
#    ContrastPair {
#        pair_id: "probe_low_contrast_white_on_pale_grey",
#        fg: iced::Color::WHITE,
#        bg: iced::Color::from_rgb(0.9, 0.9, 0.9),
#        class: ContrastClass::Body,
#    },
# 2. Run test in gate mode:
UI_CONTRAST_MODE=gate cargo test -p ui --test contrast -- --nocapture
# Expected stderr: panic with "contrast assertion failed:\n  probe_low_contrast_white_on_pale_grey = 1.07 < threshold 4.5"
# Expected exit code: nonzero.

# 3. Run test in warn mode (default):
cargo test -p ui --test contrast -- --nocapture
# Expected stderr: "WARN: contrast pair probe_low_contrast_white_on_pale_grey = 1.07 < threshold 4.5"
# Expected exit code: 0 (PASS).

# 4. Revert the probe entry; rerun cargo test to confirm clean PASS.
```

**MIN_PAIRS floor probe variant** (developer T-CONT-D6 alternative):

```bash
# Comment out 25 entries in PAIRS to drop below 60 floor.
UI_CONTRAST_MODE=gate cargo test -p ui --test contrast -- --nocapture
# Expected: panic with "theme token enumeration detected only 58 pairs;
#           refactor likely broke enumeration (MIN_PAIRS = 60)".
# Revert; rerun; PASS.
```

### Wave decomposition

**Single M-DEV wave** (~0.5 dev day) per T-CONT-T1.5. Developer
tasks T-CONT-D1..D9 already enumerated in tasks.md — no additional
decomposition needed.

### Architect risk register

- **K1 PARTIALLY TRIGGERED**: 9 opt-outs (not ≤ 3). All bounded +
  reason-stringed + reviewable. Not a route-back-to-analyst trigger
  — opt-out list is data-driven + per-token, exactly the shape R4
  prescribed. The 5-6 WARN-logging genuine sub-AA pairs (light-mode
  semantic ramp + accent-fill text) are the asserter's first signal
  to operator + the explicit reason for the 2-week WARN observation
  window.
- **K2 mitigation locked**: `MIN_PAIRS = 60` (v0.1.0 PAIRS count is
  83; 23-pair safety margin).
- **K3 NOT applicable**: test asserts on hex tokens, not pixels;
  cross-platform color profile drift is below the test layer.
- **K4 mitigation locked**: 4 reference-vector unit tests per
  D-CONT-1 (`WHITE on BLACK = 21.00`, `BLACK on WHITE = 21.00`,
  `#777 on #FFF = 4.48`, `#888 on #000 = 5.92`). Architect dry-run
  validated all 4 to 4-decimal precision.
- **No new dep**: hand-rolled formula per Q-CONT-2(a). Library /
  crate compatibility checklist N/A.
- **No production code touch**: R-NR.1 contract holds — only
  `crates/ui/tests/contrast.rs` adds.

### Operator decisions deferred to v0.2.0 promotion

Listed for the v0.2.0 brief (analyst owns):

1. **`FG_ON_ACCENT` light-mode failing 3.52** — accent button text
   recolor (analyst recommendation: tune `ACCENT.light` to
   accent-500 hex `#2A7B73` — already passes existing `ACCENT_HOVER.light`).
2. **`WARN_500` light-mode failing 2.96-3.11** — amber warn text
   recolor (analyst recommendation: tune `WARN_500.light` to a
   deeper amber e.g. `#8C6324`).
3. **`UP_500 / DOWN_500` light-mode marginal 4.33-4.46** — P&L text
   recolor (analyst recommendation: monitor WARN signal; tune at
   v0.2.0 if widget surface usage confirms `CANVAS` is hit).
4. **`fg_3_on_panel_raised_dark` 3.75** — tertiary text on dialog
   popovers (analyst recommendation: tune `FG_3.dark` from `#808993`
   to `#9098A2` to raise contrast, OR opt-out with reason
   `"dialog-tertiary-label-decorative"` if widget audit shows usage
   is non-body).

## Backtest Scenarios

_N/A — UI test infrastructure feature; no backtest scenarios
attach. The R-NR.4 anchor contract carries the equivalent
regression guarantee (75/75 anchors byte-identical pre/post)._

## Implementation

**Developer M-DEV 2026-05-29.** Single-wave delivery per architect M-T1 wave decomposition.

### Files changed

- **NEW** `crates/ui/tests/contrast.rs` — 83-entry `PAIRS` const table + 9-entry `OPT_OUTS` const table + hand-rolled WCAG 2.1 formula (3 pure fns, ~20 LoC) + 7 `#[test]` fns + 2 `#[ignore]` falsification probe stubs.

### Zero production code touched

`git diff -- crates/ui/src/` is empty. R-NR.1 contract holds.

### Test results (WARN mode default)

```
running 9 tests
test probe_low_contrast_rejects_in_gate_mode ... ignored
test probe_min_pairs_floor_fires_when_pairs_truncated ... ignored
test pairs_table_meets_minimum_count ... ok
test ref_vector_777_on_fff_is_4_48 ... ok
test ref_vector_888_on_000_is_5_92 ... ok
test ref_vector_white_on_black_is_21 ... ok
test opt_outs_all_have_reasons ... ok
test all_theme_pairs_meet_wcag ... ok
test ref_vector_black_on_white_is_21 ... ok

test result: ok. 7 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Design-intent WARN lines (6 expected, all observed)

```
WARN: contrast pair fg_3_on_panel_raised_dark = 3.75 < threshold 4.5
WARN: contrast pair fg_on_accent_on_accent_light = 3.52 < threshold 4.5
WARN: contrast pair up_500_on_canvas_light = 4.46 < threshold 4.5
WARN: contrast pair down_500_on_canvas_light = 4.33 < threshold 4.5
WARN: contrast pair warn_500_on_canvas_light = 2.96 < threshold 4.5
WARN: contrast pair warn_500_on_panel_light = 3.11 < threshold 4.5
```

### Gate mode (UI_CONTRAST_MODE=gate)

Panics with the same 6 violations as expected at v0.1.0. Operator promotes
to gate at v0.2.0 after upstream hex tune per § Operator decisions deferred
to v0.2.0 promotion.

### Falsification probes (both verified before commit)

- **P-CONT-1.A**: Low-contrast probe pair → gate panic "probe_low_contrast_white_on_pale_grey = 1.25 < threshold 4.5". PASS → reverted.
- **P-CONT-1.B**: MIN_PAIRS=200 temporarily → floor panic "theme token enumeration detected only 83 pairs; refactor likely broke enumeration (MIN_PAIRS = 200)". PASS → reverted.

### Anchors

75/75 PASS byte-identical (`bash scripts/verify_anchors.sh`). Zero anchor delta — pure test infra addition as expected.

### Sibling test non-regression

- `visual_fail_html_self_test`: 2/2 PASS
- `visual_snapshots`: 51/51 PASS

## Verification

_Tester M-FINAL links the test-final report + the WARN-mode
observation log (cargo test stderr output showing `eprintln!`
warnings, if any) + the falsification probe outcomes +
`bash scripts/verify_anchors.sh` 75/75 PASS byte-identical
pre/post + confirmation that the existing visual-snapshot tests
under `crates/ui/tests/visual_*.rs` PASS byte-identical._

## v0.2.0 close-out

> **Analyst close-out scope 2026-06-15.** v0.1.0 is `tester-done` (VERDICT
> PASS 2026-05-29, WARN default) but was NEVER formally shipped — the
> presenter was parked (`HANDOFF → presenter`, never run). The 2-week WARN
> observation window contracted at v0.1.0 (per Q-DUO-WARN) has elapsed. This
> close-out ships the asserter, promotes it to enforcing, and disposes of the
> 6 known sub-AA pairs. **Proportionate, low-risk: the RECOMMENDED path adds
> ZERO color change and ZERO visual-baseline churn.**

### Why now (close-out trigger)

Two loose ends from v0.1.0:

1. **Shipped-but-unpresented.** The asserter PASSES in WARN mode and is wired
   into `cargo test -p ui`, but no presenter deck closed the loop. Per
   AGENT.md the presenter is the sprint-review face that runs after
   `VERDICT → PASS`; v0.1.0 earned its PASS but the deck was parked. Formal
   ship is overdue.
2. **WARN-but-not-enforcing.** The asserter currently only `eprintln!`s on a
   sub-AA pair — a NEW contrast regression (e.g. a future `FG_3` hex tweak
   that drops `fg_3_on_panel_light` below 4.5) would log a WARN and the test
   would still PASS. The asserter does not yet do the one job it exists for:
   BLOCK the regression. Flipping the default to `gate` closes that gap.

### V2-R1 — Run the parked presenter on v0.1.0 (ship as-is)

- **V2-R1.1** The presenter assembles
  `spec/ui-contrast-asserter/presentations/ui-contrast-asserter-<date>.md`
  per the parked `T-CONT-P1` acceptance criteria (cross-cutting safety-duo
  framing recap; the M-T1 pair-count + opt-out-count table; the 6 design-intent
  WARN lines as the asserter's first signal; the 2-week WARN observation
  contract now elapsed; sibling-redactor cross-link; operator-decide-ready).
- **V2-R1.2** This is the FIRST deliverable and is **independent of the
  gate-flip decision** — the asserter ships as-is (WARN-default, PASSING)
  regardless of whether V2-R2/V2-R3 land in this cycle or the next. Ship is
  not blocked on the 6-pair disposition.
- **V2-R1.3** No code change. Pure presenter deck + frontmatter
  `status: shipped` on the v0.1.0 baseline once the deck is operator-approved.

### V2-R2 — Flip default `UI_CONTRAST_MODE` WARN → gate (enforce)

- **V2-R2.1** `current_mode()` in
  [`crates/ui/tests/contrast.rs:101-106`](../../crates/ui/tests/contrast.rs)
  flips its default arm from `_ => Mode::Warn` to `_ => Mode::Gate`. The env
  var `UI_CONTRAST_MODE=warn` becomes the explicit opt-out escape hatch
  (local dev, CI-pinning); operator default becomes `gate`. This is the
  v0.2.0 promotion contract pre-drawn at v0.1.0 § D-CONT-6.
- **V2-R2.2** The gate-flip is **conditional on V2-R3** — a bare flip with the
  6 sub-AA pairs still classed `Body` would turn the existing 6 WARN lines into
  a hard `panic!` and RED-fail `cargo test -p ui`. The 6 MUST be disposed
  (ratified opt-out OR tuned to ≥AA) in the same change-set as the flip.
- **V2-R2.3** Doc-comment + file-header updates in `contrast.rs` to reflect
  gate-as-default (the v0.1.0 header says "warn (default at v0.1.0)"; v0.2.0
  says "gate (default at v0.2.0); UI_CONTRAST_MODE=warn opts out").
- **V2-R2.4** Update the in-file `all_theme_pairs_meet_wcag` doc block (lines
  826-834) that enumerates the "~6 WARN-log lines" — under gate-default with
  the 6 ratified opt-out, that doc block describes the opt-out audit lines
  instead.

### V2-R3 — Disposition of the 6 sub-AA light-mode pairs (OPERATOR DECISION)

All 6 sub-AA pairs are LIGHT-mode (confirmed by the v0.1.0 tester + re-verified
by this analyst against the live `theme.rs` hex). Per-pair analysis with the
ratio gap to AA (4.5:1) and tune-vs-opt-out recommendation:

| # | pair_id | ratio | gap to 4.5 | tune feasibility | analyst rec |
|---|---------|-------|-----------|------------------|-------------|
| 1 | `up_500_on_canvas_light` | 4.46 | −0.04 | **trivial** — `UP_500.light #547A52 → #4C6F4A` lifts to 5.19 canvas / 5.47 panel | TUNE (easy AA win) if (B); else opt-out |
| 2 | `down_500_on_canvas_light` | 4.33 | −0.17 | **trivial** — `DOWN_500.light #A95F46 → #9E5740` lifts to 4.91 / 5.17 | TUNE (easy AA win) if (B); else opt-out |
| 3 | `fg_3_on_panel_raised_dark`† | 3.75 | −0.75 | **easy, single-surface** — `FG_3.dark #808993 → #9098A2` lifts to 4.56 on raised | TUNE if (B); else opt-out |
| 4 | `fg_on_accent_on_accent_light` | 3.52 | −0.98 | **easy, reuses existing hex** — `ACCENT.light #3F968D → #2A7B73` (already the `ACCENT_HOVER.light` value) lifts white-on-accent to 5.02 | TUNE if (B); else opt-out |
| 5 | `warn_500_on_panel_light` | 3.11 | −1.39 | **hard** — amber-on-light; `WARN_500.light` would need `#704F1C`-class darkening (big hue/value shift, muddies the amber) to clear panel at 7.1; `#8C6324` only reaches 5.13 panel / 4.87 canvas (marginal) | **OPT-OUT** |
| 6 | `warn_500_on_canvas_light` | 2.96 | −1.54 | **hardest** — same amber-on-light problem; the worst pair | **OPT-OUT** |

† **Correction to the brief's "all 6 are light-mode":** pair #3
`fg_3_on_panel_raised_dark` (3.75) is a **DARK-mode** pair (`FG_3.dark` on
`PANEL_RAISED.dark`). The other 5 are light-mode. The 6-as-a-set are the exact
pairs the v0.1.0 asserter WARN-logs; their disposition is what V2-R3 decides
regardless of mode. (Analyst flags this so the architect does not lock a
"6 light-mode" invariant that the code contradicts — the no-silent-divergence
non-negotiable.)

**Tune feasibility re-verified** by this analyst running the shipped WCAG
formula against the live `theme.rs` hex (candidate darkenings listed above all
computed ≥4.5 on the named surfaces). 4 of 6 are trivially tunable; the 2
`WARN_500` amber pairs are classically hard (yellow on a near-white surface
cannot reach 4.5 without becoming a brown).

#### The operator choice

**(A) Ratify all 6 as opt-out debt — RECOMMENDED (DURABLE for a close-out).**
Add 6 `OptOut(&'static str)` entries to the PAIRS table (re-class the 6 from
`Body` → `OptOut("known-light-mode-sub-aa-debt-v0.2.0")` or a per-pair reason)
PLUS 6 mirror rows in the `OPT_OUTS` manifest table, each with a mandatory
reason string naming the ratio + the v0.2.0 disposition. **Zero color change,
zero visual-baseline churn, zero rebaseline, zero operator color sign-off.**
The gate still ENFORCES for every OTHER pair (44 of 50 asserting pairs stay
`Body`/`Equity` under a hard gate) — so the a11y gate goes enforcing
*immediately* while the 6 known exceptions are honestly documented as debt.
This is the proportionate close-out: it ships the enforcing gate now and parks
the color question as documented, reviewable, per-pair debt that a later
dedicated palette-tune feature can retire. **Recommended because it makes the
gate durable-enforcing in ONE cycle without spawning a color-redesign
sub-project** — the alternative (B) couples a tidy-up to a baseline-churning
color change that needs its own operator sign-off loop.

**(B) Tune the N trivially-fixable pairs + opt-out the rest — fallback if the
operator wants the AA wins now.** Tune pairs #1, #2, #3, #4 (all reach ≥4.5
with the small darkenings above) and opt-out only #5, #6 (the hard amber).
Result: 2 opt-out entries instead of 6; 4 genuine AA fixes. **NOT zero-cost:**
tuning a Lumen token CHANGES rendered colors → changes visual snapshot
baselines (`crates/ui/tests/visual_snapshots.rs`, 51 snapshots) → requires a
rebaseline AND operator sign-off on each new hex (the colors ship to the live
cockpit). It also touches production `theme.rs` (v0.1.0 was strictly
test-only; R-NR.1 no longer holds). Pair #4 is the cleanest of the four
(reuses an existing palette hex, `ACCENT_HOVER.light`) so even a partial-(B)
"tune only #4, opt-out 5" is defensible if the operator wants one quick win.

**Disposition is the operator's call (V-CONT2-1 below). The analyst recommends
(A)** for a close-out: it is the durable choice *for this feature's stated goal*
(make the gate enforcing) and avoids spawning a color-redesign follow-on. The
color question is real but belongs to a dedicated palette-tune feature, not a
tidy-toward-done close-out. **If-budget-tightens annotation:** (A) is also the
cheaper path here — there is no cheaper fallback to name; (B) is the MORE
expensive option, chosen only if the operator explicitly wants the AA wins
banked this cycle.

### V2-acceptance criteria

- **V2-AC.1** Presenter deck `ui-contrast-asserter-<date>.md` exists,
  operator-approved; v0.1.0 baseline frontmatter `status: shipped`. (V2-R1)
- **V2-AC.2** With all 6 ratified as opt-out (path A), `current_mode()`
  defaults to `Gate`, and `UI_CONTRAST_MODE` **unset** → gate mode →
  `cargo test -p ui --test contrast` **PASSES** (the 6 are opt-out'd and
  logged as audit lines; every other `Body`/`Equity` pair is ≥ its threshold
  under a hard gate). (V2-R2 + V2-R3-A)
- **V2-AC.3** With `UI_CONTRAST_MODE=warn` set, behavior is byte-identical to
  v0.1.0 WARN mode (escape hatch preserved). (V2-R2.1)
- **V2-AC.4** A NEW deliberately-sub-AA `Body` pair (the P-CONT-1.A probe)
  added to PAIRS → gate-default (env unset) → test **FAILS** (panic). This is
  the regression-blocking proof the flip exists for. (V2-R2)
- **V2-AC.5** `opt_outs_all_have_reasons` still PASSES — every new opt-out
  entry carries a non-empty reason string. (V2-R3-A)
- **V2-AC.6** Under path (A): `git diff -- crates/ui/src/` is EMPTY (R-NR.1
  still holds — opt-out ratification is test-only); `bash scripts/verify_anchors.sh`
  byte-identical; `visual_snapshots` byte-identical (zero color change). Under
  path (B): `theme.rs` diff is the N tuned tokens, visual baselines
  rebaselined with operator color sign-off, anchors re-checked.
- **V2-AC.7** `MIN_PAIRS = 60` floor unchanged; re-classing 6 pairs `Body →
  OptOut` does not change PAIRS table length (83 entries stay).

### V2 — baseline-equity-divergence e2e gate: N/A (justified)

The CLAUDE.md non-negotiable "every strategy overlay or sizing-modifier ships
with a baseline-equity-divergence e2e test" is **N/A** for this feature.
`crates/ui/tests/contrast.rs` is **UI test-tooling** — it reads `pub const`
color tokens and asserts a closed-math WCAG ratio. It has **no strategy, no
sizing modifier, no equity curve, and touches no backtest path**. There is no
equity output to diverge. The v0.1.0 regression guarantee is carried by the
anchor contract (75/75 byte-identical) + the `visual_snapshots` byte-identity
under path (A), and by the gate-mode falsification probe (V2-AC.4) which is the
contrast-domain analogue of the divergence assertion: it proves the gate's
output (PASS vs panic) actually responds to a non-trivial input (a sub-AA
pair). Same justification class as v0.1.0 § Backtest Scenarios N/A.

### V2 — operator decisions

#### V-CONT2-1 — Disposition of the 6 sub-AA pairs: (A) ratify-all vs (B) tune-N

**Q.** How are the 6 known sub-AA pairs disposed when the gate flips to
enforcing? **(A)** re-class all 6 as documented `OPT_OUTS` debt (zero color
change, zero baseline churn) — **RECOMMENDED**; **(B)** tune the 4 trivially-
fixable pairs (#1 up_500, #2 down_500, #3 fg_3-dark, #4 fg-on-accent) to ≥AA +
opt-out the 2 hard amber pairs (#5, #6) — banks 4 AA wins but touches
production `theme.rs`, changes rendered colors, and needs a visual rebaseline +
per-hex operator sign-off.

**Default: (A).** Proportionate to a close-out; makes the gate enforcing in one
cycle; parks the color question as honest per-pair debt for a future dedicated
palette-tune feature. **This is the riskiest open question** — it determines
whether v0.2.0 stays test-only (A) or becomes a production color change (B).

#### V-CONT2-2 — Gate-flip timing: ship the gate-default in v0.2.0, or one more env-opt-in cycle?

**Q.** Does the WARN→gate default flip (V2-R2) ship IN this v0.2.0, or does
v0.2.0 ship only the presenter (V2-R1) + the 6-pair ratification (V2-R3-A) and
leave `current_mode()` defaulting to WARN for one more cycle (operator opts in
via `UI_CONTRAST_MODE=gate` until then)?

**(Recommended) Option A — flip in v0.2.0.** The 2-week WARN observation
window contracted at v0.1.0 has elapsed; the 6 WARN lines are stable and
understood; ratifying them (V2-R3-A) removes the only thing that would
RED-fail a hard gate. Holding the flip another cycle leaves the asserter
non-enforcing — a new contrast regression would still slip through as a WARN.
The whole point of the close-out is to make the gate DO ITS JOB. **Durable.**

**Option B (fallback) — presenter + ratify now, flip next cycle.** Ships the
deck + documents the 6 as opt-out but leaves WARN as default one more cycle.
Lower blast-radius (no chance a missed asserting pair RED-fails CI on the flip
day) but leaves the gate toothless and adds a v0.2.1 flip-only follow-on.
Choose only if the operator wants to eyeball the gate-mode run output (env-set)
for a cycle before making it default.

**Default: A** (flip in v0.2.0) — conditional on V-CONT2-1 disposing the 6 so
the hard gate is GREEN on first default run.

### V2 — falsifiers

- **V2-K1 — A pair currently asserting `Body` is below 4.5 but was NOT in the
  known-6.** If the gate-flip change-set's dry-run (developer/tester re-run in
  gate mode) surfaces a 7th sub-AA `Body` pair the v0.1.0 audit missed, the
  hard gate RED-fails on first default run. **Mitigation:** the developer MUST
  run `UI_CONTRAST_MODE=gate cargo test -p ui --test contrast` BEFORE flipping
  the default and confirm exactly the known 6 (now opt-out'd) and zero others.
  Route back to analyst if a 7th surfaces — that is a genuine new defect, not a
  close-out item.
- **V2-K2 — Path-(B) token tune cascades to a DARK-mode or PANEL pair.**
  Darkening `UP_500.light` is light-mode-only (the `.light` field), so it
  cannot regress a dark pair; but a careless tune that edits the `.dark` field
  too would. **Mitigation:** path-(B) edits touch ONLY the `.light` field of
  the 4 tuned tokens; the gate re-run catches any dark regression. (N/A under
  recommended path A.)
- **V2-K3 — Re-classing 6 `Body → OptOut` silently masks a real future
  regression on those same pair_ids.** If a later refactor makes
  `warn_500_on_canvas_light` somehow tunable but the opt-out entry persists,
  the gate would never re-flag it. **Mitigation:** each opt-out reason string
  names the ratio + "v0.2.0 debt" so a code-review diff of `OPT_OUTS` surfaces
  the debt; a future palette-tune feature is expected to retire these 6 entries
  and re-class them `Body`.

### V2 — hypotheses

- **V2-H1 — Path (A) is ≤ ~20 LoC + 1 deck.** 6 PAIRS re-class edits (1 line
  each: `class:` field) + 6 `OPT_OUTS` rows (~4 lines each) + the
  `current_mode()` default arm flip (1 line) + header/doc-comment updates.
  No new types, no new tests beyond what V2-AC asserts. ~0.25 dev day +
  ~0.25 tester day + presenter deck.
- **V2-H2 — Gate-flip surfaces exactly the known 6, zero new.** The v0.1.0
  M-T1 audit enumerated all 83 pairs; the WARN window logged exactly 6; no
  `theme.rs` color change has landed since (verify: `git log` on theme.rs since
  2026-05-29). So the gate-mode dry-run should show exactly the 6. If it shows
  more, V2-K1 fires.
- **V2-H3 — Path (A) keeps R-NR.1 (zero production code).** Opt-out
  ratification is entirely within `crates/ui/tests/contrast.rs`; `theme.rs`
  untouched; anchors + visual snapshots byte-identical.

## Implementation — v0.2.0

**Developer V2-M-DEV 2026-06-15.** Path (A) delivery — test-file-only change.

### Files changed

- **MODIFIED** `crates/ui/tests/contrast.rs` only. Zero production code touched. R-NR.1 holds.

### Changes

1. **V2-R2 — gate-flip**: `current_mode()` default arm flipped `_ => Mode::Warn` → `_ => Mode::Gate`. `UI_CONTRAST_MODE=warn` is now the explicit escape hatch. File-header Mode section + `all_theme_pairs_meet_wcag` doc-block updated.
2. **V2-R3 — 6-pair ratification**: 6 PAIRS entries re-classed `Body → OptOut` with honest reason strings. 6 mirror rows added to `OPT_OUTS` manifest (now 15 entries, was 9). 2 `warn_500` amber pairs: "amber-on-light cannot reach 4.5:1 AA without abandoning the amber semantic — ratified light-mode debt". Other 4: "sub-AA [dark/light]-mode pair ratified as v0.2.0 opt-out debt; candidate for a future dedicated palette-tune (4 are trivially darkenable — see analyst per-pair table)".

### Test results (gate mode, env unset — KEY AC)

```
running 9 tests
test result: ok. 7 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Hidden-regression check (V2-K1)

Pre-flight gate-mode run (before opt-outs added) confirmed exactly 6 violations, zero 7th. Gate-default run after opt-out ratification: 0 violations.

### P-CONT-1.A probe (gate-default blocks regression)

Probe pair `probe_d6_white_on_pale_grey` (WHITE on 0.9/0.9/0.9) inserted → panic "probe_d6_white_on_pale_grey = 1.25 < threshold 4.5" with env unset. Reverted; 7/7 PASS confirmed.

### Byte-identity

`git diff --stat -- '*.png'` → zero; `git diff -- crates/ui/src/` → zero. Path (A): zero color change, zero visual-baseline churn.

## Changelog

- 2026-06-15 (analyst): v0.2.0 close-out brief appended. THREE deliverables —
  V2-R1 run the parked presenter on v0.1.0 (formal ship, as-is, WARN-default
  PASSING); V2-R2 flip default `UI_CONTRAST_MODE` WARN→gate (enforce);
  V2-R3 dispose the 6 known sub-AA pairs (RECOMMENDED path A: ratify all 6 as
  documented `OPT_OUTS` debt — zero color change, zero baseline churn; gate
  enforces for every other pair). Per-pair tune-vs-opt-out table computed
  against live `theme.rs` hex (4 of 6 trivially tunable; the 2 `WARN_500`
  amber pairs classically hard). **Correction logged:** pair #3
  `fg_3_on_panel_raised_dark` is DARK-mode, not light (brief said "all 6
  light-mode"); the no-silent-divergence non-negotiable flagged so the
  architect does not lock a false invariant. baseline-equity-divergence e2e
  gate N/A (UI test-tooling; no strategy/equity) — justified. Riskiest open
  question V-CONT2-1 (disposition A vs B) + V-CONT2-2 (gate-flip timing this
  cycle vs one more env-opt-in cycle). Frontmatter version 0.1.0 → 0.2.0,
  status dev-done → proposed, owner tester → analyst. Trace
  `REQ-UI-CONTRAST-ASSERTER-001` intended change reported to orchestrator
  (v0.1.0 tester-done → present/ship; v0.2.0 scope opened) — analyst does NOT
  edit trace.toml per close-out constraint. HANDOFF → architect.
- 2026-05-29 (architect): M-T1 design pass. Q-CONT-1 (a) WARN-default,
  Q-CONT-2 (a) hand-rolled formula, Q-CONT-3 (a) in-file OPT_OUTS all
  RATIFIED on the DURABLE Recommended path. D-CONT-1..D-CONT-7 locked.
  One-pass theme.rs audit enumerated 30 ModeColor constants → 83 PAIRS
  table entries → MIN_PAIRS = 60 floor. **H2 INVALIDATED**: 9 design-
  intent opt-outs (8 `FG_4` placeholder/disabled-tier + 1
  `BORDER_STRONG` decorative), not ≤ 3. K1 PARTIALLY TRIGGERED — opt-
  out list bounded + reviewable, not a route-back. **Two genuine
  WCAG-AA violations surfaced by dry-run**: `FG_ON_ACCENT on
  ACCENT_light = 3.52` + `WARN_500 on CANVAS_light = 2.96` (plus 4
  marginal sub-AA semantic ramp pairs). WARN mode keeps these from
  blocking CI; v0.2.0 promotion contract carries the upstream-hex-tune
  decision queue. Reference vectors validated to 4-decimal precision
  (`WHITE/BLACK = 21.0000`, `#777/#FFF = 4.4781`, `#888/#000 = 5.9240`).
  No new ADR; ADR-0048 § Changelog amended (one ride-along row).
  Frontmatter flipped status: draft → arch-done, owner: analyst →
  developer. HANDOFF → developer.
- 2026-05-29 (analyst): M0 brief authored under Pick B Wave 1
  promotion per [`pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/archive/2026-Q2/pick-b-cross-cutting-safety-duo-2026-05-29.md).
  R1 pair enumeration + R2 WCAG compute + R3 WARN-mode + R4
  opt-out marker + R-NR (7 clauses) + K1-K4 falsifiers + H1-H4
  hypotheses + Q-CONT-1/2/3 all bias DURABLE + pre-drawn 4-cell
  verdict tree. ~0.5 dev day + ~0.25 tester day estimate. Trace
  row `REQ-UI-CONTRAST-ASSERTER-001` opened at `proposed`.
  HANDOFF → architect (M-T1 fast-skip likely if Q-CONT-1/2/3 all
  Recommended durable; one-pass theme.rs audit + opt-out list
  seed + MIN_PAIRS floor ratification expected at M-T1).
