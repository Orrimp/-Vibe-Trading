#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""spec_brief.py — assemble a per-feature briefing pack for sub-agents.

Re-founded 2026-07-25 (BMAD-migration Phase 5b `spec/` retirement). Goal
unchanged: keep sub-agent context windows small. Instead of having a
developer or architect grep the 4700+ line BMAD architecture spine, give
them a curated brief assembled from the STORY (the BMAD-native per-feature
record — was: feature.md + tasks.md, now merged into one file), the
architecture spine + PRD, and the evidence corpus.

Output is a single markdown document containing:
  1. The CLAUDE.md non-negotiables (always).
  2. The story file in full (Status, Acceptance Criteria, Tasks/Subtasks,
     Dev Notes, References — was: feature.md + tasks.md, now one artifact).
  3. Trace.toml rows that mention this feature (when trace.toml exists).
  4. The most recent test report for this feature (when present).
  5. Architecture-spine sections that mention this slug (best-effort grep).

The brief is written to stdout by default, or to --out <path>. Token
budget is reported on stderr so callers can verify they're under
~5k tokens (rough heuristic: chars / 4).

Usage:
    scripts/spec_brief.py chart-canvas-overhaul
    scripts/spec_brief.py chart-canvas-overhaul --out /tmp/brief.md
    scripts/spec_brief.py --list   # list valid slugs
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

import tomllib  # Python 3.11+ (enforced by PEP-723 header above)

REPO_ROOT = Path(__file__).resolve().parent.parent
# `spec/` retired 2026-07-25 (BMAD-migration Phase 5b) — see
# docs/dev-notes/bmad-migration-plan-2026-07-24.md § Phase 5b. The BMAD-native
# homes:
EVIDENCE_DIR = REPO_ROOT / "evidence"
BMAD_OUTPUT_DIR = REPO_ROOT / "_bmad-output"
PLANNING_DIR = BMAD_OUTPUT_DIR / "planning-artifacts"
STORY_DIR = BMAD_OUTPUT_DIR / "implementation-artifacts"
CLAUDE_MD = REPO_ROOT / "CLAUDE.md"
ARCHITECTURE_MD = PLANNING_DIR / "architecture.md"
TRACE_TOML = PLANNING_DIR / "trace.toml"
ANCHORS_TOML = EVIDENCE_DIR / "anchors.toml"

# How many lines of architecture to include around each match. Keep small
# because the brief should not exceed ~5k tokens / ~20k chars.
ARCH_CONTEXT_LINES = 30
ARCH_MAX_MATCHES = 8

STORY_FILENAME_RE = re.compile(r"^\d+-\d+-(.+)\.md$")
SOURCE_FOLDER_RE = re.compile(r"Source feature folder:\s*`spec/([^`]+?)/?`")


def list_slugs() -> list[str]:
    """List every resolvable slug: the story-filename form primarily, PLUS
    the original nested slug (e.g. `phase-1-foundation` for lumen sub-phases)
    recovered from the "Source feature folder:" Dev Notes line, so either
    spelling works as a lookup key."""
    slugs: set[str] = set()
    if not STORY_DIR.is_dir():
        return []
    for p in sorted(STORY_DIR.glob("*.md")):
        if p.name == "sprint-status.yaml":
            continue
        m = STORY_FILENAME_RE.match(p.name)
        if not m:
            continue
        slugs.add(m.group(1))
        text = p.read_text(encoding="utf-8", errors="replace")
        sm = SOURCE_FOLDER_RE.search(text)
        if sm:
            relpath = sm.group(1).rstrip("/")
            parts = relpath.split("/")
            if parts and parts[0] in ("v1", "v2", "v3"):
                parts = parts[1:]
            if parts:
                slugs.add(parts[-1])
    return sorted(slugs)


def find_story(slug: str) -> Path | None:
    """Resolve `slug` to a story file. Tries, in order: an exact filename
    suffix match (`*-<slug>.md` — handles the common case directly); then a
    scan for a story whose "Source feature folder:" line's FINAL path segment
    equals `slug` (handles nested slugs like lumen sub-phases, whose story
    filename carries a disambiguating `lumen-` prefix the bare original slug
    does not)."""
    if not STORY_DIR.is_dir():
        return None
    matches = sorted(STORY_DIR.glob(f"*-{slug}.md"))
    if matches:
        return matches[0]
    for p in sorted(STORY_DIR.glob("*.md")):
        text = p.read_text(encoding="utf-8", errors="replace")
        sm = SOURCE_FOLDER_RE.search(text)
        if not sm:
            continue
        relpath = sm.group(1).rstrip("/")
        parts = relpath.split("/")
        if parts and parts[0] in ("v1", "v2", "v3"):
            parts = parts[1:]
        if parts and parts[-1] == slug:
            return p
    return None


def extract_non_negotiables(claude_md_text: str) -> str:
    """Pull the 'Non-negotiables' section from CLAUDE.md if present, else a fallback."""
    m = re.search(
        r"^##\s+Non-negotiables.*?(?=^##\s|\Z)",
        claude_md_text,
        re.DOTALL | re.MULTILINE,
    )
    return m.group(0).strip() if m else "(no Non-negotiables section found in CLAUDE.md)"


def latest_test_report(slug: str) -> Path | None:
    """`reports/` mirrors the ORIGINAL `spec/`-relative path 1:1 under
    EVIDENCE_DIR (Phase 3 base-swap). Try the bare slug first, then each
    v1/v2/v3 container, then the `lumen-design-adoption/<slug>` nesting."""
    candidates = [EVIDENCE_DIR / slug]
    for prefix in ("v1", "v2", "v3"):
        candidates.append(EVIDENCE_DIR / prefix / slug)
    candidates.append(EVIDENCE_DIR / "lumen-design-adoption" / slug)
    for base in candidates:
        reports = base / "reports"
        if reports.exists():
            found = sorted(reports.glob("test-*.md"))
            if found:
                return found[-1]
    return None


def trace_rows_for(slug: str) -> list[dict]:
    if not TRACE_TOML.exists():
        return []
    with TRACE_TOML.open("rb") as f:
        data = tomllib.load(f)
    out = []
    for row in data.get("req", []):
        feat = row.get("feature")
        if feat == slug or (isinstance(feat, list) and slug in feat):
            out.append(row)
    return out


def anchor_rows() -> list[dict]:
    if not ANCHORS_TOML.exists():
        return []
    with ANCHORS_TOML.open("rb") as f:
        data = tomllib.load(f)
    return data.get("anchors", [])


def architecture_excerpts(slug: str) -> list[tuple[int, str]]:
    """Return up to ARCH_MAX_MATCHES windows of the architecture spine
    mentioning the slug. Each window is (start_line_1indexed, text)."""
    if not ARCHITECTURE_MD.exists():
        return []
    lines = ARCHITECTURE_MD.read_text(encoding="utf-8", errors="replace").splitlines()
    pat = re.compile(re.escape(slug), re.IGNORECASE)
    matches: list[int] = [i for i, line in enumerate(lines) if pat.search(line)]
    windowed: list[tuple[int, int]] = []
    for i in matches:
        start = max(0, i - ARCH_CONTEXT_LINES // 2)
        end = min(len(lines), i + ARCH_CONTEXT_LINES // 2)
        if windowed and start <= windowed[-1][1]:
            windowed[-1] = (windowed[-1][0], max(windowed[-1][1], end))
        else:
            windowed.append((start, end))
        if len(windowed) >= ARCH_MAX_MATCHES:
            break
    return [(start + 1, "\n".join(lines[start:end])) for start, end in windowed]


def render_brief(slug: str) -> str:
    story_path = find_story(slug)
    if story_path is None:
        raise SystemExit(
            f"error: no story found for slug {slug!r} under "
            f"{STORY_DIR.relative_to(REPO_ROOT)}/ (tried '*-{slug}.md' and the "
            f"'Source feature folder:' nested-slug fallback)"
        )

    parts: list[str] = []
    parts.append(f"# Brief: {slug}\n")
    parts.append(
        "_Generated by scripts/spec_brief.py. "
        "Use this brief as the primary context for your work on this feature. "
        "Open the full story / architecture spine only if the brief leaves a "
        "question unanswered._\n"
    )

    # 1. Non-negotiables
    parts.append("## Non-negotiables (from CLAUDE.md)\n")
    if CLAUDE_MD.exists():
        parts.append(extract_non_negotiables(CLAUDE_MD.read_text()))
    else:
        parts.append("(CLAUDE.md not found)")
    parts.append("")

    # 2. Story (merges what used to be feature.md + tasks.md)
    parts.append("## Story (Status, Acceptance Criteria, Tasks/Subtasks, Dev Notes)\n")
    parts.append(f"_Source: `{story_path.relative_to(REPO_ROOT)}`_\n")
    parts.append(story_path.read_text(encoding="utf-8", errors="replace"))
    parts.append("")

    # 3. Trace rows
    trace_rows = trace_rows_for(slug)
    if trace_rows:
        parts.append("## Traceability rows referencing this feature\n")
        parts.append(f"_Source: `{TRACE_TOML.relative_to(REPO_ROOT)}`_\n")
        for row in trace_rows:
            parts.append("```toml")
            parts.append("[[req]]")
            for k, v in row.items():
                if isinstance(v, list):
                    items = ", ".join(repr(x) for x in v)
                    parts.append(f"{k} = [{items}]")
                else:
                    parts.append(f"{k} = {v!r}")
            parts.append("```")
            parts.append("")

    # 4. Latest test report
    last_test = latest_test_report(slug)
    if last_test:
        parts.append("## Most recent test report (head)\n")
        parts.append(f"_Source: `{last_test.relative_to(REPO_ROOT)}`_\n")
        head = "\n".join(last_test.read_text().splitlines()[:80])
        parts.append("```markdown")
        parts.append(head)
        parts.append("```")
        parts.append("")

    # 5. Anchors for this slug (best-effort: scenario names often share a prefix)
    anchors = anchor_rows()
    if anchors:
        parts.append("## Backtest anchors (full set — locate yours by scenario name)\n")
        parts.append(f"_Source: `{ANCHORS_TOML.relative_to(REPO_ROOT)}`_\n")
        parts.append("| scenario | version | sha256 |")
        parts.append("|----------|---------|--------|")
        for a in anchors:
            sha = a.get("sha256", "")[:12]
            parts.append(f"| {a.get('scenario', '?')} | {a.get('version', '?')} | `{sha}…` |")
        parts.append("")

    # 6. Architecture excerpts
    excerpts = architecture_excerpts(slug)
    if excerpts:
        parts.append("## Architecture-spine excerpts mentioning this slug\n")
        parts.append(
            f"_Source: `{ARCHITECTURE_MD.relative_to(REPO_ROOT)}` "
            f"({len(excerpts)} windows, ~{ARCH_CONTEXT_LINES} lines each). "
            "If you need more, grep the file directly._\n"
        )
        for start_line, text in excerpts:
            parts.append(f"### Around line {start_line}")
            parts.append("```markdown")
            parts.append(text)
            parts.append("```")
            parts.append("")

    return "\n".join(parts)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("slug", nargs="?", help="feature slug (matches a story filename suffix)")
    parser.add_argument("--list", action="store_true", help="list valid slugs and exit")
    parser.add_argument("--out", type=Path, help="write to file instead of stdout")
    args = parser.parse_args(argv)

    if args.list:
        for s in list_slugs():
            print(s)
        return 0

    if not args.slug:
        parser.print_usage(sys.stderr)
        print("error: slug required (or use --list)", file=sys.stderr)
        return 2

    brief = render_brief(args.slug)
    if args.out:
        args.out.write_text(brief)
        print(f"wrote {args.out}", file=sys.stderr)
    else:
        sys.stdout.write(brief)

    char_count = len(brief)
    tok_est = char_count // 4
    print(
        f"brief: {char_count} chars (~{tok_est} tokens)",
        file=sys.stderr,
    )
    # Soft budget: a brief over ~10k tokens means the feature itself is too big
    # and should probably be split. Print a loud warning so the orchestrator
    # can decide whether to delegate as-is or compress further.
    if tok_est > 10_000:
        print(
            f"warning: brief exceeds 10k-token soft budget ({tok_est} tokens). "
            "consider splitting the story or filing a spec-auditor task.",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
