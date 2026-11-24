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

4. To prove the rules themselves (synthetic fixtures, plus two runs of a copy
   of the script against a throwaway repository layout — a temp directory, not
   a git repo — which pin `main()`'s wiring):

   ```bash
   python3 scripts/spec_lint.py --self-test   # every line PASS, exit 0
   ```

   On 2026-11-24 that is 9 lines: 8 rule self-tests, then `(registry)`, which
   FAILs if a `_self_test_*` function is defined but `self_test()` never runs
   it (bug-log `#216`).

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
| `trace-ref-unparsed` | A story with no `- Trace:` line, more than one, or one in neither accepted form (`` `REQ-…` (state=`…`) ``, text may follow the `)`; or `none — <reason>`); such a story is invisible, or only partly visible, to `status-drift` / `story-done-trace-drift` (bug-log `#216`). The line starts at column 0, anywhere in the file (by convention under `### References`; placement is not checked). The stock create-story template has none, so `_bmad/custom/bmad-create-story.toml` states the contract. |
| `trace-ref-dangling` | A story's `- Trace:` line names a REQ-id with no `[[req]]` row in `trace.toml`; that story is just as invisible to `status-drift` / `story-done-trace-drift` (bug-log `#216`). |
| `trace-broken-path` | A `trace.toml` row references a path or feature slug that doesn't resolve. |
| `dead-cargo-citation` | A `cargo test -p <pkg> --test <target>` cited in a walked `.md` or `CHANGELOG.md` whose package is not in `crates/`, or whose target is neither `tests/<target>.rs` nor a `[[test]]` of that crate (bug-log `#168`). A deliberately quoted dead command is exempt with a `dead-citation-ok` marker on its line or up to two lines above. |
| `adr-not-registered` | Reserved: listed in `CATEGORIES`, but `spec_lint.py` emits it nowhere. ADR registration is enforced by `scripts/adr_registry_check.py` (the atomic pre-commit form). |

## Routing

- **PASS** → continue. Quote `spec-lint: PASS (0 violations)` in the review
  findings.
- **FAIL** → route to the owner of the most-violated category (analyst/pm
  seam for PRD/story-content issues; architect seam for ADR/architecture;
  dev seam for `trace-broken-path` source paths; the orchestrator for
  board/triad bookkeeping).
- **`trace-ref-unparsed` / `trace-ref-dangling`** → the story's author (the
  session or workflow that last wrote its `- Trace:` line), else the
  orchestrator. Fix the story's line, or add the missing `trace.toml` row;
  never loosen the rule. While the violation stands, the triad cannot check
  that story.

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
