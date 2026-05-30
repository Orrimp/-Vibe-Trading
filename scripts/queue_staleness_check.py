#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""queue_staleness_check.py — orchestrator pre-flight: Queue/Active staleness reconciliation.

Reads spec/backlog.md, extracts slugs from MARKER-ONLY patterns within the
## Active and ## Queue sections, cross-references each slug's feature.md
frontmatter status, and reports drift (a live Queue/Active entry whose folder
status is shipped/deprecated/retired).

Exit codes:
  0 — clean (zero drift); NO output (silent success)
  1 — drift detected; markdown table on stdout
  2 — script failure (missing section, unreadable file, bad args); message on stderr

Usage:
    python3 scripts/queue_staleness_check.py                 # live run
    python3 scripts/queue_staleness_check.py --self-test     # in-process smoke
    python3 scripts/queue_staleness_check.py --backlog PATH  # override backlog path
    python3 scripts/queue_staleness_check.py --spec-dir PATH # override spec dir root

Part of the Pick C Wave 1 orchestrator hygiene compounder trio.
Per spec/queue-staleness-reconciliation/feature.md § Design D-QSR-1..6.
"""
from __future__ import annotations

import argparse
import re
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# Statuses that signal "this feature is done" — a live Queue/Active entry
# pointing at one of these is drift. Widened from the brief's initial 3 per
# architect frontmatter survey (D3.2).
SHIPPED_STATUSES: frozenset[str] = frozenset(
    {"shipped", "shipped (retired)", "deprecated", "retired", "shipped-partial"}
)

# Case-insensitive substrings that indicate the entry ALREADY annotates the
# shipped/retired state — these are CORRECT post-ship annotations, not drift.
# The `# noqa: queue-staleness` escape hatch lets the operator suppress inline.
EXCLUDE_MARKERS: list[str] = [
    "see recent",
    "shipped 2026",
    "retired 2026",
    "retired-by-context",
    "moved to recent",
    "# noqa: queue-staleness",
]

# Marker-only slug extraction regexes (D2.2).
# 1. Markdown feature link: (slug/feature.md)
_LINK_MARKER_RE = re.compile(r"\(([a-z0-9][a-z0-9.\-]*)/feature\.md\)")
# 2. Backtick-in-paren slug: (`slug`)
_BACKTICK_MARKER_RE = re.compile(r"\(`([a-z0-9][a-z0-9.\-]+)`\)")

# H2 heading pattern for section boundary detection.
_H2_RE = re.compile(r"^## ", re.MULTILINE)

# HTML comment strip (D2.3) — strip before slug extraction.
_HTML_COMMENT_RE = re.compile(r"<!--.*?-->", re.DOTALL)


# ---------------------------------------------------------------------------
# Frontmatter parsing (lifted verbatim from scripts/spec_lint.py lines 127-148)
# ---------------------------------------------------------------------------

FRONTMATTER_RE = re.compile(r"\A---\r?\n(.*?)\r?\n---\r?\n", re.DOTALL)


def parse_frontmatter(text: str) -> dict[str, str] | None:
    """Return a flat dict of YAML-style frontmatter keys, or None if absent.

    We deliberately do not pull in PyYAML; spec frontmatter is simple
    `key: value` lines. Lists/nested objects are not used and would be a
    design smell here.
    """
    m = FRONTMATTER_RE.match(text)
    if not m:
        return None
    out: dict[str, str] = {}
    for line in m.group(1).splitlines():
        line = line.rstrip()
        if not line or line.lstrip().startswith("#"):
            continue
        if ":" not in line:
            continue
        k, _, v = line.partition(":")
        out[k.strip()] = v.strip()
    return out


# ---------------------------------------------------------------------------
# Section extraction
# ---------------------------------------------------------------------------


def _extract_section(lines: list[str], heading: str) -> list[str] | None:
    """Return the lines belonging to the H2 section matching `heading`.

    Scans for an exact H2 match (`## <heading>`) and collects lines until
    the next H2 (`## `) or end-of-file. Returns None if the heading is not
    found.
    """
    start: int | None = None
    result: list[str] = []
    for i, line in enumerate(lines):
        if start is None:
            if line.rstrip() == f"## {heading}":
                start = i
        else:
            # Stop at the next H2 boundary.
            if line.startswith("## "):
                break
            result.append(line)
    return result if start is not None else None


# ---------------------------------------------------------------------------
# Entry parsing + slug extraction
# ---------------------------------------------------------------------------


def _extract_entries(section_lines: list[str]) -> list[tuple[str, str]]:
    """Split a section into (stub_text, raw_text) list entries.

    An entry is a top-level list item starting with `^- ` (optionally `- **`).
    Continuation lines (indented or blank with following content) are folded
    into the same entry. Returns a list of (entry_text,) strings, one per
    top-level bullet.
    """
    entries: list[str] = []
    current: list[str] = []
    for line in section_lines:
        if line.startswith("- "):
            if current:
                entries.append("\n".join(current))
            current = [line]
        elif current and (line.startswith("  ") or line.startswith("\t") or line == ""):
            current.append(line)
        else:
            # Non-bullet, non-continuation line (e.g. ### heading, prose paragraph).
            if current:
                entries.append("\n".join(current))
                current = []
    if current:
        entries.append("\n".join(current))
    return entries


def _extract_slugs(entry_text: str) -> list[str]:
    """Extract slugs from marker-only patterns in an entry.

    Strip HTML comments first, then apply the two marker regexes.
    De-duplicates within a single entry (D2.4).
    """
    stripped = _HTML_COMMENT_RE.sub("", entry_text)
    slugs: list[str] = []
    seen: set[str] = set()
    for m in _LINK_MARKER_RE.finditer(stripped):
        slug = m.group(1)
        if slug not in seen:
            slugs.append(slug)
            seen.add(slug)
    for m in _BACKTICK_MARKER_RE.finditer(stripped):
        slug = m.group(1)
        if slug not in seen:
            slugs.append(slug)
            seen.add(slug)
    return slugs


def _is_excluded(entry_text: str) -> bool:
    """Return True if the entry already annotates a shipped/retired state (D3.3)."""
    lower = entry_text.lower()
    return any(marker in lower for marker in EXCLUDE_MARKERS)


def _stub_excerpt(entry_text: str, max_len: int = 80) -> str:
    """Return a ≤ max_len char excerpt of the entry's first meaningful prose line.

    Collapses whitespace, pipe-escapes `|` (D4.1), strips leading `- ` bullet.
    """
    # Use the first non-empty line.
    for line in entry_text.splitlines():
        line = line.strip().lstrip("- ").strip()
        if line:
            # Collapse internal whitespace.
            excerpt = " ".join(line.split())
            # Pipe-escape for markdown table.
            excerpt = excerpt.replace("|", r"\|")
            if len(excerpt) > max_len:
                excerpt = excerpt[: max_len - 3] + "..."
            return excerpt
    return ""


# ---------------------------------------------------------------------------
# Status read
# ---------------------------------------------------------------------------


def _read_status(spec_dir: Path, slug: str) -> str | None:
    """Read the `status` field from spec_dir/<slug>/feature.md.

    Returns None if the file is missing (R6.1) or has no status key (R6.2).
    Normalises: lowercase, strip, drop inline `# comment` (R6.7).
    """
    feature_path = spec_dir / slug / "feature.md"
    if not feature_path.exists():
        return None
    try:
        text = feature_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    fm = parse_frontmatter(text)
    if fm is None or "status" not in fm:
        return None
    raw = fm["status"]
    # Normalize: strip inline # comments (R6.7).
    if "#" in raw:
        raw = raw[: raw.index("#")]
    return raw.strip().lower()


# ---------------------------------------------------------------------------
# Drift detection
# ---------------------------------------------------------------------------


@dataclass
class DriftRow:
    slug: str
    section: str  # "Active" or "Queue"
    stub_excerpt: str
    folder_status: str


def detect_drift(
    backlog_text: str,
    spec_dir: Path,
) -> list[DriftRow]:
    """Run the full reconciliation sweep. Returns a list of DriftRow items.

    Raises SystemExit(2) on structural parse failure.
    """
    lines = backlog_text.splitlines()

    active_lines = _extract_section(lines, "Active")
    if active_lines is None:
        print(
            "queue-staleness-check: ERROR: spec/backlog.md missing required '## Active' section",
            file=sys.stderr,
        )
        raise SystemExit(2)

    queue_lines = _extract_section(lines, "Queue")
    if queue_lines is None:
        print(
            "queue-staleness-check: ERROR: spec/backlog.md missing required '## Queue' section",
            file=sys.stderr,
        )
        raise SystemExit(2)

    rows: list[DriftRow] = []

    for section_name, section_lines in (("Active", active_lines), ("Queue", queue_lines)):
        entries = _extract_entries(section_lines)
        for entry in entries:
            if _is_excluded(entry):
                continue
            slugs = _extract_slugs(entry)
            for slug in slugs:
                status = _read_status(spec_dir, slug)
                if status is None:
                    # Missing folder (R6.1) or missing status key (R6.2) — skip.
                    continue
                if status in SHIPPED_STATUSES:
                    rows.append(
                        DriftRow(
                            slug=slug,
                            section=section_name,
                            stub_excerpt=_stub_excerpt(entry),
                            folder_status=status,
                        )
                    )

    # Sort by (section, slug) for byte-stability (D4.3).
    rows.sort(key=lambda r: (r.section, r.slug))
    return rows


# ---------------------------------------------------------------------------
# Output formatting (D-QSR-4)
# ---------------------------------------------------------------------------

_SUGGESTED_FIX_QUEUE = (
    'update Queue text to annotate shipped state (e.g. "shipped YYYY-MM-DD; see Recent") or remove the stale stub'
)
_SUGGESTED_FIX_ACTIVE = (
    "feature is {status}; move the Active tracking row to Recent (shipped) or annotate the ship date"
)


def format_drift_table(rows: list[DriftRow]) -> str:
    """Render the drift table per D-QSR-4 / bundle Q-HYG-EMIT dialect."""
    n = len(rows)
    plural = "s" if n != 1 else ""
    header = f"queue-staleness-check: {n} drift{plural} detected"
    table_header = "| slug | section | queue says | folder status | suggested fix |"
    table_sep = "|------|---------|-----------|---------------|----------------|"
    table_rows: list[str] = []
    for row in rows:
        if row.section == "Active":
            fix = _SUGGESTED_FIX_ACTIVE.format(status=row.folder_status)
        else:
            fix = _SUGGESTED_FIX_QUEUE
        table_rows.append(
            f"| {row.slug} | {row.section} | {row.stub_excerpt} | {row.folder_status} | {fix} |"
        )
    return "\n".join([header, table_header, table_sep] + table_rows)


# ---------------------------------------------------------------------------
# Self-test (D-QSR-3 / D3.5)
# ---------------------------------------------------------------------------


def run_self_test() -> None:
    """In-process self-test covering SC1-SC6 (D3.5). Exits 0 on all-pass, 1 on any fail."""

    failures: list[str] = []

    def assert_eq(label: str, got: object, expected: object) -> None:
        if got != expected:
            failures.append(f"  FAIL [{label}]: got {got!r}, expected {expected!r}")

    # Build a temp spec-dir with mock feature.md files.
    with tempfile.TemporaryDirectory() as tmpdir:
        spec_dir = Path(tmpdir)

        def write_feature(slug: str, status: str) -> None:
            folder = spec_dir / slug
            folder.mkdir(parents=True, exist_ok=True)
            (folder / "feature.md").write_text(
                f"---\nslug: {slug}\nstatus: {status}\n---\n# {slug}\n",
                encoding="utf-8",
            )

        # SC1 — clean: Queue entry for feat-a, status: draft → no drift.
        write_feature("feat-a", "draft")
        # SC2 — drift: Queue entry for feat-b, status: shipped, no exclude marker.
        write_feature("feat-b", "shipped")
        # SC3 — exclude-rule: feat-c shipped but stub contains "RETIRED 2026-05-21; see Recent".
        write_feature("feat-c", "shipped")
        # SC4 — K4 historical regression: v25-tcn-overlay case.
        write_feature("v25-tcn-overlay", "shipped")
        # SC5 — missing folder: feat-ghost has no feature.md on disk.
        # (no write needed)
        # SC6 — no status key: feat-nofm has frontmatter but no status line.
        folder_nofm = spec_dir / "feat-nofm"
        folder_nofm.mkdir(parents=True, exist_ok=True)
        (folder_nofm / "feature.md").write_text(
            "---\nslug: feat-nofm\n---\n# feat-nofm\n",
            encoding="utf-8",
        )

        # --- SC1 — clean ---
        backlog_sc1 = "## Active\n\n## Queue\n- **feat-a feature** (`feat-a`).\n\n## Recent (shipped)\n"
        rows_sc1 = detect_drift(backlog_sc1, spec_dir)
        assert_eq("SC1 drift-count", len(rows_sc1), 0)

        # --- SC2 — drift ---
        backlog_sc2 = "## Active\n\n## Queue\n- **feat-b feature** (`feat-b`).\n\n## Recent (shipped)\n"
        rows_sc2 = detect_drift(backlog_sc2, spec_dir)
        assert_eq("SC2 drift-count", len(rows_sc2), 1)
        if rows_sc2:
            assert_eq("SC2 slug", rows_sc2[0].slug, "feat-b")

        # --- SC3 — exclude-rule ---
        backlog_sc3 = (
            "## Active\n\n## Queue\n"
            "- **feat-c feature** (`feat-c`). **RETIRED 2026-05-21**; see Recent.\n\n"
            "## Recent (shipped)\n"
        )
        rows_sc3 = detect_drift(backlog_sc3, spec_dir)
        assert_eq("SC3 drift-count (exclude fired)", len(rows_sc3), 0)

        # --- SC4 — K4 historical regression: v25-tcn-overlay ---
        # Sub-case 4a: bare drift (no exclude annotation) → should flag.
        backlog_sc4a = (
            "## Active\n\n## Queue\n"
            "- **v2.5 TCN horizon-bump** (`v25-tcn-overlay`).\n\n"
            "## Recent (shipped)\n"
        )
        rows_sc4a = detect_drift(backlog_sc4a, spec_dir)
        assert_eq("SC4a drift-count (no exclude)", len(rows_sc4a), 1)
        if rows_sc4a:
            assert_eq("SC4a slug", rows_sc4a[0].slug, "v25-tcn-overlay")

        # Sub-case 4b: real-shape entry WITH "**RETIRED 2026-05-21**; see Recent" → EXCLUDE fires.
        backlog_sc4b = (
            "## Active\n\n## Queue\n"
            "- **v2.5 TCN horizon-bump (`v25-tcn-overlay`).** **RETIRED 2026-05-21**;"
            " see Recent (shipped).\n\n"
            "## Recent (shipped)\n"
        )
        rows_sc4b = detect_drift(backlog_sc4b, spec_dir)
        assert_eq("SC4b drift-count (exclude fired on historical real-shape)", len(rows_sc4b), 0)

        # --- SC5 — missing folder ---
        backlog_sc5 = (
            "## Active\n\n## Queue\n"
            "- **ghost feature** (`feat-ghost`).\n\n"
            "## Recent (shipped)\n"
        )
        rows_sc5 = detect_drift(backlog_sc5, spec_dir)
        assert_eq("SC5 drift-count (missing folder → skip)", len(rows_sc5), 0)

        # --- SC6 — no status key ---
        backlog_sc6 = (
            "## Active\n\n## Queue\n"
            "- **feat-nofm feature** (`feat-nofm`).\n\n"
            "## Recent (shipped)\n"
        )
        rows_sc6 = detect_drift(backlog_sc6, spec_dir)
        assert_eq("SC6 drift-count (no status key → skip)", len(rows_sc6), 0)

        # --- Additional: HTML comment suppression ---
        backlog_comment = (
            "## Active\n\n## Queue\n"
            "<!-- - **feat-b feature** (`feat-b`). -->\n\n"
            "## Recent (shipped)\n"
        )
        rows_comment = detect_drift(backlog_comment, spec_dir)
        assert_eq("HTML-comment suppression: commented entry not extracted", len(rows_comment), 0)

        # --- Additional: link marker extraction ---
        backlog_link = (
            "## Active\n\n## Queue\n"
            "- **feat-b feature** — see [feature.md](feat-b/feature.md).\n\n"
            "## Recent (shipped)\n"
        )
        rows_link = detect_drift(backlog_link, spec_dir)
        assert_eq("Link-marker extraction drift-count", len(rows_link), 1)

        # --- Additional: Active section also checked ---
        backlog_active = (
            "## Active\n"
            "- **feat-b in Active** (`feat-b`).\n\n"
            "## Queue\n\n"
            "## Recent (shipped)\n"
        )
        rows_active = detect_drift(backlog_active, spec_dir)
        assert_eq("Active-section drift-count", len(rows_active), 1)
        if rows_active:
            assert_eq("Active-section section label", rows_active[0].section, "Active")

    if failures:
        print("queue-staleness-check --self-test: FAILED", file=sys.stderr)
        for f in failures:
            print(f, file=sys.stderr)
        raise SystemExit(1)
    else:
        print("queue-staleness-check --self-test: all cases PASS")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def build_arg_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="queue_staleness_check.py",
        description="Orchestrator pre-flight: detect stale Queue/Active backlog entries.",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run in-process self-test suite (SC1-SC6); exit 0 all-pass / 1 any-fail.",
    )
    parser.add_argument(
        "--backlog",
        metavar="PATH",
        default=None,
        help="Override the backlog path (default: REPO_ROOT/spec/backlog.md).",
    )
    parser.add_argument(
        "--spec-dir",
        metavar="PATH",
        default=None,
        help="Override the spec dir root (default: REPO_ROOT/spec).",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_arg_parser()
    args = parser.parse_args(argv)

    if args.self_test:
        run_self_test()
        return 0  # run_self_test raises SystemExit on failure.

    backlog_path = Path(args.backlog) if args.backlog else REPO_ROOT / "spec" / "backlog.md"
    spec_dir = Path(args.spec_dir) if args.spec_dir else REPO_ROOT / "spec"

    # Read backlog.
    try:
        backlog_text = backlog_path.read_text(encoding="utf-8", errors="replace")
    except OSError as e:
        print(
            f"queue-staleness-check: ERROR: cannot read {backlog_path}: {e}",
            file=sys.stderr,
        )
        return 2

    # Detect drift (may exit 2 internally on missing sections).
    try:
        rows = detect_drift(backlog_text, spec_dir)
    except SystemExit as e:
        return int(e.code) if e.code is not None else 2

    if not rows:
        # Clean — silent success (R2.3).
        return 0

    # Drift detected — emit table to stdout (D1.4).
    print(format_drift_table(rows))
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
