---
slug: ui-week1-notes
status: shipped
owner: ui-designer
updated: 2026-04-17
run_id: week1-ui
commit: (pre-commit)
verdict: PASS-UI-SCOPE
---

# UI Week 1 notes — issues and gaps for Week 2

Report from the ui-designer sub-agent covering scope `T13–T20` of
`spec/tasks/v0-paper-sma.md`. All eight tasks landed with every quality
gate green (fmt / clippy / check / test / cargo run --bin cockpit under
`--features fixtures`). This document captures the asymmetries that
showed up between the ui side and the developer side during parallel
execution — the developer needs them to finish Week 2 cleanly.

## 1. `core` crate naming ambiguity in Rust 2024

**Symptom.** The developer created `crates/core` with `name = "core"` and
`edition = "2024"`. Rust 2024's edition resolver treats `core::` in path
position as ambiguous against the built-in `core` crate (part of
libstd); consumers that write `use core::Symbol;` get
`error[E0432]: no Symbol in the root`. The `audit` crate's
`src/query.rs` triggered this when building `cargo check -p audit`.

**Workaround used in `crates/ui`.** In `crates/ui/Cargo.toml`:

```toml
trading_core = { package = "core", path = "../core" }
```

This lets the ui crate refer to `trading_core::Symbol` etc. without the
`core` shadow. All widget / fixtures / state imports were migrated to
`trading_core::*` accordingly.

**Recommended fix (developer territory).** Either
(a) rename the crate to `trading_core` workspace-wide and apply the
    `package = "core"` alias everywhere it's consumed, or
(b) rename imports inside `crates/audit/src/query.rs` to use
    `::core::Symbol` with the matching `trading_core` package rename in
    `crates/audit/Cargo.toml`.

Option (a) is cleaner long-term because every crate that depends on
`core` types (which is nearly every crate) will hit the same symptom in
2024 edition. Option (b) is minimum-viable but leaves a trap for future
crates.

## 2. `audit` dependency intentionally not wired in `crates/ui`

The architect's design (R6, `audit::query` as read-only surface for the
cockpit) is scheduled to be wired in T32. Because `audit` did not
compile (see #1) when Week 1 UI work reached its first quality gate, the
ui crate's Week 1 drop ships without an `audit` dep. The `PnlRefreshed`
/ `PositionsRefreshed` / `TapeError` / `PnlError` / `PositionsError`
messages in `ui::state::Message` already carry the payload types, so
T32 only needs to:

1. Re-add `audit = { path = "../audit", features = ["query"] }` to
   `crates/ui/Cargo.toml`.
2. Add a `Subscription` in `src/bin/cockpit.rs` that periodically calls
   `audit::query::equity()` / `recent_fills()` / `position()` and maps
   results to the corresponding `Message::*Refreshed` variants.
3. Remove the `fake_cockpit_ready()` boot from the binary (or put it
   behind a `--features fixtures` gate, which is already in place).

## 3. `iced_aw` and `plotters-iced` not adopted in v0

The architect named both as "stack" in `spec/architecture.md → Frontend
— iced`. The v0 cockpit needs none of their widgets: no date pickers,
no tabs, no modal (the kill dialog is rendered inline in the same
panel), and no charts (equity curves are a v0.5 feature in the
`viewer` binary, which is deferred). Adopting them now would add
compile-time cost for no value and lock the workspace to specific
versions earlier than necessary. Revisit when the viewer lands.

## 4. iced 0.14 API surface

Pinned exact: `iced = "=0.14.0"`. Relevant API shape details that the
Week 2 wiring developer should know:

- `iced::application(boot, update, view)` where `boot: fn() -> (State,
  Task<Message>)`, `update: fn(&mut State, Message)`, `view: fn(&State)
  -> Element<Message>`. Title / theme / subscription chain on via
  `.title(...)` / `.theme(...)` / `.subscription(...)` on the returned
  `Application`.
- `Padding` takes `u16`; spacing / font sizes take `u32` via
  `Pixels::From<u32>`. Our theme tokens are `u32` with an explicit
  `as u16` cast (with `#[allow(clippy::cast_possible_truncation)]`)
  where `Padding` is required — see `widgets/frame.rs`.
- `Container::style` takes a closure `|&Theme| container::Style`; the
  background/border/text colors come through there, not on the widget
  directly.

## 5. Self-audit consistency tests

A new test file `crates/ui/tests/consistency.rs` enforces the two
design-system invariants:

- `no_inline_user_visible_strings_in_widgets` — string literals in
  `src/widgets/*.rs` must be either format-template structural (only
  punctuation / digits / placeholders) or routed via `ui::strings::*`.
- `no_inline_hex_colors_in_widgets_or_state` — only `src/theme.rs` may
  contain `#rrggbb` tokens; `src/widgets/*.rs`, `src/state.rs`, and
  `src/strings.rs` are scanned for violations.

These tests run as part of `cargo test -p ui`. Any Week 2 widget edit
that violates the invariants fails the build.

## 6. Snapshot test approach (no pixel rendering)

Widget trees under iced 0.14 are not trivially serialisable. Snapshot
tests under `crates/ui/tests/panel_snapshots.rs` instead snapshot a
**textual summary** of each panel's logical state (panel variant, the
strings the widget code would pick, the colors for signed numbers, the
kill-switch confirm-enabled bit, the latency badge threshold). The 24
snapshots cover: 4 state variants × 5 panels + extras (pause banner,
zero-qty filter, negative-P&L color, halted banner, latency thresholds).

Pixel-level layout regressions are caught manually by
`cargo run --bin cockpit --features fixtures` and by the v0 acceptance
screenshots the tester will capture in T_FINAL_B.

## 7. Crates touched

- **Created / modified (ui-designer only):** `crates/ui/**`,
  `spec/tasks/v0-paper-sma.md` (T13–T20 tick-boxes only),
  `spec/features/v0-paper-sma.md` (appended `## UI — Week 1` section).
- **Not touched:** every other crate, `spec/architecture.md`,
  `spec/product.md`.

## Changelog

- 2026-04-17 (ui-designer): initial report.
