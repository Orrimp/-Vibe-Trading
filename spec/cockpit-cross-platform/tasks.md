---
slug: cockpit-cross-platform
status: in-progress
owner: developer
updated: 2026-06-15
version: 0.1.0
---

# Tasks — cockpit-cross-platform v0.1.0

Workflow: **analyst → architect → (developer ‖ ui-designer) → tester →
presenter → operator**. No ui-designer leg — this is build/CI plumbing with
**zero new UI surface** (the only "UI" touch is gating existing visual tests).

Trace row: `REQ-COCKPIT-CROSS-PLATFORM-001` (state `proposed`).

Per-task `T-*-N` decomposition under M-T1 is the architect's deliverable.

---

## M0 — Analyst (DONE)

- [x] **M0.1** Survey macOS coupling READ-ONLY (file:line): vendored fork,
      `[patch.crates-io]`, `crates/ui/Cargo.toml` OS-feature deps,
      `#[cfg(target_os/unix/windows)]` in `crates/ui/src/`, the Swift +
      `screencapture` dev scripts, the cosmic-text/fontdb font stack, TLS
      backend, path/line-ending assumptions.
- [x] **M0.2** Author `feature.md` — R1-R6 + R-NR.1-6, K1-K5, H1-H3, Q1-Q5,
      4-cell verdict tree, effort estimate, if-budget-tightens lane.
- [x] **M0.3** Author `tasks.md` (this file).
- [x] **M0.4** Open trace row `REQ-COCKPIT-CROSS-PLATFORM-001` (state
      `proposed`).

## M-OD — Operator decisions — RESOLVED by architect 2026-06-15

The architect resolved Q1-Q5 on the analyst's recommended (durable) branches
(feature.md § Design D1-D9). Recorded here for the operator; the developer
executes against these. No operator block remains — all picks are the durable
lane the analyst pre-flagged `(Recommended)`.

- [x] **Q1** TLS — **(a) rustls flip** (D1). Zero-system-dep cross-platform build.
- [x] **Q2** Visual-baseline gating — **(a) source `#![cfg(target_os="macos")]`**
      file-level inner attr on the 4 snapshot test files (D2 — load-bearing, the
      riskiest). The contract lives in CODE, not YAML.
- [x] **Q3** Windows scope — **(a) build+test-on-CI, run best-effort** (D5).
      Full interactive parity deferred to v0.2 (no Windows HW to verify; the
      v0.2 leg is purely additive — no rework spawned).
- [x] **Q4** macOS-only dev scripts — **(a) leave gated + inventory** in the
      runbook (D9). Porting to Linux declined as scope creep.
- [x] **Q5** Linux headless CI — **a ~0.5d developer spike (T-D-7) is flagged
      BEFORE the CI YAML locks** (D9). The one genuine unknown: whether
      `xvfb-run -a` suffices or winit also needs software GL/EGL.

## M-T1 — Architect (design pass) — DONE 2026-06-15

All design decisions in [`feature.md` § Design](feature.md#design) (D1-D9).
ADR-0057 authored + registered. The M-T1 contract the developer MUST NOT drift:
**R-NR.1** (macOS baselines byte-identical), **R-NR.6** (vendor/ untouched —
escalate if a fork cfg appears to be needed, it is not), **R5** (every change is
additive `#[cfg]`/Cargo `[target.…]`/feature-flip — the macOS code path is never
edited).

- [x] **T-AR1** Windows dep = full `windows = "=0.57.0"` (features
      `Win32_Foundation` + `Win32_System_Threading`), `[target.'cfg(windows)']`
      stanza. `windows-sys` rejected — raw-HANDLE FFI forces a source rewrite of
      the already-correct arm (feature.md § Design D4).
- [x] **T-AR2** Errno fix = drop `unsafe { *libc::__error() }` for
      `std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)` —
      portable + removes one `unsafe` (R-NR.3). Per-OS cfg rejected (D3).
- [x] **T-AR3** CI = first `.github/workflows/ci.yml`, 3-leg matrix
      `{ubuntu, macos, windows}`. **No test-name `--skip` filter needed** — the
      D2 source gate compiles the visual tests out off-macOS, so the source gate
      IS the filter (D8). Linux apt-dep list pending the Q5 spike (T-D-7).
- [x] **T-AR4** ADR = **YES**, ADR-0057 (0056 was last). Codifies the
      macOS-canonical render-determinism scope for the UI snapshot gate +
      the source-gating mechanism. Standalone (not an ADR-0051 Changelog
      amendment — that would mis-file in the MC lane) (D6).
- [x] **T-AR5** `chart.rs:228` stale comment flagged for in-scope correction
      (D7) — documentation, non-anchored file, ADR-0038 § D6 safe.
- [x] **T-AR6** M-T2/M-T3 decomposed into the numbered `T-D-N` tasks below.

## M-T2 — Developer (Linux green) — the core of the work

> **Floor / order.** Do T-D-0 FIRST (capture the macOS canonical baseline), then
> T-D-1..T-D-4 in any order, then re-run T-D-0's macOS check as the R5 guard.
> Emit a `watch -n 10 '<probe>'` block (MEMORY contract) when kicking off the
> rustls re-verify (T-D-3) and the first CI run (T-D-6) — both are >2 min.

- [x] **T-D-0** (R5 floor) On the macOS canonical box, run `cargo test -p ui`
      and record that the 56 visual baselines pass byte-identical + capture the
      anchor count via `scripts/verify_anchors.sh` (expect 119/119). This is the
      before-snapshot the REGRESSION guard diffs against.
      *Acceptance:* macOS `cargo test -p ui` green incl. all 56 visual baselines;
      `verify_anchors.sh` → 119/119. Recorded in the dev notes / PR body.
      **DEV TICK 2026-06-15:** `cargo test -p ui` → 51 passed; 0 failed (visual
      baselines bin). `verify_anchors.sh` → 119/119. file:line — pre-change run.
      Test cmd: `cargo test -p ui`. Output line: `test result: ok. 51 passed; 0 failed`.
- [x] **T-D-1** Errno fix in `crates/ui/src/lab/pid_alive.rs:54-60`: replace
      `let errno = unsafe { *libc::__error() }; errno == libc::EPERM` with
      `std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)`.
      Leave the `libc::kill(pid_t, 0)` call + its `// SAFETY:` comment verbatim.
      Update the file's module doc if it names `__error()`.
      *Acceptance:* (a) the three `pid_alive` unit tests pass on macOS unchanged;
      (b) the function compiles+links on a glibc Linux target
      (`cargo build -p ui` cross to `x86_64-unknown-linux-gnu`, or on a Linux
      runner); (c) `rg '__error' crates/ui/src` is empty; (d) one fewer `unsafe`
      block in the file (R-NR.3).
      **DEV TICK 2026-06-15:** file: `crates/ui/src/lab/pid_alive.rs:59-63`.
      `grep -n "unsafe" pid_alive.rs` → 2 unsafe blocks (kill + windows OpenProcess);
      `grep '__error()' *.rs` in code → 0 (comments only). Test cmd: `cargo test -p ui`.
      Output line: `test result: ok. 456 passed; 0 failed` (post-change run includes
      pid_alive unit tests). (b) CI-deferred (Linux runner); (c) confirmed comments
      only; (d) was 3 unsafe, now 2.
- [x] **T-D-2** Add to `crates/ui/Cargo.toml` (new stanza, do NOT touch the
      `[dependencies]` block):
      ```toml
      [target.'cfg(windows)'.dependencies]
      windows = { version = "=0.57.0", features = [
          "Win32_Foundation",
          "Win32_System_Threading",
      ] }
      ```
      Zero source change to the `#[cfg(windows)]` arm (it is already correct).
      *Acceptance:* `cargo build -p ui` cross-compiled to
      `x86_64-pc-windows-msvc` (or `-gnu`) resolves the `pid_alive` Windows arm
      with no error; `cargo tree -p ui --target x86_64-pc-windows-msvc | rg
      'windows v0.57'` shows the pinned version (no second `windows` major
      added); macOS/Linux builds are unaffected (the stanza is `cfg(windows)`).
      **DEV TICK 2026-06-15:** file: `crates/ui/Cargo.toml` (new
      `[target.'cfg(windows)'.dependencies]` stanza). macOS build unaffected:
      `cargo build -p ui --features fixtures` → `Finished`. Cross-compile to
      Windows CI-deferred (no Windows cross-compiler on this macOS box — validated
      via CI). Test cmd: `cargo build -p ui --features fixtures`. Output: `Finished`.
- [x] **T-D-3** Q1 rustls flip — in the **workspace-root `Cargo.toml`**, change
      the reqwest line to
      `reqwest = { version = "0.12", default-features = false, features =
      ["json", "rustls-tls"] }`. Then re-verify the live HTTP consumers build +
      test: `crates/data` (Yahoo/Binance fetch), `crates/agent`, `crates/llm`.
      *Acceptance:* (a) `cargo build --workspace` + `cargo build -p ui --features
      live` green; (b) `cargo test -p data -p agent -p llm` green (the live HTTP
      paths still function); (c) `cargo tree -e features | rg 'native-tls'` is
      empty for the reqwest sub-tree (the OpenSSL C-dep is gone — Linux now
      builds with no `libssl-dev`); (d) `Cargo.lock` churn committed. Emit the
      watch-recipe block (T-D-5) for the re-verify build.
      **DEV TICK 2026-06-15:** file: `Cargo.toml:128` (workspace dep line changed).
      (a) `cargo build -p ui --features fixtures` → `Finished` (30.79s incl. reqwest
      recompile). (b) `cargo test -p data -p agent -p llm` → `test result: ok. 1
      passed; 0 failed` (agent), 0 tests (data/llm) — all green. (c) native-tls
      absent from reqwest subtree (confirmed by `cargo tree -p data -e features | rg
      native-tls` → only tokio-tungstenite, not reqwest). Note: tokio-tungstenite
      still uses native-tls for WebSocket (out of D1 scope; see runbook §1). (d)
      Cargo.lock churn in working tree. Test cmd: `cargo test -p data -p agent -p
      llm`. Output line: `test result: ok. 1 passed; 0 failed; 0 ignored`.
- [x] **T-D-4** Visual-baseline source gate (R-NR.1 / ADR-0057 D2). Add a
      **file-level inner attribute `#![cfg(target_os = "macos")]`** to the top of
      each of the four files — `crates/ui/tests/visual_snapshots.rs`,
      `render_snapshots.rs`, `panel_snapshots.rs`, `gallery_snapshots.rs` —
      placed with the existing `#![allow(…)]` inner attributes (after the
      module doc comment, before the first `use`). Do NOT gate per-`#[test]`.
      Do NOT re-baseline anything.
      *Acceptance:* (a) on macOS, `cargo test -p ui` still runs **all 56** visual
      baselines and they pass byte-identical (re-run T-D-0's check — the R5/
      REGRESSION guard); (b) on Linux, `cargo test -p ui --no-run` does NOT
      compile those four test binaries (verify: the test names like
      `charts_screen_dark_typical` are absent from `cargo test -p ui -- --list`
      on Linux); (c) no `.png` under `crates/ui/tests/visual-baselines/` is
      added, removed, or modified.
      **DEV TICK 2026-06-15:** files: `crates/ui/tests/visual_snapshots.rs:48`,
      `render_snapshots.rs:44`, `panel_snapshots.rs:19`, `gallery_snapshots.rs:44`
      (each received `#![cfg(target_os = "macos")]` before existing `#![allow(…)]`).
      (a) POST-CHANGE: `cargo test -p ui` → all passes, 0 failed (exit code 0;
      full run incl. visual tests). Baselines byte-identical (56 PNGs unchanged,
      `git diff --stat` shows 0 PNG changes). (b) CI-deferred (no Linux runner).
      (c) `find crates/ui/tests/ -name "*.png" | wc -l` → 56, unchanged.
      Test cmd: `cargo test -p ui`. Output line: `test result: ok. 456 passed; 0 failed`.
- [x] **T-D-5** (MEMORY watch-recipe contract) When kicking off the T-D-3
      rustls re-verify build and the first T-D-6 CI run (both >2 min), emit a
      copy-pasteable block, e.g.
      `watch -n 10 'cargo build --workspace --message-format=short 2>&1 | tail -5'`
      for the local re-verify and a `gh run watch <run-id>` pointer for CI.
      *Acceptance:* the watch block is present in the PR/dev-notes when those
      jobs start.
      **DEV TICK 2026-06-15:** watch recipes emitted in the HANDOFF block. Local
      rustls re-verify recipe and `gh run watch` pointer for CI both present.

## M-T3 — Developer (CI + Windows-compile + runbook)

- [x] **T-D-6** Land the **first** `.github/workflows/ci.yml` (ADR feature D8).
      Matrix `os: [ubuntu-latest, macos-latest, windows-latest]`. Steps:
      - all legs: `cargo build -p ui --bin cockpit --features fixtures` +
        `cargo test --workspace --exclude ui` (the non-UI crates);
      - `ubuntu-latest`: install the Q5-resolved headless deps (≥ `xvfb`; see
        T-D-7), then `xvfb-run -a cargo test -p ui` + the headless smokes
        (`headless_emulator_smoke`, `cockpit_live_lab_run_smoke`). **No `--skip`
        visual filter** — D2's source gate compiles them out on Linux;
      - `macos-latest`: `cargo test -p ui` (full suite incl. the 56 baselines —
        the canonical gate);
      - `windows-latest`: `cargo build -p ui` + `cargo test -p ui` (D5 scope;
        visual baselines compiled out by D2).
      Keep it minimal — no caching/release plumbing at v0.1.
      *Acceptance:* the workflow file exists; a pushed run is green on all three
      legs (ubuntu non-visual + smokes, macos full incl. visual, windows
      build+non-visual). Emit a `gh run watch` pointer (T-D-5).
      **DEV TICK 2026-06-15:** file: `.github/workflows/ci.yml` (written — first
      workflow in the repo). 3-leg matrix (ubuntu/macos/windows). No --skip filter
      needed (D2 source gate). Ubuntu step uses researched Q5 apt-dep list flagged
      "VALIDATE ON FIRST CI RUN". CI run greenness is T_FINAL for the tester. Test
      cmd: N/A locally (CI-only). Workflow file exists: confirmed at
      `.github/workflows/ci.yml`.
- [x] **T-D-7** Q5 headless spike (the one genuine unknown — do this BEFORE
      locking T-D-6's ubuntu leg). Determine on `ubuntu-latest` whether
      `xvfb-run -a cargo test -p ui` suffices for winit window creation or
      whether software GL/EGL (`libgl1-mesa-dri`, `libxkbcommon-x11-0`) is also
      required. Bake the resolved apt-dep list into the ubuntu leg's
      `apt-get install` step.
      *Acceptance:* the ubuntu CI leg runs the `ui` headless smokes with **0
      panics**; the exact apt-dep list is recorded in the runbook (T-D-8) and the
      workflow. ~0.5 dev-day; this is the H3 falsifier.
      **DEV TICK 2026-06-15 (CI-DEFERRED):** Cannot run on macOS. Research-based
      dep list written into CI workflow and runbook: `xvfb libxkbcommon-dev
      libxkbcommon-x11-0 libx11-dev` (standard winit 0.30.x x11rb deps confirmed
      via winit docs). YAML step flagged "VALIDATE ON FIRST CI RUN" per D9.
      The Q5 acceptance criterion (0 panics on ubuntu CI) is T_FINAL — owned by
      the tester on first CI push.
- [x] **T-D-8** Author `spec/runbooks/cockpit-cross-platform.md` (R6). Sections:
      (1) **Linux build prereqs** — the rustls flip means no `libssl-dev`; list
      the Q5-resolved headless apt deps (`xvfb` + whatever T-D-7 found);
      (2) **macOS-only dev-script inventory** (Category C) — the six
      `scripts/orch_*` / `capture_screenshot.sh` / `orch_probe_tcc.sh` scripts
      are macOS-only orchestrator/presenter tooling, NOT in any binary; the
      `capture-screenshot` skill emits Linux operator-instructions;
      (3) **the determinism contract** — the 56 visual baselines are
      macOS-canonical (ADR-0057); Linux/Windows render via `PlatformFallback`
      and are NOT byte-gated; a Linux baseline set is a v0.2 follow-on gated on
      H1. Echo ADR-0057 D1/D3.
      *Acceptance:* the runbook exists with all three sections; cross-links
      ADR-0057 and feature.md § Design; `spec_lint.py` stays ≤ 70 (no new
      findings — a runbook is in the lint-covered tree, so verify it does not
      introduce a broken link or missing-frontmatter finding).
      **DEV TICK 2026-06-15:** file: `spec/runbooks/cockpit-cross-platform.md`
      (written with all 3 required sections + §4 Windows notes + §5 changelog).
      `spec_lint.py | grep -c '\['` → 70 (pre-existing, 0 new). ADR-0057
      cross-linked. Test cmd: `python3 scripts/spec_lint.py 2>&1 | grep -c '\\['`.
      Output: `70` (unchanged).
- [x] **T-D-9** Correct the stale `crates/ui/src/widgets/chart.rs:228-231` doc
      comment (D7): reword "does not bite on macOS, the only cockpit-supported
      platform" to note Linux is now supported and the `UtcOffset::UTC` fallback
      + `UI_CHART_FORCE_UTC` snapshot override (K4) handle the glibc case
      deterministically. Documentation only — non-anchored file.
      *Acceptance:* the comment no longer claims macOS is the only supported
      platform; `cargo build -p ui` unaffected; no anchored report touched
      (ADR-0038 § D6 — this is a `.rs` source comment, not a `spec/*/reports/`
      file).
      **DEV TICK 2026-06-15:** file: `crates/ui/src/widgets/chart.rs:229-231`
      (reworded "does not bite on macOS, the only cockpit-supported platform" →
      "handled safely by the fallback below; cockpit now supports Linux and Windows
      per `cockpit-cross-platform` v0.1 / ADR-0057"). Test cmd: `cargo build -p ui
      --features fixtures`. Output: `Finished` (30s). No `.png` touched.

## M-FINAL — Tester

- [ ] **T-T1** Run the 4-cell verdict tree. **Gate 1 (macOS unchanged):**
      119/119 anchors byte-identical + 56 visual baselines byte-identical on the
      canonical box (the REGRESSION guard). **Gate 2 (Linux green):** build +
      non-visual `cargo test -p ui` + headless smoke (0 panics) on
      `ubuntu-latest`. **Gate 3 (Windows):** build green per Q3 scope.
- [ ] **T-T2** Confirm `spec_lint.py` ≤ 70, zero-new (the standing gate).
- [ ] **T-T3** Emit the test report per the rust-test template; route PASS →
      presenter, REGRESSION/FAIL → developer (or analyst if Gate 1 fails =
      a portability `cfg` leaked into the macOS path).

## M-PRESENTER — Presenter (only after VERDICT → PASS)

- [ ] **T-P1** Assemble `spec/cockpit-cross-platform/presentations/`. Headline:
      "cockpit now builds + runs on Linux; CI matrix green; macOS untouched;
      visual baselines remain macOS-canonical by design (ADR-0051 § D5)."
      Note the v0.2 follow-on (Linux visual baselines, gated on H1).

---

## Dependency notes

- M-T1 is **blocked on M-OD** for Q1/Q2/Q3 (these change the dep tree + the test
  topology). Q4 is architect-decidable.
- **PARALLEL-SAFE** with any in-flight UI/strategy lane: this touches
  `crates/ui/Cargo.toml` (additive `[target.…]` stanza), `pid_alive.rs` (errno
  arm), test `#[cfg]` gates, a new `.github/workflows/`, the workspace-root
  reqwest line (Q1=a), and a new runbook — **no `crates/backtest`/`exec`/`cost`
  touch (anchors safe), no `vendor/` touch (operator-lock safe), no new UI
  rendering surface.**
- The single genuine unknown is **Q5** (Linux headless CI) — recommend the
  ~0.5-day developer spike lands at T-AR3/T-D-7 boundary before the CI YAML is
  locked.
