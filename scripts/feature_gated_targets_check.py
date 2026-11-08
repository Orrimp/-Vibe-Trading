#!/usr/bin/env -S uv run --quiet --python 3.11 --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Every cargo target behind `required-features` must be DECLARED, with its CI status.

bug-log `#163`. `cargo test` skips a target whose `required-features` are not
enabled **silently**: no warning, no note, exit 0. Measured on this repo:

    $ cargo test -p backtest --test threshold_sweep_readonly --no-run
    error: target `threshold_sweep_readonly` in package `backtest` requires the
           features: `candle`, `realdata`

That error appears only when the target is named explicitly. Under
`cargo test --workspace --exclude ui` — which is what CI runs
(`.github/workflows/ci.yml:202`) — the same target is passed over without a word.
So **41 test functions across 12 test targets have never run in CI**, and nothing
said so.

(First stated as "46 across 13". Corrected 2026-11-08 the same day: one of those
targets, `strategy/test/tcn_overlay_tuned_builder` (5 test fns), is gated on a
name that is a NON-OPTIONAL DEPENDENCY rather than a feature, which cargo
satisfies unconditionally — so it always builds. The `DEP-SATISFIED` check below
now detects that class, because the first version of this file asserted the
wrong thing about exactly the row it was built to describe.)

This gate does not decide which features CI should enable; that is a cost
decision for the operator. It makes the exclusion **declared** instead of
silent, so a new one cannot be added without a reviewer seeing it, and a stale
declaration cannot rot unnoticed. It fails when:

  * a target has `required-features` and is NOT in `DECLARED` below;
  * a `DECLARED` entry names a target that no longer exists;
  * a target's `required-features` no longer match what `DECLARED` records;
  * fewer gated targets are found than the floor (the parser stopped seeing rows).

Run `--self-test` to re-measure the gate's own teeth.

Usage:
    scripts/feature_gated_targets_check.py [--self-test] [--list]
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CRATES = REPO / "crates"
CI_YML = REPO / ".github" / "workflows" / "ci.yml"

# ── The declaration ───────────────────────────────────────────────────────────
#
# One row per cargo target carrying `required-features`.
#
#   key   = "<crate>/<kind>/<target-name>"
#   feats = the required-features set, EXACTLY as Cargo.toml lists it
#   ci    = "built"     — some CI invocation enables every one of `feats`
#           "invisible" — CI never builds it; `why` must say what that costs
#   why   = for "invisible", the reason the exclusion is acceptable (or the
#           bug-log number tracking it). Required; an empty string is a failure.
#
# `ci` is a CLAIM about CI, checked loosely below: an "invisible" row whose
# features all appear in ci.yml is flagged, because the declaration has gone
# stale in the direction that matters (we think it is excluded; it is not).
DECLARED: dict[str, dict[str, object]] = {
    # ── backtest ─────────────────────────────────────────────────────────────
    "backtest/bin/threshold_sweep": {
        "feats": ["candle", "realdata"],
        "ci": "invisible",
        "why": "bug-log #163. Its 6 bin-local unit tests (incl. the only t_classifier "
               "coverage) never run in CI. NOTE: candle-core compiles anyway — "
               "crates/backtest/Cargo.toml:63 depends on strategy with features=[\"forecast\"] "
               "unconditionally, which pulls forecast/candle — so CI pays the ML compile "
               "cost and gets none of the ML-gated tests.",
    },
    "backtest/test/threshold_sweep_readonly": {
        "feats": ["candle", "realdata"],
        "ci": "invisible",
        "why": "bug-log #163 — 2 test fns, 0 ignored.",
    },
    "backtest/bin/run_yahoo_sma": {
        "feats": ["yahoo"],
        "ci": "invisible",
        "why": "bug-log #163. CI enables `yahoo` only for the single named target "
               "reproducibility_sample_figure (ci.yml:223), not for the package.",
    },
    "backtest/test/run_yahoo_sma_ticker_flag": {
        "feats": ["yahoo"],
        "ci": "invisible",
        "why": "bug-log #163 — 6 test fns, 0 ignored. Same single-target `yahoo` scope.",
    },
    "backtest/test/p2_verdict_rerun": {
        "feats": ["realdata", "yahoo"],
        "ci": "invisible",
        "why": "bug-log #163 — 15 test fns, 7 already #[ignore]. `realdata` additionally "
               "needs data/binance/REVISION.toml at runtime, which CI does not provision.",
    },
    "backtest/example/passive_baseline_equity": {
        "feats": ["realdata"],
        "ci": "invisible",
        "why": "An example, not a gate; carries no assertions. Listed so the inventory "
               "is complete rather than filtered.",
    },
    # ── data ─────────────────────────────────────────────────────────────────
    "data/bin/fetch_yahoo_klines": {
        "feats": ["yahoo-online"],
        "ci": "invisible",
        "why": "Network fetcher. Deliberately off in CI — no outbound calls from the "
               "matrix. Carries no test fns.",
    },
    # ── forecast (candle = the ML backend) ───────────────────────────────────
    "forecast/bin/train_tcn": {
        "feats": ["candle"], "ci": "invisible", "why": "bug-log #163 — training bin.",
    },
    "forecast/bin/forecast_distribution": {
        "feats": ["candle"], "ci": "invisible",
        "why": "bug-log #163. Also the one verdict classifier with NO bin-local test "
               "for `classify` (bug-log #154), and its target is skipped outright.",
    },
    "forecast/bin/recalibrate_sigma_train": {
        "feats": ["candle"], "ci": "invisible", "why": "bug-log #163 — overlay writer.",
    },
    "forecast/bin/train_patchtst": {
        "feats": ["candle"], "ci": "invisible", "why": "bug-log #163 — training bin.",
    },
    "forecast/test/metal_cpu_drift": {
        "feats": ["candle"], "ci": "invisible", "why": "bug-log #163 — 2 test fns, 0 ignored.",
    },
    "forecast/test/train_tcn_golden_cli": {
        "feats": ["candle"], "ci": "invisible", "why": "bug-log #163 — 3 test fns, 0 ignored.",
    },
    "forecast/test/train_tcn_audit_emits": {
        "feats": ["candle"], "ci": "invisible", "why": "bug-log #163 — 2 test fns, 0 ignored.",
    },
    "forecast/test/train_tcn_no_audit_db_writes_nothing": {
        "feats": ["candle"], "ci": "invisible", "why": "bug-log #163 — 1 test fn, 0 ignored.",
    },
    "forecast/test/recalibrate_sigma_train_readonly": {
        "feats": ["candle"], "ci": "invisible",
        "why": "bug-log #163 — 4 test fns, 1 ignored. One of the two gates named in "
               "ADR-0035's field-invariance invariant (sweep finding F4).",
    },
    "forecast/test/sigma_train_not_in_safetensors": {
        "feats": ["candle"], "ci": "invisible", "why": "bug-log #163 — 1 test fn, 0 ignored.",
    },
    "forecast/test/sigma_train_not_in_safetensors_patchtst": {
        "feats": ["candle"], "ci": "invisible", "why": "bug-log #163 — 2 test fns, 1 ignored.",
    },
    "forecast/test/forward_determinism_patchtst": {
        "feats": ["candle"], "ci": "invisible", "why": "bug-log #163 — 2 test fns, 0 ignored.",
    },
    "forecast/test/patchtst_overlay_neutrality": {
        "feats": ["candle"], "ci": "invisible", "why": "bug-log #163 — 1 test fn, 1 ignored.",
    },
    # ── strategy ─────────────────────────────────────────────────────────────
    "strategy/test/tcn_overlay_tuned_builder": {
        "feats": ["forecast"], "ci": "built",
        "why": "CORRECTED 2026-11-08: first declared `invisible` with '5 test fns, 0 ignored', "
               "and that was WRONG. `required-features = [\"forecast\"]` names strategy's "
               "non-optional DEPENDENCY `forecast` (crates/strategy/Cargo.toml:21), not the "
               "same-named feature at :11 — and cargo satisfies a required-feature that names "
               "a non-optional dependency UNCONDITIONALLY. Measured: `cargo test -p strategy "
               "--test tcn_overlay_tuned_builder --no-run` finishes clean, where the same "
               "command for a genuinely gated target errors. So this target always builds, and "
               "the gate below now detects the whole class rather than trusting this row.",
    },
    # ── ui (excluded from the workspace run; has its own matrix legs) ────────
    "ui/bin/cockpit": {
        "feats": ["fixtures"], "ci": "built",
        "why": "ci.yml:144 `cargo build -p ui --bin cockpit --features fixtures`, and the "
               "linux/windows legs run `cargo test -p ui --features fixtures`.",
    },
    "ui/bin/ui-gallery": {
        "feats": ["fixtures"], "ci": "built",
        "why": "Same `--features fixtures` legs as cockpit.",
    },
    "ui/bin/cockpit_live": {
        "feats": ["live"], "ci": "built",
        "why": "`live` is in ui's own default feature set (crates/ui/Cargo.toml:281).",
    },
    "ui/bench/chart_build_probe": {
        "feats": ["chart-build-probe"], "ci": "invisible",
        "why": "A criterion bench, not a gate; benches are never run by `cargo test` at "
               "all. Listed so the inventory is complete rather than filtered. NOTE: the "
               "sibling bench `cockpit_render` carries NO required-features \u2014 an earlier "
               "hand enumeration of this inventory got that wrong, which is why the floor "
               "below is a measured count and not a remembered one.",
    },
}

# Non-vacuity floor: the parser must find at least this many gated targets.
# Raise it deliberately when a target is added; a DROP means the parser broke.
MIN_EXPECTED_GATED = 25

TARGET_BLOCK_RE = re.compile(
    r"\[\[(bin|test|bench|example)\]\](.*?)(?=\n\[\[|\n\[[a-z]|\Z)", re.DOTALL
)
NAME_RE = re.compile(r'^\s*name\s*=\s*"([^"]+)"', re.MULTILINE)
REQ_RE = re.compile(r'^\s*required-features\s*=\s*\[([^\]]*)\]', re.MULTILINE)


DEP_INLINE_RE = re.compile(r"^\s*([A-Za-z0-9_\-]+)\s*=", re.MULTILINE)
DEP_TABLE_RE = re.compile(r"^\[(?:dev-|build-)?dependencies\.([A-Za-z0-9_\-]+)\]", re.MULTILINE)
DEP_SECTION_RE = re.compile(
    r"^\[(?:dev-|build-)?dependencies\]\n(.*?)(?=\n\[|\Z)", re.MULTILINE | re.DOTALL
)


def non_optional_deps(text: str) -> set[str]:
    """Dependency names that are NOT optional, so cargo always has them.

    Why this matters (bug-log `#163`, corrected 2026-11-08): cargo satisfies a
    `required-features` entry that names a **non-optional dependency**
    unconditionally. An OPTIONAL dependency creates an implicit feature of the
    same name, so gating on it is a real gate; a non-optional one is not. A target
    gated only on such a name therefore always builds, and declaring it
    `invisible` is simply false — which is exactly what the first version of this
    file did for `strategy/test/tcn_overlay_tuned_builder`.
    """
    names: set[str] = set()
    for m in DEP_SECTION_RE.finditer(text):
        names.update(DEP_INLINE_RE.findall(m.group(1)))
    names.update(DEP_TABLE_RE.findall(text))

    optional: set[str] = set()
    for name in names:
        inline = re.search(
            r"^\s*" + re.escape(name) + r"\s*=\s*\{[^}]*optional\s*=\s*true",
            text,
            re.MULTILINE | re.DOTALL,
        )
        table = re.search(
            r"^\[(?:dev-|build-)?dependencies\."
            + re.escape(name)
            + r"\]\n(?:(?!\n\[).*\n)*?\s*optional\s*=\s*true",
            text,
            re.MULTILINE,
        )
        if inline or table:
            optional.add(name)
    return names - optional


def scan_dep_satisfied() -> dict[str, list[str]]:
    """`<crate>/<kind>/<name>` -> required features that a non-optional dep satisfies."""
    out: dict[str, list[str]] = {}
    for manifest in sorted(CRATES.glob("*/Cargo.toml")):
        crate = manifest.parent.name
        text = manifest.read_text(encoding="utf-8")
        deps = non_optional_deps(text)
        for m in TARGET_BLOCK_RE.finditer(text):
            kind, body = m.group(1), m.group(2)
            name_m, req_m = NAME_RE.search(body), REQ_RE.search(body)
            if not (name_m and req_m):
                continue
            feats = [f.strip().strip('"') for f in req_m.group(1).split(",") if f.strip()]
            satisfied = [f for f in feats if f in deps]
            if satisfied:
                out[f"{crate}/{kind}/{name_m.group(1)}"] = satisfied
    return out


def scan() -> dict[str, list[str]]:
    """Every `<crate>/<kind>/<name>` carrying required-features, with that set."""
    found: dict[str, list[str]] = {}
    for manifest in sorted(CRATES.glob("*/Cargo.toml")):
        crate = manifest.parent.name
        text = manifest.read_text(encoding="utf-8")
        for m in TARGET_BLOCK_RE.finditer(text):
            kind, body = m.group(1), m.group(2)
            name_m, req_m = NAME_RE.search(body), REQ_RE.search(body)
            if not (name_m and req_m):
                continue
            feats = [f.strip().strip('"') for f in req_m.group(1).split(",") if f.strip()]
            found[f"{crate}/{kind}/{name_m.group(1)}"] = feats
    return found


def ci_text() -> str:
    return CI_YML.read_text(encoding="utf-8") if CI_YML.is_file() else ""


_NARROW_RE = re.compile(r"--(test|bin|bench|example)\s+(\S+)")


def _any_ci_invocation_would_build(
    ci: str, crate: str, kind: str, name: str, feats: list[str]
) -> bool:
    """Does some single ci.yml command line build `crate`'s `name` target?

    Per LINE, because features are scoped to one invocation. A line that narrows
    with `--test X` / `--bin X` builds only X, so it counts only when X is this
    target. `--workspace` selects every package unless `--exclude`d.
    """
    for line in ci.splitlines():
        if "cargo " not in line:
            continue
        selects = f"-p {crate}" in line or (
            "--workspace" in line and f"--exclude {crate}" not in line
        )
        if not selects:
            continue
        if not all(f in line for f in feats):
            continue
        narrowings = _NARROW_RE.findall(line)
        if narrowings and not any(k == kind and n == name for k, n in narrowings):
            continue
        return True
    return False


def check(found: dict[str, list[str]], ci: str) -> list[str]:
    violations: list[str] = []

    for key in sorted(found):
        if key not in DECLARED:
            violations.append(
                f"UNDECLARED  {key} requires {found[key]} but is not in DECLARED.\n"
                f"            A feature-gated target is skipped SILENTLY by `cargo test`.\n"
                f"            Add a row saying whether CI builds it, and if not, what that costs."
            )
    for key in sorted(DECLARED):
        if key not in found:
            violations.append(
                f"STALE       {key} is DECLARED but no longer carries required-features\n"
                f"            (target renamed, removed, or ungated). Drop or update the row."
            )
            continue
        row = DECLARED[key]
        want, got = list(row["feats"]), found[key]
        if want != got:
            violations.append(
                f"DRIFT       {key} requires {got}, DECLARED says {want}.\n"
                f"            The gate's CI claim was reasoned about a different feature set."
            )
        if row["ci"] not in ("built", "invisible"):
            violations.append(f"BAD-ci      {key} has ci={row['ci']!r}; expected built|invisible.")
        if not str(row["why"]).strip():
            violations.append(
                f"NO-WHY      {key} is declared without a reason. An exclusion with no\n"
                f"            stated cost is the silence this gate exists to prevent."
            )
        # Staleness check in the direction that matters: we believe it is excluded,
        # but some CI invocation looks like it would build it. Checked PER LINE and
        # narrowing-aware, because `cargo test -p backtest --features yahoo --test
        # reproducibility_sample_figure` enables `yahoo` for exactly ONE target —
        # a whole-file substring search reads that as "the package has yahoo on"
        # and flags two targets it does not build. (First version of this check did
        # exactly that; the gate caught it on its own first run.)
        if row["ci"] == "invisible":
            crate, kind, name = key.split("/", 2)
            if _any_ci_invocation_would_build(ci, crate, kind, name, got):
                violations.append(
                    f"MAYBE-BUILT {key} is declared `invisible`, but a ci.yml invocation\n"
                    f"            selects package `{crate}` with every feature in {got} and\n"
                    f"            does not narrow to another target. Re-measure: if CI does\n"
                    f"            build it, flip the row to `built` and say where."
                )
    # A required-feature naming a NON-OPTIONAL dependency is satisfied
    # unconditionally, so the target is not gated at all and must not be declared
    # invisible. See non_optional_deps() for how this was found.
    for key, names in sorted(scan_dep_satisfied().items()):
        row = DECLARED.get(key)
        if row is not None and row["ci"] == "invisible":
            violations.append(
                f"DEP-SATISFIED {key} is declared `invisible`, but its required\n"
                f"            feature(s) {names} name NON-OPTIONAL dependencies, which cargo\n"
                f"            satisfies unconditionally — so this target ALWAYS builds.\n"
                f"            Declare it `built`. (An OPTIONAL dependency would create an\n"
                f"            implicit feature of the same name and would be a real gate.)"
            )

    if len(found) < MIN_EXPECTED_GATED:
        violations.append(
            f"FLOOR       found {len(found)} gated target(s), floor is {MIN_EXPECTED_GATED}.\n"
            f"            Either targets were ungated (raise the floor deliberately) or the\n"
            f"            manifest parser stopped matching."
        )
    return violations


def self_test() -> int:
    """Probe the gate. Each perturbation must produce the violation KIND it targets.

    Matching on the kind, not merely on "some new violation", because a probe
    that fires for the wrong reason does not test what it claims. The floor probe
    taught that on this gate's first run: feeding it one undeclared row made
    UNDECLARED fire, and the FLOOR branch it was meant to exercise was never
    reached while the probe still read as OK.
    """
    print("feature-gated-targets --self-test")
    base_found = scan()
    base_ci = ci_text()
    clean = check(base_found, base_ci)

    probes: list[tuple[str, str, list[str]]] = []

    # P1 — an undeclared gated target appears.
    f = dict(base_found)
    f["newcrate/test/sneaky_gate"] = ["candle"]
    probes.append(("undeclared target", "UNDECLARED", check(f, base_ci)))

    # P2 — a declared target vanishes (rename / removal / ungating).
    f = dict(base_found)
    f.pop("forecast/test/metal_cpu_drift")
    probes.append(("declared target gone", "STALE", check(f, base_ci)))

    # P3 — required-features drift under a declared row.
    f = dict(base_found)
    f["forecast/test/metal_cpu_drift"] = ["candle", "metal"]
    probes.append(("required-features drift", "DRIFT", check(f, base_ci)))

    # P4 — the floor, ISOLATED: every surviving row is still declared and the
    # removed ones are tolerated, so only the count can be what fires. Built by
    # taking the declared set and dropping rows, then asking for the FLOOR line
    # specifically rather than for any violation.
    shrunk = {k: v for k, v in list(base_found.items())[:3]}
    probes.append(("floor", "FLOOR", check(shrunk, base_ci)))

    # P5 — a row claims `invisible` while CI would build it.
    fake_ci = base_ci + (
        "\n        run: cargo test -p forecast --features candle --no-fail-fast\n"
    )
    probes.append(("invisible row that CI would build", "MAYBE-BUILT", check(base_found, fake_ci)))

    # P7 — a dep-satisfied target wrongly declared invisible. Flip the one row
    # that IS dep-satisfied back to `invisible` and require the new kind.
    saved_ci = DECLARED["strategy/test/tcn_overlay_tuned_builder"]["ci"]
    DECLARED["strategy/test/tcn_overlay_tuned_builder"]["ci"] = "invisible"
    probes.append(
        ("dep-satisfied row declared invisible", "DEP-SATISFIED", check(base_found, base_ci))
    )
    DECLARED["strategy/test/tcn_overlay_tuned_builder"]["ci"] = saved_ci

    # P6 — an exclusion declared with no stated cost.
    saved = DECLARED["forecast/test/metal_cpu_drift"]["why"]
    DECLARED["forecast/test/metal_cpu_drift"]["why"] = "   "
    probes.append(("exclusion with no reason", "NO-WHY", check(base_found, base_ci)))
    DECLARED["forecast/test/metal_cpu_drift"]["why"] = saved

    rc = 0
    for label, kind, found_violations in probes:
        extra = [x for x in found_violations if x not in clean]
        hit = [x for x in extra if x.startswith(kind)]
        if hit:
            print(f"  OK    {kind:11} fires on: {label}")
        elif extra:
            print(
                f"  FAIL  {kind:11} did NOT fire on: {label} \u2014 the probe produced "
                f"{len(extra)} other violation(s) instead, starting "
                f"{extra[0].split()[0]!r}, so the branch under test was never reached"
            )
            rc = 1
        else:
            print(f"  FAIL  {kind:11} INERT on: {label} \u2014 no new violation at all")
            rc = 1

    if clean:
        print(f"  note  the live tree has {len(clean)} violation(s); see a plain run")
    else:
        print("  OK    control: the live tree is clean")
    print(
        "feature-gated-targets: SELF-TEST PASS"
        if rc == 0
        else "feature-gated-targets: SELF-TEST FAIL"
    )
    return rc


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        return self_test()

    found = scan()
    violations = check(found, ci_text())
    invisible = [k for k, r in DECLARED.items() if r["ci"] == "invisible" and k in found]
    built = [k for k, r in DECLARED.items() if r["ci"] == "built" and k in found]

    if "--list" in argv:
        for key in sorted(found):
            row = DECLARED.get(key)
            status = row["ci"] if row else "UNDECLARED"
            print(f"  {status:11} {key:52} {found[key]}")

    print(
        f"feature-gated targets: {len(found)} scanned "
        f"[floor {MIN_EXPECTED_GATED}] · {len(built)} built by CI · "
        f"{len(invisible)} invisible to CI"
    )
    if violations:
        for v in violations:
            print(v)
        print(f"feature-gated-targets: FAIL ({len(violations)} violation(s))")
        return 1
    print("feature-gated-targets: PASS (0 violations)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
