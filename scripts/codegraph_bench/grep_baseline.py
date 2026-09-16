#!/usr/bin/env python3
"""What does an agent lose by using grep instead of CodeGraph — and vice versa?

usage: grep_baseline.py <gt.json> <repo_root> [codegraph.db]

`metrics.py` measures CodeGraph against the `gt.py` lexer ground truth. This measures
the OTHER tool an agent reaches for — `grep -rn "name(" crates/` — against the SAME
ground truth, the same population, and the same call sites, so the two answers are
comparable rather than merely adjacent.

**It does not re-implement the census.** It executes `metrics.py`'s own population code
(everything up to `def evaluate`) and uses the `sites` / `pop` it builds. Copying those
rules would let the two measurements drift apart silently, which is the exact failure
this benchmark exists to catch elsewhere.

The grep it simulates is the one an agent types: the identifier followed by an open
paren, across every `.rs` file under `crates/`. That is deliberately the NAIVE form —
the point is to measure the default behaviour, not a hand-tuned regex.

Self-check: CodeGraph's recall is recomputed here from the index with the census's own
hit rule. If it does not land on the census's number, this harness is wrong and says so.
"""

import re
import sqlite3
import sys
from collections import Counter, defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
CALL = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")
DEF = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")


def census_population(gt_path: str, repo: str) -> dict:
    """Run metrics.py's population half and hand back its namespace."""
    src = (HERE / "metrics.py").read_text()
    marker = "\ndef evaluate(db):"
    assert marker in src, "metrics.py no longer has `def evaluate` — update the cut marker"
    ns: dict = {"__name__": "census_prefix"}
    argv = sys.argv
    sys.argv = ["metrics.py", gt_path, repo]
    try:
        exec(compile(src[: src.index(marker)], "metrics.py[population]", "exec"), ns)
    finally:
        sys.argv = argv
    assert ns["sites"] and ns["pop"], "the census produced no sites"
    return ns


def grep_index(repo: str, roots=("crates",)) -> tuple[dict, dict]:
    """Every `name(` occurrence in the tree, with enough context to classify it.

    Returns (hits, lines): hits[name] -> {(file, line): kind}, and the raw source lines
    for the audit sample. `kind` is what an agent would have to work out BY READING the
    hit — grep itself reports all of them identically.
    """
    hits: dict = defaultdict(dict)
    lines: dict = {}
    for root in roots:
        for path in sorted(Path(repo, root).rglob("*.rs")):
            rel = str(path.relative_to(repo))
            text = path.read_text(encoding="utf-8", errors="replace")
            lines[rel] = text.split("\n")
            in_block = False
            for no, line in enumerate(lines[rel], 1):
                stripped = line.strip()
                if in_block:
                    if "*/" in line:
                        in_block = False
                    continue
                if stripped.startswith("/*"):
                    in_block = "*/" not in line
                    continue
                comment_at = line.find("//")
                for m in CALL.finditer(line):
                    name, at = m.group(1), m.start(1)
                    if name in ("fn", "if", "while", "for", "match", "return", "let"):
                        continue
                    if comment_at != -1 and at > comment_at:
                        kind = "comment / doc"
                    elif line.count('"', 0, at) % 2 == 1:
                        kind = "string literal"
                    elif DEF.search(line[:at]):
                        kind = "the definition itself"
                    else:
                        kind = "code"
                    hits[name][(rel, no)] = kind
    return hits, lines


def main() -> int:
    gt_path, repo = sys.argv[1], sys.argv[2]
    db_path = sys.argv[3] if len(sys.argv) > 3 else f"{repo}/.codegraph/codegraph.db"
    ns = census_population(gt_path, repo)
    sites, pop, unique, gt = ns["sites"], ns["pop"], ns["unique"], ns["gt"]

    # Every call-like site the lexer saw, valid or excluded — so a grep hit can be told
    # apart from "a real call, but not to the function you asked about".
    any_call = {(c["file"], c["line"], c["name"]) for c in gt["calls"]}
    valid = {(s["file"], s["line"], s["name"]) for s in sites}

    hits, lines = grep_index(repo)
    popset = set(pop)

    # ── recall: of the census's call sites, which would grep put in front of you? ──
    grep_found, grep_missed_kind = 0, Counter()
    for s in sites:
        if (s["file"], s["line"]) in hits.get(s["name"], {}):
            grep_found += 1
        else:
            src = lines.get(s["file"], [])
            line = src[s["line"] - 1] if 0 < s["line"] <= len(src) else ""
            if s.get("turbofish"):
                kind = "turbofish f::<T>() — the `(` is not next to the name"
            elif re.search(r"\b%s\s*$" % re.escape(s["name"]), line):
                kind = "the `(` is on the next line"
            elif s["name"] not in line:
                kind = "hit line not in the grep corpus (file outside crates/)"
            else:
                kind = "other"
            grep_missed_kind[kind] += 1

    # ── precision: of what grep hands you for these names, how much is the answer? ──
    tp = fp = 0
    noise = Counter()
    per_symbol_hits, per_symbol_tp = {}, {}
    for name in pop:
        h = hits.get(name, {})
        per_symbol_hits[name] = len(h)
        t = 0
        for (f, ln), kind in h.items():
            if (f, ln, name) in valid:
                t += 1
            else:
                fp += 1
                noise[kind if kind != "code" else (
                    "a real call — to a DIFFERENT function of that name"
                    if (f, ln, name) in any_call else "code, but not a call to it")] += 1
        per_symbol_tp[name] = t
        tp += t

    # ── CodeGraph on the identical sites, recomputed from the index ──
    cur = sqlite3.connect(db_path).cursor()
    callers: dict = {}
    for name in pop:
        t = unique[name]
        node = None
        for nid, fp_, sl in cur.execute(
            "select id, file_path, start_line from nodes where name=? and kind in ('function','method')",
            (name,),
        ):
            if fp_ == t["file"] and abs((sl or 0) - t["line"]) <= 3:
                node = nid
                break
        callers[name] = (
            set()
            if node is None
            else {
                (fp_, sl)
                for (fp_, sl) in cur.execute(
                    "select s.file_path, s.start_line from edges e join nodes s on s.id=e.source "
                    "where e.target=? and e.kind='calls'",
                    (node,),
                )
            }
        )
    cg_found = sum(1 for s in sites if (s["file"], s["encl"]) in callers[s["name"]])
    union = sum(
        1
        for s in sites
        if (s["file"], s["encl"]) in callers[s["name"]]
        or (s["file"], s["line"]) in hits.get(s["name"], {})
    )

    tot = len(sites)
    pct = lambda a, b: f"{100.0 * a / b:5.1f}%" if b else "    -"
    print(f"POPULATION  {len(pop)} unique-name functions · {tot} call sites "
          f"(the census's own; {len(any_call)} call-like sites in the tree)\n")
    print(f"{'':<44}{'grep':>12}{'CodeGraph':>12}{'both':>12}")
    print(f"{'call sites surfaced (recall)':<44}"
          f"{pct(grep_found, tot):>12}{pct(cg_found, tot):>12}{pct(union, tot):>12}")
    print(f"{'  of which you must still read to use':<44}{'all':>12}{'none':>12}{'':>12}")
    print(f"{'precision (a hit IS a call to that fn)':<44}"
          f"{pct(tp, tp + fp):>12}{'':>12}{'':>12}")
    print(f"\nWHAT GREP HANDS YOU for these {len(pop)} symbols: {tp + fp} hit lines, "
          f"{tp} of them the answer ({pct(tp, tp + fp)})")
    for kind, n in noise.most_common():
        print(f"  {n:>6}  {kind}")
    big = sorted(pop, key=lambda n: -per_symbol_hits[n])[:8]
    print("\n  worst symbols to grep for (hits you would read : real calls):")
    for n in big:
        print(f"    {per_symbol_hits[n]:>5} : {per_symbol_tp[n]:<5}  {n}")
    med = sorted(per_symbol_hits[n] for n in pop)[len(pop) // 2]
    print(f"  median hits per symbol: {med}")
    if grep_missed_kind:
        print("\nWHAT GREP MISSES (the naive pattern cannot match these):")
        for kind, n in grep_missed_kind.most_common():
            print(f"  {n:>6}  {kind}")
    print(f"\nSELF-CHECK  CodeGraph recall recomputed here: {pct(cg_found, tot)} — compare with "
          f"metrics.py's `ALL CALL SITES (recall)` line, NOT its count-matched one: this uses the "
          f"same set-membership rule. A wide gap means this harness is wrong, not the tool.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
