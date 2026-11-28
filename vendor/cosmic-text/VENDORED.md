# cosmic-text 0.15.0, vendored with one bidi patch

This directory is a long-term local fork of the `cosmic-text` 0.15.0 crate,
wired into the workspace through `[patch.crates-io]` in the root `Cargo.toml`.
It carries ONE hunk, in `ShapeLine::build` (`src/shape.rs`). The decision, its
rationale and its maintenance contract are ADR-0106
(`_bmad-output/planning-artifacts/architecture/decisions/0106-cosmic-text-is-vendored-with-one-bidi-patch.md`);
the bug is bug-log `#239` (`docs/dev-notes/bug-log.md`). Any change here beyond
the manifest delta and the hunk below is out of scope: these files are upstream
source.

## Provenance

- Crate: `cosmic-text` 0.15.0, from the local cargo registry cache
  (`~/.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f/cosmic-text-0.15.0.crate`),
  copied with no download.
- Crate SHA-256: `173852283a9a57a3cbe365d86e74dc428a09c50421477d5ad6fe9d9509e37737`,
  equal to the checksum the root `Cargo.lock` held for the registry package
  before the patch.
- Upstream VCS sha (`.cargo_vcs_info.json`): `c82ee1c5b5b8032e91eaff1cb34294b538727a7d`
  (`pop-os/cosmic-text`).
- Upstream `src/shape.rs` SHA-256: `615110123ec7e3ab29d0dc17de8c717cab1c8ac3b02f4024fe82b54f16ddd191`
- Patched `src/shape.rs` SHA-256: `c1835fdc3311a53154508025c4733fb8be6c856407582713f1e5fa2d2332655b`

`VENDORED.sha256` holds the SHA-256 of every file in this directory except
itself, sorted by `/`-joined relative path; `cd vendor/cosmic-text && shasum -a
256 -c VENDORED.sha256` checks it. `crates/ui/tests/bidi_paragraph_separators.rs`
(W2) checks the same set on every test run, and the two lines above.

## What was kept and what was dropped

Kept, byte-identical to the crate unless noted:

- `src/` (25 files, 9,737 lines upstream, 9,748 with the hunk); only
  `src/shape.rs` differs (the hunk);
- `Cargo.toml` (the manifest delta below), `Cargo.toml.orig`, `LICENSE-MIT`,
  `LICENSE-APACHE`, `README.md` (the manifest's `readme`), `CHANGELOG.md` and
  `.cargo_vcs_info.json`.

Dropped: `fonts/`, `sample/`, `screenshots/`, `tests/`, `benches/`, the `*.sh`
scripts, `deny.toml`, the crate's own `Cargo.lock`, and the dotfiles
`.gitattributes` (LFS `filter=lfs` rules that would override the root's inside
this directory), `.gitignore`, `.github/` and `.cargo-ok`. Nothing under `src/`
reads a dropped file: there is no `include_bytes!`/`include_str!` and no
`build.rs`.

**Manifest delta, the only change outside `src/shape.rs`:** the four `[[test]]`
tables, the two `[[bench]]` tables, the two `[dev-dependencies.*]` tables and
the `[profile.test]` table are deleted (crate `Cargo.toml` lines 74-98 and
188-197). Each served only the dropped tests and benches, and a non-root
`[profile]` is ignored with a warning anyway. `name`, `version = "0.15.0"` (so
`^0.15` still matches), the explicit `[lib]`, the `auto* = false` keys, the
features and the dependencies are byte-identical.

## The hunk

In `ShapeLine::build`, upstream `let bidi = ...` becomes `let mut bidi = ...`,
and these lines are inserted after the `rtl` block:

```rust
        // TRADING-PATCH (ADR-0106, bug-log #239; upstream pop-os/cosmic-text#291):
        // `LineIter` cuts lines only at `\r`/`\n`, but `BidiInfo` starts a paragraph
        // after every Bidi_Class=B char (U+001C-U+001E, U+0085, U+2029), and the loop
        // below asserts every paragraph has paragraph 0's direction. When they
        // disagree, redo the paragraph-level step with paragraph 0's level as the
        // default (UAX #9 HL1): the whole line takes paragraph 0's direction. A line
        // upstream shapes without panicking never enters this branch.
        if bidi.paragraphs.iter().any(|p| p.level.is_rtl() != rtl) {
            let level = bidi.paragraphs[0].level;
            bidi = unicode_bidi::BidiInfo::new(line, Some(level));
        }
```

The condition is exactly the negation of "every iteration of the upstream
`assert_eq!(line_rtl, rtl)` holds", so every line upstream shapes without
panicking runs upstream's code byte for byte. The assert is kept: with
`Some(level)` every recomputed paragraph takes paragraph 0's level, so it holds.
The hunk moves it from upstream's line 960 to line 971, so triage matches a
panic located in `src/shape.rs`, never a line number.

Upstream's `# Panics` rustdoc on `ShapeLine::new` and `ShapeLine::build`
(`src/shape.rs:912-914`, `:931-933`) still says such a line panics. It is now
stale and deliberately left unedited (one-hunk rule).

## Known toolchain exposure

A path package is not lint-capped, so the crate's warn-level rustc lints print
on every build of it, and its three rustc `deny` attributes
(`#![deny(missing_debug_implementations)]`, `#![deny(unreachable_patterns)]`,
`#![deny(unused_must_use)]`, `src/lib.rs`) are hard errors here, where the
registry copy had them capped. Every toolchain bump must build this crate; a
deny hit is a decision point under ADR-0106 D4 (a second documented hunk, or
retiring the fork), never a silent edit.

## Retirement

Retire the fork when the cosmic-text that iced requires no longer asserts in
`ShapeLine::build` (upstream `pop-os/cosmic-text#291`), and only after
`crates/ui/tests/bidi_paragraph_separators.rs` B2 and B3 are green WITHOUT it:
B3 encodes the direction policy, and an upstream fix that splits lines at
class B instead would change it deliberately (ADR-0106 D4).
