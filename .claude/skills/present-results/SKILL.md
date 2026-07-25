---
name: present-results
description: Assemble a presentation for a feature — TL;DR, what changed, demo run, verification matrix, screenshot references, approval block. Use when the presenter agent needs to produce or refresh a `spec/<slug>/presentations/<slug>-<date>.md` file. Pulls evidence from spec/<slug>/{feature.md,tasks.md}, evidence/<slug>/reports/test-*.md, evidence/anchors.toml, and live binary runs.
---

# present-results

Single end-to-end pipeline for the presenter agent. Produces one
presentation file and zero external side effects (no commits, no PRs).

## Inputs

- `slug` — feature slug, e.g. `operator-success-reports`.
- `mode` — `preview` (mid-feature, design-stage) or `release` (post-tester PASS).
- `report_path` (release mode) — path to the tester's final report.
- `open_questions` (preview mode) — list of decisions the operator should weigh in on.

## Procedure

1. **Read the feature brief.** `spec/<slug>/feature.md`. Extract the
   "Why", the requirements (R-items), and the verification matrix (V-items).

2. **Read the task list.** `spec/<slug>/tasks.md`. Note any unticked
   rows and the changelog.

3. **Read the latest test report** (release mode). Pull pass/fail counts,
   anchor-gate result, perf numbers.
   - Look first under `evidence/<slug>/reports/test-*.md`.
   - If none exist, the feature is pre-Lumen (shipped before the
     2026-05-08 spec restructure) and its tester reports live in
     `spec/archive/pre-lumen-tester-reports-2026-04-to-05-03.tar.gz`.
     Extract the relevant report by name with `tar -xzf
     spec/archive/pre-lumen-tester-reports-2026-04-to-05-03.tar.gz
     -C /tmp test-*-<slug>-final.md` (or one of the wave variants —
     `tar -tzf` to list). Read from `/tmp/`. Cite the archive path
     in the verification matrix evidence column so the operator can
     re-extract on their side. Do NOT copy the archived report into
     the per-feature `reports/` folder — the archive is the canonical
     home for these reports and committing extracts undoes the
     `spec/archive/README.md` audit pattern.

4. **Run a live demo.** Pick the most representative bin command for the
   feature. Examples:
   - operator-success-reports → `cargo run -p reports --bin report -- --period 7d --ledger <fixture>`
   - cockpit / UI → `cargo run --release --bin cockpit --features fixtures` (then capture-screenshot)
   - backtest → `cargo run --release --bin backtest -- --scenario <name>`
   Capture stdout. Truncate to 30 lines max with `...` for longer output.
   Embed verbatim — do NOT paraphrase.

5. **Verify anchors.** Run `bash scripts/verify_anchors.sh`. Embed the
   PASS/FAIL summary line. If FAIL, do NOT continue with `release`
   mode — emit `HANDOFF → developer` and stop.

6. **Find existing screenshots.** Glob `evidence/<slug>/reports/screenshots/`
   and `evidence/<slug>/reports/screenshots/<feature-version>/` for any `.png` files.
   Reference each with a relative-link caption. Three branches:
   - **PNGs exist** → reference each with caption.
   - **No PNGs and feature is UI-related** (`spec/<slug>/feature.md`
     contains a `## UI` heading, or the feature folder has a
     `screenshots/` directory with a README) → use `capture-screenshot`
     skill to emit a manual-capture instruction block.
   - **No PNGs and feature is not UI-related** (no `## UI` heading,
     no `screenshots/` directory — typical for report / audit /
     risk-only features) → write `_n/a — non-UI feature_` with a
     one-line reason (e.g. "report bin emits markdown; cockpit
     `viewer` renders it inline"). Do NOT invoke
     `capture-screenshot`; there is nothing to capture.

7. **Read `evidence/anchors.toml`.** Embed the count + the first 8 chars of
   each anchor SHA — proves byte-stable output without flooding the
   presentation.

8. **Assemble the markdown** per the skeleton below.

9. **Write via `spec-update` skill** to
   `spec/<slug>/presentations/<slug>-<YYYY-MM-DD>.md`. Bump `updated:`. Add
   changelog entry. Save raw stdout under
   `spec/<slug>/presentations/artifacts/<slug>-<date>/<name>.txt` if longer
   than the 30-line embed budget.

10. **Run the pre-tick gate (mandatory):**
    `bash scripts/check_presentation.sh spec/<slug>/presentations/<slug>-<date>.md`.
    Must exit 0 and print `PRESENTATION CHECK PASS`. Quote that line
    verbatim in your closing summary. If it FAILs (any approval-block
    `[x]` detected), reset the box(es) to `[ ]` in the file and re-run
    the gate before emitting any verdict. Self-grep is not a substitute
    for this script — every prior fire of the agent has self-claimed
    "no `[x]`" while shipping pre-ticked.

11. **Emit verdict line.** Either `PRESENTATION → READY (awaiting human
    approval)` (only after step 10's PASS is quoted) or a routed
    handoff if a gate failed.

## Presentation skeleton

```markdown
---
slug: <slug>
mode: <preview|release>
status: draft
audience: human-operator
updated: <YYYY-MM-DD>
generated: <RFC3339>
---

# <Feature Title> — <preview|release>

## TL;DR
<one sentence — operator scans this in 3s>

## What changed
- <bullet 1, plain language>
- <bullet 2>
- <bullet 3>

## Why
<analyst's rationale, distilled to one paragraph; cite spec/<slug>/feature.md>

## What you can do now

| Action | Command |
|--------|---------|
| <thing 1> | `<cmd>` |
| <thing 2> | `<cmd>` |

## Live demo

```
$ <cmd>
<verbatim stdout, ≤30 lines>
```

<one-line interpretation: "Notice X. The Y row is the new feature working.">

## Screenshots
<reference + caption per .png in evidence/<slug>/reports/screenshots/, OR
 a manual-capture instruction block if missing-but-UI, OR
 `_n/a — non-UI feature_` with one-line reason>

## Verification

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1   | <desc>      | VERIFIED | <test:line> |
| ...  | ...         | ...    | ... |

## Numbers that matter
- Tests: <count> passed, <count> failed
- Anchors: <count>/<count> PASS
- Perf: <wall-clock>, <RSS>
- <other quantitative facts>

## Open decisions
<numbered list, one decision each, OR `_no decisions pending — ready to ship_`>

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

### Notes / feedback
<empty until operator fills>

## Changelog
- <date> (presenter): initial draft
```

## Failure modes

- Tester report missing → emit `HANDOFF → tester (rerun before
  presentation)`.
- Anchor gate FAIL → emit `HANDOFF → developer (anchors broken; do
  not present until fixed)`.
- Bin run crashes → embed the stack trace in the demo block, mark
  `release` mode as `BLOCKED`, route HANDOFF accordingly.
- Feature has no `## Verification` section → emit `HANDOFF → analyst
  (verification matrix missing)`.

## Re-running

The skill is idempotent. Re-running for the same slug + date
overwrites the prior file (the date is in the filename, so different
days produce different files). The orchestrator may invoke it again
when the underlying feature changes; the new file supersedes the old.
