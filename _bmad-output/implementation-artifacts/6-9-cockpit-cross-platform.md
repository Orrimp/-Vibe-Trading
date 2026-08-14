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
