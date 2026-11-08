#!/usr/bin/env python3
"""Guard the one corpus failure invisible to every other gate: a NAMESPACE COLLAPSE.

A scenario can carry several `anchors.toml` rows, one per friction condition — the
pre-friction `noop-baseline` oracle, the canonical `v5-realdata-medium-2026-05` reference at
8 bps linear slippage, and `v5-sqrt-impact-2026-05`. Those rows are supposed to hold
DIFFERENT bodies, because they describe different conditions.

Nothing checked that. `verify_anchors.sh` asks whether each row's SHA matches a committed
body; it is perfectly happy when two rows of one scenario point at the SAME body. So a
re-emission that runs one invocation and writes its output to two rows silently merges two
namespaces into one, and the gate still prints `119 / 119`.

This was not hypothetical. The 2026-10-29 re-lock wrote default (zero-slippage) output into
the CANONICAL rows of six real-data scenarios — measured: `top10-2023-fy-tcn-overlay-realdata`
under `--sim-slippage-bps 8` returns -55.88% against its canonical row's -27.52%, a 28 pp gap.
And the next re-emission was about to do the mirror of it: re-emitting the stale noop rows with
the default invocation would have given both rows the same body and destroyed "the current
reference for paper-trading alpha" (`anchors.toml:9-11`) without a single gate objecting.

So: the number of DISTINCT bodies a scenario's rows point at is declared below and may not
shrink. Fifteen scenarios legitimately share one SHA across rows — `verify_anchors.sh` itself
documents "canonical SHA = noop SHA for non-re-emittable scenarios" — so the check is not "all
rows differ", it is "the declared partition does not change in either direction".

Usage:
    uv run --python 3.11 scripts/anchor_namespace_collapse_check.py [<anchors.toml>]
    uv run --python 3.11 scripts/anchor_namespace_collapse_check.py --self-test

Exit: 0 clean · 1 a scenario's distinct-body count changed · 2 structural (the row shape
changed, or the scan found fewer rows than the floor).

Bug-log: `#179`. Protocol: ADR-0038 § D6.b / § D6.c.
"""

from __future__ import annotations

import collections
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
ANCHORS = REPO / "evidence" / "anchors.toml"

_ROW_RE = re.compile(
    r'scenario\s*=\s*"(?P<s>[^"]+)"\s*\n'
    r'\s*version\s*=\s*"(?P<v>[^"]+)"\s*\n'
    r'\s*sha256\s*=\s*"(?P<h>[0-9a-f]{64})"'
)

ROW_FLOOR = 119

# scenario -> how many DISTINCT bodies its rows point at, as of 2026-11-08.
# Only scenarios with more than one row are listed; a single-row scenario cannot collapse.
# Changing a number here is a decision: say in the commit which namespace gained or lost a
# distinct body and why.
DECLARED: dict[str, int] = {
    "btc-2023-1m-bbands-mean-revert": 2,
    "btc-2023-1m-macd-trend": 2,
    "btc-2023-1m-rsi-reversion": 2,
    "btc-2023-1m-sma-baseline-refresh": 2,
    "btc-2023-1m-sma-cross": 2,
    "forecast-distribution-bs1-realdata": 1,
    "forecast-distribution-bs1-realdata-recalibrated": 1,
    "forecast-distribution-bs2-realdata": 1,
    "forecast-distribution-bs2-realdata-recalibrated": 1,
    "forecast-distribution-patchtst-bs1-realdata": 1,
    "pairs-2023-zscore-mr": 2,
    "pairs-2024-h1-zscore-mr": 2,
    "recalibrate-sigma-train-bs1": 1,
    "recalibrate-sigma-train-bs2": 1,
    "report-sample-7d": 1,
    "report-sample-90d": 1,
    "sharpe-comparison-realdata": 1,
    "sharpe-comparison-vol-target-bs1-realbaseline": 1,
    "sharpe-comparison-vol-target-bs1-realdata": 1,
    "threshold-sweep-bs1-realdata-recalibrated": 1,
    "threshold-sweep-bs2-realdata-recalibrated": 1,
    "top10-2023-1h-momentum": 2,
    "top10-2023-fy-patchtst-overlay-realdata": 3,
    "top10-2023-fy-regime-dispatcher-realdata": 2,
    "top10-2023-fy-tcn-overlay": 2,
    "top10-2023-fy-tcn-overlay-realdata": 3,
    "top10-2023-fy-tcn-overlay-weights": 2,
    "top10-2023-fy-tcn-overlay-weights-realdata": 3,
    "top10-2023-fy-vol-target-overlay-realdata": 3,
    "top10-2024-fy-regime-dispatcher-realdata": 2,
    "top10-2024-fy-tcn-overlay": 2,
    "top10-2024-fy-tcn-overlay-realdata": 3,
    "top10-2024-fy-tcn-overlay-weights": 2,
    "top10-2024-fy-tcn-overlay-weights-realdata": 3,
    "top10-2024-h1-momentum": 2,
    "vol-verdict-bs1-realdata": 1,
}


def parse(text: str) -> dict[str, list[str]]:
    by: dict[str, list[str]] = collections.defaultdict(list)
    for m in _ROW_RE.finditer(text):
        by[m.group("s")].append(m.group("h"))
    return by


def check(by: dict[str, list[str]]) -> tuple[int, list[str]]:
    problems: list[str] = []
    for scenario, want in sorted(DECLARED.items()):
        shas = by.get(scenario)
        if shas is None:
            problems.append(f"{scenario}: declared {want} distinct body(ies) but the scenario "
                            f"has no rows at all")
            continue
        got = len(set(shas))
        if got < want:
            problems.append(
                f"{scenario}: COLLAPSE — {len(shas)} row(s) now point at {got} distinct "
                f"body(ies), declared {want}. Two conditions have been merged into one body. "
                f"Re-emit each row with ITS declared invocation (default = noop-baseline, "
                f"`--sim-slippage-bps 8` = canonical, sqrt flags = v5-sqrt-impact)."
            )
        elif got > want:
            problems.append(
                f"{scenario}: SPLIT — {len(shas)} row(s) now point at {got} distinct "
                f"body(ies), declared {want}. That may be a repair; update DECLARED and say "
                f"so in the commit."
            )
    for scenario, shas in sorted(by.items()):
        if len(shas) > 1 and scenario not in DECLARED:
            problems.append(f"{scenario}: {len(shas)} rows and no DECLARED entry — add one")
    return (1 if problems else 0), problems


def self_test() -> int:
    base = {"a": ["h1", "h2"], "b": ["h3", "h3"], "c": ["h4"]}
    saved = dict(DECLARED)
    DECLARED.clear()
    DECLARED.update({"a": 2, "b": 1})
    rc = 0
    cases = [
        ("clean control passes", base, 0),
        ("a COLLAPSE fires", {**base, "a": ["h1", "h1"]}, 1),
        ("a SPLIT fires", {**base, "b": ["h3", "h9"]}, 1),
        ("an undeclared multi-row scenario fires", {**base, "d": ["h5", "h6"]}, 1),
    ]
    for label, data, want in cases:
        got, probs = check(data)
        if got != want:
            print(f"SELF-TEST FAIL: {label}: rc={got} want {want} ({probs})", file=sys.stderr)
            rc = 1
        else:
            print(f"  OK    {label}")
    DECLARED.clear()
    DECLARED.update(saved)
    print("ANCHOR-NAMESPACE-COLLAPSE SELF-TEST " + ("PASS (4 probes)" if rc == 0 else "FAIL"))
    return rc


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    # An optional path argument lets this gate be probed against a MUTATED COPY of the real
    # file, which is the only way to show it fires on real input without editing the corpus.
    # The self-test exercises the logic; this exercises the logic AND the parser together.
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    target = Path(args[0]) if args else ANCHORS
    text = target.read_text(encoding="utf-8")
    by = parse(text)
    total = sum(len(v) for v in by.values())
    if total < ROW_FLOOR:
        print(f"FAIL  parsed {total} anchor row(s), floor {ROW_FLOOR} — the row shape changed "
              f"or the file moved. An empty scan must not read as compliance.", file=sys.stderr)
        return 2
    rc, problems = check(by)
    if problems:
        print("anchor-namespace-collapse: FAIL", file=sys.stderr)
        for p in problems:
            print(f"  {p}", file=sys.stderr)
        return rc
    multi_n = sum(1 for v in by.values() if len(v) > 1)
    print(f"anchor-namespace-collapse: PASS ({total} row(s) over {len(by)} scenario(s) "
          f"[floor {ROW_FLOOR}] · {multi_n} multi-row scenario(s), all partitions intact)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
