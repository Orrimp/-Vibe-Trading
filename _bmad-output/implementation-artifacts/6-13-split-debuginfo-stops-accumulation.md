# Story 6.13: split-debuginfo-stops-accumulation

Status: ready-for-dev

<!-- Created 2026-11-23 by bmad-create-story. Revised the same day after the orchestrator's
     three-lens adversarial review (32 findings) and its binding rulings: ON INCONCLUSIVE is a
     pre-registered replicate under standing authority (never an operator ruling); the candidate
     order is settled by its real criterion (B > C > D); DISK is informational.
     Source: bug-log #218 (filed at story 2-63's Close) and the operator's ruling of 2026-11-23
     ("#218: Fix the cause"), which chose the profile change over the prune, as the operator
     declined `cargo-sweep` on 2026-08-19 (c1474c0f) because it "mops up the effect".
     Cross-cutting (every crate's dev/test build) -> an ADR (AD-18). Build/infra only: no Rust
     source, no anchored content, no `[profile.release]` change. -->

## Story

As the operator of the Honest Advisor, who builds and tests on one long-lived `target/`,
I want the dev and test profiles to stop leaving every build's debug objects in `target/debug/deps`,
so that a build costs the same on its thousandth run as on its first, because the setting that
causes the growth is fixed (bug-log `#218`) and nothing has to clean up after it.

## Acceptance Criteria

1. **The mechanism is measured on this toolchain before anything is chosen.** A scratch probe
   (Dev Notes § The probe) is built with toolchain 1.94.1, `debug = "line-tables-only"` and
   incremental on, under five settings: `split-debuginfo` unset, `"unpacked"`, `"packed"`, `"off"`,
   and `"packed"` with the probe's dependency at `debug = false`. Each setting is built twice. It
   records:
   - `*.rcgu.o` left in `debug/deps` after build 1 and after build 2;
   - whether a `.dSYM` exists;
   - whether the panic message carries `src/lib.rs:LINE:COL`;
   - whether the `RUST_BACKTRACE=1` frames carry file:line, for **two** frames: `probe_frame`,
     compiled into the test harness crate itself, and `dep_frame`, in a path-dependency rlib.

   The table goes in the Dev Agent Record. Expected (REASONED, Dev Notes § What each setting
   does):

   | setting | objects b1 / b2 | `.dSYM` | panic location | `probe_frame` file:line | `dep_frame` file:line |
   |---|---|---|---|---|---|
   | unset | >0 / more | no | yes | yes | yes |
   | `unpacked` | >0 / more | no | yes | yes | yes |
   | `packed` | 0 / 0 | yes | yes | yes | yes |
   | `off` | 0 / 0 | no | yes | **no** | yes (rlib member) |
   | `packed` + dep `debug = false` | 0 / 0 | yes | yes | yes | **no** |

   **Any deviation halts the story** and goes to the orchestrator with the table, before any
   other wave.
2. **The A/B instrument and the pre-registration are in git BEFORE anything is measured.** One
   commit carries:
   - `scripts/build_time_ab.sh` (runner, with `--self-test`), `scripts/build_time_ab_verdict.py`
     (verdict, with `--self-test`), the committed arms file `scripts/build_time_ab/arms-6-13.toml`
     (control, candidate order, edits, expected flags) and `docs/runbooks/build-time-ab.md`, to the
     requirements in Dev Notes § The instrument;
   - this story file with its sprint-status and trace rows and the `#218` creation addendum, so
     P-6-13's text is in git;
   - both self-tests' wiring: into `.githooks/pre-commit` (each only when its own file or the hook
     is staged) and into the CI macOS governance step.

   The runner and the verdict are reviewed against P-6-13 in `bmad-code-review` **before** this
   commit. Any protocol gap found there is closed by amending the protocol text in this same commit
   (Dev Notes § The instrument → Reviewed before H). That commit is **H**. Its hash and
   `shasum -a 256` of the three script/arms files go into the Dev Agent Record before the session
   starts.
3. **K4 is measured before and after in one same-session A/B, under Dev Notes § Protocol P-6-13
   as committed at H, unchanged.**
   - Arms: A (today's `unpacked`, BEFORE), B (`packed`), C (`off`), D (`packed` plus dependencies
     at `debug = false`).
   - Record every wall, `r_k`, spread, per-run object delta, size record (informational) and
     V-rule result, and paste the committed verdict script's output verbatim.
   - The setting that ships is the one its `CHOICE:` line names.
   - An INCONCLUSIVE result ships nothing and is handled by § Protocol P-6-13 → ON INCONCLUSIVE,
     under the orchestrator's standing authority.
4. **The chosen setting ships, and only it.**
   - `Cargo.toml` `[profile.dev]` and `[profile.test]` carry the chosen `split-debuginfo` (and,
     for D, `debug = false` in the existing `[profile.dev.package."*"]`).
     `debug = "line-tables-only"` stays.
   - `[profile.release]` and `[profile.dev.package."*"] opt-level = 3` are byte-unchanged
     (`git diff` shows no release line).
   - The comment block above the profiles is rewritten (Dev Notes § Files).
   - The `Swatinem/rust-cache@v2` step gains `prefix-key: "v1-rust"`. It has none today, so it
     uses the action's default `v0-rust`. The new key makes CI's first run genuinely cold rather
     than a stale prefix restore (Dev Notes § One-time clean → CI).
   - **CLAUDE.md's Coding-rules bullet on `line-tables-only` + `split-debuginfo` is updated in
     the same commit.**
5. **ADR-0103 and its Registry row land in the same commit as AC4** (AD-18;
   `python3 scripts/adr_registry_check.py --pre-commit` passes). 0103 was the next free number at
   story creation; re-check the Registry first, because numbers are never reused. The ADR records:
   - the measured choice, the AC1 table, and AC8's footprint figures;
   - the rejected alternatives, with their reasons (Dev Notes § Rejected);
   - the backtrace consequence of the chosen setting, as measured by AC1's two frames;
   - the platform consequences, with the Windows ones labelled REASONED;
   - the one-time clean and the CI cache reset;
   - an "Enforced by" line naming the actual commands (bug-log `#191`).
6. **The setting is guarded, two-sided.** `scripts/dev_profile_debuginfo_check.py` (PEP-723
   `requires-python = ">=3.11"`, with `--self-test`) fails when:
   - the dev profile's `split-debuginfo` is not the ADR-0103 value, **including when the line is
     deleted** (deleting it reverts macOS to cargo's default, `unpacked`);
   - `[profile.dev]` `debug` is not `"line-tables-only"`, including when the line is deleted (its
     loss is `c1474c0f`'s 155 GB failure);
   - `[profile.test]` differs from `[profile.dev]` (the double-build reason in Dev Notes
     § Rejected). A key `[profile.test]` sets, sub-tables included, counts as different unless
     `[profile.dev]` sets it to the same value. That covers a different `split-debuginfo`, and a key
     set only in test, such as `incremental = false`;
   - any `[profile.dev.package.*]`, `[profile.test.package.*]` or `build-override` table sets
     `split-debuginfo` (for example `[profile.dev.package.ui] split-debuginfo = "unpacked"` would
     bring the accumulation back for ui alone);
   - `[profile.release]` gains `debug` or `split-debuginfo`;
   - D ships and the dependency `debug = false` is gone;
   - B or C ships and `[profile.dev.package."*"]` sets `debug`, which would turn the shipped
     setting into an unmeasured D.

   Each condition gets a firing and a silent self-test fixture.

   In the AC4 commit it is wired into `.githooks/pre-commit` (live `--staged` and `--self-test`
   on every commit, through the hook's `uv run` / `python3` pattern) and into the CI macOS
   governance step.
7. **The fix is confirmed on the real `target/`, with the AC4 change applied and BEFORE its
   commit**, with no cargo running:
   - The dry run lists something, and everything it lists is under `target/debug`:
     ```
     root=$(git rev-parse --show-toplevel)
     cargo clean --profile dev --dry-run -v >dryrun.out 2>dryrun.err
     test -s dryrun.out && echo LISTING-NONEMPTY || echo LISTING-EMPTY   # must print LISTING-NONEMPTY
     awk -v p="$root/target/debug" '$0 != p && index($0, p "/") != 1' dryrun.out | wc -l   # prints 0
     ```
     `wc -l < dryrun.out` is at least the file count in `dryrun.err`'s `Summary … files, … total`
     line. The listing includes directories, the Summary counts files. Record both numbers. A
     `grep -c -v` alone would print `0` on an empty listing, so it is not enough (`#187`).
   - Then `cargo clean --profile dev`. Never bare `cargo clean`: Dev Notes § One-time clean.
   - One cold `cargo test -p ui --no-run`.
   - Then five consecutive K4 builds (`touch crates/ui/src/lib.rs; cargo test -p ui --no-run --timings`)
     on the real `target/`, each adding **0** `*.rcgu.o` to `target/debug/deps`.

   Counts and walls are recorded. The walls are informational; no other day's number is a verdict.
8. **The floor holds, measured wide because the change is cross-cutting, BEFORE the AC4 commit:**
   - `bash scripts/verify_anchors.sh` prints `ANCHORS PASS (119 / 119)` before Wave 3 and after
     Wave 4;
   - `python3 scripts/spec_lint.py` PASS (0 violations), and the bug-log schema check passes;
   - `cargo fmt --all -- --check` exits 0, and `cargo clippy --workspace -- -D warnings` exits 0;
   - one `RUST_BACKTRACE=1 cargo test --workspace --no-fail-fast` exits 0. It is a single
     invocation, with nothing else running and a watch recipe. Every `test result:` line reads
     `0 failed`, and the number of `Running` lines is recorded and is greater than 0 (a build that
     fails prints no `test result:` line at all);
   - `cargo test -p trading_core --test trybuild` and
     `cargo test -p trading_core --test pit_compile_fail` pass (byte-exact rustc diagnostics);
   - `cargo test -p ui --test recipe_off_runtime_poll` passes three times under `RUST_BACKTRACE=1`
     (its timing bounds budget for backtrace symbolization).

   `rust-validate`'s audit/deny/doc legs are not run: a profile-only change (no Rust source, no
   dependency change) cannot move them. That is the one stated exception to AD-19's floor.
   `du -sh target/debug` and the total size of `*.dSYM` after the wide run go into ADR-0103 before
   the commit. This is the full-workspace figure `c1474c0f` promised and never recorded.
9. **The records move together.**
   - The trace row's `state` moves with every story Status, in the same edit:
     `in-progress` → `in-progress`, `review` → `dev-done`, `done` → `shipped`. So does the
     story's own `- Trace:` line.
   - In the AC4 commit, bug-log `#218` keeps `**Status**: open` and gains a **second** dated
     addendum recording the chosen setting and ADR-0103.
   - Its Status flips to `fixed` in a **follow-up** commit that cites the AC4 commit's hash (the
     schema's rule E7: a commit cannot cite itself). The same follow-up carries the first
     post-push CI step times, as a dated ADR-0103 § Changelog line with the decisions README's
     `updated:` marker bumped (`adr_registry_check` invariant (b)).
   - At `done`, the ADR-0082 triad moves in one pass: story `Status:`, the
     `REQ-SPLIT-DEBUGINFO-STOPS-ACCUMULATION-001` row's `state` and a CHANGELOG line under
     `## Tooling & process`.
   - After the push, CI is read once (never polled): both legs green.

## Tasks / Subtasks

Waves are sequential. Waves 2 and 4 need the box ALONE: no other cargo, rustc or agent build.
Sub-agents write files only; the orchestrator owns every commit and push.

- [ ] **Wave 0: pre-flight and the mechanism probe (AC1)**
  - [ ] When the story goes `in-progress`, set the trace row to `in-progress` and the story's
        `- Trace:` line with it (AC9); status-drift checks it on every commit.
  - [ ] Record:
    - `git rev-parse HEAD`;
    - `ls -f target/debug/deps | wc -l` (the real dev box, BEFORE);
    - `df -g /System/Volumes/Data`, `sysctl -n vm.swapusage`, `uptime`;
    - `dsymutil --version` (2026-11-23: `/usr/bin/dsymutil`, Apple LLVM 21.0.0, CommandLineTools);
    - `rustc -V` inside the repo (expect 1.94.1);
    - `echo "${CARGO_INCREMENTAL-unset}"` (expect `unset`).
  - [ ] Run `bash scripts/verify_anchors.sh` (expect `ANCHORS PASS (119 / 119)`) and
        `python3 scripts/spec_lint.py` (expect PASS), and record the lines.
  - [ ] Build the probe (Dev Notes § The probe) and record its table. Halt on any deviation from
        AC1's expectations.
- [ ] **Wave 1: the instrument and the pre-registration (AC2)**: one commit by the orchestrator, H
  - [ ] `scripts/build_time_ab_verdict.py` with `--self-test` (requirements in Dev Notes § The
        instrument). Run `--self-test` and record its PASS line and its wall time.
  - [ ] `scripts/build_time_ab.sh` (bash, not zsh) with its stub-driven `--self-test`. Run it and
        record its PASS line and wall time.
  - [ ] `scripts/build_time_ab/arms-6-13.toml`: the four arms of P-6-13 verbatim, the control (A),
        the order (B, C, D) and the expected flags.
  - [ ] Capture real `-v` rustc lines for the V11 fixtures, since the prior-art logs hold none:
    - member lines (lib, an integration test, a bin):
      `touch crates/ui/src/lib.rs; SDKROOT=… cargo test -p ui --no-run -v`;
    - the vendored non-member and registry lines:
      `SDKROOT=… CARGO_TARGET_DIR=<scratch> cargo check -p iced_tiny_skia -v`.

    Trim them into fixtures.
  - [ ] `docs/runbooks/build-time-ab.md`, with the six sections of AGENT.md § Orchestrator
        duties 5, including a watch recipe.
  - [ ] Hook: each `--self-test` in `.githooks/pre-commit`, conditional on its own file or the hook
        being staged (for example
        `git diff --cached --name-only -- scripts/build_time_ab_verdict.py .githooks/pre-commit`),
        in the `uv run` / `python3` pattern for the Python one (Dev Notes § Files).
  - [ ] CI: create the step under its final name,
        `"Governance — build-time A/B verdict + dev profile debuginfo (bug-log #218)"`, with
        `if: always() && matrix.os == 'macos-latest'`, running both instrument `--self-test`s. The
        AC4 commit adds the guard's lines to the same step.
  - [ ] **Review before H** (binding): `bmad-code-review` in a fresh context, of the runner, the
        verdict and the arms file against § Protocol P-6-13. Close any protocol gap it finds by
        amending the protocol text in this same commit.
  - [ ] Stage this story file, `sprint-status.yaml`, `trace.toml` and the `#218` creation
        addendum with it.
  - [ ] Gates: spec_lint, the hook itself, both instrument `--self-test`s.
  - [ ] After the commit: write H's hash and the three files' `shasum -a 256` into the Dev Agent
        Record.
- [ ] **Wave 2: the A/B session (AC3)**: alone, about 2–2.5 hours
  - [ ] Launch `bash scripts/build_time_ab.sh --arms scripts/build_time_ab/arms-6-13.toml --scratch-root <session scratch dir> --out <session scratch dir>/<run>`
        in the background, with a watch recipe in the same message (AGENT.md duty 4).
  - [ ] Run `uv run --quiet scripts/build_time_ab_verdict.py <run-dir>` and paste its full output.
        Its first line names the outcome class (§ Protocol → Verdicts → Outcome classes).
  - [ ] Copy the **whole** run dir (`--out`, which holds every file the verdict reads; targets
        and snapshots live outside it) into the session dir, outside `/private/tmp`, which a reboot
        clears. Do this **before** anything else, and before any ACTION recipe is sent. A
        verdict-only re-judgement runs on that copy.
  - [ ] Route on the first line:
    - `CHOICE: <arm>`: go on to Wave 3.
    - `REFUSED`, in the run dir or on the runner's stdout: fix the cause and launch again into a
      fresh `--out`. This is not a session.
    - `ALL FAIL`: the story is held (ON INCONCLUSIVE, last bullet), unless § An instrument defect's
      verdict mis-read case applies.
    - Any other exit-2 class: follow ON INCONCLUSIVE.

    Nothing ships until a `CHOICE:` exists.
  - [ ] Delete the scratch target dirs and snapshots: they are regenerable and never evidence.
- [ ] **Wave 3: apply the choice in the working tree (AC4, AC5, AC6)**: no commit yet
  - [ ] `Cargo.toml`: the chosen lines and the rewritten comment block; release untouched.
  - [ ] `CLAUDE.md`: the Coding-rules bullet.
  - [ ] `.github/workflows/ci.yml`: the rust-cache step adds `prefix-key: "v1-rust"` (with a
        comment citing `#218` and ADR-0103), and the guard's lines go into the governance step.
  - [ ] `scripts/dev_profile_debuginfo_check.py` + `--self-test`, wired into the hook.
  - [ ] Draft ADR-0103 (`accepted` at commit), its Registry row, and the README `updated:` marker.
  - [ ] Bug-log `#218`: the second dated addendum. Status stays `open`.
  - [ ] Trace row: `arch` and `tests` gain the ADR and the two checking scripts.
- [ ] **Wave 4: clean, confirm, sweep, all BEFORE the AC4 commit (AC7, AC8)**: orchestrator, alone
  - [ ] `cargo metadata --format-version 1 >/dev/null` (the manifest parses).
  - [ ] The one-time clean and the five confirming builds (AC7).
  - [ ] fmt, clippy, the wide test run, the three named targets, anchors (after). Record each
        command's literal result line.
  - [ ] Record `du -sh target/debug` and the `*.dSYM` total, and put both into the ADR-0103
        draft.
  - [ ] `docs/runbooks/ops-push-and-cache.md` § 2: replace the bare `cargo clean` with
        `cargo clean --profile dev`, plus the caveat that bare `cargo clean` deletes all of
        `target/`. Add a § Changelog line.
- [ ] **Wave 5: the AC4 commit**: orchestrator
  - [ ] One commit: Waves 3 and 4's files. Gates: `adr_registry_check.py --pre-commit`, the guard
        live and `--self-test`, spec_lint, the bug-log schema check (`--staged`, via the hook).
- [ ] **Close (AC9)**
  - [ ] Status → `review` (row → `dev-done`).
  - [ ] `bmad-code-review` in a fresh context.
  - [ ] Push on the operator's explicit "Push"; read CI once.
  - [ ] The follow-up commit: `#218` → `fixed` citing the AC4 hash, plus the CI step times in
        ADR-0103 § Changelog with the README marker.
  - [ ] The triad move at `done` (row → `shipped`).

## Dev Notes

### The cause, in four lines

- `split-debuginfo = "unpacked"` makes rustc keep every codegen unit's object file, because on
  macOS the linked binary finds its own crate's line tables through them (N_OSO entries).
- The names change with every build: rustc adds a per-invocation suffix, under incremental builds
  only (rust-lang/rust#139453, 1.88). So only workspace crates accumulate; dependencies (never
  incremental) and CI (`CARGO_INCREMENTAL=0`, set by rust-cache) do not.
- Cargo tracks none of these files, so nothing deletes them. Bug-log `#218` measured:
  - 1.76 M entries in `target/debug/deps`, 1.74 M of them `*.rcgu.o`;
  - about 14,000 more entries per `cargo test -p ui --no-run`;
  - a 27–31 s, ~750 MB start for every rustc given that directory (an empty one: 0.04–0.50 s,
    62 MB).
- K4 (story 2-63's AC5) read 563 s against Wave 0's 342 s because of it, and 36–54 s after a
  prune. The operator rejected the prune as a fix, because it treats the effect; this story
  changes the setting.

### What each setting does (cited; REASONED until AC1 measures it on 1.94.1)

The sources below are the current stable docs and source. The reviewer re-checked them against the
1.94.1 rustc source and the rust-1.94.0 cargo branch. AC1 is what makes them facts for this box.

| | macOS (canonical leg + dev box) | Windows MSVC (CI leg) |
|---|---|---|
| unset | cargo's default for a profile with debug info is `unpacked` [cargo profiles] | rustc default `packed` (`.pdb`) |
| `unpacked` (today) | loose objects kept, so they accumulate | not supported; cargo drops the flag [cargo `build_base_args`], giving `.pdb` as unset |
| `packed` | rustc runs `dsymutil` on every linked binary, then deletes the loose objects [rustc `link.rs`]; one `<bin>.dSYM` per binary, rewritten in place | supported and the native default: no change |
| `off` | no `dsymutil`; loose objects deleted. The binary's own-crate N_OSO entries then point at deleted files. Its rlib-member entries (`…/libX-<hash>.rlib(member.rcgu.o)`) still resolve while the rlib exists | not supported; cargo drops it: no change |

- **The two `unpacked` lines have done nothing on either remaining platform since 2026-08-19.** On
  macOS they restate cargo's default; on MSVC cargo drops them. `c1474c0f`'s footprint win came from
  `line-tables-only`. REASONED; AC1's "unset" row measures the macOS half. That is why the bug notes
  that deleting the lines is no fix.
- **Object deletion** [rustc `link.rs`, `preserve_objects_for_their_debuginfo`]:
  - `Off` and `Packed` return `(false, false)` ("the debuginfo in the objects has been packaged
    and the objects can be deleted").
  - `Unpacked` on a target that cannot use split DWARF (Apple) returns `(true, false)`.
  - This governs only the loose objects of the crate being linked. Rlibs reach the linker as
    archives [`add_static_crate` → `link_upstream`], so dependencies and, for an integration test,
    the library under test keep their DWARF in their rlib members under every setting.
- **`dsymutil`'s two failure modes.**
  - A `dsymutil` that runs and exits non-zero only WARNS (`ProcessingDymutilFailed`, `emit_warn`).
    The build succeeds, the loose objects are still deleted, and the binary's own-crate frames
    lose file:line. `-D warnings` does not cover it, because it is not a lint. P-6-13's V12 and the
    probe check for it.
  - A `dsymutil` that cannot be spawned is fatal (`UnableToRunDsymutil`, `emit_fatal`). The
    runner's `dsymutil --version` precondition guards against that.
- **`dsymutil` cost is the risk of `packed`.**
  - It runs synchronously inside rustc's link step, so its time falls inside the unit's duration
    in cargo's timings.
  - The Rust 1.51 release post: `dsymutil` "will need to run over the entire final binary", and
    "all dependencies always get recollected". That is why cargo's macOS default is `unpacked`.
  - It would also run on every test binary the macOS CI leg links (about 339).
- **Backtraces** [backtrace-rs `symbolize/gimli/macho.rs`]. std's symbolizer looks first for a
  `*.dSYM` next to the executable with a matching UUID. Failing that, it reads the object files
  named in the symbol table, including archive members (`ArchiveFile::parse` … `m.name() == member_name`).
  - `unpacked` and `packed` keep file:line for every frame.
  - `off` loses it on macOS only for frames compiled into the linked binary's **own crate**: a
    lib's unit-test harness (all of that crate's code), an integration test's own file, a bin's
    crate. Frames from rlibs keep it: every dependency, and the library under an integration test.
  - D (dependencies at `debug = false`) loses every frame in a non-member package, on both
    platforms.
  - Panic messages (`panicked at crates/x/src/y.rs:12:5`) come from `core::panic::Location`,
    which is compiled in, so no setting changes them.
  - lldb follows the same paths.
  - All REASONED until AC1's two frames.
- **Profiles are part of every artifact's hash** [cargo `compute_metadata`: "Throw in the profile
  we're compiling with"]. `Profile::comparable()` includes `debuginfo` and `split_debuginfo`, not
  the profile's name.
  - Changing the setting therefore rebuilds every crate once, under new file names, and the old
    artifacts stay until cleaned.
  - A test profile that differs from dev would also build every dependency twice, because cargo's
    selected profile applies to all units [cargo profiles, "Profile selection"]. That is why every
    candidate keeps `[profile.test]` equal to `[profile.dev]`. REASONED, backed by source.

### Candidates and the preference order (pre-registered; settled here, not after the run)

**Criterion: the breadth of the file:line loss.** Rank first by how many crates lose
backtrace/debugger file:line, then by how many platforms. A setting whose consequences stay on the
box where they are measured ranks above one that also changes the leg nothing local can measure.

1. **B: `split-debuginfo = "packed"`** in dev and test.
   - Loss: none, on either platform.
   - Risk: the `dsymutil` time per relink (121 ui test binaries per K4; about 339 on the macOS CI
     leg) and one dSYM per binary on disk. Bounded, because each dSYM is rewritten at a stable path.
2. **C: `split-debuginfo = "off"`** in dev and test.
   - Loss: **one crate per linked binary** (its own), on **macOS only**: an integration test's own
     file, a lib's unit-test harness (ui's 706 lib tests run inside ui's own crate), a bin's crate.
   - Kept: every dependency and the library under every integration test (rlib members); every
     panic location; Windows byte-for-byte as today (REASONED).
   - No `dsymutil`.
3. **D: B plus `debug = false` in `[profile.dev.package."*"]`** (the table that already sets
   `opt-level = 3`).
   - Loss: **every non-member package** (iced, tokio, polars, …, and the operator-maintained fork
     `vendor/iced_tiny_skia`, which enters through `[patch.crates-io]` and is not a member), on
     **both** platforms. The Windows change is unmeasurable locally.
   - Kept: every first-party member frame.
   - Role: the fallback if C FAILs, and an informational split of B's `dsymutil` cost (B vs D
     shows the dependency share).

**Why C ranks above D, and the counter-argument.**
- C ranks higher on both parts of the criterion: one crate against hundreds, and one platform
  against two. It also leaves untouched the leg that only CI observes, where the logs are 403
  anyway.
- The counter-argument: C loses first-party frames that D keeps, notably inside ui's lib unit
  tests. Those losses are bounded: the innermost panic site still prints (Location), and every
  frame in an rlib, which is most first-party code under the 118 integration-test targets, keeps
  its file:line.
- D's loss covers the vendored fork, whose `unreachable!()` panic is the AD-10 precedent.

**Every candidate:** `debug = "line-tables-only"` stays; `[profile.release]` and the dependency
`opt-level = 3` are unchanged; dev == test.

### Rejected without measurement (each goes into the ADR)

- **The prune, `cargo-sweep`, or any periodic cleanup**: it treats the effect. The operator
  declined it on 2026-08-19 (`c1474c0f`) and again on 2026-11-23. The one-off N_OSO prune from story
  2-63's Close must not come back as tooling.
- **Deleting the two lines**: on macOS that restores cargo's default, `unpacked`.
- **`off` (or `debug = false`) for `[profile.test]` alone**: dev and test would then hash
  differently, so every dependency (all at `opt-level = 3`) is compiled twice on the dev box and on
  both CI legs, and rust-cache's entry roughly doubles against GitHub's 10 GB limit (`ci.yml`'s
  cache comment, risk 1). REASONED from cargo's profile hashing.
- **`debug = false` for dev and test**: it loses file:line on Windows as well, and every macOS
  frame, for no gain over C.
- **`incremental = false`**: it would stop the accumulation, since rustc adds its per-invocation
  object-name suffix only under incremental (rust-lang/rust#139453, 1.88). It is rejected because it
  costs every edit-compile loop (the touched ui lib would rebuild from scratch).

### Protocol P-6-13 (binding: implement it, do not tune it)

**Binding text.** The binding text is this section **as committed at H** (AC2). H is the
pre-registration point.
- Protocol gaps found before H, including those the Wave 1 code review of the runner and the
  verdict finds (§ The instrument → Reviewed before H), are closed by amending this text and
  committing it in H.
- After H, no rule, threshold, arm or order changes. The only change that may land after H is
  § An instrument defect's narrow code fix.
- Results never feed back into the rules. In story 2-63 (D91–D99), two of three A/Bs were
  INCONCLUSIVE and the third passed unchanged.

**Arms** (`scripts/build_time_ab/arms-6-13.toml`, committed at H). The arms file declares:
- the control (A);
- the candidate order (B, C, D);
- each arm's Cargo.toml edits;
- each arm's expected `-C split-debuginfo=` value, and D's dependency override.

Each arm is a `git archive` export of H into the scratch root, edited only as declared:
- A (BEFORE, the control): unchanged; expects `unpacked`.
- B: `split-debuginfo = "packed"` in `[profile.dev]` and in `[profile.test]`; expects `packed`.
- C: `split-debuginfo = "off"` in both; expects `off`.
- D: as B, plus `debug = false` in `[profile.dev.package."*"]`. It expects `packed`, and no
  `-C debuginfo=` on non-member lines.

The runner applies the edits through the verdict
(`build_time_ab_verdict.py --apply-arm <arms> <arm> <export>`), so the edit and V0's expectation
come from one function. Bash parses no TOML.

**H.** H is the commit that last changed the runner, the verdict or the arms file. `--arms` must
resolve to a file tracked in the repo. The runner REFUSES if the runner, the verdict or that arms
file differs from its blob at H in the working tree.

**Runner order.** Each step runs only if the previous one succeeded.
1. The `--scratch-root`/`--out` checks. On failure, `REFUSED <reason>` goes to stdout only
   (exit 2), and nothing is created.
2. The other preconditions (§ The instrument → Runner).
3. The H check.
4. The exports, and the arm edits through `--apply-arm`.
5. V0, and the run-dir records:
   - each export's manifest (path → sha256) and a copy of its `Cargo.toml`;
   - a copy of the arms file;
   - the sha256 of the three instrument files and of their blobs at H;
   - each arm's export root.
6. The `cargo metadata` records (§ Environment). A failing `cargo metadata` is
   `REFUSED metadata <arm>`.
7. `BUILD-START <utc>`, written into the run dir immediately before arm A's P1, which is the first
   cargo call that builds.
8. P1–P4 per arm, the measured runs, then `DONE`.

A failure in steps 1–6 is a REFUSAL (§ Verdicts → Outcome classes; it does not use the re-run).
The verdict recomputes V0 from the recorded files rather than trusting a flag the runner set:
1. It checks the arms copy's sha256 against its recorded blob at H.
2. It applies the declared edits to A's `Cargo.toml` copy.
3. It requires every arm's copy to match the result byte for byte, and every other manifest entry
   to be equal across arms.

**Environment.** The dev box, alone for the whole session; the orchestrator holds it.
- The runner exports `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk` itself, as
  the prior art did. Plain cargo cannot link on this box.
- Toolchain from the export's `rust-toolchain.toml` (1.94.1). Default `-j`. `CARGO_INCREMENTAL`
  unset.
- Each arm gets its own fresh, empty `CARGO_TARGET_DIR`, under `--scratch-root` but outside
  `--out`. Its snapshots go there too. The real `target/` is never used: it carries `#218`'s
  accumulated state and old-hash artifacts.
- From each arm's `cargo metadata --format-version 1` (step 6), the runner records:
  - the workspace members' manifest directories (from `packages[].manifest_path`), for V11;
  - the **derived executable count**: ui's targets with `test: true` whose `required-features` are
    all among ui's resolved features (`resolve.nodes[].features`), for V3. ui has feature-gated
    targets (`cockpit` and `ui-gallery` need `fixtures`, `cockpit_live` needs `live`,
    `chart_build_probe` needs `chart-build-probe`), so counting every `test: true` target would
    be wrong.

**Preparation per arm (not measured).**
- P1: a cold `cargo test -p ui --no-run --timings -v`. Record the Executable list, the ui unit set,
  the rustc command lines, the wall (informational) and `du -sk` (informational).
- P2: one warm touch-build, discarded. Then record `du -sk` of the target dir **before** cloning
  (hard links intact).
- P3: the snapshot `S_X` is an APFS clone, `cp -c -R -p`, of the arm's target dir. `-p` keeps the
  mtimes that cargo's freshness checks read.
- P4: one restore from `S_X`, then `touch crates/ui/src/lib.rs; cargo test -p ui --no-run --timings`.
  - It is recorded like a measured run (Each run, steps 1–3).
  - It is judged through `build_time_ab_verdict.py --prep <run-dir> <arm>` (§ The instrument).
    `--prep` applies every V-rule that has data at that point: V1–V10, V11 from the arm's P1, and
    V12 for its own arm's side.
  - `--prep` exits 0 when no rule fires and 3 when one fires (first line `FIRED <rule>`). Any other
    exit is a crash.
  - On exit 3 the runner writes `ABORT <arm> P4 <rule>` and stops without `DONE`. On any other
    non-zero exit it writes `CRASH prep <arm> <exit>`, saves `--prep`'s stderr, and stops without
    `DONE`. Either way a misconfigured arm costs minutes, not the whole session.
  - The cross-arm halves of V3 and V4 are judged only by the final verdict.
- Disk:
  - After A's P1–P4, ABORT early if free disk < 3 × A's P2 `du -sk` + 25 GiB, before three more
    cold builds are spent.
  - After every arm's P1–P4, free disk must be ≥ 25 GiB.
  - A disk ABORT writes `ABORT <arm> DISK <free GiB> <need GiB>` and stops without `DONE`.

**Measured runs: 4 rotated 7-run palindromes, 28 runs.**

| block | order |
|---|---|
| 1 | `A B C D C B A` |
| 2 | `B C D A D C B` |
| 3 | `C D A B A D C` |
| 4 | `D A B C B A D` |

- Each block is a palindrome with a single centre run. Every arm's mean position in every block is
  4, so linear drift still cancels for every pair.
- Each arm is the single centre exactly once, gets 7 runs (2 + 2 + 2 + 1), and **no arm ever runs
  twice in a row**. The block boundaries are A|B, B|C and C|D.
- Why not an 8-run palindrome: it always repeats one arm back to back in its middle. In D99 the
  second run of such a pair was 3.7–12.2% faster in 4 of 4 pairs (40.83 → 35.89 s, …). That is
  the size of the 10% yardstick, so the block where X or A is the doubled middle would sit about at
  the NOISE bound by construction.
- In a block where an arm is the centre, its per-block mean is that single run.

Each run:
1. Outside the timer:
   - replace the arm's working target dir with a fresh `cp -c -R -p` clone of `S_X`;
   - record `du -sk` of the working target and a sorted listing of `debug/deps`;
   - `sync`, then settle 10 s;
   - record swap and Pageouts.
2. Timed: `touch crates/ui/src/lib.rs` in the arm's export, then
   `cargo test -p ui --no-run --timings`.
3. Outside the timer, record:
   - the exit code, the cargo log and the timings HTML;
   - the names added to and removed from `debug/deps` since step 1 (`comm` of the two sorted
     listings), with their counts;
   - swap and Pageouts again;
   - `du -sk` again (informational);
   - the dSYM checks.

The wait loop polls every 1 s. Every 20 s the sampler writes `SAMPLE <t> own=<n> foreign=<m>`.
`own` counts the cargo and rustc processes that are the run's cargo or descend from it; `foreign`
counts the others, and each of them is also listed. A run is killed at 1800 s and gets a
`V9 TIMEOUT` record.

**Why restore every run.** The arms differ in accumulation by construction. Without the restore,
A alone would drift upward as its `deps` grew (`#218`'s mechanism), and the spread rule would read
the very effect being fixed as noise. Restoring puts every run of an arm at the same state, so K4
compares per-build cost and ACC measures accumulation. This favours A.

Cost: about 20–40 s per restore (a whole pruned real target cloned in 54–61 s), times 28 measured
runs plus 4 P4 restores. That is roughly 15–20 min for the session.

**Wall** = max(start + duration) over the units in the timings HTML. It must agree with cargo's
`Finished … in` line (V10). Never a shell timer: the D96 defect quantized walls to 20 s.

**VOID rules.** Any one, in P4 or in any measured run, makes the session INCONCLUSIVE. There is no
substitute run except ON INCONCLUSIVE's pre-registered replicate.
- V0 (recomputed by the verdict from the recorded files, as above): an arm's export differs from
  A's beyond its declared edits, or the instrument files differ from H. The runner refuses before
  building, so a V0 VOID means the runner failed to refuse.
- V1: exit ≠ 0.
- V2: `Compiling` ≠ exactly `[ui]`.
- V3: fires when any of these holds:
  - the Executable count of P1, of P4 or of any run ≠ the arm's derived executable count;
  - that derived count is 0;
  - the four arms' derived counts differ.

  A run dir whose cargo logs contain no Executable lines therefore fires V3, so an Executable parse
  that fails cannot pass silently.
- V4: the ui unit set ≠ the arm's P1 set, or the four P1 sets differ.
- V5: the ui lib unit is not exactly one unit, or takes > 120 s. Cargo's timings name its target
  `""`, not `"lib"`: the D96 defect.
- V6: swap grows > 1024 MB, or Pageouts grow > 65,536, during the run (after minus before, from
  steps 1 and 3).
- V7: fires when any of these holds:
  - a sample has `foreign` ≥ 1;
  - a run (or P4) whose wall is ≥ 20 s has no `SAMPLE` line;
  - no `SAMPLE` line of the run has `own` ≥ 1.

  A sampler that never ran cannot pass as a clean box.
- V8: any export file other than the touched one changes between the session's start and end
  (checksums).
- V9: a run takes > 1800 s (a `V9 TIMEOUT` record).
- V10: the `Finished … in` line is missing, or it disagrees with the timings wall W. Cargo prints
  two forms, and both are parsed:
  - `in S.SSs` (under a minute): fires when |W − F| > 1 s.
  - `in Mm SSs` (a minute or more; whole seconds, truncated; F = 60·M + SS): fires when
    W < F − 1 or W ≥ F + 2.

  Prior art: A4b's wall was 76.10 s against `1m 16s`. B and D add `dsymutil` to 121 relinks, so
  walls of 60 s or more are expected.
- V11 (from P1) fires when any of these holds:
  - an arm's `--crate-name ui` rustc lines lack the expected `-C split-debuginfo=` value
    (`unpacked` for A, `packed` for B and D, `off` for C);
  - for D, a non-member line still carries `-C debuginfo=`, or a member line lacks
    `-C debuginfo=line-tables-only`;
  - P1's log yields no `--crate-name ui` rustc line;
  - for D, P1's log yields no member line, or no non-member line.

  The D clause is two-sided: the override must reach non-members and must not reach members. The
  empty-log clauses stop the rule passing on nothing.

  **The member key.** A rustc line belongs to a member if and only if its input source path lies
  under the manifest directory of a workspace member, as recorded at export time. Every other line
  is a non-member line: registry or git sources, or a non-member path package such as
  `vendor/iced_tiny_skia`.
  - Cargo prints a path package's source relative to the workspace root, which is rustc's cwd
    (`crates/ui/src/lib.rs`, `vendor/iced_tiny_skia/src/lib.rs`). A registry or git source
    appears as an absolute path [cargo `path_args`, REASONED from source].
  - A relative input path is joined to the arm's recorded export root, never resolved against the
    verdict's own cwd. Both sides are then compared as physical paths (after `realpath`).
  - The crate name is NOT the key. ui's 118 integration tests and its bins compile under their own
    crate names, and `replay-cache` compiles as `replay_cache`; keyed on crate name, V11 would fire
    on every D session.
  - No member has a `build.rs` or is a proc-macro, so the source path is a clean key.
- V12, B and D: a listed executable lacks a non-empty
  `<exe>.dSYM/Contents/Resources/DWARF/<basename>` with mtime ≥ the run's start, or the cargo log
  mentions `dsymutil`.
- V12, A and C: a `.dSYM` newer than the run's start exists in `debug/deps`.

**Verdicts.** Only the committed verdict script computes them, from the run dir; nothing is judged
by eye.
- **ACC (the cause).** A run's "added set" is the names in its step-3 added record. The test is on
  names, not on a net count: a run that removes one name and adds another is not clean.
  - Control: every A run's added set holds ≥ 1 `*.rcgu.o`. If one does not, the instrument cannot
    see the effect. The **session** is then INCONCLUSIVE (exit 2) before any candidate is judged,
    even if every candidate's K4 FAILs. Per-candidate results are still printed, as information.
  - Candidate X PASSES if its added set is empty in every X run. Otherwise it FAILS, and the verdict
    prints the added names (first 20 per run), so a reviewer can tell objects (the subject) from
    some other per-build file (also accumulation, but a different finding to record).
- **K4 (the cost).** Per block k, `r_k(X) = mean(X runs in k) / mean(A runs in k)`. Evaluated in
  this order:
  1. FAIL(X): `min r_k > 1.10`, whatever the spread, because every block puts X over the bar.
  2. Otherwise NOISE(X): `max r_k − min r_k ≤ 0.10`, or X's K4 is INCONCLUSIVE.
  3. Then PASS(X): `max r_k ≤ 1.10`.
  4. Anything else: INCONCLUSIVE.

  1.10 is the project's existing K4 yardstick (story 2-63 AC5, "≤ 10%").
- **Candidate result.** FAIL if ACC or K4 is FAIL, whatever the other says. PASS if both PASS.
  Otherwise INCONCLUSIVE.
- **DISK: informational, never a verdict.** Record per arm:
  - P1's and P2's `du -sk`, both taken before any clone. A `cp -R` clone splits hard links, and
    `du` then counts each copy.
  - The total size of `*.dSYM` in `debug/deps`.
  - A's per-run `du -sk` delta, step 3 minus step 1, both on the clone. This delta is not A's
    real-life growth: K4's touch-only builds re-link reused CGU work products, so their new objects
    add names, not bytes (D99: 167,820 objects deleted, 0 GiB freed).

  The full-workspace footprint is AC8's figure, recorded in ADR-0103.
- **CHOICE.** The first of **B, C, D** (§ Candidates; also declared in the arms file) whose
  candidate result is PASS. A candidate is passed over only on a FAIL, never on an INCONCLUSIVE.
- **Outcome classes.** The verdict's first output line names exactly one class.

  | class (first line) | meaning | exit |
  |---|---|---|
  | `CHOICE: <arm>` | a candidate passed | 0 |
  | `ALL FAIL` | every candidate's result is FAIL, with a valid control | 1 |
  | `REFUSED <reason>` | a `REFUSED` line in the run dir or on the runner's stdout, or a run dir with no `BUILD-START` | 2 |
  | `ABORTED <arm> <rule>` | an `ABORT <arm> P4 <rule>` or `ABORT <arm> DISK …` line | 2 |
  | `CANNOT MEASURE <what>` | `BUILD-START` with no `ABORT` and no `DONE` (a runner killed or crashed after it); a `CRASH prep` line; a missing file; unparseable timings; an uncaught exception in the verdict (`CANNOT MEASURE verdict-exception <type>`) | 2 |
  | `VOID <rule> <run>` | a V-rule fired in a measured run | 2 |
  | `INCONCLUSIVE <reason>` | a flat ACC control, or a non-FAIL candidate INCONCLUSIVE ahead of any PASS | 2 |

  - **One form for the result.** It is the first line, exactly `CHOICE: <arm>`. "No CHOICE line"
    means no line matching `^CHOICE:`, and no other line starts with `CHOICE`.
  - **Precedence.** The first line is the first class that holds, in this order: REFUSED, ABORTED,
    CANNOT MEASURE, VOID, INCONCLUSIVE (flat control), then the CHOICE rule (`CHOICE:`,
    `ALL FAIL`, or INCONCLUSIVE for a non-FAIL candidate).
    - Within VOID: V0 first, then the earliest run in session order, then the lowest rule number.
    - A run with exit ≠ 0 or a `V9 TIMEOUT` record is judged by V1 and V9 before its files are
      required, so it is a VOID and not CANNOT MEASURE.
    - Every other cause that holds is printed after the first line, one per line.
- **A REFUSAL is not a session.** It is recorded, the cause is fixed (for example a stray
  rust-analyzer cargo, or too little disk), and the runner is launched again into a fresh `--out`.
  It does not use the re-run.
- **ON INCONCLUSIVE** covers every other exit 2: `ABORTED`, `CANNOT MEASURE`, `VOID`,
  `INCONCLUSIVE`.
  - Record everything. Nothing ships.
  - The orchestrator re-runs this same committed protocol ONCE, unchanged, under its standing
    authority. The re-run is pre-registered here, so it is not a substitute run. Both results are
    recorded, and the re-run's stands (the D96 precedent).
  - If the recorded cause needs the operator's hands, the orchestrator asks for that ACTION, as a
    six-section recipe (AGENT.md duty 5), not for a ruling. V6 swap growth needs a reboot; V7 may be
    the operator's own cargo or rust-analyzer.
  - A second INCONCLUSIVE, or `ALL FAIL` (exit 1), holds the story at `in-progress`. The result
    goes into the Dev Agent Record and a `#218` addendum, reported as "held because Y". The only
    exception is the verdict mis-read case of § An instrument defect.
- **An instrument defect** (orchestrator rulings, rounds 2 and 3). "Unchanged" binds this
  section's text, thresholds, arms and order, not the instrument's code. A defect fixed and
  committed under the rules below does not count as changing the protocol.
  - An instrument defect means exactly one of two things:
    1. the runner or the verdict crashed, or exited on something other than a rule of this
       section;
    2. it mis-read data in a way its `--self-test` can prove.

    Nothing else counts: a threshold or a design choice that turned out badly is not a defect.
  - The fix commit H′ touches only the three instrument files. It carries a self-test pair, in the
    `--self-test` of the file that has the defect, that reproduces it: the pair fails on H and
    passes on H′. It changes no rule, threshold, arm or order, so this section is byte-identical
    between H and H′.
  - **When it may land**, and what it costs:
    - After a REFUSAL caused by an instrument defect, before the first session. No session is
      used.
    - After the first session, before the replicate, in two cases:
      - **A runner defect, or a `--prep` defect.** The runner acted on it and stopped, so its run
        dir has no complete measurement. The replicate is a new session under H′.
      - **A defect in the final verdict alone**, one that changed nothing the runner did or
        recorded. The H′ verdict re-judges the SAME run dir (its session-dir copy, Wave 2). That
        re-judgement **is** the one replicate, and its result stands; no second session follows.
  - The verdict-alone case also applies when the first session ends in `ALL FAIL`, or in
    `CHOICE:` before the AC4 commit, and a verdict mis-read is found that its `--self-test` can
    prove. A mis-read found after the AC4 commit is a bug-log entry and an ADR-0103 § Changelog
    line, not a re-judgement.
  - Both sessions' (or both judgements') outputs are kept and disclosed, with H and H′, in the Dev
    Agent Record and the ADR.

**Informational, never a verdict:** each arm's cold-build wall and sizes; any comparison with
another day's numbers (342 s, 563 s, 36–54 s). A's walls here are fresh-target walls, **not** the
real-life "before". The real "before" is `#218`'s accumulated target, which is worse.

### The instrument: why it moves into `scripts/` (decision, with reasoning)

**Decision: commit it**, as `scripts/build_time_ab.sh`, `scripts/build_time_ab_verdict.py`,
`scripts/build_time_ab/arms-6-13.toml` and `docs/runbooks/build-time-ab.md`. It does not stay a
dev-box procedure.
- It was needed three times in one day and is needed again now. It is also the measurement any
  future profile change or toolchain bump should face (ADR-0091: "raising the pin becomes
  deliberate").
- Two of the three 2026-11-23 A/Bs were INCONCLUSIVE partly from defects in a one-off verdict
  script: the lib unit keyed `"lib"` instead of `""`, and walls quantized to 20 s. A committed
  verdict with fixtures for exactly those defects keeps them fixed (bug-log `#187`: make the
  success path observable).
- The prior art lives in a session dir outside the repo, and the scratch copies were wiped by the
  reboot (D97/D98). A fresh dev context cannot rely on it.

**Reviewed before H (binding; orchestrator decision 2026-11-23).** The runner and verdict
implementations are reviewed against this story's protocol text in the dev story's code review
(`bmad-code-review`, fresh context) BEFORE commit H. Any protocol gap found there is closed by a
protocol-text amendment committed in H, not after it. The pre-registration point stays H.

The house rules shape it:
- **`#189` (subject).** The verdict's judging modes (the final verdict and `--prep`) are hermetic.
  They use the stdlib only and read the run dir and nothing else: no git, no cargo.
  - The prior art's `git ls-tree` and its hard-coded `EXE = {A: 119, B: 121}` become values the
    runner records at export time.
  - The control, the candidate order, each arm's declared edits and its expected flags come from
    the run dir's copy of the arms file, after its sha256 is checked against the recorded blob at
    H. There is no second copy of the arms in the verdict.
  - `--apply-arm` writes only the named export's `Cargo.toml`.
  - The runner's `--self-test` uses stubs and temp dirs only.
- **`#161` (derived, two-sided):** every threshold in the fixtures is derived from the script's
  declared constants, and every rule has a pair that differs in one value and expects the opposite
  result.
- **`#190` (wiring):** both `--self-test`s run in CI on every push. In the hook, each runs only
  when its own file or the hook is staged: its subject is code that can be fixed after the fact
  (the hook's own split rule).
- **`#191` (citation):** the runbook and the ADR say it is an instrument. Nothing runs the
  measurement automatically; the self-tests guard only the instrument's logic.

**The interpreter.** The runner calls the verdict through the hook's `uv run` / `python3` pattern,
and REFUSES if that interpreter reports a version below 3.11. Bare `python3` was 3.9.6 on
2026-10-10 and is `/usr/bin/python3` 3.9.6 today. Wave 2 runs
`uv run --quiet scripts/build_time_ab_verdict.py <run-dir>`.

**Verdict script** (`scripts/build_time_ab_verdict.py <run-dir>`; PEP-723 `requires-python = ">=3.11"`).
It implements every V-rule and verdict above, and prints every wall, `r_k`, spread, delta and size.
- Exit codes and the first line follow § Protocol → Outcome classes, with precedence. The result's
  one form is `CHOICE: <arm>`.
- The final verdict wraps its main. Any uncaught exception prints
  `CANNOT MEASURE verdict-exception <type>` as the first line and exits 2, so a crash never exits 1
  (`ALL FAIL`'s code) or prints anything that reads as a result.
- `--prep <run-dir> <arm>` judges one arm's P4 with every V-rule that has data at that point, plus
  V11 from P1 and V12 for the arm's own side.
  - It **uses the same functions as the final verdict**. There is no second copy of any rule in
    bash, where the D96 `"lib"`/`""` defect could live in one copy and not the other.
  - It exits 0 when no rule fires and 3 when one fires (first line `FIRED <rule>`). Any other exit
    is a crash.
- `--apply-arm <arms> <arm> <export>` applies the arm's declared edits to the export's `Cargo.toml`.
  It is the same function V0 uses to rebuild the expected copy.
- `--self-test`:
  - one firing fixture and one silent twin per V-rule and per verdict;
  - the D96 lib-target defect;
  - V10, both forms:
    - `in 30.00s` with W = 30.9 silent, W = 31.1 firing;
    - `1m 03s` with W = 64.95 silent, W = 65.05 firing;
    - `1m 03s` with W = 62.05 silent, W = 61.95 firing;
    - `1m 16s` with W = 76.10 silent (a parser that reads it as 16 s fails this one);
    - a missing `Finished` line firing;
  - V3: a run dir whose cargo logs have no Executable lines fires; the derived count excludes a
    target whose `required-features` are not enabled (a firing and a silent twin);
  - V7:
    - a 40-s run with no `SAMPLE` line fires, and its twin with `SAMPLE 20 own=3 foreign=0` stays
      silent;
    - `foreign=1` fires;
  - V11, with fixtures in cargo's real form, captured in Wave 1 and trimmed (prior-art logs hold no
    `-v` line to copy):
    - a ui integration-test line (relative `crates/ui/tests/…`, crate name ≠ package name)
      carrying `-C debuginfo=line-tables-only` stays silent;
    - a `replay_cache` line stays silent;
    - a relative `vendor/iced_tiny_skia/src/lib.rs` line carrying `-C debuginfo=` fires under D;
    - an absolute registry line carrying `-C debuginfo=` fires under D;
    - a P1 log with no `Running` lines fires, and its twin with them stays silent;
    - judged from a cwd other than the export root, every line classifies the same;
  - ACC on names: a run whose added and removed records each hold one name (net count 0) FAILS;
    its twin with both records empty PASSES;
  - V0:
    - an arms copy whose sha256 differs from the recorded blob fires;
    - an arm `Cargo.toml` with the declared edit plus one extra changed line fires;
    - the exact edit stays silent;
  - `--prep`:
    - a P4 record with V6's swap growth exits 3 with `FIRED V6`, and its twin exits 0;
    - a malformed timings HTML makes it exit neither 0 nor 3;
    - both share the rule functions with the final verdict;
  - a rule function that raises gives the first line `CANNOT MEASURE verdict-exception …`, exit 2,
    and no `^CHOICE:` line;
  - a run dir with no `BUILD-START` and no `REFUSED` line gives `REFUSED`; one with `BUILD-START`
    and no `DONE` gives `CANNOT MEASURE` (`#187`: the refusing path prints nothing that reads as a
    result);
  - precedence: V6 in run 3 together with a flat control prints `VOID V6 <run>` first, with the
    flat control after it;
  - ACC's flat control giving session INCONCLUSIVE, including when every candidate's K4 FAILs
    (exit 2, no `^CHOICE:` line, not exit 1);
  - one fixture per outcome class, each printing its class as the first line, and no `^CHOICE:`
    line except `CHOICE:`;
  - K4: `r = (1.30, 1.45, 1.35, 1.50)` → FAIL, so the candidate is passed over;
    `r = (1.05, 1.45, 1.35, 1.50)` → INCONCLUSIVE (no CHOICE);
  - B with ACC FAIL and K4 INCONCLUSIVE → B passed over, and the next candidate chosen;
    B with ACC PASS and K4 INCONCLUSIVE → no CHOICE;
  - the order B > C > D, read from the arms copy;
  - DISK values that differ wildly between arms do not change the CHOICE (a two-sided pair proving
    it is informational).

**Runner** (`scripts/build_time_ab.sh --arms <file> --scratch-root <dir> --out <dir>`). Bash with
arrays and `set -u -o pipefail`: zsh does not word-split, and there is no `timeout` binary.
- Port the structure of the prior art (`k4_ab2.sh`, below): preconditions, the disk guard, the
  sampler, the 1-s wait loop, RESULT lines.
- Add:
  - the runner order of § Protocol;
  - `--apply-arm`;
  - the run-dir records: manifests, arms copy, hashes, export roots, members and derived count;
  - `BUILD-START`;
  - P4 and its `--prep` exit handling (`ABORT … P4 …` on exit 3, `CRASH prep …` otherwise);
  - the disk `ABORT` format;
  - the per-run `cp -c -R -p` restore;
  - the step-1 `du` and listing, and the step-3 `comm` records;
  - swap and Pageouts before and after each run;
  - `SAMPLE` lines and the `V9 TIMEOUT` record;
  - the dSYM records;
  - a final `DONE`.
- These checks run first. They print `REFUSED <reason>` to stdout only and create nothing:
  - `--scratch-root` resolves (after `cd -P`) to a path inside `$(git rev-parse --show-toplevel)`;
  - `--out` exists and is not empty;
  - `--out`, an arm's export, an arm's `CARGO_TARGET_DIR` or a snapshot lies outside
    `--scratch-root`;
  - a target dir or snapshot lies inside `--out`. Targets stay outside it, so the run dir stays
    small enough to copy whole.
- The other preconditions REFUSE (`REFUSED <reason>` in the run dir, no `DONE`, before any cargo
  call) if any of these holds:
  - cargo or rustc is running;
  - less than 25 GiB is free;
  - `dsymutil --version` fails;
  - the SDK directory above does not exist, or the environment's `SDKROOT` is set to a different
    path;
  - the `uv run` / `python3` interpreter reports a version below 3.11;
  - a target dir is not empty at the start;
  - `--arms` is not a tracked file, or any of the three instrument files differs from H.
- It never writes inside the repo's working tree, so it never touches the real `target/`.
- Background loops capture `$!`, are bounded, are killed by `trap … EXIT`, and the script sweeps
  for its own strays before it exits. In 2026-11 an orphaned load test burned 11 of 14 cores for
  18 h. Use `grep -F`, not a regex, when matching its own command lines.
- **`--self-test`**, hermetic: temp dirs only, a scrubbed `GIT_*`, and stub `cargo`, `rustc`,
  `dsymutil`, `git` and `ps` first on PATH. The stubs write canned output: a timings HTML,
  `Finished` and `Executable` lines, a failing exit, a fake process table.
  - It drives one firing and one silent fixture for:
    - every precondition and path-containment check;
    - the H/arms check;
    - the foreign-process filter (own/foreign counting on the fake table);
    - each line writer: `REFUSED`, `BUILD-START`, `ABORT` (P4 and DISK), `CRASH prep`, RESULT,
      `SAMPLE`, `V9 TIMEOUT`, `DONE`;
    - the `--prep` exit handling (0, 3, other).
  - It makes no real cargo call, and asserts that no stray background process outlives it
    (`grep -F`).
  - A runner fix under H′ adds its reproducing pair here.

**Prior art** (read, port, do not call; outside the repo):
`~/.claude/projects/-Users-Vitaliy-Schreibmann-Projects-Privat-trading-trading/362d2a09-04ba-4ea6-a7c1-07605f6e187a/batch-gates/d92/`
- `k4_ab2.sh` (sha256 `2649399822c7b5f3…`): the runner.
- `k4_ab2_verdict.py` (sha256 `093cff94387914c9…`): the verdict.
- `ab3/`: the passing run's logs, 18 timings HTML files and `verdict.txt`. They are real self-test
  inputs for the timings and `Finished` parsers. They hold no `-v` rustc line, so the V11 fixtures
  come from Wave 1's capture. Copy the minimal structure, not the files.

The design history is in the decision log, `…/362d2a09-…/DECISIONS-b205-batch.md` § D91–D101.

### The probe (AC1)

- **Layout.** A scratch crate in the session scratch dir, with a copy of the repo's
  `rust-toolchain.toml` so it builds with 1.94.1 and not rustup's default.
  - Its `lib.rs` has two `#[test]`s. One calls an `#[inline(never)] fn probe_frame()` that
    panics.
  - The other calls `probe_dep::dep_frame()`, an `#[inline(never)] pub fn` that panics in a
    path-dependency crate, `probe_dep/`. That makes it an rlib member of the test binary.
- **Settings.** Each setting gets its own `CARGO_TARGET_DIR`.
  - Set `CARGO_PROFILE_TEST_DEBUG=line-tables-only`.
  - Set `CARGO_PROFILE_TEST_SPLIT_DEBUGINFO` to the value; leave it unset for "unset".
  - The fifth row (`packed` + dep `debug = false`) builds from a **copy** of the probe directory,
    whose `Cargo.toml` alone carries `[profile.dev.package.probe_dep] debug = false`. Rows 1–4 build
    from the copy without it, so their `dep_frame` reading is not an artifact of the setup. Record
    both manifests in the Dev Agent Record.
  - Naming the package keeps row 5 independent of whether cargo counts a path dependency as a
    member. It also measures that the test profile inherits a dev package override, which V11's D
    clause relies on.
- **Every cargo call for one setting** (build, run, rebuild) carries the same `SDKROOT`,
  `CARGO_TARGET_DIR`, `CARGO_PROFILE_TEST_DEBUG` and `CARGO_PROFILE_TEST_SPLIT_DEBUGINFO`.
  `CARGO_INCREMENTAL` stays unset. The run step must print no `Compiling` line; if it does, that
  row is void and is re-run.
- **Steps.**
  1. Build: `SDKROOT=… cargo test --no-run`.
  2. Run: `RUST_BACKTRACE=1 cargo test -- --nocapture`. Both tests are expected to fail; the run
     exits 101.
  3. Read the panic lines, and the `probe_frame` and `dep_frame` frames.
  4. Count `*.rcgu.o` and `.dSYM` in `debug/deps`.
  5. `touch src/lib.rs`, build again, count again.
- Record the exact probe text in the Dev Agent Record, so the probe can be re-run on the next
  toolchain bump.

### One-time clean (AC7)

- **Needed on the dev box, and it must be `cargo clean --profile dev`.**
  - The fix stops new objects, but it does not remove the ones already there. The profile change
    also makes every existing artifact stale (new hashes).
  - The rustc start-up cost scales with the entries left behind, so without a clean the box stays
    as slow as it is today.
  - Bare `cargo clean` "will delete the entire target directory" [cargo-clean docs]. `target/`
    holds directories cargo did not make: `visual-diff/`, `logs/`, `relock-1-28/`,
    `1-29-repro-failures/`, `134-anchor-dir-invariance/`, `reports_populated_curve.png` (from
    tests and tools). Cargo's own `cargo-timings/` and `tmp/` also sit outside `target/debug`.
  - `--profile` does "Remove all artifacts in the directory with the given profile name"
    [cargo-clean(1), 1.94.1]. For dev that is `target/debug`, which also holds the test profile's
    artifacts (test's dir-name is `debug`); `target/release` and every directory above survive.
  - A non-verbose `--dry-run` prints only a summary. With `-v`, every path goes to stdout, which
    is why AC7's check pipes it. Expect a walk of several minutes over today's deps
    (`ls -f` of 1.76 M entries took 60–64 s).
- **Expect a full rebuild afterwards.** All dependencies are at `opt-level = 3`; CI's cold
  `Build ui (fixtures)` took 17.8 min on macOS. Run Wave 4's commands with a watch recipe.
- **CI needs no clean, but needs a cache reset.**
  - On a key miss, rust-cache restores the newest entry under its prefix key. The first run after
    the AC4 commit would recompile every dependency under the new profile hash and could save both
    hash sets: rust-cache keeps dependency artifacts by crate name with the hash stripped, and
    pre-cleans only files older than a week (rust-cache `restore.ts`/`cleanup.ts`, read by the
    review).
  - So the AC4 commit adds `prefix-key: "v1-rust"` to the step, which makes it one genuinely cold
    run per leg. The step has no `prefix-key` today, so it uses the action's default `v0-rust`.
    Writing `prefix-key: "v0-rust"` would change nothing.
  - Record the macOS `Build ui (fixtures)` and workspace-test step times before (the last run
    before AC4) and after (the first run after it). Under B and D every test binary linked on the
    macOS leg runs `dsymutil`.
  - They go in the Dev Agent Record and in ADR-0103 § Changelog, in the follow-up commit (AC9).
- **Not ours to clean:** `.claude/worktrees/funny-snyder-b5c963/` (a leftover worktree; AGENT.md:
  propose cleanup, do not use it). If it has its own `target/`, report its size to the operator.

### Windows (REASONED; CI is the only Windows evidence)

- B and C: no change. MSVC's only supported value is `packed` (rustc docs). Cargo passes `packed`
  and drops `off` and `unpacked` [cargo `build_base_args`, `supports_debuginfo_split`], so `.pdb`
  output is the same as today.
- D: dependencies stop carrying line info, so Windows frames in non-member packages lose file:line
  too.
- Every arm: one cold CI run per leg (the `prefix-key` reset).
- `recipe_off_runtime_poll.rs` budgets 45 s (`PANIC_REPORT_WAIT`) for symbolization of "a cold
  Windows PDB". That is unchanged under B and C.

### Files

**UPDATE (read first; preserve what is listed):**
- `Cargo.toml`
  - Today: lines 42–62 are the 2026-08-19 comment block plus `[profile.dev]`/`[profile.test]`,
    each with `debug = "line-tables-only"` and `split-debuginfo = "unpacked"`. Line 64 is
    `[profile.dev.package."*"] opt-level = 3`; line 71 is `[profile.release] opt-level = 3`.
  - Change: the split lines (plus D's `debug = false`). Rewrite the comment so it states:
    - the cause (`#218`);
    - that `c1474c0f`'s `unpacked` line restated cargo's macOS default and is dropped on MSVC, so it
      changed nothing, and deleting it is no fix;
    - that deleting the new line would silently restore `unpacked` (hence the guard);
    - that its revert recipe's bare `cargo clean` deletes the whole of `target/`, so use
      `cargo clean --profile dev`;
    - the choice and its backtrace consequence, ADR-0103, and the guard script.
  - Keep the 40×-debug-tax block (lines 23–41), the `opt-level = 3` dependency override and
    `[profile.release]` exactly as they are.
- `CLAUDE.md`, Coding rules, the "Scope verification…" bullet. Its last sentence names the two
  settings. Rewrite it to name:
  - the chosen setting and its backtrace consequence;
  - ADR-0103;
  - the `#218` cause, in one clause;
  - `scripts/dev_profile_debuginfo_check.py`.

  Keep "`release` is untouched, so anchored runs and the measured cockpit render times are
  unaffected" and the rest of the bullet.
- `.githooks/pre-commit`
  - Every new gate uses the hook's own pattern, `if command -v uv …; then uv run --quiet "$root/scripts/X.py" …; else python3 "$root/scripts/X.py" …; fi`.
    Bare `python3` resolved to 3.9.6 on 2026-10-10 and `tomllib` needs 3.11 (`#189`, interpreter
    form).
  - Each instrument `--self-test` runs only when its own file or the hook is staged:
    - the verdict's when
      `git diff --cached --name-only -- scripts/build_time_ab_verdict.py .githooks/pre-commit` is
      non-empty;
    - the runner's (`bash "$root/scripts/build_time_ab.sh" --self-test`) when
      `scripts/build_time_ab.sh` or the hook is staged.

    This is how the hook already handles `bug_log_schema_check.py --self-test`. Otherwise each
    prints a "not run" line.
  - The AC6 guard runs live (`--staged`, reading `git show :Cargo.toml` with the hook's
    `GIT_INDEX_FILE` as given) and `--self-test` on every commit: a deleted line is cheapest to
    catch before the commit exists.
  - All three self-tests are hermetic: no git, or a scrubbed `GIT_*` if a fixture ever needs it
    (memory: hook env leaks into self-tests). The runner's uses a stub `git`.
  - Measure the hook's wall before and after, and put both in the commit message.
  - Do NOT touch the untracked `.githooks/post-checkout|post-commit|post-merge|pre-push`.
- `.github/workflows/ci.yml`
  - One step, `"Governance — build-time A/B verdict + dev profile debuginfo (bug-log #218)"`,
    `if: always() && matrix.os == 'macos-latest'`, `shell: bash`, shaped like its neighbours
    (`verdict_transcription_check.py`'s step).
  - Wave 1 creates it with both instrument `--self-test`s (verdict and runner). The AC4 commit adds
    the guard's live check and `--self-test`.
  - The AC4 commit also adds `prefix-key: "v1-rust"` to the `Swatinem/rust-cache@v2` step
    (lines 482–486 today: only `cache-on-failure` and `save-if`), with a comment.
- `_bmad-output/planning-artifacts/architecture/decisions/README.md`: the Registry row
  `| 0103  | … | accepted | <date> |` and the `updated:` marker (again in the follow-up, with the
  § Changelog line).
- `docs/dev-notes/bug-log.md`, `#218`: the second dated addendum in the AC4 commit; the flip to
  `fixed` in the follow-up. It is append-only: never rewrite the entry's measured text. The
  creation addendum is already in the working tree and goes into H.
- `docs/runbooks/ops-push-and-cache.md` § 2: `cargo clean` → `cargo clean --profile dev`, with the
  caveat, plus a § Changelog line (Wave 4).
- `_bmad-output/planning-artifacts/trace.toml`: the `REQ-SPLIT-DEBUGINFO-STOPS-ACCUMULATION-001` row
  (created with this story). Its `state` moves per AC9.
- This story; `sprint-status.yaml`; `CHANGELOG.md` (at `done`).

**NEW:**
- `_bmad-output/planning-artifacts/architecture/decisions/0103-<slug>.md`
- `scripts/build_time_ab.sh`, `scripts/build_time_ab_verdict.py`, `scripts/build_time_ab/arms-6-13.toml`
- `scripts/dev_profile_debuginfo_check.py`
- `docs/runbooks/build-time-ab.md`

**NOT touched:**
- `crates/**`: no Rust source changes. If a test needs editing, stop and route it; that is a
  finding.
- `vendor/**`, `evidence/**`, `docs/archive/**`, `docs/dev-notes/audit-*.md`, `rust-toolchain.toml`.
- `[profile.release]`, `[profile.dev.build-override]`: build scripts and proc-macros default to
  `debug = false`, so split-debuginfo never applies to them [cargo profiles, Build Dependencies].
- `architecture.md`. Its maintenance contract updates the spine only when an ADR changes the crate
  map, an invariant or the journey; this ADR changes none of them. AD-2 (anchors before and after)
  and AD-18 (ADR atomic) bind the process. AC8 carries AD-19's floor except rust-validate's
  audit/deny/doc legs, which a profile-only change cannot move.

### Constraints carried (verified in the sources)

- **CLAUDE.md**
  - The Coding-rules bullet moves with the change (AC4).
  - `release` stays untouched.
  - Anchors 119/119 before and after (AD-2).
  - ADR plus Registry row in one commit (AD-18).
  - No `.unwrap()` in library code (there is no library code here).
  - "Scope verification to the crates you touched" is waived by its own exception, "when a change
    is genuinely cross-cutting". A profile change is.
- **AGENT.md**
  - Sub-agents write files; the orchestrator commits.
  - Anything over 2 min gets a watch recipe.
  - Human-verification recipes have six sections.
  - The orchestrator re-runs the gates before each commit; Wave 4 precedes the AC4 commit for that
    reason.
  - The final suite runs alone.
- **Decide-and-proceed.**
  - None of this story's outcomes is a stop category (push/publish, deleting evidence, shipping on
    REGRESSION, the public thesis).
  - The only operator touchpoints are the push, and an ACTION (a reboot, quitting their apps) that
    ON INCONCLUSIVE may need.
- **The do-not-build register** (`docs/dev-notes/do-not-build-register.md`) covers product
  dead-ends. Checked: nothing in it touches build profiles.
- **Dev-box facts** (memory, re-proved 2026-11-09):
  - every cargo call needs `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk`;
  - there is no `timeout` binary, and `$PIPESTATUS` is bash-only, so write scripts for bash;
  - measure `du -sh target/debug/incremental` before any wide build (it was 40 GB on 2026-11-10);
  - the box has 14 cores and 24 GB RAM; swap was 26 GB before the 2026-11-23 reboot.
- **CI**: read once after the push, never polled (60 requests an hour by IP; bug-log `#172`). Use
  `gh run view <id> --json jobs` first; logs are 403.

### Previous story intelligence (2-63 Close, decisions D91–D101)

- **K4** is `touch crates/ui/src/lib.rs; cargo test -p ui --no-run --timings`. At `bd48e76d` it
  lists 121 executables (1.47 GB in total, 12 MB mean). Count them per run; never hard-code the
  number.
- **Drift.**
  - Walls drifted upward in **both** arms within a session (36 → 53 s in D99).
  - Fixed-order designs failed on it (D96). ABBA blocks plus a spread rule passed (D99).
  - In D99 the second run of each B pair was faster in 4 of 4 pairs.
  - P-6-13 keeps the palindrome-plus-spread-rule idea but uses 7-run single-centre palindromes,
    so no arm runs twice in a row (because of that second-run speed-up). It rotates them and adds
    the snapshot restore, because its arms differ in accumulation and D99's did not.
- **Swap.** V6 tripped once on swap growth (24 → 26 GB of swap in use, D97). A reboot brought swap
  to 8 GB, and the same protocol then passed. Docker was running both times; V6 judges, not
  Docker's presence.
- **Prior timings.** An APFS clone of a pruned real target took 54–61 s. `ls -f` of 1.76 M entries
  took 60–64 s (116 s cold); count entries only on the scratch targets and on the real target
  before and after the clean.
- **V5.** The ui lib unit's frontend once took 2–2,880 s on identical inputs (cause unknown,
  `#218`). V5 exists for exactly that.
- **D101** withdrew "a prune lasts ~100 builds". `#218` now says ~5 builds to the 200 k mark and
  ~115 back to 1.76 M (REASONED, a linear extrapolation). Nothing here depends on either figure.
- **Probes that glob** `target/debug/deps/<test>-*` (for example `nm -a … | grep -ci write_overlay`
  in `docs/dev-notes/cannot-fail-gate-sweep-2026-11-07.md`) will also match
  `<test>-<hash>.dSYM` directories under B/D, and `nm` prints an error for them. It is not a
  committed gate; note it in the ADR.

### Git intelligence

- `22ab3a20`, `bd48e76d`, `408fb46e`, `72a42a8c`, `8eabf996`: story 2-63's last wave and Close.
  `408fb46e` is the pattern for adding a self-testing script plus a CI step:
  `scripts/ci_test_target_counts.py`, with numbers declared in the script, a `--self-test` that can
  fail, and a step on both legs.
- `c1474c0f` (2026-08-19) added the profile lines. Its message records the `cargo-sweep` decline
  and promises a full-workspace size "on the next `--all-targets` run". That size was never
  recorded; AC8 records it.
- **No test asserts symbolized output.** MEASURED by grep at story creation:
  - `grep -rln -i backtrace crates --include='*.rs'` returns
    `crates/ui/tests/recipe_off_runtime_poll.rs` (comments on the symbolization time budget) and
    `crates/backtest/tests/null_data_no_crown.rs` (one unrelated comment, line 810);
  - `panicked at` appears only in doc comments;
  - no manifest depends on `backtrace`, `color-eyre` or `addr2line`.
- The tests that run binaries use exact paths: `determinism.rs` copies `target/debug/backtest`,
  and `trainer.rs` looks up `target/debug/train_tcn`. None glob, so `.dSYM` siblings cannot be
  picked up.

### Latest technical information (fetched 2026-11-23; re-checked against 1.94.1 by the review)

- [rustc codegen options, `split-debuginfo`](https://doc.rust-lang.org/rustc/codegen-options/index.html#split-debuginfo):
  the values and platform defaults; "`packed` is supported on Windows-MSVC"; unsupported values
  need nightly.
- [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html):
  - `split-debuginfo` defaults to `unpacked` on macOS for profiles with debug info;
  - `test` inherits `dev`;
  - "The selected profile applies to all Cargo targets";
  - build-override defaults to `debug = false`.
- [cargo `build_base_args`](https://doc.rust-lang.org/stable/nightly-rustc/src/cargo/compiler/mod.rs.html):
  the flag is passed only when debuginfo is on and `supports_debuginfo_split(split)` holds.
- [rustc `back/link.rs`](https://doc.rust-lang.org/stable/nightly-rustc/src/rustc_codegen_ssa/back/link.rs.html):
  `preserve_objects_for_their_debuginfo`, the Darwin `dsymutil` arm (warn on a non-zero exit, fatal
  when it cannot be spawned), and `add_static_crate` (rlibs linked as archives).
- [cargo `compute_metadata`](https://doc.rust-lang.org/stable/nightly-rustc/src/cargo/compiler/build_runner/compilation_files.rs.html):
  the profile is hashed into every unit's metadata and file names.
- [cargo `cargo_clean.rs`](https://doc.rust-lang.org/stable/nightly-rustc/src/cargo/ops/cargo_clean.rs.html):
  a dry-run prints bare paths to stdout only under `-v`, the `Summary` line through `status`, and
  `--profile` joins the profile's `dir_name`.
- [backtrace-rs `macho.rs`](https://github.com/rust-lang/backtrace-rs/blob/master/src/symbolize/gimli/macho.rs):
  the dSYM lookup next to the executable, then the symbol table's object map, including archive
  members.
- [Rust 1.51.0 release post](https://blog.rust-lang.org/2021/03/25/Rust-1.51.0/): why `unpacked`
  exists, namely `dsymutil`'s cost over the whole binary.
- [cargo clean](https://doc.rust-lang.org/cargo/commands/cargo-clean.html): bare `cargo clean`
  deletes the whole target directory; `--profile`, `--dry-run`, `--verbose`.
- rust-lang/rust#139453 (1.88): the per-invocation object-name suffix, under incremental only.

AC1 measures what this story relies on.

### Decisions made at story creation (standing authority; the orchestrator may override)

1. **Epic 6, key `6-13`.** "Remediation, Infra & Governance" is the board's infra epic, and 6-12 is
   its last story. `epics.md` is not edited: like 6-11 and 6-12, this story is board-native.
2. **Four arms, ranked B > C > D by the breadth of the file:line loss** (§ Candidates). The bug's
   test-only `off` becomes dev+test `off` (C), because a split profile doubles dependency builds.
   D stays as the fallback if C FAILs, and as the informational split of `dsymutil`'s cost. The
   order is pre-registered, settled before any result exists (orchestrator ruling).
3. **The snapshot restore per run, and 7-run palindromes** (§ Protocol). The restore costs about
   15–20 min; without it the spread rule would read the fix's effect as noise. The single-centre
   blocks mean no arm runs twice in a row, so D99's ~10% second-run speed-up cannot land inside one
   block.
4. **The instrument moves into `scripts/`** (§ The instrument). The arms live in a committed arms
   file, not hard-coded in the runner, so the runner stays reusable.
   - The arms file also carries the control, the order and the expected flags.
   - The verdict reads them from the run dir's copy, after checking its hash against H.
   - The runner applies the edits through the verdict's `--apply-arm`.
   - So there is one copy of the arms, it is in git at H, and nothing may drift from H.
5. **The static guard (AC6).** The `#218` trap is that deleting the line silently restores
   `unpacked`, and the last time it took from 2026-08-19 to 2026-11-23 to notice. If the
   orchestrator cuts AC6, the ADR's "Enforced by" must say "nothing automated" (`#191`).
6. **The trace row starts at `state = "design-complete"`, not the usual `scoped`.** `scoped` is
   not in `STATE_TO_STORY_STATUS`, so `status-drift` silently skips such stories: the `#188`/`#216`
   family. `design-complete` maps to `ready-for-dev`, so the lint checks this story from day one.
   Every Status move therefore moves the row in the same edit:
   - `in-progress` → state `in-progress`;
   - `review` → state `dev-done`;
   - `done` → state `shipped`.

   The story's own `- Trace:` line `(state=…)` moves with it.
7. **ON INCONCLUSIVE is a pre-registered replicate, not an operator ruling** (orchestrator ruling).
   DISK is informational (orchestrator ruling). An instrument defect, narrowly defined, may be fixed
   and committed before the replicate without counting as a protocol change (orchestrator ruling,
   round 2). A REFUSAL is not a session and does not use the replicate.
8. **H is the pre-registration point, and the instrument is reviewed before it** (orchestrator
   decision, round 3).
   - The runner and verdict implementations are reviewed against the protocol text in the dev
     story's code review BEFORE commit H.
   - Any protocol gap found there is closed by a protocol-text amendment committed in H, not after
     it.
   - After H, only § An instrument defect's narrow code fix may land.
   - This was the last paper review of P-6-13. The remaining defect class (the instrument's own
     implementation) is caught against code, by the self-test fixtures and that review.

### Open questions (none blocks ready-for-dev)

- **If P-6-13 is INCONCLUSIVE**, ON INCONCLUSIVE applies under the orchestrator's authority. The
  only operator request it may produce is an ACTION (a reboot, or quitting an app), sent as a
  six-section recipe.
- **If C ships**, it widens the ruled option (`off` for `[profile.test]`) to `[profile.dev]` as
  well, for the double-build reason in § Rejected. That is a design decision under
  decide-and-proceed, in none of the four stop categories. It is recorded in ADR-0103 and the
  CLAUDE.md bullet, with AC1's measured frame consequence, and not asked.

### Project Structure Notes

- Scripts follow the house pattern: a stdlib Python gate with a PEP-723 header and `--self-test`,
  wired through the hook's `uv run` / `python3` pattern and into CI, with its numbers declared in
  the script.
- Runbooks go under `docs/runbooks/`. ADR file names follow `NNNN-kebab-title.md`. The arms file
  sits in `scripts/build_time_ab/`, beside its runner.
- No variance from the unified structure. The only new kind of file is a measurement instrument in
  `scripts/`; `scripts/bench_idle_cpu.sh` and `scripts/codegraph_bench/` are precedents.

### References

- Trace: `REQ-SPLIT-DEBUGINFO-STOPS-ACCUMULATION-001` (state=`design-complete`); the row lives in
  [trace.toml](../planning-artifacts/trace.toml).
- Bug-log, in [bug-log.md](../../docs/dev-notes/bug-log.md): `#218` (the cause, the measurements,
  the proposed fixes, the ruling addendum); `#187`–`#191` (the gate rules); `#216` (the trace-parse
  lesson).
- [Cargo.toml](../../Cargo.toml) lines 42–72. [CLAUDE.md](../../CLAUDE.md) § Coding rules and
  § Non-negotiables. [AGENT.md](../../AGENT.md) § Orchestrator duties and § Parallelism rules.
- [architecture.md](../planning-artifacts/architecture.md): AD-2, AD-13, AD-18, AD-19.
- The [ADR Registry](../planning-artifacts/architecture/decisions/README.md);
  [ADR-0091](../planning-artifacts/architecture/decisions/0091-pin-the-rust-toolchain.md) (the
  toolchain pin);
  [ADR-0102](../planning-artifacts/architecture/decisions/0102-the-verification-envelope-is-two-legs.md)
  (the two CI legs).
- [Story 2-63](2-63-lab-recipe-test-harness-v0-3-extension.md), Dev Agent Record § Close (K4, the
  three A/Bs).
- [ci.yml](../../.github/workflows/ci.yml): the cache comment, the rust-cache step, the governance
  steps. [pre-commit](../../.githooks/pre-commit): the uv pattern and the conditional self-test.
- [recipe_off_runtime_poll.rs](../../crates/ui/tests/recipe_off_runtime_poll.rs) (the
  symbolization time budget); [determinism.rs](../../crates/backtest/tests/determinism.rs) (binary
  copies under `target/debug`).
- [ops-push-and-cache.md](../../docs/runbooks/ops-push-and-cache.md) § 2 (edited in Wave 4).
- [do-not-build-register.md](../../docs/dev-notes/do-not-build-register.md) (checked; nothing
  applies); [sprint-status.yaml](sprint-status.yaml).

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

- 2026-11-23 (create-story): ultimate context engine analysis completed - comprehensive developer
  guide created. Nothing was built or measured at creation beyond the greps, the doc and source
  fetches and `dsymutil --version` cited above. Every behavioural claim about the settings is
  REASONED until AC1.
- 2026-11-23 (create-story, revision 1): applied the orchestrator's three-lens review (10 defects,
  22 nits; disposition in the orchestrator's handback) and its three binding rulings. The
  candidate order changed from B > D > C to B > C > D before anything was measured.
- 2026-11-23 (create-story, revision 2): applied the round-2 decidability review (9 defects,
  11 nits) and the ruling on instrument defects. Main changes:
  - outcome classes, with REFUSAL separated from a session;
  - P4 judged through `--prep`, with the same rule functions as the final verdict;
  - V10 parses both of cargo's `Finished` forms;
  - V3 checks against a count derived from `cargo metadata`;
  - V11 keys members on source path;
  - 7-run single-centre palindromes (28 runs);
  - a flat ACC control makes the session INCONCLUSIVE;
  - AC7's dry run must list something;
  - AC8's wide run must exit 0 and have run;
  - the AC6 guard also covers `debug` and dev == test.
- 2026-11-23 (create-story, revision 3, the last paper review): applied round 3 (9 defects,
  12 nits) and the orchestrator's decision that the instrument is code-reviewed against P-6-13
  before H, which stays the pre-registration point. Main changes:
  - the runner gains a stub-driven `--self-test`;
  - `--prep` exits 0 or 3, and anything else is a crash (`CRASH prep`);
  - verdict exceptions become `CANNOT MEASURE`;
  - `BUILD-START` separates a REFUSAL from a session, with a fixed runner order;
  - the arms copy and `--apply-arm` make V0 hermetic;
  - V11 joins relative source paths to the export root;
  - ACC is judged on names (`comm`), not net counts;
  - the V7 `SAMPLE` lines and V11's empty-log clauses close two vacuity gaps;
  - class precedence, one `CHOICE:` form, and the runner sets `SDKROOT` itself;
  - the guard covers package overrides, test-only keys and an unmeasured D;
  - the instrument-defect route also covers `ALL FAIL` / `CHOICE:` mis-reads before the AC4
    commit.

### File List
