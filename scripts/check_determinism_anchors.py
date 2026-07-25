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
    in_cfg_gated_fn = False
    pending_cfg = False       # saw #[cfg(feature=...)] on previous line(s)
    brace_depth = 0
    fn_name: str = "unknown"
    fn_start_depth = 0

    # Regex patterns.
    const_anchor_re = re.compile(
        r'const\s+(ANCHOR(?:_PREFIX)?)\s*:\s*&str\s*=\s*"([0-9a-fA-F]{8,64})"'
    )
    scenario_call_re = re.compile(r'scenario_body_hex\("([^"]+)"\)')
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
            for j in range(i + 1, min(i + 8, len(lines))):
                sc_m = scenario_call_re.search(lines[j])
                if sc_m:
                    scenario = sc_m.group(1)
                    break

            if scenario is not None:
                sites.append(InTestSite(
                    lineno=i + 1,
                    fn_name=current_fn,
                    scenario=scenario,
                    const_name=const_name,
                    literal=literal,
                    cfg_gated=current_fn_cfg_gated,
                ))

        # Reset pending_cfg unless it was just set.
        if not _is_cfg_feature_line(stripped):
            pending_cfg = False

        i += 1

    return sites


# ---------------------------------------------------------------------------
# Drift detection
# ---------------------------------------------------------------------------

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
        synth = SYNTHETIC_DETERMINISM_SHAS.get(site.scenario)
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

        # --- Dual-map resolution (step 2: canonical anchors.toml) ---
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
                note="",
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
            canonical="(HARD ERROR: no canonical OR synthetic mapping)",
            match=False,
            note="add scenario to SYNTHETIC_DETERMINISM_SHAS or anchors.toml",
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
        "--pre-commit",
        action="store_true",
        dest="pre_commit",
        help="No-op if neither determinism.rs nor anchors.toml is staged.",
    )
    args = parser.parse_args()

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
        sites = parse_determinism_rs(DETERMINISM_RS)
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: could not parse {DETERMINISM_RS}: {exc}", file=sys.stderr)
        return 2

    canonical = canonical_sha_map(anchors)

    if not sites:
        print("WARN: no const ANCHOR sites found in determinism.rs", file=sys.stderr)
        return 0

    mismatches, skipped = detect_drift(sites, canonical)

    if args.write:
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
    n_synth = sum(
        1 for s in sites
        if not s.cfg_gated and s.scenario in SYNTHETIC_DETERMINISM_SHAS
    )
    n_canonical = n_ok - n_synth
    print(
        f"check_determinism_anchors: OK — {n_ok} literal(s) match "
        f"({n_canonical} canonical v5-realdata-medium-2026-05, {n_synth} synthetic; "
        f"{n_skip} skipped: cfg-gated)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
