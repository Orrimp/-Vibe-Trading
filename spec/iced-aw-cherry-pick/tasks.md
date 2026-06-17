---
slug: iced-aw-cherry-pick
status: shipped
owner: ui-designer
updated: 2026-05-13
version: 0.1.1
---

# Tasks — iced_aw cherry-pick (Brief B)

> **Status:** architect design pass complete
> ([`feature.md ## Design — architect synthesis`](feature.md#design--architect-synthesis)
> 2026-05-13). T-tasks below are concrete, file:line scoped, and ready
> for the fan-out (3 parallel lanes per Q7 resolution: M1 = B1
> date_picker, M2 = B2 spinner, M3 = B3 badge).
>
> Honest-tick discipline
> ([`AGENT.md ## Process discipline`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)
> rule 1): developer + ui-designer MUST NOT tick `[x]` without citing
> (a) file:line of change, (b) test command, (c) test-output line.
> Tester (test-runner + evaluator split per
> [`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split))
> owns the M_FINAL_* ticks.
>
> Anchor risk: **zero** (Brief B touches zero strategy / audit / exec
> / backtest paths). PNG-baseline diff: **zero** (Charts surface
> untouched).

## M0 — Architect design pass + falsifier batch

Architect-decided + resolved on 2026-05-13. All 7 analyst open
questions resolved and the 3 hypotheses landed verdicts inline via
`cargo tree` / `cargo doc` / `grep` of the iced_aw 0.14.1 source in
the cargo registry. Temporary `crates/ui/Cargo.toml` edit was added
to drive the falsifiers and reverted before this tasks.md was
written.

- [x] **T-M0-Q1** — Q1 resolved: **inline `iced_aw` imports at call
  sites** (no re-export shim). Rationale: only 3 call sites, no
  `crates/ui/src/widgets/aw.rs` module ceremony.
  Cited at [`feature.md ## Q1 — Re-export shape`](feature.md#q1--re-export-shape-inline-iced_aw-types-at-call-sites).
- [x] **T-M0-Q2** — Q2 resolved: **Catalog adapter colocates in
  `crates/ui/src/theme/iced_widget_catalogs.rs`** alongside the
  existing `cockpit_table_style_fn`. Module docstring at
  [`iced_widget_catalogs.rs:33-43`](../../crates/ui/src/theme/iced_widget_catalogs.rs)
  explicitly designates this slot.
  Cited at [`feature.md ## Q2`](feature.md#q2--catalog-adapter-colocation-confirmed-in-iced_widget_catalogsrs).
- [x] **T-M0-Q3** — Q3 resolved: **payload type is `iced_aw::core::date::Date`**
  (also re-exported as `iced_aw::date_picker::Date`). Bidirectional
  `From`/`Into` with `chrono::NaiveDate` at
  `~/.cargo/registry/.../iced_aw-0.14.1/src/core/date.rs:49-60`.
  **Determinism trap:** `Date::today()` (`:31-34`) and `State::reset()`
  (`:173-175`) call `Local::now()` — B1 MUST construct the picker
  with a const date and MUST NOT call those in test paths.
  Cited at [`feature.md ## Q3`](feature.md#q3--iced_awdate_picker-message-payload-iced_awcoredatedate).
- [x] **T-M0-Q4** — Q4 resolved: **8 call sites in 5 files**
  (analyst's "~8 panels" confirmed). **Architectural divergence:** NO
  shared `panel_state` dispatch helper — `muted_body(text)` is a
  per-panel call with informational text. Design call: introduce
  `loading_with_spinner(text, mode)` helper in `frame.rs`, preserving
  the per-panel text alongside a 16 px spinner.
  Cited at [`feature.md ## Q4`](feature.md#q4--exact-panel-set-for-b2-8-call-sites-across-5-files-analysts-8-panels-confirmed).
- [x] **T-M0-Q5** — Q5 resolved: **1 file only —
  `crates/ui/src/widgets/strategies.rs`** (analyst's "3 files" wrong;
  Risk screen has no chip surface; no hand-rolled
  `container(...).style(badge_style)` exists). B3 ships with the
  status column (strategies.rs:113-129) upgraded to `iced_aw::Badge`.
  Cited at [`feature.md ## Q5`](feature.md#q5--exact-file-set-for-b3-cratesuisrcwidgetsstrategiesrs-only-1-file-not-3).
- [x] **T-M0-Q6** — Q6 resolved: **+3 transitive crates**
  (`chrono v0.4.44`, `num-traits v0.2.19`, `once_cell v1.21.4`) —
  `chrono` is the only definite new dep; the others likely dedup with
  existing workspace deps.
  Cited at [`feature.md ## Q6`](feature.md#q6--transitive-dep-audit-3-crates-chrono-num-traits-once_cell).
- [x] **T-M0-Q7** — Q7 resolved: **3 parallel lanes ticked as
  M1/M2/M3** per Brief A's precedent. Zero shared files, zero shared
  `Message` enum variants outside cockpit surface.
  Cited at [`feature.md ## Q7`](feature.md#q7--milestone-shape-three-parallel-lanes-under-one-logical-milestone-ticked-as-m1m2m3-for-spec-tracker-clarity).
- [x] **T-M0-H4** *(architect, 2026-05-13)* — H-arch-4 falsifier ran.
  `cargo tree -p ui -i iced_aw --no-default-features` (after temp
  Cargo.toml add of `iced_aw = "0.14"` w/ `["date_picker"]` only).
  _Finding:_ zero pulls from `menu` / `tab_bar` / `badge` / `spinner`
  / `number_input`; transitive set is exactly
  `chrono + num-traits + once_cell`. License MIT, edition 2024.
  **H-arch-4 RESOLVED-PASS** — Brief B cost estimate correct.
  Cited at [`feature.md ## H-arch-4`](feature.md#h-arch-4--iced_awdate_picker-feature-flag-isolation--resolved-pass).
- [x] **T-M0-H9** *(architect, 2026-05-13)* — H-arch-9 falsifier ran.
  `grep -n "Instant::now\|SystemTime::now\|Local::now\|elapsed\|\.now("
  ~/.cargo/registry/.../iced_aw-0.14.1/src/widget/spinner.rs`
  → single hit at line 160 (`last_update: Instant::now()` in
  `fn state()`). Render path (`draw`, `:121-152`) is pure; animation
  path reads `Event::Window(RedrawRequested(now))` (iced-frame-time,
  not wall-clock) at `:180-201`. **H-arch-9 RESOLVED-PASS with
  caveat** — `*_loading.snap` baselines render at `t=0.0` since
  `iced_test` doesn't fire `RedrawRequested`. Caveat: developer must
  verify `scripts/check_no_clocks_in_ui_tests.sh` scope does NOT
  include `~/.cargo/registry/` before ticking M_FINAL.
  Cited at [`feature.md ## H-arch-9`](feature.md#h-arch-9--iced_awspinner-deterministic-render--resolved-pass-with-caveat).
- [x] **T-M0-H10** *(architect, 2026-05-13)* — H-arch-10 falsifier ran.
  Read `~/.cargo/registry/.../iced_aw-0.14.1/src/style/badge.rs:27-58`
  + `src/widget/badge.rs:115-125`. Catalog trait + `StyleFn<'a, Self,
  Style>` + `.style(impl Fn(&Theme, Status) -> Style)` builder all
  present. Structurally identical to Brief A's `cockpit_table_style_fn`
  shape. **H-arch-10 RESOLVED-PASS** — `cockpit_badge_style_fn`
  factory is the canonical wire-in.
  Cited at [`feature.md ## H-arch-10`](feature.md#h-arch-10--iced_awbadge-catalog--stylefn-hook--resolved-pass).
- [x] **T-M0-CARGOREVERT** *(architect, 2026-05-13)* — Temporary
  `iced_aw = "0.14"` Cargo.toml edit reverted before architect
  handoff. Verified by `grep -n "iced_aw"
  /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/crates/ui/Cargo.toml`
  → zero hits. Developer pass owns the real Cargo.toml landing in
  T-M1-1 below.

## M1 — B1 `iced_aw::date_picker` (developer, single lane)

**Target:** [`crates/ui/src/bin/viewer.rs`](../../crates/ui/src/bin/viewer.rs)
+ `crates/ui/Cargo.toml`. Goal: introduce `iced_aw` dep and add the
date-picker primitive as a smoke-test consumer in the viewer bin.
Scope: picker primitive only — full backtest-range wire-in is v1.11
/ Phase 4 (out-of-scope per analyst's brief).

REQ trace: **REQ-ICED-AW-001**.

- [x] **T-M1-1** *(developer, 2026-05-13)* — Add `iced_aw` dep to
  `crates/ui/Cargo.toml` with `default-features = false` and exactly
  the 3 features needed for B1+B2+B3.
  _Acceptance citations (honest-tick rule):_
  - File:line of change: `crates/ui/Cargo.toml:78` (the `iced_aw = {
    version = "0.14", default-features = false, features = ["date_picker",
    "spinner", "badge"] }` stanza), placed under the `iced` line per
    the architect's design.
  - Test command: `cargo build -p ui --tests`.
  - Test output line: `Finished \`dev\` profile [unoptimized + debuginfo]
    target(s) in 51.16s` (first compile pulling iced_aw + transitive
    `chrono + num-traits + once_cell` — matches H-arch-4 estimate).
    Zero new compile warnings on touched files.
  - Trace row satisfied: REQ-ICED-AW-001 + REQ-ICED-AW-002 +
    REQ-ICED-AW-003 (one stanza serves all three).
- [x] **T-M1-2** *(developer, 2026-05-13)* — Add the date-picker
  primitive instantiation site to the viewer bin.
  _Acceptance citations:_
  - File:line of change: `crates/ui/src/bin/viewer.rs` `fn picker_block`
    (lines 277-353 post-fmt); viewer model fields landed at
    `crates/ui/src/viewer.rs:VIEWER_PICKER_ANCHOR` (`(2024, 1, 1)`)
    + `picker_open` / `picked_date` fields on `ViewerModel`.
  - Constructor: `DatePicker::new(model.picker_open, PickerDate::from_ymd(2024, 1, 1), underlay, ViewerMessage::PickerCanceled, |picked| ViewerMessage::PickerDateSelected(...))`
    — uses the const `VIEWER_PICKER_ANCHOR` via `Date::from_ymd`, **never**
    `Date::today()` or `State::reset()`. Verified by `grep -n
    'Date::today\|State::reset' crates/ui/src/bin/viewer.rs
    crates/ui/src/viewer.rs` → zero hits.
  - Date payload conversion: `time::Date::from_calendar_date(picked.year,
    time::Month::try_from(picked.month as u8).unwrap_or(time::Month::January),
    picked.day as u8)` inside the `on_submit` lambda; `chrono::NaiveDate`
    is NOT introduced.
  - Test command (a): `cargo build -p ui --bin viewer`.
  - Test output line (a): `Finished \`dev\` profile [unoptimized +
    debuginfo] target(s) in 5.94s` (compile-clean).
  - Test command (b): `cargo test -p ui --lib viewer`.
  - Test output line (b): `test viewer::tests::viewer_picker_round_trip_open_cancel_submit
    ... ok` + `test viewer::tests::viewer_picker_anchor_is_a_valid_calendar_date
    ... ok` + `test result: ok. 2 passed; 0 failed`.
  - Trace row satisfied: REQ-ICED-AW-001.
- [x] **T-M1-3** *(developer, 2026-05-13)* — Add a panel snapshot
  fixture that captures the picker in its default-closed state.
  _Acceptance citations:_
  - File:line of change: new test fn `viewer_picker_default_closed`
    at `crates/ui/tests/panel_snapshots.rs:1177-1228` (post-fmt; in
    the Phase 4 Viewer section).
  - Snapshot file: `crates/ui/tests/snapshots/panel_snapshots__viewer_picker_default_closed.snap`
    landed via `cargo insta accept --workspace` (manually reviewed —
    captures anchor 2024-01-01, picker_open=false, underlay text,
    message round-trip).
  - Two-run determinism gate: `cargo test -p ui --test panel_snapshots
    viewer_picker_default_closed` run twice; `find crates/ui -name
    "*.snap.new"` returned zero files after each run.
  - Test command: `cargo test -p ui --test panel_snapshots viewer_picker_default_closed`.
  - Test output line (both runs): `test viewer_picker_default_closed
    ... ok` + `test result: ok. 1 passed; 0 failed; 0 ignored; 0
    measured; 68 filtered out; finished in 0.30s`.
  - Trace row satisfied: REQ-ICED-AW-001.
- [x] **T-M1-4** *(developer, 2026-05-13)* — Module doc on the viewer
  bin's new picker block referencing the Q3 Determinism trap for
  future readers.
  _Acceptance citations:_
  - File:line of change: rustdoc comment block at
    `crates/ui/src/bin/viewer.rs:267-296` (the `fn picker_block`
    docstring), mentioning the `Date::today()` / `State::reset()`
    no-go zones, the `time::Date` conversion shim, and the const
    anchor contract. Mirrored in `crates/ui/src/viewer.rs` on the
    `VIEWER_PICKER_ANCHOR` const docstring (lines 19-29).
  - Test command: `cargo doc -p ui --no-deps`.
  - Test output line: `Finished \`dev\` profile [unoptimized +
    debuginfo] target(s) in 4.17s` with **6 rustdoc warnings** — all
    pre-existing (none introduced by this pass; an initial multiple-
    anchors warning on the new docstring was fixed by relinking via
    the explicit `[label] (path)` form).
  - Trace row satisfied: REQ-ICED-AW-001.

## M2 — B2 `iced_aw::spinner` (developer, single lane)

**Target:** [`crates/ui/src/widgets/frame.rs`](../../crates/ui/src/widgets/frame.rs)
+ 8 call sites in 5 files. Goal: add a new `loading_with_spinner(text,
mode)` helper that pairs a 16 px `iced_aw::Spinner` with the existing
informational `Loading…`-style text, then swap all 8 `muted_body(X_LOADING)`
call sites to use it.

REQ trace: **REQ-ICED-AW-002**.

- [x] **T-M2-1** *(developer, 2026-05-13)* — Add the
  `loading_with_spinner` helper to `frame.rs`.
  _Acceptance criteria:_
  - File:line of change: new pub fn in
    `crates/ui/src/widgets/frame.rs` immediately below the existing
    `muted_body` (currently at `frame.rs:140-147`).
  - Shape (architect-spec'd):
    ```rust
    #[must_use]
    pub fn loading_with_spinner<'a, Message: 'a>(text: &'a str, mode: ThemeMode) -> Element<'a, Message> {
        Row::new()
            .spacing(space::S)
            .align_y(Vertical::Center)
            .push(iced_aw::Spinner::new()
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0)))
            .push(Text::new(text)
                .size(text::BODY)
                .color(color::FG_3.current(mode)))
            .into()
    }
    ```
  - Import: `use iced_aw::Spinner;` (inline import per Q1 — no shim).
  - Test command: `cargo build -p ui`.
  - Test output line: `Finished \`dev\` profile [unoptimized +
    debuginfo] target(s) in 34.62s`.
  - _Honest-tick citations:_
    - File:line of change — `crates/ui/src/widgets/frame.rs:150-188`
      (post-fmt) for the new `pub fn loading_with_spinner(text, mode)`
      helper; matches the architect's shape (`Row` of `[Spinner,
      Text]`, `space::S` spacing, `Vertical::Center`, 16 px fixed).
      Plus `crates/ui/src/widgets/frame.rs:467-491` for the new
      unit test `loading_with_spinner_uses_fg_3_text_color`.
    - Test command: `cargo test -p ui --lib widgets::frame`.
    - Test output line: `test
      widgets::frame::tests::loading_with_spinner_uses_fg_3_text_color
      ... ok` + `test result: ok. 5 passed; 0 failed; 0 ignored`.
    - **Note (color token):** the helper uses `color::FG_3.current(mode)`
      directly rather than a `color::SPINNER_TINT` alias. The
      ui-designer note below mentions a `SPINNER_TINT = FG_3` token —
      `grep -n "SPINNER_TINT" crates/ui/src/theme.rs` returns zero
      hits as of this developer pass, so the alias was not landed in
      the UI-designer pass that the developer observed. The token's
      _value_ is `FG_3`, which **is** what the helper renders;
      switching from `FG_3` to a `SPINNER_TINT` alias is a one-line
      follow-up (no behavior change) once the alias lands.
  - **ui-designer landed (2026-05-13)**: `SPINNER_TINT = FG_3` token
    landed at `crates/ui/src/theme.rs` `color::SPINNER_TINT`
    (constant alias for `FG_3`). Rationale: pins the spinner's tint
    to the same muted step `muted_body` already uses for the
    paired `Loading…` text, so the row reads as one quiet "we
    are waiting" surface rather than fighting between an
    accent-coloured indicator and a muted body. `ACCENT` would
    falsely imply "active in-flight signal"; `UP_500` would falsely
    imply "positive result". Documented in the new
    `spec/ui-design-principles.md ## Status pill colors`
    subsection's preamble + token comment in `theme.rs`. The
    developer's `loading_with_spinner` helper SHOULD import
    `color::SPINNER_TINT` (rather than `color::FG_3`) to express
    intent and to insulate against future ramp re-shading.
  - Trace row satisfied: REQ-ICED-AW-002.
- [x] **T-M2-2** *(developer, 2026-05-13)* — Swap
  all 8 `muted_body(X_LOADING)` call sites to `loading_with_spinner(X_LOADING, mode)`.
  _Acceptance criteria:_
  - File:line of change: the exact 8 sites, each updated to pass
    `mode` (`ThemeMode::Dark` or the caller's existing mode binding):
    1. `crates/ui/src/screens/strategies.rs:55`
    2. `crates/ui/src/screens/strategies.rs:245`
    3. `crates/ui/src/screens/audit.rs:55`
    4. `crates/ui/src/screens/risk.rs:45`
    5. `crates/ui/src/widgets/positions.rs:55`
    6. `crates/ui/src/widgets/strategies.rs:46`
    7. `crates/ui/src/widgets/pnl.rs:22`
    8. `crates/ui/src/widgets/agent_feed.rs:40`
  - For each call site, verify the caller has a `mode: ThemeMode` or
    `mode: ThemeMode::Dark` binding in scope. If not, thread `mode`
    from the caller's `&Cockpit` / view signature. (Spot check during
    M2: `positions.rs:55` is inside `ready_body(...)` which already
    takes `mode` per Brief A.)
  - **`muted_body` is NOT deleted** — other call sites (`POS_EMPTY`,
    `TAPE_EMPTY`, error states) keep using it. Only the LOADING-state
    callers swap. _Verified by `grep -n "muted_body(" crates/ui/src
    | wc -l` returning ≥8 surviving call sites for non-loading copy._
  - Test command: `cargo build -p ui`.
  - Test output line: `Finished \`dev\` profile [unoptimized +
    debuginfo] target(s) in 2.98s` + zero new warnings on the 5
    touched files.
  - _Honest-tick file:line citations for all 8 sites (post-fmt; line
    numbers may shift slightly from architect's estimates due to
    surrounding edits / cargo fmt re-wrap):_
    1. `crates/ui/src/screens/strategies.rs:55` (post-edit
       `loading_with_spinner(STRATEGIES_LOADING, mode)`).
    2. `crates/ui/src/screens/strategies.rs:244-247` (post-edit
       `loading_with_spinner(STRATEGIES_SPARKLINE_LOADING, mode)`).
    3. `crates/ui/src/screens/audit.rs:57` (uses outer `mode` param).
    4. `crates/ui/src/screens/risk.rs:45` (uses outer `mode` param).
    5. `crates/ui/src/widgets/positions.rs:55` (passes `ThemeMode::Dark`
       — widget-level view fn does not take `mode` yet).
    6. `crates/ui/src/widgets/strategies.rs:48` (passes `ThemeMode::Dark`).
    7. `crates/ui/src/widgets/pnl.rs:22` (passes `ThemeMode::Dark`).
    8. `crates/ui/src/widgets/agent_feed.rs:40` (passes `ThemeMode::Dark`).
  - Trace row satisfied: REQ-ICED-AW-002.
- [x] **T-M2-3** *(developer, 2026-05-13)* — Refresh affected
  `*_loading.snap` baselines (architect-revised: zero refresh
  needed — see note).
  _Acceptance citations:_
  - **Architectural divergence from the architect's pre-pass
    estimate:** the existing `*_loading.snap` baselines under
    `crates/ui/tests/snapshots/` are produced by the `panel_snapshots`
    integration test's **text-summary helpers** (`tape_summary`,
    `positions_summary`, etc. at `crates/ui/tests/panel_snapshots.rs:1779-1808`
    + neighbours), which render a `PanelState`-keyed copy string —
    NOT the actual iced widget tree. The swap from `muted_body(text)`
    to `loading_with_spinner(text, mode)` lives entirely in the
    widget render path; the text-summary helpers route via
    `strings::*_LOADING` regardless of which iced widget wraps it.
    **Zero existing snapshots changed bytes.** Confirmed by
    `cargo test -p ui --test panel_snapshots loading` → 6 passes /
    0 diffs (`agent_feed_audit_modal_loading`, `agent_feed_loading`,
    `human_control__limits_loading`, `pnl_loading`,
    `positions_loading`, `strategies_loading`) + `find crates/ui
    -name "*.snap.new"` → empty.
  - Two-run determinism gate (per H-arch-9 RESOLVED-PASS caveat):
    `cargo test -p ui --test panel_snapshots loading` run twice;
    zero `*.snap.new` files between runs. Both runs identical.
  - Test command (run 1 + run 2): `cargo test -p ui --test
    panel_snapshots loading`.
  - Test output line (both): `test result: ok. 6 passed; 0 failed;
    0 ignored; 0 measured; 63 filtered out; finished in 0.30s`.
  - Trace row satisfied: REQ-ICED-AW-002.
- [x] **T-M2-4** *(developer, 2026-05-13)* — Verify
  `scripts/check_no_clocks_in_ui_tests.sh` scope does NOT include
  `~/.cargo/registry/` (per H-arch-9 caveat).
  _Acceptance citations:_
  - File:line read (no edit needed): `scripts/check_no_clocks_in_ui_tests.sh:33-42`
    — the `WATCHLIST=(...)` array enumerates exactly 8 workspace-
    scoped files under `crates/ui/...`, NO `~/.cargo/registry/` path.
    The script is grep-against-WATCHLIST, not a tree walk, so it
    structurally cannot reach the iced_aw-0.14.1 spinner source.
  - Test command: `bash scripts/check_no_clocks_in_ui_tests.sh`.
  - Test output line: `CLOCKS PASS  (8 files / 4 patterns)`.
  - The script does NOT flag `iced_aw-0.14.1/src/widget/spinner.rs:160`.
    Caveat resolved; no escalation needed.
  - Trace row satisfied: REQ-ICED-AW-002 (acceptance criterion 3).

## M3 — B3 `iced_aw::badge` (developer + ui-designer, single lane)

**Target:** [`crates/ui/src/widgets/strategies.rs`](../../crates/ui/src/widgets/strategies.rs)
column 3 (STATUS) +
[`crates/ui/src/theme/iced_widget_catalogs.rs`](../../crates/ui/src/theme/iced_widget_catalogs.rs).
Goal: introduce a `cockpit_badge_style_fn()` parallel to the existing
`cockpit_table_style_fn`, then upgrade the strategy STATUS column from
`colored_cell(status_label, status_color)` to
`iced_aw::Badge::new(...)::style(cockpit_badge_style_fn())`.

REQ trace: **REQ-ICED-AW-003**.

- [x] **T-M3-1** *(developer + ui-designer, ~30 LOC incl. 2 unit tests)*
  — Add `cockpit_badge_style` + `cockpit_badge_style_fn` to
  `iced_widget_catalogs.rs`.
  _Acceptance criteria:_
  - File:line of change: new pair of functions appended to
    `crates/ui/src/theme/iced_widget_catalogs.rs` (parallel to the
    existing `cockpit_table_style` + `cockpit_table_style_fn` at
    `:79-97`).
  - Shape **(refined by ui-designer pass 2026-05-13 — see Visual
    review note in handoff):** factory accepts a domain
    `BadgeIntent::{Positive, Neutral, Negative}` parameter and bakes
    it into the returned closure. The architect's no-parameter stub
    `fn cockpit_badge_style_fn() -> Box<dyn Fn(&Theme, Status) -> Style>`
    would have rendered all badges with identical colour regardless
    of `StrategyStatus`; the parameter restores per-row palette
    routing while preserving the iced_aw `StyleFn`-shaped closure
    return type. `BadgeIntent` is defined in the same module to keep
    `theme` decoupled from `state::StrategyStatus`:
    ```rust
    #[must_use]
    pub fn cockpit_badge_style(
        _theme: &iced::Theme,
        status: iced_aw::style::Status,
        intent: BadgeIntent,
    ) -> iced_aw::style::badge::Style { ... }

    #[must_use]
    pub fn cockpit_badge_style_fn<'a>(
        intent: BadgeIntent,
    ) -> Box<dyn Fn(&iced::Theme, iced_aw::style::Status) -> iced_aw::style::badge::Style + 'a> {
        Box::new(move |theme, status| cockpit_badge_style(theme, status, intent))
    }
    ```
  - **Palette mapping landed** (ui-designer call, per
    `spec/ui-design-principles.md ## Status pill colors` — new
    subsection landed in this pass):
    - `Positive` → `UP_50` backdrop, `UP_500` label
    - `Neutral`  → `ACCENT_SOFT` backdrop, `FG_3` label
    - `Negative` → `DOWN_50` backdrop, `DOWN_500` label
    - Structural invariants: `PILL` radius, `border_width = 0.0`,
      `border_color = None`. Interaction modifiers: `Hover/Pressed/
      Focused/Selected` = base byte-identical to `Active`;
      `Disabled` alpha-scales 0.5 on both axes (iced_aw stock).
  - Unit tests added (5 tests, parallel to Brief A's pattern at
    `iced_widget_catalogs.rs:99-121` — expanded from architect's
    "2 tests" stub to cover the refined `BadgeIntent` surface):
    1. `cockpit_badge_style_fn_is_a_valid_style_fn` — compile-time
       guarantee the boxed-closure signature matches the iced_aw
       `StyleFn` alias.
    2. `cockpit_badge_style_routes_lumen_tokens_for_each_intent` —
       asserts UP_50/UP_500, ACCENT_SOFT/FG_3, DOWN_50/DOWN_500
       routing per intent via debug-formatted equality.
    3. `cockpit_badge_style_uses_pill_radius_and_no_border` — pins
       the structural invariants across all three intents.
    4. `cockpit_badge_style_disabled_scales_alpha` — pins the
       iced_aw stock `disabled()` shape on backdrop + label.
    5. `cockpit_badge_style_non_disabled_states_match_active` —
       pins the "status pills are informational, no hover-state
       colour" rule across Hovered/Pressed/Focused/Selected.
  - Test command: `cargo test -p ui --lib iced_widget_catalogs`.
  - **Tick state — developer pass closes the cite-blocked window.**
    The developer's parallel-lane edits to
    `crates/ui/src/widgets/{frame.rs, strategies.rs}` now compile
    cleanly (the missing `Element` import in `frame.rs` test fixed
    by `use iced::Element;` inside the test; `status_badge_cell`
    landed at `widgets/strategies.rs:330-345`). The 5 unit tests
    on the ui-designer's catalog adapter now run.
  - _Honest-tick test-output citation (developer pass, 2026-05-13):_
    `cargo test -p ui --lib iced_widget_catalogs` →
    `test theme::iced_widget_catalogs::tests::cockpit_badge_style_fn_is_a_valid_style_fn ... ok`
    `test theme::iced_widget_catalogs::tests::cockpit_badge_style_routes_lumen_tokens_for_each_intent ... ok`
    `test theme::iced_widget_catalogs::tests::cockpit_badge_style_uses_pill_radius_and_no_border ... ok`
    `test theme::iced_widget_catalogs::tests::cockpit_badge_style_disabled_scales_alpha ... ok`
    `test theme::iced_widget_catalogs::tests::cockpit_badge_style_non_disabled_states_match_active ... ok`
    + the 2 pre-existing table tests → `test result: ok. 7 passed`
    (5 new + 2 pre-existing in `iced_widget_catalogs` module).
  - Trace row satisfied: REQ-ICED-AW-003 (acceptance criterion 1).
- [x] **T-M3-2** *(developer, 2026-05-13)*
  — Replace `colored_cell(status_label, status_color)` at
  `crates/ui/src/widgets/strategies.rs` (column 3) with
  `iced_aw::Badge::new(...)::style(cockpit_badge_style_fn(intent))`.
  _Acceptance citations:_
  - File:line of change: column 3 lambda at
    `crates/ui/src/widgets/strategies.rs:124-131` (post-fmt; the
    `match &r.status { ... }` mapping `StrategyStatus` →
    `BadgeIntent`) + new `fn status_badge_cell` at lines 332-345
    constructing the `Badge`. The legacy `fn colored_cell` is
    deleted (was at the old `:320-322` location — `grep -n
    "colored_cell" crates/ui/src/widgets/strategies.rs` → zero hits).
  - Import: `use iced_aw::Badge;` + `use crate::theme::iced_widget_catalogs::{cockpit_badge_style_fn, BadgeIntent};`
    at `crates/ui/src/widgets/strategies.rs:25,37` (inline per Q1).
  - Lumen brand-bleed grep gate: `grep -nE "Color::from_rgb|Color::new\(" crates/ui/src/widgets/strategies.rs`
    → zero hits. Color routing flows through `cockpit_badge_style_fn`
    via the UI-designer's `BadgeIntent` → token mapping in
    `iced_widget_catalogs.rs:172-194`.
  - Test command (a): `cargo build -p ui`.
  - Test output line (a): `Finished \`dev\` profile [unoptimized
    + debuginfo] target(s) in 3.94s` clean.
  - Test command (b): `cargo test -p ui --test panel_snapshots strategies`.
  - Test output line (b): `test result: ok. 14 passed; 0 failed; 0
    ignored; 0 measured; 55 filtered out; finished in 0.28s`
    (14 strategies-named tests, all passing).
  - Trace row satisfied: REQ-ICED-AW-003 (acceptance criteria 2, 3, 4).
- [x] **T-M3-3** *(developer, 2026-05-13)* — Refresh `strategies_*.snap`
  baselines (architect-revised: zero refresh needed — see note).
  _Acceptance citations:_
  - **Architectural divergence from the architect's pre-pass
    estimate (same root cause as T-M2-3 above):** the existing
    `strategies_*.snap` baselines are produced by the text-summary
    helper `strategies_summary` at `crates/ui/tests/panel_snapshots.rs:1989-...`,
    which renders `StrategyRow` field values + status label via
    `strings::STRATEGIES_STATUS_*` — NOT the actual iced widget
    tree. The swap from `colored_cell` to `Badge` lives entirely
    in the widget render path; the text-summary helpers don't
    inspect the cell construction. **Zero existing snapshots
    changed bytes.** Confirmed by `cargo test -p ui --test
    panel_snapshots strategies` → 14 passes / 0 diffs, `find
    crates/ui -name "*.snap.new"` → empty.
  - Two-run determinism gate: `cargo test -p ui --test
    panel_snapshots strategies` run twice; zero `*.snap.new` files
    between runs.
  - Test command: `cargo test -p ui --test panel_snapshots strategies`.
  - Test output line (both runs): `test result: ok. 14 passed; 0
    failed; 0 ignored; 0 measured; 55 filtered out; finished in 0.28s`.
  - Trace row satisfied: REQ-ICED-AW-003.

## M_FINAL — Tester gate + presenter handoff

Test-runner + evaluator split per
[`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split).
Both M_FINAL tasks gated behind M1+M2+M3 all green.

- [x] **T-M_FINAL-1** *(test-runner)* — Run the full test matrix.
  _Ticked 2026-05-14 (test-runner):_ raw log at [`reports/test-run-2026-05-14T07-13Z.log`](reports/test-run-2026-05-14T07-13Z.log) — 13 commands captured. All builds exit 0; 267/267 tests pass × 2 runs; zero `*.snap.new`; PNG baselines byte-identical.
  _Acceptance criteria:_
  - Command: `cargo test -p ui` + `cargo test -p ui --test panel_snapshots`
    + `cargo test -p ui --test visual_snapshots` + `cargo build -p ui
    --bin viewer` + `cargo build -p ui --bin cockpit --features
    fixtures`.
  - All commands exit 0; zero `*.snap.new` files after the full run;
    PNG baselines under `crates/ui/tests/visual-baselines/` byte-
    identical (all 3 `charts_screen_dark_*.png` triples unchanged).
  - Report file: `spec/iced-aw-cherry-pick/reports/test-<timestamp>-iced-aw-cherry-pick.md`
    landed via the [`rust-test`](../../.claude/skills/rust-test/SKILL.md) skill.
  - Trace rows satisfied: REQ-ICED-AW-001, REQ-ICED-AW-002, REQ-ICED-AW-003.
- [x] **T-M_FINAL-2** *(evaluator)* — Emit VERDICT.
  _Ticked 2026-05-14 (evaluator):_ VERDICT → PASS at [`reports/evaluation-2026-05-14T07-13Z.md`](reports/evaluation-2026-05-14T07-13Z.md), log sha256 `30906659…f97d2`. 10/10 evaluation criteria green; transitive deps match H-arch-4 (chrono + num-traits + once_cell); CLOCKS PASS; anchors diff empty; honest-ticks verified.
  _Acceptance criteria:_
  - Verdict file: `spec/iced-aw-cherry-pick/reports/evaluation-<timestamp>-iced-aw-cherry-pick.md`.
  - Verdict: PASS iff (a) all 3 hypothesis falsifiers stayed UNFALSIFIED
    in the developer pass, (b) anchors PASS 11/11 (trivial — Brief B
    touched zero anchor code), (c) PNG-baselines byte-identical, (d)
    two-run determinism gate green on all refreshed snapshots, (e)
    Lumen brand-bleed grep green on touched files.
  - On PASS: HANDOFF → presenter.
- [x] **T-M_FINAL-3** *(presenter, after PASS)* — Assemble
  `spec/iced-aw-cherry-pick/presentations/iced-aw-cherry-pick-<date>.md`
  _Ticked 2026-05-14 (presenter, then operator-approved post cockpit-render-regression F1 fix):_ Presentation at [`presentations/iced-aw-cherry-pick-2026-05-14.md`](../archive/presentations-2026-Q2.tar.gz). Operator approved on 2026-05-14 after the cockpit panic was resolved by the cockpit-render-regression v1.0.0 sibling ship. Brief B's own widgets (loading_with_spinner / status_badge_cell) verified clean by orchestrator's M0 bypass tests — were never the trigger.
  via the [`present-results`](../../.claude/skills/present-results/SKILL.md)
  skill. Capture screenshots of the viewer-bin picker + a loading
  panel + a strategy status badge for the operator approval block.

## Notes

- **Read order for orchestrator:** [`feature.md ## Design — architect
  synthesis`](feature.md#design--architect-synthesis) first (Q1-Q7
  resolutions + falsifier verdicts); then this tasks.md; then the
  parent [`iced-ecosystem-evaluation/feature.md`](../iced-ecosystem-evaluation/feature.md)
  only if the multi-brief context is needed.
- **Anchor risk: zero.** Brief B touches `crates/ui/Cargo.toml`,
  `crates/ui/src/bin/viewer.rs`, `crates/ui/src/widgets/{frame,
  strategies, positions, pnl, agent_feed}.rs`,
  `crates/ui/src/screens/{strategies, audit, risk}.rs`, and
  `crates/ui/src/theme/iced_widget_catalogs.rs`. Zero touches to
  `crates/strategy/`, `crates/audit/`, `crates/exec/`,
  `crates/backtest/`, or any report-rendering template. The 11
  backtest body-SHA-256 anchors in
  [`spec/anchors.toml`](../anchors.toml) are not in scope.
- **PNG baseline impact: zero.** All 3 `charts_screen_dark_*.png`
  baselines stay byte-identical (no `iced_aw` widget lands on the
  Charts canvas).
- **Snapshot refresh budget: ~15** (≈8 spinner + ≈6 badge + 1 picker)
  per architect-revised estimate. Tester confirms exact count via
  `find crates/ui/tests/snapshots -name "*.snap" -newer <baseline-ref>`
  after T-M_FINAL-1.
- **Honest-tick discipline ([`AGENT.md ## Process discipline`](../../AGENT.md#process-discipline-lessons-from-v0--v15a) rule 1):**
  every T-M*-N tick MUST cite file:line + test command + test-output
  line. Reports under `spec/iced-aw-cherry-pick/reports/` are the
  durable audit trail.
- **Capability boundary:** the developer + ui-designer lanes need
  `cargo build`/`cargo test`/`cargo insta review`/display-server for
  snapshot review — all are within their normal sandbox per
  [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries).
  The architect's falsifier work was display-free (`cargo tree` +
  `cargo doc` + `grep` on `~/.cargo/registry/`).

## Changelog

- 2026-05-13 (architect): initial tasks.md v0.1.0. Resolved all 7
  analyst open questions inline in
  [`feature.md ## Design — architect synthesis`](feature.md#design--architect-synthesis)
  and broke Brief B into M0 (architect-decided + falsifier batch,
  all ticked) + M1 (B1 date_picker, 4 dev tasks) + M2 (B2 spinner, 4
  dev tasks) + M3 (B3 badge, 3 dev tasks) + M_FINAL (test-runner +
  evaluator + presenter). All 3 hypotheses RESOLVED-PASS (H-arch-9
  carries a caveat on the workspace clocks-grep scope; developer
  verifies in T-M2-4). HANDOFF → developer + ui-designer (parallel).
- 2026-05-13 (developer): closed M1 + M2 + M3 in the parallel lane.
  All 11 dev-owned tasks (T-M1-1..4, T-M2-1..4, T-M3-2..3) ticked
  with file:line + test-cmd + output-line honest-tick citations.
  T-M3-1 (ui-designer-owned) re-cited with the now-passing 5 unit
  tests on the catalog adapter. **Architectural divergences vs the
  architect's plan:** (i) `panel_snapshots` baselines did NOT need
  refresh under T-M2-3 / T-M3-3 — the existing baselines use
  text-summary helpers, not iced widget renders, so the
  `muted_body → loading_with_spinner` and `colored_cell → Badge`
  swaps don't change snapshot bytes (zero diffs both ways across
  ~20 loading + strategies tests); (ii) one new snapshot landed
  for T-M1-3 (`panel_snapshots__viewer_picker_default_closed.snap`)
  capturing the viewer model's picker state, NOT a widget render.
  All builds + `cargo test -p ui` + `bash scripts/check_no_clocks_in_ui_tests.sh`
  green. HANDOFF → tester (test-runner + evaluator split).
