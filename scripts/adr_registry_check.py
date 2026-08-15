#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""adr_registry_check.py — ADR registry atomicity lint.

Enforces the architect.md § ADR registry atomic-write contract on every
commit touching _bmad-output/planning-artifacts/architecture/decisions/.  Runs in pre-commit hook mode by
default (reads the staged index via `git diff --cached`).

Exit codes:
  0  Clean — all invariants hold.  Silent (R2.3).
  1  One or more drift(s) detected.  Markdown table on stderr.
  2  Script failure (git unavailable, README unparseable, etc.).

Usage:
  python3 scripts/adr_registry_check.py               # default --pre-commit
  python3 scripts/adr_registry_check.py --pre-commit  # explicit
  python3 scripts/adr_registry_check.py --self-test   # in-process self-test
  python3 scripts/adr_registry_check.py --ci          # reserved; not implemented
"""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
# BMAD migration Phase 4 (2026-07-25): the ADR corpus `git mv`d from
# spec/architecture/adr/ to this path, atomically with this repoint
# (AD-18 one-atomic-commit rule: "the lint defines the home").
ADR_DIR = REPO_ROOT / "_bmad-output" / "planning-artifacts" / "architecture" / "decisions"
README_PATH = ADR_DIR / "README.md"
# Repo-relative form, used in invariant (d) drift rows where there is no real
# file to report a path for (the whole point of (d) is that it is missing).
ADR_DIR_NAME = "_bmad-output/planning-artifacts/architecture/decisions"

# Canonical status enum per architect.md § ADR registry + README.md § Format.
# Module-level named constant so v0.2.0 can extend with 'withdrawn' in one place.
STATUS_ENUM: frozenset[str] = frozenset({"accepted", "proposed", "superseded", "deprecated"})

# Regex to extract ADR number from a README ## Registry table data row.
# Matches lines like "| 0001  | ..." — first cell must be exactly 4 digits.
_REGISTRY_ROW_RE = re.compile(r"^\|\s*(\d{4})\s*\|")

# Regex for frontmatter block (matches hash_report.py's canonical form).
_FRONTMATTER_RE = re.compile(r"^---\n.*?\n---\n", re.DOTALL)

# Regex to extract status from frontmatter body.
_STATUS_RE = re.compile(r"^status:\s*(\S+)", re.MULTILINE)

# Marker for ## Registry section header.
_REGISTRY_HEADING_RE = re.compile(r"^##\s+Registry\b", re.MULTILINE)

# Next top-level heading after ## Registry (to bound the scan).
_NEXT_HEADING_RE = re.compile(r"^##\s+\S", re.MULTILINE)


# ---------------------------------------------------------------------------
# Git seams — factored so self-test can inject fakes without a real index.
# ---------------------------------------------------------------------------

def _staged_adr_files() -> list[str]:
    """Return list of staged ADR paths (repo-relative) via git diff --cached.

    Uses the exact load-bearing command from D-ADR-2:
      git diff --cached --name-only --diff-filter=ACMR -- '_bmad-output/planning-artifacts/architecture/decisions/*.md'

    No shell=True; literal glob reaches git without shell expansion.
    Raises RuntimeError if git is unavailable or the cwd is not a repo.
    """
    try:
        result = subprocess.run(
            [
                "git", "diff", "--cached", "--name-only",
                "--diff-filter=ACMR",
                "--", "_bmad-output/planning-artifacts/architecture/decisions/*.md",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError as exc:
        raise RuntimeError("git not found on PATH") from exc

    if result.returncode not in (0, 1):
        raise RuntimeError(
            f"git diff --cached failed (rc={result.returncode}): {result.stderr.strip()}"
        )

    lines = [ln.strip() for ln in result.stdout.splitlines() if ln.strip()]
    # Filter to actual numbered ADR files (exclude README.md and TEMPLATE.md
    # by construction — they don't match the [0-9][0-9][0-9][0-9]-*.md glob,
    # but git may return them if they happen to be staged too; belt-and-suspenders).
    return [
        p for p in lines
        if re.match(r"_bmad-output/planning-artifacts/architecture/decisions/\d{4}-.*\.md$", p)
    ]


def _readme_staged() -> bool:
    """Return True if _bmad-output/planning-artifacts/architecture/decisions/README.md is staged."""
    try:
        result = subprocess.run(
            [
                "git", "diff", "--cached", "--name-only",
                "--", "_bmad-output/planning-artifacts/architecture/decisions/README.md",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError as exc:
        raise RuntimeError("git not found on PATH") from exc

    return bool(result.stdout.strip())


# ---------------------------------------------------------------------------
# README ## Registry parser (D-ADR-3).
# ---------------------------------------------------------------------------

def _parse_registered_ids(readme_text: str) -> set[str]:
    """Extract the set of registered 4-digit ADR IDs from the ## Registry table.

    Scans lines in the ## Registry section only (SHOULD requirement per D-ADR-3 §4).
    The 4-digit-first-cell regex naturally rejects the header / separator / prose rows.
    """
    # Find the ## Registry heading.
    m = _REGISTRY_HEADING_RE.search(readme_text)
    if m is None:
        return set()

    section_start = m.end()

    # Find the next ## heading after ## Registry to bound the scan.
    m_next = _NEXT_HEADING_RE.search(readme_text, section_start)
    section_text = readme_text[section_start: m_next.start() if m_next else len(readme_text)]

    ids: set[str] = set()
    for line in section_text.splitlines():
        row_m = _REGISTRY_ROW_RE.match(line)
        if row_m:
            ids.add(row_m.group(1))
    return ids


# ---------------------------------------------------------------------------
# ADR file discovery + frontmatter parse (D-ADR-4).
# ---------------------------------------------------------------------------

def _discover_adr_files() -> list[Path]:
    """Glob _bmad-output/planning-artifacts/architecture/decisions/[0-9][0-9][0-9][0-9]-*.md.

    The pattern structurally excludes README.md and TEMPLATE.md (R1.3).
    """
    files = sorted(ADR_DIR.glob("[0-9][0-9][0-9][0-9]-*.md"))
    # Belt-and-suspenders guard (SHOULD per D-ADR-4).
    return [f for f in files if f.name not in {"README.md", "TEMPLATE.md"}]


def _adr_number(path: Path) -> str:
    """Extract the zero-padded 4-digit ADR number from the filename."""
    m = re.match(r"^(\d{4})-", path.name)
    if m is None:
        raise ValueError(f"Unexpected ADR filename shape: {path.name}")
    return m.group(1)


def _parse_status(text: str) -> str | None:
    """Extract the status: value from an ADR frontmatter block.

    Returns None if no frontmatter or no status: line is found.
    (Absence is reported as invariant-(c) drift, not a crash.)
    """
    fm_m = _FRONTMATTER_RE.match(text)
    if fm_m is None:
        return None
    fm_body = fm_m.group(0)
    st_m = _STATUS_RE.search(fm_body)
    return st_m.group(1) if st_m else None


# ---------------------------------------------------------------------------
# Drift representation + emit (D-ADR-5).
# ---------------------------------------------------------------------------

class DriftRow:
    """One row in the markdown drift table."""

    def __init__(
        self,
        invariant: str,   # "(a) registry-row-missing" etc.
        file: str,        # repo-relative path
        observed: str,
        expected: str,
    ) -> None:
        self.invariant = invariant
        self.file = file
        self.observed = observed
        self.expected = expected

    def sort_key(self) -> tuple[str, str]:
        # Sort by invariant letter (a, b, c) then by file path.
        letter = self.invariant[1] if len(self.invariant) > 1 else self.invariant
        return (letter, self.file)


def _emit_drift_table(rows: list[DriftRow]) -> None:
    """Write the markdown drift table to stderr (D-ADR-5 / Q-HYG-EMIT)."""
    rows_sorted = sorted(rows, key=lambda r: r.sort_key())
    print(f"adr-registry-check: {len(rows_sorted)} drift(s) detected", file=sys.stderr)
    print("| invariant | file | observed | expected |", file=sys.stderr)
    print("|-----------|------|----------|----------|", file=sys.stderr)
    for r in rows_sorted:
        print(f"| {r.invariant} | {r.file} | {r.observed} | {r.expected} |", file=sys.stderr)


# ---------------------------------------------------------------------------
# Core invariant checks.
# ---------------------------------------------------------------------------

def _check_invariants(
    *,
    adr_files: list[Path],
    registered_ids: set[str],
    staged_adr_paths: list[str] | None,   # None = git unavailable (b skipped)
    readme_is_staged: bool,
) -> list[DriftRow]:
    """Run invariants (a), (b), (c), (d).  Returns a list of drift rows (empty = clean).

    Thin delegation to `_check_invariants_raw` with the real REPO_ROOT.

    This function used to carry its own full copy of every invariant, with
    `_check_invariants_raw` holding a second near-identical copy for the
    self-test.  That meant `--self-test` exercised the COPY and never the
    function `_run_pre_commit` actually calls: an invariant added or changed
    here alone would have stayed green.  That is precisely the defect shape
    bug-log #77 names (a gate that does not guard the code it claims to), so
    the two were collapsed into one implementation on 2026-08-15 when
    invariant (d) landed.  Keep it delegating — do not re-inline the body.
    """
    return _check_invariants_raw(
        adr_files=adr_files,
        registered_ids=registered_ids,
        staged_adr_paths=staged_adr_paths,
        readme_is_staged=readme_is_staged,
        repo_root=REPO_ROOT,
    )


# ---------------------------------------------------------------------------
# Main logic.
# ---------------------------------------------------------------------------

def _run_pre_commit() -> int:
    """Run all invariants in pre-commit (staged-diff) mode.  Returns exit code."""
    # Read README.
    if not README_PATH.exists():
        print(
            f"adr-registry-check: error: {README_PATH} not found",
            file=sys.stderr,
        )
        return 2

    try:
        readme_text = README_PATH.read_text(encoding="utf-8")
    except OSError as exc:
        print(f"adr-registry-check: error: cannot read README: {exc}", file=sys.stderr)
        return 2

    registered_ids = _parse_registered_ids(readme_text)

    # Discover ADR files.
    adr_files = _discover_adr_files()

    # Staged-diff (invariant b).
    try:
        staged_adr_paths = _staged_adr_files()
        readme_is_staged = _readme_staged()
    except RuntimeError as exc:
        print(
            f"adr-registry-check: error: git unavailable; (b) same-commit check cannot run: {exc}",
            file=sys.stderr,
        )
        return 2

    drifts = _check_invariants(
        adr_files=adr_files,
        registered_ids=registered_ids,
        staged_adr_paths=staged_adr_paths,
        readme_is_staged=readme_is_staged,
    )

    if not drifts:
        return 0

    _emit_drift_table(drifts)
    return 1


# ---------------------------------------------------------------------------
# Self-test (inline --self-test flag, D-ADR-6 / R4).
# ---------------------------------------------------------------------------

class _SelfTest(unittest.TestCase):
    """In-process self-test suite. Uses tmpdir fixtures — never mutates the real repo tree."""

    def _make_readme(self, ids: list[str]) -> str:
        """Build a minimal README.md text with a ## Registry table for the given IDs."""
        rows = "\n".join(
            f"| {id_} | test title | accepted | 2026-01-01 |" for id_ in ids
        )
        return (
            "---\n"
            "slug: test\n"
            "status: in-progress\n"
            "updated: 2026-01-01\n"
            "---\n\n"
            "## Registry\n\n"
            "| ID    | Title | Status   | Date       |\n"
            "|-------|-------|----------|------------|\n"
            f"{rows}\n\n"
            "## Changelog\n"
        )

    def _make_adr(self, number: str, status: str = "accepted") -> str:
        return (
            f"---\n"
            f"adr: {number}\n"
            f"title: test\n"
            f"status: {status}\n"
            f"date: 2026-01-01\n"
            f"supersedes: none\n"
            f"superseded-by: none\n"
            f"---\n\n"
            f"# ADR-{number}: test\n"
        )

    def _check(
        self,
        adr_texts: dict[str, str],        # filename → content
        registered_ids: list[str],
        staged_adr_paths: list[str] | None,
        readme_is_staged: bool,
    ) -> list[DriftRow]:
        """Run invariant checks against synthetic in-memory fixtures."""
        with tempfile.TemporaryDirectory() as tmpdir:
            adr_dir = Path(tmpdir) / "adr"
            adr_dir.mkdir()
            adr_files: list[Path] = []
            for name, content in adr_texts.items():
                p = adr_dir / name
                p.write_text(content, encoding="utf-8")
                # Only include files matching [0-9][0-9][0-9][0-9]-*.md
                if re.match(r"^\d{4}-.*\.md$", name):
                    if name not in {"README.md", "TEMPLATE.md"}:
                        adr_files.append(p)

            registered = set(registered_ids)
            # For staged_adr_paths we pass as-is (the seam).
            # For file paths in drifts the function uses relative_to(REPO_ROOT) but
            # our test files are in tmpdir — patch the file paths to be relative to tmpdir.
            # Instead, run _check_invariants with adr_files pointing to the tmpdir files
            # and capture the relative paths computed from the tmpdir root.
            # We need a small wrapper that uses tmpdir as the root for relative paths.
            return _check_invariants_raw(
                adr_files=adr_files,
                registered_ids=registered,
                staged_adr_paths=staged_adr_paths,
                readme_is_staged=readme_is_staged,
                repo_root=Path(tmpdir),
            )

    # --- Case 1: (a) missing row ---
    def test_case1_missing_row(self) -> None:
        # NOTE (2026-08-15): this fixture used to register id "0001" while
        # providing only the file "0099-foo.md" — a registry row with no
        # decision file, i.e. the very drift invariant (d) now detects.  It was
        # invisible while only direction (a) was enforced, and the new check
        # flagged it the moment it landed.  Registering nothing isolates (a);
        # the mixed case is covered explicitly by case 7.
        drifts = self._check(
            adr_texts={"0099-foo.md": self._make_adr("0099")},
            registered_ids=[],  # 0099 has no row → (a) only
            staged_adr_paths=None,
            readme_is_staged=False,
        )
        self.assertEqual(len(drifts), 1)
        self.assertEqual(drifts[0].invariant, "(a) registry-row-missing")
        self.assertIn("0099", drifts[0].file)

    # --- Case 2: (b) updated-not-bumped ---
    def test_case2_updated_not_bumped(self) -> None:
        drifts = self._check(
            adr_texts={"0001-foo.md": self._make_adr("0001")},
            registered_ids=["0001"],
            staged_adr_paths=["_bmad-output/planning-artifacts/architecture/decisions/0001-foo.md"],
            readme_is_staged=False,
        )
        self.assertEqual(len(drifts), 1)
        self.assertEqual(drifts[0].invariant, "(b) updated-not-bumped")

    # --- Case 3: (c) status-out-of-enum ---
    def test_case3_status_out_of_enum(self) -> None:
        drifts = self._check(
            adr_texts={"0001-foo.md": self._make_adr("0001", status="in-progress")},
            registered_ids=["0001"],
            staged_adr_paths=None,
            readme_is_staged=False,
        )
        self.assertEqual(len(drifts), 1)
        self.assertEqual(drifts[0].invariant, "(c) status-out-of-enum")
        self.assertIn("in-progress", drifts[0].observed)

    # --- Case 4: exclude-rule — TEMPLATE.md + README.md never trigger (a)/(c) ---
    def test_case4_exclude_rule(self) -> None:
        # TEMPLATE.md and README.md don't start with 4 digits; glob excludes them.
        # Confirm they are NOT in adr_files (the _check helper only adds 4-digit files).
        drifts = self._check(
            adr_texts={
                "TEMPLATE.md": self._make_adr("NNNN", status="proposed"),
                "README.md": "---\nstatus: in-progress\n---\n\n## Registry\n",
                "0001-real.md": self._make_adr("0001"),
            },
            registered_ids=["0001"],
            staged_adr_paths=None,
            readme_is_staged=False,
        )
        # 0001 has a row and accepted status → clean.
        self.assertEqual(len(drifts), 0)

    # --- Case 5: clean run → zero drifts ---
    def test_case5_clean(self) -> None:
        drifts = self._check(
            adr_texts={"0001-foo.md": self._make_adr("0001")},
            registered_ids=["0001"],
            staged_adr_paths=["_bmad-output/planning-artifacts/architecture/decisions/0001-foo.md"],
            readme_is_staged=True,
        )
        self.assertEqual(len(drifts), 0)

    # --- Case 6: (d) decision-file-missing — the converse of (a), bug-log #86 ---
    def test_case6_decision_file_missing(self) -> None:
        # Registry announces 0001 AND 0079; only 0001 exists on disk.
        drifts = self._check(
            adr_texts={"0001-foo.md": self._make_adr("0001")},
            registered_ids=["0001", "0079"],
            staged_adr_paths=None,
            readme_is_staged=False,
        )
        self.assertEqual(len(drifts), 1)
        self.assertEqual(drifts[0].invariant, "(d) decision-file-missing")
        self.assertIn("0079", drifts[0].file)

    # --- Case 7: (a) and (d) are INDEPENDENT directions, not one check ---
    # Guards against a future "simplification" that collapses them: a file with
    # no row and a row with no file must produce one drift EACH.
    def test_case7_both_directions_are_independent(self) -> None:
        drifts = self._check(
            adr_texts={"0002-orphan-file.md": self._make_adr("0002")},  # file, no row
            registered_ids=["0003"],                                    # row, no file
            staged_adr_paths=None,
            readme_is_staged=False,
        )
        kinds = sorted(d.invariant for d in drifts)
        self.assertEqual(
            kinds,
            ["(a) registry-row-missing", "(d) decision-file-missing"],
            "each direction must be reported separately",
        )


def _check_invariants_raw(
    *,
    adr_files: list[Path],
    registered_ids: set[str],
    staged_adr_paths: list[str] | None,
    readme_is_staged: bool,
    repo_root: Path,
) -> list[DriftRow]:
    """Like _check_invariants but accepts an arbitrary repo_root for relative paths.

    Used by the self-test to avoid depending on REPO_ROOT global.
    """
    drifts: list[DriftRow] = []

    # Invariant (a).
    for adr_path in adr_files:
        num = _adr_number(adr_path)
        try:
            rel = str(adr_path.relative_to(repo_root))
        except ValueError:
            rel = adr_path.name
        if num not in registered_ids:
            drifts.append(DriftRow(
                invariant="(a) registry-row-missing",
                file=rel,
                observed="no row in README.md ## Registry table",
                expected=f"add row to README.md ## Registry table for ADR-{num}",
            ))

    # Invariant (b).
    if staged_adr_paths is not None and staged_adr_paths:
        if not readme_is_staged:
            example_file = staged_adr_paths[0]
            drifts.append(DriftRow(
                invariant="(b) updated-not-bumped",
                file=example_file,
                observed="README.md not staged in this commit",
                expected="stage _bmad-output/planning-artifacts/architecture/decisions/README.md with bumped frontmatter updated:",
            ))

    # Invariant (c).
    for adr_path in adr_files:
        try:
            rel = str(adr_path.relative_to(repo_root))
        except ValueError:
            rel = adr_path.name
        try:
            text = adr_path.read_text(encoding="utf-8")
        except OSError as exc:
            drifts.append(DriftRow(
                invariant="(c) status-out-of-enum",
                file=rel,
                observed=f"cannot read file: {exc}",
                expected=f"set status: one of {{{', '.join(sorted(STATUS_ENUM))}}}",
            ))
            continue

        status = _parse_status(text)
        if status is None:
            drifts.append(DriftRow(
                invariant="(c) status-out-of-enum",
                file=rel,
                observed="no status: frontmatter",
                expected=f"set status: one of {{{', '.join(sorted(STATUS_ENUM))}}}",
            ))
        elif status not in STATUS_ENUM:
            drifts.append(DriftRow(
                invariant="(c) status-out-of-enum",
                file=rel,
                observed=f"status: {status}",
                expected=f"set status: one of {{{', '.join(sorted(STATUS_ENUM))}}}",
            ))

    # --- Invariant (d) — every registry row has a decision FILE (the converse of (a)) ---
    # AD-18 is bidirectional: "a numbered ADR ... PLUS its Registry row".  Until
    # 2026-08-15 only direction (a) was enforced, so a decision could be
    # *announced without being made* — and one had been: row 0079 was `accepted`
    # and cited by shipped source and by ADR-0078's "Consumes:" line, while
    # `0079-*.md` did not exist (86 files against 87 rows).  See bug-log #86.
    # "Every A has a B" and "every B has an A" are different checks; enforcing
    # only the cheap direction leaves the expensive one to discipline.
    file_ids = {_adr_number(p) for p in adr_files}
    for num in sorted(registered_ids - file_ids):
        drifts.append(DriftRow(
            invariant="(d) decision-file-missing",
            file=f"{ADR_DIR_NAME}/{num}-*.md",
            observed=f"README.md ## Registry has a row for ADR-{num}, but no matching file exists",
            expected=f"write {num}-<slug>.md, or delete the ADR-{num} row (numbers are never reused)",
        ))

    return drifts


def _run_self_test() -> int:
    """Run the inline self-test suite.  Returns 0 on all-pass, 1 on failure."""
    loader = unittest.TestLoader()
    suite = loader.loadTestsFromTestCase(_SelfTest)
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    return 0 if result.wasSuccessful() else 1


# ---------------------------------------------------------------------------
# Entry point.
# ---------------------------------------------------------------------------

def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="adr_registry_check",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument(
        "--pre-commit",
        action="store_true",
        default=False,
        help=(
            "Pre-commit hook mode (default): read staged index via "
            "`git diff --cached`.  No-arg invocation also uses this mode."
        ),
    )
    mode.add_argument(
        "--self-test",
        action="store_true",
        help="Run the in-process self-test suite and exit.",
    )
    mode.add_argument(
        "--ci",
        action="store_true",
        help=(
            "[RESERVED — not implemented at v0.1.0; see Q-ADR-WHEN] "
            "Post-commit CI mode using `git diff HEAD~1 HEAD` semantics."
        ),
    )
    args = parser.parse_args(argv)

    if args.ci:
        print(
            "adr-registry-check: error: --ci not implemented at v0.1.0; see Q-ADR-WHEN in "
            "_bmad-output/implementation-artifacts/6-6-adr-registry-atomic-lint.md",
            file=sys.stderr,
        )
        return 2

    if args.self_test:
        return _run_self_test()

    # Default: --pre-commit (also the no-arg case).
    return _run_pre_commit()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
