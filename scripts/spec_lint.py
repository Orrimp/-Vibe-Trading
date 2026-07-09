#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""spec_lint.py — structural integrity check for spec/.

Companion to scripts/verify_anchors.sh (which checks content hashes).
This script checks shape: dead links, missing frontmatter, orphan
feature folders, anchor coverage, trace.toml row validity, and
pipeline status drift (deck + PASS report ⇒ status ≥ presenter-done).

Exit code = number of violation CATEGORIES that triggered (0 = clean).
Pass --all to print every violation regardless of category count.

Usage:
    uv run scripts/spec_lint.py            # whole spec/ tree (preferred)
    uv run scripts/spec_lint.py spec/<slug>  # restrict to one folder
    uv run scripts/spec_lint.py --all      # verbose
    uv run scripts/spec_lint.py --self-test  # synthetic-fixture check of the
                                             # status-drift rule (exit 0 = ok)

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
                        # account, third-party data, etc.). See spec/v1/v3-llm-forecaster/reports/
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
    # target exists at spec/architecture/adr/). A raw `../`→`../../` edit breaks
    # the body-SHA; the proper fix is the ADR-0038 §D6.c documentation-link-fix
    # re-emission protocol (NOT YET CODIFIED — see CLAUDE.md). Exempted here
    # rather than re-emitting a retired line.
    (
        "spec/v1/v3-volatility-forecaster/reports/vol-verdict-bs1-realdata-20260522.md",
        "../architecture/adr/0038-vol-forecast-verdict-shape.md"
        "#d1-v-verdict-priority-tree-parallel-to-adr-0033--d3-not-extension",
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
NON_FEATURE_FOLDERS = {"design", "dev-notes", "runbooks", "archive", "architecture",
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

def check_anchors(spec_dir: Path, report: Report) -> dict[str, dict]:
    anchors_path = spec_dir / "anchors.toml"
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

def check_shipped_have_tests(spec_dir: Path, report: Report) -> None:
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
        reports_dir = child / "reports"
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


def check_status_drift(spec_dir: Path, report: Report) -> None:
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
    """
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
        has_pass = any(
            VERDICT_PASS_RE.search(p.read_text(encoding="utf-8", errors="replace"))
            for p in (child / "reports").glob("test-*.md")
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


def _self_test_status_drift() -> bool:
    """Synthetic-fixture proof that the status-drift rule fires and clears.

    Three fixtures in a tempdir: (a) drifting — tester-done + deck + PASS
    report → exactly 1 violation; (b) compliant — same artifacts at
    presenter-done → 0; (c) deck but no PASS report → 0. Returns True iff all
    three behave.
    """
    import tempfile

    def make_feature(root: Path, slug: str, status: str,
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
            (d / "reports").mkdir()
            (d / "reports" / "test-2026-06-12.md").write_text("VERDICT → PASS\n")

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        make_feature(root, "drifting", "tester-done", deck=True, pass_report=True)
        make_feature(root, "compliant", "presenter-done", deck=True, pass_report=True)
        make_feature(root, "no-pass-yet", "tester-done", deck=True, pass_report=False)
        rep = Report()
        check_status_drift(root, rep)
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


def self_test() -> int:
    """Run every rule's synthetic-fixture self-test. Exit 0 iff all pass."""
    results = [
        _self_test_status_drift(),
        _self_test_feature_shipped_trace_drift(),
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
                # Skip byte-immutable anchored report bodies under the v1 archive.
                # The 2026-06-28 v1/v2 reorg moved them one level deeper, so their
                # internal relative links are off-by-one — but they CANNOT be
                # repaired without changing the body bytes and breaking the
                # body-SHA-256 anchors (CLAUDE.md non-negotiable). Frozen evidence.
                if rel.startswith("spec/v1/") and "/reports/" in rel:
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
        help="run the status-drift rule against synthetic fixtures and exit",
    )
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()

    if not SPEC_DIR.exists():
        print(f"error: spec/ not found at {SPEC_DIR}", file=sys.stderr)
        return 99

    roots = [Path(p).resolve() for p in args.paths] if args.paths else [SPEC_DIR]
    report = Report()

    # Per-file checks (links + frontmatter).
    for md in iter_spec_md(roots):
        text = md.read_text(encoding="utf-8", errors="replace")
        check_dead_links(md, text, report)
        check_frontmatter(md, text, report)

    # Tree-level checks (only when running over the whole spec/).
    if not args.paths or any(Path(p).resolve() == SPEC_DIR for p in args.paths):
        check_orphan_features(SPEC_DIR, report)
        anchors = check_anchors(SPEC_DIR, report)
        check_trace(SPEC_DIR, report, anchors)
        check_shipped_have_tests(SPEC_DIR, report)
        check_status_drift(SPEC_DIR, report)
        check_feature_shipped_trace_drift(SPEC_DIR, report)

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
