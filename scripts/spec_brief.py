#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""spec_brief.py — assemble a per-feature briefing pack for sub-agents.

Goal: keep sub-agent context windows small. Instead of having a developer
or architect grep the 296 KB architecture.md, give them a curated brief.

Output is a single markdown document containing:
  1. The CLAUDE.md non-negotiables (always).
  2. The feature.md frontmatter + body.
  3. The tasks.md.
  4. Trace.toml rows that mention this feature (when trace.toml exists).
  5. The most recent test report for this feature (when present).
  6. Architecture sections that mention this slug (best-effort grep).

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
SPEC_DIR = REPO_ROOT / "spec"
CLAUDE_MD = REPO_ROOT / "CLAUDE.md"
ARCHITECTURE_MD = SPEC_DIR / "architecture.md"
TRACE_TOML = SPEC_DIR / "trace.toml"
ANCHORS_TOML = SPEC_DIR / "anchors.toml"

NON_FEATURE = {"design", "dev-notes", "runbooks", "archive", "architecture", "v1", "v2", "v3"}

# How many lines of architecture to include around each match. Keep small
# because the brief should not exceed ~5k tokens / ~20k chars.
ARCH_CONTEXT_LINES = 30
ARCH_MAX_MATCHES = 8


def list_slugs() -> list[str]:
    slugs = []
    # Feature folders live at spec/ root AND under spec/v1/ + spec/v2/ (2026-06-28 reorg).
    dirs = list(SPEC_DIR.iterdir())
    for container in ("v1", "v2", "v3"):
        sub = SPEC_DIR / container
        if sub.is_dir():
            dirs.extend(sub.iterdir())
    for p in sorted(dirs):
        if p.is_dir() and p.name not in NON_FEATURE and not p.name.startswith("."):
            slugs.append(p.name)
    return slugs


def extract_non_negotiables(claude_md_text: str) -> str:
    """Pull the 'Non-negotiables' section from CLAUDE.md if present, else a fallback."""
    m = re.search(
        r"^##\s+Non-negotiables.*?(?=^##\s|\Z)",
        claude_md_text,
        re.DOTALL | re.MULTILINE,
    )
    return m.group(0).strip() if m else "(no Non-negotiables section found in CLAUDE.md)"


def latest_test_report(feature_dir: Path) -> Path | None:
    reports = feature_dir / "reports"
    if not reports.exists():
        return None
    candidates = sorted(reports.glob("test-*.md"))
    return candidates[-1] if candidates else None


def architecture_excerpts(slug: str) -> list[tuple[int, str]]:
    """Return up to ARCH_MAX_MATCHES windows of architecture.md mentioning the slug.

    Each window is (start_line_1indexed, text). Best-effort: this is a
    stopgap until architecture.md is split into spec/architecture/*.md.
    """
    if not ARCHITECTURE_MD.exists():
        return []
    lines = ARCHITECTURE_MD.read_text(encoding="utf-8", errors="replace").splitlines()
    pat = re.compile(re.escape(slug), re.IGNORECASE)
    matches: list[int] = [i for i, line in enumerate(lines) if pat.search(line)]
    # De-duplicate matches that fall within the same window.
    windowed: list[tuple[int, int]] = []
    for i in matches:
        start = max(0, i - ARCH_CONTEXT_LINES // 2)
        end = min(len(lines), i + ARCH_CONTEXT_LINES // 2)
        if windowed and start <= windowed[-1][1]:
            # merge
            windowed[-1] = (windowed[-1][0], max(windowed[-1][1], end))
        else:
            windowed.append((start, end))
        if len(windowed) >= ARCH_MAX_MATCHES:
            break
    return [(start + 1, "\n".join(lines[start:end])) for start, end in windowed]


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


def render_brief(slug: str) -> str:
    # Feature folders may live at spec/<slug>, spec/v1/<slug>, or spec/v2/<slug> (2026-06-28 reorg).
    feature_dir = next(
        ((SPEC_DIR / prefix / slug) for prefix in ("", "v1", "v2", "v3")
         if (SPEC_DIR / prefix / slug).exists()),
        SPEC_DIR / slug,
    )
    if not feature_dir.exists():
        raise SystemExit(f"error: feature folder not found: {feature_dir}")

    feature_md = feature_dir / "feature.md"
    tasks_md = feature_dir / "tasks.md"

    parts: list[str] = []
    parts.append(f"# Brief: {slug}\n")
    parts.append(
        "_Generated by scripts/spec_brief.py. "
        "Use this brief as the primary context for your work on this feature. "
        "Open the full spec only if the brief leaves a question unanswered._\n"
    )

    # 1. Non-negotiables
    parts.append("## Non-negotiables (from CLAUDE.md)\n")
    if CLAUDE_MD.exists():
        parts.append(extract_non_negotiables(CLAUDE_MD.read_text()))
    else:
        parts.append("(CLAUDE.md not found)")
    parts.append("")

    # 2. Feature.md
    parts.append("## Feature spec\n")
    parts.append(f"_Source: `{feature_md.relative_to(REPO_ROOT)}`_\n")
    if feature_md.exists():
        parts.append(feature_md.read_text())
    else:
        parts.append("(missing feature.md — orphan folder)")
    parts.append("")

    # 3. Tasks.md
    parts.append("## Task list\n")
    parts.append(f"_Source: `{tasks_md.relative_to(REPO_ROOT)}`_\n")
    if tasks_md.exists():
        parts.append(tasks_md.read_text())
    else:
        parts.append("(missing tasks.md — orphan folder)")
    parts.append("")

    # 4. Trace rows
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

    # 5. Latest test report
    last_test = latest_test_report(feature_dir)
    if last_test:
        parts.append("## Most recent test report (head)\n")
        parts.append(f"_Source: `{last_test.relative_to(REPO_ROOT)}`_\n")
        head = "\n".join(last_test.read_text().splitlines()[:80])
        parts.append("```markdown")
        parts.append(head)
        parts.append("```")
        parts.append("")

    # 6. Anchors for this slug (best-effort: scenario names often share a prefix)
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

    # 7. Architecture excerpts
    excerpts = architecture_excerpts(slug)
    if excerpts:
        parts.append("## Architecture excerpts mentioning this slug\n")
        parts.append(
            f"_Source: `{ARCHITECTURE_MD.relative_to(REPO_ROOT)}` "
            f"({len(excerpts)} windows, ~{ARCH_CONTEXT_LINES} lines each). "
            "If you need more, grep the file directly — but this is the path "
            "that becomes obsolete once architecture.md is split._\n"
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
    parser.add_argument("slug", nargs="?", help="feature slug (folder name under spec/)")
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
            "consider splitting feature.md or filing a spec-auditor task.",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
