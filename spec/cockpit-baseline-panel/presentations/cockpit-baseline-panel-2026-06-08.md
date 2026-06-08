---
slug: cockpit-baseline-panel
mode: release
status: draft
owner: presenter
audience: human-operator
updated: 2026-06-08
generated: 2026-06-08T18:30:00Z
---

# Cockpit Baseline panel — the shipped passive baseline, now visible in the cockpit

> This deck closes **cockpit-baseline-panel** (v0.1.0, tester VERDICT → PASS +
> cockpit-smoke PASS, HEAD `580af5f`). It is your **first UI feature after the
> research program concluded** — and the build that connects the research output
> back to the operator tool. The program's headline deliverable, the passive
> buy-and-hold (BH) baseline, was characterized and operationalized but lived
> **only in markdown**. This feature gives it a navigable **Baseline** screen
> inside the cockpit, built entirely by reusing widgets that already existed and
> were already snapshot-tested. Every number below traces to a committed
> characterization, a snapshot file the tester locked, or a headless test I ran
> live this session — nothing is asserted without a source, and (capability
> boundary, see § Visual evidence) I did **not** boot the windowed cockpit; the
> render evidence is the textual panel-snapshots plus an operator hands-on recipe.

## TL;DR

The shipped passive buy-and-hold baseline is now a navigable **Baseline** screen
in the cockpit (Work sidebar, after Compare) — it draws the realized 2023/2024
equity curve + drawdown band + the 6-card KPI strip + a year toggle + an honest
bounded-scope caption, all by reusing existing cockpit widgets, with **zero new
crate edge, zero new widget, zero new theme token**.

## What changed

- **A new `Screen::Baseline`, navigable but not default-routed.** It lives in the
  **Work** sidebar group after Compare. The default cockpit screen stays `Live`
  (so the first-frame smoke gate stays deterministic regardless of whether the
  data CSVs are present in a checkout — architect decision D2). The screen
  composes, top-to-bottom: headline + year chips → honest caption → 6-card KPI
  strip → realized equity curve → drawdown band → a Sortino/Calmar caption line.
- **One new pure-`ui` module, `baseline/loader.rs`.** It reads the realized BH
  equity CSVs (`bh-equity-curve-{2023,2024}.csv`) into a `core::EquitySeries` —
  which gives the drawdown band for free — and it **never panics**: a missing or
  malformed CSV degrades to an Error state with a helpful "data isn't bundled in
  this build" message (the path the fixtures-only checkout hits). The six KPI
  scalars are **not** recomputed from the curve — they are embedded as a typed
  `const` sourced from the characterization §7.1 realized row, guarded by a
  re-sync test (architect decision D1; rationale in § Open decisions note 1).
- **The numbers the panel surfaces** (realized single-path, §7.1):
  **2023** — Sharpe **+1.8417** · total return **+196.22%** · Max DD **34.57%**;
  **2024** (the default view) — Sharpe **+0.8925** · total return **+91.04%** ·
  Max DD **48.95%**. Win rate and Trades render as `—` / `0` (buy-once-hold has
  no meaningful win rate — *not fabricated*).

## Why

The research program reached its terminal verdict — **active ≤ passive in the
reachable universe, this sample** — and the passive BH baseline became the thing
the operator actually runs. But that baseline was characterized only in a runbook
markdown file; the cockpit, which already ships `equity_curve` / `drawdown_band` /
`kpi_strip` widgets purpose-built for exactly this shape, surfaced none of it.
Research and tool were disconnected. This was the ui-designer's **#1-ranked,
lowest-risk** build-out candidate (operator-greenlit 2026-06-08, "build out the
cockpit/UI"): every render widget already existed and was snapshot-tested, so the
only genuinely new logic is a small CSV loader with a direct in-crate precedent
(`models/registry_read.rs`). It closes the research → cockpit loop. (Source:
[`feature.md`](../feature.md) "Why"; characterization
[`passive-baseline-characterization.md`](../../runbooks/artifacts/passive-baseline-2026-06-08/passive-baseline-characterization.md) §7.1.)

## What you can do now

| Action | Command |
|--------|---------|
| Open the cockpit and view the Baseline screen (full hands-on recipe in § Visual evidence) | `cargo run -p ui --bin cockpit --features fixtures` |
| Re-run the live render evidence I ran this session (the panel snapshots) | `cargo test -p ui --test panel_snapshots baseline_` |
| Re-run the Error-state + no-panic proof (the fixtures-checkout path) | `cargo test -p ui --test baseline_error_state` |
| Re-run the headless cockpit smoke (paints the Baseline route, no panic) | `cargo test -p ui --test headless_emulator_smoke` |
| Read the underlying data the panel shows | open [`passive-baseline-characterization.md`](../../runbooks/artifacts/passive-baseline-2026-06-08/passive-baseline-characterization.md) §7.1 |

## Live demo

I cannot boot the windowed cockpit from a sub-agent (capability boundary — see
§ Visual evidence), so the live ground-truth I ran this session is the **headless
render suite**: the deterministic equivalent of "the panel paints". These ran at
HEAD `580af5f`.

```
$ cargo test -p ui --test panel_snapshots baseline_
running 7 tests
test baseline_screen::baseline_caption_is_honest_bounded_no_overclaim ... ok
test baseline_screen::baseline_kpi_values_match_characterization_2024 ... ok
test baseline_screen::baseline_snapshot__ready_2023_toggled_dark ... ok
test baseline_screen::baseline_snapshot__error_dark ... ok
test baseline_screen::baseline_snapshot__ready_2024_dark ... ok
test baseline_screen::baseline_snapshot__ready_2024_light ... ok
test baseline_screen::baseline_snapshot__error_light ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 90 filtered out; finished in 0.30s

$ cargo test -p ui --test baseline_error_state
running 3 tests
test loader_missing_path_yields_error_both_years ... ok
test baseline_ready_state_renders_when_csvs_present ... ok
test baseline_error_state_renders_without_panic ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.94s

$ cargo test -p ui --test headless_emulator_smoke
running 2 tests
test headless_emulator_boots_cockpit_and_renders ... ok
test headless_emulator_paints_baseline_route ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.44s
```

`headless_emulator_paints_baseline_route` is the one that matters most for "does
it survive in the real shell": it boots the fixtures cockpit, navigates to
`Screen::Baseline`, boot-loads the curves through the **production**
`baseline::load_into` path, and asserts a non-empty 1280×720 rgba frame with no
panic. The two `baseline_screen::*_error_*` snapshots prove the fixtures-only
checkout (no CSVs) degrades to a helpful Error message rather than a blank or a
crash.

## Visual evidence

**Capability note — why there is no PNG screenshot in this deck.** Per the
AGENT.md sub-agent capability boundary, booting the live windowed cockpit
(`cargo run --bin cockpit` with a window) is **orchestrator-only**; I cannot open
a window or grab a real screenshot, and faking one is forbidden. So the rendered-
structure evidence here is the **committed textual panel-snapshots** the tester
locked (these capture exactly what the screen composes, in both themes), and below
them is a **hands-on recipe** so you can boot the cockpit and eyeball the live
panel yourself. This doubles as your first hands-on with the feature.

### Committed snapshot evidence (the rendered structure, both themes)

These five `.snap` files are the byte-stable rendered structure of the Baseline
screen, locked by `panel_snapshots::baseline_screen` (all 7 GREEN this session):

| Snapshot file | What it pins |
|---------------|--------------|
| [`…__ready_2024_dark.snap`](../../../crates/ui/tests/snapshots/panel_snapshots__baseline_screen__baseline_snapshot__ready_2024_dark.snap) | default 2024 view, dark theme |
| [`…__ready_2024_light.snap`](../../../crates/ui/tests/snapshots/panel_snapshots__baseline_screen__baseline_snapshot__ready_2024_light.snap) | default 2024 view, light theme |
| [`…__ready_2023_toggled_dark.snap`](../../../crates/ui/tests/snapshots/panel_snapshots__baseline_screen__baseline_snapshot__ready_2023_toggled_dark.snap) | after toggling to 2023 |
| [`…__error_dark.snap`](../../../crates/ui/tests/snapshots/panel_snapshots__baseline_screen__baseline_snapshot__error_dark.snap) | CSV-absent Error state, dark |
| [`…__error_light.snap`](../../../crates/ui/tests/snapshots/panel_snapshots__baseline_screen__baseline_snapshot__error_light.snap) | CSV-absent Error state, light |

The default 2024 dark snapshot renders verbatim as (this is the actual committed
`.snap` body — the operator can read exactly what the screen says):

```
screen: baseline
headline: Passive baseline
caption: Equal-weight buy-and-hold across 10 large-cap pairs, bought once at
  year-open and never rebalanced. Passive baseline; active ≤ passive in the
  reachable universe, this sample.
year_chips:
  [2023] inactive color=fg_muted
  [2024] active color=accent(dark) bg=panel_raised
kpi_strip:
  total_return: 91.04% color=pos
  cagr: 91.04%
  sharpe: 0.8925
  max_dd: −48.95% color=neg
  win_rate: —
  trades: 0
curve+band:
  state: ready points=367
  peak: 243941.06 trough: 86207.37 max_dd: 0.4181760895317619...
  curve_line=ACCENT curve_fill=UP_500
  band_line=DOWN_500 band_fill=DOWN_500
risk_detail: Sortino 2.51 / Calmar 5.68 (2023)  ·  Sortino 1.20 / Calmar 1.85 (2024)
```

Two things to notice for honesty's sake:

1. **The caption is the honest bounded form.** It says "passive baseline; active
   ≤ passive in the reachable universe, this sample" — **not** "passive is
   optimal" or "none can beat it". That phrasing is *binding* (R3/A3) and is
   enforced by a string-content test (`baseline_caption_is_honest_bounded_no_overclaim`,
   GREEN). This is a tool that surfaces a bounded finding, not a claim of
   universal optimality.
2. **Band-vs-card Max DD nuance (documented-expected, not a defect).** The KPI
   **card** shows Max DD `−48.95%` — the published §7.1 number, computed over the
   full 8,784 hourly bars. The drawdown **band** is drawn from the daily-sampled
   curve (~367 points), whose per-point trough comes out shallower
   (`max_dd: 0.4182` ≈ **41.8%** in the 2024 snapshot; `0.3331` ≈ 33.3% in the
   2023 snapshot vs the 34.57% card). The card is the number; the band is the
   shape. The loader deliberately does **not** reconcile them — recomputing from
   the daily curve would print a *different* number than you read in the
   characterization. You'll see this if you compare the band trough to the card;
   it is expected.

### Hands-on recipe — boot the cockpit and eyeball the Baseline panel

Self-contained operator verification (your first hands-on with the feature):

- **Command:**
  ```bash
  cargo run -p ui --bin cockpit --features fixtures
  ```
  (The standalone `cockpit` bin is fixtures-only — `--features fixtures` is
  required or the target won't compile.)
- **Steps:**
  1. Wait for the cockpit window to open (boots to the **Live** screen by
     design — Baseline is navigable, not the landing screen).
  2. In the left **Work** sidebar group, click **Baseline** (it sits **after
     Compare**).
  3. Confirm the screen shows, top to bottom: the **"Passive baseline"**
     headline + two year chips `[2023] [2024]` (2024 active), the honest caption,
     the 6-card KPI strip (Total return / CAGR / Sharpe / Max DD / Win rate /
     Trades), the realized equity **curve** (accent line, green-up fill), the
     **drawdown band** below it, and the Sortino/Calmar detail line.
  4. Click the **`2023`** chip → the curve, band, and the six KPI values should
     swap (Total return → **196.22%**, Sharpe → **1.8417**, Max DD → **−34.57%**);
     the caption + Sortino/Calmar line stay the same (they're year-agnostic).
  5. Toggle the theme (the cockpit's dark/light control) → the whole screen
     should re-render correctly in the other theme.
- **Timing:** ~10–60 s for a debug `cargo run` first build (longer on a cold
  target); the screen itself paints instantly once the window is up.
- **Expected result:** the Baseline screen renders the BH equity curve + KPI
  strip + honest caption in **both** themes; the year toggle swaps curve+metrics;
  no blank panel, no crash. The 2024 KPI strip reads
  `91.04% / 91.04% / 0.8925 / −48.95% / — / 0`.
- **Failure diagnosis:**
  - *Window opens but Baseline shows an Error message ("data isn't bundled…")* →
    the realized-curve CSVs aren't present at
    `spec/runbooks/artifacts/passive-baseline-2026-06-08/bh-equity-curve-{2023,2024}.csv`.
    They are committed at HEAD `580af5f` (I confirmed both files present, ~12 KB
    each), so this should not happen on a full checkout — but the Error path is
    *intentional* and is exactly what a minimal checkout would show. The KPI strip
    stays populated from the embedded const even in this state (honest degrade).
  - *Target won't compile, "requires the features: fixtures"* → you dropped the
    `--features fixtures` flag; re-add it.
  - *No "Baseline" entry in the sidebar* → you're not on HEAD `580af5f`; rebuild.
- **Cleanup:** close the cockpit window (Cmd-Q). Nothing is written to disk;
  the panel is read-only over committed data.

## Verification

The feature's acceptance criteria are AC1–AC7 (the feature.md `## Acceptance
criteria` section serves as the verification matrix). Each is mapped to its
passing test from the tester report
([`test-2026-06-08-cockpit-baseline-panel.md`](../reports/test-2026-06-08-cockpit-baseline-panel.md)
§8).

| AC | Description | Status | Evidence |
|----|-------------|--------|----------|
| AC1 | Baseline renders the BH result; 2024 default; toggle→2023 swaps curve+metrics | VERIFIED | `baseline_snapshot__ready_2024_{dark,light}`, `…ready_2023_toggled_dark`, `baseline_kpi_values_match_characterization_2024`, `committed_csvs_load_to_ready_first_point_100k` (first point $100k both years) — all PASS (ran live, 7/7) |
| AC2 | Four panel states behave (Loading / Ready / Error / Empty) | VERIFIED | `baseline_error_state_renders_without_panic` (Error, both themes, no panic), `baseline_snapshot__error_{dark,light}`, `header_only_path_yields_empty_state` (Empty), `default_is_y2024_curves_loading_metrics_ready` (Loading) — PASS |
| AC3 | Fixtures cockpit smoke passes — first-frame Baseline route, no panic | VERIFIED | `headless_emulator_paints_baseline_route` (1280×720, non-empty rgba, no panic — ran live); the orchestrator cockpit-smoke gate (see § Numbers, cited log) |
| AC4 | Lumen-consistent — consistency/contrast/layout green; no hardcoded colors/strings; both themes | VERIFIED | `consistency` (2 PASS), `contrast` (7 PASS), `layout_invariants` (11 PASS), zero new clippy warnings from feature files (tester § 2) |
| AC5 | Honest caption: states bounded finding; no "optimal"/"unbeatable"/"none beat it" | VERIFIED | `baseline_caption_is_honest_bounded_no_overclaim` — asserts the bounded phrase present, overclaim terms absent (ran live, PASS) |
| AC6 | Panel-snapshot both themes + sidebar flatten-invariant updated | VERIFIED | `baseline_snapshot__ready/error_{dark,light}`, `theme::tests::sidebar_groups_phase_c__flatten_matches_phase_a` (PASS) |
| AC7 | No new crate edge, no new widget, no new theme token | VERIFIED | `git diff crates/ui/Cargo.toml` empty (tester § 8); `baseline/` is pure-`ui` over `std::fs` + `trading_core`; consistency scan finds zero new hex / inline strings; no new widget file |

**D1 re-sync guard:** `baseline_metrics_match_characterization` (PASS) asserts all
six embedded §7.1 scalars equal the documented values — so a silent edit to the
const trips a test. This is the safety net for the one cost of the embed approach
(see § Open decisions note 1).

## Numbers that matter

- **Tests:** UI suite **428** lib unit tests GREEN (0 fail) + all integration
  suites GREEN. The Baseline-specific tests: **13** loader unit + **3** state unit
  + **3** error-state integration + **7** panel-snapshots + **2** headless-emulator
  = **28** feature tests, all PASS. (Tester report § 3.)
- **The one non-green test in the suite is pre-existing and does not gate PASS:**
  `lab_run_engine::inner::h3_in_memory_equals_cached_disk` — introduced in
  `a5f8647` (`v5-latency-slippage-sim`), confirmed by the tester to fail
  identically on the parent commit. Unrelated to this feature.
- **Anchors:** **119 / 119 PASS** (`verify-anchors`). No anchored file was
  touched — this is read-only UI over **non-anchored** runbook CSVs.
- **cockpit-smoke gate (orchestrator pre-tick):** **PASS — 0 panics**, 8-second
  render window. (Log:
  [`cockpit-smoke-2026-06-08T17-47Z.log`](../reports/cockpit-smoke-2026-06-08T17-47Z.log)
  — the only line is the pre-existing deprecated-`Screen::Home` warning, then
  `Finished` + `Running target/debug/cockpit`; no panic, clean exit.)
- **spec-lint:** **94 violations in 2 categories** at HEAD `580af5f` (87 dead-link
  + 7 trace-broken-path) — **exactly the documented baseline** (no regression; see
  § Open decisions note 2 for why the tester saw 95).
- **New crate edges / widgets / theme tokens:** **0 / 0 / 0** (AC7).
- **The data, in one line:** 2023 +196.22% (Sharpe 1.84, MaxDD 34.6%); 2024
  +91.04% (Sharpe 0.89, MaxDD 49.0%). Both start at $100,000.00; 2024 ends
  $191,040.25, 2023 ends $296,221.10.

## Open decisions

_No decision is required to ship — this is a clean release-mode PASS. Two design
choices are surfaced below so they're visible, not buried; neither blocks
approval, and neither commits you to a follow-up cost today._

1. **The six KPI scalars are an embedded `const`, not computed from the curve
   (architect D1=c) — carries one standing maintenance cost.** Sharpe/CAGR/total-
   return math does not exist in `crates/core` (it lives only in `backtest` /
   `forecast`), and the published §7.1 Sharpe was computed over **hourly** bars
   while the committed curve CSV is **daily-sampled** — so recomputing in-`ui`
   would either need a new `core` math module (or an AC7-violating `backtest`
   reach) **and** would print a *different* number than the characterization.
   Embedding the realized §7.1 scalars is therefore correct on the merits, not a
   shortcut. **The cost:** if the characterization is ever re-run with different
   numbers, the embedded const goes stale. **Mitigation already in place:** a
   `// RE-SYNC:` doc-comment block + the `baseline_metrics_match_characterization`
   test (which goes RED on any drift) + a runbook note. *No action needed now* —
   just be aware that a future re-characterization makes the failing re-sync test
   your trigger to update the const.

2. **spec-lint reconciliation — the tester saw 95, HEAD shows the 94 baseline (no
   regression).** The tester ran at commit `f1c1bf3` and reported **95** (one new
   `trace-broken-path` from this feature's trace row using the shorthand
   `"ADR-0030"` instead of the full path). That was corrected before HEAD; I
   re-ran spec-lint at `580af5f` this session and it reports **94/2-cat**, which
   matches the audit-2026-06-08 baseline exactly. **Nothing for you to do** — the
   regression the tester flagged was fixed in-flight; I am noting it only so the
   95-vs-94 discrepancy between the tester report and this deck is explained, not
   a surprise.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_empty until operator fills_

## Changelog

- 2026-06-08 (presenter): initial release-mode deck for cockpit-baseline-panel
  v0.1.0 (tester VERDICT → PASS + cockpit-smoke PASS, HEAD `580af5f`). Live
  evidence run this session: baseline panel-snapshots 7/7, error-state 3/3,
  headless-emulator 2/2 — all GREEN. spec-lint re-confirmed at 94/2-cat baseline
  (no regression; tester's transient 95 corrected in-flight). Visual evidence is
  the committed textual panel-snapshots (sub-agent cannot boot the windowed
  cockpit) + a 6-section hands-on recipe for the operator's first eyeball.
  Approval block ships UN-ticked.
