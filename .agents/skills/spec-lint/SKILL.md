---
name: spec-lint
description: Mechanical structural lint over the BMAD-native tree (docs/ + evidence/ + _bmad-output/) — dead intra-tree links, story Status hygiene, orphan stories vs sprint-status.yaml, anchors/trace cross-checks, and the re-founded ADR-0082 triad (story Status ↔ trace.toml state ↔ CHANGELOG index; verify the rules themselves with `python3 scripts/spec_lint.py --self-test`). Run before every review verdict and any CI / pre-commit hook. Read-only; never edits anything.
---

# spec-lint

Structural integrity check, re-founded at BMAD-migration Phase 5b onto the
story/sprint-status layout (`spec/` is retired). Pairs with `verify-anchors`
(which checks backtest-body hashes) to give the project two mechanical
gates: one for content stability (anchors) and one for shape stability
(this).

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

3. To restrict dead-link/frontmatter checks to one path:

   ```bash
   uv run scripts/spec_lint.py _bmad-output/implementation-artifacts
   ```

4. To prove the triad rules themselves (synthetic fixtures):

   ```bash
   python3 scripts/spec_lint.py --self-test   # 3/3 PASS expected
   ```

## What it checks

| Category | What triggers a violation |
|---|---|
| `dead-link` | A markdown link under `docs/` / `evidence/` / `_bmad-output/` whose target doesn't exist (frozen archives + anchored v1 report bodies are skip-listed; `KNOWN_FROZEN_DEAD_LINKS` allowlists the byte-immutable exceptions). |
| `missing-frontmatter` | Soft `updated:` check on `PRD.md`/`architecture.md`; a story with no `Status:` line or an out-of-vocabulary status. |
| `orphan-story` | A non-`backlog` `sprint-status.yaml` entry with no story file, or a story file with no board entry (the board is exhaustive). |
| `bad-anchor` | `evidence/anchors.toml` entry missing `scenario`, `version`, or `sha256`. |
| `unreferenced-anchor` | An anchor not cited by any `trace.toml` row. |
| `story-done-no-tests` | (Narrow, faithful re-founding of `shipped-no-tests` — empirically a no-op on the real tree.) |
| `status-drift` | Story `Status:` disagrees with the value its trace row's `state=` maps to (full vocabulary mapping). |
| `story-done-trace-drift` | The ADR-0082 terminal invariant: `Status: done` but the trace row's `state=` is not `shipped`/`shipped-partial`. |
| `story-done-changelog-missing` | A `Status: done` story not indexed in root `CHANGELOG.md` (by slug / REQ-id / rollup allowlist). |
| `trace-broken-path` | A `trace.toml` row references a path or feature slug that doesn't resolve. |
| `adr-not-registered` | An ADR file without its Registry row (see also `scripts/adr_registry_check.py` for the atomic pre-commit form). |

## Routing

- **PASS** → continue. Quote `spec-lint: PASS (0 violations)` in the review
  findings.
- **FAIL** → route to the owner of the most-violated category (analyst/pm
  seam for PRD/story-content issues; architect seam for ADR/architecture;
  dev seam for `trace-broken-path` source paths; the orchestrator for
  board/triad bookkeeping).

## When to invoke

Mandatory:
- Before every `bmad-code-review` verdict (every run).
- Deck pre-tick gate (alongside `check_presentation.sh`).
- Any CI / pre-commit hook the team chooses to add.

Optional:
- After any large planning-artifact edit to confirm shape integrity.

## What this skill does NOT do

- Does not modify any file. Read-only.
- Does not check anchor body SHAs — that's `verify-anchors`.
- Does not enforce prose style or word counts — only structure.
