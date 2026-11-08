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
# bug-log #183 durable fix — the reports crate holds the OTHER pin locations.
REPORTS_TESTS_DIR = REPO_ROOT / "crates" / "reports" / "tests"

# bug-log #131 — every file that carries an in-test anchor-SHA literal, not just the
# first one anybody wrote. Derived mechanically on 2026-09-28 with
#   grep -rln 'const [A-Z_]*ANCHOR[A-Z_]*\s*:\s*&str' crates/ --include='*.rs'
# and re-derivable the same way; `crates/ui/src/strings.rs` also matches that NAME grep
# and is correctly absent here because its value is the UI label "Anchor: ", not a digest
# (the hex VALUE filter in `const_sha_re` is what rules it out — see below).
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
    # bug-log #183 — the four files above were 26 of the repo's 66 mirror literals. These
    # three hold 38 more, and until this change nothing synced them: #181 (a stale pin in
    # strategy_anchors_unchanged.rs taking CI red on three legs) and #182 both came out of
    # this gap. The first of them carries its SHAs in `&[(&str, &str)]` TABLES rather than
    # scalar consts, which is why `parse_anchor_tables` exists.
    REPORTS_TESTS_DIR / "strategy_anchors_unchanged.rs",
    REPORTS_TESTS_DIR / "report_scenarios.rs",
    REPORTS_TESTS_DIR / "t1003_orchestrator_smoke.rs",
)

# bug-log #183 durable fix — mirror literals this tool deliberately does NOT sync, with the
# reason, so the next sweep does not "finish the job" by adding them.
#
# `sha_mirror_census.py` is right to COUNT these: they are 64-hex copies of a current row,
# and if the row's value changed they would stop matching. But counting a copy and syncing a
# copy are different acts. A citation names ONE historical body on purpose; the sync tool
# writes whatever the row says now. Pointing the second at the first deletes the citation.
EXCLUDED_CITATION_SITES: tuple[tuple[str, int, str], ...] = (
    (
        "crates/forecast/src/bin/forecast_distribution.rs",
        2,
        "two `predecessor_sha` fields, documented in the struct as '(for citation)'. They "
        "name the PRE-recalibration body that a recalibrated report supersedes. They equal "
        "a current `forecast-distribution-bs{1,2}-realdata` row today only because that "
        "scenario has not been re-emitted since; after the next ADR-0038 § D6.b "
        "re-emission the CORRECT value is the old one, which is exactly what a sync would "
        "overwrite. § D6.b requires keeping the superseded SHA; this tool must refuse it.",
    ),
)

# bug-log #115 non-vacuity floor: the number of anchor-literal sites this tool must at
# least SEE. Bumped deliberately when sites are added; lowering it is a reviewable act.
# It exists because the tool once reported "OK — 14 literal(s)" while 25 existed.
# 2026-09-27: 25 -> 29 with the four R-REAL real-data-row gates (bug-log #125).
# 2026-09-28: 29 -> 35 with the SCOPE fix (bug-log #131). The +6 are not new gates — they
#   were in the tree all along, in four files the tool never opened: 2 in
#   multi_pair_determinism.rs, 2 in run_yahoo_sma_ticker_flag.rs, 1 in
#   theta_surface_reproduction.rs, 1 in reproducibility_sample_figure.rs.
# 2026-11-08: 35 -> 75 with the #183 durable fix. The +40 are not new gates either —
#   37 table rows in strategy_anchors_unchanged.rs, 2 in report_scenarios.rs, 1 in
#   t1003_orchestrator_smoke.rs. 37 rows mirror 35 DISTINCT SHAs (two scenario pairs
#   share a body), which is why the census floor for that file reads 35 and this one
#   counts 37: this tool checks every pin, the census counts distinct values.
MIN_EXPECTED_SITES = 75

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
# RE-LOCKED 2026-11-08 (bug-log #151 / #181). The synthetic-path body gained the
# buy-and-hold control rows, so all three SHAs moved. Cause established, not assumed:
#   * `sma_composed_run.rs` — the run path for these three — is UNTOUCHED in the diff; the
#     only code reaching their bodies is `report/sma.rs` + `report/mod.rs::risk_line`.
#   * the REAL-DATA twin of these same three scenarios was classified D6C (figures
#     byte-identical) by scripts/reemission_classify.py, so the writer's change moves no
#     figure — measured on the twin condition, not argued.
#   * the new values were reproduced TWICE by independent paths: the tempdir-based t622/t717
#     harness, and `--force-synthetic-bars` run from the repo. Both agree to the byte.
# Superseded: macd 4d8192af… / rsi 4a744788… / bbands 5037accb…
SYNTHETIC_DETERMINISM_SHAS: dict[str, str] = {
    "btc-2023-1m-macd-trend":         "9c07d2c3fa3dbcb7645307790d03e710e3acc90e3435e7c6b43581492bb93077",
    "btc-2023-1m-rsi-reversion":      "14572b60986e9f94feb4ee5db59233d877655e75884ca9d15d3df876736a81d2",
    "btc-2023-1m-bbands-mean-revert": "fa390116c31ac9c0c5d87b6b930688bf41c45cdb0ede55eb01aef7badcc1b97c",
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
    # bug-log #183: which SYNTAX this site was read from, so the writer edits the right
    # shape. "const" is `const NAME: &str = "<sha>"`; "table-row" is one `("<scenario>",
    # "<sha>")` row of a `&[(&str, &str)]` table, where the SHA usually sits alone on its
    # line and there is no const name on it to match. Resolution does not branch on this —
    # only the text edit does.
    shape: str = "const"


# ---------------------------------------------------------------------------
# anchors.toml parser
# ---------------------------------------------------------------------------

# bug-log #115 — what a SCENARIO NAME looks like. Hoisted to module scope by the #183
# durable fix because the table parser needs the same answer, and two regexes that must
# agree on "is this a scenario" is the #180 shape all over again.
SCENARIO_SHAPE_RE = re.compile(r'"([a-z][a-z0-9]*(?:-[a-z0-9]+){2,})"')

# bug-log #115/#123/#131 — how a site DECLARES what it pins. Module scope for the same
# reason as SCENARIO_SHAPE_RE: the table parser reads the identical declarations, and a
# second copy of either pattern could drift from this one without any test noticing.
ANCHOR_NS_RE = re.compile(r'//\s*anchor-ns:\s*(\S+)')
ANCHOR_SCENARIO_RE = re.compile(r'//\s*anchor-scenario:\s*(\S+)')


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
    # bug-log #183 — the NAME filter is gone from the regex. It used to require ANCHOR in
    # the const name, which meant `report_scenarios.rs::EXPECTED_SHA_7D` and
    # `t1003_orchestrator_smoke.rs::EXPECTED_SHA_7D` — two copies of one anchored SHA,
    # and the pair that bug-log #181 found stale — could not be seen even once their files
    # were scanned. Renaming them was rejected: `t1003`'s own comment cites the other by
    # name, and `strategy_anchors_unchanged.rs` records why names in this area are not
    # churned (anchored reports and ADR-0043 cite them).
    #
    # So the regex matches ANY const holding an 8–64 hex string, and the decision "is this
    # an anchor mirror?" moves below, where the declarations are already parsed: a const is
    # a site if its name contains ANCHOR (the old rule, unchanged) OR it carries an
    # explicit `// anchor-scenario:`. That keeps an unrelated digest — a data revision, a
    # model checkpoint — out by default, while letting a site opt IN by saying what it
    # pins. It is the same principle as the resolver's: the site knows, so the site says.
    #
    # This regex is ALSO the backwards-window boundary below, deliberately the same object:
    # a window that stopped at a narrower set of consts than it matched would let one
    # site's declaration bleed into the next, which is the bleed probe P7 exists for.
    const_sha_re = re.compile(
        r'const\s+([A-Za-z0-9_]+)\s*:\s*&str\s*=\s*"([0-9a-fA-F]{8,64})"'
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
    scenario_call_re = SCENARIO_SHAPE_RE
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
    anchor_ns_re = ANCHOR_NS_RE
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
    anchor_scenario_re = ANCHOR_SCENARIO_RE
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
        const_m = const_sha_re.search(line)
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
                if const_sha_re.search(lines[j]):
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

            # bug-log #183 — the NAME filter, moved here so it can consult the
            # declaration. An ANCHOR-named const is a site exactly as before. A
            # differently-named one is a site only if it SAYS so with
            # `// anchor-scenario:`; otherwise it is some other digest and is passed over
            # in silence, not reported unresolved, because it was never ours to check.
            if "ANCHOR" not in const_name and declared_scenario is None:
                pass
            elif scenario is None:
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


def parse_anchor_tables(path: Path) -> tuple[list[InTestSite], list[UnresolvedSite]]:
    """Extract every row of every `&[(&str, &str)]` scenario→SHA table in `path`.

    bug-log #183. `crates/reports/tests/strategy_anchors_unchanged.rs` pins 37 anchored
    SHAs — more than every scalar const in this tool's original scope put together — in
    three tables of `("<scenario>", "<sha>")` rows. `parse_anchor_literals` cannot see one:
    there is no `const NAME: &str` on the line, the SHA usually sits alone, and the
    scenario it belongs to is the quoted string above it. So the shape gets its own reader.

    Two readers, one resolver. That is the line this change is careful about: #180 was two
    copies of the RESOLUTION ORDER, and the fix was to have exactly one. This function
    resolves nothing and compares nothing — it reads syntax and emits `InTestSite`, which
    `resolve_expected` and `literal_matches` then treat identically to a const site. The
    only thing that branches on `shape` afterwards is the text edit in `apply_write`.

    The table's namespace is declared ONCE, above the `const`, with the same
    `// anchor-ns:` syntax a scalar site uses — not inferred from the const's name and not
    mapped in a table in this script, which would be a second place to keep in step. The
    three tables in that file mirror three namespaces (`noop-baseline`,
    `v5-realdata-medium-2026-05`, `v5-sqrt-impact-2026-05`) and all 37 rows resolve through
    the ordinary resolver once the declaration is there.

    Rows are read token-by-token in order rather than by a row-shaped regex, because
    rustfmt wraps a long row over four lines and keeps a trailing comma inside the paren —
    `("scen",\n "sha",\n)`. A regex written against the single-line form matches none of
    them, silently, which is how the first draft of this function reported zero rows for
    all three tables while claiming to have parsed them. A SHA with no scenario above it is
    reported unresolved, never dropped.
    """
    lines = path.read_text(encoding="utf-8").splitlines()
    sites: list[InTestSite] = []
    unresolved: list[UnresolvedSite] = []

    table_decl_re = re.compile(
        r'const\s+([A-Za-z0-9_]+)\s*:\s*&\[\(&str,\s*&str\)\]\s*='
    )
    quoted_re = re.compile(r'"([^"]*)"')
    hex_re = re.compile(r'^[0-9a-fA-F]{8,64}$')

    i = 0
    while i < len(lines):
        decl = table_decl_re.search(lines[i])
        if decl is None:
            i += 1
            continue
        const_name = decl.group(1)

        # The namespace declaration belongs to THIS table: the same narrow backwards
        # window a scalar site uses, stopping at a `fn` or the previous table.
        declared_ns: str | None = None
        for j in range(i - 1, max(-1, i - 4), -1):
            if "fn " in lines[j] or table_decl_re.search(lines[j]):
                break
            ns_m = ANCHOR_NS_RE.search(lines[j])
            if ns_m:
                declared_ns = ns_m.group(1)
                break

        # Find the end of the table literal. `];` on its own line closes it.
        end = None
        for j in range(i + 1, len(lines)):
            if lines[j].strip() == "];":
                end = j
                break
        if end is None:
            raise RuntimeError(
                f"{path.name}:{i + 1}: table {const_name} has no closing '];' — refusing "
                "to guess where it ends (bug-log #183)"
            )

        pending_scenario: str | None = None
        for j in range(i + 1, end):
            for tok_m in quoted_re.finditer(lines[j]):
                tok = tok_m.group(1)
                if hex_re.match(tok):
                    if pending_scenario is None:
                        unresolved.append(UnresolvedSite(
                            file=path.name,
                            lineno=j + 1,
                            fn_name=f"(table {const_name})",
                            const_name=const_name,
                            literal=tok,
                        ))
                        continue
                    sites.append(InTestSite(
                        file=path.name,
                        lineno=j + 1,
                        fn_name=f"(table {const_name})",
                        scenario=pending_scenario,
                        const_name=const_name,
                        literal=tok,
                        cfg_gated=False,
                        declared_ns=declared_ns,
                        shape="table-row",
                    ))
                    pending_scenario = None
                elif SCENARIO_SHAPE_RE.fullmatch(f'"{tok}"'):
                    pending_scenario = tok

        i = end + 1

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
        # bug-log #183 — and the table shape, from the same file, counted into the same
        # per-file row. A file whose pins live in tables read as "0 literals" before this.
        ts, tu = parse_anchor_tables(path)
        s = s + ts
        u = u + tu
        sites.extend(s)
        unresolved.extend(u)
        per_file[path.name] = len(s) + len(u)
    return sites, unresolved, per_file


# ---------------------------------------------------------------------------
# Expected-SHA resolution — the ONE order (bug-log #180, durable fix)
# ---------------------------------------------------------------------------

def resolve_expected(
    site: InTestSite,
    anchors: list[AnchorEntry] | None,
    canonical: dict[str, str],
) -> tuple[str | None, str | None]:
    """The SHA this site asserts, and which map answered. The only copy of that order.

    bug-log #180 — this order used to exist TWICE, once in `detect_drift` and once in
    `apply_write`, the second under a comment reading "same order as detect_drift". That
    comment was the entire guarantee of agreement, and it was false in two independent
    ways: the writer read `SYNTHETIC_DETERMINISM_SHAS` BEFORE looking at the site's own
    declaration, and resolved through `canonical[scenario]` instead of `sha_for`. Both
    cost something real. The first made `--write` overwrite the three correct canonical
    pins of `realdata_row_btc_2023_1m_{macd_trend,rsi_reversion,bbands_mean_revert}` with
    the synthetic bodies' SHAs; the second made `--write` refuse every re-emission that
    mattered, because `canonical[...]` cannot see a non-canonical row at all.

    Both were repaired on 2026-11-08 by copying the detector's lines into the writer —
    which left the repo with two copies that agreed, which is the state that produced the
    bug. `#180` filed the durable fix in its own words: "one resolver called twice". This
    is it. Agreement between two functions is a claim nothing enforces; one function is
    the enforcement.

    The order, and why each step is where it is:

      1. The site declares `// anchor-ns: <substring>` → its row is
         `sha_for(anchors, scenario, declared_ns)` and NOTHING else. A declared namespace
         SUPPRESSES the synthetic map. Two sites can pin the same scenario under different
         conditions — `btc-2023-1m-*` has a synthetic-path pin and a real-data row pin —
         and a scenario-keyed override would compare the second against the first's
         expectation and call a correct gate stale (bug-log #123).
         **The site knows which row it asserts; the site wins.**
      2. The site declares nothing → `SYNTHETIC_DETERMINISM_SHAS[scenario]`, when that
         scenario has one. The determinism tests run the v0 synthetic fallback (525600
         bars, CWD=tempdir); the v5 rows were emitted from the real-data path
         (ADR-0045 § D6.3 / § D7.1b), so the scenario name alone does not say which.
      3. Otherwise the canonical `v5-realdata-medium-2026-05` row — the implicit meaning
         of every site written before bug-log #115.
      4. Nothing resolved → `(None, None)`. A pin whose row cannot be identified is a
         finding, never a skip: `detect_drift` makes it a HARD ERROR, and `--write`
         refuses the site rather than picking a row on its behalf (bug-log #77).

    Returns `(expected_sha, source_label)`, the label one of "declared-ns", "synthetic",
    "canonical", or None exactly when no SHA resolved. The summary line counts these
    labels, so the printed breakdown reports what resolution DID rather than what a
    scenario name suggests — which is the same reason the labels exist at all.

    `anchors=None` is the degraded legacy call shape: `canonical` is then the only map
    available, so a declared namespace cannot be honoured; the label still says
    "declared-ns", because that is the site's claim. This tool's own entrypoint always
    passes `anchors`.
    """
    if site.declared_ns is not None:
        sha = (
            sha_for(anchors, site.scenario, site.declared_ns)
            if anchors is not None
            else canonical.get(site.scenario)
        )
        return (sha, "declared-ns") if sha is not None else (None, None)

    synth = SYNTHETIC_DETERMINISM_SHAS.get(site.scenario)
    if synth is not None:
        return synth, "synthetic"

    sha = (
        sha_for(anchors, site.scenario, None)
        if anchors is not None
        else canonical.get(site.scenario)
    )
    return (sha, "canonical") if sha is not None else (None, None)


def literal_matches(site: InTestSite, expected: str) -> bool:
    """Whether the site's literal satisfies `expected` — the ONE comparison.

    The second half of the bug-log #180 duplication, and it had drifted the same way: a
    `*_PREFIX` site resolving to a SYNTHETIC SHA was compared by FULL equality in
    `detect_drift` and by prefix in `apply_write`, so the two could disagree about one
    site. Whether a site compares a prefix is a property of the SITE (its const name),
    not of the map that answered — so prefix wins wherever it is declared, which is what
    `apply_write` already did. The repo ships zero `*_PREFIX` sites (EX-2 converted them
    all), which is why this divergence was invisible rather than why it was harmless.
    """
    if _is_prefix_site(site.const_name):
        return expected.startswith(site.literal)
    return site.literal == expected


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
    """Return (mismatches, skipped) from comparing sites against their resolved row.

    Which row a site means is NOT decided here — `resolve_expected` is the single copy of
    that order and `literal_matches` the single copy of the comparison (bug-log #180,
    which this function used to hold one of two copies of). This turns their answer into
    report rows and nothing else.

    skipped: cfg-gated sites only (R3).
    mismatches: sites whose in-test literal != the resolved SHA, OR whose row could not be
      resolved at all. The second is a HARD ERROR rather than a skip: it closes the blind
      spot the old "not in anchors.toml → skip" branch would reopen for any newly added
      but unmapped test fn.
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

        expected, source = resolve_expected(site, anchors, canonical)

        if expected is None:
            # --- HARD ERROR — closes the blind spot ---
            # A non-cfg-gated site whose row cannot be identified is an unanchored
            # constant: it cannot be validated. Always a configuration error, so fail
            # loudly rather than skip — skipping is what the pre-#115 tool did for 11 of
            # 25 sites while printing "0 skipped".
            mismatches.append(DriftRow(
                file=site.file,
                lineno=site.lineno,
                fn_name=site.fn_name,
                scenario=site.scenario,
                in_test=site.literal[:16] + "\u2026",
                canonical="(HARD ERROR: no row for the declared namespace)",
                match=False,
                note=(
                    f"declared ns={site.declared_ns!r} matches no single anchors.toml row "
                    "for this scenario — fix the declaration or add the row"
                    if site.declared_ns is not None
                    else "add the scenario to the synthetic map or to anchors.toml"
                ),
            ))
            continue

        match = literal_matches(site, expected)
        if source == "synthetic":
            shown = expected[:16] + "\u2026 (synthetic)"
            note = "SYNTHETIC map" if match else "synthetic SHA mismatch"
        else:
            shown = expected[:16] + "\u2026"
            note = "" if site.declared_ns is None else f"ns={site.declared_ns}"

        row = DriftRow(
            file=site.file,
            lineno=site.lineno,
            fn_name=site.fn_name,
            scenario=site.scenario,
            in_test=site.literal[:16] + "\u2026",
            canonical=shown,
            match=match,
            note=note,
        )
        if not match:
            mismatches.append(row)

    return mismatches, skipped


# ---------------------------------------------------------------------------
# --write mode: rewrite stale literals in place
# ---------------------------------------------------------------------------

def apply_write(
    path: Path,
    sites: list[InTestSite],
    canonical: dict[str, str],
    anchors: list[AnchorEntry] | None = None,
) -> int:
    """Rewrite stale ANCHOR literals in `path` to the SHA `resolve_expected` names.

    Resolution is NOT re-derived here, and that is the whole point. bug-log #180: this
    function carried its own copy of the order under a comment claiming it was "the same
    order as detect_drift", and the two disagreed twice over — once by taking the
    synthetic SHA for a site that had declared its namespace (three correct canonical pins
    overwritten), once by resolving through `canonical[scenario]`, which cannot see a
    non-canonical row at all (every real re-emission refused). Copying the detector's
    lines in here on 2026-11-08 fixed the symptom and left the cause: two copies that
    agree. `--write` must write the value the CHECK demands or it is not a fix, so both
    now call `resolve_expected` / `literal_matches` and read no map themselves.

    A site whose row cannot be resolved is skipped with a warning: `--write` cannot
    fabricate a correct value, and `main()` refuses such a site up front.

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

        # The ONE resolver — the order and its reasoning live in its docstring (#180).
        target, _source = resolve_expected(site, anchors, canonical)
        if target is None:
            # No row identifiable, so there is no correct value to write. Skip with a
            # warning; the hard failure for this is detect_drift's, at check time, and
            # main() refuses a declared-but-unresolvable site before reaching here.
            where = (
                f" under declared ns '{site.declared_ns}'"
                if site.declared_ns is not None
                else ""
            )
            print(
                f"WARN: --write skipped {site.fn_name} ({path.name}:{site.lineno}): no "
                f"row resolves for scenario '{site.scenario}'{where} — fix the "
                "declaration or add the row manually.",
                file=sys.stderr,
            )
            continue

        # Already current? The same comparison the check uses, for the same reason.
        if literal_matches(site, target):
            continue

        lineno_0 = site.lineno - 1  # 0-based index
        old_line = lines_out[lineno_0]

        # bug-log #131: rewrite the site's OWN const name. The old code hardcoded
        # `ANCHOR` / `ANCHOR_PREFIX`, so a `BTC_ANCHOR_SHA` site would have had its
        # substitution silently no-op while `rewrites` still counted it — a --write that
        # reports having fixed a literal it did not touch.
        base_name = site.const_name.removesuffix("_PREFIX")
        if site.shape == "table-row":
            # bug-log #183 — a table row has no const on its line to match, so the edit is
            # keyed on the literal itself. It must occur EXACTLY once on the line: two
            # copies and there is no way to tell which row this site is, so refuse rather
            # than rewrite the wrong one. The const path below is untouched, which is what
            # keeps the five original files byte-identical through this change.
            needle = f'"{site.literal}"'
            if old_line.count(needle) != 1:
                raise RuntimeError(
                    f"--write could not rewrite {site.file}:{site.lineno} (table "
                    f"{site.const_name}): the literal appears {old_line.count(needle)} "
                    "times on its line, so the row is ambiguous — refusing"
                )
            lines_out[lineno_0] = old_line.replace(needle, f'"{target}"')
        elif _is_prefix_site(site.const_name):
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
    - **P9** the precedence rule a declared namespace has over the synthetic map. Two
      fixtures differing in exactly that dimension and expecting DIFFERENT values —
      one value pinned by a single fixture is a value nothing holds (bug-log #161).
    - **P10** that the CHECK path and the `--write` path resolve one site to the same
      value, on the configuration that broke: a site declaring `// anchor-ns:` for a
      scenario the synthetic map also knows. With a negative control, because a probe
      where both paths agree on "nothing to do" would also pass if both did nothing.
    - **P11** the structural property P9/P10 rest on: `resolve_expected` is the only
      reader of the two maps. No value-level probe can see a second copy appear; this
      one fails the moment one does.

    A linter without this mode is exactly the thing bug-log `#115` is about: it reports a
    number nobody has checked it can fail to produce.
    """
    import tempfile

    failures: list[str] = []
    n_probes = 0

    def probe(name: str, ok: bool, detail: str) -> None:
        nonlocal n_probes
        n_probes += 1
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

        # ---------------------------------------------------------------- bug-log #180
        # The fixture below IS the configuration that broke: a site declaring
        # `// anchor-ns:` for a scenario that ALSO has a synthetic SHA. Three such sites
        # ship (`realdata_row_btc_2023_1m_*`). Detection suppressed the synthetic map for
        # them; `--write` did not. Nothing could see the disagreement, because `--write`
        # was unreachable behind a refusal — so the fixture is the gate, not the refusal.
        syn_scenario = next(iter(SYNTHETIC_DETERMINISM_SHAS))
        syn_sha = SYNTHETIC_DETERMINISM_SHAS[syn_scenario]
        # DERIVED, not measured off the subject: any value that is not the synthetic one.
        declared_sha = "0123456789abcdef" * 4
        assert declared_sha != syn_sha
        fx_anchors = [AnchorEntry(syn_scenario, CANONICAL_VERSION_SUFFIX, declared_sha)]
        fx_canonical = canonical_sha_map(fx_anchors)

        # P9 — the precedence rule itself, two-sided. The pair differs in ONE dimension
        # (declared_ns present or absent) and expects DIFFERENT values; a single fixture
        # would pass against either order, which is how an order gets "pinned" by nothing.
        site_declared = InTestSite(
            file="p9.rs", lineno=3, fn_name="r", scenario=syn_scenario,
            const_name="ANCHOR", literal=declared_sha, cfg_gated=False,
            declared_ns=CANONICAL_VERSION_SUFFIX,
        )
        site_bare = site_declared._replace(declared_ns=None, literal=syn_sha)
        got_declared = resolve_expected(site_declared, fx_anchors, fx_canonical)
        got_bare = resolve_expected(site_bare, fx_anchors, fx_canonical)
        probe(
            "P9 a declared namespace suppresses the synthetic map",
            got_declared == (declared_sha, "declared-ns")
            and got_bare == (syn_sha, "synthetic"),
            f"expected declared→({declared_sha[:8]}…, 'declared-ns') and "
            f"bare→({syn_sha[:8]}…, 'synthetic'); got {got_declared} / {got_bare}",
        )

        # P10 — the two PATHS, not the one function: parse a real file, run detection over
        # it, then run the writer over it, and require them to agree. Pre-#180 `apply_write`
        # resolved `syn_sha` here and would have rewritten a CORRECT literal to it.
        def _fixture(name: str, literal: str) -> Path:
            p = tmp / name
            p.write_text(
                "fn r() {\n"
                f"    // anchor-ns: {CANONICAL_VERSION_SUFFIX}\n"
                f'    const ANCHOR: &str = "{literal}";\n'
                f'    scenario_body_hex("{syn_scenario}");\n'
                "}\n",
                encoding="utf-8",
            )
            return p

        # P10a — literal already correct for the DECLARED row: detection is silent and the
        # writer touches nothing. The old writer rewrote it to the synthetic SHA.
        p10a = _fixture("p10a.rs", declared_sha)
        s10a, _ = parse_anchor_literals(p10a)
        before = p10a.read_text(encoding="utf-8")
        mm_a, _ = detect_drift(s10a, fx_canonical, fx_anchors)
        wrote_a = apply_write(p10a, s10a, fx_canonical, fx_anchors)
        probe(
            "P10a check and write agree that a declared-ns pin is current",
            len(s10a) == 1 and not mm_a and wrote_a == 0
            and p10a.read_text(encoding="utf-8") == before,
            f"parsed {len(s10a)} site(s), {len(mm_a)} mismatch(es), {wrote_a} rewrite(s); "
            f"file {'unchanged' if p10a.read_text(encoding='utf-8') == before else 'CHANGED'}",
        )

        # P10b — the negative control. Without it, P10a would also pass if both paths did
        # nothing at all. A wrong literal must make detection fire AND make the writer
        # write the value detection named — the declared row, never the synthetic one.
        p10b = _fixture("p10b.rs", "f" * 64)
        s10b, _ = parse_anchor_literals(p10b)
        mm_b, _ = detect_drift(s10b, fx_canonical, fx_anchors)
        wrote_b = apply_write(p10b, s10b, fx_canonical, fx_anchors)
        after_b = p10b.read_text(encoding="utf-8")
        probe(
            "P10b a stale declared-ns pin is written to the row the CHECK named",
            len(mm_b) == 1 and wrote_b == 1
            and declared_sha in after_b and syn_sha not in after_b,
            f"{len(mm_b)} mismatch(es), {wrote_b} rewrite(s); declared row "
            f"{'present' if declared_sha in after_b else 'ABSENT'}, synthetic "
            f"{'PRESENT' if syn_sha in after_b else 'absent'}",
        )

        # P11 — the structural property P9/P10 rest on. Those two check what the resolver
        # RETURNS; neither can see a second copy of the order growing back inside a caller,
        # which is the defect itself. So: `resolve_expected` must be the only function that
        # names `SYNTHETIC_DETERMINISM_SHAS` or `sha_for`.
        #
        # Read off the AST, not the source text. My first version grepped the source and
        # flagged `main` for the words `sha_for(` sitting inside a COMMENT explaining this
        # very refactor — a gate that fires on prose is a gate that gets switched off. A
        # Name node is a read; a comment and a docstring are neither, and the AST does not
        # contain them at all.
        import ast

        maps = {"SYNTHETIC_DETERMINISM_SHAS", "sha_for"}
        module = ast.parse(Path(__file__).read_text(encoding="utf-8"))
        defs = {
            node.name: node
            for node in module.body
            if isinstance(node, ast.FunctionDef)
        }

        def names_read(fn_name: str) -> set[str]:
            node = defs.get(fn_name)
            if node is None:
                return {"<FUNCTION NOT FOUND: " + fn_name + ">"}
            return {
                n.id for n in ast.walk(node)
                if isinstance(n, ast.Name) and n.id in maps
            }

        offenders = {
            name: sorted(names_read(name))
            for name in ("detect_drift", "apply_write", "main")
            if names_read(name)
        }
        # Positive control: if the AST walk were broken, every set would be empty and the
        # offender check would pass while measuring nothing. The resolver MUST read both.
        resolver_reads = names_read("resolve_expected")
        probe(
            "P11 resolve_expected is the only reader of the two maps",
            not offenders and resolver_reads == maps,
            f"callers naming the maps directly: {offenders or 'none'}; "
            f"resolve_expected names {sorted(resolver_reads)} (must be both, else this "
            "probe is measuring nothing)",
        )

        # ── bug-log #183 — the TABLE shape and the declaration-gated const ──────────
        #
        # Every pair below differs in ONE dimension and expects DIFFERENT outcomes on the
        # two sides (bug-log #161: 368 green tests pinned nothing because every fixture
        # agreed). The values are DERIVED — built here, not measured off the subject.
        tbl_s1 = "probe-table-scenario-one"
        tbl_s2 = "probe-table-scenario-two"
        ns_a, ns_b = "probe-ns-alpha", "probe-ns-beta"
        sha_a, sha_b, sha_a2 = "a" * 64, "b" * 64, "c" * 64
        # tbl_s1 ALSO gets a canonical row, and P14 is the reason. Without one, a
        # regression that let an unresolvable declaration fall through to the canonical
        # map would have nothing to fall through TO in this fixture, so P14 would pass
        # while measuring nothing — the shape bug-log #161 is about. With it, the correct
        # behaviour (hard error) and the regression (quietly resolving to this value) are
        # two different outcomes, which is what makes the probe a probe.
        sha_canon = "e" * 64
        tbl_anchors = [
            AnchorEntry(tbl_s1, ns_a, sha_a),
            AnchorEntry(tbl_s1, ns_b, sha_b),
            AnchorEntry(tbl_s1, CANONICAL_VERSION_SUFFIX, sha_canon),
            AnchorEntry(tbl_s2, ns_a, sha_a2),
        ]
        tbl_canonical = canonical_sha_map(tbl_anchors)

        def _table_fixture(
            name: str, ns: str, rows: list[tuple[str, str]], gap: int = 0
        ) -> Path:
            """A rustfmt-shaped table, with `gap` comment lines between decl and const."""
            p = tmp / name
            filler = "// an explanation line, declaring nothing\n" * gap
            body = "".join(
                f'    (\n        "{sc}",\n        "{hx}",\n    ),\n' for sc, hx in rows
            )
            p.write_text(
                f"// anchor-ns: {ns}\n"
                + filler
                + "const PROBE_ANCHORS: &[(&str, &str)] = &[\n"
                + body
                + "];\n",
                encoding="utf-8",
            )
            return p

        # P12 — the declaration is READ, and selects between two rows of the same scenario.
        # If it were ignored both sides would land on the same value (or on None), so the
        # probe cannot pass while the namespace is being dropped.
        f12a = _table_fixture("p12a.rs", ns_a, [(tbl_s1, sha_a)])
        f12b = _table_fixture("p12b.rs", ns_b, [(tbl_s1, sha_b)])
        s12a, _ = parse_anchor_tables(f12a)
        s12b, _ = parse_anchor_tables(f12b)
        # Indexed defensively on purpose: a regression that makes the parser return NO
        # rows (a row regex written against the single-line form does exactly that) must
        # land as a NAMED probe failure, not an IndexError traceback that says only that
        # something went wrong somewhere.
        def _first_ns(sites: list[InTestSite]) -> str | None | object:
            return sites[0].declared_ns if sites else "<no site>"

        got12a = (
            resolve_expected(s12a[0], tbl_anchors, tbl_canonical)
            if s12a else (None, "<no site>")
        )
        got12b = (
            resolve_expected(s12b[0], tbl_anchors, tbl_canonical)
            if s12b else (None, "<no site>")
        )
        probe(
            "P12 a table's anchor-ns declaration selects its row (two-sided)",
            len(s12a) == len(s12b) == 1
            and got12a == (sha_a, "declared-ns")
            and got12b == (sha_b, "declared-ns")
            and sha_a != sha_b,
            f"ns={ns_a} → {got12a}; ns={ns_b} → {got12b} (must differ and match their rows)",
        )

        # P13 — the window is three lines WIDE ON PURPOSE, and this is the mistake that
        # cost a round trip while writing #183: the declaration was put at the TOP of a
        # five-line note, out of reach, and all three tables silently resolved as
        # undeclared. Adjacent is read; five lines up is not.
        f13 = _table_fixture("p13.rs", ns_a, [(tbl_s1, sha_a)], gap=4)
        s13, _ = parse_anchor_tables(f13)
        probe(
            "P13 a declaration five lines above the const is NOT read (two-sided)",
            len(s13) == 1
            and len(s12a) == 1
            and s13[0].declared_ns is None
            and s12a[0].declared_ns == ns_a,
            f"adjacent → {_first_ns(s12a)!r}; gap=4 → {_first_ns(s13)!r} "
            "(must be the ns, then None)",
        )

        # P14 — a declaration naming no row is a HARD ERROR, never a quiet fall back to
        # canonical. The other half of the same round trip: prose reading
        # "needs an // anchor-ns: line" inside the window IS a declaration, of the word
        # that follows it. It must fail loudly, because that is what made it findable.
        f14 = _table_fixture("p14.rs", "probe-ns-matching-no-row", [(tbl_s1, sha_a)])
        s14, _ = parse_anchor_tables(f14)
        got14 = (
            resolve_expected(s14[0], tbl_anchors, tbl_canonical)
            if s14 else (None, "<no site>")
        )
        mm14, _ = detect_drift(s14, tbl_canonical, tbl_anchors)
        probe(
            "P14 a declaration that matches no row hard-errors, not falls back",
            got14 == (None, None) and len(mm14) == 1,
            f"bogus ns resolved to {got14} with {len(mm14)} mismatch row(s) — must be "
            f"(None, None) and 1; resolving to ({sha_canon[:8]}…, 'canonical') means a "
            "declaration that names no row silently became the canonical default",
        )

        # P15 — both row SHAPES, because a regex written against the single-line form
        # matches none of the wrapped ones and reports zero rows while claiming success;
        # that is exactly what the first draft of parse_anchor_tables did. And an orphan
        # SHA with no scenario above it is a finding, not a silent drop.
        p15 = tmp / "p15.rs"
        p15.write_text(
            f"// anchor-ns: {ns_a}\n"
            "const PROBE_ANCHORS: &[(&str, &str)] = &[\n"
            f'    (\n        "{tbl_s1}",\n        "{sha_a}",\n    ),\n'
            f'    ("{tbl_s2}", "{sha_a2}"),\n'
            "];\n",
            encoding="utf-8",
        )
        s15, u15 = parse_anchor_tables(p15)
        p15b = tmp / "p15b.rs"
        p15b.write_text(
            f"// anchor-ns: {ns_a}\n"
            "const PROBE_ANCHORS: &[(&str, &str)] = &[\n"
            f'    "{sha_a}",\n'
            "];\n",
            encoding="utf-8",
        )
        s15b, u15b = parse_anchor_tables(p15b)
        probe(
            "P15 wrapped AND single-line rows parse; an orphan SHA is unresolved",
            {(x.scenario, x.literal) for x in s15}
            == {(tbl_s1, sha_a), (tbl_s2, sha_a2)}
            and not u15
            and len(s15b) == 0
            and len(u15b) == 1,
            f"mixed-shape table → {len(s15)} site(s) {sorted((x.scenario for x in s15))}; "
            f"orphan → {len(s15b)} site(s) / {len(u15b)} unresolved (must be 0 / 1)",
        )

        # P16 — the writer edits ONE row. A row has no const on its line, so the edit is
        # keyed on the literal; keyed wrongly it would rewrite the whole table and still
        # report one rewrite. The "exactly one line differs" clause is what catches that.
        # The two rows carry the SAME literal, and only the second one is stale: row 1
        # legitimately pins `sha_a`, row 2 holds `sha_a` but its row says `sha_a2`. That is
        # not a contrived shape — `strategy_anchors_unchanged.rs` really does pin one body
        # under two scenarios (`btc-2023-1m-sma-cross` and `-sma-baseline-refresh` share
        # `fc2e3b4a…`). A writer that replaced the literal file-wide instead of on the
        # site's own line would drag the CORRECT row along with the stale one, and the
        # first version of this probe could not tell the two apart: its stale value
        # appeared once, so both writers changed exactly one line and it passed.
        f16 = _table_fixture("p16.rs", ns_a, [(tbl_s1, sha_a), (tbl_s2, sha_a)])
        before16 = f16.read_text(encoding="utf-8")
        s16, _ = parse_anchor_tables(f16)
        wrote16 = apply_write(f16, s16, tbl_canonical, tbl_anchors)
        after16 = f16.read_text(encoding="utf-8")
        changed16 = [
            (b, a)
            for b, a in zip(before16.splitlines(), after16.splitlines())
            if b != a
        ]
        probe(
            "P16 --write fixes only the stale table row",
            wrote16 == 1
            and len(changed16) == 1
            and after16.count(sha_a) == 1
            and after16.count(sha_a2) == 1,
            f"{wrote16} rewrite(s), {len(changed16)} line(s) changed (must be 1 and 1); "
            f"correct row survived: {after16.count(sha_a)} × sha_a (must be 1 — 0 means "
            f"the writer dragged it along), stale row fixed: {after16.count(sha_a2)} × "
            "sha_a2 (must be 1)",
        )

        # P17 — a const whose NAME lacks ANCHOR is a site only when it declares its
        # scenario. The negative side is load-bearing: an unrelated digest (a data
        # revision, a model checkpoint) must be passed over in SILENCE, not reported
        # unresolved, or this tool starts failing on constants that were never its business.
        p17a = tmp / "p17a.rs"
        p17a.write_text(
            f"// anchor-scenario: {syn_scenario}\n"
            f'const EXPECTED_SHA_X: &str = "{syn_sha}";\n',
            encoding="utf-8",
        )
        p17b = tmp / "p17b.rs"
        p17b.write_text(
            f'const EXPECTED_SHA_X: &str = "{syn_sha}";\n', encoding="utf-8"
        )
        s17a, u17a = parse_anchor_literals(p17a)
        s17b, u17b = parse_anchor_literals(p17b)
        probe(
            "P17 a non-ANCHOR const is a site only if it declares its scenario (two-sided)",
            len(s17a) == 1
            and s17a[0].scenario == syn_scenario  # noqa: B011 — guarded by the len above
            and not u17a
            and len(s17b) == 0
            and len(u17b) == 0,
            f"declared → {len(s17a)} site(s) / {len(u17a)} unresolved; bare → "
            f"{len(s17b)} / {len(u17b)} (must be 1/0 then 0/0)",
        )

        # P18 — the two `predecessor_sha` citations are excluded BY DECISION. If a later
        # sweep adds that file to SCANNED_FILES to "finish" the census gap, this
        # contradiction fires and sends the reader to the reason.
        excluded_paths = {rel for rel, _n, _why in EXCLUDED_CITATION_SITES}
        scanned_rel = {str(p.relative_to(REPO_ROOT)) for p in SCANNED_FILES}
        probe(
            "P18 the citation file is excluded deliberately and is not also scanned",
            "crates/forecast/src/bin/forecast_distribution.rs" in excluded_paths
            and not (excluded_paths & scanned_rel)
            and all(len(why) > 120 for _r, _n, why in EXCLUDED_CITATION_SITES),
            f"excluded={sorted(excluded_paths)}; overlap with SCANNED_FILES="
            f"{sorted(excluded_paths & scanned_rel) or 'none'}",
        )

    if failures:
        print(
            f"\ncheck_determinism_anchors --self-test: FAIL — {len(failures)} probe(s) did not "
            "fire. The linter cannot be trusted to fail, which is bug-log #115.",
            file=sys.stderr,
        )
        return 1
    print(f"check_determinism_anchors --self-test: OK — {n_probes} probes fired")
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
        # bug-log #180: `--write` now resolves every site through
        # `sha_for(..., site.declared_ns)` — the same resolver `detect_drift` uses — so a
        # declared namespace is HONOURED instead of refused. The blanket refusal below
        # existed because the writer knew only the canonical map; it no longer does.
        #
        # What is still refused, per-site and never guessed: a site declaring a namespace
        # that has no matching anchors.toml row. Writing one of those would mean choosing a
        # row on the site's behalf, which is bug-log #77's mistake with the tool's own hand.
        #
        # bug-log #180 durable fix — this predicate was a FOURTH place that re-derived the
        # order (`sha_for(...) is None and scenario not in SYNTHETIC_...`). It now asks the
        # resolver, which already answers exactly this question: a declared site that
        # resolves to nothing. The dropped `scenario not in SYNTHETIC_DETERMINISM_SHAS`
        # clause only ever downgraded the refusal to a per-site WARN for a site declaring a
        # BAD namespace on one of the three synthetic scenarios — a configuration no site
        # has, and one the detector already treats as a hard error. Refusing it is the
        # consistent answer and what this refusal's own message describes.
        unresolvable = [
            st for st in sites
            if st.declared_ns is not None
            and resolve_expected(st, anchors, canonical)[0] is None
        ]
        if unresolvable:
            print(
                f"REFUSING --write: {len(unresolvable)} site(s) declare a namespace with no "
                "matching anchors.toml row, so writing would have to pick a row for them "
                "(bug-log #77). Fix the declaration or add the row. Sites:",
                file=sys.stderr,
            )
            for st in unresolvable:
                print(
                    f"  {st.file}:{st.lineno} fn {st.fn_name} — anchor-ns: {st.declared_ns}",
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
                n += apply_write(path, mine, canonical, anchors)
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
    # Count what the RESOLUTION did, not what the scenario name suggests — a scenario-keyed
    # count over-reports, because a site that declares its namespace suppresses the
    # synthetic map. That is the same "a number nobody measured" shape this tool exists to
    # catch, appearing in its own summary line.
    #
    # bug-log #180 durable fix — and the count is taken by ASKING the resolver, not by
    # re-deriving its order here. This block was the THIRD copy of that order, and the
    # arithmetic `n_ok - n_synth - n_declared` made the canonical column a remainder that
    # could absorb any disagreement between the copies without ever looking wrong.
    #
    # bug-log #131 — declared-namespace sites are a column of their own rather than folded
    # into "canonical". 13 of the 29 determinism.rs sites target a NON-canonical row;
    # calling them canonical is the same unstated assumption as scanning one file and
    # saying "29 of 29".
    label_counts: dict[str, int] = {"canonical": 0, "declared-ns": 0, "synthetic": 0}
    for s in sites:
        if s.cfg_gated:
            continue
        _, source = resolve_expected(s, anchors, canonical)
        if source is not None:
            label_counts[source] += 1
    n_synth = label_counts["synthetic"]
    n_declared = label_counts["declared-ns"]
    n_canonical = label_counts["canonical"]
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
