---
slug: ui-drop-iced-aw
status: shipped
owner: shipped
updated: 2026-05-16
---

# Tasks — Drop iced_aw v0.1

> ## Ship status (2026-05-16)
>
> All M1-M5 tasks complete. iced_aw + iced_fonts gone. Single-commit ship.
>
> Net effort: ~3 hours actual vs ~18 hours estimate. The biggest
> overestimate was on the date_picker replacement — the docstring
> revealed it was already smoke-test demo, so removal (not roll-your-
> own) was correct. The badge replacement was ~25 LOC of native
> Container+Text in `widgets/strategies.rs::status_badge_cell`.

> Effort: **~1.5-2.5 dev-days** per the
> [backlog menu cost analysis 2026-05-16](../backlog.md). Net deletion
> (~415 LOC + 1 snap file + 1 Cargo dep) — no new code authored.

## M1 — Remove date_picker (1.0d)

- [ ] **T01** — Delete `picker_block()` fn + `DatePicker`/`PickerDate`
  imports + the comment block in
  [`crates/ui/src/bin/viewer.rs`](../../crates/ui/src/bin/viewer.rs).
  Remove the `picker_block(model)` call site in the bin's `view()`.
- [ ] **T02** — Delete `VIEWER_PICKER_ANCHOR`, `picker_anchor_date()`,
  `picker_open` + `picked_date` fields, `PickerOpened`/`PickerCanceled`/
  `PickerDateSelected` message variants + their `update()` arms, and
  the two `viewer_picker_*` tests in
  [`crates/ui/src/viewer.rs`](../../crates/ui/src/viewer.rs).
- [ ] **T03** — Delete the `viewer_picker_default_closed` test in
  [`crates/ui/tests/panel_snapshots.rs`](../../crates/ui/tests/panel_snapshots.rs)
  and its baseline `.snap` file (if present).

## M2 — Remove badge Catalog scaffold (0.25d)

- [ ] **T04** — Audit `iced_aw::style::StyleFn` and
  `iced_aw::style::Status` usage outside the badge module (grep). If
  used elsewhere, replace inline with iced-native style approach.
  If not, proceed.
- [ ] **T05** — Delete badge-specific style functions in
  [`crates/ui/src/theme/iced_widget_catalogs.rs`](../../crates/ui/src/theme/iced_widget_catalogs.rs).

## M3 — Cleanup + drop dep (0.25d)

- [ ] **T06** — Grep for remaining `iced_aw` references (mostly
  comments + docstrings in theme.rs, frame.rs, widgets/mod.rs,
  widgets/strategies.rs, widgets/throttled_spinner.rs,
  tests/render_snapshots.rs). Update wording to remove forward
  references; preserve historical context where it explains a
  past decision.
- [ ] **T07** — Delete the `iced_aw` dep stanza from
  [`crates/ui/Cargo.toml`](../../crates/ui/Cargo.toml).
- [ ] **T08** — `cargo update` to refresh Cargo.lock. Verify with
  `cargo tree -p ui | grep -E "iced_aw|iced_fonts"` — must be empty.

## M4 — Verification (0.5d)

- [ ] **T09** — Run V1-V7 from
  [feature.md ## Acceptance / verification](feature.md#acceptance--verification-v-items).

## M5 — Ship (0.25d)

- [ ] **T10** — Update backlog entry. Single-commit ship per the
  precedent set by `ui-headless-emulator` and
  `ui-session-journal-iced-tester`.

## Effort summary

| Milestone | Hours |
|---|---|
| M1 — date_picker | 8 |
| M2 — badge | 2 |
| M3 — cleanup + dep drop | 2 |
| M4 — verification | 4 |
| M5 — ship | 2 |
| **Total** | **18 hours (~2 dev-days)** |

## Status

- 2026-05-16 (orchestrator): tasks authored. Implementation starting.
