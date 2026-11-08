#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""queue_staleness_check.py — orchestrator pre-flight: Queue/Active staleness reconciliation.

Re-founded 2026-07-25 (BMAD-migration Phase 5b `spec/` retirement). Reads the
BMAD-native forward-looking backlog at
`_bmad-output/planning-artifacts/backlog.md`, extracts slugs from
MARKER-ONLY patterns within its `## Active` (optional — the new backlog is
Queue-only; sprint-status.yaml now owns "what's in flight") and `## Queue`
sections, cross-references each slug's STORY `Status:` line (was:
feature.md frontmatter) at
`_bmad-output/implementation-artifacts/*-<slug>.md`, and reports drift (a
live Queue/Active entry whose story status is done/retired).

Exit codes:
  0 — clean (zero drift); NO output (silent success)
  1 — drift detected; markdown table on stdout
  2 — script failure (missing `## Queue` section, unreadable file, bad args);
      message on stderr

Usage:
    python3 scripts/queue_staleness_check.py                    # live run
    python3 scripts/queue_staleness_check.py --self-test        # in-process smoke
    python3 scripts/queue_staleness_check.py --backlog PATH     # override backlog path
    python3 scripts/queue_staleness_check.py --stories-dir PATH # override story dir root

Part of the Pick C Wave 1 orchestrator hygiene compounder trio.
Per _bmad-output/implementation-artifacts/6-7-queue-staleness-reconciliation.md
§ Design D-QSR-1..6 (originally spec/v1/queue-staleness-reconciliation/feature.md).
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
# pointing at one of these is drift. Re-keyed 2026-07-25 (BMAD-migration
# Phase 5b) onto the story `Status:` vocabulary (was: feature.md frontmatter
# `shipped`/`shipped (retired)`/`deprecated`/`retired`/`shipped-partial` — the
# Phase-2 retro-generation convention collapses all of those into `done` or
# `retired`; see every story's own Dev Notes "Status mapping:" line).
SHIPPED_STATUSES: frozenset[str] = frozenset({"done", "retired"})

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
# 3. BMAD story id in backticks: `6-9-cockpit-cross-platform` (bug-log `#166`).
#
# The two markers above are both pre-migration `spec/` forms. Since the
# 2026-07-25 BMAD migration the Queue cites work as a story id, and the
# committed shape is
#
#     tracked as story
#     `6-9-cockpit-cross-platform` (`in-progress`).
#
# so `_BACKTICK_MARKER_RE` matched the STATUS WORD in the parens and not the
# story id at all. Measured on the live backlog before this: the extracted
# "slugs" were ['in-progress', 'backlog', 'backlog', 'arch-done',
# 'ready-for-dev'] — five status words, zero story ids, none resolvable.
#
# Matched STRUCTURALLY (`<digits>-<digits>-<slug>`) rather than by looking for
# the adjacent word "story": the id often sits on its own continuation line, and
# a digit-led id cannot collide with a status word, so this needs no denylist.
_STORY_ID_MARKER_RE = re.compile(r"`([0-9]+-[0-9]+-[a-z0-9][a-z0-9.\-]*)`")

# Status words the Queue writes in the SAME `(`...`)` shape as a feature slug, so
# `_BACKTICK_MARKER_RE` captures them as subjects. They can never resolve to a
# story file, so before bug-log `#166` they were the gate's entire candidate set
# and the "0 reconciled" denominator read as 9. Measured from the corpus:
# `grep -ohE '^Status: *[a-z-]+' _bmad-output/implementation-artifacts/*.md`
# yields exactly {done, retired, backlog, ready-for-dev, in-progress}; the other
# three are the lifecycle words the BMAD workflows use on the board.
_STATUS_WORD_DENYLIST = frozenset(
    {
        "done",
        "retired",
        "backlog",
        "ready-for-dev",
        "in-progress",
        "arch-done",
        "review",
        "dev-done",
        "shipped",
        "shipped-partial",
        "open",
        "cancelled",
    }
)

# H2 heading pattern for section boundary detection.
_H2_RE = re.compile(r"^## ", re.MULTILINE)

# HTML comment strip (D2.3) — strip before slug extraction.
_HTML_COMMENT_RE = re.compile(r"<!--.*?-->", re.DOTALL)


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
        if slug in _STATUS_WORD_DENYLIST:
            # A lifecycle word, not a subject — see _STATUS_WORD_DENYLIST.
            continue
        if slug not in seen:
            slugs.append(slug)
            seen.add(slug)
    for m in _STORY_ID_MARKER_RE.finditer(stripped):
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


_STORY_STATUS_LINE_RE = re.compile(r"^Status:\s*(\S+)", re.MULTILINE)


def _read_status(stories_dir: Path, slug: str) -> str | None:
    """Read the `Status:` line from the story matching `*-<slug>.md`.

    Re-keyed 2026-07-25 (BMAD-migration Phase 5b): was
    `spec_dir/<slug>/feature.md` frontmatter `status:`; now the BMAD-native
    story record, matched by FILENAME SUFFIX (not an exact `<epic>-<story>-`
    prefix guess — story filenames sometimes sanitize the slug or add a
    disambiguating prefix, e.g. lumen sub-phases).

    Returns None if no matching story file exists (R6.1) or it has no
    `Status:` line (R6.2). Normalises: lowercase, strip, drop inline
    `# comment` (R6.7).
    """
    if not stories_dir.is_dir():
        return None
    # Two shapes, and only the second matches a BMAD story (bug-log `#166`).
    # `*-{slug}.md` assumes a prefix before the slug — true for the retired
    # `spec/` feature folders, false for all 160 story files, which are named
    # `<epic>-<story>-<slug>.md` with the id AT THE FRONT. Measured:
    # `*-6-9-cockpit-cross-platform.md` matches 0 files, `6-9-…​.md` matches 1.
    # So capturing the story id (A) is necessary but NOT sufficient — without
    # this fallback it still resolves nothing, which is why the sweep's stated
    # one-line fix was a no-op on its own.
    matches = sorted(stories_dir.glob(f"*-{slug}.md")) or sorted(
        stories_dir.glob(f"{slug}.md")
    )
    if not matches:
        return None
    try:
        text = matches[0].read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    m = _STORY_STATUS_LINE_RE.search(text)
    if m is None:
        return None
    raw = m.group(1)
    # Normalize: strip inline # comments (R6.7) — defensive; the Status:
    # line is not commented in practice, but the historical feature.md
    # convention allowed it and this is a cheap carry-over.
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


@dataclass(frozen=True)
class Coverage:
    """What the sweep actually read (bug-log `#166`).

    `cited` = subject slugs extracted from non-excluded Active/Queue entries.
    `reconciled` = how many of those resolved to a story file WITH a `Status:`
    line. Everything downstream iterates only the reconciled set, so a run with
    `reconciled == 0` has checked nothing at all — and until 2026-11-08 that was
    the live state (5 cited, all of them status words, 0 reconciled) reported as a
    silent exit 0.
    """

    cited: int
    reconciled: int


def detect_drift(
    backlog_text: str,
    stories_dir: Path,
) -> list[DriftRow]:
    """Run the full reconciliation sweep. Returns a list of DriftRow items.

    Raises SystemExit(2) on structural parse failure.

    `## Active` is OPTIONAL as of the 2026-07-25 BMAD-migration Phase 5b
    re-founding — the new `_bmad-output/planning-artifacts/backlog.md` is
    Queue-only by design (plan: "keep the prose backlog as forward-only");
    `sprint-status.yaml`'s `in-progress` rows are now the "what's live" board.
    A missing `## Active` section degrades to an empty section rather than
    a structural error; `## Queue` remains required.
    """
    lines = backlog_text.splitlines()

    active_lines = _extract_section(lines, "Active") or []

    queue_lines = _extract_section(lines, "Queue")
    if queue_lines is None:
        print(
            "queue-staleness-check: ERROR: backlog.md missing required '## Queue' section",
            file=sys.stderr,
        )
        raise SystemExit(2)

    rows: list[DriftRow] = []
    cited = 0
    reconciled = 0

    for section_name, section_lines in (("Active", active_lines), ("Queue", queue_lines)):
        entries = _extract_entries(section_lines)
        for entry in entries:
            if _is_excluded(entry):
                continue
            slugs = _extract_slugs(entry)
            for slug in slugs:
                cited += 1
                status = _read_status(stories_dir, slug)
                if status is None:
                    # No matching story file (R6.1) or no Status: line (R6.2) — skip.
                    continue
                reconciled += 1
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
    detect_drift.last_coverage = Coverage(cited=cited, reconciled=reconciled)  # type: ignore[attr-defined]
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
    """In-process self-test covering SC1-SC6 (D3.5). Exits 0 on all-pass, 1 on any fail.

    Re-founded 2026-07-25 (BMAD-migration Phase 5b): fixtures write BMAD-native
    story stubs (`{fake-epic-story}-<slug>.md`, plain `Status:` line) instead
    of `feature.md` frontmatter files.
    """

    failures: list[str] = []

    def assert_eq(label: str, got: object, expected: object) -> None:
        if got != expected:
            failures.append(f"  FAIL [{label}]: got {got!r}, expected {expected!r}")

    # Build a temp stories-dir with mock story stubs.
    with tempfile.TemporaryDirectory() as tmpdir:
        stories_dir = Path(tmpdir)

        def write_story(slug: str, status: str) -> None:
            (stories_dir / f"9-1-{slug}.md").write_text(
                f"# Story 9.1: {slug}\n\nStatus: {status}\n", encoding="utf-8",
            )

        # SC1 — clean: Queue entry for feat-a, status: backlog → no drift.
        write_story("feat-a", "backlog")
        # SC2 — drift: Queue entry for feat-b, status: done, no exclude marker.
        write_story("feat-b", "done")
        # SC3 — exclude-rule: feat-c done but stub contains "RETIRED 2026-05-21; see Recent".
        write_story("feat-c", "done")
        # SC4 — K4 historical regression: v25-tcn-overlay case.
        write_story("v25-tcn-overlay", "done")
        # SC5 — missing story: feat-ghost has no story file on disk.
        # (no write needed)
        # SC6 — no Status: line: feat-nofm has a story file but no Status: line.
        (stories_dir / "9-1-feat-nofm.md").write_text(
            "# Story 9.1: feat-nofm\n\n(no Status: line)\n", encoding="utf-8",
        )

        # --- SC1 — clean ---
        backlog_sc1 = "## Active\n\n## Queue\n- **feat-a feature** (`feat-a`).\n\n## Recent (shipped)\n"
        rows_sc1 = detect_drift(backlog_sc1, stories_dir)
        assert_eq("SC1 drift-count", len(rows_sc1), 0)

        # --- SC2 — drift ---
        backlog_sc2 = "## Active\n\n## Queue\n- **feat-b feature** (`feat-b`).\n\n## Recent (shipped)\n"
        rows_sc2 = detect_drift(backlog_sc2, stories_dir)
        assert_eq("SC2 drift-count", len(rows_sc2), 1)
        if rows_sc2:
            assert_eq("SC2 slug", rows_sc2[0].slug, "feat-b")

        # --- SC3 — exclude-rule ---
        backlog_sc3 = (
            "## Active\n\n## Queue\n"
            "- **feat-c feature** (`feat-c`). **RETIRED 2026-05-21**; see Recent.\n\n"
            "## Recent (shipped)\n"
        )
        rows_sc3 = detect_drift(backlog_sc3, stories_dir)
        assert_eq("SC3 drift-count (exclude fired)", len(rows_sc3), 0)

        # --- SC4 — K4 historical regression: v25-tcn-overlay ---
        # Sub-case 4a: bare drift (no exclude annotation) → should flag.
        backlog_sc4a = (
            "## Active\n\n## Queue\n"
            "- **v2.5 TCN horizon-bump** (`v25-tcn-overlay`).\n\n"
            "## Recent (shipped)\n"
        )
        rows_sc4a = detect_drift(backlog_sc4a, stories_dir)
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
        rows_sc4b = detect_drift(backlog_sc4b, stories_dir)
        assert_eq("SC4b drift-count (exclude fired on historical real-shape)", len(rows_sc4b), 0)

        # --- SC5 — missing story ---
        backlog_sc5 = (
            "## Active\n\n## Queue\n"
            "- **ghost feature** (`feat-ghost`).\n\n"
            "## Recent (shipped)\n"
        )
        rows_sc5 = detect_drift(backlog_sc5, stories_dir)
        assert_eq("SC5 drift-count (missing story → skip)", len(rows_sc5), 0)

        # --- SC6 — no Status: line ---
        backlog_sc6 = (
            "## Active\n\n## Queue\n"
            "- **feat-nofm feature** (`feat-nofm`).\n\n"
            "## Recent (shipped)\n"
        )
        rows_sc6 = detect_drift(backlog_sc6, stories_dir)
        assert_eq("SC6 drift-count (no Status: line → skip)", len(rows_sc6), 0)

        # --- Additional: HTML comment suppression ---
        backlog_comment = (
            "## Active\n\n## Queue\n"
            "<!-- - **feat-b feature** (`feat-b`). -->\n\n"
            "## Recent (shipped)\n"
        )
        rows_comment = detect_drift(backlog_comment, stories_dir)
        assert_eq("HTML-comment suppression: commented entry not extracted", len(rows_comment), 0)

        # --- Additional: link marker extraction ---
        backlog_link = (
            "## Active\n\n## Queue\n"
            "- **feat-b feature** — see [feature.md](feat-b/feature.md).\n\n"
            "## Recent (shipped)\n"
        )
        rows_link = detect_drift(backlog_link, stories_dir)
        assert_eq("Link-marker extraction drift-count", len(rows_link), 1)

        # --- Additional: Active section also checked ---
        backlog_active = (
            "## Active\n"
            "- **feat-b in Active** (`feat-b`).\n\n"
            "## Queue\n\n"
            "## Recent (shipped)\n"
        )
        rows_active = detect_drift(backlog_active, stories_dir)
        assert_eq("Active-section drift-count", len(rows_active), 1)
        if rows_active:
            assert_eq("Active-section section label", rows_active[0].section, "Active")

        # --- Additional (Phase 5b): Active section entirely ABSENT (the new
        # backlog.md shape) degrades to empty rather than SystemExit(2).
        backlog_no_active = "## Queue\n- **feat-b feature** (`feat-b`).\n\n## Recent (shipped)\n"
        rows_no_active = detect_drift(backlog_no_active, stories_dir)
        assert_eq("No-Active-section drift-count", len(rows_no_active), 1)

        # ── bug-log #166 probes. Every case above uses PRE-migration markup
        # (`(`feat-a`)`) and a `9-1-`-prefixed story file, which is exactly why
        # the regression was invisible: the fixtures matched the two legacy
        # markers and the `*-{slug}.md` glob, so both live blockers were
        # unreachable from the self-test. These cases use the COMMITTED shapes.
        with tempfile.TemporaryDirectory() as bmad_dir:
            bmad = Path(bmad_dir)
            # Story files as they actually exist: id AT THE FRONT, no prefix.
            (bmad / "6-9-cockpit-cross-platform.md").write_text(
                "# Story 6.9\n\nStatus: in-progress\n", encoding="utf-8"
            )
            (bmad / "2-7-lumen-phase-6-assistant-slot.md").write_text(
                "# Story 2.7\n\nStatus: done\n", encoding="utf-8"
            )

            # SC-M1 — the committed citation shape, on its own continuation line,
            # with the STATUS in the parens where the legacy marker looks.
            backlog_m1 = (
                "## Active\n\n## Queue\n"
                "- **cockpit-cross-platform CI** \u2014 shakeout open \u2014 tracked as story\n"
                "  `6-9-cockpit-cross-platform` (`in-progress`).\n\n"
                "## Recent (shipped)\n"
            )
            slugs_m1 = _extract_slugs(
                "- **x** \u2014 tracked as story\n  `6-9-cockpit-cross-platform` (`in-progress`)."
            )
            assert_eq("SC-M1 story id captured", "6-9-cockpit-cross-platform" in slugs_m1, True)
            assert_eq(
                "SC-M1 status word NOT captured as a subject",
                "in-progress" in slugs_m1,
                False,
            )
            assert_eq("SC-M1 exactly one subject", len(slugs_m1), 1)

            # SC-M2 — resolution against a bare-basename story file.
            assert_eq(
                "SC-M2 bare-basename resolves",
                _read_status(bmad, "6-9-cockpit-cross-platform"),
                "in-progress",
            )

            # SC-M3 — end to end: in-progress is NOT shipped, so no drift, and
            # the coverage record must say it reconciled something.
            rows_m3 = detect_drift(backlog_m1, bmad)
            cov_m3 = detect_drift.last_coverage  # type: ignore[attr-defined]
            assert_eq("SC-M3 drift-count", len(rows_m3), 0)
            assert_eq("SC-M3 cited", cov_m3.cited, 1)
            assert_eq("SC-M3 reconciled", cov_m3.reconciled, 1)

            # SC-M4 — end to end with a SHIPPED story: drift must be detected
            # through the new marker, not just reconciled. Without this, SC-M3
            # alone would pass on a marker that captures but never drifts.
            backlog_m4 = (
                "## Active\n\n## Queue\n"
                "- **lumen phase 6** \u2014 tracked as story\n"
                "  `2-7-lumen-phase-6-assistant-slot` (`backlog`).\n\n"
                "## Recent (shipped)\n"
            )
            rows_m4 = detect_drift(backlog_m4, bmad)
            assert_eq("SC-M4 drift detected via the story-id marker", len(rows_m4), 1)
            if rows_m4:
                assert_eq("SC-M4 slug", rows_m4[0].slug, "2-7-lumen-phase-6-assistant-slot")

            # SC-M5 — the floor: a Queue citing only status words reconciles
            # nothing, and that must NOT read as clean.
            backlog_m5 = (
                "## Active\n\n## Queue\n- **something** (`ready-for-dev`).\n\n"
                "## Recent (shipped)\n"
            )
            rows_m5 = detect_drift(backlog_m5, bmad)
            cov_m5 = detect_drift.last_coverage  # type: ignore[attr-defined]
            assert_eq("SC-M5 no rows", len(rows_m5), 0)
            assert_eq("SC-M5 reconciled is zero", cov_m5.reconciled, 0)
            assert_eq(
                "SC-M5 status word denylisted, so nothing is even cited",
                cov_m5.cited,
                0,
            )

    if failures:
        print("queue-staleness-check --self-test: FAILED", file=sys.stderr)
        for f in failures:
            print(f, file=sys.stderr)
        raise SystemExit(1)
    else:
        print("queue-staleness-check --self-test: all cases PASS (incl. 5 bug-log #166 probes)")


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
        help="Override the backlog path (default: REPO_ROOT/_bmad-output/planning-artifacts/backlog.md).",
    )
    parser.add_argument(
        "--stories-dir",
        metavar="PATH",
        default=None,
        help="Override the story dir root (default: REPO_ROOT/_bmad-output/implementation-artifacts).",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_arg_parser()
    args = parser.parse_args(argv)

    if args.self_test:
        run_self_test()
        return 0  # run_self_test raises SystemExit on failure.

    backlog_path = (
        Path(args.backlog) if args.backlog
        else REPO_ROOT / "_bmad-output" / "planning-artifacts" / "backlog.md"
    )
    stories_dir = (
        Path(args.stories_dir) if args.stories_dir
        else REPO_ROOT / "_bmad-output" / "implementation-artifacts"
    )

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
        rows = detect_drift(backlog_text, stories_dir)
    except SystemExit as e:
        return int(e.code) if e.code is not None else 2

    cov: Coverage = getattr(
        detect_drift, "last_coverage", Coverage(cited=0, reconciled=0)
    )

    # NON-VACUITY FLOOR (bug-log #166). Every check below iterates the RECONCILED
    # set, so zero reconciled means the sweep examined nothing — regardless of how
    # many slugs it cited. That was the live state until 2026-11-08 and it exited
    # 0 in silence. `cited == 0` is also a failure: the Queue always names at least
    # one tracked story, so an empty citation set means the markers stopped
    # matching, not that the backlog is empty.
    if cov.reconciled == 0:
        print(
            "queue-staleness-check: FAIL — reconciled 0 of "
            f"{cov.cited} cited slug(s). Every staleness check iterates the "
            "reconciled set, so this run verified NOTHING. Either the Queue's "
            "citation markers no longer match what it writes (see "
            "_STORY_ID_MARKER_RE) or story-file resolution broke (see "
            "_read_status). A clean exit here would be a gate that cannot fail.",
            file=sys.stderr,
        )
        return 2

    if not rows:
        # Clean, and no longer silent: R2.3's silence made "0 drift over 4
        # reconciled stories" byte-identical to "0 drift over nothing".
        print(
            f"queue-staleness-check: PASS (0 drift · "
            f"{cov.reconciled} of {cov.cited} cited slug(s) reconciled)"
        )
        return 0

    # Drift detected — emit table to stdout (D1.4).
    print(format_drift_table(rows))
    print(
        f"queue-staleness-check: {len(rows)} drift row(s) · "
        f"{cov.reconciled} of {cov.cited} cited slug(s) reconciled",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
