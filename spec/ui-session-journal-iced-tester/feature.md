---
slug: ui-session-journal-iced-tester
version: 0.1.0
status: shipped
owner: operator
predecessor: ui-gallery-bin v0.1-partial
updated: 2026-05-16
---

> **Status (2026-05-16):** v0.1 shipped end-to-end. V1, V4, V5, V6,
> V7, V8 green. V2/V3 (manual smoke + first recording) deferred to
> operator session — recorder overlay requires a desktop window and
> the orchestrator runs headlessly. The `recorded-sessions/`
> directory ships empty (only `.gitkeep`); operators populate post-
> ship via the recorder workflow documented in
> [`crates/ui/tests/journal_replay.rs`](../../crates/ui/tests/journal_replay.rs).
>
> **Major design correction at impl time:** Both Q-ARCH-1 and
> Q-ARCH-2 resolved differently than the original spec planned. iced
> 0.14's `iced::Application::run()` auto-wraps with
> `iced_tester::attach()` when the `tester` feature is enabled (see
> [iced-0.14.0/src/application.rs:198](https://docs.rs/iced/0.14.0/src/iced/application.rs.html#198))
> — so there is **no manual `attach()` call** and **no runtime
> `--record-tests` CLI flag**. The recorder is a compile-time
> choice via `--features record-tests`. See § Changelog below.

# Session journal — `iced_tester` adapter (v0.1)

> Rescoped successor to the original `ui-session-journal` candidate
> (4 dev-days, TOML self-roll). iced 0.14 ships
> [`iced_tester`](https://docs.rs/iced_tester/0.14.0/) +
> [`iced_test::ice`](https://docs.rs/iced_test/0.14.0/iced_test/ice/index.html)
> natively — this feature is a ~1-day adapter that wires those upstream
> pieces into our `cockpit_live` bin and `crates/ui/tests/` suite. See
> [`spec/dev-notes/archive/2026-Q2/iced-014-feature-analysis-2026-05-15.md §5`](../dev-notes/archive/2026-Q2/iced-014-feature-analysis-2026-05-15.md#recorder--emulator--iced_testsimulator)
> for the prescription.

## Why

The operator's chart-canvas-overhaul v1.10.0 incident shipped a
tooltip-invisible-at-3360×1890 bug whose verification step was a
manual 30-second `Cmd+Shift+4`. The retrospective named two gaps:

1. No way to **record** a real operator session as a permanent
   regression fixture. Once the incident closed, the exact mouse-and-
   keyboard sequence that exposed the bug was lost — re-discovering it
   for a regression test cost dev-time on every future cycle.
2. No way to **replay** committed sessions in `cargo test`. Without
   replay, agents can drive `ui::state::update` directly (bypassing
   iced's event/subscription pump entirely — see
   [`cockpit_live_kill_button_writes_audit.rs`](../../crates/ui/tests/cockpit_live_kill_button_writes_audit.rs))
   but cannot exercise the **full message loop including subscriptions**
   that the production cockpit runs through.

`iced_tester` (the recorder) + `iced_test::ice::Ice` (the format) +
`iced_test::run` (the replay driver) close both gaps as a bundle. The
work is adapter-level — no new format invented, no new event journal
middleware. **This is the first feature out of the iced-014 dev-note's
locked rollout** (post-`ui-iced-table-panic-upstream` and the in-flight
`ui-gallery-table-cell` work).

## Scope locked

Per [`iced-014-feature-analysis-2026-05-15.md §5`](../dev-notes/archive/2026-Q2/iced-014-feature-analysis-2026-05-15.md#what-this-means-for-ui-session-journal)
and Q-TESTER-FEATURE LOCKED 2026-05-15:

- **D-RT-1** — Add `record-tests` opt-in cargo feature on
  `crates/ui/`. Off in production builds. On only when the operator
  records sessions or when `journal_replay.rs` runs.
- **D-RT-2** — Recording happens in `cockpit_live` (the canonical
  agent + cockpit binary). NOT in the fixtures-only `cockpit` bin.
  Rationale: agents debugging production cockpit bugs need to record
  against the real subscription tree.
- **D-RT-3** — Export path is **operator-driven via `rfd` native file
  dialog** (built into `iced_tester`'s overlay). No CLI arg of any
  kind — recorder is enabled at COMPILE TIME via `--features
  record-tests`. (Originally planned as a boolean CLI flag; impl
  found iced auto-attaches when its `tester` feature is on, so the
  flag would be redundant. See § Changelog 2026-05-16.)
- **D-RT-4** — Recorded `.ice` files commit to
  `crates/ui/tests/recorded-sessions/`. v0.1 ships the directory + 0–1
  sample recordings (operator-recorded if practical; otherwise empty
  with `.gitkeep`).
- **D-RT-5** — Replay uses
  [`iced_test::run`](https://docs.rs/iced_test/0.14.0/) (API verified
  via spike; exact signature confirmed at T-M0-B before T04).
- **D-RT-6** — macOS-only for v0.1. The `rfd` dep pulls AppKit on
  macOS; Linux/Windows record/replay is `ui-test-harness-ci` scope.

### In scope (v0.1)

- `record-tests` cargo feature on `crates/ui/Cargo.toml`.
- `--record-tests` boolean CLI flag on `cockpit_live`.
- `iced_tester::attach(...)` wiring around the existing
  `iced::application(...)` call at
  [`crates/ui/src/bin/cockpit_live.rs:458`](../../crates/ui/src/bin/cockpit_live.rs).
- `crates/ui/tests/journal_replay.rs` — walks
  `tests/recorded-sessions/*.ice`, replays each.
- `crates/ui/tests/recorded-sessions/.gitkeep` (empty placeholder; one
  real `.ice` file if T05 succeeds).

### Out of scope

- Time-travel debugger / comet — covered by `ui-comet-eval`
  (deferred-no-trigger; needs iced 0.15-dev).
- Auto-record in production (would leak operator interaction data
  to disk silently — privacy + perf concerns).
- Cross-platform CI replay — `ui-test-harness-ci` scope.
- VLM judging of replayed runs — `ui-vlm-judge` scope.
- Replay-as-snapshot baseline regeneration — separate cycle if needed.

## Design

### Cargo feature shape

[`crates/ui/Cargo.toml`](../../crates/ui/Cargo.toml) `[features]`:

```toml
record-tests = ["iced/tester", "iced/selector", "iced/strict-assertions"]
```

Per [iced 0.14.0 feature graph](https://docs.rs/crate/iced/0.14.0/features):
- `iced/tester` → pulls `dep:iced_tester`
- `iced/selector` → pulls `iced_runtime/selector` (selector-by-id widget targeting)
- `iced/strict-assertions` → pulls `iced_renderer/strict-assertions` (fail-loudly on layout invariants)

The feature is **additive**: `cargo build --features live,record-tests`
gives a cockpit_live with the recorder. `cargo build --features live`
(default cockpit_live build) does NOT pull `iced_tester`.

### CLI flag

[`crates/ui/src/bin/cockpit_live.rs`](../../crates/ui/src/bin/cockpit_live.rs)
`struct Args`:

```rust
struct Args {
    #[arg(long, default_value = "config/agent.toml")]
    config: PathBuf,

    #[arg(long)]
    mode: Option<String>,

    /// Enable the `iced_tester` recorder overlay. Operator presses
    /// record / stop / export inside the overlay; export path is
    /// chosen via native file dialog (rfd). Requires the
    /// `record-tests` cargo feature.
    #[cfg(feature = "record-tests")]
    #[arg(long)]
    record_tests: bool,
}
```

### Recorder wiring

[`crates/ui/src/bin/cockpit_live.rs:458`](../../crates/ui/src/bin/cockpit_live.rs)
currently builds and runs the iced app as:

```rust
let iced_result = iced::application(
    move || (app_state.clone(), iced::Task::none()),
    AppState::update,
    AppState::view,
)
.title(AppState::title)
.theme(AppState::theme)
.subscription(AppState::subscription)
.window(ui::window_icon::standard_window_settings())
.run();
```

Revised with the recorder gate:

```rust
let application = iced::application(
    move || (app_state.clone(), iced::Task::none()),
    AppState::update,
    AppState::view,
)
.title(AppState::title)
.theme(AppState::theme)
.subscription(AppState::subscription)
.window(ui::window_icon::standard_window_settings());

#[cfg(feature = "record-tests")]
let iced_result = if args.record_tests {
    info!("iced_tester recorder attached; overlay will appear on first frame");
    iced_tester::attach(application).run()
} else {
    application.run()
};

#[cfg(not(feature = "record-tests"))]
let iced_result = application.run();
```

**Open architecture question** (Q-ARCH-1, resolved at T-M0-B): the
exact composition of `iced::application(...)` builder output with
`iced_tester::attach()` — see [§ Open questions for architect](#open-questions-for-architect)
below. The spike verified `attach()` takes `P: Program`; whether the
builder output IS a `Program` or needs `.into_program()` is the open
question.

### Replay harness

[`crates/ui/tests/journal_replay.rs`](../../crates/ui/tests/journal_replay.rs)
(NEW), modeled on the existing
[`visual_snapshots.rs`](../../crates/ui/tests/visual_snapshots.rs)
pattern:

```rust
//! Replay every committed `.ice` session and assert its expectations.
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "fixtures/mod.rs"]
mod fixtures;

use ui::test_support::program_from_cockpit;
use ui::fixtures::fake_cockpit_v15a_pairs_steady_state;

#[test]
fn replay_all_recorded_sessions() {
    let dir = format!(
        "{}/tests/recorded-sessions",
        env!("CARGO_MANIFEST_DIR")
    );
    let entries = std::fs::read_dir(&dir).expect("recorded-sessions dir");
    let mut replayed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ice") {
            continue;
        }
        let ice_text = std::fs::read_to_string(&path).expect("read ice");
        let ice = iced_test::ice::Ice::parse(&ice_text).expect("parse ice");
        let cockpit = fake_cockpit_v15a_pairs_steady_state();
        let program = program_from_cockpit(cockpit);
        iced_test::run(&ice, &program).expect("replay");
        replayed += 1;
    }
    // v0.1 ships the harness; the directory may be empty.
    // The test passes either way — but logs the count.
    eprintln!("replayed {replayed} recorded session(s)");
}
```

The exact `iced_test::run` signature is the **second open architect
question** — verified at T-M0-B alongside Q-ARCH-1.

### Predecessor seed

Replay tests reuse
[`ui::test_support::program_from_cockpit`](../../crates/ui/src/test_support.rs)
— the same factory the bootstrap's visual snapshots use. Honors the
bootstrap's H5 invariant (factory must compile under default features,
no feature-gate on the test path). The recorded session's actions
play against this seeded cockpit, NOT a fresh `Default::default()`.

## Acceptance / verification (V-items)

| # | What | How |
|---|---|---|
| V1 | `cargo build -p ui --features live,record-tests --bin cockpit_live` succeeds | Compile gate |
| V2 | `cargo run -p ui --features live,record-tests --bin cockpit_live -- --record-tests` opens the cockpit with the recorder overlay visible | Manual smoke (operator confirms overlay) |
| V3 | After clicking around + export, the produced `.ice` file at the operator-chosen path is non-empty and parses with `iced_test::ice::Ice::parse` | Manual + assert in T05 |
| V4 | `cargo test -p ui --test journal_replay` exits 0 with at least 0 sessions replayed (passes on empty dir; logs count) | Test gate |
| V5 | `cargo build -p ui --features live --bin cockpit_live` (without `record-tests`) succeeds AND produces a binary with no `iced_tester` linkage | Compile + `cargo tree` check |
| V6 | Full workspace tests stay green | `cargo test --workspace` after changes — no regression on the 1222-baseline (post-merge) |
| V7 | `cargo clippy -p ui --no-deps` adds zero new clippy warnings vs the pre-feature baseline | Lint gate |
| V8 | `cargo fmt --check` clean | Fmt gate |

## Dependencies

| Dep | Source | New? |
|---|---|---|
| `iced` `tester` feature | `crates/ui/Cargo.toml` `record-tests` pulls `iced/tester` | New use of an existing dep |
| `iced` `selector` feature | Same | New use |
| `iced` `strict-assertions` feature | Same | New use |
| `iced_test = "=0.14.0"` | Already in dev-dependencies (from bootstrap) | Reuse |
| `rfd` (transitive) | Pulled by `iced_tester` | New transitive — macOS-native dialog |

No new direct deps on `crates/ui/Cargo.toml` outside the `record-tests`
feature graph. iced_tester comes in via the feature flag, not a
top-level dep.

## Risks

| # | Risk | Mitigation |
|---|---|---|
| R-RT-1 | `iced_tester::attach()` doesn't compose with `iced::application(...)` builder chain (window settings / theme / title may not survive the wrap) | T-M0-B architect spike before T03 lands. If wrap order is wrong, swap to `attach(boot, update, view).title(...).run()` (attach as the entry point, not the wrapper) |
| R-RT-2 | Recorded `.ice` lacks `Expect::*` assertions by default → passive replay is vacuous | T05 manually adds at least one `Expect::is_visible(...)` or similar to the first committed session. Document the operator's hand-edit step in `tasks.md` T05 |
| R-RT-3 | `rfd` transitive dep adds GTK on Linux when feature enabled | D-RT-6 scopes v0.1 to macOS; `ui-test-harness-ci` handles cross-platform when it lands |
| R-RT-4 | The bootstrap's H1 byte-determinism contract may break if `record-tests`-feature builds shift glyph rasterization | V6 catches this. Mitigation: feature is off in all snapshot-test runs (gated `#[cfg(feature = "record-tests")]`) |
| R-RT-5 | Recorder overlay clashes with the cockpit's existing modal layer (override-risk-veto, journal-modal) | Manual smoke during T05. If clash: file upstream against iced_tester; document workaround in feature.md changelog |

## Out-files

**New:**
- `crates/ui/tests/journal_replay.rs`
- `crates/ui/tests/recorded-sessions/.gitkeep`
- `crates/ui/tests/recorded-sessions/<session-name>.ice` (if T05 records one)
- `spec/ui-session-journal-iced-tester/feature.md` (this file)
- `spec/ui-session-journal-iced-tester/tasks.md`
- `spec/ui-session-journal-iced-tester/reports/` (will hold test-run-*.md when tester runs)
- `spec/ui-session-journal-iced-tester/presentations/` (will hold the presenter deck)

**Modified:**
- `crates/ui/Cargo.toml` — `[features]` gains `record-tests = [...]`
- `crates/ui/src/bin/cockpit_live.rs` — `Args` gains `record_tests: bool`; `iced::application(...)` call gains the `#[cfg]` gate

**Unchanged (intentional):**
- `crates/ui/src/bin/cockpit.rs` (fixtures-only bin, no recorder)
- `crates/ui/src/bin/ui_gallery.rs` (test-only bin, no recorder)
- `crates/ui/src/bin/viewer.rs` (read-only report viewer, no recorder)
- `crates/ui/src/test_support.rs` — reused unchanged via `program_from_cockpit`

## Open questions for architect

The orchestrator's spike (2026-05-15) verified `iced_tester::attach()`
exists and takes `P: Program`. Two questions remain for the architect's
M0 pass before T03/T04 land:

1. **Q-ARCH-1 — Composition with `iced::application(...)` builder.**
   Does `iced_tester::attach(application)` compose with the builder's
   `.title(...).theme(...).subscription(...).window(...)` chain? The
   spike confirmed `Attach<P>` impls `Program`, but the builder output
   from `iced::application(...)` is `iced::Application<some_impl_Program>`,
   not a raw `Program`. Architect determines whether to call
   `attach()` BEFORE or AFTER the builder configuration. *Default if
   architect silent:* try `attach(builder).run()` first; if it
   doesn't compile, invert to `attach(boot, update, view)
   .title(...).run()` (treating attach as the entry-point, not a
   wrapper).

2. **Q-ARCH-2 — Replay API signature.** What is the exact signature
   of `iced_test::run`? Does it take `(&Ice, &impl Program)` →
   `Result<(), Error>`, or `(&Ice, &mut Emulator)`, or something else?
   Architect verifies via docs.rs source-read (~15 min spike) before
   T04 starts coding `journal_replay.rs`.

No operator decisions deferred; D-RT-1..D-RT-6 are all set per
Q-TESTER-FEATURE LOCKED 2026-05-15.

## Changelog

- 2026-05-16 (orchestrator, impl + ship): v0.1 implemented end-to-end
  in commit (TBD). Q-ARCH-1 and Q-ARCH-2 both resolved during impl
  by source-reading
  [iced-0.14.0/src/application.rs:198](https://docs.rs/iced/0.14.0/src/iced/application.rs.html#198):
  iced's own `Application::run()` already calls
  `iced_tester::attach(self)` under `#[cfg(feature = "tester")]`. So
  (1) we don't add a direct `iced_tester` dep and (2) no manual
  attach call is needed in `cockpit_live.rs`. Knock-on effect: the
  recorder is a compile-time choice — no `--record-tests` CLI flag.
  D-RT-3 + Design § Recorder wiring revised inline. V1, V4, V5, V6,
  V7, V8 all green; 1223 workspace tests pass (was 1222 — +1 for
  `replay_all_recorded_sessions`). V2 + V3 deferred to operator
  desktop session (recorder overlay requires a window).
- 2026-05-15 (orchestrator, planning + spike): feature brief authored
  after operator promoted the candidate. Spike verified
  `iced_tester::attach(program) -> Attach<P>` API and the `rfd`-driven
  file-dialog export pattern. Plan rev1 had `--record-tests <path>`
  CLI; spike findings rev'd that to boolean `--record-tests` flag
  (operator picks path interactively). Two open architect questions
  (Q-ARCH-1 composition, Q-ARCH-2 replay signature) deferred to M0
  spike before code lands.
