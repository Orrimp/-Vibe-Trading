#!/usr/bin/env python3
"""Census every file that MIRRORS a SHA from `evidence/anchors.toml`, and gate the count.

A body SHA is pinned in `anchors.toml`. It is also COPIED into source, as a `const ANCHOR`,
a `&[(&str, &str)]` table, a dict value. Each copy is a place the corpus can drift from
itself, and on 2026-11-08 four separate ones bit in a single session:

  * `#180` — detector and writer of the sync tool resolved differently;
  * `#181` — `SYNTHETIC_DETERMINISM_SHAS`, a map compared against itself;
  * `#182` — two yahoo rows filed "unchanged by construction" because one sweep did not
    render them;
  * and `crates/reports/tests/strategy_anchors_unchanged.rs`, holding **35** mirrors and
    scanned by nothing, which took CI red on all three legs.

`scripts/check_determinism_anchors.py` syncs five files. Measured here: **nine** files mirror
a current anchors row. So the question "how many places is a SHA pinned?" had no answer, and
every answer given was too low. This script answers it mechanically and pins the answer.

A **mirror** is a 64-hex literal equal to some CURRENT `anchors.toml` SHA. That definition is
deliberately narrow: a superseded SHA quoted in a comment is NOT a mirror (it names history,
and D6.b requires keeping it), and a data-revision or checkpoint digest is not one either. So
`MIRROR_FLOOR` can be pinned without churning on every re-emission:

  * a mirror going stale stops matching, the file's count DROPS below its floor -> FAIL;
  * adding history or a new unrelated digest does not move the count;
  * a file gaining mirrors, or a NEW file mirroring anything, is reported and must be declared.

Usage:
    uv run --python 3.11 scripts/sha_mirror_census.py
    uv run --python 3.11 scripts/sha_mirror_census.py --self-test
    uv run --python 3.11 scripts/sha_mirror_census.py --list      # print current counts

Exit: 0 clean · 1 a file dropped below its floor, or an undeclared file mirrors · 2 structural.

Bug-log: `#183`.
"""

from __future__ import annotations

import glob
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
ANCHORS = REPO / "evidence" / "anchors.toml"
_HEX = re.compile(r"[0-9a-f]{64}")
_ROW_SHA = re.compile(r'sha256\s*=\s*"([0-9a-f]{64})"')

SCAN_GLOBS = ("crates/*/tests/*.rs", "crates/*/src/**/*.rs", "scripts/*.py")

# file -> minimum number of distinct CURRENT-anchor mirrors it must still contain.
# Measured 2026-11-08. Lowering a number here is a claim that a mirror was deliberately
# removed; say which, and why, in the commit.
MIRROR_FLOOR: dict[str, int] = {
    "crates/backtest/tests/determinism.rs": 20,
    "crates/backtest/tests/multi_pair_determinism.rs": 2,
    "crates/backtest/tests/reproducibility_sample_figure.rs": 1,
    "crates/backtest/tests/run_yahoo_sma_ticker_flag.rs": 2,
    "crates/backtest/tests/theta_surface_reproduction.rs": 1,
    "crates/forecast/src/bin/forecast_distribution.rs": 2,
    "crates/reports/tests/report_scenarios.rs": 2,
    "crates/reports/tests/strategy_anchors_unchanged.rs": 35,
    "crates/reports/tests/t1003_orchestrator_smoke.rs": 1,
}

# Which files `check_determinism_anchors.py` keeps in sync. The REST are synced by hand, and
# that asymmetry is the standing risk this census exists to keep visible rather than fix by
# fiat — teaching the sync tool to parse a `&[(&str, &str)]` table is a separate change.
SYNCED_BY_TOOL = frozenset({
    "crates/backtest/tests/determinism.rs",
    "crates/backtest/tests/multi_pair_determinism.rs",
    "crates/backtest/tests/reproducibility_sample_figure.rs",
    "crates/backtest/tests/run_yahoo_sma_ticker_flag.rs",
    "crates/backtest/tests/theta_surface_reproduction.rs",
})


def current_shas(text: str) -> set[str]:
    return set(_ROW_SHA.findall(text))


def census(shas: set[str], root: pathlib.Path) -> dict[str, int]:
    out: dict[str, int] = {}
    for pattern in SCAN_GLOBS:
        for f in glob.glob(str(root / pattern), recursive=True):
            rel = str(pathlib.Path(f).relative_to(root))
            try:
                body = pathlib.Path(f).read_text(errors="replace")
            except OSError:
                continue
            n = len({lit for lit in _HEX.findall(body) if lit in shas})
            if n:
                out[rel] = n
    return out


def check(found: dict[str, int]) -> tuple[int, list[str]]:
    problems: list[str] = []
    for rel, floor in sorted(MIRROR_FLOOR.items()):
        got = found.get(rel, 0)
        if got < floor:
            problems.append(
                f"{rel}: {got} mirror(s) of current anchors, floor {floor}. "
                f"{floor - got} literal(s) that used to match a row no longer do — they are "
                f"STALE copies. Sync them from anchors.toml"
                + ("" if rel in SYNCED_BY_TOOL else
                   " (this file is NOT synced by check_determinism_anchors.py — by hand)")
                + "."
            )
    for rel, got in sorted(found.items()):
        if rel not in MIRROR_FLOOR:
            problems.append(
                f"{rel}: mirrors {got} current anchor SHA(s) and is UNDECLARED. Every place a "
                f"SHA is copied is a place the corpus can drift from itself — add it to "
                f"MIRROR_FLOOR (and consider whether it belongs in the sync tool)."
            )
    return (1 if problems else 0), problems


def self_test() -> int:
    saved = dict(MIRROR_FLOOR)
    MIRROR_FLOOR.clear()
    MIRROR_FLOOR.update({"a.rs": 3, "b.rs": 1})
    rc = 0
    cases = [
        ("clean control passes", {"a.rs": 3, "b.rs": 1}, 0),
        ("a STALE mirror (count drops) fires", {"a.rs": 2, "b.rs": 1}, 1),
        ("a file losing all mirrors fires", {"b.rs": 1}, 1),
        ("an UNDECLARED mirroring file fires", {"a.rs": 3, "b.rs": 1, "c.rs": 1}, 1),
        ("gaining mirrors is allowed", {"a.rs": 9, "b.rs": 1}, 0),
    ]
    for label, found, want in cases:
        got, probs = check(found)
        if got != want:
            print(f"SELF-TEST FAIL: {label}: rc={got} want {want} ({probs})", file=sys.stderr)
            rc = 1
        else:
            print(f"  OK    {label}")
    MIRROR_FLOOR.clear()
    MIRROR_FLOOR.update(saved)
    print("SHA-MIRROR-CENSUS SELF-TEST " + ("PASS (5 probes)" if rc == 0 else "FAIL"))
    return rc


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    shas = current_shas(ANCHORS.read_text(encoding="utf-8"))
    if len(shas) < 100:
        print(f"FAIL  parsed {len(shas)} anchor SHA(s) — expected >=100. The row shape changed "
              f"or the file moved; an empty parse must not read as compliance.", file=sys.stderr)
        return 2
    found = census(shas, REPO)
    if "--list" in sys.argv:
        for rel, n in sorted(found.items()):
            tool = "tool-synced" if rel in SYNCED_BY_TOOL else "BY HAND"
            print(f"  {n:3d}  {rel}  [{tool}]")
        return 0
    rc, problems = check(found)
    if problems:
        print("sha-mirror-census: FAIL", file=sys.stderr)
        for p in problems:
            print(f"  {p}", file=sys.stderr)
        return rc
    by_hand = sum(n for rel, n in found.items() if rel not in SYNCED_BY_TOOL)
    print(f"sha-mirror-census: PASS ({sum(found.values())} mirror literal(s) across "
          f"{len(found)} file(s), {len(shas)} anchor row SHAs · {by_hand} mirror(s) in files "
          f"the sync tool does NOT cover)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
