---
name: spec-lint
description: Mechanical lint pass over spec/ — detects dead intra-spec links, missing frontmatter keys, orphan feature folders, anchors not referenced by trace.toml, shipped features without tests, and ADR registry mismatches. Run as part of the presenter pre-tick gate and any CI / pre-commit hook. Read-only; never edits spec.
---

# spec-lint

Structural integrity check for `spec/`. Pairs with `verify-anchors` (which
checks backtest-body hashes) to give the project two mechanical gates: one
for content stability (anchors) and one for shape stability (this).

## Procedure

1. Run the linter from repo root via `uv` (recommended — `uv` auto-selects
   a Python ≥ 3.11 per the script's PEP-723 header):

   ```bash
   uv run scripts/spec_lint.py
   ```

   System Python ≥ 3.11 also works (`python3 scripts/spec_lint.py`).
   macOS system Python is 3.9 and will fail with `ModuleNotFoundError:
   tomllib` — use `uv run` or `/opt/homebrew/bin/python3.11+`.

   Exit code 0 = clean. Non-zero = at least one structural violation; the
   exit code equals the number of categories with at least one violation
   (so `1` means one category failed, not one file).

2. To see all violations even after early failures, pass `--all`:

   ```bash
   uv run scripts/spec_lint.py --all
   ```

3. To restrict the check to one feature folder:

   ```bash
   uv run scripts/spec_lint.py spec/chart-canvas-overhaul
   ```

## What it checks

| Category                  | What triggers a violation                                            |
|---------------------------|-----------------------------------------------------------------------|
| `dead-link`               | A markdown link to `spec/**` or `../**` whose target path doesn't exist. |
| `missing-frontmatter`     | A required key is absent on `product.md`, `architecture.md`, `feature.md`, `tasks.md`. |
| `orphan-feature`          | A `spec/<slug>/` folder missing either `feature.md` or `tasks.md`.    |
| `bad-anchor`              | `spec/anchors.toml` entry missing `scenario`, `version`, or `sha256`. |
| `unreferenced-anchor`     | (only when `spec/trace.toml` exists) anchor not cited by any trace row. |
| `shipped-no-tests`        | `feature.md` with `status: shipped` but no test report in `reports/`. |
| `trace-broken-path`       | A `trace.toml` row references a path that doesn't exist on disk.       |
| `adr-not-registered`      | (post-Phase-1A) an ADR file not registered in `architecture.md`'s ADR table. |

## Routing

- **PASS** → continue. Mention `spec-lint: PASS` in the tester report.
- **FAIL** → route to the owner of the most-violated category (analyst for
  product / feature issues; architect for ADR / architecture; developer for
  trace / orphan paths). Tester does not fix; tester reports.

## When to invoke

Mandatory:
- Tester pre-VERDICT gate (every run).
- Presenter pre-tick gate (alongside `check_presentation.sh`).
- Any CI / pre-commit hook the team chooses to add.

Optional:
- After any large `spec-update` to confirm shape integrity.

## What this skill does NOT do

- Does not modify any spec file. Read-only.
- Does not check anchor body SHAs — that's `verify-anchors`.
- Does not enforce prose style or word counts — only structure.
- Does not require `trace.toml` to exist; checks degrade gracefully.
