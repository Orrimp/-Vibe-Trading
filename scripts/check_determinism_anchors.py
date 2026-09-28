#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""check_determinism_anchors.py — static drift-linter for in-test anchor-SHA constants.

Sub-second, no engine execution.  Asserts that every non-cfg-gated
`const <…ANCHOR…>: &str = "<hex>"` site in the SCANNED_FILES list equals the
anchors.toml row it targets — the v5-realdata-medium-2026-05 row by default, or
whichever row the site names with `// anchor-ns:`.

ADR-0045 § D7.1 (Decision 2, primary gate).

bug-log #131 — the tool's own SCOPE was the unstated assumption. Until
2026-09-28 it scanned ONE file, `determinism.rs`, and printed
"OK — 29 of 29 resolved literal(s) match": true of that file, silent about the
four other test files that carry six more anchor literals. The same defect it
was built to prevent, one level up. Two things follow, and both are load-bearing:

  * the file list is explicit and the summary prints a PER-FILE count, so a file
    dropping out of scope shows up in the output rather than only in a total;
  * the const NAME pattern is broad (anything containing ANCHOR) and the filter
    that keeps non-digests out is the VALUE shape (8–64 hex chars), not the name.

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
TESTS_DIR = REPO_ROOT / "crates" / "backtest" / "tests"
DETERMINISM_RS = TESTS_DIR / "determinism.rs"

# bug-log #131 — every file that carries an in-test anchor-SHA literal, not just the
# first one anybody wrote. Derived mechanically on 2026-09-28 with
#   grep -rln 'const [A-Z_]*ANCHOR[A-Z_]*\s*:\s*&str' crates/ --include='*.rs'
# and re-derivable the same way; `crates/ui/src/strings.rs` also matches that NAME grep
# and is correctly absent here because its value is the UI label "Anchor: ", not a digest
# (the hex VALUE filter in `const_anchor_re` is what rules it out — see below).
#
# A file added to this list needs the floor below raised in the same edit. A file REMOVED
# from it shows up as a missing per-file row in the summary line, which is the point: the
# pre-#131 tool could not distinguish "this file has no literals" from "this file is not
# looked at".
SCANNED_FILES: tuple[Path, ...] = (
    DETERMINISM_RS,
    TESTS_DIR / "multi_pair_determinism.rs",
    TESTS_DIR / "run_yahoo_sma_ticker_flag.rs",
    TESTS_DIR / "theta_surface_reproduction.rs",
    TESTS_DIR / "reproducibility_sample_figure.rs",
)

# bug-log #115 non-vacuity floor: the number of anchor-literal sites this tool must at
# least SEE. Bumped deliberately when sites are added; lowering it is a reviewable act.
# It exists because the tool once reported "OK — 14 literal(s)" while 25 existed.
# 2026-09-27: 25 -> 29 with the four R-REAL real-data-row gates (bug-log #125).
# 2026-09-28: 29 -> 35 with the SCOPE fix (bug-log #131). The +6 are not new gates — they
#   were in the tree all along, in four files the tool never opened: 2 in
#   multi_pair_determinism.rs, 2 in run_yahoo_sma_ticker_flag.rs, 1 in
#   theta_surface_reproduction.rs, 1 in reproducibility_sample_figure.rs.
MIN_EXPECTED_SITES = 35

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
    file: str             # bug-log #131: which file — the tool scans five, not one
    lineno: int           # 1-based
    fn_name: str
    scenario: str
    const_name: str       # any name containing ANCHOR; *_PREFIX means prefix-compare
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


def _is_prefix_site(const_name: str) -> bool:
    """Whether the site compares a PREFIX of the anchored SHA rather than the whole thing.

    bug-log #131: was an exact `const_name == "ANCHOR_PREFIX"` test, which silently became
    full-equality for any other name once the name pattern broadened. `*_PREFIX` is the
    convention; the repo currently ships zero such sites (EX-2 converted them all), so this
    only guards the next one.
    """
    return const_name.endswith("_PREFIX")


def parse_anchor_literals(path: Path) -> tuple[list[InTestSite], list[UnresolvedSite]]:
    """Extract every anchor-SHA const site in `path`.

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
    #
    # bug-log #131 — this used to be `const (ANCHOR(?:_PREFIX)?)` : an EXACT name match. So
    # `BTC_ANCHOR_SHA`, `ETH_ANCHOR_SHA` and `ANCHORED_BODY_SHA` — six live gates across four
    # files — were invisible, and the tool said "29 of 29" without qualification.
    #
    # Now the NAME is loose (anything containing ANCHOR) and the VALUE carries the filter:
    # 8–64 hex characters. That hex requirement is what keeps a non-digest const out —
    # `crates/ui/src/strings.rs`'s `STRATEGY_REGISTRY_LAST_ANCHOR_PREFIX: &str = "Anchor: "`
    # matches the name grep and can never match this, because `"Anchor: "` is not hex. The
    # value shape is the correct filter; the name never was.
    const_anchor_re = re.compile(
        r'const\s+([A-Za-z0-9_]*ANCHOR[A-Za-z0-9_]*)\s*:\s*&str\s*=\s*"([0-9a-fA-F]{8,64})"'
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
    # bug-log #131 — the EXPLICIT scenario declaration. The forward scan below finds a
    # scenario only when a scenario-shaped quoted string happens to sit within 7 lines of the
    # const; that is true inside a test fn and false at file scope, where
    # `run_yahoo_sma_ticker_flag.rs` keeps its two constants a hundred lines from the call
    # that uses them. The alternative — planting an incidental quoted string in the test so a
    # regex finds it — is how the PREVIOUS version of this tool laundered one site's meaning
    # into another (probe P3). A site that cannot be resolved by adjacency SAYS what it pins.
    #
    # Read in the same narrow backwards window as `// anchor-ns:`, and it WINS over the
    # forward scan: an explicit declaration must not be overridable by a nearby coincidence.
    anchor_scenario_re = re.compile(r'//\s*anchor-scenario:\s*(\S+)')
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

            # bug-log #131 — a FILE-SCOPE const belongs to no fn, and `current_fn` is
            # sticky. Two consequences, and the second is not cosmetic:
            #   * it was reported under whichever fn happened to be declared above it
            #     (`run_yahoo_sma_ticker_flag.rs`'s two constants read as
            #     `fn pinned_table_allowed_yahoo_tickers_matches_data_crate`);
            #   * it INHERITED that fn's `#[cfg(feature = ...)]`, so both would have been
            #     "skipped: cfg-gated" — the six literals this change exists to check,
            #     dropped again at the last step, and counted as fine.
            at_file_scope = brace_depth == 0
            site_fn = "(file scope)" if at_file_scope else current_fn
            site_cfg_gated = False if at_file_scope else current_fn_cfg_gated

            # Look forward a few lines to find the scenario_body_hex call.
            scenario: str | None = None
            declared_ns: str | None = None
            declared_scenario: str | None = None
            # The declaration belongs to THIS site: scan BACKWARDS only, at most three
            # lines, and stop at a `fn` boundary.
            #
            # My first version scanned a wide window in both directions, and the probe that
            # caught it is the one worth keeping: removing a declaration left the tool
            # GREEN, because the site had picked up the NEXT site's declaration. A window
            # wide enough to be forgiving is wide enough to launder one site's meaning into
            # another — the same bleed as a `grep` that matches the wrong file.
            #
            # bug-log #131 reads `// anchor-scenario:` in the SAME window, for the same
            # reason and with the same narrowness. First match of each key wins; neither key
            # short-circuits the other's search, because the window is three lines and both
            # declarations belong to the site that sits under them.
            for j in range(i - 1, max(-1, i - 4), -1):
                if "fn " in lines[j]:
                    break
                # bug-log #131 — and stop at the PREVIOUS anchor const too. The two
                # `run_yahoo_sma_ticker_flag.rs` constants sit at FILE scope with no `fn`
                # between them, so the `fn` boundary alone let the second site inherit the
                # first one's declarations: probe P7 caught exactly that, one level down
                # from the bleed P3 caught. A declaration above a const belongs to THAT
                # const.
                if const_anchor_re.search(lines[j]):
                    break
                if declared_ns is None:
                    ns_m = anchor_ns_re.search(lines[j])
                    if ns_m:
                        declared_ns = ns_m.group(1)
                if declared_scenario is None:
                    sc_decl_m = anchor_scenario_re.search(lines[j])
                    if sc_decl_m:
                        declared_scenario = sc_decl_m.group(1)
                if declared_ns is not None and declared_scenario is not None:
                    break

            if declared_scenario is not None:
                # An explicit declaration is not second-guessed by adjacency.
                scenario = declared_scenario
            else:
                for j in range(i + 1, min(i + 8, len(lines))):
                    sc_m = scenario_call_re.search(lines[j])
                    if sc_m:
                        scenario = sc_m.group(1)
                        break

            if scenario is None:
                # bug-log #115: an unresolvable site is a FINDING, not a silent drop.
                unresolved.append(UnresolvedSite(
                    file=path.name,
                    lineno=i + 1,
                    fn_name=site_fn,
                    const_name=const_name,
                    literal=literal,
                ))
            else:
                sites.append(InTestSite(
                    file=path.name,
                    lineno=i + 1,
                    fn_name=site_fn,
                    scenario=scenario,
                    const_name=const_name,
                    literal=literal,
                    cfg_gated=site_cfg_gated,
                    declared_ns=declared_ns,
                ))

        # Reset pending_cfg unless it was just set.
        if not _is_cfg_feature_line(stripped):
            pending_cfg = False

        i += 1

    return sites, unresolved


def scan_files(
    paths: tuple[Path, ...] | list[Path],
) -> tuple[list[InTestSite], list[UnresolvedSite], dict[str, int]]:
    """Parse every path in `paths`, returning (sites, unresolved, per_file_counts).

    bug-log #131 — `per_file_counts` is a row PER SCANNED FILE, present even when the
    count is zero, and it is printed in the summary. A total alone cannot tell "this file
    has no literals" from "this file was never opened", and the whole finding is that the
    tool spent months in the second state while reporting the first.

    A path that does not exist is a hard error, not a zero: a file silently renamed out of
    scope would otherwise read as "no literals here".
    """
    sites: list[InTestSite] = []
    unresolved: list[UnresolvedSite] = []
    per_file: dict[str, int] = {}
    for path in paths:
        if not path.is_file():
            raise FileNotFoundError(
                f"{path} is in SCANNED_FILES but does not exist — a scanned file that "
                "vanished reads as 'no literals' unless this fails (bug-log #131)"
            )
        s, u = parse_anchor_literals(path)
        sites.extend(s)
        unresolved.extend(u)
        per_file[path.name] = len(s) + len(u)
    return sites, unresolved, per_file


# ---------------------------------------------------------------------------
# Drift detection
# ---------------------------------------------------------------------------

class UnresolvedSite(NamedTuple):
    """A `const ANCHOR` whose scenario could not be resolved (bug-log #115).

    Reported as a hard failure. The alternative — dropping it — is what let 11 of 25
    sites go unchecked for months while the tool printed "0 skipped".
    """

    file: str
    lineno: int
    fn_name: str
    const_name: str
    literal: str


class DriftRow(NamedTuple):
    file: str
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
                file=site.file,
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
                file=site.file,
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
            if _is_prefix_site(site.const_name):
                match = can.startswith(site.literal)
            else:
                match = site.literal == can

            row = DriftRow(
                file=site.file,
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
            file=site.file,
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
        if _is_prefix_site(site.const_name):
            is_match = target.startswith(site.literal)
        else:
            is_match = site.literal == target
        if is_match:
            continue

        lineno_0 = site.lineno - 1  # 0-based index
        old_line = lines_out[lineno_0]

        # bug-log #131: rewrite the site's OWN const name. The old code hardcoded
        # `ANCHOR` / `ANCHOR_PREFIX`, so a `BTC_ANCHOR_SHA` site would have had its
        # substitution silently no-op while `rewrites` still counted it — a --write that
        # reports having fixed a literal it did not touch.
        base_name = site.const_name.removesuffix("_PREFIX")
        if _is_prefix_site(site.const_name):
            new_line = re.sub(
                rf'const\s+{re.escape(site.const_name)}\s*:\s*&str\s*=\s*"[0-9a-fA-F]+"',
                f'const {base_name}: &str = "{target}"',
                old_line,
            )
            if new_line == old_line:
                raise RuntimeError(
                    f"--write could not rewrite {site.file}:{site.lineno} "
                    f"({site.const_name}) — refusing to report a rewrite that did not land"
                )
            lines_out[lineno_0] = new_line
            prefix_warnings.append(
                f"  MANUAL NEEDED: {site.fn_name} "
                f"({site.file}:{site.lineno}): "
                f"const renamed {site.const_name}→{base_name} but assert!(..starts_with..) "
                f"must be converted to assert_eq!(hex, {base_name}, ..) manually."
            )
        else:
            new_line = re.sub(
                rf'const\s+{re.escape(site.const_name)}\s*:\s*&str\s*=\s*"[0-9a-fA-F]+"',
                f'const {site.const_name}: &str = "{target}"',
                old_line,
            )
            if new_line == old_line:
                raise RuntimeError(
                    f"--write could not rewrite {site.file}:{site.lineno} "
                    f"({site.const_name}) — refusing to report a rewrite that did not land"
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
            f"| {row.fn_name} ({row.file}:{row.lineno}) "
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
                f"| {row.fn_name} ({row.file}:{row.lineno}) "
                f"| {row.note} |",
                file=sys.stderr,
            )


# ---------------------------------------------------------------------------
# Pre-commit gate (--pre-commit flag)
# ---------------------------------------------------------------------------

def _relevant_files_staged() -> bool:
    """Return True if any SCANNED_FILES entry or anchors.toml is staged.

    bug-log #131 — this list used to name determinism.rs alone, so an edit to any of the
    other four anchor-literal files skipped the pre-commit gate entirely. It is derived
    from SCANNED_FILES now, so the two cannot drift apart.
    """
    pathspecs = [str(p.relative_to(REPO_ROOT)) for p in SCANNED_FILES]
    pathspecs.append(str(ANCHORS_TOML.relative_to(REPO_ROOT)))
    try:
        result = subprocess.run(
            [
                "git", "diff", "--cached", "--name-only",
                "--",
                *pathspecs,
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
    - **P5** a const NAME it did not know. Until 2026-09-28 the regex demanded the name be
      exactly `ANCHOR` / `ANCHOR_PREFIX`, so `BTC_ANCHOR_SHA`, `ETH_ANCHOR_SHA` and
      `ANCHORED_BODY_SHA` were invisible — six live gates in four files (bug-log #131).
    - **P6** the hex VALUE filter that keeps a non-digest const (the UI's `"Anchor: "`
      label) out. The value shape is the filter; the name never was.
    - **P7** the explicit `// anchor-scenario:` form, and that it too does not bleed.
    - **P8** the file list itself: non-empty, all present, and a vanished entry raises
      rather than silently contributing zero sites.

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
        s1, u1 = parse_anchor_literals(p1)
        probe(
            "P1 three runner spellings resolve",
            len(s1) == 3 and not u1,
            f"expected 3 resolved / 0 unresolved, got {len(s1)} / {len(u1)}",
        )

        # P2 — a const with no scenario must be UNRESOLVED, not dropped.
        p2 = tmp / "p2.rs"
        p2.write_text('fn d() {\n    const ANCHOR: &str = "dddddddd";\n    something_else();\n}\n', encoding="utf-8")
        s2, u2 = parse_anchor_literals(p2)
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
        s3, _ = parse_anchor_literals(p3)
        by_fn = {s.fn_name: s.declared_ns for s in s3}
        probe(
            "P3 declaration does not bleed between sites",
            by_fn.get("e") is None and by_fn.get("f") == "noop-baseline",
            f"expected e=None, f='noop-baseline'; got {by_fn}",
        )

        # P4 — the floor is a real assertion, not decoration.
        probe(
            "P4 floor is at least the sites we ship",
            MIN_EXPECTED_SITES >= 35,
            f"MIN_EXPECTED_SITES={MIN_EXPECTED_SITES} is below the 35 sites shipped 2026-09-28 "
            "(29 in determinism.rs + 6 in the four files the pre-#131 tool never opened)",
        )

        # P5 — the broadened const NAME. `BTC_ANCHOR_SHA`, `ETH_ANCHOR_SHA` and
        # `ANCHORED_BODY_SHA` were invisible to the exact-name regex; that is six live
        # gates in four files, and the tool said "29 of 29" without qualification.
        p5 = tmp / "p5.rs"
        p5.write_text(
            '// anchor-scenario: alpha-one-two\nconst BTC_ANCHOR_SHA: &str = "aaaaaaaa";\n'
            '// anchor-scenario: beta-one-two\nconst ANCHORED_BODY_SHA: &str = "bbbbbbbb";\n',
            encoding="utf-8",
        )
        s5, u5 = parse_anchor_literals(p5)
        probe(
            "P5 non-ANCHOR-prefixed const names are seen",
            len(s5) == 2 and not u5
            and {s.const_name for s in s5} == {"BTC_ANCHOR_SHA", "ANCHORED_BODY_SHA"},
            f"expected 2 resolved named BTC_ANCHOR_SHA/ANCHORED_BODY_SHA, got "
            f"{[(s.const_name, s.scenario) for s in s5]} / {len(u5)} unresolved",
        )

        # P6 — the hex VALUE filter, which is what keeps the UI label out. The NAME
        # `STRATEGY_REGISTRY_LAST_ANCHOR_PREFIX` matches; `"Anchor: "` is not a digest.
        # If this probe stops firing, the tool has started linting UI strings.
        p6 = tmp / "p6.rs"
        p6.write_text(
            'pub const STRATEGY_REGISTRY_LAST_ANCHOR_PREFIX: &str = "Anchor: ";\n'
            '// anchor-scenario: gamma-one-two\nconst ANCHOR: &str = "cccccccc";\n',
            encoding="utf-8",
        )
        s6, u6 = parse_anchor_literals(p6)
        probe(
            "P6 a non-hex value is not an anchor literal",
            len(s6) == 1 and not u6 and s6[0].scenario == "gamma-one-two",
            f"expected only the hex site, got {[(s.const_name, s.literal) for s in s6]}",
        )

        # P7 — an EXPLICIT `// anchor-scenario:` resolves a site whose scenario is nowhere
        # near it, and does NOT bleed to the next site. Same window, same reason as P3.
        p7 = tmp / "p7.rs"
        p7.write_text(
            '// anchor-scenario: delta-one-two\n// anchor-ns: lab-yahoo-realdata-v0.1.1\n'
            'const BTC_ANCHOR_SHA: &str = "dddddddd";\n'
            'const ETH_ANCHOR_SHA: &str = "eeeeeeee";\n'
            + '//\n' * 8
            + 'fn later() {\n    body("zeta-one-two");\n}\n',
            encoding="utf-8",
        )
        s7, u7 = parse_anchor_literals(p7)
        by_const = {s.const_name: (s.scenario, s.declared_ns) for s in s7}
        probe(
            "P7 explicit scenario resolves its own site only",
            by_const.get("BTC_ANCHOR_SHA") == ("delta-one-two", "lab-yahoo-realdata-v0.1.1")
            and "ETH_ANCHOR_SHA" not in by_const
            and [u.const_name for u in u7] == ["ETH_ANCHOR_SHA"],
            f"expected BTC declared + ETH unresolved; got {by_const} / "
            f"{[u.const_name for u in u7]}",
        )

        # P8 — SCANNED_FILES is not vacuous, and a vanished file is an ERROR not a zero.
        missing = [p for p in SCANNED_FILES if not p.is_file()]
        probe(
            "P8 every scanned file exists",
            len(SCANNED_FILES) >= 5 and not missing,
            f"{len(SCANNED_FILES)} file(s) listed; missing: {[str(m) for m in missing]}",
        )
        try:
            scan_files((tmp / "does-not-exist.rs",))
            vanished_raised = False
        except FileNotFoundError:
            vanished_raised = True
        probe(
            "P8b a vanished scanned file fails rather than counting zero",
            vanished_raised,
            "scan_files() returned normally for a nonexistent path",
        )

    if failures:
        print(
            f"\ncheck_determinism_anchors --self-test: FAIL — {len(failures)} probe(s) did not "
            "fire. The linter cannot be trusted to fail, which is bug-log #115.",
            file=sys.stderr,
        )
        return 1
    print("check_determinism_anchors --self-test: OK — 9 probes fired")
    return 0


def main() -> int:  # noqa: C901
    parser = argparse.ArgumentParser(
        description=(
            "Static drift-linter: asserts every in-test anchor-SHA literal in SCANNED_FILES "
            "== the anchors.toml row it targets."
        )
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
        help="No-op unless a SCANNED_FILES entry or anchors.toml is staged.",
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
        sites, unresolved, per_file = scan_files(SCANNED_FILES)
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: could not scan the anchor-literal files: {exc}", file=sys.stderr)
        return 2

    canonical = canonical_sha_map(anchors)

    # bug-log #115 — a total parse failure used to WARN and exit 0. A linter that finds
    # nothing and reports success is the failure mode it exists to prevent.
    if not sites:
        print(
            "FAIL: no anchor-SHA const site resolved in any of the "
            f"{len(SCANNED_FILES)} scanned files. That is a parser regression, not an empty "
            "tree — see bug-log #115.",
            file=sys.stderr,
        )
        return 1

    # bug-log #131 — a file that contributes ZERO sites is reported here, not buried in the
    # total. Every file in SCANNED_FILES was put there because it HAS literals, so a zero is
    # either a deleted gate or a parser that stopped seeing it.
    empty = [name for name, n in per_file.items() if n == 0]
    if empty:
        print(
            "FAIL: "
            + ", ".join(empty)
            + " is in SCANNED_FILES but yielded 0 anchor literals. Either the gate was "
            "removed (drop the file and lower the floor in the same commit) or the parser "
            "stopped seeing it (bug-log #131).",
            file=sys.stderr,
        )
        return 1

    # Non-vacuity floor. The count is printed either way, so a silent shrink is visible.
    if len(sites) + len(unresolved) < MIN_EXPECTED_SITES:
        print(
            f"FAIL: found {len(sites) + len(unresolved)} anchor literal site(s), floor is "
            f"{MIN_EXPECTED_SITES}. Either sites were deleted (update the floor in the same "
            "commit) or the parser stopped seeing them (bug-log #115). Per file: "
            + ", ".join(f"{k}={v}" for k, v in per_file.items()),
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
                f"  {u.file}:{u.lineno} fn {u.fn_name} — {u.const_name} = "
                f"{u.literal[:16]}… (no `// anchor-scenario:` within 3 lines above, and no "
                "scenario-shaped string within 7 lines below)",
                file=sys.stderr,
            )
        print(
            "\nEither the site declares `// anchor-scenario: <name>` directly above the "
            "const, or names its scenario within 7 lines below it, or this tool is extended "
            "deliberately — never by dropping the site.",
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
                    f"  {s.file}:{s.lineno} fn {s.fn_name} — anchor-ns: {s.declared_ns}",
                    file=sys.stderr,
                )
            return 1
        # bug-log #131 — sites now come from five files, and `apply_write` rewrites BY LINE
        # NUMBER. Handing it the whole list with one path would edit one file at another
        # file's line numbers. Group by file; a path with no sites is not opened.
        n = 0
        for path in SCANNED_FILES:
            mine = [s for s in sites if s.file == path.name]
            if mine:
                n += apply_write(path, mine, canonical)
        if n:
            print(f"check_determinism_anchors: rewrote {n} stale literal(s)")
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
    # bug-log #131 — split the declared-namespace sites out instead of folding them into
    # "canonical". 13 of the 29 determinism.rs sites target a NON-canonical row; calling
    # them canonical in the summary is the same species of unstated assumption as scanning
    # one file and saying "29 of 29".
    n_declared = sum(1 for s in sites if not s.cfg_gated and s.declared_ns is not None)
    n_canonical = n_ok - n_synth - n_declared
    # The per-file breakdown. A file dropping out of scope is then visible in the output
    # rather than only in the total — the finding this whole change is about.
    per_file_str = " · ".join(f"{name} {n}" for name, n in per_file.items())
    print(
        f"check_determinism_anchors: OK — {n_ok} of {n_total} resolved literal(s) match "
        f"({n_canonical} canonical v5-realdata-medium-2026-05, {n_declared} declared-ns, "
        f"{n_synth} synthetic; {n_skip} skipped: cfg-gated; 0 unresolved). "
        f"Sites seen: {n_total} across {len(per_file)} file(s) (floor {MIN_EXPECTED_SITES}) "
        f"— {per_file_str}."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
