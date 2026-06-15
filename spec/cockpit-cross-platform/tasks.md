---
slug: cockpit-cross-platform
status: draft
owner: architect
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

## M-OD — Operator decisions (BLOCKS M-T1 on Q1/Q2/Q3)

- [ ] **Q1** TLS toolchain — (a) rustls flip [Recommended/DURABLE] vs
      (b) document native-tls prereq [fallback].
- [ ] **Q2** Visual-baseline gating mechanism — (a) `#[cfg(macos)]` in source
      [Recommended/DURABLE] vs (b) CI-filter-only [fallback]. **Riskiest.**
- [ ] **Q3** Windows scope — (a) build+test-on-CI, run best-effort
      [Recommended for v0.1] vs (b) full interactive parity [defer v0.2].
- [ ] **Q4** macOS-only dev scripts — (a) leave gated + inventory [Recommended]
      vs (b) port to Linux [scope creep, decline].
      *(Architect-decidable; surfaced for operator awareness.)*

## M-T1 — Architect (design pass) — BLOCKED on M-OD Q1/Q2/Q3

- [ ] **T-AR1** Resolve the `windows` vs `windows-sys` choice (H2) — pick the
      minimal crate for `OpenProcess`/`CloseHandle`/`PROCESS_SYNCHRONIZE` and
      pin exactly (K3). Specify the `[target.'cfg(windows)'.dependencies]`
      stanza for `crates/ui/Cargo.toml`.
- [ ] **T-AR2** Specify the A.2 errno fix shape — confirm the durable
      `std::io::Error::last_os_error().raw_os_error() == Some(EPERM)` rewrite
      (drops one `unsafe`) vs. the per-OS `__error()`/`__errno_location()` cfg.
      Architect chooses; R-NR.3 favors the unsafe-removing path.
- [ ] **T-AR3** Decide the **CI matrix shape** (R4) — `.github/workflows/`
      job(s), OS legs `{ubuntu-latest, macos-latest, [windows-latest per Q3]}`,
      and the **test-filter that excludes the 56 visual baselines on non-macOS
      legs** (consistent with the Q2 source-gate). First CI in the repo — keep
      it minimal (build + `cargo test -p ui -- --skip <visual>` + workspace
      `cargo test` non-UI).
- [ ] **T-AR4** Whether an **ADR is warranted.** Likely **yes, a short one**
      (next free = **0056**) codifying "the visual-baseline determinism scope is
      the macOS canonical box; Linux/Windows render via `PlatformFallback` and
      are NOT byte-gated" — this elevates the ADR-0051 § D5 finding from
      MC-anchors to the UI snapshot gate. Architect confirms number + writes it
      (analyst does not). If the architect judges an ADR-0051 § D5 Changelog
      amendment sufficient (no new ADR), record that decision in feature.md.
- [ ] **T-AR5** Flag the stale `chart.rs:228` comment ("does not bite on macOS,
      the only cockpit-supported platform") for correction as part of the
      Linux-build work (K4) — it is documentation, not a code bug; correcting it
      is in-scope and does NOT touch an anchored file.
- [ ] **T-AR6** Decompose M-T2/M-T3 into ordered `T-D-N` developer tasks; lock
      the M-T1 contract so the developer cannot drift R-NR.1/R-NR.6.

## M-T2 — Developer (Linux green) — the core of the work

- [ ] **T-D-1** A.2 errno fix (`pid_alive.rs`) — Linux links.
- [ ] **T-D-2** A.1 `windows`/`windows-sys` dep stanza — Windows compiles (the
      `#[cfg(windows)]` arm is already written).
- [ ] **T-D-3** Q1 resolution — if (a): flip reqwest to `rustls-tls` across the
      workspace root + re-verify `crates/data`/`agent`/`llm` live HTTP paths
      still build+test; if (b): no code, runbook-only.
- [ ] **T-D-4** R-NR.1 — gate the 56 visual-baseline tests per Q2 (source
      `#[cfg(target_os="macos")]` or CI-filter). **Assert macOS still runs all
      56 byte-identical** (this is the R5/R-NR.1 guard — run on the canonical box
      before/after).
- [ ] **T-D-5** Watch-recipe note: the rustls re-verify + first CI run are
      >2 min jobs — emit the `watch -n 10 '<probe>'` block per MEMORY contract
      when kicking them off.

## M-T3 — Developer (CI + Windows-compile + runbook)

- [ ] **T-D-6** Land the `.github/workflows/` matrix (T-AR3 shape).
- [ ] **T-D-7** Q5 spike → bake the resolved headless recipe (`xvfb-run -a …`
      + any `libgl1-mesa-dri`/`libxkbcommon` apt deps) into the Linux CI leg.
- [ ] **T-D-8** Author `spec/runbooks/cockpit-cross-platform.md` (R6): Linux
      build prereqs, macOS-only dev-script inventory (Category C), the
      "baselines are macOS-canonical, not cross-platform" contract.

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
