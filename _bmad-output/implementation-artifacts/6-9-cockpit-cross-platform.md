# Story 6.9: cockpit-cross-platform

Status: in-progress

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the cockpit on Linux/Windows: source shipped + macOS-verified; the 3-OS CI matrix ACTIVATED 2026-07-10 (P7) - the run-2 shakeout is the open in-progress work,
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

## Acceptance Criteria

1. **Given** the activated 3-OS CI matrix (ci.yml live on push/PR) with run-2 shakeout reds open, **when** the shakeout fixes land (fix-forward per the operator direction), **then** the Linux/Windows lanes go green and the story flips to done - until then it is honestly in-progress.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

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

- Trace: `REQ-COCKPIT-CROSS-PLATFORM-001` (state=`dev-done`)
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
- **macOS**: the 48-file visual drift — see the correction below.
- `leaderboard_scorecard_render` — bug-log **#96**, a product decision.

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
