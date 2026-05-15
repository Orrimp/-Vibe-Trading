---
type: dev-note
slug: iced-014-feature-analysis
owner: analyst
updated: 2026-05-15
related: [ui-testability-deep-dive-2026-05-15, ui-gallery-bin]
---

# iced 0.14 feature analysis (2026-05-15)

Targeted follow-up to
[`ui-testability-deep-dive-2026-05-15.md`](ui-testability-deep-dive-2026-05-15.md).
Operator asked three concrete questions; this note answers them with
version-precise URLs and a verdict on every queued candidate in
[`backlog.md ## Process / tooling`](../backlog.md#process--tooling).
The earlier deep-dive's §2.1 "iced ecosystem state-of-the-art" was
correct on broad strokes but **under-counted the testing/debug surface
shipped at 0.14.0** — it knew about `iced_test::Simulator` and
`screenshot()` but did not surface the `iced_tester` recorder crate,
the `ice` shareable test format, the `Emulator` headless runtime, or
the comet companion app. Those four findings reshape what we should
build next.

## TL;DR — operator's tick

1. **The strategies-Table panic is NOT fixed in 0.14.0 and is NOT on
   master.** No CHANGELOG entry, no open or closed issue mentions
   "Build quad rectangle". The closest prior art (issue
   [#2311 "Quad with non-normal height"](https://github.com/iced-rs/iced/issues/2311),
   ComboBox + tiny-skia, closed via
   [#2364](https://github.com/iced-rs/iced/pull/2364)) is the same
   family of degenerate-quad bug in tiny-skia's quad pipeline.
   **Verdict: file upstream + ship workaround (b) — gallery-only
   non-table render of the strategies cell.** See § The strategies-
   Table panic.
2. **iced 0.14 ships a real recorder/replay** (`iced_tester` crate,
   `tester` feature flag,
   [PR #3059](https://github.com/iced-rs/iced/pull/3059)) and a real
   headless runtime (`iced_test::emulator::Emulator`,
   [PR #2698](https://github.com/iced-rs/iced/pull/2698)). Both are
   available NOW at our pinned 0.14.0. **`ui-session-journal` is
   cheapened from ~4 dev-days to ~1 dev-day** — most of the work is
   already done upstream; we just wire `iced_tester::attach()` into
   `cockpit_live --record-journal` and consume `.ice` files in tests.
3. **comet is pinned at iced 0.15.0-dev (master); it does NOT compile
   against our pinned 0.14.0.** It works via the `iced_beacon`
   socket-based protocol (debug feature on the cockpit; comet runs as
   a sibling iced GUI). **Recommendation: do NOT adopt comet now; defer
   to whenever we move our pin to 0.15.** See § comet debugger.
4. **D3 (macOS-only CI) is partially retired by 0.14.** Headless mode
   testing is shipped, embeds Fira Sans as default font, and is built
   exactly to support cross-OS CI. The remaining drift class (cosmic-
   text + harfrust glyph rasterization) is testable with a 1-day
   spike. The deep-dive's §2.6 falsifier is still the right next move;
   what changed is the prior likelihood that the spike succeeds.
5. **Queue impact:** DROP `ui-session-journal` as currently scoped and
   replace with `ui-session-journal-iced-tester` (1 dev-day);
   CHEAPEN `ui-test-harness-ci` by ~1 dev-day (the headless plumbing
   is free); KEEP `ui-vlm-judge` / `ui-a11y-shadow` / `ui-inspect-mcp`
   unchanged (iced 0.14 ships no AccessKit, no MCP shim, no VLM
   plumbing); PROMOTE the new `ui-iced-table-panic-upstream` candidate
   to file the bug.

## What shipped in iced 0.14

Sources: the
[0.14.0 release notes](https://github.com/iced-rs/iced/releases/tag/0.14.0)
(2025-12-07), the
[master CHANGELOG](https://github.com/iced-rs/iced/blob/master/CHANGELOG.md)
(no `0.14.x` patch releases, no `Unreleased` testing/debug entries as
of fetch), and
[crates.io feature flags](https://docs.rs/crate/iced/0.14.0/features).
The deep-dive's §2.1 was directionally right but missed several
shipped surfaces.

### Testing crates and feature flags shipped at 0.14.0

| Surface | Status at 0.14.0 | Notes |
|---|---|---|
| `iced_test` (test-only crate) | shipped | what we already use in `ui-test-harness-bootstrap` v0.1 |
| `iced_test::Simulator` | shipped | side-effect-free test driver (no tasks/subscriptions run) |
| `iced_test::Emulator` | shipped | headless runtime that DOES run tasks and subscriptions |
| `iced_test::screenshot()` | shipped | what `visual_snapshots.rs` calls today |
| `iced_test::ice::Ice` | shipped | the shareable `.ice` test-script format |
| `iced_test::instruction::{Instruction, Interaction, Mouse, Keyboard, ...}` | shipped | recorder/replay primitives |
| `iced_test::run()` | shipped | execute a parsed `Ice` script against a `Program` |
| `iced_test::selector::Selector` | shipped | trait + `&str` text impl; no AccessKit yet |
| `iced_tester` (in-app recorder UI) | shipped | separate crate; `attach(program)` wraps a `Program` with a record/play/export overlay |
| `tester` feature flag on iced | shipped | toggles the recorder UI inside the application binary |
| `debug` feature flag on iced | shipped | enables F12 → comet beacon dispatch |
| `time-travel` feature flag on iced | shipped | requires `debug` + `Message: Clone`; comet scrubs the timeline |
| `hot` feature flag on iced | shipped | hot reload for layout/styles |
| `selector` feature flag on iced | shipped | richer widget selection used by `iced_tester` |
| `strict-assertions` feature flag on iced | shipped | tightens assertions in tests |

Sources:
[feature flags page](https://docs.rs/crate/iced/0.14.0/features),
[iced_test crate index](https://docs.iced.rs/iced_test/index.html),
[iced_tester crate](https://docs.rs/iced_tester/0.14.0/iced_tester/),
[Phoronix coverage](https://www.phoronix.com/news/Iced-0.14-Rust-GUI-LIbrary),
[byteiota Iced 0.14 writeup](https://byteiota.com/iced-0-14-rust-gui-gets-reactive-rendering-time-travel/).

### Key APIs the deep-dive missed (now confirmed)

- **`iced_test::ice::Ice::parse(...)`** — deserialises a `.ice` script
  (text format, exact grammar not yet documented on docs.rs; the PR
  description shows `click`, `type`, `expect` statements). Source:
  [`iced_test::ice` docs](https://docs.rs/iced_test/0.14.0/iced_test/ice/index.html).
- **`iced_test::run(ice, &program)`** — executes a parsed `Ice` script.
  Source:
  [`iced_test::run`](https://docs.rs/iced_test/0.14.0/iced_test/fn.run.html).
- **`iced_test::instruction::{Instruction, Interaction, Mouse,
  Keyboard, Key, Target, Expectation}`** — the typed recorder/replay
  enum hierarchy. `Instruction = Interact(Interaction) | Expect(Expectation)`;
  `Interaction = Mouse(_) | Keyboard(_)` and so on. Each variant has
  a `from_event()` constructor that converts a runtime
  `iced::event::Event` into the test format — i.e. **this is the
  recorder kernel**. Source:
  [`iced_test::instruction` docs](https://docs.rs/iced_test/0.14.0/iced_test/instruction/index.html),
  [`Instruction` enum](https://docs.rs/iced_test/0.14.0/iced_test/instruction/enum.Instruction.html),
  [`Interaction` enum](https://docs.rs/iced_test/0.14.0/iced_test/instruction/enum.Interaction.html).
- **`iced_test::emulator::Emulator`** — the headless runtime that
  actually pumps tasks/subscriptions. The deep-dive's §2.1 spotted
  `Simulator` but missed that **`Simulator` does not run subscriptions**
  while **`Emulator` does** (per the
  [iced_test crate-level docs](https://docs.iced.rs/iced_test/index.html):
  "simulator: Run application simulations without side effects /
  emulator: Execute your application in a headless runtime
  environment"). For our cockpit, this is the API that would let us
  drive `cockpit_live`-style integration tests without a window. The
  earlier "AccessKit shadow tree only path forward" framing
  ([deep-dive §2.7](ui-testability-deep-dive-2026-05-15.md#27-accessibility-as-a-testing-surface--the-load-bearing-pivot))
  still holds for assertion shape, but `Emulator` is the right
  *driver* for live integration coverage.
- **`iced_tester::Tester` and `iced_tester::attach(program)`** —
  the in-app overlay that lets a developer hit "record", interact
  with the cockpit, hit "stop", and export the captured sequence as
  a `.ice` file. Source:
  [`iced_tester` crate docs](https://docs.rs/iced_tester/0.14.0/iced_tester/).
  This is what the deep-dive §3.6 reinvents from scratch under
  `ui-session-journal`.

## comet debugger

Comet is a separate companion application (
[github.com/iced-rs/comet](https://github.com/iced-rs/comet),
53 stars, MIT, "Made with iced") that listens on the
`iced_beacon` socket protocol and renders a time-travel-debugger UI
for a running iced app.

### Stability and version

Comet's
[Cargo.toml master](https://raw.githubusercontent.com/iced-rs/comet/master/Cargo.toml)
shows:

```toml
[package]
name = "iced_comet"
version = "0.15.0-dev"

[dependencies]
iced.version = "0.15.0-dev"
iced_beacon = "0.15.0-dev"

[patch.crates-io]
iced.git = "https://github.com/iced-rs/iced.git"
iced.rev = "c307bd7321fd04750a9b13f62779d1b7c6e757e2"
```

**Verdict — comet does not compile against iced 0.14.0.** It tracks
iced master (0.15.0-dev) via a `[patch.crates-io]` git rev. The
README is one image / one video — no install instructions, no
documented `iced_beacon` port, no protocol spec, no security
guidance, no version-compatibility matrix. The 5-open-issue count
(per
[github.com/iced-rs/comet](https://github.com/iced-rs/comet))
suggests pre-alpha.

### How it plugs into our cockpit (if we ever adopt it)

- We would need to bump `iced = "=0.14.0"` to `iced = "=0.15.0"` once
  0.15 ships (timeline: unknown; iced's last major was a year between
  0.13 and 0.14).
- Enable the `debug` feature on the cockpit binary; that activates
  `iced_beacon` dispatch — performance metrics + (with `time-travel`)
  every dispatched `Message` over a local socket.
- Run `cargo install --git https://github.com/iced-rs/comet iced_comet`
  on the operator's machine; press F12 in the cockpit to summon the
  overlay.

### Security implications

The `iced_beacon` protocol is socket-based (port/binding not
documented publicly as of fetch). Per the deep-dive §3.1 reasoning
about `ui-inspect-mcp` — even localhost-only debug surfaces are a
non-trivial risk on a shared developer machine. We would want to
inspect `iced_beacon`'s binding code before turning on `debug` in
anything but `--features debug` opt-in (which is the default — the
flag is off in production builds). Recommended posture: **never
enable `debug` in `cockpit_live` or any production build path**;
keep it gated behind `--features debug` so cargo's resolver leaves
the socket code out of release artifacts.

### Should we adopt now?

**No.** Three reasons:

1. Pin mismatch — adopting forces a 0.14 → 0.15 lockstep migration we
   are not prepared for. The
   [ui-test-harness-bootstrap H1 falsifier](../ui-test-harness-bootstrap/feature.md#h1--tiny-skia-cpu-determinism-holds-across-two-runs-on-the-same-machine)
   was proven against 0.14; every baseline PNG in
   `crates/ui/tests/visual-baselines/` is a 0.14 fingerprint and
   would invalidate on a renderer or text-shaping bump.
2. Pre-alpha stability — 5 open issues, no docs, single-maintainer
   (hecrj). Adopting a 0.15.0-dev tool against 0.14.0 production is
   the wrong leverage.
3. Coverage redundancy — most of comet's value is "time-travel through
   a recorded session"; iced 0.14 already ships `iced_tester` + the
   `.ice` format for that, and our `Message` enum is `Clone`-friendly.
   We can reproduce 80% of comet's debugger story locally without
   the beacon.

**Defer to whenever we adopt iced 0.15.** Add a single-line
`spec/backlog.md` candidate `ui-comet-eval` with no scheduled spawn.

## Headless mode

Headless mode (
[PR #2698](https://github.com/iced-rs/iced/pull/2698)) is the
foundation that lets `iced_test::screenshot()` work today without a
window server. The deep-dive's §2.6 ("cross-platform CI revisit")
was right to flag D3 as worth revisiting; what's changed is the
**prior likelihood** of cross-OS determinism holding has gone up:

- iced 0.14 **embeds Fira Sans as default test font** in `iced_test`
  (per
  [PR #2698 description](https://github.com/iced-rs/iced/pull/2698)).
  That removes the macOS-Helvetica vs Linux-DejaVu drift class
  outright.
- The headless renderer is `iced_tiny_skia` running on CPU — same
  software-render path regardless of host OS. Per
  [tiny-skia README](https://github.com/linebender/tiny-skia)
  "expected to match Skia pixel-for-pixel". Bootstrap H1 confirmed
  byte-determinism on a single host; cross-OS byte-determinism is
  testable in 1 dev-day.
- PR #2698 explicitly calls out CI: "enables running iced applications
  in headless environments (Linux without X11/Wayland display
  servers), critical for automated testing pipelines."

### Does headless retire D3?

**Partial retirement.** Headless mode retires:

- The "no display server on Linux runners" problem (formerly the
  strongest case for macOS-only CI).
- The "macOS-specific renderer" risk (tiny-skia is CPU; no NSWindow
  involvement in the test path).

It does NOT retire:

- The **cosmic-text glyph rasterization** drift class. cosmic-text +
  HarfRust shape per-version deterministically with the font embedded
  (Fira Sans is now embedded), but a future cosmic-text bump still
  shifts pixels.
- The **Retina / scale_factor reproduction** question. Our `operator`
  slot is 3360×1890 @2.0x; that ratio is reproducible on any host
  (it's purely a `Viewport` arg to `screenshot()`), so this is
  actually a non-issue at the test layer. The real Retina concern
  is only relevant for the live `cockpit_live` binary; tests are
  exempt.

**Recommendation:** schedule the deep-dive's §2.6 + §5.4 D3 falsifier
in cycle 3 unchanged. Prior likelihood of byte-identical baselines
across Linux + macOS is now `[unverified — high]` rather than
`[unverified — uncertain]`. If the falsifier passes, D3 retires;
GitHub Actions Linux runners become the cheap default, macOS hardware
optional. If it fails, the failure class is documented and D3 stands.

## Recorder / emulator / `iced_test::Simulator`

The deep-dive's §3.6 (`ui-session-journal`) proposed:

> Add `cockpit_live --record-journal <path>` that serializes every
> `Message` (with the dispatched-at timestamp) into a TOML file. Add
> `cargo test --test journal_replay -- <path>` that deserialises,
> replays, asserts the final state matches a committed golden
> snapshot.

**iced 0.14 ships this already** under the `tester` feature flag.

### Surface area we already use (bootstrap v0.1)

- `iced_test::screenshot(&program, &theme, viewport, scale_factor,
  Duration)` → `iced::window::Screenshot { rgba, size, scale_factor }`.
  Used in
  [`visual_snapshots.rs`](../../crates/ui/tests/visual_snapshots.rs)
  for the three slot tests.
- `Simulator` (in-process driver, side-effect-free). Not used in v0.1
  — the bootstrap uses the free `screenshot()` function directly.

### Surface area NOT yet tapped

- **`Emulator`**
  ([docs](https://docs.rs/iced_test/0.14.0/iced_test/emulator/index.html)) —
  runs tasks AND subscriptions. This is the headless-driver for
  `cockpit_live`-style integration tests (currently we have
  [`cockpit_live_kill_button_writes_audit.rs`](../../crates/ui/tests/cockpit_live_kill_button_writes_audit.rs)
  which drives `ui::state::update` directly, bypassing iced's runtime
  pump entirely). Adopting `Emulator` would let that and similar
  tests assert on the full subscription path — closing the gap the
  deep-dive's §1.3 named ("loading spinner stuck because subscription
  never completes").
- **`Instruction`/`Interaction` recording**
  ([docs](https://docs.rs/iced_test/0.14.0/iced_test/instruction/index.html)) —
  `Interaction::from_event(event)` converts an `iced::event::Event`
  to a typed test instruction. The pattern: tap the cockpit's event
  stream, fan-out one branch through the normal `update`, another
  branch through `Interaction::from_event` into a `Vec<Instruction>`
  buffer, serialise to `.ice` on shutdown. No new format invention
  needed.
- **`iced_test::ice::Ice::parse` + `iced_test::run`** — read a `.ice`
  file in a test target, hand it to `run(&ice, &program)`, assert the
  expectations embedded in the script pass. This is the replay half.
- **`iced_tester::attach(program)`** — the in-app recorder *UI*.
  Wraps `iced::application(...)` with a record/play/export overlay.
  Operator presses record, clicks around, presses stop + export,
  saves `.ice`. No custom journal middleware needed in our code.

### What this means for `ui-session-journal`

**Cost drops from 4 dev-days to ~1 dev-day.** The work becomes:

- Add `iced_test = "=0.14.0"` + `iced_tester = "=0.14.0"` to
  `crates/ui/Cargo.toml` `[dev-dependencies]` (iced_test is already
  there; iced_tester is new).
- Add a `tester` feature flag to `crates/ui/Cargo.toml` that pulls
  `iced/tester` + `iced/selector` + `iced/strict-assertions`.
- Author `crates/ui/src/bin/cockpit_record.rs` (or a `--record-tests`
  arg on `cockpit_live`) that calls `iced_tester::attach(program)`.
- Author `crates/ui/tests/journal_replay.rs` that walks
  `crates/ui/tests/recorded-sessions/*.ice` and calls
  `iced_test::run(ice, &program)` on each.
- Commit the chart-canvas-overhaul incident as the first recorded
  session (per deep-dive §2.8).

**Drop the TOML self-roll plan.** It was correct given what the
deep-dive knew; the new finding obsoletes it.

## The strategies-Table panic

The pinned panic per
[gallery_bisect.rs](../../crates/ui/tests/gallery_bisect.rs) is at
`iced_tiny_skia::engine.rs:686` ("Build quad rectangle") when
`widgets::strategies::view` (which uses `iced::widget::table::Table`)
renders inside a fixed-height `gallery::cell::view` container.

### Is it fixed in 0.14.0?

**No.** Searches across the
[iced GitHub issues tracker](https://github.com/iced-rs/iced/issues),
the
[0.14.0 release notes](https://github.com/iced-rs/iced/releases/tag/0.14.0)
Fixed section, and the
[master CHANGELOG](https://github.com/iced-rs/iced/blob/master/CHANGELOG.md)
Unreleased section return **zero hits** for "Build quad rectangle",
zero hits for the engine.rs:686 location, and zero hits for "Table
panic" or "Length::Fixed" + Table. There is no 0.14.1 patch release;
no merged fix on master either.

### Prior art — same family of bug

Issue
[#2311 "tiny_skia backend panics if you have an Combobox menu with
zero search results"](https://github.com/iced-rs/iced/issues/2311)
documented an identical-class panic: `"Quad with non-normal height!"`
at `tiny_skia/src/backend.rs:162:17`, fixed via
[#2364](https://github.com/iced-rs/iced/pull/2364). Per WebFetch on
that issue: "the fix probably applied to the general rectangle
rendering logic rather than being combobox-specific, meaning other
widgets (like tables or menus) that render quads could have been
affected by the same issue."

That fix landed pre-0.14; the regression we're hitting at
`engine.rs:686` (note: different line + different file path —
`iced_tiny_skia/src/engine.rs` not `tiny_skia/src/backend.rs`)
appears to be a **new instance of the same degenerate-quad family**,
this time in the 0.14 `Table` widget's measure or layout path when
the parent container forces a height the table can't honour with
zero or one row.

### Workarounds available NOW

Two workarounds, both already named in the backlog candidate
`ui-gallery-table-cell`:

- **(a) Drop the height constraint in the gallery cell wrapper.**
  Lets the gallery cell grow to the table's natural height. Closes
  V5+ of `ui-gallery-bin`. Does NOT close any cockpit risk because
  the cockpit's strategies table is not gallery-rendered.
- **(b) Swap the strategies cell for a non-table render in the
  gallery only.** Renders a `Column` of `Row`s with the same row data.
  Loses the "we tested the table widget" guarantee but unblocks the
  gallery deterministically.

Both are ~1 dev-day. **Recommendation (a) over (b)** — keeping the
real `Table` widget in the gallery preserves the regression-bisect
value of the cell.

### Switching to wgpu instead?

The cockpit's
[`crates/ui/Cargo.toml`](../../crates/ui/Cargo.toml) line 77 pins
`iced` with `default-features = false, features = ["tiny-skia",
"thread-pool", "advanced", "canvas"]` — wgpu is explicitly disabled.
Adding `wgpu` to that feature list would let iced's
[fallback resolver](https://deepwiki.com/iced-rs/iced/4.1-wgpu-renderer)
prefer wgpu and would route around the tiny-skia panic. But: our
H1 byte-determinism falsifier is built on tiny-skia exclusively;
wgpu introduces driver / GPU-vendor non-determinism we explicitly
locked out. **Do NOT switch the renderer to dodge the panic.**

### File upstream?

**Yes.** The panic is a real bug in 0.14.0 affecting a public widget
(`iced::widget::table::Table`) wrapped in a public widget
(`Container::height(Length::Fixed(...))`). It's reproducible from
our [gallery_bisect.rs](../../crates/ui/tests/gallery_bisect.rs)
diagnostic with a 30-LOC fixture. Filing costs us nothing (we have
the repro already) and the iced maintainer historically closes
tiny-skia panic classes quickly (see #2311 → #2364 cadence).
Promote a new `ui-iced-table-panic-upstream` candidate — half a
dev-day to author the minimal repro + file the issue. Operator
can decide whether to also draft a PR.

## Impact on queued features

Per
[backlog.md ## Process / tooling](../backlog.md#process--tooling)
ordering, with cost adjustment vs the deep-dive's §5.1 idea table:

| Candidate | Deep-dive cost | Revised cost | Action | Reason |
|---|---|---|---|---|
| `ui-gallery-table-cell` (NEW) | n/a | 1 dev-day | KEEP — pick (a) | Per § The strategies-Table panic. The gallery cell wrapper drops the height constraint; preserves real-Table coverage. |
| `ui-iced-table-panic-upstream` (NEW, this dev-note) | n/a | 0.5 dev-day | PROMOTE | File the bug + minimal repro. Operator-decide whether to draft the PR. |
| `ui-vlm-judge` | 3 dev-days | 3 dev-days | KEEP | iced 0.14 ships no VLM plumbing. Layer 6 still requires `crates/llm`'s Anthropic provider. Unchanged. |
| `ui-a11y-shadow` | 7 dev-days | 7 dev-days | KEEP | [Iced issue #552](https://github.com/iced-rs/iced/issues/552) (AccessKit support) is still unmerged; the `iced_test::selector` shipped at 0.14 is text-only. Approach B (in-repo shadow) remains the only path. Unchanged. Deep-dive §2.7 / §3.5 analysis confirmed accurate. |
| `ui-inspect-mcp` | 4 dev-days | 4 dev-days | KEEP — DEFER | comet's beacon is the closest 0.14-shipped analogue but (a) requires iced 0.15, (b) lacks docs, (c) is read-only socket not MCP. The deep-dive's Q-MCP "defer to cycle 4" lock stands. |
| `ui-session-journal` | 4 dev-days | 1 dev-day | REPLACE | Rename to `ui-session-journal-iced-tester` and rescope per § Recorder / emulator. iced_tester + the `.ice` format do the heavy lifting; we author 2 small files. |
| `ui-test-harness-ci` | 5 dev-days | 4 dev-days | CHEAPEN | PR #2698 explicitly built for CI use; embedded Fira Sans removes the system-font drift class. The 1-day savings is the headless-plumbing work we no longer need. The cross-platform falsifier (deep-dive §2.6 / item O) stays at 1 dev-day. |
| `ui-test-harness-canvas-state-seeding` | 1 dev-day (backlog) | 1 dev-day | KEEP | `Emulator` could be an alternate path here (run a `CursorMoved` event through the live runtime to seed `ChartProgram::State`), but the existing `#[doc(hidden)]` constructor path is simpler. Re-evaluate if Emulator gets adopted for other reasons first. |
| `ui-test-harness-viewport-matrix` | 4 dev-days | 4 dev-days | KEEP | Unchanged. Layer 3 expansion. |
| `ui-test-harness-evaluator` | 3 dev-days | 3 dev-days | KEEP — DEFER per deep-dive §5.3 | Unchanged. |
| `ui-contrast-asserter` | 0.5 dev-days | 0.5 dev-days | KEEP | Unchanged. Pure-function test. |
| `ui-update-proptest` | 5 dev-days | 5 dev-days | KEEP | Unchanged. iced 0.14 changes nothing about `proptest-state-machine`. |
| `ui-mutants-pass` | 1 dev-day | 1 dev-day | KEEP | Unchanged. |
| Test reporter — visual-fail HTML artifact | 1 dev-day | 1 dev-day | KEEP | Unchanged. |

### Net effect on the schedule

- One candidate replaced (`ui-session-journal`).
- One candidate cheapened (`ui-test-harness-ci`).
- Two new candidates added (`ui-iced-table-panic-upstream`,
  `ui-comet-eval`).
- Eight candidates unchanged.
- Cycle-1 / Cycle-2 / Cycle-3 ordering (deep-dive §5.2) unchanged —
  the new findings reduce cost, not order.

## Migration questions for the operator

Same Q-* shape as the deep-dive's §6. All defaults pre-author the
operator's likely yes; the tradeoff column is the honest "what could
go wrong".

1. **Q-014-PIN — Stay pinned at `iced = "=0.14.0"` or track
   `iced = "0.14"` for patch releases?**
   - *Default if operator silent:* stay pinned at `=0.14.0`. Patch
     releases (if and when they ship) might inadvertently shift
     tiny-skia glyph rasterization and invalidate the 3 baseline PNGs
     in `crates/ui/tests/visual-baselines/`. The bootstrap's H1
     byte-determinism is a 0.14.0-fingerprint contract.
   - *Tradeoff:* a 0.14.1 fix for our strategies-Table panic would be
     gated behind a manual bump and a baseline re-bake. With strict
     pinning we don't get the fix for free.
   - *Recommended decision:* stay strict-pinned; bump manually only
     when the panel/baseline retake is scheduled.

2. **Q-COMET-EVAL — Schedule a 1-day eval of comet when iced 0.15
   ships, or defer indefinitely?**
   - *Default if operator silent:* defer indefinitely. Add
     `ui-comet-eval` to `spec/backlog.md` as a candidate without a
     spawn trigger; revisit only if our `ui-inspect-mcp` or
     `ui-session-journal-iced-tester` work surfaces a gap comet
     would close.
   - *Tradeoff:* if comet becomes the iced ecosystem's de-facto
     debugger and ships stable docs / port spec, deferring means
     building parallel infrastructure we could have re-used.
   - *Recommended decision:* defer with a 6-month revisit calendar.

3. **Q-TESTER-FEATURE — Add `iced/tester` as an opt-in feature flag
   on `crates/ui/`?**
   - *Default if operator silent:* yes — gated behind a new
     `record-tests` cargo feature. Off by default in production
     builds (no `iced_tester` linkage in `cockpit_live`). On for
     `ui-session-journal-iced-tester` work.
   - *Tradeoff:* one more feature flag in `crates/ui/Cargo.toml`,
     which already has `fixtures`, `live`, `in_process_cron`. Cargo
     feature combinatorics get hairier.
   - *Recommended decision:* yes — the feature surface is small and
     the alternative (rolling our own recorder) is strictly worse.

4. **Q-PANEL-UPSTREAM — File the strategies-Table panic upstream as
   a bug report only, or also draft the fix PR?**
   - *Default if operator silent:* file the bug report with the
     minimal repro pulled from
     [gallery_bisect.rs](../../crates/ui/tests/gallery_bisect.rs);
     do NOT draft the PR. iced's quad-panic family has a 3-month
     average fix-cadence based on
     [#2311 → #2364](https://github.com/iced-rs/iced/pull/2364)
     timing; if hecrj doesn't close the issue in 6 weeks, revisit.
   - *Tradeoff:* drafting the PR up front gets the fix to us
     sooner; it also commits an analyst-day (read tiny-skia engine
     code, propose the dimension-clamp). Probably more dev-time
     than we'd save.
   - *Recommended decision:* file only; revisit if no upstream
     activity in 6 weeks.

5. **Q-D3-RELITIGATE — Has headless mode (PR #2698) changed enough
   about D3's tradeoffs to fold the cross-platform falsifier into
   `ui-test-harness-ci` cycle, or stay with the deep-dive §5.4
   "1-day spike in cycle 3" plan?**
   - *Default if operator silent:* stay with the deep-dive's plan
     — keep the cross-platform falsifier as a discrete 1-day spike
     in cycle 3. PR #2698 raises the prior likelihood of D3
     retiring, but doesn't change the test design.
   - *Tradeoff:* if we're confident D3 retires, we could spend
     `ui-test-harness-ci`'s 4 dev-days targeting Linux/Windows
     out of the gate and save the spike. But "confident" is a
     belief, not a measurement — the spike *measures*.
   - *Recommended decision:* unchanged from deep-dive Q-D3-REVISIT.

## Sources

External URLs cited inline above; aggregated for the operator's
sweep.

- **iced 0.14 release + changelog:**
  [0.14.0 release notes](https://github.com/iced-rs/iced/releases/tag/0.14.0),
  [iced master CHANGELOG](https://github.com/iced-rs/iced/blob/master/CHANGELOG.md),
  [crates.io feature flags](https://docs.rs/crate/iced/0.14.0/features),
  [Phoronix coverage](https://www.phoronix.com/news/Iced-0.14-Rust-GUI-LIbrary),
  [byteiota Iced 0.14 writeup](https://byteiota.com/iced-0-14-rust-gui-gets-reactive-rendering-time-travel/).
- **Testing crates:**
  [iced_test 0.14 docs (docs.rs)](https://docs.rs/iced_test/0.14.0/iced_test/),
  [iced_test docs (docs.iced.rs)](https://docs.iced.rs/iced_test/index.html),
  [iced_test::ice](https://docs.rs/iced_test/0.14.0/iced_test/ice/index.html),
  [iced_test::emulator](https://docs.rs/iced_test/0.14.0/iced_test/emulator/index.html),
  [iced_test::instruction](https://docs.rs/iced_test/0.14.0/iced_test/instruction/index.html),
  [iced_test::instruction::Instruction](https://docs.rs/iced_test/0.14.0/iced_test/instruction/enum.Instruction.html),
  [iced_test::instruction::Interaction](https://docs.rs/iced_test/0.14.0/iced_test/instruction/enum.Interaction.html),
  [iced_test::run](https://docs.rs/iced_test/0.14.0/iced_test/fn.run.html),
  [iced_tester 0.14 docs](https://docs.rs/iced_tester/0.14.0/iced_tester/).
- **Pull requests:**
  [PR #2698 Headless mode testing](https://github.com/iced-rs/iced/pull/2698),
  [PR #3059 First-class end-to-end testing](https://github.com/iced-rs/iced/pull/3059),
  [PR #2879 comet debugger and devtools foundations (Added entry)](https://github.com/iced-rs/iced/blob/master/CHANGELOG.md),
  [PR #2910 Time Travel Debugging](https://github.com/iced-rs/iced/pull/2910).
- **comet:**
  [github.com/iced-rs/comet](https://github.com/iced-rs/comet),
  [comet Cargo.toml master](https://raw.githubusercontent.com/iced-rs/comet/master/Cargo.toml).
- **The strategies-Table panic prior art:**
  [iced issue #2311 "Quad with non-normal height"](https://github.com/iced-rs/iced/issues/2311),
  [iced PR #2364 (fix for #2311)](https://github.com/iced-rs/iced/pull/2364),
  [iced::widget::table::Table docs](https://docs.rs/iced/latest/iced/widget/table/struct.Table.html),
  [iced_tiny_skia docs](https://docs.iced.rs/iced_tiny_skia/index.html).
- **Renderer architecture:**
  [DeepWiki — iced wgpu renderer](https://deepwiki.com/iced-rs/iced/4.1-wgpu-renderer),
  [tiny-skia README](https://github.com/linebender/tiny-skia).
- **Accessibility (still unmerged in iced):**
  [iced issue #552 accessibility support](https://github.com/iced-rs/iced/issues/552).
- **Predecessor dev-notes:**
  [`ui-testability-deep-dive-2026-05-15.md`](ui-testability-deep-dive-2026-05-15.md),
  [`ui-testing-direction-2026-05-12.md`](ui-testing-direction-2026-05-12.md),
  [bootstrap feature.md](../ui-test-harness-bootstrap/feature.md).

## Changelog

- 2026-05-15 (analyst): initial draft. Targeted iced 0.14 changelog
  analysis. Headline findings: (1) the strategies-Table panic is
  unfixed and unmentioned upstream — file it + ship workaround (a) in
  `ui-gallery-table-cell`; (2) `iced_tester` + `.ice` format
  obsolete the deep-dive's `ui-session-journal` self-roll —
  rescope to a 1-day adapter; (3) comet is iced-0.15-dev-only — defer;
  (4) headless mode + embedded Fira Sans cheapen `ui-test-harness-ci`
  by 1 dev-day. Five operator Q-* opened (Q-014-PIN, Q-COMET-EVAL,
  Q-TESTER-FEATURE, Q-PANEL-UPSTREAM, Q-D3-RELITIGATE). No backlog
  edits in this pass — dev-note is research only; backlog candidates
  surfaced for operator-decided promotion.
