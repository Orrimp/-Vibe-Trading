#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""spec_lint.py — structural integrity check for the BMAD-native layout.

Re-founded 2026-07-25 (BMAD-migration Phase 5b) onto the story/sprint-status
layout. `spec/` is RETIRED as of this phase — every check below walks
`docs/`, `evidence/`, and `_bmad-output/` instead. The governing ledger
(`trace.toml`) now lives at `_bmad-output/planning-artifacts/trace.toml`; the
per-feature record is a **story** at
`_bmad-output/implementation-artifacts/{epic}-{story}-{slug}.md` (`Status:`
line, not YAML frontmatter) instead of a `feature.md`. See
`docs/dev-notes/bmad-migration-plan-2026-07-24.md` § Phase 5b for the plan
this executes.

The ADR-0082 single-source-of-truth triad is preserved, re-keyed onto the new
artifacts:
  - `status-drift`                — story `Status:` line vs. its trace
    `[[req]]` row `state=` (full status-vocabulary mapping, not just the
    shipped/done terminal case — see `STATE_TO_STORY_STATUS`).
  - `story-done-trace-drift`      — the ADR-0082 terminal invariant: a story
    `Status: done` whose trace row `state=` is not itself a shipped-terminal
    value (`shipped`/`shipped-partial`), or has no `state=` at all.
  - `story-done-changelog-missing` — every `Status: done` story must be
    indexed in the root `CHANGELOG.md` (by slug / REQ-id / rollup allowlist).

The triad's join key is guarded by two rules of its own (bug-log `#216`),
because the first two legs SKIP a story whose key they cannot read, and a
skip reads as a pass:
  - `trace-ref-unparsed`          — a story with no column-0 `- Trace:` line,
    more than one, or one in neither accepted form (see below).
  - `trace-ref-dangling`          — a parsed REQ-id with no `[[req]]` row in
    `trace.toml`.
A third rule keeps the line's own copy of the row honest (bug-log `#219`):
  - `trace-state-annotation-drift` — the line's `(state=…)` annotation differs
    from its row's `state=`. The annotation is a copy; it moves with the row.

Story ↔ trace-row resolution is via the REQ-id embedded in the story's own
`### References` block (its `- Trace:` line; contract below), NOT by
slug-string matching — story filenames sanitize dots to hyphens
(`v0.2.0` → `v0-2-0`) and disambiguate nested slugs (lumen phases gain a
`lumen-` prefix), so filename-derived slugs do not always equal the
`trace.toml` `feature=` slug. The REQ-id is the load-bearing join key (the
"Phase-2 story↔REQ bijection" the migration plan cites); slug string
matching is used only as a fallback for the ~16 stories with no trace
citation at all (`- Trace: none — known trace-coverage gap`) and for the
`~18` iteration/follow-up trace rows that intentionally have NO dedicated
story (they fold as `Tasks/Subtasks` bullets under their base story per the
`CHANGELOG_ROLLUP_ALLOWLIST` convention — reused here for that fold too).

The `- Trace:` line contract: every story has EXACTLY ONE line starting at
column 0 with `- Trace:`, in one of two forms —
    - Trace: `REQ-XXX-001` (state=`shipped`)      (text may follow the `)`)
    - Trace: none — <reason>                       (the declared gap)
Anything else (a bare id, a missing or doubled line, a word between the
state's closing backtick and the `)`) is `trace-ref-unparsed`.

Exit code = number of violation CATEGORIES that triggered (0 = clean).
Pass --all to print every violation regardless of category count.

Usage:
    uv run scripts/spec_lint.py            # whole tree (preferred)
    uv run scripts/spec_lint.py --all      # verbose
    uv run scripts/spec_lint.py --self-test  # synthetic-fixture check of every
                                             # self-tested rule (exit 0 = ok)

System Python (3.11+) also works:
    python3 scripts/spec_lint.py
"""
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

import tomllib  # Python 3.11+ (enforced by PEP-723 header above)

REPO_ROOT = Path(__file__).resolve().parent.parent

# `spec/` retired 2026-07-25 (BMAD-migration Phase 5b). The three roots below
# are what this script walks now:
#   - EVIDENCE_DIR  — the byte-immutable reports/presentations corpus
#     (`git mv`d from `spec/` in Phase 3 + the Phase 5b `presentations/`
#     extraction). Anchors + the trace ledger's cited test evidence live here.
#   - DOCS_DIR      — BMAD project_knowledge (dev-notes/runbooks/design +
#     `docs/archive/` — the retired-content archive, itself frozen/skipped,
#     see `iter_spec_md`).
#   - BMAD_OUTPUT_DIR — the BMAD planning + implementation artifacts: PRD,
#     architecture spine, ADR annex, epics, trace.toml, and the per-feature
#     stories this script's triad keys on.
EVIDENCE_DIR = REPO_ROOT / "evidence"
DOCS_DIR = REPO_ROOT / "docs"
BMAD_OUTPUT_DIR = REPO_ROOT / "_bmad-output"
PLANNING_DIR = BMAD_OUTPUT_DIR / "planning-artifacts"
STORY_DIR = BMAD_OUTPUT_DIR / "implementation-artifacts"
TRACE_TOML = PLANNING_DIR / "trace.toml"
SPRINT_STATUS_YAML = STORY_DIR / "sprint-status.yaml"
CHANGELOG_PATH = REPO_ROOT / "CHANGELOG.md"

# ---------------------------------------------------------------------------
# Configuration: which frontmatter keys are required on which files.
# feature.md / tasks.md no longer exist outside the frozen archive (stories
# use a plain `Status:` line, not YAML frontmatter) so the old hard-required
# dict is gone; the BMAD planning docs get the same SOFT treatment
# product.md/architecture.md had (a missing `updated:` key warns, doesn't fail).
# ---------------------------------------------------------------------------

SOFT_FRONTMATTER: dict[str, set[str]] = {
    "PRD.md":          {"updated"},
    "architecture.md": {"updated"},
}

# Story `Status:` vocabulary (BMAD stock + the project's `retired` addition —
# see sprint-status.yaml's embedded STATUS DEFINITIONS + the Phase-2 retro
# mapping comment repeated in every story's Dev Notes).
VALID_STORY_STATUSES = {"backlog", "ready-for-dev", "in-progress", "review", "done", "retired"}

# Categories — used both for grouping output and computing exit code.
CATEGORIES = (
    "dead-link",
    "missing-frontmatter",
    "orphan-story",
    "bad-anchor",
    "unreferenced-anchor",
    "story-done-no-tests",
    "status-drift",
    "story-done-trace-drift",
    "story-done-changelog-missing",
    "trace-ref-unparsed",
    "trace-ref-dangling",
    "trace-state-annotation-drift",
    "trace-broken-path",
    "adr-not-registered",
    "dead-cargo-citation",
)


@dataclass
class Violation:
    category: str
    path: Path
    detail: str

    def render(self, root: Path) -> str:
        rel = self.path.relative_to(root) if self.path.is_absolute() else self.path
        return f"  [{self.category}] {rel}: {self.detail}"


@dataclass
class Report:
    violations: list[Violation] = field(default_factory=list)

    def add(self, category: str, path: Path, detail: str) -> None:
        self.violations.append(Violation(category, path, detail))

    def by_category(self) -> dict[str, list[Violation]]:
        out: dict[str, list[Violation]] = {c: [] for c in CATEGORIES}
        for v in self.violations:
            out.setdefault(v.category, []).append(v)
        return out


# ---------------------------------------------------------------------------
# Frontmatter parsing (YAML-lite, used for PRD.md/architecture.md only now)
# ---------------------------------------------------------------------------

FRONTMATTER_RE = re.compile(r"\A---\r?\n(.*?)\r?\n---\r?\n", re.DOTALL)


def parse_frontmatter(text: str) -> dict[str, str] | None:
    """Return a flat dict of YAML-style frontmatter keys, or None if absent."""
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
# Story parsing — the BMAD-native per-feature record.
#
# Shape (see any file under `_bmad-output/implementation-artifacts/`):
#   # Story {epic}.{story}: {slug}
#
#   Status: {backlog|ready-for-dev|in-progress|review|done|retired}
#   ...
#   ### References
#   - Trace: `REQ-XXX-001` (state=`shipped`)     <- or: - Trace: none — <reason>
#   ...
#   - Source feature folder: `spec/[v1/|v2/|v3/]<slug>/` - frontmatter status
#     **`shipped`** (verbatim), ...
# ---------------------------------------------------------------------------

STORY_FILENAME_RE = re.compile(r"^(\d+)-(\d+)-(.+)\.md$")
STATUS_LINE_RE = re.compile(r"^Status:\s*(\S+)", re.MULTILINE)
TRACE_REF_RE = re.compile(r"^- Trace:\s*`([^`]+)`\s*\(state=`([^`]*)`\)", re.MULTILINE)
# The declared-gap form, reason required (bug-log `#216`). Matched one LINE at a
# time by `_trace_line_problems`, never searched over a whole file.
TRACE_NONE_RE = re.compile(r"^- Trace:[ \t]*none[ \t]+—[ \t]*\S")
SOURCE_FOLDER_RE = re.compile(r"Source feature folder:\s*`spec/([^`]+?)/?`")

# The two accepted `- Trace:` forms, spelled out for violation messages.
TRACE_FORMS_HINT = (
    "``- Trace: `REQ-…` (state=`…`)`` (text may follow the `)`) or "
    "`- Trace: none — <reason>`"
)


@dataclass
class Story:
    path: Path
    epic: str
    story_num: str
    filename_slug: str  # derived straight from the filename; may be dot-sanitized/prefixed
    status: str | None
    trace_req_id: str | None       # REQ-id cited by "- Trace:" line, if any
    # The (state=`...`) annotation on that same line; must equal the row's
    # `state=` (trace-state-annotation-drift, bug-log #219).
    trace_embedded_state: str | None
    # Line number (1-based) of the line `trace_req_id` was read from, or None.
    trace_line_no: int | None = None
    # Every way this story's `- Trace:` lines break the one-line contract;
    # empty = the contract holds. Reported by `trace-ref-unparsed` (bug-log #216).
    trace_line_problems: list[str] = field(default_factory=list)


def _trace_line_problems(text: str) -> list[str]:
    """Classify every column-0 `- Trace:` line against the two accepted forms.

    bug-log `#216`: the triad's join key comes from ONE `TRACE_REF_RE.search`
    over the whole file, and a failure to parse folds into the same `None` as
    the declared gap. The downstream rules read `None` as "not my concern", so a
    garbled, missing or doubled line was skipped with no output at all.

    `split("\\n")`, not `splitlines()`: the `^` of `re.MULTILINE` only matches
    after `\\n`, and `splitlines()` also breaks on \\x0b, \\x0c, \\x1c-\\x1e,
    \\x85 and U+2028/2029, so the two could disagree about what a line is.
    """
    lines = [
        (n, line) for n, line in enumerate(text.split("\n"), 1) if line.startswith("- Trace:")
    ]
    if not lines:
        return ["no `- Trace:` line"]
    out: list[str] = []
    if len(lines) > 1:
        out.append(
            f"{len(lines)} `- Trace:` lines (lines {', '.join(str(n) for n, _ in lines)}); "
            "the triad reads only the first line in the backticked-id form, and nothing "
            "if that one is `none`"
        )
    for n, line in lines:
        m = TRACE_REF_RE.match(line)
        # `!= "none"` mirrors parse_story's `trace_req_id` predicate exactly, so
        # "accepted" means "visible to the triad, or a declared gap with a reason".
        if (m and m.group(1) != "none") or TRACE_NONE_RE.match(line):
            continue
        out.append(f"line {n} is in neither accepted form: {line[:120]!r}")
    return out


def parse_story(path: Path) -> Story | None:
    m = STORY_FILENAME_RE.match(path.name)
    if not m:
        return None
    text = path.read_text(encoding="utf-8", errors="replace")
    sm = STATUS_LINE_RE.search(text)
    tm = TRACE_REF_RE.search(text)
    return Story(
        path=path,
        epic=m.group(1),
        story_num=m.group(2),
        filename_slug=m.group(3),
        status=sm.group(1) if sm else None,
        trace_req_id=tm.group(1) if (tm and tm.group(1) != "none") else None,
        trace_embedded_state=tm.group(2) if tm else None,
        trace_line_no=text.count("\n", 0, tm.start()) + 1 if tm else None,
        trace_line_problems=_trace_line_problems(text),
    )


def iter_stories(story_dir: Path | None = None) -> list[Story]:
    # NB: default resolved at CALL time, not def time -- self-tests reassign
    # the module-level STORY_DIR global and rely on this being live.
    if story_dir is None:
        story_dir = STORY_DIR
    if not story_dir.is_dir():
        return []
    out = []
    for p in sorted(story_dir.glob("*.md")):
        s = parse_story(p)
        if s is not None:
            out.append(s)
    return out


def story_original_relpath(story: Story, text: str | None = None) -> str | None:
    """Best-effort recovery of the ORIGINAL `spec/`-relative path (e.g.
    `v1/foo` or `lumen-design-adoption/phase-1-foundation`) from the story's
    own "Source feature folder:" Dev Notes line. Returns None for the two
    stories with no historical spec/ provenance (brand-new BMAD-era stories).
    """
    if text is None:
        text = story.path.read_text(encoding="utf-8", errors="replace")
    m = SOURCE_FOLDER_RE.search(text)
    return m.group(1).rstrip("/") if m else None


# ---------------------------------------------------------------------------
# trace.toml loading
# ---------------------------------------------------------------------------

def load_trace_rows() -> list[dict]:
    if not TRACE_TOML.exists():
        return []
    with TRACE_TOML.open("rb") as f:
        data = tomllib.load(f)
    return data.get("req", [])


def _row_feats(row: dict) -> list[str]:
    feat = row.get("feature")
    if isinstance(feat, str):
        return [feat]
    return list(feat or [])


# ---------------------------------------------------------------------------
# Slug/evidence resolution helpers (shared by several checks)
# ---------------------------------------------------------------------------

# Iteration/follow-up trace-row slugs that intentionally have NO dedicated
# story — they fold as `Tasks/Subtasks` bullets under their BASE story
# (the existing CHANGELOG rollup convention, reused here as the story-fold
# map too — same authors, same reasoning: a version-bump or umbrella-child
# folder is not a new top-level unit of work). Doubles as the
# `feature-shipped-changelog-missing` rollup allowlist below. Keep SHORT;
# a new top-level story earns its own entry, never an allowlist row.
CHANGELOG_ROLLUP_ALLOWLIST: dict[str, str] = {
    "v0-paper-sma": "CHANGELOG § Strategy — `**v0**` (Paper-trading SMA-crossover tracer bullet).",
    "v05-composed-strategies": "CHANGELOG § Strategy — `**v0.5**` (Composed strategies).",
    "v1-cross-sectional-momentum": "CHANGELOG § Strategy — `**v1**` (Cross-sectional top-N momentum).",
    "v15a-mean-reversion-pairs": "CHANGELOG § Strategy — `**v1.5a**` (Mean-reversion on z-scored pairs).",
    "v1-5b-multi-venue": "CHANGELOG § Strategy — `**v1.5b**` (Multi-venue + 1-second aggregated trades).",
    "v2-llm-strategy": "CHANGELOG § Strategy — `**v2**` (LLM news/sentiment strategy overlay).",
    "v2-1-tracing-layer-redactor": "CHANGELOG § Strategy — `**v2.1**` (tracing-Layer secret redactor).",
    "v5-latency-slippage-sim": "CHANGELOG § Strategy — `**v5**` (deterministic latency & slippage sim, v0.1→v0.5 chain).",
    "v5-latency-slippage-sim-v0.2.0-anchor-migration": "CHANGELOG § Strategy — `**v5**` line ('v0.2 anchor migration').",
    "v5-latency-slippage-sim-v0.3.0-full-path-wiring": "CHANGELOG § Strategy — `**v5**` line ('v0.3 full-path wiring').",
    "v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit": "CHANGELOG § Strategy — `**v5**` line ('v0.4 candle/realdata feature-gated re-emit').",
    "v5-latency-slippage-sim-v0.5.0-square-root-market-impact": "CHANGELOG § Strategy — `**v5**` line ('v0.5 sqrt-impact').",
    "v25-tcn-overlay": "CHANGELOG § Retired — `**v2.5 DL forecaster programme**` (TCN overlay).",
    "v25-tcn-alpha-investigation": "CHANGELOG § Retired — `**v2.5 DL forecaster programme**` (TCN alpha-investigation sub-study).",
    "v25-tcn-recalibrate": "CHANGELOG § Retired — `**v2.5 DL forecaster programme**` (TCN recalibrate sub-study).",
    "v25-tcn-threshold-tuning": "CHANGELOG § Retired — `**v2.5 DL forecaster programme**` (TCN threshold-tuning sub-study).",
    "v25-tcn-horizon-bump-or-retire": "CHANGELOG § Retired — `**v2.5 DL forecaster programme**` (TCN horizon-bump sub-study).",
    "v25a-patchtst-overlay": "CHANGELOG § Retired — `**v2.5 DL forecaster programme**` (PatchTST overlay).",
    "v3-volatility-forecaster-noop-fix": "CHANGELOG § Retired — `**v3 volatility forecaster**` line ('+ noop-fix').",
    "v3-regime-classifier": "CHANGELOG § Retired — `**v3 regime-classifier / v3 XGBoost cheap-classifier**`.",
    "cockpit-activity-audit-ledger-producer": "CHANGELOG § Cockpit — `**cockpit-activity-status-bar** + **-audit-ledger-producer** + **-llm-producer**`.",
    "cockpit-activity-llm-producer": "CHANGELOG § Cockpit — `**cockpit-activity-status-bar** + **-audit-ledger-producer** + **-llm-producer**`.",
    "reflection-memory-trader-wiring": "CHANGELOG § Core infra — `**reflection-memory** (+ trader-wiring)`.",
    "ui-rethink-phase-d-trail-followup": "CHANGELOG § Cockpit — `**ui-rethink-phase-d-trail** (+ follow-up)`.",
    "advisor-bakeoff-ranking": "CHANGELOG § Advisor — `**advisor-bakeoff F1+F2**` (F1 bake-off + F2 ranking; slug carries the -ranking tail).",
    "phase-2c-overlays": "CHANGELOG § v2 — the three children are indexed: `**advisor-vol-estimator**` / `**advisor-vol-overlay-reposition**` / `**advisor-drawdown-control-overlay**` (this folder is their shared test-report umbrella).",
    # --- Phase 5b discoveries (2026-07-25 re-founding): the story-filename
    # slug does not always equal what CHANGELOG.md cites.
    "v3-llm-forecaster": "CHANGELOG § Core infra — cited as prose \"v3 LLM-forecaster\" (space+case variant of the slug, not a gap); shipped-partial (alpha-verdict wave deferred).",
    # Lumen sub-phase stories resolve to the BARE nested slug (e.g.
    # `phase-1-foundation`, from `spec/lumen-design-adoption/phase-1-foundation/`)
    # via `_resolve_slug_for_changelog`'s "Source feature folder:" fallback —
    # NOT the story-filename form (`lumen-phase-1-foundation`), since these 6
    # stories have no trace.toml REQ-id to bridge through (a documented
    # trace-coverage gap, spec audit 2026-07-06).
    "phase-1-foundation": "CHANGELOG § Cockpit — covered by the `**lumen-design-adoption**` rollup line ('Phase 1 tokens/chrome/status-bar shipped').",
    "phase-2-shell-ia-charts": "CHANGELOG § Cockpit — covered by the `**lumen-design-adoption**` rollup line ('Phases 2-5 shipped').",
    "phase-3-detail-screens": "CHANGELOG § Cockpit — covered by the `**lumen-design-adoption**` rollup line ('Phases 2-5 shipped').",
    "phase-4-backtest-panel": "CHANGELOG § Cockpit — covered by the `**lumen-design-adoption**` rollup line ('Phases 2-5 shipped').",
    "phase-5-humancontrol-agentfeed": "CHANGELOG § Cockpit — covered by the `**lumen-design-adoption**` rollup line ('Phases 2-5 shipped').",
}

_ITERATION_SUFFIX_RE = re.compile(r"-v\d+\.\d+\.\d+.*$")


def slug_folds_into_base(slug: str) -> bool:
    """True iff `slug` is a known iteration/follow-up/umbrella-child that
    intentionally has no dedicated story (see CHANGELOG_ROLLUP_ALLOWLIST
    docstring above)."""
    if slug in CHANGELOG_ROLLUP_ALLOWLIST:
        return True
    base = _ITERATION_SUFFIX_RE.sub("", slug)
    return base != slug and bool(base)


# ---------------------------------------------------------------------------
# Check: dead intra-tree links
# ---------------------------------------------------------------------------

LINK_RE = re.compile(r"(?<!\!)\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")


def extract_links(text: str) -> list[str]:
    return LINK_RE.findall(text)


def is_external(link: str) -> bool:
    return link.startswith(("http://", "https://", "mailto:", "#"))


# Byte-immutable anchored reports whose internal links cannot be raw-edited
# without breaking the verify_anchors body-SHA gate (ADR-0038 anchor-additive
# contract). Keyed by (repo-relative report path, exact link string). Keep
# this list SHORT and each entry justified; remove an entry once its link is
# fixed. Carried verbatim through the Phase 5b `spec/` retirement — none of
# these report BODIES moved or changed in this phase, so the tuples are
# unaffected by the cutover.
KNOWN_FROZEN_DEAD_LINKS: set[tuple[str, str]] = {
    (
        "evidence/v1/v3-volatility-forecaster/reports/vol-verdict-bs1-realdata-20260522.md",
        "../architecture/adr/0038-vol-forecast-verdict-shape.md"
        "#d1-v-verdict-priority-tree-parallel-to-adr-0033--d3-not-extension",
    ),
    (
        "evidence/v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun-errata.md",
        "../../../../spec/dev-notes/p2-wobble-thesis-analysis-2026-07-10.md",
    ),
    (
        "evidence/v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun.md",
        "../../../../spec/architecture/adr/0084-p2-corpus-set-coinbase-adapter-verdict-rerun.md",
    ),
    # Phase 5b (2026-07-25) fallout: both bodies also self-cite their own
    # (now-archived) feature.md via a THIRD link that was valid before this
    # phase moved feature.md out of spec/v3/advisor-corpus-expansion/ —
    # newly-dead, not pre-existing, but the body cannot be edited to follow
    # (see the two tuples above for the identical reasoning).
    (
        "evidence/v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun-errata.md",
        "../../../../spec/v3/advisor-corpus-expansion/feature.md",
    ),
    (
        "evidence/v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun.md",
        "../../../../spec/v3/advisor-corpus-expansion/feature.md",
    ),
}


def check_dead_links(md_path: Path, text: str, report: Report) -> None:
    try:
        rel = md_path.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        rel = None
    for raw in extract_links(text):
        if is_external(raw):
            continue
        target_str = raw.split("#", 1)[0]
        if not target_str:
            continue  # pure anchor link
        target = (md_path.parent / target_str).resolve()
        if not target.exists():
            if rel is not None and (rel, raw) in KNOWN_FROZEN_DEAD_LINKS:
                continue
            report.add("dead-link", md_path, f"link target missing: {raw}")


# ---------------------------------------------------------------------------
# Check: required frontmatter (soft, PRD/architecture only now)
# ---------------------------------------------------------------------------

_CARGO_TEST_CITE_RE = re.compile(
    r"cargo test\s+-p\s+([A-Za-z0-9_\-]+)\s+--test\s+([A-Za-z0-9_]+)"
)


def _package_dirs(root: Path) -> dict[str, Path]:
    """Real package-name -> crate-directory map.

    NOT an assumption that they match: exactly one crate in this workspace is a
    mismatch (`trading_core` lives in `crates/core`), and assuming otherwise is
    what made the first run of this check report a false dead citation
    (bug-log `#168`).
    """
    out: dict[str, Path] = {}
    for manifest in sorted((root / "crates").glob("*/Cargo.toml")):
        m = re.search(r'^name\s*=\s*"([^"]+)"', manifest.read_text(encoding="utf-8"), re.M)
        if m:
            out[m.group(1)] = manifest.parent
    return out


def check_cargo_citations(
    roots: Iterable[Path],
    report: Report,
    pkg_dirs: dict[str, Path] | None = None,
    extra_files: Iterable[Path] | None = None,
) -> int:
    """Every `cargo test -p X --test Y` cited in a tracked .md must resolve.

    bug-log `#168`. ADR-0039 cited two test targets under `-p strategy` that have
    never existed there: the whole `llm_forecaster` module moved
    `strategy` -> `trader` under ADR-0041 D1 and the ADR's "Enforced by" list was
    never repointed. One of them, `llm_forecaster_verdict_mutual_exclusivity`, has
    never existed under ANY package — the gate shipped as
    `crates/trader/tests/llm_verdict_priority_tree.rs`.

    The 2026-08-15 claims ledger had already flagged that row as MISSING, and drew
    the wrong conclusion from it: the invariant was enforced all along, under a
    different name. **The audit searched for the pointer and not for the
    property.** A citation that cannot be resolved is a dead link in executable
    clothing, so it is checked here with the rest of them.

    Returns the number of citations examined, so a PASS states what it measured.
    """
    # `pkg_dirs` / `extra_files` are injection points for `--self-test`, which
    # must exercise this rule against synthetic fixtures rather than the live
    # tree (where a PASS proves nothing about whether the rule still bites).
    if pkg_dirs is None:
        pkg_dirs = _package_dirs(REPO_ROOT)
    seen = 0
    # The tree roots plus CHANGELOG.md, which makes capability claims too and
    # sits outside them (it lives at the repo root by ratified decision D3).
    scanned = list(iter_tree_md(roots)) if roots else []
    if extra_files is not None:
        scanned.extend(extra_files)
    elif CHANGELOG_PATH.is_file():
        scanned.append(CHANGELOG_PATH)
    for md in scanned:
        try:
            text = md.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        lines = text.splitlines()
        for m in _CARGO_TEST_CITE_RE.finditer(text):
            pkg, target = m.group(1), m.group(2)
            # A document may QUOTE a known-dead citation in order to document it
            # — the opposite of a stale pointer, and it must not be flagged.
            # Marked with `dead-citation-ok`, mirroring the
            # `# noqa: queue-staleness` convention in queue_staleness_check.py.
            #
            # The marker is honoured on the citation's own line OR on either of
            # the two lines above it, because a quoted command usually sits
            # inside a fenced block where an HTML comment would corrupt the very
            # text being quoted. This bug-log's own `#168` entry is that case.
            line_no = text.count("\n", 0, m.start())
            window = lines[max(0, line_no - 2) : line_no + 1]
            if any("dead-citation-ok" in l for l in window):
                continue
            seen += 1
            crate_dir = pkg_dirs.get(pkg)
            if crate_dir is None:
                report.add(
                    "dead-cargo-citation",
                    md,
                    f"cites `cargo test -p {pkg} --test {target}` but no crate in "
                    f"crates/ declares package `{pkg}`",
                )
                continue
            if (crate_dir / "tests" / f"{target}.rs").is_file():
                continue
            # A [[test]] block may give an explicit non-default path.
            if re.search(
                r'name\s*=\s*"%s"' % re.escape(target),
                (crate_dir / "Cargo.toml").read_text(encoding="utf-8"),
            ):
                continue
            report.add(
                "dead-cargo-citation",
                md,
                f"cites `cargo test -p {pkg} --test {target}`, but neither "
                f"{crate_dir.name}/tests/{target}.rs nor a [[test]] named "
                f"`{target}` exists in {crate_dir.name}/Cargo.toml",
            )
    return seen


def check_frontmatter(md_path: Path, text: str, report: Report) -> None:
    soft = SOFT_FRONTMATTER.get(md_path.name)
    if soft is None:
        return
    fm = parse_frontmatter(text)
    if fm is None:
        return  # soft check: absence is not a hard fail
    missing = sorted(soft - fm.keys())
    if missing:
        report.add(
            "missing-frontmatter",
            md_path,
            f"missing keys: {missing}",
        )


def check_story_status_values(report: Report) -> None:
    """Structural guard: every story's `Status:` line must be in the known
    vocabulary. The BMAD-native analogue of the old feature.md
    `invalid status` check (folded into missing-frontmatter for continuity —
    same violation shape: 'is this file's lifecycle field well-formed')."""
    for story in iter_stories():
        if story.status is None:
            report.add(
                "missing-frontmatter",
                story.path,
                "no `Status:` line found",
            )
        elif story.status not in VALID_STORY_STATUSES:
            report.add(
                "missing-frontmatter",
                story.path,
                f"invalid Status: {story.status!r} (allowed: {sorted(VALID_STORY_STATUSES)})",
            )


# ---------------------------------------------------------------------------
# Check: orphan stories — sprint-status.yaml <-> story-file bijection
# ---------------------------------------------------------------------------
#
# Re-founding of `orphan-feature`. sprint-status.yaml's own STATUS DEFINITIONS
# say a `backlog` story "only exists in the epic file" (no story file yet) —
# that is NOT orphan, it is the documented pre-promotion state. Anything else
# (ready-for-dev/in-progress/review/done/retired) MUST have a story file.
# The reverse direction — a story file with no sprint-status.yaml entry — is
# always a violation (the board is supposed to be exhaustive).

_SPRINT_STATUS_KEY_RE = re.compile(r"^\s{2}([A-Za-z0-9_.\-]+):\s*([A-Za-z0-9_.\-]+)")


def parse_sprint_status_board(path: Path | None = None) -> dict[str, str]:
    """Minimal line-scan parser for the `development_status:` map (avoids a
    PyYAML dependency for a flat `key: value` block — same philosophy as the
    hand-rolled frontmatter parser above). Returns {key: status}."""
    if path is None:
        path = SPRINT_STATUS_YAML
    if not path.exists():
        return {}
    out: dict[str, str] = {}
    in_block = False
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.strip() == "development_status:":
            in_block = True
            continue
        if not in_block:
            continue
        if line and not line.startswith((" ", "\t", "#")):
            break  # dedented past the block (e.g. `action_items:`)
        m = _SPRINT_STATUS_KEY_RE.match(line)
        if m:
            out[m.group(1)] = m.group(2)
    return out


def check_orphan_stories(report: Report) -> None:
    board = parse_sprint_status_board()
    # `deferred-work.md` is the bmad-code-review step-04 deferred-findings
    # ledger (one heading per review), NOT a story file — exclude it from the
    # story<->board bijection (2026-07-26, first BMAD-native review run).
    non_story_files = {"deferred-work"}
    # Retrospective documents (`epic-{N}-retro-{date}.md`) are the
    # bmad-retrospective workflow's OWN output, not stories — same category as
    # `deferred-work` above. The board tracks them as `epic-N-retrospective`
    # rows, which the loop below already skips (they fail the `^\d+-\d+-`
    # story-key match), so there is no board key for a retro file to bind to and
    # the story<->board bijection must not include them.
    # Added 2026-08-16, when the first retrospectives in this repo's history
    # (epics 5 and 7) tripped `orphan-story` purely by existing.
    _RETRO_FILE_RE = re.compile(r"^epic-\d+-retro-")
    story_filenames = (
        {
            p.stem
            for p in STORY_DIR.glob("*.md")
            if p.stem not in non_story_files and not _RETRO_FILE_RE.match(p.stem)
        }
        if STORY_DIR.is_dir()
        else set()
    )

    for key, status in board.items():
        if not re.match(r"^\d+-\d+-", key):
            continue  # epic-N / epic-N-retrospective rows, not stories
        if key in story_filenames:
            continue
        if status == "backlog":
            continue  # documented pre-promotion state — not orphan
        report.add(
            "orphan-story",
            SPRINT_STATUS_YAML,
            f"sprint-status entry {key!r} (status={status!r}) has no story file "
            f"under {STORY_DIR.relative_to(REPO_ROOT)}/",
        )

    for filename in story_filenames:
        if filename not in board:
            report.add(
                "orphan-story",
                STORY_DIR / f"{filename}.md",
                "story file has no sprint-status.yaml development_status entry",
            )


# ---------------------------------------------------------------------------
# Check: anchors.toml well-formed (mechanism unchanged; evidence_dir repointed)
# ---------------------------------------------------------------------------

def check_anchors(evidence_dir: Path, report: Report) -> dict[str, dict]:
    anchors_path = evidence_dir / "anchors.toml"
    if not anchors_path.exists():
        return {}
    with anchors_path.open("rb") as f:
        data = tomllib.load(f)
    anchors_list = data.get("anchors", [])
    by_scenario: dict[str, dict] = {}
    for a in anchors_list:
        missing = sorted({"scenario", "version", "sha256"} - a.keys())
        if missing:
            report.add(
                "bad-anchor",
                anchors_path,
                f"entry missing keys {missing}: {a}",
            )
            continue
        by_scenario[a["scenario"]] = a
    return by_scenario


# ---------------------------------------------------------------------------
# Check: trace.toml row validity (product/arch/crates/tests paths + feature
# slug resolution + anchor citations)
# ---------------------------------------------------------------------------

def check_trace(report: Report, anchors: dict[str, dict]) -> None:
    if not TRACE_TOML.exists():
        return  # not yet adopted; that's fine

    rows = load_trace_rows()
    cited_anchors: set[str] = set()

    story_filenames = {p.stem for p in STORY_DIR.glob("*.md")} if STORY_DIR.is_dir() else set()
    # Map bare filename-slug (strip the `\d+-\d+-` prefix) for a cheap direct
    # membership test before falling back to the fold-allowlist.
    filename_slugs = set()
    for fn in story_filenames:
        m = re.match(r"^\d+-\d+-(.+)$", fn)
        if m:
            filename_slugs.add(m.group(1))

    for row in rows:
        rid = row.get("id", "<no-id>")
        for field_name in ("product", "arch", "crates", "tests"):
            val = row.get(field_name)
            if val is None:
                continue
            if isinstance(val, str):
                _check_trace_path(rid, field_name, val, report)
            elif isinstance(val, list):
                for v in val:
                    _check_trace_path(rid, field_name, v, report)

        for slug in _row_feats(row):
            if slug in filename_slugs or slug_folds_into_base(slug):
                continue
            report.add(
                "trace-broken-path",
                TRACE_TOML,
                f"row {rid} field feature: slug {slug!r} matches no story file "
                f"under {STORY_DIR.relative_to(REPO_ROOT)}/ and is not a documented "
                f"fold (CHANGELOG_ROLLUP_ALLOWLIST / -vN.N.N- iteration suffix)",
            )

        raw_anchors = row.get("anchors", [])
        if isinstance(raw_anchors, list):
            for anc in raw_anchors:
                cited_anchors.add(anc)
                if anc not in anchors:
                    report.add(
                        "trace-broken-path",
                        TRACE_TOML,
                        f"row {rid}: anchor {anc!r} not in anchors.toml",
                    )

    for scenario in anchors:
        if scenario not in cited_anchors:
            report.add(
                "unreferenced-anchor",
                EVIDENCE_DIR / "anchors.toml",
                f"anchor {scenario!r} not cited by any trace.toml row",
            )


def _check_trace_path(row_id: str, field_name: str, raw: str, report: Report) -> None:
    raw_no_frag = raw.split("#", 1)[0].split("::", 1)[0]
    if not raw_no_frag:
        return
    target = (REPO_ROOT / raw_no_frag).resolve()
    if not target.exists():
        report.add(
            "trace-broken-path",
            TRACE_TOML,
            f"row {row_id} field {field_name}: missing path {raw}",
        )


# ---------------------------------------------------------------------------
# Check: story `Status: done` has at least one test report
# ---------------------------------------------------------------------------
#
# Re-founding of `shipped-no-tests`. FAITHFULLY preserves the original rule's
# scope, quirk and all: the pre-migration `check_shipped_have_tests` iterated
# `spec_dir.iterdir()` ONLY — i.e. the bare top-level feature folders, NEVER
# descending into `v1/`/`v2/`/`v3/` (unlike `check_orphan_features` and
# `check_feature_shipped_changelog_missing`, which explicitly did). None of
# the 6 bare-top-level folders (advisor-reflection-decision-loop,
# cockpit-app-bundle, cockpit-cross-platform, iced-ecosystem-evaluation,
# lumen-design-adoption, ui-gallery-table-cell) ever reached `status:
# shipped`, so this check was ALREADY a structural no-op on the real tree —
# re-scoping it to "every done story" now would be a scope WIDENING that
# surfaces ~17 pre-existing, never-gated gaps (features whose test evidence
# lives in a shared/umbrella evidence folder under a different slug, e.g.
# `phase-2c-overlays`) rather than a faithful re-founding. Kept narrow on
# purpose; verified empirically still a no-op against the real tree
# (2026-07-25).
_BARE_ORIGINAL_SLUGS = {
    "advisor-reflection-decision-loop", "cockpit-app-bundle", "cockpit-cross-platform",
    "iced-ecosystem-evaluation", "lumen-design-adoption", "ui-gallery-table-cell",
}


def check_story_done_no_tests(report: Report, evidence_dir: Path | None = None) -> None:
    if evidence_dir is None:
        evidence_dir = EVIDENCE_DIR
    for story in iter_stories():
        if story.status != "done":
            continue
        if story.filename_slug not in _BARE_ORIGINAL_SLUGS:
            continue
        reports_dir = evidence_dir / story.filename_slug / "reports"
        if not reports_dir.exists():
            report.add("story-done-no-tests", story.path, "shipped story has no reports/ directory")
            continue
        if not any(p.suffix == ".md" for p in reports_dir.glob("*.md")):
            report.add(
                "story-done-no-tests", story.path,
                "shipped story has no .md report (only screenshots / logs)",
            )


# ---------------------------------------------------------------------------
# Check: trace-ref-unparsed / trace-ref-dangling / trace-state-annotation-drift
#   — the triad's join key, and the line's copy of its row's state
# ---------------------------------------------------------------------------
#
# bug-log `#216`. `status-drift` and `story-done-trace-drift` key on the REQ-id
# a story's `- Trace:` line names, and both SKIP a story whose id is None or
# resolves to no row — by design, since "no row" is also the declared gap. So
# a line they cannot read (a bare id, story 2-63's `(state=`in-progress` since
# …)`, a `- **Trace:**` variant, a second line, no line at all) and an id with
# a typo were both skipped with no output: the rule could not see its subject,
# and that read as a pass (the `#188` family). These two rules make each such
# story a violation of its own. The remaining leg, story-done-changelog-missing,
# falls back to the filename slug and so never skipped; it needs no guard.

def check_trace_ref_unparsed(report: Report) -> None:
    for story in iter_stories():
        for problem in story.trace_line_problems:
            # The stock create-story template has no `- Trace:` line, so a fresh
            # story lands here until its author adds one; say so where it bites.
            fresh = (
                " (a fresh bmad-create-story output has none until it is added)"
                if problem.startswith("no ") else ""
            )
            # Say what the triad actually reads: a doubled line, or one broken
            # across `\n` that the whole-file search still matches, leaves the
            # story PARTLY visible, not invisible.
            seen = (
                f"status-drift and story-done-trace-drift read only line "
                f"{story.trace_line_no} ({story.trace_req_id})"
                if story.trace_req_id is not None
                else "status-drift and story-done-trace-drift cannot see this story"
            )
            report.add(
                "trace-ref-unparsed",
                story.path,
                f"{problem} — a story needs exactly one column-0 `- Trace:` line (by "
                f"convention under ### References), in one of the two accepted forms: "
                f"{TRACE_FORMS_HINT}{fresh}. Until it does, {seen} (bug-log #216)",
            )


def check_trace_ref_dangling(report: Report) -> None:
    ids = {r.get("id") for r in load_trace_rows()}
    missing_ledger = "" if TRACE_TOML.exists() else " (trace.toml itself was not found)"
    for story in iter_stories():
        if story.trace_req_id is None or story.trace_req_id in ids:
            continue
        report.add(
            "trace-ref-dangling",
            story.path,
            f"line {story.trace_line_no} names {story.trace_req_id}, which has no [[req]] "
            f"row in trace.toml{missing_ledger} — status-drift and story-done-trace-drift "
            f"skip an id they cannot resolve, so they cannot see this story until the id "
            f"is corrected or its row is added (bug-log #216)",
        )


def check_trace_state_annotation_drift(report: Report) -> None:
    """A story's `(state=…)` annotation must equal its row's `state=` (bug-log `#219`).

    `parse_story` has read the annotation into `trace_embedded_state` since the
    Phase-5b re-founding, and until this rule nothing compared it with anything:
    on 2026-11-24, 11 of 141 parsed lines disagreed with their rows (`scoped` or
    `tester-done` against `shipped`/`retired`). A reader takes the annotation for
    the requirement's state, so a copy that is neither checked nor dropped is the
    exact drift ADR-0082 exists to prevent. Only a story whose id resolves to a
    row is checked: an unreadable line is `trace-ref-unparsed`, an id with no row
    `trace-ref-dangling`, and the declared gap carries no annotation.
    """
    rows_by_id = {r.get("id"): r for r in load_trace_rows()}
    for story in iter_stories():
        if story.trace_req_id is None:
            continue
        row = rows_by_id.get(story.trace_req_id)
        if row is None:
            continue  # reported by trace-ref-dangling
        state = row.get("state")
        if story.trace_embedded_state == state:
            continue
        fix = (
            f"change the backticked state on that line to `{state}`"
            if state is not None
            else "the row has no state= field; give it one, then copy it here"
        )
        report.add(
            "trace-state-annotation-drift",
            story.path,
            f"line {story.trace_line_no} annotates {story.trace_req_id} as "
            f"(state=`{story.trace_embedded_state}`), but its trace.toml row says "
            f"state={state!r} — the annotation is a copy of the row's state and moves "
            f"with it: {fix} (bug-log #219)",
        )


def run_story_triad(report: Report) -> None:
    """The ADR-0082 triad plus the three guards on its join key, in one call.

    main() calls THIS and nothing else for the triad, and the self-tests drive
    the same function — so dropping any call below turns a self-test red, and
    `_self_test_main_runs_story_triad` asserts all six categories through
    main()'s printed report, so a main() that drops this call, or inlines only
    some of the six again, is red too (bug-log `#216`/`#219`, the `#190` wiring
    rule).
    """
    check_trace_ref_unparsed(report)
    check_trace_ref_dangling(report)
    check_trace_state_annotation_drift(report)
    check_status_drift(report)
    check_story_done_trace_drift(report)
    check_story_done_changelog_missing(report)


# ---------------------------------------------------------------------------
# Check: status-drift — story `Status:` <-> trace `[[req]]` `state=`
# ---------------------------------------------------------------------------
#
# Re-founded (2026-07-25, BMAD-migration Phase 5b) onto the story/trace
# layout, replacing the old deck+PASS-report mechanism entirely (that
# mechanism has no clean analogue now that the fine-grained pre-ship pipeline
# statuses collapse into BMAD's coarser `review` bucket). The invariant is
# now: whatever a trace row's `state=` says, the story's `Status:` line must
# be the value the Phase-2 retro-generation convention maps it to (the SAME
# mapping documented verbatim in every story's own Dev Notes: "shipped->done;
# retired/deprecated->retired; presenter/tester/dev-done->review;
# arch-done->ready-for-dev; candidate/draft/reserved->backlog"). Degrades
# gracefully (skips silently) for any `state=` value not in the table, and
# for stories with no resolvable trace row — this is deliberate: an
# unrecognised value should not manufacture false positives. The skip is NOT
# silent for the tree, though: a missing, doubled or unparseable `- Trace:`
# line is reported by `trace-ref-unparsed`, and an id with no row by
# `trace-ref-dangling` (bug-log `#216`); only the declared gap
# (`- Trace: none — <reason>`) and an unmapped `state=` are skipped unreported.
STATE_TO_STORY_STATUS: dict[str, str] = {
    "shipped": "done",
    "shipped-partial": "done",
    "retired": "retired",
    "deprecated": "retired",
    "presenter-done": "review",
    "tester-done": "review",
    "dev-done": "review",
    "tested": "review",
    "verified": "review",
    "passed": "review",
    "design-complete": "ready-for-dev",
    "arch-done": "ready-for-dev",
    "candidate": "backlog",
    "draft": "backlog",
    "reserved": "backlog",
    "proposed": "backlog",
    "roadmap": "in-progress",
    "in-progress": "in-progress",
    "active": "in-progress",
}


def check_status_drift(report: Report) -> None:
    rows_by_id = {r.get("id"): r for r in load_trace_rows()}
    for story in iter_stories():
        if story.trace_req_id is None or story.status is None:
            # Declared gap, or a line trace-ref-unparsed reports (#216); a missing
            # Status: is reported as missing-frontmatter.
            continue
        row = rows_by_id.get(story.trace_req_id)
        if row is None:
            continue  # dangling REQ-id — reported by trace-ref-dangling (#216)
        state = row.get("state")
        expected = STATE_TO_STORY_STATUS.get(state)
        if expected is None:
            continue  # unrecognised state value — degrade gracefully, don't guess
        if story.status != expected:
            report.add(
                "status-drift",
                story.path,
                f"trace row {story.trace_req_id} state={state!r} maps to story Status "
                f"{expected!r}, but story Status is {story.status!r}",
            )


def _self_test_status_drift() -> bool:
    """Synthetic-fixture proof: (a) drifting — trace state=shipped, story
    Status=review -> 1 violation; (b) compliant — state=shipped, Status=done
    -> 0; (c) unrecognised state — degrades silently -> 0."""
    import tempfile

    def write_story(dir_: Path, filename: str, status: str, req_id: str, embedded_state: str) -> Path:
        p = dir_ / filename
        p.write_text(
            f"# Story X.Y: fixture\n\nStatus: {status}\n\n"
            f"### References\n\n- Trace: `{req_id}` (state=`{embedded_state}`)\n"
        )
        return p

    with tempfile.TemporaryDirectory() as tmp:
        story_dir = Path(tmp) / "stories"
        story_dir.mkdir()
        write_story(story_dir, "1-1-drifting.md", "review", "REQ-DRIFT", "shipped")
        write_story(story_dir, "1-2-compliant.md", "done", "REQ-OK", "shipped")
        write_story(story_dir, "1-3-unrecognised.md", "backlog", "REQ-WEIRD", "some-future-value")
        trace_path = Path(tmp) / "trace.toml"
        trace_path.write_text(
            "[[req]]\n"
            'id = "REQ-DRIFT"\nfeature = "drifting"\nstate = "shipped"\n\n'
            "[[req]]\n"
            'id = "REQ-OK"\nfeature = "compliant"\nstate = "shipped"\n\n'
            "[[req]]\n"
            'id = "REQ-WEIRD"\nfeature = "unrecognised"\nstate = "some-future-value"\n'
        )

        global STORY_DIR, TRACE_TOML
        orig_story_dir, orig_trace = STORY_DIR, TRACE_TOML
        STORY_DIR, TRACE_TOML = story_dir, trace_path
        try:
            rep = Report()
            check_status_drift(rep)
        finally:
            STORY_DIR, TRACE_TOML = orig_story_dir, orig_trace

        hits = [v for v in rep.violations if v.category == "status-drift"]
        ok = len(hits) == 1 and "drifting" in str(hits[0].path)
        print(
            "spec-lint --self-test (status-drift): "
            + ("PASS — fires on drift, silent on compliant/unrecognised-state" if ok
               else f"FAIL — expected exactly 1 hit on 'drifting', got {[(str(v.path), v.detail) for v in hits]}")
        )
        return ok


# ---------------------------------------------------------------------------
# Check: story-done-trace-drift (ADR-0082 terminal invariant, re-founded)
# ---------------------------------------------------------------------------
#
# Narrow, ADR-0082-specific sibling of status-drift: a story `Status: done`
# MUST have a trace row whose `state=` is itself a shipped-terminal value
# (`shipped` or `shipped-partial`) — including the missing-state case. This
# preserves the OLD `feature-shipped-trace-drift` rule's exact intent (the
# "once shipped, the row must say shipped" invariant) on the new artifacts.

_DONE_TERMINAL_STATES = {"shipped", "shipped-partial"}


def check_story_done_trace_drift(report: Report) -> None:
    rows_by_id = {r.get("id"): r for r in load_trace_rows()}
    for story in iter_stories():
        if story.status != "done" or story.trace_req_id is None:
            continue
        row = rows_by_id.get(story.trace_req_id)
        if row is None:
            continue  # dangling REQ-id — reported by trace-ref-dangling (#216)
        state = row.get("state")
        if state not in _DONE_TERMINAL_STATES:
            shown = "<missing state= field>" if state is None else repr(state)
            report.add(
                "story-done-trace-drift",
                TRACE_TOML,
                f"story {story.path.name} is Status: done (trace {story.trace_req_id}) "
                f"but trace state is {shown} — must be \"shipped\" or \"shipped-partial\" "
                f"(ADR-0082 § D2, re-founded)",
            )


def _self_test_story_done_trace_drift() -> bool:
    """Four fixtures: (a) drifting — Status:done, state=passed -> 1 hit;
    (b) compliant — Status:done, state=shipped -> 0; (c) preship —
    Status:review, state=tested -> 0 (pre-ship aliases are legal pre-done);
    (d) missing-state — Status:done, row has no state= -> 1 hit."""
    import tempfile

    def write_story(dir_: Path, filename: str, status: str, req_id: str, embedded_state: str) -> None:
        (dir_ / filename).write_text(
            f"# Story X.Y: fixture\n\nStatus: {status}\n\n"
            f"### References\n\n- Trace: `{req_id}` (state=`{embedded_state}`)\n"
        )

    with tempfile.TemporaryDirectory() as tmp:
        story_dir = Path(tmp) / "stories"
        story_dir.mkdir()
        write_story(story_dir, "1-1-drifting.md", "done", "REQ-DRIFT", "passed")
        write_story(story_dir, "1-2-compliant.md", "done", "REQ-OK", "shipped")
        write_story(story_dir, "1-3-preship.md", "review", "REQ-PRESHIP", "tested")
        write_story(story_dir, "1-4-missing-state.md", "done", "REQ-NOSTATE", "")
        trace_path = Path(tmp) / "trace.toml"
        trace_path.write_text(
            "[[req]]\n"
            'id = "REQ-DRIFT"\nfeature = "drifting"\nstate = "passed"\n\n'
            "[[req]]\n"
            'id = "REQ-OK"\nfeature = "compliant"\nstate = "shipped"\n\n'
            "[[req]]\n"
            'id = "REQ-PRESHIP"\nfeature = "preship"\nstate = "tested"\n\n'
            "[[req]]\n"
            'id = "REQ-NOSTATE"\nfeature = "missing-state"\n'  # no state= field
        )

        global STORY_DIR, TRACE_TOML
        orig_story_dir, orig_trace = STORY_DIR, TRACE_TOML
        STORY_DIR, TRACE_TOML = story_dir, trace_path
        try:
            rep = Report()
            check_story_done_trace_drift(rep)
        finally:
            STORY_DIR, TRACE_TOML = orig_story_dir, orig_trace

        hits = [v for v in rep.violations if v.category == "story-done-trace-drift"]
        hit_files = sorted(h.detail.split("story ", 1)[1].split(" ", 1)[0] for h in hits)
        ok = hit_files == ["1-1-drifting.md", "1-4-missing-state.md"]
        print(
            "spec-lint --self-test (story-done-trace-drift): "
            + ("PASS — fires on non-terminal-state + missing-state, silent on "
               "compliant + pre-ship" if ok
               else f"FAIL — expected hits on drifting+missing-state, got {hit_files}")
        )
        return ok


# ---------------------------------------------------------------------------
# Check: story-done-changelog-missing (re-founded)
# ---------------------------------------------------------------------------
#
# Every `Status: done` story must be indexed in the root CHANGELOG.md, by
# slug / trace REQ-id / the documented rollup allowlist. Thorough by
# construction (iterates every story file, mirroring the OLD rule's v1/v2/v3
# extension — that rule, unlike shipped-no-tests, DID walk the full tree).

def _slug_indexed_in_changelog(slug: str, req_id: str | None, changelog_lower: str) -> bool:
    if slug and slug.lower() in changelog_lower:
        return True
    if req_id and req_id.lower() in changelog_lower:
        return True
    base = _ITERATION_SUFFIX_RE.sub("", slug or "")
    if base != slug and base and base.lower() in changelog_lower:
        return True
    if slug in CHANGELOG_ROLLUP_ALLOWLIST:
        return True
    return False


def _resolve_slug_for_changelog(story: Story, rows_by_id: dict[str, dict]) -> str:
    """Prefer the trace.toml `feature=` slug (via the REQ-id bridge — the
    canonical, dotted, un-sanitized form); fall back to the "Source feature
    folder:" Dev Notes line (handles nested slugs like lumen phases); fall
    back to the raw filename-derived slug for the ~2 brand-new stories with
    neither."""
    if story.trace_req_id is not None:
        row = rows_by_id.get(story.trace_req_id)
        if row is not None:
            feats = _row_feats(row)
            if feats:
                return feats[0]
    text = story.path.read_text(encoding="utf-8", errors="replace")
    relpath = story_original_relpath(story, text)
    if relpath is not None:
        parts = relpath.split("/")
        if parts and parts[0] in ("v1", "v2", "v3"):
            parts = parts[1:]
        if parts:
            return parts[-1]
    return story.filename_slug


def check_story_done_changelog_missing(report: Report) -> None:
    if not CHANGELOG_PATH.exists():
        return
    changelog_lower = CHANGELOG_PATH.read_text(encoding="utf-8", errors="replace").lower()
    rows_by_id = {r.get("id"): r for r in load_trace_rows()}

    for story in iter_stories():
        if story.status != "done":
            continue
        slug = _resolve_slug_for_changelog(story, rows_by_id)
        if not _slug_indexed_in_changelog(slug, story.trace_req_id, changelog_lower):
            report.add(
                "story-done-changelog-missing",
                story.path,
                f"story {story.path.name!r} (slug {slug!r}) is Status: done but is not "
                f"indexed in CHANGELOG.md (no slug / REQ-id / rollup-allowlist match) — "
                f"the canonical 'what's-been-built' index must reference every shipped "
                f"story (ADR-0082 § D1, re-founded)",
            )


def _self_test_story_done_changelog_missing() -> bool:
    """Five fixtures against a synthetic CHANGELOG.md: (a) indexed-slug ->
    silent; (b) missing -> 1 hit; (c) preship (Status:review) -> silent;
    (d) indexed-req (slug absent, REQ-id present) -> silent; (e) rollup-
    allowlisted slug -> silent."""
    import tempfile

    def write_story(dir_: Path, filename: str, status: str, req_id: str | None) -> None:
        trace_block = f"\n### References\n\n- Trace: `{req_id}` (state=`shipped`)\n" if req_id else \
            "\n### References\n\n- Trace: none — known trace-coverage gap\n"
        (dir_ / filename).write_text(f"# Story X.Y: fixture\n\nStatus: {status}\n{trace_block}")

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        story_dir = root / "stories"
        story_dir.mkdir()
        write_story(story_dir, "1-1-indexed-slug.md", "done", "REQ-INDEXED-SLUG-001")
        write_story(story_dir, "1-2-missing.md", "done", "REQ-MISSING-001")
        write_story(story_dir, "1-3-preship.md", "review", "REQ-PRESHIP-001")
        write_story(story_dir, "1-4-indexed-req.md", "done", "REQ-INDEXED-REQ-001")
        write_story(story_dir, "1-5-v0-paper-sma.md", "done", None)  # rollup-allowlisted slug

        trace_path = root / "trace.toml"
        trace_path.write_text(
            "[[req]]\nid = \"REQ-INDEXED-SLUG-001\"\nfeature = \"indexed-slug\"\nstate = \"shipped\"\n\n"
            "[[req]]\nid = \"REQ-MISSING-001\"\nfeature = \"missing\"\nstate = \"shipped\"\n\n"
            "[[req]]\nid = \"REQ-PRESHIP-001\"\nfeature = \"preship\"\nstate = \"tested\"\n\n"
            "[[req]]\nid = \"REQ-INDEXED-REQ-001\"\nfeature = \"indexed-req\"\nstate = \"shipped\"\n"
        )
        changelog_path = root / "CHANGELOG.md"
        changelog_path.write_text(
            "# Changelog\n\n"
            "- **indexed-slug** — a shipped-and-indexed feature.\n"
            "- some rollup line covering REQ-INDEXED-REQ-001 without the slug.\n"
        )

        global STORY_DIR, TRACE_TOML, CHANGELOG_PATH
        orig = (STORY_DIR, TRACE_TOML, CHANGELOG_PATH)
        STORY_DIR, TRACE_TOML, CHANGELOG_PATH = story_dir, trace_path, changelog_path
        try:
            rep = Report()
            check_story_done_changelog_missing(rep)
        finally:
            STORY_DIR, TRACE_TOML, CHANGELOG_PATH = orig

        hits = [v for v in rep.violations if v.category == "story-done-changelog-missing"]
        ok = len(hits) == 1 and "1-2-missing.md" in str(hits[0].path)
        print(
            "spec-lint --self-test (story-done-changelog-missing): "
            + ("PASS — fires on done-not-indexed, silent on slug/REQ-id/rollup/pre-ship "
               "matches" if ok
               else f"FAIL — expected exactly 1 hit on 'missing', got "
                    f"{[(str(v.path), v.detail) for v in hits]}")
        )
        return ok


def _self_test_dead_cargo_citation() -> bool:
    """Synthetic-fixture probe for `dead-cargo-citation` (bug-log `#168`).

    Four cases, because the rule has four ways to be wrong: miss a dead
    citation, flag a live one, flag a deliberately quoted dead one, and — the
    one that actually bit — resolve a package to the wrong directory.
    """
    import tempfile

    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        # A synthetic crate whose PACKAGE NAME differs from its DIRECTORY name,
        # which is the real shape in this workspace (`trading_core` in
        # `crates/core`) and the shape that made the first run of this rule
        # report a false dead citation.
        crate = tmp / "core"
        (crate / "tests").mkdir(parents=True)
        (crate / "Cargo.toml").write_text('[package]\nname = "trading_core"\n', encoding="utf-8")
        (crate / "tests" / "trybuild.rs").write_text("// live target\n", encoding="utf-8")
        pkg_dirs = {"trading_core": crate}

        def hits_for(body: str) -> list[Violation]:
            md = tmp / "doc.md"
            md.write_text(body, encoding="utf-8")
            r = Report()
            check_cargo_citations([], r, pkg_dirs=pkg_dirs, extra_files=[md])
            return [v for v in r.violations if v.category == "dead-cargo-citation"]

        live = hits_for("run `cargo test -p trading_core --test trybuild` to check\n")
        dead = hits_for("run `cargo test -p trading_core --test nope` to check\n")
        quoted = hits_for(
            "it used to say `cargo test -p trading_core --test nope` "
            "<!-- dead-citation-ok: quoted -->\n"
        )
        unknown_pkg = hits_for("run `cargo test -p no_such_pkg --test whatever`\n")
        # The fenced-block form: marker two lines above, inside neither the
        # fence nor the command. This is how a quoted command is exempted
        # without altering the text being quoted.
        fenced = hits_for(
            "<!-- dead-citation-ok: quoted below -->\n"
            "```bash\n"
            "cargo test -p trading_core --test nope\n"
            "```\n"
        )
        # And the negative control for the window: a marker THREE lines above
        # must NOT exempt, or the window would silently swallow real citations.
        too_far = hits_for(
            "<!-- dead-citation-ok: too far -->\n"
            "\n"
            "```bash\n"
            "cargo test -p trading_core --test nope\n"
            "```\n"
        )

        checks = [
            ("live citation silent (package name != directory name)", len(live) == 0),
            ("dead citation flagged", len(dead) == 1),
            ("quoted dead citation exempt", len(quoted) == 0),
            ("unknown package flagged", len(unknown_pkg) == 1),
            ("fenced quote exempt via marker 2 lines above", len(fenced) == 0),
            ("marker 3 lines above does NOT exempt", len(too_far) == 1),
        ]
        ok = all(passed for _, passed in checks)
        detail = ", ".join(f"{name}={'ok' if passed else 'FAIL'}" for name, passed in checks)
        print(
            "spec-lint --self-test (dead-cargo-citation): "
            + (f"PASS — {detail}" if ok else f"FAIL — {detail}")
        )
        return ok


def _self_test_root_markdown_scanned() -> bool:
    """Synthetic-fixture probe for the root-markdown enumeration (bug-log `#187`).

    The rule under test is not `check_dead_links` — that one already worked. It
    is `default_roots`: whether the repo's root-level markdown is WALKED. So the
    two sides differ in exactly that dimension and expect different outcomes:
    the same dead link, in the same file, is flagged when the root glob is in
    the roots and missed when it is not. A fixture that only asserted "dead link
    flagged" would have passed for three years against roots that never looked
    at the file.
    """
    import tempfile

    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        (tmp / "docs").mkdir()
        # `manual/` stands for a tree nobody added to a hand-written list — which
        # is what `doc/` was for four months.
        (tmp / "manual").mkdir()
        (tmp / "TODO.md").write_text("see [gone](spec/v1/gone.md)\n", encoding="utf-8")
        (tmp / "README.md").write_text("see [here](docs/live.md)\n", encoding="utf-8")
        (tmp / "docs" / "live.md").write_text("# live\n", encoding="utf-8")
        (tmp / "manual" / "guide.md").write_text("see [vanished](../spec/x.md)\n", encoding="utf-8")

        def dead_links_for(roots: list[Path]) -> list[Violation]:
            r = Report()
            for md in iter_tree_md(roots):
                check_dead_links(md, md.read_text(encoding="utf-8"), r)
            return [v for v in r.violations if v.category == "dead-link"]

        with_root = dead_links_for(default_roots(tmp))
        trees_only = dead_links_for([tmp / "docs", tmp / "evidence", tmp / "_bmad-output"])

        found = {v.detail.rsplit("/", 1)[-1] for v in with_root}
        ok_flagged = found == {"gone.md", "x.md"}
        ok_missed = trees_only == []
        ok_live_silent = all("live.md" not in v.detail for v in with_root)

    print(
        "spec-lint --self-test (root-markdown-scanned): "
        + (
            "PASS — root-level AND unlisted-subtree dead links flagged via "
            "default_roots=ok, both invisible to the three hand-listed trees=ok, "
            "live root link silent=ok"
            if (ok_flagged and ok_missed and ok_live_silent)
            else f"FAIL — flagged={ok_flagged} missed-without-root-glob={ok_missed} "
            f"live-silent={ok_live_silent}"
        )
    )
    return ok_flagged and ok_missed and ok_live_silent


def _scrub_git_env() -> None:
    """Remove every `GIT_*` variable from `os.environ` (self-test use only).

    A pre-commit hook is handed git's own environment, and under `git commit -a`
    (or `commit <paths>`) GIT_INDEX_FILE is an ABSOLUTE path to the index.lock of
    the commit being made — measured 2026-11-10, git 2.50.1. Inherited by the
    gitignore probe's fixture, `git add -f` wrote `.gitignore`, `pattern/doc.md`
    and `tracked/doc.md` into that index, the fixture commit then failed, and the
    probe reported "git unusable": wrong repository, wrong diagnosis (measured, bug-log
    `#195`). The LIVE `drop_gitignored(REPO_ROOT)` path deliberately keeps the
    variable: inside a hook, the index being committed is the right one to ask.
    """
    for key in [k for k in os.environ if k.startswith("GIT_")]:
        del os.environ[key]


def _self_test_gitignored_not_walked() -> bool:
    """Run the probe under the environment a pre-commit hook is handed.

    The probe is a real victim index aimed at by GIT_INDEX_FILE; the subject is
    `_scrub_git_env()`. Remove the scrub and the victim is written, which the probe
    reports as its own check rather than as a fixture failure.
    """
    import tempfile

    clean = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    with tempfile.TemporaryDirectory() as vd:
        victim = Path(vd).resolve()
        try:
            subprocess.run(["git", "init", "-q", str(victim)], env=clean,
                           capture_output=True, check=True)
            (victim / "victim.txt").write_text("v", encoding="utf-8")
            subprocess.run(["git", "-C", str(victim), "add", "victim.txt"], env=clean,
                           capture_output=True, check=True)
        except (OSError, subprocess.CalledProcessError) as exc:
            print(
                "spec-lint --self-test (gitignored-not-walked): "
                f"FAIL — victim index could not be built, git unusable ({exc})"
            )
            return False
        sentinel = victim / ".git" / "index"
        before = sentinel.read_bytes()
        saved = dict(os.environ)
        try:
            os.environ["GIT_INDEX_FILE"] = str(sentinel)  # the probe: what a hook is handed
            _scrub_git_env()  # the subject
            return _gitignored_not_walked_probe(sentinel, before)
        finally:
            os.environ.clear()
            os.environ.update(saved)


def _gitignored_not_walked_probe(sentinel: Path, sentinel_before: bytes) -> bool:
    """Two-sided probe for the gitignore filter (bug-log `#189`).

    The dimension under test is IGNORED-ness and nothing else: two sibling
    subtrees of one throwaway repository hold a markdown file with the SAME dead
    link, and the only thing separating them is a line in `.gitignore`. The
    ignored one must be silent and the tracked one must fire. A fixture
    asserting only "dead link flagged" would pass against a filter that drops
    everything; one asserting only "ignored file silent" would pass against a
    filter that drops nothing, or against a fixture whose file was never a
    candidate in the first place. So the pair is checked, AND the filter is
    probed directly to prove the ignored file WAS enumerated and that the filter
    is what removed it.

    The third side is the one that keeps this from silencing repo content: a
    TRACKED file matching an ignore pattern (`*.md` here, force-added) must
    survive. That is `git check-ignore`'s index-aware default, which is why this
    probe runs real `git init` + real `git check-ignore` instead of a
    reimplementation of ignore semantics.
    """
    import tempfile

    def git(*args: str, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", *args],
            cwd=str(cwd) if cwd else None,
            capture_output=True,
            text=True,
            check=True,
        )

    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td).resolve()
        try:
            git("init", "-q", str(tmp))
            # Four subtrees, one dead link each, differing ONLY in ignore/index
            # state. `/pattern/*.md` is deliberately a pattern that matches a file
            # which is then force-added, so surviving it can only be the index
            # taking precedence.
            (tmp / ".gitignore").write_text(
                "/local/\n/pattern/*.md\n", encoding="utf-8"
            )
            for sub in ("local", "pattern", "fresh", "tracked"):
                (tmp / sub).mkdir()
                (tmp / sub / "doc.md").write_text(
                    "see [gone](../nope.md)\n", encoding="utf-8"
                )
            # `fresh/` is added to NEITHER .gitignore nor the index: untracked is
            # not ignored, and that distinction is what the pre-commit hook rests
            # on — a markdown file written this minute has to be linted before it
            # can be staged.
            git("-C", str(tmp), "add", "-f", ".gitignore", "tracked/doc.md", "pattern/doc.md")
            # Every `-c` here pins something this fixture must NOT inherit from the
            # machine. `commit.gpgsign` is the one that bit: it is `true` globally on
            # the dev box with `gpg.format = ssh` and the key served by the
            # 1Password SSH agent, so when that agent naps — a known, recurring
            # failure here — `git commit` returns 128 with "1Password: failed to fill
            # whole buffer" and THIS GATE GOES RED FOR A REASON OUTSIDE ITS SUBJECT.
            # It passed in the morning and failed in the afternoon on an unchanged
            # tree, which reads as flakiness and earns a retry instead of a fix
            # (bug-log `#192`). `core.hooksPath` is pinned for the same class of
            # reason, pre-emptively: there is no global one today, and a fixture
            # that silently runs the repo's governance hooks would be absurd.
            git(
                "-C", str(tmp),
                "-c", "user.email=t@t", "-c", "user.name=t",
                "-c", "commit.gpgsign=false",
                "-c", "core.hooksPath=/dev/null",
                "commit", "-qm", "init",
            )
        except (OSError, subprocess.CalledProcessError) as exc:
            written = sentinel.read_bytes() != sentinel_before
            print(
                "spec-lint --self-test (gitignored-not-walked): "
                f"FAIL — fixture could not be built ({exc}); the hook's index was "
                + ("WRITTEN — the GIT_* scrub is not working" if written else "untouched")
            )
            return False

        def dead_link_paths(roots: list[Path]) -> set[str]:
            r = Report()
            for md in iter_tree_md(roots, repo_root=tmp):
                check_dead_links(md, md.read_text(encoding="utf-8"), r)
            return {Path(v.path).as_posix() for v in r.violations if v.category == "dead-link"}

        found = dead_link_paths(default_roots(tmp))
        ok_ignored_silent = not any(f.endswith("local/doc.md") for f in found)
        ok_tracked_flagged = any(f.endswith("tracked/doc.md") for f in found)
        ok_untracked_flagged = any(f.endswith("fresh/doc.md") for f in found)
        ok_pattern_tracked_flagged = any(f.endswith("pattern/doc.md") for f in found)

        # Probe the gate: the ignored file must have been a candidate on disk and
        # the filter must be what dropped it. Without this, "silent" is also what
        # a missing fixture looks like.
        candidates = sorted(tmp.rglob("*.md"))
        kept = drop_gitignored(candidates, tmp)
        ok_filter_bites = (
            any(c.as_posix().endswith("local/doc.md") for c in candidates)
            and not any(k.as_posix().endswith("local/doc.md") for k in kept)
        )
    ok_hook_index_untouched = sentinel.read_bytes() == sentinel_before
    checks = [
        ("ignored subtree silent", ok_ignored_silent),
        ("tracked sibling flagged", ok_tracked_flagged),
        ("untracked-but-not-ignored flagged", ok_untracked_flagged),
        ("tracked file beats its ignore pattern", ok_pattern_tracked_flagged),
        ("filter is what dropped it", ok_filter_bites),
        ("hook's GIT_INDEX_FILE never written", ok_hook_index_untouched),
    ]
    ok = all(passed for _, passed in checks)
    detail = ", ".join(f"{name}={'ok' if passed else 'FAIL'}" for name, passed in checks)
    print(
        "spec-lint --self-test (gitignored-not-walked): "
        + (f"PASS — {detail}" if ok else f"FAIL — {detail}")
    )
    return ok


_TRIAD_COUNTED = (
    "trace-ref-unparsed",
    "trace-ref-dangling",
    "trace-state-annotation-drift",
    "status-drift",
    "story-done-trace-drift",
    "story-done-changelog-missing",
)
# The join-key guards, whose details the table below checks by substring.
_TRIAD_GUARDS = _TRIAD_COUNTED[:3]


def _self_test_trace_ref_unparsed() -> bool:
    """Fixture table for the triad's join-key guards (bug-log `#216`, `#219`).

    One story (`Status: done`, except O and P), one trace.toml, one CHANGELOG.
    The forms differ in exactly one thing, the story's `- Trace:` line — except
    J, which keeps B's line and drops the slug from the CHANGELOG, and O/P,
    which are N/B on a `Status: review` story. Every count is taken from
    `run_story_triad`, the function main() calls, as the tuple
    (trace-ref-unparsed, trace-ref-dangling, trace-state-annotation-drift,
    status-drift, story-done-trace-drift, story-done-changelog-missing) — so
    removing any of its six calls turns a row red.

    B/N is the `#219` pair: the same line, differing only in the annotation —
    B's `passed` equals the row's and is silent, N's `shipped` does not and
    fires. Every other parsed form also annotates `passed`, so each is a
    further silent case. P/O is the same pair on a `review` story, where no
    other rule fires: O isolates the annotation rule, and the pair pins that it
    reads every story, not only done ones.

    The expectations are DERIVED from the construction, not measured off the
    subject: the row's `passed` maps to `review` (not the story's `done`) and is
    not shipped-terminal, so a done story the downstream rules CAN see trips
    both status-drift and story-done-trace-drift, one they cannot see trips
    neither, and a `review` story trips neither (and no changelog check, which
    reads done stories only). Both facts are asserted as preconditions first, so a later mapping
    change reports "fixture invalid" instead of passing for the wrong reason.
    A is story 2-63's pre-Close line (the incident); A/B is the requested pair.

    D is the declared gap, the one skip the triad allows on purpose, so its
    regex is pinned from the REJECTING side too: K, L and M each differ from D
    in one dimension (no dash and no reason; an ASCII hyphen; an em dash with no
    reason) and must fire. Without them any loosening of TRACE_NONE_RE stays
    green (the `#161` two-fixture rule). Every fixture's trace line is line 7,
    and the detail substrings carry that number, so an off-by-one in either
    rule's line count is red too.
    """
    import tempfile

    indexed = "# Changelog\n\n- **fixture** — a fixture feature.\n"
    unindexed = "# Changelog\n\n- **something-else** — another feature.\n"
    parsed = "- Trace: `REQ-FIXTURE-001` (state=`passed`)"
    # The fixture's `- Trace:` line is line 7: heading, blank, Status, blank,
    # `### References`, blank, line.
    neither = "line 7 is in neither accepted form"
    # name: (the `- Trace:` line(s), CHANGELOG, expected counts, detail substring)
    forms: dict[str, tuple[str, str, tuple[int, ...], str | None]] = {
        "A": ("- Trace: `REQ-FIXTURE-001` (state=`passed` since 2026-11-10)",
              indexed, (1, 0, 0, 0, 0, 0), neither),
        "B": (parsed, indexed, (0, 0, 0, 1, 1, 0), None),
        "C": (parsed + " — `in-progress` from 2026-11-10; `shipped` at Close.",
              indexed, (0, 0, 0, 1, 1, 0), None),
        "D": ("- Trace: none — known trace-coverage gap", indexed, (0, 0, 0, 0, 0, 0), None),
        "E": ("- Trace: REQ-FIXTURE-001 (state=passed)", indexed, (1, 0, 0, 0, 0, 0), neither),
        "F": ("", indexed, (1, 0, 0, 0, 0, 0), "no `- Trace:` line"),
        "G": (parsed + "\n- Trace: `REQ-OTHER-001` (state=`shipped`)",
              indexed, (1, 0, 0, 1, 1, 0), "2 `- Trace:` lines (lines 7, 8)"),
        "H": ("- Trace: `none` (state=`passed`)", indexed, (1, 0, 0, 0, 0, 0), neither),
        "I": ("- Trace: `REQ-MISSING-001` (state=`passed`)", indexed, (0, 1, 0, 0, 0, 0),
              "line 7 names REQ-MISSING-001, which has no [[req]] row"),
        "J": (parsed, unindexed, (0, 0, 0, 1, 1, 1), None),
        # Near misses of D, each one dimension off; all must fire.
        "K": ("- Trace: none", indexed, (1, 0, 0, 0, 0, 0), neither),
        "L": ("- Trace: none - known trace-coverage gap", indexed, (1, 0, 0, 0, 0, 0), neither),
        "M": ("- Trace: none —", indexed, (1, 0, 0, 0, 0, 0), neither),
        # B with a stale annotation: the row says `passed` (bug-log #219).
        "N": ("- Trace: `REQ-FIXTURE-001` (state=`shipped`)", indexed, (0, 0, 1, 1, 1, 0),
              "line 7 annotates REQ-FIXTURE-001 as (state=`shipped`), but its trace.toml "
              "row says state='passed'"),
        # The same pair on a `Status: review` story, which `passed` maps to: no
        # other rule fires, so O isolates the annotation rule, and the pair pins
        # that the rule reads every story, not only done ones.
        "O": ("- Trace: `REQ-FIXTURE-001` (state=`shipped`)", indexed, (0, 0, 1, 0, 0, 0),
              "line 7 annotates REQ-FIXTURE-001 as (state=`shipped`)"),
        "P": (parsed, indexed, (0, 0, 0, 0, 0, 0), None),
    }
    status_of = {"O": "review", "P": "review"}  # every other form: done
    compliant = {"D", "P"}  # the only forms expected to be silent

    fixture_valid = (
        STATE_TO_STORY_STATUS.get("passed") == "review"
        and "passed" not in _DONE_TERMINAL_STATES
    )

    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        story_dir = root / "stories"
        story_dir.mkdir()
        story = story_dir / "1-1-fixture.md"
        trace_path = root / "trace.toml"
        trace_path.write_text(
            '[[req]]\nid = "REQ-FIXTURE-001"\nfeature = "fixture"\nstate = "passed"\n\n'
            '[[req]]\nid = "REQ-OTHER-001"\nfeature = "other"\nstate = "shipped"\n',
            encoding="utf-8",
        )
        changelog_path = root / "CHANGELOG.md"

        def hits_for(
            line: str, changelog: str, status: str
        ) -> tuple[int, tuple[int, ...], list[str]]:
            story.write_text(
                f"# Story 1.1: fixture\n\nStatus: {status}\n\n### References\n\n{line}\n",
                encoding="utf-8",
            )
            changelog_path.write_text(changelog, encoding="utf-8")
            global STORY_DIR, TRACE_TOML, CHANGELOG_PATH
            orig = (STORY_DIR, TRACE_TOML, CHANGELOG_PATH)
            STORY_DIR, TRACE_TOML, CHANGELOG_PATH = story_dir, trace_path, changelog_path
            try:
                candidates = len(iter_stories())
                rep = Report()
                run_story_triad(rep)
            finally:
                STORY_DIR, TRACE_TOML, CHANGELOG_PATH = orig
            counts = tuple(
                sum(1 for v in rep.violations if v.category == c) for c in _TRIAD_COUNTED
            )
            details = [v.detail for v in rep.violations if v.category in _TRIAD_GUARDS]
            return candidates, counts, details

        got = {
            name: hits_for(line, cl, status_of.get(name, "done"))
            for name, (line, cl, _, _) in forms.items()
        }

    not_candidate = sorted(n for n, (c, _, _) in got.items() if c != 1)
    table_wrong = {
        n: (got[n][1], exp) for n, (_, _, exp, _) in forms.items() if got[n][1] != exp
    }
    misclassified = sorted(
        n for n, (_, _, _, sub) in forms.items()
        if sub is not None and not any(sub in d for d in got[n][2])
    )
    silent = sorted(n for n in forms if n not in compliant and sum(got[n][1]) == 0)
    unlisted_cats = [c for c in _TRIAD_GUARDS if c not in CATEGORIES]

    checks = [
        ("fixture valid (passed -> review, non-terminal)", fixture_valid),
        ("every form a candidate (1 story)", not not_candidate),
        ("table exact", not table_wrong),
        ("classification pinned per form", not misclassified),
        ("never silent except the declared gap and P", not silent),
        ("all three guard categories in CATEGORIES", not unlisted_cats),
    ]
    ok = all(passed for _, passed in checks)
    detail = ", ".join(f"{name}={'ok' if passed else 'FAIL'}" for name, passed in checks)
    if not ok:
        detail += (
            f" | not-candidate={not_candidate} table (got, expected)={table_wrong} "
            f"misclassified={misclassified} silent={silent} missing-categories={unlisted_cats}"
        )
    print(
        "spec-lint --self-test (trace-ref-unparsed): "
        + (f"PASS — {len(forms)} forms {min(forms)}-{max(forms)}, {detail}" if ok
           else f"FAIL — {detail}")
    )
    return ok


def _run_copy_on_fixture(line: str, source: str | None = None) -> tuple[int, list[str], str]:
    """Run a copy of this script, as the hook does, over a throwaway repo layout.

    `line` is story 1-1's `- Trace:` line; the rest of the fixture is fixed (see
    `_self_test_main_runs_story_triad`). `source` replaces the copied script's
    text — only `_self_test_unlisted_category_printed` passes it, to run a copy
    whose CATEGORIES omits one emitted category. Every `GIT_*` variable is
    removed from the child's environment (the `#195` leak: a hook hands its
    children an absolute GIT_INDEX_FILE). The temp directory is a repository
    LAYOUT, not a git repository; the child's gitignore filter notes exit 128
    on stderr and drops nothing, which the story rules do not depend on.
    Returns (exit code, stdout lines, stderr).
    """
    import shutil
    import tempfile

    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    with tempfile.TemporaryDirectory() as td:
        root = Path(td).resolve()
        (root / "scripts").mkdir()
        script = root / "scripts" / "spec_lint.py"
        if source is None:
            shutil.copyfile(Path(__file__).resolve(), script)
        else:
            script.write_text(source, encoding="utf-8")
        stories = root / "_bmad-output" / "implementation-artifacts"
        stories.mkdir(parents=True)
        planning = root / "_bmad-output" / "planning-artifacts"
        planning.mkdir()
        (stories / "1-1-fixture.md").write_text(
            f"# Story 1.1: fixture\n\nStatus: done\n\n### References\n\n{line}\n",
            encoding="utf-8",
        )
        (stories / "1-2-typo.md").write_text(
            "# Story 1.2: typo\n\nStatus: done\n\n### References\n\n"
            "- Trace: `REQ-TYPO-001` (state=`shipped`)\n",
            encoding="utf-8",
        )
        # 1-3: a done story whose annotation is stale against its shipped row,
        # which nothing else in the fixture flags (bug-log #219).
        (stories / "1-3-stale.md").write_text(
            "# Story 1.3: stale\n\nStatus: done\n\n### References\n\n"
            "- Trace: `REQ-STALE-001` (state=`scoped`)\n",
            encoding="utf-8",
        )
        (stories / "sprint-status.yaml").write_text(
            "development_status:\n  1-1-fixture: done\n  1-2-typo: done\n  1-3-stale: done\n",
            encoding="utf-8",
        )
        (planning / "trace.toml").write_text(
            '[[req]]\nid = "REQ-FIXTURE-001"\nfeature = "fixture"\nstate = "passed"\n\n'
            '[[req]]\nid = "REQ-STALE-001"\nfeature = "stale"\nstate = "shipped"\n',
            encoding="utf-8",
        )
        (root / "CHANGELOG.md").write_text(
            "# Changelog\n\n- **fixture** — a fixture feature.\n"
            "- **stale** — a shipped fixture feature.\n",
            encoding="utf-8",
        )
        proc = subprocess.run(
            [sys.executable, "-I", str(script)],
            cwd=str(root), env=env, capture_output=True, text=True,
            check=False, timeout=120,
        )
        return proc.returncode, proc.stdout.splitlines(), proc.stderr


def _report_section(out: list[str], cat: str) -> tuple[str | None, str]:
    """(header line, the line after it) of `cat`'s section in printed output."""
    for i, line in enumerate(out):
        if line.startswith(f"{cat} ("):
            return line, (out[i + 1] if i + 1 < len(out) else "")
    return None, ""


def _self_test_main_runs_story_triad() -> bool:
    """Pin main()'s wiring end to end (bug-log `#216`; the `#190` wiring rule).

    Every other triad self-test calls `run_story_triad` directly, so deleting
    its call from main() would leave them all green while the hook and CI —
    which run main() — went blind, and the live tree (0 malformed lines) would
    still print PASS. So this runs a copy of THIS file over a throwaway
    repository layout the way the hook does (`_run_copy_on_fixture`), then
    reads the printed report.

    Two runs, the two sides differing only in story 1-1's `- Trace:` line:
    story 2-63's pre-Close form must print a `trace-ref-unparsed (1):` section
    naming the story; the parsed form must print no such section, and must print
    the status-drift and story-done-trace-drift sections (the row says
    `passed`, the story says `done`). Both sides also carry story 1-2, `done`,
    whose line names `REQ-TYPO-001` (no such row, slug not in the CHANGELOG), so
    both must print `trace-ref-dangling (1):` and
    `story-done-changelog-missing (1):`. And both carry story 1-3, `done`,
    annotated `scoped` against its `shipped` row and flagged by nothing else, so
    both must print `trace-state-annotation-drift (1):` naming it (bug-log
    `#219`). Every one of `run_story_triad`'s six
    categories is therefore asserted through main(): a main() that inlines some
    of the calls again instead of calling `run_story_triad` is red, not only one
    that drops the call. Each header is matched exactly, so this also proves
    each category prints as a listed one, not only that it is counted.
    """
    run, section = _run_copy_on_fixture, _report_section
    try:
        rc_bad, out_bad, err_bad = run(
            "- Trace: `REQ-FIXTURE-001` (state=`passed` since 2026-11-10)"
        )
        rc_good, out_good, err_good = run("- Trace: `REQ-FIXTURE-001` (state=`passed`)")
    except (OSError, subprocess.SubprocessError) as exc:
        print(f"spec-lint --self-test (main-wiring): FAIL — could not run the copy ({exc})")
        return False

    head, first = section(out_bad, "trace-ref-unparsed")
    story_rel = "_bmad-output/implementation-artifacts/1-1-fixture.md"
    stale_rel = "_bmad-output/implementation-artifacts/1-3-stale.md"
    both = (out_bad, out_good)
    checks = [
        ("unparsed line -> `trace-ref-unparsed (1):` printed",
         head == "trace-ref-unparsed (1):"),
        ("...naming the story", f"[trace-ref-unparsed] {story_rel}:" in first),
        ("...and a non-zero exit", rc_bad != 0),
        ("parsed line -> no trace-ref-unparsed section",
         section(out_good, "trace-ref-unparsed")[0] is None),
        ("...status-drift (1) printed", section(out_good, "status-drift")[0] == "status-drift (1):"),
        ("...story-done-trace-drift (1) printed",
         section(out_good, "story-done-trace-drift")[0] == "story-done-trace-drift (1):"),
        ("both sides: trace-ref-dangling (1) printed",
         all(section(o, "trace-ref-dangling")[0] == "trace-ref-dangling (1):" for o in both)),
        ("both sides: story-done-changelog-missing (1) printed",
         all(section(o, "story-done-changelog-missing")[0] == "story-done-changelog-missing (1):"
             for o in both)),
        ("both sides: trace-state-annotation-drift (1) printed, naming 1-3",
         all(sad[0] == "trace-state-annotation-drift (1):"
             and f"[trace-state-annotation-drift] {stale_rel}:" in sad[1]
             for sad in (section(o, "trace-state-annotation-drift") for o in both))),
    ]
    ok = all(passed for _, passed in checks)
    detail = ", ".join(f"{name}={'ok' if passed else 'FAIL'}" for name, passed in checks)
    if not ok:
        # stderr too: a crashed copy must read as a crash, not as a wiring
        # regression with an empty stdout (the `#189` hazard).
        detail += (
            f" | unparsed side rc={rc_bad} stdout={out_bad[:6]}"
            f" stderr={err_bad[-300:]!r}"
            f" | parsed side rc={rc_good} stdout={out_good[:8]}"
            f" stderr={err_good[-300:]!r}"
        )
    print(
        "spec-lint --self-test (main-wiring): "
        + (f"PASS — {detail}" if ok else f"FAIL — {detail}")
    )
    return ok


def _self_test_unlisted_category_printed() -> bool:
    """A stored category missing from CATEGORIES still prints (bug-log `#220`).

    The pair, in process, through `render_report`: one report holds one
    violation in a listed category and one in a category CATEGORIES does not
    name; both sections must print, the unlisted one after every listed one and
    marked UNLISTED_CATEGORY_MARK. The listed section, and the whole output of a
    report holding only the listed violation, must equal what the pre-#220
    printer produced: ``"", f"{cat} ({n}):"`` and then each `render` line, under
    the unchanged summary line. That expectation is the old code's format,
    quoted, not the new code's output.

    And end to end, through main(): `_run_copy_on_fixture` runs a copy of this
    file whose CATEGORIES omits `trace-ref-dangling` — the shape of `#216`'s
    probe P9, which printed `spec-lint: FAIL (1 violations in 1 categories)` and
    no section. The fixture's story 1-2 fires that category on purpose, so the
    copy must print it, marked, while the listed status-drift section prints as
    before. A main() that prints with its own CATEGORIES-only loop again is red.
    """
    listed, unlisted = "status-drift", "zz-unlisted-fixture-category"
    fixture_valid = listed in CATEGORIES and unlisted not in CATEGORIES
    root = REPO_ROOT
    v_listed = Violation(listed, root / "a.md", "listed detail")
    v_unlisted = Violation(unlisted, root / "b.md", "unlisted detail")

    both_rep = Report([v_unlisted, v_listed])  # stored unlisted-first on purpose
    both_lines, both_rc = render_report(both_rep, root)
    only_rep = Report([v_listed])
    only_lines, only_rc = render_report(only_rep, root)

    old_listed_section = ["", f"{listed} (1):", v_listed.render(root)]
    old_only_output = ["spec-lint: FAIL (1 violations in 1 categories)", *old_listed_section]
    unlisted_head = f"{unlisted} (1) {UNLISTED_CATEGORY_MARK}:"

    def at(lines: list[str], head: str) -> int:
        return lines.index(head) if head in lines else -1

    i_listed, i_unlisted = at(both_lines, f"{listed} (1):"), at(both_lines, unlisted_head)

    # End to end: drop one emitted category from a copy's CATEGORIES.
    src = Path(__file__).read_text(encoding="utf-8")
    start = src.find("\nCATEGORIES = (\n")
    end = src.find("\n)\n", start) if start >= 0 else -1
    target = '\n    "trace-ref-dangling",'
    edit_ok = start >= 0 and end > start and src[start:end].count(target) == 1
    e2e_out: list[str] = []
    e2e_err = ""
    if edit_ok:
        copy = src[:start] + src[start:end].replace(target, "", 1) + src[end:]
        try:
            _, e2e_out, e2e_err = _run_copy_on_fixture(
                "- Trace: `REQ-FIXTURE-001` (state=`passed`)", source=copy
            )
        except (OSError, subprocess.SubprocessError) as exc:
            e2e_err = f"could not run the copy ({exc})"
    e2e_head, e2e_first = _report_section(e2e_out, "trace-ref-dangling")
    e2e_listed = _report_section(e2e_out, "status-drift")[0]
    typo_rel = "_bmad-output/implementation-artifacts/1-2-typo.md"

    checks = [
        ("fixture valid (one listed, one unlisted category)", fixture_valid),
        ("unlisted section printed, marked", i_unlisted >= 0
         and both_lines[i_unlisted + 1:i_unlisted + 2] == [v_unlisted.render(root)]),
        ("listed section printed as before",
         i_listed >= 1 and both_lines[i_listed - 1:i_listed + 2] == old_listed_section),
        ("unlisted after every listed section", 0 <= i_listed < i_unlisted),
        ("summary + exit code count both", both_lines[0]
         == "spec-lint: FAIL (2 violations in 2 categories)" and both_rc == 2),
        ("listed-only output byte-identical to the old printer",
         only_lines == old_only_output and only_rc == 1),
        ("e2e: copy's CATEGORIES edit applied", edit_ok),
        ("e2e: dropped category printed, marked, naming the story",
         e2e_head == f"trace-ref-dangling (1) {UNLISTED_CATEGORY_MARK}:"
         and f"[trace-ref-dangling] {typo_rel}:" in e2e_first),
        ("e2e: listed status-drift printed as before", e2e_listed == "status-drift (1):"),
    ]
    ok = all(passed for _, passed in checks)
    detail = ", ".join(f"{name}={'ok' if passed else 'FAIL'}" for name, passed in checks)
    if not ok:
        detail += f" | in-process={both_lines} | e2e stdout={e2e_out[:10]} stderr={e2e_err[-300:]!r}"
    print(
        "spec-lint --self-test (unlisted-category-printed): "
        + (f"PASS — {detail}" if ok else f"FAIL — {detail}")
    )
    return ok


def self_test() -> int:
    """Run every rule's synthetic-fixture self-test. Exit 0 iff all pass.

    The registry guard at the end is the `#190` wiring rule applied to this
    function itself: a `_self_test_*` that is defined but not listed here runs
    nowhere, and nothing else would notice (bug-log `#216`). It is two-sided:
    every test that just ran must also be visible to its scan, because a scan
    that sees nothing finds "nothing unlisted" and would print PASS (`#187`).
    """
    tests = [
        _self_test_status_drift,
        _self_test_story_done_trace_drift,
        _self_test_story_done_changelog_missing,
        _self_test_dead_cargo_citation,
        _self_test_root_markdown_scanned,
        _self_test_gitignored_not_walked,
        _self_test_trace_ref_unparsed,
        _self_test_main_runs_story_triad,
        _self_test_unlisted_category_printed,
    ]
    results = [t() for t in tests]
    defined = sorted(
        n for n, f in globals().items() if n.startswith("_self_test_") and callable(f)
    )
    listed = {t.__name__ for t in tests}
    unlisted = sorted(set(defined) - listed)
    unseen = sorted(listed - set(defined))  # a blind scan proves nothing (#187)
    print(
        "spec-lint --self-test (registry): "
        + (f"PASS — all {len(defined)} defined _self_test_* functions ran"
           if not unlisted and not unseen
           else f"FAIL — defined but never run: {unlisted}; "
                f"ran but invisible to the scan: {unseen}")
    )
    results.append(not unlisted and not unseen)
    return 0 if all(results) else 1


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------

def drop_gitignored(paths: list[Path], repo_root: Path = REPO_ROOT) -> list[Path]:
    """Drop the paths `git` ignores, so the verdict is a function of COMMITTED content.

    Until 2026-11-10 this filter did not exist and `default_roots` walked the
    working tree exactly as it sat on disk. On a box carrying any agent harness,
    editor directory or local virtualenv beside the repo, the lint read those
    files as repo documentation. Measured here: 52 dead links in exactly two
    gitignored paths — `/.agents/` (37, a second vendored BMAD skills install
    whose twin under `.claude/` is excluded BY NAME in `UNWALKED_TREES` for the
    same 37 links) and `/AGENTS.md` (15, a Codex harness file citing a `.Codex/`
    tree that does not exist here). Neither path is tracked, so CI — which
    checks out tracked files only — printed PASS on the very commit where the
    pre-commit hook printed 52 violations and refused it. A gate that is red
    locally and green remotely gets `--no-verify`, which is the failure mode the
    hook exists to prevent (bug-log `#189`).

    `git check-ignore` is index-aware by default: a TRACKED file matching an
    ignore pattern comes back NOT ignored, so this filter cannot silence repo
    content. `.claude/` is tracked (253 files) and is untouched by it — that
    exclusion stays hand-written and costed in `UNWALKED_TREES`. If git cannot
    be run at all the filter drops nothing and says so on stderr: over-reporting
    is the safe direction, but a skip has to be visible rather than assumed.
    """
    root_res = repo_root.resolve()

    def rel_or_none(p: Path) -> str | None:
        try:
            return p.resolve().relative_to(root_res).as_posix()
        except ValueError:
            return None

    rels = [r for r in (rel_or_none(p) for p in paths) if r is not None]
    if not rels:
        return list(paths)
    try:
        proc = subprocess.run(
            ["git", "-C", str(root_res), "check-ignore", "-z", "--stdin"],
            input="\0".join(rels),
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as exc:  # git absent
        print(f"  [note] gitignore filter skipped: cannot run git ({exc})", file=sys.stderr)
        return list(paths)
    # 0 = at least one path ignored, 1 = none ignored. Anything else (128: not a
    # repository, bad pathspec) is a filter that did not run, not an empty result.
    if proc.returncode not in (0, 1):
        print(
            "  [note] gitignore filter skipped: `git check-ignore` exited "
            f"{proc.returncode} ({proc.stderr.strip().splitlines()[:1]})",
            file=sys.stderr,
        )
        return list(paths)
    ignored = {line for line in proc.stdout.split("\0") if line}
    if not ignored:
        return list(paths)
    return [p for p in paths if (r := rel_or_none(p)) is None or r not in ignored]


def iter_tree_md(roots: Iterable[Path], repo_root: Path = REPO_ROOT) -> Iterable[Path]:
    candidates: list[Path] = []
    for root in roots:
        if root.is_file() and root.suffix == ".md":
            candidates.append(root)
        elif root.is_dir():
            for p in sorted(root.rglob("*.md")):
                try:
                    rel = p.relative_to(repo_root).as_posix()
                except ValueError:
                    # A root outside the repository — only `--self-test` does
                    # this. Both skips below are repo-specific paths, so there
                    # is nothing to skip.
                    candidates.append(p)
                    continue
                # Skip archived content — frozen by design (this now covers
                # BOTH the pre-existing `docs/archive/` tarball convention AND
                # the Phase 5b `docs/archive/pre-bmad-spec/` retired-spec tree).
                if "archive/" in rel:
                    continue
                # Skip byte-immutable anchored report bodies under the v1
                # corpus — pre-existing convention, unaffected by Phase 5b.
                if rel.startswith("evidence/v1/") and "/reports/" in rel:
                    continue
                candidates.append(p)
    # The hand-written `UNWALKED_TREES` list above cannot name what is not in
    # the repository; this does. Order is preserved so output stays stable.
    yield from drop_gitignored(candidates, repo_root)


# Trees deliberately NOT walked, each with the cost of excluding it MEASURED on
# the day it was excluded. An exclusion with no number beside it is how `#188`
# happened: three trees were listed, and nothing said what the rest contained.
#
# This dict is NOT the whole exclusion story and must not be read as one. It
# carries TRACKED trees (`.claude/` is 253 committed files) and the walk-time
# win of not descending into `target/`. Everything git IGNORES is removed
# separately by `drop_gitignored` below, which is why a dev box's `.agents/`,
# `AGENTS.md` and `lab-runs/` are absent from this list and still not linted
# (bug-log `#189`: they were, for three commits, and cost 52 phantom violations
# that CI never saw).
UNWALKED_TREES: dict[str, str] = {
    ".git": "not content",
    ".codegraph": "gitignored index, not content",
    "target": "build output, untracked",
    "vendor": "upstream source (the operator-locked iced_tiny_skia fork) — bug reports go upstream",
    ".claude": (
        "37 dead links on 2026-11-09, every one inside the vendored BMAD skills install "
        "(bmad-document-project 28, bmad-index-docs 5, bmad-quick-dev 4). CLAUDE.md forbids "
        "editing those in place, so covering this tree is its own task, not a drive-by."
    ),
}


def default_roots(repo_root: Path = REPO_ROOT) -> list[Path]:
    """Every markdown-bearing tree in the repository, minus `UNWALKED_TREES`.

    WALKED, not listed. Until 2026-11-09 this was a hand-written list of three
    directories, so two whole bodies of documentation sat outside every root:
    the repo's front-door markdown (`README.md`, `CLAUDE.md`, `AGENT.md`,
    `CHANGELOG.md`, `TODO.md`) and `doc/`, the user-facing manual. `TODO.md`
    carried seven dead links, `doc/` six — all created by the migration that
    moved their targets — and `doc/` additionally stated the ship-passive thesis
    in the universal form the era-qualification forbids, in nine places, for four
    months. A hand-written list ages exactly like a comment; a walk with named,
    costed exclusions does not (bug-log `#187`, `#188`).
    """
    roots = sorted(repo_root.glob("*.md"))
    roots += [
        p
        for p in sorted(repo_root.iterdir())
        if p.is_dir() and p.name not in UNWALKED_TREES
    ]
    return roots


UNLISTED_CATEGORY_MARK = "(category not in CATEGORIES)"


def render_report(report: Report, root: Path = REPO_ROOT) -> tuple[list[str], int]:
    """The lines main() prints for `report`, and its exit code.

    bug-log `#220`: sections print in CATEGORIES order, and until this fix ONLY
    those printed. `Report.add` stores any category and the summary and exit
    code count every stored one, so a rule whose category was left out of
    CATEGORIES failed the run with no line naming what failed — a gate that
    reads as broken (`#189`), seen live by `#216`'s probe P9. Any other stored
    category now prints after the listed ones, sorted, its header marked
    UNLISTED_CATEGORY_MARK. A report with only listed categories prints exactly
    what it printed before.
    """
    grouped = report.by_category()
    failed_categories = [c for c, vs in grouped.items() if vs]
    if not failed_categories:
        return ["spec-lint: PASS (0 violations)"], 0

    total = sum(len(vs) for vs in grouped.values())
    lines = [f"spec-lint: FAIL ({total} violations in {len(failed_categories)} categories)"]
    unlisted = sorted(c for c in failed_categories if c not in CATEGORIES)
    for cat in [*CATEGORIES, *unlisted]:
        vs = grouped.get(cat, [])
        if not vs:
            continue
        mark = f" {UNLISTED_CATEGORY_MARK}" if cat in unlisted else ""
        lines += ["", f"{cat} ({len(vs)}){mark}:"]
        lines += [v.render(root) for v in vs]
    return lines, len(failed_categories)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "paths",
        nargs="*",
        help="restrict dead-link/frontmatter checks to one or more paths (default: whole tree)",
    )
    parser.add_argument("--all", action="store_true", help="print every category")
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="run every rule's synthetic-fixture self-test and exit (0 = all pass)",
    )
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()

    if not BMAD_OUTPUT_DIR.exists():
        print(f"error: _bmad-output/ not found at {BMAD_OUTPUT_DIR}", file=sys.stderr)
        return 99

    roots = (
        [Path(p).resolve() for p in args.paths] if args.paths else default_roots()
    )
    report = Report()

    for md in iter_tree_md(roots):
        text = md.read_text(encoding="utf-8", errors="replace")
        check_dead_links(md, text, report)
        check_frontmatter(md, text, report)

    # Tree-level checks (only when running over the whole tree).
    if not args.paths:
        check_story_status_values(report)
        check_orphan_stories(report)
        anchors = check_anchors(EVIDENCE_DIR, report)
        check_trace(report, anchors)
        check_story_done_no_tests(report)
        cargo_cites = check_cargo_citations(roots, report)
        # The triad and its join-key guards — pinned by
        # `_self_test_main_runs_story_triad` (bug-log #216).
        run_story_triad(report)

    # Pinned end to end by `_self_test_unlisted_category_printed` (bug-log #220).
    lines, failed = render_report(report)
    for line in lines:
        print(line)
    return failed


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
