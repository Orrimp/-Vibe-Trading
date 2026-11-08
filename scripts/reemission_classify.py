#!/usr/bin/env python3
"""Classify every anchored row for an ADR-0038 § D6.b / § D6.c re-emission pass.

§ D6.c step 2 requires the affected rows enumerated **mechanically, by writer** with the
count recorded, and step 3 requires proof that a content addition did not move a figure.
Step 4 refuses any row whose figures DID move and routes it to § D6.b. Doing that by eye
over 119 rows is how a number change gets laundered inside a content change, so it is done
here instead.

Usage
-----
    uv run --python 3.11 scripts/reemission_classify.py \
        --noop-dir <d> --canon-dir <d> --sqrt-dir <d>
    uv run --python 3.11 scripts/reemission_classify.py --self-test

`--new-dir` holds freshly produced bodies, one per scenario, named
`backtest-<stamp>-<scenario>.md` (what `backtest --reports-dir` emits). Produce them with
`--reports-dir` pointed OUTSIDE `evidence/`: a fresh body inside the corpus can win
`verify_anchors.sh`'s `find | sort | tail -1` resolution and flip the gate as a side effect
(bug-log `#113`).

Buckets
-------
``UNCHANGED``  the new body's hash equals the row's — the § D6.c step 6 negative invariant.
``D6C``        differs, and every differing line is in the DECLARED additive set below.
``D6B``        differs in at least one FIGURE line — refused by § D6.c step 4, routed to
               § D6.b, which needs its own smoking gun and would-have-caught test per row.
``ORACLE``     a `noop-baseline` row on a SYNTHETIC scenario: a frozen pre-friction
               oracle, unreachable from current code by design (Q-D1=(a) forces
               `Linear{bps:8}` there). Never re-emitted.
``UNRUN``      no new body: the scenario is rendered by code outside the `backtest` CLI
               (`crates/forecast`'s sharpe-comparison and verdict bins, `crates/reports`,
               the threshold-sweep bin), or needs a feature this binary lacks. Unchanged by
               construction, but say so rather than counting it as compliance.

The body hash comes from `scripts/hash_report.py`'s `body_hash` by IMPORT, not by
reimplementation, so this tool and `verify_anchors.sh` cannot drift apart on the one
definition they share (ADR-0097 § D3's rule).
"""

from __future__ import annotations

import argparse
import difflib
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hash_report import body_hash  # noqa: E402  — the canonical definition, imported

REPO = Path(__file__).resolve().parent.parent
ANCHORS = REPO / "evidence" / "anchors.toml"

_ROW_RE = re.compile(
    r'scenario\s*=\s*"(?P<scenario>[^"]+)"\s*\n'
    r'\s*version\s*=\s*"(?P<version>[^"]+)"\s*\n'
    r'\s*sha256\s*=\s*"(?P<sha>[0-9a-f]{64})"'
)

# Lines a content addition is ALLOWED to introduce or change. Anything else differing makes
# the row a § D6.b row. Keep this list minimal and explicit — every entry widens what can
# pass as "content only".
DECLARED_ADDITIVE = (
    "| Buy and hold (equal-wt)",
    "| vs buy and hold",
    "- Risk:",      # bug-log #178 — the risk sentence became derived from the limits
    "- Size:",      # bug-log #178 — regime_dispatcher's exposure_cap stopped being a literal
)

# Lines that carry the arm's OWN result. One of these differing means the run changed, not
# just the rendering.
FIGURE_PREFIXES = (
    "| Final equity",
    "| Total return",
    "| Max drawdown",
    "| Trades",
    "| Buys",
    "| Sells",
    "| Total fees",
    "| Gross-cap refusals",
    "| Sharpe",
    "| Initial capital",
)


# Scenarios listed in `REAL_DATA_SCENARIO_IDS` (crates/backtest/src/main.rs) honour the
# CLI slippage flags. Every OTHER scenario has `Linear{bps:8}` FORCED on it by the
# v5-latency-slippage-sim v0.5.0 Q-D1=(a) decision (`7e8a7e03`), regardless of flags — so a
# default run of a synthetic scenario already IS the canonical condition, while a default run
# of a real-data scenario is the noop one. One command, two namespaces.
REAL_DATA_SCENARIOS = frozenset({
    "top10-2023-fy-momentum-realdata",
    "top10-2023-fy-patchtst-overlay-realdata",
    "top10-2023-fy-regime-dispatcher-realdata",
    "top10-2023-fy-tcn-overlay-realdata",
    "top10-2023-fy-tcn-overlay-weights-realdata",
    "top10-2023-fy-vol-target-overlay-realdata",
    "top10-2024-fy-regime-dispatcher-realdata",
    "top10-2024-fy-tcn-overlay-realdata",
    "top10-2024-fy-tcn-overlay-weights-realdata",
})


# Rows whose body is a SYNTHETIC run even though real parquet data exists on disk, so the
# auto-detect at the single-symbol dispatch site must be overridden with
# `--force-synthetic-bars` (ADR-0047 D1, the v0.3.0 Q1=(a) guard).
#
# Established by reading each committed body's own `data_source:` and `Bars replayed`:
# `btc-2023-1m-sma-cross`'s canonical row is synthetic at 525,601 one-minute bars, while
# `btc-2023-1m-macd-trend`'s canonical row on the SAME data is real at 17,544 — the corpus
# is not uniform here, and the row is what counts.
FORCED_SYNTHETIC = frozenset({
    "btc-2023-1m-sma-cross",
    "btc-2023-1m-sma-baseline-refresh",
})


def invocation_for(scenario: str, version: str) -> str:
    """Which declared invocation produces the body this row pins.

    The mapping is not trusted blindly: a row whose produced body hashes to the row's SHA
    *proves* the invocation was the right one, and a row that lands in D6B with every figure
    moved is the signal that it was not. That is why this returns a label rather than a
    silent directory choice.
    """
    if "v5-sqrt-impact-2026-05" in version:
        return "sqrt"
    if "noop-baseline" in version:
        # A `noop-baseline` row on a SYNTHETIC scenario is a FROZEN pre-friction oracle, not
        # a reproducible body. Q-D1=(a) forces `Linear{bps:8}` on those scenarios regardless
        # of flags, so the zero-slippage condition the row pins is unreachable from current
        # code BY DESIGN — anchors.toml calls these "pre-friction historical oracle, SHAs
        # from v0.1.0 ship". Re-emitting one would not fix a stale pin; it would destroy the
        # pre-friction baseline the corpus keeps deliberately.
        #
        # This mattered: without this branch the classifier put 20+ frozen oracles in D6B
        # with every figure "moved", which is true and completely the wrong conclusion.
        return "noop" if scenario in REAL_DATA_SCENARIOS else "oracle"
    if "v5-realdata-medium-2026-05" in version:
        if scenario in REAL_DATA_SCENARIOS:
            return "canon"
        return "synth" if scenario in FORCED_SYNTHETIC else "noop"
    # Legacy single-namespace rows: verify_anchors.sh takes the globally newest body, which is
    # the default invocation's.
    return "noop"


def strip_frontmatter(text: str) -> str:
    m = re.match(r"^---\n.*?\n---\n", text, re.DOTALL)
    return text[m.end():] if m else text


def classify_diff(old_body: str, new_body: str) -> tuple[str, list[str]]:
    """Return (bucket, notes) for two bodies known to differ."""
    changed = [
        line
        for line in difflib.unified_diff(
            old_body.splitlines(), new_body.splitlines(), lineterm="", n=0
        )
        if line.startswith(("+", "-")) and not line.startswith(("+++", "---"))
    ]
    payload = [ln[1:].strip() for ln in changed]

    # A figure that did not exist in the old body cannot have MOVED — it was added, which is
    # a content change. October's `Gross-cap refusals` row is exactly that case on lanes
    # whose cap is None: `| Gross-cap refusals | 0 |` appears where there was no such line.
    # Comparing per-key rather than counting `+`/`-` lines is what separates "this run
    # produced a different number" from "this renderer prints one more number", and that
    # distinction is the whole of § D6.c step 3.
    def figures(body: str) -> dict[str, str]:
        out: dict[str, str] = {}
        for line in body.splitlines():
            t = line.strip()
            if not t.startswith(FIGURE_PREFIXES):
                continue
            cells = [c.strip() for c in t.split("|")]
            if len(cells) >= 3:
                out[cells[1]] = cells[2]
        return out

    old_f, new_f = figures(old_body), figures(new_body)
    moved_figures = sorted(
        k for k in old_f.keys() & new_f.keys() if old_f[k] != new_f[k]
    )
    added_figures = sorted(new_f.keys() - old_f.keys())
    if moved_figures:
        notes = [f"MOVED {k}: {old_f[k]} -> {new_f[k]}" for k in moved_figures]
        # A row where EVERY shared figure moved is far more likely to have been run under the
        # wrong CONDITION than to have drifted: a code change moves some numbers, a different
        # data source or friction model moves all of them. determinism.rs says the same thing
        # — a default-invocation run "would be compared against a condition it never ran".
        #
        # This is not decoration. It is how `btc-2023-1m-sma-cross` was caught: its canonical
        # row pins a 525,601-bar SYNTHETIC body, the default run auto-detected 17,544 bars of
        # real data, and all eight figures moved. Routed to § D6.b it would have re-pinned a
        # frozen condition to an unrelated run.
        shared = old_f.keys() & new_f.keys()
        if shared and len(moved_figures) == len(shared):
            notes.append(
                "!! ALL {} shared figures moved — suspect a WRONG INVOCATION, not drift. "
                "Check the row's declared condition (data source, sim-slippage model) "
                "before routing this to § D6.b.".format(len(shared))
            )
        return "D6B", notes
    allowed = DECLARED_ADDITIVE + tuple(f"| {k}" for k in added_figures)
    undeclared = sorted({p[:70] for p in payload if not p.startswith(allowed) and p})
    if undeclared:
        return "D6B", [f"undeclared line: {u}" for u in undeclared]
    notes = [f"added figure (no prior value): {k}" for k in added_figures]
    return "D6C", notes


def parse_rows(text: str) -> list[dict[str, str]]:
    return [m.groupdict() for m in _ROW_RE.finditer(text)]


def index_committed_bodies() -> dict[str, Path]:
    """body-SHA -> committed report path. Exact, so a row maps to ITS body unambiguously."""
    idx: dict[str, Path] = {}
    for p in (REPO / "evidence").rglob("*/reports/*.md"):
        try:
            idx.setdefault(body_hash(p), p)
        except (OSError, UnicodeDecodeError):
            continue
    return idx


def find_new_body(new_dir: Path, scenario: str) -> Path | None:
    cands = sorted(new_dir.glob(f"*-{scenario}.md"))
    # `*-{scenario}.md` can also match a LONGER scenario name's suffix; require the stem to
    # end with exactly this scenario after a `-<stamp>-` prefix.
    exact = [c for c in cands if c.stem.endswith(scenario)]
    return exact[-1] if exact else None


def run(dirs: dict[str, Path]) -> int:
    rows = parse_rows(ANCHORS.read_text(encoding="utf-8"))
    if not rows:
        print("FAIL  parsed 0 anchor rows — the row shape changed", file=sys.stderr)
        return 2
    committed = index_committed_bodies()
    buckets: dict[str, list[tuple[dict[str, str], list[str]]]] = {
        "UNCHANGED": [], "D6C": [], "D6B": [], "UNRUN": [], "ORACLE": [],
        "NO-OLD-BODY": [],
    }

    for row in rows:
        inv = invocation_for(row["scenario"], row["version"])
        if inv == "oracle":
            buckets["ORACLE"].append((row, ["frozen pre-friction oracle — NOT re-emitted"]))
            continue
        src = dirs.get(inv)
        new = find_new_body(src, row["scenario"]) if src and src.is_dir() else None
        if new is None:
            buckets["UNRUN"].append((row, [f"invocation={inv}"]))
            continue
        old_path = committed.get(row["sha"])
        if old_path is None:
            buckets["NO-OLD-BODY"].append((row, []))
            continue
        new_sha = body_hash(new)
        if new_sha == row["sha"]:
            buckets["UNCHANGED"].append((row, []))
            continue
        bucket, notes = classify_diff(
            strip_frontmatter(old_path.read_text(encoding="utf-8")),
            strip_frontmatter(new.read_text(encoding="utf-8")),
        )
        buckets[bucket].append((row, notes + [f"invocation={inv}", f"new={new_sha}"]))

    total = len(rows)
    print(f"re-emission classification over {total} anchor row(s)\n")
    for name in ("D6B", "D6C", "UNCHANGED", "ORACLE", "UNRUN", "NO-OLD-BODY"):
        items = buckets[name]
        print(f"  {name:12s} {len(items):3d}")
    print()
    for name in ("D6B", "D6C"):
        if not buckets[name]:
            continue
        print(f"── {name} ──")
        for row, notes in buckets[name]:
            print(f"  {row['scenario']}  [{row['version']}]")
            print(f"      old={row['sha']}")
            for n in notes:
                print(f"      {n}")
        print()
    if buckets["NO-OLD-BODY"]:
        print("── NO-OLD-BODY (a row whose SHA matches no committed body — investigate) ──")
        for row, _ in buckets["NO-OLD-BODY"]:
            print(f"  {row['scenario']}  [{row['version']}]  {row['sha']}")
        print()
    # A pass where nothing differs means the run did not exercise the change.
    if not buckets["D6C"] and not buckets["D6B"]:
        print("NOTE  no row differs. Either the change is inert, or --new-dir is wrong.")
    return 1 if buckets["NO-OLD-BODY"] else 0


def self_test() -> int:
    """Probe the three classifications against hand-built bodies."""
    base = "\n## KPIs\n\n| Final equity | $100.00 |\n| Trades | 10 |\n- Risk: cap=40%\n"
    content_only = base.replace(
        "| Trades | 10 |",
        "| Trades | 10 |\n| Buy and hold (equal-wt) | +9.18% |\n| vs buy and hold | -15.40 pp |",
    ).replace("- Risk: cap=40%", "- Risk: per-symbol cap=40%; portfolio gross cap: none")
    figures_moved = content_only.replace("| Trades | 10 |", "| Trades | 9 |")
    undeclared = content_only + "| Smuggled row | 1 |\n"

    rc = 0
    # A figure ADDED (absent in the old body) is content, not movement. Without this probe
    # the per-key comparison branch is pinned by nothing: the old `+`/`-` counting classified
    # it D6B, and both readings are "a figure line appears in the diff".
    figure_added = content_only.replace(
        "| Trades | 10 |", "| Trades | 10 |\n| Gross-cap refusals | 0 |"
    )
    # ...and the same KEY present on both sides with different values must still be D6B, or
    # the branch above would launder every number change as an addition.
    same_key_moved = base.replace("| Trades | 10 |", "| Trades | 11 |")

    cases = [
        ("an ADDED figure (no prior value) -> D6C", figure_added, "D6C"),
        ("the SAME figure with a new value -> D6B", same_key_moved, "D6B"),
        ("content-only -> D6C", content_only, "D6C"),
        ("a moved figure -> D6B", figures_moved, "D6B"),
        ("an undeclared line -> D6B", undeclared, "D6B"),
    ]
    for label, new, want in cases:
        got, notes = classify_diff(base, new)
        if got != want:
            print(f"SELF-TEST FAIL: {label}: got {got} ({notes}), want {want}", file=sys.stderr)
            rc = 1
        else:
            print(f"  OK    {label}")
    # And the control: identical bodies must not be classified at all by this function's
    # callers — assert the diff is empty so a D6C verdict cannot come from no change.
    if [
        l for l in difflib.unified_diff(base.splitlines(), base.splitlines(), lineterm="")
        if l.startswith(("+", "-")) and not l.startswith(("+++", "---"))
    ]:
        print("SELF-TEST FAIL: identical bodies produced a diff", file=sys.stderr)
        rc = 1
    else:
        print("  OK    control: identical bodies produce an empty diff")
    print("REEMISSION-CLASSIFY SELF-TEST " + ("PASS (5 probes + 1 control)" if rc == 0 else "FAIL"))
    return rc


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--noop-dir", type=Path, help="bodies from the DEFAULT invocation")
    ap.add_argument("--canon-dir", type=Path, help="bodies from --sim-slippage-bps 8")
    ap.add_argument("--sqrt-dir", type=Path, help="bodies from the sqrt-impact flags")
    ap.add_argument("--synth-dir", type=Path, help="bodies from --force-synthetic-bars")
    ap.add_argument("--self-test", action="store_true")
    a = ap.parse_args()
    if a.self_test:
        return self_test()
    dirs = {k: v for k, v in
            (("noop", a.noop_dir), ("canon", a.canon_dir), ("sqrt", a.sqrt_dir),
             ("synth", a.synth_dir)) if v}
    if not dirs:
        print(
            "usage: reemission_classify.py --noop-dir <d> [--canon-dir <d>] "
            "[--sqrt-dir <d>] | --self-test",
            file=sys.stderr,
        )
        return 2
    return run(dirs)


if __name__ == "__main__":
    sys.exit(main())
