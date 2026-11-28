---
adr: 0106
title: cosmic-text 0.15.0 is vendored with one bidi patch
status: accepted
date: 2026-11-28
supersedes: none
superseded-by: none
related: ["ADR-0057 cockpit render verification is macOS-canonical (D1)", "ADR-0093 the cockpit embeds its UI face"]
---

# ADR-0106: cosmic-text 0.15.0 is vendored with one bidi patch

## Context

Bug-log `#239`. iced 0.14 shapes every line of text through cosmic-text 0.15.
`Buffer::set_text` cuts lines only at `\r` and `\n` (`LineIter`,
`line_ending.rs:55`), but unicode-bidi starts a new paragraph after every
Bidi_Class=B character, and five of them can sit inside one line: U+001C,
U+001D, U+001E, U+0085 and U+2029. `ShapeLine::build` then asserts that every
paragraph of the line has paragraph 0's direction
(`assert_eq!(line_rtl, rtl)`, `shape.rs:960`). A paragraph is RTL iff its first
strong character outside an isolate is R or AL, so `"\u{2029}\u{5d0}"`, a
separator then any RTL text, an invisible U+200F, or RTL text followed by a
separator and only digits (`"\u{5d0}\u{2029}1"`) all panic.

- **Blast radius.** The panic fires while iced holds its process-wide
  `RwLock<FontSystem>` write guard (`iced_graphics-0.14.0/src/text.rs:116-120`,
  `paragraph.rs:68-69`). The unwind poisons the lock, every later text draw
  panics on `PoisonError`, and `cockpit_live` runs iced on the main thread, so
  the process ends, and the paper session and the agent with it.
- **Reach.** The 7 routed `text_input`s (U+2029 by paste or typing; all five by
  an IME commit, whose arm filters nothing), a `config/strategies/*.toml` id or
  file stem (drawn by the Lab chip on the boot screen), persisted and replayed
  rows (Lab state, audit-ledger rows: a separator that reaches a store re-trips
  on every later session that draws it), LLM narration, report files and error
  strings that quote external text.
- **Why app code cannot stop it in a `text_input`.** The widget mutates its own
  value and re-shapes it inside its own `update` (`text_input.rs:983-995` for a
  paste, `:1023-1035` for a typed char, `:1290-1309` for an IME commit) before
  the app's `update` sees the message, so no `on_input`/`on_paste` sanitiser can
  intercept it; and a draw-site guard cannot see iced's implicit `From<&str>`
  conversions.
- **No upstream fix.** `pop-os/cosmic-text#291` ("Bidi behavior of `set_text()`
  vs. `set_rich_text()`") has been open since 2024-07-25 with no fix; cosmic-text
  0.19.0's `ShapeLine::build` still asserts (both read 2026-11-27). iced 0.14
  requires `cosmic-text ^0.15`, and an iced bump is ruled out near-term
  (CLAUDE.md, "Vendored dependencies").

## Decision

**D1. The vendored tree.** `vendor/cosmic-text/` is cosmic-text 0.15.0 copied
from the local registry cache with no download: crate SHA-256
`173852283a9a57a3cbe365d86e74dc428a09c50421477d5ad6fe9d9509e37737` (equal to
the lockfile's checksum before the patch), upstream VCS sha
`c82ee1c5b5b8032e91eaff1cb34294b538727a7d`. Kept: `src/` (25 files, 9,737
lines upstream, 9,748 with the hunk), `Cargo.toml`, `Cargo.toml.orig`, `LICENSE-MIT`, `LICENSE-APACHE`,
`README.md`, `CHANGELOG.md`, `.cargo_vcs_info.json`. Dropped: `fonts/`,
`sample/`, `screenshots/`, `tests/`, `benches/`, the `*.sh` scripts,
`deny.toml`, the crate's `Cargo.lock`, and the dotfiles `.gitattributes` (LFS
rules that would override the root's), `.gitignore`, `.github/`, `.cargo-ok`.
The manifest delta deletes only the `[[test]]`, `[[bench]]`,
`[dev-dependencies.*]` and `[profile.test]` tables; `name`, `version`,
`[lib]`, the `auto* = false` keys, the features and the dependencies are
byte-identical. Provenance, the lists, the delta, the hunk verbatim and the
upstream and patched `src/shape.rs` SHA-256s are in `vendor/cosmic-text/VENDORED.md`;
`vendor/cosmic-text/VENDORED.sha256` hashes every vendored file but itself.

**D2. The hunk.** In `ShapeLine::build`, `let bidi` becomes `let mut bidi`, and
after the `rtl` block:

```rust
if bidi.paragraphs.iter().any(|p| p.level.is_rtl() != rtl) {
    let level = bidi.paragraphs[0].level;
    bidi = unicode_bidi::BidiInfo::new(line, Some(level));
}
```

under a `TRADING-PATCH` comment. When the paragraph directions disagree, the
paragraph-level step is redone with paragraph 0's level as the default (UAX #9
HL1), so the whole line takes paragraph 0's direction.

- **Identity, by construction.** The condition is exactly the negation of
  "every iteration of the `:960` assert holds", so the branch is entered iff
  upstream would panic, and every other input runs upstream's code byte for
  byte. On an empty line `any` over an empty `Vec` is `false`, so
  `paragraphs[0]` is never indexed there.
- **The kept assert is a live invariant.** With `Some(level)` every recomputed
  paragraph takes paragraph 0's level, and `rtl` (computed before the hunk,
  never reassigned) equals `level.is_rtl()`, so the assert holds. A recompute at
  any other level panics on the disagreeing lines of the other polarity
  (probes PBIDI-ltr and PBIDI-rtl, bug-log `#239`, measure exactly that). The
  hunk moves the assert from upstream's `shape.rs:960` to
  `vendor/cosmic-text/src/shape.rs:971`, so triage matches a panic located in
  `…/src/shape.rs:`, never a line number.
- **Stale upstream rustdoc, left on purpose.** The `# Panics` sections of
  `ShapeLine::new` and `ShapeLine::build` (`shape.rs:912-914`, `:931-933`) still
  say a line whose paragraphs disagree in direction panics; under the hunk it
  does not. They are left unedited (one-hunk rule).
- **What changes on screen.** Only formerly panicking lines: they lay out as one
  line with paragraph 0's base direction, the RTL runs embedded as in
  `"abc \u{5d0}\u{5d1}"`. U+2029 itself is shaped like any other character.

**D3. Wiring.** `[patch.crates-io]` in the root `Cargo.toml` holds
`cosmic-text = { path = "vendor/cosmic-text" }` beside `iced_tiny_skia`;
`Cargo.lock`'s cosmic-text entry loses its `source` and `checksum` lines. The
path package is not a workspace member: it is reached only through
`[patch.crates-io]`, which is not a dependency edge of any member, so cargo does
not auto-add it (measured: `cargo metadata --no-deps` lists the 17 `crates/*`
members and no cosmic-text), and clippy never lints it. A direct `path =`
dependency on it from a member WOULD make it a member (cargo auto-adds path
dependencies inside the workspace root), and the workspace clippy step
(`cargo clippy --workspace -- -D warnings`, `.github/workflows/ci.yml`) would
then lint it, including its `#![deny(clippy::unwrap_used)]` and
`#![deny(clippy::cast_ptr_alignment)]` (`src/lib.rs:71,73`): never add one.
`.gitattributes` holds `vendor/cosmic-text/** -text`, so
both CI legs hash the committed bytes.

**D4. The maintenance contract.**

- The fork is never edited beyond D1's manifest delta and D2's hunk (CLAUDE.md
  vendor policy). The ui test `bidi_paragraph_separators` W2 fails on any
  change not re-recorded in `VENDORED.sha256` and `VENDORED.md` (it skips
  OS/editor cruft, `.DS_Store`, `*.swp`, `*.swo`, `*~`, and `target/`); B2, B3
  and C1-C4 catch a behaviour change that was re-recorded (probe
  PBIDI-cond-rehash); W1 fails if the patch line goes.
- **Every toolchain bump** (`rust-toolchain.toml`, `1.94.1` at this ADR) must
  build `vendor/cosmic-text`. A path package is not lint-capped, so its three
  rustc `deny` attributes (`deny(missing_debug_implementations)`,
  `deny(unreachable_patterns)`, `deny(unused_must_use)`, `src/lib.rs:75,78,80`)
  are hard errors here. A deny hit is a declared decision point: a second
  documented hunk under this ADR, or retiring the fork; never a silent edit.
  Its warn-level lints print on every build of it and fail nothing (no
  `RUSTFLAGS` is set in CI or `.cargo/`). The first build under 1.94.1 printed
  0 deny errors and 0 warnings from the crate ON macOS (bug-log `#239`, Step
  0); the Windows leg compiles other cfg arms (`src/font/fallback/windows.rs`)
  uncapped for the first time in this commit's first CI run, unmeasured here.
- **Every iced or cosmic-text bump** must check whether the new cosmic-text
  still asserts in `ShapeLine::build`. If it does, port D2. If upstream fixed
  it, retire the fork only after `bidi_paragraph_separators` B2 and B3 are green
  WITHOUT it: B3 encodes the direction policy ("a mixed line takes its first
  paragraph's direction"), and an upstream fix that splits lines at class B
  instead would change it deliberately; the reviewer then decides the policy.
- **The audit** is probe PBIDI-revert (delete the inserted block): W2, B2, B3,
  C1, C3, C4, LR-a and LR-b go red.

## Alternatives considered

- **(a) App-side sanitising** (a `shape_safe` map of the five separators to
  U+0020, an event-rewriting wrapper on every `text_input`, helpers plus a
  clippy guard on about 450 draw sites) — rejected: sanitising messages cannot
  reach a `text_input`'s own shaping, and the draw-site guard leaks through
  iced's `From<&str>` conversions for `Text`, `Element` and canvas text, so it
  is a standing per-widget discipline that cannot be checked.
- **(c) An upstream fix or version bump** — not available: 0.19.0 still
  asserts, and any newer cosmic-text needs an iced bump.
- **(d) Catching the panic** — rejected: the panicking frames are iced's
  (widget `update`/`layout`/`draw`), not app code's; the unwind has already
  poisoned iced's lock, and `RwLock::clear_poison` would race iced's own
  `.expect(...)`; the widget keeps the raw value and re-panics every frame.
- **Patch unicode-bidi** — rejected: it is shared with `stringprep`, and its
  paragraph splitting is correct per UAX #9.
- **Delete the assert** — rejected: `ShapeLine` carries one `rtl` per line and
  `adjust_levels` resets whitespace over the whole line to one paragraph's
  level, so mixed levels would be inconsistent, not just unasserted.
- **Split lines at class B in `LineIter`** (#291's literal mismatch) — rejected
  here: it changes line counts, cursor positions and heights for every
  `set_text` caller holding a class-B char, and would make a single-line
  `text_input` hold two lines. Upstream may still choose it (D4).

## Consequences

- One hunk covers every shaping path in the binary (`Text`, `text_input`,
  canvas `fill_text`, outline text, any future widget), whatever the input path.
- Enforced by `crates/ui/tests/bidi_paragraph_separators.rs`: W1 (the patch
  line, one source-less cosmic-text, one unicode-bidi in the lock), W2 (the tree
  against `VENDORED.sha256`, the hunk present, the assert kept), B1-B3 (on a
  private font system: class B is seven chars, no line panics through
  `set_text` or `set_rich_text`, a mixed line takes paragraph 0's direction),
  C1-C4 (the budget field and the Lab chip through `iced_test::Simulator`, in
  child processes so a regression cannot poison the binary) and, on macOS, the
  AD-10 pixel pairs LR-a and LR-b. CI counts the target per leg
  (`scripts/ci_test_target_counts.py`: macOS 19, Windows 15).
- `layout_invariants`' bug-log `#231` canary (its T2, which expected the trips)
  is deleted; its separator exclusion and private-shaping guard stay as
  documented belt.
- A second long-term vendored fork beside `vendor/iced_tiny_skia/`; the new
  standing duty is the toolchain-bump build of D4.
- **No upstream posting** (operator decision, 2026-11-28). The text that would
  be proposed on `pop-os/cosmic-text#291`, kept here as a note only: "Still
  reproducible in 0.15.0, and by reading still present in 0.19.0
  (`ShapeLine::build`, `assert_eq!(line_rtl, rtl)`). Minimal case:
  `buffer.set_text(&mut font_system, "\u{2029}\u{5d0}", &Attrs::new(),
  Shaping::Advanced, None)` panics with `left: true, right: false`. The same
  happens with U+0085 and U+001C-U+001E, with an invisible U+200F instead of
  Hebrew, and with RTL text followed by a separator and digits. One fix that
  keeps the assert: when the paragraph levels disagree, recompute with
  `BidiInfo::new(line, Some(bidi.paragraphs[0].level))` (UAX #9 HL1)."

## Changelog
- 2026-11-28 (orchestrator, bug-log `#239` fix): initial accept; the fork was
  approved by the operator on 2026-11-28, upstream posting declined.
