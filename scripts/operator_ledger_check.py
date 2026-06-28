#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""operator_ledger_check.py — orchestrator pre-flight: operator-side pending ledger lint.

Reads spec/dev-notes/operator-side-pending-ledger.md and enforces:
  - Table schema (per-table column count and header match).
  - Status enum {pending, FAILED, done, cancelled} on Pending rows.
  - Stale FAILED escalation (>= STALE_FAILED_DAYS days since Date surfaced).
  - Done rows have a completion date in the Completed column.
  - Cancelled rows have a cancel date in the Cancelled column.
  - FAILED rows cite a spec/dev-notes/*.md follow-up in their Notes cell.

Exit codes:
  0 — clean (or only within-window FAILED soft-warnings on stdout)
  1 — HARD schema/contract violation OR stale-FAILED escalation; markdown table on stderr
  >= 2 — script failure (missing file, bad args, structurally unparseable ledger); stderr error:

Usage:
    python3 scripts/operator_ledger_check.py                          # live run
    python3 scripts/operator_ledger_check.py --today 2026-05-29       # deterministic date
    python3 scripts/operator_ledger_check.py --ledger PATH            # override ledger path
    python3 scripts/operator_ledger_check.py --self-test              # inline self-test suite

Part of the Pick C Wave 1 orchestrator hygiene compounder trio.
Per spec/v1/operator-ledger-schema-lint/feature.md § Design D-LED-1..8 + P-LED-1.
"""
from __future__ import annotations

import argparse
import datetime
import re
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# ---------------------------------------------------------------------------
# Constants (change-management surface — K2 / K3 mitigations)
# ---------------------------------------------------------------------------

STALE_FAILED_DAYS: int = 7

# Canonical status enum for the Pending table only (F3: Done/Cancelled encode
# status by which table the row lives in — they have no Status column).
CANONICAL_STATUS: frozenset[str] = frozenset({"pending", "FAILED", "done", "cancelled"})

# SCHEMA: module-top dict keyed by table key. "columns" is the REQUIRED ordered
# prefix; extra trailing columns are accepted (K2 forward-compat).
# "status_col": name of the Status column, or None.
# "completion_col": name of the completion/cancel date column, or None.
SCHEMA: dict[str, dict] = {
    "pending": {
        "heading": "pending recipes",
        "columns": ["Date surfaced", "Recipe", "Cost", "Unblocks", "Status", "Notes"],
        "status_col": "Status",
        "date_col": "Date surfaced",
        "completion_col": None,
    },
    "done": {
        "heading": "done recipes",
        "columns": ["Date surfaced", "Recipe", "Cost", "Completed", "Outcome"],
        "status_col": None,
        "date_col": "Date surfaced",
        "completion_col": "Completed",
    },
    "cancelled": {
        "heading": "cancelled recipes",
        "columns": ["Date surfaced", "Recipe", "Cost", "Cancelled", "Reason"],
        "status_col": None,
        "date_col": "Date surfaced",
        "completion_col": "Cancelled",
    },
}

# Heading normalisation: strip "(audit trail)" suffix, lowercase, strip.
_HEADING_STRIP_RE = re.compile(r"\s*\(.*?\)\s*$")

# Separator line: a row where every non-empty cell is all dashes/colons.
_SEP_CELL_RE = re.compile(r"^[-:]+$")

# Markdown-strip patterns (for normalize_cell).
_BOLD_RE = re.compile(r"\*\*(.+?)\*\*|__(.+?)__")
_ITALIC_RE = re.compile(r"\*(.+?)\*|_(.+?)_")
_BACKTICK_RE = re.compile(r"`(.+?)`")
_LINK_RE = re.compile(r"\[([^\]]*)\]\([^)]*\)")

# dev-note citation pattern (Q-LED-NOTE check on FAILED Notes cell).
_DEVNOTE_RE = re.compile(r"spec/dev-notes/[A-Za-z0-9._\-/]+\.md")

# Date-surfaced date extraction: first ISO date token in a cell.
_DATE_RE = re.compile(r"\b(\d{4}-\d{2}-\d{2})\b")


# ---------------------------------------------------------------------------
# Cell helpers
# ---------------------------------------------------------------------------


def normalize_cell(s: str) -> str:
    """Strip markdown formatting from a cell for enum/date matching.

    Preserves the raw cell value elsewhere; this is ONLY used for status-enum
    match and date-parse, never for storing the raw cell.
    """
    # Collapse links to their text.
    s = _LINK_RE.sub(r"\1", s)
    # Strip bold/italic/backticks.
    s = _BOLD_RE.sub(r"\1\2", s)
    s = _ITALIC_RE.sub(r"\1\2", s)
    s = _BACKTICK_RE.sub(r"\1", s)
    return s.strip()


def _split_row(line: str) -> list[str] | None:
    """Split a markdown table row into cells.

    Returns None if the line is not a table row (doesn't start AND end with |
    after strip). The leading/trailing empty cells from bounding pipes are
    dropped. Escaped pipes \\| are unescaped after splitting.
    """
    s = line.strip()
    if not (s.startswith("|") and s.endswith("|")):
        return None
    # Split on unescaped pipes.
    parts = re.split(r"(?<!\\)\|", s)
    # Drop the empty first and last elements produced by bounding pipes.
    cells = [p.replace("\\|", "|").strip() for p in parts[1:-1]]
    return cells


def _is_separator(cells: list[str]) -> bool:
    """Return True if all non-empty cells are all dashes/colons (table separator)."""
    return all(_SEP_CELL_RE.match(c) for c in cells if c)


# ---------------------------------------------------------------------------
# Issue dataclass
# ---------------------------------------------------------------------------


@dataclass
class Issue:
    issue: str          # issue class
    row_label: str      # "Date surfaced + recipe excerpt"
    observed: str
    expected: str
    action: str
    is_hard: bool = True


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------


@dataclass
class ParsedRow:
    cells: list[str]
    line_no: int        # 1-based line number in the ledger
    table_key: str      # "pending" / "done" / "cancelled"


def parse_ledger(text: str) -> tuple[dict[str, list[ParsedRow]], list[Issue]]:
    """Parse the ledger markdown into per-table row lists.

    Returns (rows_by_table, structural_issues). Structural issues are HARD
    violations that prevent further per-row checks (e.g. wrong header columns,
    undersized rows).
    """
    lines = text.splitlines()
    rows_by_table: dict[str, list[ParsedRow]] = {k: [] for k in SCHEMA}
    structural_issues: list[Issue] = []

    # Build heading → table_key lookup (case-insensitive, suffix-stripped).
    heading_map: dict[str, str] = {}
    for key, schema in SCHEMA.items():
        heading_map[schema["heading"]] = key

    active_table: str | None = None
    header_seen: set[str] = set()
    separator_seen: set[str] = set()

    for i, line in enumerate(lines, start=1):
        stripped = line.rstrip()

        # Detect ## headings → switch active table context.
        if stripped.startswith("## "):
            heading_raw = stripped[3:].strip()
            heading_norm = _HEADING_STRIP_RE.sub("", heading_raw).strip().lower()
            active_table = heading_map.get(heading_norm)
            # Reset per-table tracking on each new heading.
            continue

        # If not in a known table, skip.
        if active_table is None:
            continue

        # Try to parse as a table row.
        cells = _split_row(stripped)
        if cells is None:
            # Non-table content (blank line, prose, sub-heading) — OK.
            continue

        # Separator row?
        if _is_separator(cells):
            separator_seen.add(active_table)
            continue

        # Header row? (appears before separator)
        if active_table not in header_seen:
            # This is the header row — validate it.
            required = SCHEMA[active_table]["columns"]
            norm_cells = [c.strip() for c in cells]
            # Check that the first len(required) cells match (case-insensitive).
            if len(norm_cells) < len(required):
                structural_issues.append(Issue(
                    issue="schema-table-header",
                    row_label=f"table:{active_table} line:{i}",
                    observed=f"{len(norm_cells)} header columns: {norm_cells}",
                    expected=f">= {len(required)} columns starting with {required}",
                    action="fix the table header to match the schema",
                ))
            else:
                mismatches = [
                    (required[j], norm_cells[j])
                    for j in range(len(required))
                    if norm_cells[j].lower() != required[j].lower()
                ]
                if mismatches:
                    structural_issues.append(Issue(
                        issue="schema-table-header",
                        row_label=f"table:{active_table} line:{i}",
                        observed=str([m[1] for m in mismatches]),
                        expected=str([m[0] for m in mismatches]),
                        action="fix the table header to match the schema",
                    ))
            header_seen.add(active_table)
            continue

        # Data row (after header + separator seen).
        if active_table not in separator_seen:
            # Row appeared before separator — treat as data anyway but flag.
            pass

        required = SCHEMA[active_table]["columns"]
        if len(cells) < len(required):
            # Undersized row: HARD schema-row-truncated (D-LED-2 fragility guard).
            structural_issues.append(Issue(
                issue="schema-row-truncated",
                row_label=f"table:{active_table} line:{i}",
                observed=f"{len(cells)} cells (need >= {len(required)}): {cells}",
                expected=f">= {len(required)} cells per schema",
                action="check if the row wraps across physical lines (v0.2.0 candidate) or fix the row",
            ))
            continue

        rows_by_table[active_table].append(ParsedRow(cells=cells, line_no=i, table_key=active_table))

    return rows_by_table, structural_issues


# ---------------------------------------------------------------------------
# Semantic checks
# ---------------------------------------------------------------------------


def _row_label(cells: list[str], schema_key: str) -> str:
    """Build a short row label: 'Date_surfaced + first ~40 chars of Recipe'."""
    date_raw = cells[0] if cells else "?"
    recipe_raw = cells[1] if len(cells) > 1 else "?"
    date_norm = normalize_cell(date_raw)
    recipe_norm = normalize_cell(recipe_raw)
    if len(recipe_norm) > 40:
        recipe_norm = recipe_norm[:37] + "..."
    return f"{date_norm} {recipe_norm}".strip()


def _parse_iso_date(raw: str) -> datetime.date | None:
    """Extract and parse the first ISO date token from a (possibly markdown-rich) cell."""
    norm = normalize_cell(raw)
    m = _DATE_RE.search(norm)
    if not m:
        return None
    try:
        return datetime.date.fromisoformat(m.group(1))
    except ValueError:
        return None


def check_rows(
    rows_by_table: dict[str, list[ParsedRow]],
    today: datetime.date,
) -> tuple[list[Issue], list[str]]:
    """Run per-row semantic checks.

    Returns (hard_issues, soft_lines):
      hard_issues — HARD violations (exit 1)
      soft_lines  — SOFT within-window FAILED carry-over lines (exit 0, stdout)
    """
    hard_issues: list[Issue] = []
    soft_lines: list[str] = []

    # --- Pending table checks ---
    for row in rows_by_table.get("pending", []):
        cells = row.cells
        label = _row_label(cells, "pending")
        schema = SCHEMA["pending"]
        required = schema["columns"]

        # (c) Date surfaced parses as ISO.
        date_cell = cells[0]
        date_val = _parse_iso_date(date_cell)
        if date_val is None:
            hard_issues.append(Issue(
                issue="schema-bad-date",
                row_label=label,
                observed=f"Date surfaced: {date_cell!r}",
                expected="ISO YYYY-MM-DD",
                action="fix the Date surfaced cell to an ISO date",
            ))

        # (b) Status enum check (index 4).
        status_idx = required.index("Status")
        status_raw = cells[status_idx] if len(cells) > status_idx else ""
        status_norm = normalize_cell(status_raw)
        # First whitespace-delimited token, case-fold for comparison.
        status_token = status_norm.split()[0] if status_norm.split() else ""

        # Map to canonical (FAILED is upper, others lower).
        canonical_map = {s.lower(): s for s in CANONICAL_STATUS}
        canonical_status = canonical_map.get(status_token.lower())

        if canonical_status is None:
            hard_issues.append(Issue(
                issue="schema-status-enum",
                row_label=label,
                observed=f'status: "{status_raw}"',
                expected="one of {pending, FAILED, done, cancelled}",
                action="normalize the Status cell to a canonical enum value",
            ))
            continue  # can't do stale/citation checks without a valid status

        if canonical_status == "FAILED":
            # (e) Stale-FAILED check (uses Date surfaced as the staleness clock).
            if date_val is not None:
                age_days = (today - date_val).days
                escalates_date = date_val + datetime.timedelta(days=STALE_FAILED_DAYS)

                if age_days < 0:
                    hard_issues.append(Issue(
                        issue="schema-future-date",
                        row_label=label,
                        observed=f"Date surfaced {date_val} is in the future relative to today {today}",
                        expected="Date surfaced must not be in the future",
                        action="correct the Date surfaced cell",
                    ))
                elif age_days >= STALE_FAILED_DAYS:
                    hard_issues.append(Issue(
                        issue="stale-failed",
                        row_label=label,
                        observed=f"FAILED, surfaced {date_val} ({age_days} days old)",
                        expected=f"resolve or cancel within {STALE_FAILED_DAYS} days",
                        action="escalate to analyst OR mark cancelled",
                    ))
                else:
                    # Within window — soft carry-over.
                    soft_lines.append(
                        f"- {label} — FAILED, surfaced {date_val} "
                        f"({age_days} day{'s' if age_days != 1 else ''} old; "
                        f"escalates {escalates_date})"
                    )

            # (f) Q-LED-NOTE: FAILED rows MUST cite a spec/dev-notes/*.md in Notes.
            notes_idx = required.index("Notes")
            notes_raw = cells[notes_idx] if len(cells) > notes_idx else ""
            if not _DEVNOTE_RE.search(notes_raw):
                hard_issues.append(Issue(
                    issue="missing-devnote-citation",
                    row_label=label,
                    observed=f"FAILED, Notes has no spec/dev-notes/*.md path",
                    expected="a follow-up dev-note path like spec/dev-notes/foo.md",
                    action="add investigation dev-note link to Notes cell",
                ))

    # --- Done table checks: completion date required ---
    for row in rows_by_table.get("done", []):
        cells = row.cells
        label = _row_label(cells, "done")
        schema = SCHEMA["done"]
        required = schema["columns"]

        completed_idx = required.index("Completed")
        completed_raw = cells[completed_idx] if len(cells) > completed_idx else ""
        completed_norm = normalize_cell(completed_raw)
        if not completed_norm:
            hard_issues.append(Issue(
                issue="missing-completion-date",
                row_label=label,
                observed="Completed cell empty",
                expected="ISO YYYY-MM-DD",
                action="fill the Completed cell with the completion date",
            ))
        else:
            # Try to parse as ISO date.
            completed_date = _parse_iso_date(completed_raw)
            if completed_date is None:
                hard_issues.append(Issue(
                    issue="missing-completion-date",
                    row_label=label,
                    observed=f"Completed cell not a parseable ISO date: {completed_raw!r}",
                    expected="ISO YYYY-MM-DD",
                    action="fix the Completed cell to an ISO date",
                ))

    # --- Cancelled table checks: cancel date required ---
    for row in rows_by_table.get("cancelled", []):
        cells = row.cells
        label = _row_label(cells, "cancelled")
        schema = SCHEMA["cancelled"]
        required = schema["columns"]

        cancelled_idx = required.index("Cancelled")
        cancelled_raw = cells[cancelled_idx] if len(cells) > cancelled_idx else ""
        cancelled_norm = normalize_cell(cancelled_raw)
        if not cancelled_norm:
            hard_issues.append(Issue(
                issue="missing-cancel-date",
                row_label=label,
                observed="Cancelled cell empty",
                expected="ISO YYYY-MM-DD",
                action="fill the Cancelled cell with the cancellation date",
            ))
        else:
            cancelled_date = _parse_iso_date(cancelled_raw)
            if cancelled_date is None:
                hard_issues.append(Issue(
                    issue="missing-cancel-date",
                    row_label=label,
                    observed=f"Cancelled cell not a parseable ISO date: {cancelled_raw!r}",
                    expected="ISO YYYY-MM-DD",
                    action="fix the Cancelled cell to an ISO date",
                ))

    return hard_issues, soft_lines


# ---------------------------------------------------------------------------
# Output formatting (D-LED-6 / bundle Q-HYG-EMIT dialect)
# ---------------------------------------------------------------------------


def format_hard_table(issues: list[Issue]) -> str:
    """Render HARD issues as a markdown table per D-LED-6."""
    n = len(issues)
    plural = "s" if n != 1 else ""
    header = f"operator-ledger-check: {n} issue{plural} detected"
    table_header = "| issue | row | observed | expected | action |"
    table_sep    = "|-------|-----|----------|----------|--------|"
    rows = [
        f"| {iss.issue} | {iss.row_label} | {iss.observed} | {iss.expected} | {iss.action} |"
        for iss in issues
    ]
    return "\n".join([header, table_header, table_sep] + rows)


def format_soft_block(soft_lines: list[str]) -> str:
    """Render SOFT within-window FAILED lines per D-LED-6 (stdout)."""
    n = len(soft_lines)
    plural = "s" if n != 1 else ""
    header = f"operator-ledger-check: {n} carry-over{plural} (within {STALE_FAILED_DAYS}-day window, not escalated)"
    return "\n".join([header] + soft_lines)


# ---------------------------------------------------------------------------
# Core check function
# ---------------------------------------------------------------------------


def run_check(
    ledger_path: Path,
    today: datetime.date,
) -> tuple[int, list[Issue], list[str]]:
    """Run the full ledger lint.

    Returns (exit_code, hard_issues, soft_lines).
    exit_code: 0 (clean / soft-only), 1 (HARD violations), >= 2 (script failure).
    """
    if not ledger_path.exists():
        print(
            f"error: ledger not found: {ledger_path}",
            file=sys.stderr,
        )
        return 2, [], []

    try:
        text = ledger_path.read_text(encoding="utf-8", errors="replace")
    except OSError as e:
        print(
            f"error: cannot read {ledger_path}: {e}",
            file=sys.stderr,
        )
        return 2, [], []

    rows_by_table, structural_issues = parse_ledger(text)

    hard_issues, soft_lines = check_rows(rows_by_table, today)
    all_hard = structural_issues + hard_issues

    if all_hard:
        return 1, all_hard, soft_lines
    return 0, [], soft_lines


# ---------------------------------------------------------------------------
# Self-test (D-LED-7 — 8 cases)
# ---------------------------------------------------------------------------

_BUG64_DONE_ROW = (
    "| 2026-05-28 | **Bug #64 D.1.1 cold-cache Yahoo+Run** — cockpit Lab → Yahoo → SOL → Run;"
    " confirm ticking label + working Stop + no panic | ~5 min × 3 verify rounds | 2026-05-29"
    " | **CLOSED — operator confirmed \"it works\" 2026-05-29.** Took 3 reactor-context"
    " recurrences to fully fix: (1) runner.rs:744 ticker — `rt.enter()` guard (attempt-3"
    " `a87b5fa`); (2) runner.rs:395 `tokio::time::timeout` in fetch_with_backoff —"
    " guard-construct-drop (hotfix `61abef6`); (3) hyper-util DNS resolver `spawn_blocking`"
    " at dns.rs:119 — **durable fix: rt.spawn() the whole preload onto the tokio runtime**"
    " (`0298edb`). Architect Q1 \"reqwest carries its own reactor\" assertion was falsified"
    " twice; ADR-0050 D1 rewritten to spawn-don't-guard invariant. R2 Stop fixed via"
    " CancellationToken + fetch_join.abort(). Regression-guard hardening"
    " (production-call-through test) in flight to close tester INCONCLUSIVE. NOTE: Last30d"
    " won't have data in the 2026-system-clock environment (Yahoo has no future-dated bars);"
    " use 2024 ranges for real testing. |"
)


def _make_ledger(
    pending_rows: list[str] | None = None,
    done_rows: list[str] | None = None,
    cancelled_rows: list[str] | None = None,
) -> str:
    """Build a minimal test fixture ledger string."""
    pending_body = "\n".join(pending_rows) if pending_rows else ""
    done_body = "\n".join(done_rows) if done_rows else ""
    cancelled_body = "\n".join(cancelled_rows) if cancelled_rows else ""
    return (
        "---\nslug: test-ledger\n---\n\n"
        "## Pending recipes\n\n"
        "| Date surfaced | Recipe | Cost | Unblocks | Status | Notes |\n"
        "|---|---|---|---|---|---|\n"
        + (pending_body + "\n" if pending_body else "")
        + "## Done recipes (audit trail)\n\n"
        "| Date surfaced | Recipe | Cost | Completed | Outcome |\n"
        "|---|---|---|---|---|\n"
        + (done_body + "\n" if done_body else "")
        + "## Cancelled recipes (audit trail)\n\n"
        "| Date surfaced | Recipe | Cost | Cancelled | Reason |\n"
        "|---|---|---|---|---|\n"
        + (cancelled_body + "\n" if cancelled_body else "")
    )


def run_self_test() -> None:
    """8-case inline self-test (D-LED-7). Exits 0 all-pass / 1 any-fail."""
    failures: list[str] = []
    today = datetime.date(2026, 5, 29)  # fixed reference date for determinism

    def assert_eq(label: str, got: object, expected: object) -> None:
        if got != expected:
            failures.append(f"  FAIL [{label}]: got {got!r}, expected {expected!r}")

    def assert_contains(label: str, haystack: str, needle: str) -> None:
        if needle not in haystack:
            failures.append(f"  FAIL [{label}]: {needle!r} not in output {haystack!r}")

    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)

        # --- Case 1: clean (empty Pending, valid Done row) ---
        ledger1 = _make_ledger(
            done_rows=["| 2026-05-27 | Cache populate BTC | ~3 min | 2026-05-27 | done |"],
        )
        f1 = tmp / "c1.md"
        f1.write_text(ledger1, encoding="utf-8")
        code1, hard1, soft1 = run_check(f1, today)
        assert_eq("C1-exit", code1, 0)
        assert_eq("C1-hard-count", len(hard1), 0)
        assert_eq("C1-soft-count", len(soft1), 0)

        # --- Case 2: schema-status-enum violation ---
        ledger2 = _make_ledger(
            pending_rows=["| 2026-05-28 | Foo recipe | ~5 min | nothing | blocked | some notes |"],
            done_rows=["| 2026-05-27 | Cache populate BTC | ~3 min | 2026-05-27 | done |"],
        )
        f2 = tmp / "c2.md"
        f2.write_text(ledger2, encoding="utf-8")
        code2, hard2, soft2 = run_check(f2, today)
        assert_eq("C2-exit", code2, 1)
        classes2 = [i.issue for i in hard2]
        assert_contains("C2-class", str(classes2), "schema-status-enum")

        # --- Case 3: stale-FAILED fires at 8 days ---
        # today=2026-05-29, surfaced=2026-05-21 (8 days old), has valid citation
        ledger3 = _make_ledger(
            pending_rows=[
                "| 2026-05-21 | Stale recipe | ~5 min | nothing | **FAILED** |"
                " See spec/dev-notes/bug-64-d11-attempt-3-investigation-2026-05-29.md |"
            ],
            done_rows=["| 2026-05-27 | Cache populate BTC | ~3 min | 2026-05-27 | done |"],
        )
        f3 = tmp / "c3.md"
        f3.write_text(ledger3, encoding="utf-8")
        code3, hard3, soft3 = run_check(f3, today)
        assert_eq("C3-exit", code3, 1)
        classes3 = [i.issue for i in hard3]
        assert_contains("C3-class", str(classes3), "stale-failed")

        # --- Case 4: NOT stale at 1 day (within window) → exit 0, soft line ---
        # today=2026-05-29, surfaced=2026-05-28 (1 day old), has valid citation
        ledger4 = _make_ledger(
            pending_rows=[
                "| 2026-05-28 | Recent recipe | ~5 min | nothing | FAILED |"
                " See spec/dev-notes/bug-64-d11-attempt-3-investigation-2026-05-29.md |"
            ],
            done_rows=["| 2026-05-27 | Cache populate BTC | ~3 min | 2026-05-27 | done |"],
        )
        f4 = tmp / "c4.md"
        f4.write_text(ledger4, encoding="utf-8")
        code4, hard4, soft4 = run_check(f4, today)
        assert_eq("C4-exit", code4, 0)
        assert_eq("C4-hard-count", len(hard4), 0)
        # Soft carry-over line should be present.
        if not soft4:
            failures.append("  FAIL [C4-soft]: expected soft carry-over line, got none")

        # --- Case 5: missing-completion-date in Done table ---
        ledger5 = _make_ledger(
            done_rows=["| 2026-05-27 | Cache populate BTC | ~3 min |  | done |"],
        )
        f5 = tmp / "c5.md"
        f5.write_text(ledger5, encoding="utf-8")
        code5, hard5, soft5 = run_check(f5, today)
        assert_eq("C5-exit", code5, 1)
        classes5 = [i.issue for i in hard5]
        assert_contains("C5-class", str(classes5), "missing-completion-date")

        # --- Case 6: missing-devnote-citation (FAILED, 1-day-old, NO spec/dev-notes path) ---
        ledger6 = _make_ledger(
            pending_rows=[
                "| 2026-05-28 | Uncited recipe | ~5 min | nothing | FAILED | TODO investigate |"
            ],
            done_rows=["| 2026-05-27 | Cache populate BTC | ~3 min | 2026-05-27 | done |"],
        )
        f6 = tmp / "c6.md"
        f6.write_text(ledger6, encoding="utf-8")
        code6, hard6, soft6 = run_check(f6, today)
        assert_eq("C6-exit", code6, 1)
        classes6 = [i.issue for i in hard6]
        assert_contains("C6-class", str(classes6), "missing-devnote-citation")
        # stale check should NOT fire (1-day-old is within window).
        if "stale-failed" in classes6:
            failures.append("  FAIL [C6-no-stale]: stale-failed fired unexpectedly on 1-day-old row")

        # --- Case 7: cancelled-table-exclusion ---
        # Cancelled row with valid Cancelled date — exit 0 (no completion-date required).
        ledger7 = _make_ledger(
            cancelled_rows=["| 2026-05-27 | Cancelled recipe | ~5 min | 2026-05-27 | No longer needed |"],
        )
        f7 = tmp / "c7.md"
        f7.write_text(ledger7, encoding="utf-8")
        code7, hard7, soft7 = run_check(f7, today)
        assert_eq("C7-exit", code7, 0)
        assert_eq("C7-hard-count", len(hard7), 0)

        # --- Case 8: Bug #64 D.1.1 verbatim Done row regression ---
        # The real line-36 Done row — complex Notes with embedded links/bold/backticks.
        # Done table has no Status/citation requirement; must parse clean.
        ledger8 = _make_ledger(
            done_rows=[_BUG64_DONE_ROW],
        )
        f8 = tmp / "c8.md"
        f8.write_text(ledger8, encoding="utf-8")
        code8, hard8, soft8 = run_check(f8, today)
        assert_eq("C8-exit", code8, 0)
        assert_eq("C8-hard-count", len(hard8), 0)

    if failures:
        print("operator-ledger-check --self-test: FAILED", file=sys.stderr)
        for f in failures:
            print(f, file=sys.stderr)
        raise SystemExit(1)
    else:
        print("self-test: 8 passed")


# ---------------------------------------------------------------------------
# Argument parser
# ---------------------------------------------------------------------------


def build_arg_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="operator_ledger_check.py",
        description="Orchestrator pre-flight: lint the operator-side pending ledger schema.",
    )
    parser.add_argument(
        "--today",
        metavar="YYYY-MM-DD",
        default=None,
        help=(
            "Override today's date for stale-FAILED age computation (required for"
            " deterministic self-test + P-LED-1 probe). Default: datetime.date.today()."
        ),
    )
    parser.add_argument(
        "--ledger",
        metavar="PATH",
        default=None,
        help=(
            "Override the ledger path. Default: REPO_ROOT/spec/dev-notes/operator-side-pending-ledger.md"
        ),
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run the 8-case inline self-test suite; exit 0 all-pass / 1 any-fail.",
    )
    return parser


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    parser = build_arg_parser()
    args = parser.parse_args(argv)

    if args.self_test:
        run_self_test()
        return 0  # run_self_test raises SystemExit on failure.

    # Resolve --today.
    if args.today is not None:
        try:
            today = datetime.date.fromisoformat(args.today)
        except ValueError:
            print(
                f"error: --today must be ISO YYYY-MM-DD, got: {args.today!r}",
                file=sys.stderr,
            )
            return 2
    else:
        today = datetime.date.today()

    # Resolve ledger path.
    if args.ledger is not None:
        ledger_path = Path(args.ledger)
    else:
        ledger_path = REPO_ROOT / "spec" / "dev-notes" / "operator-side-pending-ledger.md"

    exit_code, hard_issues, soft_lines = run_check(ledger_path, today)

    if exit_code >= 2:
        return exit_code

    # Emit soft carry-over lines to stdout (exit 0).
    if soft_lines:
        print(format_soft_block(soft_lines))

    # Emit hard issues to stderr and exit 1.
    if hard_issues:
        print(format_hard_table(hard_issues), file=sys.stderr)
        return 1

    # Clean — silent success (R2.3).
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
