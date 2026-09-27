#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""check_determinism_anchors.py — static drift-linter for determinism.rs in-test anchor constants.

Sub-second, no engine execution.  Asserts that every non-cfg-gated
const ANCHOR / const ANCHOR_PREFIX site in
crates/backtest/tests/determinism.rs equals the corresponding
v5-realdata-medium-2026-05 SHA in evidence/anchors.toml.

ADR-0045 § D7.1 (Decision 2, primary gate).

Exit codes:
  0  All in-test constants match anchors.toml (or --pre-commit with no
     relevant staged files).
  1  One or more in-test constants are stale (drift table on stderr).
  2  Script failure (file not found, parse error, etc.).

Usage:
  python3 scripts/check_determinism_anchors.py            # full check
  python3 scripts/check_determinism_anchors.py --write    # auto-sync stale constants
  python3 scripts/check_determinism_anchors.py --pre-commit  # no-op unless staged

R3 — cfg-gate handling: any const site inside a function that has a
  #[cfg(feature = ...)] attribute is SKIPPED (the m3_* candle pair at
  lines ~809/827 has no default-binary v5-realdata-medium mapping).
"""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path
from typing import NamedTuple

REPO_ROOT = Path(__file__).resolve().parent.parent
ANCHORS_TOML = REPO_ROOT / "evidence" / "anchors.toml"
DETERMINISM_RS = REPO_ROOT / "crates" / "backtest" / "tests" / "determinism.rs"

# bug-log #115 non-vacuity floor: the number of `const ANCHOR` sites this tool must at
# least SEE. Bumped deliberately when sites are added; lowering it is a reviewable act.
# It exists because the tool once reported "OK — 14 literal(s)" while 25 existed.
# 2026-09-27: 25 -> 29 with the four R-REAL real-data-row gates (bug-log #125).
MIN_EXPECTED_SITES = 29

# Version tag that marks the canonical in-test SHA namespace.
CANONICAL_VERSION_SUFFIX = "v5-realdata-medium-2026-05"

# Scenarios whose determinism.rs constant is the SYNTHETIC (v0-fallback) body-SHA,
# NOT the matching v5-realdata-medium-2026-05 anchors.toml SHA. These v5 anchor
# rows were emitted from the REAL-DATA path (17544-bar Binance bodies); the
# determinism tests run the v0 synthetic fallback (525600 bars, CWD=tempdir).
# See ADR-0045 § D6.3 / § D7.1b and the engine-drift-fix BLOCKER resolution.
#
# R4: these values live in BOTH this dict AND the in-test constants. They must
# stay identical. If the engine moves these synthetic SHAs, update BOTH places.
SYNTHETIC_DETERMINISM_SHAS: dict[str, str] = {
    "btc-2023-1m-macd-trend":         "4d8192af7238f5e6ab4b8c95462c402210ae846a97f2484db1c600fb6e5e9d2a",
    "btc-2023-1m-rsi-reversion":      "4a7447885164b0b2f762402d8a580e7a546543b95ed8d6f8a52feff2ce1d8ab7",
    "btc-2023-1m-bbands-mean-revert": "5037accb3118d3aafe654c58b60878e75d884bc1ce6dbaf82748c2379c80a894",
}


# ---------------------------------------------------------------------------
# Named types
# ---------------------------------------------------------------------------

class AnchorEntry(NamedTuple):
    scenario: str
    version: str
    sha256: str


class InTestSite(NamedTuple):
    lineno: int           # 1-based
    fn_name: str
    scenario: str
    const_name: str       # "ANCHOR" or "ANCHOR_PREFIX"
    literal: str
    cfg_gated: bool       # True → skip (R3)
    # bug-log #123: the namespace this site's pin belongs to, declared by the site via
    # `// anchor-ns: <substring>`. None = the canonical suffix, which is what every
    # pre-#115 site meant implicitly.
    declared_ns: str | None = None


# ---------------------------------------------------------------------------
# anchors.toml parser
# ---------------------------------------------------------------------------

def parse_anchors_toml(path: Path) -> list[AnchorEntry]:
    """Parse evidence/anchors.toml into a list of AnchorEntry records.

    Handles multi-line [[anchors]] TOML blocks.  Does NOT use a full TOML
    library to avoid requiring tomllib on Python <3.11 (though 3.11 ships it).
    Falls back to re-based parsing for compatibility.
    """
    text = path.read_text(encoding="utf-8")
    entries: list[AnchorEntry] = []

    # Split on [[anchors]] boundaries.
    blocks = re.split(r"\[\[anchors\]\]", text)
    for block in blocks[1:]:  # skip preamble before first [[anchors]]
        scenario_m = re.search(r'^scenario\s*=\s*"([^"]+)"', block, re.MULTILINE)
        version_m = re.search(r'^version\s*=\s*"([^"]+)"', block, re.MULTILINE)
        sha256_m = re.search(r'^sha256\s*=\s*"([^"]+)"', block, re.MULTILINE)
        if scenario_m and version_m and sha256_m:
            entries.append(AnchorEntry(
                scenario=scenario_m.group(1),
                version=version_m.group(1),
                sha256=sha256_m.group(1),
            ))
    return entries


def canonical_sha_map(anchors: list[AnchorEntry]) -> dict[str, str]:
    """Return {scenario: sha256} for rows whose version contains the canonical suffix."""
    return {
        a.scenario: a.sha256
        for a in anchors
        if CANONICAL_VERSION_SUFFIX in a.version
    }


def sha_for(anchors: list[AnchorEntry], scenario: str, ns: str | None) -> str | None:
    """The anchored SHA for `scenario` under the namespace `ns` (a `version` substring).

    `ns is None` means the canonical suffix — the implicit meaning of every site written
    before bug-log #115. An explicit `ns` is how a site says "my row is the noop-baseline
    one", which the real-data gates need because the DEFAULT invocation is zero-sim-slippage
    (bug-log #123). Returns None when no row matches, which the caller reports as a hard
    error rather than a skip.
    """
    needle = CANONICAL_VERSION_SUFFIX if ns is None else ns
    hits = [a.sha256 for a in anchors if a.scenario == scenario and needle in a.version]
    if len(hits) == 1:
        return hits[0]
    if len(hits) > 1:
        # Ambiguity is a finding: the declaration is not specific enough to identify one row.
        return None
    return None


# ---------------------------------------------------------------------------
# determinism.rs parser
# ---------------------------------------------------------------------------

def _is_cfg_feature_line(line: str) -> bool:
    """Return True if the line is a cfg(feature=...) attribute."""
    return bool(re.match(r"\s*#\[cfg\(feature\s*=", line))


def parse_determinism_rs(path: Path) -> list[InTestSite]:
    """Extract all const ANCHOR / const ANCHOR_PREFIX sites in determinism.rs.

    R3: skips any site inside a #[cfg(feature = ...)] function.
    Strategy:
      - Walk line by line, tracking whether we are inside a cfg-gated fn.
      - A cfg-gated fn starts when a #[cfg(feature=...)] attribute precedes
        a `#[test]` / `fn ...` line at the top scope.
      - The cfg gate is cleared after the closing brace of that fn.
    """
    lines = path.read_text(encoding="utf-8").splitlines()
    sites: list[InTestSite] = []

    # State machine.
    unresolved: list[UnresolvedSite] = []
    in_cfg_gated_fn = False
    pending_cfg = False       # saw #[cfg(feature=...)] on previous line(s)
    brace_depth = 0
    fn_name: str = "unknown"
    fn_start_depth = 0

    # Regex patterns.
    const_anchor_re = re.compile(
        r'const\s+(ANCHOR(?:_PREFIX)?)\s*:\s*&str\s*=\s*"([0-9a-fA-F]{8,64})"'
    )
    # bug-log #115 — this used to be `scenario_body_hex\("([^"]+)"\)` ONLY. Sites whose
    # runner is spelled differently — `scenario_body_hex_candle(`,
    # `assert_reproduces_canonical_anchor(`, `assert_reproduces_with_flags(`,
    # `assert_reproduces_or_report_unmeasured(`, `assert_pairs_reproduces(` — resolved to
    # None and were DROPPED silently: not counted, not warned, and reported as
    # "0 skipped". It saw 14 of 25 sites and said so as if that were all of them, and the
    # 11 it could not see were exactly the ones bug-log #111/#112 proved stale.
    #
    # Match the SCENARIO rather than the runner: any quoted argument in the window that
    # has a scenario name's shape. A new runner spelling then needs no change here, which
    # is the point — the old regex made every new helper invisible by default.
    scenario_call_re = re.compile(r'"([a-z][a-z0-9]*(?:-[a-z0-9]+){2,})"')
    # bug-log #115/#123 — a site's target NAMESPACE is a property of the site, not a
    # global constant. The old tool assumed every pin mirrored the
    # `v5-realdata-medium-2026-05` row; once it could see all 25 sites it reported 9 of
    # them "stale" because 7 target `noop-baseline` (the default real-data invocation is
    # zero-sim-slippage) and 2 target `v3.0.0-regime`. Running --write on that assumption
    # would have re-pinned correct gates to the wrong rows — bug-log #77 exactly.
    #
    # So the SITE declares it, with `// anchor-ns: <substring of the version field>`
    # within its window. No declaration means the canonical suffix, which keeps every
    # pre-existing site working unchanged.
    anchor_ns_re = re.compile(r'//\s*anchor-ns:\s*(\S+)')
    fn_decl_re = re.compile(r'^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)')

    # We also need to track the current test fn name for ANCHOR context.
    current_fn: str = "unknown"
    current_fn_cfg_gated: bool = False
    current_fn_brace_depth: int = 0

    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()

        # Detect #[cfg(feature = ...)] attribute.
        if _is_cfg_feature_line(stripped):
            pending_cfg = True
            i += 1
            continue

        # Detect fn declaration — note whether it was preceded by cfg(feature).
        fn_m = fn_decl_re.match(line)
        if fn_m and "{" in line:
            fn_nm = fn_m.group(1)
            current_fn = fn_nm
            current_fn_cfg_gated = pending_cfg
            current_fn_brace_depth = brace_depth
            pending_cfg = False

        # Track brace depth.
        brace_depth += stripped.count("{") - stripped.count("}")

        # Detect const ANCHOR / ANCHOR_PREFIX inside current fn scope.
        const_m = const_anchor_re.search(line)
        if const_m:
            const_name = const_m.group(1)
            literal = const_m.group(2)

            # Look forward a few lines to find the scenario_body_hex call.
            scenario: str | None = None
            declared_ns: str | None = None
            # The declaration belongs to THIS site: scan BACKWARDS only, at most three
            # lines, and stop at a `fn` boundary.
            #
            # My first version scanned a wide window in both directions, and the probe that
            # caught it is the one worth keeping: removing a declaration left the tool
            # GREEN, because the site had picked up the NEXT site's declaration. A window
            # wide enough to be forgiving is wide enough to launder one site's meaning into
            # another — the same bleed as a `grep` that matches the wrong file.
            for j in range(i - 1, max(-1, i - 4), -1):
                if "fn " in lines[j]:
                    break
                ns_m = anchor_ns_re.search(lines[j])
                if ns_m:
                    declared_ns = ns_m.group(1)
                    break
            for j in range(i + 1, min(i + 8, len(lines))):
                sc_m = scenario_call_re.search(lines[j])
                if sc_m:
                    scenario = sc_m.group(1)
                    break

            if scenario is None:
                # bug-log #115: an unresolvable site is a FINDING, not a silent drop.
                unresolved.append(UnresolvedSite(
                    lineno=i + 1,
                    fn_name=current_fn,
                    const_name=const_name,
                    literal=literal,
                ))
            else:
                sites.append(InTestSite(
                    lineno=i + 1,
                    fn_name=current_fn,
                    scenario=scenario,
                    const_name=const_name,
                    literal=literal,
                    cfg_gated=current_fn_cfg_gated,
                    declared_ns=declared_ns,
                ))

        # Reset pending_cfg unless it was just set.
        if not _is_cfg_feature_line(stripped):
            pending_cfg = False

        i += 1

    return sites, unresolved


# ---------------------------------------------------------------------------
# Drift detection
# ---------------------------------------------------------------------------

class UnresolvedSite(NamedTuple):
    """A `const ANCHOR` whose scenario could not be resolved (bug-log #115).

    Reported as a hard failure. The alternative — dropping it — is what let 11 of 25
    sites go unchecked for months while the tool printed "0 skipped".
    """

    lineno: int
    fn_name: str
    const_name: str
    literal: str


class DriftRow(NamedTuple):
    lineno: int
    fn_name: str
    scenario: str
    in_test: str      # truncated for display
    canonical: str    # truncated for display
    match: bool
    note: str


def detect_drift(
    sites: list[InTestSite],
    canonical: dict[str, str],
    anchors: list[AnchorEntry] | None = None,
) -> tuple[list[DriftRow], list[DriftRow]]:
    """Return (mismatches, skipped) from comparing sites against the dual-map.

    Resolution order (ADR-0045 § D7.1b / EX-4 v2):
      1. SYNTHETIC_DETERMINISM_SHAS[scenario] if present — full equality.
      2. canonical[scenario] (v5-realdata-medium-2026-05 anchors.toml row) — full
         equality; ANCHOR_PREFIX sites use startswith until EX-2 converts them.
      3. Neither map has the scenario → HARD ERROR (added to mismatches, not
         skipped). This closes the blind spot the old "not in anchors.toml → skip"
         branch would reopen for any newly added but unmapped test fn.

    skipped: cfg-gated sites only (R3).
    mismatches: sites where in-test literal != expected, OR scenario is in neither map.
    """
    mismatches: list[DriftRow] = []
    skipped: list[DriftRow] = []

    for site in sites:
        if site.cfg_gated:
            skipped.append(DriftRow(
                lineno=site.lineno,
                fn_name=site.fn_name,
                scenario=site.scenario,
                in_test=site.literal[:16] + "…",
                canonical="(cfg-gated — skip)",
                match=True,
                note="R3: cfg(feature) gate",
            ))
            continue

        # --- Dual-map resolution (step 1: synthetic override) ---
        #
        # An EXPLICIT `// anchor-ns:` declaration beats this scenario-keyed default. Two
        # sites can pin the same scenario under different conditions — `btc-2023-1m-*` has
        # a t622 pin for the synthetic path and a real-data pin for the canonical row — and
        # a scenario-level override would compare the second against the first's expectation
        # and call a correct gate stale. The site knows which row it asserts; the site wins.
        synth = None if site.declared_ns is not None else SYNTHETIC_DETERMINISM_SHAS.get(site.scenario)
        if synth is not None:
            match = site.literal == synth
            row = DriftRow(
                lineno=site.lineno,
                fn_name=site.fn_name,
                scenario=site.scenario,
                in_test=site.literal[:16] + "…",
                canonical=synth[:16] + "… (synthetic)",
                match=match,
                note="SYNTHETIC_DETERMINISM_SHAS" if match else "synthetic SHA mismatch",
            )
            if not match:
                mismatches.append(row)
            continue

        # --- Dual-map resolution (step 2: anchors.toml, NAMESPACE-AWARE) ---
        # bug-log #123: resolve the row this site DECLARES, falling back to the canonical
        # suffix when it declares nothing. Comparing a noop-baseline pin against the
        # canonical row reports a correct gate as stale — and `--write` would then "fix"
        # it by breaking it (bug-log #77).
        if anchors is not None:
            can = sha_for(anchors, site.scenario, site.declared_ns)
        else:
            can = canonical.get(site.scenario)
        if can is not None:
            # For ANCHOR_PREFIX, check starts_with; for ANCHOR, full equality.
            if site.const_name == "ANCHOR_PREFIX":
                match = can.startswith(site.literal)
            else:
                match = site.literal == can

            row = DriftRow(
                lineno=site.lineno,
                fn_name=site.fn_name,
                scenario=site.scenario,
                in_test=site.literal[:16] + "…",
                canonical=can[:16] + "…",
                match=match,
                note="" if site.declared_ns is None else f"ns={site.declared_ns}",
            )
            if not match:
                mismatches.append(row)
            continue

        # --- Dual-map resolution (step 3: HARD ERROR — closes blind spot) ---
        # A non-cfg-gated *_anchor_hash_unchanged fn whose scenario is in
        # neither map is an unanchored constant — it cannot be validated.
        # This is always a configuration error; fail loudly.
        mismatches.append(DriftRow(
            lineno=site.lineno,
            fn_name=site.fn_name,
            scenario=site.scenario,
            in_test=site.literal[:16] + "…",
            canonical="(HARD ERROR: no row for the declared namespace)",
            match=False,
            note=(
                f"declared ns={site.declared_ns!r} matches no single anchors.toml row for "
                "this scenario — fix the declaration or add the row"
                if site.declared_ns is not None
                else "add scenario to SYNTHETIC_DETERMINISM_SHAS or anchors.toml"
            ),
        ))

    return mismatches, skipped


# ---------------------------------------------------------------------------
# --write mode: rewrite stale literals in place
# ---------------------------------------------------------------------------

def apply_write(
    path: Path,
    sites: list[InTestSite],
    canonical: dict[str, str],
) -> int:
    """Rewrite stale ANCHOR literals in determinism.rs using the dual-map.

    Resolution order (mirrors detect_drift, EX-4 v2 / ADR-0045 § D7.1b):
      1. SYNTHETIC_DETERMINISM_SHAS[scenario] — written as full ANCHOR equality.
      2. canonical[scenario] (v5-realdata-medium-2026-05 anchors.toml row).
      3. Neither map → skip with a warning (HARD ERROR only at check time;
         --write cannot fabricate a correct value).

    For ANCHOR (full-hash) sites: updates the literal in place.
    For ANCHOR_PREFIX sites: updates the literal AND renames the const to ANCHOR,
    but does NOT update the assert!(..starts_with..) call — that multi-line
    Rust change requires manual review (see EX-2 in the engine-drift-fix spec).
    A warning is printed for each ANCHOR_PREFIX site so the developer knows
    to complete the assert_eq! conversion manually.

    Returns number of sites rewritten.  Skips cfg-gated and unmapped sites.
    """
    text = path.read_text(encoding="utf-8")
    lines_out = text.splitlines(keepends=True)
    rewrites = 0
    prefix_warnings: list[str] = []

    for site in sites:
        if site.cfg_gated:
            continue

        # --- Dual-map resolution for --write (same order as detect_drift) ---
        synth = SYNTHETIC_DETERMINISM_SHAS.get(site.scenario)
        if synth is not None:
            target = synth
        else:
            target = canonical.get(site.scenario)
            if target is None:
                # Neither map — cannot write a correct value; skip with warning.
                print(
                    f"WARN: --write skipped {site.fn_name} ({path.name}:{site.lineno}): "
                    f"scenario '{site.scenario}' not in SYNTHETIC_DETERMINISM_SHAS "
                    f"or anchors.toml — add it manually.",
                    file=sys.stderr,
                )
                continue

        # Check if already current (using dual-map expected value).
        if site.const_name == "ANCHOR":
            is_match = site.literal == target
        else:  # ANCHOR_PREFIX
            is_match = target.startswith(site.literal)
        if is_match:
            continue

        lineno_0 = site.lineno - 1  # 0-based index
        old_line = lines_out[lineno_0]

        if site.const_name == "ANCHOR_PREFIX":
            # Rename const ANCHOR_PREFIX → ANCHOR and update the literal.
            new_line = re.sub(
                r'const\s+ANCHOR_PREFIX\s*:\s*&str\s*=\s*"[0-9a-fA-F]+"',
                f'const ANCHOR: &str = "{target}"',
                old_line,
            )
            lines_out[lineno_0] = new_line
            prefix_warnings.append(
                f"  MANUAL NEEDED: {site.fn_name} "
                f"({path.name}:{site.lineno}): "
                f"const renamed ANCHOR_PREFIX→ANCHOR but assert!(..starts_with..) "
                f"must be converted to assert_eq!(hex, ANCHOR, ..) manually."
            )
        else:
            new_line = re.sub(
                r'const\s+ANCHOR\s*:\s*&str\s*=\s*"[0-9a-fA-F]+"',
                f'const ANCHOR: &str = "{target}"',
                old_line,
            )
            lines_out[lineno_0] = new_line

        rewrites += 1

    if rewrites:
        path.write_text("".join(lines_out), encoding="utf-8")

    if prefix_warnings:
        print("\nWARN: ANCHOR_PREFIX→ANCHOR conversion incomplete — manual assert fixup needed:", file=sys.stderr)
        for w in prefix_warnings:
            print(w, file=sys.stderr)

    return rewrites


# ---------------------------------------------------------------------------
# Reporting
# ---------------------------------------------------------------------------

def _truncate(s: str, n: int = 20) -> str:
    return s[:n] + "…" if len(s) > n else s


def print_drift_table(mismatches: list[DriftRow], skipped: list[DriftRow]) -> None:
    """Print a markdown drift table to stderr."""
    print("\n## Determinism anchor drift detected\n", file=sys.stderr)
    print(
        "| scenario | fn (file:line) | in-test | anchors.toml | match? |",
        file=sys.stderr,
    )
    print("|----------|----------------|---------|--------------|--------|", file=sys.stderr)
    for row in mismatches:
        print(
            f"| {row.scenario} "
            f"| {row.fn_name} ({DETERMINISM_RS.name}:{row.lineno}) "
            f"| `{_truncate(row.in_test)}` "
            f"| `{_truncate(row.canonical)}` "
            f"| NO |",
            file=sys.stderr,
        )
    if skipped:
        print("\n### Skipped (no canonical mapping or cfg-gated)\n", file=sys.stderr)
        print(
            "| scenario | fn (file:line) | note |",
            file=sys.stderr,
        )
        print("|----------|----------------|------|", file=sys.stderr)
        for row in skipped:
            print(
                f"| {row.scenario} "
                f"| {row.fn_name} ({DETERMINISM_RS.name}:{row.lineno}) "
                f"| {row.note} |",
                file=sys.stderr,
            )


# ---------------------------------------------------------------------------
# Pre-commit gate (--pre-commit flag)
# ---------------------------------------------------------------------------

def _relevant_files_staged() -> bool:
    """Return True if determinism.rs or anchors.toml is staged."""
    try:
        result = subprocess.run(
            [
                "git", "diff", "--cached", "--name-only",
                "--",
                "crates/backtest/tests/determinism.rs",
                "evidence/anchors.toml",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError:
        return True  # git not available → run the check unconditionally
    return bool(result.stdout.strip())


# ---------------------------------------------------------------------------
# CLI entrypoint
# ---------------------------------------------------------------------------

def _self_test() -> int:  # noqa: C901
    """Prove the linter can fail, on fixtures — bug-log #115 requirement 4.

    Every probe here corresponds to a way this tool was once silently wrong:

    - **P1** a runner spelling it did not know. The old regex matched only
      `scenario_body_hex(`, so sites using any other helper resolved to None and were
      DROPPED — 11 of 25, while the output said "0 skipped".
    - **P2** a `const ANCHOR` with no scenario anywhere near it. Previously dropped in
      silence; now a hard failure, because an unvalidatable pin is a finding.
    - **P3** a declaration bleeding from one site into the next. My first implementation
      scanned a wide window in both directions, so REMOVING a declaration left the tool
      green — it had borrowed the neighbour's. This probe is why the window is
      backwards-only and stops at a `fn` boundary.
    - **P4** the non-vacuity floor.

    A linter without this mode is exactly the thing bug-log `#115` is about: it reports a
    number nobody has checked it can fail to produce.
    """
    import tempfile

    failures: list[str] = []

    def probe(name: str, ok: bool, detail: str) -> None:
        if ok:
            print(f"  self-test {name}: FIRED (correct)")
        else:
            failures.append(f"{name}: {detail}")
            print(f"  self-test {name}: DID NOT FIRE — {detail}", file=sys.stderr)

    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)

        # P1 — three different runner spellings must all resolve.
        p1 = tmp / "p1.rs"
        p1.write_text(
            'fn a() {\n    const ANCHOR: &str = "aaaaaaaa";\n    scenario_body_hex("alpha-one-two");\n}\n'
            'fn b() {\n    const ANCHOR: &str = "bbbbbbbb";\n    scenario_body_hex_candle("beta-one-two");\n}\n'
            'fn c() {\n    const ANCHOR: &str = "cccccccc";\n'
            '    assert_reproduces_with_flags("gamma-one-two", ANCHOR, &[]);\n}\n',
            encoding="utf-8",
        )
        s1, u1 = parse_determinism_rs(p1)
        probe(
            "P1 three runner spellings resolve",
            len(s1) == 3 and not u1,
            f"expected 3 resolved / 0 unresolved, got {len(s1)} / {len(u1)}",
        )

        # P2 — a const with no scenario must be UNRESOLVED, not dropped.
        p2 = tmp / "p2.rs"
        p2.write_text('fn d() {\n    const ANCHOR: &str = "dddddddd";\n    something_else();\n}\n', encoding="utf-8")
        s2, u2 = parse_determinism_rs(p2)
        probe(
            "P2 scenario-less const is unresolved",
            len(s2) == 0 and len(u2) == 1,
            f"expected 0 resolved / 1 unresolved, got {len(s2)} / {len(u2)}",
        )

        # P3 — a declaration must NOT bleed into the next site.
        p3 = tmp / "p3.rs"
        p3.write_text(
            'fn e() {\n    const ANCHOR: &str = "eeeeeeee";\n    scenario_body_hex("delta-one-two");\n}\n'
            'fn f() {\n    // anchor-ns: noop-baseline\n    const ANCHOR: &str = "ffffffff";\n'
            '    scenario_body_hex("epsilon-one-two");\n}\n',
            encoding="utf-8",
        )
        s3, _ = parse_determinism_rs(p3)
        by_fn = {s.fn_name: s.declared_ns for s in s3}
        probe(
            "P3 declaration does not bleed between sites",
            by_fn.get("e") is None and by_fn.get("f") == "noop-baseline",
            f"expected e=None, f='noop-baseline'; got {by_fn}",
        )

        # P4 — the floor is a real assertion, not decoration.
        probe(
            "P4 floor is at least the sites we ship",
            MIN_EXPECTED_SITES >= 29,
            f"MIN_EXPECTED_SITES={MIN_EXPECTED_SITES} is below the 29 sites shipped 2026-09-27",
        )

    if failures:
        print(
            f"\ncheck_determinism_anchors --self-test: FAIL — {len(failures)} probe(s) did not "
            "fire. The linter cannot be trusted to fail, which is bug-log #115.",
            file=sys.stderr,
        )
        return 1
    print("check_determinism_anchors --self-test: OK — 4 probes fired")
    return 0


def main() -> int:  # noqa: C901
    parser = argparse.ArgumentParser(
        description="Static drift-linter: asserts determinism.rs constants == anchors.toml canonical SHAs."
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="Rewrite stale in-test literals to anchors.toml SHAs in place.",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        dest="self_test",
        help="Prove this linter can FAIL, on fixtures. Exits non-zero if any probe does not fire.",
    )
    parser.add_argument(
        "--pre-commit",
        action="store_true",
        dest="pre_commit",
        help="No-op if neither determinism.rs nor anchors.toml is staged.",
    )
    args = parser.parse_args()

    if args.self_test:
        return _self_test()

    # Pre-commit fast-path: do nothing if nothing relevant is staged.
    if args.pre_commit and not _relevant_files_staged():
        return 0

    # Load inputs.
    try:
        anchors = parse_anchors_toml(ANCHORS_TOML)
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: could not parse {ANCHORS_TOML}: {exc}", file=sys.stderr)
        return 2

    try:
        sites, unresolved = parse_determinism_rs(DETERMINISM_RS)
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: could not parse {DETERMINISM_RS}: {exc}", file=sys.stderr)
        return 2

    canonical = canonical_sha_map(anchors)

    # bug-log #115 — a total parse failure used to WARN and exit 0. A linter that finds
    # nothing and reports success is the failure mode it exists to prevent.
    if not sites:
        print(
            "FAIL: no const ANCHOR site resolved in determinism.rs. That is a parser "
            "regression, not an empty file — see bug-log #115.",
            file=sys.stderr,
        )
        return 1

    # Non-vacuity floor. The count is printed either way, so a silent shrink is visible.
    if len(sites) + len(unresolved) < MIN_EXPECTED_SITES:
        print(
            f"FAIL: found {len(sites) + len(unresolved)} const ANCHOR site(s), floor is "
            f"{MIN_EXPECTED_SITES}. Either sites were deleted (update the floor in the same "
            "commit) or the parser stopped seeing them (bug-log #115).",
            file=sys.stderr,
        )
        return 1

    if unresolved:
        print(
            f"FAIL: {len(unresolved)} const ANCHOR site(s) whose scenario could not be "
            "resolved. These would previously have been dropped SILENTLY and reported as "
            '"0 skipped" (bug-log #115):',
            file=sys.stderr,
        )
        for u in unresolved:
            print(
                f"  determinism.rs:{u.lineno} fn {u.fn_name} — {u.const_name} = "
                f"{u.literal[:16]}… (no scenario string found within 8 lines)",
                file=sys.stderr,
            )
        print(
            "\nEither the site names its scenario within 8 lines of the const, or this "
            "tool is extended deliberately — never by dropping the site.",
            file=sys.stderr,
        )
        return 1

    mismatches, skipped = detect_drift(sites, canonical, anchors)

    if args.write:
        # bug-log #115/#123 — `--write` syncs against the CANONICAL map only. For a site
        # that declares a different namespace that would re-pin a CORRECT gate to the wrong
        # row, which is bug-log #77: converting a caught drift into a silent one, by the
        # hand of the tool meant to catch it. Refuse rather than do it partially.
        declared = [s for s in sites if s.declared_ns is not None]
        if declared:
            print(
                f"REFUSING --write: {len(declared)} site(s) declare a non-canonical "
                "namespace, and --write only knows the canonical map. Writing would re-pin "
                "correct gates to the wrong rows (bug-log #77). Fix the literals by hand, "
                "or teach --write to honour `// anchor-ns:` first. Sites:",
                file=sys.stderr,
            )
            for s in declared:
                print(
                    f"  determinism.rs:{s.lineno} fn {s.fn_name} — anchor-ns: {s.declared_ns}",
                    file=sys.stderr,
                )
            return 1
        n = apply_write(DETERMINISM_RS, sites, canonical)
        if n:
            print(f"check_determinism_anchors: rewrote {n} stale literal(s) in {DETERMINISM_RS.name}")
        else:
            print("check_determinism_anchors: all literals already current — no changes")
        return 0

    if mismatches:
        print_drift_table(mismatches, skipped)
        print(
            f"\ncheck_determinism_anchors: FAIL — {len(mismatches)} stale literal(s). "
            "Run with --write to auto-sync.",
            file=sys.stderr,
        )
        return 1

    n_total = len(sites)
    n_skip = len(skipped)
    n_ok = n_total - n_skip
    # Count synthetic vs canonical matches for informational output.
    # Count what the RESOLUTION did, not what the scenario name suggests. The synthetic
    # override is skipped for a site that declares its namespace, so a scenario-keyed count
    # over-reports — the same "a number nobody measured" shape this tool exists to catch,
    # in its own summary line.
    n_synth = sum(
        1 for s in sites
        if not s.cfg_gated
        and s.declared_ns is None
        and s.scenario in SYNTHETIC_DETERMINISM_SHAS
    )
    n_canonical = n_ok - n_synth
    print(
        f"check_determinism_anchors: OK — {n_ok} of {n_total} resolved literal(s) match "
        f"({n_canonical} canonical v5-realdata-medium-2026-05, {n_synth} synthetic; "
        f"{n_skip} skipped: cfg-gated; 0 unresolved). "
        f"Sites seen: {n_total} (floor {MIN_EXPECTED_SITES})."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
