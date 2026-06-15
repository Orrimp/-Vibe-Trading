---
slug: cockpit-cross-platform
status: draft
owner: analyst
updated: 2026-06-15
version: 0.1.0
---

# cockpit-cross-platform — v0.1.0

Make the cockpit (`cargo run -p ui --bin cockpit_live --features live` and
`cargo run -p ui --bin cockpit --features fixtures`) **build and run on Linux
and Windows**, not just macOS — without changing macOS behavior (anchors
119/119, the 56 visual baselines byte-unchanged).

> **One-line:** iced + winit + tiny-skia + cosmic-text are already
> cross-platform, so the cockpit is ~90% portable by construction. The real
> blockers are narrow: a missing Windows `windows`-crate dep behind an
> already-written `#[cfg(windows)]` arm, the `libc::__error()` macOS-ism in
> the Unix PID probe, the platform-native TLS toolchain on Linux, and a set
> of macOS-only **dev/orchestration scripts** that are not part of the
> production binary. The one thing v0.1 deliberately does **NOT** attempt is
> byte-identical visual snapshots across OSes — the determinism contract
> (ADR-0043 / ADR-0051 § D5) already scopes byte-identity to the
> Apple-Silicon canonical box.

## Motivation

The cockpit today is a macOS-only artifact by convention, not by deep design.
The operator wants the option to build and run it on Linux (CI, headless
servers, a Linux workstation) and Windows. The codebase comments even claim
macOS is "the only cockpit-supported platform" (`crates/ui/src/widgets/chart.rs`
:228) — this feature scopes what it would take to retire that claim safely.

The strategic value is modest but real: a green Linux build unlocks **CI on
free Linux runners** (today there is **no `.github/workflows/` at all**) and
removes a single-point-of-failure on the operator's one Mac.

## What is actually macOS-coupled (the survey — file:line)

This is the load-bearing section. The scope hinges on it being **small**.

### Category A — REAL compile/run blockers (must fix for a green build)

1. **`windows` crate is undeclared** — `crates/ui/src/lab/pid_alive.rs:63-78`
   already contains a `#[cfg(windows)]` arm that does
   `use windows::Win32::Foundation::CloseHandle;` and
   `use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE};`.
   The `windows` crate is **NOT a direct dependency of `crates/ui`** (confirmed:
   `grep windows crates/ui/Cargo.toml` → empty; it appears in `Cargo.lock` only
   transitively via `tokio`/`mio`/`time`). **Result: the Windows build fails to
   compile.** Fix: add `windows = { version = "...", features = [...] }` under a
   `[target.'cfg(windows)'.dependencies]` stanza in `crates/ui/Cargo.toml`.
   (~5 LoC Cargo + 0 LoC source — the source arm is already written.)

2. **`libc::__error()` is a macOS/BSD symbol** —
   `crates/ui/src/lab/pid_alive.rs:59` reads errno via
   `unsafe { *libc::__error() }`. On **glibc Linux** the errno accessor is
   `*libc::__errno_location()`, not `__error()`. **Result: the Linux build
   fails to link.** Fix: gate the errno read per-OS (~6 LoC: `#[cfg(target_os
   = "macos")]` → `__error()`; `#[cfg(target_os = "linux")]` →
   `__errno_location()`), or use `std::io::Error::last_os_error().raw_os_error()
   == Some(libc::EPERM)` to drop the raw-errno call entirely (the durable fix —
   removes one `unsafe` block and is portable by construction).

3. **Native-TLS toolchain on Linux** — `reqwest = { version = "0.12",
   features = ["json"] }` (workspace root) uses reqwest's **default-TLS =
   native-tls**. The lockfile carries `native-tls`, `openssl-sys`, `schannel`,
   and `security-framework`. On macOS → Security.framework (system, no setup);
   on Windows → schannel (system, no setup); **on Linux → OpenSSL**, which needs
   the `libssl-dev` + `pkg-config` system packages present at build time.
   **This is a build-environment requirement, not a code change** — but it
   WILL surface as a confusing build failure on a bare Linux box. Decision in
   **Q1** (document the apt/dnf prereq vs. switch reqwest to `rustls-tls` for a
   zero-system-dep build). Only `crates/data`, `crates/agent`, `crates/llm` pull
   reqwest; `crates/ui` pulls it transitively under `--features live`.

### Category B — works cross-platform already (verified, no action)

4. **The vendored `vendor/iced_tiny_skia/` fork is OS-agnostic** — confirmed
   `grep -rn 'cfg(target_os|cfg(unix|cfg(windows|macos|cocoa|core-graphics|objc'
   vendor/iced_tiny_skia/src/` → **empty**. It is a pure software (CPU)
   rasterizer with zero OS-specific code; the operator-lock note is upheld. The
   `[patch.crates-io]` wiring (`Cargo.toml:169-170`) is path-based and
   OS-neutral. **No portability `cfg` is required in the fork** — this answers
   the operator's "is it OS-specific? verify" directly: **no, it is not, and no
   fork change is needed.** (If this ever changes, it is a CLAUDE.md
   operator-lock event — flagged, not assumed.)

5. **iced/winit window + event loop are cross-platform** — winit abstracts
   Cocoa/Win32/X11/Wayland. The "macOS forces GUI on main thread" topology in
   `cockpit_live.rs:24-46` is the **correct portable pattern anyway** (winit
   requires the event loop on the main thread on Windows too), so no change.

6. **`#[cfg(unix)]` PID/trainer arms have Windows counterparts** —
   `trainer.rs` cancellation uses `child.start_kill()` (tokio), which is
   `SIGKILL` on Unix and `TerminateProcess` on Windows (documented at
   `trainer.rs:19-20`). The `#[cfg(unix)]` blocks at `trainer.rs:394,430` are
   **test-only helpers** (`cancel_handle_drop_kills_child`, `assert_exited_within`)
   that shell out to `sleep`/`kill` — they will be `#[cfg]`-skipped on Windows,
   so the Windows build is fine; Windows just runs fewer tests there (Q3).
   `persistence.rs:106` already has the `#[cfg(windows)]` `USERPROFILE` home-dir
   fallback. **These are already portable.**

### Category C — macOS-only DEV/ORCHESTRATION tooling (NOT the production binary)

7. **Six `orch_*` + capture scripts are macOS-only**, by design:
   `scripts/orch_cursor_move.swift` (Swift; CoreGraphics cursor warp),
   `scripts/capture_screenshot.sh` + `scripts/orch_hover_screenshot.sh` +
   `scripts/orch_cockpit_on_screen.sh` + `scripts/orch_crop.sh` +
   `scripts/orch_probe_tcc.sh` (all use `screencapture`/`osascript`/`sips`/TCC).
   These are **orchestrator hover-debug + presenter-screenshot tooling**, NOT
   the tester's render gate and NOT linked into any binary. The
   `capture-screenshot` **skill already handles Linux**: its description says
   "Linux + headless paths emit operator instructions instead of failing."
   **v0.1 disposition: leave these macOS-gated; document them as macOS-only in
   a portability runbook.** No rewrite — that would be scope creep with zero
   production value (Q4).

### Category D — the render-determinism question (the riskiest, its own section)

8. **The 56 visual baselines are system-font-dependent and were captured on
   macOS.** Font resolution chain, fully traced:
   - The cockpit sets **no** iced default font (no `.font()` / `default_font`
     / `include_bytes!` anywhere in `crates/ui/src/bin/`). The
     `FONT_SANS`/`FONT_MONO` constants (`theme.rs:575,577`) are **CSS-style
     family strings used only in docs/comments — they are never handed to
     iced.**
   - iced 0.14 compiles in **only `Iced-Icons.ttf`** as an embedded face. The
     embedded **`FiraSans-Regular.ttf` is gated on `#[cfg(feature = "fira-sans")]`**
     (`iced_graphics-0.14.0/src/text.rs:121-127`), and **`fira-sans` is NOT
     enabled** (we use `iced = { default-features = false, features =
     ["tiny-skia","thread-pool","advanced","canvas"] }` — confirmed `grep
     fira-sans` across our manifests → empty).
   - Therefore all body text resolves through **cosmic-text's `PlatformFallback`**
     against the **per-OS system font database**
     (`cosmic-text-0.15.0/src/font/system.rs:400` `db.load_system_fonts()`;
     default families `Open Sans`/`Noto Sans Mono`/`DejaVu Serif` — none of
     which exist on a stock macOS either, so even today macOS is resolving via
     `PlatformFallback` to whatever the OS picks).
   - **Consequence: glyph shaping + rasterization differ across OSes**, so the
     56 PNG baselines in `crates/ui/tests/visual-baselines/` will **NOT** match
     pixel-for-pixel on Linux/Windows. The snapshot gate is the stated
     regression gate — it would go red on a Linux CI runner.
   - **BUT** — and this is the de-risking finding — **ADR-0051 § D5 and ADR-0043
     already declare the determinism scope as the "Apple-Silicon canonical box"
     and state "cross-platform byte-identity is explicitly NOT guaranteed."**
     `verify_anchors.sh` runs on that canonical box. So the architecture has
     **already excluded** cross-platform byte-identity from the contract; the
     snapshot gate was never designed to hold cross-OS. The macOS baselines stay
     the canonical gate on the canonical box; Linux/Windows do **not** re-anchor
     against them. This is **Q2**, the load-bearing decision.

## Requirements

- **R1 — Linux build green.** `cargo build -p ui --bin cockpit_live --features
  live` and `--bin cockpit --features fixtures` compile on Linux (glibc). The
  `__error()` fix (A.2) + the TLS toolchain prereq (A.3 / Q1) are the gating
  items.
- **R2 — Linux run green (smoke).** `cockpit --features fixtures` opens a
  window and the headless `iced_test` smoke (`headless_emulator_smoke.rs`,
  `cockpit_live_lab_run_smoke.rs`) passes on Linux with **0 panics**. (Headless
  CI may need `xvfb` / a software EGL — Q5.)
- **R3 — Windows build green.** Adds the `windows` crate dep (A.1) so the
  already-written `#[cfg(windows)]` `pid_alive` arm compiles.
  `cargo build -p ui` on Windows succeeds. **Windows *run* is best-effort at
  v0.1** (see Q3 scope) — the architect may stage Windows as run-on-CI-only or
  defer interactive Windows to v0.2.
- **R4 — CI matrix.** A new `.github/workflows/` job runs build + the
  non-visual test suite on `{ ubuntu-latest, macos-latest }` (Windows optional
  per Q3). The **visual-baseline tests are excluded from the non-macOS legs**
  (R-NR.1 makes this explicit) — they run macOS-only as today.
- **R5 — macOS unchanged.** Zero behavior change on macOS: 119/119 anchors
  byte-identical, the 56 visual baselines byte-identical, the macOS build/run
  path untouched. All portability changes are **additive `#[cfg]` / Cargo
  `[target.…]` stanzas**, never edits to the macOS code path.
- **R6 — portability runbook.** A `spec/runbooks/cockpit-cross-platform.md`
  documents: the Linux build prereqs (TLS, xvfb), the macOS-only dev-script
  inventory (Category C), and the explicit "visual baselines are macOS-canonical,
  not cross-platform" contract (echoing ADR-0051 § D5).

### Non-regression contract (R-NR — mandatory)

- **R-NR.1** The 56 visual-baseline tests (`visual_snapshots.rs`,
  `render_snapshots.rs`, `panel_snapshots.rs`, `gallery_snapshots.rs`) are
  **gated to run on macOS only** (a `#[cfg(target_os = "macos")]` guard or a CI
  job-filter — Q2 chooses the mechanism). On Linux/Windows they are **skipped,
  not re-baselined.** The macOS run keeps them byte-identical.
- **R-NR.2** The 119 backtest anchors stay byte-identical (this feature touches
  no `crates/backtest` / `crates/exec` / `crates/cost` code; the anchors live on
  the canonical box per ADR-0043 — unaffected by a UI-crate Cargo change).
- **R-NR.3** No new `unsafe` without a `// SAFETY:` comment (the A.2 durable fix
  via `std::io::Error::last_os_error()` actually *removes* one `unsafe`).
- **R-NR.4** `Decimal` is never replaced by `f64` (this feature adds no
  numeric code; called out per CLAUDE.md non-negotiable).
- **R-NR.5** No live-trading surface added (CLAUDE.md / MEMORY: live trading is
  out of scope; this is build/CI plumbing only).
- **R-NR.6** The vendored `vendor/iced_tiny_skia/` fork is **not modified**
  (operator-lock). If portability is found to require a fork `cfg` — it does
  **not**, per B.4 — that is an escalation, not a silent edit.

## Risk register (K)

- **K1 (HIGH) — cross-platform render non-determinism breaks the snapshot
  gate.** Mitigated by R-NR.1 (macOS-only gating) + the ADR-0051 § D5 finding
  that cross-platform byte-identity is already out of contract. **Residual
  risk:** if the operator later *wants* Linux visual regression, that needs a
  **separate** Linux baseline set + a Linux canonical box (a v0.2 follow-on,
  named in Q2's durable branch). Do not conflate with v0.1.
- **K2 (MED) — Linux headless rendering needs a display server.** `iced_test`
  drives the full tiny-skia readback pipeline; in CI it may need `xvfb-run` or a
  software GL/EGL (`libgl1-mesa-dri`, `libxkbcommon`). Spike in Q5. The
  **pure-CPU tiny-skia path is the asset here** — it can render fully headless
  with no GPU, but winit still wants an X/Wayland display for window creation.
- **K3 (MED) — the `windows` crate is heavy + version-churny.** The
  `windows`/`windows-sys` crates are large and revise often. Pin exactly (per
  the iced/`=0.14.0` precedent). Prefer **`windows-sys`** (smaller, C-FFI-style)
  over the full `windows` crate if only `OpenProcess`/`CloseHandle` are needed —
  the architect picks at M-T1.
- **K4 (LOW) — `time::UtcOffset::current_local_offset()` glibc unsoundness**
  (`chart.rs:228` comment). On multi-threaded glibc, the local-offset lookup can
  fail; the code already falls back to `UtcOffset::UTC` deterministically, and
  the snapshot harness forces UTC via `UI_CHART_FORCE_UTC`. **Already handled** —
  flagged so the architect knows the comment ("does not bite on macOS, the only
  cockpit-supported platform") becomes stale-and-should-be-corrected, not a bug.
- **K5 (LOW) — line-ending / path assumptions.** Persistence uses `PathBuf::join`
  (portable) and `USERPROFILE` fallback already exists (`persistence.rs:106`).
  JSON state files are LF-written by serde_json; Git `core.autocrlf` on Windows
  checkouts could perturb committed fixtures — verify `.gitattributes` pins
  `*.json`/`*.png` as binary/LF (Q-follow-on, low).

## Hypothesis register (H)

- **H1** — With `fira-sans` *enabled* and a single embedded default font set
  via `iced::application(...).default_font(...)`, renders become
  **font-deterministic across OSes** (no `PlatformFallback`), which *could* let
  one baseline set serve all platforms. **Falsifier:** enable `fira-sans`, set
  the default font, re-capture on macOS, diff vs. current 56 baselines — if they
  change, this is a *baseline migration* (touches R-NR.1) and is therefore a
  **v0.2 durable option, NOT v0.1** (v0.1 must keep baselines byte-identical).
  Recorded here so the architect sees the path exists but is correctly deferred.
- **H2** — `windows-sys` (not the full `windows` crate) is sufficient for the
  `OpenProcess`/`CloseHandle`/`PROCESS_SYNCHRONIZE` trio. Falsifier: compile the
  `#[cfg(windows)]` arm against `windows-sys` on a Windows runner.
- **H3** — `xvfb-run -a cargo test -p ui` (excluding visual baselines) is
  sufficient to run the Linux test legs headlessly. Falsifier: the CI spike in
  Q5.

## Open questions (operator / architect) — durable-biased per AGENT.md 2026-05-28

- **Q1 (TLS toolchain) — LOAD-BEARING.**
  - **(a) Switch reqwest to `rustls-tls` for a zero-system-dep build
    (Recommended — DURABLE).** `reqwest = { default-features = false, features
    = ["json","rustls-tls"] }`. Makes Linux/Windows/macOS builds identical with
    **no system OpenSSL dependency**, so a bare Linux container or a fresh
    Windows box builds with zero apt/dnf/vcpkg setup. Costs ~1 dev-day (flip +
    re-verify the live HTTP paths in `crates/data`/`agent`/`llm`) and a
    one-time `Cargo.lock` churn. Carries forward across every future CI image
    without amendment.
  - **(b) Keep native-tls; document the `libssl-dev` + `pkg-config` prereq** —
    cheap (~0 code), but spawns a "works-on-my-machine" support cost on every
    new Linux environment and a v0.2 cleanup commitment. **Fallback if budget
    tightens** — the if-budget-tightens lane.
  - *Recommendation rationale:* the rustls flip is the choice whose M-T1 lock
    carries across multiple environments without re-touching; native-tls is the
    "document a carve-out" path → fallback per the durable-over-quick rule.

- **Q2 (visual-baseline gating mechanism) — LOAD-BEARING, the riskiest.**
  - **(a) Gate visual tests `#[cfg(target_os = "macos")]` + document baselines
    as macOS-canonical (Recommended — DURABLE).** Codifies the ADR-0051 § D5
    reality directly in the test source; the gate is self-documenting and
    survives any CI re-org. ~0.5 dev-day. Names the v0.2 follow-on (a Linux
    canonical-box baseline set) without committing to it.
  - **(b) Keep tests un-gated; exclude them via CI job-filter only** — cheaper
    in source (~0 LoC) but the truth ("these are macOS-only") lives only in YAML,
    so a local `cargo test -p ui` on a contributor's Linux box goes red
    confusingly. **Fallback.**
  - *Recommendation rationale:* (a) puts the contract where the failure happens;
    (b) hides it in CI config (a carve-out) → fallback.

- **Q3 (Windows scope) — what does "Windows works" mean at v0.1?**
  - **(a) Windows build + non-visual tests green on CI; interactive run is
    best-effort/unverified (Recommended for v0.1).** Honest and bounded: the
    `windows`-crate dep fix (A.1) makes it *compile and test*; verifying the
    interactive window UX on Windows is a separate manual-QA effort the operator
    has no Windows box to do today. This is the durable-honest scope — it does
    not over-claim.
  - **(b) Full Windows interactive parity** — requires a Windows test machine +
    manual UX QA; **defer to v0.2** unless the operator has Windows hardware.
  - *Note:* here the **cheaper option (a) IS the durable-honest one** — claiming
    full Windows parity we cannot verify would be the un-durable choice. This is
    the AGENT.md exception (the architect can prove (a) spawns no rework because
    v0.2 Windows-interactive is a clean additive verification, no MIGRATION).

- **Q4 (macOS-only dev scripts) — rewrite or gate?**
  - **(a) Leave Category-C scripts macOS-only; inventory them in the runbook
    (Recommended — DURABLE-honest).** They are orchestrator/presenter tooling
    with zero production value on Linux; the `capture-screenshot` *skill* already
    emits Linux operator-instructions. ~0 code.
  - **(b) Port the screenshot scripts to Linux (`import`/`grim`/`scrot`)** —
    real effort (~1-2 dev-days) for tooling the Linux CI does not need (CI uses
    the in-Rust snapshot harness, not `screencapture`). Genuine scope creep;
    do **not** do this at v0.1.

- **Q5 (Linux headless CI) — spike.** Does `xvfb-run -a cargo test -p ui
  --features live -- --skip visual` pass on `ubuntu-latest`, or is a software
  GL/EGL stack (`libgl1-mesa-dri`, `libxkbcommon-x11-0`) also required for winit
  window creation? **Recommend a ~0.5-day developer spike** before locking the
  CI YAML (H3 falsifier). This is the one genuine unknown; everything else is
  determined by reading.

## Verdict tree (4-cell, for the tester at M-FINAL)

| Linux build+smoke | macOS unchanged (anchors 119 + 56 baselines byte-id) | Verdict |
|---|---|---|
| green | yes | **PASS** — ship v0.1 (Linux green, CI matrix, baselines macOS-canonical) |
| green | NO | **REGRESSION** — a portability `cfg` leaked into the macOS path; do not ship |
| RED | yes | **FAIL** — Linux blocker remains (TLS toolchain or a missed `cfg`); back to developer |
| RED | NO | **FAIL + REGRESSION** — re-architect; the change was not additive |

## Effort estimate

- **Recommended v0.1 (Linux green + CI matrix + Windows-compiles + runbook):
  ~3-4 dev-days.** Breakdown: A.1 windows dep (~0.25d) + A.2 errno fix (~0.5d)
  + Q1 rustls flip (~1d incl. re-verify live HTTP) + R-NR.1 baseline gating
  (~0.5d) + CI YAML + Q5 xvfb spike (~1d) + runbook (~0.5d). Plus ~0.5d architect
  M-T1.
- **If-budget-tightens (~1.5-2 dev-days):** take Q1=(b) (document native-tls
  prereq, skip the rustls flip) + Q3=(a) (Windows compile-only) + drop the CI
  matrix to a single `ubuntu-latest` build-check job. Ships "Linux builds +
  smokes locally" without the full CI matrix; carries a v0.2 cleanup commitment
  for the TLS prereq + CI hardening. Named so the operator has the cheaper lane.

## Out of scope (v0.1)

- Linux/Windows **visual regression** (a Linux canonical-box baseline set) —
  v0.2 follow-on, gated on H1 (`fira-sans` + pinned default font making renders
  font-deterministic).
- Porting the Category-C macOS dev scripts to Linux (Q4=(b)).
- Full Windows interactive UX QA (Q3=(b)) — needs Windows hardware the operator
  does not have today.
- Any modification to `vendor/iced_tiny_skia/` (operator-lock; B.4 proves none
  is needed).
- Live trading (out of scope project-wide).

## Sources

- `crates/ui/src/lab/pid_alive.rs:59,63-78` — `__error()` macOS-ism + undeclared
  `windows` crate.
- `iced_graphics-0.14.0/src/text.rs:121-127` — embedded fonts; `fira-sans`
  `#[cfg]` gate.
- `cosmic-text-0.15.0/src/font/system.rs:400` — `db.load_system_fonts()` +
  `PlatformFallback`.
- `spec/architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md`
  § D5 — "Apple-Silicon canonical box; cross-platform byte-identity explicitly
  NOT guaranteed."
- `spec/architecture/adr/0043-simulated-latency-and-slippage.md` — f64
  conversion-boundary determinism scope (inherited verbatim by ADR-0051 § D5).
- reqwest 0.12 docs — default-tls = native-tls; `rustls-tls` opt-in feature.

## Changelog

- 2026-06-15 (analyst): authored v0.1.0 draft. Surveyed macOS coupling
  (file:line) into 4 categories: A real blockers (undeclared `windows` crate;
  `__error()` glibc gap; native-tls Linux toolchain), B already-portable (the
  vendored fork is OS-agnostic — verified empty `cfg`; winit/iced cross-platform;
  trainer/persistence Windows arms exist), C macOS-only dev scripts (leave
  gated), D render-determinism (the 56 baselines are system-font-dependent;
  ADR-0051 § D5 already scopes byte-identity to the canonical box). Recommended
  bounded v0.1 = Linux build+smoke green + CI matrix + Windows-compiles +
  macOS-canonical baselines, ~3-4 dev-days. Riskiest = Q2 (visual-baseline
  gating). Opened trace row `REQ-COCKPIT-CROSS-PLATFORM-001` (proposed).
  HANDOFF → architect.
