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

Story ↔ trace-row resolution is via the REQ-id embedded in the story's own
`### References` block (a "- Trace: `REQ-XXX` (state=`...`)" line), NOT by
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
import re
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
    "trace-broken-path",
    "adr-not-registered",
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
#   - Trace: `REQ-XXX-001` (state=`shipped`)
#   ...
#   - Source feature folder: `spec/[v1/|v2/|v3/]<slug>/` - frontmatter status
#     **`shipped`** (verbatim), ...
# ---------------------------------------------------------------------------

STORY_FILENAME_RE = re.compile(r"^(\d+)-(\d+)-(.+)\.md$")
STATUS_LINE_RE = re.compile(r"^Status:\s*(\S+)", re.MULTILINE)
TRACE_REF_RE = re.compile(r"^- Trace:\s*`([^`]+)`\s*\(state=`([^`]*)`\)", re.MULTILINE)
SOURCE_FOLDER_RE = re.compile(r"Source feature folder:\s*`spec/([^`]+?)/?`")


@dataclass
class Story:
    path: Path
    epic: str
    story_num: str
    filename_slug: str  # derived straight from the filename; may be dot-sanitized/prefixed
    status: str | None
    trace_req_id: str | None       # REQ-id cited by "- Trace:" line, if any
    trace_embedded_state: str | None  # the (state=`...`) annotation on that same line


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
# unrecognised value should not manufacture false positives.
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
            continue  # no resolvable trace row, or malformed story — not this rule's concern
        row = rows_by_id.get(story.trace_req_id)
        if row is None:
            continue  # dangling REQ-id — a trace-broken-path concern, not status-drift
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
            continue  # dangling REQ-id — a trace-broken-path concern
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


def self_test() -> int:
    """Run every rule's synthetic-fixture self-test. Exit 0 iff all pass."""
    results = [
        _self_test_status_drift(),
        _self_test_story_done_trace_drift(),
        _self_test_story_done_changelog_missing(),
    ]
    return 0 if all(results) else 1


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------

def iter_tree_md(roots: Iterable[Path]) -> Iterable[Path]:
    for root in roots:
        if root.is_file() and root.suffix == ".md":
            yield root
        elif root.is_dir():
            for p in sorted(root.rglob("*.md")):
                rel = p.relative_to(REPO_ROOT).as_posix()
                # Skip archived content — frozen by design (this now covers
                # BOTH the pre-existing `docs/archive/` tarball convention AND
                # the Phase 5b `docs/archive/pre-bmad-spec/` retired-spec tree).
                if "archive/" in rel:
                    continue
                # Skip byte-immutable anchored report bodies under the v1
                # corpus — pre-existing convention, unaffected by Phase 5b.
                if rel.startswith("evidence/v1/") and "/reports/" in rel:
                    continue
                yield p


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
        [Path(p).resolve() for p in args.paths]
        if args.paths
        else [DOCS_DIR, EVIDENCE_DIR, BMAD_OUTPUT_DIR]
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
        check_status_drift(report)
        check_story_done_trace_drift(report)
        check_story_done_changelog_missing(report)

    grouped = report.by_category()
    failed_categories = [c for c, vs in grouped.items() if vs]

    if not failed_categories:
        print("spec-lint: PASS (0 violations)")
        return 0

    total = sum(len(vs) for vs in grouped.values())
    print(f"spec-lint: FAIL ({total} violations in {len(failed_categories)} categories)")
    for cat in CATEGORIES:
        vs = grouped.get(cat, [])
        if not vs:
            continue
        print(f"\n{cat} ({len(vs)}):")
        for v in vs:
            print(v.render(REPO_ROOT))

    return len(failed_categories)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
