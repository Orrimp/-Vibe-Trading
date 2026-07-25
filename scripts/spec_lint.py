#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""spec_lint.py — structural integrity check for spec/.

Companion to scripts/verify_anchors.sh (which checks content hashes).
This script checks shape: dead links, missing frontmatter, orphan
feature folders, anchor coverage, trace.toml row validity, pipeline
status drift (deck + PASS report ⇒ status ≥ presenter-done), and
CHANGELOG-index completeness (every shipped feature ⇒ a CHANGELOG.md line).

Exit code = number of violation CATEGORIES that triggered (0 = clean).
Pass --all to print every violation regardless of category count.

Usage:
    uv run scripts/spec_lint.py            # whole spec/ tree (preferred)
    uv run scripts/spec_lint.py spec/<slug>  # restrict to one folder
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
SPEC_DIR = REPO_ROOT / "spec"
# The byte-immutable reports corpus (`*/reports/` dirs) + `anchors.toml`
# `git mv`d from `spec/` to this top-level sibling root in the 2026-07-25
# BMAD-migration Phase 3 (layout preserved 1:1). `feature.md`/`tasks.md`/
# `presentations/` stay under SPEC_DIR until Phase 5b.
EVIDENCE_DIR = REPO_ROOT / "evidence"
# Project-knowledge home (BMAD `project_knowledge`). `spec/{dev-notes,runbooks,
# design,ui-design-principles.md}` `git mv`d here in the 2026-07-25 BMAD-migration
# Phase 4. Walked for dead-link + frontmatter checks alongside SPEC_DIR/EVIDENCE_DIR
# so cross-links between the three roots stay checked.
DOCS_DIR = REPO_ROOT / "docs"

# ---------------------------------------------------------------------------
# Configuration: which frontmatter keys are required on which files.
# Keep small and explicit — overengineering this is a footgun.
# ---------------------------------------------------------------------------

REQUIRED_FRONTMATTER: dict[str, set[str]] = {
    "feature.md": {"slug", "status", "owner", "updated"},
    "tasks.md":   {"slug", "status", "owner", "updated"},
}

# Files where missing frontmatter is a warning, not a hard fail.
# product.md / architecture.md historically did not carry frontmatter; we
# only require it on per-feature files for now.
SOFT_FRONTMATTER: dict[str, set[str]] = {
    "product.md":      {"updated"},
    "architecture.md": {"updated"},
}

VALID_STATUSES = {
    # Standard lifecycle vocabulary (CLAUDE.md / spec-update SKILL).
    "draft",
    "proposed",
    "in-progress",
    "shipped",
    "deprecated",
    # Project-specific additions observed in spec/ on 2026-05-13:
    "roadmap",   # multi-phase initiatives with phases under planning (lumen-design-adoption)
    "candidate", # features being evaluated for inclusion (cockpit-app-bundle, iced-ecosystem-evaluation)
    "active",    # in-progress phase of a multi-phase initiative (lumen-design-adoption sub-phases)
    "reserved",  # placeholder phase scheduled but not yet started (lumen phase-6-assistant-slot)
    # 2026-05-22 additions:
    "shipped-partial",  # first-of-kind precedent from v3-llm-forecaster v0.1.0 — code gates clean,
                        # one wave deferred due to external-dependency resolution (API key, vendor
                        # account, third-party data, etc.). See evidence/v1/v3-llm-forecaster/reports/
                        # test-final-2026-05-22.md § 14 for the protocol.
    "retired",          # research-line closure (not deletion). Used by v3-volatility-forecaster +
                        # v3-volatility-forecaster-rebaseline after the noop-fix retire decision
                        # 2026-05-22. Code stays in the tree; anchors stay locked; no further effort.
    # 2026-05-29 additions — intermediate workflow statuses for the
    # analyst → architect → developer → tester → presenter pipeline.
    # Surfaced by 2 Pick C architects: these transient mid-flight states
    # were used routinely (arch-done x5, dev-done x3, tester-done x1) but
    # never in the lint vocabulary, producing false-positive invalid-status
    # flags for in-flight features. Operator-approved enum widening.
    "arch-done",        # architect M-T1 design pass complete; pre-developer.
    "dev-done",         # developer M-DEV complete; pre-tester.
    "tester-done",      # tester VERDICT → PASS; pre-presenter.
    # 2026-05-30 addition — completes the pipeline vocabulary past the
    # presenter. Surfaced by the spec-audit-2026-05-30 sweep:
    # ui-test-harness-viewport-matrix/tasks.md carried the non-enum token
    # `present-done` (presenter deck assembled, awaiting operator approval —
    # NOT yet `shipped`). Mirrors arch-done/dev-done/tester-done: the
    # transient state between a presenter pass and the operator ship tick.
    "presenter-done",   # presenter deck assembled; pre-operator-approval.
}

# Categories — used both for grouping output and computing exit code.
CATEGORIES = (
    "dead-link",
    "missing-frontmatter",
    "orphan-feature",
    "bad-anchor",
    "unreferenced-anchor",
    "shipped-no-tests",
    "status-drift",
    "feature-shipped-trace-drift",
    "feature-shipped-changelog-missing",
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
# Frontmatter parsing
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
# Link extraction
# ---------------------------------------------------------------------------

# Matches [text](target) but skips ![alt](img) and external URLs.
LINK_RE = re.compile(r"(?<!\!)\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")


def extract_links(text: str) -> list[str]:
    return LINK_RE.findall(text)


def is_external(link: str) -> bool:
    return link.startswith(("http://", "https://", "mailto:", "#"))


# ---------------------------------------------------------------------------
# Check: dead intra-spec links
# ---------------------------------------------------------------------------

# Byte-immutable anchored reports whose internal links cannot be raw-edited
# without breaking the verify_anchors body-SHA gate (ADR-0038 anchor-additive
# contract). Keyed by (repo-relative report path, exact link string). Keep this
# list SHORT and each entry justified; remove an entry once its link is fixed.
KNOWN_FROZEN_DEAD_LINKS: set[tuple[str, str]] = {
    # v3-volatility-forecaster is RETIRED (research-line closure 2026-05-22 —
    # "anchors stay locked, no further effort"). This BS-1 report (anchored
    # scenario `vol-verdict-bs1-realdata` in anchors.toml) froze an off-by-one
    # relative link at emission: `../architecture/...` should be
    # `../../architecture/...` (the report sits two dirs deep; the ADR-0038
    # target was spec/architecture/adr/, now _bmad-output/planning-artifacts/
    # architecture/decisions/ since the 2026-07-25 BMAD-migration Phase 4
    # ADR-corpus move — the frozen link is doubly stale, but was already dead
    # pre-move). A raw `../`->`../../` edit breaks the body-SHA; the proper
    # fix is the ADR-0038 §D6.c documentation-link-fix re-emission protocol
    # (NOT YET CODIFIED — see CLAUDE.md). Exempted here rather than
    # re-emitting a retired line.
    (
        "evidence/v1/v3-volatility-forecaster/reports/vol-verdict-bs1-realdata-20260522.md",
        "../architecture/adr/0038-vol-forecast-verdict-shape.md"
        "#d1-v-verdict-priority-tree-parallel-to-adr-0033--d3-not-extension",
    ),
    # BMAD-migration Phase 4 (2026-07-25) fallout: these two evidence/v3 reports
    # are byte-immutable (anchored `backtest-*.md` bodies) and each carry ONE
    # real markdown link that resolved correctly into spec/dev-notes/ and
    # spec/architecture/adr/ respectively at emission time. Both targets moved
    # in this same migration phase (-> docs/dev-notes/, -> _bmad-output/
    # planning-artifacts/architecture/decisions/); the frozen bodies cannot be
    # edited to follow, so the tuple's second element below is the literal
    # (now-stale) string as it still reads in the frozen file -- do NOT
    # "fix" it to the new path, that would break the match. Newly-dead, not
    # pre-existing — tracked here per the same allowlist convention rather
    # than editing evidence/**.
    (
        "evidence/v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun-errata.md",
        "../../../../spec/dev-notes/p2-wobble-thesis-analysis-2026-07-10.md",
    ),
    (
        "evidence/v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun.md",
        "../../../../spec/architecture/adr/0084-p2-corpus-set-coinbase-adapter-verdict-rerun.md",
    ),
}


# Shipped features whose CHANGELOG.md index entry is a documented THEMATIC
# ROLLUP line rather than a verbatim slug — exempted from
# feature-shipped-changelog-missing. Each entry cites the exact covering
# CHANGELOG.md line (verified 2026-07-10). This mirrors KNOWN_FROZEN_DEAD_LINKS:
# a short, per-entry-justified allowlist for irreducible reality. See the
# GRANDFATHERING note above check_feature_shipped_changelog_missing for why
# these are rollups (intentional, per the 2026-06-17 spec-compression pass) and
# NOT gaps. Keep SHORT — a NEW shipped feature earns a real CHANGELOG line, not
# an allowlist row. Remove an entry the moment its feature gains a verbatim line.
CHANGELOG_ROLLUP_ALLOWLIST: dict[str, str] = {
    # --- v0…v5 strategy/engine ladder → thematic per-version rollup lines
    #     (CHANGELOG § "Strategy & backtest engine"). The folder slugs carry a
    #     descriptive tail (`v05-composed-strategies`) that the compressed
    #     `**v0.5**` line intentionally omits.
    "v0-paper-sma": "CHANGELOG § Strategy — `**v0**` (Paper-trading SMA-crossover tracer bullet).",
    "v05-composed-strategies": "CHANGELOG § Strategy — `**v0.5**` (Composed strategies).",
    "v1-cross-sectional-momentum": "CHANGELOG § Strategy — `**v1**` (Cross-sectional top-N momentum).",
    "v15a-mean-reversion-pairs": "CHANGELOG § Strategy — `**v1.5a**` (Mean-reversion on z-scored pairs).",
    "v1-5b-multi-venue": "CHANGELOG § Strategy — `**v1.5b**` (Multi-venue + 1-second aggregated trades).",
    "v2-llm-strategy": "CHANGELOG § Strategy — `**v2**` (LLM news/sentiment strategy overlay).",
    "v2-1-tracing-layer-redactor": "CHANGELOG § Strategy — `**v2.1**` (tracing-Layer secret redactor).",
    # v5 latency/slippage: one `**v5**` line names every v0.2→v0.5 sub-phase as
    # the "full anchor-migration chain" rather than the folder slugs.
    "v5-latency-slippage-sim": "CHANGELOG § Strategy — `**v5**` (deterministic latency & slippage sim, v0.1→v0.5 chain).",
    "v5-latency-slippage-sim-v0.2.0-anchor-migration": "CHANGELOG § Strategy — `**v5**` line ('v0.2 anchor migration').",
    "v5-latency-slippage-sim-v0.3.0-full-path-wiring": "CHANGELOG § Strategy — `**v5**` line ('v0.3 full-path wiring').",
    "v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit": "CHANGELOG § Strategy — `**v5**` line ('v0.4 candle/realdata feature-gated re-emit').",
    "v5-latency-slippage-sim-v0.5.0-square-root-market-impact": "CHANGELOG § Strategy — `**v5**` line ('v0.5 sqrt-impact').",
    # --- Retired DL/ML research lines → the single `**v2.5 DL forecaster
    #     programme**` rollup (CHANGELOG § "Retired research lines"), which
    #     names the TCN/PatchTST/Transformer/bake-off sub-studies collectively.
    "v25-tcn-overlay": "CHANGELOG § Retired — `**v2.5 DL forecaster programme**` (TCN overlay).",
    "v25-tcn-alpha-investigation": "CHANGELOG § Retired — `**v2.5 DL forecaster programme**` (TCN alpha-investigation sub-study).",
    "v25-tcn-recalibrate": "CHANGELOG § Retired — `**v2.5 DL forecaster programme**` (TCN recalibrate sub-study).",
    "v25-tcn-threshold-tuning": "CHANGELOG § Retired — `**v2.5 DL forecaster programme**` (TCN threshold-tuning sub-study).",
    "v25-tcn-horizon-bump-or-retire": "CHANGELOG § Retired — `**v2.5 DL forecaster programme**` (TCN horizon-bump sub-study).",
    "v25a-patchtst-overlay": "CHANGELOG § Retired — `**v2.5 DL forecaster programme**` (PatchTST overlay).",
    "v3-volatility-forecaster-noop-fix": "CHANGELOG § Retired — `**v3 volatility forecaster**` line ('+ noop-fix').",
    "v3-regime-classifier": "CHANGELOG § Retired — `**v3 regime-classifier / v3 XGBoost cheap-classifier**`.",
    # --- Iteration/follow-up folders folded into their base feature's line
    #     with a `(+ …)` suffix (the CHANGELOG's iteration convention).
    "cockpit-activity-audit-ledger-producer": "CHANGELOG § Cockpit — `**cockpit-activity-status-bar** + **-audit-ledger-producer** + **-llm-producer**`.",
    "cockpit-activity-llm-producer": "CHANGELOG § Cockpit — `**cockpit-activity-status-bar** + **-audit-ledger-producer** + **-llm-producer**`.",
    "reflection-memory-trader-wiring": "CHANGELOG § Core infra — `**reflection-memory** (+ trader-wiring)`.",
    "ui-rethink-phase-d-trail-followup": "CHANGELOG § Cockpit — `**ui-rethink-phase-d-trail** (+ follow-up)`.",
    # --- Label-shorthand: the CHANGELOG entry uses the roadmap shorthand
    #     `F1+F2` for the folder slug's `-ranking` variant.
    "advisor-bakeoff-ranking": "CHANGELOG § Advisor — `**advisor-bakeoff F1+F2**` (F1 bake-off + F2 ranking; slug carries the -ranking tail).",
    # --- v2 tester-report UMBRELLA folder (not a standalone product feature):
    #     its three overlay features are each independently indexed.
    "phase-2c-overlays": "CHANGELOG § v2 — the three children are indexed: `**advisor-vol-estimator**` / `**advisor-vol-overlay-reposition**` / `**advisor-drawdown-control-overlay**` (this folder is their shared test-report umbrella).",
}


def check_dead_links(md_path: Path, text: str, report: Report) -> None:
    try:
        rel = md_path.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        rel = None
    for raw in extract_links(text):
        if is_external(raw):
            continue
        # Strip in-page anchor fragment.
        target_str = raw.split("#", 1)[0]
        if not target_str:
            continue  # pure anchor link
        target = (md_path.parent / target_str).resolve()
        if not target.exists():
            if rel is not None and (rel, raw) in KNOWN_FROZEN_DEAD_LINKS:
                continue  # documented byte-immutable frozen link (see above)
            report.add("dead-link", md_path, f"link target missing: {raw}")


# ---------------------------------------------------------------------------
# Check: required frontmatter
# ---------------------------------------------------------------------------

def check_frontmatter(md_path: Path, text: str, report: Report) -> None:
    name = md_path.name
    required = REQUIRED_FRONTMATTER.get(name)
    soft = SOFT_FRONTMATTER.get(name)
    if required is None and soft is None:
        return

    fm = parse_frontmatter(text)
    if fm is None:
        if required:
            report.add(
                "missing-frontmatter",
                md_path,
                f"no frontmatter block (required keys: {sorted(required)})",
            )
        return

    if required:
        missing = sorted(required - fm.keys())
        if missing:
            report.add(
                "missing-frontmatter",
                md_path,
                f"missing keys: {missing}",
            )
        status = fm.get("status")
        if status and status not in VALID_STATUSES:
            report.add(
                "missing-frontmatter",
                md_path,
                f"invalid status: {status!r} (allowed: {sorted(VALID_STATUSES)})",
            )


# ---------------------------------------------------------------------------
# Check: orphan feature folders
# ---------------------------------------------------------------------------

# Folder names that are not features (cross-cutting siblings of feature folders).
# `design`/`dev-notes`/`runbooks` `git mv`d out of spec/ entirely in the
# 2026-07-25 BMAD-migration Phase 4 (-> docs/); no longer possible children of
# spec_dir.iterdir(), so dropped from this set (dead entries would be harmless
# but the folder names genuinely no longer occur here).
NON_FEATURE_FOLDERS = {"archive", "architecture",
                       "v1", "v2", "v3"}  # v1/v2/v3 are containers for feature folders
                                          # (v1/v2 = 2026-06-28 reorg; v3 = 2026-07-09 close-out phase)


def is_feature_folder(p: Path) -> bool:
    if not p.is_dir():
        return False
    if p.name in NON_FEATURE_FOLDERS:
        return False
    if p.name.startswith("."):
        return False
    return True


def check_orphan_features(spec_dir: Path, report: Report) -> None:
    # Feature folders live at spec/ root AND under spec/v1/ + spec/v2/ + spec/v3/
    # (v1/v2 = 2026-06-28 reorg; v3 = 2026-07-09 close-out phase). Lint all.
    children = list(spec_dir.iterdir())
    for container in ("v1", "v2", "v3"):
        sub = spec_dir / container
        if sub.is_dir():
            children.extend(sub.iterdir())
    for child in sorted(children):
        if not is_feature_folder(child):
            continue
        feature = child / "feature.md"
        tasks = child / "tasks.md"
        if not feature.exists():
            report.add("orphan-feature", child, "missing feature.md")
            continue
        # tasks.md is required for active development states, but not for:
        #   - candidate: feature being evaluated, not yet scoped
        #   - roadmap: parent of phase subfolders (tasks live in phases)
        #   - deprecated: archived
        #   - completed/terminal states (2026-06-17): a finished feature needs no
        #     active task list — its history lives in git + the root CHANGELOG.md
        #     index. The orphan rule guards in-flight features (draft/proposed/
        #     in-progress/arch-done/dev-done), where a missing task list is a real
        #     drift signal. Per the spec-compression pass that gutted completed
        #     feature.md files to one-line stubs and removed their tasks.md.
        # Per-status leniency, not a blanket skip.
        fm = parse_frontmatter(feature.read_text())
        status = (fm or {}).get("status", "")
        if status in {"candidate", "roadmap", "deprecated",
                      "shipped", "shipped-partial", "retired",
                      "presenter-done", "tester-done"}:
            continue
        if not tasks.exists():
            report.add(
                "orphan-feature",
                child,
                f"missing tasks.md (status={status!r}; required for non-candidate/roadmap features)",
            )


# ---------------------------------------------------------------------------
# Check: anchors.toml well-formed
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
# Check: trace.toml (optional — degrades gracefully)
# ---------------------------------------------------------------------------

def check_trace(
    spec_dir: Path,
    report: Report,
    anchors: dict[str, dict],
) -> None:
    trace_path = spec_dir / "trace.toml"
    if not trace_path.exists():
        return  # not yet adopted; that's fine

    with trace_path.open("rb") as f:
        data = tomllib.load(f)
    rows = data.get("req", [])

    cited_anchors: set[str] = set()
    cited_features: set[str] = set()

    for row in rows:
        rid = row.get("id", "<no-id>")
        # Validate path-bearing fields. `feature` is a slug (or list of slugs),
        # NOT a path — resolve to spec/<slug>/ before checking.
        for field_name in ("product", "arch", "crates", "tests"):
            val = row.get(field_name)
            if val is None:
                continue
            if isinstance(val, str):
                _check_trace_path(trace_path, rid, field_name, val, report)
            elif isinstance(val, list):
                for v in val:
                    _check_trace_path(trace_path, rid, field_name, v, report)
        # Feature slugs: check the folder exists under spec/.
        feat = row.get("feature")
        feats = [feat] if isinstance(feat, str) else (feat or [])
        for slug in feats:
            cited_features.add(slug)
            # Feature folders may live at spec/<slug>, spec/v1/<slug>,
            # spec/v2/<slug>, or spec/v3/<slug> (v1/v2 = 2026-06-28 reorg;
            # v3 = 2026-07-09 close-out phase).
            if not any((SPEC_DIR / prefix / slug).exists()
                       for prefix in ("", "v1", "v2", "v3")):
                report.add(
                    "trace-broken-path",
                    trace_path,
                    f"row {rid} field feature: missing folder spec/[v1|v2|v3/]{slug}",
                )
        # Anchor citations.
        # `anchors` may be a list of scenario-name strings (the normal case),
        # or a bare prose string such as "34/34 PASS" used when a feature
        # contributes zero new anchors but the tester still wants to record
        # the verification result inline.  Iterate only when it is a list;
        # a bare string is not path-checkable and is silently skipped here.
        raw_anchors = row.get("anchors", [])
        if isinstance(raw_anchors, list):
            for anc in raw_anchors:
                cited_anchors.add(anc)
                if anc not in anchors:
                    report.add(
                        "trace-broken-path",
                        trace_path,
                        f"row {rid}: anchor {anc!r} not in anchors.toml",
                    )

    # Anchors not referenced by any trace row.
    for scenario in anchors:
        if scenario not in cited_anchors:
            report.add(
                "unreferenced-anchor",
                spec_dir / "anchors.toml",
                f"anchor {scenario!r} not cited by any trace.toml row",
            )


def _check_trace_path(
    trace_path: Path,
    row_id: str,
    field_name: str,
    raw: str,
    report: Report,
) -> None:
    # Strip in-doc anchor fragment (#frag) AND the Rust item path (::fn).
    # Both name a sub-location WITHIN a file; the checkable unit is the file
    # itself. This mirrors the established `file.rs::test_fn` trace convention
    # (45 such rows) — previously every one was a `trace-broken-path` false
    # positive because only `#` was stripped (spec-audit-2026-05-30 SHOULD-FIX).
    raw_no_frag = raw.split("#", 1)[0].split("::", 1)[0]
    if not raw_no_frag:
        return
    # Trace paths are relative to repo root.
    target = (REPO_ROOT / raw_no_frag).resolve()
    if not target.exists():
        report.add(
            "trace-broken-path",
            trace_path,
            f"row {row_id} field {field_name}: missing path {raw}",
        )


# ---------------------------------------------------------------------------
# Check: shipped feature has at least one test report
# ---------------------------------------------------------------------------

def check_shipped_have_tests(
    spec_dir: Path, report: Report, evidence_dir: Path | None = None
) -> None:
    """``evidence_dir`` defaults to the module-level ``EVIDENCE_DIR``; tests
    may inject a synthetic root (mirrors ``check_status_drift``)."""
    if evidence_dir is None:
        evidence_dir = EVIDENCE_DIR
    for child in sorted(spec_dir.iterdir()):
        if not is_feature_folder(child):
            continue
        feature = child / "feature.md"
        if not feature.exists():
            continue
        fm = parse_frontmatter(feature.read_text())
        if not fm:
            continue
        if fm.get("status") != "shipped":
            continue
        # reports/ lives under evidence_dir (2026-07-25 Phase 3 move,
        # layout-preserving mirror of the feature-folder relative path).
        reports_dir = evidence_dir / child.relative_to(spec_dir) / "reports"
        if not reports_dir.exists():
            report.add(
                "shipped-no-tests",
                feature,
                "shipped feature has no reports/ directory",
            )
            continue
        # Accept any .md report — the project uses several naming conventions:
        # test-*.md (tester reports), backtest-*.md (strategy backtests),
        # evaluation-*.md (ad-hoc evaluations), and a few one-off names.
        # Screenshots and .log files alone don't count.
        has_test = any(p.suffix == ".md" for p in reports_dir.glob("*.md"))
        if not has_test:
            report.add(
                "shipped-no-tests",
                feature,
                "shipped feature has no .md report (only screenshots / logs)",
            )


# Frontmatter statuses that mean "the presenter cycle has been acted on" —
# at-or-past presenter-done on the pipeline, or explicitly closed out.
# A feature whose folder holds BOTH a presentation deck AND a PASS tester
# report must carry one of these; anything earlier is status drift.
STATUS_AT_OR_PAST_PRESENTER = {
    "presenter-done",
    "shipped",
    "shipped-partial",
    "retired",
    "deprecated",
}

# The tester's verdict line, as emitted by the rust-test template
# ("VERDICT → PASS"); accept the ASCII arrow too.
VERDICT_PASS_RE = re.compile(r"VERDICT\s*(?:→|->)\s*PASS")


def check_status_drift(
    spec_dir: Path, report: Report, evidence_dir: Path | None = None
) -> None:
    """The audit-2026-06-12 enforcement hook (5 consecutive audits of drift).

    Rule: if a feature folder contains a presentation deck
    (``presentations/*.md``) AND a passing tester report
    (``reports/test-*.md`` whose body carries ``VERDICT → PASS``), then
    ``feature.md``'s frontmatter ``status`` must be at-or-past
    ``presenter-done``. Catching this at lint time (presenter pre-tick /
    CI) replaces the weekly-audit archaeology that flagged the same drift
    class five audits running.

    Deliberately requires BOTH artifacts: archived decks/reports (moved to
    spec/archive tars by cleanup sweeps) make a folder skip this check —
    the rule fires at the moment drift is introduced, not retroactively.

    ``evidence_dir`` defaults to the module-level ``EVIDENCE_DIR`` (the
    2026-07-25 Phase 3 reports-corpus root); tests pass a synthetic root
    that mirrors ``spec_dir``'s fixture layout so self-tests stay hermetic.
    """
    if evidence_dir is None:
        evidence_dir = EVIDENCE_DIR
    for child in sorted(spec_dir.iterdir()):
        if not is_feature_folder(child):
            continue
        feature = child / "feature.md"
        if not feature.exists():
            continue
        fm = parse_frontmatter(feature.read_text())
        if not fm:
            continue
        status = fm.get("status", "")
        if status in STATUS_AT_OR_PAST_PRESENTER:
            continue
        decks = list((child / "presentations").glob("*.md"))
        if not decks:
            continue
        # reports/ lives under evidence_dir (2026-07-25 Phase 3 move);
        # presentations/ stays under spec_dir until Phase 5b.
        evidence_reports_dir = evidence_dir / child.relative_to(spec_dir) / "reports"
        has_pass = any(
            VERDICT_PASS_RE.search(p.read_text(encoding="utf-8", errors="replace"))
            for p in evidence_reports_dir.glob("test-*.md")
        )
        if not has_pass:
            continue
        report.add(
            "status-drift",
            feature,
            f"presenter cycle complete (deck {decks[0].name} + PASS tester "
            f"report) but status is '{status}' — must be ≥ presenter-done",
        )


# ---------------------------------------------------------------------------
# Check: feature.md `status: shipped` ⇒ trace row `state == "shipped"`
# ---------------------------------------------------------------------------
#
# ADR-0082 invariant: feature.md frontmatter `status:` is the single source of
# truth for a feature's lifecycle; a trace.toml [[req]] row's `state=` must not
# contradict it. Concretely: once a feature's feature.md reaches
# `status: shipped`, every trace row whose `feature=` slug resolves to it MUST
# read `state = "shipped"`. The tester-terminal aliases (verified/passed/tested/
# tester-done) are legal ONLY while the feature is still pre-ship, so rows whose
# feature is not shipped are never flagged here.


def feature_status_for_slug(spec_dir: Path, slug: str) -> str | None:
    """Resolve a trace `feature=` slug to its feature.md `status:` value.

    Mirrors the feature-folder resolution in ``check_trace``: a feature folder
    may live at ``spec/<slug>``, ``spec/v1/<slug>``, ``spec/v2/<slug>``, or
    ``spec/v3/<slug>`` (v1/v2 = 2026-06-28 reorg; v3 = 2026-07-09 close-out
    phase). Returns the status string, or None if no feature.md / no parseable
    frontmatter / no ``status:`` key is found.
    """
    for prefix in ("", "v1", "v2", "v3"):
        feature = (spec_dir / prefix / slug / "feature.md") if prefix \
            else (spec_dir / slug / "feature.md")
        if feature.exists():
            fm = parse_frontmatter(feature.read_text(encoding="utf-8", errors="replace"))
            return (fm or {}).get("status")
    return None


def check_feature_shipped_trace_drift(spec_dir: Path, report: Report) -> None:
    """ADR-0082 enforcement: shipped feature ⇒ trace row state == "shipped".

    For every [[req]] whose `feature=` slug resolves to an existing
    ``spec/**/feature.md`` with ``status: shipped``, assert the row's
    ``state == "shipped"``. Rows whose feature is not shipped (or whose feature
    folder is absent — that's a separate ``trace-broken-path`` concern) are not
    flagged: their tester-terminal aliases are legal pre-ship.
    """
    trace_path = spec_dir / "trace.toml"
    if not trace_path.exists():
        return  # not yet adopted; that's fine

    with trace_path.open("rb") as f:
        data = tomllib.load(f)

    for row in data.get("req", []):
        rid = row.get("id", "<no-id>")
        feat = row.get("feature")
        feats = [feat] if isinstance(feat, str) else (feat or [])
        for slug in feats:
            status = feature_status_for_slug(spec_dir, slug)
            if status != "shipped":
                continue  # pre-ship (or unresolved) — aliases are legal here
            state = row.get("state")
            if state != "shipped":
                shown = "<missing state= field>" if state is None else repr(state)
                report.add(
                    "feature-shipped-trace-drift",
                    trace_path,
                    f"row {rid} (feature {slug!r}): feature.md status is 'shipped' "
                    f"but trace state is {shown} — must be \"shipped\" (ADR-0082 § D2)",
                )


# ---------------------------------------------------------------------------
# Check: feature.md `status: shipped` ⇒ indexed in CHANGELOG.md
# ---------------------------------------------------------------------------
#
# Sibling of feature-shipped-trace-drift, extending the SAME ADR-0082 single-
# source-of-truth philosophy to the OTHER derived index. ADR-0082 § D1 makes
# feature.md `status:` the authoritative lifecycle record and names two derived
# artifacts it must not contradict: the `trace.toml` `state=` (enforced by
# feature-shipped-trace-drift) and — explicitly — the CHANGELOG that "indexes"
# the shipped set (ADR-0082 § "Alternatives", the reason feature.md wins over
# trace.toml: "the CHANGELOG indexes [feature.md]"). CHANGELOG.md's own header
# declares it "The canonical 'what's-been-built' index — one line per
# implemented feature". This rule makes that invariant mechanical: once a
# feature's feature.md reaches `status: shipped`, CHANGELOG.md MUST reference
# it. Closes the drift class R3-4b found (the entire v2 tranche + the v3
# close-out were absent from the canonical index until manually reconciled) —
# the exact bookkeeping debt the durable-contract lints are built to prevent.
#
# MATCH SEMANTICS (measured against the whole tree 2026-07-10 — 114 shipped
# features: 84 matched by raw substring, 4 by the iteration-suffix normalizer,
# 26 by the documented rollup allowlist):
#   A feature counts as INDEXED iff CHANGELOG.md contains, CASE-INSENSITIVELY,
#   any of:
#     (1) its feature-folder slug         (e.g. `advisor-overfitting-scorecard`)
#     (2) any trace.toml REQ-id whose `feature=` resolves to it
#         (e.g. `REQ-V3-P2-CORPUS-EXPANSION-001`)
#     (3) its repo-relative folder PATH   (e.g. `spec/v2/phase-2d/` — the
#         CHANGELOG sometimes cites the folder instead of a bare slug).
#     (4) — for a `…-vN.N.N-<descriptor>` ITERATION folder only — its BASE slug
#         with the trailing `-vN.N.N-…` stripped (e.g.
#         `lab-yahoo-realdata-v0.1.2-eth-usd-…` → `lab-yahoo-realdata`), which
#         the CHANGELOG indexes via its `(+ vN.N …)` iteration convention.
#   Raw substring is wrapping-ROBUST by construction: `**slug**`, `` `slug` ``,
#   and `[slug](…)` all embed the bare slug flanked by non-`[a-z0-9-]` chars, so
#   the match fires regardless of markdown emphasis/code/link decoration. It was
#   verified (2026-07-10) that none of the 84 raw matches occurs ONLY as a
#   substring of a longer slug-token — i.e. no false-positive can mask a real
#   gap through embedding.
#
# GRANDFATHERING — the CHANGELOG_ROLLUP_ALLOWLIST (below). ~30 legitimately-
# shipped features are indexed under CHANGELOG.md's *documented compression
# convention* — thematic ROLLUP lines that deliberately do NOT carry a verbatim
# slug (the `v0…v5` engine ladder rolled into `**v0**`…`**v5**`; the retired DL
# programme rolled into `**v2.5 DL forecaster programme**`; iteration folders
# like `…-v0.2.0-cleanup` folded into the base line as `(+ v0.2 cleanup)`). Per
# the 2026-06-17 spec-compression pass, these rollups are INTENTIONAL, not gaps;
# adding 30 verbatim one-liners would duplicate existing content and fight the
# CHANGELOG's own "grouped by subsystem" convention. So — like the established
# KNOWN_FROZEN_DEAD_LINKS pattern — each is allowlisted with the exact covering
# CHANGELOG line cited inline. The allowlist is HONEST: every entry was verified
# to have a real covering line (a slug is exempted only because it IS indexed,
# just under a rollup the substring match can't mechanically reach — never
# because it is genuinely absent). A DELIBERATELY-loose normalizer (e.g.
# `v05`→`v0.5` then bare-`v0.5` presence) was REJECTED: `v0.5` recurs as a
# version tag throughout the file, so it would silently PASS a genuinely-missing
# `v0.5.x` feature (false-negative) — strictly worse than an allowlist a human
# reviews. Keep this list SHORT; a NEW shipped feature earns a real CHANGELOG
# line, never an allowlist entry.


def _slug_indexed_in_changelog(
    prefix: str, slug: str, reqs: Iterable[str], changelog_lower: str,
) -> bool:
    """True iff CHANGELOG.md references this feature (slug / REQ-id / path).

    All comparisons are case-insensitive raw-substring against the whole
    lower-cased CHANGELOG text (wrapping-robust — see MATCH SEMANTICS above).
    The allowlist is consulted last so that a real CHANGELOG line always
    satisfies the check without needing an exemption.
    """
    if slug.lower() in changelog_lower:
        return True
    for rid in reqs:
        if rid and rid.lower() in changelog_lower:
            return True
    # Folder-path form, e.g. "spec/v2/phase-2d/".
    path_form = (f"spec/{prefix}/{slug}/" if prefix else f"spec/{slug}/").lower()
    if path_form in changelog_lower:
        return True
    # Iteration-folder form: a `…-vN.N.N-<descriptor>` slug is a version bump of
    # a base feature and is indexed by the BASE feature's line (the CHANGELOG's
    # `(+ vN.N …)` iteration convention). Strip the trailing `-vN.N.N-…` and
    # require the SPECIFIC base slug to be present. Safe by construction — the
    # base slug (e.g. `lab-yahoo-realdata`) is a full descriptive token, so this
    # cannot spuriously match a bare version tag the way a `v05`→`v0.5` squeeze
    # would. Only fires when the residual is a real, non-empty base slug that
    # differs from the original.
    base = re.sub(r"-v\d+\.\d+\.\d+.*$", "", slug)
    if base != slug and base and base.lower() in changelog_lower:
        return True
    if slug in CHANGELOG_ROLLUP_ALLOWLIST:
        return True
    return False


def check_feature_shipped_changelog_missing(spec_dir: Path, report: Report) -> None:
    """ADR-0082-aligned enforcement: shipped feature ⇒ a CHANGELOG.md line.

    For every ``spec/**/feature.md`` (across ``spec/``, ``spec/v1``,
    ``spec/v2``, ``spec/v3`` — mirroring the resolution used by every other
    tree-level check) whose frontmatter ``status:`` is ``shipped``, assert that
    CHANGELOG.md references it by slug, by any trace REQ-id, by folder path, or
    via the documented rollup allowlist. Non-shipped features are never flagged
    — the CHANGELOG indexes what has *shipped*, so a pre-ship feature is
    correctly absent.
    """
    # CHANGELOG.md sits at the repo root, i.e. the parent of spec/. Resolving it
    # relative to spec_dir (rather than the module-level REPO_ROOT) lets the
    # --self-test drive the check against a synthetic tree in a tempdir.
    changelog_path = spec_dir.parent / "CHANGELOG.md"
    if not changelog_path.exists():
        return  # nothing to index against; degrade gracefully
    changelog_lower = changelog_path.read_text(
        encoding="utf-8", errors="replace"
    ).lower()

    # trace.toml slug → [REQ-id, …] (best-effort; absent trace = empty map).
    slug_to_reqs: dict[str, list[str]] = {}
    trace_path = spec_dir / "trace.toml"
    if trace_path.exists():
        with trace_path.open("rb") as f:
            tdata = tomllib.load(f)
        for row in tdata.get("req", []):
            rid = row.get("id")
            feat = row.get("feature")
            feats = [feat] if isinstance(feat, str) else (feat or [])
            for s in feats:
                slug_to_reqs.setdefault(s, []).append(rid)

    # Feature folders: spec/<slug> and spec/{v1,v2,v3}/<slug>.
    containers: list[tuple[str, Path]] = [("", child) for child in spec_dir.iterdir()]
    for prefix in ("v1", "v2", "v3"):
        sub = spec_dir / prefix
        if sub.is_dir():
            containers.extend((prefix, child) for child in sub.iterdir())

    for prefix, child in sorted(containers, key=lambda pc: (pc[0], pc[1].name)):
        if not is_feature_folder(child):
            continue
        feature = child / "feature.md"
        if not feature.exists():
            continue
        fm = parse_frontmatter(feature.read_text(encoding="utf-8", errors="replace"))
        if not fm or fm.get("status") != "shipped":
            continue
        slug = child.name
        reqs = slug_to_reqs.get(slug, [])
        if not _slug_indexed_in_changelog(prefix, slug, reqs, changelog_lower):
            report.add(
                "feature-shipped-changelog-missing",
                feature,
                f"feature {slug!r} is status:shipped but is not indexed in "
                f"CHANGELOG.md (no slug / REQ-id {reqs or '[]'} / folder-path "
                f"reference, and not in the documented rollup allowlist) — the "
                f"canonical 'what's-been-built' index must reference every "
                f"shipped feature (ADR-0082 § D1)",
            )


def _self_test_status_drift() -> bool:
    """Synthetic-fixture proof that the status-drift rule fires and clears.

    Three fixtures in a tempdir: (a) drifting — tester-done + deck + PASS
    report → exactly 1 violation; (b) compliant — same artifacts at
    presenter-done → 0; (c) deck but no PASS report → 0. Returns True iff all
    three behave.
    """
    import tempfile

    def make_feature(root: Path, evidence_root: Path, slug: str, status: str,
                     deck: bool, pass_report: bool) -> None:
        d = root / slug
        d.mkdir()
        (d / "feature.md").write_text(
            f"---\nslug: {slug}\nstatus: {status}\nowner: t\nupdated: 2026-06-12\n---\n# x\n"
        )
        if deck:
            (d / "presentations").mkdir()
            (d / "presentations" / f"{slug}-2026-06-12.md").write_text("# deck\n")
        if pass_report:
            # reports/ mirrors the feature-folder relative path under a
            # SEPARATE evidence root (2026-07-25 Phase 3 — reports/ no
            # longer sits alongside feature.md/presentations/).
            e = evidence_root / slug / "reports"
            e.mkdir(parents=True)
            (e / "test-2026-06-12.md").write_text("VERDICT → PASS\n")

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp) / "spec-fixture"
        evidence_root = Path(tmp) / "evidence-fixture"
        root.mkdir()
        evidence_root.mkdir()
        make_feature(root, evidence_root, "drifting", "tester-done", deck=True, pass_report=True)
        make_feature(root, evidence_root, "compliant", "presenter-done", deck=True, pass_report=True)
        make_feature(root, evidence_root, "no-pass-yet", "tester-done", deck=True, pass_report=False)
        rep = Report()
        check_status_drift(root, rep, evidence_dir=evidence_root)
        hits = [v for v in rep.violations if v.category == "status-drift"]
        ok = len(hits) == 1 and "drifting" in str(hits[0].path)
        print(
            "spec-lint --self-test (status-drift): "
            + ("PASS — fires on drift, silent on compliant/no-pass" if ok
               else f"FAIL — expected exactly 1 hit on 'drifting', got {[(str(v.path), v.detail) for v in hits]}")
        )
        return ok


def _self_test_feature_shipped_trace_drift() -> bool:
    """Synthetic-fixture proof of the ADR-0082 feature-shipped-trace-drift rule.

    Four fixtures in a tempdir with a synthetic trace.toml:
      (a) drifting     — feature.md=shipped, row state="passed"  → 1 violation.
      (b) compliant    — feature.md=shipped, row state="shipped" → 0.
      (c) preship      — feature.md=tester-done, row state="tested" → 0
                         (tester-terminal alias legal pre-ship; feature under
                         v1/ also proves the multi-prefix slug resolution).
      (d) missing-state— feature.md=shipped, row has NO state= field → 1
                         violation (the paper-mode-equity-wiring-style case).
    Expect exactly 2 violations, on 'drifting' and 'missing-state'. Returns
    True iff the rule behaves.
    """
    import tempfile

    def write_feature(dir_: Path, slug: str, status: str) -> None:
        dir_.mkdir(parents=True)
        (dir_ / "feature.md").write_text(
            f"---\nslug: {slug}\nstatus: {status}\nowner: t\nupdated: 2026-07-09\n---\n# x\n"
        )

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        write_feature(root / "drifting", "drifting", "shipped")
        write_feature(root / "compliant", "compliant", "shipped")
        write_feature(root / "v1" / "preship", "preship", "tester-done")  # v1/ prefix
        write_feature(root / "missing-state", "missing-state", "shipped")
        (root / "trace.toml").write_text(
            "[[req]]\n"
            'id = "REQ-DRIFT"\nfeature = "drifting"\nstate = "passed"\n\n'
            "[[req]]\n"
            'id = "REQ-OK"\nfeature = "compliant"\nstate = "shipped"\n\n'
            "[[req]]\n"
            'id = "REQ-PRESHIP"\nfeature = "preship"\nstate = "tested"\n\n'
            "[[req]]\n"
            'id = "REQ-NOSTATE"\nfeature = "missing-state"\n'  # no state= field
        )
        rep = Report()
        check_feature_shipped_trace_drift(root, rep)
        hits = [v for v in rep.violations if v.category == "feature-shipped-trace-drift"]
        hit_ids = sorted(
            (h.detail.split("row ", 1)[1].split(" ", 1)[0] for h in hits)
        )
        ok = hit_ids == ["REQ-DRIFT", "REQ-NOSTATE"]
        print(
            "spec-lint --self-test (feature-shipped-trace-drift): "
            + ("PASS — fires on shipped/non-shipped-state + missing-state, "
               "silent on compliant + pre-ship alias" if ok
               else f"FAIL — expected hits on ['REQ-DRIFT', 'REQ-NOSTATE'], "
                    f"got {[(str(v.path), v.detail) for v in hits]}")
        )
        return ok


def _self_test_feature_shipped_changelog_missing() -> bool:
    """Synthetic-fixture proof of the feature-shipped-changelog-missing rule.

    Five fixtures in a tempdir with a synthetic CHANGELOG.md at the tree root:
      (a) indexed-slug   — shipped, slug appears (wrapped in ``**…**``) → silent.
      (b) missing        — shipped, NOT referenced anywhere              → 1 hit.
      (c) preship        — status:tester-done, absent from CHANGELOG     → silent
                           (pre-ship is correctly not yet indexed; also under a
                           v2/ prefix, exercising multi-prefix resolution).
      (d) indexed-req    — shipped, slug absent but its trace REQ-id is in the
                           CHANGELOG                                     → silent.
      (e) indexed-path   — shipped, slug/REQ absent but the folder PATH
                           ``spec/v3/indexed-path/`` is cited            → silent.
      (f) foo-v0.2.0-x   — shipped ITERATION folder; only its BASE slug ``foo``
                           is in the CHANGELOG (layer-4 suffix strip)    → silent.
    Expect exactly 1 violation, on 'missing'. Returns True iff the rule behaves.
    """
    import tempfile

    def write_feature(dir_: Path, slug: str, status: str) -> None:
        dir_.mkdir(parents=True)
        (dir_ / "feature.md").write_text(
            f"---\nslug: {slug}\nstatus: {status}\nowner: t\nupdated: 2026-07-10\n---\n# x\n"
        )

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        spec = root / "spec"
        spec.mkdir()
        write_feature(spec / "indexed-slug", "indexed-slug", "shipped")
        write_feature(spec / "missing", "missing", "shipped")
        write_feature(spec / "v2" / "preship", "preship", "tester-done")   # v2/ prefix
        write_feature(spec / "indexed-req", "indexed-req", "shipped")
        write_feature(spec / "v3" / "indexed-path", "indexed-path", "shipped")  # v3/ prefix
        write_feature(spec / "foo-v0.2.0-x", "foo-v0.2.0-x", "shipped")  # iteration folder
        # A trace.toml giving indexed-req a REQ-id that the CHANGELOG cites.
        (spec / "trace.toml").write_text(
            "[[req]]\n"
            'id = "REQ-INDEXED-REQ-001"\nfeature = "indexed-req"\nstate = "shipped"\n'
        )
        # Synthetic CHANGELOG: references indexed-slug (wrapped), indexed-req by
        # REQ-id, indexed-path by folder path, and 'foo' (the base of the
        # iteration folder); deliberately omits 'missing' and 'preship'.
        (root / "CHANGELOG.md").write_text(
            "# Changelog\n\n"
            "- **indexed-slug** — a shipped-and-indexed feature.\n"
            "- some rollup line covering REQ-INDEXED-REQ-001 without the slug.\n"
            "- see `spec/v3/indexed-path/` for the path-cited feature.\n"
            "- **foo** (+ v0.2 x) — base feature with an iteration bump.\n"
        )
        rep = Report()
        check_feature_shipped_changelog_missing(spec, rep)
        hits = [v for v in rep.violations
                if v.category == "feature-shipped-changelog-missing"]
        ok = len(hits) == 1 and hits[0].path.parent.name == "missing"
        print(
            "spec-lint --self-test (feature-shipped-changelog-missing): "
            + ("PASS — fires on shipped-not-indexed, silent on slug/REQ-id/path "
               "matches + pre-ship" if ok
               else f"FAIL — expected exactly 1 hit on 'missing', got "
                    f"{[(str(v.path), v.detail) for v in hits]}")
        )
        return ok


def self_test() -> int:
    """Run every rule's synthetic-fixture self-test. Exit 0 iff all pass."""
    results = [
        _self_test_status_drift(),
        _self_test_feature_shipped_trace_drift(),
        _self_test_feature_shipped_changelog_missing(),
    ]
    return 0 if all(results) else 1


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------

def iter_spec_md(roots: Iterable[Path]) -> Iterable[Path]:
    for root in roots:
        if root.is_file() and root.suffix == ".md":
            yield root
        elif root.is_dir():
            for p in sorted(root.rglob("*.md")):
                rel = p.relative_to(REPO_ROOT).as_posix()
                # Skip archived content — it's frozen by design.
                if "archive/" in rel:
                    continue
                # Skip byte-immutable anchored report bodies under the v1 corpus.
                # The 2026-06-28 v1/v2 reorg moved them one level deeper, so their
                # internal relative links are off-by-one — but they CANNOT be
                # repaired without changing the body bytes and breaking the
                # body-SHA-256 anchors (CLAUDE.md non-negotiable). Frozen evidence.
                # Repointed 2026-07-25 (BMAD-migration Phase 3): the corpus lives
                # under evidence/v1/ now, not spec/v1/ — same off-by-one, same
                # freeze, new root.
                if rel.startswith("evidence/v1/") and "/reports/" in rel:
                    continue
                yield p


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "paths",
        nargs="*",
        help="restrict to one or more paths under spec/ (default: whole spec/)",
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

    if not SPEC_DIR.exists():
        print(f"error: spec/ not found at {SPEC_DIR}", file=sys.stderr)
        return 99

    # Default roots: spec/ (feature.md/tasks.md/presentations/ + everything
    # else not yet migrated) AND evidence/ (the reports corpus, since the
    # 2026-07-25 Phase 3 move — dead-link + frontmatter checks still walk
    # every report body EVIDENCE_DIR holds, mirroring pre-move coverage) AND
    # docs/ (project-knowledge — dev-notes/runbooks/design/ui-design-principles.md,
    # since the 2026-07-25 Phase 4 move — dead-link checks walk every doc DOCS_DIR
    # holds, and cross-links spec/ <-> docs/ resolve against the same tree).
    # `iter_spec_md` no-ops gracefully if a root doesn't exist yet.
    roots = (
        [Path(p).resolve() for p in args.paths]
        if args.paths
        else [SPEC_DIR, EVIDENCE_DIR, DOCS_DIR]
    )
    report = Report()

    # Per-file checks (links + frontmatter).
    for md in iter_spec_md(roots):
        text = md.read_text(encoding="utf-8", errors="replace")
        check_dead_links(md, text, report)
        check_frontmatter(md, text, report)

    # Tree-level checks (only when running over the whole spec/).
    if not args.paths or any(Path(p).resolve() == SPEC_DIR for p in args.paths):
        check_orphan_features(SPEC_DIR, report)
        anchors = check_anchors(EVIDENCE_DIR, report)
        check_trace(SPEC_DIR, report, anchors)
        check_shipped_have_tests(SPEC_DIR, report)
        check_status_drift(SPEC_DIR, report)
        check_feature_shipped_trace_drift(SPEC_DIR, report)
        check_feature_shipped_changelog_missing(SPEC_DIR, report)

    # Render output, grouped by category.
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
