---
adr: 0105
title: A Lab Custom range travels as two typed bounds, and a keystroke commits only a new pair that parses, decided against the live range
status: accepted
date: 2026-11-28
supersedes: none
superseded-by: none
related: ["ADR-0055 Lab run persistence topology and anchor-safety (§ D9 item 3)", "ADR-0094 the Lab session persists, and the cockpit says whether it came back (§ D2)", "ADR-0057 cockpit render verification is macOS-canonical (D2)"]
---

# ADR-0105: A Lab Custom range travels as two typed bounds, and a keystroke commits only a new pair that parses, decided against the live range

## Context

Bug-log `#230`, two halves, both confirmed at HEAD `ecfb1bed`.

- **The encoding.** `LabRunConfig` carried the Lab's range as one `range_label: SmolStr`.
  `cockpit_live` joined a Custom range's two bounds into it as `Custom:{start}:{end}`, and
  `lab::runner::lab_config_to_scenario` split it back with `splitn(2, ':')`. The bound parse
  accepts RFC 3339, which contains `:`, so a START bound with a time was cut at its first colon.
  MEASURED (Step 0 of the fix session, a scratch test at HEAD):
  `Custom:2024-01-01T00:00:00Z:2024-02-01` gave
  `Err("invalid custom date '2024-01-01T00': …")`, naming text the operator never typed, while
  `Custom:2024-01-01:2024-02-01T00:00:00Z` mapped. No encoding over operator-typed text is
  unambiguous without escaping; a typed carrier is unambiguous by construction.
- **The dispatch.** Both Custom fields dispatched `LabSelectRange(Custom{..})` on every keystroke
  (MEASURED: 9 messages for `2024-01-1`, through `iced_test::Simulator`), although the widget's
  doc promised "only when both dates parse". Each keystroke assigned `range`, so it became a new
  Lab tuple (a report walk on the iced thread, cached per partial string until the next run), it
  cleared both run mirrors (the KPI strip, the equity overlay and the delta badge), and it marked
  the session dirty, so a half-typed date reached the session file about 500 ms later
  (`PersistenceDebouncer` throttles from the first mark; it is not a quiet-period debounce).
- **Three parsers for one bound.** The editor's colour (`is_valid_date`, a shape check for
  `YYYY-MM-DD` only: an RFC 3339 bound was drawn red, `2024-02-30` valid), the run's
  `custom_range_ms`, and the Yahoo cadence badge (which appended `T00:00:00Z`, so any bound with
  a time fell back to now − 90 d) disagreed.
- **Why the decision cannot live in the view.** iced 0.14 applies every message of one event
  batch, all built from one widget tree, before it draws again (`iced_winit-0.14.0/src/lib.rs`
  :1214-1231, MEASURED by reading; `iced_test`'s `simulate` likewise shares one tree). A
  commit-or-draft decision captured in a view closure is therefore stale for every later
  keystroke of the same batch, and can store a parseable draft over a different committed range.
- **What `text_input` offers.** iced_widget 0.14.2's builder has `on_input`, `on_submit` and
  `on_paste` only: no blur or focus callback (MEASURED by reading `text_input.rs:157-218`). The
  widget is controlled, so a keystroke can be shown only through the model.

## Decision

**D1 — The typed carrier.** `LabRunConfig.range: crate::lab::state::DateRange` replaces
`range_label: SmolStr`. `lab_config_to_scenario` maps it preset for preset and parses a Custom
range's two fields with `custom_range_ms`; the preset label aliases and the "unknown range"
error are gone, because nothing else produced them. `LabRunConfig::range_label()` is display
only (tracing fields, the activity labels, the no-data notices' `{window}`) and prints the text
the old field held: `Last30d`, `Last90d`, `H1_2024`, `H2_2024` and `Custom:{start}:{end}`
(`strings::LAB_RUN_RANGE_*`, `strings::fmt_lab_run_range_custom`, one `format!` so operator text
holding `{end}` prints as typed). The run is built by the library seam
`lab::runner::lab_run_config(&LabState)`, which `cockpit_live` calls; the inline capture is gone.

**D2 — One bound parse.** `lab::runner::custom_bound_ms(raw)` (RFC 3339, or `YYYY-MM-DD` read as
midnight UTC; `Err` names the bound whole, as typed) is the only reader of a Custom bound: the
run mapper and the Lab's report lookup (both through `custom_range_ms`, whose signature is
unchanged), the editor's colour, the commit rule and the cadence badge. `is_valid_date` is
deleted. The editor accepts RFC 3339 and shows it whole: each field is 210 px
(`widgets::date_range`'s `DATE_FIELD_WIDTH_PX`; the widest 25-character run, offset included,
measures 190 px at `text::BODY`), so the offset that decides a bound's validity is on screen (the
first round kept HEAD's 120 px, which clipped a bound after about 15 characters: a valid bound and
one without its offset looked the same but for the border). The hint reads "Invalid date (use
YYYY-MM-DD or RFC 3339 with an offset)"; the placeholders keep the date-only form.

**D3 — One keystroke message, classified in `update` against the live range.** Each field sends
`Message::LabCustomRangeEdited { start_raw, end_raw }`: the two texts as shown, the other field's
text taken from what the editor shows (the draft's, while one exists).
`LabState::classify_custom_edit` decides, against the live `range`:

- **Commit**: a new pair whose two bounds parse. It becomes `range` (through `set_range`), and
  the arm clears both run mirrors, exactly as `LabSelectRange` does.
- **Draft**: any other new pair. It is kept as `LabState::custom_draft`
  (`CustomRangeDraft { base, start_raw, end_raw }`, `base` the committed range it edits). Only
  the fields show it: the tuple, the mirrors, the equity cache and the saved session stay.
- **Clear**: the committed pair itself, or a keystroke that lands while a preset is selected
  (built from a frame drawn before a chip click in the same batch). No draft is kept.

A draft therefore never parses. One residual of the batch hazard is recorded, not designed out:
each field's closure still carries the OTHER field's text as the view drew it, so a batch that
commits in one field and then types in the other sends a stale other-field text; the second
keystroke then becomes a draft that drops the first one's character from the display (`range`
holds the commit). Nothing wrong is run or saved: the draft never parses, and Run refuses it by
name. The dirty mark comes from the wrapper in `state::update`:
`commits_custom_range` asks the same classifier, so the session is marked dirty exactly when
`range` changes (ADR-0094 § D2 amendment). The cost: one commit per completed date-only bound,
two for an RFC 3339 bound (its date-only prefix parses first), against one per keystroke before.

**D4 — Run runs what the fields show.** `LabState::run_range()` is the draft while it edits
`range`, else `range`; `lab_run_config` reads it. A draft never parses, so a Run from a draft is
refused by the mapper naming the bound as typed (`RunFinished.detail` carries it), which is what
an invalid field did before. The session log's `lab_run_inputs` is unchanged and records
`range`, the committed value the session file saves (story 6-11 AC1).

**D5 — Every production writer of `range` goes through `LabState::set_range`,** which drops the
draft even when the new range equals the draft's base: `LabSelectRange` (the Custom chip
included), `InspectStrategyFromLeaderboard` and `OpenLabFromCompare`. `base` is the belt for a
direct write (tests): a draft whose `base` is no longer `range` is neither shown nor run
(`lab::state::custom_fields`).

**D6 — Persistence and report names are unchanged.** The session schema stays `version: 1`
(`range` saved as before; the draft is a UI selection, cloned like `range` and `data_source`, and
never serialized), so every session written before this change loads unchanged. Pinned by
`lab::persistence::tests` PS1 and PS2: a session literal in the pre-`#230` format, with a
half-typed end bound, loads as the committed range with no draft, runs what it shows (refused
naming the bound), and re-encodes byte for byte with or without a draft over it. A
report's `custom<start_ms>to<end_ms>` token is computed by the same parse as before, so every
pair that ran before names the same report. No migration. A session saved mid-typing by an
older cockpit restores as typed and is refused on Run naming the bound, as before.

**D7 — The cadence badge** reads each bound with `custom_bound_ms`, keeps the per-bound fallback
to now − 90 d, and takes `now_ms` from its caller (`screens::lab::view` reads the clock beside
the call), so the fallback is pinnable. It reads the committed `range`, as the tuple does.

**D8 — One order rule, for every source, before any preload** (added by the review round).
`lab_config_to_scenario` refuses a Custom pair whose start is not before its end with
`strings::fmt_lab_custom_range_not_increasing`, naming both bounds as typed. `spawn_lab_run` maps
every run there before the Yahoo or Binance preload, so the rule holds on every source. Before
it, only the engine's `InvalidRange` (`start_ms > end_ms`) checked the order, after any preload:
a reversed pair inside 2023-2024 on Binance was answered "No pinned Binance data … Pick … a Custom
range inside 2023-2024", and on Yahoo it went into the preload arm (MEASURED with the check
removed, probe P28). The editor's commit rule stays per bound (D3): a reversed pair whose two
bounds parse commits, and Run refuses it here by name. The engine's check stays as the belt for
the CLI and other callers.

## Alternatives considered

- **Commit on blur** — not available: iced_widget 0.14.2 has no blur or focus callback.
- **Debounce the keystrokes** — rejected: it puts wall-clock time into `update`, makes the tests
  timing-dependent, and still commits a pause in the middle of a partial date.
- **Commit on Enter, or an Apply button** — rejected: a step nobody is told about, new copy or a
  button, and a typed but unentered pair that Run would not run.
- **Classify in the view** (the first draft's `custom_edit_message`) — rejected: stale within one
  iced batch (Context). The stale four-keystroke batch is pinned by
  `widgets::date_range::tests::a_batched_edit_back_to_the_committed_pair_leaves_no_parseable_draft`.
  No single edit reintroduces a view-time decision, because the message carries no committed
  range; that test is the regression pin should one be added back.
- **Refuse to store a parseable draft** — rejected: it silently drops text the operator typed.
- **Escape the colon-joined label** — rejected: any escape over operator text is a second
  encoding to keep in sync; the typed carrier needs none.
- **Classify a reversed pair as a Draft** (keep it out of `range` until it increases) — not
  taken (the review round's ruling): the commit rule stays per bound, as designed, and the order
  is a run rule, refused by name at Run for every source (D8).
- **Disable Run while a draft exists, or run the committed pair** — rejected: the first needs a
  new `RunState` input and new pixels; the second is a silent mismatch between the fields and the
  run.

## Consequences

- An RFC 3339 start bound runs. An RFC 3339 bound is no longer drawn red, and `2024-02-30` now
  is. A keystroke no longer clears the KPI strip, the equity overlay or the delta badge until a
  pair commits; a partial date is never written to the session file; a Yahoo Custom range with a
  time shows its real cadence; a whole RFC 3339 bound is visible in its field. A start that is
  not before its end still commits (each bound parses), and Run refuses it by name in the mapper
  for every source before any preload (D8): one rule in one place. (The first round said the
  engine's `InvalidRange` did this; that held for Synthetic only.)
- No engine, report, `evidence/` or session-schema change; AD-1 and AD-9 untouched; anchors
  119/119 before and after; no committed pixel baseline draws a Custom range.
- Enforced by:
  - `lab::runner::tests` R1-R9 (the typed mapping, the per-bound refusal, every preset,
    `range_label()`'s text, `custom_bound_ms` in one-dimension pairs, the editor-to-run seam, a
    Run from a draft refused and its twin, and R9: the order refusal per source, the equal pair
    as its boundary);
  - `lab::state::tests` S1-S7 (the classifier, the draft, `run_range`, the `base` belt, clone,
    `set_range`);
  - `state::tests` D1-D8 (through `state::update`: a draft moves nothing but the text, a commit
    clears and persists, `persists_lab_state` is unchanged, the chip, Inspect and Compare drop the
    draft, the session log keeps `range`);
  - `widgets::date_range::tests` W2, W5, W5b, K1-K4 (`iced_test::Simulator`: the message per
    keystroke, the cross-field capture with a draft in both directions, and the messages of one
    widget tree applied through `update`, the stale batch included);
  - `screens::lab::tests` G1-G3 (the badge);
  - `lab::persistence::tests` PS1-PS2 (an old-format session loads and re-encodes unchanged);
  - the AD-10 pixel file `crates/ui/tests/lab_custom_range_render.rs` F1-F8, red first at HEAD
    (F1, F3a, F6, F7; F8 is the review round's, red under P26, the 120 px field) and declared in
    `scripts/ci_test_target_counts.py` (macOS 11, Windows 0);
  - `crates/ui/tests/lab_custom_range_order.rs` O1-O3 (the real `spawn_lab_run` Task on
    Synthetic, Yahoo and Binance: a reversed pair is refused by name with no preload, its valid
    twin passes the mapper), declared 3 on both legs (`live`).
  Every pin except the twins R2, R8b and G2 and the tables R6 and S4 was probed T-T4, one edit
  each, restored byte-identical. All reddened as declared except P4, which also reddened D8. P1-P23
  and H1-H2 ran before PS1/PS2 existed; P1 re-run on the final tree reddened R1, R3, R7 and PS1,
  as declared there. The probe table is in bug-log `#230`.
- The commit decision moved out of the widget, so a future Custom editor (a calendar widget)
  sends the same message and inherits the rule.

## Changelog
- 2026-11-28 (orchestrator, bug-log `#230` fix): initial accept. ADR-0055 § D9 item 3 and
  ADR-0094 § D2 amended in the same commit. The review round, before the commit: D2's 210 px
  fields and hint copy, D3's recorded residual, D8 the order rule (correcting the Consequences'
  "the engine refuses it" sentence), and the probe record stated as measured.
