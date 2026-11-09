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
So **41 test functions across 12 test targets had never run in CI**, and nothing
said so.

**Resolved in part the same day.** The operator approved enabling `candle`, and
ci.yml gained a `forecast candle-gated tests` step — `cargo test -p forecast
--features candle` — which recovered **18 test functions across 9 targets**
(measured: 33 targets, 191 passed, 0 failed, 18 ignored). **23 test functions in
3 targets remain invisible**, all of them needing `realdata` and/or `yahoo`:
`p2_verdict_rerun` (15, 7 already ignored), `run_yahoo_sma_ticker_flag` (6) and
`threshold_sweep_readonly` (2). Both need `realdata`. (This sentence used to end
"`realdata` wants `data/binance/REVISION.toml` provisioned in CI, which is a
separate operator decision" — **wrong, corrected 2026-11-09, bug-log `#186`**:
CI provisions the corpus already, and `polars` is non-optional in
data/forecast/reports so every build compiles it anyway. `realdata` adds NOTHING
to the dependency graph; both targets are measured green. See the rows below.)

**Narrowed again 2026-11-08 (bug-log `#184`).** `run_yahoo_sma_ticker_flag` needed
only `yahoo` — no `realdata`, so no provisioning decision — and the repro-sample
step already built this package with that feature. It now has its own step, so
**17 test functions in 2 targets remained invisible**, both needing `realdata`.

**Closed 2026-11-09 (bug-log `#186`).** Both are in CI, as ci.yml's `backtest
realdata-gated p2 verdict rerun` and `backtest realdata-gated threshold-sweep
read-only guard` steps: 10 live test functions recovered (8 passed/7 ignored +
2 passed), ~50 s of Ubuntu test time against the ~31 min of headroom that leg
has while windows paces the matrix. **No `realdata`-gated test function is
invisible to CI any more.**

The sentence above used to end "blocked on its `polars` compile cost, NOT on
corpus provisioning (bug-log `#186`)". The corpus half of it was corrected on
2026-11-09 and the `polars` half was left standing **in the same pass**, cited
to the entry that disproves it: `polars` is non-optional in data, forecast and
reports, so there was never a `polars` compile cost to be blocked on. That was
the EIGHTH copy of this claim in the repo, and the SECOND time a correction of
it carried the error forward one clause over. The `graph_noop_report` printed on
every run below exists so the next reader gets the fact from a measurement
instead of from prose like this.

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
        "why": "bug-log #163. Its 6 bin-local unit tests never run in CI. UPDATED "
               "2026-11-08: `t_classifier` was LIFTED out of this bin into "
               "`backtest::stats` (bug-log #170) precisely because its only coverage "
               "lived here, where CI cannot reach it — those tests now run under "
               "`cargo test -p backtest --lib`, and the report body's band prose is "
               "checked lib-side via `include_str!`, needing no features at all. The 6 "
               "remaining bin-local tests (gate-survivor parser + report stamp) are "
               "still invisible. CORRECTED 2026-11-09 (bug-log #186): this row used to "
               "add that reaching them \"would pull `polars` into the CI build, which is "
               "the operator's open row\". False. `polars` is a NON-OPTIONAL dependency of "
               "data, forecast and reports, so every build already compiles it and "
               "`realdata` adds nothing to the graph — the SAME mistake as the candle one "
               "recorded in the next clause, three lines away. NOTE: candle-core "
               "compiles anyway — "
               "crates/backtest/Cargo.toml:63 depends on strategy with features=[\"forecast\"] "
               "unconditionally, which pulls forecast/candle — so CI pays the ML compile "
               "cost and gets none of the ML-gated tests.",
    },
    "backtest/test/threshold_sweep_readonly": {
        "feats": ["candle", "realdata"],
        "ci": "built",
        "why": "bug-log #163 — 2 test fns, 0 ignored. WIRED 2026-11-09 (bug-log #186) as "
               "ci.yml `backtest realdata-gated threshold-sweep read-only guard`, Ubuntu "
               "only, reusing the build of the p2 step above it (same feature set). "
               "NEITHER filed blocker was real: the corpus has been tracked since "
               "2026-09-27, and `polars` is non-optional in data/forecast/reports. "
               "Measured 22.4 s, 2 passed / 0 failed under `--features "
               "realdata,candle,yahoo`. Both tests spawn a NESTED `cargo run --bin "
               "threshold_sweep`; all six checkpoint sentinels they read are tracked, so "
               "CI has what they need, and `sentinel_bytes` PANICS on an absent one "
               "(bug-log #135) rather than comparing None to None.",
    },
    "backtest/bin/run_yahoo_sma": {
        "feats": ["yahoo"],
        "ci": "invisible",
        "why": "bug-log #163. CI enables `yahoo` only for the single named target "
               "reproducibility_sample_figure (ci.yml:223), not for the package.",
    },
    "backtest/test/run_yahoo_sma_ticker_flag": {
        "feats": ["yahoo"],
        "ci": "built",
        "why": "bug-log #184 — ci.yml `backtest yahoo-gated ticker-flag tests`, "
               "Ubuntu only, sharing the `yahoo` build the repro-sample step already "
               "pays for. 6 test fns, 0 ignored (measured 6 passed / 0.80 s). Two of "
               "them re-run the run_yahoo_sma BINARY to witness anchors 69 and 70 — "
               "the rows #182 re-emitted — which no on-disk hash gate can do.",
    },
    "backtest/test/p2_verdict_rerun": {
        "feats": ["realdata", "yahoo"],
        "ci": "built",
        "why": "bug-log #163 — 15 test fns, 7 already #[ignore]. CORRECTED 2026-11-09 "
               "(bug-log #186). This row used to read \"`realdata` additionally needs "
               "data/binance/REVISION.toml at runtime, which CI does not provision\" — CI "
               "DOES provision it, and did when that was written. `git ls-files "
               "data/binance` is 241 paths (10 symbols x 24 months + REVISION.toml), "
               "committed 2026-09-27 by operator decision as plain `binary` (NOT lfs), and "
               "ci.yml checks out with `lfs: true`. Nor is `polars` a cost: it is a "
               "NON-OPTIONAL dep of data/forecast/reports, already in every build. "
               "WIRED 2026-11-09 as ci.yml `backtest realdata-gated p2 verdict rerun`, "
               "Ubuntu only: 21.2 s, 8 passed / 0 failed / 7 ignored under `--features "
               "realdata,candle,yahoo`. That step carries a PRECONDITION on the corpus "
               "(24 parquets, each >10 KB so an LFS pointer stub fails it) because the one "
               "disk-touching live test, s4_binance_2324_base_smoke, SKIPs and RETURNS on "
               "absence — a green step without it would have measured nothing. "
               "data/deribit-dvol and data/yahoo-macro hold only REVISION.toml on the dev "
               "box TOO, so the local run visits CI's data state rather than a richer one: "
               "v0.dvol_regime DROPPED, v0.macro_riskon warm-up-only, 24 candidates "
               "ranked, in both places.",
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
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg): `cargo test -p forecast --features candle --no-fail-fast`. Operator-approved after bug-log #163 measured that candle-core compiles on every default run anyway (crates/backtest/Cargo.toml:63 pulls forecast/candle unconditionally), so CI was paying for the backend and skipping its tests. A bin's own #[cfg(test)] module now builds with it.",
    },
    "forecast/bin/forecast_distribution": {
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg): `cargo test -p forecast --features candle --no-fail-fast`. Operator-approved after bug-log #163 measured that candle-core compiles on every default run anyway (crates/backtest/Cargo.toml:63 pulls forecast/candle unconditionally), so CI was paying for the backend and skipping its tests. A bin's own #[cfg(test)] module now builds with it.",
    },
    "forecast/bin/recalibrate_sigma_train": {
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg): `cargo test -p forecast --features candle --no-fail-fast`. Operator-approved after bug-log #163 measured that candle-core compiles on every default run anyway (crates/backtest/Cargo.toml:63 pulls forecast/candle unconditionally), so CI was paying for the backend and skipping its tests. A bin's own #[cfg(test)] module now builds with it.",
    },
    "forecast/bin/train_patchtst": {
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg): `cargo test -p forecast --features candle --no-fail-fast`. Operator-approved after bug-log #163 measured that candle-core compiles on every default run anyway (crates/backtest/Cargo.toml:63 pulls forecast/candle unconditionally), so CI was paying for the backend and skipping its tests. A bin's own #[cfg(test)] module now builds with it.",
    },
    "forecast/test/metal_cpu_drift": {
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg). MEASURED before enabling: 33 targets, 191 passed, 0 failed, 18 ignored (the pre-existing long/corpus-gated ones; the step does not pass --ignored). bug-log #163.",
    },
    "forecast/test/train_tcn_golden_cli": {
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg). MEASURED before enabling: 33 targets, 191 passed, 0 failed, 18 ignored (the pre-existing long/corpus-gated ones; the step does not pass --ignored). bug-log #163.",
    },
    "forecast/test/train_tcn_audit_emits": {
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg). MEASURED before enabling: 33 targets, 191 passed, 0 failed, 18 ignored (the pre-existing long/corpus-gated ones; the step does not pass --ignored). bug-log #163.",
    },
    "forecast/test/train_tcn_no_audit_db_writes_nothing": {
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg). MEASURED before enabling: 33 targets, 191 passed, 0 failed, 18 ignored (the pre-existing long/corpus-gated ones; the step does not pass --ignored). bug-log #163.",
    },
    "forecast/test/recalibrate_sigma_train_readonly": {
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg). MEASURED before enabling: 33 targets, 191 passed, 0 failed, 18 ignored (the pre-existing long/corpus-gated ones; the step does not pass --ignored). bug-log #163.",
    },
    "forecast/test/sigma_train_not_in_safetensors": {
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg). MEASURED before enabling: 33 targets, 191 passed, 0 failed, 18 ignored (the pre-existing long/corpus-gated ones; the step does not pass --ignored). bug-log #163.",
    },
    "forecast/test/sigma_train_not_in_safetensors_patchtst": {
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg). MEASURED before enabling: 33 targets, 191 passed, 0 failed, 18 ignored (the pre-existing long/corpus-gated ones; the step does not pass --ignored). bug-log #163.",
    },
    "forecast/test/forward_determinism_patchtst": {
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg). MEASURED before enabling: 33 targets, 191 passed, 0 failed, 18 ignored (the pre-existing long/corpus-gated ones; the step does not pass --ignored). bug-log #163.",
    },
    "forecast/test/patchtst_overlay_neutrality": {
        "feats": ["candle"], "ci": "built",
        "why": "Built since 2026-11-08 by the `forecast candle-gated tests` step (ci.yml, Ubuntu leg). MEASURED before enabling: 33 targets, 191 passed, 0 failed, 18 ignored (the pre-existing long/corpus-gated ones; the step does not pass --ignored). bug-log #163.",
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


_FEATURES_SECTION_RE = re.compile(r"^\[features\]\s*$(.*?)(?=^\[|\Z)", re.M | re.S)
_FEATURE_ENTRY_RE = re.compile(r'^\s*([A-Za-z0-9_-]+)\s*=\s*\[([^\]]*)\]', re.M)


def default_features(crate: str) -> set[str]:
    """`crate`'s default feature set, resolved TRANSITIVELY.

    bug-log #184. A target whose `required-features` are all defaults is not gated by
    any CI invocation — it builds under a plain `cargo test -p <crate>`. `ui/bin/
    cockpit_live` is the live example: its row claims `built` and is right, because
    `live` sits in `default = ["live", "yahoo", "binance"]` (promoted there 2026-05-25),
    and no ci.yml line names the feature at all.

    Transitive, not just the literal `default` list, because a default feature may enable
    another: taking the list at face value would flag such a target as an unbacked claim
    and FAIL a correct tree. Only bare names are followed — `dep:x` is a dependency and
    `other/x` is another crate's feature; neither enables a local one.
    """
    manifest = CRATES / crate / "Cargo.toml"
    if not manifest.is_file():
        return set()
    section = _FEATURES_SECTION_RE.search(manifest.read_text(encoding="utf-8"))
    if not section:
        return set()
    table = {
        m.group(1): [f.strip().strip('"') for f in m.group(2).split(",") if f.strip()]
        for m in _FEATURE_ENTRY_RE.finditer(section.group(1))
    }
    out: set[str] = set()
    queue = list(table.get("default", []))
    while queue:
        feat = queue.pop()
        if feat in out or ":" in feat or "/" in feat:
            continue
        out.add(feat)
        queue.extend(table.get(feat, []))
    return out


WORKSPACE_CRATES = {p.parent.name for p in CRATES.glob("*/Cargo.toml")}

NORMAL_DEP_SECTION_RE = re.compile(r"^\[dependencies\]\n(.*?)(?=\n\[|\Z)", re.M | re.S)
NORMAL_DEP_TABLE_RE = re.compile(r"^\[dependencies\.([A-Za-z0-9_\-]+)\]", re.M)
_FEAT_DEP_RE = re.compile(r"^dep:([A-Za-z0-9_\-]+)$")


def normal_non_optional_deps(text: str) -> set[str]:
    """Non-optional names from `[dependencies]` ONLY — not dev- or build-.

    Deliberately narrower than `non_optional_deps`. This feeds the build closure
    below, where a name wrongly counted as present would make the gate print a
    FALSE "already in the graph" — the exact failure bug-log `#186` is about. A
    dev-dependency of a *dependency* is not compiled, so it must not count.
    """
    names: set[str] = set()
    for m in NORMAL_DEP_SECTION_RE.finditer(text):
        names.update(DEP_INLINE_RE.findall(m.group(1)))
    names.update(NORMAL_DEP_TABLE_RE.findall(text))
    optional: set[str] = set()
    for name in names:
        inline = re.search(
            r"^\s*" + re.escape(name) + r"\s*=\s*\{[^}]*optional\s*=\s*true",
            text, re.MULTILINE | re.DOTALL,
        )
        table = re.search(
            r"^\[dependencies\." + re.escape(name) + r"\]\n(?:(?!\n\[).*\n)*?\s*optional\s*=\s*true",
            text, re.MULTILINE,
        )
        if inline or table:
            optional.add(name)
    return names - optional


def build_closure(crate: str, _crates: set[str] | None = None) -> set[str]:
    """Every dependency name cargo already compiles for a default build of `crate`.

    Walks non-optional `[dependencies]` transitively through WORKSPACE crates, the
    only manifests we can read. Third-party manifests are not followed, so the set
    is a **lower bound** — it can miss a dependency that is in fact already present,
    but it cannot invent one. That is the safe direction for the claim it supports.
    """
    crates = WORKSPACE_CRATES if _crates is None else _crates
    seen: set[str] = set()
    out: set[str] = set()
    queue = [crate]
    while queue:
        c = queue.pop()
        if c in seen:
            continue
        seen.add(c)
        manifest = CRATES / c / "Cargo.toml"
        if not manifest.is_file():
            continue
        deps = normal_non_optional_deps(manifest.read_text(encoding="utf-8"))
        out |= deps
        queue.extend(d for d in deps if d in crates)
    return out - {crate}


def feature_dep_additions(crate: str, feat: str) -> list[str]:
    """The `dep:X` names `feat` claims to add, following local bare features."""
    manifest = CRATES / crate / "Cargo.toml"
    if not manifest.is_file():
        return []
    section = _FEATURES_SECTION_RE.search(manifest.read_text(encoding="utf-8"))
    if not section:
        return []
    table = {
        m.group(1): [f.strip().strip('"') for f in m.group(2).split(",") if f.strip()]
        for m in _FEATURE_ENTRY_RE.finditer(section.group(1))
    }
    adds: list[str] = []
    seen: set[str] = set()
    queue = [feat]
    while queue:
        f = queue.pop()
        if f in seen:
            continue
        seen.add(f)
        for entry in table.get(f, []):
            m = _FEAT_DEP_RE.match(entry)
            if m:
                adds.append(m.group(1))
            elif ":" not in entry and "/" not in entry:
                queue.append(entry)
    return sorted(set(adds))


def graph_noop_report() -> list[tuple[str, str, list[str]]]:
    """(crate, feature, deps-already-present) for every DECLARED row's features.

    bug-log `#186`. `realdata = ["dep:toml", "dep:polars"]` LOOKS like it pulls
    polars into the build; polars is a non-optional dependency of `data`, which
    `backtest` depends on non-optionally, so every build compiles it already and
    the feature adds nothing to the graph. That false cost was copied into six
    places and justified keeping 10 green tests out of CI for six weeks, because
    the gate checked only that a reason EXISTED, never that it was TRUE.

    Printed on every run rather than raised as a violation: the fact is what was
    missing, and a line in the CI log is unwritable-over. Deciding whether a cost
    is worth paying stays a human call; discovering the cost is zero does not.
    """
    out: list[tuple[str, str, list[str]]] = []
    closures: dict[str, set[str]] = {}
    for key in sorted(DECLARED):
        crate = key.split("/")[0]
        if crate not in closures:
            closures[crate] = build_closure(crate)
        for feat in DECLARED[key]["feats"]:  # type: ignore[union-attr]
            already = [d for d in feature_dep_additions(crate, feat) if d in closures[crate]]
            if already and (crate, feat, already) not in out:
                out.append((crate, feat, already))
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
    dep_sat = scan_dep_satisfied()

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
        # bug-log #184 — and the OTHER direction, which this gate did not check. The
        # comment above says "the direction that matters", and for a row that wrongly
        # claims exclusion it is. But a row claiming `built` is a claim too, and nothing
        # held it to anything: delete the ci.yml step that backs it and the row goes on
        # saying CI covers 6 tests that no longer run — the exact silence this file
        # exists to break, pointing the other way.
        #
        # Exempt: a target is not gated by CI at all when its features are satisfied
        # unconditionally — by a non-optional dependency (DEP-SATISFIED above) or by
        # sitting in the crate's default set. Re-measured 2026-11-09 across the 20
        # `built` rows (18 on 2026-11-08, before the two #186 rows flipped): 18 are
        # matched by an invocation, `ui/bin/cockpit_live` is default-satisfied and
        # `strategy/test/tcn_overlay_tuned_builder` is dep-satisfied. So this check is
        # exact today rather than approximately right. The two rows flipped on
        # 2026-11-09 were PROBED against this check before the flip landed: with their
        # ci.yml steps stripped, both raise UNBUILT-CLAIM, so `built` is held to the
        # workflow rather than merely stored.
        if row["ci"] == "built":
            crate, kind, name = key.split("/", 2)
            defaults = default_features(crate)
            satisfied_anyway = key in dep_sat or all(f in defaults for f in got)
            if not satisfied_anyway and not _any_ci_invocation_would_build(
                ci, crate, kind, name, got
            ):
                violations.append(
                    f"UNBUILT-CLAIM {key} is declared `built`, but no ci.yml invocation\n"
                    f"            selects package `{crate}` with every feature in {got},\n"
                    f"            and {got} is neither default nor dependency-satisfied.\n"
                    f"            Either the step that backed this row was removed — in\n"
                    f"            which case its tests are silently not running — or the\n"
                    f"            row should say `invisible` and what that costs."
                )
    # A required-feature naming a NON-OPTIONAL dependency is satisfied
    # unconditionally, so the target is not gated at all and must not be declared
    # invisible. See non_optional_deps() for how this was found.
    for key, names in sorted(dep_sat.items()):
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
    #
    # The probe must target a row that is ACTUALLY still invisible, or it goes
    # inert while reading as OK. It did exactly that on 2026-11-08: it faked a
    # `-p forecast --features candle` line, and then that line became REAL in
    # ci.yml and all 13 forecast rows flipped to `built`, leaving the probe
    # nothing to flag. So the subject is now chosen from the live `invisible`
    # set rather than hard-coded, and the probe asserts it found one.
    invisible_now = [
        (k, list(DECLARED[k]["feats"]))
        for k in sorted(DECLARED)
        if DECLARED[k]["ci"] == "invisible" and k in base_found
    ]
    assert invisible_now, (
        "no `invisible` row left to probe MAYBE-BUILT with — either every target "
        "is built by CI now (delete this probe and say so) or the declaration is wrong"
    )
    probe_key, probe_feats = invisible_now[0]
    probe_pkg = probe_key.split("/", 1)[0]
    fake_ci = base_ci + (
        f"\n        run: cargo test -p {probe_pkg} "
        + " ".join(f"--features {f}" for f in probe_feats)
        + " --no-fail-fast\n"
    )
    probes.append(
        (
            f"invisible row that CI would build ({probe_key})",
            "MAYBE-BUILT",
            check(base_found, fake_ci),
        )
    )

    # P7 — a dep-satisfied target wrongly declared invisible. Flip the one row
    # that IS dep-satisfied back to `invisible` and require the new kind.
    saved_ci = DECLARED["strategy/test/tcn_overlay_tuned_builder"]["ci"]
    DECLARED["strategy/test/tcn_overlay_tuned_builder"]["ci"] = "invisible"
    probes.append(
        ("dep-satisfied row declared invisible", "DEP-SATISFIED", check(base_found, base_ci))
    )
    DECLARED["strategy/test/tcn_overlay_tuned_builder"]["ci"] = saved_ci

    # P7 — the ci.yml step backing a `built` row is deleted. Subject: the yahoo
    # ticker-flag step (bug-log #184), chosen because `backtest` declares NO default
    # features, so nothing else in the tree would build that target — if the step goes,
    # the row is simply false. The clean run above is the other side of the pair: with
    # the step present there is no violation, so this probe cannot pass by accident.
    ci_without = "\n".join(
        ln for ln in base_ci.splitlines()
        if "--test run_yahoo_sma_ticker_flag" not in ln
    )
    assert ci_without != base_ci, (
        "P7 is INERT: no ci.yml line narrows to run_yahoo_sma_ticker_flag, so deleting "
        "it changes nothing and the probe tests nothing (bug-log #184)"
    )
    probes.append(
        ("ci step backing a `built` row deleted", "UNBUILT-CLAIM",
         check(base_found, ci_without))
    )

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

    # ── probes for the graph-noop computation (bug-log #186) ───────────────
    # Not a violation KIND, so it cannot ride the loop above. These assert the
    # computation in BOTH directions, because a reporter that can only stay silent
    # is the same defect as a guard that can only skip.
    import tempfile
    for label, optional_in_dep, want in (
        ("non-optional in the dep  -> reported", False, True),
        ("OPTIONAL in the dep      -> silent",   True,  False),
    ):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            (root / "aa").mkdir()
            (root / "bb").mkdir()
            (root / "aa" / "Cargo.toml").write_text(
                "[features]\nfeat = [\"dep:thing\"]\n\n"
                "[dependencies]\nbb = { path = \"../bb\" }\n"
                "thing = { version = \"1\", optional = true }\n"
            )
            dep_line = (
                'thing = { version = "1", optional = true }\n' if optional_in_dep
                else 'thing = "1"\n'
            )
            (root / "bb" / "Cargo.toml").write_text("[dependencies]\n" + dep_line)
            saved_crates = globals()["CRATES"]
            globals()["CRATES"] = root
            try:
                adds = feature_dep_additions("aa", "feat")
                closure = build_closure("aa", {"aa", "bb"})
                got = bool([d for d in adds if d in closure])
            finally:
                globals()["CRATES"] = saved_crates
        if got == want:
            print(f"  OK    graph-noop  {label}")
        else:
            print(f"  FAIL  graph-noop  {label} \u2014 got {got}, want {want} "
                  f"(adds={adds}, in-closure={sorted(set(adds) & closure)})")
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
    noop = graph_noop_report()
    for crate, feat, already in noop:
        print(
            f"  graph-noop  {crate}/{feat} declares dep:{{{','.join(already)}}} "
            f"which a default {crate} build ALREADY compiles — enabling it adds "
            f"NOTHING to the dependency graph (bug-log #186)"
        )
    if not noop:
        print("  graph-noop  none: every declared feature adds something to the graph")
    if violations:
        for v in violations:
            print(v)
        print(f"feature-gated-targets: FAIL ({len(violations)} violation(s))")
        return 1
    print("feature-gated-targets: PASS (0 violations)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
