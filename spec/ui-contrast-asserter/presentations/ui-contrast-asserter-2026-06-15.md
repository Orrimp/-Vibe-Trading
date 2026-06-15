---
slug: ui-contrast-asserter
mode: release
date: 2026-06-15
agent: presenter
status: awaiting-approval
tester_report: spec/ui-contrast-asserter/reports/test-2026-06-15-1300-v0.2.0-ui-contrast-asserter.md
feature: spec/ui-contrast-asserter/feature.md
---

# Operator deck — ui-contrast-asserter — 2026-06-15 (close-out: v0.1.0 + v0.2.0)

## TL;DR

The cockpit's automated colour-legibility check is built and proven; this deck
flips it from "warns but lets bad colours through" to "**blocks any new
hard-to-read colour pairing in CI**" — with zero colour change and zero visual
churn.

## What changed

- **The asserter ships (v0.1.0).** A hand-rolled WCAG 2.1 contrast test
  (`crates/ui/tests/contrast.rs`) checks **83 real foreground/background colour
  pairs** from the cockpit's Lumen palette against the AA 4.5:1 legibility bar
  (and AAA 7:1 for the equity display). It was tester-PASSed on 2026-05-29 but
  never formally presented — this closes that loop. (WCAG = the published web
  accessibility contrast standard; "4.5:1" means the text is at least 4.5×
  brighter/darker than its background.)
- **The gate now enforces (v0.2.0).** The default flips from **warn-only** to
  **enforcing**: a colour pairing that drops below AA now **fails the build**
  instead of printing a warning that everyone ignores. The old behaviour stays
  reachable as a local-dev / CI-pin opt-out (`UI_CONTRAST_MODE=warn`).
- **6 known exceptions are documented, not hidden.** 6 existing pairs sit below
  AA (5 light-mode + 1 dark; worst is amber-warning-text on a light background
  at 2.96:1, which physically can't reach 4.5:1 without ceasing to look amber).
  Per your earlier decision (path A), these are ratified as **documented
  opt-outs with reason strings** — so the gate can go enforcing **today** for
  the other ~44 asserting pairs, with **zero colour edits and zero visual
  baseline rebaseline**.

## Why

Today the "every colour pairing must be legible" rule is enforced by human
eyeball at design review, and the cockpit palette grows every cycle. A future
hex tweak to a text colour can silently drop a pairing below the legible bar,
and nobody notices until the next live cockpit launch — days or weeks later.
The asserter closes that whole regression class in a pure test, without
rendering a pixel: new colour tokens auto-inherit the check. v0.1.0 proved the
check works (it ran in warn mode for the agreed 2-week observation window, which
has now elapsed); v0.2.0 gives it teeth so it actually blocks the regression it
was built to catch. Path A is the proportionate close-out: it makes the gate
enforcing in one cycle without spawning a colour-redesign sub-project, and parks
the 6 sub-AA colours as honest, reviewable, per-pair debt that a later dedicated
palette-tune can retire.

## What the operator can do now

This is test infrastructure, not a new cockpit button. The action enabled is
**the enforcing gate itself** — from now on CI blocks sub-AA colour pairings —
plus an optional independent re-run to confirm it on your machine.

1. **Confirm the gate enforces by default (optional, ~1 s after build).** Env
   unset = enforcing; expect PASS with the 6 documented exceptions logged as
   `opt-out:` audit lines.
   ```bash
   cargo test -p ui --test contrast --no-default-features --features live -- --nocapture
   ```
   Expected: `test result: ok. 7 passed; 0 failed; 2 ignored`, plus exactly 6
   `opt-out:` lines and zero failures.

2. **Confirm the warn escape hatch still works (optional).** This reproduces the
   old v0.1.0 behaviour for local dev / CI-pinning.
   ```bash
   UI_CONTRAST_MODE=warn cargo test -p ui --test contrast --no-default-features --features live -- --nocapture
   ```
   Expected: identical PASS; the 6 pairs are opt-out-classed so no WARN lines.

3. **Prove it actually blocks a regression yourself (optional, ~2 min).**
   Temporarily add a deliberately-bad pair to the `PAIRS` table in
   `crates/ui/tests/contrast.rs` (white-on-pale-grey), run the command in step 1,
   watch it **panic** (`= 1.25 < threshold 4.5`), then revert. This is the
   falsification probe the tester already ran — recipe in the feature file
   § Falsification probe P-CONT-1.

4. **Approve / route back** — see the Approval block at the bottom.

## Live demo

Real captured stdout from the **tester's** runs at commit `61ba42d` (the
presenter does not run cargo per the close-out constraint; these are the tester's
own captures). Full transcript saved at
`spec/ui-contrast-asserter/presentations/artifacts/ui-contrast-asserter-2026-06-15/gate-mode-run-and-falsification.txt`.

**(1) Gate-default run — env UNSET, so the gate is enforcing:**

```text
running 9 tests
test probe_low_contrast_rejects_in_gate_mode ... ignored
test probe_min_pairs_floor_fires_when_pairs_truncated ... ignored
test opt_outs_all_have_reasons ... ok
test pairs_table_meets_minimum_count ... ok
test ref_vector_777_on_fff_is_4_48 ... ok
test ref_vector_888_on_000_is_5_92 ... ok
test ref_vector_black_on_white_is_21 ... ok
test ref_vector_white_on_black_is_21 ... ok
test all_theme_pairs_meet_wcag ... ok

test result: ok. 7 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The 6 ratified exceptions surface as **audit lines, not failures**:

```text
opt-out: fg_3_on_panel_raised_dark; ratio: 3.75
opt-out: fg_on_accent_on_accent_light; ratio: 3.52
opt-out: up_500_on_canvas_light; ratio: 4.46
opt-out: down_500_on_canvas_light; ratio: 4.33
opt-out: warn_500_on_canvas_light; ratio: 2.96
opt-out: warn_500_on_panel_light; ratio: 3.11
```

Exactly 6 lines — no 7th asserting pair slipped below the bar.

**(2) Falsification probe — proof the gate actually enforces.** The tester
independently inserted a deliberately-bad pair (white on pale grey) and ran with
env unset:

```text
thread 'all_theme_pairs_meet_wcag' (58389668) panicked at crates/ui/tests/contrast.rs:967:9:
contrast assertion failed:
  probe_tester_white_on_pale_grey = 1.25 < threshold 4.5

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s
```

Gate panics, exit nonzero. Probe reverted → clean re-run `7 passed; 0 failed`.
This is the load-bearing evidence: the enforcing default does the one job it
exists for.

## Verification matrix

The feature pre-registers acceptance criteria (V2-AC.1..7) rather than a
`## Verification` V1..Vn block; the matrix below is built from those, each tied
to the tester's PASS evidence (commit `61ba42d`).

| # | Criterion | Status | Evidence (one line) |
|---|---|---|---|
| V2-AC.1 | Presenter deck exists; v0.1.0 baseline ships | VERIFIED | This deck; v0.1.0 was tester-PASS 2026-05-29 (7/7) and is shipped as-is by this close-out. |
| V2-AC.2 | Env unset → gate default → `contrast` test PASSES | VERIFIED | Live demo (1) above — 7/7 PASS, 0 failed; `current_mode()` default arm is `Mode::Gate` (contrast.rs:107-110). |
| V2-AC.3 | `UI_CONTRAST_MODE=warn` byte-identical to v0.1.0 (escape hatch preserved) | VERIFIED | Tester warn run: `7 passed; 0 failed; 2 ignored`; no WARN lines (the 6 are now opt-out-classed). |
| V2-AC.4 | A NEW sub-AA pair → gate-default → test FAILS (panic) | VERIFIED | Falsification probe (2) above — panic `1.25 < 4.5`, exit nonzero; reverted to clean PASS. |
| V2-AC.5 | `opt_outs_all_have_reasons` PASSES — every opt-out has a non-empty reason | VERIFIED | Tester gate run includes `opt_outs_all_have_reasons ... ok`; 15-entry `OPT_OUTS` manifest, all reason-stringed. |
| V2-AC.6 | Path A: zero production code change; zero PNG change; anchors byte-identical | VERIFIED | `git diff 61ba42d^ 61ba42d -- crates/ui/src/` empty; `-- '*.png'` empty; only test + spec files changed. |
| V2-AC.7 | `MIN_PAIRS = 60` floor unchanged; 83 entries stay (6 re-class is Body→OptOut, not removal) | VERIFIED | `pairs_table_meets_minimum_count ... ok`; PAIRS table length unchanged at 83. |
| — | Hand-rolled WCAG formula is correct (K4 mitigation) | VERIFIED | 4 reference-vector tests pass: WHITE/BLACK=21.00, #777/#FFF=4.4781, #888/#000=5.9240. |
| — | spec-lint at baseline, zero new (R-NR / V2-AC.7-adjacent) | VERIFIED | `spec-lint: FAIL (70 violations in 2 categories)` — matches audit-2026-06-15 baseline; 0 new. |

## Numbers that matter

| Metric | Value |
|---|---|
| Colour pairs enumerated | **83** (FG ladders × surface tiers, accent fills, semantic ramp, chart strokes, borders — dark + light) |
| Pairs actively asserting (Body/Equity, hard gate) | **~44** (50 Body/Equity rows minus the 6 ratified opt-outs) |
| Documented opt-outs (`OPT_OUTS` manifest) | **15** (8 disabled-text-tier + 1 border + **6 new sub-AA debt**) |
| Sub-AA exceptions ratified this cycle (path A) | **6** — worst is `warn_500_on_canvas_light` at **2.96:1** (amber-on-light, unfixable without abandoning amber) |
| Of the 6, trivially darkenable in a future palette-tune | **4** (`up_500`, `down_500`, `fg_3-dark`, `fg-on-accent`) — parked, **not done here** |
| Of the 6, hard / permanent debt | **2** (`warn_500` amber pairs) |
| Tests | **7 passed / 0 failed / 2 ignored** (the 2 ignored are documentation probes) |
| Falsification probe | bad pair → **panic at 1.25 < 4.5**, exit nonzero |
| Reference-vector formula checks | **4 / 4** match published WCAG 2.1 values to 4 decimals |
| Production code changed (path A) | **0 bytes** (`crates/ui/src/` diff empty) |
| Visual snapshot baselines changed | **0** (`*.png` diff empty) |
| Backtest anchors | **0 delta** — N/A (no strategy/exec/backtest/audit crate touched); 75/75 byte-identical |
| spec-lint | **70 findings, 0 new** (65 dead-link + 5 trace-broken-path; = audit-2026-06-15 baseline) |

## Open decisions

**One decision, and it is binary:** approve the close-out (ship v0.1.0 + the
v0.2.0 enforcing gate) or route it back.

- **The substantive sub-question is already settled.** The 6-pair disposition
  (V-CONT2-1) was decided **path A — ratify as documented opt-outs**: zero colour
  change, zero baseline churn. The code already implements path A; this deck
  ratifies the ship, it does not re-open the colour question.
- **What approving commits you to:** from now on, **any new sub-AA colour pairing
  in the cockpit fails CI** (the gate is enforcing by default). The escape hatch
  `UI_CONTRAST_MODE=warn` stays for local dev / CI-pinning. No colour ships, no
  trading behaviour changes, no follow-up cost — path A is test-file-only.
- **Honest scope cap (do not over-read this):** the 6 sub-AA colours are **not
  fixed** — they are documented debt. 4 of them are trivially darkenable to AA in
  a **future dedicated palette-tune feature** (which would touch production
  `theme.rs`, change rendered colours, and need a visual rebaseline + per-hex
  sign-off — explicitly out of scope here). The 2 amber pairs are likely
  permanent debt. Approving means "ship the enforcing gate with these 6 honestly
  parked", not "the cockpit is now fully AA-clean".
- **If you reject:** the most likely reason would be wanting path B (tune the 4
  easy pairs to real AA now, banking the wins) instead of parking them as debt.
  Say so in the reject line and it routes back to the analyst to re-scope V2-R3
  to path B (which adds a production colour change + visual rebaseline).

## Approval block

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

_Notes / reason:_

## Feedback log

_None yet._

---

## Closing verdict (presenter)

Mechanical gates run after writing this deck:

```text
PRESENTATION CHECK PASS  (spec/ui-contrast-asserter/presentations/ui-contrast-asserter-2026-06-15.md — approval block UN-ticked)
spec-lint: FAIL (70 violations in 2 categories)
```

The `spec-lint` line reads FAIL, but that is the **established pre-existing
baseline** (70 = 65 dead-link + 5 trace-broken-path), unchanged by this deck and
**equal to** the audit-2026-06-15 baseline. Zero new findings — the gate is
"≤ 70, zero new" and that holds. No structural regression introduced since the
tester's PASS.

> **Scope note on the tester report's orchestrator-correction block.** The
> v0.2.0 test report carries a top-of-file correction about two side-issues
> (a runner.rs clippy false-alarm and a flaky `charts_screen_dark` visual test).
> Both were verified **inaccurate as v0.2.0 problems** and spawned as separate
> out-of-scope follow-up (`task_23647c48`). They are **not** part of this ship
> and do not affect the v0.2.0 `VERDICT → PASS`.

**Intended trace change** (orchestrator to apply atomically with committing this
deck — presenter does NOT edit `spec/trace.toml`): row
`REQ-UI-CONTRAST-ASSERTER-001` `state` field `tester-done → presenter-done`.
