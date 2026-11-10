# Story 6.9: cockpit-cross-platform

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the cockpit on Linux/Windows: source shipped + macOS-verified; CI ACTIVATED 2026-07-10 (P7) and narrowed to TWO legs 2026-11-09 (ADR-0102) - the Windows lane going green is the open in-progress work,
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

## Acceptance Criteria

1. **AMENDED 2026-11-10 by ADR-0102 — see § "What the two-leg narrowing did to this story".**
   **Given** the live two-leg matrix `{macos-latest, windows-latest}`, **when** both lanes
   go green on a push to `main`, **then** this story flips to done - until then it is
   honestly in-progress. The ORIGINAL wording asked for *"the Linux/Windows lanes go
   green"*, which became permanently unsatisfiable when the operator removed the Linux lane:
   a story must not be left holding an AC whose subject no longer exists.
2. **Linux is recorded as NO LONGER CI-VERIFIABLE, not as satisfied.** `crates/ui`'s
   `x11`/`wayland` iced features remain load-bearing for a Linux build (25 errors without
   them, task `#11`) and nothing compiles that graph any more. ADR-0102's alternatives
   section costs the one mechanism that would restore the guard - a build-only
   `cargo check -p ui --features fixtures` Linux leg - and explains why it was not added.
   That is the route back, and it is a new decision, not this story's remit.
3. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## What the two-leg narrowing did to this story

**2026-11-10.** On 2026-11-09 the operator asked for the Ubuntu leg removed and CI made
faster; it shipped in `4777982f` and is now recorded as **ADR-0102** (it shipped without
one, which is the AD-18 violation that ADR repairs). AD-13 is amended in the same pass.

This story's subject is *"the cockpit on Linux/Windows"*, so the narrowing lands directly
on it, and the honest accounting is three lines rather than one:

- **The Windows half is intact** and is now the whole of the open work. It had never been
  confirmable for a different reason - CI was red on every push for fifteen days
  (25 consecutive runs, 2026-09-27 -> 2026-10-11) with all three OSes failing identically,
  so "Windows is broken" and "everything is broken" were indistinguishable. That is no
  longer the case.
- **The Linux half is discharged by deletion of its subject**, the same shape as bug-log
  `#172` (one unexplained red of a step that no longer exists - retired, not diagnosed). The
  story cannot be asked to turn green a lane the operator removed. What it CAN do, and now
  does, is record that Linux portability is consequently unguarded rather than achieved.
- **Nothing here claims Linux works.** The source-level portability work this story did
  (the `windows` crate stanza, the `libc::__error()` -> `raw_os_error()` fix, rustls-tls)
  is still in the tree and was green on Ubuntu before the leg went. It is simply no longer
  *verified* by anything, and a future regression in it will be invisible until a human
  builds on Linux.

### The closure fork — written down 2026-11-10, awaiting one ruling

AC1's bar was met the same day it was written: on run `42762269706` (a push to `main`)
**`macos-latest` finished 19/19 with zero failures** and `windows-latest` was on its final
step with none. By the letter of AC1 this story is closable.

**I am not closing it, and the reason is this session's own subject.** The story is named
*cockpit-cross-platform* and its first line asks for *"the cockpit on Linux/Windows"*. One of
those two platforms is now **unverifiable by construction** — nothing builds the Linux
dependency graph any more (D3 of ADR-0102). A `done` on a cross-platform story whose Linux
half nobody can check is exactly the status-versus-reality gap bug-log `#92`, `#188` and
`#189` are about: honest in the body, misleading to anyone reading the board.

But leaving it `in-progress` for ever is the **other** defect — `#92` sat `open` for 80 days
after it was fixed, because no artifact said so. Neither default is right, so this is a scope
question rather than a bookkeeping one, and scope on a story belongs to the operator:

| option | what it records |
|---|---|
| **(a) close as Windows-complete** | the story delivered what remains checkable; Linux portability becomes a NEW backlog item owning ADR-0102's costed build-only `cargo check` leg. Honest if the closing note says Linux is unguarded, not working. |
| **(b) keep open pending the Linux leg** | the story stays the home for the whole original ask, and closes when something builds Linux again. Honest, but the board carries an item nothing is working on. |
| **(c) close and drop Linux from scope** | the cleanest board, and the one that loses the information — I do not recommend it. |

The orchestrator's preference was **(a)**: it is the only option where the board matches
reality on both halves, and it leaves the Linux work priced rather than implied.

### RULED (a) — 2026-11-10, operator

**Closed as Windows-complete. `Status: done`, trace row `shipped-partial`.**

`shipped-partial` and not `shipped` is the whole of the honesty here. Run `42762269706`
concluded **success on both legs** with zero failing steps — macOS **19/19**, including the
three steps that had never run off Ubuntu (clippy, the minimal `--no-default-features` build,
and the ADR-0097 figure reproduction) — so what remains checkable was delivered and checked.
**Linux was not tested and cannot be**, and this row says so rather than implying otherwise by
a bare `done`.

Closing evidence:
`evidence/cockpit-cross-platform/reports/test-2026-11-10-1054-UTC-cockpit-cross-platform.md`
— § 7 states the scope limitation as a finding rather than an aside, and § 6 records that the
first cached run was **slower** on both legs, so nothing here may be cited as a cache win.

**Linux portability is now a backlog item**, owning ADR-0102's costed build-only
`cargo check -p ui --features fixtures` leg — the one mechanism that would put the
`x11`/`wayland` features back behind a gate instead of a paragraph.

## Tasks / Subtasks

- [ ] `cockpit-cross-platform` 0.1.0 - the base feature (in-progress)

## Dev Notes

- Source feature folder: `spec/cockpit-cross-platform/` - frontmatter status **`in-progress`** (verbatim), version `0.1.0`, updated `2026-06-15`.
- Status mapping: `in-progress` -> `in-progress` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Deferred / not built (by decision) — superseded: CI activated 2026-07-10 (PRD §14 assumption).
- Provenance: `git log -- spec/cockpit-cross-platform` (full narrative); reports under `evidence/cockpit-cross-platform/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### CI shakeout — diagnostic leads (orchestrator, 2026-08-12, code-review burn-down)

Not this story's review; recorded here because 6-9 owns the shakeout and these were found while auditing why CI has been red on every run since activation.

**Observed failure pattern, stable across runs** (checked `31538760015` from before the burn-down and `31571952922` after — identical, so the burn-down did not cause or worsen it):

| leg | fails at | note |
|---|---|---|
| ubuntu-latest | **`Build ui (fixtures)`** | exit 101; governance gates (anchors + spec-lint) pass first |
| macos-latest | `Test workspace (non-ui crates)` | build step passes |
| windows-latest | `Test workspace (non-ui crates)` | build step passes |

Job logs are **not readable without repo admin** (`gh run view --log-failed` → `HTTP 403: Must have admin rights`), so the leads below are derived from source, not from the failure text. Whoever picks this up with admin should read the log FIRST — it likely names the cause in one line and makes all of this moot.

**Lead 1 — `ci.yml` contradicts this project's own runbook.** `docs/runbooks/cockpit-cross-platform.md` § 1 (Headless display — Q5) states plainly: *"No `libwayland-dev` is needed (the x11rb backend is preferred by winit when both X11 and Wayland headers are absent; **adding `libwayland-dev` may switch to the Wayland backend — test before adding**)."* The workflow installs `libwayland-dev` anyway, on a runner whose only display server is `xvfb` — an **X11** virtual framebuffer with no Wayland compositor. The runbook flagged this exact risk as untested; nobody tested it. (Caveat that keeps this a lead rather than a diagnosis: a backend switch explains a *runtime* failure, and the observed failure is at *build*. It may still bite once the build is fixed, so it is worth removing regardless.)

**Lead 2 — the dep list was never validated and says so.** The same section carries `Q5 validation note: "...based on winit 0.30.x documentation and x11rb requirements. It was researched at v0.1 authoring time and will be validated on the first GitHub Actions ubuntu-latest run"` and closes with *"Update this runbook after the first green CI run with the confirmed dep list."* **There has never been a green run**, so the list is still the unvalidated 2026-06 research guess. Candidates absent from it that winit/X11 builds commonly need: `libxcursor-dev`, `libxi-dev`, `libxrandr-dev`.

**Ruled out at source** (so the next person need not re-check): the `ui` crate has **zero** `target_os` gates in `crates/ui/src/`, so no Linux-specific source path exists; and `iced` is pinned `default-features = false` with `tiny-skia` (pure-software CPU rasterizer), so no OpenGL/wgpu/Vulkan/mesa dependency is involved — do not chase mesa packages.

**Also worth fixing while here:** the macOS/Windows legs fail at `Test workspace (non-ui crates)`, which is the data-dependent-test problem this story already names. Every corpus-dependent test added during the 2026-08 review burn-down was deliberately `#[ignore]`-gated (and the one non-gated addition builds its own parquet in a `TempDir`), so the burn-down neither caused nor worsened that leg — orchestrator-verified.

### References

- Trace: `REQ-COCKPIT-CROSS-PLATFORM-001` (state=`shipped-partial`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 6 (Remediation, Infra & Governance (P0-P8, lints, BMAD migration))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List

## Product-review fold-in (2026-08-04)

- **The macOS pixel gate is currently red at clean HEAD (62 baseline comparisons)** —
  glyph-localized system-font rasterization drift, proven change-independent by a stash
  A/B (`docs/dev-notes/visual-baseline-drift-2026-07-27.md`). Practical consequence: the
  application's appearance is unverified and a genuine regression would land invisibly.
- **Fix ORDER is load-bearing:** enable the embedded default font (the `fira-sans` feature
  exists but is not in defaults) FIRST — that makes glyph rasterization repo-deterministic
  and independent of the OS font DB — and only THEN re-baseline once, with per-screen human
  approval per `docs/dev-notes/iced-ui-render-verification.md`. Re-baselining on the OS font
  re-arms the same bomb for the next OS update, and would also block the cross-OS baseline
  work this story wants.
- This is H1 of the cross-platform program: the embedded font is the prerequisite for ever
  having Linux/Windows baselines at all, not just for un-sticking macOS.

## CI shakeout — measured status (orchestrator, 2026-08-29)

The 2026-08-12 leads above are **superseded**: the logs became readable and the
failures are now named rather than inferred. `gh run --log-failed` still returns
`HTTP 403` without repo admin, so `scripts/ci_run_annotated.sh` was added — it
emits failing test names, panic lines and build errors as `::error::`
ANNOTATIONS, which live in the run's metadata rather than its log and are
readable without admin.

**Two structural masks were removed first, and both had been hiding everything
else:**

1. **A failing step cancelled every step below it.** With `Test workspace` red,
   steps 9-12 reported `skipped` — including *"Test ui (full suite, macOS
   canonical gate)"*. AD-10's canonical pixel gate had not executed on a single
   commit. The four UI steps now carry `always() &&` before their OS condition.
   A failing step still fails the job; the run just stops hiding what else broke.
2. **The annotation budget is ~10 per step.** The first version spent it on bare
   test names and surfaced zero panics on the leg with the most failures. Order
   is now summary -> panics-with-assertion-text -> build errors -> names ->
   explicit elision notice.

### Fixed this pass

| failure | legs | cause |
|---|---|---|
| `Build ui (fixtures)` | ubuntu | `default-features = false` on iced left `iced_winit` with no Linux backend |
| `compile_fail_tests` | macOS, ubuntu | CI ran `@stable` (1.98.0) vs the canonical box's 1.94.1; trybuild pins exact rustc diagnostics. **ADR-0091** pins the toolchain |
| `windows_determinism_on_real_data` | all three | guard probed `data/binance/` (dir tracked) instead of the parquet corpus (gitignored) |
| `lab_binance_*` x3 | linux, windows | same defect: probed `REVISION.toml`, the ONE tracked path under `data/binance` |
| Governance gates | ubuntu | a TRACKED audit note linked to an UNTRACKED one; committing it also closed the 08-24 audit's own finding F-F |
| Windows annotated nothing | windows | grep called the log binary; `-a` |
| `t1937*` anchors | windows | **bug-log #97** — no `.gitattributes` text rules, so `core.autocrlf=true` rewrote the anchored corpus: 3103 bytes/81 CRLF vs 3022/0. The gate scripts got CRLF too, so `verify_anchors.sh` died with `set: pipefail: invalid option name` — on Windows the gate could not even run |

### Still open

- **windows**: ~10 workspace tests (`t1003_*`, `t1006_*`, `t1414_*`,
  `aggregator_*`). Their assertion text was never annotated (budget bug, now
  fixed) — the next run should name them.
- **windows + linux**: `audit_aggregator_handles_10k_event_storm`.
- **macOS**: the 48-file visual drift — RESOLVED 2026-09-16 (ADR-0093, below). The
  one-time re-baseline is DONE: all 56 baselines re-captured under the embedded face
  with per-screen operator approval, and a second consecutive run reproduced them
  byte-identically.
- `leaderboard_scorecard_render` — bug-log **#96**, FIXED 2026-09-16 (ADR-0092): the
  credibility block leads the pane, and the gate measures the block instead of the
  scrollbar.

### ⚠ H1 IS WRONG AS WRITTEN — the font prerequisite does not do what it claims

The "Product-review fold-in (2026-08-04)" above says to *"enable the embedded
default font (the `fira-sans` feature exists but is not in defaults)"* FIRST, and
calls it the prerequisite for everything cross-platform. **Measured 2026-08-29,
that step is a no-op on native:**

- `cargo tree -e features` already resolves `iced_renderer feature "fira-sans"`
  at HEAD, without `crates/ui/Cargo.toml` listing it.
- Toggling the feature explicitly on/off produces **byte-identical** snapshots
  (`assistant_slot__open_stub__typical`, SHA `29eea22ef8432708` both ways).
- Cause, at source: `iced_graphics/src/settings.rs:42` switches `default_font` to
  Fira Sans only under `cfg!(all(target_arch = "wasm32", feature = "fira-sans"))`
  — **WASM only**. On native, `Font::DEFAULT` stays `Family::SansSerif`, still
  resolved through the OS font DB. The feature loads the font into the database;
  nothing ever asks for it by name.

**And the harder half:** `iced_test::screenshot` takes no font argument
(`viewport_matrix.rs:133` -> `Emulator::new`), so setting the font on the
application builder would NOT reach the pixel gate. The app and the gate would
then disagree by construction — worse than today.

**Therefore the ~62-file re-baseline stays blocked**, but for a different reason
than recorded: not "the font feature is off" (it is on) but "nothing selects the
font, and the test harness has no way to". Full analysis, with the source lines,
in `docs/dev-notes/visual-baseline-drift-2026-07-27.md` § Correction.

### H1 RESOLVED (2026-09-16) — the cockpit embeds Inter (ADR-0093)

What was missing was not a feature flag but a face this repository owns, plus the two
places that have to agree on it:

- `crates/ui/assets/fonts/Inter-Regular.ttf` (303 KiB, SIL OFL 1.1) ships in the repo,
  and iced's `fira-sans` (431 KiB) is dropped in the same change — the binary ends up
  ~128 KiB SMALLER than before. The operator lifted the no-bundled-fonts lock for this
  one face.
- `theme::font::embedded()` LOADS the face into the global font database (once per
  process) and RETURNS the font. Both halves are load-bearing: `iced_test`'s `Emulator`
  builds its renderer from `Program::settings().default_font` and never loads
  `settings.fonts`, so a program that only NAMES the face renders through the OS font
  database in the gate while the app renders Inter — the "disagree by construction"
  failure the paragraph above warned about.
- All 14 `iced::application(..)` sites pass it to `.default_font(..)`, and all 15 canvas
  texts name `theme::font::UI` directly, because canvas text carries a concrete `font`
  field and does not inherit the renderer default.
- `crates/ui/tests/embedded_font_contract.rs` enforces every clause on every platform,
  including "no string literal draws a glyph the face lacks" — cosmic-text falls back
  PER GLYPH, so one missing character re-arms this bug for that character. Fira Sans
  lacked eight of this UI's glyphs (`✓ ✗ ⚠ ★ ● ⓘ ▸ ▾`), five of them the non-colour
  signals in ADR-0085's verbatim copy; Inter lacks three, replaced in the Lab screen.
- The byte-compare helpers assert the program's default font BEFORE rendering, so a
  baseline drawn through the OS font database cannot even be captured.

Why the operator's "widget-level font at the ui layer" ruling shipped as a renderer
default rather than 412 `.font(..)` calls: the ruling's requirement is that the app and
the gate agree, and one default plus 15 canvas fields is enforceable by a test, whereas
412 call sites are enforceable only by discipline.

**The re-baseline is unblocked** — ONE pass, per-screen operator approval, with
`sw_vers` + toolchain recorded in that commit. It also unblocks story 3-20
(advisor-honesty-surface), which was waiting on this for its AD-10 proof.

### `audit_aggregator_handles_10k_event_storm` — diagnosed, NOT fixed (2026-08-29)

Fails on linux AND windows; passes here. Panic site is
`activity_tape_audit_ledger_event_storm.rs:184`, which is **assertion 1** —
`cumulative_counter >= 1`, the WEAKEST of the file's four. So CI observed
**zero** events from a 10 000-event storm, not merely too few.

Mechanism, from source: the test fires 10k events in a tight loop, then does a
single fixed `sleep(Duration::from_millis(350))` (line 126) and drains. The
aggregator emits on a 100 ms `interval.tick()`. On a 2-4 core runner under
scheduler pressure, that task need not be polled at all inside 350 ms — so the
drain sees nothing. The file's own comment anticipates the fast case ("on very
fast machines the storm completes in << 1 ms") and not the slow one.

**Not reproduced locally, and I will not pretend otherwise.** Run under 42 CPU
burners on 14 cores: 3/3 passes, 0.38-0.43 s. A 14-core box still schedules the
aggregator promptly; the failure needs the runner's core count, which macOS
cannot simulate for a native process.

**Recommended fix, deliberately NOT applied** — it is timing-sensitive code and
CI is the only environment that reproduces the failure, so it should be changed
and verified in the same cycle, not written blind here:

> Replace the fixed 350 ms sleep with a BOUNDED POLL — wait until the drained
> cumulative counter reaches the 90 % threshold assertion 4 already requires, up
> to a generous ceiling (~5 s), and fail with "observed N of 10000 after Xms" on
> timeout. Then compute assertion 2's rate-cap budget from the ACTUAL elapsed
> wait rather than the hardcoded 350, so a longer wait cannot silently inflate
> the tick budget.

That keeps all four assertions meaningful while dropping the assumption that
350 ms of wall-clock is always enough scheduler time.

### Aggregator timing tests (2026-08-29) — one FIXED, one reframed and left alone

**`aggregator_emits_one_tick_per_window` (agent) — FIXED, verified against a real
local reproduction.**

Its docstring says "fire 500 ticks across a 350 ms span". The code sent all 500
in ONE tight loop with no delay and slept afterwards. A single instantaneous
burst is ONE non-empty window, and by design the first non-empty window emits
only `Start` — the 100 ms throttle suppresses `tick()` in the same window. So a
`Tick` appeared only if the aggregator happened to be scheduled part-way through
the burst. That is scheduler luck.

Reproduced locally under 42 CPU burners on 14 cores: `starts=1 ticks=0
end_success=1`, identical to the windows-latest failure "at least 1 Tick event
(got 0)". Fixed by sending the 500 events in 5 chunks with an 80 ms sleep
between, so the multiple non-empty windows are REAL and Ticks follow by
construction — which is what the docstring claimed all along. Also replaced the
fixed post-burst sleep with a bounded wait on the observation (`collect_until`).

Verified: **6/6 passes under the same load that previously failed 2 of 4.**

**`audit_aggregator_handles_10k_event_storm` (ui) — NOT fixed, and REFRAMED.**

This was recorded above as a linux/windows failure. It is not:

    5 consecutive runs at unmodified HEAD, macOS canonical box:
      FAILED, FAILED, ok, ok, ok        (one failure showed cumulative_counter=10 / 10000)

**The test is flaky on the canonical box too.** The CI legs are not special; a
slower runner just loses the race more often. An earlier single passing run here
is what made it look platform-specific.

An attempt to apply the same bounded-wait fix was REVERTED. Two things were
learned and are worth recording so the next attempt starts ahead:

1. Waiting for "the first Tick" is the wrong condition — it captured 6806 / 10000
   (68 %) and failed assertion 4's 90 % coverage bar. The test's own strongest
   assertion caught the weakened fix, which is exactly what it is for.
2. Waiting for 90 % coverage instead still plateaued at 8343 / 10000 after a full
   10 s. So the shortfall is NOT a timing shortfall: some of the storm never
   appears in any `Tick` at all. Candidates, unverified: the final partial window
   is flushed with `End` rather than as a `Tick`, and/or the `activity_rx`
   broadcast receiver lags and drops while undrained (consistent with the
   `cumulative_counter=10` run).

Fixing it needs the aggregator's window/flush semantics, not more waiting. Left
to whoever owns those; the reproduction recipe is 42 CPU burners and repeated
runs, which now fails often enough on macOS to iterate against.

#### Storm test — ROOT CAUSE FOUND 2026-08-29 (still not fixed; needs a design call)

Not flakiness in the ordinary sense. From `activity_audit_aggregator.rs`, the
`(n, None)` arm — the FIRST non-empty window — calls `bus.start(kind, label)` and
deliberately emits **no Tick**; its count goes into the Start LABEL only. The
source says so explicitly ("We do NOT call `h.tick(n)` immediately after
`start()`" — the 100 ms throttle would swallow it).

`cumulative_counter` in the test sums **Tick** counts only. So every event the
first window absorbs is invisible to assertion 4's 90 % coverage bar, and the
test passes or fails on how much of the storm window 1 happens to swallow:

    window 1 absorbs ~17 %  ->  coverage 83 %  -> FAIL   (measured)
    window 1 absorbs 9990   ->  cumulative_counter = 10  -> FAIL (measured)
    window 1 absorbs little ->  coverage > 90 %  -> pass  (the usual case here)

That is why it fails more on slow runners: the aggregator is scheduled less
often during the burst, so a single window absorbs a larger share.

**And the obvious fix does not work.** Recovering window 1's count means parsing
the Start label — but `format_label` collapses anything above `K2_THRESHOLD`
(9999) to the fixed string `"Audit: 9999+ writes"`. In a 10 000-event storm the
first window very often exceeds 9999, so the count is unrecoverable BY DESIGN
precisely in the case that breaks the test.

**The decision this needs** (not taken here — it is assertion-design ownership):
- (a) assert coverage over `Start`-label count + Tick counts, and shrink the
      storm below `K2_THRESHOLD` so the label stays parseable; or
- (b) drop assertion 4's coverage bar and assert the contractual property
      instead — the K2 label flip — which the file's own comment notes is already
      unit-tested in `format_label_truncates_above_k2_threshold`; or
- (c) have the aggregator emit the first window's count as a Tick too, which
      changes PRODUCT behaviour and the activity-tape semantics.

(a) and (b) are test-side; (c) is a product change. Until one is chosen the test
stays red on the slower legs and intermittently red here — 2 failures in 5 runs
at unmodified HEAD on the canonical box.

## The last two windows-latest reds — fixed at source, CI-unconfirmed (2026-09-24)

Both remaining `windows-latest` failures are addressed. **Neither fix is
CI-confirmed yet** — no Windows box was available and GitHub run logs still need
repo admin — so this section states what was proven by running and what was
reasoned. The story stays `in-progress` until a green Windows lane says otherwise.

### `t1414_v7_coinbase_outage_isolated` — a paused clock cannot span a ledger call

Not a Windows API difference and not "the runner is slow". A structural defect
that macOS and ubuntu happen to survive:

- `tokio::time::pause()` (was `coinbase_outage_isolation.rs:196`) froze the clock
  across BOTH audit-ledger interactions.
- Every `sqlx` pool acquire wraps itself in `tokio::time::timeout(30 s)` —
  `sqlx-core-0.8.6/src/pool/inner.rs:248-251` → `sqlx-core/src/rt/mod.rs:27`. That
  deadline is on the **virtual** clock.
- The SQLite work runs on a raw `std::thread::Builder` worker
  (`sqlx-sqlite-0.8.6/src/connection/worker.rs:106`), **not** `spawn_blocking`, so
  it does not inhibit tokio's auto-advance (tokio's only inhibitor is
  `runtime/blocking/schedule.rs:25`).
- So whenever the runtime parks with the write still in flight, virtual time jumps
  to the next deadline and keeps jumping — `park_thread_timeout` in
  `tokio/src/runtime/time/mod.rs`.
- `spawn_venue_supervisor` swallows the resulting `PoolTimedOut` as a non-fatal
  `warn!` (`crates/agent/src/runtime.rs:2872-2880`). The only symptom is a missing
  row: `got 0`.

**Measured on the canonical box (both by running):**

| probe | result |
|---|---|
| 16 concurrent `feed_reconnect` writes under `pause()` | **9 returned `PoolTimedOut` in 481 µs of real time**; only 7 rows landed |
| the old 200-iteration bounded poll, forced to run in full (the windows case) | **96 virtual seconds burned in 39 ms of real time** — vs sqlx's 30 s acquire deadline |

The second number is the whole bug: the poll loop added in 2026-09 to "give the
writer a chance" gives it 39 ms of real time while burning 96 s of the virtual
time the writer's deadline is measured in. It cannot help, and past 30 s it is
what kills the write.

**Fix:** the clock is now paused only across the MockFeed / watchdog phase. The
supervisors start on the real clock, the `FeedReconnect` assertion is a bounded
REAL-time poll (5 ms steps, 10 s ceiling) placed before the pause, and
`tokio::time::resume()` precedes the cancel+drain so its 5 s budget is wall-clock
again. A grep-verifiable invariant now holds: **no `audit::` call appears between
`pause()` and `resume()`.** Every assertion is unchanged.

**Not proven:** that windows-latest specifically loses this race. 30/30 passes
under 42 CPU burners on 14 cores, old code AND new — same limit the
`audit_aggregator_handles_10k_event_storm` diagnosis hit, and for the same reason.

### `t1926_no_secrets_in_artifacts` — the gate could not run on Windows, and would have gone green

The test shelled out to `bash scripts/check_no_secrets_in_llm_artifacts.sh`. On
`windows-latest` that is unrunnable-or-worse:

- `bash` resolution from a native process is not guaranteed, and `C:\Windows\
  System32\bash.exe` (the WSL launcher) would shadow Git-Bash if present.
- The script's two workhorses are absent or shadowed: `strings(1)` is binutils and
  not in Git-for-Windows; GNU `find` loses to `C:\Windows\System32\find.exe` when
  Git's `usr/bin` is off PATH.
- **The `strings` case is the dangerous one, not the loud one.** Every
  `strings … | grep -q` pipeline reports "no match" when `strings` is missing, so
  the script would print `V9 PASS` having scanned nothing — bug-log #66's class,
  on the one gate AD-19 leans on.

(CRLF is NOT a factor any more: `.gitattributes` pins `* text=auto eol=lf` since
bug-log #97.)

**Fix, option (a) per the preferred shape:** the scan is now Rust and runs on all
three legs. The script stays the single source of truth — `SecretGate::parse`
READS `PATTERNS=(…)` and `SK_RE=` out of it at runtime, and refuses to run if the
list shrank below 8 or if `SK_RE` changed spelling. The Rust scan is a strict
superset of the shell one (raw bytes, so no `strings` floor and no `grep`
line-splitting). Three controls that the shell gate never had now run every time:
the matcher must detect both fixture keys (and their upper-case forms), must stay
quiet on innocuous bytes, and the directory WALK must find a planted key in a
throwaway tree. On unix the script is still executed verbatim as an ADDITIONAL
assertion, so its own plumbing stays exercised.

**Coverage lost on Windows:** the script's `find`/`grep`/`strings` plumbing. Never
the pattern list, never the artifacts — both are covered on every leg, which is
strictly more than before, when the whole gate was absent there.

## Why the Windows lane was never confirmed — CI has been red on EVERY push since 2026-09-27 (2026-10-11)

The section above left this story `in-progress` pending "a green Windows lane". That lane could
not have gone green, and not for any Windows reason: **the matrix has failed on every push for 15
days, on all three OSes, identically.** `gh run list` shows **25 consecutive failures**
(2026-09-27 → 2026-10-11).

Reading the **full** annotation list for a job — `gh api repos/{owner}/{repo}/check-runs/<id>/annotations`
filtered to `annotation_level=="failure"`, rather than the first few lines — gives exactly three
failing tests workspace-wide, none of them platform-specific:

| test | crate | cause |
|---|---|---|
| `every_anchored_surface_derives_from_exactly_one_accepted_tuple` | `backtest` | bug-log `#142` — a 4-line parser window broken by a *mandatory* ADR-0038 D6.b disclosure comment |
| `t1937b_canonical_strategy_anchors_unchanged` | `reports` | bug-log `#143` — story 1-27's 2026-09-26 re-emission updated `anchors.toml` and never re-locked its derived projection |
| `t1937c_sqrt_impact_strategy_anchors_unchanged` | `reports` | same |

**All three are fixed as of 2026-10-11** (`#142` and `#143`), verified green locally:
`relock_manifest` 2/2 and `strategy_anchors_unchanged` 4/4, where the latter was 2/4.

### What this means for THIS story, stated carefully

The two Windows fixes recorded above may well be correct — the `tokio::time::pause()` analysis is
detailed and was reasoned from source. **But this story cannot claim them confirmed**, because a
platform-INDEPENDENT failure has been masking the platform-specific signal for 15 days. Every run
in that window went red for reasons that had nothing to do with Windows, so a Windows-specific red
would have been indistinguishable from the noise, and a Windows-specific *green* equally
invisible.

The next push is therefore the first informative run this story has had since 2026-09-24. Two
outcomes, both useful:

- **All three lanes green** → the 2026-09-24 fixes are confirmed and this story's remaining
  acceptance work is done.
- **Only `windows-latest` red** → that is the real cross-platform signal, finally legible, and the
  story continues from there with an actual failure to read.

Either way the story stays `in-progress` until a run says which. What changed today is that the
question is now *answerable*.

### The process defect, which is worth more than the fix

A gate stuck red is worth no more than a gate that cannot fail, and it is worse, because it
teaches everyone to stop reading it. This matrix is this story's entire deliverable — activated
2026-07-10 so Linux and Windows would finally be verified — and it spent 15 days reporting
FAILURE into a void while 25 pushes landed on top of it.

Part of why nobody saw it is structural, and it is the project's own guidance: CLAUDE.md
(correctly) scopes verification to `-p <crate>`, and `cargo test -p backtest --test <target>`
never compiles a sibling crate's targets — so `crates/reports` was invisible to every gate run
locally for two weeks. The cheap habit that closes the gap is **reading `gh run list` after a
push**: one command, and it would have surfaced this on 2026-09-27.
