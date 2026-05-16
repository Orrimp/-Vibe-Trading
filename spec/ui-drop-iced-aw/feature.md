---
slug: ui-drop-iced-aw
version: 0.1.0
status: shipped
owner: shipped
predecessor: ui-headless-emulator v0.1.0
supersedes: iced-aw-cherry-pick v1.0.0 (shipped 2026-05-14)
updated: 2026-05-16
---

> **Status (2026-05-16):** v0.1 shipped. All V1-V7 green. iced_aw +
> iced_fonts confirmed gone from `cargo tree -p ui`. 1216 workspace
> tests pass (down 8 from 1224 — 3 picker tests + 5 badge Catalog
> tests deleted as expected). Anchors 11/11 PASS.

# Drop iced_aw (and iced_fonts) — v0.1

> Strategic decoupling from the iced_aw + iced_fonts third-party
> ecosystem so a future iced 0.15.0 stable bump no longer waits on
> community catch-up. Surfaced 2026-05-16 by the operator after the
> aborted comet bump made the ecosystem-lag pattern explicit
> ([backlog.md ## Queue ## comet revisit trigger attempt note](../backlog.md)).

## Why

Two reasons, ranked by importance:

1. **Decouple from third-party iced ecosystem cadence.** `iced_aw` is
   community-maintained ([iced-rs/iced_aw](https://github.com/iced-rs/iced_aw),
   main branch only, currently at 0.14.1 locked to iced 0.14.0). It
   transitively pulls `iced_fonts` from
   [Redhawk18/iced_fonts](https://github.com/Redhawk18/iced_fonts)
   (not iced-rs official). Both are migration blockers for any iced
   0.15.0 bump — confirmed by the 2026-05-16 aborted comet attempt.
2. **The team already paid 1/3 of the cost** during
   `iced-aw-cherry-pick`. `crates/ui/src/widgets/throttled_spinner.rs`
   (~360 LOC, pure iced) is a deterministic-render fork of
   `iced_aw::Spinner` — the iced_aw spinner is dead code in our tree.
   Dropping iced_aw entirely is the natural extension.

`iced_test` is RETAINED — it's iced-rs official, bumps in lockstep
with iced, never an ecosystem-lag blocker.

## Scope locked

- **D-DA-1** — Drop `iced_aw` (and transitively `iced_fonts`) from
  `crates/ui/Cargo.toml`. The 3 cherry-picked widgets (`badge`,
  `spinner`, `date_picker`) are replaced or removed per below.
- **D-DA-2** — `spinner`: already self-replaced by
  `crates/ui/src/widgets/throttled_spinner.rs`. Just delete dead
  references.
- **D-DA-3** — `badge`: Catalog-style scaffold in
  `crates/ui/src/theme/iced_widget_catalogs.rs` (~150 LOC). No
  `Badge::new(...)` instances in production code; the scaffold was
  forward-prep. Delete in full.
- **D-DA-4** — `date_picker`: smoke-test demo only per
  [`crates/ui/src/bin/viewer.rs:294-296`](../../crates/ui/src/bin/viewer.rs):
  "Scope: smoke-test consumer only. Full backtest-range wire-in is
  out-of-scope for Brief B per the analyst's brief (v1.11 / Phase 4
  future work)." Delete the picker + its state, messages, helper fn,
  and snapshot test. If v1.11 ever needs a date picker for real, it
  can roll a calendar widget or use a text-input with date parsing —
  out of scope here.

## Out of scope

- Bumping iced from 0.14 → 0.15 (separate feature, gated on iced
  0.15.0 stable per comet revisit trigger).
- Authoring a replacement date_picker widget (the existing use is
  demo; v1.11 chart-x-axis-local-time is the candidate that would
  legitimize a date-input UX).
- Removing `iced_test` (kept; iced-rs official).
- Re-implementing iced_aw widgets we don't use today (calendar grid,
  number_input, color_picker, etc.).

## Design

### Removal map

| File | What to remove | LOC |
|---|---|---|
| `crates/ui/src/bin/viewer.rs` | `picker_block()` fn, `DatePicker`/`PickerDate` imports, picker comment block, call site in `view()` | ~75 |
| `crates/ui/src/viewer.rs` | `VIEWER_PICKER_ANCHOR` const, `picker_anchor_date()` fn, `ViewerModel::picker_open` + `picked_date` fields + their cold-start init, `ViewerMessage::PickerOpened`/`PickerCanceled`/`PickerDateSelected` variants, their `update()` arms, `viewer_picker_anchor_is_a_valid_calendar_date` test, `viewer_picker_round_trip_open_cancel_submit` test | ~100 |
| `crates/ui/tests/panel_snapshots.rs` | `viewer_picker_default_closed` test + the `viewer__picker_default_closed.snap` baseline | ~50 |
| `crates/ui/src/theme/iced_widget_catalogs.rs` | All `iced_aw::style::badge::*` adapter code; cockpit_badge_style_fn etc. | ~150 |
| `crates/ui/src/theme.rs`, `widgets/frame.rs`, `widgets/mod.rs`, `widgets/strategies.rs`, `widgets/throttled_spinner.rs`, `tests/render_snapshots.rs` | Stale `iced_aw` comments / docstring references (mostly comments — no functional code changes) | ~30 total |
| `crates/ui/Cargo.toml` | `iced_aw` dep stanza (lines 79-85) | -9 |

Net deletion: ~415 LOC + 1 snapshot file + 1 Cargo dep entry. No new code authored.

### What about iced_aw's StyleFn re-export?

`theme/iced_widget_catalogs.rs` re-exports `iced_aw::style::StyleFn`
and `iced_aw::style::Status` enums for use elsewhere. These aren't
just badge-specific — they're the iced_aw way of doing styled-widget
catalogs. Audit: confirm no other widgets reference these types. If
clean, delete; if anything does, replace with iced 0.14's native
`iced::widget::container::Style`-style approach.

## Acceptance / verification (V-items)

| # | What | How |
|---|---|---|
| V1 | `cargo build -p ui --features live --bin cockpit_live` succeeds without `iced_aw` | Compile gate |
| V2 | `cargo build -p ui --bin viewer` succeeds (the bin most affected) | Compile gate |
| V3 | `cargo tree -p ui` produces ZERO `iced_aw` or `iced_fonts` lines | `! cargo tree -p ui \| grep -E "iced_aw\|iced_fonts"` |
| V4 | `cargo test --workspace` stays green | No regression on the 1224-test baseline (minus 3 deleted tests = 1221+ expected) |
| V5 | `cargo clippy -p ui --no-deps` adds zero new warnings | Lint gate |
| V6 | `cargo fmt --check` clean | Fmt gate |
| V7 | `scripts/verify_anchors.sh` PASSES (no body-SHA drift) | Anchors aren't UI-coupled, but defense in depth |

## Risks

| # | Risk | Mitigation |
|---|---|---|
| R-DA-1 | `iced_aw::style::StyleFn` / `iced_aw::style::Status` used somewhere beyond badge | Grep audit before delete; replace inline if found |
| R-DA-2 | A test we forgot about asserts picker behavior and fails | V4 surfaces it; deletion + test-update if needed |
| R-DA-3 | `iced_fonts` is depended on by something else besides iced_aw | `cargo tree -p ui` post-removal verifies V3 |

## Out-files

**Deleted (NEW empty slots):**
- `crates/ui/Cargo.toml` — iced_aw dep stanza
- `crates/ui/src/bin/viewer.rs` — picker_block fn + imports + call site
- `crates/ui/src/viewer.rs` — picker state, messages, helpers, tests
- `crates/ui/tests/panel_snapshots.rs` — viewer_picker_default_closed test
- `crates/ui/tests/snapshots/panel_snapshots__viewer_picker_default_closed.snap` — baseline (if it exists)
- `crates/ui/src/theme/iced_widget_catalogs.rs` — badge adapters

**Modified:**
- Various source files — stale iced_aw comments / docstring refs

**New:**
- `spec/ui-drop-iced-aw/feature.md` (this file)
- `spec/ui-drop-iced-aw/tasks.md`

## Changelog

- 2026-05-16 (orchestrator): added `supersedes: iced-aw-cherry-pick
  v1.0.0` frontmatter cross-link per spec-hygiene F5. Sibling
  feature carries matching `superseded_by` field. Documents the
  adopt-then-drop lifecycle for future archaeology.
- 2026-05-16 (orchestrator): feature spec authored after operator
  picked "drop all iced dependencies which pull the project down" in
  response to the aborted comet bump's ecosystem-lag analysis.
