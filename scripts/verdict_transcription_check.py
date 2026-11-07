#!/usr/bin/env -S uv run --quiet --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Enforce the word "verbatim" in the three verdict-transcription test files (bug-log `#154`).

## What this gate exists for

Three binaries each carry a verdict classifier. None of them is reachable from an
integration test, because a `[[bin]]` is not a library — so each is "tested" by a file
that re-declares the types and re-implements the algorithm, then tests its own copy:

    F1-F4    forecast/src/bin/forecast_distribution.rs  <-  tests/forecast_distribution_verdict.rs
    V1-V5    forecast/src/bin/vol_verdict.rs            <-  tests/vol_verdict_mutual_exclusivity.rs
    V-REG1-5 forecast/src/bin/regime_verdict.rs         <-  tests/vreg_mutual_exclusivity.rs

That is a decided practice, not an accident: ADR-0033 D3.d established it, ADR-0038 D1.b
and ADR-0049 D4 each cite the one before, and the justification is real — inlining keeps
the tests free of the `candle` / `realdata` feature gates. All three files state that the
copy is *verbatim*.

Nothing enforced that word. Edit a shipped threshold and all fifteen fixtures plus three
100-sample mutual-exclusivity properties keep passing, green, forever. Measured on
2026-11-07 the copies were predicate-faithful, but two of four functions had already
drifted in code (dropped fields, an inlined method, a renamed local, omitted evidence
construction) with nothing to say so. This gate is that something.

## What it checks, and what it deliberately does not

**Branch fidelity** (the three classifiers). Every `if` condition in statement position is
extracted from both files -- comments and string literals stripped, local `let` bindings of
simple expressions substituted so a named threshold and its literal compare equal -- and the
two sets must match EACH OTHER *and* a set declared in this file. Declaring the expected set
is the point: it means a threshold change cannot be waved through on either side alone, and
it means a parser that silently matches nothing fails against a non-empty expectation. That
floor is not optional here -- bug-log `#142` was exactly a source-reading gate whose
four-line window matched nothing for fifteen days while reporting success.

**Input fidelity** (`compute_aggregate`). This function has ZERO branches, so a condition
check over it would compare the empty set to the empty set and report PASS -- which is the
precise failure this gate was written about, so it is not done that way. Instead: every
`agg.<field>` the shipped classifier reads must exist on BOTH `AggregateStats` declarations.
That catches the real risk (the copy's struct losing a field a predicate consumes) without
demanding textual identity of arithmetic the copy legitimately inlines.

**NOT checked**: that the two implementations compute the same *numbers*. The copy may
inline a method, rename a local, or omit the evidence `String` -- all three happen today and
all three are harmless. Only the branch conditions and the predicate's input surface are
pinned. A copy could still diverge in a way this gate does not see; what it cannot do any
more is diverge in a threshold or a dropped predicate input in silence.

Run: `uv run scripts/verdict_transcription_check.py` (add `--self-test` to probe the rules).
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent

# ── source extraction ─────────────────────────────────────────────────────────


def extract_fn(text: str, sig: str) -> str:
    """Return the first `fn` whose signature line contains `sig`, brace-matched by indent."""
    lines = text.split("\n")
    hits = [i for i, l in enumerate(lines) if sig in l and "fn " in l]
    if not hits:
        raise LookupError(f"signature not found: {sig!r}")
    start = hits[0]
    indent = len(lines[start]) - len(lines[start].lstrip())
    out = [lines[start]]
    for line in lines[start + 1 :]:
        out.append(line)
        if line.strip() == "}" and (len(line) - len(line.lstrip())) == indent:
            return "\n".join(out)
    raise LookupError(f"unterminated fn: {sig!r}")


def strip_string_literals(src: str) -> str:
    """Replace every `"..."` with `"S"`, honouring backslash escapes.

    Done BEFORE comment stripping so a `//` inside a literal cannot eat the rest of a line.
    """
    out: list[str] = []
    i, n = 0, len(src)
    while i < n:
        if src[i] == '"':
            out.append('"S"')
            i += 1
            while i < n:
                if src[i] == "\\":
                    i += 2
                    continue
                if src[i] == '"':
                    i += 1
                    break
                i += 1
        else:
            out.append(src[i])
            i += 1
    return "".join(out)


def normalize(body: str) -> str:
    body = strip_string_literals(body)
    body = re.sub(r"//[^\n]*", " ", body)
    return re.sub(r"\s+", " ", body).strip()


_SIMPLE_RHS = re.compile(r"^[A-Za-z0-9_.\s()<>=!&|*/+\-%\[\]]+$")
_LET = re.compile(
    r"\blet (?:mut )?([a-z_][a-z0-9_]*)(?:\s*:\s*[A-Za-z0-9_:<>]+)?\s*=\s*([^;]{1,200});"
)


def resolve_lets(src: str, passes: int = 5) -> str:
    """Substitute simple local bindings into later text.

    So that a shipped `let trivial_threshold = 0.95; if rate > trivial_threshold` and a
    copy's `if rate > 0.95` compare equal, and so that a predicate hidden behind
    `let calibration_anomaly = ...; if calibration_anomaly` is actually compared.
    Skips anything with `format!`, an iterator chain, or a string literal -- those are the
    evidence layer, which this gate does not pin.
    """
    for _ in range(passes):
        changed = False
        for m in list(_LET.finditer(src)):
            name, rhs = m.group(1), m.group(2).strip()
            if "format!" in rhs or ".iter()" in rhs or '"S"' in rhs:
                continue
            if not _SIMPLE_RHS.match(rhs):
                continue
            if re.search(r"\b" + re.escape(name) + r"\b", rhs):
                continue  # self-referential; leave it alone
            tail = re.sub(r"(?<![\w.])" + re.escape(name) + r"\b", "(" + rhs + ")", src[m.end() :])
            cand = src[: m.end()] + tail
            if cand != src:
                src, changed = cand, True
        if not changed:
            break
    return src


# Strips redundant parens a `let` substitution leaves around a literal (`>(0.95)` -> `>0.95`),
# but NOT a call's argument list: the negative lookbehind keeps `max(1e-12)` intact. Without it
# the normaliser silently mangled `.max(1e-12)` into `.max1e-12` on BOTH sides, which compared
# equal and so would have passed had the expected set not been declared independently.
_BARE_PAREN_NUM = re.compile(r"(?<![\w])\((-?\d+(?:\.\d+)?(?:[eE]-?\d+)?)\)")


def tidy(cond: str) -> str:
    cond = re.sub(r"\s+", "", cond)
    while True:
        nxt = _BARE_PAREN_NUM.sub(r"\1", cond)
        if nxt == cond:
            return cond
        cond = nxt


def conditions(src: str) -> list[str]:
    """Every statement-position `if` condition, paren-aware up to its opening brace.

    Expression-position `if`s (`let x = if cond {...}`) are skipped: both V-REG copies
    differ there only because the copy omits the evidence `String` those branches build.
    """
    res: list[str] = []
    i, n = 0, len(src)
    while True:
        k = src.find("if ", i)
        if k < 0:
            return res
        i = k + 3
        if k > 0 and (src[k - 1].isalnum() or src[k - 1] == "_"):
            continue  # part of an identifier
        before = src[max(0, k - 2) : k].strip()
        if before.endswith(("=", "&", "|")):
            continue  # expression position
        depth, j = 0, i
        while j < n:
            ch = src[j]
            if ch in "([":
                depth += 1
            elif ch in ")]":
                depth -= 1
            elif ch == "{" and depth == 0:
                break
            j += 1
        cond = tidy(src[i:j])
        if cond:
            res.append(cond)


def struct_fields(text: str, name: str) -> set[str]:
    m = re.search(r"struct\s+" + re.escape(name) + r"\s*\{(.*?)\n\}", text, re.S)
    if not m:
        raise LookupError(f"struct not found: {name}")
    return set(re.findall(r"^\s*(?:pub )?([a-z_][a-z0-9_]*)\s*:", m.group(1), re.M))


# ── the declared contract ─────────────────────────────────────────────────────

BRANCH_PAIRS = [
    {
        "label": "F1-F4 (forecaster checkpoint, ADR-0033 D3)",
        "shipped": ("crates/forecast/src/bin/forecast_distribution.rs", "fn classify(s: &CheckpointStats)"),
        "copy": ("crates/forecast/tests/forecast_distribution_verdict.rs", "fn classify(s: &CheckpointStats)"),
        "expect": {
            "(s.abs_p95asf64)<1e-6",
            "s.std>0.1*s.sigma_train&&(s.frac_passes_confidence_gateasf64)<1e-6",
            "s.confidence_gate_survival[5]>=1e-4&&s.frac_inside_epsilon>0.5",
        },
    },
    {
        "label": "V1-V5 (vol forecaster, ADR-0038 D1.b)",
        "shipped": ("crates/forecast/src/bin/vol_verdict.rs", "fn classify_verdict(agg:"),
        "copy": ("crates/forecast/tests/vol_verdict_mutual_exclusivity.rs", "fn classify_verdict(agg:"),
        "expect": {
            "per_symbol.iter().all(|s|s.std_sigma_hat/s.mean_sigma_hat.max(1e-12)<1e-3)",
            "agg.qlike_dispersion>3.0",
            "agg.mean_calibration_ratio<0.7||agg.mean_calibration_ratio>1.4",
            "agg.n_symbols_improving<7",
        },
    },
    {
        "label": "V-REG-1..5 (regime dispatcher, ADR-0049 D4)",
        "shipped": ("crates/forecast/src/bin/regime_verdict.rs", "fn classify_vreg("),
        "copy": ("crates/forecast/tests/vreg_mutual_exclusivity.rs", "fn classify_vreg("),
        "expect": {
            "!stats.completed_ok",
            "suppress_rate>0.95||momentum_rate>0.95",
            "(stats.estimated_switches_per_week_upper_bound())>20.0",
            "(stats.final_equity<0.0||stats.total_return<-0.90||(suppress_rate<0.01&&stats.total_bars>1000))",
        },
    },
]

# Input fidelity: `compute_aggregate` has no branches, so it gets the struct-surface check.
INPUT_PAIR = {
    "label": "V AggregateStats surface (compute_aggregate has zero branches)",
    "shipped_src": "crates/forecast/src/bin/vol_verdict.rs",
    "copy_src": "crates/forecast/tests/vol_verdict_mutual_exclusivity.rs",
    "classifier_sig": "fn classify_verdict(agg:",
    "struct_name": "AggregateStats",
    # Every field the shipped classifier reads as `agg.<field>`. Declared, so a regex that
    # finds nothing fails here instead of reporting an empty intersection as success.
    "expect_reads": {
        "qlike_dispersion",
        "qlike_garch_max",
        "qlike_garch_min",
        "mean_calibration_ratio",
        "n_symbols_improving",
    },
}

# ── the check ─────────────────────────────────────────────────────────────────


def read(rel: str) -> str:
    return (ROOT / rel).read_text(encoding="utf-8")


def check(verbose: bool = True) -> list[str]:
    violations: list[str] = []

    def say(msg: str) -> None:
        if verbose:
            print(msg)

    for pair in BRANCH_PAIRS:
        label = pair["label"]
        sp, ss = pair["shipped"]
        cp, cs = pair["copy"]
        try:
            a = set(conditions(resolve_lets(normalize(extract_fn(read(sp), ss)))))
            b = set(conditions(resolve_lets(normalize(extract_fn(read(cp), cs)))))
        except LookupError as exc:
            violations.append(f"verdict-transcription[{label}]: cannot read sources - {exc}")
            continue
        want = set(pair["expect"])
        if not want:
            violations.append(f"verdict-transcription[{label}]: declared set is EMPTY - the gate would be vacuous")
            continue
        if a != want:
            violations.append(
                f"verdict-transcription[{label}]: SHIPPED branch set moved ({sp})\n"
                f"    gained: {sorted(a - want) or '-'}\n    lost  : {sorted(want - a) or '-'}\n"
                f"    If this change is intended, update BOTH the copy in {cp} and the declared\n"
                f"    set in this script, in the same commit."
            )
        if b != want:
            violations.append(
                f"verdict-transcription[{label}]: COPY branch set moved ({cp})\n"
                f"    gained: {sorted(b - want) or '-'}\n    lost  : {sorted(want - b) or '-'}"
            )
        if a == b == want:
            say(f"  OK  {label}: {len(want)} branch conditions identical in shipped + copy")

    p = INPUT_PAIR
    try:
        shipped_txt, copy_txt = read(p["shipped_src"]), read(p["copy_src"])
        clf = normalize(extract_fn(shipped_txt, p["classifier_sig"]))
        reads = set(re.findall(r"\bagg\.([a-z_][a-z0-9_]*)\b", clf))
        f_ship = struct_fields(shipped_txt, p["struct_name"])
        f_copy = struct_fields(copy_txt, p["struct_name"])
    except LookupError as exc:
        violations.append(f"verdict-transcription[{p['label']}]: cannot read sources - {exc}")
        return violations

    want_reads = set(p["expect_reads"])
    if reads != want_reads:
        violations.append(
            f"verdict-transcription[{p['label']}]: the shipped classifier's `agg.` reads moved\n"
            f"    gained: {sorted(reads - want_reads) or '-'}\n    lost  : {sorted(want_reads - reads) or '-'}"
        )
    missing_ship = want_reads - f_ship
    missing_copy = want_reads - f_copy
    if missing_ship:
        violations.append(
            f"verdict-transcription[{p['label']}]: shipped {p['struct_name']} lacks predicate inputs {sorted(missing_ship)}"
        )
    if missing_copy:
        violations.append(
            f"verdict-transcription[{p['label']}]: the COPY's {p['struct_name']} lacks predicate inputs "
            f"{sorted(missing_copy)} - those branches cannot be exercised by any fixture"
        )
    if not (missing_ship or missing_copy) and reads == want_reads:
        say(
            f"  OK  {p['label']}: {len(want_reads)} predicate inputs present on both "
            f"{p['struct_name']} declarations"
        )

    return violations


# ── self-test: probe the rules (spec_lint.py --self-test precedent) ───────────


def self_test() -> int:
    """Mutate copies of the real sources in memory and assert each mutation is CAUGHT.

    A gate nobody probed is the thing bug-log `#154`, `#153`, `#142` and `#66` are all
    about. These probes run on every CI pass, so the gate's own teeth are re-measured
    rather than remembered.
    """
    probes = [
        (
            "a shipped threshold is loosened",
            "crates/forecast/src/bin/regime_verdict.rs",
            "let trivial_threshold = 0.95;",
            "let trivial_threshold = 0.80;",
        ),
        (
            "a shipped predicate is dropped",
            "crates/forecast/src/bin/forecast_distribution.rs",
            "if s.confidence_gate_survival[5] >= 1e-4 && s.frac_inside_epsilon > 0.5 {",
            "if s.confidence_gate_survival[5] >= 1e-4 {",
        ),
        (
            "the COPY's threshold is loosened while shipped stays put",
            "crates/forecast/tests/vol_verdict_mutual_exclusivity.rs",
            "if agg.n_symbols_improving < 7 {",
            "if agg.n_symbols_improving < 3 {",
        ),
        (
            "the COPY's struct loses a predicate input",
            "crates/forecast/tests/vol_verdict_mutual_exclusivity.rs",
            "    n_symbols_improving: usize,",
            "    n_symbols_improving_renamed: usize,",
        ),
    ]

    real_read = globals()["read"]
    failures = 0
    for name, rel, old, new in probes:
        src = real_read(rel)
        if src.count(old) != 1:
            print(f"  SELF-TEST BROKEN  {name}: probe target not unique in {rel} ({src.count(old)} hits)")
            failures += 1
            continue
        patched = {rel: src.replace(old, new)}
        globals()["read"] = lambda r, _p=patched, _o=real_read: _p.get(r) or _o(r)
        try:
            v = check(verbose=False)
        finally:
            globals()["read"] = real_read
        if v:
            print(f"  CAUGHT  {name}")
        else:
            print(f"  MISSED  {name}  <-- the gate does not have the teeth it claims")
            failures += 1

    clean = check(verbose=False)
    if clean:
        print(f"  MISSED  the unmutated tree must be clean, got {len(clean)} violation(s)")
        failures += 1
    else:
        print("  CAUGHT  (control) the unmutated tree is clean")

    if failures:
        print(f"\nverdict-transcription --self-test: FAIL ({failures} probe(s) did not behave)")
        return 1
    print(f"\nverdict-transcription --self-test: PASS ({len(probes)} probes caught + 1 control)")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    print("verdict-transcription-check (bug-log #154)")
    violations = check()
    if violations:
        print()
        for v in violations:
            print(f"FAIL {v}")
        print(f"\nverdict-transcription: FAIL ({len(violations)} violation(s))")
        return 1
    print("\nverdict-transcription: PASS (0 violations)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
